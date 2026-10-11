//! SIM-03.4b: detached status, `saved_after_detach`, `lifecycle_acked`, release
//! and Stop readiness on genuine Memory **and** redb ownership admission in
//! counter mode. Every save, refusal and uncertainty comes from real
//! Work/Completion against the actual store; no fact or completion is invented.
use super::*;

/// Removes a per-test redb file; the open store may still hold it on Linux.
struct TempDb(Option<std::path::PathBuf>);
impl Drop for TempDb {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

async fn case(redb: bool, tag: &str, lifecycle: bool) -> (TempDb, Env) {
    let path = redb.then(|| {
        std::env::temp_dir().join(format!("ferrumc-sim034b-{tag}-{}.redb", std::process::id()))
    });
    if let Some(path) = &path {
        let _ = std::fs::remove_file(path);
    }
    let mut e = Env::new(path.as_deref(), 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    if lifecycle {
        activate(&mut e, 2);
    }
    (TempDb(path), e)
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

fn admitted(out: &PlayerOwnershipTickOutcome) -> OwnedPlayerHandle {
    out.intents()
        .iter()
        .find_map(|i| match i {
            OwnershipIntent::PlayerAdmitted { player, .. } => Some(player.clone()),
            _ => None,
        })
        .unwrap()
}

/// Seeds one genuine FCPS3 record in the store and restores it.
async fn admit(e: &mut Env, name: &str, seed: u128) -> OwnedPlayerHandle {
    let player = PlayerId::offline(name);
    let before = e
        .store
        .load_owned_player(ScopedPlayerKey::new(scope(), player), identity())
        .await
        .unwrap();
    let record = NativePlayerSnapshot::empty_survival(position(), [0.0; 2])
        .try_to_record()
        .unwrap();
    e.store
        .compare_exchange_owned_player(
            PlayerOwnershipCasRequest::new(
                id(9000 + seed),
                before.expectation().clone(),
                PlayerOwnershipProposal::put(record),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let read = e.binding.read_player(&e.store, player).await.unwrap();
    admitted(&e.input(OwnedPlayerInput::restore(read)))
}

async fn rejoin(e: &mut Env, p: &OwnedPlayerHandle) -> OwnedPlayerHandle {
    let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
    let out = e.input(OwnedPlayerInput::rejoin(read));
    assert!(out.refusals.is_empty());
    admitted(&out)
}

fn detach(e: &mut Env, p: &OwnedPlayerHandle) {
    let out = e.input(OwnedPlayerInput::disconnect(p.clone()));
    assert!(out.refusals.is_empty());
}

/// Admits a genuine autosave and applies its genuine confirmed completion.
async fn save(e: &mut Env, p: &OwnedPlayerHandle, n: u128) {
    let out = e.input(OwnedPlayerInput::autosave(p.clone(), id(n)));
    assert!(out.refusals.is_empty());
    let out = e.finish(out).await;
    assert!(out.refusals.is_empty());
}

fn ack_cell(e: &mut Env, p: &OwnedPlayerHandle) {
    let ticket = e
        .shard
        .collector_lifecycle_view(p)
        .unwrap()
        .retirement_ticket()
        .unwrap();
    e.shard.acknowledge_collector_lifecycle(&ticket).unwrap();
}

fn ack_head(e: &mut Env) {
    let ticket = e.shard.peek_item_publication().unwrap().unwrap().ticket();
    e.shard.acknowledge_item_publication(&ticket).unwrap();
}

fn status(e: &Env, p: &OwnedPlayerHandle) -> DetachedOwnedPlayerStatus {
    e.shard.detached_owned_player_status(p).unwrap()
}

fn refusal(e: &mut Env, p: &OwnedPlayerHandle) -> PlayerOwnershipError {
    e.shard.release_detached_owned_player(p).unwrap_err()
}

fn assert_refused(e: &mut Env, p: &OwnedPlayerHandle, expected: DetachedReleaseRefusal) {
    let before = fingerprint(e);
    match refusal(e, p) {
        PlayerOwnershipError::DetachedRelease(actual) => assert_eq!(actual, expected),
        other => panic!("expected {expected:?}, got {other:?}"),
    }
    assert_eq!(fingerprint(e), before, "refusal {expected:?} mutated state");
}

/// Copies one player's private release-relevant scalar state.
type PlayerFacts = (
    PlayerId,
    u64,
    u64,
    bool,
    bool,
    Option<OwnershipTransferId>,
    Option<OwnershipTransferId>,
    bool,
    usize,
    [u64; 3],
);

/// Every count, byte and authority scalar a release may touch.
#[derive(Debug, PartialEq)]
struct Fingerprint {
    players: Vec<PlayerFacts>,
    player_bytes: usize,
    pending: Vec<(
        OwnershipTransferId,
        OwnershipPendingStatus,
        Option<PlayerId>,
    )>,
    pending_bytes: usize,
    input: usize,
    input_bytes: usize,
    generation: u64,
    lifecycle: Option<(usize, usize, usize)>,
    publications: Option<(usize, usize)>,
    entities: usize,
    tick: Tick,
    draining: bool,
    stopped: bool,
}

fn fingerprint(e: &Env) -> Fingerprint {
    let state = e.shard.player_ownership.as_ref().unwrap();
    Fingerprint {
        players: e
            .shard
            .players
            .iter()
            .filter_map(|(id, p)| {
                p.owned.as_ref().map(|o| {
                    (
                        *id,
                        o.admission.get(),
                        o.mutation.get(),
                        o.connected,
                        o.dirty,
                        o.pending,
                        o.saved_after_detach,
                        o.lifecycle_acked,
                        o.confirmed.data().len(),
                        [
                            p.position.x.to_bits(),
                            p.position.y.to_bits(),
                            p.position.z.to_bits(),
                        ],
                    )
                })
            })
            .collect(),
        player_bytes: state.player_bytes,
        pending: state
            .pending
            .iter()
            .map(|(id, p)| (*id, p.status, p.player))
            .collect(),
        pending_bytes: state.pending_bytes,
        input: state.input.len(),
        input_bytes: state.input_bytes,
        generation: state.generation.get(),
        lifecycle: e
            .shard
            .collector_lifecycle_status()
            .ok()
            .map(|s| (s.cells(), s.reserved_take_pins(), s.retained_take_pins())),
        publications: e
            .shard
            .item_publication_status()
            .ok()
            .map(|s| (s.reserved_groups(), s.retained_groups())),
        entities: e.shard.entities().unwrap().len(),
        tick: e.tick,
        draining: state.draining,
        stopped: state.stopped,
    }
}

/// Pre-commits another operation under `transfer`: a genuine receipt collision.
async fn collide_receipt(e: &Env, transfer: OwnershipTransferId) {
    let other = PlayerId::offline("ReleaseReceiptCollision");
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

/// Restores a never-dirtying item (sentinel age, zero delay) and frees its group.
async fn quiet_item(e: &mut Env, n: u128) -> EntityHandle {
    let h = e.restore(n, 4, ItemEntityCounters::new(-32768, 0, 5)).await;
    ack_head(e);
    h
}

async fn pickup(
    e: &mut Env,
    p: &OwnedPlayerHandle,
    n: u128,
    animate: bool,
) -> PlayerOwnershipTickOutcome {
    let read = e
        .binding
        .read_pair(&e.store, p.player(), uuid(n))
        .await
        .unwrap();
    let out = e.input(OwnedPlayerInput::pickup(
        p.clone(),
        e.handle(n),
        read,
        id(n),
        animate,
        false,
    ));
    assert!(out.refusals.is_empty());
    out
}

// ---------------------------------------------------------------- success cases

#[tokio::test]
async fn release_removes_state_bytes_and_counters_after_save_then_ack_on_both_backends() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "save-then-ack", true).await;
        let other = admit(&mut e, "ReleaseOther", 2).await;
        let p = admit(&mut e, "ReleaseTarget", 1).await;
        detach(&mut e, &p);
        let fresh = status(&e, &p);
        assert_eq!(fresh.saved_after_detach(), None);
        assert_eq!(fresh.blocking(), None);
        assert!(!fresh.dirty());
        save(&mut e, &p, 10).await;
        assert_eq!(status(&e, &p).saved_after_detach(), Some(id(10)));
        ack_cell(&mut e, &p);
        let bytes = e.shard.player_ownership.as_ref().unwrap().player_bytes;
        let own = e.shard.players[&p.player()]
            .owned
            .as_ref()
            .unwrap()
            .confirmed
            .data()
            .len();
        let cells = e.shard.collector_lifecycle_status().unwrap().cells();
        let released = e.shard.release_detached_owned_player(&p).unwrap();
        assert_eq!(released.player(), p.player());
        assert_eq!(released.freed_player_bytes(), own);
        assert_eq!(e.shard.player_count(), 1);
        assert!(!e.shard.players.contains_key(&p.player()));
        assert_eq!(
            e.shard.player_ownership.as_ref().unwrap().player_bytes,
            bytes - own
        );
        assert_eq!(e.shard.collector_lifecycle_status().unwrap().cells(), cells);
        assert_eq!(e.shard.player_ownership_pending_count(), 0);
        // The unrelated connected admission is untouched.
        assert!(e.shard.connected_owned_player_view(&other).is_ok());
        // The released admission and its handle carry no further authority.
        assert!(matches!(
            e.shard.detached_owned_player_status(&p),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        // A following boundary plans with the reduced player set.
        let out = e.run();
        assert!(out.refusals.is_empty());
        // The durable record is exactly the confirmed detached save; a later
        // genuine restore admits it as a new admission.
        let restored = rejoin_after_release(&mut e, &p).await;
        assert_ne!(restored, p);
        assert_eq!(
            e.shard.players[&p.player()]
                .owned
                .as_ref()
                .unwrap()
                .confirmed
                .data()
                .len(),
            own
        );
    }
}

async fn rejoin_after_release(e: &mut Env, p: &OwnedPlayerHandle) -> OwnedPlayerHandle {
    let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
    let out = e.input(OwnedPlayerInput::restore(read));
    assert!(out.refusals.is_empty());
    admitted(&out)
}

#[tokio::test]
async fn ack_before_save_keeps_lifecycle_acked_through_confirmed_autosave_publication() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "ack-then-save", true).await;
        let p = admit(&mut e, "ReleaseAckFirst", 1).await;
        detach(&mut e, &p);
        ack_cell(&mut e, &p);
        assert!(
            e.shard.players[&p.player()]
                .owned
                .as_ref()
                .unwrap()
                .lifecycle_acked
        );
        assert_refused(&mut e, &p, DetachedReleaseRefusal::NotSaved);
        save(&mut e, &p, 11).await;
        // The non-admitting autosave publication preserved the acknowledgement.
        assert!(
            e.shard.players[&p.player()]
                .owned
                .as_ref()
                .unwrap()
                .lifecycle_acked
        );
        let released = e.shard.release_detached_owned_player(&p).unwrap();
        assert_eq!(released.player(), p.player());
        assert_eq!(e.shard.player_count(), 0);
        assert_eq!(e.shard.player_ownership.as_ref().unwrap().player_bytes, 0);
        // With nothing left, Stop readiness and actual Stop agree.
        assert!(e.shard.owned_stop_ready().unwrap());
        let out = e.input(OwnedPlayerInput::stop());
        assert!(out.refusals.is_empty());
        assert!(matches!(
            out.intents().last(),
            Some(OwnershipIntent::Stopped)
        ));
    }
}

#[tokio::test]
async fn autosave_admitted_connected_and_confirmed_detached_counts_but_connected_confirm_does_not()
{
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "confirm-time", true).await;
        // Admitted while connected, confirmed after the genuine disconnect.
        let p = admit(&mut e, "ReleaseConfirmDetached", 1).await;
        let out = e.input(OwnedPlayerInput::autosave(p.clone(), id(20)));
        detach(&mut e, &p);
        assert_eq!(
            status(&e, &p).blocking(),
            Some((id(20), OwnershipPendingStatus::InFlight))
        );
        e.finish(out).await;
        assert_eq!(status(&e, &p).saved_after_detach(), Some(id(20)));
        // Confirmed while connected: no detached proof survives the disconnect.
        let q = admit(&mut e, "ReleaseConfirmConnected", 2).await;
        save(&mut e, &q, 21).await;
        detach(&mut e, &q);
        assert_eq!(status(&e, &q).saved_after_detach(), None);
        ack_cell(&mut e, &q);
        assert_refused(&mut e, &q, DetachedReleaseRefusal::NotSaved);
    }
}

// ------------------------------------------------- saved_after_detach semantics

#[tokio::test]
async fn saved_after_detach_latest_confirmed_wins_new_pending_resets_and_uncertainty_blocks() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "saved-latest", true).await;
        let p = admit(&mut e, "ReleaseLatest", 1).await;
        detach(&mut e, &p);
        ack_cell(&mut e, &p);
        save(&mut e, &p, 30).await;
        save(&mut e, &p, 31).await;
        assert_eq!(status(&e, &p).saved_after_detach(), Some(id(31)));
        // First retry refused definitively by a genuine receipt collision.
        let mut out = e.input(OwnedPlayerInput::autosave(p.clone(), id(32)));
        assert_eq!(status(&e, &p).saved_after_detach(), None);
        let work = out.take_work().pop().unwrap();
        collide_receipt(&e, id(32)).await;
        let completion = work.execute(&e.store).await.unwrap();
        let out = e.input(OwnedPlayerInput::complete(completion));
        assert!(matches!(
            out.refusals[0].reason,
            PlayerOwnershipError::Storage(OwnershipError::ReceiptCollision { .. })
        ));
        assert_eq!(
            status(&e, &p),
            DetachedOwnedPlayerStatus {
                saved_after_detach: None,
                blocking: None,
                dirty: false,
            }
        );
        assert_refused(&mut e, &p, DetachedReleaseRefusal::NotSaved);
        // Second retry confirmed: it is the one that counts.
        save(&mut e, &p, 33).await;
        assert_eq!(status(&e, &p).saved_after_detach(), Some(id(33)));
        // A later pending resets the proof and blocks the release.
        let mut out = e.input(OwnedPlayerInput::autosave(p.clone(), id(34)));
        let work = out.take_work().pop().unwrap();
        let query = work.query();
        assert_eq!(
            status(&e, &p),
            DetachedOwnedPlayerStatus {
                saved_after_detach: None,
                blocking: Some((id(34), OwnershipPendingStatus::InFlight)),
                dirty: false,
            }
        );
        assert_refused(&mut e, &p, DetachedReleaseRefusal::Pending);
        // Lost reply: genuine readonly clarification finds no receipt.
        drop(work);
        let mut out = e.input(OwnedPlayerInput::reconcile(query));
        let completion = out
            .take_work()
            .pop()
            .unwrap()
            .execute(&e.store)
            .await
            .unwrap();
        let mut out = e.input(OwnedPlayerInput::complete(completion));
        // Dropping the refused completion releases its lease (N3).
        assert_eq!(out.take_refusals().len(), 1);
        assert_eq!(
            status(&e, &p).blocking(),
            Some((id(34), OwnershipPendingStatus::Uncertain))
        );
        assert_eq!(status(&e, &p).saved_after_detach(), None);
        assert_refused(&mut e, &p, DetachedReleaseRefusal::Pending);
        // The old worker genuinely commits later; clarification confirms it.
        let request = e.shard.player_ownership.as_ref().unwrap().pending[&id(34)]
            .request
            .clone();
        let WorkRequest::Player(request) = request else {
            panic!("autosave must be a player CAS");
        };
        e.store
            .compare_exchange_owned_player(request)
            .await
            .unwrap();
        let mut out = e.input(OwnedPlayerInput::reconcile(query));
        let completion = out
            .take_work()
            .pop()
            .unwrap()
            .execute(&e.store)
            .await
            .unwrap();
        let out = e.input(OwnedPlayerInput::complete(completion));
        assert!(out.refusals.is_empty());
        assert_eq!(status(&e, &p).saved_after_detach(), Some(id(34)));
        assert!(e.shard.release_detached_owned_player(&p).is_ok());
    }
}

#[tokio::test]
async fn deferred_apply_after_detached_confirm_clears_proof_and_marks_unsaved() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "deferred", true).await;
        let p = admit(&mut e, "ReleaseDeferred", 1).await;
        let out = e.input(OwnedPlayerInput::autosave(p.clone(), id(40)));
        // Movement while pending is deferred, then the player detaches.
        let moved = e.input(OwnedPlayerInput::move_player(
            p.clone(),
            Some(Vec3::new(3.0, 64.0, 1.0)),
            [None; 2],
        ));
        assert!(moved.refusals.is_empty());
        detach(&mut e, &p);
        assert!(status(&e, &p).dirty());
        e.finish(out).await;
        // The confirmed save did not include the deferred pose that was then applied.
        let after = status(&e, &p);
        assert_eq!(after.saved_after_detach(), None);
        assert!(after.dirty());
        assert_eq!(after.blocking(), None);
        ack_cell(&mut e, &p);
        assert_refused(&mut e, &p, DetachedReleaseRefusal::Unsaved);
        save(&mut e, &p, 41).await;
        let after = status(&e, &p);
        assert_eq!(after.saved_after_detach(), Some(id(41)));
        assert!(!after.dirty());
        assert!(e.shard.release_detached_owned_player(&p).is_ok());
    }
}

#[tokio::test]
async fn rejoin_publication_clears_proof_and_acknowledgement_of_the_new_admission() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "rejoin-reset", true).await;
        let old = admit(&mut e, "ReleaseRejoin", 1).await;
        detach(&mut e, &old);
        save(&mut e, &old, 50).await;
        ack_cell(&mut e, &old);
        let new = rejoin(&mut e, &old).await;
        assert!(matches!(
            e.shard.detached_owned_player_status(&old),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        assert!(matches!(
            e.shard.release_detached_owned_player(&old),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        detach(&mut e, &new);
        let owned = e.shard.players[&new.player()].owned.as_ref().unwrap();
        assert_eq!(owned.saved_after_detach, None);
        assert!(!owned.lifecycle_acked);
        assert_refused(
            &mut e,
            &new,
            DetachedReleaseRefusal::LifecycleNotAcknowledged,
        );
    }
}

// --------------------------------------------------- readonly status refusals

#[tokio::test]
async fn detached_status_refuses_connected_stale_foreign_snapshot_and_uninitialized_without_mutation(
) {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "status-a", true).await;
        let (_db_b, mut b) = case(redb, "status-b", true).await;
        let p = admit(&mut e, "ReleaseStatus", 1).await;
        let foreign = admit(&mut b, "ReleaseStatus", 1).await;
        assert_eq!(p.player(), foreign.player());
        let before = fingerprint(&e);
        assert!(matches!(
            e.shard.detached_owned_player_status(&p),
            Err(PlayerOwnershipError::Ineligible("connected player"))
        ));
        detach(&mut b, &foreign);
        assert!(matches!(
            e.shard.detached_owned_player_status(&foreign),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        let snapshot = e.shard.clone();
        assert!(matches!(
            snapshot.detached_owned_player_status(&p),
            Err(PlayerOwnershipError::SnapshotOnly)
        ));
        let plain = SimShard::with_generation(ShardPos::new(0, 0), e.binding.generation());
        assert!(matches!(
            plain.detached_owned_player_status(&p),
            Err(PlayerOwnershipError::NotInitialized)
        ));
        assert_eq!(fingerprint(&e), before);
        detach(&mut e, &p);
        let new = rejoin(&mut e, &p).await;
        assert!(matches!(
            e.shard.detached_owned_player_status(&p),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        assert!(matches!(
            e.shard.detached_owned_player_status(&new),
            Err(PlayerOwnershipError::Ineligible("connected player"))
        ));
    }
}

// ----------------------------------------------------------- release refusals

#[tokio::test]
async fn release_refuses_connected_snapshot_foreign_stale_and_repeated_handles_without_mutation() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "handles-a", true).await;
        let (_db_b, mut b) = case(redb, "handles-b", true).await;
        let p = admit(&mut e, "ReleaseHandles", 1).await;
        assert_refused(&mut e, &p, DetachedReleaseRefusal::Connected);
        detach(&mut e, &p);
        save(&mut e, &p, 60).await;
        ack_cell(&mut e, &p);
        // Foreign owner with the same UUID, fully releasable in its own shard.
        let foreign = admit(&mut b, "ReleaseHandles", 1).await;
        detach(&mut b, &foreign);
        save(&mut b, &foreign, 60).await;
        ack_cell(&mut b, &foreign);
        let before = fingerprint(&e);
        assert!(matches!(
            e.shard.release_detached_owned_player(&foreign),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        let mut snapshot = e.shard.clone();
        assert!(matches!(
            snapshot.release_detached_owned_player(&p),
            Err(PlayerOwnershipError::SnapshotOnly)
        ));
        assert_eq!(snapshot.player_count(), 1);
        assert_eq!(fingerprint(&e), before);
        let released = e.shard.release_detached_owned_player(&p).unwrap();
        assert_eq!(released.player(), p.player());
        let after = fingerprint(&e);
        assert!(matches!(
            e.shard.release_detached_owned_player(&p),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        assert_eq!(fingerprint(&e), after);
        // The foreign shard still releases its own admission.
        assert!(b.shard.release_detached_owned_player(&foreign).is_ok());
    }
}

#[tokio::test]
async fn release_without_lifecycle_mode_is_a_typed_refusal_not_an_acknowledgement() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "no-lifecycle", false).await;
        let p = admit(&mut e, "ReleaseNoLifecycle", 1).await;
        detach(&mut e, &p);
        save(&mut e, &p, 70).await;
        assert_eq!(status(&e, &p).saved_after_detach(), Some(id(70)));
        assert_refused(&mut e, &p, DetachedReleaseRefusal::LifecycleNotInitialized);
    }
}

#[tokio::test]
async fn release_gates_flip_one_at_a_time_unacked_unsaved_pending_then_success() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "flip", true).await;
        let p = admit(&mut e, "ReleaseFlip", 1).await;
        detach(&mut e, &p);
        // N-1: everything but acknowledgement and save.
        assert_refused(&mut e, &p, DetachedReleaseRefusal::LifecycleNotAcknowledged);
        let out = e.input(OwnedPlayerInput::autosave(p.clone(), id(80)));
        assert_refused(&mut e, &p, DetachedReleaseRefusal::Pending);
        e.finish(out).await;
        assert_refused(&mut e, &p, DetachedReleaseRefusal::LifecycleNotAcknowledged);
        // N: the last missing gate, acknowledged by the genuine D8 ticket.
        let ticket = e
            .shard
            .collector_lifecycle_view(&p)
            .unwrap()
            .retirement_ticket()
            .unwrap();
        e.shard.acknowledge_collector_lifecycle(&ticket).unwrap();
        // A repeated acknowledgement is refused and changes nothing.
        let before = fingerprint(&e);
        assert!(e.shard.acknowledge_collector_lifecycle(&ticket).is_err());
        assert_eq!(fingerprint(&e), before);
        assert!(e.shard.release_detached_owned_player(&p).is_ok());
        // N+1: nothing remains to release.
        assert!(matches!(
            e.shard.release_detached_owned_player(&p),
            Err(PlayerOwnershipError::StaleHandle)
        ));
    }
}

#[tokio::test]
async fn queued_inputs_naming_the_player_refuse_while_unrelated_queued_inputs_do_not() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "queued", true).await;
        let p = admit(&mut e, "ReleaseQueued", 1).await;
        detach(&mut e, &p);
        save(&mut e, &p, 90).await;
        ack_cell(&mut e, &p);
        for step in 0..4 {
            let input = match step {
                0 => OwnedPlayerInput::disconnect(p.clone()),
                1 => OwnedPlayerInput::autosave(p.clone(), id(91)),
                2 => OwnedPlayerInput::move_player(p.clone(), Some(position()), [None; 2]),
                _ => OwnedPlayerInput::rejoin(
                    e.binding.read_player(&e.store, p.player()).await.unwrap(),
                ),
            };
            e.shard.enqueue_owned_player_input(input).unwrap();
            assert_refused(&mut e, &p, DetachedReleaseRefusal::QueuedInput);
            // Each queued input is applied or refused by a genuine boundary.
            let out = e.run();
            if !out.work.is_empty() {
                e.finish(out).await;
            }
        }
        // The queued rejoin was applied: the old admission is stale now.
        assert!(matches!(
            e.shard.release_detached_owned_player(&p),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        // Restore and bootstrap reads of the same UUID are queued inputs too.
        let (_db2, mut f) = case(redb, "queued-reads", true).await;
        let p = admit(&mut f, "ReleaseQueuedReads", 1).await;
        detach(&mut f, &p);
        save(&mut f, &p, 92).await;
        ack_cell(&mut f, &p);
        for bootstrap in [false, true] {
            let read = f.binding.read_player(&f.store, p.player()).await.unwrap();
            let input = if bootstrap {
                OwnedPlayerInput::bootstrap(read, position(), [0.0; 2], id(93))
            } else {
                OwnedPlayerInput::restore(read)
            };
            f.shard.enqueue_owned_player_input(input).unwrap();
            assert_refused(&mut f, &p, DetachedReleaseRefusal::QueuedInput);
            let out = f.run();
            assert!(matches!(
                out.refusals[0].reason,
                PlayerOwnershipError::AlreadyOwned | PlayerOwnershipError::Ineligible(_)
            ));
        }
        // An unrelated queued input of another player does not block.
        let unrelated = admit(&mut f, "ReleaseUnrelated", 3).await;
        f.shard
            .enqueue_owned_player_input(OwnedPlayerInput::move_player(
                unrelated,
                Some(position()),
                [None; 2],
            ))
            .unwrap();
        // Fill the queue to its 32-input cap with unrelated inputs: the bounded
        // scan still releases, and the queue keeps every input and byte charge.
        while f
            .shard
            .enqueue_owned_player_input(OwnedPlayerInput::drain())
            .is_ok()
        {}
        let queue = {
            let state = f.shard.player_ownership.as_ref().unwrap();
            (state.input.len(), state.input_bytes)
        };
        assert_eq!(queue.0, 32);
        assert!(f.shard.release_detached_owned_player(&p).is_ok());
        let state = f.shard.player_ownership.as_ref().unwrap();
        assert_eq!((state.input.len(), state.input_bytes), queue);
    }
}

#[tokio::test]
async fn pinned_reserved_and_retained_take_and_item_reservation_refuse_typed() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "pins", true).await;
        let p = admit(&mut e, "ReleasePins", 1).await;
        quiet_item(&mut e, 1).await;
        quiet_item(&mut e, 2).await;
        // Animated pickup reserves a Take pin; detach while it is pending.
        let out = pickup(&mut e, &p, 1, true).await;
        detach(&mut e, &p);
        assert_refused(&mut e, &p, DetachedReleaseRefusal::PublicationPin);
        e.finish(out).await;
        // Confirmed: the pin is retained until the FIFO head is acknowledged.
        assert_refused(&mut e, &p, DetachedReleaseRefusal::PublicationPin);
        ack_head(&mut e);
        assert_refused(&mut e, &p, DetachedReleaseRefusal::LifecycleNotAcknowledged);
        save(&mut e, &p, 100).await;
        ack_cell(&mut e, &p);
        assert!(e.shard.release_detached_owned_player(&p).is_ok());
        // Non-animated pair pickup: entity owner reservation without any pin.
        let q = admit(&mut e, "ReleaseReservation", 2).await;
        let out = pickup(&mut e, &q, 2, false).await;
        detach(&mut e, &q);
        assert_refused(&mut e, &q, DetachedReleaseRefusal::ItemReservation);
        e.finish(out).await;
        save(&mut e, &q, 101).await;
        ack_head(&mut e);
        ack_cell(&mut e, &q);
        assert!(e.shard.release_detached_owned_player(&q).is_ok());
    }
}

#[tokio::test]
async fn older_unacknowledged_admission_cell_is_a_collector_binding_refusal() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "binding", true).await;
        let first = admit(&mut e, "ReleaseBinding", 1).await;
        detach(&mut e, &first);
        let second = rejoin(&mut e, &first).await;
        detach(&mut e, &second);
        save(&mut e, &second, 110).await;
        // Acknowledging the old cell never marks the newer admission.
        ack_cell(&mut e, &first);
        assert_refused(
            &mut e,
            &second,
            DetachedReleaseRefusal::LifecycleNotAcknowledged,
        );
        let (_db2, mut f) = case(redb, "binding-b", true).await;
        let first = admit(&mut f, "ReleaseBinding", 1).await;
        detach(&mut f, &first);
        let second = rejoin(&mut f, &first).await;
        detach(&mut f, &second);
        save(&mut f, &second, 111).await;
        ack_cell(&mut f, &second);
        assert_refused(&mut f, &second, DetachedReleaseRefusal::CollectorBinding);
        ack_cell(&mut f, &first);
        assert!(f.shard.release_detached_owned_player(&second).is_ok());
        assert_eq!(f.shard.collector_lifecycle_status().unwrap().cells(), 0);
        drop(e);
    }
}

#[tokio::test]
async fn stopped_owner_refuses_release_and_stop_readiness_typed() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "stopped", true).await;
        let p = admit(&mut e, "ReleaseStopped", 1).await;
        detach(&mut e, &p);
        save(&mut e, &p, 120).await;
        ack_cell(&mut e, &p);
        // A clean detached player does not prevent Stop.
        assert!(e.shard.owned_stop_ready().unwrap());
        let out = e.input(OwnedPlayerInput::stop());
        assert!(out.refusals.is_empty());
        let before = fingerprint(&e);
        assert!(matches!(
            e.shard.release_detached_owned_player(&p),
            Err(PlayerOwnershipError::Draining)
        ));
        assert!(matches!(
            e.shard.owned_stop_ready(),
            Err(PlayerOwnershipError::Draining)
        ));
        assert_eq!(fingerprint(&e), before);
    }
}

// ---------------------------------------------------------------------- races

#[tokio::test]
async fn release_versus_rejoin_in_both_orders_is_deterministic() {
    for redb in [false, true] {
        // Rejoin input queued first: release refuses, the boundary rejoins.
        let (_db, mut e) = case(redb, "race-rejoin-first", true).await;
        let p = admit(&mut e, "ReleaseRaceA", 1).await;
        detach(&mut e, &p);
        save(&mut e, &p, 130).await;
        ack_cell(&mut e, &p);
        let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
        e.shard
            .enqueue_owned_player_input(OwnedPlayerInput::rejoin(read))
            .unwrap();
        assert_refused(&mut e, &p, DetachedReleaseRefusal::QueuedInput);
        let new = admitted(&e.run());
        assert!(matches!(
            e.shard.release_detached_owned_player(&p),
            Err(PlayerOwnershipError::StaleHandle)
        ));
        assert!(e.shard.connected_owned_player_view(&new).is_ok());
        // Release first: a rejoin with the read taken before the release is
        // refused without admission; a fresh restore admits the saved record.
        let (_db2, mut f) = case(redb, "race-release-first", true).await;
        let p = admit(&mut f, "ReleaseRaceB", 1).await;
        detach(&mut f, &p);
        save(&mut f, &p, 131).await;
        ack_cell(&mut f, &p);
        let early = f.binding.read_player(&f.store, p.player()).await.unwrap();
        f.shard.release_detached_owned_player(&p).unwrap();
        let out = f.input(OwnedPlayerInput::rejoin(early));
        assert!(matches!(
            out.refusals[0].reason,
            PlayerOwnershipError::StaleHandle
        ));
        assert_eq!(f.shard.player_count(), 0);
        let restored = rejoin_after_release(&mut f, &p).await;
        assert!(f.shard.connected_owned_player_view(&restored).is_ok());
    }
}

#[tokio::test]
async fn player_cap_n_minus_one_n_n_plus_one_has_no_automatic_eviction() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "cap", true).await;
        let mut handles = Vec::new();
        for n in 1..=4 {
            handles.push(admit(&mut e, &format!("ReleaseCap{n}"), n).await);
        }
        let victim = handles[0].clone();
        detach(&mut e, &victim);
        save(&mut e, &victim, 140).await;
        ack_cell(&mut e, &victim);
        // N = cap: a fifth UUID is refused although a releasable player exists.
        let fifth = PlayerId::offline("ReleaseCap5");
        let read = e.binding.read_player(&e.store, fifth).await.unwrap();
        let out = e.input(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [0.0; 2],
            id(141),
        ));
        assert!(matches!(
            out.refusals[0].reason,
            PlayerOwnershipError::Capacity("player count")
        ));
        assert_eq!(e.shard.player_count(), 4);
        assert!(e.shard.players.contains_key(&victim.player()));
        // N-1 after the explicit release: the next boundary admits the fifth.
        e.shard.release_detached_owned_player(&victim).unwrap();
        assert_eq!(e.shard.player_count(), 3);
        let read = e.binding.read_player(&e.store, fifth).await.unwrap();
        let out = e.input(OwnedPlayerInput::bootstrap(
            read,
            position(),
            [0.0; 2],
            id(142),
        ));
        assert!(out.refusals.is_empty());
        let out = e.finish(out).await;
        let admitted_fifth = admitted(&out);
        assert_eq!(admitted_fifth.player(), fifth);
        assert_eq!(e.shard.player_count(), 4);
    }
}

// --------------------------------------------------------- Stop readiness

async fn stop_case(redb: bool, kind: u8) -> (TempDb, Env, bool) {
    let (db, mut e) = case(redb, &format!("stop-{kind}"), true).await;
    let ready = match kind {
        // Empty owner.
        0 => true,
        // Dirty connected player after genuine movement.
        1 => {
            let p = admit(&mut e, "StopDirty", 1).await;
            e.input(OwnedPlayerInput::move_player(
                p,
                Some(Vec3::new(2.0, 64.0, 1.0)),
                [None; 2],
            ));
            false
        }
        // Pending autosave whose work is still outstanding.
        2 => {
            let p = admit(&mut e, "StopPending", 1).await;
            let mut out = e.input(OwnedPlayerInput::autosave(p, id(150)));
            drop(out.take_work());
            false
        }
        // Detached, saved, unacknowledged lifecycle cell.
        3 => {
            let p = admit(&mut e, "StopCell", 1).await;
            detach(&mut e, &p);
            save(&mut e, &p, 151).await;
            false
        }
        // Item whose counters genuinely advanced past its confirmed generation.
        4 => {
            e.restore(1, 1, ItemEntityCounters::new(0, 0, 5)).await;
            ack_head(&mut e);
            e.run();
            false
        }
        // Retained publication group not yet acknowledged.
        5 => {
            e.restore(1, 1, ItemEntityCounters::new(-32768, 32767, 5))
                .await;
            false
        }
        // Everything clarified: a released detached player and a clean item.
        6 => {
            let p = admit(&mut e, "StopCleanDetached", 2).await;
            detach(&mut e, &p);
            save(&mut e, &p, 152).await;
            ack_cell(&mut e, &p);
            e.shard.release_detached_owned_player(&p).unwrap();
            e.restore(3, 1, ItemEntityCounters::new(-32768, 32767, 5))
                .await;
            ack_head(&mut e);
            true
        }
        // A clean connected admission still holds its lifecycle cell.
        7 => {
            admit(&mut e, "StopCleanConnected", 1).await;
            false
        }
        _ => unreachable!(),
    };
    (db, e, ready)
}

#[tokio::test]
async fn owned_stop_ready_is_true_exactly_when_stop_alone_is_accepted_next_boundary() {
    for redb in [false, true] {
        for kind in 0..=7 {
            let (_db, mut e, expected) = stop_case(redb, kind).await;
            assert!(e.shard.player_ownership.as_ref().unwrap().input.is_empty());
            let ready = e.shard.owned_stop_ready().unwrap();
            assert_eq!(ready, expected, "case {kind} redb {redb}");
            // Readiness is readonly.
            let before = fingerprint(&e);
            assert_eq!(e.shard.owned_stop_ready().unwrap(), ready);
            assert_eq!(fingerprint(&e), before);
            let out = e.input(OwnedPlayerInput::stop());
            let stopped = matches!(out.intents().last(), Some(OwnershipIntent::Stopped));
            assert_eq!(stopped, ready, "case {kind} redb {redb}");
            if ready {
                assert!(out.refusals.is_empty());
                assert_eq!(out.counter_steps(), 0);
            } else {
                assert!(matches!(
                    out.refusals[0].reason,
                    PlayerOwnershipError::NotQuiescent
                ));
            }
        }
    }
}

#[tokio::test]
async fn owned_stop_ready_is_false_with_any_queued_input_and_refuses_snapshots() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "stop-queued", true).await;
        assert!(e.shard.owned_stop_ready().unwrap());
        e.shard
            .enqueue_owned_player_input(OwnedPlayerInput::drain())
            .unwrap();
        assert!(!e.shard.owned_stop_ready().unwrap());
        let snapshot = e.shard.clone();
        assert!(matches!(
            snapshot.owned_stop_ready(),
            Err(PlayerOwnershipError::SnapshotOnly)
        ));
        e.run();
        assert!(e.shard.owned_stop_ready().unwrap());
        let plain = SimShard::with_generation(ShardPos::new(0, 0), e.binding.generation());
        assert!(matches!(
            plain.owned_stop_ready(),
            Err(PlayerOwnershipError::NotInitialized)
        ));
    }
}

// ------------------------------------------------- diagnostic accessor (R3-H2)

#[tokio::test]
async fn refusal_query_accessor_names_completion_and_retry_inputs_only() {
    for redb in [false, true] {
        let (_db, mut e) = case(redb, "query-accessor", true).await;
        let p = admit(&mut e, "ReleaseAccessor", 1).await;
        let mut out = e.input(OwnedPlayerInput::autosave(p.clone(), id(160)));
        let work = out.take_work().pop().unwrap();
        let query = work.query();
        collide_receipt(&e, id(160)).await;
        let completion = work.execute(&e.store).await.unwrap();
        let mut out = e.input(OwnedPlayerInput::complete(completion));
        let refused = out.take_refusals();
        assert_eq!(refused[0].query(), Some(query));
        let mut out = e.input(OwnedPlayerInput::retry(query));
        assert_eq!(out.take_refusals()[0].query(), Some(query));
        detach(&mut e, &p);
        let mut out = e.input(OwnedPlayerInput::move_player(
            p,
            Some(position()),
            [None; 2],
        ));
        assert_eq!(out.take_refusals()[0].query(), None);
    }
}
