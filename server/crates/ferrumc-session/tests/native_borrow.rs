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
