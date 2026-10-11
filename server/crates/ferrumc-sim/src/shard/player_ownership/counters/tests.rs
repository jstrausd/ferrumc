use super::super::WorkRequest;
use super::*;
use crate::*;
use ferrumc_core::{DimensionId, EntityId, EntityUuid, PlayerId, WorldId};
use ferrumc_items::{encode_persisted_stack, ComponentPatch, ItemId, ItemStack};
use ferrumc_math::{ChunkPos, ShardPos, Vec3};
use ferrumc_storage::*;
use ferrumc_world::{FlatWorldGenerator, GenerationIdentity, WorldSeed};
use std::num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroUsize};
use std::path::Path;

mod publications;

trait Backend: WorldStore + PlayerEntityOwnershipStore {}
impl<T: WorldStore + PlayerEntityOwnershipStore> Backend for T {}
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn uuid(n: u128) -> EntityUuid {
    EntityUuid::from_bytes(n.to_be_bytes())
}
fn id(n: u128) -> OwnershipTransferId {
    OwnershipTransferId::from_bytes((n + (1_u128 << 96)).to_be_bytes())
}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(7), DimensionId::new(13))
}
fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(37))
}
fn position() -> Vec3 {
    Vec3::new(1.0, 64.0, 1.0)
}
fn stone(count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}
fn limits(period: u32, rate: i32, defer: usize) -> OwnedItemCounterLimits {
    OwnedItemCounterLimits::new(
        nz(MAX_CONFIRMED_ITEM_BYTES),
        nz(defer),
        nz(64),
        nz(MAX_OWNED_COUNTER_PLAN_ENTRIES),
        nz(MAX_OWNED_COUNTER_PLAN_BYTES),
        nz(MAX_OWNED_ENTITY_SAVE_CHARGE),
        NonZeroU32::new(period).unwrap(),
        rate,
        nz(32),
    )
    .unwrap()
}
fn config() -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            nz(MAX_OWNERSHIP_REQUEST_BYTES),
            NonZeroU64::new(10000).unwrap(),
            NonZeroU64::new(10000).unwrap(),
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

// Independent fixed native envelope, not an encode/decode roundtrip. FCIS is
// supplied by the separately tested item producer; all FCIE fields are explicit.
fn native(n: u128, count: u8, counters: ItemEntityCounters) -> PersistentEntityRecord {
    let stack = encode_persisted_stack(&stone(count)).unwrap();
    let mut bytes = b"FCIE\0\x01".to_vec();
    bytes.extend_from_slice(
        &ferrumc_registry::entity_type::EntityTypeId::from_name("item")
            .unwrap()
            .protocol_id()
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&scope().world().get().to_be_bytes());
    bytes.extend_from_slice(&scope().dimension().get().to_be_bytes());
    bytes.extend_from_slice(&uuid(n).to_bytes());
    for n in [1.0_f64, 64.0, 1.0] {
        bytes.extend_from_slice(&n.to_bits().to_be_bytes());
    }
    for n in [-0.0_f32, 0.0] {
        bytes.extend_from_slice(&n.to_bits().to_be_bytes());
    }
    for n in [-0.0_f64, 0.0, 0.0] {
        bytes.extend_from_slice(&n.to_bits().to_be_bytes());
    }
    bytes.extend_from_slice(&(-0.0_f32).to_bits().to_be_bytes());
    for n in [counters.age(), counters.pickup_delay(), counters.health()] {
        bytes.extend_from_slice(&n.to_be_bytes());
    }
    bytes.extend_from_slice(&[0; 34]);
    bytes.extend_from_slice(&(stack.len() as u32).to_be_bytes());
    assert_eq!(bytes.len(), NATIVE_ITEM_ENTITY_HEADER_BYTES);
    bytes.extend_from_slice(&stack);
    PersistentEntityRecord::new(
        ChunkPos::ORIGIN,
        EntityRecord::new(SchemaVersion::new(1), bytes).unwrap(),
    )
    .unwrap()
}
struct Env {
    shard: SimShard,
    backend: Arc<dyn Backend>,
    store: Arc<dyn PlayerEntityOwnershipStore>,
    binding: OwnershipStoreBinding,
    owner: OwnedItemCounterHandle,
    tick: Tick,
}
impl Env {
    async fn new(
        path: Option<&Path>,
        cap: usize,
        counter: OwnedItemCounterLimits,
        baseline: Tick,
    ) -> Self {
        Self::new_with_busy(path, cap, counter, baseline, None).await
    }
    async fn new_with_busy(
        path: Option<&Path>,
        cap: usize,
        counter: OwnedItemCounterLimits,
        baseline: Tick,
        busy: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Self {
        let backend: Arc<dyn Backend> = if let Some(path) = path {
            Arc::new(RedbStore::open_with_player_entity_ownership(path, config()).unwrap())
        } else {
            Arc::new(InMemoryStore::with_player_entity_ownership(config()).unwrap())
        };
        backend
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let inner: Arc<dyn PlayerEntityOwnershipStore> = backend.clone();
        let store: Arc<dyn PlayerEntityOwnershipStore> = if let Some(busy) = busy {
            Arc::new(EntityBusyStore { inner, busy })
        } else {
            inner
        };
        let binding =
            OwnershipStoreBinding::new(&store, GenerationBinding::new(scope(), identity()));
        let mut shard = SimShard::with_generation(ShardPos::new(0, 0), binding.generation());
        shard
            .initialize_entities_with_item_budget(nz(cap), nz(MAX_ITEM_ENTITY_BYTES_PER_SHARD))
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
                    nz(4),
                    nz(MAX_OWNED_PLAYER_BYTES),
                    nz(4),
                    nz(MAX_PLAYER_OWNERSHIP_PENDING_BYTES),
                    nz(32),
                    nz(MAX_PLAYER_OWNERSHIP_INPUT_BYTES),
                )
                .unwrap(),
            )
            .unwrap();
        let owner = shard
            .initialize_owned_item_counters(counter, baseline)
            .unwrap();
        Self {
            shard,
            backend,
            store,
            binding,
            owner,
            tick: baseline,
        }
    }
    fn run(&mut self) -> PlayerOwnershipTickOutcome {
        let tick = self.tick.next().unwrap();
        let outcome = self.shard.run_tick_with_player_ownership_at(tick).unwrap();
        self.tick = tick;
        outcome
    }
    fn input(&mut self, input: OwnedPlayerInput) -> PlayerOwnershipTickOutcome {
        self.shard.enqueue_owned_player_input(input).unwrap();
        self.run()
    }
    async fn finish(&mut self, mut out: PlayerOwnershipTickOutcome) -> PlayerOwnershipTickOutcome {
        let mut work = out.take_work();
        assert_eq!(work.len(), 1);
        let completion = work.remove(0).execute(&self.store).await.unwrap();
        self.input(OwnedPlayerInput::complete(completion))
    }
    async fn seed(&self, n: u128, count: u8, counters: ItemEntityCounters) -> OwnedEntityRead {
        let key = PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(n));
        let snapshot = self.store.load_owned_entity(key, identity()).await.unwrap();
        self.store
            .compare_exchange_owned_entity(
                EntityOwnershipCasRequest::new(
                    id(n + 10000),
                    snapshot.expectation().clone(),
                    EntityOwnershipProposal::put(native(n, count, counters)),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        self.binding
            .read_entity(&self.store, uuid(n))
            .await
            .unwrap()
    }
    async fn restore(&mut self, n: u128, count: u8, counters: ItemEntityCounters) -> EntityHandle {
        let read = self.seed(n, count, counters).await;
        let mut out = self.input(OwnedPlayerInput::restore_item(
            read,
            EntityId::new(n as i32),
        ));
        assert!(out.take_refusals().is_empty());
        self.handle(n)
    }
    fn handle(&self, n: u128) -> EntityHandle {
        self.shard
            .entities()
            .unwrap()
            .handle_for_uuid(uuid(n))
            .unwrap()
    }
    fn counters(&self, n: u128) -> ItemEntityCounters {
        self.shard
            .entities()
            .unwrap()
            .item_state(&self.handle(n))
            .unwrap()
            .properties()
            .counters()
    }
    async fn durable(&self, n: u128) -> OwnedEntitySnapshot {
        self.store
            .load_owned_entity(
                PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(n)),
                identity(),
            )
            .await
            .unwrap()
    }
    async fn player(&mut self, slots: Box<[ItemStack; 46]>) -> OwnedPlayerHandle {
        let player = PlayerId::offline("CounterSurvival");
        let before = self
            .store
            .load_owned_player(ScopedPlayerKey::new(scope(), player), identity())
            .await
            .unwrap();
        let snapshot = NativePlayerSnapshot::new(
            position(),
            [0.0; 2],
            GameMode::Survival,
            0,
            slots,
            ItemStack::empty(),
        );
        self.store
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    id(9000),
                    before.expectation().clone(),
                    PlayerOwnershipProposal::put(snapshot.try_to_record().unwrap()),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let read = self.binding.read_player(&self.store, player).await.unwrap();
        let out = self.input(OwnedPlayerInput::restore(read));
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
}

#[test]
fn actual_scalar_shape_fits_positive_nine_mib_cap() {
    let metadata = size_of::<[Option<OwnershipTransferId>; MAX_PLAYER_OWNERSHIP_INPUTS]>() * 2
        + size_of::<[Option<CompletionOverlay<'_>>; MAX_PLAYER_OWNERSHIP_INPUTS]>()
        + size_of::<[Option<ShadowPlayer<'_>>; MAX_PLAYER_OWNERSHIP_INPUTS]>()
        + size_of::<ShadowResources>();
    let bytes = MAX_OWNED_COUNTER_PLAN_ENTRIES
        .checked_mul(size_of::<CounterBoundaryEntry>())
        .unwrap()
        + metadata;
    assert!(
        bytes <= MAX_OWNED_COUNTER_PLAN_BYTES,
        "entry={} actual={bytes}",
        size_of::<CounterBoundaryEntry>()
    );
}

#[tokio::test]
async fn all_4096_items_receive_40_steps_independent_of_64_slot_save_scan() {
    let mut e = Env::new(None, 4096, limits(100, i32::MAX, 1024), Tick::ZERO).await;
    let batches = 128;
    for batch in 0..batches {
        for index in 0..32 {
            let n = (batch * 32 + index + 1) as u128;
            let read = e
                .seed(n, 1, ItemEntityCounters::new(0, 40 + (batches - batch), 5))
                .await;
            e.shard
                .enqueue_owned_player_input(OwnedPlayerInput::restore_item(
                    read,
                    EntityId::new(n as i32),
                ))
                .unwrap();
        }
        let out = e.run();
        assert_eq!(out.counter_steps(), ((batch + 1) * 32) as usize);
        assert!(out.item_save_slots_scanned() <= 64);
    }
    for n in 1..=4096 {
        assert_eq!(e.counters(n).pickup_delay(), 40);
    }
    for _ in 0..40 {
        let out = e.run();
        assert_eq!(out.counter_steps(), 4096);
        assert!(out.item_save_slots_scanned() <= 64);
        assert!(out.work.is_empty());
    }
    for n in 1..=4096 {
        assert_eq!(e.counters(n).pickup_delay(), 0);
    }
}

#[tokio::test]
async fn duplicate_gap_tickmax_and_legacy_bypass_preserve_original_inbox() {
    let mut e = Env::new(None, 4, limits(100, 6000, 1024), Tick::new(u64::MAX - 1)).await;
    let read = e.seed(1, 1, ItemEntityCounters::new(0, 40, 5)).await;
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::restore_item(read, EntityId::new(1)))
        .unwrap();
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(Tick::new(u64::MAX - 1)),
        Err(PlayerOwnershipError::CounterTickMismatch)
    ));
    assert_eq!(e.shard.player_ownership.as_ref().unwrap().input.len(), 1);
    assert!(matches!(
        e.shard.run_tick_with_player_ownership(),
        Err(PlayerOwnershipError::CounterTickMismatch)
    ));
    e.run();
    assert_eq!(e.counters(1).pickup_delay(), 39);
    assert!(matches!(
        e.shard.run_tick_with_player_ownership_at(Tick::ZERO),
        Err(PlayerOwnershipError::CounterTickOverflow)
    ));
    let mut normal = Env::new(None, 1, limits(100, 6000, 1024), Tick::ZERO).await;
    assert!(matches!(
        normal.shard.run_tick_with_player_ownership_at(Tick::new(2)),
        Err(PlayerOwnershipError::CounterTickMismatch)
    ));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| normal.shard.run_tick())).is_err()
    );
}

#[tokio::test]
async fn dirty_timer_save_confirms_captured_body_before_releasing_defer() {
    let mut e = Env::new(None, 4, limits(100, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    assert_eq!(e.counters(1).pickup_delay(), 39);
    let before = e.durable(1).await;
    assert_eq!(
        crate::decode_item_entity(before.expectation().key(), before.record().unwrap())
            .unwrap()
            .item_state()
            .properties()
            .counters()
            .pickup_delay(),
        40
    );
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    assert_eq!(e.counters(1).pickup_delay(), 39);
    assert_eq!(out.counter_steps(), 1);
    let out = e.finish(out).await;
    assert!(out.refusals.is_empty());
    assert_eq!(e.counters(1).pickup_delay(), 37);
    let durable = e.durable(1).await;
    assert_eq!(durable.stamp().revision().unwrap().get(), 2);
    assert_eq!(
        crate::decode_item_entity(durable.expectation().key(), durable.record().unwrap())
            .unwrap()
            .item_state()
            .properties()
            .counters()
            .pickup_delay(),
        39
    );
    let d = e.shard.entities().unwrap().item_durability(&h).unwrap();
    assert!(d.generation > d.saved);
}

#[tokio::test]
async fn exact_noop_entity_save_preserves_revision_and_empty_epochs() {
    let mut e = Env::new(None, 4, limits(100, 6000, 1024), Tick::ZERO).await;
    let h = e
        .restore(1, 4, ItemEntityCounters::new(-32768, 32767, -5))
        .await;
    let before = e.durable(1).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let CompletionEvidence::Confirmed { receipt, .. } = &completion.evidence else {
        panic!("genuine no-op did not confirm");
    };
    assert_eq!(receipt.epochs(), []);
    assert_eq!(receipt.entity().unwrap().1, receipt.entity().unwrap().2);
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert!(out.refusals.is_empty());
    assert_eq!(e.durable(1).await.stamp(), before.stamp());
    let d = e.shard.entities().unwrap().item_durability(&h).unwrap();
    assert_eq!(d.saved, d.generation);
}

#[tokio::test]
async fn pending_1024_plus_one_refuses_before_drain_and_completion_only_recovers() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let work = out.take_work().remove(0);
    for _ in 1..1024 {
        e.run();
    }
    let p = e
        .shard
        .player_ownership
        .as_ref()
        .unwrap()
        .pending
        .get(&id(1))
        .unwrap();
    assert_eq!(p.counters.unwrap().claims, 1024);
    assert_eq!(e.counters(1).age(), 1);
    let before = p
        .after_entity_record
        .as_ref()
        .unwrap()
        .record()
        .data()
        .to_vec();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::drain())
        .unwrap();
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(e.tick.next().unwrap()),
        Err(PlayerOwnershipError::CounterRecoveryRequired)
    ));
    assert!(!e.shard.player_ownership.as_ref().unwrap().draining);
    assert_eq!(
        e.shard
            .player_ownership
            .as_ref()
            .unwrap()
            .pending
            .get(&id(1))
            .unwrap()
            .after_entity_record
            .as_ref()
            .unwrap()
            .record()
            .data(),
        before
    );
    let completion = work.execute(&e.store).await.unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let out = e.shard.recover_player_ownership_completions().unwrap();
    assert!(out.refusals.is_empty());
    assert_eq!(e.shard.current_counter_tick(), e.tick);
    assert_eq!(e.counters(1).age(), 1025);
    assert!(e.shard.player_ownership.as_ref().unwrap().draining);
    assert_eq!(e.shard.player_ownership_pending_count(), 0);
}

#[tokio::test]
async fn future_confirmed_rest_generation_overflow_retains_completion_and_lease() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let p = e
        .shard
        .player_ownership
        .as_mut()
        .unwrap()
        .pending
        .get_mut(&id(1))
        .unwrap();
    p.counters.as_mut().unwrap().generation = u64::MAX;
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let bytes = e.shard.player_ownership.as_ref().unwrap().input_bytes;
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(e.tick.next().unwrap()),
        Err(PlayerOwnershipError::GenerationExhausted)
    ));
    assert_eq!(
        e.shard.player_ownership.as_ref().unwrap().input_bytes,
        bytes
    );
    assert_eq!(e.shard.player_ownership_pending_count(), 1);
    assert_eq!(e.counters(1).age(), 1);
    let out = e.shard.recover_player_ownership_completions().unwrap();
    assert!(out.refusals.is_empty());
    assert_eq!(e.shard.owned_item_generation(&h).unwrap(), u64::MAX);
}

#[tokio::test]
async fn unrelated_items_tick_through_store_reservation_and_unloaded_items_do_not() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    e.restore(2, 1, ItemEntityCounters::new(0, 40, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let frozen = e.counters(1);
    let second = e.counters(2);
    e.run();
    assert_eq!(e.counters(1), frozen);
    assert_eq!(e.counters(2).pickup_delay(), second.pickup_delay() - 1);
    e.shard
        .release_owned_chunk(ChunkPos::ORIGIN, ChunkTicket::of(TicketReason::Spawn))
        .unwrap();
    let second = e.counters(2);
    let out2 = e.run();
    assert_eq!(out2.counter_steps(), 0);
    assert_eq!(e.counters(2), second);
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(out.counter_steps(), 0);
    assert_eq!(e.counters(1).pickup_delay(), frozen.pickup_delay() - 2);
    e.shard
        .acquire_owned_chunk(
            e.backend.as_ref(),
            &FlatWorldGenerator::with_seed(WorldSeed::new(37)),
            ChunkPos::ORIGIN,
            ChunkTicket::of(TicketReason::Spawn),
        )
        .await
        .unwrap();
    e.run();
    assert_eq!(e.counters(2).pickup_delay(), second.pickup_delay() - 1);
}

#[tokio::test]
async fn due_item_is_not_pickable_and_delete_waits_for_genuine_tombstone() {
    let mut e = Env::new(None, 4, limits(100, -40000, 1024), Tick::ZERO).await;
    let h = e
        .restore(1, 4, ItemEntityCounters::new(-32768, 32767, -5))
        .await;
    let d = e.shard.entities().unwrap().item_durability(&h).unwrap();
    assert!(d.due);
    assert_eq!(e.counters(1).age(), -32768);
    let out = e.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        vec![id(1)],
    ));
    assert_eq!(out.work.len(), 1);
    assert_eq!(e.shard.entities().unwrap().len(), 1);
    let out = e.finish(out).await;
    assert!(out.refusals.is_empty());
    assert_eq!(e.shard.entities().unwrap().len(), 0);
    assert!(e.durable(1).await.stamp().is_tombstone());
}

#[tokio::test]
async fn full_and_partial_pickup_use_confirmed_before_and_live_counter_after() {
    for partial in [false, true] {
        let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
        let mut slots = Box::new(std::array::from_fn(|_| {
            if partial {
                stone(64)
            } else {
                ItemStack::empty()
            }
        }));
        if partial {
            slots[36] = stone(63);
        }
        let player = e.player(slots).await;
        let h = e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
        e.run();
        let live_age = e.counters(1).age();
        let read = e
            .binding
            .read_pair(&e.store, player.player(), uuid(1))
            .await
            .unwrap();
        let out = e.input(OwnedPlayerInput::pickup(
            player.clone(),
            h.clone(),
            read,
            id(1),
            true,
            false,
        ));
        assert!(out.refusals.is_empty());
        assert_eq!(e.counters(1).age(), live_age);
        assert_eq!(
            e.shard.owned_player_slots(&player).unwrap()[36].count(),
            if partial { 63 } else { 0 }
        );
        let out = e.finish(out).await;
        assert!(out.refusals.is_empty());
        assert_eq!(
            e.shard.owned_player_slots(&player).unwrap()[36].count(),
            if partial { 64 } else { 4 }
        );
        if partial {
            assert_eq!(e.counters(1).age(), live_age + 2);
            assert_eq!(
                e.shard
                    .entities()
                    .unwrap()
                    .item_state(&h)
                    .unwrap()
                    .stack()
                    .count(),
                3
            );
        } else {
            assert!(e.shard.entities().unwrap().is_empty());
        }
    }
}

#[tokio::test]
async fn future_confirmed_spawn_first_step_is_preflighted_at_generation_max() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let read = e.binding.read_entity(&e.store, uuid(1)).await.unwrap();
    let restored = decode_item_entity(
        PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(1)),
        &native(1, 4, ItemEntityCounters::new(0, 40, 5)),
    )
    .unwrap();
    let (key, kind, transform, stack, properties) = restored.into_parts();
    let proposal = ItemEntitySpawn::new(
        EntitySpawn::new(
            key.world(),
            key.dimension(),
            key.uuid(),
            EntityId::new(1),
            kind,
            transform,
        ),
        stack,
        properties,
    );
    let mut out = e.input(OwnedPlayerInput::spawn_item(read, proposal, id(1)));
    assert_eq!(out.counter_steps(), 0);
    assert!(e.shard.entities().unwrap().is_empty());
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .player_ownership
        .as_mut()
        .unwrap()
        .pending
        .get_mut(&id(1))
        .unwrap()
        .item_generation = Some(u64::MAX);
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(e.tick.next().unwrap()),
        Err(PlayerOwnershipError::GenerationExhausted)
    ));
    assert!(e.shard.entities().unwrap().is_empty());
    assert_eq!(e.shard.player_ownership.as_ref().unwrap().input.len(), 1);
    let out = e.shard.recover_player_ownership_completions().unwrap();
    assert!(out.refusals.is_empty());
    assert_eq!(e.counters(1).pickup_delay(), 40);
}

#[tokio::test]
async fn future_partial_pickup_rest_max_plus_one_retains_original_completion() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let mut slots = Box::new(std::array::from_fn(|_| stone(64)));
    slots[36] = stone(63);
    let player = e.player(slots).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    let read = e
        .binding
        .read_pair(&e.store, player.player(), uuid(1))
        .await
        .unwrap();
    let mut out = e.input(OwnedPlayerInput::pickup(
        player.clone(),
        h.clone(),
        read,
        id(1),
        true,
        false,
    ));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let pending = e
        .shard
        .player_ownership
        .as_mut()
        .unwrap()
        .pending
        .get_mut(&id(1))
        .unwrap();
    pending.counters.as_mut().unwrap().generation = u64::MAX;
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(e.tick.next().unwrap()),
        Err(PlayerOwnershipError::GenerationExhausted)
    ));
    assert_eq!(e.shard.owned_player_slots(&player).unwrap()[36].count(), 63);
    assert_eq!(
        e.shard
            .entities()
            .unwrap()
            .item_state(&h)
            .unwrap()
            .stack()
            .count(),
        4
    );
    assert_eq!(e.shard.player_ownership.as_ref().unwrap().input.len(), 1);
    let out = e.shard.recover_player_ownership_completions().unwrap();
    assert!(out.refusals.is_empty());
    assert_eq!(e.shard.owned_player_slots(&player).unwrap()[36].count(), 64);
    assert_eq!(
        e.shard
            .entities()
            .unwrap()
            .item_state(&h)
            .unwrap()
            .stack()
            .count(),
        3
    );
}

#[tokio::test]
async fn mixed_owned_movement_and_item_counters_work_while_ordinary_inputs_are_refused() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let player = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let age = e.counters(1).age();
    let moved = Vec3::new(2.0, 65.0, 2.0);
    assert!(matches!(
        e.shard.enqueue(GameInput::PlayerMove {
            player: player.player(),
            position: Some(moved),
            yaw: Some(15.0),
            pitch: Some(-5.0)
        }),
        Err(SimError::OwnershipInputRequired)
    ));
    assert!(e.shard.inbox.is_empty());
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::move_player(
            player.clone(),
            Some(moved),
            [Some(15.0), Some(-5.0)],
        ))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::select_hotbar(player.clone(), 3))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::set_game_mode(
            player.clone(),
            GameMode::Creative,
        ))
        .unwrap();
    let out = e.run();
    assert!(out.refusals.is_empty());
    assert_eq!(out.counter_steps(), 1);
    let actual = e.shard.actual_player(&player).unwrap();
    assert_eq!(actual.position, moved);
    assert_eq!(actual.game_mode, GameMode::Creative);
    assert_eq!(actual.owned.as_ref().unwrap().selected, 3);
    assert_eq!(e.counters(1).age(), age + 1);
    e.shard
        .entities
        .as_mut()
        .unwrap()
        .inject_counter_generation(&h, u64::MAX);
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::move_player(
            player.clone(),
            Some(position()),
            [None; 2],
        ))
        .unwrap();
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(e.tick.next().unwrap()),
        Err(PlayerOwnershipError::GenerationExhausted)
    ));
    assert_eq!(e.shard.actual_player(&player).unwrap().position, moved);
    assert_eq!(e.shard.player_ownership.as_ref().unwrap().input.len(), 1);
}

#[tokio::test]
async fn save_id_pool_refuses_duplicates_and_uses_one_id_only_on_successful_admission() {
    let mut low = limits(1, 6000, 1024);
    low.save_charge = 1;
    let mut e = Env::new(None, 4, low, Tick::ZERO).await;
    e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let mut out = e.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        vec![id(1), id(1)],
    ));
    let (_, input) = out.take_refusals().remove(0).into_parts();
    let InputKind::RefillItemSaveIds { ids, .. } = input.0 else {
        panic!("lost move-owned refill");
    };
    assert_eq!(ids, vec![id(1), id(1)]);
    let out = e.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        vec![id(1)],
    ));
    assert!(out.refusals.is_empty());
    for _ in 0..4 {
        let out = e.run();
        assert!(out.work.is_empty());
    }
    assert_eq!(
        e.shard
            .player_ownership
            .as_ref()
            .unwrap()
            .item_counters
            .as_ref()
            .unwrap()
            .save_ids,
        vec![id(1)]
    );
    let mut out = e.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        vec![id(1)],
    ));
    assert_eq!(out.take_refusals().len(), 1);
    let mut other = Env::new(None, 1, limits(1, 6000, 1024), Tick::ZERO).await;
    let mut out = other.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        vec![id(2)],
    ));
    assert!(matches!(
        out.take_refusals().remove(0).reason(),
        PlayerOwnershipError::StaleHandle
    ));
    let oversized = OwnedPlayerInput::refill_item_save_ids(e.owner.clone(), vec![id(3); 33]);
    let refusal = e.shard.enqueue_owned_player_input(oversized).unwrap_err();
    assert!(matches!(
        refusal.reason(),
        PlayerOwnershipError::Capacity("save ID input")
    ));
}

#[tokio::test]
async fn autosave_cursors_have_three_to_one_lanes_and_freeze_while_entity_work_is_pending() {
    let mut e = Env::new(None, 8, limits(1, 6000, 1024), Tick::ZERO).await;
    for n in 1..=8 {
        e.restore(n, 4, ItemEntityCounters::new(0, 0, 5)).await;
    }
    e.shard.entities.as_mut().unwrap().mark_counter_flush(None);
    let mut out = e.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        (1..=32).map(id).collect(),
    ));
    assert_eq!(out.work.len(), 1);
    let before = e
        .shard
        .player_ownership
        .as_ref()
        .unwrap()
        .item_counters
        .as_ref()
        .unwrap();
    let cursors = (
        before.normal_cursor,
        before.priority_cursor,
        before.selection_turn,
    );
    let pool = before.save_ids.len();
    for _ in 0..5 {
        let out = e.run();
        assert!(out.work.is_empty());
        assert_eq!(out.item_save_slots_scanned(), 0);
    }
    let after = e
        .shard
        .player_ownership
        .as_ref()
        .unwrap()
        .item_counters
        .as_ref()
        .unwrap();
    assert_eq!(
        (
            after.normal_cursor,
            after.priority_cursor,
            after.selection_turn
        ),
        cursors
    );
    assert_eq!(after.save_ids.len(), pool);
    let mut turns = Vec::new();
    for _ in 0..12 {
        let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
        let before = e
            .shard
            .player_ownership
            .as_ref()
            .unwrap()
            .item_counters
            .as_ref()
            .unwrap()
            .selection_turn;
        out = e.input(OwnedPlayerInput::complete(completion));
        assert!(out.refusals.is_empty());
        assert!(out.item_save_slots_scanned() <= 64);
        turns.push(before);
        // The priority stream is deliberately sustained; normal selection still
        // receives every fourth free boundary instead of starving forever.
        e.shard.entities.as_mut().unwrap().mark_counter_flush(None);
    }
    for window in turns.windows(4) {
        assert_eq!(window.iter().filter(|t| i32::from(**t) == 3).count(), 1);
    }
}

#[tokio::test]
async fn lost_reply_none_retains_exact_request_and_real_reconcile_releases_projection() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let work = out.take_work().remove(0);
    let query = work.query();
    drop(work);
    let mut out = e.input(OwnedPlayerInput::reconcile(query));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let mut out = e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(out.take_refusals().len(), 1);
    assert_eq!(e.shard.player_ownership_pending_count(), 1);
    assert_eq!(
        e.shard
            .player_ownership
            .as_ref()
            .unwrap()
            .pending
            .get(&id(1))
            .unwrap()
            .ticket
            .query,
        query
    );
    let request = e
        .shard
        .player_ownership
        .as_ref()
        .unwrap()
        .pending
        .get(&id(1))
        .unwrap()
        .request
        .clone();
    let WorkRequest::Entity(request) = request else {
        panic!("wrong retained domain");
    };
    e.store
        .compare_exchange_owned_entity(request)
        .await
        .unwrap();
    let mut out = e.input(OwnedPlayerInput::reconcile(query));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert!(out.refusals.is_empty());
    assert_eq!(e.shard.player_ownership_pending_count(), 0);
    assert_eq!(e.counters(1).age(), e.tick.get() as i32);
}

#[tokio::test]
async fn actual_signed_wrapping_sentinels_and_negative_rates_match_pinned_counter_rules() {
    for (age, delay, rate, want_age, want_delay, due) in [
        (i32::MAX, 1, 6000, i32::MIN, 0, false),
        (-32768, 32767, 6000, -32768, 32767, false),
        (-32768, 2, -40000, -32768, 1, true),
        (-2, -5, -1, -1, -5, true),
        (0, 32768, 6000, 1, 32767, false),
    ] {
        let mut e = Env::new(None, 1, limits(10000, rate, 1024), Tick::ZERO).await;
        let h = e
            .restore(1, 4, ItemEntityCounters::new(age, delay, -5))
            .await;
        assert_eq!(
            e.counters(1),
            ItemEntityCounters::new(want_age, want_delay, -5)
        );
        assert_eq!(
            e.shard.entities().unwrap().item_durability(&h).unwrap().due,
            due
        );
    }
}

#[tokio::test]
async fn redb_dirty_save_noop_unload_flush_and_reopen_restore_only_confirmed_native_time() {
    let path = std::env::temp_dir().join(format!(
        "ferrumc-dur042-counters-{}.redb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut e = Env::new(Some(&path), 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let noop = e
        .restore(2, 1, ItemEntityCounters::new(-32768, 32767, -5))
        .await;
    let gen = e.shard.owned_item_generation(&noop).unwrap();
    let out = e.input(OwnedPlayerInput::save_item(noop, gen, id(2)));
    e.finish(out).await;
    assert_eq!(e.durable(2).await.stamp().revision().unwrap().get(), 1);
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    e.finish(out).await;
    let confirmed = e.durable(1).await.record().unwrap().clone();
    let confirmed_age = decode_item_entity(
        PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(1)),
        &confirmed,
    )
    .unwrap()
    .item_state()
    .properties()
    .counters()
    .age();
    assert!(e.counters(1).age() > confirmed_age);
    drop(e);
    let mut e = Env::new(Some(&path), 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let read = e.binding.read_entity(&e.store, uuid(1)).await.unwrap();
    e.input(OwnedPlayerInput::restore_item(read, EntityId::new(99)));
    assert_eq!(e.counters(1).age(), confirmed_age + 1);
    assert_eq!(
        e.shard
            .entities()
            .unwrap()
            .get(&e.handle(1))
            .unwrap()
            .wire_id(),
        EntityId::new(99)
    );
    assert!(matches!(
        e.shard.entities().unwrap().get(&h),
        Err(EntityStoreError::ForeignHandle)
    ));
    e.shard
        .release_owned_chunk(ChunkPos::ORIGIN, ChunkTicket::of(TicketReason::Spawn))
        .unwrap();
    let mut out = e.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        vec![id(3)],
    ));
    while out.work.is_empty() {
        out = e.run();
    }
    let out = e.finish(out).await;
    assert!(out.refusals.is_empty());
    assert!(e.shard.entities().unwrap().counter_quiescent());
    let out = e.input(OwnedPlayerInput::stop());
    assert!(out.refusals.is_empty());
    assert!(matches!(
        out.intents().last(),
        Some(OwnershipIntent::Stopped)
    ));
    drop(e);
    std::fs::remove_file(path).unwrap();
}

// Genuine work receives a no-start Busy response; every successful read/write
// and all receipts are still supplied by the actual ownership backend.
struct EntityBusyStore {
    inner: Arc<dyn PlayerEntityOwnershipStore>,
    busy: Arc<std::sync::atomic::AtomicBool>,
}
#[async_trait::async_trait]
impl PlayerEntityOwnershipStore for EntityBusyStore {
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
        self.inner.load_owned_pair(p, e, identity).await
    }
    async fn commit_player_entity_transfer(
        &self,
        request: PlayerEntityTransferRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.inner.commit_player_entity_transfer(request).await
    }
    async fn compare_exchange_owned_player(
        &self,
        request: PlayerOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.inner.compare_exchange_owned_player(request).await
    }
    async fn compare_exchange_owned_entity(
        &self,
        request: EntityOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        if self.busy.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(OwnershipError::Busy);
        }
        self.inner.compare_exchange_owned_entity(request).await
    }
    async fn load_ownership_receipt(
        &self,
        query: OwnershipReceiptQuery,
    ) -> OwnershipResult<Option<OwnershipReceipt>> {
        self.inner.load_ownership_receipt(query).await
    }
}

#[tokio::test]
async fn refused_connected_rejoin_before_pickup_keeps_actual_handle_and_reservation() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, -1, 5)).await;
    let read = e.binding.read_player(&e.store, p.player).await.unwrap();
    let pair = e
        .binding
        .read_pair(&e.store, p.player, uuid(1))
        .await
        .unwrap();
    let before = e.counters(1);
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::select_hotbar(p.clone(), 0))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::rejoin(read))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::pickup(
            p.clone(),
            h.clone(),
            pair,
            id(1),
            true,
            false,
        ))
        .unwrap();
    let mut out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::AlreadyOwned
    ));
    assert_eq!(out.work.len(), 1);
    assert_eq!(
        e.counters(1),
        before,
        "actual admitted pickup must freeze the original body"
    );
    assert_eq!(
        e.shard.player_ownership.as_ref().unwrap().pending[&id(1)]
            .counters
            .unwrap()
            .claims,
        1
    );
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert!(out.refusals.is_empty());
    assert!(e.shard.entities().unwrap().get(&h).is_err());
    assert_eq!(e.shard.owned_player_slots(&p).unwrap()[36].count(), 4);
}

#[tokio::test]
async fn admissible_rejoin_invalidates_old_handle_before_pickup_and_keeps_counter_eligible() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    let read = e.binding.read_player(&e.store, p.player).await.unwrap();
    let pair = e
        .binding
        .read_pair(&e.store, p.player, uuid(1))
        .await
        .unwrap();
    let age = e.counters(1).age();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::disconnect(p.clone()))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::rejoin(read))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::pickup(
            p.clone(),
            h,
            pair,
            id(1),
            true,
            false,
        ))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::StaleHandle
    ));
    assert!(out.work.is_empty());
    assert_eq!(out.counter_steps(), 1);
    assert_eq!(e.counters(1).age(), age + 1);
    assert!(e.shard.owned_player_generation(&p).is_err());
    assert!(out
        .intents
        .iter()
        .any(|i| matches!(i, OwnershipIntent::PlayerAdmitted { .. })));
}

#[tokio::test]
async fn fifo_move_then_stop_refuses_stop_but_clean_stop_claims_no_counter_step() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    e.restore(1, 1, ItemEntityCounters::new(-32768, 32767, 5))
        .await;
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::move_player(
            p,
            Some(Vec3::new(2.0, 64.0, 1.0)),
            [None; 2],
        ))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::stop())
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::NotQuiescent
    ));
    assert_eq!(out.counter_steps(), 1);
    assert!(!e.shard.player_ownership.as_ref().unwrap().stopped);
    let mut clean = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    clean
        .restore(1, 1, ItemEntityCounters::new(-32768, 32767, 5))
        .await;
    let out = clean.input(OwnedPlayerInput::stop());
    assert!(out.refusals.is_empty());
    assert_eq!(out.counter_steps(), 0);
    assert!(clean.shard.player_ownership.as_ref().unwrap().stopped);
}

#[tokio::test]
async fn actual_busy_keeps_exact_request_and_retry_publishes_all_deferred_counter_claims() {
    let busy = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut e = Env::new_with_busy(
        None,
        4,
        limits(10000, 6000, 1024),
        Tick::ZERO,
        Some(busy.clone()),
    )
    .await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let work = out.take_work().remove(0);
    let query = work.query();
    let captured = e.shard.player_ownership.as_ref().unwrap().pending[&id(1)]
        .after_entity_record
        .clone()
        .unwrap();
    busy.store(true, std::sync::atomic::Ordering::SeqCst);
    let completion = work.execute(&e.store).await.unwrap();
    // Busy refusal preserves its genuine input/lease, so a FIFO retry in
    // the same boundary must itself refuse Reserved until the caller drops it.
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::retry(query))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 2);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Storage(OwnershipError::Busy)
    ));
    assert!(matches!(
        out.refusals[1].reason,
        PlayerOwnershipError::Reserved
    ));
    assert!(out.work.is_empty());
    drop(out);
    let mut out = e.input(OwnedPlayerInput::retry(query));
    let retry = out.take_work().remove(0);
    assert_eq!(retry.query(), query);
    for _ in 0..7 {
        e.run();
    }
    assert_eq!(e.counters(1).age(), 1);
    assert!(entity_equal(
        &captured,
        e.shard.player_ownership.as_ref().unwrap().pending[&id(1)]
            .after_entity_record
            .as_ref()
            .unwrap()
    ));
    busy.store(false, std::sync::atomic::Ordering::SeqCst);
    let completion = retry.execute(&e.store).await.unwrap();
    e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(e.shard.player_ownership_pending_count(), 0);
    assert_eq!(e.counters(1).age(), e.tick.get() as i32);
    assert!(entity_equal(
        &captured,
        e.durable(1).await.record().unwrap()
    ));
}

#[tokio::test]
async fn real_cas_conflict_retains_quarantined_native_body_and_never_creates_retry_authority() {
    let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, -5, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let work = out.take_work().remove(0);
    let query = work.query();
    let current = e.durable(1).await;
    e.store
        .compare_exchange_owned_entity(
            EntityOwnershipCasRequest::new(
                id(88),
                current.expectation().clone(),
                EntityOwnershipProposal::put(native(1, 4, ItemEntityCounters::new(99, -5, 5))),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let completion = work.execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Storage(OwnershipError::CasConflict { .. })
    ));
    let pending = &e.shard.player_ownership.as_ref().unwrap().pending[&id(1)];
    assert_eq!(pending.status, OwnershipPendingStatus::Quarantined);
    assert_eq!(pending.ticket.query, query);
    assert_eq!(pending.counters.unwrap().counters.pickup_delay(), -5);
    drop(out);
    let out = e.input(OwnedPlayerInput::retry(query));
    assert!(out.work.is_empty());
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::UnprovenWorkerLifetime
    ));
    assert_eq!(e.counters(1).age(), 1);
    assert_eq!(e.shard.player_ownership_pending_count(), 1);
    let out = e.input(OwnedPlayerInput::stop());
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::NotQuiescent
    ));
}

#[tokio::test]
async fn confirmed_remove_frees_slot_for_same_boundary_restore_and_first_exact_step() {
    let mut e = Env::new(None, 1, limits(10000, 1, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    let read = e.seed(2, 4, ItemEntityCounters::new(-100, 40, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::restore_item(read, EntityId::new(2)))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let out = e.run();
    assert!(out.refusals.is_empty());
    assert_eq!(out.counter_steps(), 1);
    assert!(e.shard.entities().unwrap().get(&h).is_err());
    assert_eq!(e.counters(2), ItemEntityCounters::new(-99, 39, 5));
    assert!(e.durable(1).await.record().is_none());
}

#[tokio::test]
async fn unloaded_pending_flush_keeps_priority_after_confirmation_and_completion_only_saves() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .release_owned_chunk(ChunkPos::ORIGIN, ChunkTicket::of(TicketReason::Spawn))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::refill_item_save_ids(
            e.owner.clone(),
            vec![id(2)],
        ))
        .unwrap();
    let tick = e.tick;
    let mut out = e.shard.recover_player_ownership_completions().unwrap();
    assert!(out.refusals.is_empty());
    assert_eq!(
        out.work.len(),
        1,
        "unload priority must survive dirty defer release"
    );
    assert_eq!(e.shard.current_counter_tick(), tick);
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let out = e.shard.recover_player_ownership_completions().unwrap();
    assert!(out.refusals.is_empty());
    let d = e.shard.entities().unwrap().item_durability(&h).unwrap();
    assert_eq!(d.saved, d.generation);
    assert_eq!(d.saved_tick, tick);
}

#[tokio::test]
async fn positive_plan_count_and_byte_plus_one_refuse_before_native_restore_or_drain() {
    for byte_limit in [false, true] {
        let mut counter = limits(10000, 6000, 1024);
        if byte_limit {
            counter.plan_bytes = 1;
        } else {
            counter.plan_entries = 1;
        }
        let mut e = Env::new(None, 1, counter, Tick::ZERO).await;
        let read = e.seed(1, 1, ItemEntityCounters::new(0, 40, 5)).await;
        e.shard
            .enqueue_owned_player_input(OwnedPlayerInput::restore_item(read, EntityId::new(1)))
            .unwrap();
        assert!(matches!(
            e.shard.run_tick_with_player_ownership_at(Tick::new(1)),
            Err(PlayerOwnershipError::Capacity("counter plan"))
        ));
        assert_eq!(e.shard.player_ownership.as_ref().unwrap().input.len(), 1);
        assert_eq!(e.shard.current_counter_tick(), Tick::ZERO);
        assert!(e.shard.entities().unwrap().is_empty());
        e.shard
            .player_ownership
            .as_mut()
            .unwrap()
            .item_counters
            .as_mut()
            .unwrap()
            .limits = limits(10000, 6000, 1024);
        let out = e.run();
        assert!(out.refusals.is_empty());
        assert_eq!(e.counters(1).pickup_delay(), 39);
    }
}

#[tokio::test]
async fn exact_confirmed_body_cap_and_save_charge_plus_one_preserve_dirty_owner() {
    let body = native(1, 4, ItemEntityCounters::new(0, 40, 5))
        .record()
        .data()
        .len();
    let mut counter = limits(10000, 6000, 1024);
    counter.confirmed_bytes = body;
    counter.save_charge = 661 + 7 * body - 1;
    let mut e = Env::new(None, 2, counter, Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let read = e.seed(2, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let out = e.input(OwnedPlayerInput::restore_item(read, EntityId::new(2)));
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Capacity("confirmed item bytes")
    ));
    assert_eq!(out.counter_steps(), 1);
    assert_eq!(e.shard.entities().unwrap().len(), 1);
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Capacity("entity save charge")
    ));
    assert!(out.work.is_empty());
    assert_eq!(e.counters(1).age(), 3);
    e.shard
        .player_ownership
        .as_mut()
        .unwrap()
        .item_counters
        .as_mut()
        .unwrap()
        .limits
        .save_charge += 1;
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(2)));
    assert_eq!(out.work.len(), 1);
    assert_eq!(e.counters(1).age(), 3);
}

#[tokio::test]
async fn snapshot_checks_confirmed_aggregate_before_first_native_decode_and_rejects_corruption() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 40, 5)).await;
    let entities = e.shard.entities.as_mut().unwrap();
    let valid = entities.item_durability(&h).unwrap().confirmed.clone();
    let mut bytes = valid.record().data().to_vec();
    bytes[0] = 0;
    let corrupt = PersistentEntityRecord::new(
        valid.chunk(),
        EntityRecord::new(SchemaVersion::new(1), bytes).unwrap(),
    )
    .unwrap();
    entities.inject_confirmed_state(&h, corrupt.clone(), 1);
    assert!(matches!(
        entities.try_snapshot(),
        Err(ItemEntityError::ItemByteBudgetExceeded { limit: 1, .. })
    ));
    entities.inject_confirmed_state(&h, corrupt, MAX_CONFIRMED_ITEM_BYTES);
    assert!(matches!(
        entities.try_snapshot(),
        Err(ItemEntityError::InvalidMagic)
    ));
}

#[tokio::test]
async fn terminal_owner_generation_max_refuses_before_scratch_drain_counter_cursor_and_tick() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    e.restore(1, 1, ItemEntityCounters::new(0, 40, 5)).await;
    e.shard.player_ownership.as_mut().unwrap().generation =
        PlayerLocalGeneration(NonZeroU64::new(u64::MAX - 1).unwrap());
    let out = e.run();
    assert_eq!(out.counter_steps(), 1);
    assert_eq!(e.counters(1).age(), 2);
    e.shard.player_ownership.as_mut().unwrap().generation =
        PlayerLocalGeneration(NonZeroU64::new(u64::MAX).unwrap());
    let state = e
        .shard
        .player_ownership
        .as_ref()
        .unwrap()
        .item_counters
        .as_ref()
        .unwrap();
    let cursors = (
        state.normal_cursor,
        state.priority_cursor,
        state.selection_turn,
    );
    let scratch_capacity = state.scratch.capacity();
    let tick = e.tick;
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::drain())
        .unwrap();
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(tick.next().unwrap()),
        Err(PlayerOwnershipError::GenerationExhausted)
    ));
    let state = e.shard.player_ownership.as_ref().unwrap();
    assert_eq!(state.input.len(), 1);
    assert!(!state.draining);
    let counter = state.item_counters.as_ref().unwrap();
    assert_eq!(
        (
            counter.normal_cursor,
            counter.priority_cursor,
            counter.selection_turn
        ),
        cursors
    );
    assert_eq!(counter.scratch.capacity(), scratch_capacity);
    assert_eq!(counter.tick, tick);
    assert_eq!(e.counters(1).age(), 2);
    let out = e.shard.recover_player_ownership_completions().unwrap();
    assert!(out.refusals.is_empty());
    assert!(e.shard.player_ownership.as_ref().unwrap().draining);
    assert_eq!(e.shard.current_counter_tick(), tick);
}

#[tokio::test]
async fn max_minus_one_actual_save_admission_reaches_terminal_generation_without_false_completion()
{
    let mut e = Env::new(None, 1, limits(1, 6000, 1024), Tick::ZERO).await;
    let h = e.restore(1, 1, ItemEntityCounters::new(0, 40, 5)).await;
    e.shard.player_ownership.as_mut().unwrap().generation =
        PlayerLocalGeneration(NonZeroU64::new(u64::MAX - 1).unwrap());
    let gen = e.shard.owned_item_generation(&h).unwrap();
    let mut out = e.input(OwnedPlayerInput::save_item(h.clone(), gen, id(1)));
    assert_eq!(out.work.len(), 1);
    assert_eq!(out.counter_steps(), 1);
    assert_eq!(
        e.shard.player_ownership.as_ref().unwrap().generation.get(),
        u64::MAX
    );
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    assert!(matches!(
        e.shard.recover_player_ownership_completions(),
        Err(PlayerOwnershipError::GenerationExhausted)
    ));
    assert_eq!(e.shard.player_ownership.as_ref().unwrap().input.len(), 1);
    assert_eq!(e.shard.player_ownership_pending_count(), 1);
    assert_eq!(e.counters(1).age(), 1);
}
