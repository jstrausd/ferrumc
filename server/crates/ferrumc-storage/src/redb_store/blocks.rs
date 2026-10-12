//! Actual Format7 transactions, explicit adapter and shared retained-byte accounting.
use super::uuid_migration::files::{remove_owned_path, FileIdentity};
use super::{
    ownership::{self, backend, Cutpoint},
    RedbStore, StoreOrigin, CHUNK_OVERLAY_TABLE, CHUNK_TABLE, META_TABLE, PLAYER_TABLE,
};
use crate::block_durability::{
    engine::{self, Budget},
    ownership_adapter, raw, BlockCommitOutcome, BlockDurabilityConfig, BlockDurabilityError,
    BlockMutationRequest, BlockReceipt, BlockReceiptQuery, BlockResult, BlockSnapshotSpec,
    ChunkImageCasRequest, OwnedBlockSnapshot, PlayerBlockDurabilityStore, PlayerBlockRequest,
    Request,
};
use crate::ownership::{engine as owned, Runtime};
use crate::{OwnershipCommitDisposition, OwnershipError, OwnershipResult};
use async_trait::async_trait;
use redb::{
    Database, Durability, ReadableTable, ReadableTableMetadata, TableDefinition, TableHandle,
};
use std::{fs::OpenOptions, io::ErrorKind, path::Path, sync::Arc};

pub(super) const HEADER: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new(raw::CHUNK_OWNER);
pub(super) const JOURNAL: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new(raw::JOURNAL);
pub(super) const CHUNK_LIMIT: &str = "block_revision_limit";
pub(super) const CHUNK_COUNT: &str = "block_revision_count";
pub(super) const REQUEST_LIMIT: &str = "block_request_limit";
pub(super) const RECEIPT_LIMIT: &str = "operation_receipt_byte_limit";
pub(super) const RECEIPT_BYTES: &str = "operation_receipt_bytes";
pub(super) const JOURNAL_LIMIT: &str = "scoped_journal_row_limit";
pub(super) const JOURNAL_BYTE_LIMIT: &str = "scoped_journal_byte_limit";
pub(super) const JOURNAL_COUNT: &str = "scoped_journal_count";
pub(super) const JOURNAL_BYTES: &str = "scoped_journal_bytes";
pub(super) const WATERMARK: &str = "last_scoped_mutation_id";
pub(super) fn block_backend(e: impl std::fmt::Display) -> BlockDurabilityError {
    BlockDurabilityError::Backend(e.to_string())
}
/// Removes a database file created by this very call after its fresh initialization failed.
/// Removal requires the created inode at the path and positive proof of an empty catalog:
/// no table at all through the still-open handle, or a zero-length file when redb never
/// initialized it. Otherwise the file is left; a later open names it (`CatalogMismatch`).
fn abandon_fresh(
    path: &Path,
    identity: Option<FileIdentity>,
    db: Option<Database>,
    error: BlockDurabilityError,
) -> BlockDurabilityError {
    let empty = match db {
        Some(db) => {
            let empty = db.begin_read().is_ok_and(|txn| {
                txn.list_tables().is_ok_and(|mut t| t.next().is_none())
                    && txn
                        .list_multimap_tables()
                        .is_ok_and(|mut t| t.next().is_none())
            });
            drop(db);
            empty
        }
        None => std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() == 0),
    };
    if let (true, Some(identity)) = (empty, identity) {
        if let Err(cleanup) = remove_owned_path(path, identity) {
            tracing::warn!(%cleanup, %error, "refused fresh Format7 file was left in place");
        }
    }
    error
}
pub(super) fn agreement(c: BlockDurabilityConfig) -> [(&'static str, u64); 5] {
    [
        (REQUEST_LIMIT, c.capacity.request as u64),
        (CHUNK_LIMIT, c.capacity.chunks),
        (RECEIPT_LIMIT, c.capacity.receipt_bytes),
        (JOURNAL_LIMIT, c.capacity.journal_rows),
        (JOURNAL_BYTE_LIMIT, c.capacity.journal_bytes),
    ]
}
pub(super) fn counts(
    m: &impl ReadableTable<&'static str, u64>,
    c: BlockDurabilityConfig,
) -> BlockResult<engine::Counts> {
    let get = |k| {
        m.get(k)
            .map_err(block_backend)?
            .map(|v| v.value())
            .ok_or(BlockDurabilityError::CatalogMismatch("missing block meta"))
    };
    for (k, v) in agreement(c) {
        if get(k)? != v {
            return Err(BlockDurabilityError::CapacityContractMismatch);
        }
    }
    let counts = engine::Counts {
        chunks: get(CHUNK_COUNT)?,
        receipt_bytes: get(RECEIPT_BYTES)?,
        journal_rows: get(JOURNAL_COUNT)?,
        journal_bytes: get(JOURNAL_BYTES)?,
        watermark: m.get(WATERMARK).map_err(block_backend)?.map(|v| v.value()),
    };
    if counts.chunks > c.capacity.chunks
        || counts.receipt_bytes > c.capacity.receipt_bytes
        || counts.journal_rows > c.capacity.journal_rows
        || counts.journal_bytes > c.capacity.journal_bytes
        || counts.watermark != counts.journal_rows.checked_sub(1)
    {
        return Err(raw::bad("meta", "block counts exceed agreement/watermark"));
    }
    Ok(counts)
}
/// Format7 freshness also counts Chunk histories, scoped journal rows and their counters.
pub(super) fn state_is_empty(txn: &redb::WriteTransaction) -> ferrumc_core::Result<bool> {
    if txn
        .list_tables()
        .map_err(super::backend_err)?
        .any(|t| t.name() == HEADER.name())
    {
        let meta = txn.open_table(META_TABLE).map_err(super::backend_err)?;
        for key in [CHUNK_COUNT, RECEIPT_BYTES, JOURNAL_COUNT, JOURNAL_BYTES] {
            if meta
                .get(key)
                .map_err(super::backend_err)?
                .is_none_or(|n| n.value() != 0)
            {
                return Ok(false);
            }
        }
        if meta.get(WATERMARK).map_err(super::backend_err)?.is_some() {
            return Ok(false);
        }
        drop(meta);
        for table in [HEADER, JOURNAL] {
            if !txn
                .open_table(table)
                .map_err(super::backend_err)?
                .is_empty()
                .map_err(super::backend_err)?
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
pub(super) fn extend_read(
    txn: &redb::ReadTransaction,
    r: &mut ownership::ReadReader,
    c: BlockDurabilityConfig,
) -> OwnershipResult<()> {
    r.block_config = Some(c);
    r.budget = Some(Budget::runtime(c));
    r.blocks = Some([
        txn.open_table(CHUNK_TABLE).map_err(backend)?,
        txn.open_table(CHUNK_OVERLAY_TABLE).map_err(backend)?,
        txn.open_table(HEADER).map_err(backend)?,
        txn.open_table(JOURNAL).map_err(backend)?,
    ]);
    Ok(())
}
pub(super) fn reader(
    txn: &redb::WriteTransaction,
    c: BlockDurabilityConfig,
) -> OwnershipResult<ownership::WriteReader<'_>> {
    let mut r = ownership::write_reader(txn, c.capacity.ownership)?;
    r.block_config = Some(c);
    r.budget = Some(Budget::runtime(c));
    r.blocks = Some([
        txn.open_table(CHUNK_TABLE).map_err(backend)?,
        txn.open_table(CHUNK_OVERLAY_TABLE).map_err(backend)?,
        txn.open_table(HEADER).map_err(backend)?,
        txn.open_table(JOURNAL).map_err(backend)?,
    ]);
    Ok(r)
}
pub(super) fn commit_v1(
    db: &Database,
    owner: &Arc<()>,
    c: BlockDurabilityConfig,
    request: crate::ownership::Request,
    mut hook: impl FnMut(Cutpoint) -> OwnershipResult<()>,
) -> OwnershipResult<crate::OwnershipCommitOutcome> {
    let mut txn = db.begin_write().map_err(backend)?;
    txn.set_durability(Durability::Immediate);
    hook(Cutpoint::BeforePlan)?;
    let prepared = {
        let r = reader(&txn, c)?;
        let meter = engine::Meter {
            reader: &r,
            budget: r.budget.clone().ok_or(OwnershipError::Unsupported)?,
        };
        owned::prepare(&meter, owner, c.capacity.ownership, request)?
    };
    match prepared {
        owned::Prepared::Replay(r) => Ok(owned::outcome(
            *r,
            owner,
            OwnershipCommitDisposition::Replayed,
        )),
        owned::Prepared::Commit(plan) => {
            let current = counts(&txn.open_table(META_TABLE).map_err(backend)?, c)
                .map_err(ownership_adapter)?;
            let bytes = current
                .receipt_bytes
                .checked_add(272)
                .filter(|n| *n <= c.capacity.receipt_bytes)
                .ok_or_else(|| {
                    ownership_adapter(BlockDurabilityError::HistoryCapacity {
                        resource: "receipt bytes",
                        max: c.capacity.receipt_bytes,
                    })
                })?;
            let receipt = plan.receipt.clone();
            ownership::apply(&txn, *plan, &mut hook)?;
            {
                let mut meta = txn.open_table(META_TABLE).map_err(backend)?;
                hook(Cutpoint::BeforeRow)?;
                meta.insert(RECEIPT_BYTES, bytes).map_err(backend)?;
                hook(Cutpoint::AfterRow)?;
            }
            hook(Cutpoint::BeforeCommit)?;
            txn.commit().map_err(|e| OwnershipError::CommitUncertain {
                query: receipt.query(),
                cause: e.to_string(),
            })?;
            hook(Cutpoint::AfterCommit).map_err(|e| OwnershipError::ExecutionUncertain {
                query: receipt.query(),
                cause: e.to_string(),
            })?;
            Ok(owned::outcome(
                receipt,
                owner,
                OwnershipCommitDisposition::Committed,
            ))
        }
    }
}
fn row(
    txn: &redb::WriteTransaction,
    t: TableDefinition<'_, &[u8], &[u8]>,
    k: &[u8],
    v: Option<&[u8]>,
    hook: &mut impl FnMut(Cutpoint) -> BlockResult<()>,
) -> BlockResult<()> {
    let mut table = txn.open_table(t).map_err(block_backend)?;
    hook(Cutpoint::BeforeRow)?;
    match v {
        Some(v) => {
            table.insert(k, v).map_err(block_backend)?;
        }
        None => {
            table.remove(k).map_err(block_backend)?;
        }
    }
    hook(Cutpoint::AfterRow)
}
fn apply(
    txn: &redb::WriteTransaction,
    plan: engine::Plan,
    hook: &mut impl FnMut(Cutpoint) -> BlockResult<()>,
) -> BlockResult<()> {
    if let Some(p) = plan.player {
        row(txn, PLAYER_TABLE, &p.key, p.record.as_deref(), hook)?;
        row(
            txn,
            ownership::PLAYER_OWNER,
            &p.key,
            Some(&crate::ownership::raw::encode_player_header(p.header)),
            hook,
        )?;
    }
    for c in plan.chunks {
        let key = crate::codec::chunk_key_bytes(c.key);
        if let Some(v) = c.full {
            row(txn, CHUNK_TABLE, &key, v.as_deref(), hook)?;
        }
        if let Some(v) = c.overlay {
            row(txn, CHUNK_OVERLAY_TABLE, &key, v.as_deref(), hook)?;
        }
        row(txn, HEADER, &key, Some(&c.header), hook)?;
    }
    for (id, b) in plan.journal {
        row(txn, JOURNAL, &id.to_be_bytes(), Some(&b), hook)?;
    }
    row(
        txn,
        ownership::RECEIPT,
        &plan.receipt.query.id.to_bytes(),
        Some(&plan.receipt.bytes),
        hook,
    )?;
    let mut meta = txn.open_table(META_TABLE).map_err(block_backend)?;
    let values = [
        (ownership::RECEIPT_COUNT, plan.receipts),
        (ownership::REVISION_COUNT, plan.revisions),
        (CHUNK_COUNT, plan.counts.chunks),
        (RECEIPT_BYTES, plan.counts.receipt_bytes),
        (JOURNAL_COUNT, plan.counts.journal_rows),
        (JOURNAL_BYTES, plan.counts.journal_bytes),
    ];
    for (k, v) in values {
        hook(Cutpoint::BeforeRow)?;
        meta.insert(k, v).map_err(block_backend)?;
        hook(Cutpoint::AfterRow)?;
    }
    if let Some(n) = plan.counts.watermark {
        hook(Cutpoint::BeforeRow)?;
        meta.insert(WATERMARK, n).map_err(block_backend)?;
        hook(Cutpoint::AfterRow)?;
    }
    Ok(())
}
pub(super) fn commit(
    db: &Database,
    owner: &Arc<()>,
    c: BlockDurabilityConfig,
    request: &Request,
    mut hook: impl FnMut(Cutpoint) -> BlockResult<()>,
) -> BlockResult<BlockCommitOutcome> {
    let mut txn = db.begin_write().map_err(block_backend)?;
    txn.set_durability(Durability::Immediate);
    hook(Cutpoint::BeforePlan)?;
    let prepared = {
        let r = reader(&txn, c)?;
        let mut b = r.budget.clone().ok_or(BlockDurabilityError::Unsupported)?;
        let meter = engine::Meter {
            reader: &r,
            budget: b.clone(),
        };
        engine::prepare(&meter, owner, request, c, &mut b)?
    };
    match prepared {
        engine::Prepared::Replay(receipt) => Ok(BlockCommitOutcome {
            receipt,
            disposition: OwnershipCommitDisposition::Replayed,
        }),
        engine::Prepared::Commit(plan) => {
            let receipt = plan.receipt.clone();
            let query = receipt.query;
            apply(&txn, plan, &mut hook)?;
            hook(Cutpoint::BeforeCommit)?;
            txn.commit()
                .map_err(|e| BlockDurabilityError::CommitUncertain {
                    query,
                    cause: e.to_string(),
                })?;
            hook(Cutpoint::AfterCommit).map_err(|e| BlockDurabilityError::ExecutionUncertain {
                query,
                cause: e.to_string(),
            })?;
            Ok(BlockCommitOutcome {
                receipt,
                disposition: OwnershipCommitDisposition::Committed,
            })
        }
    }
}
impl RedbStore {
    /// Creates exactly17 tables in one Immediate transaction or validates existing7 without migration.
    /// Explicit scratch directory owns one disposable bounded range index; startup is synchronous.
    /// Every prerequisite, including the scratch index, is proved before the database path is
    /// touched. A refused fresh create removes only the file it created itself, and only while
    /// that file provably holds no table; an existing file is never removed.
    pub fn open_with_player_block_durability(
        path: impl AsRef<Path>,
        config: BlockDurabilityConfig,
        scratch_directory: impl AsRef<Path>,
    ) -> BlockResult<Self> {
        if !cfg!(unix) {
            return Err(BlockDurabilityError::UnsupportedPlatform);
        }
        let path = path.as_ref();
        let index = super::block_validation::Index::create(
            scratch_directory.as_ref(),
            config.ownership.open_limits().max_journal_index_bytes(),
        )?;
        let mut builder = Database::builder();
        builder.set_cache_size(64 * 1024 * 1024);
        let db = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => {
                // Without the created handle's identity the path is never unlinked.
                let identity = file.metadata().ok().and_then(|m| FileIdentity::of(&m).ok());
                let db = match builder.create_file(file) {
                    Ok(db) => db,
                    Err(e) => {
                        let error = abandon_fresh(path, identity, None, block_backend(e));
                        return Err(super::block_validation::keep_cause(error, index.finish()));
                    }
                };
                if let Err(error) =
                    super::block_validation::initialize(&db, StoreOrigin::Fresh, config, index)
                {
                    return Err(abandon_fresh(path, identity, Some(db), error));
                }
                db
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                let db = match builder.open(path) {
                    Ok(db) => db,
                    Err(e) => {
                        return Err(super::block_validation::keep_cause(
                            block_backend(e),
                            index.finish(),
                        ));
                    }
                };
                super::block_validation::initialize(&db, StoreOrigin::Existing, config, index)?;
                db
            }
            Err(e) => {
                return Err(super::block_validation::keep_cause(
                    block_backend(e),
                    index.finish(),
                ));
            }
        };
        Ok(Self {
            db: Arc::new(db),
            generation_mode: crate::generation::GenerationMode::BlockDurability,
            entity_incarnation: Arc::new(()),
            ownership_runtime: Some(Arc::new(Runtime::new(config.ownership))),
            block_config: Some(config),
        })
    }
    fn block_settings(&self) -> BlockResult<BlockDurabilityConfig> {
        self.block_config
            .filter(|_| self.generation_mode == crate::generation::GenerationMode::BlockDurability)
            .ok_or(BlockDurabilityError::Unsupported)
    }
    async fn block_read<T: Send + 'static>(
        &self,
        f: impl FnOnce(
                &engine::Meter<'_, ownership::ReadReader>,
                &Arc<()>,
                BlockDurabilityConfig,
                &mut Budget,
            ) -> BlockResult<T>
            + Send
            + 'static,
    ) -> BlockResult<T> {
        let c = self.block_settings()?;
        let permit = self
            .ownership_runtime
            .as_ref()
            .ok_or(BlockDurabilityError::Unsupported)?
            .permit()
            .map_err(|e| {
                if e == OwnershipError::Busy {
                    BlockDurabilityError::Busy
                } else {
                    e.into()
                }
            })?;
        let db = self.db.clone();
        let owner = self.entity_incarnation.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let txn = db.begin_read().map_err(block_backend)?;
            let mut r = ownership::read_reader(&txn, c.capacity.ownership)?;
            extend_read(&txn, &mut r, c)?;
            let mut b = r.budget.clone().ok_or(BlockDurabilityError::Unsupported)?;
            let meter = engine::Meter {
                reader: &r,
                budget: b.clone(),
            };
            f(&meter, &owner, c, &mut b)
        })
        .await
        .map_err(block_backend)?
    }
    async fn block_write(&self, r: Request) -> BlockResult<BlockCommitOutcome> {
        self.block_write_hooks(r, |_| Ok(())).await
    }
    pub(super) async fn block_write_hooks(
        &self,
        r: Request,
        hook: impl FnMut(Cutpoint) -> BlockResult<()> + Send + 'static,
    ) -> BlockResult<BlockCommitOutcome> {
        let c = self.block_settings()?;
        if r.q.len() > c.capacity.request {
            return Err(BlockDurabilityError::ByteCapacity {
                resource: "request",
                len: r.q.len() as u64,
                max: c.capacity.request as u64,
            });
        }
        let permit = self
            .ownership_runtime
            .as_ref()
            .ok_or(BlockDurabilityError::Unsupported)?
            .permit()
            .map_err(|e| {
                if e == OwnershipError::Busy {
                    BlockDurabilityError::Busy
                } else {
                    e.into()
                }
            })?;
        let db = self.db.clone();
        let owner = self.entity_incarnation.clone();
        let query = r.query;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            commit(&db, &owner, c, &r, hook)
        })
        .await
        .map_err(|e| BlockDurabilityError::ExecutionUncertain {
            query,
            cause: e.to_string(),
        })?
    }
    pub(super) async fn block_load_full(
        &self,
        key: crate::ChunkKey,
    ) -> BlockResult<Option<crate::ChunkRecord>> {
        self.block_read(move |r, _, _, b| {
            let scope = crate::GenerationKey::new(key.world(), key.dimension());
            let id = crate::persistent_entity::engine::EntityRead::identity(r, scope)?
                .ok_or(crate::StorageError::MissingGenerationIdentity { key: Some(scope) })?;
            let (_, p) = engine::chunk_state(r, key, id, b)?;
            p.full
                .map(|bytes| {
                    b.cells(98_304)?;
                    b.bytes(bytes.len() as u64)?;
                    crate::codec::decode_chunk_record(&bytes).map_err(Into::into)
                })
                .transpose()
        })
        .await
    }
    pub(super) async fn block_load_overlay(
        &self,
        key: crate::ChunkKey,
    ) -> BlockResult<Option<crate::ChunkOverlayRecord>> {
        self.block_read(move |r, _, _, b| {
            let scope = crate::GenerationKey::new(key.world(), key.dimension());
            let id = crate::persistent_entity::engine::EntityRead::identity(r, scope)?
                .ok_or(crate::StorageError::MissingGenerationIdentity { key: Some(scope) })?;
            let (_, p) = engine::chunk_state(r, key, id, b)?;
            p.overlay
                .map(|bytes| {
                    b.cells(98_304)?;
                    b.bytes(bytes.len() as u64)?;
                    crate::codec::decode_chunk_overlay_record(&bytes).map_err(Into::into)
                })
                .transpose()
        })
        .await
    }
}
#[async_trait]
impl PlayerBlockDurabilityStore for RedbStore {
    async fn load_owned_blocks(&self, spec: BlockSnapshotSpec) -> BlockResult<OwnedBlockSnapshot> {
        self.block_read(move |r, o, c, b| engine::snapshot(r, o, spec, c, b))
            .await
    }
    async fn commit_player_blocks(&self, r: PlayerBlockRequest) -> BlockResult<BlockCommitOutcome> {
        self.block_write(r.0).await
    }
    async fn commit_blocks(&self, r: BlockMutationRequest) -> BlockResult<BlockCommitOutcome> {
        self.block_write(r.0).await
    }
    async fn compare_exchange_chunk_images(
        &self,
        r: ChunkImageCasRequest,
    ) -> BlockResult<BlockCommitOutcome> {
        self.block_write(r.0).await
    }
    async fn load_block_receipt(&self, q: BlockReceiptQuery) -> BlockResult<Option<BlockReceipt>> {
        self.block_read(move |r, _, _, b| engine::lookup(r, q, b))
            .await
    }
}

#[cfg(test)]
mod tests;
