//! Future-boundary scalar planning and actual native item durability consumer.

#[cfg(test)]
mod tests;

use ferrumc_core::{EntityUuid, Tick};
use ferrumc_storage::OwnershipTransferId;
use std::mem::size_of;

use super::{
    borrowed_pickup_shape, entity_equal, persisted_stack_size, record_equal, same_owner,
    standing_drop_parts, Arc, Candidate, CompletionEvidence, DropSelection, EntityPublication,
    GameMode, InputKind, InventoryPlan, OwnedPairRead, OwnedPlayerHandle, OwnedPlayerInventory,
    OwnershipError, OwnershipIntent, OwnershipPendingStatus, OwnershipStamp, PendingKind,
    PendingPlayerMutation, PlayerLocalGeneration, PlayerOwnershipCompletion, PlayerOwnershipError,
    PlayerOwnershipTickOutcome, PlayerRecord, PreparedOwnedEntity, SimShard, Vec3,
};
use crate::item_counters::{CounterBoundaryEntry, DeferredOwnedCounters, OwnedItemCounterState};
use crate::{ItemDespawnDecision, ItemEntityCounters, OwnedItemCounterLimits};

#[derive(Clone, Copy)]
struct CompletionOverlay<'a> {
    pending: &'a PendingPlayerMutation,
    completion: &'a PlayerOwnershipCompletion,
    confirmed: bool,
    generation: PlayerLocalGeneration,
    deferred_start: PlayerLocalGeneration,
    releases_unreserved: bool,
}
#[derive(Clone, Copy)]
struct ShadowPlayer<'a> {
    id: ferrumc_core::PlayerId,
    admission: PlayerLocalGeneration,
    mutation: PlayerLocalGeneration,
    position: Vec3,
    look: [f32; 2],
    mode: GameMode,
    selected: u8,
    slots: &'a [ferrumc_items::ItemStack; crate::NATIVE_PLAYER_SLOT_COUNT],
    carried: &'a ferrumc_items::ItemStack,
    record: &'a PlayerRecord,
    stamp: OwnershipStamp,
    pending: Option<OwnershipTransferId>,
    connected: bool,
    dirty: bool,
    invalid: bool,
}

struct ShadowResources {
    publications: Option<crate::item_publication::PublicationLedger>,
    pending_count: usize,
    pending_bytes: usize,
    player_bytes: usize,
    pending_player_bytes: usize,
    generation: PlayerLocalGeneration,
    new_ids: [Option<OwnershipTransferId>; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
    new_player_ids: [Option<ferrumc_core::PlayerId>; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
    dispatched: [Option<OwnershipTransferId>; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
    dispatched_count: usize,
    restored: [Option<(ferrumc_core::PlayerId, usize)>; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
    restored_count: usize,
    player_count: usize,
    bootstraps: usize,
    bootstrap_bytes: usize,
    entity_capacity: usize,
    admitted: usize,
}

impl SimShard {
    /// Enables resident counters on an empty concrete owner, using its last completed tick.
    ///
    /// This is Vanilla-without-plugins counter policy; movement, activation and
    /// Java despawn events are separate consumers. No wall-clock catch-up runs.
    pub fn initialize_owned_item_counters(
        &mut self,
        limits: OwnedItemCounterLimits,
        last_completed: Tick,
    ) -> Result<crate::OwnedItemCounterHandle, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.item_counters.is_some() || !state.pending.is_empty() || !state.input.is_empty() {
            return Err(PlayerOwnershipError::AlreadyInitialized);
        }
        self.entities
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .initialize_counters(limits.confirmed_bytes)?;
        if let Some(s) = &mut self.player_ownership {
            s.item_counters = Some(OwnedItemCounterState::new(limits, last_completed));
        }
        Ok(crate::OwnedItemCounterHandle {
            owner: Arc::downgrade(&self.ownership_state()?.owner),
        })
    }

    pub(super) fn refill_counter_save_ids(
        &mut self,
        owner: &crate::OwnedItemCounterHandle,
        ids: &[OwnershipTransferId],
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if !same_owner(&owner.owner, &state.owner) {
            return Err(PlayerOwnershipError::StaleHandle);
        }
        let counter = state
            .item_counters
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        if ids.len()
            > counter
                .limits
                .save_ids
                .saturating_sub(counter.save_ids.len())
        {
            return Err(PlayerOwnershipError::Capacity("save ID pool"));
        }
        for (index, id) in ids.iter().enumerate() {
            if ids[..index].contains(id)
                || counter.save_ids.contains(id)
                || state.pending.contains_key(id)
            {
                return Err(PlayerOwnershipError::Reserved);
            }
        }
        if let Some(c) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.item_counters.as_mut())
        {
            c.save_ids
                .try_reserve_exact(ids.len())
                .map_err(|_| PlayerOwnershipError::AllocationFailed)?;
            c.save_ids.extend_from_slice(ids);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn current_counter_tick(&self) -> Tick {
        self.player_ownership
            .as_ref()
            .and_then(|s| s.item_counters.as_ref())
            .map_or(Tick::ZERO, |s| s.tick)
    }

    pub(super) fn publication_counter_tick(&self) -> Tick {
        self.player_ownership
            .as_ref()
            .and_then(|s| s.item_counters.as_ref())
            .map_or(Tick::ZERO, |c| c.processing_tick.unwrap_or(c.tick))
    }

    /// Runs exactly the next checked logical boundary, after whole-boundary preflight.
    /// Every eligible resident item steps once; only save selection is budgeted.
    pub fn run_tick_with_player_ownership_at(
        &mut self,
        tick: Tick,
    ) -> Result<PlayerOwnershipTickOutcome, PlayerOwnershipError> {
        if self.ownership_state()?.stopped {
            return Err(PlayerOwnershipError::Draining);
        }
        let state = self
            .ownership_state()?
            .item_counters
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let next = state
            .tick
            .next()
            .ok_or(PlayerOwnershipError::CounterTickOverflow)?;
        if tick != next {
            return Err(PlayerOwnershipError::CounterTickMismatch);
        }
        self.ownership_state()?.generation.next()?;
        let mut entries = if let Some(c) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.item_counters.as_mut())
        {
            std::mem::take(&mut c.scratch)
        } else {
            return Err(PlayerOwnershipError::NotInitialized);
        };
        entries.clear();
        let planned = self.plan_counter_boundary(tick, &mut entries);
        if let Err(error) = planned {
            self.restore_counter_scratch(entries);
            return Err(error);
        }
        let extra = self
            .ownership_state()?
            .input
            .len()
            .checked_add(1)
            .ok_or(PlayerOwnershipError::Capacity("future item slots"))?;
        if let Err(error) = self
            .entities
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .reserve_counter_boundary_slots(extra)
        {
            self.restore_counter_scratch(entries);
            return Err(error);
        }
        if let Some(c) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.item_counters.as_mut())
        {
            c.processing_tick = Some(tick);
        }
        let mut outcome = self.run_player_ownership_boundary()?;
        outcome.counter_steps = entries
            .iter()
            .filter(|e| {
                e.present
                    && e.resident
                    && (e.deferred.is_some() || !e.due || e.first_due == Some(tick))
            })
            .count();
        self.apply_counter_boundary(tick, &entries);
        if let Some(c) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.item_counters.as_mut())
        {
            c.tick = tick;
            c.processing_tick = None;
        }
        self.restore_counter_scratch(entries);
        self.select_counter_save(&mut outcome);
        Ok(outcome)
    }

    /// Clarifies genuine outstanding work without claiming a counter/gameplay tick.
    /// Gameplay inputs retain their original FIFO position and byte charges.
    pub fn recover_player_ownership_completions(
        &mut self,
    ) -> Result<PlayerOwnershipTickOutcome, PlayerOwnershipError> {
        self.ownership_state()?;
        let mut outcome = PlayerOwnershipTickOutcome::default();
        // Check future publication/defer generations before touching any genuine
        // completion. Repeated IDs cannot claim the same retained publication twice.
        let mut generation = self.ownership_state()?.generation;
        let mut released = [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS];
        let mut released_count = 0;
        for (input, _) in &self.ownership_state()?.input {
            if let InputKind::Completion(c) = &input.0 {
                if released[..released_count].contains(&Some(c.query().id())) {
                    continue;
                }
                if let Ok(p) = self.validate_completion(c) {
                    if let Some(confirmed) = completion_release(self, p, c) {
                        self.preflight_counter_completion(p.ticket.query.id(), confirmed)?;
                        if confirmed {
                            generation = generation.next()?;
                        }
                        for owned in self.players.values().filter_map(|p| p.owned.as_ref()) {
                            if has_deferred_player(owned)
                                && (owned.pending == Some(c.query().id())
                                    || (owned.pending.is_none() && released_count == 0))
                            {
                                generation = generation.next()?;
                            }
                        }
                        released[released_count] = Some(c.query().id());
                        released_count += 1;
                    }
                }
            }
        }
        let queued = if let Some(s) = &mut self.player_ownership {
            std::mem::take(&mut s.input)
        } else {
            return Err(PlayerOwnershipError::NotInitialized);
        };
        for (input, charge) in queued {
            if matches!(
                input.0,
                InputKind::Completion(_)
                    | InputKind::Reconcile(_)
                    | InputKind::Retry(_)
                    | InputKind::Drain
                    | InputKind::RefillItemSaveIds { .. }
            ) {
                if let Some(s) = &mut self.player_ownership {
                    s.input_bytes -= charge;
                }
                self.apply_owned_input(input, &mut outcome);
            } else if let Some(s) = &mut self.player_ownership {
                s.input.push_back((input, charge));
            }
        }
        self.select_counter_save(&mut outcome);
        Ok(outcome)
    }

    fn restore_counter_scratch(&mut self, mut entries: Vec<CounterBoundaryEntry>) {
        entries.clear();
        if let Some(c) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.item_counters.as_mut())
        {
            c.scratch = entries;
        }
    }

    pub(super) fn prepare_counter_candidate(
        &self,
        candidate: &Candidate,
    ) -> Result<(Option<u64>, Option<DeferredOwnedCounters>), PlayerOwnershipError> {
        if self.ownership_state()?.item_counters.is_none() {
            return Ok((None, None));
        }
        let Some(prepared) = &candidate.after_entity else {
            return Ok((None, None));
        };
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        match prepared {
            PreparedOwnedEntity::Spawn { .. } => Ok((Some(1), None)),
            PreparedOwnedEntity::Replace { handle, .. }
            | PreparedOwnedEntity::Remove { handle, .. } => {
                let current = entities.item_state(handle)?.properties().counters();
                let durable = entities.item_durability(handle)?;
                let generation = if matches!(candidate.kind, PendingKind::Pickup)
                    && candidate.after_entity_record.is_some()
                {
                    durable
                        .generation
                        .checked_add(1)
                        .ok_or(PlayerOwnershipError::GenerationExhausted)?
                } else {
                    durable.generation
                };
                let after = match prepared {
                    PreparedOwnedEntity::Replace { item, .. } => item.properties().counters(),
                    _ => current,
                };
                Ok((
                    Some(generation),
                    Some(DeferredOwnedCounters {
                        original_counters: current,
                        original_generation: durable.generation,
                        counters: after,
                        generation,
                        claims: 0,
                        first: None,
                        last: None,
                        due: durable.due,
                        first_due: durable.first_due,
                    }),
                ))
            }
        }
    }

    pub(super) fn preflight_counter_completion(
        &self,
        id: OwnershipTransferId,
        confirmed: bool,
    ) -> Result<(), PlayerOwnershipError> {
        if self.ownership_state()?.item_counters.is_none() {
            return Ok(());
        }
        let p = self
            .ownership_state()?
            .pending
            .get(&id)
            .ok_or(PlayerOwnershipError::CompletionMismatch)?;
        let e = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        if confirmed {
            let old = p
                .before_entity
                .as_ref()
                .map_or(0, |r| r.record().data().len());
            let new = p
                .after_entity_record
                .as_ref()
                .map_or(0, |r| r.record().data().len());
            e.check_confirmed_replacement(old, new)?;
        }
        // All projected generations were checked at each admitted claim. Never
        // increment on acknowledgement: confirmation captures a precise generation.
        if p.item_generation == Some(0)
            || p.counters
                .is_some_and(|d| d.generation == 0 || d.original_generation == 0)
        {
            return Err(PlayerOwnershipError::GenerationExhausted);
        }
        Ok(())
    }

    pub(super) fn publish_counter_confirmation(
        &mut self,
        publication: &EntityPublication,
        pending: &mut PendingPlayerMutation,
    ) {
        let tick = self.publication_counter_tick();
        if let Some(e) = &mut self.entities {
            if let EntityPublication::Spawned(h) | EntityPublication::Changed(h) = publication {
                let priority = e.item_durability(h).is_ok_and(|d| d.priority);
                if let (Some(record), Some(generation)) =
                    (pending.after_entity_record.take(), pending.item_generation)
                {
                    e.install_confirmed(h, record, generation, tick);
                }
                if let Some(d) = pending.counters {
                    let Ok(base) = e.get(h) else {
                        panic!("confirmed item publication disappeared");
                    };
                    e.apply_counter_publication(base.uuid(), d);
                }
                e.restore_counter_flush(h, priority);
            }
        }
    }

    pub(super) fn release_counter_projection(&mut self, pending: &PendingPlayerMutation) {
        if let (Some(mut d), Some(expected), Some(e)) = (
            pending.counters,
            &pending.entity_expected,
            &mut self.entities,
        ) {
            d.counters = d.original_counters;
            d.generation = d.original_generation;
            e.apply_counter_publication(expected.key().uuid(), d);
        }
    }

    pub(super) fn quarantine_counter_conflict(&mut self, id: OwnershipTransferId) {
        if let Some(d) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.pending.get_mut(&id))
            .and_then(|p| p.counters.as_mut())
        {
            // Conflict supplies no new publication authority. Keep the visible
            // original body frozen and retain only its bounded private projection.
            d.counters = d.original_counters;
            d.generation = d.original_generation;
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one readonly completion prefix preserves genuine publication order"
    )]
    fn plan_counter_boundary(
        &self,
        tick: Tick,
        entries: &mut Vec<CounterBoundaryEntry>,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let config = state
            .item_counters
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .limits;
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let needed = entities
            .len()
            .checked_add(state.input.len())
            .and_then(|n| n.checked_add(1))
            .ok_or(PlayerOwnershipError::Capacity("counter plan count"))?;
        let metadata =
            size_of::<[Option<OwnershipTransferId>; crate::MAX_PLAYER_OWNERSHIP_INPUTS]>()
                + size_of::<[Option<CompletionOverlay<'_>>; crate::MAX_PLAYER_OWNERSHIP_INPUTS]>()
                + size_of::<[Option<ShadowPlayer<'_>>; crate::MAX_PLAYER_OWNERSHIP_INPUTS]>()
                + size_of::<ShadowResources>()
                + size_of::<[Option<OwnershipTransferId>; crate::MAX_PLAYER_OWNERSHIP_INPUTS]>();
        let bytes = needed
            .checked_mul(size_of::<CounterBoundaryEntry>())
            .and_then(|n| n.checked_add(metadata))
            .ok_or(PlayerOwnershipError::Capacity("counter plan bytes"))?;
        if needed > config.plan_entries || bytes > config.plan_bytes {
            return Err(PlayerOwnershipError::Capacity("counter plan"));
        }
        if entries.capacity() < needed {
            entries
                .try_reserve_exact(needed)
                .map_err(|_| PlayerOwnershipError::AllocationFailed)?;
        }
        entities.append_counter_entries(&self.chunks, entries);
        // One existing entityoperation; unrelated slots retain scalar authority.
        for p in state.pending.values() {
            if let (Some(d), Some(expected)) = (p.counters, &p.entity_expected) {
                if let Some(entry) = entries.iter_mut().find(|e| e.uuid == expected.key().uuid()) {
                    bind_projection(entry, p.ticket.query.id(), d);
                }
            }
        }
        let mut consumed = [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS];
        let mut consumed_len = 0;
        let mut overlays = [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS];
        let mut future_generation = state.generation;
        let mut handled = [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS];
        let mut handled_len = 0;
        let mut future_retired = 0;
        let mut publication_ledger = state.publications.as_ref().map(|p| p.ledger);
        for (input, _) in &state.input {
            let InputKind::Completion(c) = &input.0 else {
                continue;
            };
            if handled[..handled_len].contains(&Some(c.query().id())) {
                continue;
            }
            let Ok(pending) = self.validate_completion(c) else {
                continue;
            };
            handled[handled_len] = Some(c.query().id());
            handled_len += 1;
            if matches!(
                c.evidence,
                CompletionEvidence::Refused(OwnershipError::CasConflict { .. })
            ) {
                if let (Some(d), Some(expected)) = (pending.counters, &pending.entity_expected) {
                    if let Some(entry) =
                        entries.iter_mut().find(|e| e.uuid == expected.key().uuid())
                    {
                        entry.counters = d.original_counters;
                        entry.generation = d.original_generation;
                    }
                }
                continue;
            }
            let Some(confirmed) = completion_release(self, pending, c) else {
                continue;
            };
            self.preflight_counter_completion(c.query().id(), confirmed)?;
            self.preflight_deferred_for(Some(c.query().id()), u64::from(confirmed))?;
            self.check_pending_publication(pending)?;
            if let (Some(ledger), Some(publication)) =
                (&mut publication_ledger, &pending.publication)
            {
                if confirmed {
                    let _ = ledger.publish(publication.charge);
                } else {
                    ledger.cancel(publication.charge);
                }
            }
            if confirmed {
                future_generation = future_generation.next()?;
            }
            let completion_generation = future_generation;
            let releases_unreserved = consumed_len == 0;
            for player in self.players.values().filter_map(|p| p.owned.as_ref()) {
                let freeing = player.pending == Some(c.query().id())
                    || (consumed_len == 0 && player.pending.is_none());
                if freeing
                    && (player.deferred_position.is_some()
                        || player.deferred_look.iter().any(Option::is_some)
                        || player.deferred_mode.is_some()
                        || player.deferred_selected.is_some())
                {
                    future_generation = future_generation.next()?;
                }
            }
            overlays[consumed_len] = Some(CompletionOverlay {
                pending,
                completion: c,
                confirmed,
                generation: completion_generation,
                deferred_start: completion_generation,
                releases_unreserved,
            });
            consumed[consumed_len] = Some(c.query().id());
            consumed_len += 1;
            if let Some(prepared) = &pending.after_entity {
                let uuid = prepared_uuid(entities, prepared)?;
                let old = entries.iter().position(|e| e.uuid == uuid);
                if confirmed {
                    match prepared {
                        PreparedOwnedEntity::Remove { .. } => {
                            if let Some(i) = old {
                                future_retired +=
                                    usize::from(entities.counter_slot_retires(entries[i].slot));
                                entries[i].present = false;
                                entries[i].deferred = None;
                            }
                        }
                        PreparedOwnedEntity::Spawn { base, item, .. } => {
                            entries.push(CounterBoundaryEntry {
                                original_counters: item.properties().counters(),
                                original_generation: 1,
                                slot: usize::MAX,
                                uuid,
                                counters: item.properties().counters(),
                                generation: pending.item_generation.unwrap_or(1),
                                deferred: None,
                                claims: 0,
                                due: false,
                                resident: self.chunks.is_loaded(base.transform.chunk()),
                                present: true,
                                saved: pending.item_generation.unwrap_or(1),
                                wire: base.wire_id,
                                body_len: item.native_body_len(),
                                confirmed_len: pending
                                    .after_entity_record
                                    .as_ref()
                                    .map_or(0, |r| r.record().data().len()),
                                first_due: None,
                            });
                        }
                        PreparedOwnedEntity::Replace { item, .. } => {
                            let i = old.ok_or(PlayerOwnershipError::CompletionMismatch)?;
                            let entry = &mut entries[i];
                            entry.counters = item.properties().counters();
                            entry.generation = pending
                                .item_generation
                                .ok_or(PlayerOwnershipError::CompletionMismatch)?;
                            entry.saved = entry.generation;

                            entry.body_len = item.native_body_len();
                            entry.confirmed_len = pending
                                .after_entity_record
                                .as_ref()
                                .map_or(0, |r| r.record().data().len());
                            if let Some(d) = pending.counters {
                                entry.counters = d.counters;
                                entry.generation = d.generation;
                                entry.due = d.due;
                                entry.first_due = d.first_due;
                            }
                            entry.deferred = None;
                            entry.claims = 0;
                        }
                    }
                } else if let (Some(i), Some(d)) = (old, pending.counters) {
                    let entry = &mut entries[i];
                    entry.counters = d.original_counters;
                    entry.generation = d.original_generation;
                    entry.due = d.due;
                    entry.first_due = d.first_due;
                    entry.deferred = None;
                    entry.claims = 0;
                }
            }
        }
        let stopped = self.plan_counter_actions(
            entries,
            &consumed[..consumed_len],
            &overlays[..consumed_len],
            future_generation,
            future_retired,
            publication_ledger,
        )?;
        if stopped {
            for entry in entries.iter_mut() {
                entry.resident = false;
            }
        }
        for entry in entries.iter_mut().filter(|e| e.present) {
            if !entry.resident {
                continue;
            }
            step_boundary_entry(entry, tick, config)?;
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "explicit scalar FIFO dispatch mirrors the concrete admission consumer without body clones"
    )]
    fn plan_counter_actions<'a>(
        &'a self,
        entries: &mut Vec<CounterBoundaryEntry>,
        consumed: &[Option<OwnershipTransferId>],
        overlays: &[Option<CompletionOverlay<'a>>],
        future_generation: PlayerLocalGeneration,
        future_retired: usize,
        publication_ledger: Option<crate::item_publication::PublicationLedger>,
    ) -> Result<bool, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let mut reserved = state
            .pending
            .values()
            .any(|p| p.after_entity.is_some() && !consumed.contains(&Some(p.ticket.query.id())));
        let mut draining = state.draining;
        let mut live_bytes = entries
            .iter()
            .filter(|e| e.present)
            .map(|e| e.body_len)
            .sum::<usize>();
        let mut confirmed_bytes = entries
            .iter()
            .filter(|e| e.present)
            .map(|e| e.confirmed_len)
            .sum::<usize>();
        let mut resources = ShadowResources {
            publications: publication_ledger,
            pending_count: state
                .pending
                .values()
                .filter(|p| !consumed.contains(&Some(p.ticket.query.id())))
                .count(),
            pending_bytes: state
                .pending
                .values()
                .filter(|p| !consumed.contains(&Some(p.ticket.query.id())))
                .map(|p| p.charge)
                .sum(),
            player_bytes: state.player_bytes,
            pending_player_bytes: state
                .pending
                .values()
                .filter(|p| !consumed.contains(&Some(p.ticket.query.id())))
                .filter_map(|p| p.after_player_record.as_ref())
                .map(|r| r.data().len())
                .sum(),
            generation: future_generation,
            new_ids: [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
            new_player_ids: [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
            dispatched: [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
            dispatched_count: 0,
            restored: [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS],
            restored_count: 0,
            player_count: self.players.len(),
            bootstraps: state
                .pending
                .values()
                .filter(|p| {
                    matches!(p.kind, PendingKind::Bootstrap)
                        && !consumed.contains(&Some(p.ticket.query.id()))
                })
                .count(),
            bootstrap_bytes: state
                .pending
                .values()
                .filter(|p| {
                    matches!(p.kind, PendingKind::Bootstrap)
                        && !consumed.contains(&Some(p.ticket.query.id()))
                })
                .filter_map(|p| p.after_player_record.as_ref())
                .map(|r| r.data().len())
                .sum(),
            entity_capacity: entities.counter_effective_capacity() - future_retired,
            admitted: 0,
        };
        for overlay in overlays.iter().flatten().filter(|o| o.confirmed) {
            if matches!(overlay.pending.kind, PendingKind::Bootstrap) {
                resources.player_count += 1;
            }
            if let Some(after) = &overlay.pending.after_player_record {
                resources.player_bytes = resources.player_bytes
                    - overlay
                        .pending
                        .before_player
                        .as_ref()
                        .map_or(0, |r| r.data().len())
                    + after.data().len();
            }
        }
        let mut players = [None; crate::MAX_PLAYER_OWNERSHIP_INPUTS];
        let mut player_count = 0;
        let actions = state
            .input
            .iter()
            .filter(|(input, _)| !matches!(input.0, InputKind::Completion(_)))
            .count();
        let mut remaining = actions;
        let mut stopped = false;
        for (input, _) in &state.input {
            if matches!(input.0, InputKind::Completion(_)) {
                continue;
            }
            remaining -= 1;
            if let InputKind::Drain = input.0 {
                draining = true;
                continue;
            }
            if draining
                && !matches!(
                    input.0,
                    InputKind::Autosave { .. }
                        | InputKind::EntitySave { .. }
                        | InputKind::RefillItemSaveIds { .. }
                        | InputKind::Disconnect(_)
                        | InputKind::Stop
                        | InputKind::Retry(_)
                        | InputKind::Reconcile(_)
                )
            {
                continue;
            }
            let player_handle = match &input.0 {
                InputKind::Pickup { player, .. }
                | InputKind::Drop { player, .. }
                | InputKind::Autosave { player, .. }
                | InputKind::Click { player, .. }
                | InputKind::Move { player, .. }
                | InputKind::Mode { player, .. }
                | InputKind::Select { player, .. }
                | InputKind::Disconnect(player) => Some(player),
                _ => None,
            };
            let player_index = if let Some(handle) = player_handle {
                if !same_owner(&handle.owner, &state.owner) {
                    continue;
                }
                if resources.restored[..resources.restored_count]
                    .iter()
                    .flatten()
                    .any(|(id, _)| *id == handle.player)
                {
                    continue;
                }
                let index =
                    players[..player_count]
                        .iter()
                        .position(|p: &Option<ShadowPlayer<'_>>| {
                            p.is_some_and(|p| p.id == handle.player)
                        });
                let index = if let Some(index) = index {
                    index
                } else {
                    let Ok(player) = self.shadow_counter_player(handle, overlays) else {
                        continue;
                    };
                    let index = player_count;
                    players[index] = Some(player);
                    player_count += 1;
                    index
                };
                if players[index].is_none_or(|p| p.invalid || p.admission != handle.admission) {
                    continue;
                }
                Some(index)
            } else {
                None
            };
            let player = player_index.and_then(|i| players[i]);
            match &input.0 {
                InputKind::Stop => {
                    let players_clean =
                        self.players
                            .iter()
                            .filter(|(_, p)| p.owned.is_some())
                            .all(|(id, _)| {
                                if resources.restored[..resources.restored_count]
                                    .iter()
                                    .flatten()
                                    .any(|(restored, _)| restored == id)
                                {
                                    return true;
                                }
                                if let Some(p) = players[..player_count]
                                    .iter()
                                    .flatten()
                                    .find(|p| p.id == *id)
                                {
                                    return !p.dirty && p.pending.is_none();
                                }
                                self.handle_owned(*id)
                                    .and_then(|h| self.shadow_counter_player(&h, overlays))
                                    .is_ok_and(|p| !p.dirty && p.pending.is_none())
                            });
                    if remaining == 0
                        && resources
                            .publications
                            .is_none_or(crate::item_publication::PublicationLedger::is_empty)
                        && resources.pending_count == 0
                        && players_clean
                        && entries
                            .iter()
                            .filter(|e| e.present)
                            .all(|e| e.generation == e.saved && !e.due)
                    {
                        stopped = true;
                    }
                }
                InputKind::RestoreItem { read, wire_id } if !reserved => {
                    if self.validate_binding(&read.binding).is_err() || !read.native_item_checked {
                        continue;
                    }
                    let Some(record) = read.snapshot.record() else {
                        continue;
                    };
                    let Ok(header) = crate::entity::checked_native_header(
                        read.snapshot.expectation().key(),
                        record,
                    ) else {
                        continue;
                    };
                    let base = crate::EntitySpawn::new(
                        read.snapshot.expectation().key().world(),
                        read.snapshot.expectation().key().dimension(),
                        read.snapshot.expectation().key().uuid(),
                        *wire_id,
                        header.entity_type,
                        header.transform,
                    );
                    let body = record.record().data().len();
                    if !self.shadow_spawn_admitted(
                        entries,
                        base,
                        body,
                        live_bytes,
                        confirmed_bytes,
                        resources.entity_capacity,
                    ) {
                        continue;
                    }
                    let Ok(publication_charge) = shadow_publication_charge(
                        &resources,
                        Some(body - crate::NATIVE_ITEM_ENTITY_HEADER_BYTES),
                    ) else {
                        continue;
                    };
                    reserve_shadow_publication(&mut resources, publication_charge, true);
                    live_bytes += body;
                    confirmed_bytes += body;
                    entries.push(CounterBoundaryEntry {
                        original_counters: header.properties.counters(),
                        original_generation: 1,
                        slot: usize::MAX,
                        uuid: base.uuid,
                        counters: header.properties.counters(),
                        generation: 1,
                        deferred: None,
                        claims: 0,
                        due: false,
                        resident: true,
                        present: true,

                        saved: 1,
                        wire: *wire_id,
                        body_len: body,
                        confirmed_len: body,
                        first_due: None,
                    });
                }
                InputKind::EntitySave {
                    item,
                    generation,
                    id,
                } if !reserved => {
                    let Ok(base) = entities.get(item) else {
                        continue;
                    };
                    let Some(entry) = entries
                        .iter_mut()
                        .find(|e| e.present && e.uuid == base.uuid())
                    else {
                        continue;
                    };
                    if entry.slot == usize::MAX || entry.generation != *generation {
                        continue;
                    }
                    let after = if entry.due { 0 } else { entry.body_len };
                    let save_charge = entry
                        .confirmed_len
                        .checked_mul(2)
                        .and_then(|n| after.checked_mul(5).and_then(|a| n.checked_add(a)))
                        .and_then(|n| n.checked_add(661));
                    if save_charge.is_none_or(|n| {
                        n > state
                            .item_counters
                            .as_ref()
                            .map_or(0, |c| c.limits.save_charge)
                    }) {
                        continue;
                    }
                    let publication_charge = if entry.due {
                        let Ok(charge) = shadow_publication_charge(&resources, None) else {
                            continue;
                        };
                        charge
                    } else {
                        None
                    };
                    if !self.shadow_candidate_admitted(
                        &mut resources,
                        consumed,
                        *id,
                        entry.confirmed_len,
                        after,
                        None,
                    )? {
                        continue;
                    }
                    reserve_shadow_publication(&mut resources, publication_charge, false);
                    entry.original_counters = entry.counters;
                    entry.original_generation = entry.generation;
                    entry.deferred = Some(*id);
                    entry.claims = 0;
                    reserved = true;
                }
                InputKind::Pickup {
                    item,
                    read,
                    id,
                    cancelled: false,
                    ..
                } if !reserved => {
                    let Some(p) = player else {
                        continue;
                    };
                    if !self.shadow_pair_admitted(p, read) {
                        continue;
                    }
                    let Ok(base) = entities.get(item) else {
                        continue;
                    };
                    let Some(entry) = entries
                        .iter_mut()
                        .find(|e| e.present && e.uuid == base.uuid())
                    else {
                        continue;
                    };
                    if entry.slot == usize::MAX || entry.due || !entry.resident {
                        continue;
                    }
                    let mut source = entities.item_state(item)?;
                    let mut confirmed = &entities.item_durability(item)?.confirmed;
                    let mut stamp = entities.owned_expectation(item)?.stamp();
                    for o in overlays.iter().flatten().filter(|o| o.confirmed) {
                        if o.pending
                            .entity_expected
                            .as_ref()
                            .is_some_and(|e| e.key().uuid() == entry.uuid)
                        {
                            if let Some(PreparedOwnedEntity::Replace { item, .. }) =
                                &o.pending.after_entity
                            {
                                source = item;
                            }
                            if let Some(after) = &o.pending.after_entity_record {
                                confirmed = after;
                            }
                            if let CompletionEvidence::Confirmed { receipt, .. } =
                                &o.completion.evidence
                            {
                                stamp = receipt.entity().map_or(stamp, |(_, _, s)| s);
                            }
                        }
                    }
                    let entity_read = read.snapshot.entity();
                    if entity_read.expectation().key() != entities.owned_expectation(item)?.key()
                        || entity_read.stamp() != stamp
                        || entity_read
                            .record()
                            .is_none_or(|r| !entity_equal(r, confirmed))
                    {
                        continue;
                    }
                    let Ok(shape) = borrowed_pickup_shape(
                        p.slots,
                        p.carried,
                        p.selected,
                        p.id,
                        p.mode,
                        p.position,
                        source.stack(),
                        entry.counters,
                        source.properties().owner_target(),
                        base.transform().position(),
                    ) else {
                        continue;
                    };
                    let after_entity = if shape.remainder == 0 {
                        0
                    } else {
                        entry.body_len
                    };
                    let stack_charge = if shape.remainder == 0 {
                        None
                    } else {
                        Some(entry.body_len - crate::NATIVE_ITEM_ENTITY_HEADER_BYTES)
                    };
                    let Ok(publication_charge) =
                        shadow_publication_charge(&resources, stack_charge)
                    else {
                        continue;
                    };
                    if !self.shadow_candidate_admitted(
                        &mut resources,
                        consumed,
                        *id,
                        p.record.data().len() + entry.confirmed_len,
                        shape.after_len + after_entity,
                        Some((p.id, p.record.data().len(), shape.after_len)),
                    )? {
                        continue;
                    }
                    reserve_shadow_publication(&mut resources, publication_charge, false);
                    let generation = if shape.remainder == 0 {
                        entry.generation
                    } else {
                        entry
                            .generation
                            .checked_add(1)
                            .ok_or(PlayerOwnershipError::GenerationExhausted)?
                    };
                    entry.original_counters = entry.counters;
                    entry.original_generation = entry.generation;
                    entry.generation = generation;
                    entry.deferred = Some(*id);
                    entry.claims = 0;
                    if shape.remainder != 0 && entry.counters.pickup_delay() < 0 {
                        entry.counters = ItemEntityCounters::new(
                            entry.counters.age(),
                            0,
                            entry.counters.health(),
                        );
                    }
                    if let Some(i) = player_index {
                        if let Some(p) = &mut players[i] {
                            p.pending = Some(*id);
                        }
                    }
                    reserved = true;
                }
                InputKind::SpawnItem { read, proposal, id } if !reserved => {
                    if self.validate_binding(&read.binding).is_err()
                        || !read.snapshot.stamp().is_fresh()
                        || read.snapshot.record().is_some()
                        || read.snapshot.expectation().key().uuid() != proposal.base().uuid
                    {
                        continue;
                    }
                    let Ok(stack_len) = persisted_stack_size(proposal.stack()) else {
                        continue;
                    };
                    let body = crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + stack_len;
                    if proposal.stack().count() == 0
                        || !self.shadow_spawn_admitted(
                            entries,
                            proposal.base(),
                            body,
                            live_bytes,
                            confirmed_bytes,
                            resources.entity_capacity,
                        )
                    {
                        continue;
                    }
                    let Ok(publication_charge) =
                        shadow_publication_charge(&resources, Some(stack_len))
                    else {
                        continue;
                    };
                    if self.shadow_candidate_admitted(
                        &mut resources,
                        consumed,
                        *id,
                        0,
                        body,
                        None,
                    )? {
                        reserve_shadow_publication(&mut resources, publication_charge, false);
                        reserved = true;
                    }
                }
                InputKind::Drop {
                    read,
                    selection,
                    wire_id,
                    id,
                    seed,
                    cancelled: false,
                    ..
                } if !reserved => {
                    let Some(p) = player else {
                        continue;
                    };
                    if !self.shadow_pair_admitted(p, read)
                        || p.mode == GameMode::Spectator
                        || !read.snapshot.entity().stamp().is_fresh()
                        || read.snapshot.entity().record().is_some()
                    {
                        continue;
                    }
                    let Ok(mut plan) = InventoryPlan::new(p.slots, p.carried) else {
                        continue;
                    };
                    let Ok((source, _)) =
                        plan.drop_selected(p.selected, *selection == DropSelection::All)
                    else {
                        continue;
                    };
                    let Ok(after) = plan.encoded_len() else {
                        continue;
                    };
                    let Ok(stack_len) = persisted_stack_size(source) else {
                        continue;
                    };
                    let body = crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + stack_len;
                    let Ok((base, _)) = standing_drop_parts(
                        state.binding.generation().key(),
                        p.id,
                        read.snapshot.entity().expectation().key().uuid(),
                        *wire_id,
                        p.position,
                        p.look,
                        *seed,
                    ) else {
                        continue;
                    };
                    if !self.shadow_spawn_admitted(
                        entries,
                        base,
                        body,
                        live_bytes,
                        confirmed_bytes,
                        resources.entity_capacity,
                    ) {
                        continue;
                    }
                    let Ok(publication_charge) =
                        shadow_publication_charge(&resources, Some(stack_len))
                    else {
                        continue;
                    };
                    if self.shadow_candidate_admitted(
                        &mut resources,
                        consumed,
                        *id,
                        p.record.data().len(),
                        after + body,
                        Some((p.id, p.record.data().len(), after)),
                    )? {
                        reserve_shadow_publication(&mut resources, publication_charge, false);
                        if let Some(i) = player_index {
                            if let Some(p) = &mut players[i] {
                                p.pending = Some(*id);
                            }
                        }
                        reserved = true;
                    }
                }
                InputKind::Autosave { id, .. } | InputKind::Click { id, .. } => {
                    let Some(p) = player else {
                        continue;
                    };
                    if p.pending.is_some() {
                        continue;
                    }
                    let Ok(mut plan) = InventoryPlan::new(p.slots, p.carried) else {
                        continue;
                    };
                    if let InputKind::Click {
                        generation,
                        slot,
                        button,
                        ..
                    } = input.0
                    {
                        if !p.connected
                            || p.mode == GameMode::Spectator
                            || generation != p.mutation
                            || plan.click(slot, button).is_err()
                        {
                            continue;
                        }
                    }
                    let Ok(after) = plan.encoded_len() else {
                        continue;
                    };
                    if self.shadow_candidate_admitted(
                        &mut resources,
                        consumed,
                        *id,
                        p.record.data().len(),
                        after,
                        Some((p.id, p.record.data().len(), after)),
                    )? {
                        if let Some(i) = player_index {
                            if let Some(p) = &mut players[i] {
                                p.pending = Some(*id);
                            }
                        }
                    }
                }
                InputKind::Move { position, look, .. } => {
                    let Some(i) = player_index else {
                        continue;
                    };
                    let Some(p) = &mut players[i] else {
                        continue;
                    };
                    if !p.connected
                        || position.is_some_and(|p| {
                            !super::super::is_valid_position(p)
                                || self.require_player_region(p).is_err()
                        })
                        || look.iter().flatten().any(|v| !v.is_finite())
                    {
                        continue;
                    }
                    if p.pending.is_none() {
                        resources.generation.next()?;
                        if position.is_none() && look.iter().all(Option::is_none) {
                            continue;
                        }
                        resources.generation = resources.generation.next()?;
                        p.mutation = resources.generation;
                        p.dirty = true;
                        if let Some(position) = position {
                            p.position = *position;
                        }
                        for (dest, new) in p.look.iter_mut().zip(look) {
                            if let Some(new) = new {
                                *dest = *new;
                            }
                        }
                    }
                }
                InputKind::Select { selected, .. } => {
                    let Some(i) = player_index else {
                        continue;
                    };
                    let Some(p) = &mut players[i] else {
                        continue;
                    };
                    if !p.connected || *selected > 8 {
                        continue;
                    }
                    if p.pending.is_none() {
                        resources.generation = resources.generation.next()?;
                        p.mutation = resources.generation;
                        p.dirty = true;
                        p.selected = *selected;
                    }
                }
                InputKind::Mode { mode, .. } => {
                    let Some(i) = player_index else {
                        continue;
                    };
                    let Some(p) = &mut players[i] else {
                        continue;
                    };
                    if !p.connected {
                        continue;
                    }
                    if p.pending.is_none() {
                        resources.generation = resources.generation.next()?;
                        p.mutation = resources.generation;
                        p.dirty = true;
                        p.mode = *mode;
                    }
                }
                InputKind::Disconnect(_) => {
                    if let Some(i) = player_index {
                        if let Some(p) = &mut players[i] {
                            p.connected = false;
                        }
                    }
                }
                InputKind::RestorePlayer(read) | InputKind::Rejoin(read) => {
                    let rejoin = matches!(input.0, InputKind::Rejoin(_));
                    let id = read.snapshot.expectation().key().id();
                    let Some(record) = read.snapshot.record() else {
                        continue;
                    };
                    let Some(position) = read.native_position else {
                        continue;
                    };
                    if self.validate_binding(&read.binding).is_err()
                        || self.require_player_region(position).is_err()
                    {
                        continue;
                    }
                    let existing = if let Some(p) = players[..player_count]
                        .iter()
                        .flatten()
                        .find(|p| p.id == id)
                    {
                        Some(*p)
                    } else {
                        self.handle_owned(id)
                            .ok()
                            .and_then(|h| self.shadow_counter_player(&h, overlays).ok())
                    };
                    if !self.shadow_player_admission(
                        &resources,
                        consumed,
                        id,
                        record.data().len(),
                        rejoin,
                        existing,
                    ) {
                        continue;
                    }
                    if rejoin
                        && existing.is_none_or(|p| {
                            p.stamp != read.snapshot.stamp() || !record_equal(p.record, record)
                        })
                    {
                        continue;
                    }
                    resources.generation = resources.generation.next()?;
                    resources.player_bytes = resources.player_bytes
                        - existing.map_or(0, |p| p.record.data().len())
                        + record.data().len();
                    if existing.is_none() {
                        resources.player_count += 1;
                    }
                    resources.restored[resources.restored_count] = Some((id, record.data().len()));
                    resources.restored_count += 1;
                }
                InputKind::Bootstrap {
                    read,
                    position,
                    look,
                    id,
                } => {
                    let player_id = read.snapshot.expectation().key().id();
                    if self.validate_binding(&read.binding).is_err()
                        || !read.snapshot.stamp().is_fresh()
                        || read.snapshot.record().is_some()
                        || self.require_player_region(*position).is_err()
                        || [position.x, position.y, position.z]
                            .iter()
                            .any(|n| !n.is_finite() || n.abs() > 30_000_000.0)
                        || look.iter().any(|n| !n.is_finite())
                    {
                        continue;
                    }
                    let existing = self
                        .handle_owned(player_id)
                        .ok()
                        .and_then(|h| self.shadow_counter_player(&h, overlays).ok());
                    if !self.shadow_player_admission(
                        &resources, consumed, player_id, 558, false, existing,
                    ) {
                        continue;
                    }
                    if self.shadow_candidate_admitted(
                        &mut resources,
                        consumed,
                        *id,
                        0,
                        558,
                        Some((player_id, 0, 558)),
                    )? {
                        resources.bootstraps += 1;
                        resources.bootstrap_bytes += 558;
                    }
                }
                InputKind::Retry(query) | InputKind::Reconcile(query) => {
                    if consumed.contains(&Some(query.id()))
                        || resources.dispatched[..resources.dispatched_count]
                            .contains(&Some(query.id()))
                    {
                        continue;
                    }
                    let Some(pending) = state.pending.get(&query.id()) else {
                        continue;
                    };
                    if pending.ticket.query != *query {
                        continue;
                    }
                    // Refused inputs retain their genuine completion lease in the
                    // outcome. Only a consumed publication removes this pending;
                    // other queued completions cannot free it for same-boundary retry.
                    if pending.buffer.strong_count() != 0
                        || (matches!(input.0, InputKind::Retry(_))
                            && pending.status != OwnershipPendingStatus::Prepared)
                    {
                        continue;
                    }
                    resources.generation = resources.generation.next()?;
                    resources.dispatched[resources.dispatched_count] = Some(query.id());
                    resources.dispatched_count += 1;
                }
                _ => {}
            }
        }
        Ok(stopped)
    }

    fn shadow_counter_player<'a>(
        &'a self,
        handle: &OwnedPlayerHandle,
        overlays: &[Option<CompletionOverlay<'a>>],
    ) -> Result<ShadowPlayer<'a>, PlayerOwnershipError> {
        let player = self.actual_player(handle)?;
        let owned = player
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        let mut view = ShadowPlayer {
            id: handle.player,
            admission: owned.admission,
            mutation: owned.mutation,
            position: player.position,
            look: [player.yaw, player.pitch],
            mode: player.game_mode,
            selected: owned.selected,
            slots: &owned.slots,
            carried: &owned.carried,
            record: &owned.confirmed,
            stamp: owned.expected.stamp(),
            pending: owned.pending,
            connected: owned.connected,
            dirty: owned.dirty,
            invalid: false,
        };
        for o in overlays.iter().flatten() {
            if o.confirmed && o.pending.player == Some(handle.player) {
                if let (Some(snapshot), Some(record)) =
                    (&o.pending.after_player, &o.pending.after_player_record)
                {
                    view.position = snapshot.position;
                    view.look = snapshot.look;
                    view.mode = snapshot.game_mode;
                    view.selected = snapshot.selected;
                    view.slots = &snapshot.slots;
                    view.carried = &snapshot.carried;
                    view.record = record;
                    view.mutation = o.generation;
                    view.dirty = false;
                    if let CompletionEvidence::Confirmed { receipt, .. } = &o.completion.evidence {
                        view.stamp = receipt.player().map_or(view.stamp, |(_, _, s)| s);
                    }
                }
            }
            let releasing = owned.pending == Some(o.pending.ticket.query.id())
                || (owned.pending.is_none() && o.releases_unreserved);
            if !releasing {
                continue;
            }
            view.pending = None;
            if has_deferred_player(owned) {
                let rank = self
                    .players
                    .iter()
                    .filter(|(id, p)| {
                        **id <= handle.player
                            && p.owned.as_ref().is_some_and(|p| {
                                has_deferred_player(p)
                                    && (p.pending == Some(o.pending.ticket.query.id())
                                        || (p.pending.is_none() && o.releases_unreserved))
                            })
                    })
                    .count() as u64;
                view.mutation = PlayerLocalGeneration(
                    o.deferred_start
                        .get()
                        .checked_add(rank)
                        .and_then(std::num::NonZeroU64::new)
                        .ok_or(PlayerOwnershipError::GenerationExhausted)?,
                );
                view.dirty = true;
            }
            if let Some(position) = owned.deferred_position {
                view.position = position;
            }
            for (dest, new) in view.look.iter_mut().zip(owned.deferred_look) {
                if let Some(new) = new {
                    *dest = new;
                }
            }
            if let Some(mode) = owned.deferred_mode {
                view.mode = mode;
            }
            if let Some(selected) = owned.deferred_selected {
                view.selected = selected;
            }
        }
        Ok(view)
    }

    fn shadow_pair_admitted(&self, player: ShadowPlayer<'_>, read: &OwnedPairRead) -> bool {
        self.validate_binding(&read.binding).is_ok()
            && player.pending.is_none()
            && player.connected
            && read.snapshot.player().expectation().key().id() == player.id
            && read.snapshot.player().stamp() == player.stamp
            && read
                .snapshot
                .player()
                .record()
                .is_some_and(|r| record_equal(r, player.record))
    }

    fn shadow_spawn_admitted(
        &self,
        entries: &[CounterBoundaryEntry],
        base: crate::EntitySpawn,
        body: usize,
        live: usize,
        confirmed: usize,
        capacity: usize,
    ) -> bool {
        let Some(e) = &self.entities else {
            return false;
        };
        let Some(c) = self
            .player_ownership
            .as_ref()
            .and_then(|s| s.item_counters.as_ref())
        else {
            return false;
        };
        let owner = e.owner();
        base.world == owner.world()
            && base.dimension == owner.dimension()
            && base.entity_type.resource_location() == "minecraft:item"
            && owner.region().contains_chunk(
                owner.world(),
                owner.dimension(),
                base.transform.chunk(),
            )
            && self.chunks.is_loaded(base.transform.chunk())
            && body <= crate::MAX_NATIVE_ITEM_ENTITY_BYTES
            && body <= e.item_byte_budget().saturating_sub(live)
            && body <= c.limits.confirmed_bytes.saturating_sub(confirmed)
            && entries.iter().filter(|e| e.present).count() < capacity
            && !entries
                .iter()
                .any(|e| e.present && (e.uuid == base.uuid || e.wire == base.wire_id))
    }

    fn shadow_candidate_admitted(
        &self,
        resources: &mut ShadowResources,
        consumed: &[Option<OwnershipTransferId>],
        id: OwnershipTransferId,
        before: usize,
        after: usize,
        player: Option<(ferrumc_core::PlayerId, usize, usize)>,
    ) -> Result<bool, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.pending.contains_key(&id) && !consumed.contains(&Some(id))
            || resources.new_ids[..resources.admitted].contains(&Some(id))
            || resources.pending_count >= state.limits.pending
        {
            return Ok(false);
        }
        let Some(charge) = before
            .checked_mul(2)
            .and_then(|n| after.checked_mul(5).and_then(|a| n.checked_add(a)))
            .and_then(|n| n.checked_add(661))
        else {
            return Ok(false);
        };
        if charge
            > state
                .limits
                .pending_bytes
                .saturating_sub(resources.pending_bytes)
        {
            return Ok(false);
        }
        if let Some((_, old, new)) = player {
            let used = resources
                .player_bytes
                .saturating_sub(old)
                .saturating_add(resources.pending_player_bytes);
            if new > state.limits.player_bytes.saturating_sub(used) {
                return Ok(false);
            }
        }
        resources.generation = resources.generation.next()?;
        resources.new_ids[resources.admitted] = Some(id);
        resources.new_player_ids[resources.admitted] = player.map(|(id, _, _)| id);
        resources.admitted += 1;
        resources.pending_count += 1;
        resources.pending_bytes += charge;
        if let Some((_, _, new)) = player {
            resources.pending_player_bytes += new;
        }
        Ok(true)
    }

    fn shadow_player_admission(
        &self,
        resources: &ShadowResources,
        consumed: &[Option<OwnershipTransferId>],
        id: ferrumc_core::PlayerId,
        len: usize,
        rejoin: bool,
        existing: Option<ShadowPlayer<'_>>,
    ) -> bool {
        let Some(state) = &self.player_ownership else {
            return false;
        };
        if resources.restored[..resources.restored_count]
            .iter()
            .flatten()
            .any(|(p, _)| *p == id)
            || resources.new_player_ids[..resources.admitted].contains(&Some(id))
            || state
                .pending
                .values()
                .any(|p| p.player == Some(id) && !consumed.contains(&Some(p.ticket.query.id())))
        {
            return false;
        }
        if let Some(p) = existing {
            if !rejoin || p.connected || p.pending.is_some() || p.dirty {
                return false;
            }
        } else if rejoin
            || self.players.contains_key(&id)
            || resources.player_count + resources.bootstraps >= state.limits.players
        {
            return false;
        }
        let old = existing.map_or(0, |p| p.record.data().len());
        len <= state
            .limits
            .player_bytes
            .saturating_sub(resources.player_bytes - old + resources.bootstrap_bytes)
    }

    fn apply_counter_boundary(&mut self, tick: Tick, entries: &[CounterBoundaryEntry]) {
        for entry in entries.iter().filter(|e| e.present && e.resident) {
            if let Some(id) = entry.deferred {
                let Some(pending) = self
                    .player_ownership
                    .as_mut()
                    .and_then(|s| s.pending.get_mut(&id))
                else {
                    panic!("preflighted item counter pending disappeared");
                };
                let Some(d) = &mut pending.counters else {
                    panic!("preflighted item counter projection disappeared");
                };
                d.counters = entry.counters;
                d.generation = entry.generation;
                d.original_counters = entry.original_counters;
                d.original_generation = entry.original_generation;
                d.claims = entry.claims;
                d.first = d.first.or(Some(tick));
                d.last = Some(tick);
                d.due = entry.due;
                d.first_due = entry.first_due;
            } else if let Some(e) = &mut self.entities {
                e.apply_counter_entry(entry);
            }
        }
    }

    pub(super) fn plan_counter_save(
        &self,
        handle: &crate::EntityHandle,
        generation: u64,
    ) -> Result<Candidate, PlayerOwnershipError> {
        let counter = self
            .ownership_state()?
            .item_counters
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        if entities.counter_entity_reserved() {
            return Err(PlayerOwnershipError::Reserved);
        }
        let d = entities.item_durability(handle)?;
        if generation != d.generation || generation == 0 {
            return Err(PlayerOwnershipError::StaleGeneration);
        }
        let before_len = d.confirmed.record().data().len();
        let after_len = if d.due {
            0
        } else {
            entities.item_state(handle)?.native_body_len()
        };
        let charge = self.pending_charge(before_len, after_len)?;
        if charge > counter.limits.save_charge {
            return Err(PlayerOwnershipError::Capacity("entity save charge"));
        }
        entities.check_confirmed_replacement(before_len, after_len)?;
        let (prepared, after) = entities.prepare_owned_item_save(handle, d.due)?;
        Ok(Candidate {
            collector: None,
            kind: if d.due {
                PendingKind::EntityDespawn
            } else {
                PendingKind::EntityAutosave
            },
            player_expected: None,
            entity_expected: Some(entities.owned_expectation(handle)?.clone()),
            before_player: None,
            before_entity: Some(d.confirmed.clone()),
            after_player: None,
            after_player_record: None,
            after_entity: Some(prepared),
            after_entity_record: after,
            local_before: None,
            moved: 0,
            original_count: 0,
            stat_item: None,
            animate: false,
            charge,
        })
    }

    /// Returns the checked current local item generation, without save acknowledgement.
    pub fn owned_item_generation(
        &self,
        handle: &crate::EntityHandle,
    ) -> Result<u64, PlayerOwnershipError> {
        self.ownership_state()?;
        Ok(self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .item_durability(handle)?
            .generation)
    }

    fn select_counter_save(&mut self, outcome: &mut PlayerOwnershipTickOutcome) {
        let Some(entities) = &self.entities else {
            return;
        };
        if entities.counter_entity_reserved() {
            return;
        }
        let Some(counter) = self
            .player_ownership
            .as_ref()
            .and_then(|s| s.item_counters.as_ref())
        else {
            return;
        };
        if self.player_ownership.as_ref().is_some_and(|s| s.stopped) || entities.slot_count() == 0 {
            return;
        }
        let limits = counter.limits;
        let priority_lane = counter.selection_turn != 3;
        let tick = counter.tick;
        let slot_count = entities.slot_count();
        let mut cursor = if priority_lane {
            counter.priority_cursor
        } else {
            counter.normal_cursor
        } % slot_count;
        let mut selected = None;
        for _ in 0..limits.save_scan.min(slot_count) {
            outcome.save_scanned += 1;
            let index = cursor;
            cursor = (cursor + 1) % slot_count;
            let Some(handle) = entities.slot_handle(index) else {
                continue;
            };
            let Ok(d) = entities.item_durability(&handle) else {
                continue;
            };
            let needs = if priority_lane {
                d.priority || d.due
            } else {
                d.generation != d.saved
                    && tick.get().saturating_sub(d.saved_tick.get())
                        >= u64::from(limits.save_period)
            };
            if needs {
                selected = Some((handle, d.generation));
                break;
            }
        }
        if let Some(c) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.item_counters.as_mut())
        {
            if priority_lane {
                c.priority_cursor = cursor;
            } else {
                c.normal_cursor = cursor;
            }
            c.selection_turn = (c.selection_turn + 1) % 4;
        }
        let Some((handle, generation)) = selected else {
            return;
        };
        let Some(id) = self
            .player_ownership
            .as_ref()
            .and_then(|s| s.item_counters.as_ref())
            .and_then(|c| c.save_ids.first())
            .copied()
        else {
            outcome.intents.push(OwnershipIntent::SaveIdsNeeded);
            return;
        };
        let admission = self
            .plan_counter_save(&handle, generation)
            .and_then(|candidate| self.install_candidate(id, candidate, outcome));
        match admission {
            Ok(()) => {
                if let Some(c) = self
                    .player_ownership
                    .as_mut()
                    .and_then(|s| s.item_counters.as_mut())
                {
                    c.save_ids.remove(0);
                }
            }
            Err(reason) => outcome.intents.push(OwnershipIntent::ItemSaveBlocked {
                item: handle,
                reason,
            }),
        }
    }
}

// The native header and fixed FCIS count byte make shadow body sizing exact.
// The actual admission uses the same checked charge and ledger predicate.
fn shadow_publication_charge(
    resources: &ShadowResources,
    stack_len: Option<usize>,
) -> Result<Option<usize>, crate::ItemPublicationError> {
    let Some(ledger) = resources.publications else {
        return Ok(None);
    };
    let charge = crate::item_publication::publication_charge(stack_len)?;
    ledger.check(charge)?;
    Ok(Some(charge))
}

fn reserve_shadow_publication(
    resources: &mut ShadowResources,
    charge: Option<usize>,
    restored: bool,
) {
    if let (Some(ledger), Some(charge)) = (&mut resources.publications, charge) {
        ledger.reserve(charge);
        if restored {
            let _ = ledger.publish(charge);
        }
    }
}

fn prepared_uuid(
    entities: &crate::EntityStore,
    prepared: &PreparedOwnedEntity,
) -> Result<EntityUuid, PlayerOwnershipError> {
    Ok(match prepared {
        PreparedOwnedEntity::Spawn { base, .. } => base.uuid,
        PreparedOwnedEntity::Replace { handle, .. }
        | PreparedOwnedEntity::Remove { handle, .. } => entities.get(handle)?.uuid(),
    })
}

fn completion_release(
    shard: &SimShard,
    p: &PendingPlayerMutation,
    c: &PlayerOwnershipCompletion,
) -> Option<bool> {
    match &c.evidence {
        CompletionEvidence::Confirmed {
            receipt,
            current: Ok(current),
        } if SimShard::confirm_current(p, receipt, current).is_ok() => Some(true),
        CompletionEvidence::Refused(source)
            if !matches!(
                source,
                OwnershipError::Busy
                    | OwnershipError::HistoryCapacity { .. }
                    | OwnershipError::CasConflict { .. }
            ) =>
        {
            Some(false)
        }
        _ => {
            let _ = shard;
            None
        }
    }
}

fn has_deferred_player(p: &OwnedPlayerInventory) -> bool {
    p.deferred_position.is_some()
        || p.deferred_look.iter().any(Option::is_some)
        || p.deferred_mode.is_some()
        || p.deferred_selected.is_some()
}

fn bind_projection(
    entry: &mut CounterBoundaryEntry,
    id: OwnershipTransferId,
    d: DeferredOwnedCounters,
) {
    entry.counters = d.counters;
    entry.generation = d.generation;
    entry.deferred = Some(id);
    entry.claims = d.claims;
    entry.original_counters = d.original_counters;
    entry.original_generation = d.original_generation;
    entry.due = d.due;
    entry.first_due = d.first_due;
}

fn scalar_step(
    counters: ItemEntityCounters,
    generation: u64,
    rate: i32,
) -> Result<(ItemEntityCounters, u64, bool), PlayerOwnershipError> {
    let step = crate::step_inactive_item_counters(counters, rate);
    let next = if step.counters() == counters {
        generation
    } else {
        generation
            .checked_add(1)
            .ok_or(PlayerOwnershipError::GenerationExhausted)?
    };
    Ok((
        step.counters(),
        next,
        step.decision() == ItemDespawnDecision::Due,
    ))
}

fn step_boundary_entry(
    entry: &mut CounterBoundaryEntry,
    tick: Tick,
    limits: OwnedItemCounterLimits,
) -> Result<(), PlayerOwnershipError> {
    if entry.deferred.is_none() && entry.due {
        return Ok(());
    }
    if entry.deferred.is_some() {
        entry.claims = entry
            .claims
            .checked_add(1)
            .filter(|n| *n <= limits.defer)
            .ok_or(PlayerOwnershipError::CounterRecoveryRequired)?;
        let (original, generation, _) = scalar_step(
            entry.original_counters,
            entry.original_generation,
            limits.despawn_rate,
        )?;
        entry.original_counters = original;
        entry.original_generation = generation;
    }
    let (counters, generation, due) =
        scalar_step(entry.counters, entry.generation, limits.despawn_rate)?;
    entry.counters = counters;
    entry.generation = generation;
    entry.due |= due;
    if due {
        entry.first_due = entry.first_due.or(Some(tick));
    }
    Ok(())
}
