//! A single owned bounded redb file contains ID uniqueness and serial epoch proof state.

use super::super::{
    uuid_entities,
    uuid_migration::{
        files::{DiskBudget, TemporaryFile},
        UuidEntityMigrationScratchKind,
    },
};
use super::{backend, source, Error, OwnershipMigrationLimits, Result};
use redb::{Database, ReadableTable, TableDefinition, WriteTransaction};
use std::path::Path;
use std::sync::Arc;

const IDS: TableDefinition<'_, &[u8], u64> = TableDefinition::new("import_ids");
const COUNTS: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new("chunk_counts");
pub(super) struct Index {
    db: Option<Database>,
    file: Option<TemporaryFile>,
    pub(super) budget: Arc<DiskBudget>,
}
impl Index {
    pub(super) fn create(
        directory: &Path,
        limits: OwnershipMigrationLimits,
        scan: source::Scan,
    ) -> Result<Self> {
        let resources = scan
            .players
            .checked_add(scan.entities)
            .ok_or(Error::ScratchCapacity)?;
        let logical = super::plan::HEADER_BYTES
            .checked_add(
                resources
                    .checked_mul(super::plan::ROW_BYTES + 24)
                    .ok_or(Error::ScratchCapacity)?,
            )
            .and_then(|n| scan.chunks.checked_mul(32).and_then(|c| n.checked_add(c)))
            .and_then(|n| n.checked_add(128))
            .ok_or(Error::ScratchCapacity)?;
        if logical > limits.scratch_bytes {
            return Err(Error::ScratchCapacity);
        }
        let budget = DiskBudget::new(
            UuidEntityMigrationScratchKind::ReceiptIndex,
            limits.scratch_bytes,
        );
        let file = TemporaryFile::create(directory, "ferrumc-ownership-index")?;
        let db = file
            .file()?
            .open_database_with_cache(true, Some(&budget), 16 * 1024 * 1024)?;
        let txn = db
            .begin_write()
            .map_err(|e| budget.map("create migration index", e))?;
        txn.open_table(IDS)
            .map_err(|e| budget.map("create ID table", e))?;
        txn.open_table(COUNTS)
            .map_err(|e| budget.map("create count table", e))?;
        txn.commit()
            .map_err(|e| budget.map("commit migration index", e))?;
        Ok(Self {
            db: Some(db),
            file: Some(file),
            budget,
        })
    }
    fn db(&self) -> Result<&Database> {
        self.db.as_ref().ok_or(Error::ScratchCapacity)
    }
    pub(super) fn insert_id(&self, id: [u8; 16], ordinal: u64) -> Result<()> {
        let txn = self
            .db()?
            .begin_write()
            .map_err(|e| self.budget.map("ID transaction", e))?;
        {
            let mut ids = txn
                .open_table(IDS)
                .map_err(|e| self.budget.map("ID table", e))?;
            if ids
                .insert(id.as_slice(), ordinal)
                .map_err(|e| self.budget.map("ID insert", e))?
                .is_some()
            {
                return Err(Error::DuplicateImportId);
            }
        }
        txn.commit().map_err(|e| self.budget.map("ID commit", e))?;
        Ok(())
    }
    pub(super) fn clear_ids(&self) -> Result<()> {
        let txn = self
            .db()?
            .begin_write()
            .map_err(|e| self.budget.map("clear ID transaction", e))?;
        txn.delete_table(IDS)
            .map_err(|e| self.budget.map("clear ID table", e))?;
        txn.open_table(IDS)
            .map_err(|e| self.budget.map("recreate ID table", e))?;
        txn.commit()
            .map_err(|e| self.budget.map("clear ID commit", e))?;
        Ok(())
    }
    pub(super) fn reset_counts(&self, source: &WriteTransaction) -> Result<()> {
        let txn = self
            .db()?
            .begin_write()
            .map_err(|e| self.budget.map("count transaction", e))?;
        {
            let mut counts = txn
                .open_table(COUNTS)
                .map_err(|e| self.budget.map("count table", e))?;
            let epochs = source
                .open_table(uuid_entities::EPOCH_TABLE)
                .map_err(|e| backend("source epochs", e))?;
            let index = source
                .open_table(uuid_entities::INDEX_TABLE)
                .map_err(|e| backend("source chunkindex", e))?;
            for row in epochs
                .iter()
                .map_err(|e| backend("source epochs iterate", e))?
            {
                let (key, value) = row.map_err(|e| backend("source epoch row", e))?;
                let mut record = [0; 16];
                record[..8].copy_from_slice(value.value());
                record[8..]
                    .copy_from_slice(&source::chunk_count(&index, key.value())?.to_be_bytes());
                counts
                    .insert(key.value(), record.as_slice())
                    .map_err(|e| self.budget.map("count insert", e))?;
            }
        }
        txn.commit()
            .map_err(|e| self.budget.map("count commit", e))?;
        Ok(())
    }
    pub(super) fn advance(&self, key: &[u8], before: u64, after: u64) -> Result<()> {
        let txn = self
            .db()?
            .begin_write()
            .map_err(|e| self.budget.map("serial count transaction", e))?;
        {
            let mut counts = txn
                .open_table(COUNTS)
                .map_err(|e| self.budget.map("serial count table", e))?;
            let (epoch, remaining) = {
                let record = counts
                    .get(key)
                    .map_err(|e| self.budget.map("serial count lookup", e))?
                    .ok_or(Error::TargetDeltaMismatch)?;
                decode(record.value())?
            };
            if epoch != before || before.checked_add(1) != Some(after) || remaining == 0 {
                return Err(Error::TargetDeltaMismatch);
            }
            let mut bytes = [0; 16];
            bytes[..8].copy_from_slice(&after.to_be_bytes());
            bytes[8..].copy_from_slice(&(remaining - 1).to_be_bytes());
            counts
                .insert(key, bytes.as_slice())
                .map_err(|e| self.budget.map("serial count update", e))?;
        }
        txn.commit()
            .map_err(|e| self.budget.map("serial count commit", e))?;
        Ok(())
    }
    pub(super) fn verify_final(&self, target: &WriteTransaction) -> Result<()> {
        let txn = self
            .db()?
            .begin_read()
            .map_err(|e| backend("final count read", e))?;
        let counts = txn
            .open_table(COUNTS)
            .map_err(|e| backend("final count table", e))?;
        let epochs = target
            .open_table(uuid_entities::EPOCH_TABLE)
            .map_err(|e| backend("target epochs", e))?;
        for row in counts
            .iter()
            .map_err(|e| backend("final count iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("final count row", e))?;
            let (expected, remaining) = decode(value.value())?;
            let actual = epochs
                .get(key.value())
                .map_err(|e| backend("target epoch lookup", e))?
                .ok_or(Error::TargetDeltaMismatch)?;
            if remaining != 0 || actual.value() != expected.to_be_bytes() {
                return Err(Error::TargetDeltaMismatch);
            }
        }
        Ok(())
    }
    pub(super) fn cleanup(mut self) -> Result<()> {
        self.db.take();
        if let Some(file) = self.file.take() {
            file.cleanup()?;
        }
        Ok(())
    }
}
fn decode(bytes: &[u8]) -> Result<(u64, u64)> {
    if bytes.len() != 16 {
        return Err(Error::TargetDeltaMismatch);
    }
    Ok((
        u64::from_be_bytes(
            bytes[..8]
                .try_into()
                .map_err(|_| Error::TargetDeltaMismatch)?,
        ),
        u64::from_be_bytes(
            bytes[8..]
                .try_into()
                .map_err(|_| Error::TargetDeltaMismatch)?,
        ),
    ))
}
impl Drop for Index {
    fn drop(&mut self) {
        self.db.take();
        self.file.take();
    }
}
