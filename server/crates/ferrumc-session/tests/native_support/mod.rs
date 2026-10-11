//! Shared NET-04.2e1 test environment: genuine simulation admissions on real
//! memory and redb ownership stores in counter mode, the router with the
//! validated registry layout, and a driver-like lane pump. No fake facts.
#![allow(dead_code, reason = "each test binary uses a different subset")]

use std::num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ferrumc_codec::BoundedString;
use ferrumc_core::{DimensionId, GameMode, PlayerId, Tick, WorldId};
use ferrumc_math::{ChunkPos, ShardPos, Vec3};
use ferrumc_proto::generated::configuration::{KnownPack, RegistryData, RegistryEntry};
use ferrumc_proto::generated::play::{JoinGame, SpawnInfo};
use ferrumc_session::{
    CollectorSessionLimits, NativeJoinReservation, NativeShardReceiver, PlayerSessionHandle,
    SessionRouter,
};
use ferrumc_sim::{
    ChunkTicket, CollectorLifecycleLimits, GenerationBinding, ItemPublicationLimits,
    NativePlayerSnapshot, OwnedItemCounterHandle, OwnedItemCounterLimits, OwnedPlayerHandle,
    OwnedPlayerInput, OwnershipIntent, OwnershipStoreBinding, PlayerOwnershipLimits,
    PlayerOwnershipTickOutcome, ShardId, ShardPartitioner, SimShard, TicketReason,
    MAX_COLLECTOR_LIFECYCLE_BYTES, MAX_CONFIRMED_ITEM_BYTES, MAX_ITEM_ENTITY_BYTES_PER_SHARD,
    MAX_ITEM_PUBLICATION_PREPARE_BYTES, MAX_ITEM_PUBLICATION_RETAINED_BYTES,
    MAX_OWNED_COUNTER_PLAN_BYTES, MAX_OWNED_COUNTER_PLAN_ENTRIES, MAX_OWNED_ENTITY_SAVE_CHARGE,
    MAX_OWNED_PLAYER_BYTES, MAX_PLAYER_OWNERSHIP_INPUT_BYTES, MAX_PLAYER_OWNERSHIP_PENDING_BYTES,
};
use ferrumc_storage::{
    GenerationKey, InMemoryStore, OwnershipCapacity, OwnershipConfig, OwnershipOpenLimits,
    OwnershipTransferId, PlayerEntityOwnershipStore, PlayerOwnershipCasRequest,
    PlayerOwnershipProposal, RedbStore, ScopedPlayerKey, WorldStore, MAX_OWNERSHIP_REQUEST_BYTES,
};
use ferrumc_world::{FlatWorldGenerator, GenerationIdentity, WorldSeed};

/// Independent SHA-256 `BiomeZoomSeed` fixture of seed 37 (little-endian bytes),
/// the same value the simulation's own producer test pins.
pub const SEED_37_HASH: i64 = 8_828_412_754_801_553_711;
pub const VIEW_DISTANCE: i32 = 10;
pub const SIMULATION_DISTANCE: i32 = 10;

pub trait Backend: PlayerEntityOwnershipStore + WorldStore {}
impl<T: PlayerEntityOwnershipStore + WorldStore> Backend for T {}

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

pub fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
pub fn id(n: u8) -> OwnershipTransferId {
    OwnershipTransferId::from_bytes([n; 16])
}
pub fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(0), DimensionId::new(0))
}
pub fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(37))
}
pub fn generation() -> GenerationBinding {
    GenerationBinding::new(scope(), identity())
}
pub fn home() -> ShardId {
    ShardPartitioner::for_shard(WorldId::new(0), DimensionId::new(0), ShardPos::new(0, 0)).unwrap()
}
pub fn player(name: &str) -> PlayerId {
    PlayerId::offline(name)
}
pub fn spawn() -> Vec3 {
    Vec3::new(8.5, 64.0, 8.5)
}
fn config() -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            nz(MAX_OWNERSHIP_REQUEST_BYTES),
            NonZeroU64::new(10_000).unwrap(),
            NonZeroU64::new(10_000).unwrap(),
        )
        .unwrap(),
        OwnershipOpenLimits::new(
            NonZeroU64::new(1_000_000).unwrap(),
            NonZeroU64::new(128 * 1024 * 1024).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU16::new(8).unwrap(),
        )
        .unwrap(),
    )
}

/// Which real ownership backend an environment uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    Memory,
    Redb,
}

pub const STORES: [Store; 2] = [Store::Memory, Store::Redb];

/// One native shard initialized in the contract's order (§10.2): ownership,
/// publications, lifecycle, counter mode at `Tick::ZERO`, initial refill.
pub struct SimEnv {
    pub shard: SimShard,
    pub store: Arc<dyn PlayerEntityOwnershipStore>,
    pub binding: OwnershipStoreBinding,
    pub counter: OwnedItemCounterHandle,
    pub tick: Tick,
    backend: Arc<dyn Backend>,
    path: Option<PathBuf>,
    next_id: u8,
}

impl Drop for SimEnv {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl SimEnv {
    pub async fn new(store: Store) -> Self {
        Self::with_inputs(store, 32).await
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the contract's ordered native initialization"
    )]
    pub async fn with_inputs(store: Store, inputs: usize) -> Self {
        let (backend, path): (Arc<dyn Backend>, Option<PathBuf>) = match store {
            Store::Memory => (
                Arc::new(InMemoryStore::with_player_entity_ownership(config()).unwrap()),
                None,
            ),
            Store::Redb => {
                let path = std::env::temp_dir().join(format!(
                    "ferrumc-net042e1-{}-{}.redb",
                    std::process::id(),
                    NEXT_FILE.fetch_add(1, Ordering::SeqCst)
                ));
                (
                    Arc::new(
                        RedbStore::open_with_player_entity_ownership(&path, config()).unwrap(),
                    ),
                    Some(path),
                )
            }
        };
        backend
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let store: Arc<dyn PlayerEntityOwnershipStore> = backend.clone();
        let binding = OwnershipStoreBinding::new(&store, generation());
        let mut shard = SimShard::with_generation(ShardPos::new(0, 0), binding.generation());
        shard
            .initialize_entities_with_item_budget(nz(8), nz(MAX_ITEM_ENTITY_BYTES_PER_SHARD))
            .unwrap();
        shard
            .loaded_chunks_mut()
            .acquire(
                backend.as_ref(),
                &FlatWorldGenerator::with_seed(WorldSeed::new(37)),
                ChunkPos::ORIGIN,
                ChunkTicket::of(TicketReason::Spawn),
            )
            .await
            .unwrap();
        shard
            .initialize_player_ownership(
                binding.clone(),
                PlayerOwnershipLimits::new(
                    nz(16),
                    nz(MAX_OWNED_PLAYER_BYTES),
                    nz(32),
                    nz(MAX_PLAYER_OWNERSHIP_PENDING_BYTES),
                    nz(inputs),
                    nz(MAX_PLAYER_OWNERSHIP_INPUT_BYTES),
                )
                .unwrap(),
            )
            .unwrap();
        shard
            .initialize_item_publications(
                ItemPublicationLimits::new(
                    nz(1),
                    nz(MAX_ITEM_PUBLICATION_RETAINED_BYTES),
                    nz(MAX_ITEM_PUBLICATION_PREPARE_BYTES),
                )
                .unwrap(),
            )
            .unwrap();
        shard
            .initialize_collector_lifecycle(
                CollectorLifecycleLimits::new(nz(MAX_COLLECTOR_LIFECYCLE_BYTES)).unwrap(),
            )
            .unwrap();
        let counter = shard
            .initialize_owned_item_counters(
                OwnedItemCounterLimits::new(
                    nz(MAX_CONFIRMED_ITEM_BYTES),
                    nz(64),
                    nz(64),
                    nz(MAX_OWNED_COUNTER_PLAN_ENTRIES),
                    nz(MAX_OWNED_COUNTER_PLAN_BYTES),
                    nz(MAX_OWNED_ENTITY_SAVE_CHARGE),
                    NonZeroU32::new(100).unwrap(),
                    1,
                    nz(4),
                )
                .unwrap(),
                Tick::ZERO,
            )
            .unwrap();
        let mut env = Self {
            shard,
            store,
            binding,
            counter,
            tick: Tick::ZERO,
            backend,
            path,
            next_id: 100,
        };
        let ids = (0..4).map(|_| env.fresh_id()).collect();
        env.shard
            .enqueue_owned_player_input(OwnedPlayerInput::refill_item_save_ids(
                env.counter.clone(),
                ids,
            ))
            .unwrap();
        let mut out = env.boundary();
        assert!(out.take_refusals().is_empty(), "initial refill applied");
        env
    }

    pub fn fresh_id(&mut self) -> OwnershipTransferId {
        self.next_id = self.next_id.checked_add(1).unwrap();
        id(self.next_id)
    }

    /// One checked normal boundary at exactly the next logical tick.
    pub fn boundary(&mut self) -> PlayerOwnershipTickOutcome {
        let tick = self.tick.next().unwrap();
        let out = self.shard.run_tick_with_player_ownership_at(tick).unwrap();
        self.tick = tick;
        out
    }

    /// Admits `player` through a genuine stored Survival FCPS3 restore at `position`.
    pub async fn restore(&mut self, player: PlayerId, position: Vec3) -> OwnedPlayerHandle {
        let key = ScopedPlayerKey::new(scope(), player);
        let before = self.store.load_owned_player(key, identity()).await.unwrap();
        let snapshot = NativePlayerSnapshot::empty_survival(position, [-0.0, 12.5]);
        let transfer = self.fresh_id();
        self.store
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    transfer,
                    before.expectation().clone(),
                    PlayerOwnershipProposal::put(snapshot.try_to_record().unwrap()),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let read = self.binding.read_player(&self.store, player).await.unwrap();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::restore(read))
            .unwrap();
        let mut out = self.boundary();
        assert!(out.take_refusals().is_empty(), "restore admitted");
        admitted(&out)
    }

    /// Admits a never-seen `player` through a genuine receipt-backed bootstrap.
    pub async fn bootstrap(&mut self, player: PlayerId, position: Vec3) -> OwnedPlayerHandle {
        let read = self.binding.read_player(&self.store, player).await.unwrap();
        let transfer = self.fresh_id();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::bootstrap(
                read,
                position,
                [-0.0, 12.5],
                transfer,
            ))
            .unwrap();
        let mut out = self.boundary();
        let mut work = out.take_work();
        assert_eq!(work.len(), 1, "bootstrap work");
        let completion = work.remove(0).execute(&self.store).await.unwrap();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
            .unwrap();
        let mut out = self.boundary();
        assert!(out.take_refusals().is_empty(), "bootstrap admitted");
        admitted(&out)
    }

    /// Applies one native disconnect directly in the simulation.
    pub fn detach_in_sim(&mut self, handle: &OwnedPlayerHandle) {
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::disconnect(handle.clone()))
            .unwrap();
        let mut out = self.boundary();
        assert!(out.take_refusals().is_empty(), "disconnect applied");
    }

    /// Saves the genuine admission through a receipt-backed autosave and
    /// returns the selected hotbar slot of the durable record.
    pub async fn persisted_selected(&mut self, handle: &OwnedPlayerHandle) -> u8 {
        let transfer = self.fresh_id();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::autosave(handle.clone(), transfer))
            .unwrap();
        let mut out = self.boundary();
        assert!(out.take_refusals().is_empty(), "autosave admitted");
        let mut work = out.take_work();
        assert_eq!(work.len(), 1);
        let completion = work.remove(0).execute(&self.store).await.unwrap();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
            .unwrap();
        let mut out = self.boundary();
        assert!(out.take_refusals().is_empty(), "autosave confirmed");
        let loaded = self
            .store
            .load_owned_player(ScopedPlayerKey::new(scope(), handle.player()), identity())
            .await
            .unwrap();
        NativePlayerSnapshot::try_from_record(loaded.record().unwrap())
            .unwrap()
            .selected()
    }

    pub fn position(&self, handle: &OwnedPlayerHandle) -> Vec3 {
        self.shard
            .connected_owned_player_view(handle)
            .unwrap()
            .position()
    }
}

pub fn admitted(out: &PlayerOwnershipTickOutcome) -> OwnedPlayerHandle {
    out.intents()
        .iter()
        .find_map(|intent| match intent {
            OwnershipIntent::PlayerAdmitted { player, .. } => Some(player.clone()),
            _ => None,
        })
        .expect("a genuine PlayerAdmitted intent")
}

/// The exact typed dimension registry and core pack of the app's
/// `ConfigRegistries::build` producer.
pub fn registries() -> (RegistryData, KnownPack) {
    (
        RegistryData::new(
            bs("minecraft:dimension_type"),
            vec![RegistryEntry::new(bs("minecraft:overworld"), None)],
        ),
        KnownPack::new(bs("minecraft"), bs("core"), bs("1.21.8")),
    )
}

pub fn bs(value: &str) -> BoundedString<32_767> {
    BoundedString::new(value.to_owned()).unwrap()
}

pub fn limits(sessions: usize) -> CollectorSessionLimits {
    CollectorSessionLimits::new(
        nz(sessions),
        nz(sessions - 1),
        nz(sessions * 8),
        nz(sessions),
        nz(4096),
        nz(65_536),
    )
    .unwrap()
}

/// A router with collector sessions initialized for the supported layout.
pub fn router(lane: usize, reserve: usize, sessions: usize) -> SessionRouter {
    let mut router = SessionRouter::with_capacities_and_control_reserve(lane, 64, reserve);
    let (registry, pack) = registries();
    router
        .initialize_collector_sessions(limits(sessions), &registry, &pack, SIMULATION_DISTANCE)
        .unwrap();
    router
}

/// Every `JoinGame` field, so a test can change exactly one.
#[derive(Debug, Clone)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors the protocol's JoinGame flags"
)]
pub struct JoinFields {
    pub entity_id: i32,
    pub hardcore: bool,
    pub world_names: Vec<&'static str>,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug: bool,
    pub respawn_screen: bool,
    pub limited_crafting: bool,
    pub secure_chat: bool,
    pub dimension_type: i32,
    pub dimension_name: &'static str,
    pub hashed_seed: i64,
    pub gamemode: u8,
    pub previous_gamemode: u8,
    pub debug: bool,
    pub flat: bool,
    pub portal_cooldown: i32,
    pub sea_level: i32,
}

impl JoinFields {
    /// The values the validated layout requires for `mode` and seed 37.
    pub fn valid(mode: GameMode) -> Self {
        Self {
            entity_id: 1,
            hardcore: false,
            world_names: vec!["minecraft:overworld"],
            max_players: 32,
            view_distance: VIEW_DISTANCE,
            simulation_distance: SIMULATION_DISTANCE,
            reduced_debug: false,
            respawn_screen: true,
            limited_crafting: false,
            secure_chat: false,
            dimension_type: 0,
            dimension_name: "minecraft:overworld",
            hashed_seed: SEED_37_HASH,
            gamemode: mode.as_id(),
            previous_gamemode: 255,
            debug: false,
            flat: true,
            portal_cooldown: 0,
            sea_level: 63,
        }
    }

    pub fn build(&self) -> JoinGame {
        JoinGame::new(
            self.entity_id,
            self.hardcore,
            self.world_names.iter().map(|name| bs(name)).collect(),
            self.max_players,
            self.view_distance,
            self.simulation_distance,
            self.reduced_debug,
            self.respawn_screen,
            self.limited_crafting,
            SpawnInfo::new(
                self.dimension_type,
                bs(self.dimension_name),
                self.hashed_seed,
                self.gamemode,
                self.previous_gamemode,
                self.debug,
                self.flat,
                None,
                self.portal_cooldown,
                self.sea_level,
            ),
            self.secure_chat,
        )
    }
}

/// One admitted native session together with its genuine sim handle.
pub struct Admitted {
    pub handle: OwnedPlayerHandle,
    pub session: PlayerSessionHandle,
}

/// Reserve → genuine restore admission → `admit_native_join`.
pub async fn join(
    router: &mut SessionRouter,
    env: &mut SimEnv,
    lane: &NativeShardReceiver,
    name: &str,
    position: Vec3,
) -> Admitted {
    let reservation = router
        .reserve_native_join(player(name), generation(), position)
        .unwrap();
    let handle = env.restore(player(name), position).await;
    let session = admit(router, env, lane, &reservation, &handle).unwrap();
    Admitted { handle, session }
}

pub fn admit(
    router: &mut SessionRouter,
    env: &SimEnv,
    lane: &NativeShardReceiver,
    reservation: &NativeJoinReservation,
    handle: &OwnedPlayerHandle,
) -> Result<PlayerSessionHandle, ferrumc_session::CollectorSessionError> {
    let view = env.shard.connected_owned_player_view(handle).unwrap();
    let join = JoinFields::valid(view.game_mode()).build();
    router.admit_native_join(reservation, lane, handle, &view, join, "native", Vec::new())
}

/// Driver-like pump: drains the lane into the real simulation queue and runs
/// one checked normal boundary.
pub fn pump(env: &mut SimEnv, lane: &mut NativeShardReceiver) -> PlayerOwnershipTickOutcome {
    while let Ok(input) = lane.try_recv() {
        let (_key, _class, input) = input.into_parts();
        env.shard.enqueue_owned_player_input(input).unwrap();
    }
    env.boundary()
}
