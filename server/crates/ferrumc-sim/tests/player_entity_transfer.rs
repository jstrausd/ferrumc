//! Actual native authority with both real ownership producers and an independent
//! count model. Storage bytes are seeded before the normal readonly admission.

use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ferrumc_core::{DimensionId, EntityId, EntityUuid, GameMode, PlayerId, WorldId};
use ferrumc_items::{
    ComponentPatch, ComponentTypeId, ComponentValue, ItemId, ItemStack, OpaqueComponent,
};
use ferrumc_math::{ChunkPos, ShardPos, Vec3};
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_sim::*;
use ferrumc_storage::*;
use ferrumc_world::{FlatWorldGenerator, GenerationIdentity, WorldSeed};

mod collector_lifecycle;

trait Backend: PlayerEntityOwnershipStore + WorldStore {}
impl<T: PlayerEntityOwnershipStore + WorldStore> Backend for T {}
static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn id(n: u8) -> OwnershipTransferId {
    OwnershipTransferId::from_bytes([n; 16])
}
fn uuid(n: u8) -> EntityUuid {
    EntityUuid::from_bytes([n; 16])
}
fn player() -> PlayerId {
    PlayerId::offline("OwnedSurvival")
}
fn position() -> Vec3 {
    Vec3::new(1.0, 64.0, 1.0)
}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(7), DimensionId::new(13))
}
fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(37))
}
fn config() -> OwnershipConfig {
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
fn limits() -> PlayerOwnershipLimits {
    PlayerOwnershipLimits::new(
        nz(3),
        nz(MAX_OWNED_PLAYER_BYTES),
        nz(4),
        nz(MAX_PLAYER_OWNERSHIP_PENDING_BYTES),
        nz(16),
        nz(MAX_PLAYER_OWNERSHIP_INPUT_BYTES),
    )
    .unwrap()
}
fn stone(count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}
fn filled(count: u8) -> Box<[ItemStack; 46]> {
    Box::new(std::array::from_fn(|_| stone(count)))
}
fn empty() -> Box<[ItemStack; 46]> {
    Box::new(std::array::from_fn(|_| ItemStack::empty()))
}

struct Env {
    shard: SimShard,
    backend: Arc<dyn Backend>,
    store: Arc<dyn PlayerEntityOwnershipStore>,
    binding: OwnershipStoreBinding,
    path: Option<PathBuf>,
}
impl Drop for Env {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}
impl Env {
    async fn new(redb: bool) -> Self {
        Self::with_limits(redb, limits()).await
    }
    async fn with_limits(redb: bool, owner_limits: PlayerOwnershipLimits) -> Self {
        let (backend, path): (Arc<dyn Backend>, Option<PathBuf>) = if redb {
            let path = std::env::temp_dir().join(format!(
                "ferrumc-dur04-{}-{}.redb",
                std::process::id(),
                NEXT_FILE.fetch_add(1, Ordering::SeqCst)
            ));
            (
                Arc::new(RedbStore::open_with_player_entity_ownership(&path, config()).unwrap()),
                Some(path),
            )
        } else {
            (
                Arc::new(InMemoryStore::with_player_entity_ownership(config()).unwrap()),
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
                &FlatWorldGenerator::with_seed(WorldSeed::new(37)),
                ChunkPos::new(0, 0),
                ChunkTicket::of(TicketReason::Spawn),
            )
            .await
            .unwrap();
        shard
            .initialize_player_ownership(binding.clone(), owner_limits)
            .unwrap();
        Self {
            shard,
            backend,
            store,
            binding,
            path,
        }
    }
    async fn bind_override(&mut self, store: Arc<dyn PlayerEntityOwnershipStore>) {
        assert_eq!(self.shard.player_count(), 0);
        let binding =
            OwnershipStoreBinding::new(&store, GenerationBinding::new(scope(), identity()));
        let mut shard = SimShard::with_generation(ShardPos::new(0, 0), binding.generation());
        shard
            .initialize_entities_with_item_budget(nz(8), nz(1024 * 1024))
            .unwrap();
        shard
            .loaded_chunks_mut()
            .acquire(
                self.backend.as_ref(),
                &FlatWorldGenerator::with_seed(WorldSeed::new(37)),
                ChunkPos::new(0, 0),
                ChunkTicket::of(TicketReason::Spawn),
            )
            .await
            .unwrap();
        shard
            .initialize_player_ownership(binding.clone(), limits())
            .unwrap();
        self.shard = shard;
        self.binding = binding;
        self.store = store;
    }
    fn tick(&mut self, input: OwnedPlayerInput) -> PlayerOwnershipTickOutcome {
        self.shard.enqueue_owned_player_input(input).unwrap();
        self.shard.run_tick_with_player_ownership().unwrap()
    }
    async fn finish(&mut self, mut out: PlayerOwnershipTickOutcome) -> PlayerOwnershipTickOutcome {
        let work = out.take_work();
        assert_eq!(work.len(), 1);
        let completion = work
            .into_iter()
            .next()
            .unwrap()
            .execute(&self.store)
            .await
            .unwrap();
        self.tick(OwnedPlayerInput::complete(completion))
    }
    async fn seed_player(
        &mut self,
        slots: Box<[ItemStack; 46]>,
        selected: u8,
    ) -> OwnedPlayerHandle {
        let key = ScopedPlayerKey::new(scope(), player());
        let before = self.store.load_owned_player(key, identity()).await.unwrap();
        let snapshot = NativePlayerSnapshot::new(
            position(),
            [-0.0, 0.0],
            GameMode::Survival,
            selected,
            slots,
            ItemStack::empty(),
        );
        self.store
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    id(200),
                    before.expectation().clone(),
                    PlayerOwnershipProposal::put(snapshot.try_to_record().unwrap()),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let read = self
            .binding
            .read_player(&self.store, player())
            .await
            .unwrap();
        let mut out = self.tick(OwnedPlayerInput::restore(read));
        assert!(out.take_refusals().is_empty());
        out.intents()
            .iter()
            .find_map(|i| {
                if let OwnershipIntent::PlayerAdmitted { player, .. } = i {
                    Some(player.clone())
                } else {
                    None
                }
            })
            .unwrap()
    }
    async fn spawn(&mut self, count: u8, delay: i32) -> EntityHandle {
        let proposal = ItemEntitySpawn::new(
            EntitySpawn::new(
                scope().world(),
                scope().dimension(),
                uuid(1),
                EntityId::new(71),
                EntityTypeId::from_name("item").unwrap(),
                EntityTransform::try_new(position(), 0.0, 0.0).unwrap(),
            ),
            stone(count),
            ItemEntityProperties::try_new(
                Vec3::new(-0.0, 0.0, 0.0),
                -0.0,
                ItemEntityCounters::new(0, delay, 5),
                None,
                Some(uuid(9)),
            )
            .unwrap(),
        );
        let read = self
            .binding
            .read_entity(&self.store, uuid(1))
            .await
            .unwrap();
        let out = self.tick(OwnedPlayerInput::spawn_item(read, proposal, id(201)));
        assert!(self.shard.entities().unwrap().is_empty());
        let mut out = self.finish(out).await;
        assert!(out.take_refusals().is_empty());
        self.shard
            .entities()
            .unwrap()
            .handle_for_uuid(uuid(1))
            .unwrap()
    }
    async fn pair(&self, entity: EntityUuid) -> OwnedPairRead {
        self.binding
            .read_pair(&self.store, player(), entity)
            .await
            .unwrap()
    }
    fn count(&self, handle: &OwnedPlayerHandle, slot: usize) -> u8 {
        self.shard.owned_player_slots(handle).unwrap()[slot].count()
    }
}

async fn drop_contract(redb: bool) {
    let mut env = Env::new(redb).await;
    let mut slots = empty();
    slots[38] = stone(9);
    let handle = env.seed_player(slots, 2).await;
    for (operation, selection, expected, moved) in
        [(1, DropSelection::One, 8, 1), (2, DropSelection::All, 0, 8)]
    {
        let entity = uuid(operation + 10);
        let read = env.pair(entity).await;
        let out = env.tick(OwnedPlayerInput::drop_selected(
            handle.clone(),
            read,
            selection,
            EntityId::new(i32::from(operation)),
            id(operation),
            123,
            false,
        ));
        assert_eq!(env.count(&handle, 38), expected + moved);
        assert!(env
            .shard
            .entities()
            .unwrap()
            .handle_for_uuid(entity)
            .is_none());
        let out = env.finish(out).await;
        assert!(!out.intents().is_empty());
        assert_eq!(env.count(&handle, 38), expected);
        let item = env
            .shard
            .entities()
            .unwrap()
            .handle_for_uuid(entity)
            .unwrap();
        let state = env.shard.entities().unwrap().item_state(&item).unwrap();
        assert_eq!(state.stack().count(), moved);
        assert_eq!(state.properties().counters().pickup_delay(), 40);
        assert_eq!(state.properties().counters().age(), 0);
        assert_eq!(state.properties().counters().health(), 5);
        assert_eq!(state.properties().owner_target(), None);
        assert_eq!(
            state.properties().thrower(),
            Some(EntityUuid::from_bytes(*player().as_uuid().as_bytes()))
        );
    }
}
#[tokio::test]
async fn memory_drop_one_and_all_publish_only_after_real_commit() {
    drop_contract(false).await;
}
#[tokio::test]
async fn redb_drop_one_and_all_publish_only_after_real_commit() {
    drop_contract(true).await;
}

async fn pickup_contract(redb: bool, partial: bool) {
    let mut env = Env::new(redb).await;
    let mut slots = if partial { filled(64) } else { empty() };
    slots[39] = stone(if partial { 63 } else { 60 });
    let handle = env.seed_player(slots, 3).await;
    let item = env.spawn(5, 0).await;
    let read = env.pair(uuid(1)).await;
    let out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item.clone(),
        read,
        id(1),
        true,
        false,
    ));
    assert_eq!(env.count(&handle, 39), if partial { 63 } else { 60 });
    assert_eq!(
        env.shard
            .entities()
            .unwrap()
            .item_state(&item)
            .unwrap()
            .stack()
            .count(),
        5
    );
    let out = env.finish(out).await;
    assert_eq!(env.count(&handle, 39), 64);
    assert!(out.intents().iter().any(|i| matches!(i,OwnershipIntent::Take { count:5,moved,.. } if *moved == if partial {1} else {5})));
    if partial {
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
        assert!(out
            .intents()
            .iter()
            .any(|i| matches!(i, OwnershipIntent::ItemChanged { .. })));
    } else {
        assert_eq!(env.count(&handle, 36), 1);
        assert!(env.shard.entities().unwrap().get(&item).is_err());
        assert!(env
            .store
            .load_owned_entity(
                PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(1)),
                identity()
            )
            .await
            .unwrap()
            .stamp()
            .is_tombstone());
    }
}
#[tokio::test]
async fn memory_partial_pickup_conserves_and_reports_original_paper_count() {
    pickup_contract(false, true).await;
}
#[tokio::test]
async fn redb_partial_pickup_conserves_and_reports_original_paper_count() {
    pickup_contract(true, true).await;
}
#[tokio::test]
async fn memory_full_pickup_removes_only_after_real_tombstone() {
    pickup_contract(false, false).await;
}
#[tokio::test]
async fn redb_full_pickup_removes_only_after_real_tombstone() {
    pickup_contract(true, false).await;
}

#[tokio::test]
async fn selected_hotbar_occupied_offhand_then_hotbar_then_main() {
    let mut env = Env::new(false).await;
    let mut slots = empty();
    slots[37] = stone(63);
    slots[45] = stone(62);
    slots[36] = stone(63);
    slots[9] = stone(61);
    let handle = env.seed_player(slots, 1).await;
    let item = env.spawn(8, 0).await;
    let read = env.pair(uuid(1)).await;
    let out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item,
        read,
        id(1),
        false,
        false,
    ));
    env.finish(out).await;
    for slot in [37, 45, 36, 9] {
        assert_eq!(env.count(&handle, slot), 64);
    }
    assert_eq!(env.count(&handle, 38), 1);
    assert_eq!(env.count(&handle, 0), 0);
    assert_eq!(env.count(&handle, 1), 0);
}

#[tokio::test]
async fn empty_offhand_and_craft_armor_recipe_cursor_are_not_pickup_destinations() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let read = env.pair(uuid(1)).await;
    let out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item,
        read,
        id(1),
        false,
        false,
    ));
    env.finish(out).await;
    assert_eq!(env.count(&handle, 36), 5);
    for slot in (0..9).chain(std::iter::once(45)) {
        assert_eq!(env.count(&handle, slot), 0);
    }
    assert_eq!(env.shard.owned_player_carried(&handle).unwrap().count(), 0);
}

#[tokio::test]
async fn full_inventory_delay_and_cancel_preserve_actual_beforeimages() {
    for (full, delay, cancel) in [(true, 0, false), (false, 40, false), (false, 0, true)] {
        let mut env = Env::new(false).await;
        let handle = env
            .seed_player(if full { filled(64) } else { empty() }, 0)
            .await;
        let item = env.spawn(5, delay).await;
        let read = env.pair(uuid(1)).await;
        let mut out = env.tick(OwnedPlayerInput::pickup(
            handle.clone(),
            item.clone(),
            read,
            id(1),
            true,
            cancel,
        ));
        assert!(out.take_work().is_empty());
        if !cancel {
            assert_eq!(out.take_refusals().len(), 1);
        }
        assert_eq!(
            env.shard
                .entities()
                .unwrap()
                .item_state(&item)
                .unwrap()
                .stack()
                .count(),
            5
        );
        assert_eq!(env.count(&handle, 36), if full { 64 } else { 0 });
    }
}

#[tokio::test]
async fn selected_drop_cancel_and_spectator_preserve_stack() {
    let mut env = Env::new(false).await;
    let mut slots = empty();
    slots[36] = stone(7);
    let handle = env.seed_player(slots, 0).await;
    let read = env.pair(uuid(10)).await;
    let mut out = env.tick(OwnedPlayerInput::drop_selected(
        handle.clone(),
        read,
        DropSelection::All,
        EntityId::new(1),
        id(1),
        4,
        true,
    ));
    assert!(out.take_work().is_empty());
    assert_eq!(env.count(&handle, 36), 7);
    env.tick(OwnedPlayerInput::set_game_mode(
        handle.clone(),
        GameMode::Spectator,
    ));
    let read = env.pair(uuid(10)).await;
    let mut out = env.tick(OwnedPlayerInput::drop_selected(
        handle.clone(),
        read,
        DropSelection::All,
        EntityId::new(1),
        id(2),
        4,
        false,
    ));
    assert_eq!(out.take_refusals().len(), 1);
    assert_eq!(env.count(&handle, 36), 7);
}

#[tokio::test]
async fn one_resource_reservation_rejects_second_pickup_and_parallel_player_autosave() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let read1 = env.pair(uuid(1)).await;
    let read2 = env.pair(uuid(1)).await;
    env.shard
        .enqueue_owned_player_input(OwnedPlayerInput::pickup(
            handle.clone(),
            item.clone(),
            read1,
            id(1),
            true,
            false,
        ))
        .unwrap();
    env.shard
        .enqueue_owned_player_input(OwnedPlayerInput::pickup(
            handle.clone(),
            item,
            read2,
            id(2),
            true,
            false,
        ))
        .unwrap();
    env.shard
        .enqueue_owned_player_input(OwnedPlayerInput::autosave(handle.clone(), id(3)))
        .unwrap();
    let mut out = env.shard.run_tick_with_player_ownership().unwrap();
    assert_eq!(out.take_refusals().len(), 2);
    env.finish(out).await;
    assert_eq!(env.count(&handle, 36), 5);
    assert_eq!(env.shard.player_ownership_pending_count(), 0);
}

#[tokio::test]
async fn movement_mode_and_selection_defer_until_matching_completion_then_new_generation() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let old = env.shard.owned_player_generation(&handle).unwrap();
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item,
        read,
        id(1),
        true,
        false,
    ));
    env.tick(OwnedPlayerInput::move_player(
        handle.clone(),
        Some(Vec3::new(2.0, 64.0, 1.0)),
        [Some(12.0), None],
    ));
    env.tick(OwnedPlayerInput::set_game_mode(
        handle.clone(),
        GameMode::Adventure,
    ));
    env.tick(OwnedPlayerInput::select_hotbar(handle.clone(), 7));
    assert_eq!(env.shard.player_position(player()), Some(position()));
    assert_eq!(
        env.shard.player_game_mode(player()),
        Some(GameMode::Survival)
    );
    assert_eq!(env.shard.owned_player_generation(&handle).unwrap(), old);
    let completion = out
        .take_work()
        .pop()
        .unwrap()
        .execute(&env.store)
        .await
        .unwrap();
    let out = env.tick(OwnedPlayerInput::complete(completion));
    assert!(!out.intents().is_empty());
    assert_eq!(
        env.shard.player_position(player()),
        Some(Vec3::new(2.0, 64.0, 1.0))
    );
    assert_eq!(
        env.shard.player_game_mode(player()),
        Some(GameMode::Adventure)
    );
    assert!(env.shard.owned_player_generation(&handle).unwrap().get() > old.get());
    let record = env
        .store
        .load_owned_player(ScopedPlayerKey::new(scope(), player()), identity())
        .await
        .unwrap();
    assert_eq!(
        NativePlayerSnapshot::try_from_record(record.record().unwrap())
            .unwrap()
            .position(),
        position()
    );
    let out = env.tick(OwnedPlayerInput::autosave(handle.clone(), id(2)));
    env.finish(out).await;
    let record = env
        .store
        .load_owned_player(ScopedPlayerKey::new(scope(), player()), identity())
        .await
        .unwrap();
    let snapshot = NativePlayerSnapshot::try_from_record(record.record().unwrap()).unwrap();
    assert_eq!(snapshot.selected(), 7);
    assert_eq!(snapshot.look()[0], 12.0);
}

#[tokio::test]
async fn disconnect_retains_pending_rejoin_changes_handle_and_dirty_stop_refuses() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let read = env.pair(uuid(1)).await;
    let out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item,
        read,
        id(1),
        true,
        false,
    ));
    env.tick(OwnedPlayerInput::disconnect(handle.clone()));
    let read = env.binding.read_player(&env.store, player()).await.unwrap();
    let mut refused = env.tick(OwnedPlayerInput::rejoin(read));
    assert!(matches!(
        refused.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::Reserved
    ));
    env.finish(out).await;
    assert_eq!(env.count(&handle, 36), 5);
    let read = env.binding.read_player(&env.store, player()).await.unwrap();
    let out = env.tick(OwnedPlayerInput::rejoin(read));
    let new = out
        .intents()
        .iter()
        .find_map(|i| {
            if let OwnershipIntent::PlayerAdmitted { player, .. } = i {
                Some(player.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert!(matches!(
        env.shard.owned_player_slots(&handle),
        Err(PlayerOwnershipError::StaleHandle)
    ));
    env.tick(OwnedPlayerInput::select_hotbar(new.clone(), 1));
    let mut out = env.tick(OwnedPlayerInput::stop());
    assert_eq!(out.take_refusals().len(), 1);
    env.tick(OwnedPlayerInput::drain());
    let out = env.tick(OwnedPlayerInput::autosave(new, id(2)));
    env.finish(out).await;
    let out = env.tick(OwnedPlayerInput::stop());
    assert!(out
        .intents()
        .iter()
        .any(|i| matches!(i, OwnershipIntent::Stopped)));
}

#[tokio::test]
async fn bootstrap_uses_actual_fresh_miss_and_has_no_creative_starter_items() {
    for redb in [false, true] {
        let mut env = Env::new(redb).await;
        let read = env.binding.read_player(&env.store, player()).await.unwrap();
        let out = env.tick(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [-0.0, 0.0],
            id(1),
        ));
        assert_eq!(env.shard.player_count(), 0);
        let out = env.finish(out).await;
        let handle = out
            .intents()
            .iter()
            .find_map(|i| {
                if let OwnershipIntent::PlayerAdmitted { player, .. } = i {
                    Some(player.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            env.shard.player_game_mode(player()),
            Some(GameMode::Survival)
        );
        assert!(env
            .shard
            .owned_player_slots(&handle)
            .unwrap()
            .iter()
            .all(|s| s.item().is_none()));
        let out = env.tick(OwnedPlayerInput::autosave(handle, id(2)));
        let mut out = env.finish(out).await;
        assert!(out.take_refusals().is_empty());
    }
}

#[tokio::test]
async fn normal_slot_left_right_click_conserves_cursor_and_refuses_recipe_armor_and_stale_generation(
) {
    let mut env = Env::new(false).await;
    let mut slots = empty();
    slots[9] = stone(7);
    let handle = env.seed_player(slots, 0).await;
    let old = env.shard.owned_player_generation(&handle).unwrap();
    let out = env.tick(OwnedPlayerInput::normal_slot_click(
        handle.clone(),
        old,
        9,
        NormalSlotButton::Right,
        id(1),
    ));
    env.finish(out).await;
    assert_eq!(env.count(&handle, 9), 3);
    assert_eq!(env.shard.owned_player_carried(&handle).unwrap().count(), 4);
    let mut out = env.tick(OwnedPlayerInput::normal_slot_click(
        handle.clone(),
        old,
        10,
        NormalSlotButton::Left,
        id(2),
    ));
    assert_eq!(out.take_refusals().len(), 1);
    for slot in [0, 5, 8, 46] {
        let generation = env.shard.owned_player_generation(&handle).unwrap();
        let mut out = env.tick(OwnedPlayerInput::normal_slot_click(
            handle.clone(),
            generation,
            slot,
            NormalSlotButton::Left,
            id(3),
        ));
        assert_eq!(out.take_refusals().len(), 1);
    }
    let generation = env.shard.owned_player_generation(&handle).unwrap();
    let out = env.tick(OwnedPlayerInput::normal_slot_click(
        handle.clone(),
        generation,
        10,
        NormalSlotButton::Right,
        id(4),
    ));
    env.finish(out).await;
    assert_eq!(env.count(&handle, 10), 1);
    assert_eq!(env.shard.owned_player_carried(&handle).unwrap().count(), 3);
}

#[tokio::test]
async fn checkpoint_absence_never_releases_and_positive_resolution_is_fresh_readonly_admission() {
    let mut env = Env::new(false).await;
    let mut slots = empty();
    slots[36] = stone(3);
    let handle = env.seed_player(slots, 0).await;
    let read = env.pair(uuid(2)).await;
    let mut out = env.tick(OwnedPlayerInput::drop_selected(
        handle.clone(),
        read,
        DropSelection::One,
        EntityId::new(3),
        id(1),
        4,
        false,
    ));
    let work = out.take_work().pop().unwrap();
    let query = work.query();
    let checkpoint = env.shard.player_ownership_checkpoint(query).unwrap();
    let bytes = checkpoint.try_to_bytes().unwrap();
    let checkpoint = OwnershipPendingCheckpoint::try_from_bytes(&bytes).unwrap();
    assert!(matches!(
        env.binding
            .clarify_checkpoint(&env.store, &checkpoint)
            .await,
        Err(PlayerOwnershipError::UnprovenWorkerLifetime)
    ));
    assert_eq!(env.shard.player_ownership_pending_count(), 1);
    assert_eq!(env.count(&handle, 36), 3);
    for at in 0..bytes.len() {
        assert!(
            OwnershipPendingCheckpoint::try_from_bytes(&bytes[..at]).is_err(),
            "truncation {at}"
        );
    }
    assert_checkpoint_malformed(&bytes);
    let completion = work.execute(&env.store).await.unwrap();
    let resolution = env
        .binding
        .clarify_checkpoint(&env.store, &checkpoint)
        .await
        .unwrap();
    let (player_read, entity_read) = resolution.into_reads();
    assert!(player_read.is_some());
    assert!(entity_read.is_some());
    env.tick(OwnedPlayerInput::complete(completion));
    assert_eq!(env.count(&handle, 36), 2);
    let out = env.tick(OwnedPlayerInput::autosave(handle, id(2)));
    env.finish(out).await;
    // An identical player-only save does not change the durable revision, so
    // the original receipt remains current and can still be clarified.
    assert!(env
        .binding
        .clarify_checkpoint(&env.store, &checkpoint)
        .await
        .is_ok());
}

#[tokio::test]
async fn cloned_owned_shard_and_every_legacy_mutation_entry_stay_inactive() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    assert!(matches!(
        env.shard.enqueue(GameInput::SetGameMode {
            player: player(),
            mode: GameMode::Creative
        }),
        Err(SimError::OwnershipInputRequired)
    ));
    let mut called = false;
    assert!(matches!(
        env.shard.run_tick_with_entity_work(|_| {
            called = true;
        }),
        Err(EntityStoreError::OwnershipRequired)
    ));
    assert!(!called);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        env.shard.loaded_chunks_mut();
    }))
    .is_err());
    let mut copy = env.shard.clone();
    assert!(matches!(
        copy.enqueue_owned_player_input(OwnedPlayerInput::select_hotbar(handle.clone(), 3))
            .unwrap_err()
            .reason(),
        PlayerOwnershipError::SnapshotOnly
    ));
    assert!(matches!(
        copy.initialize_player_ownership(env.binding.clone(), limits()),
        Err(PlayerOwnershipError::AlreadyInitialized)
    ));
    assert_eq!(copy.run_tick(), [] as [GameOutput; 0]);
    assert_eq!(
        env.shard.player_game_mode(player()),
        Some(GameMode::Survival)
    );
    let mut entity_copy = env.shard.entities().unwrap().clone();
    let spawn = EntitySpawn::new(
        scope().world(),
        scope().dimension(),
        uuid(30),
        EntityId::new(30),
        EntityTypeId::from_name("zombie").unwrap(),
        EntityTransform::try_new(position(), 0.0, 0.0).unwrap(),
    );
    assert!(matches!(
        entity_copy.spawn(spawn),
        Err(EntityStoreError::OwnershipRequired)
    ));
}

#[tokio::test]
async fn unloaded_pending_item_registration_survives_and_new_transfer_refuses() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let read = env.pair(uuid(1)).await;
    let out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item.clone(),
        read,
        id(1),
        true,
        false,
    ));
    env.shard
        .release_owned_chunk(ChunkPos::new(0, 0), ChunkTicket::of(TicketReason::Spawn))
        .unwrap();
    assert!(env.shard.entities().unwrap().get(&item).is_ok());
    assert_eq!(env.shard.player_ownership_pending_count(), 1);
    env.finish(out).await;
    assert_eq!(env.count(&handle, 36), 5);
    assert!(env.shard.entities().unwrap().is_empty());
}

#[tokio::test]
async fn supported_typed_capacity_is_conservative_and_preserves_patch() {
    let mut env = Env::new(false).await;
    let mut slots = empty();
    slots[36] = ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        2,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(2)], vec![]),
    )
    .unwrap();
    let handle = env.seed_player(slots, 0).await;
    let read = env.pair(uuid(2)).await;
    let out = env.tick(OwnedPlayerInput::drop_selected(
        handle.clone(),
        read,
        DropSelection::One,
        EntityId::new(3),
        id(1),
        4,
        false,
    ));
    env.finish(out).await;
    assert_eq!(env.count(&handle, 36), 1);
    let item = env
        .shard
        .entities()
        .unwrap()
        .handle_for_uuid(uuid(2))
        .unwrap();
    assert_eq!(
        env.shard
            .entities()
            .unwrap()
            .item_state(&item)
            .unwrap()
            .stack()
            .components()
            .added(),
        &[ComponentValue::MaxStackSize(2)]
    );
}

// The wrapper delegates successful writes/reads to an actual producer. Faults
// are injected only at the driver's reply/current-read boundary; no arbitrary
// receipt or Completion constructor is available to these consumers.
struct FaultStore {
    inner: Arc<dyn PlayerEntityOwnershipStore>,
    mode: AtomicU8,
    armed: AtomicBool,
}
impl FaultStore {
    fn new(inner: Arc<dyn PlayerEntityOwnershipStore>) -> Self {
        Self {
            inner,
            mode: AtomicU8::new(0),
            armed: AtomicBool::new(false),
        }
    }
    fn fault(&self, mode: u8) {
        self.mode.store(mode, Ordering::SeqCst);
        self.armed.store(true, Ordering::SeqCst);
    }
    fn active(&self) -> u8 {
        if self.armed.load(Ordering::SeqCst) {
            self.mode.load(Ordering::SeqCst)
        } else {
            0
        }
    }
    fn after(
        &self,
        query: OwnershipReceiptQuery,
        result: OwnershipResult<OwnershipCommitOutcome>,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        if self.active() == 2 {
            result?;
            Err(OwnershipError::CommitUncertain {
                query,
                cause: "lost postcommit reply".into(),
            })
        } else {
            result
        }
    }
}
#[async_trait::async_trait]
impl PlayerEntityOwnershipStore for FaultStore {
    async fn load_owned_player(
        &self,
        key: ScopedPlayerKey,
        identity: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerSnapshot> {
        self.inner.load_owned_player(key, identity).await
    }
    async fn load_owned_entity(
        &self,
        key: PersistentEntityKey,
        identity: GenerationIdentity,
    ) -> OwnershipResult<OwnedEntitySnapshot> {
        self.inner.load_owned_entity(key, identity).await
    }
    async fn load_owned_pair(
        &self,
        p: ScopedPlayerKey,
        e: PersistentEntityKey,
        identity: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerEntitySnapshot> {
        if self.active() == 1 {
            return Err(OwnershipError::Backend(
                "injected current pair read failure".into(),
            ));
        }
        self.inner.load_owned_pair(p, e, identity).await
    }
    async fn commit_player_entity_transfer(
        &self,
        request: PlayerEntityTransferRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        let query = request.receipt_query();
        if self.active() == 3 {
            return Err(OwnershipError::Busy);
        }
        if self.active() == 4 {
            return Err(OwnershipError::ExecutionUncertain {
                query,
                cause: "driver join reply unavailable".into(),
            });
        }
        self.after(
            query,
            self.inner.commit_player_entity_transfer(request).await,
        )
    }
    async fn compare_exchange_owned_player(
        &self,
        request: PlayerOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.after(
            request.receipt_query(),
            self.inner.compare_exchange_owned_player(request).await,
        )
    }
    async fn compare_exchange_owned_entity(
        &self,
        request: EntityOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.after(
            request.receipt_query(),
            self.inner.compare_exchange_owned_entity(request).await,
        )
    }
    async fn load_ownership_receipt(
        &self,
        query: OwnershipReceiptQuery,
    ) -> OwnershipResult<Option<OwnershipReceipt>> {
        if matches!(self.active(), 2 | 4) {
            return Err(OwnershipError::Backend(
                "injected receipt reply failure".into(),
            ));
        }
        self.inner.load_ownership_receipt(query).await
    }
}

async fn fault_env(redb: bool) -> (Env, Arc<FaultStore>, OwnedPlayerHandle, EntityHandle) {
    let mut env = Env::new(redb).await;
    let fault = Arc::new(FaultStore::new(env.store.clone()));
    env.bind_override(fault.clone()).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    (env, fault, handle, item)
}

#[tokio::test]
async fn confirmed_commit_with_current_read_failure_keeps_both_reservations_until_refresh() {
    for redb in [false, true] {
        let (mut env, fault, handle, item) = fault_env(redb).await;
        let read = env.pair(uuid(1)).await;
        let mut out = env.tick(OwnedPlayerInput::pickup(
            handle.clone(),
            item.clone(),
            read,
            id(1),
            true,
            false,
        ));
        let work = out.take_work().pop().unwrap();
        let query = work.query();
        fault.fault(1);
        let completion = work.execute(&env.store).await.unwrap();
        let mut out = env.tick(OwnedPlayerInput::complete(completion));
        assert!(matches!(
            out.take_refusals().pop().unwrap().reason(),
            PlayerOwnershipError::Storage(OwnershipError::Backend(_))
        ));
        assert_eq!(env.count(&handle, 36), 0);
        assert!(env.shard.entities().unwrap().get(&item).is_ok());
        assert_eq!(
            env.shard
                .player_ownership_checkpoint(query)
                .unwrap()
                .status(),
            OwnershipPendingStatus::NeedsRefresh
        );
        let mut out = env.tick(OwnedPlayerInput::autosave(handle.clone(), id(2)));
        assert!(matches!(
            out.take_refusals().pop().unwrap().reason(),
            PlayerOwnershipError::Reserved
        ));
        fault.fault(0);
        let out = env.tick(OwnedPlayerInput::reconcile(query));
        env.finish(out).await;
        assert_eq!(env.count(&handle, 36), 5);
        assert!(env.shard.entities().unwrap().get(&item).is_err());
    }
}

#[tokio::test]
async fn uncertain_actual_postcommit_and_stale_completion_cannot_release_a_new_dispatch() {
    let (mut env, fault, handle, item) = fault_env(false).await;
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item,
        read,
        id(1),
        true,
        false,
    ));
    let work = out.take_work().pop().unwrap();
    let query = work.query();
    fault.fault(2);
    let completion = work.execute(&env.store).await.unwrap();
    let mut out = env.tick(OwnedPlayerInput::complete(completion));
    let (_, stale) = out.take_refusals().pop().unwrap().into_parts();
    assert_eq!(env.count(&handle, 36), 0);
    assert_eq!(
        env.shard
            .player_ownership_checkpoint(query)
            .unwrap()
            .status(),
        OwnershipPendingStatus::Uncertain
    );
    let mut out = env.tick(OwnedPlayerInput::retry(query));
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::Reserved
    ));
    let mut out = env.tick(OwnedPlayerInput::reconcile(query));
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::Reserved
    ));
    let mut rejected = env.tick(stale);
    assert!(matches!(
        rejected.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::CompletionMismatch
    ));
    let mut out = env.tick(OwnedPlayerInput::retry(query));
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::UnprovenWorkerLifetime
    ));
    fault.fault(0);
    let out = env.tick(OwnedPlayerInput::reconcile(query));
    env.finish(out).await;
    assert_eq!(env.count(&handle, 36), 5);
}

#[tokio::test]
async fn missing_receipt_after_uncertain_execution_never_proves_abort() {
    let (mut env, fault, handle, item) = fault_env(false).await;
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item.clone(),
        read,
        id(1),
        true,
        false,
    ));
    let work = out.take_work().pop().unwrap();
    let query = work.query();
    fault.fault(4);
    let completion = work.execute(&env.store).await.unwrap();
    env.tick(OwnedPlayerInput::complete(completion));
    fault.fault(0);
    let out = env.tick(OwnedPlayerInput::reconcile(query));
    let mut out = env.finish(out).await;
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::UnprovenWorkerLifetime
    ));
    assert_eq!(env.shard.player_ownership_pending_count(), 1);
    assert_eq!(env.count(&handle, 36), 0);
    assert!(env.shard.entities().unwrap().get(&item).is_ok());
    let mut out = env.tick(OwnedPlayerInput::stop());
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::NotQuiescent
    ));
}

#[tokio::test]
async fn busy_did_not_start_and_identical_retry_retains_original_count_and_postimage() {
    let (mut env, fault, handle, item) = fault_env(false).await;
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item,
        read,
        id(1),
        true,
        false,
    ));
    let work = out.take_work().pop().unwrap();
    let query = work.query();
    fault.fault(3);
    let completion = work.execute(&env.store).await.unwrap();
    env.tick(OwnedPlayerInput::complete(completion));
    assert_eq!(
        env.shard
            .player_ownership_checkpoint(query)
            .unwrap()
            .status(),
        OwnershipPendingStatus::Prepared
    );
    assert_eq!(env.count(&handle, 36), 0);
    fault.fault(0);
    let mut out = env.tick(OwnedPlayerInput::retry(query));
    let work = out.take_work().pop().unwrap();
    assert_eq!(work.query(), query);
    let completion = work.execute(&env.store).await.unwrap();
    env.tick(OwnedPlayerInput::complete(completion));
    assert_eq!(env.count(&handle, 36), 5);
}

#[tokio::test]
async fn historical_receipt_after_later_write_quarantines_and_never_overwrites_current() {
    for redb in [false, true] {
        let (mut env, fault, handle, item) = fault_env(redb).await;
        let read = env.pair(uuid(1)).await;
        let mut out = env.tick(OwnedPlayerInput::pickup(
            handle.clone(),
            item.clone(),
            read,
            id(1),
            true,
            false,
        ));
        let work = out.take_work().pop().unwrap();
        let query = work.query();
        let checkpoint = env.shard.player_ownership_checkpoint(query).unwrap();
        fault.fault(2);
        let completion = work.execute(&env.store).await.unwrap();
        env.tick(OwnedPlayerInput::complete(completion));
        let current = fault
            .inner
            .load_owned_player(ScopedPlayerKey::new(scope(), player()), identity())
            .await
            .unwrap();
        let mut slots = empty();
        slots[36] = stone(6);
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
        fault
            .inner
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    id(2),
                    current.expectation().clone(),
                    PlayerOwnershipProposal::put(record),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        fault.fault(0);
        assert!(matches!(
            env.binding
                .clarify_checkpoint(&env.store, &checkpoint)
                .await,
            Err(PlayerOwnershipError::Superseded)
        ));
        let out = env.tick(OwnedPlayerInput::reconcile(query));
        let mut out = env.finish(out).await;
        assert!(matches!(
            out.take_refusals().pop().unwrap().reason(),
            PlayerOwnershipError::Superseded
        ));
        assert_eq!(
            env.shard
                .player_ownership_checkpoint(query)
                .unwrap()
                .status(),
            OwnershipPendingStatus::Quarantined
        );
        assert_eq!(env.count(&handle, 36), 0);
        assert!(env.shard.entities().unwrap().get(&item).is_ok());
        assert_eq!(env.shard.player_ownership_pending_count(), 1);
    }
}

#[tokio::test]
async fn wrong_backend_returns_whole_work_before_start_and_foreign_owner_rejects_completion() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item,
        read,
        id(1),
        true,
        false,
    ));
    let work = out.take_work().pop().unwrap();
    let query = work.query();
    let other = Env::new(false).await;
    let (reason, work) = work.execute(&other.store).await.unwrap_err().into_parts();
    assert!(matches!(reason, PlayerOwnershipError::ForeignStore));
    assert_eq!(work.query(), query);
    let completion = work.execute(&env.store).await.unwrap();
    let mut other = other;
    let mut out = other.tick(OwnedPlayerInput::complete(completion));
    let (reason, input) = out.take_refusals().pop().unwrap().into_parts();
    assert!(matches!(reason, PlayerOwnershipError::CompletionMismatch));
    env.tick(input);
    assert_eq!(env.count(&handle, 36), 5);
}

#[tokio::test]
async fn owned_input_count_and_exact_byte_limits_refuse_newest_intact() {
    let tiny = PlayerOwnershipLimits::new(
        nz(3),
        nz(MAX_OWNED_PLAYER_BYTES),
        nz(4),
        nz(MAX_PLAYER_OWNERSHIP_PENDING_BYTES),
        nz(1),
        nz(512),
    )
    .unwrap();
    let mut env = Env::with_limits(false, tiny).await;
    env.shard
        .enqueue_owned_player_input(OwnedPlayerInput::drain())
        .unwrap();
    let (reason, input) = env
        .shard
        .enqueue_owned_player_input(OwnedPlayerInput::stop())
        .unwrap_err()
        .into_parts();
    assert!(matches!(
        reason,
        PlayerOwnershipError::Capacity("input count")
    ));
    env.shard.run_tick_with_player_ownership().unwrap();
    env.shard.enqueue_owned_player_input(input).unwrap();
    let out = env.shard.run_tick_with_player_ownership().unwrap();
    assert!(out
        .intents()
        .iter()
        .any(|i| matches!(i, OwnershipIntent::Stopped)));
    let too_small = PlayerOwnershipLimits::new(
        nz(3),
        nz(MAX_OWNED_PLAYER_BYTES),
        nz(4),
        nz(MAX_PLAYER_OWNERSHIP_PENDING_BYTES),
        nz(1),
        nz(511),
    )
    .unwrap();
    let mut env = Env::with_limits(false, too_small).await;
    assert!(matches!(
        env.shard
            .enqueue_owned_player_input(OwnedPlayerInput::drain())
            .unwrap_err()
            .reason(),
        PlayerOwnershipError::Capacity("input bytes")
    ));
}

#[tokio::test]
async fn exact_pending_charge_and_player_count_byte_boundaries_preserve_fresh_items() {
    const EMPTY: usize = 558;
    const CHARGE: usize = 661 + 5 * EMPTY;
    for (pending_bytes, player_bytes, succeeds) in [
        (CHARGE, EMPTY, true),
        (CHARGE - 1, EMPTY, false),
        (CHARGE, EMPTY - 1, false),
    ] {
        let cap = PlayerOwnershipLimits::new(
            nz(1),
            nz(player_bytes),
            nz(1),
            nz(pending_bytes),
            nz(4),
            nz(8000),
        )
        .unwrap();
        let mut env = Env::with_limits(false, cap).await;
        let read = env.binding.read_player(&env.store, player()).await.unwrap();
        let mut out = env.tick(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [0.0, 0.0],
            id(1),
        ));
        if succeeds {
            let work = out.take_work().pop().unwrap();
            let query = work.query();
            let checkpoint = env.shard.player_ownership_checkpoint(query).unwrap();
            assert_eq!(checkpoint.version(), 1);
            let read = env
                .binding
                .read_player(&env.store, PlayerId::offline("Second"))
                .await
                .unwrap();
            let mut refused = env.tick(OwnedPlayerInput::bootstrap(
                read,
                position(),
                [0.0, 0.0],
                id(2),
            ));
            assert_eq!(refused.take_refusals().len(), 1);
            let completion = work.execute(&env.store).await.unwrap();
            env.tick(OwnedPlayerInput::complete(completion));
            assert_eq!(env.shard.player_count(), 1);
        } else {
            assert!(out.take_work().is_empty());
            assert_eq!(out.take_refusals().len(), 1);
            assert_eq!(env.shard.player_count(), 0);
            assert_eq!(env.shard.player_ownership_pending_count(), 0);
        }
    }
}

#[tokio::test]
async fn lost_work_buffer_allows_readonly_reconcile_but_receipt_none_retains_reservation() {
    let mut env = Env::new(false).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item.clone(),
        read,
        id(1),
        true,
        false,
    ));
    let work = out.take_work().pop().unwrap();
    let query = work.query();
    drop(work);
    let out = env.tick(OwnedPlayerInput::reconcile(query));
    let mut out = env.finish(out).await;
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::UnprovenWorkerLifetime
    ));
    assert_eq!(env.count(&handle, 36), 0);
    assert!(env.shard.entities().unwrap().get(&item).is_ok());
    assert_eq!(env.shard.player_ownership_pending_count(), 1);
}

async fn independent_item_record(proposal: ItemEntitySpawn) -> PersistentEntityRecord {
    let mut legacy_shard =
        SimShard::in_dimension(ShardPos::new(0, 0), scope().world(), scope().dimension());
    legacy_shard
        .initialize_entities_with_item_budget(nz(1), nz(10000))
        .unwrap();
    let storage = InMemoryStore::new();
    legacy_shard
        .loaded_chunks_mut()
        .acquire(
            &storage,
            &FlatWorldGenerator::new(),
            ChunkPos::new(0, 0),
            ChunkTicket::of(TicketReason::Spawn),
        )
        .await
        .unwrap();
    let (item, _) = legacy_shard
        .run_tick_with_entity_work(|c| c.spawn_item(proposal).unwrap())
        .unwrap();
    let (_, record) = legacy_shard
        .entities()
        .unwrap()
        .capture_item(&item)
        .unwrap();
    record
}

#[tokio::test]
async fn definitive_spawn_conflict_returns_the_whole_caller_owned_stack_and_properties() {
    let mut env = Env::new(false).await;
    let proposal = ItemEntitySpawn::new(
        EntitySpawn::new(
            scope().world(),
            scope().dimension(),
            uuid(1),
            EntityId::new(71),
            EntityTypeId::from_name("item").unwrap(),
            EntityTransform::try_new(position(), 0.0, 0.0).unwrap(),
        ),
        stone(5),
        ItemEntityProperties::try_new(
            Vec3::new(-0.0, 11.5, 0.0),
            -0.0,
            ItemEntityCounters::new(9, 40, 5),
            None,
            Some(uuid(9)),
        )
        .unwrap(),
    );
    let read = env.binding.read_entity(&env.store, uuid(1)).await.unwrap();
    let mut out = env.tick(OwnedPlayerInput::spawn_item(read, proposal, id(1)));
    let work = out.take_work().pop().unwrap();
    let expected = env
        .store
        .load_owned_entity(
            PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(1)),
            identity(),
        )
        .await
        .unwrap();
    let legacy = ItemEntitySpawn::new(
        EntitySpawn::new(
            scope().world(),
            scope().dimension(),
            uuid(1),
            EntityId::new(72),
            EntityTypeId::from_name("item").unwrap(),
            EntityTransform::try_new(position(), 0.0, 0.0).unwrap(),
        ),
        stone(2),
        ItemEntityProperties::try_new(
            Vec3::new(0.0, 0.0, 0.0),
            0.0,
            ItemEntityCounters::new(0, 0, 5),
            None,
            None,
        )
        .unwrap(),
    );
    let record = independent_item_record(legacy).await;
    env.store
        .compare_exchange_owned_entity(
            EntityOwnershipCasRequest::new(
                id(2),
                expected.expectation().clone(),
                EntityOwnershipProposal::put(record),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let completion = work.execute(&env.store).await.unwrap();
    let mut out = env.tick(OwnedPlayerInput::complete(completion));
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::Storage(OwnershipError::CasConflict { .. })
    ));
    let proposal = out
        .take_intents()
        .into_iter()
        .find_map(|i| {
            if let OwnershipIntent::ItemSpawnRefused { proposal, .. } = i {
                Some(proposal)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(proposal.stack().count(), 5);
    assert_eq!(proposal.properties().motion().y, 11.5);
    assert_eq!(
        proposal.properties().motion().x.to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(
        proposal.properties().bob_offset().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(proposal.properties().counters().age(), 9);
    assert!(env.shard.entities().unwrap().is_empty());
    assert_eq!(env.shard.player_ownership_pending_count(), 0);
}

async fn reopened_redb_env(path: PathBuf) -> Env {
    let backend: Arc<dyn Backend> =
        Arc::new(RedbStore::open_with_player_entity_ownership(&path, config()).unwrap());
    let store: Arc<dyn PlayerEntityOwnershipStore> = backend.clone();
    let binding = OwnershipStoreBinding::new(&store, GenerationBinding::new(scope(), identity()));
    let mut shard = SimShard::with_generation(ShardPos::new(0, 0), binding.generation());
    shard
        .initialize_entities_with_item_budget(nz(8), nz(1024 * 1024))
        .unwrap();
    shard
        .loaded_chunks_mut()
        .acquire(
            backend.as_ref(),
            &FlatWorldGenerator::with_seed(WorldSeed::new(37)),
            ChunkPos::new(0, 0),
            ChunkTicket::of(TicketReason::Spawn),
        )
        .await
        .unwrap();
    shard
        .initialize_player_ownership(binding.clone(), limits())
        .unwrap();
    Env {
        shard,
        backend,
        store,
        binding,
        path: Some(path),
    }
}

#[tokio::test]
async fn redb_reopen_restores_actual_player_and_item_with_fresh_local_and_wire_handles() {
    let mut env = Env::new(true).await;
    let mut slots = empty();
    slots[36] = stone(3);
    let old = env.seed_player(slots, 0).await;
    let read = env.pair(uuid(2)).await;
    let out = env.tick(OwnedPlayerInput::drop_selected(
        old.clone(),
        read,
        DropSelection::One,
        EntityId::new(3),
        id(1),
        4,
        false,
    ));
    env.finish(out).await;
    let old_item = env
        .shard
        .entities()
        .unwrap()
        .handle_for_uuid(uuid(2))
        .unwrap();
    let path = env.path.take().unwrap();
    drop(env);
    let mut env = reopened_redb_env(path).await;
    let read = env.binding.read_player(&env.store, player()).await.unwrap();
    let out = env.tick(OwnedPlayerInput::restore(read));
    let handle = out
        .intents()
        .iter()
        .find_map(|i| {
            if let OwnershipIntent::PlayerAdmitted { player, .. } = i {
                Some(player.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(env.count(&handle, 36), 2);
    assert!(matches!(
        env.shard.owned_player_slots(&old),
        Err(PlayerOwnershipError::StaleHandle)
    ));
    let read = env.binding.read_entity(&env.store, uuid(2)).await.unwrap();
    env.tick(OwnedPlayerInput::restore_item(read, EntityId::new(777)));
    let item = env
        .shard
        .entities()
        .unwrap()
        .handle_for_uuid(uuid(2))
        .unwrap();
    assert!(env.shard.entities().unwrap().get(&old_item).is_err());
    assert_eq!(
        env.shard.entities().unwrap().get(&item).unwrap().wire_id(),
        EntityId::new(777)
    );
    assert_eq!(
        env.shard
            .entities()
            .unwrap()
            .item_state(&item)
            .unwrap()
            .stack()
            .count(),
        1
    );
    assert_eq!(
        env.shard
            .entities()
            .unwrap()
            .item_state(&item)
            .unwrap()
            .properties()
            .counters()
            .pickup_delay(),
        40
    );
}

#[tokio::test]
async fn storage_aba_delete_recreate_conflict_never_changes_actual_item_or_player() {
    let mut env = Env::new(true).await;
    let handle = env.seed_player(empty(), 0).await;
    let item = env.spawn(5, 0).await;
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item.clone(),
        read,
        id(1),
        true,
        false,
    ));
    let work = out.take_work().pop().unwrap();
    let key = PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(1));
    let before = env.store.load_owned_entity(key, identity()).await.unwrap();
    let record = before.record().unwrap().clone();
    env.store
        .compare_exchange_owned_entity(
            EntityOwnershipCasRequest::new(
                id(2),
                before.expectation().clone(),
                EntityOwnershipProposal::delete(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let missing = env.store.load_owned_entity(key, identity()).await.unwrap();
    assert!(missing.stamp().is_tombstone());
    env.store
        .compare_exchange_owned_entity(
            EntityOwnershipCasRequest::new(
                id(3),
                missing.expectation().clone(),
                EntityOwnershipProposal::put(record),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let completion = work.execute(&env.store).await.unwrap();
    let mut out = env.tick(OwnedPlayerInput::complete(completion));
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::Storage(OwnershipError::CasConflict { .. })
    ));
    assert_eq!(env.count(&handle, 36), 0);
    assert_eq!(
        env.shard
            .entities()
            .unwrap()
            .item_state(&item)
            .unwrap()
            .stack()
            .count(),
        5
    );
    assert_eq!(env.shard.player_ownership_pending_count(), 0);
}

#[tokio::test]
async fn pending_count_limit_and_resident_gate_refuse_before_new_authority_or_items() {
    let cap = PlayerOwnershipLimits::new(
        nz(3),
        nz(MAX_OWNED_PLAYER_BYTES),
        nz(1),
        nz(MAX_PLAYER_OWNERSHIP_PENDING_BYTES),
        nz(8),
        nz(MAX_PLAYER_OWNERSHIP_INPUT_BYTES),
    )
    .unwrap();
    let mut env = Env::with_limits(false, cap).await;
    let read = env.binding.read_player(&env.store, player()).await.unwrap();
    let out = env.tick(OwnedPlayerInput::bootstrap(
        read,
        position(),
        [0.0, 0.0],
        id(1),
    ));
    let read = env
        .binding
        .read_player(&env.store, PlayerId::offline("Second"))
        .await
        .unwrap();
    let mut rejected = env.tick(OwnedPlayerInput::bootstrap(
        read,
        position(),
        [0.0, 0.0],
        id(2),
    ));
    assert!(matches!(
        rejected.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::Capacity("pending count")
    ));
    let out = env.finish(out).await;
    let handle = out
        .intents()
        .iter()
        .find_map(|i| {
            if let OwnershipIntent::PlayerAdmitted { player, .. } = i {
                Some(player.clone())
            } else {
                None
            }
        })
        .unwrap();
    let item = env.spawn(5, 0).await;
    env.shard
        .release_owned_chunk(ChunkPos::new(0, 0), ChunkTicket::of(TicketReason::Spawn))
        .unwrap();
    let read = env.pair(uuid(1)).await;
    let mut out = env.tick(OwnedPlayerInput::pickup(
        handle.clone(),
        item.clone(),
        read,
        id(3),
        true,
        false,
    ));
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::EntityStore(EntityStoreError::ChunkNotResident { .. })
    ));
    assert_eq!(env.count(&handle, 36), 0);
    assert_eq!(
        env.shard
            .entities()
            .unwrap()
            .item_state(&item)
            .unwrap()
            .stack()
            .count(),
        5
    );
}

fn assert_checkpoint_malformed(bytes: &[u8]) {
    // Version-1 fixed fields: 16-byte scope/header, 27-byte generation,
    // 16-byte transfer ID, 32-byte SHA; both UUID+stamp fields are 25 bytes.
    assert_eq!(bytes[91], 1);
    assert_eq!(bytes[117], 1);
    assert_eq!(bytes[143], 1);
    for (at, value) in [
        (0, b'X'),
        (5, 2),
        (6, 255),
        (7, 255),
        (91, 2),
        (108, 255),
        (117, 2),
        (134, 255),
        (143, 2),
        (148, 255),
        (153, b'X'),
        (191, 9),
        (6, 5),
    ] {
        let mut malformed = bytes.to_vec();
        malformed[at] = value;
        assert!(
            OwnershipPendingCheckpoint::try_from_bytes(&malformed).is_err(),
            "field {at}"
        );
    }
    let mut oversized = bytes.to_vec();
    oversized[149..153].copy_from_slice(&((MAX_PLAYER_DATA_LEN as u32) + 1).to_be_bytes());
    assert!(OwnershipPendingCheckpoint::try_from_bytes(&oversized).is_err());
    let mut trailing = bytes.to_vec();
    trailing.push(0);
    assert!(OwnershipPendingCheckpoint::try_from_bytes(&trailing).is_err());
}

#[tokio::test]
async fn removed_or_opaque_native_capacity_policy_refuses_without_losing_items() {
    for patch in [
        ComponentPatch::new(vec![], vec![ComponentTypeId::new(1)]),
        ComponentPatch::new(
            vec![ComponentValue::Opaque(
                OpaqueComponent::new(1, vec![64]).unwrap(),
            )],
            vec![],
        ),
    ] {
        let mut env = Env::new(false).await;
        let mut slots = empty();
        slots[36] =
            ItemStack::try_new(ItemId::from_name("stone").unwrap(), 2, patch.clone()).unwrap();
        let handle = env.seed_player(slots, 0).await;
        let read = env.pair(uuid(2)).await;
        let mut out = env.tick(OwnedPlayerInput::drop_selected(
            handle.clone(),
            read,
            DropSelection::One,
            EntityId::new(3),
            id(1),
            4,
            false,
        ));
        assert!(out.take_work().is_empty());
        assert!(matches!(
            out.take_refusals().pop().unwrap().reason(),
            PlayerOwnershipError::UnresolvedComponentPolicy(1)
        ));
        assert_eq!(env.count(&handle, 36), 2);
        assert_eq!(
            env.shard.owned_player_slots(&handle).unwrap()[36].components(),
            &patch
        );
        assert_eq!(env.shard.player_ownership_pending_count(), 0);
        assert!(env.shard.entities().unwrap().is_empty());
        assert!(env
            .store
            .load_owned_entity(
                PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(2)),
                identity()
            )
            .await
            .unwrap()
            .stamp()
            .is_fresh());
    }
}

#[tokio::test]
async fn bootstrap_never_converts_a_real_player_tombstone_into_fresh_survival() {
    for redb in [false, true] {
        let mut env = Env::new(redb).await;
        let key = ScopedPlayerKey::new(scope(), player());
        let fresh = env.store.load_owned_player(key, identity()).await.unwrap();
        env.store
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    id(201),
                    fresh.expectation().clone(),
                    PlayerOwnershipProposal::put(
                        NativePlayerSnapshot::empty_survival(position(), [0.0, 0.0])
                            .try_to_record()
                            .unwrap(),
                    ),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let live = env.store.load_owned_player(key, identity()).await.unwrap();
        env.store
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    id(202),
                    live.expectation().clone(),
                    PlayerOwnershipProposal::delete(),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let read = env.binding.read_player(&env.store, player()).await.unwrap();
        let mut out = env.tick(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [0.0, 0.0],
            id(1),
        ));
        assert!(out.take_work().is_empty());
        assert!(matches!(
            out.take_refusals().pop().unwrap().reason(),
            PlayerOwnershipError::Ineligible("bootstrap requires never-seen miss")
        ));
        assert_eq!(env.shard.player_count(), 0);
        assert_eq!(env.shard.player_ownership_pending_count(), 0);
        assert!(env
            .store
            .load_owned_player(key, identity())
            .await
            .unwrap()
            .stamp()
            .is_tombstone());
    }
}
