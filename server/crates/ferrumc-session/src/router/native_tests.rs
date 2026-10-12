//! Crate-internal NET-04.2e1 tests that need private router state: checked
//! wire/session-counter exhaustion at `MAX`, a poisoned movement cell, each
//! take/replace branch of a movement cell including a replacement between
//! dequeue and cell lock, and a verdict from another probe. Admissions are
//! genuine (memory store).

use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};
use std::sync::Arc;

use ferrumc_codec::BoundedString;
use ferrumc_core::{DimensionId, PlayerId, WorldId};
use ferrumc_math::{ChunkPos, ShardPos, Vec3};
use ferrumc_proto::generated::configuration::{KnownPack, RegistryData, RegistryEntry};
use ferrumc_proto::generated::play::{
    JoinGame, ServerboundPlayPacket, SetPlayerPosition, SpawnInfo,
};
use ferrumc_sim::{
    ChunkTicket, GenerationBinding, NativePlayerSnapshot, OwnedPlayerHandle, OwnedPlayerInput,
    OwnershipIntent, OwnershipStoreBinding, PlayerOwnershipLimits, ShardPartitioner, SimShard,
    TicketReason, MAX_OWNED_PLAYER_BYTES, MAX_PLAYER_OWNERSHIP_INPUT_BYTES,
    MAX_PLAYER_OWNERSHIP_PENDING_BYTES,
};
use ferrumc_storage::{
    GenerationKey, InMemoryStore, OwnershipCapacity, OwnershipConfig, OwnershipOpenLimits,
    OwnershipTransferId, PlayerEntityOwnershipStore, PlayerOwnershipCasRequest,
    PlayerOwnershipProposal, ScopedPlayerKey, MAX_OWNERSHIP_REQUEST_BYTES,
};
use ferrumc_world::{FlatWorldGenerator, GenerationIdentity, WorldSeed};

use super::*;
use crate::native::{NativeLaneRecv, NativeLifecycleVerdict};
use crate::router::DEFAULT_VIEW_DISTANCE;
use crate::{SessionError, ShardLease};
use ferrumc_sim::ShardId;

fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn generation() -> GenerationBinding {
    GenerationBinding::new(
        GenerationKey::new(WorldId::new(0), DimensionId::new(0)),
        GenerationIdentity::flat_overworld(WorldSeed::new(37)),
    )
}
fn home() -> ShardId {
    ShardPartitioner::for_shard(WorldId::new(0), DimensionId::new(0), ShardPos::new(0, 0)).unwrap()
}
fn bs(value: &str) -> BoundedString<32_767> {
    BoundedString::new(value.to_owned()).unwrap()
}
fn spawn() -> Vec3 {
    Vec3::new(8.5, 64.0, 8.5)
}

fn native_router() -> (SessionRouter, ShardLease, NativeShardReceiver) {
    let mut router = SessionRouter::with_capacities(64, 16);
    let registry = RegistryData::new(
        bs("minecraft:dimension_type"),
        vec![RegistryEntry::new(bs("minecraft:overworld"), None)],
    );
    let pack = KnownPack::new(bs("minecraft"), bs("core"), bs("1.21.8"));
    router
        .initialize_collector_sessions(
            CollectorSessionLimits::new(nz(8), nz(7), nz(64), nz(8), nz(1024), nz(4096)).unwrap(),
            &registry,
            &pack,
            10,
        )
        .unwrap();
    let (_, lease, lane) = router
        .register_native_world_shard(home(), generation())
        .unwrap()
        .into_parts();
    (router, lease, lane)
}

struct Env {
    shard: SimShard,
    store: Arc<dyn PlayerEntityOwnershipStore>,
    binding: OwnershipStoreBinding,
}

async fn admitted(player: PlayerId) -> (Env, OwnedPlayerHandle) {
    let backend = Arc::new(
        InMemoryStore::with_player_entity_ownership(OwnershipConfig::new(
            OwnershipCapacity::new(
                nz(MAX_OWNERSHIP_REQUEST_BYTES),
                NonZeroU64::new(1000).unwrap(),
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
        ))
        .unwrap(),
    );
    ferrumc_storage::WorldStore::ensure_generation_identity(
        backend.as_ref(),
        generation().key(),
        generation().identity(),
    )
    .await
    .unwrap();
    let store: Arc<dyn PlayerEntityOwnershipStore> = backend.clone();
    let binding = OwnershipStoreBinding::new(&store, generation());
    let mut shard = SimShard::with_generation(ShardPos::new(0, 0), generation());
    shard
        .initialize_entities_with_item_budget(nz(4), nz(1024 * 1024))
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
                nz(8),
                nz(MAX_PLAYER_OWNERSHIP_INPUT_BYTES),
            )
            .unwrap(),
        )
        .unwrap();
    let key = ScopedPlayerKey::new(generation().key(), player);
    let before = store
        .load_owned_player(key, generation().identity())
        .await
        .unwrap();
    store
        .compare_exchange_owned_player(
            PlayerOwnershipCasRequest::new(
                OwnershipTransferId::from_bytes([9; 16]),
                before.expectation().clone(),
                PlayerOwnershipProposal::put(
                    NativePlayerSnapshot::empty_survival(spawn(), [0.0, 0.0])
                        .try_to_record()
                        .unwrap(),
                ),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let read = binding.read_player(&store, player).await.unwrap();
    shard
        .enqueue_owned_player_input(OwnedPlayerInput::restore(read))
        .unwrap();
    let out = shard.run_tick_with_player_ownership().unwrap();
    let handle = out
        .intents()
        .iter()
        .find_map(|intent| match intent {
            OwnershipIntent::PlayerAdmitted { player, .. } => Some(player.clone()),
            _ => None,
        })
        .unwrap();
    (
        Env {
            shard,
            store,
            binding,
        },
        handle,
    )
}

fn join_game(view: &ferrumc_sim::ConnectedOwnedPlayerView<'_>) -> JoinGame {
    JoinGame::new(
        1,
        false,
        vec![bs("minecraft:overworld")],
        32,
        DEFAULT_VIEW_DISTANCE,
        10,
        false,
        true,
        false,
        SpawnInfo::new(
            0,
            bs("minecraft:overworld"),
            view.hashed_seed(),
            view.game_mode().as_id(),
            255,
            false,
            true,
            None,
            0,
            63,
        ),
        false,
    )
}

async fn native_session() -> (
    Env,
    SessionRouter,
    ShardLease,
    NativeShardReceiver,
    PlayerSessionKey,
) {
    let (mut router, lease, lane) = native_router();
    let player = PlayerId::offline("Internal");
    let reservation = router
        .reserve_native_join(player, generation(), spawn())
        .unwrap();
    let (env, handle) = admitted(player).await;
    let view = env.shard.connected_owned_player_view(&handle).unwrap();
    let session = router
        .admit_native_join(
            &reservation,
            &lane,
            &handle,
            &view,
            join_game(&view),
            "i",
            vec![],
        )
        .unwrap();
    let key = session.session_key().unwrap();
    // Keep the env's store/binding alive with the shard.
    let _ = (&env.store, &env.binding);
    (env, router, lease, lane, key)
}

#[test]
fn legacy_wire_allocator_issues_max_once_then_refuses_before_slow_viewer_and_player_join() {
    let mut router = SessionRouter::with_capacities(16, 4);
    let mut shard = router.register_shard(ShardPos::new(0, 0));
    router.next_entity_id = Some(i32::MAX - 1);
    let mut first = router
        .join_player(PlayerId::offline("Max1"), "Max1", spawn())
        .unwrap();
    let _second = router
        .join_player(PlayerId::offline("Max"), "Max", spawn())
        .unwrap();
    assert_eq!(
        router.player_entity_id(PlayerId::offline("Max1")),
        Some(i32::MAX - 1)
    );
    assert_eq!(
        router.player_entity_id(PlayerId::offline("Max")),
        Some(i32::MAX)
    );
    assert_eq!(router.next_entity_id, None);
    // Make both viewers slow (no room for a mandatory add+spawn pair).
    let filler = ClientboundPlayPacket::SynchronizePlayerPosition(
        ferrumc_proto::generated::play::SynchronizePlayerPosition::new(
            0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0,
        ),
    );
    router.broadcast_play_packet(&filler);
    router.broadcast_play_packet(&filler);
    while shard.try_recv().is_ok() {}
    assert_eq!(
        router
            .join_player(PlayerId::offline("Wrapped"), "Wrapped", spawn())
            .err(),
        Some(SessionError::EntityIdsExhausted)
    );
    // No slow-viewer disconnect, no PlayerJoin, no retained leave.
    assert_eq!(router.player_count(), 2);
    assert_eq!(router.pending_disconnect_count(), 0);
    assert!(shard.try_recv().is_err());
    assert!(first.try_recv().is_some());
    // Control: with an id available the same join does disconnect the slow
    // viewer first, so the refusal above really preceded that side effect.
    router.next_entity_id = Some(7);
    router
        .join_player(PlayerId::offline("Control"), "Control", spawn())
        .unwrap();
    assert!(!router.is_player_connected(PlayerId::offline("Max")));
}

#[test]
fn legacy_refused_join_consumes_no_wire_id() {
    let mut router = SessionRouter::with_capacities(16, 1);
    let _shard = router.register_shard(ShardPos::new(0, 0));
    router
        .join_player(PlayerId::offline("Seen"), "Seen", spawn())
        .unwrap();
    // One in-range player needs two mandatory packets; capacity 1 refuses.
    assert_eq!(
        router
            .join_player(PlayerId::offline("TooBig"), "TooBig", spawn())
            .err(),
        Some(SessionError::OutboundFull {
            player: PlayerId::offline("TooBig")
        })
    );
    assert_eq!(router.next_entity_id, Some(3));
}

#[test]
fn native_reservation_burns_max_once_and_refuses_exhausted_counters_without_side_effects() {
    let (mut router, _lease, _lane) = native_router();
    router.next_entity_id = Some(i32::MAX);
    router.next_session = Some(NonZeroU64::MAX);
    let last = router
        .reserve_native_join(PlayerId::offline("Last"), generation(), spawn())
        .unwrap();
    let wire = match &router.native.entry(last.key.slot).unwrap().state {
        EntryState::Reserved(record) => record.wire_id,
        other => panic!("reservation entry expected: {other:?}"),
    };
    assert_eq!(wire, i32::MAX);
    assert_eq!(router.next_entity_id, None);
    assert_eq!(router.next_session, None);
    assert_eq!(
        router
            .reserve_native_join(PlayerId::offline("After"), generation(), spawn())
            .err(),
        Some(CollectorSessionError::SessionIdsExhausted)
    );
    router.next_session = Some(NonZeroU64::MIN);
    assert_eq!(
        router
            .reserve_native_join(PlayerId::offline("After"), generation(), spawn())
            .err(),
        Some(CollectorSessionError::EntityIdsExhausted)
    );
    assert_eq!(router.native_reservation_count(), 1);
    assert_eq!(router.next_session, Some(NonZeroU64::MIN));
    router.cancel_native_join(last).unwrap();
    assert_eq!(router.next_entity_id, None, "a cancel never reissues");
}

#[test]
fn legacy_and_native_share_one_monotonic_wire_allocator() {
    let (mut router, lease, lane) = native_router();
    let reservation = router
        .reserve_native_join(PlayerId::offline("Native"), generation(), spawn())
        .unwrap();
    router.cancel_native_join(reservation).unwrap();
    router
        .unregister_native_shard_registration(&lease, lane)
        .map_err(|e| e.into_parts().0)
        .unwrap();
    let _shard = router.register_shard(ShardPos::new(0, 0));
    router
        .join_player(PlayerId::offline("Legacy"), "Legacy", spawn())
        .unwrap();
    assert_eq!(
        router.player_entity_id(PlayerId::offline("Legacy")),
        Some(3)
    );
}

#[tokio::test]
async fn poisoned_movement_cell_is_reported_typed_and_never_merged_again() {
    let (_env, mut router, _lease, mut lane, key) = native_session().await;
    let packet =
        |x| ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(x, 64.0, 9.0, 0));
    assert_eq!(
        router.route_native_play(&key, &packet(9.0)),
        Ok(NativeRouted::Queued)
    );
    let cell = Arc::clone(
        router
            .native
            .session(key.slot)
            .unwrap()
            .movement
            .as_ref()
            .unwrap(),
    );
    cell.poison_for_test();
    // The poisoned cell refuses the merge; the newer movement gets its own entry.
    assert_eq!(
        router.route_native_play(&key, &packet(10.0)),
        Ok(NativeRouted::Queued)
    );
    assert_eq!(
        lane.try_recv().err(),
        Some(NativeLaneRecv::MovementCellPoisoned {
            session: key.clone()
        })
    );
    let newer = lane.try_recv().unwrap();
    assert_eq!(newer.session(), &key);
    assert!(format!("{:?}", newer.into_parts().2).contains("10.0"));
}

/// The `x` of a taken movement input.
fn taken_x(input: NativeShardInput) -> f64 {
    let debug = format!("{:?}", input.into_parts().2);
    let start = debug.find("x: ").unwrap() + 3;
    debug[start..].split(',').next().unwrap().parse().unwrap()
}

#[tokio::test]
#[allow(clippy::float_cmp, reason = "positions round-trip exact bit patterns")]
async fn each_take_and_replace_branch_of_a_movement_cell_is_decided_deterministically() {
    let (_env, mut router, lease, mut lane, key) = native_session().await;
    let packet =
        |x| ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(x, 64.0, 9.0, 0));

    // Replace before take: the pending entry carries the newer state.
    assert_eq!(
        router.route_native_play(&key, &packet(10.0)),
        Ok(NativeRouted::Queued)
    );
    assert_eq!(
        router.route_native_play(&key, &packet(11.0)),
        Ok(NativeRouted::Coalesced)
    );
    assert_eq!(taken_x(lane.try_recv().unwrap()), 11.0);
    assert!(matches!(lane.try_recv(), Err(NativeLaneRecv::Empty)));

    // Take before replace: the taken cell refuses the merge (`Taken`) and the
    // newer state gets its own entry.
    assert_eq!(
        router.route_native_play(&key, &packet(12.0)),
        Ok(NativeRouted::Queued)
    );
    assert_eq!(taken_x(lane.try_recv().unwrap()), 12.0);
    assert_eq!(
        router.route_native_play(&key, &packet(13.0)),
        Ok(NativeRouted::Queued)
    );
    assert_eq!(taken_x(lane.try_recv().unwrap()), 13.0);

    // Replace between dequeue and cell lock: the receiver already dequeued the
    // entry but has not taken the cell, so the merge still lands in it and the
    // take yields the newer state without a second entry.
    assert_eq!(
        router.route_native_play(&key, &packet(14.0)),
        Ok(NativeRouted::Queued)
    );
    let Ok(LaneItem::Movement {
        key: item_key,
        cell,
    }) = lane.receiver.try_recv()
    else {
        panic!("a pending movement entry");
    };
    assert_eq!(
        router.route_native_play(&key, &packet(15.0)),
        Ok(NativeRouted::Coalesced)
    );
    assert!(matches!(lane.try_recv(), Err(NativeLaneRecv::Empty)));
    // Finish the take through the receiver's own path.
    let (sender, receiver) = lane_channel(1);
    sender
        .try_send(LaneItem::Movement {
            key: item_key,
            cell,
        })
        .unwrap();
    let mut detached = NativeShardReceiver {
        identity: Arc::new(NativeLaneIdentity::new(lease.coverage())),
        receiver,
    };
    let taken = detached.try_recv().unwrap();
    assert_eq!(taken.session(), &key);
    assert_eq!(taken_x(taken), 15.0);
    // Once taken, the next movement needs its own entry again.
    assert_eq!(
        router.route_native_play(&key, &packet(16.0)),
        Ok(NativeRouted::Queued)
    );
    assert_eq!(taken_x(lane.try_recv().unwrap()), 16.0);
    assert!(matches!(lane.try_recv(), Err(NativeLaneRecv::Empty)));
}

#[tokio::test]
async fn a_verdict_of_another_probe_refuses_revalidation() {
    let (env, mut router, lease, lane, key) = native_session().await;
    let (_, _, current) = router
        .replace_native_shard_registration(&lease, &lane)
        .unwrap()
        .into_parts();
    let report = router
        .revalidate_native_endpoints(&current, &mut |handle, probe| {
            let view = env.shard.connected_owned_player_view(handle).unwrap();
            let pose = LivePose {
                position: view.position(),
                look: view.look(),
                game_mode: view.game_mode(),
            };
            // A matched outcome under a sequence no probe of this call carries.
            NativeRevalidationVerdict {
                seq: probe.seq.wrapping_add(1),
                outcome: VerdictOutcome::Matched(pose),
            }
        })
        .unwrap();
    assert_eq!(report.refused(), 1);
    assert_eq!(report.revalidated(), 0);
    assert_eq!(
        router.native_session_status(&key).unwrap().endpoint(),
        crate::NativeEndpointState::RevalidationRefused
    );
}

#[test]
fn a_legacy_input_resolved_to_a_native_endpoint_is_returned_intact() {
    let (router, lease, _lane) = native_router();
    let binding = super::super::ShardBinding {
        coverage: lease.coverage(),
        registration_id: lease.registration_id(),
        home: home(),
    };
    let input = ferrumc_sim::GameInput::PlayerMove {
        player: PlayerId::offline("Legacy"),
        position: Some(spawn()),
        yaw: None,
        pitch: None,
    };
    let policy = crate::InputDeliveryPolicy::for_input(&input);
    let refused = router
        .send_to_shard_owned(binding, input.clone(), policy)
        .unwrap_err();
    assert_eq!(refused.input(), &input);
    assert_eq!(
        refused.error(),
        &SessionError::NativeEndpointRequiresSession { home: home() }
    );
    // The UUID-only legacy route never reaches the native lane.
    assert!(router
        .route_game_input_owned(PlayerId::offline("Legacy"), input)
        .is_err());
}

// ---------------------------------------------------------------- NET-04.2e2

/// A1 / R3-3: the reservation allocates the one table entry; admission,
/// disconnect and adoption take it over in place, and a freed entry is reused
/// before the table grows.
#[tokio::test]
async fn table_entries_are_taken_over_in_place_without_growth() {
    let (env, mut router, _lease, _lane, key) = native_session().await;
    let slots = router.native.slots.len();
    let capacity = router.native.slots.capacity();
    assert_eq!(slots, 1);
    assert!(matches!(
        router.native.entry(key.slot).unwrap().state,
        EntryState::Session(_)
    ));
    let token = router.disconnect_native(&key).unwrap();
    assert_eq!(router.native.slots.len(), slots);
    assert_eq!(router.native.slots.capacity(), capacity);
    assert!(router
        .native
        .session(key.slot)
        .unwrap()
        .retention
        .as_ref()
        .is_some_and(|r| r.lineage.get() == token.lineage()));
    // A second reservation grows the table by exactly one entry, fallibly.
    let second = router
        .reserve_native_join(PlayerId::offline("Second"), generation(), spawn())
        .unwrap();
    assert_eq!(second.key.slot, 1);
    let grown = router.native.slots.capacity();
    // Cancel frees it; the next reservation reuses the same slot index.
    router.cancel_native_join(second).unwrap();
    let third = router
        .reserve_native_join(PlayerId::offline("Third"), generation(), spawn())
        .unwrap();
    assert_eq!(third.key.slot, 1);
    assert_eq!(router.native.slots.len(), 2);
    assert_eq!(router.native.slots.capacity(), grown);
    // Adoption replaces the reserved state in the same slot.
    let (third_env, handle) = admitted(PlayerId::offline("Third")).await;
    let view = third_env
        .shard
        .connected_owned_player_view(&handle)
        .unwrap();
    let adopted = router
        .adopt_unbound_native_admission(third, &handle, &view)
        .unwrap();
    assert!(matches!(
        router.native.entry(1).unwrap().state,
        EntryState::Record(_)
    ));
    assert_eq!(router.native.slots.len(), 2);
    assert_eq!(router.native.slots.capacity(), grown);
    assert_ne!(adopted.lineage(), token.lineage());
    drop(env);
}

#[tokio::test]
async fn exhausted_lineage_refuses_before_any_effect_and_poisons_an_adoption() {
    let (_env, mut router, _lease, mut lane, key) = native_session().await;
    router.native.next_lineage = None;
    assert_eq!(
        router.disconnect_native(&key),
        Err(CollectorSessionError::LineageExhausted)
    );
    let status = router.native_session_status(&key).unwrap();
    assert_eq!(status.phase(), crate::NativeSessionPhase::Active);
    assert_eq!(status.disconnect(), None);
    assert!(matches!(lane.try_recv(), Err(NativeLaneRecv::Empty)));
    // The last lineage is issued exactly once.
    router.native.next_lineage = Some(NonZeroU64::MAX);
    let token = router.disconnect_native(&key).unwrap();
    assert_eq!(token.lineage(), u64::MAX);
    assert_eq!(router.native.next_lineage, None);
    let reservation = router
        .reserve_native_join(PlayerId::offline("Late"), generation(), spawn())
        .unwrap();
    let (env, handle) = admitted(PlayerId::offline("Late")).await;
    let view = env.shard.connected_owned_player_view(&handle).unwrap();
    let broken = router
        .adopt_unbound_native_admission(reservation, &handle, &view)
        .unwrap_err();
    assert_eq!(
        broken.kind(),
        crate::AdoptionInvariantKind::LineageExhausted
    );
    assert_eq!(
        router.native_poisoned(PlayerId::offline("Late")),
        Some(crate::AdoptionInvariantKind::LineageExhausted)
    );
}

#[tokio::test]
async fn lifecycle_verdicts_of_another_probe_or_an_exhausted_sequence_prove_nothing() {
    let (_env, mut router, _lease, _lane, key) = native_session().await;
    let token = router.disconnect_native(&key).unwrap();
    let forged_facts = crate::native::LifecycleFacts {
        matches: true,
        detached: true,
        live: false,
        reserved_pins: 0,
        retained_pins: 0,
    };
    // A detached fact under a sequence no probe of this call carries.
    let outcome = router
        .native_detached(key.player, &mut |_, probe| NativeLifecycleVerdict {
            seq: probe.seq.wrapping_add(1),
            facts: Some(forged_facts),
            ticket: None,
            acknowledged: None,
        })
        .unwrap();
    assert_eq!(outcome, crate::NativeDetachOutcome::NotDetached);
    assert_eq!(
        router.native_retention_status(token).unwrap().phase(),
        crate::NativeRetentionPhase::Leaving
    );
    // An exhausted probe sequence refuses before the closure runs.
    router.probe_seq = u64::MAX;
    assert_eq!(
        router.native_detached(key.player, &mut |_, _| unreachable!()),
        Err(CollectorSessionError::ProbesExhausted)
    );
    assert_eq!(
        router.retire_detached_native_sessions(&mut |_, _| unreachable!()),
        Ok(crate::NativeRetirementReport::default())
    );
}
