//! Scalar admission credits and original-admission publication pins.

use std::collections::TryReserveError;
use std::mem::size_of;
use std::num::NonZeroUsize;

use ferrumc_core::{GameMode, PlayerId};
use ferrumc_math::Vec3;

use crate::{
    GenerationBinding, HotbarSlot, ItemCollectorBinding, OwnedPlayerHandle, PlayerLocalGeneration,
};

/// Hard ceiling for the checked scalar lifecycle outer layout, not an RSS bound.
pub const MAX_COLLECTOR_LIFECYCLE_BYTES: usize = 1024 * 1024;

/// Classified lifecycle configuration, admission or acknowledgement refusal.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CollectorLifecycleError {
    /// The positive byte allowance exceeds its hard ceiling.
    #[error("invalid collector lifecycle byte limit")]
    InvalidLimit,
    /// Checked layout, count or pin arithmetic overflowed.
    #[error("collector lifecycle arithmetic overflow")]
    SizeOverflow,
    /// The actual owner has not selected lifecycle mode.
    #[error("collector lifecycle is not initialized")]
    NotInitialized,
    /// This owner already selected lifecycle mode.
    #[error("collector lifecycle is already initialized")]
    AlreadyInitialized,
    /// Admissions or resources already exist on the actual owner.
    #[error("collector lifecycle requires an empty owner")]
    NotEmpty,
    /// The exact scalar outer layout exceeds the configured byte allowance.
    #[error("collector lifecycle layout requires {required} bytes, limit {limit}")]
    ByteCapacity {
        /// Checked bytes required by the actual Rust layout.
        required: usize,
        /// Configured positive scalar byte allowance.
        limit: usize,
    },
    /// A new admission would exceed the configured cell count.
    #[error("collector lifecycle cells exhausted")]
    CellCapacity,
    /// Reserved plus retained Take pins would exceed the publication group cap.
    #[error("collector lifecycle Take pins exhausted")]
    PinCapacity,
    /// The bounded outer cell allocation failed before activation.
    #[error("collector lifecycle allocation failed: {0}")]
    AllocationFailed(#[source] TryReserveError),
    /// The original admission has no matching retained cell.
    #[error("collector lifecycle admission is foreign, stale or acknowledged")]
    AdmissionMismatch,
    /// The ticket is foreign, stale, repeated or no longer detached with zero pins.
    #[error("collector lifecycle acknowledgement mismatch")]
    AckMismatch,
}

/// Explicit positive scalar byte cap; cell and pin counts come from actual owner limits.
#[derive(Debug, Clone, Copy)]
pub struct CollectorLifecycleLimits {
    bytes: usize,
}
impl CollectorLifecycleLimits {
    /// Validates a positive allowance of at most one MiB without allocating.
    pub fn new(bytes: NonZeroUsize) -> Result<Self, CollectorLifecycleError> {
        if bytes.get() > MAX_COLLECTOR_LIFECYCLE_BYTES {
            return Err(CollectorLifecycleError::InvalidLimit);
        }
        Ok(Self { bytes: bytes.get() })
    }
}

/// Borrow of the actual connected native `PlayerState` and its actual owner.
/// No cloned snapshot, public constructor or mutation handle is available.
///
/// ```compile_fail
/// use ferrumc_sim::ConnectedOwnedPlayerView;
/// fn copy<'a>(view: &ConnectedOwnedPlayerView<'a>) -> ConnectedOwnedPlayerView<'a> {
///     view.clone()
/// }
/// ```
pub struct ConnectedOwnedPlayerView<'a> {
    pub(crate) player: &'a crate::shard::PlayerState,
    pub(crate) state: &'a crate::player::PlayerOwnershipState,
    pub(crate) id: PlayerId,
    /// Checked copy of the borrowed admission's `OwnedPlayerInventory::selected`;
    /// the shared shard borrow `'a` keeps it equal to that field.
    pub(crate) selected: HotbarSlot,
}
impl ConnectedOwnedPlayerView<'_> {
    /// Returns the actual player's UUID identity.
    pub const fn player(&self) -> PlayerId {
        self.id
    }
    /// Returns the owner's complete current generation binding.
    pub fn generation(&self) -> GenerationBinding {
        self.state.binding.generation()
    }
    /// Returns the actual native position, preserving float bits.
    pub const fn position(&self) -> Vec3 {
        self.player.position
    }
    /// Returns actual native yaw and pitch without normalization.
    pub const fn look(&self) -> [f32; 2] {
        [self.player.yaw, self.player.pitch]
    }
    /// Returns the actual native mode, including fresh-bootstrap Survival.
    pub const fn game_mode(&self) -> GameMode {
        self.player.game_mode
    }
    /// Returns the applied selected hotbar slot of exactly this admission.
    ///
    /// The value is the owner's single `selected` field: an accepted
    /// `select_hotbar` writes it when applied, and every native player pending
    /// (autosave, leave-save, drop, pickup, click) materializes
    /// `NativePlayerSnapshot::selected` from it. A
    /// selection still deferred behind a pending save or refused input is not
    /// visible, exactly like position, look and mode. Bootstrap, restore and
    /// rejoin seed it from the admitted durable snapshot.
    pub const fn selected(&self) -> HotbarSlot {
        self.selected
    }
    /// Uses the existing World `BiomeZoomSeed` producer on the actual world seed.
    pub fn hashed_seed(&self) -> i64 {
        ferrumc_world::biome_zoom::BiomeZoomSeed::from_world_seed(
            self.state.binding.generation().identity().seed(),
        )
        .bits()
    }
    /// Compares a genuine handle with this borrowed actual admission and owner.
    pub fn matches_owned_player(&self, handle: &OwnedPlayerHandle) -> bool {
        crate::player::same_owner(&handle.owner, &self.state.owner)
            && self.id == handle.player
            && self
                .player
                .owned
                .as_ref()
                .is_some_and(|o| o.admission == handle.admission)
    }
}

/// Sealed evidence minted once by the real native Disconnect apply.
/// It proves Sim detachment only; Session retirement and socket receipt are separate.
///
/// ```compile_fail
/// use ferrumc_sim::{ItemCollectorBinding, NativePlayerDetachFact};
/// fn forge(collector: ItemCollectorBinding) -> NativePlayerDetachFact {
///     NativePlayerDetachFact { collector }
/// }
/// ```
/// ```compile_fail
/// use ferrumc_sim::NativePlayerDetachFact;
/// fn copy(fact: &NativePlayerDetachFact) -> NativePlayerDetachFact { fact.clone() }
/// ```
#[derive(Debug)]
pub struct NativePlayerDetachFact {
    collector: ItemCollectorBinding,
}
impl NativePlayerDetachFact {
    /// Returns the originally detached player identity.
    pub const fn player(&self) -> PlayerId {
        self.collector.player()
    }
    /// Returns the originally detached admission, never a later rejoin.
    pub const fn admission(&self) -> PlayerLocalGeneration {
        self.collector.admission()
    }
    /// Returns the full original generation binding.
    pub const fn generation(&self) -> GenerationBinding {
        self.collector.generation()
    }
    /// Compares private original owner, player and admission to a genuine handle.
    pub fn matches_owned_player(&self, handle: &OwnedPlayerHandle) -> bool {
        self.collector.matches_owned_player(handle)
    }
}

/// Exact detached, source-pins-zero cell ticket; never a network receipt.
#[derive(Debug)]
pub struct CollectorLifecycleTicket {
    pub(crate) key: LifecycleCellKey,
    pub(crate) collector: ItemCollectorBinding,
}

/// Short borrow of a real retained historical admission cell, including after rejoin.
///
/// ```compile_fail
/// use ferrumc_sim::CollectorLifecycleView;
/// fn copy<'a>(view: &CollectorLifecycleView<'a>) -> CollectorLifecycleView<'a> {
///     view.clone()
/// }
/// ```
pub struct CollectorLifecycleView<'a> {
    pub(crate) cell: &'a CollectorLifecycleCell,
    pub(crate) live: Option<ConnectedOwnedPlayerView<'a>>,
}
impl CollectorLifecycleView<'_> {
    /// Compares this historical cell with a genuine original handle.
    pub fn matches_owned_player(&self, handle: &OwnedPlayerHandle) -> bool {
        self.cell
            .collector
            .as_ref()
            .is_some_and(|c| c.matches_owned_player(handle))
    }
    /// Returns the full generation of the original admission.
    pub const fn generation(&self) -> GenerationBinding {
        self.cell.generation
    }
    /// Borrows actual connected state only when this remains the current admission.
    pub fn live(&self) -> Option<ConnectedOwnedPlayerView<'_>> {
        self.live.as_ref().map(|v| ConnectedOwnedPlayerView {
            player: v.player,
            state: v.state,
            id: v.id,
            selected: v.selected,
        })
    }
    /// Borrows the exact retained fact minted by native Disconnect apply.
    pub fn detach(&self) -> Option<&NativePlayerDetachFact> {
        self.cell.detach.as_ref()
    }
    /// Returns pending original-admission Take pins.
    pub const fn reserved_take_pins(&self) -> usize {
        self.cell.reserved
    }
    /// Returns confirmed, unacknowledged original-admission Take pins.
    pub const fn retained_take_pins(&self) -> usize {
        self.cell.retained
    }
    /// Mints an exact ticket only after native detach and zero source pins.
    /// Driver must separately complete genuine Session retirement and consumer refs.
    pub fn retirement_ticket(&self) -> Option<CollectorLifecycleTicket> {
        if self.cell.detach.is_none() || self.cell.reserved != 0 || self.cell.retained != 0 {
            return None;
        }
        Some(CollectorLifecycleTicket {
            key: self.cell.key,
            collector: self.cell.collector.as_ref()?.clone(),
        })
    }
}

/// Descriptive scalar accounting; no lifecycle mutation authority.
#[derive(Debug, Clone, Copy)]
pub struct CollectorLifecycleStatus {
    pub(crate) credits: LifecycleCredits,
    pub(crate) bytes: usize,
}
impl CollectorLifecycleStatus {
    /// Returns all reserved-bootstrap and admitted historical cells.
    pub const fn cells(self) -> usize {
        self.credits.cells
    }
    /// Returns the checked owned-player plus publication-group cell ceiling.
    pub const fn cell_limit(self) -> usize {
        self.credits.cell_limit
    }
    /// Returns pending Take pins across all original admissions.
    pub const fn reserved_take_pins(self) -> usize {
        self.credits.reserved
    }
    /// Returns confirmed Take pins across all original admissions.
    pub const fn retained_take_pins(self) -> usize {
        self.credits.retained
    }
    /// Returns the checked Vec header plus actual `Option<Cell>` array charge.
    pub const fn scalar_bytes(self) -> usize {
        self.bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LifecycleCellKey {
    slot: usize,
    credit: PlayerLocalGeneration,
}

#[derive(Debug)]
pub(crate) struct CollectorLifecycleCell {
    key: LifecycleCellKey,
    player: PlayerId,
    generation: GenerationBinding,
    collector: Option<ItemCollectorBinding>,
    pub(crate) detach: Option<NativePlayerDetachFact>,
    reserved: usize,
    retained: usize,
}

// Countershadow copies only these fixed scalars and uses these exact transitions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LifecycleCredits {
    pub(crate) cells: usize,
    cell_limit: usize,
    reserved: usize,
    retained: usize,
    pin_limit: usize,
}
impl LifecycleCredits {
    pub(crate) fn check_cell(self) -> Result<(), CollectorLifecycleError> {
        if self.cells >= self.cell_limit {
            return Err(CollectorLifecycleError::CellCapacity);
        }
        Ok(())
    }
    pub(crate) fn reserve_cell(&mut self) {
        assert!(
            self.check_cell().is_ok(),
            "lifecycle cell credit disappeared"
        );
        self.cells += 1;
    }
    pub(crate) fn release_cell(&mut self) {
        assert!(self.cells > 0, "lifecycle cell credit released twice");
        self.cells -= 1;
    }
    pub(crate) fn check_pin(self) -> Result<(), CollectorLifecycleError> {
        let n = self
            .reserved
            .checked_add(self.retained)
            .and_then(|n| n.checked_add(1))
            .ok_or(CollectorLifecycleError::SizeOverflow)?;
        if n > self.pin_limit {
            return Err(CollectorLifecycleError::PinCapacity);
        }
        Ok(())
    }
    pub(crate) fn reserve_pin(&mut self) {
        assert!(self.check_pin().is_ok(), "lifecycle pin credit disappeared");
        self.reserved += 1;
    }
    pub(crate) fn cancel_pin(&mut self) {
        assert!(self.reserved > 0, "reserved lifecycle pin released twice");
        self.reserved -= 1;
    }
    pub(crate) fn retain_pin(&mut self) {
        self.cancel_pin();
        self.retained += 1;
    }
    fn acknowledge_pin(&mut self) {
        assert!(self.retained > 0, "retained lifecycle pin released twice");
        self.retained -= 1;
    }
}

#[derive(Debug)]
pub(crate) struct CollectorLifecycleState {
    cells: Vec<Option<CollectorLifecycleCell>>,
    pub(crate) credits: LifecycleCredits,
    bytes: usize,
}
impl CollectorLifecycleState {
    pub(crate) fn new(
        players: usize,
        groups: usize,
        limits: CollectorLifecycleLimits,
    ) -> Result<Self, CollectorLifecycleError> {
        let count = players
            .checked_add(groups)
            .ok_or(CollectorLifecycleError::SizeOverflow)?;
        let bytes = count
            .checked_mul(size_of::<Option<CollectorLifecycleCell>>())
            .and_then(|n| n.checked_add(size_of::<Vec<Option<CollectorLifecycleCell>>>()))
            .ok_or(CollectorLifecycleError::SizeOverflow)?;
        if bytes > limits.bytes {
            return Err(CollectorLifecycleError::ByteCapacity {
                required: bytes,
                limit: limits.bytes,
            });
        }
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(count)
            .map_err(CollectorLifecycleError::AllocationFailed)?;
        cells.resize_with(count, || None);
        Ok(Self {
            cells,
            credits: LifecycleCredits {
                cells: 0,
                cell_limit: count,
                reserved: 0,
                retained: 0,
                pin_limit: groups,
            },
            bytes,
        })
    }
    pub(crate) fn status(&self) -> CollectorLifecycleStatus {
        CollectorLifecycleStatus {
            credits: self.credits,
            bytes: self.bytes,
        }
    }
    pub(crate) fn staged_cell(
        &self,
        credit: PlayerLocalGeneration,
    ) -> Result<LifecycleCellKey, CollectorLifecycleError> {
        self.credits.check_cell()?;
        let slot = self
            .cells
            .iter()
            .position(Option::is_none)
            .ok_or(CollectorLifecycleError::CellCapacity)?;
        Ok(LifecycleCellKey { slot, credit })
    }
    pub(crate) fn reserve_cell(
        &mut self,
        key: LifecycleCellKey,
        player: PlayerId,
        generation: GenerationBinding,
    ) {
        assert!(
            self.cells[key.slot].is_none(),
            "staged lifecycle slot disappeared"
        );
        self.credits.reserve_cell();
        self.cells[key.slot] = Some(CollectorLifecycleCell {
            key,
            player,
            generation,
            collector: None,
            detach: None,
            reserved: 0,
            retained: 0,
        });
    }
    fn cell(&self, key: LifecycleCellKey) -> Option<&CollectorLifecycleCell> {
        self.cells.get(key.slot)?.as_ref().filter(|c| c.key == key)
    }
    fn cell_mut(&mut self, key: LifecycleCellKey) -> &mut CollectorLifecycleCell {
        let Some(cell) = self
            .cells
            .get_mut(key.slot)
            .and_then(Option::as_mut)
            .filter(|c| c.key == key)
        else {
            panic!("reserved original lifecycle cell disappeared");
        };
        cell
    }
    pub(crate) fn lookup(
        &self,
        handle: &OwnedPlayerHandle,
    ) -> Result<&CollectorLifecycleCell, CollectorLifecycleError> {
        self.cells
            .iter()
            .flatten()
            .find(|c| {
                c.collector
                    .as_ref()
                    .is_some_and(|b| b.matches_owned_player(handle))
            })
            .ok_or(CollectorLifecycleError::AdmissionMismatch)
    }
    /// Whether any retained cell, of any admission, still names this player.
    pub(crate) fn binds_player(&self, player: PlayerId) -> bool {
        self.cells.iter().flatten().any(|c| c.player == player)
    }
    pub(crate) fn staged_pin(
        &self,
        collector: &ItemCollectorBinding,
    ) -> Result<LifecycleCellKey, CollectorLifecycleError> {
        self.credits.check_pin()?;
        self.cells
            .iter()
            .flatten()
            .find(|c| c.collector.as_ref() == Some(collector))
            .map(|c| c.key)
            .ok_or(CollectorLifecycleError::AdmissionMismatch)
    }
    pub(crate) fn seal(&mut self, key: LifecycleCellKey, handle: &OwnedPlayerHandle) {
        let cell = self.cell_mut(key);
        assert!(
            cell.collector.is_none() && cell.player == handle.player,
            "bootstrap lifecycle seal differs"
        );
        cell.collector = Some(ItemCollectorBinding::admitted(handle, cell.generation));
    }
    pub(crate) fn cancel_cell(&mut self, key: LifecycleCellKey) {
        let cell = self.cell_mut(key);
        assert!(
            cell.collector.is_none() && cell.reserved == 0 && cell.retained == 0,
            "admitted lifecycle credit aborted"
        );
        self.cells[key.slot] = None;
        self.credits.release_cell();
    }
    pub(crate) fn detach(
        &mut self,
        handle: &OwnedPlayerHandle,
    ) -> Result<(), CollectorLifecycleError> {
        let key = self.lookup(handle)?.key;
        let cell = self.cell_mut(key);
        if cell.detach.is_none() {
            let Some(collector) = &cell.collector else {
                panic!("admitted lifecycle binding disappeared");
            };
            cell.detach = Some(NativePlayerDetachFact {
                collector: collector.clone(),
            });
        }
        Ok(())
    }
    pub(crate) fn reserve_pin(&mut self, key: LifecycleCellKey) {
        self.credits.reserve_pin();
        self.cell_mut(key).reserved += 1;
    }
    pub(crate) fn has_reserved_pin(&self, key: LifecycleCellKey) -> bool {
        self.cell(key).is_some_and(|c| c.reserved != 0)
    }
    pub(crate) fn cancel_pin(&mut self, key: LifecycleCellKey) {
        let c = self.cell_mut(key);
        assert!(c.reserved > 0);
        c.reserved -= 1;
        self.credits.cancel_pin();
    }
    pub(crate) fn retain_pin(&mut self, key: LifecycleCellKey) {
        let c = self.cell_mut(key);
        assert!(c.reserved > 0);
        c.reserved -= 1;
        c.retained += 1;
        self.credits.retain_pin();
    }
    pub(crate) fn acknowledge_pin(&mut self, key: LifecycleCellKey) {
        let c = self.cell_mut(key);
        assert!(c.retained > 0);
        c.retained -= 1;
        self.credits.acknowledge_pin();
    }
    pub(crate) fn acknowledge(
        &mut self,
        ticket: &CollectorLifecycleTicket,
    ) -> Result<(), CollectorLifecycleError> {
        let cell = self
            .cell(ticket.key)
            .ok_or(CollectorLifecycleError::AckMismatch)?;
        if cell.collector.as_ref() != Some(&ticket.collector)
            || cell.generation != ticket.collector.generation()
            || cell.detach.is_none()
            || cell.reserved != 0
            || cell.retained != 0
        {
            return Err(CollectorLifecycleError::AckMismatch);
        }
        self.cells[ticket.key.slot] = None;
        self.credits.release_cell();
        Ok(())
    }
}
