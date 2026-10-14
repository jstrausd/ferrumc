//! SIM-03.4c: `ConnectedOwnedPlayerView::selected` on genuine Memory **and** redb
//! ownership admission in counter mode. Every admission, save and completion
//! comes from real Work against the actual store; the view is compared with the
//! private owner field and with the durable FCPS3 `selected` byte.
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

async fn case(redb: bool, tag: &str) -> (TempDb, Env) {
    let path = redb.then(|| {
        std::env::temp_dir().join(format!("ferrumc-sim034c-{tag}-{}.redb", std::process::id()))
    });
    if let Some(path) = &path {
        let _ = std::fs::remove_file(path);
    }
    let mut e = Env::new(path.as_deref(), 4, limits(10000, 6000, 1024), Tick::ZERO).await;
    e.shard
        .initialize_item_publications(
            ItemPublicationLimits::new(
                nz(2),
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
    (TempDb(path), e)
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

fn resynced(out: &PlayerOwnershipTickOutcome, p: &OwnedPlayerHandle) -> bool {
    out.intents()
        .iter()
        .any(|i| matches!(i, OwnershipIntent::Resync { player, .. } if player == p))
}

fn no_resync(out: &PlayerOwnershipTickOutcome) -> bool {
    out.intents()
        .iter()
        .all(|i| !matches!(i, OwnershipIntent::Resync { .. }))
}

/// Typed slot read through the public view of exactly this admission.
fn view_slot(e: &Env, p: &OwnedPlayerHandle) -> u8 {
    let view = e.shard.connected_owned_player_view(p).unwrap();
    assert!(view.matches_owned_player(p));
    view.selected().index()
}

/// The private owner field that `select_hotbar` writes and saves materialize.
fn field(e: &Env, p: &OwnedPlayerHandle) -> (u8, Option<u8>) {
    let owned = e.shard.actual_player(p).unwrap().owned.as_ref().unwrap();
    (owned.selected, owned.deferred_selected)
}

/// The `selected` byte of the current durable FCPS3 record.
async fn durable(e: &Env, player: PlayerId) -> u8 {
    let snapshot = e
        .store
        .load_owned_player(ScopedPlayerKey::new(scope(), player), identity())
        .await
        .unwrap();
    NativePlayerSnapshot::try_from_record(snapshot.record().unwrap())
        .unwrap()
        .selected()
}

/// Asserts view, owner field (nothing deferred) and durable record agree.
async fn assert_slot(e: &Env, p: &OwnedPlayerHandle, live: u8, stored: u8) {
    assert_eq!(view_slot(e, p), live);
    assert_eq!(field(e, p), (live, None));
    assert_eq!(durable(e, p.player()).await, stored);
}

async fn restore(e: &mut Env, name: &str, seed: u128, selected: u8) -> OwnedPlayerHandle {
    let player = PlayerId::offline(name);
    let before = e
        .store
        .load_owned_player(ScopedPlayerKey::new(scope(), player), identity())
        .await
        .unwrap();
    let record = NativePlayerSnapshot::new(
        position(),
        [0.0; 2],
        GameMode::Survival,
        selected,
        Box::new(std::array::from_fn(|_| ItemStack::empty())),
        ItemStack::empty(),
    )
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
    let out = e.input(OwnedPlayerInput::restore(read));
    assert!(out.refusals.is_empty());
    admitted(&out)
}

async fn save(e: &mut Env, p: &OwnedPlayerHandle, n: u128) {
    let out = e.input(OwnedPlayerInput::autosave(p.clone(), id(n)));
    assert!(out.refusals.is_empty());
    let out = e.finish(out).await;
    assert!(out.refusals.is_empty());
}

fn select(e: &mut Env, p: &OwnedPlayerHandle, slot: u8) -> PlayerOwnershipTickOutcome {
    e.input(OwnedPlayerInput::select_hotbar(p.clone(), slot))
}

fn refused(out: &PlayerOwnershipTickOutcome) -> &PlayerOwnershipError {
    assert_eq!(out.refusals.len(), 1);
    out.refusals[0].reason()
}

#[test]
fn hotbar_slot_accepts_exactly_zero_to_eight() {
    for index in 0..=8 {
        assert_eq!(HotbarSlot::new(index).unwrap().index(), index);
    }
    for index in [9, 10, 36, u8::MAX] {
        assert_eq!(HotbarSlot::new(index), None);
    }
    assert_eq!(HotbarSlot::COUNT, 9);
    assert!(HotbarSlot::new(2) < HotbarSlot::new(3));
}

async fn bootstrap_and_restore(redb: bool) {
    let (_db, mut e) = case(redb, "admit").await;
    let fresh = PlayerId::offline("SelectedBootstrap");
    let read = e.binding.read_player(&e.store, fresh).await.unwrap();
    let out = e.input(OwnedPlayerInput::bootstrap(
        read,
        position(),
        [0.0; 2],
        id(1),
    ));
    assert!(out.refusals.is_empty());
    // Not admitted before its genuine confirmed completion.
    assert!(out
        .intents()
        .iter()
        .all(|i| !matches!(i, OwnershipIntent::PlayerAdmitted { .. })));
    let out = e.finish(out).await;
    assert!(out.refusals.is_empty());
    let boot = admitted(&out);
    assert_slot(&e, &boot, 0, 0).await;

    for (seed, slot) in [(1, 8), (2, 0), (3, 5)] {
        let p = restore(&mut e, &format!("SelectedRestore{seed}"), seed, slot).await;
        assert_slot(&e, &p, slot, slot).await;
    }
    // The bootstrap admission is unaffected by later admissions.
    assert_slot(&e, &boot, 0, 0).await;
}

#[tokio::test]
async fn bootstrap_and_restore_seed_the_view_from_the_durable_record_memory() {
    bootstrap_and_restore(false).await;
}

#[tokio::test]
async fn bootstrap_and_restore_seed_the_view_from_the_durable_record_redb() {
    bootstrap_and_restore(true).await;
}

async fn applied_refused_and_deferred(redb: bool) {
    let (_db, mut e) = case(redb, "select").await;
    let p = restore(&mut e, "SelectedLive", 1, 5).await;

    // Applied: the view follows the field at once; the store follows a save.
    let out = select(&mut e, &p, 7);
    assert!(out.refusals.is_empty());
    assert!(out
        .intents()
        .iter()
        .any(|i| matches!(i, OwnershipIntent::PlayerChanged { player, .. } if *player == p)));
    assert_slot(&e, &p, 7, 5).await;
    save(&mut e, &p, 10).await;
    assert_slot(&e, &p, 7, 7).await;

    // Refused out of range: unchanged, with the Resync a held-slot resync reads.
    let out = select(&mut e, &p, 9);
    assert!(matches!(
        refused(&out),
        PlayerOwnershipError::Malformed("selection")
    ));
    assert!(resynced(&out, &p));
    assert_slot(&e, &p, 7, 7).await;

    // Not yet applied: deferred behind a genuine in-flight autosave.
    let pending = e.input(OwnedPlayerInput::autosave(p.clone(), id(11)));
    assert!(pending.refusals.is_empty());
    let out = select(&mut e, &p, 2);
    assert!(out.refusals.is_empty());
    assert_eq!(view_slot(&e, &p), 7);
    assert_eq!(field(&e, &p), (7, Some(2)));
    // A later deferred selection replaces the earlier one, still invisible.
    let out = select(&mut e, &p, 3);
    assert!(out.refusals.is_empty());
    assert_eq!(view_slot(&e, &p), 7);
    assert_eq!(field(&e, &p), (7, Some(3)));
    // The genuine completion confirms the captured 7, then applies the deferral.
    let out = e.finish(pending).await;
    assert!(out.refusals.is_empty());
    assert_slot(&e, &p, 3, 7).await;
    save(&mut e, &p, 12).await;
    assert_slot(&e, &p, 3, 3).await;
}

#[tokio::test]
async fn applied_refused_and_deferred_selection_memory() {
    applied_refused_and_deferred(false).await;
}

#[tokio::test]
async fn applied_refused_and_deferred_selection_redb() {
    applied_refused_and_deferred(true).await;
}

async fn detach_and_rejoin(redb: bool) {
    let (_db, mut e) = case(redb, "rejoin").await;
    let p = restore(&mut e, "SelectedRejoin", 1, 1).await;
    let out = select(&mut e, &p, 4);
    assert!(out.refusals.is_empty());
    assert_slot(&e, &p, 4, 1).await;

    let out = e.input(OwnedPlayerInput::disconnect(p.clone()));
    assert!(out.refusals.is_empty());
    assert!(matches!(
        e.shard.connected_owned_player_view(&p),
        Err(PlayerOwnershipError::Ineligible("detached player"))
    ));
    assert!(e
        .shard
        .collector_lifecycle_view(&p)
        .unwrap()
        .live()
        .is_none());
    // A detached selection is refused and leaves the field unchanged.
    let out = select(&mut e, &p, 6);
    assert!(matches!(
        refused(&out),
        PlayerOwnershipError::Ineligible("detached selection")
    ));
    assert_eq!(field(&e, &p), (4, None));

    // The leave-save persists the applied selection; rejoin reads it back.
    save(&mut e, &p, 10).await;
    assert_eq!(durable(&e, p.player()).await, 4);
    let read = e.binding.read_player(&e.store, p.player()).await.unwrap();
    let out = e.input(OwnedPlayerInput::rejoin(read));
    assert!(out.refusals.is_empty());
    let q = admitted(&out);
    assert_ne!(p, q);
    assert_slot(&e, &q, 4, 4).await;
    let history = e.shard.collector_lifecycle_view(&q).unwrap();
    assert_eq!(history.live().unwrap().selected().index(), 4);

    // The original admission stays stale: no view, and its input changes nothing.
    assert!(matches!(
        e.shard.connected_owned_player_view(&p),
        Err(PlayerOwnershipError::StaleHandle)
    ));
    assert!(e
        .shard
        .collector_lifecycle_view(&p)
        .unwrap()
        .live()
        .is_none());
    let out = select(&mut e, &p, 8);
    assert!(matches!(refused(&out), PlayerOwnershipError::StaleHandle));
    assert!(no_resync(&out));
    assert_slot(&e, &q, 4, 4).await;

    let out = select(&mut e, &q, 0);
    assert!(out.refusals.is_empty());
    assert_slot(&e, &q, 0, 4).await;
}

#[tokio::test]
async fn detach_and_rejoin_selection_memory() {
    detach_and_rejoin(false).await;
}

#[tokio::test]
async fn detach_and_rejoin_selection_redb() {
    detach_and_rejoin(true).await;
}

async fn foreign_handle(redb: bool) {
    let (_db, mut e) = case(redb, "foreign").await;
    let (_other_db, mut other) = case(redb, "foreign-other").await;
    let p = restore(&mut e, "SelectedForeign", 1, 2).await;
    // Same PlayerId on another owner: a genuine but foreign handle.
    let foreign = restore(&mut other, "SelectedForeign", 1, 6).await;
    assert_eq!(foreign.player(), p.player());
    assert!(matches!(
        e.shard.connected_owned_player_view(&foreign),
        Err(PlayerOwnershipError::StaleHandle)
    ));
    assert!(matches!(
        e.shard.collector_lifecycle_view(&foreign),
        Err(PlayerOwnershipError::StaleHandle)
    ));
    assert!(!e
        .shard
        .connected_owned_player_view(&p)
        .unwrap()
        .matches_owned_player(&foreign));
    // Enqueue checks capacity only; the boundary apply refuses the foreign owner.
    let out = select(&mut e, &foreign, 7);
    assert!(matches!(refused(&out), PlayerOwnershipError::StaleHandle));
    assert!(no_resync(&out));
    assert_slot(&e, &p, 2, 2).await;
    assert_slot(&other, &foreign, 6, 6).await;
}

#[tokio::test]
async fn foreign_handle_produces_no_view_memory() {
    Box::pin(foreign_handle(false)).await;
}

#[tokio::test]
async fn foreign_handle_produces_no_view_redb() {
    Box::pin(foreign_handle(true)).await;
}

/// White-box guard: no writer stores an index above 8, but if one did, view
/// creation refuses rather than wrapping or clamping the typed slot.
#[tokio::test]
async fn out_of_range_owner_field_refuses_view_creation() {
    let (_db, mut e) = case(false, "guard").await;
    let p = restore(&mut e, "SelectedGuard", 1, 3).await;
    let set = |e: &mut Env, value: u8| {
        e.shard
            .players
            .get_mut(&p.player())
            .unwrap()
            .owned
            .as_mut()
            .unwrap()
            .selected = value;
    };
    set(&mut e, 9);
    assert!(matches!(
        e.shard.connected_owned_player_view(&p),
        Err(PlayerOwnershipError::Malformed("selected hotbar slot"))
    ));
    assert!(e
        .shard
        .collector_lifecycle_view(&p)
        .unwrap()
        .live()
        .is_none());
    set(&mut e, 3);
    assert_slot(&e, &p, 3, 3).await;
}
