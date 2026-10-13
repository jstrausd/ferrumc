//! World and simulation bring-up: load the spawn area through the sim
//! load-or-generate flow and pre-build the clientbound packets every joining
//! player receives.
//!
//! The spawn chunks are loaded once at startup into the single shard's
//! [`LoadedChunkMap`](ferrumc_sim::LoadedChunkMap) via
//! [`acquire_spawn`](ferrumc_sim::LoadedChunkMap::acquire_spawn) (try the
//! [`WorldStore`], else generate the selected terrain) and kept resident by a `Spawn`
//! ticket. From them we build a shared [`JoinKit`]: the `JoinGame` packet, the
//! spawn position, and the *list of spawn chunk positions*. Connection tasks clone
//! the `Arc<JoinKit>` and replay the framing the moment a client reaches play.
//!
//! The spawn-area chunk blobs themselves are **not** cached in the kit: they are
//! fetched live from the resident shard chunks at join time (via a
//! [`StreamChunks`](crate::driver::SimCommand::StreamChunks) round-trip), so a
//! block another player placed in a spawn chunk is reflected on every (re)join —
//! a cached snapshot would replay the stale pre-edit terrain instead.

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ferrumc_anvil::{region_pos_from_path, Region};
use ferrumc_codec::{BoundedBytes, BoundedString};
use ferrumc_core::{DimensionId, WorldId};
use ferrumc_math::{BlockPos, ChunkPos, Vec3};
use ferrumc_proto::generated::play::{ChunkDataAndLight, Heightmap, JoinGame, SpawnInfo};
use ferrumc_proto::types::BlockPosition;
use ferrumc_registry::dimension;

use ferrumc_sim::{
    ChunkGenerationExecutor, ChunkTicket, GenerationBinding, LoadedChunkMap, RegionLimits,
    SimShard, SpawnChunkTickets,
};
use ferrumc_storage::{
    ChunkKey, ChunkRecord, GenerationKey, InMemoryStore, PlayerStore, RedbStore, SchemaVersion,
    WorldStore,
};
use ferrumc_world::{
    encode_chunk_section_data, pack_motion_blocking_heightmap, pack_world_surface_heightmap, Chunk,
    ChunkLightData, FallibleChunkGenerator, GenerationIdentity, GeneratorKind, HeightmapKind,
    RuntimeWorldGenerator, WorldSeed,
};

use crate::config::AppConfig;
use crate::persistence::PersistenceTarget;
use ferrumc_config::WorldGenerationPreset;
use sha2::{Digest, Sha256};

/// Placeholder protocol entity id assigned to a joining player in `JoinGame`.
///
/// The simulation does not yet allocate entity ids; `1` stands in for the slice.
const PLAYER_ENTITY_ID: i32 = 1;

/// Maximum players advertised in `JoinGame`. Informational only this milestone.
const MAX_PLAYERS: i32 = 32;

/// Dimension-type registry index for the overworld in the minimal registry.
const OVERWORLD_DIMENSION_TYPE: i32 = 0;

/// Sea level reported in `SpawnInfo`, matching the flat profile's grass surface.
const SEA_LEVEL: i32 = 63;

/// Creative game mode id, sent so a barebones client has a defined mode.
const GAMEMODE_CREATIVE: u8 = 1;

/// "No previous game mode" sentinel (`-1` as an unsigned byte).
const NO_PREVIOUS_GAMEMODE: u8 = u8::MAX;

/// `MOTION_BLOCKING` heightmap type id, the one heightmap the slice transmits.
///
/// The 1.21.5+ array-form heightmap is keyed by numeric type; `4` selects
/// `MOTION_BLOCKING` (the highest block that blocks motion or contains a fluid).
const MOTION_BLOCKING_HEIGHTMAP: i32 = 4;

/// Protocol heightmap type for the highest non-air block.
const WORLD_SURFACE_HEIGHTMAP: i32 = 1;

/// Maximum number of imported chunks buffered before a flush to the store during
/// an Anvil import.
///
/// The import streams region files one at a time and writes the decoded chunks to
/// the store in batches of this size. It bounds the peak save-batch allocation —
/// comfortably under [`ferrumc_storage::MAX_SAVE_BATCH`] (4096) — so a large
/// multi-region map never forces a single giant write nor materialises the whole
/// world in one buffer.
const ANVIL_IMPORT_BATCH: usize = 256;

/// The shared join payload replayed to every player the instant they reach play.
///
/// Built once at startup and shared behind an [`Arc`](std::sync::Arc). It is the
/// keystone framing of the vertical slice: the `JoinGame` that puts the client in
/// play, the absolute spawn position to synchronize to, and the *positions* of the
/// spawn-area chunk columns.
///
/// The chunk blobs are deliberately **not** stored here. A joining connection
/// fetches them live from the resident shard chunks at join time (see
/// [`send_join_kit`](crate::connection)), so an edit placed in a spawn chunk by a
/// previous session is reflected on the next join; a cached snapshot would replay
/// the stale pre-edit terrain.
#[derive(Debug, Clone)]
pub(crate) struct JoinKit {
    /// The `JoinGame` packet announcing the (flat) overworld.
    join_game: JoinGame,
    /// The world-spawn position the player is placed at.
    spawn_position: ferrumc_math::Vec3,
    /// The chunk the spawn point falls in, sent as `Set Center Chunk` so the
    /// client centres its view square on the spawn area before the chunks arrive.
    spawn_chunk: ChunkPos,
    /// The block-aligned world spawn, sent as `Set Default Spawn Position`.
    spawn_block: BlockPosition,
    /// The positions of the spawn-area chunk columns, in deterministic order. The
    /// connection fetches each column's live blob from the shard at join time.
    spawn_positions: Vec<ChunkPos>,
}

impl JoinKit {
    /// The `JoinGame` packet to send first.
    pub(crate) fn join_game(&self) -> &JoinGame {
        &self.join_game
    }

    /// The absolute world-spawn position to synchronize the client to.
    pub(crate) fn spawn_position(&self) -> ferrumc_math::Vec3 {
        self.spawn_position
    }

    /// The chunk the spawn point falls in (the `Set Center Chunk` target).
    pub(crate) fn spawn_chunk(&self) -> ChunkPos {
        self.spawn_chunk
    }

    /// The block-aligned world spawn (the `Set Default Spawn Position` target).
    pub(crate) fn spawn_block(&self) -> BlockPosition {
        self.spawn_block
    }

    /// The chunk positions covered by the spawn batch sent at join.
    ///
    /// A connection both fetches the live blob for each of these from the shard at
    /// join time and seeds its per-player loaded-chunk set with them, so chunk
    /// streaming never re-sends a chunk the client already received at join.
    pub(crate) fn chunk_positions(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.spawn_positions.iter().copied()
    }
}

/// The simulation pieces produced by [`build_world`], ready to drive.
///
/// The shard already owns the resident spawn chunks; `join_kit` is the shared
/// clientbound payload derived from them. The verified target and generator are handed
/// to the driver so it can run the load-or-generate flow for chunks streamed in
/// around a moving player (the same store the spawn area was loaded from, so
/// chunk persistence stays consistent).
pub(crate) struct WorldSetup {
    /// The single simulation shard owning the spawn area.
    pub(crate) shard: SimShard,
    /// The shared join payload replayed to connecting players.
    pub(crate) join_kit: std::sync::Arc<JoinKit>,
    /// The chunk store the driver and storage worker share, behind the
    /// [`WorldStore`] trait so the backend (durable redb or in-memory) is chosen
    /// once at startup and the rest of the app stays backend-agnostic.
    pub(crate) persistence: PersistenceTarget,
    /// The player-record view of the same backend allocation, accessed
    /// through the [`PlayerStore`] trait. Connection tasks load a player's saved
    /// state on join and save it on leave/shutdown through this handle.
    pub(crate) player_store: Arc<dyn PlayerStore>,
    /// The terrain generator the driver uses on a store miss.
    pub(crate) generator: RuntimeGeneration,
}

/// Shared generator and its single bounded CPU lane, reused through startup and streaming.
#[derive(Clone)]
pub(crate) struct RuntimeGeneration {
    generator: Arc<dyn FallibleChunkGenerator>,
    executor: ChunkGenerationExecutor,
}

impl RuntimeGeneration {
    pub(crate) fn new(generator: Arc<dyn FallibleChunkGenerator>) -> Self {
        Self {
            generator,
            // One worker bounds concurrent preview CPU and scratch memory. Full
            // admission fails explicitly; no unbounded generation queue exists.
            executor: ChunkGenerationExecutor::new(NonZeroUsize::MIN),
        }
    }

    pub(crate) fn baseline_generator(&self) -> Arc<dyn FallibleChunkGenerator> {
        Arc::clone(&self.generator)
    }

    pub(crate) fn identity(&self) -> GenerationIdentity {
        self.generator.identity()
    }

    /// Admit a warmed raw baseline through the ordinary storage/overlay gate.
    pub(crate) async fn acquire_warmed(
        &self,
        chunks: &mut LoadedChunkMap,
        store: &dyn WorldStore,
        chunk: Arc<Chunk>,
        ticket: ChunkTicket,
    ) -> anyhow::Result<()> {
        let pos = chunk.pos();
        let generator = Arc::new(WarmedBaseline {
            identity: self.identity(),
            chunk,
        });
        chunks
            .acquire_offthread(store, generator, &self.executor, pos, ticket)
            .await?;
        Ok(())
    }

    pub(crate) async fn acquire(
        &self,
        chunks: &mut LoadedChunkMap,
        store: &dyn WorldStore,
        pos: ChunkPos,
        ticket: ChunkTicket,
    ) -> anyhow::Result<()> {
        chunks
            .acquire_offthread(
                store,
                Arc::clone(&self.generator),
                &self.executor,
                pos,
                ticket,
            )
            .await?;
        Ok(())
    }
}

/// Single-column immutable producer; it never bypasses stored records or overlays.
struct WarmedBaseline {
    identity: GenerationIdentity,
    chunk: Arc<Chunk>,
}

impl FallibleChunkGenerator for WarmedBaseline {
    fn identity(&self) -> GenerationIdentity {
        self.identity
    }

    fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, ferrumc_world::WorldError> {
        if pos != self.chunk.pos() {
            return Err(ferrumc_world::WorldError::NaturalGeneration {
                stage: ferrumc_world::NaturalGenerationStage::Installation,
                detail: "warmed baseline requested for a different column".to_owned(),
            });
        }
        Ok((*self.chunk).clone())
    }
}

/// Returns the chunk position the spawn point falls in.
fn spawn_center_chunk(config: &AppConfig) -> ChunkPos {
    let spawn = config.spawn();
    BlockPos::new(
        spawn.x.floor() as i32,
        spawn.y.floor() as i32,
        spawn.z.floor() as i32,
    )
    .to_chunk_pos()
}

/// Builds the single shard, loads the spawn area through the load-or-generate
/// flow, and pre-builds the shared [`JoinKit`].
///
/// # Errors
///
/// Returns an error if a spawn chunk fails to load from the store, if a spawn
/// chunk is unexpectedly absent after acquisition, or if the dimension name or a
/// chunk blob exceeds its protocol bound.
pub(crate) async fn build_world(
    config: &AppConfig,
    shard_pos: ferrumc_math::ShardPos,
) -> anyhow::Result<WorldSetup> {
    let OpenedStore {
        store,
        player_store,
    } = open_store(config).await?;
    let key = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
    let identity = reconcile_generation(&*store, key, config).await?;
    let runtime_generator = tokio::task::spawn_blocking(move || match identity.generator_kind() {
        GeneratorKind::FerrumcFlat => Ok(RuntimeWorldGenerator::flat(identity.seed())),
        GeneratorKind::FerrumcNaturalPreview => match identity.generator_revision() {
            1 => RuntimeWorldGenerator::natural(identity.seed()).map_err(anyhow::Error::from),
            2 => RuntimeWorldGenerator::natural_with_grass(identity.seed())
                .map_err(anyhow::Error::from),
            _ => anyhow::bail!("unsupported natural generation revision {identity:?}"),
        },
        _ => anyhow::bail!("unsupported persisted generation identity {identity:?}"),
    })
    .await
    .map_err(|error| anyhow::anyhow!("generator binding task failed: {error}"))??;
    let generator = RuntimeGeneration::new(Arc::new(runtime_generator));
    identity.ensure_compatible(generator.identity())?;
    let binding = GenerationBinding::new(key, identity);
    let persistence = PersistenceTarget::verify(Arc::clone(&store), binding).await?;
    let spawn = SpawnChunkTickets::new(spawn_center_chunk(config), config.spawn_chunk_radius());

    let mut shard = SimShard::with_generation(shard_pos, binding);
    shard.enable_placement_completions();
    // Bound the region build commands (/fill, /replace, /undo) with the operator's
    // configured caps; the shard re-checks these defensively on every region edit.
    shard.set_region_limits(RegionLimits {
        max_volume: config.max_region_fill_volume(),
        max_undo_entries: config.region_undo_history(),
        // The per-tick application budget keeps its built-in default; only the
        // operator-facing volume + undo caps are configurable.
        ..RegionLimits::default()
    });

    // If a prebuilt Anvil map is configured, import its region files into the
    // store BEFORE the spawn area is acquired, so the spawn chunks (and any later
    // streamed chunks) come from the loaded map instead of flat terrain. The
    // import keys chunks for the shard's own world/dimension and stamps them with
    // its chunk schema version, so the load-or-generate flow finds them. A
    // missing/empty/malformed map aborts startup here (before connections), rather
    // than silently serving flat terrain.
    if let Some(region_dir) = config.world().anvil_import_dir() {
        let loaded = shard.loaded_chunks();
        let imported = import_anvil_world(
            &*store,
            region_dir,
            loaded.world(),
            loaded.dimension(),
            loaded.schema_version(),
        )
        .await?;
        tracing::info!(
            chunks = imported,
            dir = %region_dir.display(),
            "imported Anvil world into the store",
        );
    }

    let positions: Vec<_> = spawn.positions().collect();
    tracing::info!(
        chunks = positions.len(),
        ?identity,
        "loading bounded spawn area"
    );
    for (index, pos) in positions.iter().copied().enumerate() {
        generator
            .acquire(shard.loaded_chunks_mut(), &*store, pos, spawn.ticket())
            .await?;
        tracing::info!(
            completed = index + 1,
            total = positions.len(),
            x = pos.x(),
            z = pos.z(),
            "spawn chunk ready"
        );
    }
    let spawn_position = if identity.generator_kind() == GeneratorKind::FerrumcNaturalPreview {
        let selected = natural_surface_spawn(shard.loaded_chunks(), config.spawn(), &positions)?;
        tracing::warn!(x = selected.x, y = selected.y, z = selected.z,
            "natural Overworld preview ready: terrain/biomes/surfaces/carvers; features, ores, trees and structures are not populated");
        tracing::warn!("preview lighting computes direct vertical skylight only; lateral light propagation and emitted block light are not implemented");
        selected
    } else {
        config.spawn()
    };

    let join_kit = std::sync::Arc::new(build_join_kit(config, &spawn, identity, spawn_position)?);
    Ok(WorldSetup {
        shard,
        join_kit,
        persistence,
        player_store,
        generator,
    })
}

/// The two trait-object views of the one store backend opened at startup.
///
/// Both `Arc`s point at the *same* concrete backend object, so chunk and player
/// records share one redb file (or one in-memory map). They are separate handles
/// only because the driver/storage-worker reach the backend through [`WorldStore`]
/// while connection tasks reach it through [`PlayerStore`].
struct OpenedStore {
    store: Arc<dyn WorldStore>,
    player_store: Arc<dyn PlayerStore>,
}

/// Opens the world store the runtime will use, selected by
/// [`AppConfig::world_dir`].
///
/// `Some(dir)` opens the durable redb-backed [`RedbStore`] at `dir/world.redb`
/// (creating the directory if needed) — the runtime default. `None` constructs an
/// [`InMemoryStore`], used by tests and any ephemeral run. Either way the concrete
/// backend is erased to the [`WorldStore`] and [`PlayerStore`] trait objects so
/// the driver, storage worker, and connection tasks stay backend-agnostic; the two
/// handles share the single backend object (one redb file / one in-memory map).
///
/// # Errors
///
/// Returns an error if the world directory cannot be created or the redb file
/// cannot be opened.
async fn open_store(config: &AppConfig) -> anyhow::Result<OpenedStore> {
    let world_dir = config.world_dir().map(Path::to_path_buf);
    tokio::task::spawn_blocking(move || open_store_blocking(world_dir.as_deref()))
        .await
        .map_err(|error| anyhow::anyhow!("world store opening task failed: {error}"))?
}

fn open_store_blocking(world_dir: Option<&Path>) -> anyhow::Result<OpenedStore> {
    if let Some(dir) = world_dir {
        std::fs::create_dir_all(dir)
            .map_err(|err| anyhow::anyhow!("creating world directory {}: {err}", dir.display()))?;
        let path = dir.join("world.redb");
        let store = RedbStore::open_with_generation_metadata(&path).map_err(|err| {
            anyhow::anyhow!(
                "opening format-4 world store {}: {err}; legacy format 3 requires an explicit offline migration: ferrumc migrate-generation --source <world.redb> --backup <new-backup.redb> --scope 0:0 --declare-legacy-flat-v1-seed-zero (stop the server first)",
                path.display()
            )
        })?;
        Ok(opened_store(Arc::new(store)))
    } else {
        Ok(opened_store(Arc::new(
            InMemoryStore::with_generation_metadata(),
        )))
    }
}

/// Keeps persisted seed/preset authoritative; omissions only choose defaults on fresh stores.
async fn reconcile_generation(
    store: &dyn WorldStore,
    key: GenerationKey,
    config: &AppConfig,
) -> anyhow::Result<GenerationIdentity> {
    let configured = config.world();
    let stored = store.load_generation_identity(key).await?;
    let seed = WorldSeed::new(
        configured
            .seed()
            .unwrap_or_else(|| stored.map_or(0, |identity| identity.seed().value())),
    );
    let expected = match configured.preset() {
        Some(WorldGenerationPreset::FerrumcFlat) => GenerationIdentity::flat_overworld(seed),
        Some(WorldGenerationPreset::FerrumcNaturalPreview) => {
            GenerationIdentity::natural_overworld(seed)
        }
        Some(WorldGenerationPreset::FerrumcNaturalGrassPreview) => {
            GenerationIdentity::natural_overworld_with_grass(seed)
        }
        Some(preset) => anyhow::bail!("unsupported world generation preset {preset:?}"),
        None => match stored {
            Some(identity) => match identity.generator_kind() {
                GeneratorKind::FerrumcFlat => GenerationIdentity::flat_overworld(seed),
                GeneratorKind::FerrumcNaturalPreview if identity.generator_revision() == 2 => {
                    GenerationIdentity::natural_overworld_with_grass(seed)
                }
                GeneratorKind::FerrumcNaturalPreview => GenerationIdentity::natural_overworld(seed),
                _ => anyhow::bail!("unsupported persisted generation identity {identity:?}"),
            },
            None => GenerationIdentity::flat_overworld(seed),
        },
    };
    if let Some(stored) = stored {
        expected.ensure_compatible(stored)?;
    }
    // A stored natural identity also applies when the preset was omitted. Refuse
    // before declaration/import because Anvil full snapshots omit our palettes.
    anyhow::ensure!(
        expected.generator_kind() != GeneratorKind::FerrumcNaturalPreview
            || configured.anvil_import_dir().is_none(),
        "ferrumc_natural_preview cannot use anvil_import_dir: imported full chunks do not preserve preview biome palettes"
    );
    // This is the first world-bearing startup write. A fresh omission explicitly
    // selects FerrumC-flat-v1 seed zero, never a recovered historical seed.
    store.ensure_generation_identity(key, expected).await?;
    Ok(expected)
}

/// Erases one concrete backend `Arc` into the [`WorldStore`] and [`PlayerStore`]
/// trait-object handles, both sharing the single backend object.
fn opened_store<T: WorldStore + PlayerStore + 'static>(store: Arc<T>) -> OpenedStore {
    OpenedStore {
        store: Arc::clone(&store) as Arc<dyn WorldStore>,
        player_store: store as Arc<dyn PlayerStore>,
    }
}

/// Imports every chunk from every `r.<x>.<z>.mca` region file in `region_dir`
/// into `store`, keyed for `world`/`dimension` and stamped with `schema_version`.
///
/// Returns the number of chunks imported.
///
/// # Bounded memory
///
/// The import is bounded so a large map cannot exhaust memory: region files are
/// processed one at a time, each read, decompressed, and NBT-parsed off the async
/// runtime via [`tokio::task::spawn_blocking`] (the work is blocking file I/O plus
/// CPU-bound parsing, which must never run on a Tokio worker), and the decoded
/// chunks are written to `store` in [`ANVIL_IMPORT_BATCH`]-sized batches. Peak
/// memory is therefore roughly one region's decoded chunks plus one save batch,
/// never the whole world. The Anvil reader itself caps each region file's on-disk
/// size and each chunk's decompressed size (see `ferrumc_anvil::AnvilLimits`).
///
/// # Errors
///
/// Returns an error if `region_dir` cannot be read, if it holds no region files,
/// if a region file is malformed (truncated, bad compression, corrupt NBT, …), or
/// if a store write fails. A malformed map aborts startup rather than silently
/// serving partial or flat terrain.
async fn import_anvil_world(
    store: &dyn WorldStore,
    region_dir: &Path,
    world: WorldId,
    dimension: DimensionId,
    schema_version: SchemaVersion,
) -> anyhow::Result<usize> {
    let import_path = region_dir.to_path_buf();
    let region_dir = import_path.clone();
    let region_paths = tokio::task::spawn_blocking(move || collect_region_paths(&region_dir))
        .await
        .map_err(|error| anyhow::anyhow!("anvil directory task failed: {error}"))??;
    if region_paths.is_empty() {
        anyhow::bail!(
            "anvil import directory {} contains no r.<x>.<z>.mca region files",
            import_path.display()
        );
    }

    let mut imported = 0usize;
    let mut batch: Vec<(ChunkKey, ChunkRecord)> = Vec::with_capacity(ANVIL_IMPORT_BATCH);
    for path in region_paths {
        // Read + decompress + NBT-parse the whole region off the async runtime:
        // blocking file I/O and CPU-bound work, never run on a Tokio worker.
        let chunks =
            tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(ChunkPos, Chunk)>> {
                let region = Region::open(&path)
                    .map_err(|err| anyhow::anyhow!("opening region {}: {err}", path.display()))?;
                region
                    .iter_chunks()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|err| anyhow::anyhow!("decoding region {}: {err}", path.display()))
            })
            .await
            .map_err(|err| anyhow::anyhow!("anvil import task panicked: {err}"))??;

        for (pos, chunk) in chunks {
            batch.push((
                ChunkKey::new(world, dimension, pos),
                ChunkRecord::new(schema_version, chunk),
            ));
            imported += 1;
            // Flush in bounded batches so one large map never forces a giant write.
            if batch.len() >= ANVIL_IMPORT_BATCH {
                let full = std::mem::replace(&mut batch, Vec::with_capacity(ANVIL_IMPORT_BATCH));
                store
                    .save_chunks(full)
                    .await
                    .map_err(|err| anyhow::anyhow!("saving imported chunks: {err}"))?;
            }
        }
    }
    if !batch.is_empty() {
        store
            .save_chunks(batch)
            .await
            .map_err(|err| anyhow::anyhow!("saving imported chunks: {err}"))?;
    }
    Ok(imported)
}

/// Collects the paths of every `r.<x>.<z>.mca` region file in `region_dir`, in a
/// deterministic (sorted) order.
///
/// Non-region entries (`level.dat`, a `data/` subdirectory, …) are skipped.
///
/// # Errors
///
/// Returns an error if `region_dir` cannot be read (for example, it does not
/// exist) — the configured-but-missing case that must fail startup loudly.
fn collect_region_paths(region_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let read_err = |err: std::io::Error| {
        anyhow::anyhow!(
            "reading anvil import directory {}: {err}",
            region_dir.display()
        )
    };
    let entries = std::fs::read_dir(region_dir).map_err(read_err)?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.map_err(read_err)?.path();
        // Only r.<x>.<z>.mca region files; everything else in the directory is
        // ignored so a real world folder (level.dat, data/, …) imports cleanly.
        if region_pos_from_path(&path).is_ok() {
            paths.push(path);
        }
    }
    // Deterministic import order regardless of directory iteration order.
    paths.sort();
    Ok(paths)
}

/// Selects a dry column with two clear blocks from the already bounded spawn area.
/// This preview rule is deliberately narrower than Vanilla's spawn search.
fn natural_surface_spawn(
    chunks: &LoadedChunkMap,
    configured: Vec3,
    positions: &[ChunkPos],
) -> anyhow::Result<Vec3> {
    let requested = BlockPos::new(configured.x.floor() as i32, 0, configured.z.floor() as i32);
    if let Some(y) = safe_surface_y(chunks, requested) {
        return Ok(Vec3::new(configured.x, f64::from(y), configured.z));
    }
    for &pos in positions {
        let Some(chunk) = chunks.get(pos) else {
            continue;
        };
        let heightmap = chunk.heightmap(HeightmapKind::WorldSurface);
        for z in 0..16 {
            for x in 0..16 {
                let column = BlockPos::new(pos.x() * 16 + x, 0, pos.z() * 16 + z);
                if let Some(y) = heightmap
                    .height(x as u8, z as u8)
                    .and_then(|height| safe_column_y(chunk, column, height))
                {
                    return Ok(Vec3::new(
                        f64::from(column.x()) + 0.5,
                        f64::from(y),
                        f64::from(column.z()) + 0.5,
                    ));
                }
            }
        }
    }
    anyhow::bail!("natural preview has no safe dry spawn in the bounded resident spawn area; choose different spawn x/z or seed")
}

fn safe_surface_y(chunks: &LoadedChunkMap, column: BlockPos) -> Option<i32> {
    let chunk = chunks.get(column.to_chunk_pos())?;
    let x = column.x().rem_euclid(16) as u8;
    let z = column.z().rem_euclid(16) as u8;
    let ground_y = chunk.heightmap(HeightmapKind::WorldSurface).height(x, z)?;
    safe_column_y(chunk, column, ground_y)
}

fn safe_column_y(chunk: &Chunk, column: BlockPos, mut ground_y: i32) -> Option<i32> {
    // The optional grass feature adds one passable surface cell. Select its real
    // support, preserving the revision-1 terrain/platform rules.
    let top = chunk.get_block(BlockPos::new(column.x(), ground_y, column.z()))?;
    if is_short_grass(top) {
        ground_y = ground_y.checked_sub(1)?;
    }
    if ground_y >= 318 {
        return None;
    }
    let ground = chunk.get_block(BlockPos::new(column.x(), ground_y, column.z()))?;
    let name = ferrumc_registry::block_state::state_id_to_block_name(ground.as_u32())?;
    // Restrict support to preview terrain materials; fluids, snow layers, leaves
    // and a later imported non-solid block must not become a spawn platform.
    if !matches!(
        name,
        "stone"
            | "deepslate"
            | "dirt"
            | "grass_block"
            | "podzol"
            | "coarse_dirt"
            | "sand"
            | "red_sand"
            | "sandstone"
            | "red_sandstone"
            | "gravel"
            | "snow_block"
            | "ice"
            | "packed_ice"
            | "blue_ice"
            | "terracotta"
            | "white_terracotta"
            | "orange_terracotta"
            | "brown_terracotta"
            | "red_terracotta"
    ) {
        return None;
    }
    for y in [ground_y + 1, ground_y + 2] {
        let state = chunk.get_block(BlockPos::new(column.x(), y, column.z()))?;
        if !state.is_air() && !is_short_grass(state) {
            return None;
        }
    }
    Some(ground_y + 1)
}

fn is_short_grass(state: ferrumc_world::BlockStateId) -> bool {
    ferrumc_registry::block_state::state_id_to_block_name(state.as_u32()) == Some("short_grass")
}

/// Assembles the shared [`JoinKit`] framing.
///
/// Records only the spawn chunk *positions*; the connection fetches each column's
/// live blob from the resident shard chunks at join time, so the kit never holds a
/// stale snapshot of a spawn chunk.
fn build_join_kit(
    config: &AppConfig,
    spawn: &SpawnChunkTickets,
    identity: GenerationIdentity,
    spawn_position: Vec3,
) -> anyhow::Result<JoinKit> {
    let spawn_block = BlockPosition::new(
        spawn_position.x.floor() as i32,
        spawn_position.y.floor() as i32,
        spawn_position.z.floor() as i32,
    );
    Ok(JoinKit {
        join_game: build_join_game(config, identity)?,
        spawn_position,
        spawn_chunk: BlockPos::new(
            spawn_position.x.floor() as i32,
            spawn_position.y.floor() as i32,
            spawn_position.z.floor() as i32,
        )
        .to_chunk_pos(),
        spawn_block,
        spawn_positions: spawn.positions().collect(),
    })
}

/// Builds the `JoinGame` packet for the flat overworld.
fn build_join_game(config: &AppConfig, identity: GenerationIdentity) -> anyhow::Result<JoinGame> {
    let overworld = BoundedString::<32_767>::new(dimension::OVERWORLD.to_string())
        .map_err(|err| anyhow::anyhow!("overworld dimension name invalid: {err}"))?;
    let spawn_info = SpawnInfo::new(
        OVERWORLD_DIMENSION_TYPE,
        overworld.clone(),
        obfuscate_seed(identity.seed()),
        GAMEMODE_CREATIVE,
        NO_PREVIOUS_GAMEMODE,
        false,
        true,
        None,
        0,
        SEA_LEVEL,
    );
    Ok(JoinGame::new(
        PLAYER_ENTITY_ID,
        false,
        vec![overworld],
        MAX_PLAYERS,
        config.view_distance(),
        config.simulation_distance(),
        false,
        true,
        false,
        spawn_info,
        false,
    ))
}

/// Matches the original `BiomeManager`'s Guava `hashLong`/`asLong` byte order.
fn obfuscate_seed(seed: WorldSeed) -> i64 {
    let digest = Sha256::digest(seed.value().to_le_bytes());
    i64::from_le_bytes(std::array::from_fn(|index| digest[index]))
}

/// Builds the `ChunkDataAndLight` packet for one resident chunk.
///
/// The payload is the real 1.21.8 wire form: the paletted section blob from
/// [`encode_chunk_section_data`], the `MOTION_BLOCKING` heightmap packed by
/// [`pack_motion_blocking_heightmap`], and direct vertical sky light from
/// [`ChunkLightData::vertical_sky`] (no block light or lateral propagation).
/// Block entities are sent separately by the driver.
///
/// Shared by the join-kit builder (for the spawn batch) and the driver (for
/// chunks streamed in around a moving player), so both emit byte-identical chunk
/// packets.
///
/// # Errors
///
/// Returns an error if the section blob or a packed heightmap cannot be produced
/// (only for out-of-range block data, which the flat generator never emits), or
/// if an encoded buffer exceeds its protocol length bound.
pub(crate) fn chunk_packet(pos: ChunkPos, chunk: &Chunk) -> anyhow::Result<ChunkDataAndLight> {
    let blob = BoundedBytes::<2_097_152>::new(encode_chunk_section_data(chunk)?)
        .map_err(|err| anyhow::anyhow!("chunk blob exceeds protocol bound: {err}"))?;

    let heightmaps = vec![
        Heightmap::new(
            MOTION_BLOCKING_HEIGHTMAP,
            pack_motion_blocking_heightmap(chunk)?,
        ),
        Heightmap::new(
            WORLD_SURFACE_HEIGHTMAP,
            pack_world_surface_heightmap(chunk)?,
        ),
    ];

    // Preview skylight follows open vertical columns; lateral propagation and
    // emitted block light remain outside this preview's lighting contract.
    let light = ChunkLightData::vertical_sky(chunk);
    let sky_light = light
        .sky_light()
        .iter()
        .map(|section| BoundedBytes::<2048>::new(section.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| anyhow::anyhow!("sky-light section exceeds protocol bound: {err}"))?;

    Ok(ChunkDataAndLight::new(
        pos.x(),
        pos.z(),
        heightmaps,
        blob,
        Vec::new(),
        light.sky_light_mask().to_vec(),
        light.block_light_mask().to_vec(),
        light.empty_sky_light_mask().to_vec(),
        light.empty_block_light_mask().to_vec(),
        sky_light,
        Vec::new(),
    ))
}

#[cfg(test)]
mod tests {
    use ferrumc_math::ShardPos;
    use ferrumc_world::{BlockStateId, FlatWorldGenerator};

    use super::*;
    use crate::config::AppConfig;

    /// The committed Anvil fixture directory (one `r.0.0.mca` region holding the
    /// single real vanilla chunk `(0, 0)`), reused from the `ferrumc-anvil` crate.
    fn anvil_fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crates/ferrumc-anvil/tests/fixtures")
    }

    /// Reads chunk `(0, 0)` straight from the fixture via the Anvil reader: the
    /// source-of-truth terrain to compare the imported/loaded chunk against.
    fn fixture_chunk() -> Chunk {
        Region::open(anvil_fixtures_dir().join("r.0.0.mca"))
            .expect("region opens")
            .read_chunk(0, 0)
            .expect("chunk reads")
            .expect("chunk (0, 0) is present")
    }

    /// Asserts two chunks hold identical blocks across the full buildable column
    /// range. Compares block data only (not dirty masks), so it is robust to the
    /// store round-trip clearing dirty state.
    fn assert_same_blocks(actual: &Chunk, expected: &Chunk) {
        for y in -64..320 {
            for z in 0..16 {
                for x in 0..16 {
                    let pos = BlockPos::new(x, y, z);
                    assert_eq!(
                        actual.get_block(pos),
                        expected.get_block(pos),
                        "block mismatch at {pos:?}"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn import_anvil_world_persists_fixture_chunks() {
        let store = InMemoryStore::new();
        let world = WorldId::new(0);
        let dimension = DimensionId::new(0);

        let imported = import_anvil_world(
            &store,
            &anvil_fixtures_dir(),
            world,
            dimension,
            ferrumc_sim::CHUNK_SCHEMA_VERSION,
        )
        .await
        .expect("fixture imports");
        // The fixture region holds exactly one present chunk, at (0, 0).
        assert_eq!(imported, 1);

        let loaded = store
            .load_chunk(ChunkKey::new(world, dimension, ChunkPos::new(0, 0)))
            .await
            .expect("load succeeds")
            .expect("chunk (0, 0) was imported")
            .into_chunk();

        // The store-loaded chunk matches the chunk the Anvil reader produces
        // directly from the same file: the startup path persisted the real
        // imported terrain, not flat generation.
        assert_same_blocks(&loaded, &fixture_chunk());

        // Explicit known-block spot check: a 1.18+ overworld has a full bedrock
        // floor at y == -64.
        let bedrock = BlockStateId::from_name("bedrock").expect("bedrock is known");
        for z in 0..16 {
            for x in 0..16 {
                assert_eq!(loaded.get_block(BlockPos::new(x, -64, z)), Some(bedrock));
            }
        }
    }

    #[tokio::test]
    async fn import_anvil_world_rejects_a_missing_directory() {
        let store = InMemoryStore::new();
        let missing = anvil_fixtures_dir().join("does-not-exist");
        let err = import_anvil_world(
            &store,
            &missing,
            WorldId::new(0),
            DimensionId::new(0),
            ferrumc_sim::CHUNK_SCHEMA_VERSION,
        )
        .await
        .expect_err("a missing import directory must abort startup");
        assert!(
            err.to_string().contains("anvil import directory"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn build_world_imports_configured_anvil_map() {
        let config = AppConfig::from_toml_str(&format!(
            "[world]\nanvil_import_dir = {:?}\n",
            anvil_fixtures_dir().to_string_lossy()
        ))
        .expect("fixture import config is valid");
        let setup = build_world(&config, ShardPos::new(0, 0))
            .await
            .expect("world builds with anvil import");

        // Spawn radius 2 -> a 5x5 resident square around chunk (0, 0).
        let expected_resident = (2 * usize::from(config.spawn_chunk_radius()) + 1).pow(2);
        assert_eq!(
            setup.shard.loaded_chunks().loaded_count(),
            expected_resident
        );

        // Chunk (0, 0) is present in the fixture, so the resident spawn chunk is
        // the imported vanilla terrain rather than flat generation.
        let resident = setup
            .shard
            .loaded_chunks()
            .get(ChunkPos::new(0, 0))
            .expect("spawn chunk (0, 0) resident");
        assert_same_blocks(resident, &fixture_chunk());

        // A neighbouring spawn chunk absent from the fixture falls back to flat
        // generation: chunk (2, 2) keeps the flat grass surface at y = 63.
        let flat = setup
            .shard
            .loaded_chunks()
            .get(ChunkPos::new(2, 2))
            .expect("neighbour spawn chunk resident");
        let grass = BlockStateId::from_name("grass_block").expect("grass_block is known");
        assert_eq!(
            flat.get_block(ChunkPos::new(2, 2).origin_block(63)),
            Some(grass)
        );
    }

    #[tokio::test]
    async fn build_world_loads_spawn_and_builds_kit() {
        let config = AppConfig::default();
        let setup = build_world(&config, ShardPos::new(0, 0))
            .await
            .expect("world builds");

        // Radius 2 -> a 5x5 spawn square, all resident, all covered by the kit's
        // position list.
        let expected = (2 * usize::from(config.spawn_chunk_radius()) + 1).pow(2);
        assert_eq!(setup.shard.loaded_chunks().loaded_count(), expected);
        assert_eq!(setup.join_kit.chunk_positions().count(), expected);
        assert_eq!(setup.join_kit.spawn_position(), config.spawn());
        assert_eq!(
            setup.join_kit.join_game().view_distance(),
            config.view_distance()
        );
        // The spawn point (8, 64, 8) falls in chunk (0, 0) and block (8, 64, 8).
        assert_eq!(
            setup.join_kit.spawn_chunk(),
            ferrumc_math::ChunkPos::new(0, 0)
        );
        let block = setup.join_kit.spawn_block();
        assert_eq!((block.x(), block.y(), block.z()), (8, 64, 8));

        // Every spawn column builds the real wire payload from its live resident
        // chunk (the same `chunk_packet` the join-time fetch uses): a non-empty
        // paletted section blob, the 37-long MOTION_BLOCKING heightmap, and
        // conservative vertical sky light over all 26 light sections.
        for pos in setup.join_kit.chunk_positions() {
            let resident = setup
                .shard
                .loaded_chunks()
                .get(pos)
                .expect("spawn chunk resident");
            let chunk = chunk_packet(pos, resident).expect("chunk packet builds");
            assert_ne!(chunk.chunk_data().as_slice(), [] as [u8; 0]);

            let heightmaps = chunk.heightmaps();
            assert_eq!(heightmaps.len(), 2);
            assert_eq!(heightmaps[0].kind(), 4);
            assert_eq!(heightmaps[0].data().len(), 37);
            assert_eq!(heightmaps[1].kind(), 1);
            assert_eq!(heightmaps[1].data(), heightmaps[0].data());

            assert_eq!(chunk.sky_light_mask(), &[0x03FF_FFFF]);
            assert_eq!(chunk.empty_block_light_mask(), &[0x03FF_FFFF]);
            assert_eq!(chunk.block_light_mask(), [] as [i64; 0]);
            assert_eq!(chunk.sky_light().len(), 26);
            assert_eq!(
                chunk.block_light(),
                [] as [ferrumc_codec::BoundedBytes<2048>; 0]
            );
            for section in chunk.sky_light() {
                assert_eq!(section.as_slice().len(), 2048);
            }
        }
    }
    #[test]
    fn biome_seed_obfuscation_matches_original_server_jar_goldens() {
        // Oracle invoked original Mojang 1.21.8 dnz.a(long), not a recreated
        // formula. Provenance and runner are beside the committed fixture.
        for line in include_str!("../tests/fixtures/biome_seed_goldens.tsv").lines() {
            let (seed, expected) = line.split_once(' ').expect("golden seed/output");
            let seed = seed.parse::<i64>().expect("numeric original seed");
            let expected = expected.parse::<i64>().expect("numeric original hash");
            assert_eq!(
                obfuscate_seed(WorldSeed::new(seed)),
                expected,
                "seed {seed}"
            );
        }
    }

    #[tokio::test]
    async fn fresh_world_stores_chosen_seed_and_omission_reuses_it_after_restart() {
        let temp = tempfile::tempdir().expect("world directory");
        let config =
            AppConfig::from_toml_str("[world]\nseed = -123456789\npreset = \"ferrumc_flat\"\n")
                .expect("seed config")
                .with_world_dir(Some(temp.path().to_path_buf()))
                .expect("directory");
        let setup = build_world(&config, ShardPos::new(0, 0))
            .await
            .expect("fresh seed world");
        let identity = setup.persistence.binding().identity();
        assert_eq!(identity.seed().value(), -123_456_789);
        assert_eq!(setup.generator.identity(), identity);
        assert_eq!(
            setup.join_kit.join_game().world_state().hashed_seed(),
            -3_252_562_806_299_980_166
        );
        let key = setup.persistence.binding().key();
        assert_eq!(
            setup
                .persistence
                .store()
                .load_generation_identity(key)
                .await
                .expect("stored seed"),
            Some(identity)
        );
        drop(setup);
        let omitted = AppConfig::default()
            .with_world_dir(Some(temp.path().to_path_buf()))
            .expect("directory");
        let restarted = build_world(&omitted, ShardPos::new(0, 0))
            .await
            .expect("omitted seed restart");
        assert_eq!(restarted.persistence.binding().identity(), identity);
        assert_eq!(
            restarted.join_kit.join_game().world_state().hashed_seed(),
            -3_252_562_806_299_980_166
        );
    }

    #[tokio::test]
    async fn explicit_seed_mismatch_refuses_before_import_and_keeps_identity() {
        let store = InMemoryStore::with_generation_metadata();
        let key = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
        let original = GenerationIdentity::flat_overworld(WorldSeed::new(41));
        store
            .ensure_generation_identity(key, original)
            .await
            .expect("original identity");
        let key_chunk = ChunkKey::new(key.world(), key.dimension(), ChunkPos::new(0, 0));
        let record = ChunkRecord::new(
            ferrumc_sim::CHUNK_SCHEMA_VERSION,
            FlatWorldGenerator::with_seed(original.seed()).generate(key_chunk.pos()),
        );
        store
            .save_chunk(key_chunk, record.clone())
            .await
            .expect("original chunk");
        let config = AppConfig::from_toml_str(
            "[world]\nseed = 42\nanvil_import_dir = \"/missing-before-mismatch\"\n",
        )
        .expect("mismatch config");
        let error = reconcile_generation(&store, key, &config)
            .await
            .expect_err("explicit mismatch");
        assert!(error
            .to_string()
            .contains("incompatible generation identity"));
        assert_eq!(
            store
                .load_generation_identity(key)
                .await
                .expect("original identity retained"),
            Some(original)
        );
        assert_eq!(
            store
                .load_chunk(key_chunk)
                .await
                .expect("original chunk retained"),
            Some(record)
        );
    }

    #[tokio::test]
    async fn fresh_omission_explicitly_selects_flat_seed_zero() {
        let setup = build_world(&AppConfig::default(), ShardPos::new(0, 0))
            .await
            .expect("fresh omitted world");
        assert_eq!(
            setup.persistence.binding().identity(),
            GenerationIdentity::flat_overworld(WorldSeed::new(0))
        );
        assert_eq!(
            setup.join_kit.join_game().world_state().hashed_seed(),
            8_794_265_229_978_523_055
        );
    }
    #[tokio::test]
    async fn natural_identity_omission_preserves_preset_and_explicit_changes_are_refused() {
        let store = InMemoryStore::with_generation_metadata();
        let key = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
        let selected =
            AppConfig::from_toml_str("[world]\nseed = 17\npreset = \"ferrumc_natural_preview\"\n")
                .expect("natural config");
        let identity = reconcile_generation(&store, key, &selected)
            .await
            .expect("fresh natural identity");
        assert_eq!(
            identity,
            GenerationIdentity::natural_overworld(WorldSeed::new(17))
        );
        assert_eq!(
            reconcile_generation(&store, key, &AppConfig::default())
                .await
                .expect("omitted restart"),
            identity
        );
        for config in [
            "[world]\npreset = \"ferrumc_flat\"\n",
            "[world]\nseed = 18\n",
        ] {
            let changed = AppConfig::from_toml_str(config).expect("mismatch config");
            assert!(reconcile_generation(&store, key, &changed).await.is_err());
            assert_eq!(
                store
                    .load_generation_identity(key)
                    .await
                    .expect("retained identity"),
                Some(identity)
            );
        }
    }

    #[tokio::test]
    async fn grass_preview_omission_retains_revision_and_rejects_baseline_switches() {
        let store = InMemoryStore::with_generation_metadata();
        let key = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
        let identity = GenerationIdentity::natural_overworld_with_grass(WorldSeed::new(-42));
        store
            .ensure_generation_identity(key, identity)
            .await
            .expect("declare grass");
        assert_eq!(
            reconcile_generation(&store, key, &AppConfig::default())
                .await
                .expect("reuse"),
            identity
        );
        for declaration in [
            "preset='ferrumc_natural_preview'",
            "seed=43",
            "anvil_import_dir='/unused'",
        ] {
            let config =
                AppConfig::from_toml_str(&format!("[world]\n{declaration}")).expect("parse");
            assert!(
                reconcile_generation(&store, key, &config).await.is_err(),
                "{declaration}"
            );
            assert_eq!(
                store.load_generation_identity(key).await.expect("read"),
                Some(identity)
            );
        }
    }

    #[test]
    fn short_grass_is_passable_spawn_space_on_a_real_support() {
        let mut chunk = FlatWorldGenerator::new().generate(ChunkPos::ORIGIN);
        let grass = BlockStateId::new(
            ferrumc_registry::block_state::block_default_state("short_grass")
                .expect("original grass"),
        );
        chunk
            .set_block(BlockPos::new(8, 64, 8), grass)
            .expect("place plant");
        assert_eq!(safe_column_y(&chunk, BlockPos::new(8, 0, 8), 64), Some(64));
        chunk
            .set_block(BlockPos::new(8, 63, 8), BlockStateId::AIR)
            .expect("remove support");
        assert_eq!(safe_column_y(&chunk, BlockPos::new(8, 0, 8), 64), None);
    }

    #[tokio::test]
    async fn natural_spawn_uses_dry_surface_and_refuses_all_water_ring() {
        let store = InMemoryStore::new();
        let pos = ChunkPos::new(0, 0);
        let mut chunks = LoadedChunkMap::new(WorldId::new(0), DimensionId::new(0));
        chunks
            .acquire(
                &store,
                &FlatWorldGenerator::new(),
                pos,
                SpawnChunkTickets::new(pos, 0).ticket(),
            )
            .await
            .expect("resident terrain");
        let configured = Vec3::new(8.5, 200.0, 8.5);
        assert_eq!(
            natural_surface_spawn(&chunks, configured, &[pos]).expect("dry surface"),
            Vec3::new(8.5, 64.0, 8.5)
        );
        let water = BlockStateId::new(
            ferrumc_registry::block_state::block_default_state("water").expect("water registry"),
        );
        let chunk = chunks.get_mut(pos).expect("resident chunk");
        for z in 0..16 {
            for x in 0..16 {
                chunk
                    .set_block(BlockPos::new(x, 64, z), water)
                    .expect("water above ground");
            }
        }
        assert!(natural_surface_spawn(&chunks, configured, &[pos]).is_err());
    }
    #[tokio::test]
    async fn stored_natural_identity_refuses_omitted_preset_anvil_before_world_changes() {
        let store = InMemoryStore::with_generation_metadata();
        let key = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
        let original = GenerationIdentity::natural_overworld(WorldSeed::new(21));
        store
            .ensure_generation_identity(key, original)
            .await
            .expect("stored natural identity");
        let chunk_key = ChunkKey::new(key.world(), key.dimension(), ChunkPos::new(0, 0));
        let record = ChunkRecord::new(
            ferrumc_sim::CHUNK_SCHEMA_VERSION,
            FlatWorldGenerator::new().generate(chunk_key.pos()),
        );
        store
            .save_chunk(chunk_key, record.clone())
            .await
            .expect("original full snapshot");
        let config = AppConfig::from_toml_str("[world]\nanvil_import_dir = \"/unused/region\"\n")
            .expect("omission resolved only after storage read");
        let error = reconcile_generation(&store, key, &config)
            .await
            .expect_err("stored natural import refused");
        assert!(error.to_string().contains("cannot use anvil_import_dir"));
        assert_eq!(
            store
                .load_generation_identity(key)
                .await
                .expect("identity untouched"),
            Some(original)
        );
        assert_eq!(
            store.load_chunk(chunk_key).await.expect("chunk untouched"),
            Some(record)
        );
    }
}
