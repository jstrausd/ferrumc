//! Real ownership admission/receipt paths for the opt-in retained item producer.

use super::*;

fn policy(groups: usize, bytes: usize, prepare: usize) -> ItemPublicationLimits {
    ItemPublicationLimits::new(nz(groups), nz(bytes), nz(prepare)).unwrap()
}

fn activate(e: &mut Env, groups: usize) {
    e.shard
        .initialize_item_publications(policy(groups, 1024 * 1024, 1024 * 1024))
        .unwrap();
}

fn head(e: &Env) -> &ItemPublicationGroup {
    e.shard.peek_item_publication().unwrap().unwrap()
}

fn ack(e: &mut Env) {
    let ticket = head(e).ticket();
    e.shard.acknowledge_item_publication(&ticket).unwrap();
}

fn proposal(n: u128, count: u8, counters: ItemEntityCounters) -> ItemEntitySpawn {
    let native = decode_item_entity(
        PersistentEntityKey::new(scope().world(), scope().dimension(), uuid(n)),
        &native(n, count, counters),
    )
    .unwrap();
    let (key, kind, transform, stack, properties) = native.into_parts();
    ItemEntitySpawn::new(
        EntitySpawn::new(
            key.world(),
            key.dimension(),
            key.uuid(),
            EntityId::new(n as i32),
            kind,
            transform,
        ),
        stack,
        properties,
    )
}

async fn spawn(e: &mut Env, n: u128, count: u8) -> PlayerOwnershipTickOutcome {
    let read = e.binding.read_entity(&e.store, uuid(n)).await.unwrap();
    e.input(OwnedPlayerInput::spawn_item(
        read,
        proposal(n, count, ItemEntityCounters::new(0, 0, 5)),
        id(n),
    ))
}

#[tokio::test]
async fn activation_is_once_only_empty_actual_owner_and_snapshot_cannot_produce_or_ack() {
    let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    assert!(matches!(
        e.shard.peek_item_publication(),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::NotInitialized
        ))
    ));
    activate(&mut e, 2);
    assert!(matches!(
        e.shard.initialize_item_publications(policy(2, 1024, 1024)),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::AlreadyInitialized
        ))
    ));
    e.restore(1, 1, ItemEntityCounters::new(-32768, 32767, 5))
        .await;
    let ticket = head(&e).ticket();
    let mut snapshot = e.shard.clone();
    assert!(matches!(
        snapshot.peek_item_publication(),
        Err(PlayerOwnershipError::SnapshotOnly)
    ));
    assert!(matches!(
        snapshot.acknowledge_item_publication(&ticket),
        Err(PlayerOwnershipError::SnapshotOnly)
    ));
    assert!(matches!(
        snapshot.initialize_item_publications(policy(2, 1024, 1024)),
        Err(PlayerOwnershipError::SnapshotOnly)
    ));
    assert!(snapshot
        .player_ownership
        .as_ref()
        .unwrap()
        .publications
        .is_none());
    assert_eq!(
        e.shard.item_publication_status().unwrap().retained_groups(),
        1
    );
    let mut nonempty = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    nonempty
        .restore(1, 1, ItemEntityCounters::new(-32768, 32767, 5))
        .await;
    assert!(matches!(
        nonempty
            .shard
            .initialize_item_publications(policy(1, 1024, 1024)),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::NotEmpty
        ))
    ));
}

#[tokio::test]
async fn spawn_waits_for_real_receipt_and_retained_postimage_survives_later_removal() {
    let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    let out = spawn(&mut e, 1, 4).await;
    assert_eq!(out.work.len(), 1);
    assert!(e.shard.peek_item_publication().unwrap().is_none());
    let status = e.shard.item_publication_status().unwrap();
    assert_eq!(
        (
            status.reserved_groups(),
            status.retained_groups(),
            status.last_published()
        ),
        (1, 0, 0)
    );
    e.finish(out).await;
    assert_eq!(head(&e).kind(), ItemPublicationKind::Spawn);
    assert_eq!(
        head(&e).origin().kind(),
        ItemPublicationOriginKind::ConfirmedReceipt
    );
    assert!(head(&e).origin().receipt_query().is_some());
    let instance = head(&e).instance().clone();
    let body = encode_persisted_stack(head(&e).postimage().unwrap().stack()).unwrap();
    let initial = head(&e).postimage().unwrap().properties();
    let h = e.handle(1);
    for _ in 0..3 {
        e.run();
    }
    let pair = e
        .binding
        .read_pair(&e.store, p.player(), uuid(1))
        .await
        .unwrap();
    let out = e.input(OwnedPlayerInput::pickup(
        p.clone(),
        h.clone(),
        pair,
        id(2),
        true,
        false,
    ));
    e.finish(out).await;
    assert!(e.shard.entities().unwrap().get(&h).is_err());
    assert_eq!(head(&e).instance(), &instance);
    assert_eq!(head(&e).postimage().unwrap().properties(), initial);
    assert_eq!(
        encode_persisted_stack(head(&e).postimage().unwrap().stack()).unwrap(),
        body
    );
    assert_eq!(head(&e).sequence().get(), 1);
    ack(&mut e);
    assert_eq!(head(&e).kind(), ItemPublicationKind::Removed);
    assert_eq!(head(&e).instance(), &instance);
    assert!(head(&e).postimage().is_none());
    assert_eq!(head(&e).sequence().get(), 2);
    assert_eq!(head(&e).take().unwrap().original_count(), 4);
    assert_eq!(head(&e).take().unwrap().moved(), 4);
}

#[tokio::test]
async fn restore_captures_native_origin_and_cutpoint_before_first_counter_step() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    e.restore(1, 7, ItemEntityCounters::new(12, 40, -5)).await;
    let group = head(&e);
    assert_eq!(group.kind(), ItemPublicationKind::Restore);
    assert_eq!(
        group.origin().kind(),
        ItemPublicationOriginKind::NativeRestore
    );
    assert!(group.origin().receipt_query().is_none());
    assert_eq!(group.origin().stamp(), e.durable(1).await.stamp());
    assert_eq!(
        group.postimage().unwrap().properties().counters(),
        ItemEntityCounters::new(12, 40, -5)
    );
    assert_eq!(e.counters(1), ItemEntityCounters::new(13, 39, -5));
    assert_eq!(
        group.encoded_charge(),
        ITEM_PUBLICATION_SCALAR_CHARGE + ferrumc_items::persisted_stack_size(&stone(7)).unwrap()
    );
    assert_eq!(group.instance().generation(), e.binding.generation());
    assert_eq!(
        group.postimage().unwrap().transform().position(),
        position()
    );
}

#[tokio::test]
async fn partial_and_full_pickup_capture_original_admission_and_actual_confirm_counter_cutpoint() {
    for partial in [false, true] {
        let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
        activate(&mut e, 2);
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
        let p = e.player(slots).await;
        let h = e.restore(1, 4, ItemEntityCounters::new(0, -1, 5)).await;
        ack(&mut e);
        let pair = e
            .binding
            .read_pair(&e.store, p.player(), uuid(1))
            .await
            .unwrap();
        let out = e.input(OwnedPlayerInput::pickup(
            p.clone(),
            h,
            pair,
            id(1),
            true,
            false,
        ));
        assert!(e.shard.peek_item_publication().unwrap().is_none());
        let admitted_count = e.counters(1).age();
        for _ in 0..3 {
            e.run();
        }
        e.finish(out).await;
        let group = head(&e);
        assert_eq!(
            group.kind(),
            if partial {
                ItemPublicationKind::Changed
            } else {
                ItemPublicationKind::Removed
            }
        );
        let take = group.take().unwrap();
        assert_eq!(
            (take.original_count(), take.moved()),
            (4, if partial { 1 } else { 4 })
        );
        assert!(take.collector().matches_owned_player(&p));
        assert_eq!(take.collector().player(), p.player());
        assert_eq!(take.collector().generation(), e.binding.generation());
        if partial {
            let postimage = group.postimage().unwrap();
            assert_eq!(postimage.stack().count(), 3);
            assert_eq!(postimage.properties().counters().pickup_delay(), 0);
            assert_eq!(postimage.properties().counters().age(), admitted_count + 4);
            assert_eq!(
                e.counters(1).age(),
                postimage.properties().counters().age() + 1
            );
        } else {
            assert!(group.postimage().is_none());
        }
        let collector = take.collector().clone();
        e.input(OwnedPlayerInput::disconnect(p.clone()));
        // The helper compares captured admissions, not connectivity.
        assert!(collector.matches_owned_player(&p));
        let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
        let out = e.input(OwnedPlayerInput::rejoin(read));
        let joined = out
            .intents()
            .iter()
            .find_map(|intent| {
                if let OwnershipIntent::PlayerAdmitted { player, .. } = intent {
                    Some(player.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert!(!collector.matches_owned_player(&joined));
        assert!(collector.matches_owned_player(&p));
        assert_eq!(head(&e).take().unwrap().collector(), &collector);
        let mut foreign = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
        let other = foreign
            .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
            .await;
        assert_eq!(other.player(), p.player());
        assert_eq!(other.admission, collector.admission());
        assert!(!collector.matches_owned_player(&other));
    }
}

#[tokio::test]
async fn selected_drop_emits_spawn_without_take_and_only_after_joint_confirmation() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let mut slots = Box::new(std::array::from_fn(|_| ItemStack::empty()));
    slots[36] = stone(9);
    let p = e.player(slots).await;
    let pair = e
        .binding
        .read_pair(&e.store, p.player(), uuid(1))
        .await
        .unwrap();
    let out = e.input(OwnedPlayerInput::drop_selected(
        p.clone(),
        pair,
        DropSelection::One,
        EntityId::new(1),
        id(1),
        23,
        false,
    ));
    assert_eq!(e.shard.owned_player_slots(&p).unwrap()[36].count(), 9);
    assert!(e.shard.peek_item_publication().unwrap().is_none());
    e.finish(out).await;
    assert_eq!(e.shard.owned_player_slots(&p).unwrap()[36].count(), 8);
    assert_eq!(head(&e).kind(), ItemPublicationKind::Spawn);
    assert_eq!(head(&e).postimage().unwrap().stack().count(), 1);
    assert_eq!(
        head(&e)
            .postimage()
            .unwrap()
            .properties()
            .counters()
            .pickup_delay(),
        40
    );
    assert!(head(&e).take().is_none());
}

#[tokio::test]
async fn group_cap_counts_pending_plus_retained_and_refuses_restore_before_counter_admission() {
    for groups in [1, 2, 3] {
        let mut e = Env::new(None, 4, limits(10000, 6000, 1024), Tick::ZERO).await;
        activate(&mut e, groups);
        for n in 1..=groups as u128 {
            e.restore(n, 4, ItemEntityCounters::new(-32768, 32767, 5))
                .await;
        }
        let ticket = head(&e).ticket();
        let rejected = groups as u128 + 1;
        let read = e.seed(rejected, 4, ItemEntityCounters::new(1, 40, 5)).await;
        let out = e.input(OwnedPlayerInput::restore_item(
            read,
            EntityId::new(rejected as i32),
        ));
        assert_eq!(out.refusals.len(), 1);
        assert!(matches!(
            out.refusals[0].reason,
            PlayerOwnershipError::Publication(ItemPublicationError::Capacity {
                budget: ItemPublicationBudget::Groups,
                ..
            })
        ));
        assert_eq!(out.counter_steps(), groups);
        assert_eq!(e.shard.entities().unwrap().len(), groups);
        assert_eq!(
            e.shard.item_publication_status().unwrap().last_published(),
            groups as u64
        );
        e.shard.acknowledge_item_publication(&ticket).unwrap();
        let mut out = spawn(&mut e, rejected + 1, 4).await;
        assert_eq!(out.work.len(), 1);
        let status = e.shard.item_publication_status().unwrap();
        assert_eq!(
            (status.retained_groups(), status.reserved_groups()),
            (groups - 1, 1)
        );
        let another = spawn(&mut e, rejected + 2, 4).await;
        assert!(another.work.is_empty());
        assert!(matches!(
            another.refusals[0].reason,
            PlayerOwnershipError::Reserved
        ));
        let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
        e.input(OwnedPlayerInput::complete(completion));
        assert_eq!(
            e.shard.item_publication_status().unwrap().retained_groups(),
            groups
        );
    }
}

#[tokio::test]
async fn exact_retained_and_prepare_byte_boundaries_preserve_refused_restore_and_spawn() {
    let charge =
        ITEM_PUBLICATION_SCALAR_CHARGE + ferrumc_items::persisted_stack_size(&stone(4)).unwrap();
    for prepare in [false, true] {
        for delta in [-1_isize, 0, 1] {
            let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
            let cap = charge.checked_add_signed(delta).unwrap();
            e.shard
                .initialize_item_publications(policy(
                    1,
                    if prepare { 1024 } else { cap },
                    if prepare { cap } else { 1024 },
                ))
                .unwrap();
            let out = spawn(&mut e, 1, 4).await;
            if delta < 0 {
                assert_eq!(out.refusals.len(), 1);
                assert!(out.work.is_empty());
                assert!(
                    matches!(out.refusals[0].reason, PlayerOwnershipError::Publication(ItemPublicationError::Capacity {budget,..}) if budget == if prepare {ItemPublicationBudget::PrepareBytes} else {ItemPublicationBudget::RetainedBytes})
                );
                assert!(e.shard.entities().unwrap().is_empty());
                assert_eq!(e.shard.player_ownership_pending_count(), 0);
                assert_eq!(
                    e.shard.item_publication_status().unwrap().reserved_bytes(),
                    0
                );
            } else {
                assert_eq!(
                    e.shard.item_publication_status().unwrap().reserved_bytes(),
                    charge
                );
                e.finish(out).await;
                assert_eq!(head(&e).encoded_charge(), charge);
                assert_eq!(
                    e.shard.item_publication_status().unwrap().retained_bytes(),
                    charge
                );
                ack(&mut e);
                assert_eq!(
                    e.shard.item_publication_status().unwrap().retained_bytes(),
                    0
                );
            }
        }
    }
}

#[tokio::test]
async fn acknowledgement_rejects_foreign_owner_stale_head_and_empty_without_losing_data() {
    let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    let mut other = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    activate(&mut other, 2);
    for env in [&mut e, &mut other] {
        env.restore(1, 1, ItemEntityCounters::new(-32768, 32767, 5))
            .await;
        env.restore(2, 1, ItemEntityCounters::new(-32768, 32767, 5))
            .await;
    }
    let old = head(&e).ticket();
    let foreign = head(&other).ticket();
    // This test-only access borrows the real retained tail and mints its normal
    // ticket; it neither constructs an origin nor edits ticket identities.
    let future = e
        .shard
        .player_ownership
        .as_ref()
        .unwrap()
        .publications
        .as_ref()
        .unwrap()
        .test_tail_ticket();
    assert_ne!(head(&e).instance(), head(&other).instance());
    assert!(matches!(
        e.shard.acknowledge_item_publication(&future),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::AckMismatch
        ))
    ));
    assert!(matches!(
        e.shard.acknowledge_item_publication(&foreign),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::AckMismatch
        ))
    ));
    assert_eq!(head(&e).sequence().get(), 1);
    e.shard.acknowledge_item_publication(&old).unwrap();
    assert!(matches!(
        e.shard.acknowledge_item_publication(&old),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::AckMismatch
        ))
    ));
    assert_eq!(head(&e).sequence().get(), 2);
    ack(&mut e);
    assert!(matches!(
        e.shard.acknowledge_item_publication(&old),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::AckWithoutHead
        ))
    ));
    assert_eq!(
        e.shard.item_publication_status().unwrap().retained_groups(),
        0
    );
}

#[tokio::test]
async fn full_outbox_blocks_due_save_without_consuming_id_and_unload_preserves_retained_groups() {
    let mut e = Env::new(None, 1, limits(10000, 1, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let h = e.restore(1, 1, ItemEntityCounters::new(0, 0, 5)).await;
    let ticket = head(&e).ticket();
    let mut out = e.input(OwnedPlayerInput::refill_item_save_ids(
        e.owner.clone(),
        vec![id(1)],
    ));
    assert!(out.work.is_empty());
    assert_eq!(
        e.shard
            .player_ownership
            .as_ref()
            .unwrap()
            .item_counters
            .as_ref()
            .unwrap()
            .save_ids
            .len(),
        1
    );
    assert!(out
        .intents
        .iter()
        .any(|i| matches!(i, OwnershipIntent::ItemSaveBlocked { .. })));
    e.shard
        .release_owned_chunk(ChunkPos::ORIGIN, ChunkTicket::of(TicketReason::Spawn))
        .unwrap();
    assert_eq!(head(&e).sequence().get(), 1);
    assert_eq!(e.shard.entities().unwrap().len(), 1);
    e.shard.acknowledge_item_publication(&ticket).unwrap();
    out = e.run();
    assert_eq!(out.work.len(), 1);
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        1
    );
    e.finish(out).await;
    assert!(e.shard.entities().unwrap().get(&h).is_err());
    assert_eq!(head(&e).kind(), ItemPublicationKind::Removed);
    assert!(head(&e).take().is_none());
    assert_eq!(
        e.shard
            .player_ownership
            .as_ref()
            .unwrap()
            .item_counters
            .as_ref()
            .unwrap()
            .save_ids
            .len(),
        0
    );
}

#[tokio::test]
async fn timer_only_and_noop_autosaves_need_no_publication_credit_or_extra_group() {
    let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    for (n, counters) in [
        (1, ItemEntityCounters::new(0, 40, 5)),
        (2, ItemEntityCounters::new(-32768, 32767, 5)),
    ] {
        let h = e.restore(n, 4, counters).await;
        let gen = e.shard.owned_item_generation(&h).unwrap();
        let out = e.input(OwnedPlayerInput::save_item(h, gen, id(n)));
        assert_eq!(
            e.shard.item_publication_status().unwrap().reserved_groups(),
            0
        );
        assert!(e.shard.player_ownership.as_ref().unwrap().pending[&id(n)]
            .publication
            .is_none());
        e.finish(out).await;
        assert_eq!(
            e.shard.item_publication_status().unwrap().retained_groups(),
            n as usize
        );
        assert_eq!(
            e.shard.item_publication_status().unwrap().last_published(),
            n as u64
        );
    }
}

#[tokio::test]
async fn receipt_none_then_genuine_reconciliation_retains_one_credit_and_publishes_once() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let mut out = spawn(&mut e, 1, 4).await;
    let work = out.take_work().remove(0);
    let query = work.query();
    drop(work);
    let mut out = e.input(OwnedPlayerInput::reconcile(query));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(out.refusals.len(), 1);
    assert_eq!(
        e.shard.player_ownership.as_ref().unwrap().pending[&id(1)].status,
        OwnershipPendingStatus::Uncertain
    );
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        1
    );
    assert!(e.shard.peek_item_publication().unwrap().is_none());
    drop(out);
    let WorkRequest::Entity(request) = e.shard.player_ownership.as_ref().unwrap().pending[&id(1)]
        .request
        .clone()
    else {
        panic!("entity request required")
    };
    e.store
        .compare_exchange_owned_entity(request)
        .await
        .unwrap();
    let mut out = e.input(OwnedPlayerInput::reconcile(query));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert!(out.refusals.is_empty());
    let status = e.shard.item_publication_status().unwrap();
    assert_eq!(
        (
            status.reserved_groups(),
            status.retained_groups(),
            status.last_published()
        ),
        (0, 1, 1)
    );
    assert_eq!(head(&e).origin().receipt_query(), Some(query));
    let out = e.input(OwnedPlayerInput::reconcile(query));
    assert!(out.work.is_empty());
    assert_eq!(
        e.shard.item_publication_status().unwrap().last_published(),
        1
    );
}

#[tokio::test]
async fn actual_busy_retry_retains_one_prepared_body_and_sequence_is_assigned_only_at_confirm() {
    let busy = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut e = Env::new_with_busy(
        None,
        1,
        limits(10000, 6000, 1024),
        Tick::ZERO,
        Some(busy.clone()),
    )
    .await;
    activate(&mut e, 1);
    let mut out = spawn(&mut e, 1, 4).await;
    let work = out.take_work().remove(0);
    let query = work.query();
    busy.store(true, std::sync::atomic::Ordering::SeqCst);
    let completion = work.execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Storage(OwnershipError::Busy)
    ));
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        1
    );
    assert_eq!(
        e.shard.item_publication_status().unwrap().last_published(),
        0
    );
    drop(out);
    let mut out = e.input(OwnedPlayerInput::retry(query));
    busy.store(false, std::sync::atomic::Ordering::SeqCst);
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.input(OwnedPlayerInput::complete(completion));
    assert_eq!(head(&e).sequence().get(), 1);
    assert_eq!(head(&e).origin().receipt_query(), Some(query));
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        0
    );
}

#[tokio::test]
async fn superseded_real_receipt_quarantines_without_postimage_or_released_credit() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let mut out = spawn(&mut e, 1, 4).await;
    let work = out.take_work().remove(0);
    let query = work.query();
    drop(work);
    let WorkRequest::Entity(request) = e.shard.player_ownership.as_ref().unwrap().pending[&id(1)]
        .request
        .clone()
    else {
        panic!("entity request required")
    };
    e.store
        .compare_exchange_owned_entity(request)
        .await
        .unwrap();
    let current = e.durable(1).await;
    e.store
        .compare_exchange_owned_entity(
            EntityOwnershipCasRequest::new(
                id(88),
                current.expectation().clone(),
                EntityOwnershipProposal::put(native(1, 8, ItemEntityCounters::new(99, 0, 5))),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mut out = e.input(OwnedPlayerInput::reconcile(query));
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Superseded
    ));
    assert_eq!(
        e.shard.player_ownership.as_ref().unwrap().pending[&id(1)].status,
        OwnershipPendingStatus::Quarantined
    );
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        1
    );
    assert!(e.shard.peek_item_publication().unwrap().is_none());
    assert!(e.shard.entities().unwrap().is_empty());
}

#[tokio::test]
async fn drain_allows_peek_and_ack_and_stop_rechecks_retained_confirm_in_shadow() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let mut out = spawn(&mut e, 1, 4).await;
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .release_owned_chunk(ChunkPos::ORIGIN, ChunkTicket::of(TicketReason::Spawn))
        .unwrap();
    // Completions are the real prefix even if Stop was enqueued first.
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::stop())
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::NotQuiescent
    ));
    assert_eq!(out.counter_steps(), 0);
    assert!(!e.shard.player_ownership.as_ref().unwrap().stopped);
    let out = e.input(OwnedPlayerInput::drain());
    assert!(out.refusals.is_empty());
    let status = e.shard.item_publication_status().unwrap();
    assert_eq!(status.retained_groups(), 1);
    assert!(!e.shard.is_tick_quiescent());
    ack(&mut e);
    let out = e.input(OwnedPlayerInput::stop());
    assert!(out.refusals.is_empty());
    assert!(e.shard.player_ownership.as_ref().unwrap().stopped);
    assert!(e.shard.is_tick_quiescent());
}

// Faults affect actual off-tick operations, never manufacture a receipt or
// current snapshot. Modes: 1 no-start backend failure; 2 committed but current
// read unavailable; 3 committed with lost reply and unavailable receipt read.
struct FaultStore {
    inner: Arc<dyn PlayerEntityOwnershipStore>,
    mode: Arc<std::sync::atomic::AtomicU8>,
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
        if self.mode.load(std::sync::atomic::Ordering::SeqCst) == 2 {
            return Err(OwnershipError::Backend(
                "injected current read unavailable".into(),
            ));
        }
        self.inner.load_owned_entity(key, identity).await
    }
    async fn load_owned_pair(
        &self,
        player: ScopedPlayerKey,
        entity: PersistentEntityKey,
        identity: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerEntitySnapshot> {
        self.inner.load_owned_pair(player, entity, identity).await
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
        let mode = self.mode.load(std::sync::atomic::Ordering::SeqCst);
        if mode == 1 {
            return Err(OwnershipError::Backend(
                "injected failure before commit attempt".into(),
            ));
        }
        let query = request.receipt_query();
        let result = self.inner.compare_exchange_owned_entity(request).await?;
        if mode == 3 {
            return Err(OwnershipError::ExecutionUncertain {
                query,
                cause: "injected lost reply after genuine commit".into(),
            });
        }
        Ok(result)
    }
    async fn load_ownership_receipt(
        &self,
        query: OwnershipReceiptQuery,
    ) -> OwnershipResult<Option<OwnershipReceipt>> {
        if self.mode.load(std::sync::atomic::Ordering::SeqCst) == 3 {
            return Err(OwnershipError::Backend(
                "injected receipt read unavailable".into(),
            ));
        }
        self.inner.load_ownership_receipt(query).await
    }
}

async fn fault_env() -> (Env, Arc<std::sync::atomic::AtomicU8>) {
    let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    let mode = Arc::new(std::sync::atomic::AtomicU8::new(0));
    let store: Arc<dyn PlayerEntityOwnershipStore> = Arc::new(FaultStore {
        inner: e.store.clone(),
        mode: mode.clone(),
    });
    let binding = OwnershipStoreBinding::new(&store, e.binding.generation());
    // Replace the still-empty fixture owner; all subsequent admissions bind the
    // actual wrapper allocation from construction, just as production does.
    let mut shard = SimShard::with_generation(ShardPos::new(0, 0), binding.generation());
    shard
        .initialize_entities_with_item_budget(nz(2), nz(MAX_ITEM_ENTITY_BYTES_PER_SHARD))
        .unwrap();
    shard
        .loaded_chunks_mut()
        .acquire(
            e.backend.as_ref(),
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
        .initialize_owned_item_counters(limits(10000, 6000, 1024), Tick::ZERO)
        .unwrap();
    e.shard = shard;
    e.binding = binding;
    e.store = store;
    e.owner = owner;
    activate(&mut e, 1);
    (e, mode)
}

#[tokio::test]
async fn definitive_no_start_abort_releases_credit_and_shadow_can_stop_in_same_boundary() {
    let (mut e, mode) = fault_env().await;
    let mut out = spawn(&mut e, 1, 4).await;
    mode.store(1, std::sync::atomic::Ordering::SeqCst);
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::stop())
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Storage(OwnershipError::Backend(_))
    ));
    let status = e.shard.item_publication_status().unwrap();
    assert_eq!(
        (
            status.reserved_groups(),
            status.retained_groups(),
            status.last_published()
        ),
        (0, 0, 0)
    );
    assert!(e.shard.entities().unwrap().is_empty());
    assert!(e.shard.player_ownership.as_ref().unwrap().stopped);
    assert_eq!(out.counter_steps(), 0);
}

#[tokio::test]
async fn abort_prefix_frees_full_credit_before_actual_restore_and_shadow_first_step() {
    let (mut e, mode) = fault_env().await;
    let read = e.seed(2, 4, ItemEntityCounters::new(100, 40, 5)).await;
    let mut out = spawn(&mut e, 1, 4).await;
    mode.store(1, std::sync::atomic::Ordering::SeqCst);
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    mode.store(0, std::sync::atomic::Ordering::SeqCst);
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::restore_item(read, EntityId::new(2)))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert_eq!(out.counter_steps(), 1);
    assert_eq!(e.counters(2), ItemEntityCounters::new(101, 39, 5));
    assert_eq!(head(&e).instance().uuid(), uuid(2));
    assert_eq!(head(&e).sequence().get(), 1);
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        0
    );
}

#[tokio::test]
async fn current_read_failure_and_real_committed_lost_reply_keep_credit_until_reconcile() {
    for fault in [2, 3] {
        let (mut e, mode) = fault_env().await;
        let mut out = spawn(&mut e, 1, 4).await;
        let work = out.take_work().remove(0);
        let query = work.query();
        mode.store(fault, std::sync::atomic::Ordering::SeqCst);
        let completion = work.execute(&e.store).await.unwrap();
        let out = e.input(OwnedPlayerInput::complete(completion));
        assert_eq!(out.refusals.len(), 1);
        assert_eq!(
            e.shard.player_ownership.as_ref().unwrap().pending[&id(1)].status,
            if fault == 2 {
                OwnershipPendingStatus::NeedsRefresh
            } else {
                OwnershipPendingStatus::Uncertain
            }
        );
        assert_eq!(
            e.shard.item_publication_status().unwrap().reserved_groups(),
            1
        );
        assert_eq!(
            e.shard.item_publication_status().unwrap().last_published(),
            0
        );
        assert!(e.shard.peek_item_publication().unwrap().is_none());
        drop(out);
        mode.store(0, std::sync::atomic::Ordering::SeqCst);
        let mut out = e.input(OwnedPlayerInput::reconcile(query));
        let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
        let out = e.input(OwnedPlayerInput::complete(completion));
        assert!(out.refusals.is_empty());
        assert_eq!(head(&e).sequence().get(), 1);
        assert_eq!(head(&e).origin().receipt_query(), Some(query));
        assert_eq!(
            e.shard.item_publication_status().unwrap().reserved_groups(),
            0
        );
    }
}

#[tokio::test]
async fn same_boundary_confirm_retains_full_credit_so_following_restore_is_not_shadow_admitted() {
    let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let read = e.seed(2, 4, ItemEntityCounters::new(100, 40, 5)).await;
    let mut out = spawn(&mut e, 1, 4).await;
    let completion = out.take_work().remove(0).execute(&e.store).await.unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::restore_item(read, EntityId::new(2)))
        .unwrap();
    e.shard
        .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
        .unwrap();
    let out = e.run();
    assert_eq!(out.refusals.len(), 1);
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Publication(ItemPublicationError::Capacity {
            budget: ItemPublicationBudget::Groups,
            ..
        })
    ));
    assert_eq!(out.counter_steps(), 1);
    assert_eq!(e.shard.entities().unwrap().len(), 1);
    assert_eq!(head(&e).instance().uuid(), uuid(1));
    assert_eq!(
        e.shard.item_publication_status().unwrap().last_published(),
        1
    );
}

#[tokio::test]
async fn removed_instance_and_ticket_remain_bound_through_slot_reuse_and_same_wire() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    let p = e
        .player(Box::new(std::array::from_fn(|_| ItemStack::empty())))
        .await;
    let old = e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    let instance = head(&e).instance().clone();
    ack(&mut e);
    let pair = e
        .binding
        .read_pair(&e.store, p.player(), uuid(1))
        .await
        .unwrap();
    let out = e.input(OwnedPlayerInput::pickup(
        p,
        old.clone(),
        pair,
        id(1),
        true,
        false,
    ));
    e.finish(out).await;
    let removed = head(&e).ticket();
    let read = e
        .seed(2, 7, ItemEntityCounters::new(-32768, 32767, 5))
        .await;
    let out = e.input(OwnedPlayerInput::restore_item(read, EntityId::new(1)));
    assert!(out.refusals.is_empty());
    assert_eq!(head(&e).instance(), &instance);
    assert!(e.shard.entities().unwrap().get(&old).is_err());
    e.shard.acknowledge_item_publication(&removed).unwrap();
    assert_eq!(head(&e).instance().wire_id(), instance.wire_id());
    assert_ne!(head(&e).instance(), &instance);
    assert_ne!(
        head(&e).instance().slot_generation(),
        instance.slot_generation()
    );
    assert!(matches!(
        e.shard.acknowledge_item_publication(&removed),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::AckMismatch
        ))
    ));
}

#[tokio::test]
async fn actual_redb_pickup_reopen_mints_fresh_instance_and_never_replays_ephemeral_groups() {
    let path = std::env::temp_dir().join(format!(
        "ferrumc-sim034-publication-{}.redb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut e = Env::new(Some(&path), 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    let mut slots = Box::new(std::array::from_fn(|_| stone(64)));
    slots[36] = stone(63);
    let p = e.player(slots).await;
    let h = e.restore(1, 4, ItemEntityCounters::new(0, 0, 5)).await;
    let original = head(&e).instance().clone();
    ack(&mut e);
    let pair = e
        .binding
        .read_pair(&e.store, p.player(), uuid(1))
        .await
        .unwrap();
    let out = e.input(OwnedPlayerInput::pickup(p, h, pair, id(1), true, false));
    e.finish(out).await;
    assert_eq!(head(&e).postimage().unwrap().stack().count(), 3);
    let old_ticket = head(&e).ticket();
    drop(e);
    let mut e = Env::new(Some(&path), 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    assert!(e.shard.peek_item_publication().unwrap().is_none());
    let read = e.binding.read_entity(&e.store, uuid(1)).await.unwrap();
    let out = e.input(OwnedPlayerInput::restore_item(read, EntityId::new(1)));
    assert!(out.refusals.is_empty());
    assert_eq!(head(&e).postimage().unwrap().stack().count(), 3);
    assert_eq!(head(&e).sequence().get(), 1);
    assert_ne!(head(&e).instance(), &original);
    assert!(matches!(
        e.shard.acknowledge_item_publication(&old_ticket),
        Err(PlayerOwnershipError::Publication(
            ItemPublicationError::AckMismatch
        ))
    ));
    drop(e);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn actual_final_sequence_confirms_once_then_spawn_and_restore_refuse_before_work() {
    let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 2);
    // Reach an otherwise uniterable integer boundary without changing any
    // body, receipt, owner, reservation or completion evidence.
    e.shard
        .player_ownership
        .as_mut()
        .unwrap()
        .publications
        .as_mut()
        .unwrap()
        .set_test_sequence_boundary(u64::MAX - 1);
    let out = spawn(&mut e, 1, 4).await;
    assert_eq!(
        e.shard.item_publication_status().unwrap().last_published(),
        u64::MAX - 1
    );
    e.finish(out).await;
    assert_eq!(head(&e).sequence().get(), u64::MAX);
    ack(&mut e);
    let out = spawn(&mut e, 2, 4).await;
    assert!(out.work.is_empty());
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Publication(ItemPublicationError::SequenceExhausted)
    ));
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        0
    );
    let read = e.seed(3, 4, ItemEntityCounters::new(100, 40, 5)).await;
    let out = e.input(OwnedPlayerInput::restore_item(read, EntityId::new(3)));
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Publication(ItemPublicationError::SequenceExhausted)
    ));
    assert_eq!(out.counter_steps(), 1);
    assert_eq!(e.shard.entities().unwrap().len(), 1);
    assert_eq!(
        e.shard.item_publication_status().unwrap().last_published(),
        u64::MAX
    );
    let h = e.handle(1);
    let generation = e.shard.owned_item_generation(&h).unwrap();
    let out = e.input(OwnedPlayerInput::save_item(h, generation, id(9)));
    assert_eq!(out.work.len(), 1);
    e.finish(out).await;
    assert!(e.shard.peek_item_publication().unwrap().is_none());
    assert_eq!(
        e.shard.item_publication_status().unwrap().last_published(),
        u64::MAX
    );
}

#[tokio::test]
async fn aggregate_retained_byte_n_minus_one_n_plus_one_gates_second_pending_body() {
    let charge =
        ITEM_PUBLICATION_SCALAR_CHARGE + ferrumc_items::persisted_stack_size(&stone(4)).unwrap();
    for delta in [-1_isize, 0, 1] {
        let mut e = Env::new(None, 2, limits(10000, 6000, 1024), Tick::ZERO).await;
        let cap = (2 * charge).checked_add_signed(delta).unwrap();
        e.shard
            .initialize_item_publications(policy(2, cap, charge))
            .unwrap();
        e.restore(1, 4, ItemEntityCounters::new(-32768, 32767, 5))
            .await;
        let out = spawn(&mut e, 2, 4).await;
        if delta < 0 {
            assert!(out.work.is_empty());
            assert!(
                matches!(out.refusals[0].reason,PlayerOwnershipError::Publication(ItemPublicationError::Capacity {budget:ItemPublicationBudget::RetainedBytes,used,requested,max}) if used==charge && requested==charge && max==cap)
            );
            assert_eq!(
                e.shard.item_publication_status().unwrap().reserved_bytes(),
                0
            );
            assert_eq!(e.shard.entities().unwrap().len(), 1);
        } else {
            let status = e.shard.item_publication_status().unwrap();
            assert_eq!(
                (status.retained_bytes(), status.reserved_bytes()),
                (charge, charge)
            );
            e.finish(out).await;
            let status = e.shard.item_publication_status().unwrap();
            assert_eq!(
                (status.retained_bytes(), status.reserved_bytes()),
                (2 * charge, 0)
            );
        }
    }
}

#[tokio::test]
async fn actual_cas_conflict_keeps_reserved_body_quarantined_without_publishing() {
    let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
    activate(&mut e, 1);
    let mut out = spawn(&mut e, 1, 4).await;
    let work = out.take_work().remove(0);
    e.seed(1, 8, ItemEntityCounters::new(99, 0, 5)).await;
    let completion = work.execute(&e.store).await.unwrap();
    let out = e.input(OwnedPlayerInput::complete(completion));
    assert!(matches!(
        out.refusals[0].reason,
        PlayerOwnershipError::Storage(OwnershipError::CasConflict { .. })
    ));
    assert_eq!(
        e.shard.player_ownership.as_ref().unwrap().pending[&id(1)].status,
        OwnershipPendingStatus::Quarantined
    );
    assert_eq!(
        e.shard.item_publication_status().unwrap().reserved_groups(),
        1
    );
    assert_eq!(
        e.shard.item_publication_status().unwrap().last_published(),
        0
    );
    assert!(e.shard.peek_item_publication().unwrap().is_none());
    assert!(e.shard.entities().unwrap().is_empty());
}

#[tokio::test]
async fn scope_or_generation_mismatch_refuses_peek_ack_before_changing_retained_head() {
    for foreign in [
        GenerationBinding::new(
            GenerationKey::new(WorldId::new(99), DimensionId::new(13)),
            identity(),
        ),
        GenerationBinding::new(
            scope(),
            GenerationIdentity::flat_overworld(WorldSeed::new(38)),
        ),
    ] {
        let mut e = Env::new(None, 1, limits(10000, 6000, 1024), Tick::ZERO).await;
        activate(&mut e, 1);
        e.restore(1, 4, ItemEntityCounters::new(-32768, 32767, 5))
            .await;
        let ticket = head(&e).ticket();
        let original = std::mem::replace(
            &mut e.shard.chunks,
            LoadedChunkMap::with_generation(foreign),
        );
        assert!(matches!(
            e.shard.peek_item_publication(),
            Err(PlayerOwnershipError::ScopeMismatch)
        ));
        assert!(matches!(
            e.shard.acknowledge_item_publication(&ticket),
            Err(PlayerOwnershipError::ScopeMismatch)
        ));
        e.shard.chunks = original;
        assert_eq!(head(&e).sequence().get(), 1);
        assert_eq!(
            e.shard.item_publication_status().unwrap().retained_groups(),
            1
        );
        e.shard.acknowledge_item_publication(&ticket).unwrap();
    }
}
