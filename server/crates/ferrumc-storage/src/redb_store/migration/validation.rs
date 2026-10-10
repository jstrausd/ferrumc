//! Bounded row-at-a-time validation of the complete existing durable layout.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use ferrumc_math::ChunkPos;
use redb::{Database, ReadableTable, TableDefinition, TableHandle};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::FileIdentity;
use super::{
    backend, io_error, GenerationMigrationError as Error, GenerationMigrationTable as Kind,
};
use super::{LegacyGenerationDeclaration, MigrationResult, MAX_MIGRATION_PLUGIN_ID_LEN};
use crate::codec;
use crate::redb_store::{
    decode_journal_receipt, mutation_id_from_key, CHUNK_OVERLAY_TABLE, CHUNK_TABLE, ENTITY_TABLE,
    GENERATION_STORE_FORMAT_VERSION, GENERATION_TABLE, META_FORMAT_KEY, META_LAST_MUTATION_ID_KEY,
    META_TABLE, MUTATION_BATCH_TABLE, MUTATION_LOG_TABLE, PLAYER_TABLE, PLUGIN_TABLE,
    STORE_FORMAT_VERSION,
};
use crate::{
    GenerationKey, JournalBatchId, StorageError, MAX_GENERATION_IDENTITIES, MAX_PLUGIN_KEY_LEN,
    MAX_PLUGIN_VALUE_LEN,
};

const INDEX_CACHE_BYTES: usize = 16 * 1024 * 1024;
const RECEIPT_INDEX: TableDefinition<'_, u64, &[u8]> = TableDefinition::new("receipt_ranges");

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Scan {
    pub(super) format: u64,
    pub(super) counts: [u64; 9],
    pub(super) logical_bytes: u64,
    pub(super) sha256: [u8; 32],
}

struct Fingerprint {
    hash: Sha256,
    counts: [u64; 9],
    logical_bytes: u64,
}

impl Fingerprint {
    fn new() -> Self {
        let mut hash = Sha256::new();
        hash.update(b"FerrumC generation migration logical snapshot v1\0");
        Self {
            hash,
            counts: [0; 9],
            logical_bytes: 0,
        }
    }

    fn start_table(&mut self, name: &str) {
        self.hash.update(b"table\0");
        self.hash.update((name.len() as u64).to_be_bytes());
        self.hash.update(name.as_bytes());
    }

    fn row(&mut self, kind: Kind, key: &[u8], value: &[u8]) -> MigrationResult<()> {
        self.counts[kind.index()] = self.counts[kind.index()]
            .checked_add(1)
            .ok_or(Error::CounterOverflow)?;
        self.logical_bytes = self
            .logical_bytes
            .checked_add(key.len() as u64)
            .and_then(|v| v.checked_add(value.len() as u64))
            .ok_or(Error::CounterOverflow)?;
        self.hash.update(b"row\0");
        self.hash.update((key.len() as u64).to_be_bytes());
        self.hash.update(key);
        self.hash.update((value.len() as u64).to_be_bytes());
        self.hash.update(value);
        Ok(())
    }

    fn finish_table(&mut self, kind: Kind) {
        self.hash.update(b"end\0");
        self.hash.update(self.counts[kind.index()].to_be_bytes());
    }

    fn finish(self, format: u64) -> Scan {
        Scan {
            format,
            counts: self.counts,
            logical_bytes: self.logical_bytes,
            sha256: self.hash.finalize().into(),
        }
    }
}

fn record_error(table: &'static str, source: StorageError) -> Error {
    Error::Record {
        table,
        source: Box::new(source),
    }
}

fn validate_catalog(txn: &redb::WriteTransaction) -> MigrationResult<u64> {
    let expected = [
        META_TABLE.name(),
        CHUNK_TABLE.name(),
        CHUNK_OVERLAY_TABLE.name(),
        ENTITY_TABLE.name(),
        PLAYER_TABLE.name(),
        PLUGIN_TABLE.name(),
        MUTATION_LOG_TABLE.name(),
        MUTATION_BATCH_TABLE.name(),
    ];
    let mut tables = 0;
    let mut generation_table = false;
    for table in txn.list_tables().map_err(|e| backend("catalog read", e))? {
        if table.name() == GENERATION_TABLE.name() {
            generation_table = true;
        } else if !expected.contains(&table.name()) {
            return Err(Error::CatalogMismatch);
        }
        tables += 1;
        if tables > 9 {
            return Err(Error::CatalogMismatch);
        }
    }
    if txn
        .list_multimap_tables()
        .map_err(|e| backend("multimap catalog", e))?
        .next()
        .is_some()
        || tables != 8 + usize::from(generation_table)
    {
        return Err(Error::CatalogMismatch);
    }
    let meta = txn
        .open_table(META_TABLE)
        .map_err(|e| backend("metadata shape", e))?;
    let version = meta
        .get(META_FORMAT_KEY)
        .map_err(|e| backend("format marker", e))?
        .map(|v| v.value());
    match (version, generation_table) {
        (Some(STORE_FORMAT_VERSION), false) => Ok(STORE_FORMAT_VERSION),
        (Some(GENERATION_STORE_FORMAT_VERSION), true) => Ok(GENERATION_STORE_FORMAT_VERSION),
        (Some(STORE_FORMAT_VERSION | GENERATION_STORE_FORMAT_VERSION), _) => {
            Err(Error::CatalogMismatch)
        }
        _ => Err(Error::UnsupportedFormat(version)),
    }
}

fn require_scope(declaration: &LegacyGenerationDeclaration, bytes: &[u8]) -> MigrationResult<()> {
    let key = GenerationKey::decode(bytes).map_err(|e| record_error("scope", e))?;
    if declaration.scopes().binary_search(&key).is_err() {
        return Err(Error::UndeclaredScope { key });
    }
    Ok(())
}

fn chunk_pos(key: &[u8]) -> ChunkPos {
    ChunkPos::new(
        i32::from_be_bytes([key[8], key[9], key[10], key[11]]),
        i32::from_be_bytes([key[12], key[13], key[14], key[15]]),
    )
}

fn validate_plugin(key: &[u8], value: &[u8]) -> MigrationResult<()> {
    let prefix: [u8; 4] = key
        .get(..4)
        .and_then(|v| v.try_into().ok())
        .ok_or(Error::InvalidPluginKey)?;
    let plugin_len = u32::from_be_bytes(prefix) as usize;
    if plugin_len > MAX_MIGRATION_PLUGIN_ID_LEN {
        return Err(Error::InvalidPluginKey);
    }
    let end = 4_usize
        .checked_add(plugin_len)
        .ok_or(Error::InvalidPluginKey)?;
    let plugin = key.get(4..end).ok_or(Error::InvalidPluginKey)?;
    let storage_key = key.get(end..).ok_or(Error::InvalidPluginKey)?;
    if storage_key.len() > MAX_PLUGIN_KEY_LEN
        || std::str::from_utf8(plugin).is_err()
        || std::str::from_utf8(storage_key).is_err()
    {
        return Err(Error::InvalidPluginKey);
    }
    if value.len() > MAX_PLUGIN_VALUE_LEN {
        return Err(record_error(
            PLUGIN_TABLE.name(),
            StorageError::ValueTooLarge {
                len: value.len(),
                max: MAX_PLUGIN_VALUE_LEN,
            },
        ));
    }
    Ok(())
}

/// No full-world collection is constructed. The only variable heap result is
/// the <=4096-scope caller declaration; record decoding retains one row at once.
pub(super) fn scan(
    txn: &redb::WriteTransaction,
    declaration: &LegacyGenerationDeclaration,
    source_path: &Path,
) -> MigrationResult<Scan> {
    let format = validate_catalog(txn)?;
    let mut fingerprint = Fingerprint::new();
    let watermark = scan_metadata(txn, &mut fingerprint)?;
    scan_payloads(txn, declaration, &mut fingerprint)?;
    let last = scan_journal(txn, declaration, &mut fingerprint)?;
    if watermark.is_some() && declaration.scopes().len() != 1 {
        return Err(Error::AmbiguousUnscopedState);
    }
    if watermark != last {
        return Err(Error::JournalWatermark {
            stored: watermark,
            last,
        });
    }
    scan_receipts(txn, declaration, source_path, last, &mut fingerprint)?;
    if format == GENERATION_STORE_FORMAT_VERSION {
        scan_descriptors(txn, declaration, &mut fingerprint)?;
    }
    Ok(fingerprint.finish(format))
}

fn scan_metadata(
    txn: &redb::WriteTransaction,
    fingerprint: &mut Fingerprint,
) -> MigrationResult<Option<u64>> {
    let mut watermark = None;
    {
        let meta = txn
            .open_table(META_TABLE)
            .map_err(|e| backend("scan metadata", e))?;
        fingerprint.start_table(META_TABLE.name());
        for row in meta.iter().map_err(|e| backend("metadata iterate", e))? {
            let (key, value) = row.map_err(|e| backend("metadata row", e))?;
            match key.value() {
                META_FORMAT_KEY => {}
                META_LAST_MUTATION_ID_KEY => watermark = Some(value.value()),
                _ => return Err(Error::UnknownMetadata),
            }
            fingerprint.row(
                Kind::Metadata,
                key.value().as_bytes(),
                &value.value().to_be_bytes(),
            )?;
        }
        fingerprint.finish_table(Kind::Metadata);
    }
    Ok(watermark)
}

fn scan_payloads(
    txn: &redb::WriteTransaction,
    declaration: &LegacyGenerationDeclaration,
    fingerprint: &mut Fingerprint,
) -> MigrationResult<()> {
    for (definition, kind, name) in [
        (CHUNK_TABLE, Kind::Chunks, "ferrumc:chunk"),
        (
            CHUNK_OVERLAY_TABLE,
            Kind::ChunkOverlays,
            "ferrumc:chunk_overlay",
        ),
        (ENTITY_TABLE, Kind::Entities, "ferrumc:entity"),
        (PLAYER_TABLE, Kind::Players, "ferrumc:player"),
        (PLUGIN_TABLE, Kind::PluginValues, "ferrumc:plugin"),
    ] {
        let table = txn
            .open_table(definition)
            .map_err(|e| backend("scan table shape", e))?;
        fingerprint.start_table(name);
        for row in table.iter().map_err(|e| backend("table iterate", e))? {
            let (key, value) = row.map_err(|e| backend("table row", e))?;
            let (key, value) = (key.value(), value.value());
            let expected_len = match kind {
                Kind::Chunks | Kind::ChunkOverlays | Kind::Players => Some(16),
                Kind::Entities => Some(12),
                _ => None,
            };
            if expected_len.is_some_and(|len| key.len() != len) {
                return Err(Error::MalformedKey {
                    table: name,
                    len: key.len(),
                });
            }
            match kind {
                Kind::Chunks => {
                    require_scope(declaration, &key[..8])?;
                    let record =
                        codec::decode_chunk_record(value).map_err(|e| record_error(name, e))?;
                    let expected = chunk_pos(key);
                    let actual = record.chunk().pos();
                    if expected != actual {
                        return Err(record_error(
                            name,
                            StorageError::ChunkPositionMismatch { expected, actual },
                        ));
                    }
                }
                Kind::ChunkOverlays => {
                    require_scope(declaration, &key[..8])?;
                    let record = codec::decode_chunk_overlay_record(value)
                        .map_err(|e| record_error(name, e))?;
                    let expected = chunk_pos(key);
                    let actual = record.pos();
                    if expected != actual {
                        return Err(record_error(
                            name,
                            StorageError::ChunkPositionMismatch { expected, actual },
                        ));
                    }
                }
                Kind::Entities => {
                    require_scope(declaration, &key[..8])?;
                    codec::decode_entity_record(value).map_err(|e| record_error(name, e))?;
                }
                Kind::Players => {
                    if declaration.scopes().len() != 1 {
                        return Err(Error::AmbiguousUnscopedState);
                    }
                    codec::decode_player_record(value).map_err(|e| record_error(name, e))?;
                }
                Kind::PluginValues => validate_plugin(key, value)?,
                _ => return Err(Error::CatalogMismatch),
            }
            fingerprint.row(kind, key, value)?;
        }
        fingerprint.finish_table(kind);
    }
    Ok(())
}

fn scan_journal(
    txn: &redb::WriteTransaction,
    declaration: &LegacyGenerationDeclaration,
    fingerprint: &mut Fingerprint,
) -> MigrationResult<Option<u64>> {
    let mut last = None;
    {
        let journal = txn
            .open_table(MUTATION_LOG_TABLE)
            .map_err(|e| backend("scan journal", e))?;
        let mut expected = Some(0);
        fingerprint.start_table(MUTATION_LOG_TABLE.name());
        for row in journal.iter().map_err(|e| backend("journal iterate", e))? {
            let (key, value) = row.map_err(|e| backend("journal row", e))?;
            if declaration.scopes().len() != 1 {
                return Err(Error::AmbiguousUnscopedState);
            }
            let id = mutation_id_from_key(key.value())
                .map_err(|e| record_error(MUTATION_LOG_TABLE.name(), e))?;
            if expected != Some(id) {
                return Err(Error::JournalSequence {
                    expected,
                    actual: id,
                });
            }
            let record = codec::decode_mutation_log_record(value.value())
                .map_err(|e| record_error(MUTATION_LOG_TABLE.name(), e))?;
            if record.id() != id {
                return Err(Error::JournalRecordId {
                    key: id,
                    record: record.id(),
                });
            }
            last = Some(id);
            expected = id.checked_add(1);
            fingerprint.row(Kind::JournalRecords, key.value(), value.value())?;
        }
        fingerprint.finish_table(Kind::JournalRecords);
    }
    Ok(last)
}

fn scan_receipts(
    txn: &redb::WriteTransaction,
    declaration: &LegacyGenerationDeclaration,
    source_path: &Path,
    last: Option<u64>,
    fingerprint: &mut Fingerprint,
) -> MigrationResult<()> {
    {
        let receipts = txn
            .open_table(MUTATION_BATCH_TABLE)
            .map_err(|e| backend("scan receipts", e))?;
        let mut index = None;
        fingerprint.start_table(MUTATION_BATCH_TABLE.name());
        for row in receipts.iter().map_err(|e| backend("receipt iterate", e))? {
            let (key, value) = row.map_err(|e| backend("receipt row", e))?;
            if declaration.scopes().len() != 1 {
                return Err(Error::AmbiguousUnscopedState);
            }
            let key_bytes: [u8; 16] = key.value().try_into().map_err(|_| Error::MalformedKey {
                table: MUTATION_BATCH_TABLE.name(),
                len: key.value().len(),
            })?;
            let batch = JournalBatchId::from_bytes(key_bytes);
            let receipt = decode_journal_receipt(batch, value.value())
                .map_err(|e| record_error(MUTATION_BATCH_TABLE.name(), e))?;
            if let Some(range) = receipt.id_range() {
                if last.is_none_or(|last| *range.end() > last) {
                    let id = last
                        .and_then(|last| last.checked_add(1))
                        .map_or(*range.start(), |next| next.max(*range.start()));
                    return Err(record_error(
                        MUTATION_BATCH_TABLE.name(),
                        StorageError::JournalReceiptMissingRecord {
                            batch_id: batch,
                            id,
                        },
                    ));
                }
                if index.is_none() {
                    index = Some(ReceiptIndex::new(source_path)?);
                }
                if let Some(index) = &mut index {
                    index.insert(*range.start(), *range.end(), batch)?;
                }
            }
            fingerprint.row(Kind::JournalReceipts, key.value(), value.value())?;
        }
        if let Some(index) = &index {
            index.validate_nonoverlapping()?;
        }
        fingerprint.finish_table(Kind::JournalReceipts);
    }
    Ok(())
}

fn scan_descriptors(
    txn: &redb::WriteTransaction,
    declaration: &LegacyGenerationDeclaration,
    fingerprint: &mut Fingerprint,
) -> MigrationResult<()> {
    let table = txn
        .open_table(GENERATION_TABLE)
        .map_err(|e| backend("scan identities", e))?;
    fingerprint.start_table(GENERATION_TABLE.name());
    for row in table.iter().map_err(|e| backend("identity iterate", e))? {
        let (key, value) = row.map_err(|e| backend("identity row", e))?;
        let decoded_key = GenerationKey::decode(key.value())
            .map_err(|e| record_error(GENERATION_TABLE.name(), e))?;
        require_scope(declaration, key.value())?;
        let stored = crate::generation::decode_identity(decoded_key, value.value())
            .map_err(|e| record_error(GENERATION_TABLE.name(), e))?;
        if stored != declaration.identity() {
            return Err(record_error(
                GENERATION_TABLE.name(),
                StorageError::GenerationIdentityMismatch {
                    key: decoded_key,
                    stored,
                    requested: declaration.identity(),
                },
            ));
        }
        fingerprint.row(Kind::GenerationIdentities, key.value(), value.value())?;
        if fingerprint.counts[Kind::GenerationIdentities.index()] > MAX_GENERATION_IDENTITIES as u64
        {
            return Err(Error::TooManyScopes);
        }
    }
    if fingerprint.counts[Kind::GenerationIdentities.index()] != declaration.scopes().len() as u64 {
        return Err(Error::DescriptorSetMismatch);
    }
    fingerprint.finish_table(Kind::GenerationIdentities);
    Ok(())
}

/// A disk index sorts arbitrary UUID-keyed receipt ranges with bounded cache.
/// No rows are committed or added to the source database; dropping rolls back
/// the private index and removes only its individually owned temporary file.
struct ReceiptIndex {
    txn: Option<redb::WriteTransaction>,
    db: Option<Database>,
    path: PathBuf,
    identity: FileIdentity,
}

impl ReceiptIndex {
    fn new(source_path: &Path) -> MigrationResult<Self> {
        let parent = source_path
            .parent()
            .ok_or_else(|| Error::InvalidFile(source_path.to_owned()))?;
        for _ in 0..16 {
            let path = parent.join(format!(
                ".ferrumc-migration-receipts-{}.redb",
                Uuid::new_v4()
            ));
            let file = match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io_error("create receipt index", &path, e)),
            };
            let mut index = Self {
                txn: None,
                db: None,
                identity: FileIdentity::from_metadata(
                    &file
                        .metadata()
                        .map_err(|e| io_error("stat receipt index", &path, e))?,
                )?,
                path,
            };
            let mut builder = Database::builder();
            builder.set_cache_size(INDEX_CACHE_BYTES);
            let db = builder
                .create_file(file)
                .map_err(|e| backend("receipt index open", e))?;
            let txn = db
                .begin_write()
                .map_err(|e| backend("receipt index transaction", e))?;
            index.db = Some(db);
            index.txn = Some(txn);
            return Ok(index);
        }
        Err(io_error(
            "create receipt index",
            parent,
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "receipt index name collision limit reached",
            ),
        ))
    }

    fn transaction(&self) -> MigrationResult<&redb::WriteTransaction> {
        self.txn.as_ref().ok_or_else(|| {
            backend(
                "receipt index ownership",
                redb::StorageError::Io(std::io::Error::other(
                    "receipt index transaction is absent",
                )),
            )
        })
    }

    fn insert(&mut self, first: u64, last: u64, batch: JournalBatchId) -> MigrationResult<()> {
        let mut table = self
            .transaction()?
            .open_table(RECEIPT_INDEX)
            .map_err(|e| backend("receipt index table", e))?;
        let mut encoded = [0; 24];
        encoded[..8].copy_from_slice(&last.to_be_bytes());
        encoded[8..].copy_from_slice(batch.as_bytes());
        if let Some(previous) = table
            .get(first)
            .map_err(|e| backend("receipt index get", e))?
        {
            let (_, previous_batch) = decode_index_value(previous.value())?;
            return Err(Error::OverlappingReceipts {
                first: previous_batch,
                second: batch,
            });
        }
        table
            .insert(first, encoded.as_slice())
            .map_err(|e| backend("receipt index insert", e))?;
        Ok(())
    }

    fn validate_nonoverlapping(&self) -> MigrationResult<()> {
        let table = self
            .transaction()?
            .open_table(RECEIPT_INDEX)
            .map_err(|e| backend("receipt index scan table", e))?;
        let mut previous = None;
        for row in table
            .iter()
            .map_err(|e| backend("receipt index iterate", e))?
        {
            let (first, value) = row.map_err(|e| backend("receipt index row", e))?;
            let (last, batch) = decode_index_value(value.value())?;
            if let Some((previous_last, previous_batch)) = previous {
                if first.value() <= previous_last {
                    return Err(Error::OverlappingReceipts {
                        first: previous_batch,
                        second: batch,
                    });
                }
            }
            previous = Some((last, batch));
        }
        Ok(())
    }
}

fn decode_index_value(bytes: &[u8]) -> MigrationResult<(u64, JournalBatchId)> {
    let bytes: &[u8; 24] = bytes.try_into().map_err(|_| Error::MalformedKey {
        table: "private receipt index",
        len: bytes.len(),
    })?;
    let last = u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]);
    let mut batch = [0; 16];
    batch.copy_from_slice(&bytes[8..]);
    Ok((last, JournalBatchId::from_bytes(batch)))
}

impl Drop for ReceiptIndex {
    fn drop(&mut self) {
        drop(self.txn.take());
        drop(self.db.take());
        let owned_path = fs::symlink_metadata(&self.path).is_ok_and(|metadata| {
            metadata.is_file() && FileIdentity::from_metadata(&metadata).ok() == Some(self.identity)
        });
        if !owned_path {
            tracing::warn!(path = %self.path.display(), "owned migration receipt index path was removed/replaced; cleanup skipped");
            return;
        }
        if let Err(error) = fs::remove_file(&self.path) {
            // Only this precisely owned index is eligible for deletion. A failed
            // cleanup is operational evidence, never a broad temp-directory purge.
            tracing::warn!(path = %self.path.display(), %error, "cannot remove owned migration receipt index");
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn private_index_parser_and_report_counters_are_bounded() {
        for len in [0, 1, 23, 25, 32] {
            assert!(decode_index_value(&vec![0; len]).is_err());
        }
        let bytes = [255; 24];
        assert_eq!(
            decode_index_value(&bytes).unwrap(),
            (u64::MAX, JournalBatchId::from_bytes([255; 16]))
        );
        let mut fingerprint = Fingerprint::new();
        fingerprint.counts[Kind::Chunks.index()] = u64::MAX;
        assert!(matches!(
            fingerprint.row(Kind::Chunks, &[], &[]),
            Err(Error::CounterOverflow)
        ));
        let mut fingerprint = Fingerprint::new();
        fingerprint.logical_bytes = u64::MAX;
        assert!(matches!(
            fingerprint.row(Kind::Chunks, &[0], &[]),
            Err(Error::CounterOverflow)
        ));
    }

    #[test]
    fn fingerprint_separates_row_lengths_and_table_boundaries() {
        let hash = |name: &str, key: &[u8], value: &[u8]| {
            let mut fingerprint = Fingerprint::new();
            fingerprint.start_table(name);
            fingerprint.row(Kind::Chunks, key, value).unwrap();
            fingerprint.finish_table(Kind::Chunks);
            fingerprint.finish(3).sha256
        };
        assert_ne!(hash("a", b"ab", b"c"), hash("a", b"a", b"bc"));
        assert_ne!(hash("a", b"ab", b"c"), hash("b", b"ab", b"c"));
    }

    #[test]
    fn receipt_index_cleanup_never_deletes_a_replacement_file() {
        let dir = tempfile::tempdir().unwrap();
        let index = ReceiptIndex::new(&dir.path().join("source.redb")).unwrap();
        let path = index.path.clone();
        let moved = dir.path().join("original-index.redb");
        fs::rename(&path, &moved).unwrap();
        fs::write(&path, b"unrelated replacement").unwrap();
        drop(index);
        assert_eq!(fs::read(&path).unwrap(), b"unrelated replacement");
        assert!(moved.is_file());
    }
}
