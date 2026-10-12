//! Explicit streaming Format7 catalog and both receipt families; no normalization writes.
use super::uuid_migration::{
    files::{DiskBudget, TemporaryFile},
    UuidEntityMigrationError, UuidEntityMigrationScratchKind,
};
use super::{
    blocks::{
        agreement, block_backend, counts, reader, CHUNK_COUNT, CHUNK_LIMIT, HEADER, JOURNAL,
        JOURNAL_BYTES, JOURNAL_BYTE_LIMIT, JOURNAL_COUNT, JOURNAL_LIMIT, RECEIPT_BYTES,
        RECEIPT_LIMIT, REQUEST_LIMIT, WATERMARK,
    },
    ownership, ownership_validation as six, StoreOrigin, META_FORMAT_KEY,
    META_LAST_MUTATION_ID_KEY, META_TABLE,
};
use crate::block_durability::{
    engine::{self, Budget},
    raw, BlockDurabilityConfig, BlockDurabilityError, BlockResult,
};
use crate::ownership::{engine as owned, raw as owned_raw};
use crate::persistent_entity::engine::EntityRead;
use redb::{Database, Durability, ReadableTable, TableDefinition, TableHandle};
use std::{path::Path, sync::Arc};

// The shared owned-file machinery is reused; its UUID-migration wording is not.
fn scratch_error(e: UuidEntityMigrationError) -> BlockDurabilityError {
    use UuidEntityMigrationError as E;
    match e {
        E::ScratchBudgetExceeded { .. } => BlockDurabilityError::ScratchCapacity,
        E::UnsupportedPlatform => BlockDurabilityError::UnsupportedPlatform,
        E::Cleanup { path, source } => {
            BlockDurabilityError::ScratchCleanup(format!("{}: {source}", path.display()))
        }
        E::Io {
            operation,
            path,
            source,
        } => BlockDurabilityError::ScratchUnavailable(format!(
            "{operation} {}: {source}",
            path.display()
        )),
        E::Symlink(path) => BlockDurabilityError::ScratchUnavailable(format!(
            "refusing final symlink {}",
            path.display()
        )),
        E::InvalidFile(path) | E::PathChanged(path) | E::UnexpectedFileAlias(path) => {
            BlockDurabilityError::ScratchUnavailable(format!(
                "owned index path is not one regular file: {}",
                path.display()
            ))
        }
        E::Busy(path) => BlockDurabilityError::ScratchUnavailable(format!(
            "owned index file is locked: {}",
            path.display()
        )),
        E::Backend { operation, source } => {
            BlockDurabilityError::ScratchUnavailable(format!("index {operation}: {source}"))
        }
        other => block_backend(other),
    }
}
const RANGES: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new("ranges");
/// One disposable range index, created before any database path is touched.
pub(super) struct Index {
    txn: Option<redb::WriteTransaction>,
    db: Option<Database>,
    file: Option<TemporaryFile>,
    budget: Arc<DiskBudget>,
}
impl Index {
    pub(super) fn create(directory: &Path, limit: u64) -> BlockResult<Self> {
        let budget = DiskBudget::new(UuidEntityMigrationScratchKind::ReceiptIndex, limit);
        let file =
            TemporaryFile::create(directory, "ferrumc-block-ranges").map_err(scratch_error)?;
        let db = match file
            .file()
            .and_then(|f| f.open_database_with_cache(true, Some(&budget), 16 * 1024 * 1024))
        {
            Ok(db) => db,
            Err(e) => {
                file.cleanup().map_err(scratch_error)?;
                return Err(scratch_error(e));
            }
        };
        let txn = match db.begin_write() {
            Ok(txn) => txn,
            Err(e) => {
                drop(db);
                file.cleanup().map_err(scratch_error)?;
                return Err(block_backend(e));
            }
        };
        Ok(Self {
            txn: Some(txn),
            db: Some(db),
            file: Some(file),
            budget,
        })
    }
    fn insert(&self, domain: u8, first: u64, last: u64) -> BlockResult<()> {
        let txn = self
            .txn
            .as_ref()
            .ok_or_else(|| raw::bad("scratch", "closed"))?;
        let mut t = txn
            .open_table(RANGES)
            .map_err(|e| scratch_error(self.budget.map("range table", e)))?;
        let mut k = [0; 9];
        k[0] = domain;
        k[1..].copy_from_slice(&first.to_be_bytes());
        if t.get(k.as_slice()).map_err(block_backend)?.is_some() {
            return Err(raw::bad("ranges", "duplicate range start"));
        }
        t.insert(k.as_slice(), last.to_be_bytes().as_slice())
            .map_err(|e| scratch_error(self.budget.map("range insert", e)))?;
        Ok(())
    }
    #[allow(clippy::many_single_char_names)] // Row-local k/v/t names.
    fn validate(&self, b: &mut Budget) -> BlockResult<()> {
        let txn = self
            .txn
            .as_ref()
            .ok_or_else(|| raw::bad("scratch", "closed"))?;
        let t = txn.open_table(RANGES).map_err(block_backend)?;
        let mut previous: Option<(u8, u64)> = None;
        for row in t.iter().map_err(block_backend)? {
            let (k, v) = row.map_err(block_backend)?;
            b.bytes(17)?;
            let mut c = raw::Cursor::new(k.value());
            let domain = c.u8()?;
            let first = c.u64()?;
            c.finish()?;
            let last = u64::from_be_bytes(
                v.value()
                    .try_into()
                    .map_err(|_| raw::bad("scratch", "value"))?,
            );
            if previous.is_some_and(|(d, end)| d == domain && first <= end) {
                return Err(raw::bad("ranges", "overlap"));
            }
            previous = Some((domain, last));
        }
        Ok(())
    }
    pub(super) fn finish(mut self) -> BlockResult<()> {
        let aborted = self
            .txn
            .take()
            .map(redb::WriteTransaction::abort)
            .transpose()
            .map_err(block_backend);
        drop(self.db.take());
        if let Some(file) = self.file.take() {
            file.cleanup().map_err(scratch_error)?;
        }
        aborted?;
        if let Some(e) = self.budget.error() {
            return Err(scratch_error(e));
        }
        Ok(())
    }
}
/// A failed operation keeps its own error as the reported cause. A scratch cleanup that also
/// fails is logged alongside it instead of replacing it; alone, it is the reported error.
pub(super) fn keep_cause(
    cause: BlockDurabilityError,
    cleanup: BlockResult<()>,
) -> BlockDurabilityError {
    if let Err(cleanup) = cleanup {
        tracing::warn!(%cleanup, error = %cause, "Format7 scratch index cleanup also failed");
    }
    cause
}
fn then_cleanup(result: BlockResult<()>, cleanup: BlockResult<()>) -> BlockResult<()> {
    match result {
        Ok(()) => cleanup,
        Err(cause) => Err(keep_cause(cause, cleanup)),
    }
}
impl Drop for Index {
    fn drop(&mut self) {
        self.txn.take();
        self.db.take();
        self.file.take();
    }
}
fn catalog(txn: &redb::WriteTransaction) -> BlockResult<()> {
    let mut n = 0;
    for t in txn.list_tables().map_err(block_backend)? {
        n += 1;
        if n > 17
            || t.name() != META_TABLE.name()
                && !six::TABLES.iter().any(|d| d.name() == t.name())
                && t.name() != HEADER.name()
                && t.name() != JOURNAL.name()
        {
            return Err(BlockDurabilityError::CatalogMismatch("unknown table"));
        }
    }
    if n != 17
        || txn
            .list_multimap_tables()
            .map_err(block_backend)?
            .next()
            .is_some()
    {
        return Err(BlockDurabilityError::CatalogMismatch(
            "missing table/multimap",
        ));
    }
    txn.open_table(META_TABLE)
        .map_err(|_| BlockDurabilityError::CatalogMismatch("meta type"))?;
    for d in six::TABLES.into_iter().chain([HEADER, JOURNAL]) {
        txn.open_table(d)
            .map_err(|_| BlockDurabilityError::CatalogMismatch("data type"))?;
    }
    Ok(())
}
/// Consumes the preflighted index; it is cleaned up on every path, typed on failure.
pub(super) fn initialize(
    db: &Database,
    origin: StoreOrigin,
    c: BlockDurabilityConfig,
    index: Index,
) -> BlockResult<()> {
    let mut slot = Some(index);
    let result = initialize_with(db, origin, c, &mut slot);
    then_cleanup(result, slot.take().map_or(Ok(()), Index::finish))
}
fn initialize_with(
    db: &Database,
    origin: StoreOrigin,
    c: BlockDurabilityConfig,
    index: &mut Option<Index>,
) -> BlockResult<()> {
    let mut take = || index.take().ok_or_else(|| raw::bad("scratch", "closed"));
    let mut txn = db.begin_write().map_err(block_backend)?;
    txn.set_durability(Durability::Immediate);
    match origin {
        StoreOrigin::Fresh => {
            {
                let mut m = txn.open_table(META_TABLE).map_err(block_backend)?;
                for (k, v) in [
                    (META_FORMAT_KEY, 7),
                    (ownership::RECEIPT_COUNT, 0),
                    (ownership::REVISION_COUNT, 0),
                    (
                        ownership::REQUEST_LIMIT,
                        c.capacity.ownership.max_request_bytes() as u64,
                    ),
                    (
                        ownership::RECEIPT_LIMIT,
                        c.capacity.ownership.max_receipts(),
                    ),
                    (
                        ownership::REVISION_LIMIT,
                        c.capacity.ownership.max_revision_rows(),
                    ),
                    (CHUNK_COUNT, 0),
                    (RECEIPT_BYTES, 0),
                    (JOURNAL_COUNT, 0),
                    (JOURNAL_BYTES, 0),
                ]
                .into_iter()
                .chain(agreement(c))
                {
                    m.insert(k, v).map_err(block_backend)?;
                }
            }
            for d in six::TABLES.into_iter().chain([HEADER, JOURNAL]) {
                txn.open_table(d).map_err(block_backend)?;
            }
            scan(&txn, c, take()?)?;
            txn.commit().map_err(block_backend)?;
        }
        StoreOrigin::Existing => {
            if !txn
                .list_tables()
                .map_err(block_backend)?
                .any(|t| t.name() == META_TABLE.name())
            {
                return Err(BlockDurabilityError::CatalogMismatch(
                    if txn.list_tables().map_err(block_backend)?.next().is_none()
                        && txn
                            .list_multimap_tables()
                            .map_err(block_backend)?
                            .next()
                            .is_none()
                    {
                        // Earlier versions could leave this after a refused fresh create.
                        "empty redb file without any FerrumC table (likely left by an interrupted \
                         fresh create); nothing was committed: verify, move it aside and open again"
                    } else {
                        "missing marker table"
                    },
                ));
            }
            let actual = txn
                .open_table(META_TABLE)
                .map_err(|_| BlockDurabilityError::CatalogMismatch("meta type"))?
                .get(META_FORMAT_KEY)
                .map_err(block_backend)?
                .map(|v| v.value());
            match actual {
                Some(actual @ 3..=6) => {
                    return Err(BlockDurabilityError::MigrationRequired { actual })
                }
                Some(7) => {}
                _ => return Err(BlockDurabilityError::CatalogMismatch("unknown marker")),
            }
            scan(&txn, c, take()?)?;
        }
    }
    Ok(())
}
struct RawBudget {
    rows: u64,
    bytes: u64,
    config: BlockDurabilityConfig,
}
impl RawBudget {
    fn row(&mut self, k: &[u8], v: &[u8]) -> BlockResult<()> {
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or(BlockDurabilityError::ScanCapacity("rows"))?;
        self.bytes = self
            .bytes
            .checked_add(k.len() as u64)
            .and_then(|n| n.checked_add(v.len() as u64))
            .ok_or(BlockDurabilityError::ScanCapacity("raw bytes"))?;
        if self.rows > self.config.ownership.open_limits().max_scan_rows() {
            return Err(BlockDurabilityError::ScanCapacity("rows"));
        }
        if self.bytes > self.config.ownership.open_limits().max_scan_bytes() {
            return Err(BlockDurabilityError::ScanCapacity("raw bytes"));
        }
        Ok(())
    }
}
/// Validates the whole Format7 catalog; the index is cleaned up on every path.
pub(super) fn scan(
    txn: &redb::WriteTransaction,
    c: BlockDurabilityConfig,
    index: Index,
) -> BlockResult<()> {
    let result = scan_with(txn, c, &index);
    then_cleanup(result, index.finish())
}
/// Consume exactly one operation range while its redb row guard stays in this stack frame.
/// No Q bytes are copied and no owner contains a borrow into itself. The guard and parsed
/// frame are dropped before fetching the next operation. Range starts, rather than a cache
/// of old IDs, reject reuse/interleaving: every next operation must start at the next ID.
fn scoped_journal(
    table: &impl ReadableTable<&'static [u8], &'static [u8]>,
    reader: &impl owned::Read,
    raw_budget: &mut RawBudget,
    relation: &mut Budget,
    #[cfg(test)] observation: &tests::JournalObservation,
) -> BlockResult<(u64, u64)> {
    let mut rows = table.iter().map_err(block_backend)?;
    let mut count = 0u64;
    let mut bytes = 0u64;
    while let Some(row) = rows.next() {
        let (key, value) = row.map_err(block_backend)?;
        // Admit and validate the entering row before accessing its owning receipt.
        scoped_row(key.value(), value.value(), count, raw_budget, relation)?;
        let (_, operation, _, _, _) = raw::journal_ref(value.value())?;
        let operation_key = operation.to_bytes();
        let receipt_bytes = owned::Read::owned(reader, owned::Table::Receipt, &operation_key)?
            .ok_or_else(|| raw::bad(raw::JOURNAL, "orphan receipt"))?;
        #[cfg(test)]
        let receipt_bytes = observation.retain(receipt_bytes);
        // The reader charges key+row; frame_q charges the actual Q parse/hash once.
        let (receipt, q) = raw::frame_q(&operation_key, receipt_bytes.as_ref(), relation)?;
        if receipt.first != Some(count) || receipt.count == 0 {
            return Err(raw::bad(raw::JOURNAL, "operation range start"));
        }
        let end = count
            .checked_add(u64::from(receipt.count))
            .ok_or(BlockDurabilityError::SequenceExhausted)?;
        let mut current = (key, value);
        for (ordinal, expected) in q.effects.iter().enumerate() {
            if ordinal != 0 {
                current = rows
                    .next()
                    .ok_or_else(|| raw::bad(raw::JOURNAL, "truncated operation range"))?
                    .map_err(block_backend)?;
                scoped_row(
                    current.0.value(),
                    current.1.value(),
                    count,
                    raw_budget,
                    relation,
                )?;
            }
            let (scope, actual_operation, actual_ordinal, sequence, effect) =
                raw::journal_ref(current.1.value())?;
            if scope != q.scope
                || actual_operation != operation
                || usize::from(actual_ordinal) != ordinal
                || sequence != count
                || receipt
                    .first
                    .and_then(|first| first.checked_add(ordinal as u64))
                    != Some(count)
                || effect.bytes != expected.bytes
            {
                return Err(raw::bad(raw::JOURNAL, "ordinal/receipt disagreement"));
            }
            bytes = bytes
                .checked_add((current.0.value().len() + current.1.value().len()) as u64)
                .ok_or(BlockDurabilityError::ScanCapacity("journal bytes"))?;
            count = count
                .checked_add(1)
                .ok_or(BlockDurabilityError::SequenceExhausted)?;
        }
        if count != end {
            return Err(raw::bad(raw::JOURNAL, "operation range end"));
        }
    }
    Ok((count, bytes))
}
fn scoped_row(
    key: &[u8],
    value: &[u8],
    expected: u64,
    raw_budget: &mut RawBudget,
    relation: &mut Budget,
) -> BlockResult<()> {
    raw_budget.row(key, value)?;
    relation.bytes((key.len() + value.len()) as u64)?;
    let id = u64::from_be_bytes(
        key.try_into()
            .map_err(|_| raw::bad(raw::JOURNAL, "key length"))?,
    );
    if id != expected || id == u64::MAX {
        return Err(raw::bad(raw::JOURNAL, "noncontiguous scoped sequence"));
    }
    if raw::journal_ref(value)?.3 != id {
        return Err(raw::bad(raw::JOURNAL, "sequence/key"));
    }
    Ok(())
}
// One streaming pass keeps every table's relation dispatch and counter sum together.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn scan_with(
    txn: &redb::WriteTransaction,
    c: BlockDurabilityConfig,
    index: &Index,
) -> BlockResult<()> {
    catalog(txn)?;
    let mut reader = reader(txn, c)?;
    let mut raw_budget = RawBudget {
        rows: 0,
        bytes: 0,
        config: c,
    };
    let mut relation = Budget::startup(c);
    reader.budget = Some(relation.clone());
    let r = engine::Meter {
        reader: &reader,
        budget: relation.clone(),
    };
    let mut mandatory = 0u16;
    let mut old_watermark = None;
    let expected: [(&str, Option<u64>); 15] = [
        (META_FORMAT_KEY, Some(7)),
        (ownership::RECEIPT_COUNT, None),
        (ownership::REVISION_COUNT, None),
        (
            ownership::REQUEST_LIMIT,
            Some(c.capacity.ownership.max_request_bytes() as u64),
        ),
        (
            ownership::RECEIPT_LIMIT,
            Some(c.capacity.ownership.max_receipts()),
        ),
        (
            ownership::REVISION_LIMIT,
            Some(c.capacity.ownership.max_revision_rows()),
        ),
        (REQUEST_LIMIT, Some(c.capacity.request as u64)),
        (CHUNK_LIMIT, Some(c.capacity.chunks)),
        (CHUNK_COUNT, None),
        (RECEIPT_LIMIT, Some(c.capacity.receipt_bytes)),
        (RECEIPT_BYTES, None),
        (JOURNAL_LIMIT, Some(c.capacity.journal_rows)),
        (JOURNAL_BYTE_LIMIT, Some(c.capacity.journal_bytes)),
        (JOURNAL_COUNT, None),
        (JOURNAL_BYTES, None),
    ];
    for row in reader.meta.iter().map_err(block_backend)? {
        let (k, v) = row.map_err(block_backend)?;
        raw_budget.row(k.value().as_bytes(), &v.value().to_be_bytes())?;
        if k.value() == META_LAST_MUTATION_ID_KEY {
            old_watermark = Some(v.value());
            continue;
        }
        if k.value() == WATERMARK {
            continue;
        }
        let (i, (_, exp)) = expected
            .iter()
            .enumerate()
            .find(|(_, (key, _))| *key == k.value())
            .ok_or(BlockDurabilityError::CatalogMismatch("unknown metadata"))?;
        mandatory |= 1 << i;
        if exp.is_some_and(|exp| exp != v.value()) {
            return Err(if k.value() == META_FORMAT_KEY {
                BlockDurabilityError::CatalogMismatch("marker")
            } else {
                BlockDurabilityError::CapacityContractMismatch
            });
        }
    }
    if mandatory != 32767 {
        return Err(BlockDurabilityError::CatalogMismatch(
            "missing mandatory meta",
        ));
    }
    let (receipts, revisions) = owned::Read::counts(&r)?;
    let persisted = counts(&reader.meta, c)?;
    let mut actual = [0u64; 16];
    let mut receipt_bytes = 0u64;
    let mut journal_bytes = 0u64;
    let mut descriptors = 0usize;
    let mut old_last = None;
    let mut scoped_count = 0u64;
    for (slot, d) in six::TABLES.into_iter().chain([HEADER, JOURNAL]).enumerate() {
        let name = match slot {
            0..=13 => six::TABLE_NAMES[slot],
            14 => raw::CHUNK_OWNER,
            _ => raw::JOURNAL,
        };
        // redb permits only one open mutable table handle in a writer.
        // Stream the very handles already used for all relation reads.
        let extra;
        let blocks = reader
            .blocks
            .as_ref()
            .ok_or(BlockDurabilityError::Unsupported)?;
        let table = match name {
            "ferrumc:generation" => &reader.uuid.generations,
            "ferrumc:entity" => &reader.uuid.entities,
            "ferrumc:entity_locator" => &reader.uuid.locators,
            "ferrumc:entity_chunk" => &reader.uuid.indexes,
            "ferrumc:entity_chunk_epoch" => &reader.uuid.epochs,
            "ferrumc:player" => &reader.players,
            owned_raw::PLAYER_OWNER => &reader.player_headers,
            owned_raw::ENTITY_OWNER => &reader.entity_headers,
            owned_raw::RECEIPT => &reader.receipts,
            "ferrumc:chunk" => &blocks[0],
            "ferrumc:chunk_overlay" => &blocks[1],
            raw::CHUNK_OWNER => &blocks[2],
            raw::JOURNAL => &blocks[3],
            _ => {
                extra = txn.open_table(d).map_err(block_backend)?;
                &extra
            }
        };
        if name == raw::JOURNAL {
            let (rows, bytes) = scoped_journal(
                table,
                &r,
                &mut raw_budget,
                &mut relation,
                #[cfg(test)]
                &tests::JournalObservation::default(),
            )?;
            scoped_count = rows;
            journal_bytes = bytes;
            actual[slot] = rows;
            continue;
        }
        for row in table.iter().map_err(block_backend)? {
            let (k, v) = row.map_err(block_backend)?;
            let (k, v) = (k.value(), v.value());
            raw_budget.row(k, v)?;
            relation.bytes((k.len() + v.len()) as u64)?;
            actual[slot] = actual[slot]
                .checked_add(1)
                .ok_or(BlockDurabilityError::ScanCapacity("rows"))?;
            match name {
                "ferrumc:generation" => {
                    let key = crate::GenerationKey::decode(k)?;
                    crate::generation::decode_identity(key, v)?;
                    descriptors += 1;
                    if descriptors > crate::MAX_GENERATION_IDENTITIES {
                        return Err(raw::bad(name, "descriptor limit"));
                    }
                }
                "ferrumc:chunk" | "ferrumc:chunk_overlay" | raw::CHUNK_OWNER => {
                    let key = crate::persistent_entity::raw::decode_chunk(k, name)?;
                    let scope = crate::GenerationKey::new(key.world(), key.dimension());
                    let identity = r.identity(scope)?.ok_or(
                        crate::StorageError::MissingGenerationIdentity { key: Some(scope) },
                    )?;
                    engine::chunk_state(&r, key, identity, &mut relation)?;
                    if name == raw::CHUNK_OWNER {
                        raw::header(v)?;
                    } else {
                        relation.cells(98_304)?;
                        six::common_row(&r, name, k, v)?;
                    }
                }
                "ferrumc:player" | owned_raw::PLAYER_OWNER => {
                    six::owned_player_row(&r, name, k, v)?;
                }
                owned_raw::ENTITY_OWNER => six::owned_entity_row(&r, k, v)?,
                owned_raw::RECEIPT => {
                    receipt_bytes = receipt_bytes
                        .checked_add((k.len() + v.len()) as u64)
                        .ok_or(BlockDurabilityError::ScanCapacity("receipt bytes"))?;
                    if v.starts_with(b"FCTR") {
                        let receipt = owned_raw::receipt(k, v)?;
                        owned::receipt_relations(&r, &receipt)?;
                    } else {
                        let receipt = raw::frame(k, v, &mut relation)?;
                        if receipt.request_bytes().len() > c.capacity.request {
                            return Err(raw::bad("FCBR", "request over persisted cap"));
                        }
                        engine::receipt_relations(&r, &receipt, &mut relation)?;
                        if let Some((first, count)) = receipt.journal_range() {
                            index.insert(1, first, first + u64::from(count) - 1)?;
                        }
                    }
                }
                "ferrumc:mutation_log" => {
                    if descriptors == 0 {
                        return Err(raw::bad(name, "missing descriptor"));
                    }
                    let id = super::mutation_id_from_key(k)?;
                    if old_last.map_or(Some(0), |n: u64| n.checked_add(1)) != Some(id)
                        || id == u64::MAX
                        || crate::codec::decode_mutation_log_record(v)?.id() != id
                    {
                        return Err(raw::bad(name, "legacy sequence/record"));
                    }
                    old_last = Some(id);
                }
                "ferrumc:mutation_batch" => {
                    if descriptors == 0 {
                        return Err(raw::bad(name, "missing descriptor"));
                    }
                    let id = crate::JournalBatchId::from_bytes(
                        k.try_into().map_err(|_| raw::bad(name, "ID length"))?,
                    );
                    let receipt = super::decode_journal_receipt(id, v)?;
                    if let Some(range) = receipt.id_range() {
                        if old_last.is_none_or(|last| *range.end() > last) {
                            return Err(raw::bad(name, "missing range"));
                        }
                        index.insert(0, *range.start(), *range.end())?;
                    }
                }
                _ => six::common_row(&owned::EntityAdapter(&r), name, k, v)?,
            }
        }
    }
    if old_last != old_watermark
        || persisted.watermark != scoped_count.checked_sub(1)
        || persisted.journal_rows != scoped_count
        || persisted.journal_bytes != journal_bytes
        || persisted.chunks != actual[14]
        || persisted.receipt_bytes != receipt_bytes
        || receipts != actual[13]
        || revisions
            != actual[11]
                .checked_add(actual[12])
                .ok_or(BlockDurabilityError::ScanCapacity("counts"))?
    {
        return Err(raw::bad("meta", "actual counters/watermarks differ"));
    }
    if receipts > c.capacity.ownership.max_receipts()
        || revisions > c.capacity.ownership.max_revision_rows()
    {
        return Err(raw::bad("meta", "shared count exceeds capacity"));
    }
    index.validate(&mut relation)?;
    Ok(())
}

#[cfg(test)]
mod tests;
