//! Bounded legacy ground-item gameplay using the existing concrete entity owner.
//!
//! Native durable player ownership is a separate exclusive mode. The legacy
//! application must persist move-owned pickups/drop requests before treating
//! them as crash durable. No timeout proves that an offered stack was returned.

// Refusals deliberately return the complete owned request without another allocation.
#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroUsize;
use std::sync::{Arc, Weak};

use ferrumc_core::{EntityId, EntityUuid, PlayerId};
use ferrumc_items::{
    encode_item_entity_metadata, encode_persisted_stack, persisted_stack_size, ItemMetadataError,
    ItemPersistenceError, ItemStack,
};
use ferrumc_math::Vec3;

use crate::{
    EntityData, EntityHandle, EntitySpawn, EntityTransform, ItemEntityError, ItemEntityProperties,
    ItemEntitySnapshot, ItemEntitySpawn, ShardId,
};

pub(crate) const MAX_GROUND_EVENTS: usize = 256;
pub(crate) const MAX_GROUND_EVENT_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_GROUND_OFFERS: usize = 32;
pub(crate) const MAX_GROUND_IDENTITIES: usize = 128;
pub(crate) const GROUND_TICK_BUDGET: usize = 32;
pub(crate) const MAX_GROUND_REGIONS: usize = 64;

/// Queued exact native record restore, with a separately supplied fresh wire id.
#[derive(Debug)]
pub struct GroundItemRestore {
    pub(crate) proposal: crate::RestoredItemEntity,
    pub(crate) wire_id: EntityId,
}
impl GroundItemRestore {
    /// Retains the complete decoded native state until canonical owner admission.
    pub fn new(proposal: crate::RestoredItemEntity, wire_id: EntityId) -> Self {
        Self { proposal, wire_id }
    }
    /// Returns both unchanged values after a refused enqueue.
    pub fn into_parts(self) -> (crate::RestoredItemEntity, EntityId) {
        (self.proposal, self.wire_id)
    }
}

/// Refused restore queue admission retaining its entire request.
#[derive(Debug)]
pub struct GroundItemRestoreRefusal {
    pub(crate) reason: GroundItemError,
    pub(crate) restore: GroundItemRestore,
}
impl GroundItemRestoreRefusal {
    /// Borrows the classified reason.
    pub const fn reason(&self) -> &GroundItemError {
        &self.reason
    }
    /// Recovers the original native record state and fresh wire id.
    pub fn into_parts(self) -> (GroundItemError, GroundItemRestore) {
        (self.reason, self.restore)
    }
}

/// Explicit legacy ground-owner limits, independent of the entity byte budget.
#[derive(Debug, Clone, Copy)]
pub struct GroundItemLimits {
    pub(crate) items: usize,
    pub(crate) pending_bytes: usize,
}

impl GroundItemLimits {
    /// Bounds live, offered and queued items together (at most 4096), and each
    /// aggregate reserved native/publication bytes (at most four MiB).
    pub fn new(items: NonZeroUsize, pending_bytes: NonZeroUsize) -> Result<Self, GroundItemError> {
        if items.get() > 4096 || pending_bytes.get() > 4 * 1024 * 1024 {
            return Err(GroundItemError::InvalidLimit);
        }
        Ok(Self {
            items: items.get(),
            pending_bytes: pending_bytes.get(),
        })
    }
}

/// Classified refusal without dropping the caller's owned item input.
#[derive(Debug, thiserror::Error)]
pub enum GroundItemError {
    /// Explicit resource agreement is outside the supported positive caps.
    #[error("invalid ground item limits")]
    InvalidLimit,
    /// This shard has no selected ground-item consumer.
    #[error("ground items are not initialized")]
    NotInitialized,
    /// A live consumer or copied snapshot cannot be replaced.
    #[error("ground items are already initialized")]
    AlreadyInitialized,
    /// Native player ownership cannot share the legacy consumer.
    #[error("ground item operation requires legacy ownership")]
    OwnershipRequired,
    /// Copied simulation state grants no competing ground-item authority.
    #[error("ground item snapshot has no write authority")]
    SnapshotOnly,
    /// Current chunk scope differs from the entity owner.
    #[error("ground item owner scope differs")]
    ScopeMismatch,
    /// An item, identity, event, encoded-byte or response queue is full.
    #[error("ground item capacity exhausted: {0}")]
    Capacity(&'static str),
    /// Persistent UUID or transient wire identity would alias an existing item.
    #[error("duplicate or invalid ground item identity")]
    InvalidIdentity,
    /// Offer ticket is foreign, stale or already has an admitted response.
    #[error("stale ground item pickup ticket")]
    StaleTicket,
    /// Returned stack changed identity/components or increased the count.
    #[error("ground item pickup remainder is not lossless")]
    InvalidRemainder,
    /// Existing concrete entity admission/snapshot failed.
    #[error(transparent)]
    Entity(#[from] ItemEntityError),
    /// Trusted publication metadata cannot represent the complete stack.
    #[error(transparent)]
    Metadata(#[from] ItemMetadataError),
    /// A death batch requires the genuine admitted dead legacy player.
    #[error("player death drop state is invalid")]
    InvalidDeath,
    /// This dead life already admitted its single complete death inventory batch.
    #[error("death drops already admitted")]
    DeathAlreadyAdmitted,
    /// Complete stack preflight failed.
    #[error(transparent)]
    Item(#[from] ItemPersistenceError),
}

/// Driver-supplied persistent and transient identities; simulation mints neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundItemIdentity {
    pub(crate) uuid: EntityUuid,
    pub(crate) wire_id: EntityId,
}

impl GroundItemIdentity {
    /// Admits an explicitly allocated non-player wire id, at least two.
    pub fn new(uuid: EntityUuid, wire_id: EntityId) -> Result<Self, GroundItemError> {
        if wire_id.get() < 2 {
            return Err(GroundItemError::InvalidIdentity);
        }
        Ok(Self { uuid, wire_id })
    }
    /// Returns the supplied persistent UUID.
    pub const fn uuid(self) -> EntityUuid {
        self.uuid
    }
    /// Returns the supplied non-reusing router wire id.
    pub const fn wire_id(self) -> EntityId {
        self.wire_id
    }
}

/// Refused identity refill retaining its complete original vector.
#[derive(Debug)]
pub struct GroundItemIdentityRefusal {
    pub(crate) reason: GroundItemError,
    pub(crate) identities: Vec<GroundItemIdentity>,
}
impl GroundItemIdentityRefusal {
    /// Borrows the refusal classification.
    pub const fn reason(&self) -> &GroundItemError {
        &self.reason
    }
    /// Returns every unconsumed original identity.
    pub fn into_parts(self) -> (GroundItemError, Vec<GroundItemIdentity>) {
        (self.reason, self.identities)
    }
}

/// Move-owned stack queued for admission into the existing concrete entity store.
#[derive(Debug, Clone)]
pub struct GroundItemDrop {
    pub(crate) position: Vec3,
    pub(crate) stack: ItemStack,
    pub(crate) pickup_delay: i32,
    pub(crate) target: Option<EntityUuid>,
    pub(crate) thrower: Option<EntityUuid>,
}
impl GroundItemDrop {
    /// Describes a drop without granting admission or durable authority.
    pub fn new(
        position: Vec3,
        stack: ItemStack,
        pickup_delay: i32,
        target: Option<EntityUuid>,
        thrower: Option<EntityUuid>,
    ) -> Self {
        Self {
            position,
            stack,
            pickup_delay,
            target,
            thrower,
        }
    }
    /// Recovers all original values after a refused queue admission.
    pub fn into_parts(self) -> (Vec3, ItemStack, i32, Option<EntityUuid>, Option<EntityUuid>) {
        (
            self.position,
            self.stack,
            self.pickup_delay,
            self.target,
            self.thrower,
        )
    }
}

/// Refused whole-batch admission retains every original stack and property.
#[derive(Debug)]
pub struct GroundItemBatchRefusal {
    pub(crate) reason: GroundItemError,
    pub(crate) drops: Vec<GroundItemDrop>,
}
impl GroundItemBatchRefusal {
    /// Borrows the classified preflight refusal.
    pub const fn reason(&self) -> &GroundItemError {
        &self.reason
    }
    /// Recovers the complete original batch; no member was partially admitted.
    pub fn into_parts(self) -> (GroundItemError, Vec<GroundItemDrop>) {
        (self.reason, self.drops)
    }
}

/// Refused drop admission retains the complete input stack and properties.
#[derive(Debug)]
pub struct GroundItemDropRefusal {
    pub(crate) reason: GroundItemError,
    pub(crate) drop: GroundItemDrop,
}
impl GroundItemDropRefusal {
    /// Borrows the classified reason.
    pub const fn reason(&self) -> &GroundItemError {
        &self.reason
    }
    /// Returns the unmodified owned request.
    pub fn into_parts(self) -> (GroundItemError, GroundItemDrop) {
        (self.reason, self.drop)
    }
}

/// Single-use offer authority bound to one actual runtime incarnation.
///
/// This is not cloneable, serializable, a wire identity or an inventory grant.
#[derive(Debug)]
pub struct GroundItemPickupTicket {
    pub(crate) owner: Weak<()>,
    pub(crate) sequence: u64,
}

/// App's conserving response, owning every unit it could not deposit.
#[derive(Debug)]
pub struct GroundItemPickupResult {
    pub(crate) ticket: GroundItemPickupTicket,
    pub(crate) remainder: ItemStack,
}
impl GroundItemPickupResult {
    /// Returns the genuine ticket and the exact unconsumed stack to the owner.
    pub fn new(ticket: GroundItemPickupTicket, remainder: ItemStack) -> Self {
        Self { ticket, remainder }
    }
    /// Recovers both owned fields for retry after a refused enqueue.
    pub fn into_parts(self) -> (GroundItemPickupTicket, ItemStack) {
        (self.ticket, self.remainder)
    }
}

/// Refused response retains its ticket and complete remainder.
#[derive(Debug)]
pub struct GroundItemPickupRefusal {
    pub(crate) reason: GroundItemError,
    pub(crate) result: GroundItemPickupResult,
}
impl GroundItemPickupRefusal {
    /// Borrows the classified refusal.
    pub const fn reason(&self) -> &GroundItemError {
        &self.reason
    }
    /// Returns the intact response for retry or reconciliation.
    pub fn into_parts(self) -> (GroundItemError, GroundItemPickupResult) {
        (self.reason, self.result)
    }
}

/// Move-owned bounded runtime publications, drained once by the App driver.
#[derive(Debug)]
pub enum GroundItemEvent {
    /// Readonly spawn snapshot; the canonical payload remains in `EntityStore`.
    Spawn {
        /// Complete checked display state, with no pickup authority.
        snapshot: ItemEntitySnapshot,
    },
    /// The canonical entity moved after collision/residency checks.
    Move {
        /// Current base identity and transform.
        base: EntityData,
    },
    /// An intentional despawn or fully accepted pickup removed the entity.
    Remove {
        /// Removed entity identity, not a durable tombstone.
        base: EntityData,
    },
    /// The actual stack moved out of `EntityStore` for an App inventory deposit.
    /// App must return the complete stack if the player is absent/disconnected.
    PickupOffer {
        /// Unforgeable single-use response authority.
        ticket: GroundItemPickupTicket,
        /// Intended receiving player.
        player: PlayerId,
        /// Existing displayed item wire id.
        wire_id: EntityId,
        /// Sole live transferable payload, never a cloned inventory grant.
        stack: ItemStack,
    },
    /// A partial or refused pickup restored the exact remaining canonical stack.
    StackChanged {
        /// Complete checked display snapshot.
        snapshot: ItemEntitySnapshot,
    },
    /// App's response proved exactly how many units moved to its inventory.
    PickupAccepted {
        /// Receiving player named by the original offer.
        player: PlayerId,
        /// Existing item wire id, also for a full pickup's animation.
        wire_id: EntityId,
        /// Exact original count minus exact returned count.
        count: u8,
    },
}

#[derive(Debug)]
pub(crate) struct GroundOffer {
    pub(crate) base: EntityData,
    pub(crate) properties: ItemEntityProperties,
    pub(crate) player: PlayerId,
    pub(crate) before: Vec<u8>,
    pub(crate) count: u8,
    pub(crate) response_queued: bool,
}

#[derive(Debug)]
pub(crate) struct GroundItems {
    pub(crate) authority: Option<Arc<()>>,
    pub(crate) owner: ShardId,
    pub(crate) limits: GroundItemLimits,
    pub(crate) active: BTreeMap<EntityUuid, EntityHandle>,
    pub(crate) regions: BTreeMap<ShardId, crate::EntityStore>,
    pub(crate) region_claims: BTreeSet<ShardId>,
    pub(crate) restores: VecDeque<(usize, GroundItemRestore)>,
    pub(crate) restore_bytes: usize,
    pub(crate) unpublished: BTreeSet<EntityUuid>,
    pub(crate) cursor: Option<EntityUuid>,
    pub(crate) identities: VecDeque<GroundItemIdentity>,
    pub(crate) pending: VecDeque<(usize, GroundItemDrop)>,
    pub(crate) pending_bytes: usize,
    pub(crate) item_bytes: usize,
    pub(crate) owner_byte_budget: usize,
    pub(crate) offers: BTreeMap<u64, GroundOffer>,
    pub(crate) offer_bytes: usize,
    pub(crate) last_offer: u64,
    pub(crate) responses: VecDeque<GroundItemPickupResult>,
    pub(crate) events: VecDeque<(usize, GroundItemEvent)>,
    pub(crate) event_bytes: usize,
}

impl Clone for GroundItems {
    fn clone(&self) -> Self {
        // Snapshot copies grant no second pickup/drop/outbox producer. Pending
        // payloads remain bounded admitted snapshots; live canonical slots are
        // independently copied by EntityStore with a fresh store incarnation.
        Self {
            authority: None,
            owner: self.owner,
            limits: self.limits,
            active: BTreeMap::new(),
            regions: self.regions.clone(),
            region_claims: self.region_claims.clone(),
            restores: self
                .restores
                .iter()
                .map(|(size, restore)| {
                    (
                        *size,
                        GroundItemRestore::new(
                            restore.proposal.snapshot_admitted(),
                            restore.wire_id,
                        ),
                    )
                })
                .collect(),
            restore_bytes: self.restore_bytes,
            unpublished: BTreeSet::new(),
            cursor: None,
            identities: VecDeque::new(),
            pending: self.pending.clone(),
            pending_bytes: self.pending_bytes,
            item_bytes: self.item_bytes,
            owner_byte_budget: self.owner_byte_budget,
            offers: BTreeMap::new(),
            offer_bytes: 0,
            last_offer: self.last_offer,
            responses: VecDeque::new(),
            events: VecDeque::new(),
            event_bytes: 0,
        }
    }
}

impl GroundItems {
    pub(crate) fn new(owner: ShardId, limits: GroundItemLimits) -> Self {
        Self {
            authority: Some(Arc::new(())),
            owner,
            limits,
            active: BTreeMap::new(),
            regions: BTreeMap::new(),
            region_claims: BTreeSet::from([owner]),
            restores: VecDeque::with_capacity(GROUND_TICK_BUDGET),
            restore_bytes: 0,
            unpublished: BTreeSet::new(),
            cursor: None,
            identities: VecDeque::with_capacity(MAX_GROUND_IDENTITIES),
            pending: VecDeque::new(),
            pending_bytes: 0,
            item_bytes: 0,
            owner_byte_budget: 0,
            offers: BTreeMap::new(),
            offer_bytes: 0,
            last_offer: 0,
            responses: VecDeque::with_capacity(MAX_GROUND_OFFERS),
            events: VecDeque::with_capacity(MAX_GROUND_EVENTS),
            event_bytes: 0,
        }
    }

    pub(crate) fn check_live(&self) -> Result<(), GroundItemError> {
        if self.authority.is_none() {
            Err(GroundItemError::SnapshotOnly)
        } else {
            Ok(())
        }
    }

    pub(crate) fn can_publish(&self, size: usize, count: usize) -> bool {
        self.events.len() + count <= MAX_GROUND_EVENTS
            && self.event_bytes + size <= MAX_GROUND_EVENT_BYTES
    }

    pub(crate) fn publish(&mut self, size: usize, event: GroundItemEvent) {
        assert!(
            self.can_publish(size, 1),
            "ground item publication must have a reserved credit"
        );
        self.event_bytes += size;
        self.events.push_back((size, event));
    }

    pub(crate) fn reserve_drop_bytes(
        &self,
        stacks: &[ItemStack],
    ) -> Result<usize, GroundItemError> {
        self.reserve_drop_iter(stacks.iter())
    }

    pub(crate) fn reserve_drop_iter<'a>(
        &self,
        stacks: impl ExactSizeIterator<Item = &'a ItemStack>,
    ) -> Result<usize, GroundItemError> {
        self.check_live()?;
        if self.active.len()
            + self.offers.len()
            + self.pending.len()
            + self.restores.len()
            + stacks.len()
            > self.limits.items
        {
            return Err(GroundItemError::Capacity("item slots"));
        }
        let mut bytes = 0usize;
        for stack in stacks {
            if stack.item().is_none() {
                return Err(GroundItemError::InvalidRemainder);
            }
            let size = persisted_stack_size(stack)? + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES;
            if encode_item_entity_metadata(stack)?.as_bytes().len() > size {
                return Err(GroundItemError::Capacity("metadata reservation"));
            }
            if size > crate::MAX_NATIVE_ITEM_ENTITY_BYTES {
                return Err(GroundItemError::Capacity("native body"));
            }
            bytes = bytes
                .checked_add(size)
                .ok_or(GroundItemError::Capacity("pending bytes"))?;
        }
        if self.item_bytes + bytes > self.owner_byte_budget
            || self.item_bytes + bytes > self.limits.pending_bytes
        {
            return Err(GroundItemError::Capacity("reserved entity bytes"));
        }
        if bytes + self.pending_bytes > self.limits.pending_bytes {
            return Err(GroundItemError::Capacity("pending bytes"));
        }
        Ok(bytes)
    }

    pub(crate) fn admit_drop(&mut self, size: usize, drop: GroundItemDrop) {
        let owner = self.owner_for_position(drop.position);
        assert!(
            self.region_claims.contains(&owner) || self.region_claims.len() < MAX_GROUND_REGIONS,
            "ground drop region must have an admitted claim"
        );
        self.region_claims.insert(owner);
        self.pending_bytes += size;
        self.item_bytes += size;
        self.pending.push_back((size, drop));
    }

    pub(crate) fn remainder_matches(
        offer: &GroundOffer,
        remainder: &ItemStack,
    ) -> Result<(), GroundItemError> {
        let after = encode_persisted_stack(remainder)?;
        if remainder.item().is_none() {
            return Ok(());
        }
        if remainder.count() > offer.count || after.len() != offer.before.len() {
            return Err(GroundItemError::InvalidRemainder);
        }
        // FCIS1 has one fixed count byte after its canonical resource name.
        let length = u16::from_be_bytes([offer.before[7], offer.before[8]]) as usize;
        let count_offset = 9 + length;
        if after[..count_offset] != offer.before[..count_offset]
            || after[count_offset + 1..] != offer.before[count_offset + 1..]
        {
            return Err(GroundItemError::InvalidRemainder);
        }
        Ok(())
    }

    pub(crate) fn spawn_base(
        &self,
        identity: GroundItemIdentity,
        transform: EntityTransform,
    ) -> EntitySpawn {
        let Some(item_type) = ferrumc_registry::entity_type::EntityTypeId::from_name("item") else {
            panic!("pinned item registry is missing");
        };
        let owner = crate::ShardPartitioner::for_chunk(
            self.owner.world(),
            self.owner.dimension(),
            transform.chunk(),
        );
        EntitySpawn::new(
            owner.world(),
            owner.dimension(),
            identity.uuid,
            identity.wire_id,
            item_type,
            transform,
        )
    }

    pub(crate) fn proposal(
        &self,
        identity: GroundItemIdentity,
        drop: GroundItemDrop,
    ) -> Result<ItemEntitySpawn, GroundItemDropRefusal> {
        let checked = (|| {
            let transform =
                EntityTransform::try_new(drop.position, 0.0, 0.0).map_err(ItemEntityError::from)?;
            let properties = ItemEntityProperties::try_new(
                Vec3::new(0.0, 0.2, 0.0),
                0.0,
                crate::ItemEntityCounters::new(0, drop.pickup_delay, 5),
                drop.target,
                drop.thrower,
            )?;
            Ok::<_, GroundItemError>((transform, properties))
        })();
        match checked {
            Ok((transform, properties)) => Ok(ItemEntitySpawn::new(
                self.spawn_base(identity, transform),
                drop.stack,
                properties,
            )),
            Err(reason) => Err(GroundItemDropRefusal { reason, drop }),
        }
    }

    pub(crate) fn owner_for_position(&self, position: Vec3) -> ShardId {
        let block = ferrumc_math::BlockPos::new(
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        );
        crate::ShardPartitioner::for_block(self.owner.world(), self.owner.dimension(), block)
    }

    pub(crate) fn check_region(&self, owner: ShardId) -> Result<(), GroundItemError> {
        if !self.region_claims.contains(&owner) && self.region_claims.len() >= MAX_GROUND_REGIONS {
            Err(GroundItemError::Capacity("canonical region owners"))
        } else {
            Ok(())
        }
    }

    pub(crate) fn ensure_region(&mut self, owner: ShardId) -> Result<(), GroundItemError> {
        self.check_region(owner)?;
        if owner != self.owner && !self.regions.contains_key(&owner) {
            let slots =
                NonZeroUsize::new(self.limits.items).ok_or(GroundItemError::InvalidLimit)?;
            let bytes =
                NonZeroUsize::new(self.owner_byte_budget).ok_or(GroundItemError::InvalidLimit)?;
            let store = crate::EntityStore::with_item_byte_budget(owner, slots, bytes)?;
            self.regions.insert(owner, store);
        }
        self.region_claims.insert(owner);
        Ok(())
    }
}

pub(crate) fn regional_store<'a>(
    primary: &'a crate::EntityStore,
    regions: &'a BTreeMap<ShardId, crate::EntityStore>,
    owner: ShardId,
) -> Option<&'a crate::EntityStore> {
    if primary.owner() == owner {
        Some(primary)
    } else {
        regions.get(&owner)
    }
}

pub(crate) fn regional_store_mut<'a>(
    primary: &'a mut crate::EntityStore,
    regions: &'a mut BTreeMap<ShardId, crate::EntityStore>,
    owner: ShardId,
) -> Option<&'a mut crate::EntityStore> {
    if primary.owner() == owner {
        Some(primary)
    } else {
        regions.get_mut(&owner)
    }
}
