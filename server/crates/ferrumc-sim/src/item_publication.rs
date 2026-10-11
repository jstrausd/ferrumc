//! Reserved immutable item facts; no network, durability or mutation capability.

use std::collections::{TryReserveError, VecDeque};
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{Arc, Weak};

use ferrumc_core::{EntityId, EntityUuid, PlayerId};
use ferrumc_items::{ItemPersistenceError, ItemStack};
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_storage::{OwnershipReceiptQuery, OwnershipStamp};

use crate::{
    EntityData, EntityHandle, EntityTransform, GenerationBinding, ItemEntityProperties,
    OwnedPlayerHandle, PlayerLocalGeneration, ShardId,
};

#[cfg(test)]
mod tests;

/// Local maximum pending-plus-retained publication groups.
pub const MAX_ITEM_PUBLICATION_GROUPS: usize = 256;
/// Local maximum pending-plus-retained encoded charge, not decoded heap or RSS.
pub const MAX_ITEM_PUBLICATION_RETAINED_BYTES: usize = 8 * 1024 * 1024;
/// Local maximum encoded charge of one staged publication preparation.
pub const MAX_ITEM_PUBLICATION_PREPARE_BYTES: usize = 1024 * 1024;
/// Fixed logical scalar charge per group, not a wire format or allocator measure.
pub const ITEM_PUBLICATION_SCALAR_CHARGE: usize = 256;

/// The explicitly selected publication resource policy.
#[derive(Debug, Clone, Copy)]
pub struct ItemPublicationLimits {
    groups: usize,
    retained_bytes: usize,
    prepare_bytes: usize,
}

/// One bounded publication resource category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPublicationBudget {
    /// Pending and retained group slots together.
    Groups,
    /// Pending and retained logical encoded charges together.
    RetainedBytes,
    /// One additional staged publication body.
    PrepareBytes,
}

/// A classified refusal before a publication reservation or acknowledgement.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ItemPublicationError {
    /// Configuration exceeds a positive local hard ceiling.
    #[error("invalid item publication {budget:?} limit {requested}, maximum {max}")]
    InvalidLimits {
        /// Resource selected by the caller.
        budget: ItemPublicationBudget,
        /// Requested positive limit.
        requested: usize,
        /// Local hard maximum.
        max: usize,
    },
    /// The actual owner has not selected publication mode.
    #[error("item publications are not initialized")]
    NotInitialized,
    /// Publication mode is once-only and cannot reset its sequence.
    #[error("item publications are already initialized")]
    AlreadyInitialized,
    /// Initializing would omit existing registrations, inputs or reservations.
    #[error("item publication initialization requires an empty owner")]
    NotEmpty,
    /// A positive local allowance would be exceeded.
    #[error("item publication {budget:?} capacity: {used} + {requested} exceeds {max}")]
    Capacity {
        /// Exhausted resource.
        budget: ItemPublicationBudget,
        /// Already accounted usage.
        used: usize,
        /// Additional requested usage.
        requested: usize,
        /// Configured local maximum.
        max: usize,
    },
    /// Checked charge or count arithmetic overflowed.
    #[error("item publication size arithmetic overflowed")]
    SizeOverflow,
    /// Existing publications and reserved credits exhaust the sequence domain.
    #[error("item publication sequence exhausted")]
    SequenceExhausted,
    /// The bounded outer queue allocation failed before mode activation.
    #[error("item publication queue allocation failed: {0}")]
    AllocationFailed(#[source] TryReserveError),
    /// Existing borrowed lossless model validation refused a source.
    #[error(transparent)]
    Model(#[from] ItemPersistenceError),
    /// There is no retained FIFO head to acknowledge.
    #[error("item publication acknowledgement has no head")]
    AckWithoutHead,
    /// The ticket does not bind the actual owner and current FIFO head.
    #[error("item publication acknowledgement is foreign, stale or reordered")]
    AckMismatch,
}

impl ItemPublicationLimits {
    /// Selects positive explicit limits below the local hard ceilings.
    ///
    /// # Errors
    /// Returns [`ItemPublicationError::InvalidLimits`] for an exceeded ceiling.
    pub fn new(
        groups: NonZeroUsize,
        retained_bytes: NonZeroUsize,
        prepare_bytes: NonZeroUsize,
    ) -> Result<Self, ItemPublicationError> {
        for (budget, requested, max) in [
            (
                ItemPublicationBudget::Groups,
                groups.get(),
                MAX_ITEM_PUBLICATION_GROUPS,
            ),
            (
                ItemPublicationBudget::RetainedBytes,
                retained_bytes.get(),
                MAX_ITEM_PUBLICATION_RETAINED_BYTES,
            ),
            (
                ItemPublicationBudget::PrepareBytes,
                prepare_bytes.get(),
                MAX_ITEM_PUBLICATION_PREPARE_BYTES,
            ),
        ] {
            if requested > max {
                return Err(ItemPublicationError::InvalidLimits {
                    budget,
                    requested,
                    max,
                });
            }
        }
        Ok(Self {
            groups: groups.get(),
            retained_bytes: retained_bytes.get(),
            prepare_bytes: prepare_bytes.get(),
        })
    }
}

/// Actual scope and generational store instance, independent of UUID/wire aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInstanceBinding {
    handle: EntityHandle,
    generation: GenerationBinding,
    owner: ShardId,
    uuid: EntityUuid,
    wire_id: EntityId,
    entity_type: EntityTypeId,
}

impl ItemInstanceBinding {
    pub(crate) fn new(
        handle: EntityHandle,
        generation: GenerationBinding,
        data: EntityData,
    ) -> Self {
        Self {
            handle,
            generation,
            owner: data.owner(),
            uuid: data.uuid(),
            wire_id: data.wire_id(),
            entity_type: data.entity_type(),
        }
    }
    /// Returns the actual owner, including world/dimension/shard scope.
    pub const fn owner(&self) -> ShardId {
        self.owner
    }
    /// Returns the complete actual baseline declaration.
    pub const fn generation(&self) -> GenerationBinding {
        self.generation
    }
    /// Returns the item's persistent UUID.
    pub const fn uuid(&self) -> EntityUuid {
        self.uuid
    }
    /// Returns the actual registered wire ID, without global allocation authority.
    pub const fn wire_id(&self) -> EntityId {
        self.wire_id
    }
    /// Returns the actual registered entity type.
    pub const fn entity_type(&self) -> EntityTypeId {
        self.entity_type
    }
    /// Returns the original slot generation, retained even after removal.
    pub const fn slot_generation(&self) -> u64 {
        self.handle.publication_generation()
    }
}

/// Non-wrapping owner-local publication order; not a durable revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPublicationSequence(NonZeroU64);
impl ItemPublicationSequence {
    /// Returns the assigned nonzero order value.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// The actual source of publication authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPublicationOriginKind {
    /// Genuine receipt and current-image confirmation.
    ConfirmedReceipt,
    /// Genuine store-bound native restore, without a new commit receipt.
    NativeRestore,
}

/// Sealed source evidence, exposing data rather than a storage/write factory.
#[derive(Debug)]
pub struct ItemPublicationOrigin {
    kind: ItemPublicationOriginKind,
    query: Option<OwnershipReceiptQuery>,
    stamp: OwnershipStamp,
}
impl ItemPublicationOrigin {
    pub(crate) const fn confirmed(query: OwnershipReceiptQuery, stamp: OwnershipStamp) -> Self {
        Self {
            kind: ItemPublicationOriginKind::ConfirmedReceipt,
            query: Some(query),
            stamp,
        }
    }
    pub(crate) const fn restored(stamp: OwnershipStamp) -> Self {
        Self {
            kind: ItemPublicationOriginKind::NativeRestore,
            query: None,
            stamp,
        }
    }
    /// Returns genuine confirmation versus native restore provenance.
    pub const fn kind(&self) -> ItemPublicationOriginKind {
        self.kind
    }
    /// Returns the actual receipt query, absent for restore.
    pub const fn receipt_query(&self) -> Option<OwnershipReceiptQuery> {
        self.query
    }
    /// Returns the actual confirmed durable stamp; live counters may be newer.
    pub const fn stamp(&self) -> OwnershipStamp {
        self.stamp
    }
}

/// One actual mutation effect. Optional Take precedes Changed or Removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPublicationKind {
    /// Confirmed new registration.
    Spawn,
    /// Actual native restore admission.
    Restore,
    /// Confirmed visible postimage replacement.
    Changed,
    /// Confirmed removal of exactly the bound instance.
    Removed,
}

/// Immutable complete stack and actual scalar state at the documented cutpoint.
#[derive(Debug)]
pub struct ItemPublicationPostimage {
    stack: ItemStack,
    transform: EntityTransform,
    properties: ItemEntityProperties,
}
impl ItemPublicationPostimage {
    /// Borrows the unchanged complete authoritative item representation.
    pub const fn stack(&self) -> &ItemStack {
        &self.stack
    }
    /// Returns the actual published transform.
    pub const fn transform(&self) -> EntityTransform {
        self.transform
    }
    /// Returns actual properties after counterconfirmation, before the later tick.
    pub const fn properties(&self) -> ItemEntityProperties {
        self.properties
    }
}

/// Actual validated pickup admission, without a invented collector wire ID.
#[derive(Debug, Clone)]
pub struct ItemCollectorBinding {
    owner: Weak<()>,
    player: PlayerId,
    admission: PlayerLocalGeneration,
    generation: GenerationBinding,
}
impl PartialEq for ItemCollectorBinding {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.owner, &other.owner)
            && self.player == other.player
            && self.admission == other.admission
            && self.generation == other.generation
    }
}
impl Eq for ItemCollectorBinding {}
impl ItemCollectorBinding {
    pub(crate) fn admitted(handle: &OwnedPlayerHandle, generation: GenerationBinding) -> Self {
        Self {
            owner: handle.owner.clone(),
            player: handle.player,
            admission: handle.admission,
            generation,
        }
    }
    /// Returns the actual pickup player UUID identity.
    pub const fn player(&self) -> PlayerId {
        self.player
    }
    /// Returns the captured admission, never updated by a later rejoin.
    pub const fn admission(&self) -> PlayerLocalGeneration {
        self.admission
    }
    /// Returns the actual scope and baseline of that admission.
    pub const fn generation(&self) -> GenerationBinding {
        self.generation
    }
    /// Compares the original captured admission with an existing actual handle.
    ///
    /// Checks private owner allocation, player and admission without exposing
    /// a Weak or producing a mutation handle. This is not an active/connected
    /// or current-scope proof; Session must separately validate current scope,
    /// generation and its actual Session/collector-wire binding.
    pub fn matches_owned_player(&self, handle: &OwnedPlayerHandle) -> bool {
        Weak::ptr_eq(&self.owner, &handle.owner)
            && self.player == handle.player
            && self.admission == handle.admission
    }
}

/// Take data from the same confirmed mutation; collector audience belongs to Session.
#[derive(Debug)]
pub struct ItemTakeFact {
    collector: ItemCollectorBinding,
    original_count: u8,
    moved: u8,
}
impl ItemTakeFact {
    /// Borrows the actual collector admission, without network authority.
    pub const fn collector(&self) -> &ItemCollectorBinding {
        &self.collector
    }
    /// Returns original animation count, independently of moved inventory quantity.
    pub const fn original_count(&self) -> u8 {
        self.original_count
    }
    /// Returns the genuinely conserved inventory quantity.
    pub const fn moved(&self) -> u8 {
        self.moved
    }
}

/// Cheap producer-minted FIFO ticket, not proof that sockets received data.
#[derive(Debug, Clone)]
pub struct ItemPublicationTicket {
    owner: Weak<()>,
    instance: ItemInstanceBinding,
    sequence: ItemPublicationSequence,
}

/// A retained sealed immutable publication; no public construction or Clone.
#[derive(Debug)]
pub struct ItemPublicationGroup {
    ticket: ItemPublicationTicket,
    origin: ItemPublicationOrigin,
    kind: ItemPublicationKind,
    postimage: Option<ItemPublicationPostimage>,
    take: Option<ItemTakeFact>,
    pub(crate) lifecycle_pin: Option<crate::collector_lifecycle::LifecycleCellKey>,
    charge: usize,
}
impl ItemPublicationGroup {
    /// Borrows the full actual store instance, even after its removal.
    pub const fn instance(&self) -> &ItemInstanceBinding {
        &self.ticket.instance
    }
    /// Returns the actual assigned publication order.
    pub const fn sequence(&self) -> ItemPublicationSequence {
        self.ticket.sequence
    }
    /// Borrows actual confirmation or restore provenance.
    pub const fn origin(&self) -> &ItemPublicationOrigin {
        &self.origin
    }
    /// Returns the mutation effect, after any optional Take phase.
    pub const fn kind(&self) -> ItemPublicationKind {
        self.kind
    }
    /// Borrows complete actual state; absent for removal.
    pub const fn postimage(&self) -> Option<&ItemPublicationPostimage> {
        self.postimage.as_ref()
    }
    /// Borrows optional animation data which must precede Changed/Removed output.
    pub const fn take(&self) -> Option<&ItemTakeFact> {
        self.take.as_ref()
    }
    /// Mints a cheap actual-owner acknowledgement ticket, without copying a body.
    pub fn ticket(&self) -> ItemPublicationTicket {
        self.ticket.clone()
    }
    /// Returns admitted logical encoded charge, not heap or wire measurements.
    pub const fn encoded_charge(&self) -> usize {
        self.charge
    }
}

/// Descriptive publication resource accounting, with no publication authority.
#[derive(Debug, Clone, Copy)]
pub struct ItemPublicationStatus(pub(crate) PublicationLedger);
impl ItemPublicationStatus {
    /// Returns the retained FIFO group count.
    pub const fn retained_groups(self) -> usize {
        self.0.retained
    }
    /// Returns pending reserved groups.
    pub const fn reserved_groups(self) -> usize {
        self.0.reserved
    }
    /// Returns retained logical encoded charge.
    pub const fn retained_bytes(self) -> usize {
        self.0.retained_bytes
    }
    /// Returns pending logical encoded charge.
    pub const fn reserved_bytes(self) -> usize {
        self.0.reserved_bytes
    }
    /// Returns the one-preparation charge limit.
    pub const fn prepare_bytes_limit(self) -> usize {
        self.0.limits.prepare_bytes
    }
    /// Returns the last actual published sequence; zero means none.
    pub const fn last_published(self) -> u64 {
        self.0.last
    }
}

pub(crate) fn publication_charge(stack_len: Option<usize>) -> Result<usize, ItemPublicationError> {
    ITEM_PUBLICATION_SCALAR_CHARGE
        .checked_add(stack_len.unwrap_or(0))
        .ok_or(ItemPublicationError::SizeOverflow)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PublicationLedger {
    limits: ItemPublicationLimits,
    reserved: usize,
    retained: usize,
    reserved_bytes: usize,
    retained_bytes: usize,
    last: u64,
}
impl PublicationLedger {
    pub(crate) const fn group_limit(self) -> usize {
        self.limits.groups
    }
    pub(crate) fn check(&self, charge: usize) -> Result<(), ItemPublicationError> {
        let groups = self
            .reserved
            .checked_add(self.retained)
            .ok_or(ItemPublicationError::SizeOverflow)?;
        let bytes = self
            .reserved_bytes
            .checked_add(self.retained_bytes)
            .ok_or(ItemPublicationError::SizeOverflow)?;
        for (budget, used, requested, max) in [
            (ItemPublicationBudget::Groups, groups, 1, self.limits.groups),
            (
                ItemPublicationBudget::RetainedBytes,
                bytes,
                charge,
                self.limits.retained_bytes,
            ),
            (
                ItemPublicationBudget::PrepareBytes,
                0,
                charge,
                self.limits.prepare_bytes,
            ),
        ] {
            if used
                .checked_add(requested)
                .ok_or(ItemPublicationError::SizeOverflow)?
                > max
            {
                return Err(ItemPublicationError::Capacity {
                    budget,
                    used,
                    requested,
                    max,
                });
            }
        }
        let credits =
            u64::try_from(self.reserved).map_err(|_| ItemPublicationError::SizeOverflow)?;
        self.last
            .checked_add(credits)
            .and_then(|n| n.checked_add(1))
            .ok_or(ItemPublicationError::SequenceExhausted)?;
        Ok(())
    }
    pub(crate) fn reserve(&mut self, charge: usize) {
        debug_assert!(self.check(charge).is_ok());
        self.reserved += 1;
        self.reserved_bytes += charge;
    }
    pub(crate) fn cancel(&mut self, charge: usize) {
        debug_assert!(self.reserved > 0 && self.reserved_bytes >= charge);
        self.reserved -= 1;
        self.reserved_bytes -= charge;
    }
    pub(crate) fn publish(&mut self, charge: usize) -> ItemPublicationSequence {
        self.cancel(charge);
        self.retained += 1;
        self.retained_bytes += charge;
        let Some(value) = self.last.checked_add(1).and_then(NonZeroU64::new) else {
            panic!("reserved publication sequence credit disappeared");
        };
        self.last = value.get();
        ItemPublicationSequence(value)
    }
    pub(crate) const fn is_empty(self) -> bool {
        self.reserved == 0 && self.retained == 0
    }
    pub(crate) fn has_reservation(self, charge: usize) -> bool {
        self.reserved != 0
            && self.reserved_bytes >= charge
            && u64::try_from(self.reserved)
                .ok()
                .and_then(|n| self.last.checked_add(n))
                .is_some()
    }
}

#[derive(Debug)]
pub(crate) struct ItemPublicationState {
    pub(crate) ledger: PublicationLedger,
    queue: VecDeque<ItemPublicationGroup>,
}
impl ItemPublicationState {
    #[cfg(test)]
    pub(crate) fn set_test_sequence_boundary(&mut self, last: u64) {
        assert!(self.ledger.is_empty());
        self.ledger.last = last;
    }

    #[cfg(test)]
    pub(crate) fn test_tail_ticket(&self) -> ItemPublicationTicket {
        self.queue.back().unwrap().ticket()
    }

    pub(crate) fn new(limits: ItemPublicationLimits) -> Result<Self, ItemPublicationError> {
        let mut queue = VecDeque::new();
        queue
            .try_reserve_exact(limits.groups)
            .map_err(ItemPublicationError::AllocationFailed)?;
        Ok(Self {
            ledger: PublicationLedger {
                limits,
                reserved: 0,
                retained: 0,
                reserved_bytes: 0,
                retained_bytes: 0,
                last: 0,
            },
            queue,
        })
    }
    pub(crate) fn peek(&self) -> Option<&ItemPublicationGroup> {
        self.queue.front()
    }
    /// Whether a retained Take group names this player as its collector.
    pub(crate) fn retains_collector(&self, player: PlayerId) -> bool {
        self.queue.iter().any(|g| {
            g.take
                .as_ref()
                .is_some_and(|t| t.collector.player == player)
        })
    }
    pub(crate) fn acknowledge(
        &mut self,
        owner: &Arc<()>,
        ticket: &ItemPublicationTicket,
    ) -> Result<Option<crate::collector_lifecycle::LifecycleCellKey>, ItemPublicationError> {
        let head = self
            .queue
            .front()
            .ok_or(ItemPublicationError::AckWithoutHead)?;
        if !Weak::ptr_eq(&ticket.owner, &Arc::downgrade(owner))
            || !Weak::ptr_eq(&head.ticket.owner, &ticket.owner)
            || head.ticket.instance != ticket.instance
            || head.ticket.sequence != ticket.sequence
        {
            return Err(ItemPublicationError::AckMismatch);
        }
        let charge = head.charge;
        let pin = head.lifecycle_pin;
        self.queue.pop_front();
        self.ledger.retained -= 1;
        self.ledger.retained_bytes -= charge;
        Ok(pin)
    }
    pub(crate) fn publish(
        &mut self,
        prepared: PreparedItemPublication,
        owner: Weak<()>,
        instance: ItemInstanceBinding,
        scalars: Option<(EntityTransform, ItemEntityProperties)>,
        origin: ItemPublicationOrigin,
    ) {
        let sequence = self.ledger.publish(prepared.charge);
        let postimage = match (prepared.stack, scalars) {
            (Some(stack), Some((transform, properties))) => Some(ItemPublicationPostimage {
                stack,
                transform,
                properties,
            }),
            (None, None) => None,
            _ => panic!("reserved publication postimage disagrees with actual publication"),
        };
        debug_assert!(
            self.queue.len() < self.ledger.limits.groups
                && self.queue.len() < self.queue.capacity()
        );
        self.queue.push_back(ItemPublicationGroup {
            ticket: ItemPublicationTicket {
                owner,
                instance,
                sequence,
            },
            origin,
            kind: prepared.kind,
            postimage,
            take: prepared.take,
            lifecycle_pin: prepared.lifecycle_pin,
            charge: prepared.charge,
        });
    }
}

#[derive(Debug)]
pub(crate) struct PreparedItemPublication {
    pub(crate) lifecycle_pin: Option<crate::collector_lifecycle::LifecycleCellKey>,
    pub(crate) kind: ItemPublicationKind,
    pub(crate) stack: Option<ItemStack>,
    pub(crate) removed: Option<ItemInstanceBinding>,
    pub(crate) take: Option<ItemTakeFact>,
    pub(crate) charge: usize,
}
impl PreparedItemPublication {
    pub(crate) fn take(
        collector: ItemCollectorBinding,
        original_count: u8,
        moved: u8,
    ) -> ItemTakeFact {
        ItemTakeFact {
            collector,
            original_count,
            moved,
        }
    }
}
