//! NET-04.2e2: native player tracking between native sessions (audience and
//! reconcile in both directions), the genuine pose producer (A3), caps
//! (N-1/N/N+1) and the read-only collector delivery disposition including the
//! retirement proof, on genuine memory **and** redb ownership in counter mode.

mod native_support;

use std::num::NonZeroUsize;

use ferrumc_core::GameMode;
use ferrumc_math::Vec3;
use ferrumc_net::Criticality;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, ServerboundPlayPacket, SetPlayerPosition,
};
use ferrumc_session::{
    CollectorDeliveryDisposition, CollectorSessionError, CollectorSessionLimits,
    NativeDetachOutcome, NativePoseOutcome, NativeRouted, NativeSessionPhase, PlayerSessionHandle,
    PlayerSessionKey, SessionRouter,
};
use native_support::e2::{Driver, Serviced};
use native_support::*;

fn nzu(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

/// A router with explicit outbound capacity, view distance and limits.
fn custom_router(
    outbound: usize,
    view_distance: i32,
    limits: CollectorSessionLimits,
) -> SessionRouter {
    let mut router = SessionRouter::with_capacities_and_control_reserve(64, outbound, 16);
    router.set_view_distance(view_distance);
    let (registry, pack) = registries();
    router
        .initialize_collector_sessions(limits, &registry, &pack, SIMULATION_DISTANCE)
        .unwrap();
    router
}

async fn custom_driver(store: Store, router: SessionRouter) -> Driver {
    let env = SimEnv::new(store).await;
    let mut router = router;
    let (_, _, lane) = router
        .register_native_world_shard(home(), generation())
        .unwrap()
        .into_parts();
    Driver {
        router,
        env,
        lane,
        applied: 0,
    }
}

/// Reserve → genuine restore → admission with the router's view distance.
async fn try_join(
    d: &mut Driver,
    name: &str,
    position: Vec3,
) -> (
    ferrumc_sim::OwnedPlayerHandle,
    Result<PlayerSessionHandle, CollectorSessionError>,
    Option<ferrumc_session::NativeJoinReservation>,
) {
    let reservation = d
        .router
        .reserve_native_join(player(name), generation(), position)
        .unwrap();
    let handle = d.env.restore(player(name), position).await;
    d.applied += 1;
    let view = d.env.shard.connected_owned_player_view(&handle).unwrap();
    let mut fields = JoinFields::valid(view.game_mode());
    fields.view_distance = d.router.view_distance();
    let result = d.router.admit_native_join(
        &reservation,
        &d.lane,
        &handle,
        &view,
        fields.build(),
        name,
        Vec::new(),
    );
    let pending = result.is_err().then_some(reservation);
    (handle, result, pending)
}

struct Joined {
    handle: ferrumc_sim::OwnedPlayerHandle,
    session: PlayerSessionHandle,
    key: PlayerSessionKey,
}

async fn join_vd(d: &mut Driver, name: &str, position: Vec3) -> Joined {
    let (handle, session, _) = try_join(d, name, position).await;
    let mut session = session.unwrap();
    assert!(matches!(
        session.try_recv().unwrap().packet(),
        ClientboundPlayPacket::JoinGame(_)
    ));
    let key = session.session_key().unwrap();
    Joined {
        handle,
        session,
        key,
    }
}

fn at(x: f64) -> ServerboundPlayPacket {
    ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(x, 64.0, 8.5, 0))
}

/// Every queued packet of a session, with its criticality.
fn drain(session: &mut PlayerSessionHandle) -> Vec<(Criticality, ClientboundPlayPacket)> {
    let mut out = Vec::new();
    while let Some(message) = session.try_recv() {
        out.push((message.criticality(), message.packet().clone()));
    }
    out
}

fn kinds(packets: &[(Criticality, ClientboundPlayPacket)]) -> Vec<&'static str> {
    packets
        .iter()
        .map(|(_, packet)| match packet {
            ClientboundPlayPacket::PlayerInfoUpdate(_) => "tab-add",
            ClientboundPlayPacket::SpawnEntity(_) => "spawn",
            ClientboundPlayPacket::RemoveEntities(_) => "entity-remove",
            ClientboundPlayPacket::RemovePlayerInfo(_) => "tab-remove",
            ClientboundPlayPacket::UpdateEntityPosition(_)
            | ClientboundPlayPacket::UpdateEntityPositionAndRotation(_)
            | ClientboundPlayPacket::EntityTeleport(_) => "move",
            ClientboundPlayPacket::SetHeadRotation(_) => "head",
            ClientboundPlayPacket::UpdateEntityRotation(_) => "rotation",
            ClientboundPlayPacket::SynchronizePlayerPosition(_) => "sync",
            ClientboundPlayPacket::GameEvent(_) => "game-event",
            ClientboundPlayPacket::SetActionBarText(_) => "notice",
            _ => "other",
        })
        .collect()
}

/// Routes one movement and applies it through a driver iteration (lane →
/// simulation → genuine `PlayerChanged` → pose producer).
fn move_to(d: &mut Driver, key: &PlayerSessionKey, x: f64) {
    assert_eq!(
        d.router.route_native_play(key, &at(x)),
        Ok(NativeRouted::Queued)
    );
    let (_, seen) = d.iterate();
    assert_eq!(seen.changed, 1, "one genuine pose update");
}

fn limits(
    sessions: usize,
    per_viewer: usize,
    total: usize,
    audience: usize,
    work: usize,
) -> CollectorSessionLimits {
    CollectorSessionLimits::new(
        nzu(sessions),
        nzu(per_viewer),
        nzu(total),
        nzu(audience),
        nzu(work),
        nzu(65_536),
    )
    .unwrap()
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "enter, move, leave and re-entry both ways"
)]
async fn native_sessions_track_each_other_in_both_directions_with_reentry() {
    for store in STORES {
        let router = custom_router(64, 2, limits(8, 7, 64, 8, 4096));
        let mut d = custom_driver(store, router).await;
        let mut a = join_vd(&mut d, "TrackA", Vec3::new(8.5, 64.0, 8.5)).await;
        // Chunk 5 vs chunk 0: outside view distance 2, no burst.
        let mut b = join_vd(&mut d, "TrackB", Vec3::new(88.5, 64.0, 8.5)).await;
        assert_eq!(kinds(&drain(&mut a.session)), [] as [&str; 0]);
        assert_eq!(kinds(&drain(&mut b.session)), [] as [&str; 0]);
        assert_eq!(d.router.native_baseline_count(), 0);
        let b_wire = d
            .router
            .native_session_status(&b.key)
            .unwrap()
            .remote_wire_id();
        let a_wire = d
            .router
            .native_session_status(&a.key)
            .unwrap()
            .remote_wire_id();

        // B enters A's view: both directions get tab-add + spawn (mandatory).
        move_to(&mut d, &b.key, 40.5);
        let seen_by_a = drain(&mut a.session);
        assert_eq!(kinds(&seen_by_a), ["tab-add", "spawn"]);
        assert!(seen_by_a.iter().all(|(c, _)| *c == Criticality::Mandatory));
        let ClientboundPlayPacket::SpawnEntity(spawn) = &seen_by_a[1].1 else {
            unreachable!()
        };
        assert_eq!(spawn.entity_id(), b_wire);
        assert_eq!(spawn.x().to_bits(), 40.5_f64.to_bits(), "genuine sim pose");
        let seen_by_b = drain(&mut b.session);
        assert_eq!(kinds(&seen_by_b), ["tab-add", "spawn"]);
        let ClientboundPlayPacket::SpawnEntity(spawn) = &seen_by_b[1].1 else {
            unreachable!()
        };
        assert_eq!(spawn.entity_id(), a_wire);
        assert_eq!(d.router.native_baseline_count(), 2);

        // A move inside view: droppable movement at the viewer only.
        move_to(&mut d, &b.key, 41.5);
        let seen_by_a = drain(&mut a.session);
        assert_eq!(kinds(&seen_by_a), ["move", "head"]);
        assert!(seen_by_a.iter().all(|(c, _)| *c == Criticality::Droppable));
        assert_eq!(
            kinds(&drain(&mut b.session)),
            [] as [&str; 0],
            "no self echo"
        );

        // B leaves view: mandatory entity removes both ways, tab entries stay.
        move_to(&mut d, &b.key, 100.5);
        assert_eq!(kinds(&drain(&mut a.session)), ["entity-remove"]);
        assert_eq!(kinds(&drain(&mut b.session)), ["entity-remove"]);
        assert_eq!(d.router.native_baseline_count(), 2, "still tab-listed");

        // Re-entry: spawn only, no second tab-add.
        move_to(&mut d, &b.key, 30.5);
        assert_eq!(kinds(&drain(&mut a.session)), ["spawn"]);
        assert_eq!(kinds(&drain(&mut b.session)), ["spawn"]);
        assert_eq!(d.router.native_baseline_count(), 2);

        // The viewer moving away reconciles the other direction as well.
        move_to(&mut d, &a.key, 120.5);
        assert_eq!(kinds(&drain(&mut b.session)), ["entity-remove"]);
        assert_eq!(kinds(&drain(&mut a.session)), ["entity-remove"]);
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines, reason = "the A3 producer cases per backend")]
async fn pose_producer_uses_the_current_sim_pose_with_follow_up_sync_and_mode_events() {
    for store in STORES {
        let router = custom_router(64, 10, limits(8, 7, 64, 8, 4096));
        let mut d = custom_driver(store, router).await;
        let mut a = join_vd(&mut d, "PoseA", spawn()).await;

        // (a) A region correction uses the latest genuine pose, not the
        // admission pose.
        move_to(&mut d, &a.key, 12.5);
        assert_eq!(
            d.router.route_native_play(&a.key, &at(300.5)),
            Err(CollectorSessionError::NativeRegionBoundary)
        );
        let packets = drain(&mut a.session);
        assert_eq!(kinds(&packets), ["sync", "notice"]);
        let ClientboundPlayPacket::SynchronizePlayerPosition(sync) = &packets[0].1 else {
            unreachable!()
        };
        assert_eq!(sync.x().to_bits(), 12.5_f64.to_bits());
        // Root 2(b): the router's sync carries no id; the connection stamps it.
        assert_eq!(sync.teleport_id(), 0);
        assert_eq!(packets[0].0, Criticality::Mandatory);

        // (b) An older lane item applied after a correction: one follow-up sync
        // to the genuine pose, then none while the pose is unchanged.
        assert_eq!(
            d.router.route_native_play(&a.key, &at(14.5)),
            Ok(NativeRouted::Queued)
        );
        assert_eq!(
            d.router.route_native_play(&a.key, &at(300.5)),
            Err(CollectorSessionError::NativeRegionBoundary)
        );
        let packets = drain(&mut a.session);
        let ClientboundPlayPacket::SynchronizePlayerPosition(sync) = &packets[0].1 else {
            unreachable!()
        };
        assert_eq!(
            sync.x().to_bits(),
            12.5_f64.to_bits(),
            "correction to the current pose"
        );
        let (_, seen) = d.iterate();
        assert_eq!(seen.changed, 1);
        let packets = drain(&mut a.session);
        assert_eq!(kinds(&packets), ["sync"]);
        let ClientboundPlayPacket::SynchronizePlayerPosition(sync) = &packets[0].1 else {
            unreachable!()
        };
        assert_eq!(
            sync.x().to_bits(),
            14.5_f64.to_bits(),
            "follow-up to the applied pose"
        );
        let view = d.env.shard.connected_owned_player_view(&a.handle).unwrap();
        assert_eq!(
            d.router.native_player_changed(&view),
            Ok(NativePoseOutcome::Updated)
        );
        assert_eq!(kinds(&drain(&mut a.session)), [] as [&str; 0]);
        // New client movement ends the follow-up window.
        move_to(&mut d, &a.key, 15.5);
        assert_eq!(kinds(&drain(&mut a.session)), [] as [&str; 0]);

        // (c) A genuine mode change is presented with a mandatory GameEvent.
        assert_eq!(
            d.router.route_native_game_mode(&a.key, GameMode::Creative),
            Ok(NativeRouted::Queued)
        );
        let (_, seen) = d.iterate();
        assert_eq!(seen.changed, 1);
        let packets = drain(&mut a.session);
        assert_eq!(kinds(&packets), ["game-event"]);
        let ClientboundPlayPacket::GameEvent(event) = &packets[0].1 else {
            unreachable!()
        };
        assert_eq!((event.reason(), event.value()), (3, 1.0));
        assert_eq!(packets[0].0, Criticality::Mandatory);
        assert_eq!(
            d.router.native_session_status(&a.key).unwrap().game_mode(),
            GameMode::Creative
        );

        // (d) Resync: mandatory sync to the genuine pose.
        let view = d.env.shard.connected_owned_player_view(&a.handle).unwrap();
        assert_eq!(d.router.native_resync(&view), Ok(NativePoseOutcome::Synced));
        let packets = drain(&mut a.session);
        assert_eq!(kinds(&packets), ["sync"]);

        // (e) A view of an admission no session holds is stale; nothing changes.
        let stranger = d.env.restore(player("Stranger"), spawn()).await;
        let view = d.env.shard.connected_owned_player_view(&stranger).unwrap();
        assert_eq!(
            d.router.native_player_changed(&view),
            Ok(NativePoseOutcome::Stale)
        );
        assert_eq!(d.router.native_resync(&view), Ok(NativePoseOutcome::Stale));
        assert_eq!(kinds(&drain(&mut a.session)), [] as [&str; 0]);
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines, reason = "each cap at N-1/N/N+1")]
async fn peer_burst_caps_refuse_the_admission_before_any_send_and_keep_the_reservation() {
    for store in STORES {
        // Per-viewer baselines: 1 refuses the third session, 2 admits it.
        for (per_viewer, admitted) in [(1, false), (2, true)] {
            let router = custom_router(64, 10, limits(4, per_viewer, 64, 4, 4096));
            let mut d = custom_driver(store, router).await;
            let _a = join_vd(&mut d, "CapA", spawn()).await;
            let _b = join_vd(&mut d, "CapB", Vec3::new(12.5, 64.0, 8.5)).await;
            let before = d.router.native_baseline_count();
            let (_, result, pending) = try_join(&mut d, "CapC", Vec3::new(16.5, 64.0, 8.5)).await;
            assert_eq!(result.is_ok(), admitted, "per viewer {per_viewer}");
            if !admitted {
                assert_eq!(result.err(), Some(CollectorSessionError::BaselineCapacity));
                assert_eq!(d.router.native_baseline_count(), before);
                assert_eq!(d.router.native_reservation_count(), 1, "still pending");
                assert_eq!(d.router.native_session_count(), 2);
                assert!(d.router.cancel_native_join(pending.unwrap()).is_ok());
            }
        }
        // Total baselines: A+B use 2; C needs 4 more. 5 refuses, 6 admits.
        for (total, admitted) in [(5, false), (6, true)] {
            let router = custom_router(64, 10, limits(4, 3, total, 4, 4096));
            let mut d = custom_driver(store, router).await;
            let _a = join_vd(&mut d, "TotA", spawn()).await;
            let _b = join_vd(&mut d, "TotB", Vec3::new(12.5, 64.0, 8.5)).await;
            assert_eq!(d.router.native_baseline_count(), 2);
            let (_, result, _) = try_join(&mut d, "TotC", Vec3::new(16.5, 64.0, 8.5)).await;
            assert_eq!(result.is_ok(), admitted, "total {total}");
            assert_eq!(
                d.router.native_baseline_count(),
                if admitted { 6 } else { 2 }
            );
        }
        // Work P + 3R + 2P² + 8P + 8 with P = 3, R = 4: 65.
        for (work, admitted) in [(64, false), (65, true)] {
            let router = custom_router(64, 10, limits(4, 3, 64, 4, work));
            let mut d = custom_driver(store, router).await;
            let _a = join_vd(&mut d, "WorkA", spawn()).await;
            let _b = join_vd(&mut d, "WorkB", Vec3::new(12.5, 64.0, 8.5)).await;
            let (_, result, _) = try_join(&mut d, "WorkC", Vec3::new(16.5, 64.0, 8.5)).await;
            if admitted {
                assert!(result.is_ok());
            } else {
                assert_eq!(result.err(), Some(CollectorSessionError::WorkCapacity));
            }
        }
        // Joiner outbound: JoinGame + 2 mandatory per peer must fit.
        for (outbound, admitted) in [(4, false), (5, true)] {
            let router = custom_router(outbound, 10, limits(4, 3, 64, 4, 4096));
            let mut d = custom_driver(store, router).await;
            let mut a = join_vd(&mut d, "OutA", spawn()).await;
            let mut b = join_vd(&mut d, "OutB", Vec3::new(12.5, 64.0, 8.5)).await;
            drain(&mut a.session);
            drain(&mut b.session);
            let (_, result, _) = try_join(&mut d, "OutC", Vec3::new(16.5, 64.0, 8.5)).await;
            if admitted {
                let mut c = result.unwrap();
                assert_eq!(drain(&mut c).len(), 5);
            } else {
                assert_eq!(result.err(), Some(CollectorSessionError::PeerBurstCapacity));
                assert_eq!(
                    kinds(&drain(&mut a.session)),
                    [] as [&str; 0],
                    "no send before the refusal"
                );
            }
        }
    }
}

#[tokio::test]
async fn a_viewer_that_cannot_take_a_mandatory_spawn_is_set_leaving() {
    for store in STORES {
        let router = custom_router(3, 10, limits(4, 3, 64, 4, 4096));
        let mut d = custom_driver(store, router).await;
        // A keeps a correction sync and its notice queued (2 of 3).
        let a = join_vd(&mut d, "SlowA", spawn()).await;
        assert_eq!(
            d.router.route_native_play(&a.key, &at(300.5)),
            Err(CollectorSessionError::NativeRegionBoundary)
        );
        // B's burst fits B; A takes the tab-add but not the mandatory spawn.
        let b = join_vd(&mut d, "SlowB", Vec3::new(12.5, 64.0, 8.5)).await;
        let status = d.router.native_session_status(&a.key).unwrap();
        assert_eq!(status.phase(), NativeSessionPhase::Leaving);
        assert_eq!(
            status.disconnect(),
            Some(ferrumc_session::NativeDisconnect::Sent)
        );
        assert_eq!(
            d.router.native_session_status(&b.key).unwrap().phase(),
            NativeSessionPhase::Active
        );
        // The leaving viewer's own baselines are gone; B still lists A.
        assert_eq!(d.router.native_baseline_count(), 1);
        drop(a.session);
    }
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "tracked audience, retirement proof and D7/D8"
)]
async fn collector_delivery_tracks_the_audience_and_proves_retirement_after_a_reserved_pickup() {
    for store in STORES {
        let router = custom_router(64, 2, limits(8, 7, 64, 8, 4096));
        let mut d = custom_driver(store, router).await;
        let mut a = join_vd(&mut d, "Collector", spawn()).await;
        let mut b = join_vd(&mut d, "Observer", Vec3::new(20.5, 64.0, 8.5)).await;
        let mut c = join_vd(&mut d, "Far", Vec3::new(100.5, 64.0, 8.5)).await;
        for session in [&mut a.session, &mut b.session, &mut c.session] {
            drain(session);
        }
        let a_wire = d
            .router
            .native_session_status(&a.key)
            .unwrap()
            .remote_wire_id();

        // Tracked: a confirmed animated pickup of a live collector.
        d.env.item(1, spawn()).await;
        let mut pickup = d.env.pickup(&a.handle, 1, true).await;
        assert_eq!(d.env.complete_all(&mut pickup).await, 1);
        d.iterate();
        {
            let group = d.env.shard.peek_item_publication().unwrap().unwrap();
            let collector = group.take().unwrap().collector();
            let lifecycle = d.env.shard.collector_lifecycle_view(&a.handle).unwrap();
            let CollectorDeliveryDisposition::Tracked(audience) =
                d.router.collector_delivery(collector, &lifecycle).unwrap()
            else {
                panic!("tracked expected");
            };
            let viewers: Vec<_> = audience
                .viewers()
                .map(|v| {
                    (
                        v.session().clone(),
                        v.collector_wire_id(),
                        v.is_collector_self(),
                    )
                })
                .collect();
            assert_eq!(
                viewers,
                [(a.key.clone(), 1, true), (b.key.clone(), a_wire, false)],
                "self with wire 1, the observer with its baseline wire, not the far viewer"
            );
        }
        d.env.ack_head();

        // Disconnect with a reserved pickup.
        d.env.item(2, spawn()).await;
        let mut reserved = d.env.pickup(&a.handle, 2, true).await;
        let (token, _) = disconnect(&mut d.router, &a.key);
        let (_, seen) = d.iterate();
        assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
        assert_eq!(d.env.complete_all(&mut reserved).await, 1);
        let (_, seen) = d.iterate();
        assert_eq!(seen.detached_intents, [token], "detached PlayerChanged");
        {
            let group = d.env.shard.peek_item_publication().unwrap().unwrap();
            let collector = group.take().unwrap().collector();
            let lifecycle = d.env.shard.collector_lifecycle_view(&a.handle).unwrap();
            assert_eq!(lifecycle.retained_take_pins(), 1);
            assert_eq!(
                d.router.collector_delivery(collector, &lifecycle).err(),
                Some(CollectorSessionError::CollectorNotCurrent),
                "leaving, not yet retired"
            );
        }
        // Leave-save and D5.
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (mut out, _) = d.iterate();
        assert_eq!(d.env.complete_all(&mut out).await, 1);
        d.iterate();
        assert_eq!(d.service(token, true), Serviced::Confirmed);
        assert_eq!(d.retire().retired(), 1);
        // No ticket while the pin is retained.
        let ack = d.acknowledge();
        assert_eq!((ack.acknowledged(), ack.waiting()), (0, 1));
        {
            let group = d.env.shard.peek_item_publication().unwrap().unwrap();
            let collector = group.take().unwrap().collector();
            let lifecycle = d.env.shard.collector_lifecycle_view(&a.handle).unwrap();
            let CollectorDeliveryDisposition::Retired(proof) =
                d.router.collector_delivery(collector, &lifecycle).unwrap()
            else {
                panic!("retirement proof expected");
            };
            assert_eq!(proof.player(), player("Collector"));
            assert_eq!(proof.session(), &a.key);
        }
        // Head acknowledgement (NET-04.2b stand-in) releases exactly the pin.
        d.env.ack_head();
        let lifecycle = d.env.shard.collector_lifecycle_view(&a.handle).unwrap();
        assert_eq!(
            (
                lifecycle.reserved_take_pins(),
                lifecycle.retained_take_pins()
            ),
            (0, 0)
        );
        // D7/D8: exactly one ticket; the record stays until release.
        assert_eq!(d.acknowledge().acknowledged(), 1);
        assert_eq!(d.acknowledge().acknowledged(), 0);
        assert_eq!(d.router.native_record_count(), 1);
        // The release gate passes, so `lifecycle_acked` was set by D8.
        assert_eq!(d.release(token), Ok(()));
        assert_eq!(d.router.native_record_count(), 0);
        // The observer got EntityRemove then TabRemove at D5.
        let packets = drain(&mut b.session);
        assert_eq!(kinds(&packets), ["entity-remove", "tab-remove"]);
    }
}

#[tokio::test]
async fn collector_delivery_refusals_are_typed() {
    for store in STORES {
        // Audience limit 1 (N+1 = self + observer).
        let router = custom_router(64, 10, limits(4, 3, 64, 1, 4096));
        let mut d = custom_driver(store, router).await;
        let a = join_vd(&mut d, "AudA", spawn()).await;
        let _b = join_vd(&mut d, "AudB", Vec3::new(12.5, 64.0, 8.5)).await;
        d.env.item(1, spawn()).await;
        let mut pickup = d.env.pickup(&a.handle, 1, true).await;
        assert_eq!(d.env.complete_all(&mut pickup).await, 1);
        d.iterate();
        let group = d.env.shard.peek_item_publication().unwrap().unwrap();
        let collector = group.take().unwrap().collector();
        let lifecycle = d.env.shard.collector_lifecycle_view(&a.handle).unwrap();
        assert_eq!(
            d.router.collector_delivery(collector, &lifecycle).err(),
            Some(CollectorSessionError::AudienceCapacity)
        );
        // Another router has no binding for the collector at all.
        let other = custom_router(64, 10, limits(4, 3, 64, 4, 4096));
        assert_eq!(
            other.collector_delivery(collector, &lifecycle).err(),
            Some(CollectorSessionError::CollectorBindingUnavailable)
        );
    }
}
