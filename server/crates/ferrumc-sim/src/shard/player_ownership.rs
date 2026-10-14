//! Actual player authority and receipt-confirmed tick-boundary consumers.

mod counters;
mod lifecycle;
mod publications;
mod release;

use ferrumc_core::{EntityId, EntityUuid, GameMode, PlayerId};
use ferrumc_items::{persisted_stack_size, plan_split, ItemStack};
use ferrumc_math::{ChunkPos, Vec3};
use ferrumc_storage::{
    EntityOwnershipCasRequest, EntityOwnershipProposal, OwnedEntityExpectation,
    OwnedPlayerExpectation, OwnershipError, OwnershipReceipt, OwnershipReceiptQuery,
    OwnershipRevision, OwnershipStamp, OwnershipTransferId, PersistentEntityRecord,
    PlayerEntityTransferRequest, PlayerOwnershipCasRequest, PlayerOwnershipProposal, PlayerRecord,
};
use std::sync::Arc;

use super::{PlayerState, SimShard};
use crate::entity::ownership::{EntityPublication, PreparedOwnedEntity};
use crate::player::evidence::{
    check_receipt_epochs, CompletionEvidence, CompletionTicket, CurrentResources, WorkKeys,
    WorkRequest,
};
use crate::player::pending::{InputKind, PendingKind};
use crate::player::plan::InventoryPlan;
use crate::player::{
    same_owner, OwnedPlayerInventory, PendingPlayerMutation, PlayerOwnershipState,
};
use crate::{
    DropSelection, NativePlayerSnapshot, NormalSlotButton, OwnedEntityRead, OwnedPairRead,
    OwnedPlayerHandle, OwnedPlayerInput, OwnedPlayerInputRefusal, OwnedPlayerRead, OwnershipIntent,
    OwnershipPendingStatus, OwnershipStoreBinding, PlayerLocalGeneration,
    PlayerOwnershipCompletion, PlayerOwnershipError, PlayerOwnershipLimits,
    PlayerOwnershipStorageWork, PlayerOwnershipTickOutcome,
};

#[derive(Debug)]
struct Candidate {
    collector: Option<crate::ItemCollectorBinding>,
    kind: PendingKind,
    player_expected: Option<OwnedPlayerExpectation>,
    entity_expected: Option<OwnedEntityExpectation>,
    before_player: Option<PlayerRecord>,
    before_entity: Option<PersistentEntityRecord>,
    after_player: Option<NativePlayerSnapshot>,
    after_player_record: Option<PlayerRecord>,
    after_entity: Option<PreparedOwnedEntity>,
    after_entity_record: Option<PersistentEntityRecord>,
    local_before: Option<PlayerLocalGeneration>,
    moved: u8,
    original_count: u8,
    stat_item: Option<ferrumc_items::ItemId>,
    animate: bool,
    charge: usize,
}

struct BorrowedPickupShape<'a> {
    plan: InventoryPlan<'a>,
    moved: u8,
    remainder: u8,
    after_len: usize,
}

#[allow(
    clippy::too_many_arguments,
    reason = "borrowed player and item scalar facts share one checked pickup rule"
)]
fn borrowed_pickup_shape<'a>(
    slots: &'a [ItemStack; crate::NATIVE_PLAYER_SLOT_COUNT],
    carried: &'a ItemStack,
    selected: u8,
    player_id: PlayerId,
    mode: GameMode,
    position: Vec3,
    source: &'a ItemStack,
    counters: crate::ItemEntityCounters,
    target: Option<EntityUuid>,
    item_position: Vec3,
) -> Result<BorrowedPickupShape<'a>, PlayerOwnershipError> {
    if mode == GameMode::Spectator {
        return Err(PlayerOwnershipError::Ineligible("spectator pickup"));
    }
    if counters.pickup_delay() > 0 {
        return Err(PlayerOwnershipError::Ineligible("pickup delay"));
    }
    let player_uuid = EntityUuid::from_bytes(*player_id.as_uuid().as_bytes());
    if target.is_some_and(|id| id != player_uuid) {
        return Err(PlayerOwnershipError::Ineligible("pickup target"));
    }
    let delta = position - item_position;
    if delta.x.abs() > 1.5 || delta.y.abs() > 2.0 || delta.z.abs() > 1.5 {
        return Err(PlayerOwnershipError::Ineligible(
            "item outside conservative pickup envelope",
        ));
    }
    let mut plan = InventoryPlan::new(slots, carried)?;
    let moved = plan.pickup(selected, source)?;
    if moved == 0 {
        return Err(PlayerOwnershipError::Ineligible("inventory full"));
    }
    let remainder = source.count() - moved;
    let after_len = plan.encoded_len()?;
    Ok(BorrowedPickupShape {
        plan,
        moved,
        remainder,
        after_len,
    })
}

impl Candidate {
    fn request(&self, id: OwnershipTransferId) -> Result<WorkRequest, PlayerOwnershipError> {
        match (&self.player_expected, &self.entity_expected) {
            (Some(p), Some(e)) => Ok(WorkRequest::Pair(PlayerEntityTransferRequest::new(
                id,
                p.clone(),
                e.clone(),
                PlayerOwnershipProposal::put(
                    self.after_player_record
                        .as_ref()
                        .ok_or(PlayerOwnershipError::Malformed("missing player postimage"))?
                        .clone(),
                ),
                self.after_entity_record
                    .as_ref()
                    .map_or_else(EntityOwnershipProposal::delete, |r| {
                        EntityOwnershipProposal::put(r.clone())
                    }),
            )?)),
            (Some(p), None) => Ok(WorkRequest::Player(PlayerOwnershipCasRequest::new(
                id,
                p.clone(),
                PlayerOwnershipProposal::put(
                    self.after_player_record
                        .as_ref()
                        .ok_or(PlayerOwnershipError::Malformed("missing player postimage"))?
                        .clone(),
                ),
            )?)),
            (None, Some(e)) => Ok(WorkRequest::Entity(EntityOwnershipCasRequest::new(
                id,
                e.clone(),
                self.after_entity_record
                    .as_ref()
                    .map_or_else(EntityOwnershipProposal::delete, |r| {
                        EntityOwnershipProposal::put(r.clone())
                    }),
            )?)),
            (None, None) => Err(PlayerOwnershipError::Malformed("empty candidate")),
        }
    }
}

fn bytes(player: Option<&PlayerRecord>, entity: Option<&PersistentEntityRecord>) -> usize {
    player.map_or(0, |r| r.data().len()) + entity.map_or(0, |r| r.record().data().len())
}

fn record_equal(a: &PlayerRecord, b: &PlayerRecord) -> bool {
    a.schema_version() == b.schema_version()
        && a.game_mode() == b.game_mode()
        && a.data() == b.data()
}

fn entity_equal(a: &PersistentEntityRecord, b: &PersistentEntityRecord) -> bool {
    a.chunk() == b.chunk()
        && a.record().schema_version() == b.record().schema_version()
        && a.record().data() == b.record().data()
}

fn changed_stamp(before: OwnershipStamp, after: OwnershipStamp, live: bool, changed: bool) -> bool {
    if !changed {
        return before == after
            && if live {
                after.is_live()
            } else {
                after.is_tombstone()
            };
    }
    let revision = before
        .revision()
        .map_or(Some(1), |r| r.get().checked_add(1));
    after.revision().map(OwnershipRevision::get) == revision
        && if live {
            after.is_live()
        } else {
            after.is_tombstone()
        }
}

impl SimShard {
    /// Selects the concrete native player authority on an empty bound shard.
    ///
    /// Entities must already have an explicit item budget. This never opens a
    /// backend or activates the legacy application/connection inventory.
    pub fn initialize_player_ownership(
        &mut self,
        binding: OwnershipStoreBinding,
        limits: PlayerOwnershipLimits,
    ) -> Result<(), PlayerOwnershipError> {
        if self.player_ownership.is_some()
            || self.ground_items.is_some()
            || self.survival.is_some()
            || self.water.is_some()
            || self.furnaces.is_some()
            || self.mobs.is_some()
            || !self.players.is_empty()
            || !self.inbox.is_empty()
            || !self.pending_region_work.is_empty()
        {
            return Err(PlayerOwnershipError::AlreadyInitialized);
        }
        if self.chunks.generation_binding() != Some(binding.generation()) {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        let entities = self
            .entities
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        if entities.item_byte_budget() == 0 {
            return Err(PlayerOwnershipError::NotInitialized);
        }
        entities.initialize_ownership()?;
        self.player_ownership = Some(PlayerOwnershipState::new(binding, limits));
        Ok(())
    }

    fn ownership_state(&self) -> Result<&PlayerOwnershipState, PlayerOwnershipError> {
        let state = self
            .player_ownership
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        if state.snapshot_only {
            return Err(PlayerOwnershipError::SnapshotOnly);
        }
        if self.chunks.generation_binding() != Some(state.binding.generation()) {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        if entities.owner().world() != state.binding.generation().key().world()
            || entities.owner().dimension() != state.binding.generation().key().dimension()
            || entities.owner().position() != self.shard_pos
        {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        Ok(state)
    }

    /// Admits an intact move-owned input under both explicit count and byte caps.
    /// Refusal returns that same input; no player/entity value changes on enqueue.
    pub fn enqueue_owned_player_input(
        &mut self,
        input: OwnedPlayerInput,
    ) -> Result<(), OwnedPlayerInputRefusal> {
        let checked = (|| {
            let state = self.ownership_state()?;
            if state.stopped {
                return Err(PlayerOwnershipError::Draining);
            }
            let charge = input.charge()?;
            if state.input.len() >= state.limits.inputs {
                return Err(PlayerOwnershipError::Capacity("input count"));
            }
            if charge > state.limits.input_bytes.saturating_sub(state.input_bytes) {
                return Err(PlayerOwnershipError::Capacity("input bytes"));
            }
            Ok(charge)
        })();
        let charge = match checked {
            Ok(c) => c,
            Err(reason) => {
                return Err(OwnedPlayerInputRefusal {
                    reason,
                    input: Box::new(input),
                })
            }
        };
        if let Some(state) = self.player_ownership.as_mut() {
            if state.input.try_reserve(1).is_err() {
                return Err(OwnedPlayerInputRefusal {
                    reason: PlayerOwnershipError::AllocationFailed,
                    input: Box::new(input),
                });
            }
            state.input.push_back((input, charge));
            state.input_bytes += charge;
        }
        Ok(())
    }

    /// Applies genuine completions first, then admitted concrete actions in FIFO order.
    /// Returned work is executed externally; this synchronous method performs no I/O.
    pub fn run_tick_with_player_ownership(
        &mut self,
    ) -> Result<PlayerOwnershipTickOutcome, PlayerOwnershipError> {
        if self.ownership_state()?.item_counters.is_some() {
            return Err(PlayerOwnershipError::CounterTickMismatch);
        }
        self.run_player_ownership_boundary()
    }

    fn run_player_ownership_boundary(
        &mut self,
    ) -> Result<PlayerOwnershipTickOutcome, PlayerOwnershipError> {
        self.ownership_state()?;
        let mut outcome = PlayerOwnershipTickOutcome::default();
        let mut actions = std::collections::VecDeque::new();
        let input = if let Some(state) = self.player_ownership.as_mut() {
            state.input_bytes = 0;
            std::mem::take(&mut state.input)
        } else {
            return Err(PlayerOwnershipError::NotInitialized);
        };
        for (input, _) in input {
            if matches!(input.0, InputKind::Completion(_)) {
                self.apply_owned_input(input, &mut outcome);
            } else {
                actions.push_back(input);
            }
        }
        if let Some(state) = self.player_ownership.as_mut() {
            state.boundary_remaining = actions.len();
        }
        for input in actions {
            if let Some(state) = self.player_ownership.as_mut() {
                state.boundary_remaining -= 1;
            }
            self.apply_owned_input(input, &mut outcome);
        }
        if self
            .player_ownership
            .as_ref()
            .is_none_or(|s| s.item_counters.is_none())
        {
            outcome.game = self.run_tick();
        }
        Ok(outcome)
    }

    fn apply_owned_input(
        &mut self,
        input: OwnedPlayerInput,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) {
        if let Err(reason) = self.apply_owned_kind(&input.0, outcome) {
            self.resync_refused_input(&input.0, outcome);
            outcome.refusals.push(OwnedPlayerInputRefusal {
                reason,
                input: Box::new(input),
            });
        }
    }

    fn resync_refused_input(&self, input: &InputKind, outcome: &mut PlayerOwnershipTickOutcome) {
        let candidate = match input {
            InputKind::Drop { player, .. }
            | InputKind::Pickup { player, .. }
            | InputKind::Autosave { player, .. }
            | InputKind::Click { player, .. }
            | InputKind::Move { player, .. }
            | InputKind::Mode { player, .. }
            | InputKind::Select { player, .. }
            | InputKind::Disconnect(player) => Some(player.clone()),
            InputKind::Completion(c)
                if self
                    .ownership_state()
                    .is_ok_and(|s| same_owner(&c.ticket.owner, &s.owner)) =>
            {
                c.ticket
                    .keys
                    .player
                    .and_then(|p| self.handle_owned(p.id()).ok())
            }
            _ => None,
        };
        if let Some(player) = candidate {
            if let Ok(generation) = self.owned_player_generation(&player) {
                outcome
                    .intents
                    .push(OwnershipIntent::Resync { player, generation });
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "explicit concrete action dispatch preserves input ownership on refusal"
    )]
    fn apply_owned_kind(
        &mut self,
        input: &InputKind,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.stopped {
            return Err(PlayerOwnershipError::Draining);
        }
        if state.draining
            && !matches!(
                input,
                InputKind::Completion(_)
                    | InputKind::Retry(_)
                    | InputKind::Reconcile(_)
                    | InputKind::Autosave { .. }
                    | InputKind::RefillItemSaveIds { .. }
                    | InputKind::EntitySave { .. }
                    | InputKind::Disconnect(_)
                    | InputKind::Drain
                    | InputKind::Stop
            )
        {
            return Err(PlayerOwnershipError::Draining);
        }
        match input {
            InputKind::RefillItemSaveIds { owner, ids } => self.refill_counter_save_ids(owner, ids),
            InputKind::EntitySave {
                item,
                generation,
                id,
            } => {
                let candidate = self.plan_counter_save(item, *generation)?;
                self.install_candidate(*id, candidate, outcome)
            }
            InputKind::RestorePlayer(read) => self.restore_owned_player(read, false, outcome),
            InputKind::Rejoin(read) => self.restore_owned_player(read, true, outcome),
            InputKind::Bootstrap {
                read,
                position,
                look,
                id,
            } => {
                self.validate_binding(&read.binding)?;
                if !read.snapshot.stamp().is_fresh() || read.snapshot.record().is_some() {
                    return Err(PlayerOwnershipError::Ineligible(
                        "bootstrap requires never-seen miss",
                    ));
                }
                self.check_player_admission(read.snapshot.expectation().key().id(), 558, false)?;
                self.require_player_region(*position)?;
                self.staged_lifecycle_cell(self.ownership_state()?.generation.next()?)?;
                // The empty native body is fixed, so even its slot allocation
                // follows pending admission rather than spending it on refusal.
                let charge = self.pending_charge(0, 558)?;
                let snapshot = NativePlayerSnapshot::empty_survival(*position, *look);
                let record = snapshot.try_to_record()?;
                let candidate = Candidate {
                    collector: None,
                    kind: PendingKind::Bootstrap,
                    player_expected: Some(read.snapshot.expectation().clone()),
                    entity_expected: None,
                    before_player: None,
                    before_entity: None,
                    after_player: Some(snapshot),
                    after_player_record: Some(record),
                    after_entity: None,
                    after_entity_record: None,
                    local_before: None,
                    moved: 0,
                    original_count: 0,
                    stat_item: None,
                    animate: false,
                    charge,
                };
                self.install_candidate(*id, candidate, outcome)
            }
            InputKind::RestoreItem { read, wire_id } => {
                self.restore_owned_item(read, *wire_id, outcome)
            }
            InputKind::SpawnItem { read, proposal, id } => {
                self.validate_binding(&read.binding)?;
                if !read.snapshot.stamp().is_fresh()
                    || read.snapshot.record().is_some()
                    || read.snapshot.expectation().key().uuid() != proposal.base().uuid
                {
                    return Err(PlayerOwnershipError::StaleSnapshot);
                }
                let len = crate::NATIVE_ITEM_ENTITY_HEADER_BYTES
                    + persisted_stack_size(proposal.stack())?;
                let charge = self.pending_charge(0, len)?;
                let entities = self
                    .entities
                    .as_ref()
                    .ok_or(PlayerOwnershipError::NotInitialized)?;
                let (prepared, record) = entities.prepare_owned_spawn(&self.chunks, proposal)?;
                let candidate = Candidate {
                    collector: None,
                    kind: PendingKind::SpawnItem,
                    player_expected: None,
                    entity_expected: Some(read.snapshot.expectation().clone()),
                    before_player: None,
                    before_entity: None,
                    after_player: None,
                    after_player_record: None,
                    after_entity: Some(prepared),
                    after_entity_record: Some(record),
                    local_before: None,
                    moved: 0,
                    original_count: 0,
                    stat_item: None,
                    animate: false,
                    charge,
                };
                self.install_candidate(*id, candidate, outcome)
            }
            InputKind::Drop {
                player,
                read,
                selection,
                wire_id,
                id,
                seed,
                cancelled,
            } => {
                if *cancelled {
                    self.actual_player(player)?;
                    return Ok(());
                }
                let candidate = self.plan_owned_drop(player, read, *selection, *wire_id, *seed)?;
                self.install_candidate(*id, candidate, outcome)
            }
            InputKind::Pickup {
                player,
                item,
                read,
                id,
                animate,
                cancelled,
            } => {
                if *cancelled {
                    self.actual_player(player)?;
                    return Ok(());
                }
                let candidate = self.plan_owned_pickup(player, item, read, *animate)?;
                self.install_candidate(*id, candidate, outcome)
            }
            InputKind::Autosave { player, id } => {
                let candidate = self.plan_owned_player_write(player, None)?;
                self.install_candidate(*id, candidate, outcome)
            }
            InputKind::Click {
                player,
                generation,
                slot,
                button,
                id,
            } => {
                let candidate =
                    self.plan_owned_player_write(player, Some((*generation, *slot, *button)))?;
                self.install_candidate(*id, candidate, outcome)
            }
            InputKind::Move {
                player,
                position,
                look,
            } => self.move_owned_player(player, *position, *look, outcome),
            InputKind::Mode { player, mode } => self.mode_owned_player(player, *mode, outcome),
            InputKind::Select { player, selected } => {
                self.select_owned_player(player, *selected, outcome)
            }
            InputKind::Disconnect(player) => {
                self.apply_native_disconnect(player)?;
                outcome.intents.push(OwnershipIntent::PlayerDetached {
                    player: player.player,
                });
                Ok(())
            }
            InputKind::Completion(completion) => self.complete_owned(completion, outcome),
            InputKind::Retry(query) => self.dispatch_retained(*query, false, outcome),
            InputKind::Reconcile(query) => self.dispatch_retained(*query, true, outcome),
            InputKind::Drain => {
                if let Some(state) = self.player_ownership.as_mut() {
                    state.draining = true;
                }
                if let Some(e) = &mut self.entities {
                    e.mark_counter_flush(None);
                }
                outcome.intents.push(OwnershipIntent::Draining);
                Ok(())
            }
            InputKind::Stop => {
                if !self.ownership_quiescent() {
                    return Err(PlayerOwnershipError::NotQuiescent);
                }
                if let Some(state) = self.player_ownership.as_mut() {
                    state.draining = true;
                    state.stopped = true;
                }
                outcome.intents.push(OwnershipIntent::Stopped);
                Ok(())
            }
        }
    }

    fn validate_binding(
        &self,
        binding: &OwnershipStoreBinding,
    ) -> Result<(), PlayerOwnershipError> {
        if !self.ownership_state()?.binding.matches(binding) {
            return Err(PlayerOwnershipError::ForeignStore);
        }
        Ok(())
    }

    fn actual_player(
        &self,
        handle: &OwnedPlayerHandle,
    ) -> Result<&PlayerState, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if !same_owner(&handle.owner, &state.owner) {
            return Err(PlayerOwnershipError::StaleHandle);
        }
        let player = self
            .players
            .get(&handle.player)
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        let owned = player
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if owned.admission != handle.admission {
            return Err(PlayerOwnershipError::StaleHandle);
        }
        Ok(player)
    }

    fn handle_owned(&self, id: PlayerId) -> Result<OwnedPlayerHandle, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let owned = self
            .players
            .get(&id)
            .and_then(|p| p.owned.as_ref())
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        Ok(OwnedPlayerHandle {
            owner: Arc::downgrade(&state.owner),
            player: id,
            admission: owned.admission,
        })
    }

    /// Borrows the actual native inventory publication generation.
    pub fn owned_player_generation(
        &self,
        handle: &OwnedPlayerHandle,
    ) -> Result<PlayerLocalGeneration, PlayerOwnershipError> {
        Ok(self
            .actual_player(handle)?
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?
            .mutation)
    }

    /// Borrows all original owner slots without cloning or granting write access.
    pub fn owned_player_slots(
        &self,
        handle: &OwnedPlayerHandle,
    ) -> Result<&[ItemStack; crate::NATIVE_PLAYER_SLOT_COUNT], PlayerOwnershipError> {
        Ok(&self
            .actual_player(handle)?
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?
            .slots)
    }

    /// Borrows the one actual cursor stack without a connection-side mirror.
    pub fn owned_player_carried(
        &self,
        handle: &OwnedPlayerHandle,
    ) -> Result<&ItemStack, PlayerOwnershipError> {
        Ok(&self
            .actual_player(handle)?
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?
            .carried)
    }

    /// Reports bounded retained work for the explicit external driver.
    pub fn player_ownership_pending_count(&self) -> usize {
        self.player_ownership
            .as_ref()
            .map_or(0, |s| s.pending.len())
    }

    /// Captures one bounded checked retained record; it conveys no live authority.
    pub fn player_ownership_checkpoint(
        &self,
        query: OwnershipReceiptQuery,
    ) -> Result<crate::OwnershipPendingCheckpoint, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let pending = state
            .pending
            .get(&query.id())
            .ok_or(PlayerOwnershipError::CompletionMismatch)?;
        if pending.ticket.query != query {
            return Err(PlayerOwnershipError::CompletionMismatch);
        }
        let checkpoint = pending.checkpoint();
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    pub(super) fn ownership_quiescent(&self) -> bool {
        self.player_ownership
            .as_ref()
            .is_none_or(|s| self.owned_state_quiescent(s))
    }

    /// The single Stop predicate shared by the Stop apply and `owned_stop_ready`.
    /// It only borrows and scans actual state; it never allocates.
    fn owned_state_quiescent(&self, s: &PlayerOwnershipState) -> bool {
        !s.snapshot_only
            && s.publications.as_ref().is_none_or(|p| p.ledger.is_empty())
            && s.lifecycle.as_ref().is_none_or(|l| l.credits.cells == 0)
            && s.pending.is_empty()
            && s.input.is_empty()
            && s.boundary_remaining == 0
            && self
                .entities
                .as_ref()
                .is_none_or(crate::EntityStore::counter_quiescent)
            && self.players.values().all(|p| {
                p.owned
                    .as_ref()
                    .is_none_or(|o| !o.dirty && o.pending.is_none())
            })
    }

    fn check_player_admission(
        &self,
        id: PlayerId,
        len: usize,
        rejoin: bool,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if let Some(lifecycle) = &state.lifecycle {
            lifecycle.credits.check_cell()?;
        }
        if let Some(existing) = self.players.get(&id) {
            let owned = existing
                .owned
                .as_ref()
                .ok_or(PlayerOwnershipError::AlreadyOwned)?;
            if !rejoin || owned.connected {
                return Err(PlayerOwnershipError::AlreadyOwned);
            }
            if owned.pending.is_some() {
                return Err(PlayerOwnershipError::Reserved);
            }
            if owned.dirty {
                return Err(PlayerOwnershipError::StaleSnapshot);
            }
        } else {
            let bootstraps = state
                .pending
                .values()
                .filter(|p| matches!(p.kind, PendingKind::Bootstrap))
                .count();
            if self.players.len() + bootstraps >= state.limits.players {
                return Err(PlayerOwnershipError::Capacity("player count"));
            }
        }
        if state.pending.values().any(|p| p.player == Some(id)) {
            return Err(PlayerOwnershipError::Reserved);
        }
        let old = self
            .players
            .get(&id)
            .and_then(|p| p.owned.as_ref())
            .map_or(0, |o| o.confirmed.data().len());
        let reserved = state
            .pending
            .values()
            .filter(|p| matches!(p.kind, PendingKind::Bootstrap))
            .map(|p| p.after_player_record.as_ref().map_or(0, |r| r.data().len()))
            .sum::<usize>();
        if len
            > state
                .limits
                .player_bytes
                .saturating_sub(state.player_bytes - old + reserved)
        {
            return Err(PlayerOwnershipError::Capacity("player bytes"));
        }
        Ok(())
    }

    fn restore_owned_player(
        &mut self,
        read: &OwnedPlayerRead,
        rejoin: bool,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        self.validate_binding(&read.binding)?;
        let record = read
            .snapshot
            .record()
            .ok_or(PlayerOwnershipError::Ineligible(
                "restore requires live FCPS3 record",
            ))?;
        let id = read.snapshot.expectation().key().id();
        self.check_player_admission(id, record.data().len(), rejoin)?;
        if rejoin {
            let owned = self
                .players
                .get(&id)
                .and_then(|p| p.owned.as_ref())
                .ok_or(PlayerOwnershipError::StaleHandle)?;
            if owned.expected.stamp() != read.snapshot.stamp()
                || !record_equal(&owned.confirmed, record)
            {
                return Err(PlayerOwnershipError::StaleSnapshot);
            }
        }
        let lifecycle_cell =
            self.staged_lifecycle_cell(self.ownership_state()?.generation.next()?)?;
        let snapshot = NativePlayerSnapshot::try_from_record(record)?;
        self.require_player_region(snapshot.position)?;
        let generation = self.next_owned_generation()?;
        self.reserve_lifecycle_cell(lifecycle_cell, id);
        self.publish_player(
            id,
            snapshot,
            record.clone(),
            read.snapshot.expectation().clone(),
            generation,
            true,
        );
        self.seal_lifecycle_cell(lifecycle_cell, &self.handle_owned(id)?);
        outcome.intents.push(OwnershipIntent::PlayerAdmitted {
            player: self.handle_owned(id)?,
            generation,
        });
        Ok(())
    }

    fn restore_owned_item(
        &mut self,
        read: &OwnedEntityRead,
        wire: EntityId,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        self.validate_binding(&read.binding)?;
        let record = read
            .snapshot
            .record()
            .ok_or(PlayerOwnershipError::Ineligible(
                "restore requires live FCIE record",
            ))?;
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        entities.check_confirmed_replacement(0, record.record().data().len())?;
        let restored = crate::decode_item_entity(read.snapshot.expectation().key(), record)?;
        let (key, kind, transform, stack, properties) = restored.into_parts();
        let proposal = crate::ItemEntitySpawn::new(
            crate::EntitySpawn::new(
                key.world(),
                key.dimension(),
                key.uuid(),
                wire,
                kind,
                transform,
            ),
            stack,
            properties,
        );
        let entities = self
            .entities
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let (prepared, checked) = entities.prepare_owned_spawn(&self.chunks, &proposal)?;
        if !entity_equal(&checked, record) {
            return Err(PlayerOwnershipError::StaleSnapshot);
        }
        // Restore is readonly durable admission, not a new save/delete receipt.
        let id = OwnershipTransferId::from_bytes(key.uuid().to_bytes());
        let publication =
            self.prepare_item_publication(&prepared, crate::ItemPublicationKind::Restore)?;
        self.entities
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .reserve_owned(id, &prepared)?;
        self.reserve_publication(publication.as_ref());
        let EntityPublication::Spawned(item) = self
            .entities
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .publish_owned(id, prepared, read.snapshot.expectation().clone())
        else {
            return Err(PlayerOwnershipError::CompletionMismatch);
        };
        if self
            .player_ownership
            .as_ref()
            .is_some_and(|s| s.item_counters.is_some())
        {
            let tick = self.publication_counter_tick();
            self.entities
                .as_mut()
                .ok_or(PlayerOwnershipError::NotInitialized)?
                .install_confirmed(&item, checked, 1, tick);
        }
        self.publish_item_fact(
            publication,
            &EntityPublication::Spawned(item.clone()),
            crate::ItemPublicationOrigin::restored(read.snapshot.stamp()),
        );
        outcome.intents.push(OwnershipIntent::ItemRestored { item });
        Ok(())
    }

    fn require_player_region(&self, position: Vec3) -> Result<(), PlayerOwnershipError> {
        let transform = crate::EntityTransform::try_new(position, 0.0, 0.0)?;
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        if !entities.owner().region().contains_chunk(
            entities.owner().world(),
            entities.owner().dimension(),
            transform.chunk(),
        ) {
            return Err(PlayerOwnershipError::Ineligible(
                "cross-shard player position",
            ));
        }
        Ok(())
    }

    fn next_owned_generation(&mut self) -> Result<PlayerLocalGeneration, PlayerOwnershipError> {
        self.player_ownership
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .next_generation()
    }

    fn pending_charge(&self, before: usize, after: usize) -> Result<usize, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.pending.len() >= state.limits.pending {
            return Err(PlayerOwnershipError::Capacity("pending count"));
        }
        let charge = before
            .checked_mul(2)
            .and_then(|n| after.checked_mul(5).and_then(|m| n.checked_add(m)))
            .and_then(|n| n.checked_add(512 + 149))
            .ok_or(PlayerOwnershipError::Capacity("pending bytes"))?;
        if charge
            > state
                .limits
                .pending_bytes
                .saturating_sub(state.pending_bytes)
        {
            return Err(PlayerOwnershipError::Capacity("pending bytes"));
        }
        Ok(charge)
    }

    fn validate_pair(
        &self,
        handle: &OwnedPlayerHandle,
        read: &OwnedPairRead,
    ) -> Result<&PlayerState, PlayerOwnershipError> {
        self.validate_binding(&read.binding)?;
        let player = self.actual_player(handle)?;
        let owned = player
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if owned.pending.is_some() {
            return Err(PlayerOwnershipError::Reserved);
        }
        if !owned.connected {
            return Err(PlayerOwnershipError::Ineligible("detached player"));
        }
        let p = read.snapshot.player();
        if p.expectation().key().id() != handle.player
            || p.stamp() != owned.expected.stamp()
            || p.record()
                .is_none_or(|r| !record_equal(r, &owned.confirmed))
        {
            return Err(PlayerOwnershipError::StaleSnapshot);
        }
        Ok(player)
    }

    fn player_plan(player: &PlayerState) -> Result<InventoryPlan<'_>, PlayerOwnershipError> {
        let owned = player
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        InventoryPlan::new(&owned.slots, &owned.carried)
    }

    fn check_player_post_bytes(
        &self,
        owned: &OwnedPlayerInventory,
        after: usize,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let other = state
            .player_bytes
            .checked_sub(owned.confirmed.data().len())
            .ok_or(PlayerOwnershipError::Malformed("player accounting"))?;
        let additional = state
            .pending
            .values()
            .filter_map(|p| p.after_player_record.as_ref())
            .map(|r| r.data().len())
            .sum::<usize>();
        if after > state.limits.player_bytes.saturating_sub(other + additional) {
            return Err(PlayerOwnershipError::Capacity("player bytes"));
        }
        Ok(())
    }

    fn plan_owned_player_write(
        &self,
        handle: &OwnedPlayerHandle,
        click: Option<(PlayerLocalGeneration, usize, NormalSlotButton)>,
    ) -> Result<Candidate, PlayerOwnershipError> {
        let player = self.actual_player(handle)?;
        let owned = player
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if owned.pending.is_some() {
            return Err(PlayerOwnershipError::Reserved);
        }
        let mut plan = Self::player_plan(player)?;
        let kind = if let Some((generation, slot, button)) = click {
            if !owned.connected || player.game_mode == GameMode::Spectator {
                return Err(PlayerOwnershipError::Ineligible("inactive/spectator click"));
            }
            if generation != owned.mutation {
                return Err(PlayerOwnershipError::StaleGeneration);
            }
            plan.click(slot, button)?;
            PendingKind::Click
        } else {
            PendingKind::Autosave
        };
        let after_len = plan.encoded_len()?;
        self.check_player_post_bytes(owned, after_len)?;
        let charge = self.pending_charge(owned.confirmed.data().len(), after_len)?;
        let snapshot = plan.materialize(
            player.position,
            [player.yaw, player.pitch],
            player.game_mode,
            owned.selected,
        )?;
        let record = snapshot.try_to_record()?;
        Ok(Candidate {
            collector: None,
            kind,
            player_expected: Some(owned.expected.clone()),
            entity_expected: None,
            before_player: Some(owned.confirmed.clone()),
            before_entity: None,
            after_player: Some(snapshot),
            after_player_record: Some(record),
            after_entity: None,
            after_entity_record: None,
            local_before: Some(owned.mutation),
            moved: 0,
            original_count: 0,
            stat_item: None,
            animate: false,
            charge,
        })
    }

    fn plan_owned_drop(
        &self,
        handle: &OwnedPlayerHandle,
        read: &OwnedPairRead,
        selection: DropSelection,
        wire: EntityId,
        seed: i64,
    ) -> Result<Candidate, PlayerOwnershipError> {
        let player = self.validate_pair(handle, read)?;
        let owned = player
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if player.game_mode == GameMode::Spectator {
            return Err(PlayerOwnershipError::Ineligible("spectator drop"));
        }
        let entity = read.snapshot.entity();
        if !entity.stamp().is_fresh() || entity.record().is_some() {
            return Err(PlayerOwnershipError::StaleSnapshot);
        }
        let mut plan = Self::player_plan(player)?;
        let (source, moved) =
            plan.drop_selected(owned.selected, selection == DropSelection::All)?;
        let after_len = plan.encoded_len()?;
        self.check_player_post_bytes(owned, after_len)?;
        let entity_len = crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + persisted_stack_size(source)?;
        let charge = self.pending_charge(owned.confirmed.data().len(), after_len + entity_len)?;
        let dropped = plan_split(source, moved)?.into_stacks().1;
        let snapshot = plan.materialize(
            player.position,
            [player.yaw, player.pitch],
            player.game_mode,
            owned.selected,
        )?;
        let record = snapshot.try_to_record()?;
        let proposal = standing_drop(
            self.ownership_state()?.binding.generation().key(),
            handle.player,
            entity.expectation().key().uuid(),
            wire,
            player.position,
            [player.yaw, player.pitch],
            seed,
            dropped,
        )?;
        let (prepared, entity_record) = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .prepare_owned_spawn(&self.chunks, &proposal)?;
        Ok(Candidate {
            collector: None,
            kind: PendingKind::Drop,
            player_expected: Some(read.snapshot.player().expectation().clone()),
            entity_expected: Some(entity.expectation().clone()),
            before_player: Some(owned.confirmed.clone()),
            before_entity: None,
            after_player: Some(snapshot),
            after_player_record: Some(record),
            after_entity: Some(prepared),
            after_entity_record: Some(entity_record),
            local_before: Some(owned.mutation),
            moved,
            original_count: moved,
            stat_item: source.item(),
            animate: false,
            charge,
        })
    }

    fn plan_owned_pickup(
        &self,
        handle: &OwnedPlayerHandle,
        item: &crate::EntityHandle,
        read: &OwnedPairRead,
        animate: bool,
    ) -> Result<Candidate, PlayerOwnershipError> {
        let player = self.validate_pair(handle, read)?;
        // Original-admission pin admission precedes every candidate/body copy.
        self.preflight_take_pin(handle, animate)?;
        let owned = player
            .owned
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if player.game_mode == GameMode::Spectator {
            return Err(PlayerOwnershipError::Ineligible("spectator pickup"));
        }
        let entities = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let base = entities.get(item)?;
        let actual = entities.item_state(item)?;
        let expected = entities.owned_expectation(item)?;
        let read_entity = read.snapshot.entity();
        if expected.key() != read_entity.expectation().key()
            || expected.stamp() != read_entity.stamp()
        {
            return Err(PlayerOwnershipError::StaleSnapshot);
        }
        let legacy_capture;
        let (key, before) = if self.ownership_state()?.item_counters.is_some() {
            let durable = entities.item_durability(item)?;
            if durable.due {
                return Err(PlayerOwnershipError::Ineligible("item despawn due"));
            }
            (expected.key(), &durable.confirmed)
        } else {
            legacy_capture = entities.capture_item(item)?;
            (legacy_capture.0, &legacy_capture.1)
        };
        if key != read_entity.expectation().key()
            || read_entity
                .record()
                .is_none_or(|r| !entity_equal(r, before))
        {
            return Err(PlayerOwnershipError::StaleSnapshot);
        }
        let properties = actual.properties();
        let BorrowedPickupShape {
            plan,
            moved,
            remainder,
            after_len,
        } = borrowed_pickup_shape(
            &owned.slots,
            &owned.carried,
            owned.selected,
            handle.player,
            player.game_mode,
            player.position,
            actual.stack(),
            properties.counters(),
            properties.owner_target(),
            base.transform().position(),
        )?;
        self.check_player_post_bytes(owned, after_len)?;
        let after_entity_len = if remainder == 0 {
            0
        } else {
            before.record().data().len()
        };
        let charge = self.pending_charge(
            owned.confirmed.data().len() + before.record().data().len(),
            after_len + after_entity_len,
        )?;
        let stack = if remainder == 0 {
            None
        } else {
            Some(plan_split(actual.stack(), remainder)?.into_stacks().1)
        };
        let (prepared, after_entity) =
            entities.prepare_owned_pickup(&self.chunks, item, stack.as_ref())?;
        let snapshot = plan.materialize(
            player.position,
            [player.yaw, player.pitch],
            player.game_mode,
            owned.selected,
        )?;
        let record = snapshot.try_to_record()?;
        Ok(Candidate {
            collector: Some(self.capture_item_collector(handle)?),
            kind: PendingKind::Pickup,
            player_expected: Some(read.snapshot.player().expectation().clone()),
            entity_expected: Some(expected.clone()),
            before_player: Some(owned.confirmed.clone()),
            before_entity: Some(before.clone()),
            after_player: Some(snapshot),
            after_player_record: Some(record),
            after_entity: Some(prepared),
            after_entity_record: after_entity,
            local_before: Some(owned.mutation),
            moved,
            original_count: actual.stack().count(),
            stat_item: actual.stack().item(),
            animate,
            charge,
        })
    }

    fn install_candidate(
        &mut self,
        id: OwnershipTransferId,
        candidate: Candidate,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self.ownership_state()?;
        if state.pending.contains_key(&id) {
            return Err(PlayerOwnershipError::Reserved);
        }
        let generation = state.generation.next()?;
        let lifecycle_cell = if matches!(candidate.kind, PendingKind::Bootstrap) {
            self.staged_lifecycle_cell(generation)?
        } else {
            None
        };
        self.validate_candidate_charge(&candidate)?;
        let publication = self.prepare_candidate_publication(&candidate)?;
        let (item_generation, counters) = self.prepare_counter_candidate(&candidate)?;
        let request = candidate.request(id)?;
        let Candidate {
            collector: _,
            kind,
            player_expected,
            entity_expected,
            before_player,
            before_entity,
            after_player,
            after_player_record,
            after_entity,
            after_entity_record,
            local_before,
            moved,
            original_count,
            stat_item,
            animate,
            charge,
        } = candidate;
        let state = self.ownership_state()?;
        let player = player_expected.as_ref().map(|p| p.key().id());
        let ticket = CompletionTicket {
            binding: state.binding.clone(),
            owner: Arc::downgrade(&state.owner),
            generation,
            query: request.query(),
            keys: WorkKeys {
                player: player_expected.as_ref().map(OwnedPlayerExpectation::key),
                entity: entity_expected.as_ref().map(OwnedEntityExpectation::key),
            },
        };
        if let Some(prepared) = &after_entity {
            self.entities
                .as_mut()
                .ok_or(PlayerOwnershipError::NotInitialized)?
                .reserve_owned(id, prepared)?;
        }
        if let Some(player) = player {
            if let Some(owned) = self.players.get_mut(&player).and_then(|p| p.owned.as_mut()) {
                // A newly admitted pending supersedes any earlier detached save proof.
                (owned.pending, owned.saved_after_detach) = (Some(id), None);
            }
        }
        if matches!(kind, PendingKind::Bootstrap) {
            let Some(player) = player else {
                panic!("bootstrap player disappeared");
            };
            self.reserve_lifecycle_cell(lifecycle_cell, player);
        }
        self.reserve_publication(publication.as_ref());
        let buffer = Arc::new(());
        let pending = PendingPlayerMutation {
            lifecycle_cell,
            publication,
            buffer: Arc::downgrade(&buffer),
            kind,
            ticket: ticket.clone(),
            request: request.clone(),
            player,
            before_player,
            before_entity,
            player_expected,
            entity_expected,
            after_player,
            after_player_record,
            after_entity,
            after_entity_record,
            local_before,
            item_generation,
            counters,
            status: OwnershipPendingStatus::InFlight,
            charge,
            moved,
            original_count,
            stat_item,
            animate,
        };
        if let Some(state) = self.player_ownership.as_mut() {
            state.generation = generation;
            state.pending_bytes += charge;
            state.pending.insert(id, pending);
        }
        outcome.work.push(PlayerOwnershipStorageWork {
            buffer,
            ticket,
            request: Some(request),
        });
        Ok(())
    }

    fn validate_candidate_charge(&self, candidate: &Candidate) -> Result<(), PlayerOwnershipError> {
        let actual = self.pending_charge(
            bytes(
                candidate.before_player.as_ref(),
                candidate.before_entity.as_ref(),
            ),
            bytes(
                candidate.after_player_record.as_ref(),
                candidate.after_entity_record.as_ref(),
            ),
        )?;
        if candidate.charge != actual {
            return Err(PlayerOwnershipError::Malformed(
                "planned encoded charge differs",
            ));
        }
        Ok(())
    }

    fn dispatch_retained(
        &mut self,
        query: OwnershipReceiptQuery,
        clarify: bool,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        let state = self
            .player_ownership
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let pending = state
            .pending
            .get_mut(&query.id())
            .ok_or(PlayerOwnershipError::CompletionMismatch)?;
        if pending.ticket.query != query {
            return Err(PlayerOwnershipError::CompletionMismatch);
        }
        if pending.buffer.strong_count() != 0 {
            return Err(PlayerOwnershipError::Reserved);
        }
        if !clarify && pending.status != OwnershipPendingStatus::Prepared {
            return Err(PlayerOwnershipError::UnprovenWorkerLifetime);
        }
        let generation = state.generation.next()?;
        let request = if clarify {
            None
        } else {
            Some(pending.request.clone())
        };
        state.generation = generation;
        pending.ticket.generation = generation;
        pending.status = OwnershipPendingStatus::InFlight;
        let buffer = Arc::new(());
        pending.buffer = Arc::downgrade(&buffer);
        outcome.work.push(PlayerOwnershipStorageWork {
            buffer,
            ticket: pending.ticket.clone(),
            request,
        });
        Ok(())
    }

    fn validate_completion<'a>(
        &'a self,
        completion: &PlayerOwnershipCompletion,
    ) -> Result<&'a PendingPlayerMutation, PlayerOwnershipError> {
        let state = self.ownership_state()?;
        let ticket = &completion.ticket;
        if !same_owner(&ticket.owner, &state.owner) || !state.binding.matches(&ticket.binding) {
            return Err(PlayerOwnershipError::CompletionMismatch);
        }
        let pending = state
            .pending
            .get(&ticket.query.id())
            .ok_or(PlayerOwnershipError::CompletionMismatch)?;
        if pending.ticket.query != ticket.query
            || pending.ticket.generation != ticket.generation
            || !std::sync::Weak::ptr_eq(&pending.buffer, &Arc::downgrade(&completion.buffer))
            || pending.status != OwnershipPendingStatus::InFlight
        {
            return Err(PlayerOwnershipError::CompletionMismatch);
        }
        if let Some(id) = pending.player {
            if !matches!(pending.kind, PendingKind::Bootstrap) {
                let owned = self
                    .players
                    .get(&id)
                    .and_then(|p| p.owned.as_ref())
                    .ok_or(PlayerOwnershipError::CompletionMismatch)?;
                if owned.pending != Some(ticket.query.id())
                    || Some(owned.mutation) != pending.local_before
                {
                    return Err(PlayerOwnershipError::CompletionMismatch);
                }
            }
        }
        if let Some(prepared) = &pending.after_entity {
            self.entities
                .as_ref()
                .ok_or(PlayerOwnershipError::NotInitialized)?
                .check_reserved(ticket.query.id(), prepared)?;
        }
        Ok(pending)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one joint receipt/current image check preserves atomic ownership authority"
    )]
    fn confirm_current(
        pending: &PendingPlayerMutation,
        receipt: &OwnershipReceipt,
        current: &CurrentResources,
    ) -> Result<
        (
            Option<OwnedPlayerExpectation>,
            Option<OwnedEntityExpectation>,
        ),
        PlayerOwnershipError,
    > {
        let binding = pending.ticket.binding.generation();
        if receipt.query() != pending.ticket.query
            || receipt.scope() != binding.key()
            || receipt.generation_identity() != binding.identity()
        {
            return Err(PlayerOwnershipError::CompletionMismatch);
        }
        let (player, entity) = match current {
            CurrentResources::Pair(pair) => (Some(pair.player()), Some(pair.entity())),
            CurrentResources::Player(player) => (Some(player), None),
            CurrentResources::Entity(entity) => (None, Some(entity)),
        };
        let player_expected = if let Some(expected) = &pending.player_expected {
            let (id, before, after) = receipt
                .player()
                .ok_or(PlayerOwnershipError::CompletionMismatch)?;
            let changed = pending
                .before_player
                .as_ref()
                .zip(pending.after_player_record.as_ref())
                .is_none_or(|(a, b)| !record_equal(a, b));
            if id != expected.key().id()
                || before != expected.stamp()
                || !changed_stamp(before, after, true, changed)
            {
                return Err(PlayerOwnershipError::CompletionMismatch);
            }
            let current = player.ok_or(PlayerOwnershipError::CompletionMismatch)?;
            if current.expectation().key() != expected.key()
                || current.expectation().generation_identity() != binding.identity()
                || current.stamp() != after
                || current
                    .record()
                    .zip(pending.after_player_record.as_ref())
                    .is_none_or(|(a, b)| !record_equal(a, b))
            {
                return Err(PlayerOwnershipError::Superseded);
            }
            Some(current.expectation().clone())
        } else {
            if receipt.player().is_some() || player.is_some() {
                return Err(PlayerOwnershipError::CompletionMismatch);
            }
            None
        };
        let entity_expected = if let Some(expected) = &pending.entity_expected {
            let (id, before, after) = receipt
                .entity()
                .ok_or(PlayerOwnershipError::CompletionMismatch)?;
            if id != expected.key().uuid()
                || before != expected.stamp()
                || !changed_stamp(
                    before,
                    after,
                    pending.after_entity_record.is_some(),
                    pending
                        .before_entity
                        .as_ref()
                        .zip(pending.after_entity_record.as_ref())
                        .is_none_or(|(a, b)| !entity_equal(a, b)),
                )
                || receipt.before_entity_chunk()
                    != pending
                        .before_entity
                        .as_ref()
                        .map(PersistentEntityRecord::chunk)
                || receipt.after_entity_chunk()
                    != pending
                        .after_entity_record
                        .as_ref()
                        .map(PersistentEntityRecord::chunk)
            {
                return Err(PlayerOwnershipError::CompletionMismatch);
            }
            let current = entity.ok_or(PlayerOwnershipError::CompletionMismatch)?;
            let same = match (current.record(), &pending.after_entity_record) {
                (Some(a), Some(b)) => entity_equal(a, b),
                (None, None) => true,
                _ => false,
            };
            if current.expectation().key() != expected.key()
                || current.expectation().generation_identity() != binding.identity()
                || current.stamp() != after
                || !same
            {
                return Err(PlayerOwnershipError::Superseded);
            }
            Some(current.expectation().clone())
        } else {
            if receipt.entity().is_some() || entity.is_some() {
                return Err(PlayerOwnershipError::CompletionMismatch);
            }
            None
        };
        check_receipt_epochs(
            receipt,
            binding,
            pending.before_entity.as_ref(),
            pending.after_entity_record.as_ref(),
        )?;
        Ok((player_expected, entity_expected))
    }

    fn set_pending_status(&mut self, id: OwnershipTransferId, status: OwnershipPendingStatus) {
        if let Some(pending) = self
            .player_ownership
            .as_mut()
            .and_then(|s| s.pending.get_mut(&id))
        {
            pending.status = status;
        }
    }

    fn complete_owned(
        &mut self,
        completion: &PlayerOwnershipCompletion,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        let pending = self.validate_completion(completion)?;
        let id = completion.query().id();
        match &completion.evidence {
            CompletionEvidence::Unresolved(source) => {
                let reason = PlayerOwnershipError::Storage(source.clone());
                self.set_pending_status(id, OwnershipPendingStatus::Uncertain);
                Err(reason)
            }
            CompletionEvidence::ReceiptAbsent => {
                self.set_pending_status(id, OwnershipPendingStatus::Uncertain);
                Err(PlayerOwnershipError::UnprovenWorkerLifetime)
            }
            CompletionEvidence::Refused(source) => {
                let reason = PlayerOwnershipError::Storage(source.clone());
                if matches!(
                    source,
                    OwnershipError::Busy | OwnershipError::HistoryCapacity { .. }
                ) {
                    self.set_pending_status(id, OwnershipPendingStatus::Prepared);
                } else if self.ownership_state()?.item_counters.is_some()
                    && matches!(source, OwnershipError::CasConflict { .. })
                {
                    self.quarantine_counter_conflict(id);
                    self.set_pending_status(id, OwnershipPendingStatus::Quarantined);
                } else {
                    self.preflight_deferred_for(Some(id), 0)?;
                    self.release_pending(id, outcome)?;
                    self.apply_deferred(outcome)?;
                }
                Err(reason)
            }
            CompletionEvidence::Confirmed {
                current: Err(source),
                ..
            } => {
                let reason = PlayerOwnershipError::Storage(source.clone());
                self.set_pending_status(id, OwnershipPendingStatus::NeedsRefresh);
                Err(reason)
            }
            CompletionEvidence::Confirmed {
                receipt,
                current: Ok(current),
            } => {
                let (player_expected, entity_expected) =
                    match Self::confirm_current(pending, receipt, current) {
                        Ok(expected) => expected,
                        Err(reason) => {
                            self.set_pending_status(id, OwnershipPendingStatus::Quarantined);
                            return Err(reason);
                        }
                    };
                self.publish_confirmed(
                    completion.query(),
                    player_expected,
                    entity_expected,
                    outcome,
                )
            }
        }
    }

    fn publish_confirmed(
        &mut self,
        query: OwnershipReceiptQuery,
        player_expected: Option<OwnedPlayerExpectation>,
        entity_expected: Option<OwnedEntityExpectation>,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        let id = query.id();
        let generation = self.ownership_state()?.generation.next()?;
        self.preflight_deferred_for(Some(id), 1)?;
        self.preflight_counter_completion(id, true)?;
        let retained = self
            .ownership_state()?
            .pending
            .get(&id)
            .ok_or(PlayerOwnershipError::CompletionMismatch)?;
        self.check_pending_publication(retained)?;
        let mut pending = self.remove_pending(id)?;
        let take_wire = pending
            .after_entity
            .as_ref()
            .and_then(|prepared| match prepared {
                PreparedOwnedEntity::Replace { handle, .. } => self
                    .entities
                    .as_ref()
                    .and_then(|e| e.get(handle).ok())
                    .map(|b| b.wire_id()),
                PreparedOwnedEntity::Remove { base, .. } => Some(base.wire_id()),
                PreparedOwnedEntity::Spawn { base, .. } => Some(base.wire_id),
            });
        if let (Some(player), Some(snapshot), Some(record), Some(expected)) = (
            pending.player,
            pending.after_player.take(),
            pending.after_player_record.take(),
            player_expected,
        ) {
            let admitted = matches!(pending.kind, PendingKind::Bootstrap);
            self.publish_player(player, snapshot, record, expected, generation, admitted);
            if matches!(pending.kind, PendingKind::Autosave) {
                self.record_detached_save(player, id);
            }
            let handle = self.handle_owned(player)?;
            if admitted {
                self.seal_lifecycle_cell(pending.lifecycle_cell, &handle);
            }
            outcome.intents.push(if admitted {
                OwnershipIntent::PlayerAdmitted {
                    player: handle,
                    generation,
                }
            } else {
                OwnershipIntent::PlayerChanged {
                    player: handle,
                    generation,
                }
            });
        }
        if let (Some(prepared), Some(expected)) = (pending.after_entity.take(), entity_expected) {
            let origin = crate::ItemPublicationOrigin::confirmed(query, expected.stamp());
            let publication = self
                .entities
                .as_mut()
                .ok_or(PlayerOwnershipError::NotInitialized)?
                .publish_owned(id, prepared, expected);
            self.publish_counter_confirmation(&publication, &mut pending);
            self.publish_item_fact(pending.publication.take(), &publication, origin);
            Self::publish_entity_intent(query, publication, outcome);
        }
        if let Some(state) = self.player_ownership.as_mut() {
            state.generation = generation;
        }
        if let (Some(player), Some(item)) = (pending.player, pending.stat_item) {
            let dropped = matches!(pending.kind, PendingKind::Drop);
            outcome.intents.push(OwnershipIntent::Stat {
                query,
                player,
                item,
                count: pending.original_count,
                dropped,
            });
            if pending.animate && !dropped {
                if let Some(item) = take_wire {
                    outcome.intents.push(OwnershipIntent::Take {
                        query,
                        player,
                        item,
                        count: pending.original_count,
                        moved: pending.moved,
                    });
                }
            }
        }
        self.apply_deferred(outcome)?;
        Ok(())
    }

    fn publish_entity_intent(
        query: OwnershipReceiptQuery,
        publication: EntityPublication,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) {
        match publication {
            EntityPublication::Spawned(item) => {
                outcome
                    .intents
                    .push(OwnershipIntent::ItemSpawned { query, item });
            }
            EntityPublication::Changed(item) => {
                outcome
                    .intents
                    .push(OwnershipIntent::ItemChanged { query, item });
            }
            EntityPublication::Removed(base) => {
                outcome.intents.push(OwnershipIntent::ItemRemoved {
                    query,
                    uuid: base.uuid(),
                    wire_id: base.wire_id(),
                });
            }
        }
    }

    fn remove_pending(
        &mut self,
        id: OwnershipTransferId,
    ) -> Result<PendingPlayerMutation, PlayerOwnershipError> {
        let state = self
            .player_ownership
            .as_mut()
            .ok_or(PlayerOwnershipError::NotInitialized)?;
        let pending = state
            .pending
            .remove(&id)
            .ok_or(PlayerOwnershipError::CompletionMismatch)?;
        state.pending_bytes -= pending.charge;
        Ok(pending)
    }

    fn release_pending(
        &mut self,
        id: OwnershipTransferId,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        self.preflight_counter_completion(id, false)?;
        let mut pending = self.remove_pending(id)?;
        self.release_bootstrap_lifecycle(&pending);
        self.release_counter_projection(&pending);
        self.cancel_publication(pending.publication.take());
        if matches!(pending.kind, PendingKind::SpawnItem) {
            if let Some(PreparedOwnedEntity::Spawn { base, item, .. }) = pending.after_entity.take()
            {
                let (stack, properties) = item.into_parts();
                outcome.intents.push(OwnershipIntent::ItemSpawnRefused {
                    query: pending.ticket.query,
                    proposal: crate::ItemEntitySpawn::new(base, stack, properties),
                });
            }
        }
        if let Some(player) = pending.player {
            if let Some(owned) = self.players.get_mut(&player).and_then(|p| p.owned.as_mut()) {
                owned.pending = None;
            }
        }
        if let Some(entities) = &mut self.entities {
            entities.release_owned(id);
        }
        Ok(())
    }

    fn publish_player(
        &mut self,
        id: PlayerId,
        snapshot: NativePlayerSnapshot,
        record: PlayerRecord,
        expected: OwnedPlayerExpectation,
        generation: PlayerLocalGeneration,
        admitted: bool,
    ) {
        let old = self.players.remove(&id);
        let old_len = old
            .as_ref()
            .and_then(|p| p.owned.as_ref())
            .map_or(0, |o| o.confirmed.data().len());
        let previous = old.and_then(|p| p.owned);
        let admission = if admitted {
            generation
        } else {
            previous.as_ref().map_or(generation, |o| o.admission)
        };
        let connected = previous.as_ref().is_none_or(|o| o.connected) || admitted;
        // Only a new admission resets its own lifecycle acknowledgement.
        let lifecycle_acked = !admitted && previous.as_ref().is_some_and(|o| o.lifecycle_acked);
        let mut owned = OwnedPlayerInventory {
            slots: snapshot.slots,
            carried: snapshot.carried,
            selected: snapshot.selected,
            expected,
            confirmed: record,
            admission,
            mutation: generation,
            connected,
            dirty: false,
            pending: None,
            deferred_position: None,
            deferred_look: [None; 2],
            deferred_mode: None,
            deferred_selected: None,
            saved_after_detach: None,
            lifecycle_acked,
        };
        if let Some(previous) = previous {
            owned.deferred_position = previous.deferred_position;
            owned.deferred_look = previous.deferred_look;
            owned.deferred_mode = previous.deferred_mode;
            owned.deferred_selected = previous.deferred_selected;
        }
        if let Some(state) = self.player_ownership.as_mut() {
            state.player_bytes = state.player_bytes - old_len + owned.confirmed.data().len();
        }
        self.players.insert(
            id,
            PlayerState {
                position: snapshot.position,
                yaw: snapshot.look[0],
                pitch: snapshot.look[1],
                game_mode: snapshot.game_mode,
                owned: Some(owned),
                survival: None,
            },
        );
    }

    fn move_owned_player(
        &mut self,
        handle: &OwnedPlayerHandle,
        position: Option<Vec3>,
        look: [Option<f32>; 2],
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        self.actual_player(handle)?;
        if self
            .actual_player(handle)?
            .owned
            .as_ref()
            .is_some_and(|o| o.pending.is_none())
        {
            self.ownership_state()?.generation.next()?;
        }
        if position.is_some_and(|p| !super::is_valid_position(p))
            || look.iter().flatten().any(|v| !v.is_finite())
        {
            return Err(PlayerOwnershipError::Malformed("movement/look"));
        }
        if let Some(position) = position {
            self.require_player_region(position)?;
        }
        let owned = self
            .players
            .get_mut(&handle.player)
            .and_then(|p| p.owned.as_mut())
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if !owned.connected {
            return Err(PlayerOwnershipError::Ineligible("detached movement"));
        }
        if let Some(p) = position {
            owned.deferred_position = Some(p);
        }
        for (dest, value) in owned.deferred_look.iter_mut().zip(look) {
            if value.is_some() {
                *dest = value;
            }
        }
        self.apply_deferred(outcome)
    }

    fn mode_owned_player(
        &mut self,
        handle: &OwnedPlayerHandle,
        mode: GameMode,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        self.actual_player(handle)?;
        if self
            .actual_player(handle)?
            .owned
            .as_ref()
            .is_some_and(|o| o.pending.is_none())
        {
            self.ownership_state()?.generation.next()?;
        }
        let owned = self
            .players
            .get_mut(&handle.player)
            .and_then(|p| p.owned.as_mut())
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if !owned.connected {
            return Err(PlayerOwnershipError::Ineligible("detached mode change"));
        }
        owned.deferred_mode = Some(mode);
        self.apply_deferred(outcome)
    }

    fn select_owned_player(
        &mut self,
        handle: &OwnedPlayerHandle,
        selected: u8,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        self.actual_player(handle)?;
        if self
            .actual_player(handle)?
            .owned
            .as_ref()
            .is_some_and(|o| o.pending.is_none())
        {
            self.ownership_state()?.generation.next()?;
        }
        if selected > 8 {
            return Err(PlayerOwnershipError::Malformed("selection"));
        }
        let owned = self
            .players
            .get_mut(&handle.player)
            .and_then(|p| p.owned.as_mut())
            .ok_or(PlayerOwnershipError::StaleHandle)?;
        if !owned.connected {
            return Err(PlayerOwnershipError::Ineligible("detached selection"));
        }
        owned.deferred_selected = Some(selected);
        self.apply_deferred(outcome)
    }

    fn apply_deferred(
        &mut self,
        outcome: &mut PlayerOwnershipTickOutcome,
    ) -> Result<(), PlayerOwnershipError> {
        self.preflight_deferred_for(None, 0)?;
        let eligible: Vec<_> = self
            .players
            .iter()
            .filter_map(|(&id, p)| {
                p.owned
                    .as_ref()
                    .filter(|o| {
                        o.pending.is_none()
                            && (o.deferred_position.is_some()
                                || o.deferred_look.iter().any(Option::is_some)
                                || o.deferred_mode.is_some()
                                || o.deferred_selected.is_some())
                    })
                    .map(|_| id)
            })
            .collect();
        for id in eligible {
            let generation = self.next_owned_generation()?;
            let player = self
                .players
                .get_mut(&id)
                .ok_or(PlayerOwnershipError::StaleHandle)?;
            let owned = player
                .owned
                .as_mut()
                .ok_or(PlayerOwnershipError::StaleHandle)?;
            if let Some(position) = owned.deferred_position.take() {
                player.position = position;
            }
            if let Some(yaw) = owned.deferred_look[0].take() {
                player.yaw = yaw;
            }
            if let Some(pitch) = owned.deferred_look[1].take() {
                player.pitch = pitch;
            }
            if let Some(mode) = owned.deferred_mode.take() {
                player.game_mode = mode;
            }
            if let Some(selected) = owned.deferred_selected.take() {
                owned.selected = selected;
            }
            owned.mutation = generation;
            owned.dirty = true;
            owned.saved_after_detach = None;
            outcome.intents.push(OwnershipIntent::PlayerChanged {
                player: self.handle_owned(id)?,
                generation,
            });
        }
        Ok(())
    }

    fn preflight_deferred_for(
        &self,
        completing: Option<OwnershipTransferId>,
        additional: u64,
    ) -> Result<(), PlayerOwnershipError> {
        let count = self
            .players
            .values()
            .filter_map(|p| p.owned.as_ref())
            .filter(|o| {
                (o.pending.is_none() || (completing.is_some() && o.pending == completing))
                    && (o.deferred_position.is_some()
                        || o.deferred_look.iter().any(Option::is_some)
                        || o.deferred_mode.is_some()
                        || o.deferred_selected.is_some())
            })
            .count() as u64;
        self.ownership_state()?
            .generation
            .get()
            .checked_add(count)
            .and_then(|n| n.checked_add(additional))
            .ok_or(PlayerOwnershipError::GenerationExhausted)?;
        Ok(())
    }

    /// Acquires a chunk through the bound map while retaining every item reservation.
    /// No mutable chunk-map replacement authority escapes this checked call.
    pub async fn acquire_owned_chunk(
        &mut self,
        store: &dyn ferrumc_storage::WorldStore,
        generator: &dyn ferrumc_world::ChunkGenerator,
        pos: ChunkPos,
        ticket: crate::ChunkTicket,
    ) -> Result<crate::ChunkAcquired, PlayerOwnershipError> {
        self.ownership_state()?;
        let owner = self
            .entities
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)?
            .owner();
        if !owner
            .region()
            .contains_chunk(owner.world(), owner.dimension(), pos)
        {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        Ok(self.chunks.acquire(store, generator, pos, ticket).await?)
    }

    /// Releases one ticket; unloaded item registrations and pending authority remain.
    pub fn release_owned_chunk(
        &mut self,
        pos: ChunkPos,
        ticket: crate::ChunkTicket,
    ) -> Result<crate::TicketRelease, PlayerOwnershipError> {
        self.ownership_state()?;
        let release = self.chunks.release(pos, ticket);
        if !self.chunks.is_loaded(pos) {
            if let Some(e) = &mut self.entities {
                e.mark_counter_flush(Some(pos));
            }
        }
        Ok(release)
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "all native drop inputs are captured once in a concrete postimage"
)]
fn standing_drop(
    scope: ferrumc_storage::GenerationKey,
    player: PlayerId,
    uuid: EntityUuid,
    wire: EntityId,
    position: Vec3,
    look: [f32; 2],
    seed: i64,
    stack: ItemStack,
) -> Result<crate::ItemEntitySpawn, PlayerOwnershipError> {
    let (base, properties) = standing_drop_parts(scope, player, uuid, wire, position, look, seed)?;
    Ok(crate::ItemEntitySpawn::new(base, stack, properties))
}

#[allow(
    clippy::too_many_arguments,
    reason = "pure native drop facts are shared by readonly planning and publication"
)]
fn standing_drop_parts(
    scope: ferrumc_storage::GenerationKey,
    player: PlayerId,
    uuid: EntityUuid,
    wire: EntityId,
    position: Vec3,
    look: [f32; 2],
    seed: i64,
) -> Result<(crate::EntitySpawn, crate::ItemEntityProperties), PlayerOwnershipError> {
    use ferrumc_world::random::WorldgenRandomSource;
    let mut constructor =
        WorldgenRandomSource::legacy(ferrumc_world::WorldSeed::new(seed.wrapping_add(1)));
    let bob = constructor.next_float() * std::f32::consts::PI * 2.0;
    let rotation = constructor.next_float() * 360.0;
    // Constructor motion is overwritten by Player.drop, but consumes its own RNG.
    constructor.next_double();
    constructor.next_double();
    let mut random = WorldgenRandomSource::legacy(ferrumc_world::WorldSeed::new(seed));
    let pitch = look[1] * (std::f64::consts::PI / 180.0) as f32;
    let yaw = look[0] * (std::f64::consts::PI / 180.0) as f32;
    let angle = random.next_float() * (std::f64::consts::PI * 2.0) as f32;
    let radius = 0.02_f32 * random.next_float();
    let motion = Vec3::new(
        f64::from(-mth(yaw, false) * mth(pitch, true) * 0.3_f32)
            + f64::from(angle).cos() * f64::from(radius),
        f64::from(
            -mth(pitch, false) * 0.3_f32
                + 0.1_f32
                + (random.next_float() - random.next_float()) * 0.1_f32,
        ),
        f64::from(mth(yaw, true) * mth(pitch, true) * 0.3_f32)
            + f64::from(angle).sin() * f64::from(radius),
    );
    let position = Vec3::new(
        position.x,
        position.y + f64::from(1.62_f32) - f64::from(0.3_f32),
        position.z,
    );
    let transform = crate::EntityTransform::try_new(position, rotation, 0.0)?;
    let kind = ferrumc_registry::entity_type::EntityTypeId::from_name("minecraft:item")
        .ok_or(PlayerOwnershipError::Ineligible("missing item entity type"))?;
    let base = crate::EntitySpawn::new(
        scope.world(),
        scope.dimension(),
        uuid,
        wire,
        kind,
        transform,
    );
    let properties = crate::ItemEntityProperties::try_new(
        motion,
        bob,
        crate::ItemEntityCounters::new(0, 40, 5),
        None,
        Some(EntityUuid::from_bytes(*player.as_uuid().as_bytes())),
    )?;
    Ok((base, properties))
}

fn mth(value: f32, cos: bool) -> f32 {
    // Match the native float index/truncation and LUT sample. The full pose /
    // floating-point differential acceptance remains an explicitly separate gate.
    let scaled = value * 10430.378_f32 + if cos { 16384.0_f32 } else { 0.0 };
    let index = f64::from(scaled as i32 & 65535);
    (index * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32
}
