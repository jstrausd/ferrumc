//! NET-04.2e2: the native session lifecycle on genuine memory **and** redb
//! ownership admission in counter mode — D1–D10, the leave-save state machine
//! (§7.3, R2-A1), retirement (R1), acknowledgement (R2), release (§7.5), G3
//! adoption (§8) and the A2 revalidation disposition. Every status, fact and
//! completion comes from the real simulation and store. Round 2 adds the
//! regressions of F1 (final save of a `DisconnectNotApplied` record), F2
//! (refused disconnect across further replacements) and the §7.3 amendment V1
//! (Root: resends count only after the disconnect left the lane).

mod native_support;

use ferrumc_math::Vec3;
use ferrumc_net::Criticality;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, ServerboundPlayPacket, ServerboundSetHeldItem,
};
use ferrumc_session::{
    CollectorSessionError, LeaveSaveQuarantineReason, NativeDetachOutcome, NativeDisconnect,
    NativeInputClass, NativeLeaveSaveState, NativeRetentionPhase, PlayerSessionKey,
};
use ferrumc_sim::{OwnedPlayerHandle, OwnedPlayerInput, OwnershipIntent, PlayerOwnershipError};
use native_support::e2::{Driver, Serviced};
use native_support::*;

/// Router (lane 64, reserve 16) plus one registered native shard and env.
async fn driver(store: Store, env: EnvLimits, sessions: usize) -> Driver {
    let env = SimEnv::configured(store, env).await;
    let mut router = router(64, 16, sessions);
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

async fn join_at(d: &mut Driver, name: &str, position: Vec3) -> Admitted {
    let mut admitted = join(&mut d.router, &mut d.env, &d.lane, name, position).await;
    let first = admitted.session.try_recv().unwrap();
    assert!(matches!(first.packet(), ClientboundPlayPacket::JoinGame(_)));
    drain_peer_burst(&mut admitted.session);
    admitted
}

fn key(admitted: &Admitted) -> PlayerSessionKey {
    admitted.session.session_key().unwrap()
}

/// Runs the leave-save to `SaveConfirmed` with genuine work and completions.
async fn save_to_confirmed(d: &mut Driver, token: ferrumc_session::NativeRetentionToken) {
    assert_eq!(d.service(token, true), Serviced::Submitted);
    let (mut out, _) = d.iterate();
    assert_eq!(d.env.complete_all(&mut out).await, 1, "leave-save work");
    d.iterate();
    assert_eq!(d.service(token, true), Serviced::Confirmed);
}

fn status_of(
    d: &Driver,
    token: ferrumc_session::NativeRetentionToken,
) -> ferrumc_session::NativeRetentionStatus {
    d.router.native_retention_status(token).unwrap()
}

#[tokio::test]
#[allow(clippy::too_many_lines, reason = "one D1-D10 chain per backend")]
async fn bound_leave_runs_d2_to_d10_and_releases_on_both_backends() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let mut leaver = join_at(&mut d, "Leaver", spawn()).await;
        let mut viewer = join_at(&mut d, "Viewer", Vec3::new(12.5, 64.0, 8.5)).await;
        let leaver_key = key(&leaver);
        // The leaver's burst listed and spawned the viewer, and vice versa.
        assert_eq!(d.router.native_baseline_count(), 2);
        assert_eq!(drain_peer_burst(&mut leaver.session), 2);

        // D2 (A5): the token; the disconnect is the session's only lane item.
        let (token, sent) = disconnect(&mut d.router, &leaver_key);
        assert_eq!(sent, Some(NativeDisconnect::Sent));
        assert_eq!(d.router.disconnect_native(&leaver_key), Ok(token));
        assert_eq!(status_of(&d, token).phase(), NativeRetentionPhase::Leaving);
        assert_eq!(
            status_of(&d, token).leave_save(),
            NativeLeaveSaveState::AwaitDetach
        );
        assert!(d
            .router
            .native_lane_input_live(&leaver_key, NativeInputClass::Control));
        // Before the disconnect is applied the status reports a connected player.
        assert_eq!(d.service(token, true), Serviced::Waiting);
        // The release is refused at every earlier stage without effect.
        assert_eq!(
            d.release(token),
            Err(CollectorSessionError::RetentionNotReleasable("not retired"))
        );

        // D3: the genuine fact, confirmed through the closure.
        let (_, seen) = d.iterate();
        assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
        assert_eq!(status_of(&d, token).phase(), NativeRetentionPhase::Detached);
        // A4: control of a detached session is no longer live; A2: nothing retained.
        assert!(!d
            .router
            .native_lane_input_live(&leaver_key, NativeInputClass::Control));
        assert_eq!(d.router.retry_native_disconnects(), 0);

        // D4: one genuine leave-save; one attempt.
        save_to_confirmed(&mut d, token).await;
        assert_eq!(status_of(&d, token).attempts(), 1);
        assert_eq!(
            status_of(&d, token).leave_save(),
            NativeLeaveSaveState::Confirmed
        );
        assert_eq!(
            d.release(token),
            Err(CollectorSessionError::RetentionNotReleasable("not retired"))
        );

        // D5: retirement removes the mapping and sends EntityRemove, then TabRemove.
        let report = d.retire();
        assert_eq!((report.retired(), report.waiting()), (1, 0));
        assert!(d.router.native_session_status(&leaver_key).is_none());
        assert_eq!(
            d.router.route_native_play(
                &leaver_key,
                &ferrumc_proto::generated::play::ServerboundPlayPacket::SetPlayerPosition(
                    ferrumc_proto::generated::play::SetPlayerPosition::new(9.0, 64.0, 9.0, 0)
                )
            ),
            Err(CollectorSessionError::StaleSession)
        );
        let remove = viewer.session.try_recv().unwrap();
        assert_eq!(remove.criticality(), Criticality::Mandatory);
        let ClientboundPlayPacket::RemoveEntities(remove) = remove.packet() else {
            panic!("EntityRemove first: {:?}", remove.packet());
        };
        let leaver_wire = remove.entity_ids()[0];
        assert!(leaver_wire >= 2);
        let tab = viewer.session.try_recv().unwrap();
        assert_eq!(tab.criticality(), Criticality::Mandatory);
        assert!(matches!(
            tab.packet(),
            ClientboundPlayPacket::RemovePlayerInfo(_)
        ));
        assert!(viewer.session.try_recv().is_none());
        assert_eq!(d.router.native_baseline_count(), 0);
        assert_eq!(d.router.native_record_count(), 1);
        assert_eq!(
            status_of(&d, token).phase(),
            NativeRetentionPhase::Retired {
                acknowledged: false
            }
        );
        assert_eq!(
            d.release(token),
            Err(CollectorSessionError::RetentionNotReleasable(
                "lifecycle not acknowledged"
            ))
        );

        // D7/D8: exactly one ticket; the record stays until the release.
        let ack = d.acknowledge();
        assert_eq!(
            (ack.acknowledged(), ack.removed(), ack.waiting()),
            (1, 0, 0)
        );
        let again = d.acknowledge();
        assert_eq!(
            again.acknowledged() + again.waiting(),
            0,
            "ticket used once"
        );
        assert_eq!(
            status_of(&d, token).phase(),
            NativeRetentionPhase::Retired { acknowledged: true }
        );
        assert_eq!(d.router.native_record_count(), 1);

        // D10 / §7.5: release through the session closure frees L2.
        let players = d.env.shard.player_count();
        assert_eq!(d.release(token), Ok(()));
        assert_eq!(d.env.shard.player_count(), players - 1);
        assert_eq!(d.router.native_record_count(), 0);
        assert_eq!(d.release(token), Err(CollectorSessionError::StaleRetention));
        assert!(d.router.native_retention_status(token).is_none());
        // The viewer is untouched and still active.
        assert_eq!(d.router.native_session_count(), 1);
        assert!(d.router.native_session_status(&key(&viewer)).is_some());
        drop(leaver.session.try_recv());
    }
}

/// The status closure of a refused or stale handle is a typed quarantine.
fn quarantined(serviced: Serviced) -> ferrumc_session::LeaveSaveQuarantineReason {
    match serviced {
        Serviced::Quarantined(q) => q.reason(),
        other => panic!("expected quarantine, got {other:?}"),
    }
}

#[allow(dead_code, reason = "shared helper for later tests")]
fn handle_of(admitted: &Admitted) -> OwnedPlayerHandle {
    admitted.handle.clone()
}

#[allow(dead_code, reason = "shared helper for later tests")]
fn is_ineligible(error: &PlayerOwnershipError) -> bool {
    matches!(error, PlayerOwnershipError::Ineligible(_))
}

#[allow(dead_code, reason = "shared helper for later tests")]
fn reason_of(serviced: Serviced) -> ferrumc_session::LeaveSaveQuarantineReason {
    quarantined(serviced)
}

/// Reserve + genuine restore admission, but the reply is closed: adoption.
async fn adopt_restored(
    d: &mut Driver,
    name: &str,
) -> (ferrumc_session::NativeRetentionToken, OwnedPlayerHandle) {
    let reservation = d
        .router
        .reserve_native_join(player(name), generation(), spawn())
        .unwrap();
    let handle = d.env.restore(player(name), spawn()).await;
    let view = d.env.shard.connected_owned_player_view(&handle).unwrap();
    let token = d
        .router
        .adopt_unbound_native_admission(reservation, &handle, &view)
        .unwrap();
    (token, handle)
}

/// Drives an adopted (unbound) record through its submitted disconnect, D3,
/// the leave-save, the unbound retirement and D7/D8.
async fn unbound_chain_to_acknowledged(
    d: &mut Driver,
    token: ferrumc_session::NativeRetentionToken,
) {
    let status = status_of(d, token);
    assert!(!status.bound());
    assert_eq!(status.phase(), NativeRetentionPhase::Leaving);
    // The adopted disconnect is a gated class-2 submit, not a lane item.
    assert_eq!(d.service(token, false), Serviced::Waiting, "gate closed");
    assert_eq!(d.service(token, true), Serviced::Submitted);
    assert!(matches!(
        d.lane.try_recv(),
        Err(ferrumc_session::NativeLaneRecv::Empty)
    ));
    let (_, seen) = d.iterate();
    assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
    assert_eq!(status_of(d, token).phase(), NativeRetentionPhase::Detached);
    save_to_confirmed(d, token).await;
    let shard = &d.env.shard;
    d.router
        .retire_unbound_native_admission(token, &mut |handle, probe| {
            probe.submit(shard.collector_lifecycle_view(handle).ok())
        })
        .unwrap();
    assert_eq!(
        status_of(d, token).phase(),
        NativeRetentionPhase::Retired {
            acknowledged: false
        }
    );
    assert_eq!(d.acknowledge().acknowledged(), 1);
}

#[tokio::test]
async fn g3_closed_reply_restore_adoption_runs_the_unbound_chain_and_releases() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, handle) = adopt_restored(&mut d, "Adopted").await;
        assert_eq!(d.router.native_reservation_count(), 0);
        // Before its retirement the UUID cannot reserve again (G3 (c), before D9).
        assert_eq!(
            d.router
                .reserve_native_join(player("Adopted"), generation(), spawn())
                .err(),
            Some(CollectorSessionError::DuplicatePlayer {
                player: player("Adopted")
            })
        );
        // A detached intent of an admission no record holds is stale.
        let stranger = d.env.restore(player("Stranger"), spawn()).await;
        assert_eq!(d.router.native_detached_intent(&stranger), None);
        assert_eq!(d.router.native_detached_intent(&handle), Some(token));
        assert_eq!(d.router.native_record_count(), 1);
        // The unbound retirement refuses before the genuine detach fact.
        let shard = &d.env.shard;
        assert_eq!(
            d.router
                .retire_unbound_native_admission(token, &mut |h, probe| {
                    probe.submit(shard.collector_lifecycle_view(h).ok())
                }),
            Err(CollectorSessionError::RetirementRefused("not detached"))
        );
        // The adopted admission never becomes a session or a key.
        assert_eq!(d.router.native_session_count(), 0);
        unbound_chain_to_acknowledged(&mut d, token).await;
        assert_eq!(d.release(token), Ok(()));
        assert!(matches!(
            d.env.shard.detached_owned_player_status(&handle),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        assert_eq!(d.router.native_record_count(), 0);
    }
}

/// D2–D9 of a bound session up to a retired, confirmed record.
async fn bound_leave_to_retired(
    d: &mut Driver,
    name: &str,
) -> (ferrumc_session::NativeRetentionToken, OwnedPlayerHandle) {
    let admitted = join_at(d, name, spawn()).await;
    let (token, _) = disconnect(&mut d.router, &key(&admitted));
    let (_, seen) = d.iterate();
    assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
    save_to_confirmed(d, token).await;
    assert_eq!(d.retire().retired(), 1);
    (token, admitted.handle)
}

#[tokio::test]
async fn g3_closed_reply_rejoin_adoption_supersedes_the_old_record() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (old, old_handle) = bound_leave_to_retired(&mut d, "Returner").await;
        // After D9, before the release: the rejoin branch (§7.4).
        let reservation = d
            .router
            .reserve_native_join(player("Returner"), generation(), spawn())
            .unwrap();
        let read = d
            .env
            .binding
            .read_player(&d.env.store, player("Returner"))
            .await
            .unwrap();
        d.env
            .shard
            .enqueue_owned_player_input(ferrumc_sim::OwnedPlayerInput::rejoin(read))
            .unwrap();
        let out = d.env.boundary();
        d.applied += 1;
        let handle = native_support::e2::admitted_in(&out).expect("genuine rejoin");
        assert_ne!(handle, old_handle);
        let view = d.env.shard.connected_owned_player_view(&handle).unwrap();
        let token = d
            .router
            .adopt_unbound_native_admission(reservation, &handle, &view)
            .unwrap();
        assert_ne!(token, old);
        assert_eq!(
            status_of(&d, old).phase(),
            NativeRetentionPhase::Superseded {
                acknowledged: false
            }
        );
        assert_eq!(d.service(old, true), Serviced::Superseded);
        assert_eq!(
            d.release(old),
            Err(CollectorSessionError::RetentionSuperseded)
        );
        // The old cell is acknowledged and the superseded record removed.
        let ack = d.acknowledge();
        assert_eq!((ack.acknowledged(), ack.removed()), (0, 1));
        assert!(d.router.native_retention_status(old).is_none());
        // The genuine simulation release of the old handle is stale.
        assert!(matches!(
            d.env.shard.release_detached_owned_player(&old_handle),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        unbound_chain_to_acknowledged(&mut d, token).await;
        assert_eq!(d.release(token), Ok(()));
    }
}

#[tokio::test]
async fn rejoin_after_save_before_release_supersedes_and_the_old_release_is_stale() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        // Before D5 the same UUID cannot reserve again (L1 analogue in the session).
        let leaving = join_at(&mut d, "Again", spawn()).await;
        let (old, _) = disconnect(&mut d.router, &key(&leaving));
        assert_eq!(
            d.router
                .reserve_native_join(player("Again"), generation(), spawn())
                .err(),
            Some(CollectorSessionError::DuplicatePlayer {
                player: player("Again")
            })
        );
        let (_, seen) = d.iterate();
        assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
        save_to_confirmed(&mut d, old).await;
        assert_eq!(d.retire().retired(), 1);
        // The old cell acknowledged before the rejoin: removal on supersede.
        assert_eq!(d.acknowledge().acknowledged(), 1);
        let reservation = d
            .router
            .reserve_native_join(player("Again"), generation(), spawn())
            .unwrap();
        let read = d
            .env
            .binding
            .read_player(&d.env.store, player("Again"))
            .await
            .unwrap();
        d.env
            .shard
            .enqueue_owned_player_input(ferrumc_sim::OwnedPlayerInput::rejoin(read))
            .unwrap();
        let out = d.env.boundary();
        d.applied += 1;
        let handle = native_support::e2::admitted_in(&out).expect("genuine rejoin");
        let mut session = admit(&mut d.router, &d.env, &d.lane, &reservation, &handle).unwrap();
        assert!(matches!(
            session.try_recv().unwrap().packet(),
            ClientboundPlayPacket::JoinGame(_)
        ));
        // The acknowledged old record is superseded and removed in the same step.
        assert!(d.router.native_retention_status(old).is_none());
        assert_eq!(d.release(old), Err(CollectorSessionError::StaleRetention));
        assert!(matches!(
            d.env.shard.release_detached_owned_player(&leaving.handle),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        assert_eq!(d.router.native_session_count(), 1);
        assert_eq!(d.router.native_record_count(), 0);
    }
}

#[tokio::test]
async fn adoption_invariant_breaks_are_fatal_typed_and_poison_the_entry() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let other = d
            .env
            .restore(player("Other"), Vec3::new(20.5, 64.0, 8.5))
            .await;
        // View of another admission: ViewMismatch, entry poisoned.
        let reservation = d
            .router
            .reserve_native_join(player("Victim"), generation(), spawn())
            .unwrap();
        let handle = d.env.restore(player("Victim"), spawn()).await;
        let wrong = d.env.shard.connected_owned_player_view(&other).unwrap();
        let broken = d
            .router
            .adopt_unbound_native_admission(reservation, &handle, &wrong)
            .unwrap_err();
        assert_eq!(broken.player(), player("Victim"));
        assert_eq!(
            broken.kind(),
            ferrumc_session::AdoptionInvariantKind::ViewMismatch
        );
        assert_eq!(
            d.router.native_poisoned(player("Victim")),
            Some(ferrumc_session::AdoptionInvariantKind::ViewMismatch)
        );
        // The poisoned entry counts, blocks the UUID and its registration.
        assert_eq!(
            d.router
                .reserve_native_join(player("Victim"), generation(), spawn())
                .err(),
            Some(CollectorSessionError::DuplicatePlayer {
                player: player("Victim")
            })
        );
        // The simulation player stays admitted (no silent drop).
        assert!(d.env.shard.connected_owned_player_view(&handle).is_ok());

        // Handle of another player: PlayerMismatch.
        let reservation = d
            .router
            .reserve_native_join(player("Mismatch"), generation(), spawn())
            .unwrap();
        let view = d.env.shard.connected_owned_player_view(&other).unwrap();
        assert_eq!(
            d.router
                .adopt_unbound_native_admission(reservation, &other, &view)
                .unwrap_err()
                .kind(),
            ferrumc_session::AdoptionInvariantKind::PlayerMismatch
        );

        // A reservation of another router: foreign, nothing poisoned here.
        let mut foreign_router = router(64, 16, 8);
        let _foreign_registration = foreign_router
            .register_native_world_shard(home(), generation())
            .unwrap();
        let foreign = foreign_router
            .reserve_native_join(player("Foreign"), generation(), spawn())
            .unwrap();
        let foreign_handle = d.env.restore(player("Foreign"), spawn()).await;
        let view = d
            .env
            .shard
            .connected_owned_player_view(&foreign_handle)
            .unwrap();
        assert_eq!(
            d.router
                .adopt_unbound_native_admission(foreign, &foreign_handle, &view)
                .unwrap_err()
                .kind(),
            ferrumc_session::AdoptionInvariantKind::ForeignReservation
        );
        assert_eq!(d.router.native_poisoned(player("Foreign")), None);
        assert_eq!(d.router.native_record_count(), 0);
    }
}

#[tokio::test]
async fn adoption_is_capacity_infallible_with_the_session_limit_exhausted_by_reservations() {
    for store in STORES {
        // Session limit 3: N-1, N reservations succeed, N+1 is refused.
        let mut d = driver(store, EnvLimits::default(), 3).await;
        let names = ["Cap0", "Cap1", "Cap2"];
        let mut reservations = Vec::new();
        for name in names {
            reservations.push(
                d.router
                    .reserve_native_join(player(name), generation(), spawn())
                    .unwrap(),
            );
        }
        assert_eq!(
            d.router
                .reserve_native_join(player("Cap3"), generation(), spawn())
                .err(),
            Some(CollectorSessionError::SessionCapacity { limit: 3 })
        );
        // A cancel before the admission frees the entry deterministically.
        let cancelled = reservations.pop().unwrap();
        assert_eq!(d.router.cancel_native_join(cancelled), Ok(player("Cap2")));
        reservations.push(
            d.router
                .reserve_native_join(player("Cap3"), generation(), spawn())
                .unwrap(),
        );
        // Every adoption takes its reserved entry over: no capacity refusal.
        let mut tokens = Vec::new();
        for (reservation, name) in reservations.into_iter().zip(["Cap0", "Cap1", "Cap3"]) {
            let handle = d.env.restore(player(name), spawn()).await;
            let view = d.env.shard.connected_owned_player_view(&handle).unwrap();
            tokens.push(
                d.router
                    .adopt_unbound_native_admission(reservation, &handle, &view)
                    .unwrap(),
            );
        }
        assert_eq!(d.router.native_record_count(), 3);
        assert_eq!(d.router.native_reservation_count(), 0);
        let lineages: std::collections::BTreeSet<u64> =
            tokens.iter().map(|t| t.lineage()).collect();
        assert_eq!(lineages.len(), 3, "distinct lineages");
        assert_eq!(
            d.router
                .reserve_native_join(player("Cap4"), generation(), spawn())
                .err(),
            Some(CollectorSessionError::SessionCapacity { limit: 3 })
        );
    }
}

#[tokio::test]
async fn unregister_refuses_while_records_adopted_or_poisoned_entries_exist() {
    for store in STORES {
        let env = SimEnv::new(store).await;
        let mut router = router(64, 16, 8);
        let registration = router
            .register_native_world_shard(home(), generation())
            .unwrap();
        let lease = registration.lease();
        let (_, _, lane) = registration.into_parts();
        let mut d = Driver {
            router,
            env,
            lane,
            applied: 0,
        };
        let (token, _) = adopt_restored(&mut d, "Holder").await;
        let Driver {
            mut router, lane, ..
        } = d;
        let refused = router
            .unregister_native_shard_registration(&lease, lane)
            .unwrap_err();
        assert!(matches!(
            refused.error(),
            CollectorSessionError::NativeEndpointInUse {
                sessions: 1,
                reservations: 0,
                ..
            }
        ));
        assert!(router.native_retention_status(token).is_some());
    }
}

/// Joins `name`, disconnects it and applies the disconnect (D2, D3).
async fn detached_session(
    d: &mut Driver,
    name: &str,
) -> (ferrumc_session::NativeRetentionToken, Admitted) {
    let admitted = join_at(d, name, spawn()).await;
    let (token, _) = disconnect(&mut d.router, &key(&admitted));
    let (_, seen) = d.iterate();
    assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
    (token, admitted)
}

fn blocking(
    d: &Driver,
    handle: &OwnedPlayerHandle,
) -> Option<(
    ferrumc_sim::OwnershipTransferId,
    ferrumc_sim::OwnershipPendingStatus,
)> {
    d.env
        .shard
        .detached_owned_player_status(handle)
        .unwrap()
        .blocking()
}

#[tokio::test]
async fn r2_a1_due_with_a_foreign_pickup_waits_without_attempts_then_saves() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let admitted = join_at(&mut d, "Picker", spawn()).await;
        d.env.item(1, spawn()).await;
        d.applied += 1;
        // A pickup started before the detach, its work still out.
        let mut pickup = d.env.pickup(&admitted.handle, 1, true).await;
        d.applied += 1;
        let (token, _) = disconnect(&mut d.router, &key(&admitted));
        let (_, seen) = d.iterate();
        assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
        assert!(matches!(
            blocking(&d, &admitted.handle),
            Some((_, ferrumc_sim::OwnershipPendingStatus::InFlight))
        ));
        // Due + foreign blocking: Waiting, no attempt (R2-A1).
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(
            status_of(&d, token).leave_save(),
            NativeLeaveSaveState::Waiting
        );
        d.idle(50);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 0);
        // The pickup confirms; its intent reaches the session as a detached intent.
        assert_eq!(d.env.complete_all(&mut pickup).await, 1);
        let (_, seen) = d.iterate();
        assert_eq!(seen.detached_intents, [token]);
        assert_eq!(blocking(&d, &admitted.handle), None);
        // Waiting resumes to Due and submits the leave-save in the same step.
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (mut out, _) = d.iterate();
        assert_eq!(d.env.complete_all(&mut out).await, 1);
        d.iterate();
        assert_eq!(d.service(token, true), Serviced::Confirmed);
        assert_eq!(status_of(&d, token).attempts(), 1);
    }
}

#[tokio::test]
async fn foreign_autosave_confirmed_after_detach_is_the_leave_save() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let admitted = join_at(&mut d, "Early", spawn()).await;
        let mut autosave = d.env.foreign_autosave(&admitted.handle);
        d.applied += 1;
        let (token, _) = disconnect(&mut d.router, &key(&admitted));
        d.iterate();
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(d.env.complete_all(&mut autosave).await, 1);
        let (_, seen) = d.iterate();
        assert_eq!(seen.detached_intents, [token]);
        // A genuine autosave of this admission confirmed while detached counts.
        assert_eq!(d.service(token, true), Serviced::Confirmed);
        assert_eq!(status_of(&d, token).attempts(), 0);
    }
}

#[tokio::test]
async fn prepared_foreign_pending_counts_only_after_1200_boundaries_then_quarantines() {
    for store in STORES {
        // One persisted receipt: the restore seed uses it, so every later
        // commit is refused with a genuine `HistoryCapacity` (status Prepared).
        let limits = EnvLimits {
            receipts: 1,
            ..EnvLimits::default()
        };
        let mut d = driver(store, limits, 8).await;
        let admitted = join_at(&mut d, "Prepared", spawn()).await;
        let mut autosave = d.env.foreign_autosave(&admitted.handle);
        d.applied += 1;
        let (token, _) = disconnect(&mut d.router, &key(&admitted));
        d.iterate();
        assert_eq!(d.env.complete_all(&mut autosave).await, 1);
        d.iterate();
        assert!(matches!(
            blocking(&d, &admitted.handle),
            Some((_, ferrumc_sim::OwnershipPendingStatus::Prepared))
        ));
        assert_eq!(d.service(token, true), Serviced::Waiting);
        let since = d.applied;
        // N-1 / N / N+1 around the 1200-boundary grace plus one interval.
        d.idle(1299);
        assert_eq!(d.applied - since, 1299);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 0);
        d.idle(1);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 1);
        d.idle(700);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 8);
        d.idle(100);
        assert_eq!(
            quarantined(d.service(token, true)),
            ferrumc_session::LeaveSaveQuarantineReason::AttemptsExhausted
        );
        // L2 and the simulation player stay; no release without a save.
        assert!(d
            .env
            .shard
            .detached_owned_player_status(&admitted.handle)
            .is_ok());
        assert!(d.router.native_retention_status(token).is_some());
    }
}

/// Drops the work of `out` (lost reply) and clarifies it genuinely: no receipt
/// exists, the pending becomes `Uncertain`.
async fn make_uncertain(d: &mut Driver, mut out: ferrumc_sim::PlayerOwnershipTickOutcome) {
    let work = out.take_work().pop().unwrap();
    let query: ferrumc_sim::OwnershipReceiptQuery = work.query();
    drop(work);
    d.env
        .shard
        .enqueue_owned_player_input(ferrumc_sim::OwnedPlayerInput::reconcile(query))
        .unwrap();
    let (mut clarify, _) = d.iterate();
    assert_eq!(d.env.complete_all(&mut clarify).await, 1);
    let (_, seen) = d.iterate();
    assert_eq!(
        seen.refusals, 1,
        "the ReceiptAbsent completion is dropped (N3)"
    );
}

#[tokio::test]
async fn uncertain_pickup_counts_every_100_quarantines_and_renews_one_final_save() {
    for store in STORES {
        // An unresolved pickup's bound counter projection defers every normal
        // boundary; with the default allowance (64) the shard needs recovery
        // long before 900 boundaries and no `applied` boundary would count.
        // The maximum allowance keeps normal boundaries running (R3-A5 note).
        let limits = EnvLimits {
            defer: ferrumc_sim::MAX_OWNED_COUNTER_DEFER,
            ..EnvLimits::default()
        };
        let mut d = driver(store, limits, 8).await;
        let admitted = join_at(&mut d, "Uncertain", spawn()).await;
        d.env.item(1, spawn()).await;
        d.applied += 1;
        // A pickup started before the detach whose reply is lost for good.
        let pickup = d.env.pickup(&admitted.handle, 1, true).await;
        d.applied += 1;
        let (token, _) = disconnect(&mut d.router, &key(&admitted));
        d.iterate();
        make_uncertain(&mut d, pickup).await;
        assert!(matches!(
            blocking(&d, &admitted.handle),
            Some((_, ferrumc_sim::OwnershipPendingStatus::Uncertain))
        ));
        assert_eq!(d.service(token, true), Serviced::Waiting);
        d.idle(800);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 8);
        d.idle(100);
        assert_eq!(
            quarantined(d.service(token, true)),
            ferrumc_session::LeaveSaveQuarantineReason::AttemptsExhausted
        );
        // The quarantine is sticky and visible; nothing is released.
        d.idle(5);
        assert!(matches!(d.service(token, true), Serviced::Quarantined(_)));
        // Stop phase A: exactly one renewed attempt.
        assert_eq!(d.router.renew_native_leave_save(token), Ok(true));
        assert_eq!(d.router.renew_native_leave_save(token), Ok(false));
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 7);
        d.idle(100);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 8);
        // R3-A5 interplay: the unresolved pickup exhausts the counter defer
        // allowance; from then on normal boundaries fail before any mutation,
        // no boundary is applied and the leave-save counters stand still.
        let mut failed = None;
        for _ in 0..64 {
            let tick = d.env.tick.next().unwrap();
            match d.env.shard.run_tick_with_player_ownership_at(tick) {
                Ok(_) => {
                    d.env.tick = tick;
                    d.applied += 1;
                }
                Err(error) => {
                    failed = Some(error);
                    break;
                }
            }
        }
        assert!(matches!(
            failed,
            Some(PlayerOwnershipError::CounterRecoveryRequired)
        ));
        let attempts = status_of(&d, token).attempts();
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), attempts);
        assert_eq!(d.router.renew_native_leave_save(token), Ok(false));
    }
}

#[tokio::test]
async fn own_inflight_leave_save_counts_quarantines_and_a_late_confirmation_still_releases() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, admitted) = detached_session(&mut d, "Slow").await;
        assert_eq!(d.service(token, true), Serviced::Submitted);
        // The leave-save work stays out (slow worker): own pending InFlight.
        let (mut held, _) = d.iterate();
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(
            status_of(&d, token).leave_save(),
            NativeLeaveSaveState::Pending
        );
        d.idle(799);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 8);
        d.idle(1);
        assert_eq!(
            quarantined(d.service(token, true)),
            ferrumc_session::LeaveSaveQuarantineReason::AttemptsExhausted
        );
        assert_eq!(d.retire().retired(), 1);
        assert_eq!(d.acknowledge().acknowledged(), 1);
        assert_eq!(
            d.release(token),
            Err(CollectorSessionError::RetentionNotReleasable(
                "leave-save not confirmed"
            ))
        );
        // A later genuine confirmation of this admission's leave-save.
        assert_eq!(d.env.complete_all(&mut held).await, 1);
        d.iterate();
        assert_eq!(d.service(token, true), Serviced::Confirmed);
        assert_eq!(d.release(token), Ok(()));
        assert!(matches!(
            d.env.shard.detached_owned_player_status(&admitted.handle),
            Err(PlayerOwnershipError::StaleHandle)
        ));
    }
}

#[tokio::test]
async fn own_leave_save_cas_conflict_quarantines_immediately() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, admitted) = detached_session(&mut d, "Conflict").await;
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (mut out, _) = d.iterate();
        // Another writer changes the durable player first: genuine CAS conflict.
        let scoped = ferrumc_storage::ScopedPlayerKey::new(scope(), admitted.handle.player());
        let current = d
            .env
            .store
            .load_owned_player(scoped, identity())
            .await
            .unwrap();
        let other = ferrumc_sim::NativePlayerSnapshot::empty_survival(
            Vec3::new(1.5, 64.0, 1.5),
            [0.0, 0.0],
        );
        let transfer = d.env.fresh_id();
        d.env
            .store
            .compare_exchange_owned_player(
                ferrumc_storage::PlayerOwnershipCasRequest::new(
                    transfer,
                    current.expectation().clone(),
                    ferrumc_storage::PlayerOwnershipProposal::put(other.try_to_record().unwrap()),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(d.env.complete_all(&mut out).await, 1);
        let (_, seen) = d.iterate();
        assert_eq!(seen.refusals, 1);
        assert!(matches!(
            blocking(&d, &admitted.handle),
            Some((_, ferrumc_sim::OwnershipPendingStatus::Quarantined))
        ));
        assert_eq!(
            quarantined(d.service(token, true)),
            ferrumc_session::LeaveSaveQuarantineReason::PendingQuarantined
        );
        assert_eq!(status_of(&d, token).attempts(), 1);
    }
}

#[tokio::test]
async fn definitive_leave_save_refusal_resubmits_after_20_applied_boundaries() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, admitted) = detached_session(&mut d, "Collide").await;
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (mut out, _) = d.iterate();
        let (id, _) = blocking(&d, &admitted.handle).unwrap();
        // Another operation commits first under the same id: ReceiptCollision.
        let bystander =
            ferrumc_storage::ScopedPlayerKey::new(scope(), player("CollisionBystander"));
        let current = d
            .env
            .store
            .load_owned_player(bystander, identity())
            .await
            .unwrap();
        d.env
            .store
            .compare_exchange_owned_player(
                ferrumc_storage::PlayerOwnershipCasRequest::new(
                    id,
                    current.expectation().clone(),
                    ferrumc_storage::PlayerOwnershipProposal::put(
                        ferrumc_sim::NativePlayerSnapshot::empty_survival(spawn(), [0.0, 0.0])
                            .try_to_record()
                            .unwrap(),
                    ),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(d.env.complete_all(&mut out).await, 1);
        let (_, seen) = d.iterate();
        assert_eq!(seen.refusals, 1);
        assert_eq!(blocking(&d, &admitted.handle), None);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).leave_save(), NativeLeaveSaveState::Due);
        assert_eq!(
            status_of(&d, token).attempts(),
            1,
            "the refused attempt counts"
        );
        d.idle(19);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        d.idle(1);
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (mut out, _) = d.iterate();
        assert_eq!(d.env.complete_all(&mut out).await, 1);
        d.iterate();
        assert_eq!(d.service(token, true), Serviced::Confirmed);
        assert_eq!(status_of(&d, token).attempts(), 2);
    }
}

#[tokio::test]
async fn capacity_refusal_resubmits_after_20_and_an_enqueue_refusal_counts_nothing() {
    for store in STORES {
        // One pending at a time: a foreign player's autosave holds it.
        let limits = EnvLimits {
            pending: 1,
            ..EnvLimits::default()
        };
        let mut d = driver(store, limits, 8).await;
        let holder = d
            .env
            .restore(player("Holder"), Vec3::new(20.5, 64.0, 8.5))
            .await;
        d.applied += 1;
        let (token, admitted) = detached_session(&mut d, "Capped").await;
        let mut held = d.env.foreign_autosave(&holder);
        d.applied += 1;
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (_, seen) = d.iterate();
        assert_eq!(seen.refusals, 1, "pending capacity refused the leave-save");
        assert_eq!(blocking(&d, &admitted.handle), None);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).leave_save(), NativeLeaveSaveState::Due);
        assert_eq!(status_of(&d, token).attempts(), 1);
        assert_eq!(d.env.complete_all(&mut held).await, 1);
        d.idle(20);
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (mut out, _) = d.iterate();
        assert_eq!(d.env.complete_all(&mut out).await, 1);
        d.iterate();
        assert_eq!(d.service(token, true), Serviced::Confirmed);
        assert_eq!(status_of(&d, token).attempts(), 2);
    }
    for store in STORES {
        // One input slot: a queued input makes the simulation refuse the enqueue.
        let limits = EnvLimits {
            inputs: 1,
            ..EnvLimits::default()
        };
        let mut d = driver(store, limits, 8).await;
        let (token, _) = detached_session(&mut d, "Unsent").await;
        d.env
            .shard
            .enqueue_owned_player_input(ferrumc_sim::OwnedPlayerInput::drain())
            .unwrap();
        assert_eq!(d.service(token, true), Serviced::Unsent);
        assert_eq!(status_of(&d, token).leave_save(), NativeLeaveSaveState::Due);
        assert_eq!(
            status_of(&d, token).attempts(),
            0,
            "the unsent submit is rolled back"
        );
        assert_eq!(
            d.router.native_retention_unsent(token),
            Err(CollectorSessionError::NoRetentionSubmit)
        );
    }
}

#[tokio::test]
async fn status_closure_refusals_are_typed() {
    for store in STORES {
        // Ineligible("connected player"), §7.3 amendment V1 (Root): a
        // disconnect that stays in the lane counts no resend, however many
        // applied boundaries pass.
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let admitted = join_at(&mut d, "Stuck", spawn()).await;
        let (token, _) = disconnect(&mut d.router, &key(&admitted));
        for _ in 0..100 {
            let _ = d.env.boundary();
            d.applied += 1;
            assert_eq!(d.service(token, true), Serviced::Waiting);
        }
        assert_eq!(
            status_of(&d, token).leave_save(),
            NativeLeaveSaveState::AwaitDetach
        );
        // Counter-check: every disconnect is taken but its handover is lost.
        // A resend counts only after the previous disconnect was taken and a
        // boundary followed; the ninth taken disconnect quarantines.
        let (reason, controls, iterations, _) = quarantine_by_lost_disconnects(&mut d, token);
        assert_eq!(reason, LeaveSaveQuarantineReason::DisconnectNotApplied);
        assert_eq!(controls, 9, "the first disconnect plus eight resends");
        assert_eq!(
            iterations, 18,
            "each resend waits for its take and a boundary"
        );
        assert!(matches!(
            d.lane.try_recv(),
            Err(ferrumc_session::NativeLaneRecv::Empty)
        ));
        assert_eq!(d.router.retry_native_disconnects(), 0);
        assert!(d
            .env
            .shard
            .connected_owned_player_view(&admitted.handle)
            .is_ok());

        // StaleHandle / foreign owner (same UUID in another shard): quarantine.
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, admitted) = detached_session(&mut d, "Foreigner").await;
        let mut foreign = SimEnv::new(store).await;
        let _other = foreign.restore(admitted.handle.player(), spawn()).await;
        let step = d
            .router
            .service_native_retention(
                token,
                d.applied,
                true,
                &mut |h| foreign.shard.detached_owned_player_status(h),
                &mut || unreachable!("no submit for a stale handle"),
            )
            .unwrap();
        assert!(matches!(
            step,
            ferrumc_session::NativeRetentionStep::Quarantined(q)
                if q.reason() == ferrumc_session::LeaveSaveQuarantineReason::StatusInvariant
        ));

        // SnapshotOnly / NotInitialized: wiring breaks, typed error, no change.
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, _) = detached_session(&mut d, "Wiring").await;
        let snapshot = d.env.shard.clone();
        let before = status_of(&d, token);
        assert_eq!(
            d.router
                .service_native_retention(
                    token,
                    d.applied,
                    true,
                    &mut |h| snapshot.detached_owned_player_status(h),
                    &mut || unreachable!(),
                )
                .err(),
            Some(CollectorSessionError::NativeStatusInvariant)
        );
        let bare = ferrumc_sim::SimShard::new(ferrumc_math::ShardPos::new(0, 0));
        assert_eq!(
            d.router
                .service_native_retention(
                    token,
                    d.applied,
                    true,
                    &mut |h| bare.detached_owned_player_status(h),
                    &mut || unreachable!(),
                )
                .err(),
            Some(CollectorSessionError::NativeStatusInvariant)
        );
        assert_eq!(status_of(&d, token), before);
    }
}

/// One replacement of the registration; returns the new lane and the drained
/// old lane's items (applied to `target`).
fn replace(
    router: &mut ferrumc_session::SessionRouter,
    lease: &ferrumc_session::ShardLease,
    old: &mut ferrumc_session::NativeShardReceiver,
) -> (
    ferrumc_session::ShardLease,
    ferrumc_session::NativeShardReceiver,
) {
    let replaced = router
        .replace_native_shard_registration(lease, old)
        .unwrap();
    let new_lease = replaced.lease();
    let (_, _, lane) = replaced.into_parts();
    (new_lease, lane)
}

#[tokio::test]
#[allow(clippy::too_many_lines, reason = "both A2 outcomes per backend")]
async fn refused_revalidation_disconnect_ends_in_detach_or_typed_quarantine_never_rebound() {
    for store in STORES {
        // (1) The current lane feeds the genuine owner: normal detach path.
        let mut env = SimEnv::new(store).await;
        let mut router = router(64, 16, 8);
        let registration = router
            .register_native_world_shard(home(), generation())
            .unwrap();
        let lease = registration.lease();
        let (_, _, mut old_lane) = registration.into_parts();
        let refused = join(&mut router, &mut env, &old_lane, "Refused", spawn()).await;
        let other = join(
            &mut router,
            &mut env,
            &old_lane,
            "Witness",
            Vec3::new(20.5, 64.0, 8.5),
        )
        .await;
        let (_, lane) = replace(&mut router, &lease, &mut old_lane);
        let witness = other.handle.clone();
        let shard = &env.shard;
        let report = router
            .revalidate_native_endpoints(&lane, &mut |handle, probe| {
                if handle.player() == player("Refused") {
                    probe.submit(shard.connected_owned_player_view(&witness).ok())
                } else {
                    probe.submit(shard.connected_owned_player_view(handle).ok())
                }
            })
            .unwrap();
        assert_eq!((report.revalidated(), report.refused()), (1, 1));
        let refused_key = key(&refused);
        let status = router.native_session_status(&refused_key).unwrap();
        assert_eq!(
            status.endpoint(),
            ferrumc_session::NativeEndpointState::RevalidationRefused
        );
        assert_eq!(status.disconnect(), Some(NativeDisconnect::Sent));
        assert_eq!(router.retry_native_disconnects(), 0);
        let mut d = Driver {
            router,
            env,
            lane,
            applied: 0,
        };
        let (token, _) = disconnect(&mut d.router, &refused_key);
        let (_, seen) = d.iterate();
        assert_eq!(seen.refusals, 0);
        assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
        save_to_confirmed(&mut d, token).await;
        assert_eq!(d.retire().retired(), 1);
        assert_eq!(d.acknowledge().acknowledged(), 1);
        assert_eq!(d.release(token), Ok(()));
        assert_eq!(d.router.retry_native_disconnects(), 0);

        // (2) The current lane feeds another owner: the simulation refuses the
        // disconnect at apply (StaleHandle); the record is quarantined, the
        // disconnect is never resent and never rebound.
        let mut env = SimEnv::new(store).await;
        let mut foreign = SimEnv::new(store).await;
        let mut router = native_support::router(64, 16, 8);
        let registration = router
            .register_native_world_shard(home(), generation())
            .unwrap();
        let lease = registration.lease();
        let (_, _, mut old_lane) = registration.into_parts();
        let stranded = join(&mut router, &mut env, &old_lane, "Stranded", spawn()).await;
        let (_, lane) = replace(&mut router, &lease, &mut old_lane);
        let lookalike = foreign.restore(player("Stranded"), spawn()).await;
        let foreign_shard = &foreign.shard;
        let report = router
            .revalidate_native_endpoints(&lane, &mut |_handle, probe| {
                probe.submit(foreign_shard.connected_owned_player_view(&lookalike).ok())
            })
            .unwrap();
        assert_eq!(report.refused(), 1);
        let stranded_key = key(&stranded);
        assert_eq!(
            router
                .native_session_status(&stranded_key)
                .unwrap()
                .disconnect(),
            Some(NativeDisconnect::Sent)
        );
        let mut d = Driver {
            router,
            env: foreign,
            lane,
            applied: 0,
        };
        let (token, _) = disconnect(&mut d.router, &stranded_key);
        let (_, seen) = d.iterate();
        assert_eq!(
            seen.refusals, 1,
            "the foreign owner refused the stored handle"
        );
        assert_eq!(seen.detached, [] as [NativeDetachOutcome; 0]);
        assert_eq!(
            quarantined(d.service(token, true)),
            ferrumc_session::LeaveSaveQuarantineReason::StatusInvariant
        );
        d.iterate();
        assert!(matches!(d.service(token, true), Serviced::Quarantined(_)));
        assert!(
            matches!(
                d.lane.try_recv(),
                Err(ferrumc_session::NativeLaneRecv::Empty)
            ),
            "never resent"
        );
        assert_eq!(d.router.retry_native_disconnects(), 0);
        let again = d
            .router
            .revalidate_native_endpoints(&d.lane, &mut |_, _| unreachable!("never rebound"))
            .unwrap();
        assert_eq!(again.revalidated() + again.refused() + again.open(), 0);
        assert_eq!(
            d.router
                .native_session_status(&stranded_key)
                .unwrap()
                .endpoint(),
            ferrumc_session::NativeEndpointState::RevalidationRefused
        );
        // The genuine original admission is untouched in its own shard.
        assert!(env
            .shard
            .connected_owned_player_view(&stranded.handle)
            .is_ok());
    }
}

#[tokio::test]
async fn session_api_works_against_the_sim_reexports_without_a_storage_type() {
    // H1 (SIM-03.4b review): the session's production API names only
    // `ferrumc_sim::OwnershipTransferId`; the leave-save id the driver supplies
    // through that type is the one the simulation reports, and the genuine
    // work query is a `ferrumc_sim::OwnershipReceiptQuery`.
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, admitted) = detached_session(&mut d, "Reexport").await;
        let chosen: ferrumc_sim::OwnershipTransferId = d.env.fresh_id();
        let shard = &d.env.shard;
        let step = d
            .router
            .service_native_retention(
                token,
                d.applied,
                true,
                &mut |h| shard.detached_owned_player_status(h),
                &mut || chosen,
            )
            .unwrap();
        let ferrumc_session::NativeRetentionStep::Submit(input) = step else {
            panic!("leave-save submit expected");
        };
        d.env
            .shard
            .enqueue_owned_player_input(input.into_parts().2)
            .unwrap();
        let (mut out, _) = d.iterate();
        let query: ferrumc_sim::OwnershipReceiptQuery = out.take_work()[0].query();
        assert_eq!(query.id(), chosen);
        assert_eq!(
            blocking(&d, &admitted.handle).map(|(id, _)| id),
            Some(chosen)
        );
    }
}

#[tokio::test]
async fn own_uncertain_leave_save_counts_every_100_boundaries() {
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let (token, admitted) = detached_session(&mut d, "OwnUncertain").await;
        assert_eq!(d.service(token, true), Serviced::Submitted);
        let (out, _) = d.iterate();
        make_uncertain(&mut d, out).await;
        assert!(matches!(
            blocking(&d, &admitted.handle),
            Some((_, ferrumc_sim::OwnershipPendingStatus::Uncertain))
        ));
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(
            status_of(&d, token).leave_save(),
            NativeLeaveSaveState::Pending
        );
        d.idle(99);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 1);
        d.idle(1);
        assert_eq!(d.service(token, true), Serviced::Waiting);
        assert_eq!(status_of(&d, token).attempts(), 2);
        d.idle(700);
        assert_eq!(
            quarantined(d.service(token, true)),
            ferrumc_session::LeaveSaveQuarantineReason::AttemptsExhausted
        );
    }
}

/// One iteration of a driver whose handover to the simulation is lost: it
/// takes every lane item but never enqueues it, then runs one normal boundary.
/// The simulation applies every disconnect of a connected admission it gets,
/// so a lost handover is the only way a taken disconnect stays unapplied.
fn lose_lane_items(d: &mut Driver) -> Vec<OwnedPlayerInput> {
    let mut lost = Vec::new();
    while let Ok(item) = d.lane.try_recv() {
        let (_, class, input) = item.into_parts();
        assert_eq!(class, NativeInputClass::Control);
        lost.push(input);
    }
    let _ = d.env.boundary();
    d.applied += 1;
    lost
}

/// Loses every taken disconnect of `token` until its record is quarantined;
/// returns the reason, the taken disconnects, the iterations and the last
/// taken disconnect input.
fn quarantine_by_lost_disconnects(
    d: &mut Driver,
    token: ferrumc_session::NativeRetentionToken,
) -> (LeaveSaveQuarantineReason, usize, usize, OwnedPlayerInput) {
    let mut controls = 0;
    let mut newest = None;
    for iteration in 1..=40 {
        let mut batch = lose_lane_items(d);
        controls += batch.len();
        if let Some(input) = batch.pop() {
            newest = Some(input);
        }
        match d.service(token, true) {
            Serviced::Waiting => {}
            other => return (quarantined(other), controls, iteration, newest.unwrap()),
        }
    }
    panic!("no quarantine after 40 iterations");
}

/// F1 (round 2, Root): the stop phase A final save of a
/// `DisconnectNotApplied` record whose player is still connected is legitimate:
/// the next service returns the record to its quarantine without an attempt
/// and without a resend; it is no `NativeStatusInvariant` (the review's P4b),
/// and the other records run on. If the disconnect was applied meanwhile, the
/// renewed final save is a genuine leave-save.
#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "both final-save outcomes per backend"
)]
async fn final_save_of_a_disconnect_not_applied_record_never_breaks_the_driver() {
    for store in STORES {
        for applied_late in [false, true] {
            let mut d = driver(store, EnvLimits::default(), 8).await;
            let stuck = join_at(&mut d, "Stuck", spawn()).await;
            let other = join_at(&mut d, "Other", Vec3::new(12.5, 64.0, 8.5)).await;
            let (stuck_token, _) = disconnect(&mut d.router, &key(&stuck));
            let (reason, _, _, last_taken) = quarantine_by_lost_disconnects(&mut d, stuck_token);
            assert_eq!(reason, LeaveSaveQuarantineReason::DisconnectNotApplied);
            // Stop phase A: every active session leaves through the lane.
            let (other_token, _) = disconnect(&mut d.router, &key(&other));
            if applied_late {
                // The driver finally hands the last taken disconnect over.
                d.env.shard.enqueue_owned_player_input(last_taken).unwrap();
            } else {
                drop(last_taken);
            }
            let (_, seen) = d.iterate();
            assert_eq!(seen.refusals, 0);
            assert_eq!(
                seen.detached,
                vec![NativeDetachOutcome::Confirmed; 1 + usize::from(applied_late)]
            );
            // Exactly one renewed final save per quarantined record.
            assert_eq!(d.router.renew_native_leave_save(stuck_token), Ok(true));
            assert_eq!(d.router.renew_native_leave_save(stuck_token), Ok(false));
            if applied_late {
                save_to_confirmed(&mut d, stuck_token).await;
            } else {
                // `Driver::service` unwraps: the former `NativeStatusInvariant`
                // would panic here.
                assert_eq!(
                    quarantined(d.service(stuck_token, true)),
                    LeaveSaveQuarantineReason::DisconnectNotApplied
                );
                let status = status_of(&d, stuck_token);
                assert_eq!(
                    status.leave_save(),
                    NativeLeaveSaveState::Quarantined(
                        LeaveSaveQuarantineReason::DisconnectNotApplied
                    )
                );
                assert_eq!(status.attempts(), 0, "the final save made no attempt");
                assert_eq!(status.phase(), NativeRetentionPhase::Leaving);
            }
            // The other record runs on in the same service round.
            save_to_confirmed(&mut d, other_token).await;
            let retiring = 1 + usize::from(applied_late);
            assert_eq!(d.retire().retired(), retiring);
            assert_eq!(d.acknowledge().acknowledged(), retiring);
            assert_eq!(d.release(other_token), Ok(()));
            if applied_late {
                assert_eq!(d.release(stuck_token), Ok(()));
                assert_eq!(d.router.native_record_count(), 0);
            } else {
                assert_eq!(
                    d.release(stuck_token),
                    Err(CollectorSessionError::RetentionNotReleasable("not retired"))
                );
                d.iterate();
                assert!(matches!(
                    d.service(stuck_token, true),
                    Serviced::Quarantined(q)
                        if q.reason() == LeaveSaveQuarantineReason::DisconnectNotApplied
                ));
                assert!(
                    matches!(
                        d.lane.try_recv(),
                        Err(ferrumc_session::NativeLaneRecv::Empty)
                    ),
                    "no resend"
                );
                assert!(d
                    .env
                    .shard
                    .connected_owned_player_view(&stuck.handle)
                    .is_ok());
            }
            assert_eq!(d.router.retry_native_disconnects(), 0);
        }
    }
}

/// F2 (round 2, Root): a session whose revalidation was refused and whose
/// disconnect is retained (one-slot lane full) survives two further
/// replacements; its disconnect from the stored handle follows the newest
/// lane, `retry_native_disconnects` reaches zero, and the record ends: applied
/// → detach chain and release, foreign owner → `StaleHandle` quarantine (the
/// review's P5).
#[tokio::test]
#[allow(clippy::too_many_lines, reason = "both A2 outcomes per backend")]
async fn refused_retained_disconnect_survives_every_replacement_and_ends() {
    for store in STORES {
        for foreign_owner in [false, true] {
            let mut env = SimEnv::new(store).await;
            let mut router = router(1, 0, 8);
            let registration = router
                .register_native_world_shard(home(), generation())
                .unwrap();
            let lease1 = registration.lease();
            let (_, _, mut lane1) = registration.into_parts();
            let bee = join(&mut router, &mut env, &lane1, "Bee", spawn()).await;
            let ay = join(
                &mut router,
                &mut env,
                &lane1,
                "Ay",
                Vec3::new(20.5, 64.0, 8.5),
            )
            .await;
            let (bee_key, ay_key) = (key(&bee), key(&ay));
            let (lease2, mut lane2) = replace(&mut router, &lease1, &mut lane1);
            assert!(lane1.try_recv().is_err());
            assert_eq!(
                disconnect(&mut router, &bee_key).1,
                Some(NativeDisconnect::Retained)
            );
            // Bee rebinds (its disconnect fills the one-slot lane); Ay's view
            // mismatches, so Ay is refused and its disconnect is retained.
            let bee_handle = bee.handle.clone();
            let shard = &env.shard;
            let report = router
                .revalidate_native_endpoints(&lane2, &mut |handle, probe| {
                    if handle.player() == player("Ay") {
                        probe.submit(shard.connected_owned_player_view(&bee_handle).ok())
                    } else {
                        probe.submit(shard.connected_owned_player_view(handle).ok())
                    }
                })
                .unwrap();
            assert_eq!((report.revalidated(), report.refused()), (1, 1));
            assert_eq!(
                router.native_session_status(&ay_key).unwrap().disconnect(),
                Some(NativeDisconnect::Retained)
            );
            assert_eq!(router.retry_native_disconnects(), 1, "lane full");
            // Two further replacements before the next retry; the driver drains
            // every old lane completely (only Bee's disconnect).
            let (lease3, mut lane3) = replace(&mut router, &lease2, &mut lane2);
            let (_, lane4) = replace(&mut router, &lease3, &mut lane3);
            let mut old = Vec::new();
            for lane in [&mut lane2, &mut lane3] {
                while let Ok(item) = lane.try_recv() {
                    old.push(item.into_parts().2);
                }
            }
            assert_eq!(old.len(), 1, "only Bee's disconnect");
            for input in old {
                env.shard.enqueue_owned_player_input(input).unwrap();
            }
            let mut out = env.boundary();
            assert!(out.take_refusals().is_empty());
            for intent in out.intents() {
                if let OwnershipIntent::PlayerDetached { player } = intent {
                    let shard = &env.shard;
                    let outcome = router
                        .native_detached(*player, &mut |h, probe| {
                            probe.submit(shard.collector_lifecycle_view(h).ok())
                        })
                        .unwrap();
                    assert_eq!(outcome, NativeDetachOutcome::Confirmed);
                }
            }
            // Ay stays refused and is never rebound; its disconnect follows
            // the newest lane.
            assert_eq!(router.retry_native_disconnects(), 0);
            let status = router.native_session_status(&ay_key).unwrap();
            assert_eq!(
                status.endpoint(),
                ferrumc_session::NativeEndpointState::RevalidationRefused
            );
            assert_eq!(status.disconnect(), Some(NativeDisconnect::Sent));
            let token = router.disconnect_native(&ay_key).unwrap();
            if foreign_owner {
                // The newest lane feeds another owner of the same UUID: the
                // simulation refuses the stored handle at apply.
                let mut foreign = SimEnv::new(store).await;
                let _lookalike = foreign.restore(player("Ay"), spawn()).await;
                let mut d = Driver {
                    router,
                    env: foreign,
                    lane: lane4,
                    applied: 1,
                };
                let (_, seen) = d.iterate();
                assert_eq!(
                    seen.refusals, 1,
                    "the foreign owner refused the stored handle"
                );
                assert_eq!(
                    quarantined(d.service(token, true)),
                    LeaveSaveQuarantineReason::StatusInvariant
                );
                d.iterate();
                assert!(matches!(d.service(token, true), Serviced::Quarantined(_)));
                assert!(
                    matches!(
                        d.lane.try_recv(),
                        Err(ferrumc_session::NativeLaneRecv::Empty)
                    ),
                    "never resent"
                );
                assert_eq!(d.router.retry_native_disconnects(), 0);
                assert!(env.shard.connected_owned_player_view(&ay.handle).is_ok());
            } else {
                let mut d = Driver {
                    router,
                    env,
                    lane: lane4,
                    applied: 1,
                };
                let (_, seen) = d.iterate();
                assert_eq!(seen.refusals, 0);
                assert_eq!(seen.detached, [NativeDetachOutcome::Confirmed]);
                save_to_confirmed(&mut d, token).await;
                assert_eq!(d.retire().retired(), 2, "Ay and Bee");
                assert_eq!(d.acknowledge().acknowledged(), 2);
                assert_eq!(d.release(token), Ok(()));
                assert_eq!(d.router.retry_native_disconnects(), 0);
                assert!(d.env.shard.connected_owned_player_view(&ay.handle).is_err());
            }
        }
    }
}

/// §7.3 amendment V1 (Root): hotbar spam of another client builds a lane
/// backlog, and the driver takes at most `WINDOW` items per iteration. The
/// victim's queued disconnect counts no resend and is never quarantined; its
/// genuine detach follows once it is taken, then the normal leave-save (the
/// review's P7).
#[tokio::test]
async fn lane_backlog_of_another_client_never_quarantines_a_queued_disconnect() {
    const WINDOW: usize = 4;
    for store in STORES {
        let mut d = driver(store, EnvLimits::default(), 8).await;
        let spammer = join_at(&mut d, "Spammer", spawn()).await;
        let victim = join_at(&mut d, "Victim", Vec3::new(12.5, 64.0, 8.5)).await;
        let spam_key = key(&spammer);
        // Lane 64 with control reserve 16: exactly 48 data items fit.
        for i in 0..48_i16 {
            let packet =
                ServerboundPlayPacket::ServerboundSetHeldItem(ServerboundSetHeldItem::new(i % 9));
            assert!(d.router.route_native_play(&spam_key, &packet).is_ok());
        }
        let (token, sent) = disconnect(&mut d.router, &key(&victim));
        assert_eq!(sent, Some(NativeDisconnect::Sent));
        let mut classes = Vec::new();
        let mut detached_at = None;
        for iteration in 1..=40 {
            for _ in 0..WINDOW {
                let Ok(item) = d.lane.try_recv() else { break };
                let (_, class, input) = item.into_parts();
                classes.push(class);
                d.env.shard.enqueue_owned_player_input(input).unwrap();
            }
            let mut out = d.env.boundary();
            d.applied += 1;
            assert!(out.take_refusals().is_empty());
            for intent in out.intents() {
                if let OwnershipIntent::PlayerDetached { player } = intent {
                    let shard = &d.env.shard;
                    let outcome = d
                        .router
                        .native_detached(*player, &mut |h, probe| {
                            probe.submit(shard.collector_lifecycle_view(h).ok())
                        })
                        .unwrap();
                    assert_eq!(outcome, NativeDetachOutcome::Confirmed);
                    detached_at = Some(iteration);
                }
            }
            if detached_at.is_some() {
                break;
            }
            assert_eq!(
                d.service(token, true),
                Serviced::Waiting,
                "iteration {iteration}"
            );
            let status = status_of(&d, token);
            assert_eq!(status.leave_save(), NativeLeaveSaveState::AwaitDetach);
            assert_eq!(status.phase(), NativeRetentionPhase::Leaving);
        }
        // Item 49 (the disconnect) leaves the lane in iteration 13.
        assert_eq!(detached_at, Some(13));
        assert_eq!(classes.len(), 49);
        assert_eq!(
            classes
                .iter()
                .filter(|c| **c == NativeInputClass::Control)
                .count(),
            1,
            "no resend while the disconnect was queued"
        );
        assert_eq!(classes.last(), Some(&NativeInputClass::Control));
        save_to_confirmed(&mut d, token).await;
        assert_eq!(status_of(&d, token).attempts(), 1);
        assert_eq!(d.retire().retired(), 1);
        assert_eq!(d.acknowledge().acknowledged(), 1);
        assert_eq!(d.release(token), Ok(()));
        assert!(d.router.native_session_status(&spam_key).is_some());
    }
}
