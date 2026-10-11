//! Concrete item payloads in the existing owner slots, never a second owner.

pub(super) mod codec;
#[cfg(test)]
mod tests;

use std::num::NonZeroUsize;
use std::sync::Arc;

use ferrumc_core::{EntityId, EntityUuid};
use ferrumc_items::{encode_persisted_stack, ItemPersistenceError, ItemStack};
use ferrumc_math::{ChunkPos, Vec3};
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_storage::{PersistentEntityKey, PersistentEntityRecord, StorageError};

use super::{
    EntityData, EntityHandle, EntitySpawn, EntityStore, EntityStoreError, EntityTickContext,
    EntityTransform, EntityTransformComponent,
};
use crate::ShardId;

pub(crate) use codec::checked_native_header;
pub use codec::decode_item_entity;

/// Maximum native item body, including the entire header and FCIS stack.
pub const MAX_NATIVE_ITEM_ENTITY_BYTES: usize = ferrumc_storage::MAX_ENTITY_DATA_LEN;
/// Fixed bytes preceding the FCIS payload in native FCIE schema 1.
pub const NATIVE_ITEM_ENTITY_HEADER_BYTES: usize = 144;
/// Explicit maximum aggregate native item bytes in one owner store (16 MiB).
///
/// This bounds encoded bodies, not decoded heap, slots, indexes or snapshots.
pub const MAX_ITEM_ENTITY_BYTES_PER_SHARD: usize = 16 * 1024 * 1024;

/// A non-finite native item property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPropertyComponent {
    /// Motion along x.
    MotionX,
    /// Motion along y.
    MotionY,
    /// Motion along z.
    MotionZ,
    /// Visual bob offset.
    BobOffset,
}

/// The independent UUID reference in a malformed native body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemReferenceField {
    /// UUID restricting player pickup.
    OwnerTarget,
    /// UUID identifying the thrower, separate from the pickup owner.
    Thrower,
}

/// An outer UUID key field that differs from the native header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemNativeKeyField {
    /// World scope.
    World,
    /// Dimension scope.
    Dimension,
    /// Persistent UUID.
    Uuid,
}

/// Classified, non-normalizing item admission, snapshot or codec refusal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ItemEntityError {
    /// Existing owner/handle/index admission reason.
    #[error(transparent)]
    Store(#[from] EntityStoreError),
    /// Full lossless stack validation failed.
    #[error(transparent)]
    Stack(#[from] ItemPersistenceError),
    /// Opaque bounded storage record construction failed.
    #[error(transparent)]
    StorageRecord(#[from] StorageError),
    /// A concrete item operation was attempted on a different registered type.
    #[error("item operation on entity type {actual:?}")]
    WrongEntityType {
        /// Registered type that was refused.
        actual: EntityTypeId,
    },
    /// A legacy base-only item registration is an explicit placeholder.
    #[error("item registration has no owned item state")]
    MissingItemState,
    /// Empty stacks cannot be admitted as concrete live item entities.
    #[error("empty item entity stack")]
    EmptyStack,
    /// Native finite policy never silently sanitizes invalid motion/bob.
    #[error("non-finite item property {component:?}")]
    NonFiniteProperty {
        /// Refused field.
        component: ItemPropertyComponent,
    },
    /// Explicit aggregate allowance exceeds the resource ceiling.
    #[error("item byte budget {requested} exceeds {max}")]
    InvalidItemByteBudget {
        /// Requested bytes.
        requested: usize,
        /// Hard ceiling.
        max: usize,
    },
    /// A valid complete payload cannot fit the owner's explicit allowance.
    #[error("item bytes {required} exceed budget {limit}")]
    ItemByteBudgetExceeded {
        /// Complete required native bytes.
        required: usize,
        /// Explicit owner allowance.
        limit: usize,
    },
    /// A valid FCIS stack may still exceed the stricter native body cap.
    #[error("native item body {size} exceeds {max}")]
    NativeBodyTooLarge {
        /// Entire native body size.
        size: usize,
        /// Whole-body limit.
        max: usize,
    },
    /// A sealed state's cached stack length no longer matches checked encoding.
    #[error("cached item length {cached} differs from {actual}")]
    CachedItemLengthMismatch {
        /// Cached FCIS bytes.
        cached: usize,
        /// Revalidated FCIS bytes.
        actual: usize,
    },
    /// Stored aggregate accounting differs from the revalidated live sum.
    #[error("cached item aggregate {cached} differs from {actual}")]
    AggregateItemBytesMismatch {
        /// Cached owner usage.
        cached: usize,
        /// Revalidated live usage.
        actual: usize,
    },
    /// Storage payload schema is independent of native/FCIS/store versions.
    #[error("unsupported item payload schema {found}")]
    UnsupportedPayloadSchema {
        /// Stored payload schema.
        found: u32,
    },
    /// Native item body lacks its FCIE signature.
    #[error("invalid native item signature")]
    InvalidMagic,
    /// No decoder exists for this FCIE version.
    #[error("unsupported native item version {found}")]
    UnsupportedNativeVersion {
        /// Stored FCIE version.
        found: u16,
    },
    /// Native header ended before a fixed field could be read.
    #[error("native item truncated at {offset}, requires {needed} bytes")]
    TruncatedNative {
        /// Field's start offset.
        offset: usize,
        /// Required field bytes.
        needed: usize,
    },
    /// Header type is unknown or is not the pinned registered item type.
    #[error("invalid native item entity type {raw}")]
    InvalidNativeEntityType {
        /// Unmodified header type id.
        raw: i32,
    },
    /// Header identity must agree exactly with the outer UUID key.
    #[error("native item key mismatch: {field:?}")]
    NativeKeyMismatch {
        /// Differing typed key field.
        field: ItemNativeKeyField,
    },
    /// Floored position differs from the outer column.
    #[error("native item chunk {actual:?} differs from outer {expected:?}")]
    NativeChunkMismatch {
        /// Outer checked column.
        expected: ChunkPos,
        /// Position-derived column.
        actual: ChunkPos,
    },
    /// Loading must not silently normalize stored rotation bits.
    #[error("noncanonical native transform {component:?}")]
    NonCanonicalTransform {
        /// Field changed by checked transform construction.
        component: EntityTransformComponent,
    },
    /// UUID presence tags admit only zero and one.
    #[error("invalid {field:?} presence {value}")]
    InvalidUuidPresence {
        /// Independent reference.
        field: ItemReferenceField,
        /// Unrecognized tag.
        value: u8,
    },
    /// Absent references require sixteen canonical zero reserved bytes.
    #[error("nonzero absent {field:?} UUID")]
    NonZeroAbsentUuid {
        /// Independent reference.
        field: ItemReferenceField,
    },
    /// Declared FCIS length must consume exactly the remainder within body cap.
    #[error("FCIS length {declared} differs from remaining {remaining}")]
    InvalidFcisLength {
        /// Unmodified length field.
        declared: u32,
        /// Actual bytes following the header.
        remaining: usize,
    },
    /// Decoding/re-encoding changed native FCIS bytes; no lossy fallback exists.
    #[error("noncanonical native FCIS bytes")]
    NonCanonicalFcis,
}

/// Complete signed Java runtime item counters, without NBT short truncation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemEntityCounters {
    age: i32,
    pickup_delay: i32,
    health: i32,
}

impl ItemEntityCounters {
    /// Preserves every signed value including wrapping boundaries and sentinels.
    pub const fn new(age: i32, pickup_delay: i32, health: i32) -> Self {
        Self {
            age,
            pickup_delay,
            health,
        }
    }
    /// Returns the entire runtime age.
    pub const fn age(self) -> i32 {
        self.age
    }
    /// Returns the entire runtime pickup delay.
    pub const fn pickup_delay(self) -> i32 {
        self.pickup_delay
    }
    /// Returns the entire runtime health; timer steps never cause health death.
    pub const fn health(self) -> i32 {
        self.health
    }
}

/// Sealed finite native properties; target and thrower are independent UUIDs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemEntityProperties {
    motion: Vec3,
    bob_offset: f32,
    counters: ItemEntityCounters,
    owner_target: Option<EntityUuid>,
    thrower: Option<EntityUuid>,
}

impl ItemEntityProperties {
    /// Admits finite motion/bob without changing any bits, including signed zero.
    ///
    /// Every finite motion magnitude is preserved, including values above ten;
    /// native storage does not apply Vanilla NBT load's magnitude sanitization.
    pub fn try_new(
        motion: Vec3,
        bob_offset: f32,
        counters: ItemEntityCounters,
        owner_target: Option<EntityUuid>,
        thrower: Option<EntityUuid>,
    ) -> Result<Self, ItemEntityError> {
        let properties = Self {
            motion,
            bob_offset,
            counters,
            owner_target,
            thrower,
        };
        properties.validate()?;
        Ok(properties)
    }
    pub(in crate::entity) fn with_counters(self, counters: ItemEntityCounters) -> Self {
        Self { counters, ..self }
    }
    pub(super) fn validate(self) -> Result<(), ItemEntityError> {
        for (value, component) in [
            (self.motion.x, ItemPropertyComponent::MotionX),
            (self.motion.y, ItemPropertyComponent::MotionY),
            (self.motion.z, ItemPropertyComponent::MotionZ),
            (f64::from(self.bob_offset), ItemPropertyComponent::BobOffset),
        ] {
            if !value.is_finite() {
                return Err(ItemEntityError::NonFiniteProperty { component });
            }
        }
        Ok(())
    }
    /// Returns all finite motion values without normalization.
    pub const fn motion(self) -> Vec3 {
        self.motion
    }
    /// Returns the preserved finite bob offset.
    pub const fn bob_offset(self) -> f32 {
        self.bob_offset
    }
    /// Returns the signed runtime counters.
    pub const fn counters(self) -> ItemEntityCounters {
        self.counters
    }
    /// Returns the UUID restricting pickup, independent of thrower.
    pub const fn owner_target(self) -> Option<EntityUuid> {
        self.owner_target
    }
    /// Returns the thrower's persistent UUID without resolving a live pointer.
    pub const fn thrower(self) -> Option<EntityUuid> {
        self.thrower
    }
}

/// One owned stack and its sealed properties in an existing entity owner slot.
///
/// There is no raw constructor, mutable stack accessor or public Clone.
/// ```compile_fail
/// use ferrumc_sim::ItemEntityState;
/// fn mutate(state: &mut ItemEntityState) { state.stack().components().added().clear(); }
/// ```
#[derive(Debug)]
pub struct ItemEntityState {
    pub(super) stack: ItemStack,
    pub(super) properties: ItemEntityProperties,
    pub(super) fcis_len: usize,
}

impl ItemEntityState {
    /// Borrows the entire authoritative stack, never a normalized view.
    pub const fn stack(&self) -> &ItemStack {
        &self.stack
    }
    /// Returns the sealed properties.
    pub const fn properties(&self) -> ItemEntityProperties {
        self.properties
    }
    /// Returns the admitted whole-body length including native header.
    pub const fn native_body_len(&self) -> usize {
        NATIVE_ITEM_ENTITY_HEADER_BYTES + self.fcis_len
    }
    /// Returns ownership of the unchanged stack and all properties.
    pub fn into_parts(self) -> (ItemStack, ItemEntityProperties) {
        (self.stack, self.properties)
    }
    pub(super) fn snapshot_admitted(&self) -> Self {
        Self {
            stack: self.stack.clone(),
            properties: self.properties,
            fcis_len: self.fcis_len,
        }
    }
    fn checked_bytes(&self) -> Result<Vec<u8>, ItemEntityError> {
        self.properties.validate()?;
        let bytes = checked_stack_bytes(&self.stack)?;
        if bytes.len() != self.fcis_len {
            return Err(ItemEntityError::CachedItemLengthMismatch {
                cached: self.fcis_len,
                actual: bytes.len(),
            });
        }
        Ok(bytes)
    }
}

/// Move-owned admission proposal; construction itself grants no owner authority.
#[derive(Debug)]
pub struct ItemEntitySpawn {
    base: EntitySpawn,
    stack: ItemStack,
    properties: ItemEntityProperties,
}
impl ItemEntitySpawn {
    /// Retains all input ownership until tick-context checked admission succeeds.
    pub const fn new(
        base: EntitySpawn,
        stack: ItemStack,
        properties: ItemEntityProperties,
    ) -> Self {
        Self {
            base,
            stack,
            properties,
        }
    }
    /// Returns the explicit base identity/transform proposal.
    pub const fn base(&self) -> EntitySpawn {
        self.base
    }
    /// Borrows the unmodified proposed stack.
    pub const fn stack(&self) -> &ItemStack {
        &self.stack
    }
    /// Returns the sealed proposed properties.
    pub const fn properties(&self) -> ItemEntityProperties {
        self.properties
    }
    /// Returns every proposed value without cloning or disposal.
    pub fn into_parts(self) -> (EntitySpawn, ItemStack, ItemEntityProperties) {
        (self.base, self.stack, self.properties)
    }
}

/// Failed admission retaining the complete move-owned spawn proposal.
#[derive(Debug)]
pub struct ItemEntitySpawnRefusal {
    reason: ItemEntityError,
    proposal: Box<ItemEntitySpawn>,
}
impl ItemEntitySpawnRefusal {
    fn new(reason: ItemEntityError, proposal: ItemEntitySpawn) -> Self {
        Self {
            reason,
            proposal: Box::new(proposal),
        }
    }
    /// Borrows the classified admission reason.
    pub const fn reason(&self) -> &ItemEntityError {
        &self.reason
    }
    /// Recovers both the reason and every original input.
    pub fn into_parts(self) -> (ItemEntityError, ItemEntitySpawn) {
        (self.reason, *self.proposal)
    }
}

/// Failed replacement retaining the entire incoming stack and existing owner.
#[derive(Debug)]
pub struct ItemStackRefusal {
    reason: ItemEntityError,
    stack: ItemStack,
}
impl ItemStackRefusal {
    /// Borrows the classified replacement reason.
    pub const fn reason(&self) -> &ItemEntityError {
        &self.reason
    }
    /// Returns the entire original incoming stack without normalization.
    pub fn into_parts(self) -> (ItemEntityError, ItemStack) {
        (self.reason, self.stack)
    }
}

/// Owned removal result containing base data and any entire item payload.
#[derive(Debug)]
pub struct RemovedEntity {
    base: EntityData,
    item: Option<ItemEntityState>,
}
impl RemovedEntity {
    /// Returns the base record; this is not a durable delete acknowledgement.
    pub const fn base(&self) -> EntityData {
        self.base
    }
    /// Borrows any complete owned payload.
    pub const fn item_state(&self) -> Option<&ItemEntityState> {
        self.item.as_ref()
    }
    /// Moves every removed value to the caller, without silent item disposal.
    pub fn into_parts(self) -> (EntityData, Option<ItemEntityState>) {
        (self.base, self.item)
    }
}

/// Checked item snapshot, without permission to activate a competing owner.
#[derive(Debug)]
pub struct ItemEntitySnapshot {
    base: EntityData,
    item: ItemEntityState,
}
impl ItemEntitySnapshot {
    /// Returns the snapshotted base data, including its transient wire reference.
    pub const fn base(&self) -> EntityData {
        self.base
    }
    /// Borrows the complete independently copied item state.
    pub const fn item_state(&self) -> &ItemEntityState {
        &self.item
    }
    /// Returns ownership of the checked base and entire copied payload.
    pub fn into_parts(self) -> (EntityData, ItemEntityState) {
        (self.base, self.item)
    }
}

/// Validated native restore proposal with persistent identity and no wire ID.
#[derive(Debug)]
pub struct RestoredItemEntity {
    key: PersistentEntityKey,
    entity_type: EntityTypeId,
    transform: EntityTransform,
    item: ItemEntityState,
}
impl RestoredItemEntity {
    /// Returns the validated persistent scope and UUID.
    pub const fn key(&self) -> PersistentEntityKey {
        self.key
    }
    /// Returns the registered item type.
    pub const fn entity_type(&self) -> EntityTypeId {
        self.entity_type
    }
    /// Returns the validated stored transform.
    pub const fn transform(&self) -> EntityTransform {
        self.transform
    }
    /// Borrows the complete decoded state.
    pub const fn item_state(&self) -> &ItemEntityState {
        &self.item
    }
    /// Returns every persistent field and owned payload; no wire ID is invented.
    pub fn into_parts(
        self,
    ) -> (
        PersistentEntityKey,
        EntityTypeId,
        EntityTransform,
        ItemStack,
        ItemEntityProperties,
    ) {
        (
            self.key,
            self.entity_type,
            self.transform,
            self.item.stack,
            self.item.properties,
        )
    }
}

/// Failed restore returning the complete proposal and supplied new wire ID.
#[derive(Debug)]
pub struct ItemEntityRestoreRefusal {
    reason: ItemEntityError,
    proposal: Box<RestoredItemEntity>,
    new_wire_id: EntityId,
}
impl ItemEntityRestoreRefusal {
    fn new(reason: ItemEntityError, proposal: RestoredItemEntity, new_wire_id: EntityId) -> Self {
        Self {
            reason,
            proposal: Box::new(proposal),
            new_wire_id,
        }
    }
    /// Borrows the classified restore reason.
    pub const fn reason(&self) -> &ItemEntityError {
        &self.reason
    }
    /// Recovers the unmodified proposal and explicit supplied wire ID.
    pub fn into_parts(self) -> (ItemEntityError, RestoredItemEntity, EntityId) {
        (self.reason, *self.proposal, self.new_wire_id)
    }
}

/// Caller-selected counter projection; this does not calculate activation range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemCounterMode {
    /// Projection of the nonempty active path, without physics/merge phases.
    Active,
    /// Projection of Paper EAR inactiveTick, distinct from unloaded/offline.
    Inactive,
}
/// Bounded decision; no item is removed and no event is emitted by a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemDespawnDecision {
    /// Age is below the exact signed configured threshold.
    Keep,
    /// Caller must process its despawn event/cancellation before owned removal.
    Due,
}
/// Pure counter projection and its separate despawn decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemCounterStep {
    counters: ItemEntityCounters,
    decision: ItemDespawnDecision,
}
impl ItemCounterStep {
    /// Returns the projected signed counters.
    pub const fn counters(self) -> ItemEntityCounters {
        self.counters
    }
    /// Returns the decision without mutating an entity or dispatching an event.
    pub const fn decision(self) -> ItemDespawnDecision {
        self.decision
    }
}

/// Projects one nonempty active tick's counters with Java int wrapping.
///
/// Full active ticks decrement delay before physics/merge and increment age
/// afterwards. This combined projection cannot replace those interleaved phases.
pub fn step_active_item_counters(
    counters: ItemEntityCounters,
    despawn_rate: i32,
) -> ItemCounterStep {
    step_item_counters(counters, despawn_rate)
}
/// Projects one EAR inactive tick, never wall-clock catch-up or an unloaded step.
pub fn step_inactive_item_counters(
    counters: ItemEntityCounters,
    despawn_rate: i32,
) -> ItemCounterStep {
    step_item_counters(counters, despawn_rate)
}
fn step_item_counters(mut counters: ItemEntityCounters, despawn_rate: i32) -> ItemCounterStep {
    if counters.pickup_delay > 0 && counters.pickup_delay != 32767 {
        counters.pickup_delay -= 1;
    }
    if counters.age != -32768 {
        counters.age = counters.age.wrapping_add(1);
    }
    let decision = if counters.age >= despawn_rate {
        ItemDespawnDecision::Due
    } else {
        ItemDespawnDecision::Keep
    };
    ItemCounterStep { counters, decision }
}

pub(super) fn require_item_type(entity_type: EntityTypeId) -> Result<(), ItemEntityError> {
    if entity_type.resource_location() != "minecraft:item" {
        return Err(ItemEntityError::WrongEntityType {
            actual: entity_type,
        });
    }
    Ok(())
}

pub(super) fn checked_stack_bytes(stack: &ItemStack) -> Result<Vec<u8>, ItemEntityError> {
    if stack.item().is_none() {
        return Err(ItemEntityError::EmptyStack);
    }
    let bytes = encode_persisted_stack(stack)?;
    let size = NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len();
    if size > MAX_NATIVE_ITEM_ENTITY_BYTES {
        return Err(ItemEntityError::NativeBodyTooLarge {
            size,
            max: MAX_NATIVE_ITEM_ENTITY_BYTES,
        });
    }
    Ok(bytes)
}

impl EntityStore {
    /// Creates one lazy bounded owner with an explicit aggregate item allowance.
    pub fn with_item_byte_budget(
        owner: ShardId,
        capacity: NonZeroUsize,
        item_bytes: NonZeroUsize,
    ) -> Result<Self, ItemEntityError> {
        if item_bytes.get() > MAX_ITEM_ENTITY_BYTES_PER_SHARD {
            return Err(ItemEntityError::InvalidItemByteBudget {
                requested: item_bytes.get(),
                max: MAX_ITEM_ENTITY_BYTES_PER_SHARD,
            });
        }
        let mut store = Self::new(owner, capacity)?;
        store.item_byte_budget = item_bytes.get();
        Ok(store)
    }
    /// Returns the explicit encoded-body allowance; legacy construction has zero.
    pub const fn item_byte_budget(&self) -> usize {
        self.item_byte_budget
    }
    /// Returns the sum of complete admitted native item bodies, not RSS.
    pub const fn item_bytes_used(&self) -> usize {
        self.item_bytes_used
    }

    pub(super) fn item_index(&self, handle: &EntityHandle) -> Result<usize, ItemEntityError> {
        let index = self.index(handle)?;
        let base = self.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        require_item_type(base.entity_type)?;
        if self.slots[index].item.is_none() {
            return Err(ItemEntityError::MissingItemState);
        }
        Ok(index)
    }
    /// Borrows a concrete payload, refusing foreign/stale and placeholder handles.
    pub fn item_state(&self, handle: &EntityHandle) -> Result<&ItemEntityState, ItemEntityError> {
        let index = self.item_index(handle)?;
        self.slots[index]
            .item
            .as_ref()
            .ok_or(ItemEntityError::MissingItemState)
    }
    pub(super) fn checked_item_bytes(&self, index: usize) -> Result<Vec<u8>, ItemEntityError> {
        let base = self.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        require_item_type(base.entity_type)?;
        if base.owner != self.owner {
            return Err(EntityStoreError::ForeignHandle.into());
        }
        self.bucket(base.chunk())?;
        codec::validate_transform(base.transform)?;
        self.slots[index]
            .item
            .as_ref()
            .ok_or(ItemEntityError::MissingItemState)?
            .checked_bytes()
    }
    fn measured_item_bytes(&self) -> Result<usize, ItemEntityError> {
        let mut total = 0;
        for (index, slot) in self.slots.iter().enumerate() {
            if slot.item.is_some() {
                let bytes = self.checked_item_bytes(index)?;
                total += NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len();
                // Stop before the next addition: bounded by 16 MiB + one body.
                if total > self.item_byte_budget {
                    return Err(ItemEntityError::ItemByteBudgetExceeded {
                        required: total,
                        limit: self.item_byte_budget,
                    });
                }
            }
        }
        Ok(total)
    }
    fn validate_item_accounting(&self) -> Result<(), ItemEntityError> {
        let actual = self.measured_item_bytes()?;
        if actual != self.item_bytes_used {
            return Err(ItemEntityError::AggregateItemBytesMismatch {
                cached: self.item_bytes_used,
                actual,
            });
        }
        Ok(())
    }
    fn validate_cached_item_accounting(&self) -> Result<(), ItemEntityError> {
        let mut actual = 0;
        for item in self.slots.iter().filter_map(|slot| slot.item.as_ref()) {
            let size = NATIVE_ITEM_ENTITY_HEADER_BYTES
                .checked_add(item.fcis_len)
                .ok_or(ItemEntityError::NativeBodyTooLarge {
                    size: usize::MAX,
                    max: MAX_NATIVE_ITEM_ENTITY_BYTES,
                })?;
            if size > MAX_NATIVE_ITEM_ENTITY_BYTES {
                return Err(ItemEntityError::NativeBodyTooLarge {
                    size,
                    max: MAX_NATIVE_ITEM_ENTITY_BYTES,
                });
            }
            actual += size;
            if actual > self.item_byte_budget {
                return Err(ItemEntityError::ItemByteBudgetExceeded {
                    required: actual,
                    limit: self.item_byte_budget,
                });
            }
        }
        if actual != self.item_bytes_used {
            return Err(ItemEntityError::AggregateItemBytesMismatch {
                cached: self.item_bytes_used,
                actual,
            });
        }
        Ok(())
    }
    pub(super) fn replaced_item_bytes(
        &self,
        old: usize,
        new: usize,
    ) -> Result<usize, ItemEntityError> {
        let Some(remaining) = self.item_bytes_used.checked_sub(old) else {
            let actual = self.measured_item_bytes()?;
            return Err(ItemEntityError::AggregateItemBytesMismatch {
                cached: self.item_bytes_used,
                actual,
            });
        };
        // Admitted usage <=16 MiB and one new body <=256 KiB, even on 32-bit.
        let required =
            remaining
                .checked_add(new)
                .ok_or(ItemEntityError::ItemByteBudgetExceeded {
                    required: usize::MAX,
                    limit: self.item_byte_budget,
                })?;
        if required > self.item_byte_budget {
            return Err(ItemEntityError::ItemByteBudgetExceeded {
                required,
                limit: self.item_byte_budget,
            });
        }
        Ok(required)
    }
    /// Revalidates every payload and aggregate before any stack is copied.
    ///
    /// The result has a fresh incarnation, not competing active owner authority.
    pub fn try_snapshot(&self) -> Result<Self, ItemEntityError> {
        self.validate_item_accounting()?;
        self.validate_confirmed_accounting()?;
        Ok(Self {
            owner: self.owner,
            capacity: self.capacity,
            item_byte_budget: self.item_byte_budget,
            item_bytes_used: self.item_bytes_used,
            incarnation: Arc::new(()),
            slots: self
                .slots
                .iter()
                .map(super::Slot::snapshot_admitted)
                .collect(),
            free: self.free,
            retired: self.retired,
            by_uuid: self.by_uuid.clone(),
            by_wire: self.by_wire.clone(),
            buckets: self.buckets,
            ownership_mode: self.ownership_mode,
            snapshot_only: self.ownership_mode || self.snapshot_only,
            reservation: None,
            confirmed_item_bytes: self.confirmed_item_bytes,
            confirmed_item_cap: self.confirmed_item_cap,
        })
    }
    /// Takes a checked copy of one item, including after its chunk unloads.
    pub fn snapshot_item(
        &self,
        handle: &EntityHandle,
    ) -> Result<ItemEntitySnapshot, ItemEntityError> {
        let index = self.item_index(handle)?;
        drop(self.checked_item_bytes(index)?);
        self.validate_cached_item_accounting()?;
        let base = self.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        let item = self.slots[index]
            .item
            .as_ref()
            .ok_or(ItemEntityError::MissingItemState)?
            .snapshot_admitted();
        Ok(ItemEntitySnapshot { base, item })
    }
    /// Captures one checked native UUID record without I/O, dirty clear or save ack.
    pub fn capture_item(
        &self,
        handle: &EntityHandle,
    ) -> Result<(PersistentEntityKey, PersistentEntityRecord), ItemEntityError> {
        let index = self.item_index(handle)?;
        let bytes = self.checked_item_bytes(index)?;
        self.validate_cached_item_accounting()?;
        let base = self.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        let item = self.slots[index]
            .item
            .as_ref()
            .ok_or(ItemEntityError::MissingItemState)?;
        codec::encode_record(base, item.properties, &bytes)
    }
}

impl EntityTickContext<'_> {
    fn resident_item_index(&self, handle: &EntityHandle) -> Result<usize, ItemEntityError> {
        self.store.require_legacy_write()?;
        let index = self.store.item_index(handle)?;
        let base = self.store.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        self.require_resident(base.chunk())?;
        Ok(index)
    }
    fn prepare_item_spawn(
        &self,
        base: EntitySpawn,
        stack: &ItemStack,
        properties: ItemEntityProperties,
    ) -> Result<(usize, usize, usize), ItemEntityError> {
        self.store.require_legacy_write()?;
        if base.world != self.store.owner.world() || base.dimension != self.store.owner.dimension()
        {
            return Err(EntityStoreError::ScopeMismatch {
                owner: self.store.owner,
                world: base.world,
                dimension: base.dimension,
            }
            .into());
        }
        require_item_type(base.entity_type)?;
        let bucket = self.store.validate_spawn(base)?;
        self.require_resident(base.transform.chunk())?;
        properties.validate()?;
        let bytes = checked_stack_bytes(stack)?;
        let used = self
            .store
            .replaced_item_bytes(0, NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len())?;
        Ok((bucket, bytes.len(), used))
    }
    /// Admits an entire owned payload atomically, returning all inputs on refusal.
    ///
    /// There is deliberately no corresponding public mutable `EntityStore` method.
    /// ```compile_fail
    /// use ferrumc_sim::{EntityStore, ItemEntitySpawn};
    /// fn bypass(store: &mut EntityStore, item: ItemEntitySpawn) { store.spawn_item(item); }
    /// ```
    pub fn spawn_item(
        &mut self,
        proposal: ItemEntitySpawn,
    ) -> Result<EntityHandle, ItemEntitySpawnRefusal> {
        let (bucket, fcis_len, used) =
            match self.prepare_item_spawn(proposal.base, &proposal.stack, proposal.properties) {
                Ok(prepared) => prepared,
                Err(reason) => return Err(ItemEntitySpawnRefusal::new(reason, proposal)),
            };
        if let Err(error) = self.store.reserve_spawn_slot() {
            return Err(ItemEntitySpawnRefusal::new(error.into(), proposal));
        }
        let item = ItemEntityState {
            stack: proposal.stack,
            properties: proposal.properties,
            fcis_len,
        };
        let handle = self.store.commit_spawn(proposal.base, Some(item), bucket);
        self.store.item_bytes_used = used;
        Ok(handle)
    }
    /// Replaces a complete stack, returning old ownership or the intact refused input.
    pub fn replace_item_stack(
        &mut self,
        handle: &EntityHandle,
        stack: ItemStack,
    ) -> Result<ItemStack, ItemStackRefusal> {
        let prepared = (|| {
            let index = self.resident_item_index(handle)?;
            let old_len =
                NATIVE_ITEM_ENTITY_HEADER_BYTES + self.store.checked_item_bytes(index)?.len();
            let new_len = checked_stack_bytes(&stack)?.len();
            let used = self
                .store
                .replaced_item_bytes(old_len, NATIVE_ITEM_ENTITY_HEADER_BYTES + new_len)?;
            Ok::<_, ItemEntityError>((index, new_len, used))
        })();
        let (index, fcis_len, used) = match prepared {
            Ok(prepared) => prepared,
            Err(reason) => return Err(ItemStackRefusal { reason, stack }),
        };
        let Some(item) = self.store.slots[index].item.as_mut() else {
            return Err(ItemStackRefusal {
                reason: ItemEntityError::MissingItemState,
                stack,
            });
        };
        let old = std::mem::replace(&mut item.stack, stack);
        item.fcis_len = fcis_len;
        self.store.item_bytes_used = used;
        Ok(old)
    }
    /// Replaces sealed Copy properties without a stack clone or event dispatch.
    pub fn set_item_properties(
        &mut self,
        handle: &EntityHandle,
        properties: ItemEntityProperties,
    ) -> Result<ItemEntityProperties, ItemEntityError> {
        let index = self.resident_item_index(handle)?;
        properties.validate()?;
        let item = self.store.slots[index]
            .item
            .as_mut()
            .ok_or(ItemEntityError::MissingItemState)?;
        Ok(std::mem::replace(&mut item.properties, properties))
    }
    /// Removes base plus complete payload, including cleanup after ticket unload.
    pub fn remove_owned(
        &mut self,
        handle: &EntityHandle,
    ) -> Result<RemovedEntity, ItemEntityError> {
        self.store.require_legacy_write()?;
        let index = self.store.index(handle)?;
        let base = self.store.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        let bucket = self.store.bucket(base.chunk())?;
        let old_len = if self.store.slots[index].item.is_some() {
            NATIVE_ITEM_ENTITY_HEADER_BYTES + self.store.checked_item_bytes(index)?.len()
        } else {
            0
        };
        let used = self.store.replaced_item_bytes(old_len, 0)?;
        let (base, item) = self.store.remove_slot(index, base, bucket);
        self.store.item_bytes_used = used;
        Ok(RemovedEntity { base, item })
    }
    /// Restores an exact checked UUID payload with an explicit newly supplied wire ID.
    pub fn restore_item(
        &mut self,
        proposal: RestoredItemEntity,
        new_wire_id: EntityId,
    ) -> Result<EntityHandle, ItemEntityRestoreRefusal> {
        let base = EntitySpawn::new(
            proposal.key.world(),
            proposal.key.dimension(),
            proposal.key.uuid(),
            new_wire_id,
            proposal.entity_type,
            proposal.transform,
        );
        let (bucket, fcis_len, used) =
            match self.prepare_item_spawn(base, &proposal.item.stack, proposal.item.properties) {
                Ok(prepared) if prepared.1 == proposal.item.fcis_len => prepared,
                Ok(prepared) => {
                    return Err(ItemEntityRestoreRefusal::new(
                        ItemEntityError::CachedItemLengthMismatch {
                            cached: proposal.item.fcis_len,
                            actual: prepared.1,
                        },
                        proposal,
                        new_wire_id,
                    ))
                }
                Err(reason) => {
                    return Err(ItemEntityRestoreRefusal::new(reason, proposal, new_wire_id))
                }
            };
        if let Err(error) = self.store.reserve_spawn_slot() {
            return Err(ItemEntityRestoreRefusal::new(
                error.into(),
                proposal,
                new_wire_id,
            ));
        }
        let item = ItemEntityState {
            fcis_len,
            ..proposal.item
        };
        let handle = self.store.commit_spawn(base, Some(item), bucket);
        self.store.item_bytes_used = used;
        Ok(handle)
    }
    /// Applies one bounded counter projection; repeated calls mean repeated steps.
    ///
    /// This neither runs physics nor emits an event/removes an item. A complete
    /// active tick must preserve the original interleaving with physics/merge.
    pub fn step_item_counters(
        &mut self,
        handle: &EntityHandle,
        mode: ItemCounterMode,
        despawn_rate: i32,
    ) -> Result<ItemDespawnDecision, ItemEntityError> {
        let index = self.resident_item_index(handle)?;
        let item = self.store.slots[index]
            .item
            .as_mut()
            .ok_or(ItemEntityError::MissingItemState)?;
        let step = match mode {
            ItemCounterMode::Active => {
                step_active_item_counters(item.properties.counters, despawn_rate)
            }
            ItemCounterMode::Inactive => {
                step_inactive_item_counters(item.properties.counters, despawn_rate)
            }
        };
        item.properties.counters = step.counters;
        Ok(step.decision)
    }
    /// Resets only age after caller-handled cancellation; no second delay step.
    pub fn reset_item_despawn_age(&mut self, handle: &EntityHandle) -> Result<(), ItemEntityError> {
        let index = self.resident_item_index(handle)?;
        let item = self.store.slots[index]
            .item
            .as_mut()
            .ok_or(ItemEntityError::MissingItemState)?;
        item.properties.counters.age = 0;
        Ok(())
    }
}
