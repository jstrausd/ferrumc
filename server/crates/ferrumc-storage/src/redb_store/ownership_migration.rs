//! Explicit offline Format5-to-6 migration through genuine private Singles CAS.

use super::uuid_migration::{
    files::{self, DiskBudget, LockedFile, TemporaryFile},
    UuidEntityMigrationScratchKind,
};
use crate::redb_store::{
    ownership, uuid_entities, CHUNK_OVERLAY_TABLE, CHUNK_TABLE, ENTITY_TABLE, GENERATION_TABLE,
    META_FORMAT_KEY, META_TABLE, MUTATION_BATCH_TABLE, MUTATION_LOG_TABLE, PLAYER_TABLE,
    PLUGIN_TABLE,
};
use crate::{OwnershipConfig, RedbStore};
use redb::{Database, Durability, ReadableTable, TableDefinition};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

mod checkpoint;
mod error;
mod index;
mod limits;
mod mapping;
mod paths;
mod plan;
mod source;
mod stage;
#[cfg(all(test, unix))]
mod tests;
mod verify;

pub use error::{OwnershipMigrationError, OwnershipMigrationFileRole};
pub use limits::OwnershipMigrationLimits;
pub use paths::{OwnershipMigrationOutputPaths, OwnershipMigrationResumePaths};

type Error = OwnershipMigrationError;
type Role = OwnershipMigrationFileRole;
type Result<T> = std::result::Result<T, Error>;
type Hook<'a> = &'a mut dyn FnMut(Cutpoint) -> Result<()>;

const TABLES: [TableDefinition<'static, &'static [u8], &'static [u8]>; 14] = [
    GENERATION_TABLE,
    CHUNK_TABLE,
    CHUNK_OVERLAY_TABLE,
    ENTITY_TABLE,
    uuid_entities::LOCATOR_TABLE,
    uuid_entities::INDEX_TABLE,
    uuid_entities::EPOCH_TABLE,
    PLAYER_TABLE,
    PLUGIN_TABLE,
    MUTATION_LOG_TABLE,
    MUTATION_BATCH_TABLE,
    ownership::PLAYER_OWNER,
    ownership::ENTITY_OWNER,
    ownership::RECEIPT,
];

fn backend(operation: &'static str, cause: impl std::fmt::Display) -> Error {
    Error::Backend {
        operation,
        cause: cause.to_string(),
    }
}
fn io(operation: &'static str, role: Role, source: std::io::Error) -> Error {
    Error::Io {
        operation,
        role,
        source,
    }
}
fn metadata(config: OwnershipConfig, n: u64) -> [(&'static str, u64); 5] {
    [
        (ownership::RECEIPT_COUNT, n),
        (ownership::REVISION_COUNT, n),
        (
            ownership::REQUEST_LIMIT,
            config.capacity().max_request_bytes() as u64,
        ),
        (ownership::RECEIPT_LIMIT, config.capacity().max_receipts()),
        (
            ownership::REVISION_LIMIT,
            config.capacity().max_revision_rows(),
        ),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cutpoint {
    BeforePreparationRow,
    AfterPreparationRow,
    BeforePlanRow,
    AfterPlanRow,
    BeforeStageCas,
    AfterStageCas,
    BeforeCopyCreate,
    AfterCopyCreate,
    BeforeCopyWrite,
    AfterCopyWrite,
    BeforeCopyOpen,
    AfterCopyOpen,
    BeforeCopyScan,
    AfterCopyScan,
    BeforeCopyClose,
    AfterCopyClose,
    BeforeCopyCleanup,
    AfterCopyCleanup,
    BeforeRow,
    AfterRow,
    BeforeMeta,
    AfterMeta,
    BeforeMarker,
    AfterMarker,
    BeforeTargetScan,
    AfterTargetScan,
    BeforeCommit,
    AfterCommit,
    BeforeReopen,
    AfterReopen,
}

/// Move-only exclusive offline Source5 preparation, with bounded validated inputs.
/// Synchronous I/O; no live gameplay/store capability is returned.
pub struct PreparedOwnershipMigration {
    db: Option<Database>,
    source: LockedFile,
    mapping: LockedFile,
    scan: source::Scan,
    mapping_sha: [u8; 32],
    scratch: PathBuf,
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
}
impl std::fmt::Debug for PreparedOwnershipMigration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedOwnershipMigration")
            .field("source", &self.source)
            .field("scan", &self.scan)
            .finish_non_exhaustive()
    }
}
impl Drop for PreparedOwnershipMigration {
    fn drop(&mut self) {
        self.db.take();
    }
}
impl PreparedOwnershipMigration {
    fn db(&self) -> Result<&Database> {
        self.db.as_ref().ok_or(Error::SourceDrift)
    }
    /// Actual original player count; this observation grants no write authority.
    pub const fn player_count(&self) -> u64 {
        self.scan.players
    }
    /// Actual original UUID-entity count.
    pub const fn entity_count(&self) -> u64 {
        self.scan.entities
    }
    /// Complete retained original Format5 fingerprint.
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.scan.sha
    }
    /// Creates new bounded evidence, actual Stage CAS, close/reopen proofs and checkpoint.
    pub fn prepare_files(
        self,
        paths: OwnershipMigrationOutputPaths,
    ) -> Result<BoundedOwnershipMigration> {
        self.prepare_files_observed(paths, &mut |_| Ok(()))
    }
    #[allow(clippy::too_many_lines)] // Keep the audited file creation, CAS, close/reopen and seal lifetime in one scope.
    fn prepare_files_observed(
        self,
        paths: OwnershipMigrationOutputPaths,
        hook: Hook<'_>,
    ) -> Result<BoundedOwnershipMigration> {
        paths::distinct(&[
            &self.source.path,
            &self.mapping.path,
            &paths.backup,
            &paths.plan,
            &paths.stage,
            &paths.checkpoint,
        ])?;
        checkpoint::extent(
            &[
                &self.source.path,
                &self.mapping.path,
                &paths.backup,
                &paths.plan,
                &paths.stage,
            ],
            self.limits,
        )?;
        let anchor = paths.scratch.join(".ownership-journal-anchor");
        let index = index::Index::create(&paths.scratch, self.limits, self.scan)?;
        self.revalidate(&anchor)?;
        let budget = DiskBudget::new(
            UuidEntityMigrationScratchKind::VerificationCopy,
            self.limits.verification_copy_bytes,
        );
        let backup = open_new(&paths.backup, Role::Backup)?;
        let backupdb = backup.open_database(true, Some(&budget))?;
        stage::copy_original(self.db()?, &backupdb, false, self.config, hook)?;
        drop(backupdb);
        backup.sync()?;
        files::require_clean_header(&backup.file, &backup.path)?;
        let backup_evidence =
            checkpoint::Evidence::capture(&backup, true, self.limits.verification_copy_bytes)?;
        with_copy(
            &backup,
            backup_evidence.physical,
            &paths.scratch,
            self.limits,
            hook,
            |db, _| {
                let txn = db
                    .begin_write()
                    .map_err(|e| backend("backup validation", e))?;
                if source::scan(&txn, &anchor, self.config, self.limits)? != self.scan {
                    return Err(Error::EvidenceDrift(Role::Backup));
                }
                txn.abort()
                    .map_err(|e| backend("abort backup validation", e))?;
                Ok(())
            },
        )?;
        let plan = open_new(&paths.plan, Role::Plan)?;
        let stagefile = open_new(&paths.stage, Role::Stage)?;
        let stagedb = stagefile.open_database(true, Some(&budget))?;
        stage::copy_original(self.db()?, &stagedb, true, self.config, hook)?;
        let owner = Arc::new(());
        let header = plan::Header {
            source: self.scan.sha,
            mapping: self.mapping_sha,
            capacity: plan::capacity_hash(self.config.capacity()),
            players: self.scan.players,
            entities: self.scan.entities,
        };
        stage::make_plan(
            self.db()?,
            &stagedb,
            &self.mapping,
            &plan,
            &index,
            header,
            self.config,
            self.limits,
            &owner,
            hook,
        )?;
        self.revalidate(&anchor)?;
        stage::bootstrap(
            self.db()?,
            &stagedb,
            &plan,
            self.config,
            self.limits,
            &owner,
            hook,
        )?;
        let (targetraw, targetsha) = {
            let txn = stagedb
                .begin_write()
                .map_err(|e| backend("Stage target proof", e))?;
            let result = verify::target(
                self.db()?,
                &txn,
                &plan,
                &self.mapping,
                &index,
                self.scan,
                self.mapping_sha,
                self.config,
                self.limits,
                &anchor,
            )?;
            txn.abort()
                .map_err(|e| backend("abort Stage target proof", e))?;
            result
        };
        drop(stagedb);
        stagefile.sync()?;
        // A genuine close/reopen occurs before sealing; sealed original is never reopened.
        let reopened = stagefile.open_database(false, Some(&budget))?;
        {
            let txn = reopened
                .begin_write()
                .map_err(|e| backend("reopened Stage proof", e))?;
            let result = verify::target(
                self.db()?,
                &txn,
                &plan,
                &self.mapping,
                &index,
                self.scan,
                self.mapping_sha,
                self.config,
                self.limits,
                &anchor,
            )?;
            if result != (targetraw, targetsha) {
                return Err(Error::TargetDeltaMismatch);
            }
            txn.abort()
                .map_err(|e| backend("abort reopened Stage proof", e))?;
        }
        drop(reopened);
        stagefile.sync()?;
        files::require_clean_header(&stagefile.file, &stagefile.path)?;
        index.cleanup()?;
        let stage_evidence =
            checkpoint::Evidence::capture(&stagefile, true, self.limits.verification_copy_bytes)?;
        let plan_evidence = checkpoint::Evidence::capture(&plan, true, self.limits.scratch_bytes)?;
        let n = header.count()?;
        let record = checkpoint::Record {
            id: *Uuid::new_v4().as_bytes(),
            source: self.scan.sha,
            target: targetsha,
            mapping: self.mapping_sha,
            plan: plan_evidence.physical.1,
            counts: [
                self.scan.raw.rows,
                self.scan.raw.bytes,
                targetraw.rows,
                targetraw.bytes,
                self.scan.players,
                self.scan.entities,
                self.scan.chunks,
                n,
                n,
            ],
            evidence: [
                checkpoint::Evidence::capture(&self.source, false, 0)?,
                checkpoint::Evidence::capture(
                    &self.mapping,
                    true,
                    mapping::HEADER_BYTES + self.limits.mapping_rows * mapping::ROW_BYTES,
                )?,
                backup_evidence,
                plan_evidence,
                stage_evidence,
            ],
            limits: self.limits,
            capacity: checkpoint::capacity(self.config),
        };
        record.validate_shape(self.config)?;
        let checkpoint = open_new(&paths.checkpoint, Role::Checkpoint)?;
        checkpoint.write_exact(&record.encode()?)?;
        checkpoint.sync()?;
        if checkpoint::Record::read(&checkpoint, self.limits)? != record {
            return Err(Error::EvidenceDrift(Role::Checkpoint));
        }
        let checkpoint_evidence =
            checkpoint::Evidence::capture(&checkpoint, true, self.limits.checkpoint_bytes as u64)?;
        let bounded = BoundedOwnershipMigration {
            prepared: self,
            backup,
            plan,
            stage: stagefile,
            checkpoint,
            checkpoint_evidence,
            record,
            scratch: paths.scratch,
        };
        bounded.evidence()?;
        Ok(bounded)
    }
    fn revalidate(&self, anchor: &Path) -> Result<()> {
        paths::directory(&self.scratch)?;
        self.source.verify_single_link()?;
        self.mapping.verify_single_link()?;
        let mapping = mapping::Reader::open(&self.mapping, self.limits)?;
        if self.mapping.hash_expected_len(mapping.extent())?.1 != self.mapping_sha {
            return Err(Error::EvidenceDrift(Role::Mapping));
        }
        let txn = self
            .db()?
            .begin_write()
            .map_err(|e| backend("source revalidation", e))?;
        if source::scan(&txn, anchor, self.config, self.limits)? != self.scan {
            return Err(Error::SourceDrift);
        }
        source::validate_inputs(&txn, &self.mapping, self.scan, self.config, self.limits)?;
        txn.abort()
            .map_err(|e| backend("abort source revalidation", e))?;
        Ok(())
    }
}

/// Sealed move-only exact import authority; all evidence exists before Source mutation.
#[derive(Debug)]
pub struct BoundedOwnershipMigration {
    prepared: PreparedOwnershipMigration,
    backup: LockedFile,
    plan: LockedFile,
    stage: LockedFile,
    checkpoint: LockedFile,
    checkpoint_evidence: checkpoint::Evidence,
    record: checkpoint::Record,
    scratch: PathBuf,
}
/// Confirmed complete closed/reopened target; no live Store authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipMigrationOutcome {
    checkpoint: [u8; 16],
    source: [u8; 32],
    target: [u8; 32],
    players: u64,
    entities: u64,
}
impl OwnershipMigrationOutcome {
    /// Immutable checkpoint UUID identifying retained restart evidence.
    pub const fn checkpoint_id(self) -> [u8; 16] {
        self.checkpoint
    }
    /// Complete original logical Format5 hash.
    pub const fn source_sha256(self) -> [u8; 32] {
        self.source
    }
    /// Complete independently checked logical Format6 hash.
    pub const fn target_sha256(self) -> [u8; 32] {
        self.target
    }
    /// Actual imported player histories.
    pub const fn player_count(self) -> u64 {
        self.players
    }
    /// Actual imported entity histories.
    pub const fn entity_count(self) -> u64 {
        self.entities
    }
}
/// Exact resume classification; existing Format6 never bootstraps again.
#[derive(Debug)]
pub enum OwnershipMigrationResume {
    /// Exact original Format5, sealed authority is ready for its one atomic import.
    ReadyToCommit(Box<BoundedOwnershipMigration>),
    /// Exact fully validated target already committed; no write capability.
    AlreadyCommitted(OwnershipMigrationOutcome),
}
impl BoundedOwnershipMigration {
    fn evidence(&self) -> Result<()> {
        for (i, (file, role)) in [
            (&self.prepared.source, Role::Source),
            (&self.prepared.mapping, Role::Mapping),
            (&self.backup, Role::Backup),
            (&self.plan, Role::Plan),
            (&self.stage, Role::Stage),
        ]
        .into_iter()
        .enumerate()
        {
            self.record.evidence[i].verify(file, role, i != 0)?;
        }
        self.checkpoint_evidence
            .verify(&self.checkpoint, Role::Checkpoint, true)?;
        if checkpoint::Record::read(&self.checkpoint, self.prepared.limits)? != self.record {
            return Err(Error::EvidenceDrift(Role::Checkpoint));
        }
        Ok(())
    }
    fn outcome(&self) -> OwnershipMigrationOutcome {
        OwnershipMigrationOutcome {
            checkpoint: self.record.id,
            source: self.record.source,
            target: self.record.target,
            players: self.record.counts[4],
            entities: self.record.counts[5],
        }
    }
    fn uncertain(&self, error: &Error) -> Error {
        Error::CommitUncertain {
            checkpoint: self.record.id,
            target: self.record.target,
            cause: error.to_string(),
        }
    }
    /// Revalidates evidence, performs one genuine Source writer import and close/reopen.
    pub fn migrate(self) -> Result<OwnershipMigrationOutcome> {
        self.migrate_observed(&mut |_| Ok(()))
    }
    fn migrate_observed(mut self, hook: Hook<'_>) -> Result<OwnershipMigrationOutcome> {
        self.evidence()?;
        let anchor = self.scratch.join(".ownership-journal-anchor");
        self.prepared.revalidate(&anchor)?;
        // Backup and Stage are reopened only through distinct disposable physical copies.
        with_copy(
            &self.backup,
            self.record.evidence[2].physical,
            &self.scratch,
            self.prepared.limits,
            hook,
            |db, _| {
                self.record.source_scan(db, &anchor, self.prepared.config)?;
                Ok(())
            },
        )?;
        let mut attempted = false;
        let result = with_copy(
            &self.stage,
            self.record.evidence[4].physical,
            &self.scratch,
            self.prepared.limits,
            hook,
            |stage, hook| {
                let index =
                    index::Index::create(&self.scratch, self.prepared.limits, self.prepared.scan)?;
                {
                    let txn = stage
                        .begin_write()
                        .map_err(|e| backend("Stage copy delta proof", e))?;
                    let (raw, hash) = verify::target(
                        self.prepared.db()?,
                        &txn,
                        &self.plan,
                        &self.prepared.mapping,
                        &index,
                        self.prepared.scan,
                        self.prepared.mapping_sha,
                        self.prepared.config,
                        self.prepared.limits,
                        &anchor,
                    )?;
                    if hash != self.record.target
                        || raw.rows != self.record.counts[2]
                        || raw.bytes != self.record.counts[3]
                    {
                        return Err(Error::TargetDeltaMismatch);
                    }
                    txn.abort()
                        .map_err(|e| backend("abort Stage copy proof", e))?;
                }
                index.cleanup()?;
                self.evidence()?;
                atomic_import(&self, stage, &anchor, hook, &mut attempted)
            },
        );
        if let Err(error) = result {
            return Err(if attempted {
                self.uncertain(&error)
            } else {
                error
            });
        }
        let result = (|| {
            hook(Cutpoint::BeforeReopen)?;
            self.prepared.db.take();
            self.prepared.source.sync()?;
            let reopened = self.prepared.source.open_database(false, None)?;
            let txn = reopened
                .begin_write()
                .map_err(|e| backend("Source reopened target proof", e))?;
            let (raw, hash) =
                stage::target_scan(&txn, &anchor, self.prepared.config, self.prepared.limits)?;
            if hash != self.record.target
                || raw.rows != self.record.counts[2]
                || raw.bytes != self.record.counts[3]
            {
                return Err(Error::TargetDeltaMismatch);
            }
            txn.abort()
                .map_err(|e| backend("abort Source reopened proof", e))?;
            drop(reopened);
            hook(Cutpoint::AfterReopen)?;
            self.evidence()?;
            Ok(self.outcome())
        })();
        result.map_err(|error| self.uncertain(&error))
    }
}

impl RedbStore {
    /// Exclusively prepares only a complete valid Format5 source and explicit mapping.
    /// `scratch_directory` authorizes bounded owned journal indices during preparation.
    /// Synchronous Unix-only offline I/O; no implicit upgrade or live handle.
    pub fn prepare_player_entity_ownership_migration(
        source: impl AsRef<Path>,
        mapping: impl AsRef<Path>,
        scratch_directory: impl AsRef<Path>,
        config: OwnershipConfig,
        limits: OwnershipMigrationLimits,
    ) -> Result<PreparedOwnershipMigration> {
        limits.validate()?;
        let scratch = paths::directory(scratch_directory.as_ref())?;
        let source = LockedFile::open(&paths::path(source.as_ref())?, false, false)?;
        let mapping = LockedFile::open_readonly(&paths::path(mapping.as_ref())?)?;
        mapping.separate_from(&source)?;
        let mapping_reader = mapping::Reader::open(&mapping, limits)?;
        let mapping_sha = mapping.hash_expected_len(mapping_reader.extent())?.1;
        let db = source.open_database(false, None)?;
        let txn = db.begin_write().map_err(|e| backend("prepare Source", e))?;
        let anchor = scratch.join(".ownership-journal-anchor");
        let scan = source::scan(&txn, &anchor, config, limits)?;
        source::validate_inputs(&txn, &mapping, scan, config, limits)?;
        verify::forecast(scan, config, limits)?;
        txn.abort()
            .map_err(|e| backend("abort prepare Source", e))?;
        Ok(PreparedOwnershipMigration {
            db: Some(db),
            source,
            mapping,
            scan,
            mapping_sha,
            scratch,
            config,
            limits,
        })
    }
    /// Validates exactly the caller's sealed paths, without CAS or ID regeneration.
    #[allow(clippy::too_many_lines)] // Exact caller bindings precede all opens and both mutually exclusive resume proofs.
    pub fn resume_player_entity_ownership_migration(
        paths: OwnershipMigrationResumePaths,
        config: OwnershipConfig,
        limits: OwnershipMigrationLimits,
    ) -> Result<OwnershipMigrationResume> {
        limits.validate()?;
        let checkpoint = LockedFile::open_readonly(&paths.output.checkpoint)?;
        let record = checkpoint::Record::read(&checkpoint, limits)?;
        record.validate_shape(config)?;
        if record.capacity != checkpoint::capacity(config) {
            return Err(Error::MalformedCheckpoint("capacity agreement"));
        }
        let selected = [
            &paths.source,
            &paths.mapping,
            &paths.output.backup,
            &paths.output.plan,
            &paths.output.stage,
        ];
        for (bound, selected) in record.evidence.iter().zip(selected) {
            if &bound.path != selected {
                return Err(Error::MalformedCheckpoint("caller path binding"));
            }
        }
        // Check exact canonical caller bindings before opening any recorded file.
        let source = LockedFile::open(&paths.source, false, false)?;
        let mapping = LockedFile::open_readonly(&paths.mapping)?;
        let backup = LockedFile::open_readonly(&paths.output.backup)?;
        let plan = LockedFile::open_readonly(&paths.output.plan)?;
        let stage = LockedFile::open_readonly(&paths.output.stage)?;
        let files = [&source, &mapping, &backup, &plan, &stage, &checkpoint];
        for (i, file) in files.iter().enumerate() {
            for other in &files[..i] {
                file.separate_from(other)?;
            }
        }
        for (i, (file, role)) in [
            (&source, Role::Source),
            (&mapping, Role::Mapping),
            (&backup, Role::Backup),
            (&plan, Role::Plan),
            (&stage, Role::Stage),
        ]
        .into_iter()
        .enumerate()
        {
            record.evidence[i].verify(file, role, i != 0)?;
        }
        mapping::Reader::open(&mapping, limits)?;
        plan::Reader::open(&plan, limits)?;
        if mapping.hash_expected_len(record.evidence[1].physical.0)?.1 != record.mapping
            || plan.hash_expected_len(record.evidence[3].physical.0)?.1 != record.plan
        {
            return Err(Error::MalformedCheckpoint("mapping/plan digest binding"));
        }
        let anchor = paths.output.scratch.join(".ownership-journal-anchor");
        let db = source.open_database(false, None)?;
        let original = with_copy(
            &backup,
            record.evidence[2].physical,
            &paths.output.scratch,
            limits,
            &mut |_| Ok(()),
            |backupdb, _| record.source_scan(backupdb, &anchor, config),
        )?;
        let marker = {
            let txn = db
                .begin_read()
                .map_err(|e| backend("resume format read", e))?;
            let meta = txn
                .open_table(META_TABLE)
                .map_err(|_| Error::CatalogMismatch)?;
            let value = meta
                .get(META_FORMAT_KEY)
                .map_err(|e| backend("resume format", e))?
                .map(|v| v.value());
            value
        };
        let prepared = PreparedOwnershipMigration {
            db: Some(db),
            source,
            mapping,
            scan: original,
            mapping_sha: record.mapping,
            scratch: paths.output.scratch.clone(),
            config,
            limits,
        };
        let checkpoint_evidence =
            checkpoint::Evidence::capture(&checkpoint, true, limits.checkpoint_bytes as u64)?;
        let bounded = BoundedOwnershipMigration {
            prepared,
            backup,
            plan,
            stage,
            checkpoint,
            checkpoint_evidence,
            record,
            scratch: paths.output.scratch,
        };
        match marker {
            Some(5) => {
                bounded.prepared.revalidate(&anchor)?;
                with_copy(
                    &bounded.stage,
                    bounded.record.evidence[4].physical,
                    &bounded.scratch,
                    limits,
                    &mut |_| Ok(()),
                    |db, _| {
                        let index = index::Index::create(&bounded.scratch, limits, original)?;
                        let txn = db
                            .begin_write()
                            .map_err(|e| backend("resume Stage proof", e))?;
                        let (_, hash) = verify::target(
                            bounded.prepared.db()?,
                            &txn,
                            &bounded.plan,
                            &bounded.prepared.mapping,
                            &index,
                            original,
                            bounded.prepared.mapping_sha,
                            config,
                            limits,
                            &anchor,
                        )?;
                        if hash != bounded.record.target {
                            return Err(Error::TargetDeltaMismatch);
                        }
                        txn.abort()
                            .map_err(|e| backend("abort resume Stage proof", e))?;
                        index.cleanup()?;
                        Ok(())
                    },
                )?;
                bounded.evidence()?;
                Ok(OwnershipMigrationResume::ReadyToCommit(Box::new(bounded)))
            }
            Some(6) => {
                with_copy(
                    &bounded.backup,
                    bounded.record.evidence[2].physical,
                    &bounded.scratch,
                    limits,
                    &mut |_| Ok(()),
                    |db, _| {
                        let index = index::Index::create(&bounded.scratch, limits, original)?;
                        let txn = bounded
                            .prepared
                            .db()?
                            .begin_write()
                            .map_err(|e| backend("resume committed Source proof", e))?;
                        let (raw, hash) = verify::target(
                            db,
                            &txn,
                            &bounded.plan,
                            &bounded.prepared.mapping,
                            &index,
                            original,
                            bounded.prepared.mapping_sha,
                            config,
                            limits,
                            &anchor,
                        )?;
                        if hash != bounded.record.target
                            || raw.rows != bounded.record.counts[2]
                            || raw.bytes != bounded.record.counts[3]
                        {
                            return Err(Error::TargetDeltaMismatch);
                        }
                        txn.abort()
                            .map_err(|e| backend("abort resumed Source proof", e))?;
                        index.cleanup()?;
                        Ok(())
                    },
                )?;
                with_copy(
                    &bounded.stage,
                    bounded.record.evidence[4].physical,
                    &bounded.scratch,
                    limits,
                    &mut |_| Ok(()),
                    |db, _| {
                        let txn = db
                            .begin_write()
                            .map_err(|e| backend("resume sealed Stage proof", e))?;
                        let (_, hash) = stage::target_scan(&txn, &anchor, config, limits)?;
                        if hash != bounded.record.target {
                            return Err(Error::TargetDeltaMismatch);
                        }
                        txn.abort()
                            .map_err(|e| backend("abort sealed Stage proof", e))?;
                        Ok(())
                    },
                )?;
                bounded.evidence()?;
                Ok(OwnershipMigrationResume::AlreadyCommitted(
                    bounded.outcome(),
                ))
            }
            _ => Err(Error::UnsupportedSourceFormat(marker)),
        }
    }
}

fn open_new(path: &Path, role: Role) -> Result<LockedFile> {
    LockedFile::open(path, true, false).map_err(|error| match error {
        super::uuid_migration::UuidEntityMigrationError::EvidenceExists(_) => {
            Error::EvidenceExists(role)
        }
        other => other.into(),
    })
}

fn with_copy<T>(
    original: &LockedFile,
    physical: (u64, [u8; 32]),
    scratch: &Path,
    limits: OwnershipMigrationLimits,
    hook: Hook<'_>,
    f: impl FnOnce(&Database, Hook<'_>) -> Result<T>,
) -> Result<T> {
    original.verify_single_link()?;
    let budget = DiskBudget::new(
        UuidEntityMigrationScratchKind::VerificationCopy,
        limits.verification_copy_bytes,
    );
    budget.admit(physical.0)?;
    if original.hash_expected_len(physical.0)? != physical {
        return Err(Error::TargetDeltaMismatch);
    }
    files::require_clean_header(&original.file, &original.path)?;
    hook(Cutpoint::BeforeCopyCreate)?;
    let temporary = TemporaryFile::create(scratch, "ferrumc-ownership-verify")?;
    let result = (|| {
        hook(Cutpoint::AfterCopyCreate)?;
        let file = temporary.file()?;
        hook(Cutpoint::BeforeCopyWrite)?;
        files::copy_closed(original, file, &budget, physical)?;
        hook(Cutpoint::AfterCopyWrite)?;
        hook(Cutpoint::BeforeCopyOpen)?;
        let db = file.open_database(false, Some(&budget))?;
        hook(Cutpoint::AfterCopyOpen)?;
        hook(Cutpoint::BeforeCopyScan)?;
        let result = f(&db, hook);
        hook(Cutpoint::AfterCopyScan)?;
        hook(Cutpoint::BeforeCopyClose)?;
        drop(db);
        hook(Cutpoint::AfterCopyClose)?;
        if let Some(error) = budget.error() {
            return Err(error.into());
        }
        result
    })();
    let cleanup_hook = hook(Cutpoint::BeforeCopyCleanup);
    temporary.cleanup()?;
    cleanup_hook?;
    hook(Cutpoint::AfterCopyCleanup)?;
    original.verify_single_link()?;
    if original.hash_expected_len(physical.0)? != physical {
        return Err(Error::TargetDeltaMismatch);
    }
    result
}

fn atomic_import(
    bounded: &BoundedOwnershipMigration,
    stage: &Database,
    anchor: &Path,
    hook: Hook<'_>,
    attempted: &mut bool,
) -> Result<()> {
    let prepared = &bounded.prepared;
    let stage = stage
        .begin_read()
        .map_err(|e| backend("import Stage read", e))?;
    let mut txn = prepared
        .db()?
        .begin_write()
        .map_err(|e| backend("actual Source writer", e))?;
    txn.set_durability(Durability::Immediate);
    if source::scan(&txn, anchor, prepared.config, prepared.limits)? != prepared.scan {
        return Err(Error::SourceDrift);
    }
    bounded.evidence()?;
    for definition in [
        ownership::PLAYER_OWNER,
        ownership::ENTITY_OWNER,
        ownership::RECEIPT,
        uuid_entities::EPOCH_TABLE,
    ] {
        let from = stage
            .open_table(definition)
            .map_err(|e| backend("import Stage table", e))?;
        let mut to = txn
            .open_table(definition)
            .map_err(|e| backend("import Source table", e))?;
        for row in from
            .iter()
            .map_err(|e| backend("import Stage iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("import Stage row", e))?;
            hook(Cutpoint::BeforeRow)?;
            to.insert(key.value(), value.value())
                .map_err(|e| backend("import Source row", e))?;
            hook(Cutpoint::AfterRow)?;
        }
    }
    {
        let mut meta = txn
            .open_table(META_TABLE)
            .map_err(|e| backend("import Source metadata", e))?;
        for (key, value) in metadata(prepared.config, bounded.record.counts[7]) {
            hook(Cutpoint::BeforeMeta)?;
            meta.insert(key, value)
                .map_err(|e| backend("import capacity/count", e))?;
            hook(Cutpoint::AfterMeta)?;
        }
        hook(Cutpoint::BeforeMarker)?;
        meta.insert(META_FORMAT_KEY, 6)
            .map_err(|e| backend("import format marker", e))?;
        hook(Cutpoint::AfterMarker)?;
    }
    hook(Cutpoint::BeforeTargetScan)?;
    let (raw, hash) = stage::target_scan(&txn, anchor, prepared.config, prepared.limits)?;
    if hash != bounded.record.target
        || raw.rows != bounded.record.counts[2]
        || raw.bytes != bounded.record.counts[3]
    {
        return Err(Error::TargetDeltaMismatch);
    }
    hook(Cutpoint::AfterTargetScan)?;
    bounded.evidence()?;
    hook(Cutpoint::BeforeCommit)?;
    *attempted = true;
    txn.commit()
        .map_err(|e| backend("actual Source commit", e))?;
    hook(Cutpoint::AfterCommit)?;
    Ok(())
}
