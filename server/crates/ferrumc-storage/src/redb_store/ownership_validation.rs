//! Separate bounded Format6 startup grammar; never disguises an ownership store as Format5.

use std::path::Path;

use redb::{Database, ReadableTable, TableDefinition, TableHandle};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::ownership::{
    self, backend, ENTITY_OWNER, PLAYER_OWNER, RECEIPT, RECEIPT_COUNT, RECEIPT_LIMIT,
    REQUEST_LIMIT, REVISION_COUNT, REVISION_LIMIT,
};
use super::{
    uuid_entities, StoreOrigin, CHUNK_OVERLAY_TABLE, CHUNK_TABLE, ENTITY_TABLE, GENERATION_TABLE,
    META_FORMAT_KEY, META_LAST_MUTATION_ID_KEY, META_TABLE, MUTATION_BATCH_TABLE,
    MUTATION_LOG_TABLE, PLAYER_TABLE, PLUGIN_TABLE,
};
use crate::ownership::{engine, raw};
use crate::persistent_entity::engine::EntityRead;
use crate::persistent_entity::{engine as uuid_engine, raw as uuid_raw};
use crate::{
    GenerationKey, OwnershipConfig, OwnershipError, OwnershipOpenLimits, OwnershipResult,
    PersistentEntityKey, ScopedPlayerKey, StorageError, MAX_GENERATION_IDENTITIES,
    MAX_PLUGIN_KEY_LEN, MAX_PLUGIN_VALUE_LEN, MAX_UUID_STORE_PLUGIN_ID_LEN,
};

pub(super) const TABLES: [TableDefinition<'_, &[u8], &[u8]>; 14] = [
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
    PLAYER_OWNER,
    ENTITY_OWNER,
    RECEIPT,
];
pub(super) const TABLE_NAMES: [&str; 14] = [
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
    raw::PLAYER_OWNER,
    raw::ENTITY_OWNER,
    raw::RECEIPT,
];

pub(super) fn initialize(
    db: &Database,
    origin: StoreOrigin,
    path: &Path,
    config: OwnershipConfig,
) -> OwnershipResult<()> {
    let txn = db.begin_write().map_err(backend)?;
    match origin {
        StoreOrigin::Fresh => {
            {
                let mut meta = txn.open_table(META_TABLE).map_err(backend)?;
                for (key, value) in [
                    (META_FORMAT_KEY, 6),
                    (RECEIPT_COUNT, 0),
                    (REVISION_COUNT, 0),
                    (REQUEST_LIMIT, config.capacity().max_request_bytes() as u64),
                    (RECEIPT_LIMIT, config.capacity().max_receipts()),
                    (REVISION_LIMIT, config.capacity().max_revision_rows()),
                ] {
                    meta.insert(key, value).map_err(backend)?;
                }
            }
            for table in TABLES {
                txn.open_table(table).map_err(backend)?;
            }
            scan(&txn, path, config)?;
            txn.commit().map_err(backend)?;
        }
        StoreOrigin::Existing => {
            if !txn
                .list_tables()
                .map_err(backend)?
                .any(|t| t.name() == META_TABLE.name())
            {
                return Err(OwnershipError::CatalogMismatch("missing metadata table"));
            }
            let actual = {
                let meta = txn
                    .open_table(META_TABLE)
                    .map_err(|_| OwnershipError::CatalogMismatch("wrong metadata table type"))?;
                let actual = meta
                    .get(META_FORMAT_KEY)
                    .map_err(backend)?
                    .map(|v| v.value());
                actual
            };
            match actual {
                Some(actual @ 3..=5) => return Err(OwnershipError::MigrationRequired { actual }),
                Some(6) => {}
                _ => return Err(OwnershipError::CatalogMismatch("unknown/missing format")),
            }
            scan(&txn, path, config)?;
        }
    }
    Ok(())
}
fn catalog(txn: &redb::WriteTransaction) -> OwnershipResult<()> {
    let mut count = 0usize;
    for table in txn.list_tables().map_err(backend)? {
        count += 1;
        if count > 15
            || (table.name() != META_TABLE.name()
                && !TABLES.iter().any(|t| t.name() == table.name()))
        {
            return Err(OwnershipError::CatalogMismatch("unknown/extra table"));
        }
    }
    if count != 15
        || txn
            .list_multimap_tables()
            .map_err(backend)?
            .next()
            .is_some()
    {
        return Err(OwnershipError::CatalogMismatch(
            "missing table/unexpected multimap",
        ));
    }
    txn.open_table(META_TABLE)
        .map_err(|_| OwnershipError::CatalogMismatch("wrong metadata type"))?;
    for definition in TABLES {
        txn.open_table(definition)
            .map_err(|_| OwnershipError::CatalogMismatch("wrong data table type"))?;
    }
    Ok(())
}
struct Budget {
    rows: u64,
    bytes: u64,
    limits: OwnershipOpenLimits,
    hash: Sha256,
    counts: [u64; 15],
}
impl Budget {
    fn new(limits: OwnershipOpenLimits) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"FerrumC ownership storage logical snapshot v1\0");
        Self {
            rows: 0,
            bytes: 0,
            limits,
            hash,
            counts: [0; 15],
        }
    }
    fn table(&mut self, name: &str) {
        self.hash.update(b"table\0");
        self.hash.update((name.len() as u64).to_be_bytes());
        self.hash.update(name.as_bytes());
    }
    fn row(&mut self, slot: usize, key: &[u8], value: &[u8]) -> OwnershipResult<()> {
        let rows = self
            .rows
            .checked_add(1)
            .ok_or(OwnershipError::ScanCapacity("rows"))?;
        let bytes = self
            .bytes
            .checked_add(key.len() as u64)
            .and_then(|n| n.checked_add(value.len() as u64))
            .ok_or(OwnershipError::ScanCapacity("bytes"))?;
        if rows > self.limits.max_scan_rows() {
            return Err(OwnershipError::ScanCapacity("rows"));
        }
        if bytes > self.limits.max_scan_bytes() {
            return Err(OwnershipError::ScanCapacity("bytes"));
        }
        self.rows = rows;
        self.bytes = bytes;
        self.counts[slot] += 1;
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
fn journal_error(e: super::uuid_migration::UuidEntityMigrationError) -> OwnershipError {
    use super::uuid_migration::UuidEntityMigrationError as E;
    match e {
        E::ScratchBudgetExceeded { .. } => OwnershipError::ScratchCapacity,
        E::Record { source, .. } => OwnershipError::Record(*source),
        E::InvalidRow { table, reason } => raw::bad(table, reason),
        E::UnsupportedPlatform => {
            OwnershipError::Backend("journal scratch requires Unix file identity".into())
        }
        other => backend(other),
    }
}
pub(super) fn common_row(
    reader: &impl EntityRead,
    name: &'static str,
    key: &[u8],
    value: &[u8],
) -> OwnershipResult<()> {
    match name {
        "ferrumc:chunk" | "ferrumc:chunk_overlay" => {
            let chunk = uuid_raw::decode_chunk(key, name)?;
            uuid_engine::require_identity(
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
        uuid_raw::ENTITY => {
            let entity = uuid_raw::decode_key(key)?;
            let bytes = reader
                .get(crate::persistent_entity::engine::RawTable::Entity, key)?
                .ok_or_else(|| raw::bad(name, "streamed entity disappeared"))?;
            uuid_engine::verify(reader, entity, bytes)?;
        }
        uuid_raw::LOCATOR => {
            let (world, uuid) = uuid_raw::decode_locator_key(key)?;
            let (dimension, chunk) = uuid_raw::decode_locator(value)?;
            let entity = PersistentEntityKey::new(world, dimension, uuid);
            let bytes = reader
                .get(
                    crate::persistent_entity::engine::RawTable::Entity,
                    &uuid_raw::key_bytes(entity),
                )?
                .ok_or_else(|| raw::bad(name, "locator has no entity"))?;
            if uuid_engine::verify(reader, entity, bytes)?.chunk.pos() != chunk {
                return Err(raw::bad(name, "locator chunk mismatch"));
            }
        }
        uuid_raw::INDEX => {
            let (chunk, uuid) = uuid_raw::decode_index(key, value)?;
            let entity = PersistentEntityKey::new(chunk.world(), chunk.dimension(), uuid);
            let bytes = reader
                .get(
                    crate::persistent_entity::engine::RawTable::Entity,
                    &uuid_raw::key_bytes(entity),
                )?
                .ok_or_else(|| raw::bad(name, "index has no entity"))?;
            if uuid_engine::verify(reader, entity, bytes)?.chunk != chunk {
                return Err(raw::bad(name, "index chunk mismatch"));
            }
        }
        uuid_raw::EPOCH => {
            let chunk = uuid_raw::decode_chunk(key, name)?;
            uuid_engine::require_identity(
                reader,
                GenerationKey::new(chunk.world(), chunk.dimension()),
                None,
            )?;
            uuid_raw::decode_epoch(value)?;
        }
        "ferrumc:plugin" => {
            let prefix = key
                .get(..4)
                .ok_or_else(|| raw::bad(name, "truncated plugin key"))?;
            let len = u32::from_be_bytes(raw::array(prefix, name)?) as usize;
            if len > MAX_UUID_STORE_PLUGIN_ID_LEN {
                return Err(raw::bad(name, "plugin ID exceeds durable bound"));
            }
            let end = 4 + len;
            let plugin = key
                .get(4..end)
                .ok_or_else(|| raw::bad(name, "truncated plugin ID"))?;
            let storage_key = key
                .get(end..)
                .ok_or_else(|| raw::bad(name, "missing plugin storage key"))?;
            if storage_key.len() > MAX_PLUGIN_KEY_LEN
                || std::str::from_utf8(plugin).is_err()
                || std::str::from_utf8(storage_key).is_err()
            {
                return Err(raw::bad(name, "invalid plugin key length/UTF-8"));
            }
            if value.len() > MAX_PLUGIN_VALUE_LEN {
                return Err(StorageError::ValueTooLarge {
                    len: value.len(),
                    max: MAX_PLUGIN_VALUE_LEN,
                }
                .into());
            }
        }
        _ => return Err(raw::bad("validator", "unknown common row dispatch")),
    }
    Ok(())
}
fn scan_metadata(
    reader: &ownership::WriteReader<'_>,
    budget: &mut Budget,
    config: OwnershipConfig,
) -> OwnershipResult<Option<u64>> {
    budget.table(META_TABLE.name());
    let mut watermark = None;
    let mut mandatory = 0u8;
    for row in reader.meta.iter().map_err(backend)? {
        let (key, value) = row.map_err(backend)?;
        let key = key.value();
        let value = value.value();
        budget.row(0, key.as_bytes(), &value.to_be_bytes())?;
        let (bit, expected) = match key {
            META_FORMAT_KEY => (0, Some(6)),
            RECEIPT_COUNT => (1, None),
            REVISION_COUNT => (2, None),
            REQUEST_LIMIT => (3, Some(config.capacity().max_request_bytes() as u64)),
            RECEIPT_LIMIT => (4, Some(config.capacity().max_receipts())),
            REVISION_LIMIT => (5, Some(config.capacity().max_revision_rows())),
            META_LAST_MUTATION_ID_KEY => {
                watermark = Some(value);
                continue;
            }
            _ => return Err(OwnershipError::CatalogMismatch("unknown metadata key")),
        };
        mandatory |= 1 << bit;
        if let Some(expected) = expected {
            if value != expected {
                return Err(if key == META_FORMAT_KEY {
                    OwnershipError::CatalogMismatch("incorrect format")
                } else {
                    OwnershipError::CapacityContractMismatch
                });
            }
        }
    }
    if mandatory != 63 {
        return Err(OwnershipError::CatalogMismatch(
            "missing mandatory ownership metadata",
        ));
    }
    budget.end(0);
    Ok(watermark)
}
pub(super) fn owned_player_row(
    reader: &impl engine::Read,
    name: &'static str,
    key: &[u8],
    value: &[u8],
) -> OwnershipResult<()> {
    let id = ferrumc_core::PlayerId::from_uuid(Uuid::from_bytes(raw::array(key, name)?));
    let header = if name == raw::PLAYER_OWNER {
        raw::player_header(value)?
    } else {
        let b = engine::Read::owned(reader, engine::Table::PlayerHeader, key)?
            .ok_or_else(|| raw::bad(raw::PLAYER_OWNER, "player row has no owner"))?;
        raw::player_header(b.as_ref())?
    };
    let expected =
        reader
            .identity(header.scope)?
            .ok_or(StorageError::MissingGenerationIdentity {
                key: Some(header.scope),
            })?;
    engine::player_state(reader, ScopedPlayerKey::new(header.scope, id), expected)?;
    Ok(())
}
pub(super) fn owned_entity_row(
    reader: &impl engine::Read,
    key: &[u8],
    value: &[u8],
) -> OwnershipResult<()> {
    let header = raw::entity_header(key, value)?;
    let (_, uuid) = crate::persistent_entity::raw::decode_locator_key(key)?;
    let expected =
        reader
            .identity(header.scope)?
            .ok_or(StorageError::MissingGenerationIdentity {
                key: Some(header.scope),
            })?;
    engine::entity_state(
        reader,
        PersistentEntityKey::new(header.scope.world(), header.scope.dimension(), uuid),
        expected,
    )?;
    Ok(())
}
fn scan_row(
    reader: &ownership::WriteReader<'_>,
    name: &'static str,
    key: &[u8],
    value: &[u8],
    descriptors: &mut usize,
    journal: &mut super::uuid_migration::validation::JournalValidation,
    path: &Path,
) -> OwnershipResult<()> {
    match name {
        "ferrumc:generation" => {
            let key = GenerationKey::decode(key)?;
            crate::generation::decode_identity(key, value)?;
            *descriptors += 1;
            if *descriptors > MAX_GENERATION_IDENTITIES {
                return Err(StorageError::GenerationIdentityLimit {
                    max: MAX_GENERATION_IDENTITIES,
                }
                .into());
            }
        }
        "ferrumc:player" | raw::PLAYER_OWNER => owned_player_row(reader, name, key, value)?,
        raw::ENTITY_OWNER => owned_entity_row(reader, key, value)?,
        raw::RECEIPT => {
            let r = raw::receipt(key, value)?;
            engine::receipt_relations(reader, &r)?;
        }
        "ferrumc:mutation_log" | "ferrumc:mutation_batch" => journal
            .row(*descriptors, name, key, value, path)
            .map_err(journal_error)?,
        _ => common_row(&engine::EntityAdapter(reader), name, key, value)?,
    }
    Ok(())
}
pub(super) fn scan(
    txn: &redb::WriteTransaction,
    path: &Path,
    config: OwnershipConfig,
) -> OwnershipResult<[u8; 32]> {
    catalog(txn)?;
    let reader = ownership::write_reader(txn, config.capacity())?;
    let mut budget = Budget::new(config.open_limits());
    let mut journal = super::uuid_migration::validation::JournalValidation::new(
        config.open_limits().max_journal_index_bytes(),
    );
    let result = (|| {
        let watermark = scan_metadata(&reader, &mut budget, config)?;
        let mut descriptors = 0usize;
        for (index, definition) in TABLES.into_iter().enumerate() {
            let slot = index + 1;
            let name = TABLE_NAMES[index];
            let owned;
            let table = match name {
                "ferrumc:generation" => &reader.uuid.generations,
                "ferrumc:entity" => &reader.uuid.entities,
                "ferrumc:entity_locator" => &reader.uuid.locators,
                "ferrumc:entity_chunk" => &reader.uuid.indexes,
                "ferrumc:entity_chunk_epoch" => &reader.uuid.epochs,
                "ferrumc:player" => &reader.players,
                raw::PLAYER_OWNER => &reader.player_headers,
                raw::ENTITY_OWNER => &reader.entity_headers,
                raw::RECEIPT => &reader.receipts,
                _ => {
                    owned = txn.open_table(definition).map_err(backend)?;
                    &owned
                }
            };
            budget.table(name);
            for row in table.iter().map_err(backend)? {
                let (key, value) = row.map_err(backend)?;
                let (key, value) = (key.value(), value.value());
                budget.row(slot, key, value)?;
                scan_row(
                    &reader,
                    name,
                    key,
                    value,
                    &mut descriptors,
                    &mut journal,
                    path,
                )?;
            }
            budget.end(slot);
        }
        journal.validate(watermark).map_err(journal_error)?;
        let (receipts, revisions) = engine::Read::counts(&reader)?;
        if receipts != budget.counts[14]
            || revisions
                != budget.counts[12]
                    .checked_add(budget.counts[13])
                    .ok_or(OwnershipError::ScanCapacity("counter overflow"))?
        {
            return Err(raw::bad("ferrumc:meta", "ownership count mismatch"));
        }
        if receipts > config.capacity().max_receipts()
            || revisions > config.capacity().max_revision_rows()
        {
            return Err(raw::bad(
                "ferrumc:meta",
                "ownership count exceeds persisted capacity",
            ));
        }
        tracing::debug!(rows=budget.rows,bytes=budget.bytes,counts=?budget.counts,"validated complete ownership storage snapshot");
        Ok(budget.hash.finalize().into())
    })();
    // Cleanup runs on success and failure; its own refusal is never silently discarded.
    match result {
        Ok(hash) => journal.finish(Ok(hash)).map_err(journal_error),
        Err(error) => {
            journal.finish(Ok(())).map_err(journal_error)?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ownership::tests::{setup, snapshot};
    use super::*;
    use crate::ownership::tests::{config, entity, id, identity, player, scope};
    use crate::{
        EntityOwnershipProposal, OwnershipCapacity, OwnershipResource, PlayerEntityOwnershipStore,
        PlayerEntityTransferRequest, PlayerOwnershipProposal, RedbStore, WorldStore,
        MAX_OWNERSHIP_REQUEST_BYTES,
    };
    use ferrumc_core::{DimensionId, EntityUuid, WorldId};
    fn bounded(rows: u64, bytes: u64, scratch: u64) -> OwnershipConfig {
        OwnershipConfig::new(
            config().capacity(),
            OwnershipOpenLimits::new(
                std::num::NonZeroU64::new(rows).unwrap(),
                std::num::NonZeroU64::new(bytes).unwrap(),
                std::num::NonZeroU64::new(scratch).unwrap(),
                std::num::NonZeroU16::new(8).unwrap(),
            )
            .unwrap(),
        )
    }
    #[tokio::test]
    async fn complete_raw_scan_admits_exact_rows_bytes_and_refuses_one_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("exact-scan.redb");
        let store = setup(&path).await;
        let raw = snapshot(&store);
        let rows = raw.values().map(|rows| rows.len() as u64).sum::<u64>();
        let bytes = raw
            .values()
            .flatten()
            .map(|(k, v)| (k.len() + v.len()) as u64)
            .sum::<u64>();
        drop(store);
        let exact = RedbStore::open_with_player_entity_ownership(
            &path,
            bounded(rows, bytes, 8 * 1024 * 1024),
        )
        .unwrap();
        assert_eq!(snapshot(&exact), raw);
        drop(exact);
        assert!(matches!(
            RedbStore::open_with_player_entity_ownership(
                &path,
                bounded(rows - 1, bytes, 8 * 1024 * 1024)
            ),
            Err(OwnershipError::ScanCapacity("rows"))
        ));
        assert!(matches!(
            RedbStore::open_with_player_entity_ownership(
                &path,
                bounded(rows, bytes - 1, 8 * 1024 * 1024)
            ),
            Err(OwnershipError::ScanCapacity("bytes"))
        ));
        let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
        assert_eq!(snapshot(&reopened), raw);
    }
    type Journal = super::super::uuid_migration::validation::JournalValidation;
    type RawSnapshot = std::collections::BTreeMap<String, RawRows>;
    type RawRows = Vec<(Vec<u8>, Vec<u8>)>;
    fn live_journal(path: &Path, rows: &RawSnapshot) -> Journal {
        let mut journal = Journal::new(8 * 1024 * 1024);
        for name in ["ferrumc:mutation_log", "ferrumc:mutation_batch"] {
            for (key, value) in &rows[name] {
                journal.row(1, name, key, value, path).unwrap();
            }
        }
        journal.validate(Some(0)).unwrap();
        journal
    }
    fn scratch(parent: &Path) -> std::path::PathBuf {
        let paths: Vec<_> = std::fs::read_dir(parent)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".ferrumc-uuid-migration-receipts-")
            })
            .collect();
        assert_eq!(paths.len(), 1);
        paths[0].clone()
    }
    #[tokio::test]
    async fn actual_bounded_journal_scratch_cleans_owned_files_and_preserves_replacements() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.redb");
        let store = setup(&path).await;
        let log = crate::BlockMutationLogRecord::new(
            crate::SchemaVersion::new(1),
            77,
            1,
            crate::MutationActor::System,
            ferrumc_math::BlockPos::new(0, 64, 0),
            ferrumc_world::BlockStateId::new(0),
            ferrumc_world::BlockStateId::new(1),
            crate::MutationLogCause::Test,
        );
        store
            .append_block_mutation_batch(crate::JournalBatchId::from_bytes([7; 16]), vec![log])
            .await
            .unwrap();
        let rows = snapshot(&store);
        let journal = live_journal(&path, &rows);
        let temporary = scratch(dir.path());
        // redb also flushes during close: retain only a read-only handle after unlink.
        let retained = std::fs::File::open(&temporary).unwrap();
        journal.finish(Ok(())).unwrap();
        let extent = retained.metadata().unwrap().len();
        assert!(extent > 1);
        drop(retained);
        assert!(!temporary.exists());
        drop(store);
        let exact = RedbStore::open_with_player_entity_ownership(
            &path,
            bounded(1000, 8 * 1024 * 1024, extent),
        )
        .unwrap();
        assert_eq!(snapshot(&exact), rows);
        drop(exact);
        assert!(matches!(
            RedbStore::open_with_player_entity_ownership(
                &path,
                bounded(1000, 8 * 1024 * 1024, extent - 1)
            ),
            Err(OwnershipError::ScratchCapacity)
        ));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        let journal = live_journal(&path, &rows);
        let temporary = scratch(dir.path());
        let moved = dir.path().join("retained-owned-scratch");
        std::fs::rename(&temporary, &moved).unwrap();
        std::fs::write(&temporary, b"foreign replacement").unwrap();
        assert!(journal.finish(Ok(())).is_err());
        assert_eq!(std::fs::read(&temporary).unwrap(), b"foreign replacement");
        assert!(moved.exists());
        let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
        assert_eq!(snapshot(&reopened), rows);
    }

    fn corrupt_missing_or_meta(txn: &redb::WriteTransaction, case: u8) {
        match case {
            0 => {
                txn.open_table(PLAYER_OWNER)
                    .unwrap()
                    .remove(raw::player_key(player()).as_slice())
                    .unwrap();
            }
            1 => {
                txn.open_table(ENTITY_OWNER)
                    .unwrap()
                    .remove(raw::entity_key(entity()).as_slice())
                    .unwrap();
            }
            2 => {
                txn.open_table(RECEIPT)
                    .unwrap()
                    .remove(id(1).to_bytes().as_slice())
                    .unwrap();
            }
            3 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert(RECEIPT_COUNT, 2)
                    .unwrap();
            }
            4 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert(REVISION_COUNT, 1)
                    .unwrap();
            }
            5 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert("unknown_ownership", 1)
                    .unwrap();
            }
            6 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .remove(REQUEST_LIMIT)
                    .unwrap();
            }
            7 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert(RECEIPT_LIMIT, 1001)
                    .unwrap();
            }
            8 => {
                txn.open_table(PLAYER_TABLE)
                    .unwrap()
                    .remove(raw::player_key(player()).as_slice())
                    .unwrap();
            }
            9 => {
                txn.open_table(ENTITY_TABLE)
                    .unwrap()
                    .remove(crate::persistent_entity::raw::key_bytes(entity()).as_slice())
                    .unwrap();
            }
            10 => {
                txn.open_table(uuid_entities::LOCATOR_TABLE)
                    .unwrap()
                    .remove(raw::entity_key(entity()).as_slice())
                    .unwrap();
            }
            11 => {
                txn.open_table(uuid_entities::INDEX_TABLE)
                    .unwrap()
                    .remove(
                        crate::persistent_entity::raw::index_key(
                            entity(),
                            ferrumc_math::ChunkPos::new(0, -2),
                        )
                        .as_slice(),
                    )
                    .unwrap();
            }
            12 => {
                txn.open_table(GENERATION_TABLE)
                    .unwrap()
                    .remove(scope().encode().as_slice())
                    .unwrap();
            }
            _ => unreachable!(),
        }
    }
    fn corrupt_bytes(txn: &redb::WriteTransaction, case: u8) {
        match case {
            13 => {
                let mut table = txn.open_table(PLAYER_OWNER).unwrap();
                let key = raw::player_key(player());
                let mut b = table.get(key.as_slice()).unwrap().unwrap().value().to_vec();
                b[6] = 2;
                table.insert(key.as_slice(), b.as_slice()).unwrap();
            }
            14 => {
                let mut table = txn.open_table(ENTITY_OWNER).unwrap();
                let key = raw::entity_key(entity());
                let mut b = table.get(key.as_slice()).unwrap().unwrap().value().to_vec();
                b[18] = 0;
                table.insert(key.as_slice(), b.as_slice()).unwrap();
            }
            15 => {
                let mut table = txn.open_table(RECEIPT).unwrap();
                let key = id(1).to_bytes();
                let mut b = table.get(key.as_slice()).unwrap().unwrap().value().to_vec();
                b[241] = 1;
                table.insert(key.as_slice(), b.as_slice()).unwrap();
            }
            16 => {
                let mut table = txn.open_table(ENTITY_OWNER).unwrap();
                let key = raw::entity_key(entity());
                let mut b = table.get(key.as_slice()).unwrap().unwrap().value().to_vec();
                b[10] = 3;
                table.insert(key.as_slice(), b.as_slice()).unwrap();
            }
            17 => {
                let mut table = txn.open_table(RECEIPT).unwrap();
                let key = id(1).to_bytes();
                let mut b = table.get(key.as_slice()).unwrap().unwrap().value().to_vec();
                b[7] ^= 1;
                table.insert(key.as_slice(), b.as_slice()).unwrap();
            }
            18 => {
                txn.open_table(RECEIPT)
                    .unwrap()
                    .insert([4; 15].as_slice(), [0; 256].as_slice())
                    .unwrap();
            }
            19 => {
                txn.open_table(PLAYER_TABLE)
                    .unwrap()
                    .insert([5; 15].as_slice(), [0; 5].as_slice())
                    .unwrap();
            }
            20 => {
                txn.open_table(ENTITY_OWNER)
                    .unwrap()
                    .insert([6; 19].as_slice(), [0; 35].as_slice())
                    .unwrap();
            }
            21 => {
                let mut table = txn.open_table(RECEIPT).unwrap();
                let key = id(1).to_bytes();
                let mut b = table.get(key.as_slice()).unwrap().unwrap().value().to_vec();
                b[190] = 2;
                table.insert(key.as_slice(), b.as_slice()).unwrap();
            }
            _ => unreachable!(),
        }
    }
    #[tokio::test]
    async fn complete_catalog_header_receipt_and_relation_scan_never_repairs() {
        let dir = tempfile::tempdir().unwrap();
        for case in 0..22 {
            let path = dir.path().join(format!("malformed-{case}.redb"));
            let store = setup(&path).await;
            let txn = store.db.begin_write().unwrap();
            if case < 13 {
                corrupt_missing_or_meta(&txn, case);
            } else {
                corrupt_bytes(&txn, case);
            }
            txn.commit().unwrap();
            let before = snapshot(&store);
            drop(store);
            assert!(
                RedbStore::open_with_player_entity_ownership(&path, config()).is_err(),
                "case{case}"
            );
            let raw_db = Database::open(&path).unwrap();
            let wrapper = RedbStore {
                db: std::sync::Arc::new(raw_db),
                generation_mode: crate::generation::GenerationMode::Ownership,
                entity_incarnation: std::sync::Arc::new(()),
                ownership_runtime: None,
                block_config: None,
            };
            assert_eq!(snapshot(&wrapper), before, "case{case}");
        }
    }
    #[test]
    fn old_formats_and_unknown_or_incomplete_catalog_require_explicit_admission() {
        let dir = tempfile::tempdir().unwrap();
        for (format, opener) in [(3, 0), (4, 1), (5, 2)] {
            let path = dir.path().join(format!("old-{format}.redb"));
            let store = match opener {
                0 => RedbStore::open(&path),
                1 => RedbStore::open_with_generation_metadata(&path),
                _ => RedbStore::open_with_uuid_entities(&path),
            }
            .unwrap();
            let before = snapshot(&store);
            drop(store);
            assert!(
                matches!(RedbStore::open_with_player_entity_ownership(&path,config()),Err(OwnershipError::MigrationRequired {actual}) if actual==format)
            );
            let store = match opener {
                0 => RedbStore::open(&path),
                1 => RedbStore::open_with_generation_metadata(&path),
                _ => RedbStore::open_with_uuid_entities(&path),
            }
            .unwrap();
            assert_eq!(snapshot(&store), before);
        }
        for case in 0..4 {
            let path = dir.path().join(format!("catalog-{case}.redb"));
            let store = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
            let txn = store.db.begin_write().unwrap();
            match case {
                0 => {
                    txn.delete_table(ENTITY_OWNER).unwrap();
                }
                1 => {
                    txn.open_table(TableDefinition::<&[u8], &[u8]>::new("unknown:table"))
                        .unwrap();
                }
                2 => {
                    txn.delete_table(ENTITY_OWNER).unwrap();
                    txn.open_table(TableDefinition::<&str, u64>::new(raw::ENTITY_OWNER))
                        .unwrap();
                }
                3 => {
                    txn.open_multimap_table(redb::MultimapTableDefinition::<&[u8], &[u8]>::new(
                        "unknown:multimap",
                    ))
                    .unwrap();
                }
                _ => unreachable!(),
            }
            txn.commit().unwrap();
            drop(store);
            assert!(
                matches!(
                    RedbStore::open_with_player_entity_ownership(&path, config()),
                    Err(OwnershipError::CatalogMismatch(_))
                ),
                "catalog{case}"
            );
        }
    }
    #[tokio::test]
    async fn persisted_capacity_and_bounded_scan_are_verified_without_reset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("limits.redb");
        let store = setup(&path).await;
        let before = snapshot(&store);
        drop(store);
        let changed = OwnershipConfig::new(
            OwnershipCapacity::new(
                std::num::NonZeroUsize::new(MAX_OWNERSHIP_REQUEST_BYTES).unwrap(),
                std::num::NonZeroU64::new(1001).unwrap(),
                std::num::NonZeroU64::new(100).unwrap(),
            )
            .unwrap(),
            config().open_limits(),
        );
        assert!(matches!(
            RedbStore::open_with_player_entity_ownership(&path, changed),
            Err(OwnershipError::CapacityContractMismatch)
        ));
        for byte_limit in [false, true] {
            let limits = OwnershipOpenLimits::new(
                std::num::NonZeroU64::new(if byte_limit { 1_000_000 } else { 1 }).unwrap(),
                std::num::NonZeroU64::new(if byte_limit { 1 } else { 128 * 1024 * 1024 }).unwrap(),
                std::num::NonZeroU64::new(8 * 1024 * 1024).unwrap(),
                std::num::NonZeroU16::new(8).unwrap(),
            )
            .unwrap();
            assert!(matches!(
                RedbStore::open_with_player_entity_ownership(
                    &path,
                    OwnershipConfig::new(config().capacity(), limits)
                ),
                Err(OwnershipError::ScanCapacity(_))
            ));
        }
        let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
        assert_eq!(snapshot(&reopened), before);
    }
    #[tokio::test]
    async fn global_player_and_world_uuid_scopes_never_select_another_descriptor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scope.redb");
        let store = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
        let other = GenerationKey::new(WorldId::new(4), DimensionId::new(3));
        store
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        store
            .ensure_generation_identity(other, identity())
            .await
            .unwrap();
        let pair = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        store
            .commit_player_entity_transfer(
                PlayerEntityTransferRequest::new(
                    id(1),
                    pair.player().expectation().clone(),
                    pair.entity().expectation().clone(),
                    PlayerOwnershipProposal::put(crate::ownership::tests::p(1)),
                    EntityOwnershipProposal::put(crate::ownership::tests::e(0, 1)),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let before = snapshot(&store);
        let other_player = ScopedPlayerKey::new(other, player().id());
        assert!(matches!(
            store.load_owned_player(other_player, identity()).await,
            Err(OwnershipError::ScopeConflict {
                resource: OwnershipResource::Player
            })
        ));
        let other_entity = PersistentEntityKey::new(
            other.world(),
            other.dimension(),
            EntityUuid::from_bytes(entity().uuid().to_bytes()),
        );
        assert!(matches!(
            store.load_owned_entity(other_entity, identity()).await,
            Err(OwnershipError::ScopeConflict {
                resource: OwnershipResource::Entity
            })
        ));
        assert_eq!(snapshot(&store), before);
        assert!(store
            .ensure_generation_identity(
                GenerationKey::new(WorldId::new(5), DimensionId::new(2)),
                identity()
            )
            .await
            .is_err());
    }
}
