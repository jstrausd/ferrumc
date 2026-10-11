//! Genuine native admissions and Work/Completion paths; no arbitrary fact factory.
use super::*;

#[tokio::test]
async fn genuine_partial_pickup_takes_one_pin_only_for_animation_and_retains_changed_body() {
    for redb in [false, true] {
        for animate in [false, true] {
            let mut e = Env::new(redb).await;
            activate(&mut e, 1);
            let mut slots = filled(64);
            slots[36] = stone(63);
            let p = e.seed_player(slots, 0).await;
            let item = e.spawn(5, 0).await;
            ack_head(&mut e);
            let pair = e.pair(uuid(1)).await;
            let out = e.tick(OwnedPlayerInput::pickup(
                p.clone(),
                item,
                pair,
                id(1),
                animate,
                false,
            ));
            assert_eq!(pins(&e, &p), (usize::from(animate), 0));
            e.tick(OwnedPlayerInput::disconnect(p.clone()));
            e.finish(out).await;
            assert_eq!(pins(&e, &p), (0, usize::from(animate)));
            let group = e.shard.peek_item_publication().unwrap().unwrap();
            assert_eq!(group.kind(), ItemPublicationKind::Changed);
            assert_eq!(group.postimage().unwrap().stack().count(), 4);
            if animate {
                assert_eq!(group.take().unwrap().original_count(), 5);
                assert_eq!(group.take().unwrap().moved(), 1);
                assert!(e
                    .shard
                    .collector_lifecycle_view(&p)
                    .unwrap()
                    .retirement_ticket()
                    .is_none());
            } else {
                assert!(group.take().is_none());
                assert!(e
                    .shard
                    .collector_lifecycle_view(&p)
                    .unwrap()
                    .retirement_ticket()
                    .is_some());
            }
            ack_head(&mut e);
            assert_eq!(pins(&e, &p), (0, 0));
            assert_eq!(e.count(&p, 36), 64);
        }
    }
}

fn activate(env: &mut Env, groups: usize) {
    env.shard
        .initialize_item_publications(
            ItemPublicationLimits::new(
                nz(groups),
                nz(MAX_ITEM_PUBLICATION_RETAINED_BYTES),
                nz(MAX_ITEM_PUBLICATION_PREPARE_BYTES),
            )
            .unwrap(),
        )
        .unwrap();
    env.shard
        .initialize_collector_lifecycle(
            CollectorLifecycleLimits::new(nz(MAX_COLLECTOR_LIFECYCLE_BYTES)).unwrap(),
        )
        .unwrap();
}
fn ack_head(env: &mut Env) {
    let ticket = env.shard.peek_item_publication().unwrap().unwrap().ticket();
    env.shard.acknowledge_item_publication(&ticket).unwrap();
}
fn pins(env: &Env, p: &OwnedPlayerHandle) -> (usize, usize) {
    let v = env.shard.collector_lifecycle_view(p).unwrap();
    (v.reserved_take_pins(), v.retained_take_pins())
}
fn admitted(out: &PlayerOwnershipTickOutcome) -> OwnedPlayerHandle {
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
async fn rejoin(env: &mut Env, p: &OwnedPlayerHandle) -> OwnedPlayerHandle {
    let read = env
        .binding
        .read_player(&env.store, p.player())
        .await
        .unwrap();
    let mut out = env.tick(OwnedPlayerInput::rejoin(read));
    assert!(out.take_refusals().is_empty());
    admitted(&out)
}

#[tokio::test]
async fn bootstrap_and_restored_views_borrow_actual_mode_pose_and_seed_on_both_backends() {
    for redb in [false, true] {
        let mut e = Env::new(redb).await;
        activate(&mut e, 2);
        let read = e.binding.read_player(&e.store, player()).await.unwrap();
        let pose = Vec3::new(-0.0, 65.25, 2.125);
        let look = [-0.0, 271.5];
        let out = e.tick(OwnedPlayerInput::bootstrap(read, pose, look, id(1)));
        assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 1);
        let out = e.finish(out).await;
        let p = admitted(&out);
        let v = e.shard.connected_owned_player_view(&p).unwrap();
        assert_eq!(v.player(), player());
        assert!(v.matches_owned_player(&p));
        assert_eq!(v.generation(), e.binding.generation());
        assert_eq!(v.game_mode(), GameMode::Survival);
        assert_eq!(v.position().x.to_bits(), pose.x.to_bits());
        assert_eq!(v.position().y.to_bits(), pose.y.to_bits());
        assert_eq!(v.position().z.to_bits(), pose.z.to_bits());
        assert_eq!(v.look().map(f32::to_bits), look.map(f32::to_bits));
        // Independent SHA256 fixture of signed37 little-endian bytes.
        assert_eq!(v.hashed_seed(), 8_828_412_754_801_553_711_i64);
        let newpose = Vec3::new(4.125, 66.5, -0.0);
        e.tick(OwnedPlayerInput::move_player(
            p.clone(),
            Some(newpose),
            [Some(451.25), Some(-0.0)],
        ));
        e.tick(OwnedPlayerInput::set_game_mode(
            p.clone(),
            GameMode::Creative,
        ));
        let v = e.shard.connected_owned_player_view(&p).unwrap();
        assert_eq!(v.game_mode(), GameMode::Creative);
        assert_eq!(v.position().z.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(
            v.look().map(f32::to_bits),
            [451.25_f32.to_bits(), (-0.0_f32).to_bits()]
        );
        let out = e.tick(OwnedPlayerInput::autosave(p.clone(), id(2)));
        e.finish(out).await;
        e.tick(OwnedPlayerInput::disconnect(p.clone()));
        let newer = rejoin(&mut e, &p).await;
        let v = e.shard.connected_owned_player_view(&newer).unwrap();
        assert_eq!(v.game_mode(), GameMode::Creative);
        assert_eq!(v.position().x.to_bits(), newpose.x.to_bits());
        assert_eq!(
            v.look().map(f32::to_bits),
            [451.25_f32.to_bits(), (-0.0_f32).to_bits()]
        );
    }
}

#[tokio::test]
async fn foreign_same_uuid_snapshot_and_stale_views_refuse_while_drain_preserves_connected_view() {
    let mut a = Env::new(false).await;
    let mut b = Env::new(false).await;
    activate(&mut a, 1);
    activate(&mut b, 1);
    let pa = a.seed_player(empty(), 0).await;
    let pb = b.seed_player(empty(), 0).await;
    assert_eq!(pa.player(), pb.player());
    assert!(matches!(
        a.shard.connected_owned_player_view(&pb),
        Err(PlayerOwnershipError::StaleHandle)
    ));
    assert!(!a
        .shard
        .connected_owned_player_view(&pa)
        .unwrap()
        .matches_owned_player(&pb));
    assert!(matches!(
        a.shard.collector_lifecycle_view(&pb),
        Err(PlayerOwnershipError::StaleHandle)
    ));
    let mut snapshot = a.shard.clone();
    assert!(matches!(
        snapshot.connected_owned_player_view(&pa),
        Err(PlayerOwnershipError::SnapshotOnly)
    ));
    assert!(matches!(
        snapshot.collector_lifecycle_view(&pa),
        Err(PlayerOwnershipError::SnapshotOnly)
    ));
    assert!(matches!(
        snapshot.initialize_collector_lifecycle(CollectorLifecycleLimits::new(nz(1024)).unwrap()),
        Err(PlayerOwnershipError::SnapshotOnly)
    ));
    a.tick(OwnedPlayerInput::drain());
    assert!(a.shard.connected_owned_player_view(&pa).is_ok());
    a.tick(OwnedPlayerInput::disconnect(pa.clone()));
    let ticket = a
        .shard
        .collector_lifecycle_view(&pa)
        .unwrap()
        .retirement_ticket()
        .unwrap();
    assert!(matches!(
        snapshot.acknowledge_collector_lifecycle(&ticket),
        Err(PlayerOwnershipError::SnapshotOnly)
    ));
    a.shard.acknowledge_collector_lifecycle(&ticket).unwrap();
    a.tick(OwnedPlayerInput::stop());
    assert!(matches!(
        a.shard.connected_owned_player_view(&pa),
        Err(PlayerOwnershipError::Draining)
    ));
}

#[tokio::test]
async fn applied_detach_is_exact_idempotent_and_historical_disconnect_never_mutates_rejoin() {
    for redb in [false, true] {
        let mut e = Env::new(redb).await;
        activate(&mut e, 1);
        let p = e.seed_player(empty(), 0).await;
        e.shard
            .enqueue_owned_player_input(OwnedPlayerInput::disconnect(p.clone()))
            .unwrap();
        assert!(e
            .shard
            .collector_lifecycle_view(&p)
            .unwrap()
            .detach()
            .is_none());
        assert!(e.shard.connected_owned_player_view(&p).is_ok());
        e.shard.run_tick_with_player_ownership().unwrap();
        let address = {
            let v = e.shard.collector_lifecycle_view(&p).unwrap();
            let fact = v.detach().unwrap();
            assert!(fact.matches_owned_player(&p));
            assert_eq!(fact.player(), p.player());
            assert_eq!(fact.generation(), e.binding.generation());
            assert!(v.live().is_none());
            std::ptr::from_ref(fact)
        };
        e.tick(OwnedPlayerInput::disconnect(p.clone()));
        assert_eq!(
            std::ptr::from_ref(
                e.shard
                    .collector_lifecycle_view(&p)
                    .unwrap()
                    .detach()
                    .unwrap()
            ),
            address
        );
        let newer = rejoin(&mut e, &p).await;
        assert!(matches!(
            e.shard.connected_owned_player_view(&p),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        assert!(!e
            .shard
            .collector_lifecycle_view(&p)
            .unwrap()
            .detach()
            .unwrap()
            .matches_owned_player(&newer));
        assert!(e
            .shard
            .collector_lifecycle_view(&newer)
            .unwrap()
            .detach()
            .is_none());
        let mut out = e.tick(OwnedPlayerInput::disconnect(p.clone()));
        assert!(out.take_refusals().is_empty());
        assert!(e.shard.connected_owned_player_view(&newer).is_ok());
        assert_eq!(
            std::ptr::from_ref(
                e.shard
                    .collector_lifecycle_view(&p)
                    .unwrap()
                    .detach()
                    .unwrap()
            ),
            address
        );
    }
}

#[tokio::test]
async fn reserved_pickup_detach_confirm_rejoin_and_fifo_ack_release_original_pin_once() {
    for redb in [false, true] {
        let mut e = Env::new(redb).await;
        activate(&mut e, 2);
        let p = e.seed_player(empty(), 0).await;
        let item = e.spawn(5, 0).await;
        let spawn_ticket = e.shard.peek_item_publication().unwrap().unwrap().ticket();
        let pair = e.pair(uuid(1)).await;
        let out = e.tick(OwnedPlayerInput::pickup(
            p.clone(),
            item,
            pair,
            id(1),
            true,
            false,
        ));
        assert_eq!(pins(&e, &p), (1, 0));
        e.tick(OwnedPlayerInput::disconnect(p.clone()));
        assert!(e
            .shard
            .collector_lifecycle_view(&p)
            .unwrap()
            .retirement_ticket()
            .is_none());
        e.finish(out).await;
        assert_eq!(pins(&e, &p), (0, 1));
        let newer = rejoin(&mut e, &p).await;
        assert_eq!(pins(&e, &newer), (0, 0));
        assert_eq!(pins(&e, &p), (0, 1));
        e.shard.acknowledge_item_publication(&spawn_ticket).unwrap();
        let group = e.shard.peek_item_publication().unwrap().unwrap();
        assert_eq!(group.kind(), ItemPublicationKind::Removed);
        assert_eq!(group.take().unwrap().original_count(), 5);
        assert!(group.take().unwrap().collector().matches_owned_player(&p));
        assert!(!group
            .take()
            .unwrap()
            .collector()
            .matches_owned_player(&newer));
        let ticket = group.ticket();
        assert!(e.shard.acknowledge_item_publication(&spawn_ticket).is_err());
        assert_eq!(pins(&e, &p), (0, 1));
        e.tick(OwnedPlayerInput::drain());
        e.shard.acknowledge_item_publication(&ticket).unwrap();
        assert_eq!(pins(&e, &p), (0, 0));
        assert!(e.shard.acknowledge_item_publication(&ticket).is_err());
        let life = e
            .shard
            .collector_lifecycle_view(&p)
            .unwrap()
            .retirement_ticket()
            .unwrap();
        e.shard.acknowledge_collector_lifecycle(&life).unwrap();
        assert!(e.shard.acknowledge_collector_lifecycle(&life).is_err());
        assert!(e.shard.connected_owned_player_view(&newer).is_ok());
        let mut blocked = e.tick(OwnedPlayerInput::stop());
        assert!(matches!(
            blocked.take_refusals().pop().unwrap().reason(),
            PlayerOwnershipError::NotQuiescent
        ));
        e.tick(OwnedPlayerInput::disconnect(newer.clone()));
        let life = e
            .shard
            .collector_lifecycle_view(&newer)
            .unwrap()
            .retirement_ticket()
            .unwrap();
        e.shard.acknowledge_collector_lifecycle(&life).unwrap();
        let mut out = e.tick(OwnedPlayerInput::stop());
        assert!(out.take_refusals().is_empty());
        assert!(matches!(
            out.intents().last(),
            Some(OwnershipIntent::Stopped)
        ));
    }
}

#[tokio::test]
async fn lifecycle_tickets_refuse_foreign_duplicates_and_reused_cells_without_retargeting() {
    let mut a = Env::new(false).await;
    let mut b = Env::new(false).await;
    activate(&mut a, 1);
    activate(&mut b, 1);
    let pa = a.seed_player(empty(), 0).await;
    let pb = b.seed_player(empty(), 0).await;
    a.tick(OwnedPlayerInput::disconnect(pa.clone()));
    b.tick(OwnedPlayerInput::disconnect(pb.clone()));
    let old = a
        .shard
        .collector_lifecycle_view(&pa)
        .unwrap()
        .retirement_ticket()
        .unwrap();
    assert!(b.shard.acknowledge_collector_lifecycle(&old).is_err());
    assert_eq!(b.shard.collector_lifecycle_status().unwrap().cells(), 1);
    a.shard.acknowledge_collector_lifecycle(&old).unwrap();
    let new = rejoin(&mut a, &pa).await;
    a.tick(OwnedPlayerInput::disconnect(new.clone()));
    assert!(a.shard.acknowledge_collector_lifecycle(&old).is_err());
    let current = a
        .shard
        .collector_lifecycle_view(&new)
        .unwrap()
        .retirement_ticket()
        .unwrap();
    a.shard.acknowledge_collector_lifecycle(&current).unwrap();
    assert_eq!(a.shard.collector_lifecycle_status().unwrap().cells(), 0);
}

#[tokio::test]
async fn exact_byte_count_boundaries_and_pinless_rejoin_history_refuse_before_admission() {
    assert!(CollectorLifecycleLimits::new(nz(MAX_COLLECTOR_LIFECYCLE_BYTES + 1)).is_err());
    let mut reference = Env::new(false).await;
    activate(&mut reference, 1);
    let status = reference.shard.collector_lifecycle_status().unwrap();
    assert_eq!(status.cell_limit(), 4); // actual3 player limit + actual1 group limit
    let bytes = status.scalar_bytes();
    for cap in [bytes - 1, bytes, bytes + 1] {
        let mut e = Env::new(false).await;
        e.shard
            .initialize_item_publications(
                ItemPublicationLimits::new(nz(1), nz(1024), nz(1024)).unwrap(),
            )
            .unwrap();
        let result = e
            .shard
            .initialize_collector_lifecycle(CollectorLifecycleLimits::new(nz(cap)).unwrap());
        assert_eq!(result.is_ok(), cap >= bytes);
        if cap < bytes {
            assert!(matches!(
                result,
                Err(PlayerOwnershipError::Lifecycle(
                    CollectorLifecycleError::ByteCapacity { .. }
                ))
            ));
        }
    }
    let mut p = reference.seed_player(empty(), 0).await;
    let mut history = vec![p.clone()];
    for n in 2..=4 {
        reference.tick(OwnedPlayerInput::disconnect(p.clone()));
        p = rejoin(&mut reference, &p).await;
        history.push(p.clone());
        assert_eq!(
            reference
                .shard
                .collector_lifecycle_status()
                .unwrap()
                .cells(),
            n
        );
    }
    reference.tick(OwnedPlayerInput::disconnect(p.clone()));
    let read = reference
        .binding
        .read_player(&reference.store, p.player())
        .await
        .unwrap();
    let mut out = reference.tick(OwnedPlayerInput::rejoin(read));
    assert!(matches!(
        out.take_refusals().pop().unwrap().reason(),
        PlayerOwnershipError::Lifecycle(CollectorLifecycleError::CellCapacity)
    ));
    assert_eq!(
        reference
            .shard
            .collector_lifecycle_status()
            .unwrap()
            .cells(),
        4
    );
    let old = reference
        .shard
        .collector_lifecycle_view(&history[0])
        .unwrap()
        .retirement_ticket()
        .unwrap();
    reference
        .shard
        .acknowledge_collector_lifecycle(&old)
        .unwrap();
    let new = rejoin(&mut reference, &p).await;
    assert!(reference.shard.connected_owned_player_view(&new).is_ok());
    assert!(reference
        .shard
        .acknowledge_collector_lifecycle(&old)
        .is_err());
}

#[tokio::test]
async fn busy_refresh_commit_uncertainty_and_receipt_absence_keep_original_pin_and_cell() {
    for redb in [false, true] {
        for mode in [1, 2, 3, 4] {
            let mut e = Env::new(redb).await;
            let fault = Arc::new(FaultStore::new(e.store.clone()));
            e.bind_override(fault.clone()).await;
            activate(&mut e, 1);
            let p = e.seed_player(empty(), 0).await;
            let item = e.spawn(5, 0).await;
            ack_head(&mut e);
            let pair = e.pair(uuid(1)).await;
            let mut out = e.tick(OwnedPlayerInput::pickup(
                p.clone(),
                item,
                pair,
                id(1),
                true,
                false,
            ));
            let work = out.take_work().pop().unwrap();
            let query = work.query();
            e.tick(OwnedPlayerInput::disconnect(p.clone()));
            fault.fault(mode);
            let completion = work.execute(&e.store).await.unwrap();
            let mut refused = e.tick(OwnedPlayerInput::complete(completion));
            assert_eq!(pins(&e, &p), (1, 0));
            assert!(e
                .shard
                .collector_lifecycle_view(&p)
                .unwrap()
                .retirement_ticket()
                .is_none());
            let mut blocked = e.tick(OwnedPlayerInput::stop());
            assert!(!blocked.take_refusals().is_empty());
            refused.take_refusals();
            drop(refused);
            fault.fault(0);
            let out = e.tick(if mode == 3 {
                OwnedPlayerInput::retry(query)
            } else {
                OwnedPlayerInput::reconcile(query)
            });
            let mut out = e.finish(out).await;
            if mode == 4 {
                assert!(!out.take_refusals().is_empty());
                assert_eq!(pins(&e, &p), (1, 0));
            } else {
                assert_eq!(pins(&e, &p), (0, 1));
                ack_head(&mut e);
                assert_eq!(pins(&e, &p), (0, 0));
            }
        }
    }
}

#[tokio::test]
async fn definite_real_cas_abort_releases_reserved_pin_once_and_preserves_inventory() {
    for redb in [false, true] {
        let mut e = Env::new(redb).await;
        activate(&mut e, 1);
        let p = e.seed_player(empty(), 0).await;
        let item = e.spawn(5, 0).await;
        ack_head(&mut e);
        let pair = e.pair(uuid(1)).await;
        let mut out = e.tick(OwnedPlayerInput::pickup(
            p.clone(),
            item.clone(),
            pair,
            id(1),
            true,
            false,
        ));
        let work = out.take_work().pop().unwrap();
        let query = work.query();
        let current = e
            .store
            .load_owned_entity(
                PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(1)),
                identity(),
            )
            .await
            .unwrap();
        e.store
            .compare_exchange_owned_entity(
                EntityOwnershipCasRequest::new(
                    id(2),
                    current.expectation().clone(),
                    EntityOwnershipProposal::delete(),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let completion = work.execute(&e.store).await.unwrap();
        e.tick(OwnedPlayerInput::disconnect(p.clone()));
        e.tick(OwnedPlayerInput::complete(completion));
        assert_eq!(pins(&e, &p), (0, 0));
        assert_eq!(e.count(&p, 36), 0);
        assert!(e.shard.entities().unwrap().get(&item).is_ok());
        assert_eq!(e.shard.player_ownership_pending_count(), 0);
        let mut retry = e.tick(OwnedPlayerInput::retry(query));
        assert!(!retry.take_refusals().is_empty());
        assert_eq!(pins(&e, &p), (0, 0));
        let ticket = e
            .shard
            .collector_lifecycle_view(&p)
            .unwrap()
            .retirement_ticket()
            .unwrap();
        e.shard.acknowledge_collector_lifecycle(&ticket).unwrap();
    }
}

#[tokio::test]
async fn bootstrap_refusal_releases_cell_but_lost_postcommit_reply_retains_until_real_confirm() {
    for uncertain in [false, true] {
        let mut e = Env::new(false).await;
        let fault = Arc::new(FaultStore::new(e.store.clone()));
        e.bind_override(fault.clone()).await;
        activate(&mut e, 1);
        let read = e.binding.read_player(&e.store, player()).await.unwrap();
        let mut out = e.tick(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [0.0; 2],
            id(1),
        ));
        let work = out.take_work().pop().unwrap();
        let query = work.query();
        assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 1);
        if uncertain {
            fault.fault(2);
        } else {
            let current = fault
                .inner
                .load_owned_player(ScopedPlayerKey::new(scope(), player()), identity())
                .await
                .unwrap();
            fault
                .inner
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
        }
        let completion = work.execute(&e.store).await.unwrap();
        e.tick(OwnedPlayerInput::complete(completion));
        assert_eq!(
            e.shard.collector_lifecycle_status().unwrap().cells(),
            usize::from(uncertain)
        );
        assert_eq!(e.shard.player_count(), 0);
        if uncertain {
            fault.fault(0);
            let out = e.tick(OwnedPlayerInput::reconcile(query));
            let out = e.finish(out).await;
            let p = admitted(&out);
            assert!(e
                .shard
                .collector_lifecycle_view(&p)
                .unwrap()
                .live()
                .is_some());
            assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), 1);
        }
    }
}

#[tokio::test]
async fn lifecycle_initialization_refuses_existing_admissions_inbox_pending_and_drain() {
    let config = || CollectorLifecycleLimits::new(nz(MAX_COLLECTOR_LIFECYCLE_BYTES)).unwrap();
    let mut missing = Env::new(false).await;
    assert!(matches!(
        missing.shard.initialize_collector_lifecycle(config()),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::NotInitialized
        ))
    ));
    for kind in 0..4 {
        let mut e = Env::new(false).await;
        e.shard
            .initialize_item_publications(
                ItemPublicationLimits::new(nz(1), nz(1024), nz(1024)).unwrap(),
            )
            .unwrap();
        match kind {
            0 => {
                e.seed_player(empty(), 0).await;
            }
            1 => {
                e.shard
                    .enqueue_owned_player_input(OwnedPlayerInput::drain())
                    .unwrap();
            }
            2 => {
                let read = e.binding.read_player(&e.store, player()).await.unwrap();
                e.tick(OwnedPlayerInput::bootstrap(
                    read,
                    position(),
                    [0.0; 2],
                    id(1),
                ));
            }
            _ => {
                e.tick(OwnedPlayerInput::drain());
            }
        }
        assert!(e.shard.initialize_collector_lifecycle(config()).is_err());
    }
    let mut e = Env::new(false).await;
    activate(&mut e, 1);
    assert!(matches!(
        e.shard.initialize_collector_lifecycle(config()),
        Err(PlayerOwnershipError::Lifecycle(
            CollectorLifecycleError::AlreadyInitialized
        ))
    ));
}

#[tokio::test]
async fn superseded_real_receipt_quarantines_take_pin_without_releasing_lifecycle_or_inventory() {
    for redb in [false, true] {
        let mut e = Env::new(redb).await;
        let fault = Arc::new(FaultStore::new(e.store.clone()));
        e.bind_override(fault.clone()).await;
        activate(&mut e, 1);
        let p = e.seed_player(empty(), 0).await;
        let item = e.spawn(5, 0).await;
        ack_head(&mut e);
        let pair = e.pair(uuid(1)).await;
        let mut out = e.tick(OwnedPlayerInput::pickup(
            p.clone(),
            item,
            pair,
            id(1),
            true,
            false,
        ));
        let work = out.take_work().pop().unwrap();
        let query = work.query();
        fault.fault(2);
        let completion = work.execute(&e.store).await.unwrap();
        e.tick(OwnedPlayerInput::complete(completion));
        let current = fault
            .inner
            .load_owned_player(ScopedPlayerKey::new(scope(), player()), identity())
            .await
            .unwrap();
        fault
            .inner
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
        fault.fault(0);
        let out = e.tick(OwnedPlayerInput::reconcile(query));
        e.finish(out).await;
        assert_eq!(
            e.shard.player_ownership_checkpoint(query).unwrap().status(),
            OwnershipPendingStatus::Quarantined
        );
        assert_eq!(pins(&e, &p), (1, 0));
        assert_eq!(e.count(&p, 36), 0);
        e.tick(OwnedPlayerInput::disconnect(p.clone()));
        e.tick(OwnedPlayerInput::drain());
        assert!(e
            .shard
            .collector_lifecycle_view(&p)
            .unwrap()
            .retirement_ticket()
            .is_none());
        let mut out = e.tick(OwnedPlayerInput::stop());
        assert!(!out.take_refusals().is_empty());
        assert_eq!(pins(&e, &p), (1, 0));
    }
}
