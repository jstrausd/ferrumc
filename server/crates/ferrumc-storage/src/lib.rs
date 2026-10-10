#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod codec;
mod error;
mod generation;
mod journal;
mod key;
mod memory;
mod ownership;
mod persistent_entity;
mod record;
mod redb_store;
mod schema;
mod store;

pub use error::{PersistentEntityCursorRejection, StorageError};
pub use generation::{GenerationInitialization, GenerationKey, MAX_GENERATION_IDENTITIES};
pub use journal::{JournalAppendReceipt, JournalBatchId};
pub use key::{ChunkKey, EntityKey, StorageKey, MAX_PLUGIN_KEY_LEN};
pub use memory::InMemoryStore;
pub use ownership::{
    EntityOwnershipCasRequest, EntityOwnershipProposal, OwnedEntityExpectation,
    OwnedEntitySnapshot, OwnedPlayerEntitySnapshot, OwnedPlayerExpectation, OwnedPlayerSnapshot,
    OwnershipCapacity, OwnershipCommitDisposition, OwnershipCommitOutcome, OwnershipConfig,
    OwnershipEpochChange, OwnershipError, OwnershipHistory, OwnershipOpenLimits, OwnershipReceipt,
    OwnershipReceiptQuery, OwnershipRequestIdentity, OwnershipResource, OwnershipResult,
    OwnershipRevision, OwnershipStamp, OwnershipTransferId, PlayerEntityOwnershipStore,
    PlayerEntityTransferRequest, PlayerOwnershipCasRequest, PlayerOwnershipProposal,
    ScopedPlayerKey, MAX_OWNERSHIP_INFLIGHT, MAX_OWNERSHIP_REQUEST_BYTES,
};
pub use persistent_entity::{
    PersistentEntityCursor, PersistentEntityEntry, PersistentEntityEpoch, PersistentEntityKey,
    PersistentEntityPage, PersistentEntityRecord, PersistentEntityStore, PersistentEntityWrite,
    MAX_PERSISTENT_ENTITY_BATCH_BYTES, MAX_PERSISTENT_ENTITY_PAGE_BYTES,
    MAX_PERSISTENT_ENTITY_PAGE_ENTRIES, MAX_PERSISTENT_ENTITY_WRITES,
};
pub use record::{
    BlockMutationLogRecord, ChunkOverlayRecord, ChunkRecord, EntityRecord, MutationActor,
    MutationLogCause, PlayerRecord, MAX_CHUNK_BLOCK_ENTITY_DATA_LEN, MAX_CHUNK_RECORD_LEN,
    MAX_ENTITY_DATA_LEN, MAX_OVERLAY_BLOCK_ENTITIES, MAX_OVERLAY_SECTIONS, MAX_PLAYER_DATA_LEN,
};
pub use redb_store::migration::{
    BackedUpGenerationMigration, GenerationMigrationBackup, GenerationMigrationError,
    GenerationMigrationOutcome, GenerationMigrationPreparation, GenerationMigrationReport,
    GenerationMigrationTable, LegacyGenerationDeclaration, PreparedGenerationMigration,
    MAX_MIGRATION_PLUGIN_ID_LEN,
};
pub use redb_store::uuid_migration::{
    BackedUpUuidEntityMigration, CheckpointedUuidEntityMigration, PreparedUuidEntityMigration,
    UuidEntityMigrationBackup, UuidEntityMigrationError, UuidEntityMigrationLimits,
    UuidEntityMigrationOutcome, UuidEntityMigrationOutcomeKind, UuidEntityMigrationReport,
    UuidEntityMigrationResume, UuidEntityMigrationScratchKind, UuidEntityMigrationTable,
};
pub use redb_store::RedbStore;
pub use schema::SchemaVersion;
pub use store::{
    PlayerStore, PluginStore, WorldStore, MAX_PLUGIN_VALUE_LEN, MAX_SAVE_BATCH,
    MAX_UUID_STORE_PLUGIN_ID_LEN,
};
