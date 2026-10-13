//! Public generator admission, immutable store authorization and restart tests.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use ferrumc_core::{DimensionId, Result as CoreResult, ServerError, WorldId};
use ferrumc_math::{BlockPos, ChunkPos, ShardPos};
use ferrumc_sim::{
    load_or_generate, load_or_generate_bound, load_or_generate_bound_offthread,
    load_or_generate_legacy, ChunkGenerationExecutor, ChunkProvenance, ChunkTicket,
    GenerationBinding, LoadedChunkMap, SimError, SimShard, TicketReason, CHUNK_SCHEMA_VERSION,
    OVERLAY_SCHEMA_VERSION,
};
use ferrumc_storage::{
    BlockMutationLogRecord, ChunkKey, ChunkOverlayRecord, ChunkRecord, EntityKey, EntityRecord,
    GenerationInitialization, GenerationKey, InMemoryStore, RedbStore, SchemaVersion, StorageError,
    WorldStore,
};
use ferrumc_world::{
    BlockEntity, BlockStateId, Chunk, ChunkGenerator, FallibleChunkGenerator, FlatWorldGenerator,
    GenerationIdentity, Sign, SignKind, WorldError, WorldSeed,
};

const WORLD: WorldId = WorldId::new(7);
const DIMENSION: DimensionId = DimensionId::new(13);
const SCOPE: GenerationKey = GenerationKey::new(WORLD, DIMENSION);

fn identity(seed: i64) -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(seed))
}

fn binding() -> GenerationBinding {
    GenerationBinding::new(SCOPE, identity(-982))
}

fn key(pos: ChunkPos) -> ChunkKey {
    ChunkKey::new(WORLD, DIMENSION, pos)
}

fn ticket() -> ChunkTicket {
    ChunkTicket::of(TicketReason::Spawn)
}

fn one() -> NonZeroUsize {
    NonZeroUsize::new(1).unwrap()
}

fn generator() -> FlatWorldGenerator {
    FlatWorldGenerator::with_seed(binding().identity().seed())
}

/// An immutable read-only boundary fixture; malformed payloads are deliberate
/// negative admission inputs, not a compliant producer or a mutable store.
struct ReadStore {
    metadata: CoreResult<Option<GenerationIdentity>>,
    base: Option<ChunkRecord>,
    overlay: Option<ChunkOverlayRecord>,
    metadata_completion: Option<Arc<AtomicBool>>,
    metadata_reads: AtomicUsize,
    world_reads: AtomicUsize,
    writes: AtomicUsize,
}

impl ReadStore {
    fn new(metadata: CoreResult<Option<GenerationIdentity>>) -> Self {
        Self {
            metadata,
            base: None,
            overlay: None,
            metadata_completion: None,
            metadata_reads: AtomicUsize::new(0),
            world_reads: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
        }
    }

    fn refuse_write(&self) -> ServerError {
        self.writes.fetch_add(1, Ordering::Relaxed);
        ServerError::unsupported("read-only fixture")
    }
}

#[async_trait]
impl WorldStore for ReadStore {
    async fn load_generation_identity(
        &self,
        _key: GenerationKey,
    ) -> CoreResult<Option<GenerationIdentity>> {
        self.metadata_reads.fetch_add(1, Ordering::Relaxed);
        if let Some(completion) = &self.metadata_completion {
            tokio::task::yield_now().await;
            completion.store(true, Ordering::Relaxed);
        }
        self.metadata.clone()
    }

    async fn ensure_generation_identity(
        &self,
        _key: GenerationKey,
        _requested: GenerationIdentity,
    ) -> CoreResult<GenerationInitialization> {
        Err(self.refuse_write())
    }

    async fn load_chunk(&self, _key: ChunkKey) -> CoreResult<Option<ChunkRecord>> {
        self.world_reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.base.clone())
    }

    async fn load_chunk_overlay(&self, _key: ChunkKey) -> CoreResult<Option<ChunkOverlayRecord>> {
        self.world_reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.overlay.clone())
    }

    async fn save_chunk(&self, _key: ChunkKey, _record: ChunkRecord) -> CoreResult<()> {
        Err(self.refuse_write())
    }

    async fn save_chunks(&self, _chunks: Vec<(ChunkKey, ChunkRecord)>) -> CoreResult<()> {
        Err(self.refuse_write())
    }

    async fn delete_chunk(&self, _key: ChunkKey) -> CoreResult<bool> {
        Err(self.refuse_write())
    }

    async fn save_chunk_overlays(
        &self,
        _overlays: Vec<(ChunkKey, ChunkOverlayRecord)>,
    ) -> CoreResult<()> {
        Err(self.refuse_write())
    }

    async fn append_block_mutations(
        &self,
        _mutations: Vec<BlockMutationLogRecord>,
    ) -> CoreResult<()> {
        Err(self.refuse_write())
    }

    async fn load_entity(&self, _key: EntityKey) -> CoreResult<Option<EntityRecord>> {
        Ok(None)
    }

    async fn save_entity(&self, _key: EntityKey, _record: EntityRecord) -> CoreResult<()> {
        Err(self.refuse_write())
    }

    async fn save_entities(&self, _entities: Vec<(EntityKey, EntityRecord)>) -> CoreResult<()> {
        Err(self.refuse_write())
    }

    async fn delete_entity(&self, _key: EntityKey) -> CoreResult<bool> {
        Err(self.refuse_write())
    }
}

struct CountingGenerator {
    baseline: GenerationIdentity,
    output: Option<ChunkPos>,
    identity_calls: AtomicUsize,
    generated: AtomicUsize,
}

impl CountingGenerator {
    fn new(baseline: GenerationIdentity) -> Self {
        Self {
            baseline,
            output: None,
            identity_calls: AtomicUsize::new(0),
            generated: AtomicUsize::new(0),
        }
    }
}

impl ChunkGenerator for CountingGenerator {
    fn identity(&self) -> GenerationIdentity {
        self.identity_calls.fetch_add(1, Ordering::Relaxed);
        self.baseline
    }

    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        self.generated.fetch_add(1, Ordering::Relaxed);
        FlatWorldGenerator::with_seed(self.baseline.seed()).generate(self.output.unwrap_or(pos))
    }
}

#[test]
fn declaration_is_explicit_and_shard_constructors_preserve_scope_and_capacity() {
    let declared = binding();
    assert_eq!(declared.key(), SCOPE);
    assert_eq!(declared.identity(), identity(-982));
    let map = LoadedChunkMap::with_generation_and_schema_version(declared, SchemaVersion::new(9));
    assert_eq!(map.world(), WORLD);
    assert_eq!(map.dimension(), DIMENSION);
    assert_eq!(map.generation_binding(), Some(declared));
    assert_eq!(map.schema_version(), SchemaVersion::new(9));
    assert_eq!(map.loaded_count(), 0);
    let shard = SimShard::with_generation(ShardPos::new(0, 0), declared);
    assert_eq!(shard.loaded_chunks().generation_binding(), Some(declared));
    let shard = SimShard::with_generation_and_inbox_capacity(ShardPos::new(0, 0), declared, one());
    assert_eq!(shard.inbox_capacity(), 1);
    assert_eq!(shard.loaded_chunks().world(), WORLD);
    assert_eq!(shard.loaded_chunks().dimension(), DIMENSION);
}

#[tokio::test]
async fn metadata_refusals_are_classified_and_never_become_chunk_misses_or_initialization() {
    let failures = [
        Ok(None),
        Ok(Some(identity(982))),
        Err(StorageError::GenerationMetadataUnsupported.into()),
        Err(StorageError::GenerationIdentityDecode {
            key: SCOPE,
            source: GenerationIdentity::decode(&[]).unwrap_err(),
        }
        .into()),
    ];
    for (index, metadata) in failures.into_iter().enumerate() {
        let store = ReadStore::new(metadata);
        let generator = CountingGenerator::new(binding().identity());
        let mut map = LoadedChunkMap::with_generation(binding());
        let error = map
            .acquire(&store, &generator, ChunkPos::ORIGIN, ticket())
            .await
            .unwrap_err();
        match index {
            0 => assert_eq!(error, SimError::GenerationIdentityMissing { key: SCOPE }),
            1 => assert_eq!(
                error,
                SimError::GenerationIdentityMismatch {
                    key: SCOPE,
                    expected: binding().identity(),
                    stored: identity(982),
                }
            ),
            2 => assert!(matches!(
                error,
                SimError::GenerationStore {
                    source: ServerError::Unsupported(_),
                    ..
                }
            )),
            3 => assert!(matches!(
                error,
                SimError::GenerationStore {
                    source: ServerError::Internal { .. },
                    ..
                }
            )),
            _ => unreachable!(),
        }
        assert_eq!(map.loaded_count(), 0);
        assert_eq!(map.ticket_count(ChunkPos::ORIGIN), 0);
        assert!(!map.has_persist_dirty());
        assert_eq!(store.metadata_reads.load(Ordering::Relaxed), 1);
        assert_eq!(store.world_reads.load(Ordering::Relaxed), 0);
        assert_eq!(store.writes.load(Ordering::Relaxed), 0);
        assert_eq!(generator.generated.load(Ordering::Relaxed), 0);
    }
}

#[tokio::test]
async fn scope_and_column_bounds_refuse_before_any_generator_or_storage_call() {
    let store = ReadStore::new(Ok(Some(binding().identity())));
    let generator = CountingGenerator::new(binding().identity());
    for wrong_key in [
        ChunkKey::new(WorldId::new(8), DIMENSION, ChunkPos::ORIGIN),
        ChunkKey::new(WORLD, DimensionId::new(14), ChunkPos::ORIGIN),
    ] {
        assert!(matches!(
            load_or_generate_bound(&store, &generator, binding(), wrong_key).await,
            Err(SimError::GenerationScopeMismatch { .. })
        ));
    }
    for pos in [
        ChunkPos::new(-134_217_729, 0),
        ChunkPos::new(134_217_728, 0),
        ChunkPos::new(0, i32::MIN),
        ChunkPos::new(0, i32::MAX),
    ] {
        assert!(
            matches!(load_or_generate_bound(&store, &generator, binding(), key(pos)).await,
            Err(SimError::ChunkLoad { pos: failed, .. }) if failed == pos)
        );
    }
    assert_eq!(store.metadata_reads.load(Ordering::Relaxed), 0);
    assert_eq!(store.world_reads.load(Ordering::Relaxed), 0);
    assert_eq!(generator.identity_calls.load(Ordering::Relaxed), 0);
    assert_eq!(generator.generated.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn complete_column_extremes_are_accepted_and_regenerated_deterministically() {
    let store = ReadStore::new(Ok(Some(binding().identity())));
    for pos in [
        ChunkPos::new(-134_217_728, 134_217_727),
        ChunkPos::new(134_217_727, -134_217_728),
    ] {
        let (chunk, provenance) = load_or_generate_bound(&store, &generator(), binding(), key(pos))
            .await
            .unwrap();
        assert_eq!(provenance, ChunkProvenance::Generated);
        assert_eq!(chunk, generator().generate(pos));
    }
}

fn edit(map: &mut LoadedChunkMap, pos: ChunkPos, y: i32, state: u32) -> BlockPos {
    let block = pos.origin_block(y);
    let chunk = map.get_mut(pos).unwrap();
    chunk.set_block(block, BlockStateId::new(state)).unwrap();
    chunk.mark_persist_dirty(block);
    block
}

#[tokio::test]
async fn resident_ticket_acquire_rechecks_store_and_generator_without_mutating_dirty_facts() {
    let store = InMemoryStore::with_generation_metadata();
    store
        .ensure_generation_identity(SCOPE, binding().identity())
        .await
        .unwrap();
    let mut map = LoadedChunkMap::with_generation(binding());
    let pos = ChunkPos::ORIGIN;
    map.acquire(&store, &generator(), pos, ticket())
        .await
        .unwrap();
    edit(&mut map, pos, 70, 41);
    let before = map.get(pos).unwrap().clone();
    let wrong_generator = CountingGenerator::new(identity(77));
    assert!(matches!(
        map.acquire(&store, &wrong_generator, pos, ticket()).await,
        Err(SimError::GeneratorIdentityMismatch { .. })
    ));
    assert_eq!(wrong_generator.generated.load(Ordering::Relaxed), 0);
    for metadata in [
        Ok(None),
        Ok(Some(identity(77))),
        Err(ServerError::capacity("fixture")),
    ] {
        let incompatible = ReadStore::new(metadata);
        assert!(map
            .acquire(&incompatible, &generator(), pos, ticket())
            .await
            .is_err());
        assert_eq!(map.ticket_count(pos), 1);
        assert_eq!(map.get(pos), Some(&before));
        assert_eq!(incompatible.world_reads.load(Ordering::Relaxed), 0);
    }
    assert!(map
        .acquire(&store, &generator(), pos, ticket())
        .await
        .unwrap()
        .is_already_resident());
    assert_eq!(map.ticket_count(pos), 2);
    assert_eq!(map.get(pos), Some(&before));
}

#[tokio::test]
async fn malformed_generator_output_never_enters_residency() {
    let store = ReadStore::new(Ok(Some(binding().identity())));
    let mut generator = CountingGenerator::new(binding().identity());
    generator.output = Some(ChunkPos::new(1, 0)); // Deliberately violates the generator contract.
    let mut map = LoadedChunkMap::with_generation(binding());
    let error = map
        .acquire(&store, &generator, ChunkPos::ORIGIN, ticket())
        .await
        .unwrap_err();
    assert!(matches!(error, SimError::ChunkLoad { .. }));
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(map.ticket_count(ChunkPos::ORIGIN), 0);
    assert_eq!(store.world_reads.load(Ordering::Relaxed), 1);
    assert_eq!(store.writes.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn generator_mismatch_refuses_before_world_reads_and_unstable_identity_refuses_admission() {
    // Deliberately violates ChunkGenerator's stable identity requirement.
    struct UnstableGenerator(AtomicUsize);
    impl ChunkGenerator for UnstableGenerator {
        fn identity(&self) -> GenerationIdentity {
            if self.0.load(Ordering::Relaxed) == 0 {
                binding().identity()
            } else {
                identity(77)
            }
        }
        fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
            self.0.fetch_add(1, Ordering::Relaxed);
            generator().generate(pos)
        }
    }
    let store = ReadStore::new(Ok(Some(binding().identity())));
    let incompatible = CountingGenerator::new(identity(77));
    let mut map = LoadedChunkMap::with_generation(binding());
    assert!(matches!(
        map.acquire(&store, &incompatible, ChunkPos::ORIGIN, ticket())
            .await,
        Err(SimError::GeneratorIdentityMismatch { .. })
    ));
    assert_eq!(store.metadata_reads.load(Ordering::Relaxed), 0);
    assert_eq!(store.world_reads.load(Ordering::Relaxed), 0);
    assert_eq!(incompatible.generated.load(Ordering::Relaxed), 0);
    let unstable = UnstableGenerator(AtomicUsize::new(0));
    assert!(matches!(
        map.acquire(&store, &unstable, ChunkPos::ORIGIN, ticket())
            .await,
        Err(SimError::GeneratorIdentityMismatch { .. })
    ));
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(map.ticket_count(ChunkPos::ORIGIN), 0);
    assert_eq!(store.writes.load(Ordering::Relaxed), 0);
}

// Metadata is immutable. Only the deliberately contract-violating generator
// changes during its awaited read; there are no timers or scheduling races.
struct MetadataChangingGenerator {
    completed: Arc<AtomicBool>,
    generated: AtomicUsize,
}

impl ChunkGenerator for MetadataChangingGenerator {
    fn identity(&self) -> GenerationIdentity {
        if self.completed.load(Ordering::Relaxed) {
            identity(77)
        } else {
            binding().identity()
        }
    }

    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        self.generated.fetch_add(1, Ordering::Relaxed);
        generator().generate(pos)
    }
}

fn metadata_changing_fixture() -> (ReadStore, MetadataChangingGenerator) {
    let completed = Arc::new(AtomicBool::new(false));
    let generator = MetadataChangingGenerator {
        completed: Arc::clone(&completed),
        generated: AtomicUsize::new(0),
    };
    let mut store = ReadStore::new(Ok(Some(binding().identity())));
    store.metadata_completion = Some(completed);
    (store, generator)
}

fn assert_metadata_await_refusal(
    store: &ReadStore,
    generator: &MetadataChangingGenerator,
    error: &SimError,
) {
    assert_eq!(
        error,
        &SimError::GeneratorIdentityMismatch {
            key: SCOPE,
            expected: binding().identity(),
            actual: identity(77),
        }
    );
    assert_eq!(store.metadata_reads.load(Ordering::Relaxed), 1);
    assert_eq!(
        store.world_reads.load(Ordering::Relaxed),
        0,
        "identity changed during metadata await; no world read is authorized"
    );
    assert_eq!(store.writes.load(Ordering::Relaxed), 0);
    assert_eq!(generator.generated.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn metadata_await_identity_change_refuses_free_loader_before_the_first_world_read() {
    let (store, generator) = metadata_changing_fixture();
    let error = load_or_generate_bound(&store, &generator, binding(), key(ChunkPos::ORIGIN))
        .await
        .unwrap_err();
    assert_metadata_await_refusal(&store, &generator, &error);
}

#[tokio::test]
async fn metadata_await_identity_change_refuses_acquire_before_the_first_world_read() {
    let (store, generator) = metadata_changing_fixture();
    let mut map = LoadedChunkMap::with_generation(binding());
    let error = map
        .acquire(&store, &generator, ChunkPos::ORIGIN, ticket())
        .await
        .unwrap_err();
    assert_metadata_await_refusal(&store, &generator, &error);
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(map.ticket_count(ChunkPos::ORIGIN), 0);
}

#[tokio::test]
async fn metadata_capture_refusals_preserve_dirty_chunks_without_world_io_or_writes() {
    let admitted = ReadStore::new(Ok(Some(binding().identity())));
    let mut map = LoadedChunkMap::with_generation(binding());
    let pos = ChunkPos::ORIGIN;
    map.acquire(&admitted, &generator(), pos, ticket())
        .await
        .unwrap();
    edit(&mut map, pos, 70, 41);
    let before = map.get(pos).unwrap().clone();
    for metadata in [
        Ok(None),
        Ok(Some(identity(77))),
        Err(ServerError::unsupported("metadata")),
        Err(StorageError::GenerationIdentityDecode {
            key: SCOPE,
            source: GenerationIdentity::decode(&[]).unwrap_err(),
        }
        .into()),
    ] {
        let rejected = ReadStore::new(metadata);
        assert!(map
            .try_take_persist_dirty_bound(&rejected, 1, one())
            .await
            .is_err());
        assert_eq!(map.get(pos), Some(&before));
        assert_eq!(map.ticket_count(pos), 1);
        assert_eq!(rejected.world_reads.load(Ordering::Relaxed), 0);
        assert_eq!(rejected.writes.load(Ordering::Relaxed), 0);
    }
}

#[tokio::test]
async fn stored_full_base_and_complete_overlay_entity_snapshot_remain_authoritative() {
    let store = InMemoryStore::with_generation_metadata();
    store
        .ensure_generation_identity(SCOPE, binding().identity())
        .await
        .unwrap();
    let pos = ChunkPos::new(4, -8);
    let mut base = Chunk::new(pos);
    let imported = pos.origin_block(24);
    let edited = pos.origin_block(70);
    let sign_pos = pos.origin_block(130);
    base.set_block(imported, BlockStateId::new(41)).unwrap();
    base.set_block(edited, BlockStateId::new(42)).unwrap();
    base.set_block_entity(sign_pos, BlockEntity::Sign(Sign::new(SignKind::Sign)))
        .unwrap();
    store
        .save_chunk(
            key(pos),
            ChunkRecord::new(SchemaVersion::new(9), base.clone()),
        )
        .await
        .unwrap();
    let empty =
        ChunkOverlayRecord::try_from_chunk(OVERLAY_SCHEMA_VERSION, pos, &Chunk::new(pos), 1)
            .unwrap();
    store
        .save_chunk_overlays(vec![(key(pos), empty)])
        .await
        .unwrap();
    let mut map = LoadedChunkMap::with_generation(binding());
    let counted = CountingGenerator::new(binding().identity());
    assert_eq!(
        map.acquire(&store, &counted, pos, ticket())
            .await
            .unwrap()
            .provenance(),
        Some(ChunkProvenance::Loaded)
    );
    assert_eq!(counted.generated.load(Ordering::Relaxed), 0);
    assert_eq!(
        map.get(pos).unwrap().block_entity(sign_pos),
        base.block_entity(sign_pos)
    );
    assert_eq!(
        map.get(pos).unwrap().get_block(imported),
        Some(BlockStateId::new(41))
    );
    edit(&mut map, pos, 70, 51);
    map.get_mut(pos).unwrap().remove_block_entity(sign_pos);
    map.get_mut(pos).unwrap().mark_persist_dirty(sign_pos);
    let batch = map
        .try_take_persist_dirty_bound(&store, 2, one())
        .await
        .unwrap();
    store.save_chunk_overlays(batch).await.unwrap();
    let (reloaded, provenance) = load_or_generate_bound(&store, &counted, binding(), key(pos))
        .await
        .unwrap();
    assert_eq!(provenance, ChunkProvenance::Loaded);
    assert_eq!(reloaded.get_block(imported), Some(BlockStateId::new(41)));
    assert_eq!(reloaded.get_block(edited), Some(BlockStateId::new(51)));
    assert_eq!(reloaded.block_entity(sign_pos), None);
    assert!(!reloaded.dirty_sections().any());
    assert!(!reloaded.persist_dirty_sections().any());
    assert!(reloaded.persist_edited_sections().any());
    assert_eq!(counted.generated.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn overlay_schema_and_wrong_column_fail_the_bound_admission() {
    let pos = ChunkPos::ORIGIN;
    for (schema, overlay_pos) in [
        (SchemaVersion::new(2), pos),
        (SchemaVersion::new(4), pos),
        (OVERLAY_SCHEMA_VERSION, ChunkPos::new(1, 0)),
    ] {
        let mut store = ReadStore::new(Ok(Some(binding().identity())));
        store.overlay = Some(
            ChunkOverlayRecord::try_from_chunk(schema, overlay_pos, &Chunk::new(overlay_pos), 0)
                .unwrap(),
        );
        let mut map = LoadedChunkMap::with_generation(binding());
        assert!(matches!(
            map.acquire(&store, &generator(), pos, ticket()).await,
            Err(SimError::ChunkLoad { .. })
        ));
        assert_eq!(map.loaded_count(), 0);
        assert_eq!(map.ticket_count(pos), 0);
        assert_eq!(store.writes.load(Ordering::Relaxed), 0);
    }
}

#[tokio::test]
async fn verified_capture_accepts_arc_clones_without_any_tick_path_storage_io() {
    let concrete = Arc::new(ReadStore::new(Ok(Some(binding().identity()))));
    let store: Arc<dyn WorldStore> = concrete.clone();
    let verified = binding().verify_store(&store).await.unwrap();
    assert_eq!(verified.binding(), binding());
    let mut map = LoadedChunkMap::with_generation(binding());
    map.acquire(store.as_ref(), &generator(), ChunkPos::ORIGIN, ticket())
        .await
        .unwrap();
    edit(&mut map, ChunkPos::ORIGIN, 70, 51);
    let metadata_reads = concrete.metadata_reads.load(Ordering::Relaxed);
    let world_reads = concrete.world_reads.load(Ordering::Relaxed);
    let cloned = Arc::clone(&store);
    let batch = map
        .try_take_persist_dirty_verified(&cloned, &verified.clone(), 4, one())
        .unwrap();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].1.updated_at_tick(), 4);
    assert!(!map.has_persist_dirty());
    assert_eq!(
        concrete.metadata_reads.load(Ordering::Relaxed),
        metadata_reads
    );
    assert_eq!(concrete.world_reads.load(Ordering::Relaxed), world_reads);
    assert_eq!(concrete.writes.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn verified_capture_refuses_other_allocation_scope_and_baseline_without_clearing() {
    let store: Arc<dyn WorldStore> = Arc::new(ReadStore::new(Ok(Some(binding().identity()))));
    let verified = binding().verify_store(&store).await.unwrap();
    let mut map = LoadedChunkMap::with_generation(binding());
    let pos = ChunkPos::ORIGIN;
    map.acquire(store.as_ref(), &generator(), pos, ticket())
        .await
        .unwrap();
    edit(&mut map, pos, 70, 51);
    let before = map.get(pos).unwrap().clone();
    let other: Arc<dyn WorldStore> = Arc::new(ReadStore::new(Ok(Some(binding().identity()))));
    assert_eq!(
        map.try_take_persist_dirty_verified(&other, &verified, 1, one()),
        Err(SimError::GenerationStoreAuthorizationMismatch { key: SCOPE })
    );
    let other_scope = GenerationBinding::new(
        GenerationKey::new(WORLD, DimensionId::new(14)),
        binding().identity(),
    );
    // The read-only fixture returns its fixed descriptor; it has no mutable metadata.
    let scoped = other_scope.verify_store(&store).await.unwrap();
    assert!(matches!(
        map.try_take_persist_dirty_verified(&store, &scoped, 1, one()),
        Err(SimError::GenerationScopeMismatch { .. })
    ));
    let other_identity = GenerationBinding::new(SCOPE, identity(77));
    let different: Arc<dyn WorldStore> = Arc::new(ReadStore::new(Ok(Some(identity(77)))));
    let mismatched = other_identity.verify_store(&different).await.unwrap();
    assert!(matches!(
        map.try_take_persist_dirty_verified(&different, &mismatched, 1, one()),
        Err(SimError::GenerationIdentityMismatch { .. })
    ));
    assert_eq!(map.get(pos), Some(&before));
    assert_eq!(map.ticket_count(pos), 1);
    assert_eq!(
        map.try_take_persist_dirty_verified(&store, &verified, 2, one())
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn weak_authorization_does_not_keep_backend_alive_or_authorize_a_replacement() {
    let store: Arc<dyn WorldStore> = Arc::new(ReadStore::new(Ok(Some(binding().identity()))));
    let weak = Arc::downgrade(&store);
    let verified = binding().verify_store(&store).await.unwrap();
    drop(store);
    assert!(weak.upgrade().is_none());
    let replacement: Arc<dyn WorldStore> = Arc::new(ReadStore::new(Ok(Some(binding().identity()))));
    let mut map = LoadedChunkMap::with_generation(binding());
    assert_eq!(
        map.try_take_persist_dirty_verified(&replacement, &verified, 1, one()),
        Err(SimError::GenerationStoreAuthorizationMismatch { key: SCOPE })
    );
}

#[tokio::test]
async fn explicit_legacy_aliases_preserve_format3_and_refuse_metadata_stores() {
    let legacy = InMemoryStore::new();
    let flat = FlatWorldGenerator::new();
    let pos = ChunkPos::ORIGIN;
    assert_eq!(
        load_or_generate(&legacy, &flat, key(pos)).await.unwrap(),
        load_or_generate_legacy(&legacy, &flat, key(pos))
            .await
            .unwrap()
    );
    let mut map = LoadedChunkMap::new_legacy(WORLD, DIMENSION);
    map.acquire(&legacy, &flat, pos, ticket()).await.unwrap();
    edit(&mut map, pos, 70, 51);
    let before = map.get(pos).unwrap().clone();
    let metadata = InMemoryStore::with_generation_metadata();
    for initialized in [false, true] {
        if initialized {
            metadata
                .ensure_generation_identity(SCOPE, flat.identity())
                .await
                .unwrap();
        }
        assert_eq!(
            map.acquire(&metadata, &flat, pos, ticket()).await,
            Err(SimError::GenerationBindingRequired { key: SCOPE })
        );
        assert!(matches!(
            load_or_generate_legacy(&metadata, &flat, key(pos)).await,
            Err(SimError::GenerationBindingRequired { .. })
        ));
        assert_eq!(map.ticket_count(pos), 1);
        assert_eq!(map.get(pos), Some(&before));
    }
    assert_eq!(
        map.try_take_persist_dirty_bound(&metadata, 1, one()).await,
        Err(SimError::GenerationBindingRequired { key: SCOPE })
    );
    assert_eq!(map.get(pos), Some(&before));
    assert_eq!(
        map.try_take_persist_dirty_bounded(1, one()).unwrap().len(),
        1
    );
    assert_eq!(
        LoadedChunkMap::new(WORLD, DIMENSION).generation_binding(),
        None
    );
    assert_eq!(
        LoadedChunkMap::with_schema_version(WORLD, DIMENSION, SchemaVersion::new(9))
            .schema_version(),
        SchemaVersion::new(9)
    );
}

#[tokio::test]
async fn dyn_generators_cannot_replace_the_explicit_seed_zero_legacy_baseline() {
    let legacy = InMemoryStore::new();
    let mut map = LoadedChunkMap::new_legacy(WORLD, DIMENSION);
    let pos = ChunkPos::ORIGIN;
    let incompatible = CountingGenerator::new(identity(77));
    assert!(
        matches!(map.acquire(&legacy, &incompatible, pos, ticket()).await,
        Err(SimError::GeneratorIdentityMismatch { expected, actual, .. })
        if expected == identity(0) && actual == identity(77))
    );
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(incompatible.generated.load(Ordering::Relaxed), 0);
    map.acquire(&legacy, &FlatWorldGenerator::new(), pos, ticket())
        .await
        .unwrap();
    assert!(matches!(
        map.acquire(&legacy, &incompatible, pos, ticket()).await,
        Err(SimError::GeneratorIdentityMismatch { .. })
    ));
    assert_eq!(map.ticket_count(pos), 1);
    assert!(matches!(
        load_or_generate_legacy(
            &legacy,
            &FlatWorldGenerator::with_seed(WorldSeed::new(77)),
            key(pos)
        )
        .await,
        Err(SimError::GeneratorIdentityMismatch { .. })
    ));
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "ferrumc-wg003-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("create private restart fixture: {error}"),
            }
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn regenerated_overlay_accumulates_edits_across_real_redb_restarts_and_refuses_new_seed() {
    let directory = TestDirectory::new();
    let path = directory.0.join("world.redb");
    let pos = ChunkPos::new(-3, 9);
    let first = pos.origin_block(70);
    let second = pos.origin_block(130);
    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    store
        .ensure_generation_identity(SCOPE, binding().identity())
        .await
        .unwrap();
    let mut map = LoadedChunkMap::with_generation(binding());
    map.acquire(&store, &generator(), pos, ticket())
        .await
        .unwrap();
    assert!(!map.has_persist_dirty());
    assert_eq!(
        map.try_take_persist_dirty_bound(&store, 1, one())
            .await
            .unwrap(),
        Vec::new()
    );
    assert!(store.load_chunk(key(pos)).await.unwrap().is_none());
    edit(&mut map, pos, 70, 41);
    let batch = map
        .try_take_persist_dirty_bound(&store, 2, one())
        .await
        .unwrap();
    store.save_chunk_overlays(batch).await.unwrap();
    drop(map);
    drop(store);

    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    assert_eq!(
        store.load_generation_identity(SCOPE).await.unwrap(),
        Some(binding().identity())
    );
    let mut map = LoadedChunkMap::with_generation(binding());
    assert_eq!(
        map.acquire(&store, &generator(), pos, ticket())
            .await
            .unwrap()
            .provenance(),
        Some(ChunkProvenance::Loaded)
    );
    assert_eq!(
        map.get(pos).unwrap().get_block(first),
        Some(BlockStateId::new(41))
    );
    assert!(!map.has_persist_dirty());
    edit(&mut map, pos, 130, 42);
    let batch = map
        .try_take_persist_dirty_bound(&store, 3, one())
        .await
        .unwrap();
    assert_eq!(batch[0].1.section_count(), 2);
    store.save_chunk_overlays(batch).await.unwrap();
    drop(map);
    drop(store);

    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    let changed = GenerationBinding::new(SCOPE, identity(983));
    let mut rejected = LoadedChunkMap::with_generation(changed);
    assert!(matches!(
        rejected
            .acquire(
                &store,
                &FlatWorldGenerator::with_seed(changed.identity().seed()),
                pos,
                ticket()
            )
            .await,
        Err(SimError::GenerationIdentityMismatch { .. })
    ));
    assert_eq!(rejected.loaded_count(), 0);
    assert_eq!(rejected.ticket_count(pos), 0);
    let mut map = LoadedChunkMap::with_generation(binding());
    map.acquire(&store, &generator(), pos, ticket())
        .await
        .unwrap();
    let chunk = map.get(pos).unwrap();
    assert_eq!(chunk.get_block(first), Some(BlockStateId::new(41)));
    assert_eq!(chunk.get_block(second), Some(BlockStateId::new(42)));
    assert_eq!(
        chunk.get_block(pos.origin_block(63)),
        generator().generate(pos).get_block(pos.origin_block(63))
    );
    assert!(!chunk.persist_dirty_sections().any());
    assert!(!chunk.dirty_sections().any());
    assert!(store.load_chunk(key(pos)).await.unwrap().is_none());
    assert_eq!(
        store.load_generation_identity(SCOPE).await.unwrap(),
        Some(binding().identity())
    );
}

#[test]
fn generation_and_capture_error_conversion_preserves_backend_classification() {
    for source in [
        ServerError::unsupported("metadata"),
        ServerError::capacity("metadata"),
        ServerError::internal("decode"),
    ] {
        assert_eq!(
            ServerError::from(SimError::GenerationStore {
                key: SCOPE,
                source: source.clone()
            }),
            source
        );
    }
    assert!(matches!(
        ServerError::from(SimError::GenerationIdentityMissing { key: SCOPE }),
        ServerError::InvalidState(_)
    ));
    let source = StorageError::ChunkPositionOutsideDomain {
        pos: ChunkPos::new(i32::MAX, 0),
    };
    assert_eq!(
        ServerError::from(SimError::PersistCapture {
            source: source.clone()
        }),
        ServerError::from(source)
    );
    assert_eq!(CHUNK_SCHEMA_VERSION, SchemaVersion::new(1));
}

struct FallibleFixture {
    identity: GenerationIdentity,
    generated: AtomicUsize,
    failure: Option<WorldError>,
    wrong_column: bool,
    creator: std::thread::ThreadId,
    used_worker: AtomicBool,
}

impl FallibleFixture {
    fn new(failure: Option<WorldError>) -> Self {
        Self {
            identity: binding().identity(),
            generated: AtomicUsize::new(0),
            failure,
            wrong_column: false,
            creator: std::thread::current().id(),
            used_worker: AtomicBool::new(false),
        }
    }
}

impl FallibleChunkGenerator for FallibleFixture {
    fn identity(&self) -> GenerationIdentity {
        self.identity
    }

    fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
        self.generated.fetch_add(1, Ordering::Relaxed);
        self.used_worker.store(
            std::thread::current().id() != self.creator,
            Ordering::Relaxed,
        );
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(Chunk::new(if self.wrong_column {
            ChunkPos::new(19, 23)
        } else {
            pos
        }))
    }
}

fn executor() -> ChunkGenerationExecutor {
    ChunkGenerationExecutor::new(one())
}

#[tokio::test]
async fn offthread_producer_failure_preserves_error_and_never_admits_residency() {
    let store = ReadStore::new(Ok(Some(binding().identity())));
    let error = WorldError::ValueTooWide { value: 7, bits: 1 };
    let generator = Arc::new(FallibleFixture::new(Some(error.clone())));
    let mut map = LoadedChunkMap::with_generation(binding());
    let pos = ChunkPos::ORIGIN;
    assert_eq!(
        map.acquire_offthread(&store, generator.clone(), &executor(), pos, ticket())
            .await,
        Err(SimError::ChunkGeneration { pos, source: error })
    );
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(map.ticket_count(pos), 0);
    assert_eq!(generator.generated.load(Ordering::Relaxed), 1);
    assert!(generator.used_worker.load(Ordering::Relaxed));
    assert_eq!(store.world_reads.load(Ordering::Relaxed), 1);
    assert_eq!(store.writes.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn offthread_stored_full_base_bypasses_a_failing_producer() {
    let mut store = ReadStore::new(Ok(Some(binding().identity())));
    let pos = ChunkPos::ORIGIN;
    let block = pos.origin_block(100);
    let mut chunk = Chunk::new(pos);
    chunk.set_block(block, BlockStateId::new(41)).unwrap();
    store.base = Some(ChunkRecord::new(CHUNK_SCHEMA_VERSION, chunk));
    let generator = Arc::new(FallibleFixture::new(Some(
        WorldError::InvalidBitsPerEntry { bits: 0 },
    )));
    let (chunk, provenance) = load_or_generate_bound_offthread(
        &store,
        generator.clone(),
        &executor(),
        binding(),
        key(pos),
    )
    .await
    .unwrap();
    assert_eq!(provenance, ChunkProvenance::Loaded);
    assert_eq!(chunk.get_block(block).unwrap(), BlockStateId::new(41));
    assert_eq!(generator.generated.load(Ordering::Relaxed), 0);
    assert!(!chunk.dirty_sections().any());
}

#[tokio::test]
async fn offthread_scope_domain_and_identity_refuse_before_world_reads() {
    let store = ReadStore::new(Ok(Some(binding().identity())));
    let generator = Arc::new(FallibleFixture::new(None));
    assert!(matches!(
        load_or_generate_bound_offthread(
            &store,
            generator.clone(),
            &executor(),
            binding(),
            ChunkKey::new(WorldId::new(1), DIMENSION, ChunkPos::ORIGIN)
        )
        .await,
        Err(SimError::GenerationScopeMismatch { .. })
    ));
    assert!(matches!(
        load_or_generate_bound_offthread(
            &store,
            generator.clone(),
            &executor(),
            binding(),
            key(ChunkPos::new(i32::MAX, 0))
        )
        .await,
        Err(SimError::ChunkLoad { .. })
    ));
    let mut wrong = FallibleFixture::new(None);
    wrong.identity = identity(333);
    assert!(matches!(
        load_or_generate_bound_offthread(
            &store,
            Arc::new(wrong),
            &executor(),
            binding(),
            key(ChunkPos::ORIGIN)
        )
        .await,
        Err(SimError::GeneratorIdentityMismatch { .. })
    ));
    assert_eq!(store.metadata_reads.load(Ordering::Relaxed), 0);
    assert_eq!(store.world_reads.load(Ordering::Relaxed), 0);
    assert_eq!(generator.generated.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn offthread_missing_or_mismatched_metadata_never_generates() {
    let generator = Arc::new(FallibleFixture::new(None));
    for metadata in [
        Ok(None),
        Ok(Some(identity(99))),
        Err(ServerError::unsupported("metadata")),
    ] {
        let store = ReadStore::new(metadata);
        let mut map = LoadedChunkMap::with_generation(binding());
        assert!(map
            .acquire_offthread(
                &store,
                generator.clone(),
                &executor(),
                ChunkPos::ORIGIN,
                ticket()
            )
            .await
            .is_err());
        assert_eq!(map.loaded_count(), 0);
        assert_eq!(store.world_reads.load(Ordering::Relaxed), 0);
    }
    assert_eq!(generator.generated.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn offthread_wrong_produced_column_refuses_without_ticket() {
    let store = ReadStore::new(Ok(Some(binding().identity())));
    let mut generator = FallibleFixture::new(None);
    generator.wrong_column = true;
    let mut map = LoadedChunkMap::with_generation(binding());
    assert!(matches!(
        map.acquire_offthread(
            &store,
            Arc::new(generator),
            &executor(),
            ChunkPos::ORIGIN,
            ticket()
        )
        .await,
        Err(SimError::ChunkLoad { .. })
    ));
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(map.ticket_count(ChunkPos::ORIGIN), 0);
}

#[tokio::test]
async fn offthread_flat_path_matches_existing_bytes_and_resident_ticket_semantics() {
    let store = ReadStore::new(Ok(Some(binding().identity())));
    let pos = ChunkPos::new(-3, 2);
    let generator = Arc::new(generator());
    let expected = generator.generate(pos);
    let mut map = LoadedChunkMap::with_generation(binding());
    assert_eq!(
        map.acquire_offthread(&store, generator.clone(), &executor(), pos, ticket())
            .await
            .unwrap(),
        ferrumc_sim::ChunkAcquired::NewlyResident(ChunkProvenance::Generated)
    );
    assert_eq!(map.get(pos), Some(&expected));
    assert!(map
        .acquire_offthread(&store, generator, &executor(), pos, ticket())
        .await
        .unwrap()
        .is_already_resident());
    assert_eq!(map.ticket_count(pos), 2);
    assert_eq!(store.world_reads.load(Ordering::Relaxed), 2);
}

impl FallibleChunkGenerator for MetadataChangingGenerator {
    fn identity(&self) -> GenerationIdentity {
        ChunkGenerator::identity(self)
    }

    fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
        Ok(ChunkGenerator::generate_chunk(self, pos))
    }
}

#[tokio::test]
async fn offthread_metadata_await_identity_change_refuses_before_world_read_or_task() {
    let (store, generator) = metadata_changing_fixture();
    let generator = Arc::new(generator);
    let error = load_or_generate_bound_offthread(
        &store,
        generator.clone(),
        &executor(),
        binding(),
        key(ChunkPos::ORIGIN),
    )
    .await
    .unwrap_err();
    assert_metadata_await_refusal(&store, &generator, &error);
    let (store, generator) = metadata_changing_fixture();
    let generator = Arc::new(generator);
    let mut map = LoadedChunkMap::with_generation(binding());
    let error = map
        .acquire_offthread(
            &store,
            generator.clone(),
            &executor(),
            ChunkPos::ORIGIN,
            ticket(),
        )
        .await
        .unwrap_err();
    assert_metadata_await_refusal(&store, &generator, &error);
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(map.ticket_count(ChunkPos::ORIGIN), 0);
}
