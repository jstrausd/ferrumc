//! NET-04.2e1: endpoint replacement, suspension, closure revalidation
//! (Root R3-A1) and unregistration with genuine memory and redb admissions.

mod native_support;

use ferrumc_math::Vec3;
use ferrumc_net::Criticality;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, ServerboundPlayPacket, ServerboundSetHeldItem, SetPlayerPosition,
};
use ferrumc_session::{
    CollectorSessionError, NativeDisconnect, NativeEndpointState, NativeInputClass, NativeLaneRecv,
    NativeRouted, NativeSessionPhase, ShardDirectoryError,
};
use ferrumc_sim::PlayerOwnershipError;
use native_support::*;

fn at(x: f64, z: f64) -> ServerboundPlayPacket {
    ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(x, 64.0, z, 0))
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    reason = "one replacement scenario across five sessions A-E"
)]
async fn replacement_suspends_and_closure_revalidation_rebinds_active_and_leaving_sessions() {
    for store in STORES {
        let mut env = SimEnv::new(store).await;
        let mut router = router(64, 16, 8);
        let registration = router
            .register_native_world_shard(home(), generation())
            .unwrap();
        let lease = registration.lease();
        let (_, _, mut old_lane) = registration.into_parts();
        let positions = [
            Vec3::new(8.5, 64.0, 8.5),
            Vec3::new(20.5, 64.0, 8.5),
            Vec3::new(30.5, 64.0, 8.5),
            Vec3::new(40.5, 64.0, 8.5),
            Vec3::new(50.5, 64.0, 8.5),
        ];
        let mut sessions = Vec::new();
        for (name, position) in ["A", "B", "C", "D", "E"].into_iter().zip(positions) {
            let mut admitted = join(&mut router, &mut env, &old_lane, name, position).await;
            let _join_game = admitted.session.try_recv().unwrap();
            sessions.push(admitted);
        }
        // NET-04.2e2: native sessions see each other; every pair is in view.
        for admitted in &mut sessions {
            assert_eq!(drain_peer_burst(&mut admitted.session), 4 * 2);
        }
        let key = |i: usize| sessions[i].session.session_key().unwrap();
        let (a, b, c, d, e) = (key(0), key(1), key(2), key(3), key(4));

        // Old lane items: A moves, B leaves (its disconnect is not applied yet).
        assert_eq!(
            router.route_native_play(&a, &at(9.5, 9.5)),
            Ok(NativeRouted::Queued)
        );
        assert_eq!(disconnect(&mut router, &b).1, Some(NativeDisconnect::Sent));

        // Refused replacements leave the endpoint unchanged.
        let mut foreign = native_support::router(64, 16, 8);
        let foreign_registration = foreign
            .register_native_world_shard(home(), generation())
            .unwrap();
        assert_eq!(
            router
                .replace_native_shard_registration(&foreign_registration.lease(), &old_lane)
                .err(),
            Some(CollectorSessionError::Directory(
                ShardDirectoryError::ForeignLease {
                    coverage: lease.coverage()
                }
            ))
        );
        let (_, _, foreign_lane) = foreign_registration.into_parts();
        assert_eq!(
            router
                .replace_native_shard_registration(&lease, &foreign_lane)
                .err(),
            Some(CollectorSessionError::StaleReceiver)
        );
        for session in [&a, &c, &d, &e] {
            assert_eq!(
                router.native_session_status(session).unwrap().endpoint(),
                NativeEndpointState::Bound
            );
        }

        let replaced = router
            .replace_native_shard_registration(&lease, &old_lane)
            .unwrap();
        let new_lease = replaced.lease();
        assert_eq!(new_lease.registration_id(), lease.registration_id());
        assert_ne!(new_lease.generation(), lease.generation());
        let (_, _, mut lane) = replaced.into_parts();
        assert!(matches!(
            router.replace_native_shard_registration(&lease, &lane),
            Err(CollectorSessionError::Directory(
                ShardDirectoryError::StaleLease { .. }
            ))
        ));
        for session in [&a, &b, &c, &d, &e] {
            assert_eq!(
                router.native_session_status(session).unwrap().endpoint(),
                NativeEndpointState::Suspended
            );
        }

        // Routing while suspended: movement discarded, authoritative input
        // answered later by a resync, disconnect retained.
        assert_eq!(
            router.route_native_play(&a, &at(10.5, 10.5)),
            Err(CollectorSessionError::EndpointSuspended)
        );
        assert_eq!(
            router.route_native_play(
                &a,
                &ServerboundPlayPacket::ServerboundSetHeldItem(ServerboundSetHeldItem::new(3))
            ),
            Err(CollectorSessionError::EndpointSuspended)
        );
        assert_eq!(
            disconnect(&mut router, &c).1,
            Some(NativeDisconnect::Retained)
        );
        assert_eq!(
            router.retry_native_disconnects(),
            1,
            "suspended: still retained"
        );
        // The revalidation must stay a probe-only path: D's genuine player
        // detaches in the simulation, E is probed with a foreign view.
        env.detach_in_sim(&sessions[3].handle);

        // The driver drains the old receiver completely before the new one.
        let mut old_items = Vec::new();
        loop {
            match old_lane.try_recv() {
                Ok(input) => {
                    let (key, class, input) = input.into_parts();
                    old_items.push((key, class));
                    env.shard.enqueue_owned_player_input(input).unwrap();
                }
                Err(NativeLaneRecv::Disconnected) => break,
                Err(other) => panic!("old lane: {other:?}"),
            }
        }
        assert_eq!(
            old_items,
            [
                (a.clone(), NativeInputClass::Data),
                (b.clone(), NativeInputClass::Control)
            ]
        );
        assert!(matches!(lane.try_recv(), Err(NativeLaneRecv::Empty)));

        // Revalidation in the same iteration, before the boundary applies B's
        // disconnect: B is a leaving session that still has a live view.
        let a_handle = sessions[0].handle.clone();
        let e_player = sessions[4].handle.player();
        let shard = &env.shard;
        let report = router
            .revalidate_native_endpoints(&lane, &mut |handle, probe| {
                if handle.player() == e_player {
                    probe.submit(shard.connected_owned_player_view(&a_handle).ok())
                } else {
                    probe.submit(shard.connected_owned_player_view(handle).ok())
                }
            })
            .unwrap();
        assert_eq!(report.revalidated(), 3, "A, B (leaving) and C");
        assert_eq!(report.detached(), 1, "D");
        assert_eq!(report.refused(), 1, "E: NativeEndpointRevalidationRefused");
        assert_eq!(report.open(), 0);

        let status = |k| router.native_session_status(k).unwrap();
        assert_eq!(status(&a).endpoint(), NativeEndpointState::Bound);
        assert_eq!(status(&a).phase(), NativeSessionPhase::Active);
        assert_eq!(status(&b).endpoint(), NativeEndpointState::Bound);
        assert_eq!(status(&b).phase(), NativeSessionPhase::Leaving);
        assert_eq!(status(&c).disconnect(), Some(NativeDisconnect::Sent));
        assert_eq!(status(&d).endpoint(), NativeEndpointState::Detached);
        assert_eq!(status(&d).phase(), NativeSessionPhase::Leaving);
        assert_eq!(
            status(&e).endpoint(),
            NativeEndpointState::RevalidationRefused
        );
        assert_eq!(status(&e).phase(), NativeSessionPhase::Leaving);
        // A2 / Root 2(d): the refused session's disconnect from its stored
        // handle went to the current lane (e1: retained forever).
        assert_eq!(status(&e).disconnect(), Some(NativeDisconnect::Sent));
        assert_eq!(router.retry_native_disconnects(), 0);

        // A gets a mandatory sync to its genuine pose for the refused selection.
        let sync = sessions[0]
            .session
            .try_recv()
            .expect("resync after revalidation");
        assert_eq!(sync.criticality(), Criticality::Mandatory);
        let ClientboundPlayPacket::SynchronizePlayerPosition(sync) = sync.packet() else {
            panic!("resync must be a position sync");
        };
        assert_eq!(
            sync.x().to_bits(),
            8.5_f64.to_bits(),
            "genuine pose before the move"
        );
        // E's own outbound lane is closed by the controlled leave.
        assert!(sessions[4].session.try_recv().is_none());

        // C's retained disconnect is the first item on the new lane, then E's
        // disconnect from its stored handle (A2), then new A input.
        assert_eq!(
            router.route_native_play(&a, &at(11.5, 11.5)),
            Ok(NativeRouted::Queued)
        );
        let first = lane.try_recv().unwrap();
        assert_eq!(first.session(), &c);
        assert_eq!(first.class(), NativeInputClass::Control);
        let refused = lane.try_recv().unwrap();
        assert_eq!(refused.session(), &e);
        assert_eq!(refused.class(), NativeInputClass::Control);
        let second = lane.try_recv().unwrap();
        assert_eq!(second.session(), &a);
        for item in [first, refused, second] {
            env.shard
                .enqueue_owned_player_input(item.into_parts().2)
                .unwrap();
        }
        let mut out = env.boundary();
        assert!(out.take_refusals().is_empty());
        assert_eq!(
            env.position(&sessions[0].handle),
            Vec3::new(11.5, 64.0, 11.5)
        );
        // The genuine owner applied E's disconnect: the normal detach path.
        assert_eq!(
            router.native_detached(e_player, &mut |handle, probe| {
                probe.submit(env.shard.collector_lifecycle_view(handle).ok())
            }),
            Ok(ferrumc_session::NativeDetachOutcome::Confirmed)
        );
        for detached in [1, 2, 3, 4] {
            assert!(matches!(
                env.shard
                    .connected_owned_player_view(&sessions[detached].handle),
                Err(PlayerOwnershipError::Ineligible(_))
            ));
        }

        // Liveness of held lane inputs after revalidation.
        assert!(router.native_lane_input_live(&a, NativeInputClass::Data));
        assert!(!router.native_lane_input_live(&b, NativeInputClass::Data));
        assert!(router.native_lane_input_live(&b, NativeInputClass::Control));
        assert!(!router.native_lane_input_live(&d, NativeInputClass::Control));
        assert!(!router.native_lane_input_live(&e, NativeInputClass::Control));

        // Revalidation with a stale receiver is refused; a second call has no work.
        assert_eq!(
            router
                .revalidate_native_endpoints(&old_lane, &mut |_, probe| probe.submit(None))
                .err(),
            Some(CollectorSessionError::StaleReceiver)
        );
        let again = router
            .revalidate_native_endpoints(&lane, &mut |_, _| unreachable!("no suspended session"))
            .unwrap();
        assert_eq!(again.revalidated() + again.refused() + again.detached(), 0);

        // Unregister refuses while sessions reference the registration and
        // returns the receiver intact.
        let refused = router
            .unregister_native_shard_registration(&new_lease, lane)
            .unwrap_err();
        assert_eq!(
            refused.error(),
            &CollectorSessionError::NativeEndpointInUse {
                registration_id: new_lease.registration_id(),
                sessions: 5,
                reservations: 0,
            }
        );
        let (_, mut lane) = refused.into_parts();
        assert_eq!(
            router.route_native_play(&a, &at(12.5, 12.5)),
            Ok(NativeRouted::Queued)
        );
        assert!(
            lane.try_recv().is_ok(),
            "the returned receiver is still the live lane"
        );
        let unregister_stale = router
            .unregister_native_shard_registration(&lease, lane)
            .unwrap_err();
        assert!(matches!(
            unregister_stale.error(),
            CollectorSessionError::Directory(ShardDirectoryError::StaleLease { .. })
        ));
    }
}

#[tokio::test]
async fn replacement_without_sessions_and_unregister_success() {
    let mut router = router(64, 16, 8);
    let registration = router
        .register_native_world_shard(home(), generation())
        .unwrap();
    let lease = registration.lease();
    let (_, _, lane) = registration.into_parts();
    let replaced = router
        .replace_native_shard_registration(&lease, &lane)
        .unwrap();
    let report = router
        .revalidate_native_endpoints(&lane, &mut |_, probe| probe.submit(None))
        .err();
    assert_eq!(report, Some(CollectorSessionError::StaleReceiver));
    let new_lease = replaced.lease();
    let (_, _, current) = replaced.into_parts();
    let refused = router
        .unregister_native_shard_registration(&new_lease, lane)
        .unwrap_err();
    assert_eq!(refused.error(), &CollectorSessionError::StaleReceiver);
    assert_eq!(
        router
            .unregister_native_shard_registration(&new_lease, current)
            .map_err(|e| e.into_parts().0),
        Ok(home())
    );
    // Unregister/register creates a new lineage.
    let again = router
        .register_native_world_shard(home(), generation())
        .unwrap();
    assert_ne!(again.lease().registration_id(), new_lease.registration_id());
}
