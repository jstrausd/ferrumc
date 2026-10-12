//! NET-04.2e1: native routing over real bounded lanes into genuine admissions,
//! movement coalescing (Root R3-A2 / F4), region pre-check (A10), lane-full
//! dispositions and the driver's held input (R2-A5).
#![allow(clippy::float_cmp, reason = "positions round-trip exact bit patterns")]

mod native_support;

use ferrumc_core::GameMode;
use ferrumc_math::Vec3;
use ferrumc_net::Criticality;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, ConfirmTeleportation, PlayerAction, ServerboundKeepAlive,
    ServerboundPlayPacket, ServerboundSetHeldItem, SetPlayerPosition, SetPlayerPositionAndRotation,
    SetPlayerRotation,
};
use ferrumc_session::{
    CollectorSessionError, NativeDisconnect, NativeInputClass, NativeLaneRecv, NativeRouted,
    NativeSessionPhase, NativeShardReceiver, PlayerSessionHandle, PlayerSessionKey, SessionRouter,
};
use ferrumc_sim::{OwnedPlayerInput, PlayerOwnershipError};
use native_support::*;

fn at(x: f64, z: f64) -> ServerboundPlayPacket {
    ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(x, 64.0, z, 0))
}
fn at_look(x: f64, yaw: f32, pitch: f32) -> ServerboundPlayPacket {
    ServerboundPlayPacket::SetPlayerPositionAndRotation(SetPlayerPositionAndRotation::new(
        x, 64.0, 8.5, yaw, pitch, 0,
    ))
}
fn look(yaw: f32, pitch: f32) -> ServerboundPlayPacket {
    ServerboundPlayPacket::SetPlayerRotation(SetPlayerRotation::new(yaw, pitch, 0))
}
fn held(slot: i16) -> ServerboundPlayPacket {
    ServerboundPlayPacket::ServerboundSetHeldItem(ServerboundSetHeldItem::new(slot))
}

/// Movement steps the concurrent producer routes at least.
const STEPS: u32 = 2_000;
/// Upper bound of producer steps while waiting for both race branches; the
/// last position `1 + MAX_STEPS * STEP_X` stays inside the home region.
const MAX_STEPS: u32 = 100_000;
/// Position increment per step, a power of two so every position is exact.
const STEP_X: f64 = 1.0 / 1024.0;

struct Fixture {
    env: SimEnv,
    router: SessionRouter,
    lane: NativeShardReceiver,
    handle: ferrumc_sim::OwnedPlayerHandle,
    session: PlayerSessionHandle,
    key: PlayerSessionKey,
}

async fn fixture(store: Store, lane: usize, reserve: usize, inputs: usize) -> Fixture {
    let mut env = SimEnv::with_inputs(store, inputs).await;
    let mut router = router(lane, reserve, 8);
    let (_, _, lane) = router
        .register_native_world_shard(home(), generation())
        .unwrap()
        .into_parts();
    let admitted = join(&mut router, &mut env, &lane, "Mover", spawn()).await;
    let mut session = admitted.session;
    let _join_game = session.try_recv().unwrap();
    let key = session.session_key().unwrap();
    Fixture {
        env,
        router,
        lane,
        handle: admitted.handle,
        session,
        key,
    }
}

/// Drains the lane, returning each input's key and class while the inputs
/// themselves go to the genuine simulation queue.
fn drain_into_sim(f: &mut Fixture) -> Vec<NativeInputClass> {
    let mut classes = Vec::new();
    while let Ok(input) = f.lane.try_recv() {
        assert_eq!(input.session(), &f.key);
        let (_, class, input) = input.into_parts();
        classes.push(class);
        f.env.shard.enqueue_owned_player_input(input).unwrap();
    }
    classes
}

#[tokio::test]
async fn routed_movement_and_selection_reach_the_genuine_admission_on_both_backends() {
    for store in STORES {
        let mut f = fixture(store, 64, 16, 32).await;
        assert_eq!(
            f.router.route_native_play(&f.key, &at(20.25, 30.5)),
            Ok(NativeRouted::Queued)
        );
        assert_eq!(
            f.router.route_native_play(&f.key, &held(4)),
            Ok(NativeRouted::Queued)
        );
        assert_eq!(
            f.router.route_native_game_mode(&f.key, GameMode::Creative),
            Ok(NativeRouted::Queued)
        );
        assert_eq!(
            drain_into_sim(&mut f),
            [
                NativeInputClass::Data,
                NativeInputClass::Data,
                NativeInputClass::Data
            ]
        );
        let mut out = f.env.boundary();
        assert!(out.take_refusals().is_empty());
        let view = f.env.shard.connected_owned_player_view(&f.handle).unwrap();
        assert_eq!(view.position(), Vec3::new(20.25, 64.0, 30.5));
        assert_eq!(view.game_mode(), GameMode::Creative);
        let selected = f.env.persisted_selected(&f.handle.clone()).await;
        assert_eq!(selected, 4);
        // Keep-alive has no simulation effect.
        assert_eq!(
            f.router.route_native_play(
                &f.key,
                &ServerboundPlayPacket::ServerboundKeepAlive(ServerboundKeepAlive::new(1))
            ),
            Ok(NativeRouted::Ignored)
        );
        assert!(matches!(f.lane.try_recv(), Err(NativeLaneRecv::Empty)));
    }
}

#[tokio::test]
async fn movement_spam_uses_one_lane_slot_and_delivers_the_newest_state() {
    for store in STORES {
        let mut f = fixture(store, 8, 2, 32).await;
        assert_eq!(
            f.router.route_native_play(&f.key, &at(9.0, 9.0)),
            Ok(NativeRouted::Queued)
        );
        for step in 1..=200_u32 {
            let x = 9.0 + f64::from(step) * 0.25;
            assert_eq!(
                f.router.route_native_play(&f.key, &at(x, 9.0)),
                Ok(NativeRouted::Coalesced),
                "step {step}"
            );
        }
        // Rotation-only movement merges per field without dropping the position.
        assert_eq!(
            f.router.route_native_play(&f.key, &look(90.0, -15.5)),
            Ok(NativeRouted::Coalesced)
        );
        assert_eq!(drain_into_sim(&mut f), [NativeInputClass::Data]);
        let mut out = f.env.boundary();
        assert!(out.take_refusals().is_empty());
        let view = f.env.shard.connected_owned_player_view(&f.handle).unwrap();
        assert_eq!(view.position(), Vec3::new(59.0, 64.0, 9.0));
        assert_eq!(view.look(), [90.0, -15.5]);
    }
}

#[tokio::test]
async fn movement_behind_a_non_movement_input_is_never_pulled_forward() {
    for store in STORES {
        let mut f = fixture(store, 16, 2, 32).await;
        f.router.route_native_play(&f.key, &at(10.0, 10.0)).unwrap();
        f.router.route_native_play(&f.key, &at(11.0, 10.0)).unwrap();
        f.router.route_native_play(&f.key, &held(2)).unwrap();
        assert_eq!(
            f.router.route_native_play(&f.key, &at(12.0, 10.0)),
            Ok(NativeRouted::Queued)
        );
        assert_eq!(
            f.router.route_native_play(&f.key, &at(13.0, 10.0)),
            Ok(NativeRouted::Coalesced)
        );
        let mut order = Vec::new();
        while let Ok(input) = f.lane.try_recv() {
            let (_, _, input) = input.into_parts();
            order.push(format!("{input:?}"));
        }
        assert_eq!(order.len(), 3);
        assert!(order[0].contains("Move") && order[0].contains("11.0"));
        assert!(order[1].contains("Select"));
        assert!(order[2].contains("Move") && order[2].contains("13.0"));
    }
}

#[tokio::test]
async fn teleport_confirm_resync_and_region_correction_close_the_cell() {
    for store in STORES {
        let mut f = fixture(store, 16, 2, 32).await;
        // Teleport confirmation.
        f.router.route_native_play(&f.key, &at(10.0, 10.0)).unwrap();
        assert_eq!(
            f.router.route_native_play(
                &f.key,
                &ServerboundPlayPacket::ConfirmTeleportation(ConfirmTeleportation::new(1))
            ),
            Ok(NativeRouted::Barrier)
        );
        assert_eq!(
            f.router.route_native_play(&f.key, &at(11.0, 10.0)),
            Ok(NativeRouted::Queued)
        );
        // Region correction (a resync to the genuine pose).
        assert_eq!(
            f.router.route_native_play(&f.key, &at(200.0, 10.0)),
            Err(CollectorSessionError::NativeRegionBoundary)
        );
        assert_eq!(
            f.router.route_native_play(&f.key, &at(12.0, 10.0)),
            Ok(NativeRouted::Queued)
        );
        let mut count = 0;
        while f.lane.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 3, "each barrier forced a new lane entry");
    }
}

#[tokio::test]
async fn take_before_replace_yields_old_then_new_and_replace_before_take_yields_new() {
    for store in STORES {
        let mut f = fixture(store, 16, 2, 32).await;
        // Take first: the old state is delivered and the newer one gets its own entry.
        f.router.route_native_play(&f.key, &at(10.0, 10.0)).unwrap();
        let first = format!("{:?}", f.lane.try_recv().unwrap().into_parts().2);
        assert_eq!(
            f.router.route_native_play(&f.key, &at(11.0, 10.0)),
            Ok(NativeRouted::Queued)
        );
        let second = format!("{:?}", f.lane.try_recv().unwrap().into_parts().2);
        assert!(first.contains("10.0") && second.contains("11.0"));
        // Replace first: only the newer state is delivered.
        f.router.route_native_play(&f.key, &at(12.0, 10.0)).unwrap();
        assert_eq!(
            f.router.route_native_play(&f.key, &at(13.0, 10.0)),
            Ok(NativeRouted::Coalesced)
        );
        let third = format!("{:?}", f.lane.try_recv().unwrap().into_parts().2);
        assert!(third.contains("13.0") && !third.contains("12.0"));
        assert!(matches!(f.lane.try_recv(), Err(NativeLaneRecv::Empty)));
    }
}

/// What the concurrent producer observed: its last step and how often each
/// race branch decided a step.
struct RaceCounts {
    last_step: u32,
    /// `Queued` after the first step: the receiver took the pending entry
    /// before the router could replace it (take before replace).
    take_before_replace: u32,
    /// `Coalesced`: the router replaced the still-pending entry before the
    /// receiver took it (replace before take).
    replace_before_take: u32,
}

#[tokio::test]
async fn concurrent_take_and_replace_never_lose_or_reorder_the_newest_state() {
    for store in STORES {
        let f = fixture(store, 1024, 16, 32).await;
        let Fixture {
            mut router,
            mut lane,
            key,
            env,
            ..
        } = f;
        let (counts, taken) = std::thread::scope(|scope| {
            let producer = scope.spawn(|| {
                let mut counts = RaceCounts {
                    last_step: 0,
                    take_before_replace: 0,
                    replace_before_take: 0,
                };
                // Route at least STEPS, then until both branches were decided
                // concurrently, bounded by MAX_STEPS.
                for step in 1..=MAX_STEPS {
                    let x = 1.0 + f64::from(step) * STEP_X;
                    match router.route_native_play(&key, &at(x, 9.0)).unwrap() {
                        NativeRouted::Queued if step == 1 => {}
                        NativeRouted::Queued => counts.take_before_replace += 1,
                        NativeRouted::Coalesced => counts.replace_before_take += 1,
                        other => panic!("step {step}: unexpected {other:?}"),
                    }
                    counts.last_step = step;
                    if step >= STEPS {
                        if counts.take_before_replace > 0 && counts.replace_before_take > 0 {
                            break;
                        }
                        // On a loaded host the receiver thread may not run at
                        // all during a fast producer burst; yielding lets it
                        // take between steps. The race stays unsynchronized.
                        std::thread::yield_now();
                    }
                }
                counts
            });
            let mut taken = Vec::new();
            loop {
                match lane.try_recv() {
                    Ok(input) => taken.push(format!("{:?}", input.into_parts().2)),
                    Err(NativeLaneRecv::Empty) if producer.is_finished() => break,
                    Err(NativeLaneRecv::Empty) => std::thread::yield_now(),
                    Err(other) => panic!("unexpected {other:?}"),
                }
            }
            while let Ok(input) = lane.try_recv() {
                taken.push(format!("{:?}", input.into_parts().2));
            }
            (producer.join().unwrap(), taken)
        });
        drop(env);
        let xs: Vec<f64> = taken
            .iter()
            .map(|debug| {
                let start = debug.find("x: ").unwrap() + 3;
                debug[start..].split(',').next().unwrap().parse().unwrap()
            })
            .collect();
        // Kept in the captured output as the race evidence of this run.
        eprintln!(
            "{store:?}: steps {}, take before replace {}, replace before take {}",
            counts.last_step, counts.take_before_replace, counts.replace_before_take
        );
        // Both branches really happened while the threads raced.
        assert!(
            counts.take_before_replace > 0,
            "{store:?}: take before replace never observed in {} steps",
            counts.last_step
        );
        assert!(
            counts.replace_before_take > 0,
            "{store:?}: replace before take never observed in {} steps",
            counts.last_step
        );
        assert_eq!(
            counts.take_before_replace + counts.replace_before_take + 1,
            counts.last_step
        );
        // Every queued entry was taken exactly once; coalesced steps took no slot.
        assert_eq!(xs.len(), counts.take_before_replace as usize + 1);
        assert!(
            xs.windows(2).all(|w| w[0] < w[1]),
            "strictly newer each take"
        );
        assert_eq!(
            *xs.last().unwrap(),
            1.0 + f64::from(counts.last_step) * STEP_X
        );
    }
}

/// Positions around every region edge of shard (0, 0), blocks `[0, 128)`.
fn edge_positions() -> Vec<(f64, f64)> {
    let eps = 1e-9;
    vec![
        (0.0, 8.0),
        (-0.0, 8.0),
        (-eps, 8.0),
        (eps, 8.0),
        (127.999_999, 8.0),
        (128.0 - eps, 8.0),
        (128.0, 8.0),
        (128.0 + eps, 8.0),
        (8.0, -eps),
        (8.0, 127.5),
        (8.0, 128.0),
        (15.999_999_999, 16.0),
        (16.0, 15.999_999_999),
    ]
}

#[tokio::test]
async fn region_decisions_match_the_simulation_derivation_on_both_backends() {
    for store in STORES {
        let mut f = fixture(store, 64, 16, 32).await;
        for (x, z) in edge_positions() {
            let position = Vec3::new(x, 64.0, z);
            let routed = f.router.route_native_play(&f.key, &at(x, z));
            // The simulation's own decision for the same position.
            f.env
                .shard
                .enqueue_owned_player_input(OwnedPlayerInput::move_player(
                    f.handle.clone(),
                    Some(position),
                    [None, None],
                ))
                .unwrap();
            let mut probe = f.env.boundary();
            let sim_refused = probe
                .take_refusals()
                .into_iter()
                .any(|r| matches!(r.into_parts().0, PlayerOwnershipError::Ineligible(_)));
            match routed {
                Ok(NativeRouted::Queued) => {
                    assert!(!sim_refused, "({x}, {z}) session accepted, sim refused");
                    let mut out = pump(&mut f.env, &mut f.lane);
                    assert!(out.take_refusals().is_empty());
                    assert_eq!(f.env.position(&f.handle).x.to_bits(), x.to_bits());
                }
                Err(CollectorSessionError::NativeRegionBoundary) => {
                    assert!(sim_refused, "({x}, {z}) session refused, sim accepted");
                    let correction = f.session.try_recv().expect("mandatory correction");
                    assert_eq!(correction.criticality(), Criticality::Mandatory);
                    let ClientboundPlayPacket::SynchronizePlayerPosition(sync) =
                        correction.packet()
                    else {
                        panic!("correction must be a position sync");
                    };
                    // The last genuine pose the session observed (admission).
                    assert_eq!(sync.x().to_bits(), spawn().x.to_bits());
                    let notice = f.session.try_recv().expect("region notice");
                    assert_eq!(notice.criticality(), Criticality::Droppable);
                    assert!(matches!(
                        notice.packet(),
                        ClientboundPlayPacket::SetActionBarText(_)
                    ));
                    assert!(matches!(f.lane.try_recv(), Err(NativeLaneRecv::Empty)));
                }
                other => panic!("({x}, {z}) unexpected {other:?}"),
            }
            // Return the sim player into the region for the next probe.
            f.env
                .shard
                .enqueue_owned_player_input(OwnedPlayerInput::move_player(
                    f.handle.clone(),
                    Some(spawn()),
                    [None, None],
                ))
                .unwrap();
            f.env.boundary();
        }
    }
}

#[tokio::test]
async fn invalid_transforms_are_corrected_without_a_lane_item() {
    for store in STORES {
        let mut f = fixture(store, 64, 16, 32).await;
        let invalid = [
            at(f64::NAN, 8.0),
            at(f64::INFINITY, 8.0),
            at(3.1e7, 8.0),
            at(8.0, -3.1e7),
            look(f32::NAN, 0.0),
            at_look(8.0, 0.0, f32::INFINITY),
        ];
        for packet in invalid {
            assert_eq!(
                f.router.route_native_play(&f.key, &packet),
                Err(CollectorSessionError::InvalidTransform)
            );
            let correction = f.session.try_recv().unwrap();
            assert!(matches!(
                correction.packet(),
                ClientboundPlayPacket::SynchronizePlayerPosition(_)
            ));
            assert!(f.session.try_recv().is_some(), "droppable notice");
        }
        assert!(matches!(f.lane.try_recv(), Err(NativeLaneRecv::Empty)));
    }
}

#[tokio::test]
async fn malformed_and_unsupported_client_events_are_refused_typed() {
    let mut f = fixture(Store::Memory, 64, 16, 32).await;
    for slot in [-1_i16, 9, i16::MIN, i16::MAX] {
        assert_eq!(
            f.router.route_native_play(&f.key, &held(slot)),
            Err(CollectorSessionError::InvalidHotbarSlot { slot })
        );
    }
    for slot in [0_i16, 8] {
        assert_eq!(
            f.router.route_native_play(&f.key, &held(slot)),
            Ok(NativeRouted::Queued)
        );
    }
    let dig = ServerboundPlayPacket::PlayerAction(PlayerAction::new(
        0,
        ferrumc_proto::types::BlockPosition::new(1, 63, 1),
        1,
        1,
    ));
    assert_eq!(
        f.router.route_native_play(&f.key, &dig),
        Err(CollectorSessionError::UnsupportedNativeAction(
            "block action"
        ))
    );
    let mut count = 0;
    while f.lane.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 2);
}

#[tokio::test]
async fn foreign_stale_and_leaving_keys_never_address_a_session() {
    let mut f = fixture(Store::Memory, 64, 16, 32).await;
    // Same UUID in another router.
    let mut other_env = SimEnv::new(Store::Memory).await;
    let mut other = router(64, 16, 8);
    let (_, _, other_lane) = other
        .register_native_world_shard(home(), generation())
        .unwrap()
        .into_parts();
    let foreign = join(&mut other, &mut other_env, &other_lane, "Mover", spawn()).await;
    let foreign_key = foreign.session.session_key().unwrap();
    assert_eq!(foreign_key.player(), f.key.player());
    assert_ne!(foreign_key, f.key);
    assert_eq!(
        f.router.route_native_play(&foreign_key, &at(9.0, 9.0)),
        Err(CollectorSessionError::StaleSession)
    );
    assert_eq!(
        f.router.disconnect_native(&foreign_key),
        Err(CollectorSessionError::StaleSession)
    );
    assert!(f.router.native_session_status(&foreign_key).is_none());
    assert!(!f
        .router
        .native_lane_input_live(&foreign_key, NativeInputClass::Control));
    // Leaving sessions accept no gameplay.
    assert_eq!(
        disconnect(&mut f.router, &f.key).1,
        Some(NativeDisconnect::Sent)
    );
    assert_eq!(
        f.router.route_native_play(&f.key, &at(9.0, 9.0)),
        Err(CollectorSessionError::SessionNotActive)
    );
    assert_eq!(
        f.router.route_native_game_mode(&f.key, GameMode::Creative),
        Err(CollectorSessionError::SessionNotActive)
    );
    // Legacy UUID-only state is untouched by native sessions.
    assert!(!f.router.is_player_connected(f.key.player()));
    assert_eq!(f.router.player_count(), 0);
}

#[tokio::test]
async fn disconnect_native_is_idempotent_and_detaches_the_genuine_admission() {
    for store in STORES {
        let mut f = fixture(store, 64, 16, 32).await;
        f.router.route_native_play(&f.key, &at(9.0, 9.0)).unwrap();
        let (token, first) = disconnect(&mut f.router, &f.key);
        assert_eq!(first, Some(NativeDisconnect::Sent));
        assert_eq!(
            disconnect(&mut f.router, &f.key),
            (token, Some(NativeDisconnect::Sent)),
            "idempotent: same token, still one disconnect"
        );
        let status = f.router.native_session_status(&f.key).unwrap();
        assert_eq!(status.phase(), NativeSessionPhase::Leaving);
        assert!(status.ever_active());
        // The session's own outbound lane closed; the entry stays for e2.
        assert!(f.session.try_recv().is_none());
        assert_eq!(f.router.native_session_count(), 1);
        assert_eq!(
            drain_into_sim(&mut f),
            [NativeInputClass::Data, NativeInputClass::Control],
            "exactly one disconnect, behind the earlier movement"
        );
        let mut out = f.env.boundary();
        assert!(out.take_refusals().is_empty());
        assert!(matches!(
            f.env.shard.connected_owned_player_view(&f.handle),
            Err(PlayerOwnershipError::Ineligible(_))
        ));
        let lifecycle = f.env.shard.collector_lifecycle_view(&f.handle).unwrap();
        assert!(lifecycle.detach().is_some(), "genuine detach fact");
    }
}

#[tokio::test]
async fn full_lane_turns_data_into_a_controlled_leave_and_retains_the_disconnect() {
    for store in STORES {
        // Capacity 4, reserve 1: three data slots, one control slot.
        let mut f = fixture(store, 4, 1, 32).await;
        for slot in 0..3 {
            assert_eq!(
                f.router.route_native_play(&f.key, &held(slot)),
                Ok(NativeRouted::Queued)
            );
        }
        assert_eq!(
            f.router.route_native_play(&f.key, &held(3)),
            Err(CollectorSessionError::NativeLaneFull { home: home() })
        );
        let status = f.router.native_session_status(&f.key).unwrap();
        assert_eq!(status.phase(), NativeSessionPhase::Leaving);
        assert_eq!(
            status.disconnect(),
            Some(NativeDisconnect::Sent),
            "used the reserve"
        );
        assert_eq!(
            drain_into_sim(&mut f),
            [
                NativeInputClass::Data,
                NativeInputClass::Data,
                NativeInputClass::Data,
                NativeInputClass::Control
            ]
        );

        // Without reserve room the disconnect is retained and retried, deduplicated.
        let mut g = fixture(store, 4, 1, 32).await;
        let second = {
            let reservation = g
                .router
                .reserve_native_join(player("Second"), generation(), Vec3::new(30.0, 64.0, 30.0))
                .unwrap();
            let handle = g
                .env
                .restore(player("Second"), Vec3::new(30.0, 64.0, 30.0))
                .await;
            admit(&mut g.router, &g.env, &g.lane, &reservation, &handle).unwrap()
        };
        let second_key = second.session_key().unwrap();
        for slot in 0..3 {
            g.router.route_native_play(&g.key, &held(slot)).unwrap();
        }
        assert_eq!(
            disconnect(&mut g.router, &g.key).1,
            Some(NativeDisconnect::Sent)
        );
        let (second_token, retained) = disconnect(&mut g.router, &second_key);
        assert_eq!(retained, Some(NativeDisconnect::Retained));
        assert_eq!(
            disconnect(&mut g.router, &second_key),
            (second_token, Some(NativeDisconnect::Retained))
        );
        assert_eq!(g.router.retry_native_disconnects(), 1);
        let mut controls = 0;
        while let Ok(input) = g.lane.try_recv() {
            controls += usize::from(input.class() == NativeInputClass::Control);
        }
        assert_eq!(controls, 1);
        assert_eq!(g.router.retry_native_disconnects(), 0);
        assert_eq!(
            g.router
                .native_session_status(&second_key)
                .unwrap()
                .disconnect(),
            Some(NativeDisconnect::Sent)
        );
        let mut controls = 0;
        while let Ok(input) = g.lane.try_recv() {
            assert_eq!(input.session(), &second_key);
            controls += usize::from(input.class() == NativeInputClass::Control);
        }
        assert_eq!(
            controls, 1,
            "exactly one retained disconnect, never duplicated"
        );
    }
}

#[tokio::test]
async fn held_lane_input_keeps_its_key_and_fifo_and_is_dropped_once_not_live() {
    for store in STORES {
        // The simulation queue admits one input: the second lane item is refused
        // at enqueue and held by the driver with its key and class.
        let mut f = fixture(store, 64, 16, 1).await;
        f.router.route_native_play(&f.key, &held(1)).unwrap();
        f.router.route_native_play(&f.key, &held(2)).unwrap();
        f.router.route_native_play(&f.key, &at(30.0, 30.0)).unwrap();

        let mut parked: Option<(PlayerSessionKey, NativeInputClass, OwnedPlayerInput)> = None;
        let mut delivered = 0;
        for _iteration in 0..4 {
            // Retry the held input before any further lane item (FIFO per session).
            if let Some((key, class, input)) = parked.take() {
                assert!(f.router.native_lane_input_live(&key, class));
                match f.env.shard.enqueue_owned_player_input(input) {
                    Ok(()) => delivered += 1,
                    Err(refusal) => parked = Some((key, class, refusal.into_parts().1)),
                }
            }
            while parked.is_none() {
                let Ok(input) = f.lane.try_recv() else { break };
                let (key, class, input) = input.into_parts();
                match f.env.shard.enqueue_owned_player_input(input) {
                    Ok(()) => delivered += 1,
                    Err(refusal) => {
                        assert!(matches!(
                            refusal.reason(),
                            PlayerOwnershipError::Capacity(_)
                        ));
                        parked = Some((key, class, refusal.into_parts().1));
                    }
                }
            }
            assert!(parked.iter().count() <= 1);
            let mut out = f.env.boundary();
            assert!(out.take_refusals().is_empty());
        }
        assert_eq!(delivered, 3, "no lane input lost");
        let selected = f.env.persisted_selected(&f.handle.clone()).await;
        assert_eq!(selected, 2);
        assert_eq!(f.env.position(&f.handle), Vec3::new(30.0, 64.0, 30.0));

        // Once the session is leaving, a held data input is no longer live and the
        // driver drops it; a held control input stays live.
        f.router.route_native_play(&f.key, &held(5)).unwrap();
        f.router.route_native_play(&f.key, &held(6)).unwrap();
        let first = f.lane.try_recv().unwrap().into_parts();
        f.env.shard.enqueue_owned_player_input(first.2).unwrap();
        let (key, class, input) = f.lane.try_recv().unwrap().into_parts();
        let refused = f.env.shard.enqueue_owned_player_input(input).unwrap_err();
        assert!(matches!(
            refused.reason(),
            PlayerOwnershipError::Capacity(_)
        ));
        assert!(f.router.native_lane_input_live(&key, class));
        f.router.disconnect_native(&f.key).unwrap();
        assert!(
            !f.router.native_lane_input_live(&key, class),
            "dropped: data of a leaving session"
        );
        assert!(f
            .router
            .native_lane_input_live(&key, NativeInputClass::Control));
    }
}
