//! Complete4 admission, separate twelve-slot grammar and bounded receipt checks.

use std::path::Path;
use std::sync::Arc;

use redb::{Database, ReadableTable, TableDefinition, TableHandle, WriteTransaction};
use sha2::{Digest, Sha256};

use super::files::{DiskBudget, TemporaryFile};
use super::{
    backend, MigrationResult, UuidEntityMigrationError as Error, UuidEntityMigrationLimits,
    UuidEntityMigrationScratchKind,
};
use crate::redb_store::uuid_entities::{EPOCH_TABLE, INDEX_TABLE, LOCATOR_TABLE};
use crate::redb_store::{
    decode_journal_receipt, mutation_id_from_key, CHUNK_OVERLAY_TABLE, CHUNK_TABLE, ENTITY_TABLE,
    GENERATION_TABLE, META_FORMAT_KEY, META_LAST_MUTATION_ID_KEY, META_TABLE, MUTATION_BATCH_TABLE,
    MUTATION_LOG_TABLE, PLAYER_TABLE, PLUGIN_TABLE,
};
use crate::{
    GenerationKey, JournalBatchId, StorageError, MAX_GENERATION_IDENTITIES, MAX_PLUGIN_KEY_LEN,
    MAX_PLUGIN_VALUE_LEN, MAX_UUID_STORE_PLUGIN_ID_LEN,
};

pub(super) const TABLES: [TableDefinition<'_, &[u8], &[u8]>; 11] = [
    GENERATION_TABLE,
    CHUNK_TABLE,
    CHUNK_OVERLAY_TABLE,
    ENTITY_TABLE,
    LOCATOR_TABLE,
    INDEX_TABLE,
    EPOCH_TABLE,
    PLAYER_TABLE,
    PLUGIN_TABLE,
    MUTATION_LOG_TABLE,
    MUTATION_BATCH_TABLE,
];
const RECEIPT_INDEX: TableDefinition<'_, u64, &[u8]> = TableDefinition::new("receipt_ranges");

pub(super) fn record(table: &'static str, source: StorageError) -> Error {
    Error::Record {
        table,
        source: Box::new(source),
    }
}
fn bad(table: &'static str, reason: &'static str) -> Error {
    Error::InvalidRow { table, reason }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Scan {
    pub(super) format: u64,
    pub(super) counts: [u64; 12],
    pub(super) logical_bytes: u64,
    pub(super) sha256: [u8; 32],
    pub(super) target_sha256: [u8; 32],
}

struct Fingerprint {
    hash: Sha256,
    counts: [u64; 12],
    bytes: u64,
}
impl Fingerprint {
    fn new() -> Self {
        let mut hash = Sha256::new();
        hash.update(b"FerrumC UUID entity migration logical snapshot v1\0");
        Self {
            hash,
            counts: [0; 12],
            bytes: 0,
        }
    }
    fn table(&mut self, name: &str, present: bool, metadata: bool) {
        self.hash.update(b"table\0");
        self.hash.update((name.len() as u64).to_be_bytes());
        self.hash.update(name.as_bytes());
        self.hash.update(if metadata { [1, 2] } else { [3, 3] });
        self.hash.update([u8::from(present)]);
    }
    fn row(&mut self, slot: usize, key: &[u8], value: &[u8]) -> MigrationResult<()> {
        self.counts[slot] = self.counts[slot]
            .checked_add(1)
            .ok_or(Error::CounterOverflow)?;
        self.bytes = self
            .bytes
            .checked_add(key.len() as u64)
            .and_then(|n| n.checked_add(value.len() as u64))
            .ok_or(Error::CounterOverflow)?;
        self.hash.update(b"row\0");
        self.hash.update((key.len() as u64).to_be_bytes());
        self.hash.update(key);
        self.hash.update((value.len() as u64).to_be_bytes());
        self.hash.update(value);
        Ok(())
    }
    fn end(&mut self, slot: usize) {
        self.hash.update(b"end\0");
        self.hash.update(self.counts[slot].to_be_bytes());
    }
}

pub(super) fn marker(txn: &WriteTransaction) -> MigrationResult<Option<u64>> {
    let meta_present = txn
        .list_tables()
        .map_err(|e| backend("catalog", e))?
        .any(|t| t.name() == META_TABLE.name());
    if !meta_present {
        return Err(Error::CatalogMismatch);
    }
    let meta = txn
        .open_table(META_TABLE)
        .map_err(|_| Error::CatalogMismatch)?;
    meta.get(META_FORMAT_KEY)
        .map(|v| v.map(|v| v.value()))
        .map_err(|e| backend("format marker", e))
}

fn catalog4(txn: &WriteTransaction) -> MigrationResult<()> {
    match marker(txn)? {
        Some(4) => {}
        Some(5) => return Err(Error::CompletedVerificationRequiresCheckpoint),
        other => return Err(Error::WrongFormat(other)),
    }
    let mut count = 0;
    for table in txn.list_tables().map_err(|e| backend("catalog", e))? {
        if table.name() != META_TABLE.name()
            && !TABLES
                .iter()
                .enumerate()
                .any(|(i, t)| !(4..=6).contains(&i) && t.name() == table.name())
        {
            return Err(Error::CatalogMismatch);
        }
        count += 1;
        if count > 9 {
            return Err(Error::CatalogMismatch);
        }
    }
    if count != 9
        || txn
            .list_multimap_tables()
            .map_err(|e| backend("multimap catalog", e))?
            .next()
            .is_some()
    {
        return Err(Error::CatalogMismatch);
    }
    for (index, table) in TABLES.iter().enumerate() {
        if !(4..=6).contains(&index) {
            txn.open_table(*table).map_err(|_| Error::CatalogMismatch)?;
        }
    }
    Ok(())
}

pub(super) fn fingerprint(txn: &WriteTransaction, format: u64) -> MigrationResult<Scan> {
    let mut actual = Fingerprint::new();
    let mut target = Fingerprint::new();
    actual.table(META_TABLE.name(), true, true);
    target.table(META_TABLE.name(), true, true);
    {
        let meta = txn
            .open_table(META_TABLE)
            .map_err(|e| backend("fingerprint metadata", e))?;
        for row in meta
            .iter()
            .map_err(|e| backend("fingerprint metadata iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("fingerprint metadata row", e))?;
            actual.row(0, key.value().as_bytes(), &value.value().to_be_bytes())?;
            let projected = if key.value() == META_FORMAT_KEY {
                5
            } else {
                value.value()
            };
            target.row(0, key.value().as_bytes(), &projected.to_be_bytes())?;
        }
    }
    actual.end(0);
    target.end(0);
    for (index, definition) in TABLES.iter().enumerate() {
        let slot = index + 1;
        let present = format == 5 || !(5..=7).contains(&slot);
        actual.table(definition.name(), present, false);
        target.table(definition.name(), true, false);
        if present {
            let table = txn
                .open_table(*definition)
                .map_err(|e| backend("fingerprint table", e))?;
            for row in table
                .iter()
                .map_err(|e| backend("fingerprint iterate", e))?
            {
                let (k, v) = row.map_err(|e| backend("fingerprint row", e))?;
                actual.row(slot, k.value(), v.value())?;
                target.row(slot, k.value(), v.value())?;
            }
        }
        actual.end(slot);
        target.end(slot);
    }
    Ok(Scan {
        format,
        counts: actual.counts,
        logical_bytes: actual.bytes,
        sha256: actual.hash.finalize().into(),
        target_sha256: target.hash.finalize().into(),
    })
}

pub(super) fn scan_format4(
    txn: &WriteTransaction,
    path: &Path,
    limits: UuidEntityMigrationLimits,
) -> MigrationResult<Scan> {
    catalog4(txn)?;
    let generations = txn
        .open_table(GENERATION_TABLE)
        .map_err(|e| backend("generation table", e))?;
    let mut descriptor_count = 0usize;
    for row in generations
        .iter()
        .map_err(|e| backend("generation iterate", e))?
    {
        let (key, value) = row.map_err(|e| backend("generation row", e))?;
        let key =
            GenerationKey::decode(key.value()).map_err(|e| record(GENERATION_TABLE.name(), e))?;
        crate::generation::decode_identity(key, value.value())
            .map_err(|e| record(GENERATION_TABLE.name(), e))?;
        descriptor_count += 1;
        if descriptor_count > MAX_GENERATION_IDENTITIES {
            return Err(Error::TooManyDescriptors);
        }
    }
    let mut journal = JournalValidation::new(limits.max_receipt_index_bytes());
    let result = (|| {
        let watermark = {
            let meta = txn
                .open_table(META_TABLE)
                .map_err(|e| backend("metadata", e))?;
            let mut watermark = None;
            for row in meta.iter().map_err(|e| backend("metadata iterate", e))? {
                let (key, value) = row.map_err(|e| backend("metadata row", e))?;
                match key.value() {
                    META_FORMAT_KEY if value.value() == 4 => {}
                    META_LAST_MUTATION_ID_KEY => watermark = Some(value.value()),
                    _ => {
                        return Err(bad(
                            META_TABLE.name(),
                            "unknown metadata or incorrect marker",
                        ));
                    }
                }
            }
            watermark
        };
        for (index, definition) in TABLES.iter().enumerate() {
            if index == 0 || (4..=6).contains(&index) {
                continue;
            }
            let table = txn
                .open_table(*definition)
                .map_err(|e| backend("payload table", e))?;
            for row in table.iter().map_err(|e| backend("payload iterate", e))? {
                let (key, value) = row.map_err(|e| backend("payload row", e))?;
                let (key, value) = (key.value(), value.value());
                let name = definition.name();
                match name {
                    "ferrumc:chunk" | "ferrumc:chunk_overlay" => {
                        validate_chunk_row(&generations, name, key, value)?;
                    }
                    "ferrumc:entity" => return Err(Error::LegacyEntitiesPresent),
                    "ferrumc:player" => {
                        require_unscoped(descriptor_count, name)?;
                        if key.len() != 16 {
                            return Err(bad(name, "invalid player UUID length"));
                        }
                        crate::codec::decode_player_record(value).map_err(|e| record(name, e))?;
                    }
                    "ferrumc:plugin" => validate_plugin(key, value)?,
                    "ferrumc:mutation_log" | "ferrumc:mutation_batch" => {
                        journal.row(descriptor_count, name, key, value, path)?;
                    }
                    _ => return Err(Error::CatalogMismatch),
                }
            }
        }
        journal.validate(watermark)?;
        drop(generations);
        fingerprint(txn, 4)
    })();
    journal.finish(result)
}

fn validate_chunk_row(
    generations: &impl ReadableTable<&'static [u8], &'static [u8]>,
    name: &'static str,
    key: &[u8],
    value: &[u8],
) -> MigrationResult<()> {
    let chunk =
        crate::persistent_entity::raw::decode_chunk(key, name).map_err(|e| record(name, e))?;
    let scope = GenerationKey::new(chunk.world(), chunk.dimension());
    let identity = generations
        .get(scope.encode().as_slice())
        .map_err(|e| backend("descriptor point lookup", e))?
        .ok_or_else(|| {
            record(
                name,
                StorageError::MissingGenerationIdentity { key: Some(scope) },
            )
        })?;
    crate::generation::decode_identity(scope, identity.value()).map_err(|e| record(name, e))?;
    let actual = if name == CHUNK_TABLE.name() {
        crate::codec::decode_chunk_record(value)
            .map_err(|e| record(name, e))?
            .chunk()
            .pos()
    } else {
        crate::codec::decode_chunk_overlay_record(value)
            .map_err(|e| record(name, e))?
            .pos()
    };
    if actual != chunk.pos() {
        return Err(record(
            name,
            StorageError::ChunkPositionMismatch {
                expected: chunk.pos(),
                actual,
            },
        ));
    }
    Ok(())
}

pub(in crate::redb_store) fn require_unscoped(
    count: usize,
    table: &'static str,
) -> MigrationResult<()> {
    if count == 0 {
        return Err(bad(
            table,
            "unscoped state has no actual generation descriptor",
        ));
    }
    Ok(())
}

fn validate_plugin(key: &[u8], value: &[u8]) -> MigrationResult<()> {
    let name = PLUGIN_TABLE.name();
    let prefix: [u8; 4] = key
        .get(..4)
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| bad(name, "truncated namespace prefix"))?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length > MAX_UUID_STORE_PLUGIN_ID_LEN {
        return Err(bad(name, "plugin namespace exceeds4096 UTF-8 bytes"));
    }
    let end = 4usize
        .checked_add(length)
        .ok_or_else(|| bad(name, "namespace prefix overflow"))?;
    let plugin = key
        .get(4..end)
        .ok_or_else(|| bad(name, "truncated namespace"))?;
    let storage_key = key.get(end..).ok_or_else(|| bad(name, "truncated key"))?;
    if storage_key.len() > MAX_PLUGIN_KEY_LEN
        || std::str::from_utf8(plugin).is_err()
        || std::str::from_utf8(storage_key).is_err()
    {
        return Err(bad(name, "invalid key length/UTF-8"));
    }
    if value.len() > MAX_PLUGIN_VALUE_LEN {
        return Err(record(
            name,
            StorageError::ValueTooLarge {
                len: value.len(),
                max: MAX_PLUGIN_VALUE_LEN,
            },
        ));
    }
    Ok(())
}

pub(in crate::redb_store) struct JournalValidation {
    last: Option<u64>,
    index: Option<ReceiptIndex>,
    budget: Arc<DiskBudget>,
}
impl JournalValidation {
    pub(in crate::redb_store) fn new(limit: u64) -> Self {
        Self {
            last: None,
            index: None,
            budget: DiskBudget::new(UuidEntityMigrationScratchKind::ReceiptIndex, limit),
        }
    }
    pub(in crate::redb_store) fn row(
        &mut self,
        descriptors: usize,
        name: &'static str,
        key: &[u8],
        value: &[u8],
        path: &Path,
    ) -> MigrationResult<()> {
        require_unscoped(descriptors, name)?;
        match name {
            "ferrumc:mutation_log" => {
                let id = mutation_id_from_key(key).map_err(|e| record(name, e))?;
                let expected = self.last.map_or(Some(0), |last| last.checked_add(1));
                if expected != Some(id) {
                    return Err(bad(name, "journal is not contiguous from zero"));
                }
                if crate::codec::decode_mutation_log_record(value)
                    .map_err(|e| record(name, e))?
                    .id()
                    != id
                {
                    return Err(bad(name, "journal key/record ID mismatch"));
                }
                self.last = Some(id);
            }
            "ferrumc:mutation_batch" => {
                let bytes: [u8; 16] = key
                    .try_into()
                    .map_err(|_| bad(name, "invalid receipt UUID length"))?;
                let batch = JournalBatchId::from_bytes(bytes);
                let receipt = decode_journal_receipt(batch, value).map_err(|e| record(name, e))?;
                if let Some(range) = receipt.id_range() {
                    if self.last.is_none_or(|last| *range.end() > last) {
                        return Err(bad(name, "receipt range has missing journal records"));
                    }
                    if self.index.is_none() {
                        self.index = Some(ReceiptIndex::new(path, Arc::clone(&self.budget))?);
                    }
                    if let Some(index) = &mut self.index {
                        index.insert(*range.start(), *range.end())?;
                    }
                }
            }
            _ => return Err(Error::CatalogMismatch),
        }
        Ok(())
    }
    pub(in crate::redb_store) fn validate(&self, watermark: Option<u64>) -> MigrationResult<()> {
        if self.last != watermark {
            return Err(bad(META_TABLE.name(), "journal watermark mismatch"));
        }
        if let Some(index) = &self.index {
            index.validate()?;
        }
        Ok(())
    }
    pub(in crate::redb_store) fn finish<T>(
        mut self,
        result: MigrationResult<T>,
    ) -> MigrationResult<T> {
        if let Some(index) = self.index.take() {
            index.cleanup()?;
        }
        if let Some(error) = self.budget.error() {
            return Err(error);
        }
        result
    }
}

struct ReceiptIndex {
    txn: Option<WriteTransaction>,
    db: Option<Database>,
    temporary: Option<TemporaryFile>,
    budget: Arc<DiskBudget>,
}
impl ReceiptIndex {
    fn new(source: &Path, budget: Arc<DiskBudget>) -> MigrationResult<Self> {
        let parent = source
            .parent()
            .ok_or_else(|| Error::InvalidFile(source.to_owned()))?;
        let temporary = TemporaryFile::create(parent, "ferrumc-uuid-migration-receipts")?;
        let file = temporary.file()?;
        let mut builder = Database::builder();
        builder.set_cache_size(16 * 1024 * 1024);
        let shared = super::files::SharedBackend::new(file.backend(), Some(Arc::clone(&budget)));
        let db = match builder.create_with_backend(shared) {
            Ok(db) => db,
            Err(e) => {
                temporary.cleanup()?;
                return Err(budget.map("open receipt index", e));
            }
        };
        let txn = match db.begin_write() {
            Ok(txn) => txn,
            Err(e) => {
                drop(db);
                temporary.cleanup()?;
                return Err(budget.map("receipt index transaction", e));
            }
        };
        Ok(Self {
            txn: Some(txn),
            db: Some(db),
            temporary: Some(temporary),
            budget,
        })
    }
    fn txn(&self) -> MigrationResult<&WriteTransaction> {
        self.txn
            .as_ref()
            .ok_or_else(|| bad("receipt index", "transaction already closed"))
    }
    fn insert(&mut self, first: u64, last: u64) -> MigrationResult<()> {
        let mut table = self
            .txn()?
            .open_table(RECEIPT_INDEX)
            .map_err(|e| self.budget.map("receipt index table", e))?;
        if table
            .get(first)
            .map_err(|e| self.budget.map("receipt range lookup", e))?
            .is_some()
        {
            return Err(bad(
                MUTATION_BATCH_TABLE.name(),
                "overlapping receipt ranges",
            ));
        }
        table
            .insert(first, last.to_be_bytes().as_slice())
            .map_err(|e| self.budget.map("receipt range insert", e))?;
        Ok(())
    }
    fn validate(&self) -> MigrationResult<()> {
        let table = self
            .txn()?
            .open_table(RECEIPT_INDEX)
            .map_err(|e| self.budget.map("receipt index table", e))?;
        let mut previous = None;
        for row in table
            .iter()
            .map_err(|e| self.budget.map("receipt index iterate", e))?
        {
            let (first, last) = row.map_err(|e| self.budget.map("receipt index row", e))?;
            let bytes: [u8; 8] = last
                .value()
                .try_into()
                .map_err(|_| bad("receipt index", "invalid own range value"))?;
            if previous.is_some_and(|end| first.value() <= end) {
                return Err(bad(
                    MUTATION_BATCH_TABLE.name(),
                    "overlapping receipt ranges",
                ));
            }
            previous = Some(u64::from_be_bytes(bytes));
        }
        Ok(())
    }
    fn cleanup(mut self) -> MigrationResult<()> {
        let aborted = self
            .txn
            .take()
            .map(redb::WriteTransaction::abort)
            .transpose()
            .map_err(|e| self.budget.map("abort receipt index", e));
        drop(self.db.take());
        if let Some(temporary) = self.temporary.take() {
            temporary.cleanup()?;
        }
        aborted?;
        Ok(())
    }
}

pub(super) fn scan_format5(
    txn: &WriteTransaction,
    path: &Path,
    limits: UuidEntityMigrationLimits,
) -> MigrationResult<Scan> {
    crate::redb_store::uuid_validation::validate_complete_migration(
        txn,
        path,
        limits.max_receipt_index_bytes(),
    )?;
    fingerprint(txn, 5)
}

pub(super) fn compare_exact_delta(
    source: &WriteTransaction,
    backup: &WriteTransaction,
) -> MigrationResult<()> {
    {
        let source = source
            .open_table(META_TABLE)
            .map_err(|e| backend("delta source metadata", e))?;
        let backup = backup
            .open_table(META_TABLE)
            .map_err(|e| backend("delta backup metadata", e))?;
        let mut a = source
            .iter()
            .map_err(|e| backend("delta metadata iterate", e))?;
        let mut b = backup
            .iter()
            .map_err(|e| backend("delta metadata iterate", e))?;
        loop {
            match (a.next(), b.next()) {
                (None, None) => break,
                (Some(a), Some(b)) => {
                    let (ak, av) = a.map_err(|e| backend("delta metadata row", e))?;
                    let (bk, bv) = b.map_err(|e| backend("delta metadata row", e))?;
                    if ak.value() != bk.value()
                        || if ak.value() == META_FORMAT_KEY {
                            av.value() != 5 || bv.value() != 4
                        } else {
                            av.value() != bv.value()
                        }
                    {
                        return Err(Error::MigrationDeltaMismatch);
                    }
                }
                _ => return Err(Error::MigrationDeltaMismatch),
            }
        }
    }
    for (index, definition) in TABLES.iter().enumerate() {
        if (4..=6).contains(&index) {
            if source
                .open_table(*definition)
                .map_err(|e| backend("delta new index", e))?
                .iter()
                .map_err(|e| backend("delta new index iterate", e))?
                .next()
                .is_some()
            {
                return Err(Error::MigrationDeltaMismatch);
            }
            continue;
        }
        let a = source
            .open_table(*definition)
            .map_err(|e| backend("delta source table", e))?;
        let b = backup
            .open_table(*definition)
            .map_err(|e| backend("delta backup table", e))?;
        let mut a = a.iter().map_err(|e| backend("delta source iterate", e))?;
        let mut b = b.iter().map_err(|e| backend("delta backup iterate", e))?;
        loop {
            match (a.next(), b.next()) {
                (None, None) => break,
                (Some(a), Some(b)) => {
                    let (ak, av) = a.map_err(|e| backend("delta source row", e))?;
                    let (bk, bv) = b.map_err(|e| backend("delta backup row", e))?;
                    if ak.value() != bk.value() || av.value() != bv.value() {
                        return Err(Error::MigrationDeltaMismatch);
                    }
                }
                _ => return Err(Error::MigrationDeltaMismatch),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fingerprint_presence_type_and_length_domains_are_distinct_and_overflow_refuses() {
        fn hash(present: bool, meta: bool, key: &[u8], value: &[u8]) -> [u8; 32] {
            let mut fp = Fingerprint::new();
            fp.table("same", present, meta);
            if present {
                fp.row(0, key, value).unwrap();
            }
            fp.end(0);
            fp.hash.finalize().into()
        }
        assert_ne!(hash(false, false, b"", b""), hash(true, false, b"", b""));
        assert_ne!(
            hash(true, true, b"a", b"bc"),
            hash(true, false, b"a", b"bc")
        );
        assert_ne!(
            hash(true, false, b"a", b"bc"),
            hash(true, false, b"ab", b"c")
        );
        let mut fp = Fingerprint::new();
        fp.counts[0] = u64::MAX;
        assert!(matches!(fp.row(0, b"", b""), Err(Error::CounterOverflow)));
        let mut fp = Fingerprint::new();
        fp.bytes = u64::MAX;
        assert!(matches!(fp.row(0, b"a", b""), Err(Error::CounterOverflow)));
    }
}
