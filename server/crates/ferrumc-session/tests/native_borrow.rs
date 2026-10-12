//! Root R3-A1 / F1: the revalidation closure compiles against a driver slot
//! shape with a shared **and** with an exclusive shard borrow, disjoint from the
//! borrowed lane, and the view lives only inside the closure call. The failing
//! contract forms are `compile_fail` doctests on `NativeRevalidationProbe`.

mod native_support;

use ferrumc_math::ShardPos;
use ferrumc_session::{NativeEndpointState, NativeShardReceiver, SessionRouter};
use ferrumc_sim::{OwnedPlayerInput, OwnershipIntent, SimShard};
use native_support::*;

/// The app-private driver slot reduced to the two fields the call touches.
struct NativeShardSlot {
    shard: SimShard,
    lane: NativeShardReceiver,
}

async fn suspended_slot(
    store: Store,
) -> (
    SimEnv,
    SessionRouter,
    NativeShardSlot,
    ferrumc_session::PlayerSessionKey,
) {
    let mut env = SimEnv::new(store).await;
    let mut router = router(64, 16, 8);
    let registration = router
        .register_native_world_shard(home(), generation())
        .unwrap();
    let lease = registration.lease();
    let (_, _, lane) = registration.into_parts();
    let admitted = join(&mut router, &mut env, &lane, "Slot", spawn()).await;
    let key = admitted.session.session_key().unwrap();
    let (_, _, lane) = router
        .replace_native_shard_registration(&lease, &lane)
        .unwrap()
        .into_parts();
    let shard = std::mem::replace(&mut env.shard, SimShard::new(ShardPos::new(0, 0)));
    (env, router, NativeShardSlot { shard, lane }, key)
}

#[tokio::test]
async fn revalidation_closure_takes_a_shared_shard_borrow_beside_the_lane_borrow() {
    for store in STORES {
        let (_env, mut router, slot, key) = suspended_slot(store).await;
        let report = router
            .revalidate_native_endpoints(&slot.lane, &mut |handle, probe| {
                probe.submit(slot.shard.connected_owned_player_view(handle).ok())
            })
            .unwrap();
        assert_eq!(report.revalidated(), 1);
        assert_eq!(
            router.native_session_status(&key).unwrap().endpoint(),
            NativeEndpointState::Bound
        );
    }
}

#[tokio::test]
async fn revalidation_closure_takes_an_exclusive_shard_borrow_and_mutates_after_submit() {
    for store in STORES {
        let (env, mut router, mut slot, key) = suspended_slot(store).await;
        let shard = &mut slot.shard;
        let mut mutated = 0;
        let report = router
            .revalidate_native_endpoints(&slot.lane, &mut |handle, probe| {
                // The view is derived from the exclusive borrow and dropped by
                // `submit`; the same borrow then mutates the shard.
                let verdict = probe.submit(shard.connected_owned_player_view(handle).ok());
                shard
                    .enqueue_owned_player_input(OwnedPlayerInput::drain())
                    .unwrap();
                mutated += 1;
                verdict
            })
            .unwrap();
        assert_eq!(report.revalidated(), 1);
        assert_eq!(mutated, 1);
        assert_eq!(
            router.native_session_status(&key).unwrap().endpoint(),
            NativeEndpointState::Bound
        );
        // The mutation is genuine: the next boundary applies the drain.
        let out = slot
            .shard
            .run_tick_with_player_ownership_at(env.tick.next().unwrap())
            .unwrap();
        assert!(out
            .intents()
            .iter()
            .any(|intent| matches!(intent, OwnershipIntent::Draining)));
    }
}

/// One normal boundary of the slot's shard at the env's next tick.
fn boundary(
    slot: &mut NativeShardSlot,
    env: &mut SimEnv,
) -> ferrumc_sim::PlayerOwnershipTickOutcome {
    let tick = env.tick.next().unwrap();
    let out = slot.shard.run_tick_with_player_ownership_at(tick).unwrap();
    env.tick = tick;
    out
}

/// Root decision 2(a) (NET-04.2e2): D3 with a shared shard borrow and D7/D8 as
/// one closure with the exclusive shard borrow (view, owned ticket, then the
/// acknowledgement), each disjoint from a live borrow of the slot's lane, and
/// the release closure on the same exclusive borrow.
#[tokio::test]
async fn lifecycle_closures_take_shared_and_exclusive_shard_borrows_beside_the_lane() {
    for store in STORES {
        let mut env = SimEnv::new(store).await;
        let mut router = router(64, 16, 8);
        let (_, _, lane) = router
            .register_native_world_shard(home(), generation())
            .unwrap()
            .into_parts();
        let admitted = join(&mut router, &mut env, &lane, "Borrow", spawn()).await;
        let key = admitted.session.session_key().unwrap();
        let shard = std::mem::replace(&mut env.shard, SimShard::new(ShardPos::new(0, 0)));
        let mut slot = NativeShardSlot { shard, lane };
        let token = router.disconnect_native(&key).unwrap();
        while let Ok(item) = slot.lane.try_recv() {
            slot.shard
                .enqueue_owned_player_input(item.into_parts().2)
                .unwrap();
        }
        let out = boundary(&mut slot, &mut env);
        assert!(out
            .intents()
            .iter()
            .any(|i| matches!(i, OwnershipIntent::PlayerDetached { .. })));
        assert_eq!(
            router.native_detached(key.player(), &mut |handle, probe| {
                probe.submit(slot.shard.collector_lifecycle_view(handle).ok())
            }),
            Ok(ferrumc_session::NativeDetachOutcome::Confirmed)
        );
        // Leave-save with the status closure on a shared borrow.
        let id = env.fresh_id();
        let step = router
            .service_native_retention(
                token,
                1,
                true,
                &mut |h| slot.shard.detached_owned_player_status(h),
                &mut || id,
            )
            .unwrap();
        let ferrumc_session::NativeRetentionStep::Submit(input) = step else {
            panic!("leave-save submit");
        };
        slot.shard
            .enqueue_owned_player_input(input.into_parts().2)
            .unwrap();
        let mut out = boundary(&mut slot, &mut env);
        let completion = out.take_work().remove(0).execute(&env.store).await.unwrap();
        slot.shard
            .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
            .unwrap();
        boundary(&mut slot, &mut env);
        assert!(matches!(
            router.service_native_retention(
                token,
                2,
                true,
                &mut |h| slot.shard.detached_owned_player_status(h),
                &mut || unreachable!("confirmed"),
            ),
            Ok(ferrumc_session::NativeRetentionStep::SaveConfirmed)
        ));
        assert_eq!(
            router
                .retire_detached_native_sessions(&mut |handle, probe| {
                    probe.submit(slot.shard.collector_lifecycle_view(handle).ok())
                })
                .map(ferrumc_session::NativeRetirementReport::retired),
            Ok(1)
        );
        // D7/D8: shared view, owned ticket, then the exclusive acknowledgement,
        // while `lane` (a borrow of another slot field) stays alive.
        let lane = &slot.lane;
        let shard = &mut slot.shard;
        let report = router
            .acknowledge_retired_native_records(&mut |handle, probe| {
                probe
                    .submit(shard.collector_lifecycle_view(handle).ok())
                    .acknowledge(|ticket| shard.acknowledge_collector_lifecycle(ticket))
            })
            .unwrap();
        assert_eq!(report.acknowledged(), 1);
        // The lane borrow was alive across the exclusive shard borrow.
        std::hint::black_box(lane);
        assert_eq!(
            router.release_native_retention(token, &mut |handle| {
                shard.release_detached_owned_player(handle)
            }),
            Ok(())
        );
    }
}
