//! Independent public consumer checks using actual ownership backends.
use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ferrumc_core::{DimensionId, EntityId, EntityUuid, GameMode, PlayerId, WorldId};
use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
use ferrumc_math::{ChunkPos, ShardPos, Vec3};
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_sim::*;
use ferrumc_storage::*;
use ferrumc_world::{FlatWorldGenerator, GenerationIdentity, WorldSeed};

trait RootBackend: WorldStore + PlayerEntityOwnershipStore {}
impl<T: WorldStore + PlayerEntityOwnershipStore> RootBackend for T {}
static PATH_SEQUENCE: AtomicUsize = AtomicUsize::new(0);
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn transfer(n: u8) -> OwnershipTransferId {
    OwnershipTransferId::from_bytes([n; 16])
}
fn uuid(n: u8) -> EntityUuid {
    EntityUuid::from_bytes([n; 16])
}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(19), DimensionId::new(11))
}
fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(-19))
}
fn position() -> Vec3 {
    Vec3::new(3.0, 64.0, 4.0)
}
fn empty() -> Box<[ItemStack; 46]> {
    Box::new(std::array::from_fn(|_| ItemStack::empty()))
}
fn stone(count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}
fn counts(shard: &SimShard, player: &OwnedPlayerHandle) -> u32 {
    shard
        .owned_player_slots(player)
        .unwrap()
        .iter()
        .map(|s| u32::from(s.count()))
        .sum::<u32>()
        + u32::from(shard.owned_player_carried(player).unwrap().count())
}
fn root_config() -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            nz(MAX_OWNERSHIP_REQUEST_BYTES),
            NonZeroU64::new(10000).unwrap(),
            NonZeroU64::new(1000).unwrap(),
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
struct RootEnv {
    shard: SimShard,
    store: Arc<dyn PlayerEntityOwnershipStore>,
    binding: OwnershipStoreBinding,
    _backend: Arc<dyn RootBackend>,
    path: Option<PathBuf>,
}
impl Drop for RootEnv {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}
impl RootEnv {
    async fn new(redb: bool) -> Self {
        let config = root_config();
        let (backend, path): (Arc<dyn RootBackend>, Option<PathBuf>) = if redb {
            let path = std::env::temp_dir().join(format!(
                "ferrumc-root-dur04-{}-{}.redb",
                std::process::id(),
                PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            (
                Arc::new(RedbStore::open_with_player_entity_ownership(&path, config).unwrap()),
                Some(path),
            )
        } else {
            (
                Arc::new(InMemoryStore::with_player_entity_ownership(config).unwrap()),
                None,
            )
        };
        backend
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let store: Arc<dyn PlayerEntityOwnershipStore> = backend.clone();
        let binding =
            OwnershipStoreBinding::new(&store, GenerationBinding::new(scope(), identity()));
        let mut shard = SimShard::with_generation(ShardPos::new(0, 0), binding.generation());
        shard
            .initialize_entities_with_item_budget(nz(8), nz(1024 * 1024))
            .unwrap();
        shard
            .loaded_chunks_mut()
            .acquire(
                backend.as_ref(),
                &FlatWorldGenerator::with_seed(WorldSeed::new(-19)),
                ChunkPos::ORIGIN,
                ChunkTicket::of(TicketReason::Spawn),
            )
            .await
            .unwrap();
        shard
            .initialize_player_ownership(
                binding.clone(),
                PlayerOwnershipLimits::new(
                    nz(4),
                    nz(MAX_OWNED_PLAYER_BYTES),
                    nz(4),
                    nz(MAX_PLAYER_OWNERSHIP_PENDING_BYTES),
                    nz(16),
                    nz(MAX_PLAYER_OWNERSHIP_INPUT_BYTES),
                )
                .unwrap(),
            )
            .unwrap();
        Self {
            shard,
            store,
            binding,
            _backend: backend,
            path,
        }
    }
    fn tick(&mut self, input: OwnedPlayerInput) -> PlayerOwnershipTickOutcome {
        self.shard.enqueue_owned_player_input(input).unwrap();
        self.shard.run_tick_with_player_ownership().unwrap()
    }
    async fn execute(&self, mut out: PlayerOwnershipTickOutcome) -> PlayerOwnershipCompletion {
        assert!(out.take_refusals().is_empty());
        let mut work = out.take_work();
        assert_eq!(work.len(), 1);
        work.pop().unwrap().execute(&self.store).await.unwrap()
    }
    async fn finish(&mut self, out: PlayerOwnershipTickOutcome) -> PlayerOwnershipTickOutcome {
        let completion = self.execute(out).await;
        let mut done = self.tick(OwnedPlayerInput::complete(completion));
        assert!(done.take_refusals().is_empty());
        done
    }
    async fn player(
        &mut self,
        name: &str,
        slots: Box<[ItemStack; 46]>,
        seed_id: u8,
    ) -> OwnedPlayerHandle {
        let player = PlayerId::offline(name);
        let before = self
            .store
            .load_owned_player(ScopedPlayerKey::new(scope(), player), identity())
            .await
            .unwrap();
        let record = NativePlayerSnapshot::new(
            position(),
            [0.0, 0.0],
            GameMode::Survival,
            0,
            slots,
            ItemStack::empty(),
        )
        .try_to_record()
        .unwrap();
        self.store
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    transfer(seed_id),
                    before.expectation().clone(),
                    PlayerOwnershipProposal::put(record),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let read = self.binding.read_player(&self.store, player).await.unwrap();
        let mut out = self.tick(OwnedPlayerInput::restore(read));
        assert!(out.take_refusals().is_empty());
        out.intents()
            .iter()
            .find_map(|i| match i {
                OwnershipIntent::PlayerAdmitted { player, .. } => Some(player.clone()),
                _ => None,
            })
            .unwrap()
    }
    async fn pair(&self, player: &OwnedPlayerHandle, entity: EntityUuid) -> OwnedPairRead {
        self.binding
            .read_pair(&self.store, player.player(), entity)
            .await
            .unwrap()
    }
    async fn spawn(&mut self, entity: EntityUuid, count: u8) -> EntityHandle {
        let base = EntitySpawn::new(
            scope().world(),
            scope().dimension(),
            entity,
            EntityId::new(523),
            EntityTypeId::from_name("minecraft:item").unwrap(),
            EntityTransform::try_new(position(), 0.0, 0.0).unwrap(),
        );
        let properties = ItemEntityProperties::try_new(
            Vec3::new(0.0, 0.0, 0.0),
            0.0,
            ItemEntityCounters::new(0, 0, 5),
            None,
            None,
        )
        .unwrap();
        let read = self.binding.read_entity(&self.store, entity).await.unwrap();
        let out = self.tick(OwnedPlayerInput::spawn_item(
            read,
            ItemEntitySpawn::new(base, stone(count), properties),
            transfer(160),
        ));
        self.finish(out).await;
        self.shard
            .entities()
            .unwrap()
            .handle_for_uuid(entity)
            .unwrap()
    }
}

#[tokio::test]
async fn absent_receipt_never_publishes_or_releases_a_pending_bootstrap() {
    for redb in [false, true] {
        let mut env = RootEnv::new(redb).await;
        let player = PlayerId::offline("RootUnproven");
        let read = env.binding.read_player(&env.store, player).await.unwrap();
        let mut out = env.tick(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [0.0, 0.0],
            transfer(1),
        ));
        assert_eq!(env.shard.player_count(), 0);
        let mut work = out.take_work();
        assert_eq!(work.len(), 1);
        let work = work.pop().unwrap();
        let query = work.query();
        let completion = work
            .into_receipt_clarification()
            .execute(&env.store)
            .await
            .unwrap();
        let mut out = env.tick(OwnedPlayerInput::complete(completion));
        let refusals = out.take_refusals();
        assert_eq!(refusals.len(), 1);
        assert!(matches!(
            refusals[0].reason(),
            PlayerOwnershipError::UnprovenWorkerLifetime
        ));
        assert_eq!(
            env.shard
                .player_ownership_checkpoint(query)
                .unwrap()
                .status(),
            OwnershipPendingStatus::Uncertain
        );
        assert_eq!(env.shard.player_ownership_pending_count(), 1);
        assert_eq!(env.shard.player_count(), 0);
        assert!(env
            .store
            .load_owned_player(ScopedPlayerKey::new(scope(), player), identity())
            .await
            .unwrap()
            .record()
            .is_none());
        drop(refusals);
        let mut retry = env.tick(OwnedPlayerInput::retry(query));
        assert!(retry.take_work().is_empty());
        assert!(matches!(
            retry.take_refusals()[0].reason(),
            PlayerOwnershipError::UnprovenWorkerLifetime
        ));
        env.tick(OwnedPlayerInput::drain());
        let mut stop = env.tick(OwnedPlayerInput::stop());
        assert!(matches!(
            stop.take_refusals()[0].reason(),
            PlayerOwnershipError::NotQuiescent
        ));
        assert_eq!(env.shard.player_ownership_pending_count(), 1);
    }
}

#[tokio::test]
async fn deferred_selection_and_movement_enter_the_next_conserving_drop() {
    for redb in [false, true] {
        let mut env = RootEnv::new(redb).await;
        let mut slots = empty();
        slots[36] = stone(9);
        slots[37] = stone(5);
        let player = env.player("RootDeferred", slots, 140).await;
        let read = env.pair(&player, uuid(11)).await;
        let first = env.tick(OwnedPlayerInput::drop_selected(
            player.clone(),
            read,
            DropSelection::One,
            EntityId::new(611),
            transfer(11),
            123,
            false,
        ));
        assert_eq!(counts(&env.shard, &player), 14);
        let mut selection = env.tick(OwnedPlayerInput::select_hotbar(player.clone(), 1));
        assert!(selection.take_refusals().is_empty());
        let moved = Vec3::new(6.0, 64.0, 7.0);
        let mut movement = env.tick(OwnedPlayerInput::move_player(
            player.clone(),
            Some(moved),
            [Some(17.0), Some(-23.0)],
        ));
        assert!(movement.take_refusals().is_empty());
        env.finish(first).await;
        assert_eq!(counts(&env.shard, &player), 13);
        let read = env.pair(&player, uuid(12)).await;
        let second = env.tick(OwnedPlayerInput::drop_selected(
            player.clone(),
            read,
            DropSelection::One,
            EntityId::new(612),
            transfer(12),
            456,
            false,
        ));
        env.finish(second).await;
        let slots = env.shard.owned_player_slots(&player).unwrap();
        assert_eq!(slots[36].count(), 8);
        assert_eq!(slots[37].count(), 4);
        let native = env
            .store
            .load_owned_player(ScopedPlayerKey::new(scope(), player.player()), identity())
            .await
            .unwrap();
        let native = NativePlayerSnapshot::try_from_record(native.record().unwrap()).unwrap();
        assert_eq!(native.selected(), 1);
        assert_eq!(native.position(), moved);
        assert_eq!(
            native.look().map(f32::to_bits),
            [17.0_f32.to_bits(), (-23.0_f32).to_bits()]
        );
        let entities = env.shard.entities().unwrap();
        let total = counts(&env.shard, &player)
            + [uuid(11), uuid(12)]
                .into_iter()
                .map(|id| {
                    u32::from(
                        entities
                            .item_state(&entities.handle_for_uuid(id).unwrap())
                            .unwrap()
                            .stack()
                            .count(),
                    )
                })
                .sum::<u32>();
        assert_eq!(total, 14);
    }
}

#[tokio::test]
async fn two_competing_pickups_require_a_fresh_pair_and_conserve_all_items() {
    for redb in [false, true] {
        let mut env = RootEnv::new(redb).await;
        let mut slots = empty();
        for slot in &mut slots[9..46] {
            *slot = stone(64);
        }
        slots[9] = stone(62);
        let first = env.player("RootCollectorOne", slots, 141).await;
        let second = env.player("RootCollectorTwo", empty(), 142).await;
        let baseline = counts(&env.shard, &first) + counts(&env.shard, &second);
        let entity = uuid(22);
        let item = env.spawn(entity, 6).await;
        let a = env.pair(&first, entity).await;
        let b = env.pair(&second, entity).await;
        env.shard
            .enqueue_owned_player_input(OwnedPlayerInput::pickup(
                first.clone(),
                item.clone(),
                a,
                transfer(21),
                true,
                false,
            ))
            .unwrap();
        env.shard
            .enqueue_owned_player_input(OwnedPlayerInput::pickup(
                second.clone(),
                item.clone(),
                b,
                transfer(22),
                true,
                false,
            ))
            .unwrap();
        let mut out = env.shard.run_tick_with_player_ownership().unwrap();
        let refusals = out.take_refusals();
        assert_eq!(refusals.len(), 1);
        assert!(matches!(
            refusals[0].reason(),
            PlayerOwnershipError::Reserved
        ));
        assert_eq!(
            counts(&env.shard, &first) + counts(&env.shard, &second),
            baseline
        );
        env.finish(out).await;
        assert_eq!(counts(&env.shard, &first), baseline + 2);
        assert_eq!(
            env.shard
                .entities()
                .unwrap()
                .item_state(&item)
                .unwrap()
                .stack()
                .count(),
            4
        );
        let fresh = env.pair(&second, entity).await;
        let out = env.tick(OwnedPlayerInput::pickup(
            second.clone(),
            item.clone(),
            fresh,
            transfer(23),
            true,
            false,
        ));
        env.finish(out).await;
        assert_eq!(
            counts(&env.shard, &first) + counts(&env.shard, &second),
            baseline + 6
        );
        assert!(env
            .shard
            .entities()
            .unwrap()
            .handle_for_uuid(entity)
            .is_none());
        assert!(env
            .store
            .load_owned_entity(
                PersistentEntityKey::new(scope().world(), scope().dimension(), entity),
                identity()
            )
            .await
            .unwrap()
            .record()
            .is_none());
        assert_eq!(env.shard.player_ownership_pending_count(), 0);
    }
}
