//! Readonly detached status, Stop readiness and the bounded detached release.
//!
//! Release runs only between boundaries, never inside a counter or recovery
//! boundary. Every gate is a borrowed scan that precedes the single mutation,
//! so a refusal leaves the shard unchanged. One call visits at most the
//! player map entry, 32 queued inputs, 32 pendings (twice), 256 retained
//! publication groups and 384 lifecycle cells; it never allocates.

use ferrumc_core::PlayerId;
use ferrumc_storage::OwnershipTransferId;

use super::{InputKind, PlayerOwnershipState, SimShard};
use crate::player::OwnedPlayerInventory;
use crate::{
    DetachedOwnedPlayerStatus, DetachedReleaseRefusal, OwnedPlayerHandle, PlayerOwnershipError,
    ReleasedOwnedPlayer,
};

fn has_deferred(owned: &OwnedPlayerInventory) -> bool {
    owned.deferred_position.is_some()
        || owned.deferred_look.iter().any(Option::is_some)
        || owned.deferred_mode.is_some()
        || owned.deferred_selected.is_some()
}

/// Whether one queued input names this player by handle, read, completion keys
/// or the pending its retry/reconcile query addresses.
fn input_names_player(state: &PlayerOwnershipState, input: &InputKind, player: PlayerId) -> bool {
    match input {
        InputKind::Drop { player: p, .. }
        | InputKind::Pickup { player: p, .. }
        | InputKind::Autosave { player: p, .. }
        | InputKind::Click { player: p, .. }
        | InputKind::Move { player: p, .. }
        | InputKind::Mode { player: p, .. }
        | InputKind::Select { player: p, .. }
        | InputKind::Disconnect(p) => p.player == player,
        InputKind::RestorePlayer(read)
        | InputKind::Rejoin(read)
        | InputKind::Bootstrap { read, .. } => read.snapshot.expectation().key().id() == player,
        InputKind::Completion(completion) => completion
            .ticket
            .keys
            .player
            .is_some_and(|key| key.id() == player),
        InputKind::Retry(query) | InputKind::Reconcile(query) => state
            .pending
            .get(&query.id())
            .is_some_and(|p| p.player == Some(player)),
        InputKind::EntitySave { .. }
        | InputKind::RefillItemSaveIds { .. }
        | InputKind::RestoreItem { .. }
        | InputKind::SpawnItem { .. }
        | InputKind::Drain
        | InputKind::Stop => false,
    }
}

impl SimShard {
    /// Reads the detached status of exactly the original admission.
    ///
    /// Readonly and allocation-free. Foreign, stale or released handles refuse
    /// with `StaleHandle`, copied shards with `SnapshotOnly`, and a still
    /// connected admission with `Ineligible("connected player")`.
    pub fn detached_owned_player_status(
        &self,
        original: &OwnedPlayerHandle,
    ) -> Result<DetachedOwnedPlayerStatus, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let owned = self
            .actual_player(original)?
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if owned.connected {
            return Err(PlayerOwnershipError::Ineligible("connected player"));
        }
        let blocking = match owned.pending {
            Some(id) => Some((
                id,
                state
                    .pending
                    .get(&id)
                    .ok_or(PlayerOwnershipError::Malformed("player pending accounting"))?
                    .status,
            )),
            None => None,
        };
        Ok(DetachedOwnedPlayerStatus {
            saved_after_detach: owned.saved_after_detach,
            blocking,
            dirty: owned.dirty || has_deferred(owned),
        })
    }

    /// Reports whether a Stop enqueued as the only input is accepted by the
    /// next normal boundary. Uses the same predicate as the Stop apply itself;
    /// readonly and allocation-free. A stopped owner refuses with `Draining`.
    pub fn owned_stop_ready(&self) -> Result<bool, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.stopped {
            return Err(PlayerOwnershipError::Draining);
        }
        Ok(self.owned_state_quiescent(state))
    }

    /// Removes the detached native state of exactly the original admission.
    ///
    /// The caller supplies no query. All gates run before the only mutation:
    /// actual live owner (not `SnapshotOnly`, not stopped), actual admission,
    /// detached, lifecycle mode with this admission's cell acknowledged, no
    /// queued input, publication pin, item/chunk reservation, pending or other
    /// collector binding of the same player, clean state and a confirmed
    /// `saved_after_detach`. Success removes the `PlayerState` and its confirmed
    /// bytes from player accounting in the same step and returns a receipt
    /// without authority. There is no automatic eviction.
    pub fn release_detached_owned_player(
        &mut self,
        original: &OwnedPlayerHandle,
    ) -> Result<ReleasedOwnedPlayer, PlayerOwnershipError> {
        let freed = self.check_detached_release(original)?;
        let id = original.player;
        let remaining = self
            .ownership_state()?
            .player_bytes
            .checked_sub(freed)
            .ok_or(PlayerOwnershipError::Malformed("player accounting"))?;
        if self.players.remove(&id).is_none() {
            return Err(PlayerOwnershipError::StaleHandle);
        }
        if let Some(state) = self.player_ownership.as_mut() {
            state.player_bytes = remaining;
        }
        Ok(ReleasedOwnedPlayer {
            player: id,
            freed_player_bytes: freed,
        })
    }

    /// Records the genuine confirmed autosave of a detached admission; the
    /// latest confirmation wins. Called only from confirmed publication.
    pub(super) fn record_detached_save(&mut self, player: PlayerId, id: OwnershipTransferId) {
        if let Some(owned) = self.players.get_mut(&player).and_then(|p| p.owned.as_mut()) {
            if !owned.connected {
                owned.saved_after_detach = Some(id);
            }
        }
    }

    /// Runs every release gate on borrowed state and returns the confirmed bytes.
    fn check_detached_release(
        &self,
        original: &OwnedPlayerHandle,
    ) -> Result<usize, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.stopped {
            return Err(PlayerOwnershipError::Draining);
        }
        let owned = self
            .actual_player(original)?
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        let id = original.player;
        if owned.connected {
            return Err(DetachedReleaseRefusal::Connected.into());
        }
        let lifecycle = state
            .lifecycle
            .as_ref()
            .ok_or(DetachedReleaseRefusal::LifecycleNotInitialized)?;
        if state
            .input
            .iter()
            .any(|(input, _)| input_names_player(state, &input.0, id))
        {
            return Err(DetachedReleaseRefusal::QueuedInput.into());
        }
        let pinned = state.pending.values().any(|p| {
            p.publication
                .as_ref()
                .and_then(|publication| publication.take.as_ref())
                .is_some_and(|take| take.collector().player() == id)
        }) || state
            .publications
            .as_ref()
            .is_some_and(|p| p.retains_collector(id));
        if pinned {
            return Err(DetachedReleaseRefusal::PublicationPin.into());
        }
        if state
            .pending
            .values()
            .any(|p| p.player == Some(id) && p.after_entity.is_some())
        {
            return Err(DetachedReleaseRefusal::ItemReservation.into());
        }
        if owned.pending.is_some() || state.pending.values().any(|p| p.player == Some(id)) {
            return Err(DetachedReleaseRefusal::Pending.into());
        }
        if !owned.lifecycle_acked {
            return Err(DetachedReleaseRefusal::LifecycleNotAcknowledged.into());
        }
        if lifecycle.binds_player(id) {
            return Err(DetachedReleaseRefusal::CollectorBinding.into());
        }
        if owned.dirty || has_deferred(owned) {
            return Err(DetachedReleaseRefusal::Unsaved.into());
        }
        if owned.saved_after_detach.is_none() {
            return Err(DetachedReleaseRefusal::NotSaved.into());
        }
        Ok(owned.confirmed.data().len())
    }
}
