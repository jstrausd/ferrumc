//! UUID runtime transactions; every blocking redb access stays off Tokio workers.

use std::num::NonZeroU16;
use std::ops::Bound;

use async_trait::async_trait;
use ferrumc_core::Result;
use ferrumc_world::GenerationIdentity;
use redb::{AccessGuard, ReadableTable, TableDefinition};

use super::{backend_err, join_err, RedbStore, ENTITY_TABLE, GENERATION_TABLE};
use crate::generation::decode_identity;
use crate::persistent_entity::engine::{self, EntityRead, Plan, RawRows, RawTable};
use crate::persistent_entity::{raw, require_mode};
use crate::{
    ChunkKey, GenerationKey, PersistentEntityCursor, PersistentEntityKey, PersistentEntityPage,
    PersistentEntityRecord, PersistentEntityStore, PersistentEntityWrite, StorageError,
};

pub(super) const LOCATOR_TABLE: TableDefinition<'_, &[u8], &[u8]> =
    TableDefinition::new(raw::LOCATOR);
pub(super) const INDEX_TABLE: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new(raw::INDEX);
pub(super) const EPOCH_TABLE: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new(raw::EPOCH);

fn storage_backend(error: impl std::fmt::Display) -> StorageError {
    StorageError::backend(error.to_string())
}

pub(super) struct RawGuard<'a>(pub(super) AccessGuard<'a, &'static [u8]>);
impl AsRef<[u8]> for RawGuard<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0.value()
    }
}

pub(super) struct Reader<T> {
    pub(super) entities: T,
    pub(super) locators: T,
    pub(super) indexes: T,
    pub(super) epochs: T,
    pub(super) generations: T,
}
impl<T: ReadableTable<&'static [u8], &'static [u8]>> EntityRead for Reader<T> {
    type Bytes<'a>
        = RawGuard<'a>
    where
        Self: 'a;
    fn get(
        &self,
        table: RawTable,
        key: &[u8],
    ) -> std::result::Result<Option<Self::Bytes<'_>>, StorageError> {
        let table = match table {
            RawTable::Entity => &self.entities,
            RawTable::Locator => &self.locators,
            RawTable::Index => &self.indexes,
            RawTable::Epoch => &self.epochs,
        };
        table
            .get(key)
            .map(|value| value.map(RawGuard))
            .map_err(storage_backend)
    }
    fn identity(
        &self,
        key: GenerationKey,
    ) -> std::result::Result<Option<GenerationIdentity>, StorageError> {
        self.generations
            .get(key.encode().as_slice())
            .map_err(storage_backend)?
            .map(|bytes| decode_identity(key, bytes.value()))
            .transpose()
    }
    fn index_rows(
        &self,
        lower: Bound<Vec<u8>>,
        upper: Bound<Vec<u8>>,
    ) -> std::result::Result<RawRows<'_, Self::Bytes<'_>>, StorageError> {
        let range = self
            .indexes
            .range::<&[u8]>((
                lower.as_ref().map(Vec::as_slice),
                upper.as_ref().map(Vec::as_slice),
            ))
            .map_err(storage_backend)?;
        Ok(Box::new(range.map(|entry| {
            entry
                .map(|(key, value)| (RawGuard(key), RawGuard(value)))
                .map_err(storage_backend)
        })))
    }
}

pub(super) fn write_reader(
    txn: &redb::WriteTransaction,
) -> Result<Reader<redb::Table<'_, &'static [u8], &'static [u8]>>> {
    Ok(Reader {
        entities: txn.open_table(ENTITY_TABLE).map_err(backend_err)?,
        locators: txn.open_table(LOCATOR_TABLE).map_err(backend_err)?,
        indexes: txn.open_table(INDEX_TABLE).map_err(backend_err)?,
        epochs: txn.open_table(EPOCH_TABLE).map_err(backend_err)?,
        generations: txn.open_table(GENERATION_TABLE).map_err(backend_err)?,
    })
}

fn read_reader(
    txn: &redb::ReadTransaction,
) -> Result<Reader<redb::ReadOnlyTable<&'static [u8], &'static [u8]>>> {
    Ok(Reader {
        entities: txn.open_table(ENTITY_TABLE).map_err(backend_err)?,
        locators: txn.open_table(LOCATOR_TABLE).map_err(backend_err)?,
        indexes: txn.open_table(INDEX_TABLE).map_err(backend_err)?,
        epochs: txn.open_table(EPOCH_TABLE).map_err(backend_err)?,
        generations: txn.open_table(GENERATION_TABLE).map_err(backend_err)?,
    })
}

/// The private hook aborts returned-error tests at the actual producer cutpoints.
pub(super) fn apply_plan(
    txn: &redb::WriteTransaction,
    plan: Plan,
    mut after_write: impl FnMut() -> std::result::Result<(), StorageError>,
) -> Result<()> {
    let mut entities = txn.open_table(ENTITY_TABLE).map_err(backend_err)?;
    let mut locators = txn.open_table(LOCATOR_TABLE).map_err(backend_err)?;
    let mut indexes = txn.open_table(INDEX_TABLE).map_err(backend_err)?;
    let mut epochs = txn.open_table(EPOCH_TABLE).map_err(backend_err)?;
    for change in plan.changes {
        if let Some(old) = change.old_chunk {
            indexes
                .remove(raw::index_key(change.key, old.pos()).as_slice())
                .map_err(backend_err)?;
            after_write()?;
        }
        if let Some((chunk, bytes)) = change.new {
            entities
                .insert(raw::key_bytes(change.key).as_slice(), bytes.as_slice())
                .map_err(backend_err)?;
            after_write()?;
            locators
                .insert(
                    raw::locator_key(change.key).as_slice(),
                    raw::locator_value(change.key, chunk.pos()).as_slice(),
                )
                .map_err(backend_err)?;
            after_write()?;
            indexes
                .insert(
                    raw::index_key(change.key, chunk.pos()).as_slice(),
                    [].as_slice(),
                )
                .map_err(backend_err)?;
            after_write()?;
        } else {
            entities
                .remove(raw::key_bytes(change.key).as_slice())
                .map_err(backend_err)?;
            after_write()?;
            locators
                .remove(raw::locator_key(change.key).as_slice())
                .map_err(backend_err)?;
            after_write()?;
        }
    }
    for (chunk, epoch) in plan.epochs {
        epochs
            .insert(
                crate::codec::chunk_key_bytes(chunk).as_slice(),
                epoch.to_be_bytes().as_slice(),
            )
            .map_err(backend_err)?;
        after_write()?;
    }
    Ok(())
}

/// The reply cutpoint distinguishes durable success from a lost acknowledgement.
pub(super) fn commit_plan(
    txn: redb::WriteTransaction,
    plan: Plan,
    after_commit: impl FnOnce() -> std::result::Result<(), StorageError>,
) -> Result<()> {
    apply_plan(&txn, plan, || Ok(()))?;
    txn.commit().map_err(backend_err)?;
    after_commit()?;
    Ok(())
}

#[async_trait]
impl PersistentEntityStore for RedbStore {
    async fn load_persistent_entity(
        &self,
        key: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> Result<Option<PersistentEntityRecord>> {
        if self.generation_mode == crate::generation::GenerationMode::Ownership {
            return Ok(
                crate::PlayerEntityOwnershipStore::load_owned_entity(self, key, expected)
                    .await?
                    .record,
            );
        }
        require_mode(self.generation_mode)?;
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read().map_err(backend_err)?;
            let reader = read_reader(&txn)?;
            engine::require_identity(&reader, engine::scope(key), Some(expected))?;
            let result = engine::lookup(&reader, key)?
                .filter(|entity| entity.key == key)
                .map(|entity| raw::envelope(entity.bytes.as_ref())?.owned())
                .transpose()
                .map_err(Into::into);
            result
        })
        .await
        .map_err(join_err)?
    }
    async fn save_persistent_entity(&self, write: PersistentEntityWrite) -> Result<()> {
        self.save_persistent_entities(vec![write]).await
    }
    async fn save_persistent_entities(&self, writes: Vec<PersistentEntityWrite>) -> Result<()> {
        require_mode(self.generation_mode)?;
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let writes = engine::prepare(writes)?;
            if writes.is_empty() {
                return Ok(());
            }
            let txn = db.begin_write().map_err(backend_err)?;
            let plan = engine::save_plan(&write_reader(&txn)?, writes)?;
            if plan.changes.is_empty() {
                return Ok(());
            }
            commit_plan(txn, plan, || Ok(()))
        })
        .await
        .map_err(join_err)?
    }
    async fn delete_persistent_entity(
        &self,
        key: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> Result<bool> {
        require_mode(self.generation_mode)?;
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write().map_err(backend_err)?;
            let (plan, removed) = engine::delete_plan(&write_reader(&txn)?, key, expected)?;
            if removed {
                commit_plan(txn, plan, || Ok(()))?;
            }
            Ok(removed)
        })
        .await
        .map_err(join_err)?
    }
    async fn move_persistent_entity(
        &self,
        source: PersistentEntityKey,
        source_expected: GenerationIdentity,
        destination: PersistentEntityWrite,
    ) -> Result<()> {
        require_mode(self.generation_mode)?;
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let destination = engine::prepare(vec![destination])?.pop().ok_or(
                StorageError::InvalidPersistentEntityMove {
                    reason: "missing destination proposal",
                },
            )?;
            let txn = db.begin_write().map_err(backend_err)?;
            let plan =
                engine::move_plan(&write_reader(&txn)?, source, source_expected, destination)?;
            commit_plan(txn, plan, || Ok(()))
        })
        .await
        .map_err(join_err)?
    }
    async fn load_persistent_entity_page(
        &self,
        chunk: ChunkKey,
        expected: GenerationIdentity,
        cursor: Option<PersistentEntityCursor>,
        limit: NonZeroU16,
    ) -> Result<PersistentEntityPage> {
        if self.generation_mode == crate::generation::GenerationMode::Ownership {
            return self
                .owned_uuid_page(chunk, expected, cursor, limit)
                .await
                .map_err(Into::into);
        }
        require_mode(self.generation_mode)?;
        let db = self.db.clone();
        let owner = self.entity_incarnation.clone();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read().map_err(backend_err)?;
            engine::page(
                &read_reader(&txn)?,
                &owner,
                chunk,
                expected,
                cursor.as_ref(),
                limit,
            )
            .map_err(Into::into)
        })
        .await
        .map_err(join_err)?
    }
}
