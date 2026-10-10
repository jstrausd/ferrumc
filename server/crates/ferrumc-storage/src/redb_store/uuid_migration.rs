//! Explicit offline Format4-to-5 migration with immutable restart evidence.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use ferrumc_core::ServerError;
use redb::Database;

use super::RedbStore;
use crate::StorageError;

mod backup;
mod checkpoint;
pub(super) mod files;
pub(super) mod validation;

#[cfg(all(test, unix))]
mod tests;

pub(in crate::redb_store) type MigrationResult<T> = Result<T, UuidEntityMigrationError>;

/// The temporary-file category whose explicit byte budget was exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UuidEntityMigrationScratchKind {
    /// A physical copy used to verify a closed backup without opening it.
    VerificationCopy,
    /// The disposable index used to check journal receipt overlap.
    ReceiptIndex,
}

/// Precise refusal or operational failure of the explicit offline migration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum UuidEntityMigrationError {
    /// Positive budgets and the combined checkpoint extent must fit u64.
    #[error("UUID migration requires positive budgets with representable combined scratch extent")]
    InvalidLimits,
    /// File identity and directory sync currently require Unix support.
    #[error("UUID migration requires Unix file identity and directory fsync")]
    UnsupportedPlatform,
    /// A file is already exclusively locked; no lock wait is attempted.
    #[error("UUID migration file is busy: {0}")]
    Busy(PathBuf),
    /// The final component of a supplied path is a symlink.
    #[error("UUID migration refuses a final symlink: {0}")]
    Symlink(PathBuf),
    /// A supplied file is not a suitable regular file.
    #[error("UUID migration requires a regular file with the expected length: {0}")]
    InvalidFile(PathBuf),
    /// A locked path no longer names the captured regular file.
    #[error("UUID migration path identity changed: {0}")]
    PathChanged(PathBuf),
    /// A file has unapproved hard links or aliases another role.
    #[error("UUID migration refuses an unrelated file alias: {0}")]
    UnexpectedFileAlias(PathBuf),
    /// An immutable backup/checkpoint destination already exists.
    #[error("UUID migration refuses to overwrite existing evidence: {0}")]
    EvidenceExists(PathBuf),
    /// The pinned redb header or geometry is unsupported or malformed.
    #[error("invalid or unsupported redb header; migration refused")]
    InvalidRedbHeader,
    /// Separate recovery is necessary before this offline operation.
    #[error("redb recovery is required before UUID migration")]
    RecoveryRequired,
    /// The requested source layout is not supported by this entry point.
    #[error("unsupported UUID migration source format {0:?}")]
    WrongFormat(Option<u64>),
    /// Format5 needs the pre-commit checkpoint to prove the exact delta.
    #[error("completed UUID migration verification requires the recorded checkpoint")]
    CompletedVerificationRequiresCheckpoint,
    /// The exact table catalog or table types do not match the format.
    #[error("UUID migration table catalog/type mismatch")]
    CatalogMismatch,
    /// A row grammar or a metadata invariant was refused.
    #[error("invalid UUID migration row in {table}: {reason}")]
    InvalidRow {
        /// Durable table containing the row.
        table: &'static str,
        /// Specific grammar or relationship failure.
        reason: &'static str,
    },
    /// An existing bounded storage decoder refused a record.
    #[error("UUID migration record in {table}: {source}")]
    Record {
        /// Durable table containing the record.
        table: &'static str,
        /// Original precise bounded-codec failure.
        #[source]
        source: Box<StorageError>,
    },
    /// Complete Format5 validation refused the durable source.
    #[error("complete UUID migration validation failed: {0}")]
    Validation(Box<ServerError>),
    /// Every legacy entity row needs a separate explicit identity-mapping ticket.
    #[error("UUID migration refuses existing wire-keyed entity rows")]
    LegacyEntitiesPresent,
    /// At most4096 real generation descriptors are admitted.
    #[error("UUID migration generation descriptor limit exceeded")]
    TooManyDescriptors,
    /// A checked row/byte counter could not represent the stored state.
    #[error("UUID migration counter overflow")]
    CounterOverflow,
    /// The source no longer equals the captured Format4 snapshot.
    #[error("UUID migration source snapshot changed")]
    SourceChanged,
    /// Closed backup bytes, identity or logical snapshot differ from evidence.
    #[error("UUID migration backup does not match recorded evidence")]
    BackupMismatch,
    /// The fixed checkpoint grammar, checksum or constants are malformed.
    #[error("malformed UUID migration checkpoint: {0}")]
    CheckpointMalformed(&'static str),
    /// Published checkpoint bytes or file identity changed.
    #[error("UUID migration checkpoint changed")]
    CheckpointChanged,
    /// The known staging name is missing, replaced or aliases unrelated data.
    #[error("UUID migration checkpoint staging identity changed: {0}")]
    CheckpointStagingChanged(PathBuf),
    /// Valid Format5 is not exactly the captured offline migration result.
    #[error("UUID migration result differs from the exact marker/empty-index delta")]
    MigrationDeltaMismatch,
    /// A temporary file cannot grow beyond its explicitly supplied budget.
    #[error("UUID migration {kind:?} byte budget {limit} exceeded")]
    ScratchBudgetExceeded {
        /// Temporary-file category.
        kind: UuidEntityMigrationScratchKind,
        /// Maximum admitted file extent in bytes.
        limit: u64,
    },
    /// Disposable cleanup failed or found a replaced path.
    #[error("UUID migration temporary cleanup failed for {path}: {source}")]
    Cleanup {
        /// Owned temporary path involved.
        path: PathBuf,
        /// Specific cleanup failure.
        #[source]
        source: io::Error,
    },
    /// A filesystem operation failed; its original error is retained.
    #[error("UUID migration {operation} failed for {path}: {source}")]
    Io {
        /// Action which failed.
        operation: &'static str,
        /// File/directory involved.
        path: PathBuf,
        /// Original I/O cause.
        #[source]
        source: io::Error,
    },
    /// A redb operation failed before the source commit boundary.
    #[error("UUID migration redb {operation} failed: {source}")]
    Backend {
        /// Action which failed.
        operation: &'static str,
        /// Original redb cause.
        #[source]
        source: Box<redb::Error>,
    },
    /// Commit failed operationally and may already have changed durable state.
    #[error("UUID migration commit outcome is uncertain; use checked resume: {source}")]
    CommitUncertain {
        /// Original commit cause; failure is not an abort acknowledgement.
        #[source]
        source: Box<redb::Error>,
    },
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> UuidEntityMigrationError {
    UuidEntityMigrationError::Io {
        operation,
        path: path.to_owned(),
        source,
    }
}

fn backend(operation: &'static str, source: impl Into<redb::Error>) -> UuidEntityMigrationError {
    UuidEntityMigrationError::Backend {
        operation,
        source: Box::new(source.into()),
    }
}

impl From<ServerError> for UuidEntityMigrationError {
    fn from(error: ServerError) -> Self {
        Self::Validation(Box::new(error))
    }
}

/// Explicit per-operation temporary file extent limits; no unlimited default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UuidEntityMigrationLimits {
    copy_bytes: u64,
    receipt_bytes: u64,
}

impl UuidEntityMigrationLimits {
    /// Admits positive budgets whose combined temporary extent is representable.
    pub fn new(
        max_verification_copy_bytes: u64,
        max_receipt_index_bytes: u64,
    ) -> MigrationResult<Self> {
        if max_verification_copy_bytes == 0
            || max_receipt_index_bytes == 0
            || max_verification_copy_bytes
                .checked_add(max_receipt_index_bytes)
                .and_then(|sum| sum.checked_add(checkpoint::LENGTH as u64))
                .is_none()
        {
            return Err(UuidEntityMigrationError::InvalidLimits);
        }
        Ok(Self {
            copy_bytes: max_verification_copy_bytes,
            receipt_bytes: max_receipt_index_bytes,
        })
    }
    /// Maximum physical verification-copy extent, including redb housekeeping.
    pub const fn max_verification_copy_bytes(self) -> u64 {
        self.copy_bytes
    }
    /// Maximum disposable receipt-index file extent.
    pub const fn max_receipt_index_bytes(self) -> u64 {
        self.receipt_bytes
    }
}

/// Stable twelve-slot categories of the separate migration fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum UuidEntityMigrationTable {
    /// Format and journal watermark.
    Metadata,
    /// Actual immutable generation identities.
    GenerationIdentities,
    /// Complete chunk snapshots.
    Chunks,
    /// Edited overlays, including empty overlays.
    ChunkOverlays,
    /// Entity table; migration requires it to be empty.
    Entities,
    /// New world/UUID locator table.
    EntityLocators,
    /// New chunk/UUID index table.
    ChunkEntities,
    /// New historical chunk epoch table.
    ChunkEntityEpochs,
    /// Existing opaque player records.
    Players,
    /// Existing namespaced plugin values.
    PluginValues,
    /// Contiguous mutation records.
    JournalRecords,
    /// Batch receipts, including empty receipts.
    JournalReceipts,
}

/// Complete captured Format4 baseline; clones do not retain a file lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UuidEntityMigrationReport {
    source_path: PathBuf,
    source_identity: files::FileIdentity,
    source_file_len: u64,
    scan: validation::Scan,
}

impl UuidEntityMigrationReport {
    /// Canonical path observed at preparation, not a retained lock.
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }
    /// Source layout marker; this report always describes Format4.
    pub const fn source_format(&self) -> u64 {
        4
    }
    /// Only supported target layout.
    pub const fn target_format(&self) -> u64 {
        5
    }
    /// Observed source physical length; never a retry equality criterion.
    pub const fn source_file_len(&self) -> u64 {
        self.source_file_len
    }
    /// Completely validated row count in the stable category.
    pub const fn records(&self, table: UuidEntityMigrationTable) -> u64 {
        self.scan.counts[table as usize]
    }
    /// Sum of raw key/value bytes, independently of physical redb pages.
    pub const fn logical_bytes(&self) -> u64 {
        self.scan.logical_bytes
    }
    /// Full actual4 fingerprint, including all twelve table presence markers.
    pub const fn snapshot_sha256(&self) -> &[u8; 32] {
        &self.scan.sha256
    }
    /// Exact expected5 fingerprint; only marker and three table presences differ.
    pub const fn expected_target_sha256(&self) -> &[u8; 32] {
        &self.scan.target_sha256
    }
}

/// Fully verified closed backup; evidence alone has no commit authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UuidEntityMigrationBackup {
    path: PathBuf,
    identity: files::FileIdentity,
    file_len: u64,
    file_sha256: [u8; 32],
    snapshot_sha256: [u8; 32],
}

impl UuidEntityMigrationBackup {
    /// Separate canonical backup path, never silently overwritten.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Verified closed physical file length.
    pub const fn file_len(&self) -> u64 {
        self.file_len
    }
    /// SHA256 of the closed original backup; verification never opens it as redb.
    pub const fn file_sha256(&self) -> &[u8; 32] {
        &self.file_sha256
    }
    /// Completely validated Format4 logical snapshot reproduced by the backup.
    pub const fn snapshot_sha256(&self) -> &[u8; 32] {
        &self.snapshot_sha256
    }
}

/// Exclusive complete4 preflight; it cannot commit before backup and checkpoint.
pub struct PreparedUuidEntityMigration {
    db: Database,
    source: files::LockedFile,
    report: UuidEntityMigrationReport,
    limits: UuidEntityMigrationLimits,
}

impl fmt::Debug for PreparedUuidEntityMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedUuidEntityMigration")
            .field("report", &self.report)
            .finish_non_exhaustive()
    }
}

/// Exclusive verified backup authority; only checkpoint publication enables commit.
///
/// ```compile_fail
/// fn no_commit_without_checkpoint(value: ferrumc_storage::BackedUpUuidEntityMigration) {
///     value.migrate();
/// }
/// ```
pub struct BackedUpUuidEntityMigration {
    prepared: PreparedUuidEntityMigration,
    backup_file: files::LockedFile,
    backup: UuidEntityMigrationBackup,
}

impl fmt::Debug for BackedUpUuidEntityMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackedUpUuidEntityMigration")
            .field("report", &self.prepared.report)
            .field("backup", &self.backup)
            .finish_non_exhaustive()
    }
}

/// The sole commit authority, holding source, backup and durable checkpoint locks.
pub struct CheckpointedUuidEntityMigration {
    backed: Box<BackedUpUuidEntityMigration>,
    checkpoint: Box<checkpoint::Authority>,
}

impl fmt::Debug for CheckpointedUuidEntityMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckpointedUuidEntityMigration")
            .field("backed", &self.backed)
            .field("checkpoint", &self.checkpoint.path())
            .finish_non_exhaustive()
    }
}

/// Whether success came from an actual commit or full checked completed verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UuidEntityMigrationOutcomeKind {
    /// A real Immediate-durability source transaction committed.
    Committed,
    /// Complete source/backup/checkpoint validation proved the exact prior delta.
    VerifiedCompleted,
}

/// Evidence returned only after commit or complete checked resume.
#[derive(Debug, Clone)]
pub struct UuidEntityMigrationOutcome {
    report: Box<UuidEntityMigrationReport>,
    backup: Box<UuidEntityMigrationBackup>,
    checkpoint_path: PathBuf,
    checkpoint_sha256: [u8; 32],
    target_sha256: [u8; 32],
    kind: UuidEntityMigrationOutcomeKind,
}

impl UuidEntityMigrationOutcome {
    /// Captured pre-migration4 baseline; physical source length is informational.
    pub fn report(&self) -> &UuidEntityMigrationReport {
        &self.report
    }
    /// Immutable verified closed backup evidence.
    pub fn backup(&self) -> &UuidEntityMigrationBackup {
        &self.backup
    }
    /// Immutable published checkpoint path.
    pub fn checkpoint_path(&self) -> &Path {
        &self.checkpoint_path
    }
    /// SHA256 of the complete344-byte checkpoint file.
    pub const fn checkpoint_sha256(&self) -> &[u8; 32] {
        &self.checkpoint_sha256
    }
    /// Full migration-grammar fingerprint of the checked5 result.
    pub const fn target_snapshot_sha256(&self) -> &[u8; 32] {
        &self.target_sha256
    }
    /// Actual commit versus checked completed verification.
    pub const fn kind(&self) -> UuidEntityMigrationOutcomeKind {
        self.kind
    }
}

/// Explicit checked restart result; generic valid5 is never sufficient.
#[derive(Debug)]
pub enum UuidEntityMigrationResume {
    /// Captured4 was unchanged, all immutable evidence verified, commit is enabled.
    Ready(CheckpointedUuidEntityMigration),
    /// Complete5 exactly equals the captured marker/three-empty-table delta.
    AlreadyCompleted(UuidEntityMigrationOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    BackupCreated,
    BackupRow,
    BackupCommitted,
    BackupClosed,
    BackupSynced,
    BackupHashed,
    VerificationCreated,
    VerificationCopied,
    VerificationValidated,
    VerificationCleaned,
    CheckpointCreated,
    CheckpointWritten,
    CheckpointFileSynced,
    CheckpointLinked,
    CheckpointDirectorySynced,
    CheckpointStagingUnlinked,
    CheckpointReady,
    TableCreated(usize),
    MarkerWritten,
    StagedValidated,
    BeforeCommit,
    Committed,
}

type Observer<'a> = &'a mut dyn FnMut(Stage) -> MigrationResult<()>;

impl RedbStore {
    /// Completely validates existing Format4 offline and retains its exclusive lock.
    ///
    /// Synchronous blocking I/O: never call on an async worker or simulation tick.
    /// Does not create a missing source, infer UUIDs/seeds, repair, or auto-upgrade.
    /// File locks fail fast with `Busy`. Physical source housekeeping is permitted;
    /// `FerrumC` tables/rows/marker remain unchanged until the explicit commit.
    pub fn prepare_uuid_entity_migration(
        path: impl AsRef<Path>,
        limits: UuidEntityMigrationLimits,
    ) -> MigrationResult<PreparedUuidEntityMigration> {
        let source = files::LockedFile::open(path.as_ref(), false, false)?;
        let db = source.open_database(false, None)?;
        let txn = db.begin_write().map_err(|e| backend("prepare", e))?;
        let scan = validation::scan_format4(&txn, &source.path, limits)?;
        txn.abort().map_err(|e| backend("abort preflight", e))?;
        let report = UuidEntityMigrationReport {
            source_path: source.path.clone(),
            source_identity: source.identity,
            source_file_len: source.len()?,
            scan,
        };
        Ok(PreparedUuidEntityMigration {
            db,
            source,
            report,
            limits,
        })
    }
}

impl PreparedUuidEntityMigration {
    /// Captured complete4 report; this object, not a report clone, holds the lock.
    pub fn report(&self) -> &UuidEntityMigrationReport {
        &self.report
    }
}

impl BackedUpUuidEntityMigration {
    /// Complete4 baseline while retaining exclusive source/backup locks.
    pub fn report(&self) -> &UuidEntityMigrationReport {
        &self.prepared.report
    }
    /// Verified closed backup evidence.
    pub fn backup(&self) -> &UuidEntityMigrationBackup {
        &self.backup
    }
}

impl CheckpointedUuidEntityMigration {
    /// Captured complete4 baseline.
    pub fn report(&self) -> &UuidEntityMigrationReport {
        &self.backed.prepared.report
    }
    /// Verified closed backup evidence.
    pub fn backup(&self) -> &UuidEntityMigrationBackup {
        &self.backed.backup
    }
    /// Canonical immutable checkpoint path.
    pub fn checkpoint_path(&self) -> &Path {
        self.checkpoint.path()
    }

    /// Atomically creates the three empty indexes and changes4 to5 with Immediate durability.
    ///
    /// Blocking offline I/O. Fully rechecks source, immutable backup and checkpoint
    /// before commit. `CommitUncertain` requires checked resume: it is not an abort
    /// acknowledgement. The checkpoint and original backup are never rewritten.
    pub fn migrate(self) -> MigrationResult<UuidEntityMigrationOutcome> {
        self.migrate_observed(&mut |_| Ok(()))
    }

    fn verify_files(&self) -> MigrationResult<()> {
        self.backed.prepared.source.verify_single_link()?;
        self.backed.backup_file.verify_single_link()?;
        self.checkpoint.verify(true)?;
        if self.backed.prepared.source.identity != self.checkpoint.data.source
            || self.backed.backup_file.identity != self.checkpoint.data.backup
            || self.backed.backup_file.hash()?
                != (
                    self.checkpoint.data.backup_len,
                    self.checkpoint.data.backup_hash,
                )
        {
            return Err(UuidEntityMigrationError::BackupMismatch);
        }
        if self.checkpoint.identity() == self.backed.prepared.source.identity
            || self.checkpoint.identity() == self.backed.backup_file.identity
        {
            return Err(UuidEntityMigrationError::UnexpectedFileAlias(
                self.checkpoint.path().to_owned(),
            ));
        }
        Ok(())
    }

    fn outcome(&self, kind: UuidEntityMigrationOutcomeKind) -> UuidEntityMigrationOutcome {
        UuidEntityMigrationOutcome {
            report: Box::new(self.backed.prepared.report.clone()),
            backup: Box::new(self.backed.backup.clone()),
            checkpoint_path: self.checkpoint.path().to_owned(),
            checkpoint_sha256: self.checkpoint.hash(),
            target_sha256: self.checkpoint.data.scan.target_sha256,
            kind,
        }
    }

    fn migrate_observed(
        self,
        observe: Observer<'_>,
    ) -> MigrationResult<UuidEntityMigrationOutcome> {
        self.verify_files()?;
        let prepared = &self.backed.prepared;
        let mut txn = prepared
            .db
            .begin_write()
            .map_err(|e| backend("source migration transaction", e))?;
        txn.set_durability(redb::Durability::Immediate);
        let before = validation::scan_format4(&txn, &prepared.source.path, prepared.limits)?;
        if before != prepared.report.scan || before != self.checkpoint.data.scan {
            return Err(UuidEntityMigrationError::SourceChanged);
        }
        for (index, table) in [
            super::uuid_entities::LOCATOR_TABLE,
            super::uuid_entities::INDEX_TABLE,
            super::uuid_entities::EPOCH_TABLE,
        ]
        .into_iter()
        .enumerate()
        {
            txn.open_table(table)
                .map_err(|e| backend("create UUID index", e))?;
            observe(Stage::TableCreated(index))?;
        }
        txn.open_table(super::META_TABLE)
            .map_err(|e| backend("source marker table", e))?
            .insert(super::META_FORMAT_KEY, 5u64)
            .map_err(|e| backend("source marker write", e))?;
        observe(Stage::MarkerWritten)?;
        let after = validation::scan_format5(&txn, &prepared.source.path, prepared.limits)?;
        if after.counts != before.counts
            || after.logical_bytes != before.logical_bytes
            || after.sha256 != before.target_sha256
        {
            return Err(UuidEntityMigrationError::MigrationDeltaMismatch);
        }
        let backup = backup::verify_closed_copy(
            &self.backed.backup_file,
            (self.backed.backup.file_len, self.backed.backup.file_sha256),
            prepared.limits,
            Some(&txn),
            observe,
        )?;
        if backup != before {
            return Err(UuidEntityMigrationError::BackupMismatch);
        }
        observe(Stage::StagedValidated)?;
        observe(Stage::BeforeCommit)?;
        self.verify_files()?;
        txn.commit()
            .map_err(|e| UuidEntityMigrationError::CommitUncertain {
                source: Box::new(e.into()),
            })?;
        observe(Stage::Committed)?;
        Ok(self.outcome(UuidEntityMigrationOutcomeKind::Committed))
    }
}

impl RedbStore {
    /// Fully checks immutable restart evidence and either returns4 authority or proves5 completion.
    ///
    /// Blocking offline I/O; no lock waits or automatic recovery. A valid generic5
    /// source is insufficient: all twelve presences/counts/hash and the exact raw
    /// marker/empty-index delta must equal the recorded pre-commit checkpoint.
    /// Source physical length/hash are never retry equality criteria.
    pub fn resume_uuid_entity_migration(
        source_path: impl AsRef<Path>,
        checkpoint_path: impl AsRef<Path>,
        backup_path: impl AsRef<Path>,
        limits: UuidEntityMigrationLimits,
    ) -> MigrationResult<UuidEntityMigrationResume> {
        Self::resume_uuid_observed(
            source_path.as_ref(),
            checkpoint_path.as_ref(),
            backup_path.as_ref(),
            limits,
            &mut |_| Ok(()),
        )
    }

    fn resume_uuid_observed(
        source_path: &Path,
        checkpoint_path: &Path,
        backup_path: &Path,
        limits: UuidEntityMigrationLimits,
        observe: Observer<'_>,
    ) -> MigrationResult<UuidEntityMigrationResume> {
        let checkpoint = checkpoint::Authority::load(checkpoint_path)?;
        let (source, backup_file) = open_resume_files(&checkpoint, source_path, backup_path)?;
        let physical = (checkpoint.data.backup_len, checkpoint.data.backup_hash);
        if backup_file.hash()? != physical {
            return Err(UuidEntityMigrationError::BackupMismatch);
        }
        let db = source.open_database(false, None)?;
        let txn = db
            .begin_write()
            .map_err(|e| backend("resume transaction", e))?;
        let format = validation::marker(&txn)?;
        let current = match format {
            Some(4) => validation::scan_format4(&txn, &source.path, limits)?,
            Some(5) => validation::scan_format5(&txn, &source.path, limits)?,
            other => return Err(UuidEntityMigrationError::WrongFormat(other)),
        };
        let baseline = &checkpoint.data.scan;
        let matches = if current.format == 4 {
            current == *baseline
        } else {
            current.counts == baseline.counts
                && current.logical_bytes == baseline.logical_bytes
                && current.sha256 == baseline.target_sha256
        };
        if !matches {
            return Err(if current.format == 4 {
                UuidEntityMigrationError::SourceChanged
            } else {
                UuidEntityMigrationError::MigrationDeltaMismatch
            });
        }
        let backup_scan = backup::verify_closed_copy(
            &backup_file,
            physical,
            limits,
            (current.format == 5).then_some(&txn),
            observe,
        )?;
        if backup_scan != *baseline {
            return Err(UuidEntityMigrationError::BackupMismatch);
        }
        txn.abort()
            .map_err(|e| backend("abort resume validation", e))?;
        source.verify_single_link()?;
        backup_file.verify_single_link()?;
        checkpoint.verify(false)?;
        if backup_file.hash()? != physical {
            return Err(UuidEntityMigrationError::BackupMismatch);
        }
        checkpoint.stabilize(observe)?;
        let report = UuidEntityMigrationReport {
            source_path: source.path.clone(),
            source_identity: source.identity,
            source_file_len: checkpoint.data.source_file_len,
            scan: baseline.clone(),
        };
        let backup = UuidEntityMigrationBackup {
            path: backup_file.path.clone(),
            identity: backup_file.identity,
            file_len: physical.0,
            file_sha256: physical.1,
            snapshot_sha256: baseline.sha256,
        };
        let value = CheckpointedUuidEntityMigration {
            backed: Box::new(BackedUpUuidEntityMigration {
                prepared: PreparedUuidEntityMigration {
                    db,
                    source,
                    report,
                    limits,
                },
                backup_file,
                backup,
            }),
            checkpoint: Box::new(checkpoint),
        };
        value.verify_files()?;
        if current.format == 4 {
            Ok(UuidEntityMigrationResume::Ready(value))
        } else {
            Ok(UuidEntityMigrationResume::AlreadyCompleted(value.outcome(
                UuidEntityMigrationOutcomeKind::VerifiedCompleted,
            )))
        }
    }
}

fn open_resume_files(
    checkpoint: &checkpoint::Authority,
    source_path: &Path,
    backup_path: &Path,
) -> MigrationResult<(files::LockedFile, files::LockedFile)> {
    let source = files::LockedFile::open(source_path, false, false)?;
    if source.identity != checkpoint.data.source {
        return Err(UuidEntityMigrationError::SourceChanged);
    }
    if source.identity == checkpoint.identity()
        || files::canonical_path(backup_path)? == source.path
    {
        return Err(UuidEntityMigrationError::UnexpectedFileAlias(
            source.path.clone(),
        ));
    }
    let backup_file = files::LockedFile::open(backup_path, false, false)?;
    backup_file.separate_from(&source)?;
    if backup_file.identity != checkpoint.data.backup
        || backup_file.identity == checkpoint.identity()
    {
        return Err(UuidEntityMigrationError::BackupMismatch);
    }
    files::require_clean_header(&backup_file.file, &backup_file.path)?;
    Ok((source, backup_file))
}
