//! Actual countershadow lifecycle admission, FIFO credits and quiescence.
use super::*;

async fn collide_receipt(e: &Env, transfer: OwnershipTransferId) {
    let other = PlayerId::offline("LifecycleReceiptCollision");
    let current = e
        .store
        .load_owned_player(ScopedPlayerKey::new(scope(), other), identity())
        .await
        .unwrap();
    e.store
        .compare_exchange_owned_player(
            PlayerOwnershipCasRequest::new(
                transfer,
                current.expectation().clone(),
                PlayerOwnershipProposal::put(
                    NativePlayerSnapshot::empty_survival(position(), [0.0; 2])
                        .try_to_record()
                        .unwrap(),
                ),
            )
            .unwrap(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn real_receipt_collision_abort_frees_pin_before_same_boundary_replacement_pickup() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    ack(&mut e);
    let mut out = pickup(&mut e, &p, 1).await;
    let work = out.take_work().pop().unwrap();
    collide_receipt(&e, id(1)).await;
    let completion = work.execute(&e.store).await.unwrap();
    let read = e
        .binding
        .read_pair(&e.store, p.player(), uuid(1))
        .await
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::pickup(
            p.clone(),
            e.handle(1),
            read,
            id(3),
            true,
            false,
        ))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Storage(OwnershipError::ReceiptCollision { .. })
    ));
    assert_eq!(out.work.len(), 1);
    assert_eq!(pins(&e, &p), (1, 0));
    assert_eq!(e.shard.player_ownership_pending_count(), 1);
    e.finish(out).await;
    assert_eq!(pins(&e, &p), (0, 1));
    ack(&mut e);
    assert_eq!(pins(&e, &p), (0, 0));
}

#[tokio::test]
async fn real_bootstrap_abort_at_cell_cap_frees_credit_for_same_boundary_new_bootstrap() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let mut p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    for _ in 0..3 {
        e.input(OwnedPlayerInput::disconnect(p.clone()));
        let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
        p = admission(&e.input(OwnedPlayerInput::rejoin(read)));
    }
    let fresh = PlayerId::offline("PendingLifecycleBootstrap");
    let read = e.binding.read_player(&e.store, fresh).await.unwrap();
    let mut out = e.input(OwnedPlayerInput::bootstrap(
        read,
        position(),
        [0.0; 2],
        id(1),
    ));
    let work = out.take_work().pop().unwrap();
    assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 5);
    collide_receipt(&e, id(1)).await;
    let completion = work.execute(&e.store).await.unwrap();
    let read = e.binding.read_player(&e.store, fresh).await.unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [0.0; 2],
            id(3),
        ))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert_eq!(out.work.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Storage(OwnershipError::ReceiptCollision { .. })
    ));
    assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 5);
    let out = e.finish(out).await;
    let fresh = admission(&out);
    assert!(e.shard.connected_owned_player_view(&fresh).is_ok());
    assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 5);
}

fn activate(e: &mut Env, groups: usize) {
    e.shard
        .initialize_item_publications(
            ItemPublicationLimits::new(
                nz(groups),
                nz(MAX_ITEM_PUBLICATION_RETAINED_BYTES),
                nz(MAX_ITEM_PUBLICATION_PREPARE_BYTES),
            )
            .unwrap(),
        )
        .unwrap();
    e.shard
        .initialize_collector_lifecycle(
            CollectorLifecycleLimits::new(nz(MAX_COLLECTOR_LIFECYCLE_BYTES)).unwrap(),
        )
        .unwrap();
}
fn ack(e: &mut Env) {
    let ticket = e.shard.peek_item_publication().unwrap().unwrap().ticket();
    e.shard.acknowledge_item_publication(&ticket).unwrap();
}
fn pins(e: &Env, p: &OwnedPlayerHandle) -> (usize, usize) {
    let v = e.shard.collector_lifecycle_view(p).unwrap();
    (v.reserved_take_pins(), v.retained_take_pins())
}
fn admission(out: &PlayerOwnershipTickOutcome) -> OwnedPlayerHandle {
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
async fn pickup(e: &mut Env, p: &OwnedPlayerHandle, n: u128) -> PlayerOwnershipTickOutcome {
    let read = e
        .binding
        .read_pair(&e.store, p.player(), uuid(n))
        .await
        .unwrap();
    e.input(OwnedPlayerInput::pickup(
        p.clone(),
        e.handle(n),
        read,
        id(n),
        true,
        false,
    ))
}

#[tokio::test]
async fn genuine_take_pins_at_n_minus_one_n_plus_one_and_reordered_ack_preserve_original_cells() {
    let mut e = Env::new(None, 3, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    for n in 1..=3 {
        e.restore(n, 1, ItemEntityCounters::new(-32768, 0, 5)).await;
        ack(&mut e);
    }
    let out = pickup(&mut e, &p, 1).await;
    e.finish(out).await;
    assert_eq!(pins(&e, &p), (0, 1));
    let out = pickup(&mut e, &p, 2).await;
    assert_eq!(pins(&e, &p), (1, 1));
    let mut refused = pickup(&mut e, &p, 3).await;
    assert!(refused.work.is_empty());
    assert!(!refused.take_refusals().is_empty());
    assert_eq!(pins(&e, &p), (1, 1));
    e.finish(out).await;
    let tail = e
        .shard
        .player_ownership
        .as_ref()
        .unwrap()
        .publications
        .as_ref()
        .unwrap()
        .test_tail_ticket();
    assert!(e.shard.acknowledge_item_publication(&tail).is_err());
    assert_eq!(pins(&e, &p), (0, 2));
    let out = pickup(&mut e, &p, 3).await;
    assert!(out.work.is_empty());
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Publication(ItemPublicationError::Capacity {
            budget: ItemPublicationBudget::Groups,
            ..
        }) | PlayerOwnershipError::Lifecycle(CollectorLifecycleError::PinCapacity)
    ));
    assert_eq!(e.counters(3).age(), -32768);
    ack(&mut e);
    assert_eq!(pins(&e, &p), (0, 1));
    let out = pickup(&mut e, &p, 3).await;
    assert_eq!(out.work.len(), 1);
    assert_eq!(pins(&e, &p), (1, 1));
    e.input(OwnedPlayerInput::disconnect(p.clone()));
    e.finish(out).await;
    assert_eq!(pins(&e, &p), (0, 2));
    ack(&mut e);
    ack(&mut e);
    assert_eq!(pins(&e, &p), (0, 0));
}

#[tokio::test]
async fn countershadow_caps_rejoin_history_and_actual_stop_waits_for_last_lifecycle_ack() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let mut p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    let mut history = vec![p.clone()];
    for n in 2..=5 {
        e.input(OwnedPlayerInput::disconnect(p.clone()));
        let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
        let out = e.input(OwnedPlayerInput::rejoin(read));
        assert!(out.refusals.is_empty());
        p = admission(&out);
        history.push(p.clone());
        assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), n);
    }
    e.input(OwnedPlayerInput::disconnect(p.clone()));
    let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
    let generation = e.shard.player_ownership.as_ref().unwrap().generation;
    let out = e.input(OwnedPlayerInput::rejoin(read));
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Lifecycle(CollectorLifecycleError::CellCapacity)
    ));
    assert_eq!(
        e.shard.player_ownership.as_ref().unwrap().generation,
        generation
    );
    e.input(OwnedPlayerInput::drain());
    for (i, handle) in history.iter().enumerate() {
        assert!(!e.shard.is_tick_quiescent());
        let out = e.input(OwnedPlayerInput::stop());
        assert!(!out.refusals.is_empty());
        let ticket = e
            .shard
            .collector_lifecycle_view(handle)
            .unwrap()
            .retirement_ticket()
            .unwrap();
        e.shard.acknowledge_collector_lifecycle(&ticket).unwrap();
        assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 4 - i);
    }
    assert!(e.shard.is_tick_quiescent());
    let out = e.input(OwnedPlayerInput::stop());
    assert!(out.refusals.is_empty());
    assert!(matches!(
        out.intents().last(),
        Some(OwnershipIntent::Stopped)
    ));
    let tick = e.tick;
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(tick.next().unwrap()),
        Err(PlayerOwnershipError::Draining)
    ));
}

#[tokio::test]
async fn counter_cas_quarantine_holds_original_take_pin_through_detach_and_drain() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    ack(&mut e);
    let mut out = pickup(&mut e, &p, 1).await;
    let work = out.take_work().pop().unwrap();
    let query = work.query();
    let current = e.durable(1).await;
    e.store
        .compare_exchange_owned_entity(
            EntityOwnershipCasRequest::new(
                id(88),
                current.expectation().clone(),
                EntityOwnershipProposal::put(native(1, 4, ItemEntityCounters::new(99, 0, 5))),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let completion = work.execute(&e.store).await.unwrap();
    e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(
        e.shard.player_ownership_checkpoint(query).unwrap().status(),
        OwnershipPendingStatus::Quarantined
    );
    assert_eq!(pins(&e, &p), (1, 0));
    e.input(OwnedPlayerInput::disconnect(p.clone()));
    e.input(OwnedPlayerInput::drain());
    assert!(e
        .shard
        .collector_lifecycle_view(&p)
        .unwrap()
        .retirement_ticket()
        .is_none());
    assert!(!e.shard.is_tick_quiescent());
    let out = e.input(OwnedPlayerInput::stop());
    assert!(!out.refusals.is_empty());
    assert_eq!(pins(&e, &p), (1, 0));
    assert_eq!(e.shard.owned_player_slots(&p).unwrap()[36].count(), 0);
}

#[tokio::test]
async fn quarantined_bootstrap_cell_and_scope_or_scratch_refusal_retain_authority_and_input() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let player = PlayerId::offline("LifecycleBootstrap");
    let read = e.binding.read_player(&e.store, player).await.unwrap();
    let mut out = e.input(OwnedPlayerInput::bootstrap(
        read,
        position(),
        [0.0; 2],
        id(1),
    ));
    let work = out.take_work().pop().unwrap();
    let current = e
        .store
        .load_owned_player(ScopedPlayerKey::new(scope(), player), identity())
        .await
        .unwrap();
    e.store
        .compare_exchange_owned_player(
            PlayerOwnershipCasRequest::new(
                id(2),
                current.expectation().clone(),
                PlayerOwnershipProposal::put(
                    NativePlayerSnapshot::empty_survival(position(), [0.0; 2])
                        .try_to_record()
                        .unwrap(),
                ),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let completion = work.execute(&e.store).await.unwrap();
    // Counter-mode real CAS conflicts quarantine even Bootstrap; no fake abort.
    e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 1);
    assert_eq!(e.shard.player_count(), 0);
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::drain())
        .unwrap();
    e.shard
        .player_ownership
        .as_mut()
        .unwrap()
        .item_counters
        .as_mut()
        .unwrap()
        .limits
        .plan_bytes = 1;
    let tick = e.tick;
    assert!(matches!(
        e.shard
            .run_tick_with_player_ownership_at(tick.next().unwrap()),
        Err(PlayerOwnershipError::Capacity("counter plan"))
    ));
    assert_eq!(e.shard.player_ownership.as_ref().unwrap().input.len(), 1);
    assert_eq!(e.shard.current_counter_tick(), tick);
    let changed = GenerationBinding::new(
        scope(),
        GenerationIdentity::flat_overworld(WorldSeed::new(38)),
    );
    e.shard.chunks = LoadedChunkMap::with_generation(changed);
    assert!(matches!(
        e.shard.collector_lifecycle_status(),
        Err(PlayerOwnershipError::ScopeMismatch)
    ));
    assert_eq!(
        e.shard
            .player_ownership
            .as_ref()
            .unwrap()
            .lifecycle
            .as_ref()
            .unwrap()
            .credits
            .cells,
        1
    );
}
