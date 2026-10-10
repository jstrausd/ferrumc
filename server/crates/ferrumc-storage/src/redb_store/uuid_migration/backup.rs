//! Raw nine-table backup and copy-only verification of immutable evidence.

use redb::{Database, Durability, ReadableTable, WriteTransaction};
use std::path::Path;

use super::files::{self, DiskBudget, LockedFile, TemporaryFile};
use super::{
    backend, validation, BackedUpUuidEntityMigration, MigrationResult, Observer,
    PreparedUuidEntityMigration, Stage, UuidEntityMigrationBackup,
    UuidEntityMigrationError as Error, UuidEntityMigrationLimits, UuidEntityMigrationScratchKind,
};
use crate::redb_store::META_TABLE;

impl PreparedUuidEntityMigration {
    /// Creates and fully verifies a separate raw Format4 backup without overwrite.
    ///
    /// Synchronous offline I/O. Failed copy/sync/verification may preserve an own
    /// incomplete backup for explicit inspection, but never enables source commit.
    pub fn create_backup(
        self,
        path: impl AsRef<Path>,
    ) -> MigrationResult<BackedUpUuidEntityMigration> {
        self.create_backup_observed(path.as_ref(), &mut |_| Ok(()))
    }

    pub(super) fn create_backup_observed(
        self,
        path: &Path,
        observe: Observer<'_>,
    ) -> MigrationResult<BackedUpUuidEntityMigration> {
        self.source.verify_single_link()?;
        let backup_file = LockedFile::open(path, true, false)?;
        backup_file.separate_from(&self.source)?;
        observe(Stage::BackupCreated)?;
        let db = backup_file.open_database(true, None)?;
        copy_format4(&self.db, &db, observe)?;
        drop(db);
        observe(Stage::BackupClosed)?;
        backup_file.sync()?;
        observe(Stage::BackupSynced)?;
        files::require_clean_header(&backup_file.file, &backup_file.path)?;
        let physical = backup_file.hash()?;
        observe(Stage::BackupHashed)?;
        self.finish_backup(backup_file, physical, observe)
    }

    /// Reuses only a previously recorded closed-file SHA256 and matching full4 data.
    ///
    /// The locked original is never redb-opened: its physical bytes/hash therefore
    /// remain unchanged through verification success, error and a lost reply.
    pub fn verify_existing_backup(
        self,
        path: impl AsRef<Path>,
        remembered_closed_file_sha256: [u8; 32],
    ) -> MigrationResult<BackedUpUuidEntityMigration> {
        self.source.verify_single_link()?;
        let backup_file = LockedFile::open(path.as_ref(), false, false)?;
        backup_file.separate_from(&self.source)?;
        files::require_clean_header(&backup_file.file, &backup_file.path)?;
        let physical = backup_file.hash()?;
        if physical.1 != remembered_closed_file_sha256 {
            return Err(Error::BackupMismatch);
        }
        self.finish_backup(backup_file, physical, &mut |_| Ok(()))
    }

    fn finish_backup(
        self,
        backup_file: LockedFile,
        physical: (u64, [u8; 32]),
        observe: Observer<'_>,
    ) -> MigrationResult<BackedUpUuidEntityMigration> {
        let scan = verify_closed_copy(&backup_file, physical, self.limits, None, observe)?;
        if scan != self.report.scan {
            return Err(Error::BackupMismatch);
        }
        self.source.verify_single_link()?;
        backup_file.verify_single_link()?;
        if backup_file.hash()? != physical {
            return Err(Error::BackupMismatch);
        }
        let backup = UuidEntityMigrationBackup {
            path: backup_file.path.clone(),
            identity: backup_file.identity,
            file_len: physical.0,
            file_sha256: physical.1,
            snapshot_sha256: scan.sha256,
        };
        Ok(BackedUpUuidEntityMigration {
            prepared: self,
            backup_file,
            backup,
        })
    }
}

fn copy_format4(
    source: &Database,
    destination: &Database,
    observe: Observer<'_>,
) -> MigrationResult<()> {
    let source = source
        .begin_read()
        .map_err(|e| backend("backup source read", e))?;
    let mut dest = destination
        .begin_write()
        .map_err(|e| backend("backup destination write", e))?;
    dest.set_durability(Durability::Immediate);
    {
        let source = source
            .open_table(META_TABLE)
            .map_err(|e| backend("backup source metadata", e))?;
        let mut target = dest
            .open_table(META_TABLE)
            .map_err(|e| backend("backup destination metadata", e))?;
        for row in source
            .iter()
            .map_err(|e| backend("backup metadata iterate", e))?
        {
            let (k, v) = row.map_err(|e| backend("backup metadata row", e))?;
            target
                .insert(k.value(), v.value())
                .map_err(|e| backend("backup metadata insert", e))?;
            observe(Stage::BackupRow)?;
        }
    }
    for (index, definition) in validation::TABLES.iter().enumerate() {
        if (4..=6).contains(&index) {
            continue;
        }
        let source = source
            .open_table(*definition)
            .map_err(|e| backend("backup source table", e))?;
        let mut target = dest
            .open_table(*definition)
            .map_err(|e| backend("backup destination table", e))?;
        for row in source
            .iter()
            .map_err(|e| backend("backup table iterate", e))?
        {
            let (k, v) = row.map_err(|e| backend("backup table row", e))?;
            target
                .insert(k.value(), v.value())
                .map_err(|e| backend("backup table insert", e))?;
            observe(Stage::BackupRow)?;
        }
    }
    dest.commit().map_err(|e| backend("backup commit", e))?;
    observe(Stage::BackupCommitted)
}

pub(super) fn verify_closed_copy(
    backup: &LockedFile,
    physical: (u64, [u8; 32]),
    limits: UuidEntityMigrationLimits,
    source5: Option<&WriteTransaction>,
    observe: Observer<'_>,
) -> MigrationResult<validation::Scan> {
    backup.verify_single_link()?;
    files::require_clean_header(&backup.file, &backup.path)?;
    let budget = DiskBudget::new(
        UuidEntityMigrationScratchKind::VerificationCopy,
        limits.max_verification_copy_bytes(),
    );
    budget.admit(physical.0)?;
    if backup.hash()? != physical {
        return Err(Error::BackupMismatch);
    }
    let parent = backup
        .path
        .parent()
        .ok_or_else(|| Error::InvalidFile(backup.path.clone()))?;
    let temporary = TemporaryFile::create(parent, "ferrumc-uuid-migration-verify")?;
    let result = (|| {
        observe(Stage::VerificationCreated)?;
        let file = temporary.file()?;
        files::copy_closed(backup, file, &budget, physical)?;
        observe(Stage::VerificationCopied)?;
        let db = file.open_database(false, Some(&budget))?;
        let txn = db
            .begin_write()
            .map_err(|e| budget.map("verification transaction", e))?;
        let scan = validation::scan_format4(&txn, &file.path, limits)?;
        if let Some(source) = source5 {
            validation::compare_exact_delta(source, &txn)?;
        }
        txn.abort()
            .map_err(|e| budget.map("abort verification", e))?;
        drop(db);
        if let Some(error) = budget.error() {
            return Err(error);
        }
        observe(Stage::VerificationValidated)?;
        Ok(scan)
    })();
    temporary.cleanup()?;
    observe(Stage::VerificationCleaned)?;
    backup.verify_single_link()?;
    if backup.hash()? != physical {
        return Err(Error::BackupMismatch);
    }
    result
}
