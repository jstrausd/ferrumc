//! Complete bounded Format5 startup validation, without source normalization.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use ferrumc_core::{Result, ServerError};
use redb::{Database, ReadableTable, TableDefinition, TableHandle};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::uuid_entities::{write_reader, EPOCH_TABLE, INDEX_TABLE, LOCATOR_TABLE};
use super::{
    backend_err, decode_journal_receipt, incompatible_data_err, mutation_id_from_key,
    schema_table_err, StoreOrigin, CHUNK_OVERLAY_TABLE, CHUNK_TABLE, ENTITY_TABLE,
    GENERATION_TABLE, META_FORMAT_KEY, META_LAST_MUTATION_ID_KEY, META_TABLE, MUTATION_BATCH_TABLE,
    MUTATION_LOG_TABLE, PLAYER_TABLE, PLUGIN_TABLE,
};
use crate::persistent_entity::engine::{self, EntityRead, RawTable};
use crate::persistent_entity::raw;
use crate::{
    GenerationKey, JournalBatchId, PersistentEntityKey, StorageError, MAX_PLUGIN_KEY_LEN,
    MAX_PLUGIN_VALUE_LEN, MAX_UUID_STORE_PLUGIN_ID_LEN,
};

const FORMAT: u64 = 5;
const INDEX_CACHE_BYTES: usize = 16 * 1024 * 1024;
const RECEIPT_INDEX: TableDefinition<'_, u64, &[u8]> = TableDefinition::new("receipt_ranges");
const TABLE_NAMES: [&str; 11] = [
    "ferrumc:generation",
    "ferrumc:chunk",
    "ferrumc:chunk_overlay",
    "ferrumc:entity",
    "ferrumc:entity_locator",
    "ferrumc:entity_chunk",
    "ferrumc:entity_chunk_epoch",
    "ferrumc:player",
    "ferrumc:plugin",
    "ferrumc:mutation_log",
    "ferrumc:mutation_batch",
];
const TABLES: [TableDefinition<'_, &[u8], &[u8]>; 11] = [
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

fn bad(table: &'static str, reason: &'static str) -> ServerError {
    raw::corrupt(table, reason).into()
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Scan {
    pub(super) version: u64,
    pub(super) counts: [u64; 12],
    pub(super) logical_bytes: u64,
    pub(super) sha256: [u8; 32],
}
struct Fingerprint {
    hash: Sha256,
    counts: [u64; 12],
    logical_bytes: u64,
}
impl Fingerprint {
    fn new() -> Self {
        let mut hash = Sha256::new();
        hash.update(b"FerrumC UUID storage logical snapshot v1\0");
        Self {
            hash,
            counts: [0; 12],
            logical_bytes: 0,
        }
    }
    fn table(&mut self, name: &str) {
        self.hash.update(b"table\0");
        self.hash.update((name.len() as u64).to_be_bytes());
        self.hash.update(name.as_bytes());
    }
    fn row(&mut self, index: usize, key: &[u8], value: &[u8]) -> Result<()> {
        self.counts[index] = self.counts[index]
            .checked_add(1)
            .ok_or_else(|| bad("validator", "row counter overflow"))?;
        self.logical_bytes = self
            .logical_bytes
            .checked_add(key.len() as u64)
            .and_then(|n| n.checked_add(value.len() as u64))
            .ok_or_else(|| bad("validator", "byte counter overflow"))?;
        self.hash.update(b"row\0");
        self.hash.update((key.len() as u64).to_be_bytes());
        self.hash.update(key);
        self.hash.update((value.len() as u64).to_be_bytes());
        self.hash.update(value);
        Ok(())
    }
    fn end(&mut self, index: usize) {
        self.hash.update(b"end\0");
        self.hash.update(self.counts[index].to_be_bytes());
    }
    fn finish(self) -> Scan {
        Scan {
            version: FORMAT,
            counts: self.counts,
            logical_bytes: self.logical_bytes,
            sha256: self.hash.finalize().into(),
        }
    }
}

pub(super) fn initialize(db: &Database, origin: StoreOrigin, path: &Path) -> Result<()> {
    let txn = db.begin_write().map_err(backend_err)?;
    {
        let mut meta = txn.open_table(META_TABLE).map_err(schema_table_err)?;
        let version = meta
            .get(META_FORMAT_KEY)
            .map_err(backend_err)?
            .map(|v| v.value());
        match (origin, version) {
            (StoreOrigin::Fresh, None) => {
                meta.insert(META_FORMAT_KEY, FORMAT).map_err(backend_err)?;
            }
            (StoreOrigin::Existing, Some(FORMAT)) => {}
            (_, Some(format @ (3 | 4))) => {
                return Err(StorageError::PersistentEntityMigrationRequired { format }.into());
            }
            _ => return Err(incompatible_data_err()),
        }
    }
    if matches!(origin, StoreOrigin::Fresh) {
        for table in TABLES {
            txn.open_table(table).map_err(schema_table_err)?;
        }
        txn.commit().map_err(backend_err)?;
        return Ok(());
    }
    validate_catalog(&txn)?;
    let report = scan(&txn, path)?;
    tracing::debug!(version = report.version, counts = ?report.counts, logical_bytes = report.logical_bytes, sha256 = ?report.sha256, "validated complete UUID storage snapshot");
    drop(txn);
    Ok(())
}

fn validate_catalog(txn: &redb::WriteTransaction) -> Result<()> {
    let mut count = 0;
    for table in txn.list_tables().map_err(backend_err)? {
        if table.name() != META_TABLE.name() && !TABLES.iter().any(|t| t.name() == table.name()) {
            return Err(bad("catalog", "unknown table"));
        }
        count += 1;
        if count > 12 {
            return Err(bad("catalog", "too many tables"));
        }
    }
    if count != 12
        || txn
            .list_multimap_tables()
            .map_err(backend_err)?
            .next()
            .is_some()
    {
        return Err(bad("catalog", "missing table or unexpected multimap"));
    }
    txn.open_table(META_TABLE).map_err(schema_table_err)?;
    for table in TABLES {
        txn.open_table(table).map_err(schema_table_err)?;
    }
    Ok(())
}

/// Point descriptor lookups and one decoded row; no full-world collection.
pub(super) fn scan(txn: &redb::WriteTransaction, path: &Path) -> Result<Scan> {
    scan_impl(txn, path, None).map_err(|error| match error {
        super::uuid_migration::UuidEntityMigrationError::Validation(error) => *error,
        other => ServerError::internal(other.to_string()),
    })
}

/// Complete unchanged Format5 rules with the offline producer's bounded index.
pub(super) fn validate_complete_migration(
    txn: &redb::WriteTransaction,
    path: &Path,
    receipt_limit: u64,
) -> super::uuid_migration::MigrationResult<Scan> {
    validate_catalog(txn)?;
    scan_impl(txn, path, Some(receipt_limit))
}

fn scan_impl(
    txn: &redb::WriteTransaction,
    path: &Path,
    receipt_limit: Option<u64>,
) -> super::uuid_migration::MigrationResult<Scan> {
    let mut normal_journal = JournalValidation {
        last: None,
        receipts: None,
    };
    let mut offline_journal =
        receipt_limit.map(super::uuid_migration::validation::JournalValidation::new);
    let result = (|| {
        let mut fingerprint = Fingerprint::new();
        let watermark = {
            let meta = txn.open_table(META_TABLE).map_err(schema_table_err)?;
            let mut watermark = None;
            fingerprint.table(META_TABLE.name());
            for row in meta.iter().map_err(backend_err)? {
                let (key, value) = row.map_err(backend_err)?;
                match key.value() {
                    META_FORMAT_KEY if value.value() == FORMAT => {}
                    META_LAST_MUTATION_ID_KEY => watermark = Some(value.value()),
                    _ => {
                        return Err(
                            bad("ferrumc:meta", "unknown metadata or incorrect format").into()
                        );
                    }
                }
                fingerprint.row(0, key.value().as_bytes(), &value.value().to_be_bytes())?;
            }
            fingerprint.end(0);
            watermark
        };
        let reader = write_reader(txn)?;
        let descriptors = super::validate_generation_table(&reader.generations)?;
        for (slot, definition) in TABLES.into_iter().enumerate() {
            let index = slot + 1;
            let name = TABLE_NAMES[slot];
            // Borrow already-open relation tables: redb forbids duplicate writable opens.
            let owned;
            let table = match name {
                "ferrumc:generation" => &reader.generations,
                raw::ENTITY => &reader.entities,
                raw::LOCATOR => &reader.locators,
                raw::INDEX => &reader.indexes,
                raw::EPOCH => &reader.epochs,
                _ => {
                    owned = txn.open_table(definition).map_err(schema_table_err)?;
                    &owned
                }
            };
            fingerprint.table(name);
            for row in table.iter().map_err(backend_err)? {
                let (key, value) = row.map_err(backend_err)?;
                let (key, value) = (key.value(), value.value());
                if name == "ferrumc:mutation_log" || name == "ferrumc:mutation_batch" {
                    if let Some(journal) = &mut offline_journal {
                        journal.row(descriptors, name, key, value, path)?;
                    } else {
                        validate_journal_row(
                            &mut normal_journal,
                            descriptors,
                            name,
                            key,
                            value,
                            path,
                        )?;
                    }
                } else {
                    validate_payload_row(&reader, descriptors, name, key, value)?;
                }
                fingerprint.row(index, key, value)?;
            }
            fingerprint.end(index);
        }
        if let Some(journal) = &offline_journal {
            journal.validate(watermark)?;
        } else {
            if normal_journal.last != watermark {
                return Err(bad("ferrumc:meta", "journal watermark disagrees").into());
            }
            if let Some(index) = &normal_journal.receipts {
                index.validate_nonoverlapping()?;
            }
        }
        Ok(fingerprint.finish())
    })();
    match offline_journal {
        Some(journal) => journal.finish(result),
        None => result,
    }
}

fn validate_payload_row(
    reader: &impl EntityRead,
    descriptors: usize,
    name: &'static str,
    key: &[u8],
    value: &[u8],
) -> Result<()> {
    match name {
        "ferrumc:generation" => {
            crate::generation::decode_identity(GenerationKey::decode(key)?, value)?;
        }
        "ferrumc:chunk" | "ferrumc:chunk_overlay" => {
            let chunk = raw::decode_chunk(key, name)?;
            engine::require_identity(
                reader,
                GenerationKey::new(chunk.world(), chunk.dimension()),
                None,
            )?;
            let actual = if name == CHUNK_TABLE.name() {
                crate::codec::decode_chunk_record(value)?.chunk().pos()
            } else {
                crate::codec::decode_chunk_overlay_record(value)?.pos()
            };
            if actual != chunk.pos() {
                return Err(StorageError::ChunkPositionMismatch {
                    expected: chunk.pos(),
                    actual,
                }
                .into());
            }
        }
        raw::ENTITY => {
            let entity = raw::decode_key(key)?;
            let bytes = reader
                .get(RawTable::Entity, key)?
                .ok_or_else(|| bad(raw::ENTITY, "streamed record disappeared"))?;
            engine::verify(reader, entity, bytes)?;
        }
        raw::LOCATOR => {
            let (world, uuid) = raw::decode_locator_key(key)?;
            let (dimension, chunk) = raw::decode_locator(value)?;
            let entity = PersistentEntityKey::new(world, dimension, uuid);
            let bytes = reader
                .get(RawTable::Entity, &raw::key_bytes(entity))?
                .ok_or_else(|| bad(raw::LOCATOR, "locator has no record"))?;
            if engine::verify(reader, entity, bytes)?.chunk.pos() != chunk {
                return Err(bad(raw::LOCATOR, "locator chunk disagrees"));
            }
        }
        raw::INDEX => {
            let (chunk, uuid) = raw::decode_index(key, value)?;
            let entity = PersistentEntityKey::new(chunk.world(), chunk.dimension(), uuid);
            let bytes = reader
                .get(RawTable::Entity, &raw::key_bytes(entity))?
                .ok_or_else(|| bad(raw::INDEX, "chunk index has no record"))?;
            if engine::verify(reader, entity, bytes)?.chunk != chunk {
                return Err(bad(raw::INDEX, "chunk index disagrees"));
            }
        }
        raw::EPOCH => {
            let chunk = raw::decode_chunk(key, raw::EPOCH)?;
            engine::require_identity(
                reader,
                GenerationKey::new(chunk.world(), chunk.dimension()),
                None,
            )?;
            raw::decode_epoch(value)?;
        }
        "ferrumc:player" => {
            if key.len() != 16 {
                return Err(bad(name, "invalid player UUID length"));
            }
            require_unscoped(descriptors)?;
            crate::codec::decode_player_record(value)?;
        }
        "ferrumc:plugin" => validate_plugin(key, value)?,
        _ => return Err(bad("validator", "unknown table dispatch")),
    }
    Ok(())
}

struct JournalValidation {
    last: Option<u64>,
    receipts: Option<ReceiptIndex>,
}
fn validate_journal_row(
    state: &mut JournalValidation,
    descriptors: usize,
    name: &'static str,
    key: &[u8],
    value: &[u8],
    path: &Path,
) -> Result<()> {
    match name {
        "ferrumc:mutation_log" => {
            require_unscoped(descriptors)?;
            let id = mutation_id_from_key(key)?;
            let expected = match state.last {
                Some(last) => last.checked_add(1),
                None => Some(0),
            };
            if expected != Some(id) {
                return Err(bad(name, "journal is not contiguous from zero"));
            }
            if crate::codec::decode_mutation_log_record(value)?.id() != id {
                return Err(bad(name, "record ID disagrees with journal key"));
            }
            state.last = Some(id);
        }
        "ferrumc:mutation_batch" => {
            require_unscoped(descriptors)?;
            let bytes: [u8; 16] = key
                .try_into()
                .map_err(|_| bad(name, "invalid journal batch UUID length"))?;
            let batch = JournalBatchId::from_bytes(bytes);
            let receipt = decode_journal_receipt(batch, value)?;
            if let Some(range) = receipt.id_range() {
                if state.last.is_none_or(|last| *range.end() > last) {
                    let id = state
                        .last
                        .and_then(|last| last.checked_add(1))
                        .map_or(*range.start(), |next| next.max(*range.start()));
                    return Err(StorageError::JournalReceiptMissingRecord {
                        batch_id: batch,
                        id,
                    }
                    .into());
                }
                if state.receipts.is_none() {
                    state.receipts = Some(ReceiptIndex::new(path)?);
                }
                if let Some(index) = &mut state.receipts {
                    index.insert(*range.start(), *range.end())?;
                }
            }
        }
        _ => return Err(bad("validator", "unknown journal table")),
    }
    Ok(())
}

fn require_unscoped(count: usize) -> Result<()> {
    if count == 0 {
        return Err(StorageError::MissingGenerationIdentity { key: None }.into());
    }
    Ok(())
}
fn validate_plugin(key: &[u8], value: &[u8]) -> Result<()> {
    let prefix: [u8; 4] = key
        .get(..4)
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| bad("ferrumc:plugin", "truncated plugin key"))?;
    let len = u32::from_be_bytes(prefix) as usize;
    if len > MAX_UUID_STORE_PLUGIN_ID_LEN {
        return Err(bad(
            "ferrumc:plugin",
            "plugin ID exceeds durable validation bound",
        ));
    }
    let end = 4 + len;
    let plugin = key
        .get(4..end)
        .ok_or_else(|| bad("ferrumc:plugin", "truncated plugin ID"))?;
    let storage_key = key
        .get(end..)
        .ok_or_else(|| bad("ferrumc:plugin", "missing plugin storage key"))?;
    if storage_key.len() > MAX_PLUGIN_KEY_LEN
        || std::str::from_utf8(plugin).is_err()
        || std::str::from_utf8(storage_key).is_err()
    {
        return Err(bad("ferrumc:plugin", "invalid plugin key length or UTF-8"));
    }
    if value.len() > MAX_PLUGIN_VALUE_LEN {
        return Err(StorageError::ValueTooLarge {
            len: value.len(),
            max: MAX_PLUGIN_VALUE_LEN,
        }
        .into());
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
}
impl FileIdentity {
    fn of(metadata: &fs::Metadata) -> Result<Self> {
        if !metadata.is_file() {
            return Err(bad(
                "private receipt index",
                "owned receipt index must be a regular file",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(Self {
                dev: metadata.dev(),
                ino: metadata.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = metadata;
            Err(ServerError::unsupported(
                "safe temporary receipt index requires Unix file identity",
            ))
        }
    }
}
/// A private disk range index with16MiB cache and identity-checked cleanup.
struct ReceiptIndex {
    txn: Option<redb::WriteTransaction>,
    db: Option<Database>,
    path: PathBuf,
    identity: FileIdentity,
}
impl ReceiptIndex {
    fn new(source: &Path) -> Result<Self> {
        #[cfg(not(unix))]
        {
            let _ = source;
            return Err(ServerError::unsupported(
                "safe temporary receipt index requires Unix file identity",
            ));
        }
        #[cfg(unix)]
        {
            let parent = source
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            for _ in 0..16 {
                let path = parent.join(format!(".ferrumc-uuid-receipts-{}.redb", Uuid::new_v4()));
                let file = match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(&path)
                {
                    Ok(file) => file,
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(backend_err(e)),
                };
                let mut index = Self {
                    txn: None,
                    db: None,
                    identity: FileIdentity::of(&file.metadata().map_err(backend_err)?)?,
                    path,
                };
                let db = Database::builder()
                    .set_cache_size(INDEX_CACHE_BYTES)
                    .create_file(file)
                    .map_err(backend_err)?;
                let txn = db.begin_write().map_err(backend_err)?;
                index.db = Some(db);
                index.txn = Some(txn);
                return Ok(index);
            }
            Err(bad(
                "private receipt index",
                "temporary filename collision bound exceeded",
            ))
        }
    }
    fn transaction(&self) -> Result<&redb::WriteTransaction> {
        self.txn
            .as_ref()
            .ok_or_else(|| bad("private receipt index", "missing owned transaction"))
    }
    fn insert(&mut self, first: u64, last: u64) -> Result<()> {
        let mut table = self
            .transaction()?
            .open_table(RECEIPT_INDEX)
            .map_err(backend_err)?;
        if table.get(first).map_err(backend_err)?.is_some() {
            return Err(bad(
                "ferrumc:mutation_batch",
                "overlapping journal receipt ranges",
            ));
        }
        table
            .insert(first, last.to_be_bytes().as_slice())
            .map_err(backend_err)?;
        Ok(())
    }
    fn validate_nonoverlapping(&self) -> Result<()> {
        let table = self
            .transaction()?
            .open_table(RECEIPT_INDEX)
            .map_err(backend_err)?;
        let mut previous = None;
        for row in table.iter().map_err(backend_err)? {
            let (first, last) = row.map_err(backend_err)?;
            let bytes: [u8; 8] = last
                .value()
                .try_into()
                .map_err(|_| bad("private receipt index", "malformed range value"))?;
            if previous.is_some_and(|end| first.value() <= end) {
                return Err(bad(
                    "ferrumc:mutation_batch",
                    "overlapping journal receipt ranges",
                ));
            }
            previous = Some(u64::from_be_bytes(bytes));
        }
        Ok(())
    }
}
impl Drop for ReceiptIndex {
    fn drop(&mut self) {
        drop(self.txn.take());
        drop(self.db.take());
        let owned = fs::symlink_metadata(&self.path)
            .is_ok_and(|m| m.is_file() && FileIdentity::of(&m).ok() == Some(self.identity));
        if !owned {
            tracing::warn!(path = %self.path.display(), "owned UUID receipt index removed/replaced; cleanup skipped");
            return;
        }
        if let Err(error) = fs::remove_file(&self.path) {
            tracing::warn!(path = %self.path.display(), %error, "cannot remove owned UUID receipt index");
        }
    }
}

#[cfg(test)]
mod tests {
    use ferrumc_core::{DimensionId, EntityId, EntityUuid, WorldId};
    use ferrumc_math::{BlockPos, ChunkPos};
    use ferrumc_world::{BlockStateId, GenerationIdentity, WorldSeed};

    use super::super::uuid_entities::{apply_plan, commit_plan};
    use super::*;
    use crate::{
        BlockMutationLogRecord, ChunkKey, EntityKey, EntityRecord, MutationActor, MutationLogCause,
        PersistentEntityRecord, PersistentEntityStore, PersistentEntityWrite, RedbStore,
        SchemaVersion, WorldStore,
    };

    fn identity() -> GenerationIdentity {
        GenerationIdentity::flat_overworld(WorldSeed::new(0))
    }
    fn key(dimension: u32, uuid: u8) -> PersistentEntityKey {
        PersistentEntityKey::new(
            WorldId::new(0),
            DimensionId::new(dimension),
            EntityUuid::from_bytes([uuid; 16]),
        )
    }
    fn proposal(dimension: u32, uuid: u8, x: i32, body: u8) -> PersistentEntityWrite {
        PersistentEntityWrite::new(
            key(dimension, uuid),
            identity(),
            PersistentEntityRecord::new(
                ChunkPos::new(x, 0),
                EntityRecord::new(SchemaVersion::new(0), vec![body]).unwrap(),
            )
            .unwrap(),
        )
    }
    fn log(id: u64) -> BlockMutationLogRecord {
        BlockMutationLogRecord::new(
            SchemaVersion::new(0),
            id,
            1,
            MutationActor::System,
            BlockPos::new(0, 0, 0),
            BlockStateId::new(0),
            BlockStateId::new(1),
            MutationLogCause::Test,
        )
    }
    async fn setup(path: &Path) -> RedbStore {
        let store = RedbStore::open_with_uuid_entities(path).unwrap();
        for dimension in 0..2 {
            store
                .ensure_generation_identity(
                    GenerationKey::new(WorldId::new(0), DimensionId::new(dimension)),
                    identity(),
                )
                .await
                .unwrap();
        }
        store
            .save_persistent_entity(proposal(0, 1, 0, 1))
            .await
            .unwrap();
        store
    }
    type Snapshot = Vec<(String, Vec<u8>, Vec<u8>)>;
    fn snapshot(db: &Database) -> Snapshot {
        let txn = db.begin_read().unwrap();
        let mut output = Vec::new();
        for entry in txn.open_table(META_TABLE).unwrap().iter().unwrap() {
            let (key, value) = entry.unwrap();
            output.push((
                META_TABLE.name().into(),
                key.value().as_bytes().to_vec(),
                value.value().to_be_bytes().to_vec(),
            ));
        }
        for definition in TABLES {
            for row in txn.open_table(definition).unwrap().iter().unwrap() {
                let (key, value) = row.unwrap();
                output.push((
                    definition.name().into(),
                    key.value().to_vec(),
                    value.value().to_vec(),
                ));
            }
        }
        output
    }
    fn raw_replace(
        txn: &redb::WriteTransaction,
        definition: TableDefinition<'_, &[u8], &[u8]>,
        key: &[u8],
        value: &[u8],
    ) {
        txn.open_table(definition)
            .unwrap()
            .insert(key, value)
            .unwrap();
    }
    #[tokio::test]
    async fn actual_producer_transaction_cutpoints_leave_complete_raw_snapshot_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("p.redb");
        let store = setup(&path).await;
        let before = snapshot(&store.db);
        let mut count = 0;
        {
            let txn = store.db.begin_write().unwrap();
            let plan = engine::move_plan(
                &write_reader(&txn).unwrap(),
                key(0, 1),
                identity(),
                engine::prepare(vec![proposal(1, 1, 2, 2)])
                    .unwrap()
                    .pop()
                    .unwrap(),
            )
            .unwrap();
            apply_plan(&txn, plan, || {
                count += 1;
                Ok(())
            })
            .unwrap();
            // Drop rather than commit this count-only transaction.
        }
        assert_eq!(count, 8);
        for cut in 1..=count {
            {
                let txn = store.db.begin_write().unwrap();
                let plan = engine::move_plan(
                    &write_reader(&txn).unwrap(),
                    key(0, 1),
                    identity(),
                    engine::prepare(vec![proposal(1, 1, 2, 2)])
                        .unwrap()
                        .pop()
                        .unwrap(),
                )
                .unwrap();
                let mut writes = 0;
                assert!(apply_plan(&txn, plan, || {
                    writes += 1;
                    if writes == cut {
                        Err(raw::corrupt("test cutpoint", "returned-error abort"))
                    } else {
                        Ok(())
                    }
                })
                .is_err());
            }
            assert_eq!(snapshot(&store.db), before, "cutpoint {cut}");
            let txn = store.db.begin_write().unwrap();
            scan(&txn, &path).unwrap();
        }
        drop(store);
        let reopened = RedbStore::open_with_uuid_entities(&path).unwrap();
        assert_eq!(snapshot(&reopened.db), before);
    }
    #[tokio::test]
    async fn max_epoch_and_lost_reply_behaviors_are_not_claimed_as_ownership_cas() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("p.redb");
        let store = setup(&path).await;
        let chunk = ChunkKey::new(WorldId::new(0), DimensionId::new(0), ChunkPos::new(0, 0));
        {
            let txn = store.db.begin_write().unwrap();
            raw_replace(
                &txn,
                EPOCH_TABLE,
                &crate::codec::chunk_key_bytes(chunk),
                &u64::MAX.to_be_bytes(),
            );
            txn.commit().unwrap();
        }
        let before = snapshot(&store.db);
        store
            .save_persistent_entity(proposal(0, 1, 0, 1))
            .await
            .unwrap();
        assert!(store
            .save_persistent_entities(vec![proposal(0, 2, 2, 2), proposal(0, 1, 0, 2)])
            .await
            .is_err());
        assert!(store
            .delete_persistent_entity(key(0, 1), identity())
            .await
            .is_err());
        assert_eq!(snapshot(&store.db), before);
        {
            let txn = store.db.begin_write().unwrap();
            raw_replace(
                &txn,
                EPOCH_TABLE,
                &crate::codec::chunk_key_bytes(chunk),
                &1u64.to_be_bytes(),
            );
            txn.commit().unwrap();
        }
        // The actual producer commits, then its reply cutpoint reports failure.
        let before_move = snapshot(&store.db);
        {
            let txn = store.db.begin_write().unwrap();
            let plan = engine::move_plan(
                &write_reader(&txn).unwrap(),
                key(0, 1),
                identity(),
                engine::prepare(vec![proposal(1, 1, 2, 2)])
                    .unwrap()
                    .pop()
                    .unwrap(),
            )
            .unwrap();
            assert!(commit_plan(txn, plan, || Err(StorageError::backend(
                "injected lost reply after committed success"
            )))
            .is_err());
        }
        let after = snapshot(&store.db);
        assert_ne!(after, before_move);
        assert!(store
            .move_persistent_entity(key(0, 1), identity(), proposal(1, 1, 2, 2))
            .await
            .is_err());
        assert_eq!(snapshot(&store.db), after);
        // A stable identical save is currently a no-op, without a durable operation receipt.
        store
            .save_persistent_entity(proposal(1, 1, 2, 2))
            .await
            .unwrap();
        assert_eq!(snapshot(&store.db), after);
    }
    #[tokio::test]
    #[allow(clippy::too_many_lines)] // Keep the complete12-table negative matrix visible together.
    async fn all_twelve_table_grammars_and_reverse_relations_are_strictly_validated() {
        for case in 0..25 {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("p.redb");
            let store = setup(&path).await;
            {
                let txn = store.db.begin_write().unwrap();
                let entity = key(0, 1);
                let column = ChunkKey::new(entity.world(), entity.dimension(), ChunkPos::new(0, 0));
                match case {
                    0 => raw_replace(&txn, GENERATION_TABLE, &[0], &[0]),
                    1 => raw_replace(
                        &txn,
                        GENERATION_TABLE,
                        &engine::scope(entity).encode(),
                        &[0],
                    ),
                    2 => raw_replace(&txn, CHUNK_TABLE, &[0], &[0]),
                    3 => raw_replace(
                        &txn,
                        CHUNK_OVERLAY_TABLE,
                        &crate::codec::chunk_key_bytes(column),
                        &[0],
                    ),
                    4 => raw_replace(&txn, ENTITY_TABLE, &[0], &[0]),
                    5 => {
                        let mut bytes = raw::encode(proposal(0, 1, 0, 1).record());
                        bytes.push(0);
                        raw_replace(&txn, ENTITY_TABLE, &raw::key_bytes(entity), &bytes);
                    }
                    6 => {
                        txn.open_table(LOCATOR_TABLE)
                            .unwrap()
                            .remove(raw::locator_key(entity).as_slice())
                            .unwrap();
                    }
                    7 => raw_replace(
                        &txn,
                        LOCATOR_TABLE,
                        &raw::locator_key(key(0, 2)),
                        &raw::locator_value(key(0, 2), column.pos()),
                    ),
                    8 => raw_replace(&txn, LOCATOR_TABLE, &[0; 20], &[0]),
                    9 => raw_replace(
                        &txn,
                        INDEX_TABLE,
                        &raw::index_key(key(0, 2), column.pos()),
                        &[],
                    ),
                    10 => raw_replace(
                        &txn,
                        INDEX_TABLE,
                        &raw::index_key(entity, column.pos()),
                        &[0],
                    ),
                    11 => raw_replace(
                        &txn,
                        INDEX_TABLE,
                        &crate::codec::chunk_key_bytes(column),
                        &[],
                    ),
                    12 => raw_replace(
                        &txn,
                        EPOCH_TABLE,
                        &crate::codec::chunk_key_bytes(column),
                        &[0; 8],
                    ),
                    13 => {
                        txn.open_table(EPOCH_TABLE)
                            .unwrap()
                            .remove(crate::codec::chunk_key_bytes(column).as_slice())
                            .unwrap();
                    }
                    14 => raw_replace(
                        &txn,
                        EPOCH_TABLE,
                        &crate::codec::chunk_key_bytes(ChunkKey::new(
                            WorldId::new(7),
                            entity.dimension(),
                            column.pos(),
                        )),
                        &1u64.to_be_bytes(),
                    ),
                    15 => raw_replace(&txn, PLAYER_TABLE, &[0; 17], &[0]),
                    16 => raw_replace(&txn, PLUGIN_TABLE, &[0; 3], &[]),
                    17 => raw_replace(
                        &txn,
                        PLUGIN_TABLE,
                        &[0; 4],
                        &vec![0; MAX_PLUGIN_VALUE_LEN + 1],
                    ),
                    18 => raw_replace(
                        &txn,
                        MUTATION_LOG_TABLE,
                        &1u64.to_be_bytes(),
                        &crate::codec::encode_mutation_log_record(&log(1)),
                    ),
                    19 => raw_replace(
                        &txn,
                        MUTATION_LOG_TABLE,
                        &0u64.to_be_bytes(),
                        &crate::codec::encode_mutation_log_record(&log(7)),
                    ),
                    20 => {
                        txn.open_table(META_TABLE)
                            .unwrap()
                            .insert(META_LAST_MUTATION_ID_KEY, 0)
                            .unwrap();
                    }
                    21 => raw_replace(&txn, MUTATION_BATCH_TABLE, &[1; 16], &[0]),
                    22 => {
                        let mut value = [0; 16];
                        value[8..].copy_from_slice(&1u64.to_be_bytes());
                        raw_replace(&txn, MUTATION_BATCH_TABLE, &[1; 16], &value);
                    }
                    23 => {
                        txn.open_table(META_TABLE)
                            .unwrap()
                            .insert("unknown_metadata", 1)
                            .unwrap();
                    }
                    24 => {
                        txn.open_table(INDEX_TABLE)
                            .unwrap()
                            .remove(raw::index_key(entity, column.pos()).as_slice())
                            .unwrap();
                    }
                    _ => unreachable!(),
                }
                txn.commit().unwrap();
            }
            let before = snapshot(&store.db);
            drop(store);
            assert!(
                RedbStore::open_with_uuid_entities(&path).is_err(),
                "corruption case {case}"
            );
            let db = Database::open(&path).unwrap();
            assert_eq!(snapshot(&db), before, "corruption case {case}");
        }
    }
    #[tokio::test]
    async fn journal_and_receipts_preserve_validation_replay_and_bounded_disk_cleanup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("p.redb");
        let store = setup(&path).await;
        for batch in [3, 1, 2] {
            let id = JournalBatchId::from_bytes([batch; 16]);
            let receipt = store
                .append_block_mutation_batch(id, vec![log(99), log(100)])
                .await
                .unwrap();
            assert_eq!(
                store
                    .append_block_mutation_batch(id, vec![log(77), log(88)])
                    .await
                    .unwrap(),
                receipt
            );
        }
        let before = snapshot(&store.db);
        let report = scan(&store.db.begin_write().unwrap(), &path).unwrap();
        assert_eq!(report.version, 5);
        assert_eq!(report.counts.len(), 12);
        assert_eq!(report.counts[10], 6);
        assert_eq!(report.counts[11], 3);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        drop(store);
        let reopened = RedbStore::open_with_uuid_entities(&path).unwrap();
        assert_eq!(snapshot(&reopened.db), before);
        {
            let txn = reopened.db.begin_write().unwrap();
            let mut receipt = [0; 16];
            receipt[..8].copy_from_slice(&1u64.to_be_bytes());
            receipt[8..].copy_from_slice(&2u64.to_be_bytes());
            raw_replace(&txn, MUTATION_BATCH_TABLE, &[7; 16], &receipt);
            txn.commit().unwrap();
        }
        let before = snapshot(&reopened.db);
        drop(reopened);
        assert!(RedbStore::open_with_uuid_entities(&path).is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        let db = Database::open(&path).unwrap();
        assert_eq!(snapshot(&db), before);
    }
    #[tokio::test]
    async fn strict_opener_rejects_old_formats_and_retains_all_original_raw_rows_and_catalog() {
        for version in [3, 4] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("p.redb");
            let old = if version == 3 {
                RedbStore::open(&path).unwrap()
            } else {
                RedbStore::open_with_generation_metadata(&path).unwrap()
            };
            if version == 4 {
                old.ensure_generation_identity(
                    GenerationKey::new(WorldId::new(0), DimensionId::new(0)),
                    identity(),
                )
                .await
                .unwrap();
            }
            old.save_entity(
                EntityKey::new(WorldId::new(0), DimensionId::new(0), EntityId::new(9)),
                EntityRecord::new(SchemaVersion::new(7), vec![1, 2, 3]).unwrap(),
            )
            .await
            .unwrap();
            let all_before = available_snapshot(&old.db);
            let before = {
                let txn = old.db.begin_read().unwrap();
                let names: Vec<_> = txn
                    .list_tables()
                    .unwrap()
                    .map(|t| t.name().to_owned())
                    .collect();
                let value = txn
                    .open_table(ENTITY_TABLE)
                    .unwrap()
                    .get(
                        crate::codec::entity_key_bytes(EntityKey::new(
                            WorldId::new(0),
                            DimensionId::new(0),
                            EntityId::new(9),
                        ))
                        .as_slice(),
                    )
                    .unwrap()
                    .unwrap()
                    .value()
                    .to_vec();
                (names, value)
            };
            drop(old);
            assert!(
                matches!(RedbStore::open_with_uuid_entities(&path), Err(ServerError::InvalidState(message)) if message.contains(&format!("format {version} to 5")))
            );
            let db = Database::open(&path).unwrap();
            assert_eq!(available_snapshot(&db), all_before);
            let txn = db.begin_read().unwrap();
            assert_eq!(
                txn.list_tables()
                    .unwrap()
                    .map(|t| t.name().to_owned())
                    .collect::<Vec<_>>(),
                before.0
            );
            assert_eq!(
                txn.open_table(ENTITY_TABLE)
                    .unwrap()
                    .iter()
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .1
                    .value(),
                before.1
            );
            assert_eq!(
                txn.open_table(META_TABLE)
                    .unwrap()
                    .get(META_FORMAT_KEY)
                    .unwrap()
                    .unwrap()
                    .value(),
                version
            );
        }
    }
    fn available_snapshot(db: &Database) -> Snapshot {
        let txn = db.begin_read().unwrap();
        let mut output = Vec::new();
        for definition in txn.list_tables().unwrap() {
            let name = definition.name();
            if name == META_TABLE.name() {
                for entry in txn.open_table(META_TABLE).unwrap().iter().unwrap() {
                    let (key, value) = entry.unwrap();
                    output.push((
                        name.into(),
                        key.value().as_bytes().to_vec(),
                        value.value().to_be_bytes().to_vec(),
                    ));
                }
            } else {
                for entry in txn
                    .open_table(TableDefinition::<&[u8], &[u8]>::new(name))
                    .unwrap()
                    .iter()
                    .unwrap()
                {
                    let (key, value) = entry.unwrap();
                    output.push((name.into(), key.value().to_vec(), value.value().to_vec()));
                }
            }
        }
        output
    }
    #[tokio::test]
    async fn exact_catalog_multimaps_table_types_and_descriptor_caps_refuse_without_normalizing() {
        for case in 0..5 {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("p.redb");
            let store = setup(&path).await;
            {
                let txn = store.db.begin_write().unwrap();
                match case {
                    0 => {
                        txn.delete_table(EPOCH_TABLE).unwrap();
                    }
                    1 => {
                        raw_replace(&txn, TableDefinition::new("unknown_table"), &[1], &[2]);
                    }
                    2 => {
                        txn.open_multimap_table(redb::MultimapTableDefinition::<u64, u64>::new(
                            "unknown_multimap",
                        ))
                        .unwrap()
                        .insert(1, 2)
                        .unwrap();
                    }
                    3 => {
                        txn.delete_table(GENERATION_TABLE).unwrap();
                        txn.open_table(TableDefinition::<&str, u64>::new("ferrumc:generation"))
                            .unwrap()
                            .insert("wrong_type", 7)
                            .unwrap();
                    }
                    4 => {
                        let mut table = txn.open_table(GENERATION_TABLE).unwrap();
                        for world in 1..=crate::MAX_GENERATION_IDENTITIES as u32 {
                            table
                                .insert(
                                    GenerationKey::new(WorldId::new(world), DimensionId::new(0))
                                        .encode()
                                        .as_slice(),
                                    identity().encode().as_slice(),
                                )
                                .unwrap();
                        }
                    }
                    _ => unreachable!(),
                }
                txn.commit().unwrap();
            }
            let before = if case == 3 {
                None
            } else {
                Some(available_snapshot(&store.db))
            };
            let names = store
                .db
                .begin_read()
                .unwrap()
                .list_tables()
                .unwrap()
                .map(|table| table.name().to_owned())
                .collect::<Vec<_>>();
            drop(store);
            assert!(
                RedbStore::open_with_uuid_entities(&path).is_err(),
                "catalog case {case}"
            );
            let db = Database::open(&path).unwrap();
            assert_eq!(
                db.begin_read()
                    .unwrap()
                    .list_tables()
                    .unwrap()
                    .map(|table| table.name().to_owned())
                    .collect::<Vec<_>>(),
                names
            );
            if let Some(before) = before {
                assert_eq!(available_snapshot(&db), before);
            }
            if case == 2 {
                let txn = db.begin_read().unwrap();
                let multimap = txn
                    .open_multimap_table(redb::MultimapTableDefinition::<u64, u64>::new(
                        "unknown_multimap",
                    ))
                    .unwrap();
                assert_eq!(multimap.get(1).unwrap().next().unwrap().unwrap().value(), 2);
            }
            if case == 3 {
                assert_eq!(
                    db.begin_read()
                        .unwrap()
                        .open_table(TableDefinition::<&str, u64>::new("ferrumc:generation"))
                        .unwrap()
                        .get("wrong_type")
                        .unwrap()
                        .unwrap()
                        .value(),
                    7
                );
            }
        }
    }
    #[tokio::test]
    async fn empty_historical_chunk_epoch_is_valid_at_reopen_and_blocks_fresh_generation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("p.redb");
        let store = setup(&path).await;
        store
            .delete_persistent_entity(key(0, 1), identity())
            .await
            .unwrap();
        let before = snapshot(&store.db);
        drop(store);
        let store = RedbStore::open_with_uuid_entities(&path).unwrap();
        assert_eq!(snapshot(&store.db), before);
        assert!(store
            .ensure_generation_identity(
                GenerationKey::new(WorldId::new(7), DimensionId::new(7)),
                identity()
            )
            .await
            .is_err());
        assert_eq!(snapshot(&store.db), before);
    }
    #[test]
    fn unscoped_player_or_empty_receipt_requires_an_actual_descriptor() {
        for definition in [PLAYER_TABLE, MUTATION_BATCH_TABLE] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("p.redb");
            let store = RedbStore::open_with_uuid_entities(&path).unwrap();
            let txn = store.db.begin_write().unwrap();
            let body = if definition.name() == PLAYER_TABLE.name() {
                crate::codec::encode_player_record(
                    &crate::PlayerRecord::new(
                        SchemaVersion::new(0),
                        ferrumc_core::GameMode::Survival,
                        vec![],
                    )
                    .unwrap(),
                )
            } else {
                vec![0; 16]
            };
            raw_replace(&txn, definition, &[0; 16], &body);
            txn.commit().unwrap();
            let before = snapshot(&store.db);
            drop(store);
            assert!(matches!(
                RedbStore::open_with_uuid_entities(&path),
                Err(ServerError::InvalidState(_))
            ));
            let db = Database::open(&path).unwrap();
            assert_eq!(snapshot(&db), before);
        }
    }
    #[test]
    fn receipt_index_counter_fingerprint_and_file_identity_cleanup_are_bounded() {
        let mut fingerprint = Fingerprint::new();
        fingerprint.counts[0] = u64::MAX;
        assert!(fingerprint.row(0, &[], &[]).is_err());
        let mut fingerprint = Fingerprint::new();
        fingerprint.logical_bytes = u64::MAX;
        assert!(fingerprint.row(0, &[0], &[]).is_err());
        #[cfg(unix)]
        {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("source.redb");
            let index = ReceiptIndex::new(&source).unwrap();
            let path = index.path.clone();
            let moved = directory.path().join("moved");
            fs::rename(&path, &moved).unwrap();
            fs::write(&path, b"replacement belongs to someone else").unwrap();
            drop(index);
            assert_eq!(
                fs::read(&path).unwrap(),
                b"replacement belongs to someone else"
            );
            assert!(moved.exists());
            assert!(!source.exists());
        }
    }
}
