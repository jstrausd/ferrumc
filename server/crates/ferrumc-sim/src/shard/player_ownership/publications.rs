//! Reserved publication admission and the actual post-counterconfirmation seal.

use super::{Candidate, EntityPublication, PendingKind, PreparedOwnedEntity, SimShard};
use crate::item_publication::{publication_charge, ItemPublicationState, PreparedItemPublication};
use crate::{
    ItemPublicationError, ItemPublicationGroup, ItemPublicationKind, ItemPublicationLimits,
    ItemPublicationOrigin, ItemPublicationStatus, ItemPublicationTicket, PlayerOwnershipError,
};

impl SimShard {
    pub(super) fn capture_item_collector(
        &self,
        handle: &crate::OwnedPlayerHandle,
    ) -> Result<crate::ItemCollectorBinding, PlayerOwnershipError> {
        Ok(crate::ItemCollectorBinding::admitted(
            handle,
            self.ownership_state()?.binding.generation(),
        ))
    }

    /// Enables retained publications on an empty actual item owner, once only.
    ///
    /// The outer queue is fallibly reserved before activation. Existing bounded
    /// item deepclones retain their infallible allocator boundary; encoded charge
    /// is not a decoded heap or RSS measurement. No Session/App consumer is enabled.
    ///
    /// # Errors
    /// Refuses a snapshot, mismatched scope, active/draining owner, nonempty item
    /// store/input/pending, repeated configuration or failed bounded reservation.
    pub fn initialize_item_publications(
        &mut self,
        limits: ItemPublicationLimits,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.publications.is_some() {
            return Err(ItemPublicationError::AlreadyInitialized.into());
        }
        if state.draining || state.stopped {
            return Err(PlayerOwnershipError::Draining);
        }
        if !state.input.is_empty()
            || !state.pending.is_empty()
            || self
                .entities
                .as_ref()
                .is_none_or(|e| !e.is_empty() || e.counter_entity_reserved())
        {
            return Err(ItemPublicationError::NotEmpty.into());
        }
        let publications = ItemPublicationState::new(limits)?;
        if let Some(state) = self.player_ownership.as_mut() {
            state.publications = Some(publications);
        }
        Ok(())
    }

    /// Returns descriptive accounting without exposing publication authority.
    ///
    /// # Errors
    /// Refuses missing actual ownership, snapshot/scope mismatch or inactive mode.
    pub fn item_publication_status(&self) -> Result<ItemPublicationStatus, PlayerOwnershipError> {
        Ok(ItemPublicationStatus(
            self.ownership_state()?
                .publications
                .as_ref()
                .ok_or(ItemPublicationError::NotInitialized)?
                .ledger,
        ))
    }

    /// Borrows the retained FIFO head without draining or cloning its body.
    /// Available during Drain so accepted output can become quiescent.
    ///
    /// # Errors
    /// Refuses missing ownership, snapshots, mismatched scope or inactive mode.
    pub fn peek_item_publication(
        &self,
    ) -> Result<Option<&ItemPublicationGroup>, PlayerOwnershipError> {
        Ok(self
            .ownership_state()?
            .publications
            .as_ref()
            .ok_or(ItemPublicationError::NotInitialized)?
            .peek())
    }

    /// Acknowledges exactly the actual-owner ticket for the current FIFO head.
    ///
    /// Driver must call this only after complete Session admission; a ticket is
    /// ownership/FIFO evidence, not proof of socket delivery. Partial/refused
    /// output must retain the head. Drain permits acknowledgements.
    ///
    /// # Errors
    /// Refuses a foreign/snapshot/scope owner, missing head or stale/reordered
    /// ticket without changing any retained data or charge.
    pub fn acknowledge_item_publication(
        &mut self,
        ticket: &ItemPublicationTicket,
    ) -> Result<(), PlayerOwnershipError> {
        self.ownership_state()?;
        let Some(state) = self.player_ownership.as_mut() else {
            return Err(PlayerOwnershipError::NotInitialized);
        };
        state
            .publications
            .as_mut()
            .ok_or(ItemPublicationError::NotInitialized)?
            .acknowledge(&state.owner, ticket)?;
        Ok(())
    }

    pub(super) fn prepare_candidate_publication(
        &self,
        candidate: &Candidate,
    ) -> Result<Option<PreparedItemPublication>, PlayerOwnershipError> {
        let Some(prepared) = &candidate.after_entity else {
            return Ok(None);
        };
        if matches!(candidate.kind, PendingKind::EntityAutosave) {
            return Ok(None);
        }
        let take = if candidate.animate && matches!(candidate.kind, PendingKind::Pickup) {
            let collector = candidate
                .collector
                .as_ref()
                .ok_or(PlayerOwnershipError::CompletionMismatch)?;
            Some(PreparedItemPublication::take(
                collector.clone(),
                candidate.original_count,
                candidate.moved,
            ))
        } else {
            None
        };
        let kind = match prepared {
            PreparedOwnedEntity::Spawn { .. } => ItemPublicationKind::Spawn,
            PreparedOwnedEntity::Replace { .. } => ItemPublicationKind::Changed,
            PreparedOwnedEntity::Remove { .. } => ItemPublicationKind::Removed,
        };
        let mut result = self.prepare_item_publication(prepared, kind)?;
        if let Some(result) = &mut result {
            result.take = take;
        }
        Ok(result)
    }

    pub(super) fn prepare_item_publication(
        &self,
        prepared: &PreparedOwnedEntity,
        kind: ItemPublicationKind,
    ) -> Result<Option<PreparedItemPublication>, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let Some(publications) = &state.publications else {
            return Ok(None);
        };
        let stack = match prepared {
            PreparedOwnedEntity::Spawn { item, .. } | PreparedOwnedEntity::Replace { item, .. } => {
                Some(item.stack())
            }
            PreparedOwnedEntity::Remove { .. } => None,
        };
        let size = stack
            .map(ferrumc_items::persisted_stack_size)
            .transpose()
            .map_err(ItemPublicationError::from)?;
        let charge = publication_charge(size)?;
        publications.ledger.check(charge)?;
        let removed = if let PreparedOwnedEntity::Remove { handle, .. } = prepared {
            Some(
                self.entities
                    .as_ref()
                    .ok_or(PlayerOwnershipError::NotInitialized)?
                    .publication_instance(handle, state.binding.generation())?,
            )
        } else {
            None
        };
        // The additional bounded clone occurs only after borrowed model/charge checks.
        Ok(Some(PreparedItemPublication {
            kind,
            stack: stack.cloned(),
            removed,
            take: None,
            charge,
        }))
    }

    pub(super) fn reserve_publication(&mut self, prepared: Option<&PreparedItemPublication>) {
        if let Some(prepared) = prepared {
            let Some(publications) = self
                .player_ownership
                .as_mut()
                .and_then(|s| s.publications.as_mut())
            else {
                panic!("staged publication owner disappeared");
            };
            publications.ledger.reserve(prepared.charge);
        }
    }

    pub(super) fn check_pending_publication(
        &self,
        pending: &super::PendingPlayerMutation,
    ) -> Result<(), PlayerOwnershipError> {
        if let Some(prepared) = &pending.publication {
            if self
                .ownership_state()?
                .publications
                .as_ref()
                .is_none_or(|p| !p.ledger.has_reservation(prepared.charge))
            {
                return Err(PlayerOwnershipError::CompletionMismatch);
            }
        }
        Ok(())
    }

    pub(super) fn cancel_publication(&mut self, prepared: Option<PreparedItemPublication>) {
        if let Some(prepared) = prepared {
            let Some(publications) = self
                .player_ownership
                .as_mut()
                .and_then(|s| s.publications.as_mut())
            else {
                panic!("reserved publication owner disappeared");
            };
            publications.ledger.cancel(prepared.charge);
        }
    }

    pub(super) fn publish_item_fact(
        &mut self,
        prepared: Option<PreparedItemPublication>,
        publication: &EntityPublication,
        origin: ItemPublicationOrigin,
    ) {
        let Some(mut prepared) = prepared else {
            return;
        };
        let Some(state) = self.player_ownership.as_ref() else {
            panic!("confirmed publication owner disappeared");
        };
        let generation = state.binding.generation();
        let owner = std::sync::Arc::downgrade(&state.owner);
        let (instance, scalars) = match publication {
            EntityPublication::Spawned(h) | EntityPublication::Changed(h) => {
                let Some(entities) = &self.entities else {
                    panic!("actual publication store disappeared");
                };
                let (instance, transform, properties) = entities.publication_scalars(h, generation);
                (instance, Some((transform, properties)))
            }
            EntityPublication::Removed(base) => {
                let Some(instance) = prepared.removed.take() else {
                    panic!("actual removal instance was not reserved");
                };
                debug_assert!(
                    instance.uuid() == base.uuid() && instance.wire_id() == base.wire_id()
                );
                (instance, None)
            }
        };
        let Some(publications) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.publications.as_mut())
        else {
            panic!("reserved publication queue disappeared");
        };
        publications.publish(prepared, owner, instance, scalars, origin);
    }
}
