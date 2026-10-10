//! One actual redb transaction owns both CAS resources and all durable evidence.

use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::ops::Bound;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use ferrumc_world::GenerationIdentity;
use redb::{Database, Durability, ReadableTable, TableDefinition};

use super::{
    uuid_entities, RedbStore, StoreOrigin, ENTITY_TABLE, GENERATION_TABLE, META_TABLE, PLAYER_TABLE,
};
use crate::ownership::{engine, raw, Request, Runtime};
use crate::persistent_entity::engine::{EntityRead, RawRows, RawTable};
use crate::persistent_entity::{engine as uuid_engine, raw as uuid_raw};
use crate::{
    ChunkKey, EntityOwnershipCasRequest, GenerationKey, OwnedEntitySnapshot,
    OwnedPlayerEntitySnapshot, OwnedPlayerSnapshot, OwnershipCapacity, OwnershipCommitDisposition,
    OwnershipCommitOutcome, OwnershipConfig, OwnershipError, OwnershipReceipt,
    OwnershipReceiptQuery, OwnershipResult, PersistentEntityCursor, PersistentEntityKey,
    PersistentEntityPage, PlayerEntityOwnershipStore, PlayerEntityTransferRequest,
    PlayerOwnershipCasRequest, ScopedPlayerKey, StorageError,
};

pub(super) const PLAYER_OWNER: TableDefinition<'_, &[u8], &[u8]> =
    TableDefinition::new(raw::PLAYER_OWNER);
pub(super) const ENTITY_OWNER: TableDefinition<'_, &[u8], &[u8]> =
    TableDefinition::new(raw::ENTITY_OWNER);
pub(super) const RECEIPT: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new(raw::RECEIPT);
pub(super) const RECEIPT_COUNT: &str = "ownership_receipt_count";
pub(super) const REVISION_COUNT: &str = "ownership_revision_count";
pub(super) const REQUEST_LIMIT: &str = "ownership_request_limit";
pub(super) const RECEIPT_LIMIT: &str = "ownership_receipt_limit";
pub(super) const REVISION_LIMIT: &str = "ownership_revision_limit";

pub(super) fn backend(e: impl std::fmt::Display) -> OwnershipError {
    OwnershipError::Backend(e.to_string())
}
pub(super) struct Reader<T, M> {
    pub(super) uuid: uuid_entities::Reader<T>,
    pub(super) players: T,
    pub(super) player_headers: T,
    pub(super) entity_headers: T,
    pub(super) receipts: T,
    pub(super) meta: M,
    pub(super) capacity: OwnershipCapacity,
}
pub(super) type WriteReader<'a> =
    Reader<redb::Table<'a, &'static [u8], &'static [u8]>, redb::Table<'a, &'static str, u64>>;
type ReadReader = Reader<
    redb::ReadOnlyTable<&'static [u8], &'static [u8]>,
    redb::ReadOnlyTable<&'static str, u64>,
>;
impl<T: ReadableTable<&'static [u8], &'static [u8]>, M: ReadableTable<&'static str, u64>> EntityRead
    for Reader<T, M>
{
    type Bytes<'a>
        = uuid_entities::RawGuard<'a>
    where
        Self: 'a;
    fn get(
        &self,
        table: RawTable,
        key: &[u8],
    ) -> std::result::Result<Option<Self::Bytes<'_>>, StorageError> {
        self.uuid.get(table, key)
    }
    fn identity(
        &self,
        key: GenerationKey,
    ) -> std::result::Result<Option<GenerationIdentity>, StorageError> {
        self.uuid.identity(key)
    }
    fn index_rows(
        &self,
        lower: Bound<Vec<u8>>,
        upper: Bound<Vec<u8>>,
    ) -> std::result::Result<RawRows<'_, Self::Bytes<'_>>, StorageError> {
        self.uuid.index_rows(lower, upper)
    }
}
impl<T: ReadableTable<&'static [u8], &'static [u8]>, M: ReadableTable<&'static str, u64>>
    engine::Read for Reader<T, M>
{
    fn owned(&self, table: engine::Table, key: &[u8]) -> OwnershipResult<Option<Self::Bytes<'_>>> {
        let t = match table {
            engine::Table::Player => &self.players,
            engine::Table::PlayerHeader => &self.player_headers,
            engine::Table::EntityHeader => &self.entity_headers,
            engine::Table::Receipt => &self.receipts,
        };
        Ok(t.get(key).map_err(backend)?.map(uuid_entities::RawGuard))
    }
    fn counts(&self) -> OwnershipResult<(u64, u64)> {
        let value = |key| {
            self.meta
                .get(key)
                .map_err(backend)?
                .map(|v| v.value())
                .ok_or(OwnershipError::CatalogMismatch(
                    "missing ownership metadata",
                ))
        };
        if value(super::META_FORMAT_KEY)? != 6 {
            return Err(OwnershipError::CatalogMismatch("wrong ownership format"));
        }
        if value(REQUEST_LIMIT)? != self.capacity.max_request_bytes() as u64
            || value(RECEIPT_LIMIT)? != self.capacity.max_receipts()
            || value(REVISION_LIMIT)? != self.capacity.max_revision_rows()
        {
            return Err(OwnershipError::CapacityContractMismatch);
        }
        Ok((value(RECEIPT_COUNT)?, value(REVISION_COUNT)?))
    }
}
pub(super) fn write_reader(
    txn: &redb::WriteTransaction,
    capacity: OwnershipCapacity,
) -> OwnershipResult<WriteReader<'_>> {
    Ok(Reader {
        uuid: uuid_entities::write_reader(txn).map_err(backend)?,
        players: txn.open_table(PLAYER_TABLE).map_err(backend)?,
        player_headers: txn.open_table(PLAYER_OWNER).map_err(backend)?,
        entity_headers: txn.open_table(ENTITY_OWNER).map_err(backend)?,
        receipts: txn.open_table(RECEIPT).map_err(backend)?,
        meta: txn.open_table(META_TABLE).map_err(backend)?,
        capacity,
    })
}
fn read_reader(
    txn: &redb::ReadTransaction,
    capacity: OwnershipCapacity,
) -> OwnershipResult<ReadReader> {
    Ok(Reader {
        uuid: uuid_entities::Reader {
            entities: txn.open_table(ENTITY_TABLE).map_err(backend)?,
            locators: txn
                .open_table(uuid_entities::LOCATOR_TABLE)
                .map_err(backend)?,
            indexes: txn
                .open_table(uuid_entities::INDEX_TABLE)
                .map_err(backend)?,
            epochs: txn
                .open_table(uuid_entities::EPOCH_TABLE)
                .map_err(backend)?,
            generations: txn.open_table(GENERATION_TABLE).map_err(backend)?,
        },
        players: txn.open_table(PLAYER_TABLE).map_err(backend)?,
        player_headers: txn.open_table(PLAYER_OWNER).map_err(backend)?,
        entity_headers: txn.open_table(ENTITY_OWNER).map_err(backend)?,
        receipts: txn.open_table(RECEIPT).map_err(backend)?,
        meta: txn.open_table(META_TABLE).map_err(backend)?,
        capacity,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cutpoint {
    BeforePlan,
    BeforeRow,
    AfterRow,
    BeforeCommit,
    AfterCommit,
}
fn row(
    txn: &redb::WriteTransaction,
    table: TableDefinition<'_, &[u8], &[u8]>,
    key: &[u8],
    value: Option<&[u8]>,
    hook: &mut impl FnMut(Cutpoint) -> OwnershipResult<()>,
) -> OwnershipResult<()> {
    let mut t = txn.open_table(table).map_err(backend)?;
    hook(Cutpoint::BeforeRow)?;
    if let Some(v) = value {
        t.insert(key, v).map_err(backend)?;
    } else {
        t.remove(key).map_err(backend)?;
    }
    hook(Cutpoint::AfterRow)
}
fn apply(
    txn: &redb::WriteTransaction,
    plan: engine::Plan,
    hook: &mut impl FnMut(Cutpoint) -> OwnershipResult<()>,
) -> OwnershipResult<()> {
    if let Some(p) = plan.player {
        row(txn, PLAYER_TABLE, &p.key, p.record.as_deref(), hook)?;
        row(
            txn,
            PLAYER_OWNER,
            &p.key,
            Some(&raw::encode_player_header(p.header)),
            hook,
        )?;
    }
    for change in plan.entity.changes {
        if let Some(old) = change.old_chunk {
            row(
                txn,
                uuid_entities::INDEX_TABLE,
                &uuid_raw::index_key(change.key, old.pos()),
                None,
                hook,
            )?;
        }
        if let Some((chunk, bytes)) = change.new {
            row(
                txn,
                ENTITY_TABLE,
                &uuid_raw::key_bytes(change.key),
                Some(&bytes),
                hook,
            )?;
            row(
                txn,
                uuid_entities::LOCATOR_TABLE,
                &uuid_raw::locator_key(change.key),
                Some(&uuid_raw::locator_value(change.key, chunk.pos())),
                hook,
            )?;
            row(
                txn,
                uuid_entities::INDEX_TABLE,
                &uuid_raw::index_key(change.key, chunk.pos()),
                Some(&[]),
                hook,
            )?;
        } else {
            row(
                txn,
                ENTITY_TABLE,
                &uuid_raw::key_bytes(change.key),
                None,
                hook,
            )?;
            row(
                txn,
                uuid_entities::LOCATOR_TABLE,
                &uuid_raw::locator_key(change.key),
                None,
                hook,
            )?;
        }
    }
    for (chunk, epoch) in plan.entity.epochs {
        row(
            txn,
            uuid_entities::EPOCH_TABLE,
            &crate::codec::chunk_key_bytes(chunk),
            Some(&epoch.to_be_bytes()),
            hook,
        )?;
    }
    if let Some((key, h)) = plan.entity_header {
        row(
            txn,
            ENTITY_OWNER,
            &key,
            Some(&raw::encode_entity_header(h)),
            hook,
        )?;
    }
    row(
        txn,
        RECEIPT,
        &plan.receipt.query.id.to_bytes(),
        Some(&raw::encode_receipt(&plan.receipt)),
        hook,
    )?;
    let mut meta = txn.open_table(META_TABLE).map_err(backend)?;
    for (key, value) in [
        (RECEIPT_COUNT, plan.receipt_count),
        (REVISION_COUNT, plan.revision_count),
    ] {
        hook(Cutpoint::BeforeRow)?;
        meta.insert(key, value).map_err(backend)?;
        hook(Cutpoint::AfterRow)?;
    }
    Ok(())
}
/// Actual producer hookpoints; returned precommit errors drop/abort this transaction.
pub(super) fn commit(
    db: &Database,
    owner: &Arc<()>,
    capacity: OwnershipCapacity,
    request: Request,
    mut hook: impl FnMut(Cutpoint) -> OwnershipResult<()>,
) -> OwnershipResult<OwnershipCommitOutcome> {
    let mut txn = db.begin_write().map_err(backend)?;
    txn.set_durability(Durability::Immediate);
    hook(Cutpoint::BeforePlan)?;
    let prepared = {
        let reader = write_reader(&txn, capacity)?;
        engine::prepare(&reader, owner, capacity, request)?
    };
    match prepared {
        engine::Prepared::Replay(r) => Ok(engine::outcome(
            *r,
            owner,
            OwnershipCommitDisposition::Replayed,
        )),
        engine::Prepared::Commit(plan) => {
            let receipt = plan.receipt.clone();
            apply(&txn, *plan, &mut hook)?;
            hook(Cutpoint::BeforeCommit)?;
            txn.commit().map_err(|e| OwnershipError::CommitUncertain {
                query: receipt.query(),
                cause: e.to_string(),
            })?;
            hook(Cutpoint::AfterCommit).map_err(|e| OwnershipError::ExecutionUncertain {
                query: receipt.query(),
                cause: e.to_string(),
            })?;
            Ok(engine::outcome(
                receipt,
                owner,
                OwnershipCommitDisposition::Committed,
            ))
        }
    }
}
impl RedbStore {
    /// Opens only fresh or fully valid Format6 with the exact persisted capacity agreement.
    /// Synchronous startup I/O belongs outside an async/simulation hot path.
    pub fn open_with_player_entity_ownership(
        path: impl AsRef<Path>,
        config: OwnershipConfig,
    ) -> OwnershipResult<Self> {
        let path = path.as_ref();
        let mut builder = Database::builder();
        builder.set_cache_size(64 * 1024 * 1024);
        let (db, origin) = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => (
                builder.create_file(file).map_err(backend)?,
                StoreOrigin::Fresh,
            ),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                (builder.open(path).map_err(backend)?, StoreOrigin::Existing)
            }
            Err(e) => return Err(backend(e)),
        };
        super::ownership_validation::initialize(&db, origin, path, config)?;
        Ok(Self {
            db: Arc::new(db),
            generation_mode: crate::generation::GenerationMode::Ownership,
            entity_incarnation: Arc::new(()),
            ownership_runtime: Some(Arc::new(Runtime::new(config))),
        })
    }
    fn runtime(&self) -> OwnershipResult<&Arc<Runtime>> {
        self.ownership_runtime
            .as_ref()
            .filter(|_| self.generation_mode == crate::generation::GenerationMode::Ownership)
            .ok_or(OwnershipError::Unsupported)
    }
    async fn read_owned<T: Send + 'static>(
        &self,
        f: impl FnOnce(&ReadReader, &Arc<()>) -> OwnershipResult<T> + Send + 'static,
    ) -> OwnershipResult<T> {
        let runtime = self.runtime()?;
        let permit = runtime.permit()?;
        let capacity = runtime.config.capacity();
        let db = self.db.clone();
        let owner = self.entity_incarnation.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let txn = db.begin_read().map_err(backend)?;
            f(&read_reader(&txn, capacity)?, &owner)
        })
        .await
        .map_err(backend)?
    }
    async fn write_owned(&self, request: Request) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned_hooks(request, |_| Ok(())).await
    }
    async fn write_owned_hooks(
        &self,
        request: Request,
        hook: impl FnMut(Cutpoint) -> OwnershipResult<()> + Send + 'static,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        let runtime = self.runtime()?;
        request.admission(runtime.config.capacity().max_request_bytes())?;
        let permit = runtime.permit()?;
        let capacity = runtime.config.capacity();
        let db = self.db.clone();
        let owner = self.entity_incarnation.clone();
        let query = request.query();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            commit(&db, &owner, capacity, request, hook)
        })
        .await
        .map_err(|e| OwnershipError::ExecutionUncertain {
            query,
            cause: e.to_string(),
        })?
    }
    pub(super) async fn owned_uuid_page(
        &self,
        chunk: ChunkKey,
        expected: GenerationIdentity,
        cursor: Option<PersistentEntityCursor>,
        limit: std::num::NonZeroU16,
    ) -> OwnershipResult<PersistentEntityPage> {
        self.read_owned(move |r, owner| {
            Ok(uuid_engine::page(
                &engine::EntityAdapter(r),
                owner,
                chunk,
                expected,
                cursor.as_ref(),
                limit,
            )?)
        })
        .await
    }
}
#[async_trait]
impl PlayerEntityOwnershipStore for RedbStore {
    async fn load_owned_player(
        &self,
        key: ScopedPlayerKey,
        expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerSnapshot> {
        self.read_owned(move |r, o| engine::load_player(r, o, key, expected))
            .await
    }
    async fn load_owned_entity(
        &self,
        key: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedEntitySnapshot> {
        self.read_owned(move |r, o| engine::load_entity(r, o, key, expected))
            .await
    }
    async fn load_owned_pair(
        &self,
        player: ScopedPlayerKey,
        entity: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerEntitySnapshot> {
        self.read_owned(move |r, o| engine::load_pair(r, o, player, entity, expected))
            .await
    }
    async fn commit_player_entity_transfer(
        &self,
        request: PlayerEntityTransferRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned(request.0).await
    }
    async fn compare_exchange_owned_player(
        &self,
        request: PlayerOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned(request.0).await
    }
    async fn compare_exchange_owned_entity(
        &self,
        request: EntityOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned(request.0).await
    }
    async fn load_ownership_receipt(
        &self,
        query: OwnershipReceiptQuery,
    ) -> OwnershipResult<Option<OwnershipReceipt>> {
        self.read_owned(move |r, _| engine::lookup_receipt(r, query))
            .await
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::{
        EntityOwnershipProposal, OwnershipResource, OwnershipStamp, PersistentEntityStore,
        PlayerOwnershipProposal, WorldStore,
    };
    use std::collections::BTreeMap;
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering};

    use redb::{backends::FileBackend, StorageBackend, TableHandle};

    use super::*;
    use crate::ownership::tests::{config, e, entity, id, identity, p, player, scope};
    fn force_player_max(
        txn: &redb::WriteTransaction,
        receipt: &mut OwnershipReceipt,
        tombstone: bool,
    ) {
        let mut owners = txn.open_table(PLAYER_OWNER).unwrap();
        let key = raw::player_key(player());
        let mut header =
            raw::player_header(owners.get(key.as_slice()).unwrap().unwrap().value()).unwrap();
        header.stamp = OwnershipStamp::checked(if tombstone { 2 } else { 1 }, u64::MAX).unwrap();
        owners
            .insert(key.as_slice(), raw::encode_player_header(header).as_slice())
            .unwrap();
        receipt.before_player = OwnershipStamp::checked(1, u64::MAX - 1).unwrap();
        receipt.after_player = header.stamp;
        if tombstone {
            txn.open_table(PLAYER_TABLE)
                .unwrap()
                .remove(key.as_slice())
                .unwrap();
        }
    }
    fn force_entity_max(
        txn: &redb::WriteTransaction,
        receipt: &mut OwnershipReceipt,
        tombstone: bool,
    ) {
        let mut owners = txn.open_table(ENTITY_OWNER).unwrap();
        let key = raw::entity_key(entity());
        let mut header =
            raw::entity_header(&key, owners.get(key.as_slice()).unwrap().unwrap().value()).unwrap();
        header.stamp = OwnershipStamp::checked(if tombstone { 2 } else { 1 }, u64::MAX).unwrap();
        owners
            .insert(key.as_slice(), raw::encode_entity_header(header).as_slice())
            .unwrap();
        receipt.before_entity = OwnershipStamp::checked(1, u64::MAX - 1).unwrap();
        receipt.after_entity = header.stamp;
        receipt.before_chunk = receipt.after_chunk;
        receipt.epochs[0].before = 1;
        receipt.epochs[0].after = 2;
        if tombstone {
            receipt.after_chunk = None;
            txn.open_table(ENTITY_TABLE)
                .unwrap()
                .remove(uuid_raw::key_bytes(entity()).as_slice())
                .unwrap();
            txn.open_table(uuid_entities::LOCATOR_TABLE)
                .unwrap()
                .remove(key.as_slice())
                .unwrap();
            txn.open_table(uuid_entities::INDEX_TABLE)
                .unwrap()
                .remove(
                    uuid_raw::index_key(entity(), ferrumc_math::ChunkPos::new(0, -2)).as_slice(),
                )
                .unwrap();
        }
    }
    fn force_max_boundary(store: &RedbStore, case: u8) {
        let txn = store.db.begin_write().unwrap();
        let mut receipts = txn.open_table(RECEIPT).unwrap();
        let receipt_key = id(1).to_bytes();
        let mut receipt = raw::receipt(
            &receipt_key,
            receipts
                .get(receipt_key.as_slice())
                .unwrap()
                .unwrap()
                .value(),
        )
        .unwrap();
        match case {
            0 | 3 => force_player_max(&txn, &mut receipt, case == 3),
            1 | 4 => force_entity_max(&txn, &mut receipt, case == 4),
            2 => {
                receipt.epochs[0].before = u64::MAX - 1;
                receipt.epochs[0].after = u64::MAX;
            }
            5 => {
                txn.open_table(uuid_entities::EPOCH_TABLE)
                    .unwrap()
                    .insert(
                        crate::codec::chunk_key_bytes(ChunkKey::new(
                            scope().world(),
                            scope().dimension(),
                            ferrumc_math::ChunkPos::new(1, -2),
                        ))
                        .as_slice(),
                        u64::MAX.to_be_bytes().as_slice(),
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        txn.open_table(uuid_entities::EPOCH_TABLE)
            .unwrap()
            .insert(
                crate::codec::chunk_key_bytes(receipt.epochs[0].chunk).as_slice(),
                receipt.epochs[0].after.to_be_bytes().as_slice(),
            )
            .unwrap();
        receipts
            .insert(
                receipt_key.as_slice(),
                raw::encode_receipt(&receipt).as_slice(),
            )
            .unwrap();
        drop(receipts);
        txn.commit().unwrap();
    }
    #[tokio::test]
    async fn persisted_player_entity_revision_and_epoch_max_never_wrap() {
        let dir = tempfile::tempdir().unwrap();
        for case in [0, 1, 2, 5] {
            let path = dir.path().join(format!("max-{case}.redb"));
            let store = setup(&path).await;
            force_max_boundary(&store, case);
            drop(store);
            let store = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
            let noop = request(&store, 2, 0, 1).await;
            assert_eq!(
                store
                    .write_owned(noop.clone())
                    .await
                    .unwrap()
                    .receipt()
                    .epochs(),
                []
            );
            let before = snapshot(&store);
            let changed = request(&store, 3, 1, 2).await;
            let query = changed.query();
            let result = store.write_owned(changed).await;
            match case {
                0 => assert!(matches!(
                    result,
                    Err(OwnershipError::RevisionExhausted {
                        resource: OwnershipResource::Player
                    })
                )),
                1 => assert!(matches!(
                    result,
                    Err(OwnershipError::RevisionExhausted {
                        resource: OwnershipResource::Entity
                    })
                )),
                _ => assert!(matches!(
                    result,
                    Err(OwnershipError::Record(
                        StorageError::PersistentEntityEpochExhausted { .. }
                    ))
                )),
            }
            assert_eq!(snapshot(&store), before);
            assert!(store.load_ownership_receipt(query).await.unwrap().is_none());
            drop(store);
            let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
            assert_eq!(snapshot(&reopened), before);
            assert_eq!(
                reopened.write_owned(noop).await.unwrap().disposition(),
                OwnershipCommitDisposition::Replayed
            );
        }
    }
    #[tokio::test]
    async fn retained_player_and_entity_tombstones_at_max_cannot_delete_or_recreate() {
        let dir = tempfile::tempdir().unwrap();
        for case in [3, 4] {
            let path = dir.path().join(format!("tomb-max-{case}.redb"));
            let store = setup(&path).await;
            force_max_boundary(&store, case);
            drop(store);
            let store = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
            let pair = store
                .load_owned_pair(player(), entity(), identity())
                .await
                .unwrap();
            let before = snapshot(&store);
            let (delete, put, queries) = if case == 3 {
                assert!(pair.player().stamp().is_tombstone());
                let delete = PlayerOwnershipCasRequest::new(
                    id(2),
                    pair.player().expectation().clone(),
                    PlayerOwnershipProposal::delete(),
                )
                .unwrap();
                let put = PlayerOwnershipCasRequest::new(
                    id(3),
                    pair.player().expectation().clone(),
                    PlayerOwnershipProposal::put(p(2)),
                )
                .unwrap();
                let queries = [delete.receipt_query(), put.receipt_query()];
                (
                    store.compare_exchange_owned_player(delete).await,
                    store.compare_exchange_owned_player(put).await,
                    queries,
                )
            } else {
                assert!(pair.entity().stamp().is_tombstone());
                let delete = EntityOwnershipCasRequest::new(
                    id(2),
                    pair.entity().expectation().clone(),
                    EntityOwnershipProposal::delete(),
                )
                .unwrap();
                let put = EntityOwnershipCasRequest::new(
                    id(3),
                    pair.entity().expectation().clone(),
                    EntityOwnershipProposal::put(e(0, 2)),
                )
                .unwrap();
                let queries = [delete.receipt_query(), put.receipt_query()];
                (
                    store.compare_exchange_owned_entity(delete).await,
                    store.compare_exchange_owned_entity(put).await,
                    queries,
                )
            };
            assert!(matches!(delete, Err(OwnershipError::AlreadyAbsent { .. })));
            assert!(matches!(put, Err(OwnershipError::RevisionExhausted { .. })));
            for query in queries {
                assert!(store.load_ownership_receipt(query).await.unwrap().is_none());
            }
            assert_eq!(snapshot(&store), before);
            drop(store);
            let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
            assert_eq!(snapshot(&reopened), before);
        }
    }
    #[tokio::test]
    async fn readonly_uuid_points_and_page_lookahead_validate_the_unified_owner_state() {
        let dir = tempfile::tempdir().unwrap();
        let store = setup(&dir.path().join("page.redb")).await;
        let second = PersistentEntityKey::new(
            scope().world(),
            scope().dimension(),
            ferrumc_core::EntityUuid::from_bytes([0x13; 16]),
        );
        let before = store.load_owned_entity(second, identity()).await.unwrap();
        store
            .compare_exchange_owned_entity(
                EntityOwnershipCasRequest::new(
                    id(2),
                    before.expectation().clone(),
                    EntityOwnershipProposal::put(e(0, 2)),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let chunk = ChunkKey::new(
            scope().world(),
            scope().dimension(),
            ferrumc_math::ChunkPos::new(0, -2),
        );
        let page = store
            .load_persistent_entity_page(
                chunk,
                identity(),
                None,
                std::num::NonZeroU16::new(1).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(page.entries().len(), 1);
        assert!(page.next().is_some());
        let txn = store.db.begin_write().unwrap();
        txn.open_table(ENTITY_OWNER)
            .unwrap()
            .remove(raw::entity_key(second).as_slice())
            .unwrap();
        txn.commit().unwrap();
        let raw_before = snapshot(&store);
        assert!(store
            .load_persistent_entity(second, identity())
            .await
            .is_err());
        assert!(store.load_owned_entity(second, identity()).await.is_err());
        assert!(store
            .load_persistent_entity_page(
                chunk,
                identity(),
                None,
                std::num::NonZeroU16::new(1).unwrap()
            )
            .await
            .is_err());
        assert!(store
            .load_persistent_entity_page(
                chunk,
                identity(),
                page.next().cloned(),
                std::num::NonZeroU16::new(1).unwrap()
            )
            .await
            .is_err());
        assert_eq!(snapshot(&store), raw_before);
    }
    #[tokio::test]
    async fn cancelled_caller_cannot_release_the_running_real_worker_permit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = setup(&dir.path().join("cancel.redb")).await;
        let request = request(&store, 2, 1, 2).await;
        let retry = request.clone();
        let query = request.query();
        store.ownership_runtime = Some(Arc::new(Runtime::new(
            crate::ownership::tests::config_inflight(1),
        )));
        let store = Arc::new(store);
        let writer_store = store.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        // Exactly one release signal; blocking occurs only in the actual DB worker.
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        let caller = tokio::spawn(async move {
            let mut entered = Some(entered_tx);
            writer_store
                .write_owned_hooks(request, move |point| {
                    if point == Cutpoint::BeforeRow {
                        if let Some(tx) = entered.take() {
                            tx.send(()).unwrap();
                            release_rx.recv().unwrap();
                        }
                    }
                    Ok(())
                })
                .await
        });
        entered_rx.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        assert!(matches!(
            store.load_owned_player(player(), identity()).await,
            Err(OwnershipError::Busy)
        ));
        assert!(matches!(
            store.load_ownership_receipt(query).await,
            Err(OwnershipError::Busy)
        ));
        release_tx.send(()).unwrap();
        crate::ownership::tests::wait_for_permit(store.runtime().unwrap()).await;
        let receipt = store.load_ownership_receipt(query).await.unwrap().unwrap();
        let outcome = store.write_owned(retry).await.unwrap();
        assert_eq!(outcome.disposition(), OwnershipCommitDisposition::Replayed);
        assert_eq!(outcome.receipt(), &receipt);
    }

    type Rows = Vec<(Vec<u8>, Vec<u8>)>;
    pub(in crate::redb_store) fn snapshot(store: &RedbStore) -> BTreeMap<String, Rows> {
        let txn = store.db.begin_read().unwrap();
        let mut snapshot = BTreeMap::new();
        for definition in txn.list_tables().unwrap() {
            let name = definition.name();
            let rows = if name == META_TABLE.name() {
                let table = txn.open_table(META_TABLE).unwrap();
                table
                    .iter()
                    .unwrap()
                    .map(|row| {
                        let (k, v) = row.unwrap();
                        (
                            k.value().as_bytes().to_vec(),
                            v.value().to_be_bytes().to_vec(),
                        )
                    })
                    .collect()
            } else {
                let table = txn
                    .open_table(TableDefinition::<&[u8], &[u8]>::new(name))
                    .unwrap();
                table
                    .iter()
                    .unwrap()
                    .map(|row| {
                        let (k, v) = row.unwrap();
                        (k.value().to_vec(), v.value().to_vec())
                    })
                    .collect()
            };
            snapshot.insert(name.to_owned(), rows);
        }
        snapshot
    }
    async fn request(store: &RedbStore, n: u8, x: i32, body: u8) -> Request {
        let pair = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        PlayerEntityTransferRequest::new(
            id(n),
            pair.player().expectation().clone(),
            pair.entity().expectation().clone(),
            PlayerOwnershipProposal::put(p(body)),
            EntityOwnershipProposal::put(e(x, body)),
        )
        .unwrap()
        .0
    }
    pub(in crate::redb_store) async fn setup(path: &Path) -> RedbStore {
        let store = RedbStore::open_with_player_entity_ownership(path, config()).unwrap();
        store
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let r = request(&store, 1, 0, 1).await;
        store.write_owned(r).await.unwrap();
        store
    }

    #[tokio::test]
    async fn every_actual_row_and_precommit_cutpoint_aborts_all_fifteen_tables() {
        let dir = tempfile::tempdir().unwrap();
        let count_store = setup(&dir.path().join("count.redb")).await;
        let r = request(&count_store, 2, 1, 2).await;
        let mut count = 0;
        commit(
            &count_store.db,
            &count_store.entity_incarnation,
            config().capacity(),
            r,
            |point| {
                if point != Cutpoint::AfterCommit {
                    count += 1;
                }
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            count, 26,
            "12 actual row writes each before/after, plus before plan/commit"
        );
        drop(count_store);
        for stop in 0..count {
            let path = dir.path().join(format!("abort-{stop}.redb"));
            let store = setup(&path).await;
            let before = snapshot(&store);
            let r = request(&store, 2, 1, 2).await;
            let query = r.query();
            let mut visited = 0;
            let result = commit(
                &store.db,
                &store.entity_incarnation,
                config().capacity(),
                r,
                |_| {
                    let current = visited;
                    visited += 1;
                    if current == stop {
                        Err(OwnershipError::Backend("actual cutpoint refusal".into()))
                    } else {
                        Ok(())
                    }
                },
            );
            assert!(
                matches!(result, Err(OwnershipError::Backend(_))),
                "cutpoint{stop}"
            );
            assert_eq!(snapshot(&store), before, "cutpoint{stop}");
            assert!(store.load_ownership_receipt(query).await.unwrap().is_none());
            drop(store);
            let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
            assert_eq!(snapshot(&reopened), before, "reopen cutpoint{stop}");
        }
    }
    #[tokio::test]
    async fn committed_lost_reply_and_actual_worker_panic_replay_original_evidence() {
        for panic in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("lost.redb");
            let store = setup(&path).await;
            let r = request(&store, 2, 1, 2).await;
            let retry = r.clone();
            let query = r.query();
            let result = store
                .write_owned_hooks(r, move |point| {
                    if point == Cutpoint::AfterCommit {
                        assert!(!panic, "injected actual worker panic after commit");
                        return Err(OwnershipError::Backend("lost acknowledgement".into()));
                    }
                    Ok(())
                })
                .await;
            assert!(
                matches!(result,Err(OwnershipError::ExecutionUncertain {query:actual,..}) if actual==query)
            );
            let receipt = store.load_ownership_receipt(query).await.unwrap().unwrap();
            assert_eq!(
                store.write_owned(retry.clone()).await.unwrap().receipt(),
                &receipt
            );
            let final_rows = snapshot(&store);
            drop(store);
            let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
            let outcome = reopened.write_owned(retry).await.unwrap();
            assert_eq!(outcome.disposition(), OwnershipCommitDisposition::Replayed);
            assert_eq!(outcome.receipt(), &receipt);
            assert_eq!(snapshot(&reopened), final_rows);
        }
    }

    #[derive(Debug)]
    struct SyncFault {
        inner: FileBackend,
        fail: Arc<AtomicBool>,
    }
    impl StorageBackend for SyncFault {
        fn len(&self) -> io::Result<u64> {
            self.inner.len()
        }
        fn read(&self, offset: u64, len: usize) -> io::Result<Vec<u8>> {
            self.inner.read(offset, len)
        }
        fn set_len(&self, len: u64) -> io::Result<()> {
            self.inner.set_len(len)
        }
        fn write(&self, offset: u64, bytes: &[u8]) -> io::Result<()> {
            self.inner.write(offset, bytes)
        }
        fn sync_data(&self, eventual: bool) -> io::Result<()> {
            if self.fail.swap(false, Ordering::SeqCst) {
                return Err(io::Error::other("injected actual commit sync failure"));
            }
            self.inner.sync_data(eventual)
        }
    }
    #[tokio::test]
    async fn actual_commit_io_failure_is_uncertain_and_checked_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("io.redb");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let fail = Arc::new(AtomicBool::new(false));
        let db = Database::builder()
            .set_cache_size(64 * 1024 * 1024)
            .create_with_backend(SyncFault {
                inner: FileBackend::new(file).unwrap(),
                fail: fail.clone(),
            })
            .unwrap();
        super::super::ownership_validation::initialize(&db, StoreOrigin::Fresh, &path, config())
            .unwrap();
        let store = RedbStore {
            db: Arc::new(db),
            generation_mode: crate::generation::GenerationMode::Ownership,
            entity_incarnation: Arc::new(()),
            ownership_runtime: Some(Arc::new(Runtime::new(config()))),
        };
        store
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let r = request(&store, 1, 0, 1).await;
        let query = r.query();
        let result = store
            .write_owned_hooks(r.clone(), move |point| {
                if point == Cutpoint::BeforeCommit {
                    fail.store(true, Ordering::SeqCst);
                }
                Ok(())
            })
            .await;
        assert!(
            matches!(result,Err(OwnershipError::CommitUncertain {query:actual,..}) if actual==query)
        );
        drop(store);
        let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
        let prior = reopened.load_ownership_receipt(query).await.unwrap();
        let retry = if prior.is_some() {
            r
        } else {
            request(&reopened, 1, 0, 1).await
        };
        assert_eq!(retry.query(), query);
        let outcome = reopened.write_owned(retry.clone()).await.unwrap();
        assert_eq!(
            outcome.disposition(),
            if prior.is_some() {
                OwnershipCommitDisposition::Replayed
            } else {
                OwnershipCommitDisposition::Committed
            }
        );
        assert_eq!(
            reopened.write_owned(retry).await.unwrap().receipt(),
            outcome.receipt()
        );
    }
}
