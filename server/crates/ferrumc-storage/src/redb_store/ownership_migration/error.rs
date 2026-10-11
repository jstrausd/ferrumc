//! Precise bounded migration refusals; no successful fallback.

use crate::{ChunkKey, GenerationKey, OwnershipError, StorageError};
use ferrumc_core::PlayerId;
use std::io;

/// Role of a caller-selected, identity-bound offline file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipMigrationFileRole {
    /// Locked original database.
    Source,
    /// Explicit player scope mapping.
    Mapping,
    /// Immutable closed Format5 backup.
    Backup,
    /// Immutable resource identity/import plan.
    Plan,
    /// Immutable genuine CAS-produced Format6 staging database.
    Stage,
    /// Immutable restart evidence.
    Checkpoint,
    /// Owned disposable bounded scratch.
    Scratch,
}

/// Specific refusal or failure of explicit offline Format5-to-6 migration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OwnershipMigrationError {
    /// A positive budget exceeded its reviewed hard maximum or integer shape.
    #[error("invalid ownership migration budget")]
    InvalidLimit,
    /// File identity and directory fsync require Unix.
    #[error("ownership migration requires Unix")]
    UnsupportedPlatform,
    /// An exclusive lock was unavailable; no wait or write occurred.
    #[error("ownership migration source or evidence is busy")]
    SourceBusy,
    /// The original catalog is not the explicit supported Format5 source.
    #[error("ownership migration requires Format5, found {0:?}")]
    UnsupportedSourceFormat(Option<u64>),
    /// Separate redb recovery is required before migration.
    #[error("ownership migration refuses recovery-required data")]
    RecoveryRequired,
    /// Table catalog or types differ from the exact supported grammar.
    #[error("ownership migration catalog mismatch")]
    CatalogMismatch,
    /// A complete original row decoder refused data.
    #[error("invalid ownership migration source row: {0}")]
    MalformedSourceRow(#[source] Box<StorageError>),
    /// The explicit mapping omitted this actual original player.
    #[error("player mapping missing {0:?}")]
    MappingMissingPlayer(PlayerId),
    /// An explicit mapping refers to no actual original player.
    #[error("unused player mapping {0:?}")]
    MappingUnusedPlayer(PlayerId),
    /// More than one mapping row names the same player.
    #[error("duplicate player mapping")]
    MappingDuplicate,
    /// Fixed rows are not strictly sorted.
    #[error("player mapping is not sorted")]
    MappingUnsorted,
    /// Scope or complete identity differs from the real source descriptor.
    #[error("generation mapping mismatch for {0:?}")]
    GenerationMismatch(GenerationKey),
    /// A recorded actual `UUIDv4` operation ID occurred twice.
    #[error("duplicate import operation ID")]
    DuplicateImportId,
    /// Before any bootstrap, the real epoch plus entity count was exhausted.
    #[error("ownership migration epoch exhausted for {chunk:?}: {source_epoch}+{count}")]
    EpochExhausted {
        /// Actual chunk.
        chunk: ChunkKey,
        /// Existing retained epoch.
        source_epoch: u64,
        /// Actual entities to import into that chunk.
        count: u64,
    },
    /// A checked streamed raw rows/bytes bound was exceeded.
    #[error("ownership migration {kind} needs {required}, limit {limit}")]
    ScanCapacity {
        /// Logical resource category.
        kind: &'static str,
        /// Actual checked requirement.
        required: u64,
        /// Operator maximum.
        limit: u64,
    },
    /// Immutable plan row or representable extent bound was exceeded.
    #[error("ownership migration plan capacity exceeded")]
    PlanCapacity,
    /// Combined logical or physical disposable index bound was exceeded.
    #[error("ownership migration scratch capacity exceeded")]
    ScratchCapacity,
    /// Actual copied file extent or redb housekeeping growth exceeded its budget.
    #[error("ownership migration verification copy capacity exceeded")]
    VerificationCopyCapacity,
    /// Actual fixed checkpoint plus selected paths exceeded its explicit bound.
    #[error("ownership migration checkpoint capacity exceeded")]
    CheckpointCapacity,
    /// Import resources exceed the real persisted receipt/revision agreement.
    #[error("ownership migration history capacity exceeded")]
    HistoryCapacity,
    /// An original proposal cannot fit the actual canonical request agreement.
    #[error("ownership migration request capacity exceeded")]
    RequestCapacity,
    /// Two selected paths or file identities alias different roles.
    #[error("ownership migration file alias refused")]
    PathAlias,
    /// Final path component is a symlink.
    #[error("ownership migration symlink refused")]
    SymlinkRefused,
    /// A retained file no longer names the owned inode.
    #[error("ownership migration file replaced: {0:?}")]
    FileReplaced(OwnershipMigrationFileRole),
    /// Evidence destinations must be new, never overwritten.
    #[error("ownership migration evidence already exists: {0:?}")]
    EvidenceExists(OwnershipMigrationFileRole),
    /// Exact mapping envelope/row grammar is malformed.
    #[error("malformed ownership migration mapping: {0}")]
    MalformedMapping(&'static str),
    /// Exact immutable plan envelope/row grammar is malformed.
    #[error("malformed ownership migration plan: {0}")]
    MalformedPlan(&'static str),
    /// Exact immutable checkpoint grammar, digest or binding is malformed.
    #[error("malformed ownership migration checkpoint: {0}")]
    MalformedCheckpoint(&'static str),
    /// Original logical rows changed since the captured locked preparation.
    #[error("ownership migration source drift")]
    SourceDrift,
    /// Immutable bytes, identity or logical evidence differs from its binding.
    #[error("ownership migration evidence drift: {0:?}")]
    EvidenceDrift(OwnershipMigrationFileRole),
    /// The actual existing Stage read/request/CAS producer refused an operation.
    #[error("ownership migration Stage CAS refused: {0}")]
    StageCasFailure(#[source] OwnershipError),
    /// A genuine Stage outcome is not the exact requested committed bootstrap.
    #[error("ownership migration Stage outcome mismatch")]
    StageOutcomeMismatch,
    /// Entire expected15-table target differs from the declared original delta.
    #[error("ownership migration target delta mismatch")]
    TargetDeltaMismatch,
    /// Existing fully bounded validator or file producer refusal, retained typed.
    #[error("ownership migration existing producer refused: {0}")]
    Producer(#[source] Box<super::super::uuid_migration::UuidEntityMigrationError>),
    /// Actual I/O cause with the exact bounded operation/file role.
    #[error("ownership migration {operation} on {role:?}: {source}")]
    Io {
        /// Failed operation.
        operation: &'static str,
        /// Caller-selected or owned file role.
        role: OwnershipMigrationFileRole,
        /// Original error.
        #[source]
        source: io::Error,
    },
    /// Actual redb/backend failure before source commit.
    #[error("ownership migration backend {operation}: {cause}")]
    Backend {
        /// Failed operation.
        operation: &'static str,
        /// Original backend diagnostic.
        cause: String,
    },
    /// Actual source commit or subsequent verification needs exact resume.
    #[error("ownership migration commit uncertain: {cause}")]
    CommitUncertain {
        /// Existing immutable checkpoint identity.
        checkpoint: [u8; 16],
        /// Exact expected committed logical target.
        target: [u8; 32],
        /// Actual diagnostic; no repeated bootstrap is authorized.
        cause: String,
    },
}

impl From<StorageError> for OwnershipMigrationError {
    fn from(error: StorageError) -> Self {
        Self::MalformedSourceRow(Box::new(error))
    }
}
impl From<OwnershipError> for OwnershipMigrationError {
    fn from(error: OwnershipError) -> Self {
        Self::StageCasFailure(error)
    }
}
impl From<super::super::uuid_migration::UuidEntityMigrationError> for OwnershipMigrationError {
    fn from(error: super::super::uuid_migration::UuidEntityMigrationError) -> Self {
        use super::super::uuid_migration::UuidEntityMigrationError as E;
        match error {
            E::UnsupportedPlatform => Self::UnsupportedPlatform,
            E::Busy(_) => Self::SourceBusy,
            E::RecoveryRequired => Self::RecoveryRequired,
            E::Symlink(_) => Self::SymlinkRefused,
            E::UnexpectedFileAlias(_) => Self::PathAlias,
            E::ScratchBudgetExceeded { kind, .. } => match kind {
                super::super::uuid_migration::UuidEntityMigrationScratchKind::VerificationCopy => {
                    Self::VerificationCopyCapacity
                }
                super::super::uuid_migration::UuidEntityMigrationScratchKind::ReceiptIndex => {
                    Self::ScratchCapacity
                }
            },
            other => Self::Producer(Box::new(other)),
        }
    }
}
