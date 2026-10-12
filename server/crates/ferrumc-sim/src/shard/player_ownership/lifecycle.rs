//! Actual native producer APIs and admission-cell installation.

use super::{PendingKind, SimShard};
use crate::collector_lifecycle::{CollectorLifecycleState, LifecycleCellKey};
use crate::{
    CollectorLifecycleError, CollectorLifecycleLimits, CollectorLifecycleStatus,
    CollectorLifecycleTicket, CollectorLifecycleView, ConnectedOwnedPlayerView, HotbarSlot,
    OwnedPlayerHandle, PlayerLocalGeneration, PlayerOwnershipError,
};

impl SimShard {
    /// Enables bounded original-admission cells on the empty actual owner.
    /// Publication mode must already be selected; its actual group cap sets the
    /// pin ceiling. The complete scalar layout is fallibly reserved once.
    ///
    /// # Errors
    /// Refuses snapshots, scope mismatch, repeated initialization, nonempty or
    /// draining/stopped owners, invalid byte caps or bounded allocation failure.
    pub fn initialize_collector_lifecycle(
        &mut self,
        limits: CollectorLifecycleLimits,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.lifecycle.is_some() {
            return Err(CollectorLifecycleError::AlreadyInitialized.into());
        }
        if state.draining || state.stopped {
            return Err(PlayerOwnershipError::Draining);
        }
        let publications = state
            .publications
            .as_ref()
            .ok_or(crate::ItemPublicationError::NotInitialized)?;
        if !self.players.is_empty()
            || !state.input.is_empty()
            || !state.pending.is_empty()
            || !publications.ledger.is_empty()
            || self
                .entities
                .as_ref()
                .is_none_or(|e| !e.is_empty() || e.counter_entity_reserved())
        {
            return Err(CollectorLifecycleError::NotEmpty.into());
        }
        let lifecycle = CollectorLifecycleState::new(
            state.limits.players,
            publications.ledger.group_limit(),
            limits,
        )?;
        if let Some(state) = &mut self.player_ownership {
            state.lifecycle = Some(lifecycle);
        }
        Ok(())
    }

    /// Borrows actual native connected state with exact private owner/admission checks.
    /// Drain allows existing output consumers; stopped, detached, foreign, stale
    /// and `SnapshotOnly` owners cannot produce this view.
    pub fn connected_owned_player_view<'a>(
        &'a self,
        handle: &OwnedPlayerHandle,
    ) -> Result<ConnectedOwnedPlayerView<'a>, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.stopped {
            return Err(PlayerOwnershipError::Draining);
        }
        let player = self.actual_player(handle)?;
        let owned = match &player.owned {
            Some(owned) if owned.connected => owned,
            _ => return Err(PlayerOwnershipError::Ineligible("detached player")),
        };
        // Every writer stores only `0..=8` (FCPS3 encode/decode, select
        // validation), so this refusal is unreachable; it keeps the typed
        // accessor infallible without trusting that invariant silently.
        let selected = HotbarSlot::new(owned.selected)
            .ok_or(PlayerOwnershipError::Malformed("selected hotbar slot"))?;
        Ok(ConnectedOwnedPlayerView {
            player,
            state,
            id: handle.player,
            selected,
        })
    }

    /// Borrows the exact historical admission cell, even after genuine rejoin.
    /// Live state is present only for the same still-connected current admission.
    pub fn collector_lifecycle_view<'a>(
        &'a self,
        original: &OwnedPlayerHandle,
    ) -> Result<CollectorLifecycleView<'a>, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if !crate::player::same_owner(&original.owner, &state.owner) {
            return Err(PlayerOwnershipError::StaleHandle);
        }
        let cell = state
            .lifecycle
            .as_ref()
            .ok_or(CollectorLifecycleError::NotInitialized)?
            .lookup(original)?;
        Ok(CollectorLifecycleView {
            cell,
            live: self.connected_owned_player_view(original).ok(),
        })
    }

    /// Returns descriptive bounded lifecycle scalar accounting.
    pub fn collector_lifecycle_status(
        &self,
    ) -> Result<CollectorLifecycleStatus, PlayerOwnershipError> {
        Ok(self
            .ownership_state()?
            .lifecycle
            .as_ref()
            .ok_or(CollectorLifecycleError::NotInitialized)?
            .status())
    }

    /// Releases exactly the ticket's detached source-pins-zero admission cell.
    /// Driver must first prove actual Session retirement and zero consumer refs;
    /// this Sim ticket contains no network receipt or Session authority.
    /// Drain permits acknowledgement. Foreign/stale/repeated tickets preserve credits.
    /// A successful acknowledgement also marks exactly that admission's state as
    /// lifecycle-acknowledged for the detached release gate.
    pub fn acknowledge_collector_lifecycle(
        &mut self,
        ticket: &CollectorLifecycleTicket,
    ) -> Result<(), PlayerOwnershipError> {
        self.ownership_state()?;
        let state = self
            .player_ownership
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        state
            .lifecycle
            .as_mut()
            .ok_or(CollectorLifecycleError::NotInitialized)?
            .acknowledge(ticket)?;
        // The validated cell belonged to this owner; mark exactly that admission,
        // never a later rejoin of the same player.
        if let Some(owned) = self
            .players
            .get_mut(&ticket.collector.player())
            .and_then(|p| p.owned.as_mut())
        {
            if owned.admission == ticket.collector.admission() {
                owned.lifecycle_acked = true;
            }
        }
        Ok(())
    }

    pub(super) fn staged_lifecycle_cell(
        &self,
        generation: PlayerLocalGeneration,
    ) -> Result<Option<LifecycleCellKey>, PlayerOwnershipError> {
        self.ownership_state()?
            .lifecycle
            .as_ref()
            .map(|l| l.staged_cell(generation))
            .transpose()
            .map_err(Into::into)
    }

    pub(super) fn reserve_lifecycle_cell(
        &mut self,
        key: Option<LifecycleCellKey>,
        player: ferrumc_core::PlayerId,
    ) {
        if let Some(key) = key {
            let Some(state) = &mut self.player_ownership else {
                panic!("staged lifecycle owner disappeared");
            };
            let generation = state.binding.generation();
            let Some(lifecycle) = &mut state.lifecycle else {
                panic!("staged lifecycle mode disappeared");
            };
            lifecycle.reserve_cell(key, player, generation);
        }
    }

    pub(super) fn seal_lifecycle_cell(
        &mut self,
        key: Option<LifecycleCellKey>,
        handle: &OwnedPlayerHandle,
    ) {
        if let Some(key) = key {
            let Some(lifecycle) = self
                .player_ownership
                .as_mut()
                .and_then(|s| s.lifecycle.as_mut())
            else {
                panic!("reserved lifecycle mode disappeared");
            };
            lifecycle.seal(key, handle);
        }
    }

    pub(super) fn apply_native_disconnect(
        &mut self,
        handle: &OwnedPlayerHandle,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if let Some(lifecycle) = &state.lifecycle {
            let cell = lifecycle.lookup(handle)?;
            // Repeated historical Disconnect is idempotent and never touches a rejoin.
            if cell.detach.is_some() {
                return Ok(());
            }
        }
        self.actual_player(handle)?;
        if let Some(owned) = self
            .players
            .get_mut(&handle.player)
            .and_then(|p| p.owned.as_mut())
        {
            owned.connected = false;
        }
        if let Some(lifecycle) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.lifecycle.as_mut())
        {
            lifecycle.detach(handle)?;
        }
        Ok(())
    }

    pub(super) fn preflight_take_pin(
        &self,
        handle: &OwnedPlayerHandle,
        animate: bool,
    ) -> Result<(), PlayerOwnershipError> {
        if animate {
            if let Some(lifecycle) = &self.ownership_state()?.lifecycle {
                lifecycle.staged_pin(&self.capture_item_collector(handle)?)?;
            }
        }
        Ok(())
    }

    pub(super) fn release_bootstrap_lifecycle(&mut self, pending: &super::PendingPlayerMutation) {
        if matches!(pending.kind, PendingKind::Bootstrap) {
            if let Some(key) = pending.lifecycle_cell {
                let Some(lifecycle) = self
                    .player_ownership
                    .as_mut()
                    .and_then(|s| s.lifecycle.as_mut())
                else {
                    panic!("reserved bootstrap lifecycle disappeared");
                };
                lifecycle.cancel_cell(key);
            }
        }
    }
}
