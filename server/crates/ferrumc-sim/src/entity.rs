//! Bounded owner-local entity identity, transforms and chunk indexes.
//!
//! Registration is distinct from chunk residency, visibility, entity ticking
//! and persistence. Item payloads use the same owner slots and a tick capability;
//! physics, pickup, tracking and durable storage consumers remain separate.

mod item;
pub(crate) mod ownership;
pub use item::*;

use std::collections::BTreeMap;
use std::fmt;
use std::iter::FusedIterator;
use std::num::NonZeroUsize;
use std::sync::{Arc, Weak};

use ferrumc_core::{DimensionId, EntityId, EntityUuid, WorldId};
use ferrumc_math::{BlockPos, ChunkPos, ShardPos, Vec3};
use ferrumc_registry::entity_type::EntityTypeId;

use crate::{LoadedChunkMap, ShardId};

/// Maximum configured entity slot capacity in one owner-local store.
///
/// This `FerrumC` resource ceiling bounds lazy slot growth and both identity
/// indexes. It is not an original Minecraft per-shard entity limit.
pub const MAX_ENTITIES_PER_SHARD: usize = 65_536;

/// Maximum absolute x/z accepted by this producer, in blocks.
///
/// The finite original setter alone has no such bound; this is an explicit
/// `FerrumC` admission limit, never silent position clamping.
pub const MAX_ENTITY_HORIZONTAL_POSITION: f64 = 30_000_000.0;

/// Maximum absolute y accepted by this producer, in blocks.
///
/// This is an admission ceiling, not the world's build-height or ticking range.
pub const MAX_ENTITY_VERTICAL_POSITION: f64 = 20_000_000.0;

/// The invalid component of a rejected entity transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityTransformComponent {
    /// Non-finite or out-of-range x position.
    X,
    /// Non-finite or out-of-range y position.
    Y,
    /// Non-finite or out-of-range z position.
    Z,
    /// Non-finite yaw.
    Yaw,
    /// Non-finite pitch.
    Pitch,
}

/// Classified entity admission or owner-local mutation refusal.
///
/// Every returned error preserves all stored records, identities and indexes.
/// Process allocation failure in standard-library tree insertion is not a
/// recoverable admission result. No method acknowledges durable persistence.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EntityStoreError {
    /// The configured capacity exceeds the documented ceiling.
    #[error("entity capacity {requested} exceeds limit {limit}")]
    InvalidCapacity {
        /// Requested nonzero capacity.
        requested: usize,
        /// Maximum accepted capacity.
        limit: usize,
    },
    /// All available live slots are occupied.
    #[error("entity store is full (capacity {capacity})")]
    Full {
        /// Configured slot ceiling.
        capacity: usize,
    },
    /// Retired slots exhausted the ceiling even though fewer entities are live.
    #[error("entity slot capacity exhausted by occupied or retired slots ({capacity})")]
    SlotsExhausted {
        /// Configured slot ceiling, including retired slots.
        capacity: usize,
    },
    /// Bounded lazy slot reservation failed before any entity/index changed.
    #[error("bounded entity slot allocation failed")]
    AllocationFailed,
    /// A transform is malformed or exceeds the explicit position ceiling.
    #[error("invalid entity transform component {component:?}")]
    InvalidTransform {
        /// Component that prevented admission.
        component: EntityTransformComponent,
    },
    /// The request or current chunk-map scope differs from the canonical owner.
    #[error("entity scope differs from owner {owner}")]
    ScopeMismatch {
        /// Store's canonical owner.
        owner: ShardId,
        /// Refused request or current chunk-map world.
        world: WorldId,
        /// Refused request or current chunk-map dimension.
        dimension: DimensionId,
    },
    /// A target chunk lies outside the owner's fixed 8x8 region.
    #[error("entity chunk {chunk:?} lies outside owner {owner}")]
    OutsideOwner {
        /// Owner whose state remains unchanged.
        owner: ShardId,
        /// Refused typed target chunk.
        chunk: ChunkPos,
    },
    /// A UUID is already registered in this store.
    #[error("duplicate owner-local entity UUID {uuid}")]
    DuplicateUuid {
        /// Existing instance identity.
        uuid: EntityUuid,
    },
    /// A wire instance id is already registered in this store.
    #[error("duplicate owner-local entity wire id {wire_id}")]
    DuplicateWireId {
        /// Existing protocol instance reference.
        wire_id: EntityId,
    },
    /// Player state already has a separate owner and cannot be duplicated here.
    #[error("player type cannot be admitted into the non-player entity store")]
    PlayerType,
    /// The handle was issued by a different owner or store incarnation.
    #[error("entity handle belongs to another owner or store incarnation")]
    ForeignHandle,
    /// The issued slot/generation no longer describes a live entity.
    #[error("entity handle is stale")]
    StaleHandle,
    /// The shard's arbitrary position cannot name a complete ownership region.
    #[error("entity owner position {position:?} cannot describe a complete shard")]
    InvalidOwner {
        /// Invalid typed shard position.
        position: ShardPos,
    },
    /// One-time initialization cannot replace an existing store or handles.
    #[error("shard entity store is already initialized")]
    AlreadyInitialized,
    /// The entity tick entry was refused before running work or draining input.
    #[error("shard entity store is not initialized")]
    NotInitialized,
    /// Tick-context spawn/move requires a currently ticket-resident target.
    #[error("entity target chunk {chunk:?} is not resident")]
    ChunkNotResident {
        /// Missing target chunk in the owner's loaded map.
        chunk: ChunkPos,
    },
    /// Legacy base-only removal cannot silently dispose of an owned payload.
    #[error("item payload requires tick-context owned removal")]
    PayloadRequiresOwnedRemove,
    /// An item payload may only move through its owner's tick capability.
    #[error("item mutation requires an entity tick context")]
    ItemMutationRequiresTickContext,
    /// Receipt-confirmed mode disallows legacy or copied-store writers.
    #[error("entity mutation requires the live receipt-confirmed owner")]
    OwnershipRequired,
}

/// A validated finite entity position and normalized body rotation.
///
/// Construction refuses invalid coordinates/rotations rather than modifying
/// an existing record. For finite inputs, yaw uses `% 360` and pitch uses
/// `clamp(pitch % 360, -90, 90)`, matching the reviewed original rotation path.
/// No velocity, head rotation, bounding box or movement physics is modelled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityTransform {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

impl EntityTransform {
    /// Validates a transform against the documented finite admission bounds.
    pub fn try_new(position: Vec3, yaw: f32, pitch: f32) -> Result<Self, EntityStoreError> {
        for (value, limit, component) in [
            (
                position.x,
                MAX_ENTITY_HORIZONTAL_POSITION,
                EntityTransformComponent::X,
            ),
            (
                position.y,
                MAX_ENTITY_VERTICAL_POSITION,
                EntityTransformComponent::Y,
            ),
            (
                position.z,
                MAX_ENTITY_HORIZONTAL_POSITION,
                EntityTransformComponent::Z,
            ),
        ] {
            if !value.is_finite() || value.abs() > limit {
                return Err(EntityStoreError::InvalidTransform { component });
            }
        }
        for (value, component) in [
            (yaw, EntityTransformComponent::Yaw),
            (pitch, EntityTransformComponent::Pitch),
        ] {
            if !value.is_finite() {
                return Err(EntityStoreError::InvalidTransform { component });
            }
        }
        Ok(Self {
            position,
            yaw: yaw % 360.0,
            pitch: (pitch % 360.0).clamp(-90.0, 90.0),
        })
    }

    /// Returns the admitted position in blocks.
    pub const fn position(self) -> Vec3 {
        self.position
    }

    /// Returns finite normalized body yaw in degrees.
    pub const fn yaw(self) -> f32 {
        self.yaw
    }

    /// Returns finite normalized/clamped pitch in degrees.
    pub const fn pitch(self) -> f32 {
        self.pitch
    }

    /// Derives the typed chunk using floor, including negative positions.
    pub fn chunk(self) -> ChunkPos {
        // Validated bounds fit i32 before flooring. Truncation before floor
        // would incorrectly assign -0.5 to chunk zero.
        BlockPos::new(
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        )
        .to_chunk_pos()
    }
}

/// Explicit supplied identities, scope, type and transform for one admission.
///
/// The producer never invents UUIDs or allocates global wire ids. A refused
/// request contains only Copy base data; it transfers no item or durable owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntitySpawn {
    pub(crate) world: WorldId,
    pub(crate) dimension: DimensionId,
    pub(crate) uuid: EntityUuid,
    pub(crate) wire_id: EntityId,
    pub(crate) entity_type: EntityTypeId,
    pub(crate) transform: EntityTransform,
}

impl EntitySpawn {
    /// Creates an explicit request; store/context admission validates ownership.
    pub const fn new(
        world: WorldId,
        dimension: DimensionId,
        uuid: EntityUuid,
        wire_id: EntityId,
        entity_type: EntityTypeId,
        transform: EntityTransform,
    ) -> Self {
        Self {
            world,
            dimension,
            uuid,
            wire_id,
            entity_type,
            transform,
        }
    }
}

/// Immutable base identity and transform of a registered non-player entity.
///
/// This record omits complete original metadata, living/item components,
/// collision, tracking and save state. Removing it is not a durable tombstone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityData {
    owner: ShardId,
    uuid: EntityUuid,
    wire_id: EntityId,
    entity_type: EntityTypeId,
    transform: EntityTransform,
}

impl EntityData {
    /// Returns the canonical world/dimension/shard owner.
    pub const fn owner(self) -> ShardId {
        self.owner
    }
    /// Returns the supplied persistent instance UUID.
    pub const fn uuid(self) -> EntityUuid {
        self.uuid
    }
    /// Returns the supplied wire instance id, distinct from the type id.
    pub const fn wire_id(self) -> EntityId {
        self.wire_id
    }
    /// Returns the validated static entity type.
    pub const fn entity_type(self) -> EntityTypeId {
        self.entity_type
    }
    /// Returns the current validated transform.
    pub const fn transform(self) -> EntityTransform {
        self.transform
    }
    /// Returns the indexed typed chunk of the current position.
    pub fn chunk(self) -> ChunkPos {
        self.transform.chunk()
    }
}

/// Opaque owner-local generational handle for one store incarnation.
///
/// A weak allocation identity prevents old handles from aliasing a recreated
/// same-owner store, without retaining the store or records. Handles are not
/// serialized UUIDs/wire ids. Cloning a handle allocates no heap object.
#[derive(Clone)]
pub struct EntityHandle {
    incarnation: Weak<()>,
    owner: ShardId,
    slot: usize,
    generation: u64,
}

impl fmt::Debug for EntityHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Allocation addresses are not deterministic simulation data.
        formatter
            .debug_struct("EntityHandle")
            .field("owner", &self.owner)
            .field("slot", &self.slot)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EntityHandle {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.slot == other.slot
            && self.generation == other.generation
            && Weak::ptr_eq(&self.incarnation, &other.incarnation)
    }
}
impl Eq for EntityHandle {}

#[derive(Debug)]
struct Slot {
    generation: u64,
    entity: Option<EntityData>,
    item: Option<ItemEntityState>,
    ownership: Option<ferrumc_storage::OwnedEntityExpectation>,
    previous: Option<usize>,
    next: Option<usize>,
    next_free: Option<usize>,
}

impl Slot {
    // Called only after the whole snapshot passes payload and aggregate checks.
    fn snapshot_admitted(&self) -> Self {
        Self {
            generation: self.generation,
            entity: self.entity,
            item: self.item.as_ref().map(ItemEntityState::snapshot_admitted),
            ownership: self.ownership.clone(),
            previous: self.previous,
            next: self.next,
            next_free: self.next_free,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Bucket {
    first: Option<usize>,
    last: Option<usize>,
    count: usize,
}

/// Bounded exclusively owned registration and spatial storage for one shard.
///
/// Standalone mutations are synchronous owner operations, independent of chunk
/// residency. The shard's tick context additionally enforces resident target
/// admission. Callers must schedule standalone work at their owner boundary.
/// UUID/wire duplicate checks are local; process-wide admission/transfer is a
/// future consumer. Queries return immutable data/borrowed iterators.
///
/// `Clone` copies bounded data with a **fresh incarnation**: original handles
/// are rejected by the copy. This is a simulation snapshot, not permission to
/// activate a second global owner with duplicate UUIDs or wire ids.
#[derive(Debug)]
pub struct EntityStore {
    owner: ShardId,
    capacity: usize,
    item_byte_budget: usize,
    item_bytes_used: usize,
    incarnation: Arc<()>,
    slots: Vec<Slot>,
    free: Option<usize>,
    retired: usize,
    by_uuid: BTreeMap<EntityUuid, usize>,
    by_wire: BTreeMap<EntityId, usize>,
    buckets: [Bucket; 64],
    ownership_mode: bool,
    snapshot_only: bool,
    reservation: Option<ferrumc_storage::OwnershipTransferId>,
}

impl Clone for EntityStore {
    fn clone(&self) -> Self {
        // Public setters seal this invariant. The legacy infallible trait can
        // only fail-stop on impossible private corruption; checked callers use
        // try_snapshot instead. Never clone or normalize a corrupt payload.
        match self.try_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("entity snapshot invariant violated: {error}"),
        }
    }
}

impl EntityStore {
    /// Creates an empty store with explicit nonzero capacity and canonical owner.
    ///
    /// Slots grow lazily, bounded by capacity; no unbounded default is provided.
    pub fn new(owner: ShardId, capacity: NonZeroUsize) -> Result<Self, EntityStoreError> {
        if capacity.get() > MAX_ENTITIES_PER_SHARD {
            return Err(EntityStoreError::InvalidCapacity {
                requested: capacity.get(),
                limit: MAX_ENTITIES_PER_SHARD,
            });
        }
        Ok(Self {
            owner,
            capacity: capacity.get(),
            item_byte_budget: 0,
            item_bytes_used: 0,
            incarnation: Arc::new(()),
            slots: Vec::new(),
            free: None,
            retired: 0,
            by_uuid: BTreeMap::new(),
            by_wire: BTreeMap::new(),
            buckets: [Bucket::default(); 64],
            ownership_mode: false,
            snapshot_only: false,
            reservation: None,
        })
    }

    /// Returns this store's canonical logical owner.
    pub const fn owner(&self) -> ShardId {
        self.owner
    }
    /// Returns the explicit maximum slot count, including retired slots.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
    /// Returns the live registered count, independent of chunk load/tick state.
    pub fn len(&self) -> usize {
        self.by_uuid.len()
    }
    /// Returns whether no entity is registered.
    pub fn is_empty(&self) -> bool {
        self.by_uuid.is_empty()
    }
    /// Returns the permanently retired slot count after generation exhaustion.
    pub const fn retired_slot_count(&self) -> usize {
        self.retired
    }
    /// Returns whether occupied/retired slots prevent another admission.
    pub fn is_full(&self) -> bool {
        self.len() + self.retired >= self.capacity
    }

    fn bucket(&self, chunk: ChunkPos) -> Result<usize, EntityStoreError> {
        if !self
            .owner
            .region()
            .contains_chunk(self.owner.world(), self.owner.dimension(), chunk)
        {
            return Err(EntityStoreError::OutsideOwner {
                owner: self.owner,
                chunk,
            });
        }
        Ok((chunk.x().rem_euclid(8) + chunk.z().rem_euclid(8) * 8) as usize)
    }

    fn validate_spawn(&self, spawn: EntitySpawn) -> Result<usize, EntityStoreError> {
        if spawn.world != self.owner.world() || spawn.dimension != self.owner.dimension() {
            return Err(EntityStoreError::ScopeMismatch {
                owner: self.owner,
                world: spawn.world,
                dimension: spawn.dimension,
            });
        }
        if spawn.entity_type.is_player() {
            return Err(EntityStoreError::PlayerType);
        }
        let bucket = self.bucket(spawn.transform.chunk())?;
        if self.by_uuid.contains_key(&spawn.uuid) {
            return Err(EntityStoreError::DuplicateUuid { uuid: spawn.uuid });
        }
        if self.by_wire.contains_key(&spawn.wire_id) {
            return Err(EntityStoreError::DuplicateWireId {
                wire_id: spawn.wire_id,
            });
        }
        if self.len() >= self.capacity {
            return Err(EntityStoreError::Full {
                capacity: self.capacity,
            });
        }
        if self.free.is_none() && self.slots.len() >= self.capacity {
            return Err(EntityStoreError::SlotsExhausted {
                capacity: self.capacity,
            });
        }
        Ok(bucket)
    }

    fn handle(&self, index: usize) -> EntityHandle {
        EntityHandle {
            incarnation: Arc::downgrade(&self.incarnation),
            owner: self.owner,
            slot: index,
            generation: self.slots[index].generation,
        }
    }

    fn index(&self, handle: &EntityHandle) -> Result<usize, EntityStoreError> {
        if self.owner != handle.owner
            || !Weak::ptr_eq(&handle.incarnation, &Arc::downgrade(&self.incarnation))
        {
            return Err(EntityStoreError::ForeignHandle);
        }
        let slot = self
            .slots
            .get(handle.slot)
            .ok_or(EntityStoreError::StaleHandle)?;
        if slot.generation != handle.generation || slot.entity.is_none() {
            return Err(EntityStoreError::StaleHandle);
        }
        Ok(handle.slot)
    }

    fn append(&mut self, index: usize, bucket: usize) {
        let previous = self.buckets[bucket].last;
        self.slots[index].previous = previous;
        self.slots[index].next = None;
        if let Some(previous) = previous {
            self.slots[previous].next = Some(index);
        } else {
            self.buckets[bucket].first = Some(index);
        }
        self.buckets[bucket].last = Some(index);
        self.buckets[bucket].count += 1;
    }

    fn detach(&mut self, index: usize, bucket: usize) {
        let previous = self.slots[index].previous;
        let next = self.slots[index].next;
        if let Some(previous) = previous {
            self.slots[previous].next = next;
        } else {
            self.buckets[bucket].first = next;
        }
        if let Some(next) = next {
            self.slots[next].previous = previous;
        } else {
            self.buckets[bucket].last = previous;
        }
        self.slots[index].previous = None;
        self.slots[index].next = None;
        self.buckets[bucket].count -= 1;
    }

    /// Registers supplied non-player base data after complete preflight.
    ///
    /// Success may allocate only within the explicit store capacity. Errors
    /// leave all records, identity and chunk indexes unchanged. Residency and
    /// process-wide UUID/wire allocation are separate consumer contracts.
    pub fn spawn(&mut self, spawn: EntitySpawn) -> Result<EntityHandle, EntityStoreError> {
        self.require_legacy_write()?;
        let bucket = self.validate_spawn(spawn)?;
        self.reserve_spawn_slot()?;
        Ok(self.commit_spawn(spawn, None, bucket))
    }

    fn reserve_spawn_slot(&mut self) -> Result<(), EntityStoreError> {
        if self.free.is_none() && self.slots.len() == self.slots.capacity() {
            let target = self.slots.len().saturating_mul(2).max(4).min(self.capacity);
            self.slots
                .try_reserve_exact(target - self.slots.len())
                .map_err(|_| EntityStoreError::AllocationFailed)?;
        }
        Ok(())
    }

    // All fallible admission and reservation precedes this ownership move.
    fn commit_spawn(
        &mut self,
        spawn: EntitySpawn,
        item: Option<ItemEntityState>,
        bucket: usize,
    ) -> EntityHandle {
        let entity = EntityData {
            owner: self.owner,
            uuid: spawn.uuid,
            wire_id: spawn.wire_id,
            entity_type: spawn.entity_type,
            transform: spawn.transform,
        };
        let index = if let Some(index) = self.free {
            self.free = self.slots[index].next_free;
            self.slots[index].next_free = None;
            self.slots[index].entity = Some(entity);
            self.slots[index].item = item;
            self.slots[index].ownership = None;
            index
        } else {
            let index = self.slots.len();
            self.slots.push(Slot {
                generation: 0,
                entity: Some(entity),
                item,
                ownership: None,
                previous: None,
                next: None,
                next_free: None,
            });
            index
        };
        self.by_uuid.insert(spawn.uuid, index);
        self.by_wire.insert(spawn.wire_id, index);
        self.append(index, bucket);
        self.handle(index)
    }

    /// Looks up immutable base data, refusing foreign/stale handles without allocation.
    pub fn get(&self, handle: &EntityHandle) -> Result<&EntityData, EntityStoreError> {
        let index = self.index(handle)?;
        self.slots[index]
            .entity
            .as_ref()
            .ok_or(EntityStoreError::StaleHandle)
    }

    /// Issues a current local handle for a registered UUID without allocation.
    pub fn handle_for_uuid(&self, uuid: EntityUuid) -> Option<EntityHandle> {
        self.by_uuid.get(&uuid).map(|&index| self.handle(index))
    }

    /// Issues a current local handle for a registered wire instance id without allocation.
    pub fn handle_for_wire_id(&self, wire_id: EntityId) -> Option<EntityHandle> {
        self.by_wire.get(&wire_id).map(|&index| self.handle(index))
    }

    /// Iterates this typed owner-local chunk in deterministic admission/move order.
    ///
    /// Queries touch only its fixed bucket and linked occupants, never scan all
    /// entity slots. Iterator budgets/pagination use `.take`/`.skip`; borrowing
    /// prevents concurrent mutation. No visibility/ticking filter is implied.
    pub fn in_chunk(&self, chunk: ChunkPos) -> Result<EntityChunkIter<'_>, EntityStoreError> {
        let bucket = self.buckets[self.bucket(chunk)?];
        Ok(EntityChunkIter {
            store: self,
            next: bucket.first,
            remaining: bucket.count,
        })
    }

    /// Replaces a valid transform and updates the chunk index without allocation.
    ///
    /// Crossing the fixed owner boundary is refused atomically; a cross-shard
    /// transfer consumer must coordinate that change later. Same-chunk updates
    /// preserve iteration position, cross-chunk moves append to the target.
    pub fn update_transform(
        &mut self,
        handle: &EntityHandle,
        transform: EntityTransform,
    ) -> Result<(), EntityStoreError> {
        self.require_legacy_write()?;
        let index = self.index(handle)?;
        if self.slots[index].item.is_some() {
            return Err(EntityStoreError::ItemMutationRequiresTickContext);
        }
        self.update_transform_admitted(handle, transform)
    }

    fn update_transform_admitted(
        &mut self,
        handle: &EntityHandle,
        transform: EntityTransform,
    ) -> Result<(), EntityStoreError> {
        let index = self.index(handle)?;
        let entity = self.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        let old = self.bucket(entity.chunk())?;
        let new = self.bucket(transform.chunk())?;
        if old != new {
            self.detach(index, old);
            self.append(index, new);
        }
        self.slots[index].entity = Some(EntityData {
            transform,
            ..entity
        });
        Ok(())
    }

    /// Removes one live registration from every index and invalidates its handle.
    ///
    /// Generation increments before reuse; maximum generation retires the slot
    /// forever rather than wrapping. Returned base data is not a save/delete ack.
    pub fn remove(&mut self, handle: &EntityHandle) -> Result<EntityData, EntityStoreError> {
        self.require_legacy_write()?;
        let index = self.index(handle)?;
        if self.slots[index].item.is_some() {
            return Err(EntityStoreError::PayloadRequiresOwnedRemove);
        }
        let entity = self.slots[index]
            .entity
            .ok_or(EntityStoreError::StaleHandle)?;
        let bucket = self.bucket(entity.chunk())?;
        Ok(self.remove_slot(index, entity, bucket).0)
    }

    fn remove_slot(
        &mut self,
        index: usize,
        entity: EntityData,
        bucket: usize,
    ) -> (EntityData, Option<ItemEntityState>) {
        let item = self.slots[index].item.take();
        self.detach(index, bucket);
        self.by_uuid.remove(&entity.uuid);
        self.by_wire.remove(&entity.wire_id);
        self.slots[index].entity = None;
        self.slots[index].ownership = None;
        if let Some(generation) = self.slots[index].generation.checked_add(1) {
            self.slots[index].generation = generation;
            self.slots[index].next_free = self.free;
            self.free = Some(index);
        } else {
            self.retired += 1;
        }
        (entity, item)
    }
}

/// Allocation-free borrowed iterator over occupants of one indexed chunk.
pub struct EntityChunkIter<'a> {
    store: &'a EntityStore,
    next: Option<usize>,
    remaining: usize,
}

impl<'a> Iterator for EntityChunkIter<'a> {
    type Item = &'a EntityData;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next?;
        let slot = &self.store.slots[index];
        self.next = slot.next;
        self.remaining -= 1;
        slot.entity.as_ref()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
impl ExactSizeIterator for EntityChunkIter<'_> {}
impl FusedIterator for EntityChunkIter<'_> {}

/// Tick-entry-only non-player mutation capability tied to this owner's chunks.
///
/// Created exclusively by `SimShard::run_tick_with_entity_work`. It exposes
/// immutable registration queries and checked operations, never raw mutable
/// store/chunk access. Resident targets are required for spawn/move. Residency
/// is ticket memory state, not original entity-ticking/activation visibility.
///
/// A registration reference cannot escape its tick work capability:
///
/// ```compile_fail
/// use ferrumc_sim::{EntityStore, SimShard};
/// fn escape(shard: &mut SimShard) {
///     let _: &EntityStore = shard
///         .run_tick_with_entity_work(|context| context.entities())
///         .unwrap().0;
/// }
/// ```
///
/// Immutable registration access cannot bypass checked context mutation:
///
/// ```compile_fail
/// use ferrumc_sim::{EntityHandle, SimShard};
/// fn bypass(shard: &SimShard, handle: &EntityHandle) {
///     shard.entities().unwrap().remove(handle).unwrap();
/// }
/// ```
pub struct EntityTickContext<'a> {
    store: &'a mut EntityStore,
    chunks: &'a LoadedChunkMap,
}

impl<'a> EntityTickContext<'a> {
    // The shard checks its current map scope against the store owner before
    // constructing this capability. Both borrows prevent replacement in work.
    pub(crate) fn new(store: &'a mut EntityStore, chunks: &'a LoadedChunkMap) -> Self {
        Self { store, chunks }
    }

    /// Returns immutable owner-local registration and borrowed queries.
    pub fn entities(&self) -> &EntityStore {
        self.store
    }

    /// Reports current ticket residency for a chunk inside this exact owner.
    ///
    /// No entity ticking, activation or tracking status is inferred from it.
    pub fn is_chunk_resident(&self, chunk: ChunkPos) -> Result<bool, EntityStoreError> {
        self.store.bucket(chunk)?;
        Ok(self.chunks.is_loaded(chunk))
    }

    fn require_resident(&self, chunk: ChunkPos) -> Result<(), EntityStoreError> {
        if !self.is_chunk_resident(chunk)? {
            return Err(EntityStoreError::ChunkNotResident { chunk });
        }
        Ok(())
    }

    /// Registers base data only after complete owner and resident-target checks.
    pub fn spawn(&mut self, spawn: EntitySpawn) -> Result<EntityHandle, EntityStoreError> {
        self.store.validate_spawn(spawn)?;
        self.require_resident(spawn.transform.chunk())?;
        self.store.spawn(spawn)
    }

    /// Applies an allocation-free transform update to a currently resident target.
    ///
    /// This can move a registration out of a since-unloaded source chunk into
    /// another resident chunk. It does not force-load either chunk or transfer
    /// ownership across shards.
    pub fn update_transform(
        &mut self,
        handle: &EntityHandle,
        transform: EntityTransform,
    ) -> Result<(), EntityStoreError> {
        self.store.require_legacy_write()?;
        self.store.get(handle)?;
        self.store.bucket(transform.chunk())?;
        self.require_resident(transform.chunk())?;
        self.store.update_transform_admitted(handle, transform)
    }

    /// Removes a valid registration even if its old chunk is no longer resident.
    ///
    /// Releasing a chunk ticket does not implicitly remove this owner data;
    /// cleanup remains possible without residency and supplies no durable ack.
    pub fn remove(&mut self, handle: &EntityHandle) -> Result<EntityData, EntityStoreError> {
        self.store.remove(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> ShardId {
        ShardId::try_new(WorldId::new(0), DimensionId::new(0), ShardPos::new(0, 0)).unwrap()
    }
    fn spawn(uuid: u8) -> EntitySpawn {
        EntitySpawn::new(
            WorldId::new(0),
            DimensionId::new(0),
            EntityUuid::from_bytes([uuid; 16]),
            EntityId::new(i32::from(uuid)),
            EntityTypeId::from_name("item").unwrap(),
            EntityTransform::try_new(Vec3::new(1.0, 64.0, 1.0), 0.0, 0.0).unwrap(),
        )
    }

    #[test]
    fn generation_max_retires_forever_without_aliasing() {
        let mut store = EntityStore::new(owner(), NonZeroUsize::MIN).unwrap();
        let old = store.spawn(spawn(1)).unwrap();
        store.slots[old.slot].generation = u64::MAX;
        let max = store
            .handle_for_uuid(EntityUuid::from_bytes([1; 16]))
            .unwrap();
        assert_eq!(store.get(&old), Err(EntityStoreError::StaleHandle));
        store.remove(&max).unwrap();
        assert!(store.is_empty());
        assert!(store.is_full());
        assert_eq!(store.retired_slot_count(), 1);
        assert_eq!(store.slots[old.slot].generation, u64::MAX);
        assert!(store.free.is_none());
        assert_eq!(
            store.spawn(spawn(1)),
            Err(EntityStoreError::SlotsExhausted { capacity: 1 })
        );
        assert_eq!(store.get(&max), Err(EntityStoreError::StaleHandle));
        assert!(store
            .in_chunk(ChunkPos::new(0, 0))
            .unwrap()
            .next()
            .is_none());
    }

    #[test]
    fn generation_last_valid_reuse_then_retirement_leaves_other_slot_usable() {
        let mut store = EntityStore::new(owner(), NonZeroUsize::new(2).unwrap()).unwrap();
        let initial = store.spawn(spawn(1)).unwrap();
        store.slots[initial.slot].generation = u64::MAX - 1;
        let last = store.handle(initial.slot);
        store.remove(&last).unwrap();
        let max = store.spawn(spawn(2)).unwrap();
        assert_eq!(max.slot, last.slot);
        assert_eq!(max.generation, u64::MAX);
        assert_eq!(store.get(&last), Err(EntityStoreError::StaleHandle));
        store.remove(&max).unwrap();
        let next = store.spawn(spawn(3)).unwrap();
        assert_ne!(next.slot, max.slot);
        assert_eq!(store.len(), 1);
        assert!(store.is_full());
        assert_eq!(store.retired_slot_count(), 1);
        assert_eq!(store.get(&max), Err(EntityStoreError::StaleHandle));
        assert_eq!(store.in_chunk(ChunkPos::new(0, 0)).unwrap().count(), 1);
    }

    #[test]
    fn slots_grow_lazily_with_bounded_reservation_and_reuse_never_grows() {
        let mut store = EntityStore::new(owner(), NonZeroUsize::new(7).unwrap()).unwrap();
        assert_eq!(store.slots.capacity(), 0);
        let first = store.spawn(spawn(1)).unwrap();
        for id in 2..=7 {
            store.spawn(spawn(id)).unwrap();
        }
        assert!(store.slots.capacity() <= 7);
        let capacity = store.slots.capacity();
        let count = store.slots.len();
        store.remove(&first).unwrap();
        store.spawn(spawn(8)).unwrap();
        assert_eq!(store.slots.capacity(), capacity);
        assert_eq!(store.slots.len(), count);
    }
}
