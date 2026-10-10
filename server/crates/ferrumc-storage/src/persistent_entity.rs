//! Explicit UUID persistence, independent of runtime entity handles.

use std::num::NonZeroU16;
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use ferrumc_core::{DimensionId, EntityUuid, Result, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_world::GenerationIdentity;

use crate::{ChunkKey, EntityRecord, StorageError};

pub(crate) mod engine;
pub(crate) mod raw;

/// Maximum writes admitted by one atomic UUID entity save.
pub const MAX_PERSISTENT_ENTITY_WRITES: usize = 64;
/// Maximum encoded key-plus-envelope bytes admitted by one save (4 MiB).
pub const MAX_PERSISTENT_ENTITY_BATCH_BYTES: usize = 4 * 1024 * 1024;
/// Maximum encoded key-plus-envelope bytes returned in one page (1 MiB).
pub const MAX_PERSISTENT_ENTITY_PAGE_BYTES: usize = 1024 * 1024;
/// Maximum entity entries returned in one page.
pub const MAX_PERSISTENT_ENTITY_PAGE_ENTRIES: u16 = 64;

/// A durable entity's world, dimension and complete UUID.
///
/// Runtime and player identifiers require an explicit domain conversion:
/// ```compile_fail,E0308
/// use ferrumc_core::{WorldId, DimensionId, EntityId};
/// use ferrumc_storage::PersistentEntityKey;
/// PersistentEntityKey::new(WorldId::new(0), DimensionId::new(0), EntityId::new(1));
/// ```
///
/// Player identity also requires an explicit domain decision:
/// ```compile_fail,E0308
/// use ferrumc_core::{WorldId, DimensionId, PlayerId};
/// use ferrumc_storage::PersistentEntityKey;
/// PersistentEntityKey::new(WorldId::new(0), DimensionId::new(0), PlayerId::offline("Steve"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersistentEntityKey {
    world: WorldId,
    dimension: DimensionId,
    uuid: EntityUuid,
}

impl PersistentEntityKey {
    /// Preserves the caller's typed scope and UUID without generating an identity.
    #[must_use]
    pub const fn new(world: WorldId, dimension: DimensionId, uuid: EntityUuid) -> Self {
        Self {
            world,
            dimension,
            uuid,
        }
    }
    /// Returns the world handle.
    #[must_use]
    pub const fn world(self) -> WorldId {
        self.world
    }
    /// Returns the dimension handle.
    #[must_use]
    pub const fn dimension(self) -> DimensionId {
        self.dimension
    }
    /// Returns all persistent identity bits.
    #[must_use]
    pub const fn uuid(self) -> EntityUuid {
        self.uuid
    }
}

/// A checked chunk column and bounded, opaque simulation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentEntityRecord {
    chunk: ChunkPos,
    record: EntityRecord,
}

impl PersistentEntityRecord {
    /// Rejects columns whose origins or maximum local offsets exceed i32 blocks.
    pub fn new(chunk: ChunkPos, record: EntityRecord) -> std::result::Result<Self, StorageError> {
        crate::record::validate_chunk_position(chunk)?;
        Ok(Self { chunk, record })
    }
    /// Returns the chunk column occupied by the persistent entity.
    #[must_use]
    pub const fn chunk(&self) -> ChunkPos {
        self.chunk
    }
    /// Borrows the bounded simulation payload and its independent schema.
    #[must_use]
    pub const fn record(&self) -> &EntityRecord {
        &self.record
    }
    /// Returns ownership of the opaque payload.
    #[must_use]
    pub fn into_record(self) -> EntityRecord {
        self.record
    }
}

/// One atomic save proposal paired with the exact immutable generation identity.
///
/// A seed alone cannot replace the complete immutable descriptor:
/// ```compile_fail,E0308
/// use ferrumc_core::{WorldId, DimensionId, EntityUuid};
/// use ferrumc_math::ChunkPos;
/// use ferrumc_storage::{EntityRecord, PersistentEntityKey, PersistentEntityRecord,
///     PersistentEntityWrite, SchemaVersion};
/// use ferrumc_world::WorldSeed;
/// let key = PersistentEntityKey::new(WorldId::new(0), DimensionId::new(0), EntityUuid::from_bytes([0; 16]));
/// let record = PersistentEntityRecord::new(ChunkPos::new(0, 0),
///     EntityRecord::new(SchemaVersion::new(0), vec![]).unwrap()).unwrap();
/// PersistentEntityWrite::new(key, WorldSeed::new(0), record);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentEntityWrite {
    key: PersistentEntityKey,
    expected: GenerationIdentity,
    record: PersistentEntityRecord,
}

impl PersistentEntityWrite {
    /// Builds a proposal; bounded batch and relationship admission occurs atomically.
    #[must_use]
    pub const fn new(
        key: PersistentEntityKey,
        expected: GenerationIdentity,
        record: PersistentEntityRecord,
    ) -> Self {
        Self {
            key,
            expected,
            record,
        }
    }
    /// Returns the durable key.
    #[must_use]
    pub const fn key(&self) -> PersistentEntityKey {
        self.key
    }
    /// Returns the exact expected generator baseline.
    #[must_use]
    pub const fn expected_identity(&self) -> GenerationIdentity {
        self.expected
    }
    /// Borrows the proposed entity record.
    #[must_use]
    pub const fn record(&self) -> &PersistentEntityRecord {
        &self.record
    }
}

/// Persisted mutation epoch of one chunk's entity set; absent history is zero.
///
/// Epochs invalidate pagination; they are not ownership versions or a CAS token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistentEntityEpoch(u64);
impl PersistentEntityEpoch {
    /// Returns zero for no history, otherwise the positive persisted epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// An entity returned by bounded chunk pagination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentEntityEntry {
    key: PersistentEntityKey,
    record: PersistentEntityRecord,
}
impl PersistentEntityEntry {
    /// Returns the durable identity.
    #[must_use]
    pub const fn key(&self) -> PersistentEntityKey {
        self.key
    }
    /// Borrows the checked entity record.
    #[must_use]
    pub const fn record(&self) -> &PersistentEntityRecord {
        &self.record
    }
    /// Returns the owned checked entity record.
    #[must_use]
    pub fn into_record(self) -> PersistentEntityRecord {
        self.record
    }
}

/// Transient pagination continuation bound to one live backend incarnation.
///
/// No raw constructor or serialization exists. A weak owner prevents cursors
/// from retaining a database. Reopening creates a new incarnation, even if
/// bytes and epochs have not changed. Arc clones of one backend share authority.
///
/// Even existing epoch values cannot be used to forge a raw cursor:
/// ```compile_fail,E0451
/// use std::sync::Arc;
/// use ferrumc_core::{WorldId, DimensionId, EntityUuid};
/// use ferrumc_math::ChunkPos;
/// use ferrumc_storage::{ChunkKey, PersistentEntityCursor, PersistentEntityEpoch};
/// use ferrumc_world::{GenerationIdentity, WorldSeed};
/// fn forge(epoch: PersistentEntityEpoch) {
///     let _ = PersistentEntityCursor {
///         owner: Arc::downgrade(&Arc::new(())),
///         chunk: ChunkKey::new(WorldId::new(0), DimensionId::new(0), ChunkPos::new(0, 0)),
///         expected: GenerationIdentity::flat_overworld(WorldSeed::new(0)),
///         epoch,
///         last: EntityUuid::from_bytes([0; 16]),
///     };
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PersistentEntityCursor {
    owner: Weak<()>,
    chunk: ChunkKey,
    expected: GenerationIdentity,
    epoch: PersistentEntityEpoch,
    last: EntityUuid,
}

impl PersistentEntityCursor {
    /// Returns the exact chunk column bound to this continuation.
    #[must_use]
    pub const fn chunk(&self) -> ChunkKey {
        self.chunk
    }

    /// Returns the complete expected generator identity bound to continuation.
    #[must_use]
    pub const fn expected_identity(&self) -> GenerationIdentity {
        self.expected
    }

    /// Returns the snapshot epoch; this is not an ownership CAS token.
    #[must_use]
    pub const fn epoch(&self) -> PersistentEntityEpoch {
        self.epoch
    }

    /// Returns the UUID of the last completely emitted entry.
    #[must_use]
    pub const fn last_uuid(&self) -> EntityUuid {
        self.last
    }
}

/// A bounded, checked prefix of one chunk's persistent entity set.
#[derive(Debug)]
pub struct PersistentEntityPage {
    entries: Vec<PersistentEntityEntry>,
    epoch: PersistentEntityEpoch,
    next: Option<PersistentEntityCursor>,
}
impl PersistentEntityPage {
    /// Borrows the emitted entries, in canonical UUID byte order.
    #[must_use]
    pub fn entries(&self) -> &[PersistentEntityEntry] {
        &self.entries
    }
    /// Returns the snapshot epoch used to bind continuation.
    #[must_use]
    pub const fn epoch(&self) -> PersistentEntityEpoch {
        self.epoch
    }
    /// Borrows continuation; absence proves encountered prefix exhaustion.
    #[must_use]
    pub const fn next(&self) -> Option<&PersistentEntityCursor> {
        self.next.as_ref()
    }
    /// Returns owned entries and continuation without copying payloads.
    #[must_use]
    pub fn into_parts(self) -> (Vec<PersistentEntityEntry>, Option<PersistentEntityCursor>) {
        (self.entries, self.next)
    }
}

/// Explicit UUID persistence; implementations default to Unsupported.
///
/// Saves, deletes and cross-dimension moves are atomic but have no durable
/// operation receipts or ownership CAS. Callers must resolve ambiguous replies
/// and may not assume blind retries or player/entity transactions are safe.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait PersistentEntityStore: Send + Sync {
    /// Loads the scoped entity after exact baseline and encountered-index checks.
    async fn load_persistent_entity(
        &self,
        _key: PersistentEntityKey,
        _expected: GenerationIdentity,
    ) -> Result<Option<PersistentEntityRecord>> {
        Err(StorageError::PersistentEntitiesUnsupported.into())
    }
    /// Atomically saves one entity within its dimension, including chunk changes.
    async fn save_persistent_entity(&self, _write: PersistentEntityWrite) -> Result<()> {
        Err(StorageError::PersistentEntitiesUnsupported.into())
    }
    /// Atomically saves at most64 proposals and4MiB encoded bytes; duplicates refuse.
    async fn save_persistent_entities(&self, _writes: Vec<PersistentEntityWrite>) -> Result<()> {
        Err(StorageError::PersistentEntitiesUnsupported.into())
    }
    /// Atomically deletes one scoped entity; a checked miss returns false.
    async fn delete_persistent_entity(
        &self,
        _key: PersistentEntityKey,
        _expected: GenerationIdentity,
    ) -> Result<bool> {
        Err(StorageError::PersistentEntitiesUnsupported.into())
    }
    /// Atomically changes dimensions for the same world/UUID and exact baselines.
    ///
    /// Source must exist; dimensions must differ; destination must be vacant.
    async fn move_persistent_entity(
        &self,
        _source: PersistentEntityKey,
        _source_expected: GenerationIdentity,
        _destination: PersistentEntityWrite,
    ) -> Result<()> {
        Err(StorageError::PersistentEntitiesUnsupported.into())
    }
    /// Returns1..64 entries and at most1MiB encoded key/envelope bytes.
    ///
    /// Continuations bind backend, chunk, exact identity and unchanged epoch.
    async fn load_persistent_entity_page(
        &self,
        _chunk: ChunkKey,
        _expected: GenerationIdentity,
        _cursor: Option<PersistentEntityCursor>,
        _limit: NonZeroU16,
    ) -> Result<PersistentEntityPage> {
        Err(StorageError::PersistentEntitiesUnsupported.into())
    }
}

pub(crate) fn require_mode(
    mode: crate::generation::GenerationMode,
) -> std::result::Result<(), StorageError> {
    if mode == crate::generation::GenerationMode::UuidEntities {
        Ok(())
    } else {
        Err(StorageError::PersistentEntitiesUnsupported)
    }
}

pub(crate) fn validate_cursor(
    cursor: &PersistentEntityCursor,
    owner: &Arc<()>,
    chunk: ChunkKey,
    expected: GenerationIdentity,
    epoch: Option<PersistentEntityEpoch>,
) -> std::result::Result<(), StorageError> {
    use crate::PersistentEntityCursorRejection as Reason;
    let reason = if !Weak::ptr_eq(&cursor.owner, &Arc::downgrade(owner))
        || cursor.owner.upgrade().is_none()
    {
        Some(Reason::Backend)
    } else if cursor.chunk != chunk {
        Some(Reason::Chunk)
    } else if cursor.expected != expected {
        Some(Reason::GenerationIdentity)
    } else if epoch.is_some_and(|epoch| cursor.epoch != epoch) {
        Some(Reason::Epoch)
    } else {
        None
    };
    match reason {
        Some(reason) => Err(StorageError::InvalidPersistentEntityCursor { reason }),
        None => Ok(()),
    }
}
