use std::collections::BTreeMap;
use std::sync::Barrier;

use ferrumc_core::{DimensionId, EntityId, GameMode, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_world::{BlockStateId, Chunk, GenerationError, WorldSeed};
use redb::MultimapTableHandle;

use super::*;
use crate::{MutationActor, MutationLogCause, SchemaVersion};

type Rows = BTreeMap<Vec<u8>, Vec<u8>>;

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    meta: BTreeMap<String, u64>,
    tables: BTreeMap<String, Rows>,
    multimaps: Vec<String>,
}

fn key(scope: u32) -> GenerationKey {
    GenerationKey::new(WorldId::new(scope), DimensionId::new(0))
}

fn identity(seed: i64) -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(seed))
}

fn mutation() -> BlockMutationLogRecord {
    BlockMutationLogRecord::new(
        SchemaVersion::new(1),
        0,
        10,
        MutationActor::System,
        BlockPos::new(0, 64, 0),
        BlockStateId::new(1),
        BlockStateId::new(2),
        MutationLogCause::Test,
    )
}

fn snapshot_db(db: &Database) -> Snapshot {
    let txn = db.begin_read().unwrap();
    let meta = txn
        .open_table(META_TABLE)
        .unwrap()
        .iter()
        .unwrap()
        .map(|row| {
            let (key, value) = row.unwrap();
            (key.value().to_owned(), value.value())
        })
        .collect();
    let mut tables = BTreeMap::new();
    for handle in txn.list_tables().unwrap() {
        let name = handle.name();
        if name == META_TABLE.name() {
            continue;
        }
        let table = txn
            .open_table(TableDefinition::<&[u8], &[u8]>::new(name))
            .unwrap();
        let rows = table
            .iter()
            .unwrap()
            .map(|row| {
                let (key, value) = row.unwrap();
                (key.value().to_vec(), value.value().to_vec())
            })
            .collect();
        tables.insert(name.to_owned(), rows);
    }
    let multimaps = txn
        .list_multimap_tables()
        .unwrap()
        .map(|handle| handle.name().to_owned())
        .collect();
    Snapshot {
        meta,
        tables,
        multimaps,
    }
}

fn insert_raw(
    store: &RedbStore,
    table: TableDefinition<'_, &[u8], &[u8]>,
    key: &[u8],
    value: &[u8],
) {
    let txn = store.db.begin_write().unwrap();
    txn.open_table(table).unwrap().insert(key, value).unwrap();
    txn.commit().unwrap();
}

#[tokio::test]
async fn format_openers_refuse_opposite_layout_without_logical_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let path3 = dir.path().join("format3.redb");
    let before3 = {
        let store = RedbStore::open(&path3).unwrap();
        store
            .put(
                &PluginId::new("legacy"),
                StorageKey::new("data").unwrap(),
                vec![9],
            )
            .await
            .unwrap();
        snapshot_db(&store.db)
    };
    let error = RedbStore::open_with_generation_metadata(&path3).unwrap_err();
    assert_eq!(
        error,
        ServerError::from(StorageError::GenerationMigrationRequired)
    );
    assert_eq!(snapshot_db(&Database::open(&path3).unwrap()), before3);
    assert_eq!(before3.tables.len() + 1, 8);

    let path4 = dir.path().join("format4.redb");
    let before4 = {
        let store = RedbStore::open_with_generation_metadata(&path4).unwrap();
        store
            .ensure_generation_identity(key(1), identity(90))
            .await
            .unwrap();
        snapshot_db(&store.db)
    };
    assert_eq!(
        RedbStore::open(&path4).unwrap_err(),
        ServerError::from(StorageError::IncompatiblePreAlphaData)
    );
    assert_eq!(snapshot_db(&Database::open(&path4).unwrap()), before4);
    assert_eq!(before4.tables.len() + 1, 9);
    assert_eq!(before4.meta[META_FORMAT_KEY], 4);
}

#[test]
fn format4_catalog_cannot_silently_create_missing_or_accept_unknown_tables() {
    for category in ["missing-generation", "missing-chunk", "unknown", "multimap"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.redb");
        let before = {
            let store = RedbStore::open_with_generation_metadata(&path).unwrap();
            let txn = store.db.begin_write().unwrap();
            match category {
                "missing-generation" => {
                    txn.delete_table(GENERATION_TABLE).unwrap();
                }
                "missing-chunk" => {
                    txn.delete_table(CHUNK_TABLE).unwrap();
                }
                "unknown" => {
                    txn.open_table(TableDefinition::<&[u8], &[u8]>::new("unknown"))
                        .unwrap();
                }
                "multimap" => {
                    txn.open_multimap_table(redb::MultimapTableDefinition::<&[u8], &[u8]>::new(
                        "unknown-multimap",
                    ))
                    .unwrap();
                }
                _ => unreachable!(),
            }
            txn.commit().unwrap();
            snapshot_db(&store.db)
        };
        assert_eq!(
            RedbStore::open_with_generation_metadata(&path).unwrap_err(),
            ServerError::from(StorageError::IncompatiblePreAlphaData),
            "{category}"
        );
        assert_eq!(
            snapshot_db(&Database::open(&path).unwrap()),
            before,
            "{category}"
        );
    }
}

#[test]
fn format4_generation_table_type_mismatch_is_invalid_state_and_unmodified() {
    const WRONG: TableDefinition<'_, &str, u64> = TableDefinition::new("ferrumc:generation");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("type.redb");
    {
        let store = RedbStore::open_with_generation_metadata(&path).unwrap();
        let txn = store.db.begin_write().unwrap();
        txn.delete_table(GENERATION_TABLE).unwrap();
        txn.open_table(WRONG)
            .unwrap()
            .insert("unchanged", 55)
            .unwrap();
        txn.commit().unwrap();
    }
    assert_eq!(
        RedbStore::open_with_generation_metadata(&path).unwrap_err(),
        ServerError::from(StorageError::IncompatiblePreAlphaData)
    );
    let db = Database::open(&path).unwrap();
    let txn = db.begin_read().unwrap();
    assert_eq!(
        txn.open_table(WRONG)
            .unwrap()
            .get("unchanged")
            .unwrap()
            .unwrap()
            .value(),
        55
    );
    assert_eq!(
        txn.open_table(META_TABLE)
            .unwrap()
            .get(META_FORMAT_KEY)
            .unwrap()
            .unwrap()
            .value(),
        4
    );
    assert_eq!(txn.list_tables().unwrap().count(), 9);
}

#[tokio::test]
async fn all_malformed_or_unknown_identity_values_are_classified_and_never_replaced() {
    let valid = identity(0).encode();
    let mut cases: Vec<(Vec<u8>, GenerationError)> = Vec::new();
    for len in [0, 1, 26, 28, 128] {
        let mut value = valid.to_vec();
        value.resize(len, 0);
        cases.push((
            value,
            GenerationError::InvalidLength {
                actual: len,
                expected: valid.len(),
            },
        ));
    }
    let mut value = valid;
    value[0] = 0;
    cases.push((
        value.to_vec(),
        GenerationError::InvalidMagic {
            actual: [0, b'C', b'G', b'I'],
        },
    ));
    let mut value = valid;
    value[4..6].copy_from_slice(&2u16.to_be_bytes());
    cases.push((
        value.to_vec(),
        GenerationError::UnsupportedSchema { schema: 2 },
    ));
    let mut value = valid;
    value[10..12].copy_from_slice(&9u16.to_be_bytes());
    cases.push((
        value.to_vec(),
        GenerationError::UnsupportedMinecraftVersion {
            major: 1,
            minor: 21,
            patch: 9,
        },
    ));
    let mut value = valid;
    value[12..14].copy_from_slice(&2u16.to_be_bytes());
    cases.push((
        value.to_vec(),
        GenerationError::UnknownGenerator { kind: 2 },
    ));
    let mut value = valid;
    value[14..18].copy_from_slice(&2u32.to_be_bytes());
    cases.push((
        value.to_vec(),
        GenerationError::UnsupportedGeneratorRevision {
            kind: ferrumc_world::GeneratorKind::FerrumcFlat,
            revision: 2,
        },
    ));
    let mut value = valid;
    value[26] = 1;
    cases.push((
        value.to_vec(),
        GenerationError::UnsupportedDimension { dimension: 1 },
    ));
    for (value, source) in cases {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid-value.redb");
        let before = {
            let store = RedbStore::open_with_generation_metadata(&path).unwrap();
            insert_raw(&store, GENERATION_TABLE, &key(1).encode(), &value);
            let before = snapshot_db(&store.db);
            let expected = ServerError::from(StorageError::GenerationIdentityDecode {
                key: key(1),
                source,
            });
            assert_eq!(
                store.load_generation_identity(key(1)).await.unwrap_err(),
                expected
            );
            assert_eq!(
                store
                    .ensure_generation_identity(key(1), identity(0))
                    .await
                    .unwrap_err(),
                expected
            );
            // Missing another scope cannot silently create a baseline beside corruption.
            assert_eq!(
                store
                    .ensure_generation_identity(key(2), identity(0))
                    .await
                    .unwrap_err(),
                expected
            );
            assert_eq!(snapshot_db(&store.db), before);
            before
        };
        assert_eq!(
            RedbStore::open_with_generation_metadata(&path).unwrap_err(),
            ServerError::from(StorageError::GenerationIdentityDecode {
                key: key(1),
                source
            })
        );
        assert_eq!(snapshot_db(&Database::open(&path).unwrap()), before);
    }
}

#[tokio::test]
async fn malformed_generation_keys_are_classified_before_initialization_or_open() {
    for len in [0, 1, 7, 9, 128] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid-key.redb");
        let before = {
            let store = RedbStore::open_with_generation_metadata(&path).unwrap();
            insert_raw(
                &store,
                GENERATION_TABLE,
                &vec![0; len],
                &identity(0).encode(),
            );
            let before = snapshot_db(&store.db);
            assert_eq!(
                store
                    .ensure_generation_identity(key(1), identity(0))
                    .await
                    .unwrap_err(),
                ServerError::from(StorageError::MalformedGenerationKey { len })
            );
            assert_eq!(snapshot_db(&store.db), before);
            before
        };
        assert_eq!(
            RedbStore::open_with_generation_metadata(&path).unwrap_err(),
            ServerError::from(StorageError::MalformedGenerationKey { len })
        );
        assert_eq!(snapshot_db(&Database::open(&path).unwrap()), before);
    }
}

fn seed_world_state(store: &RedbStore, category: &str) {
    let pos = ChunkPos::new(0, 0);
    let chunk_key = ChunkKey::new(WorldId::new(99), DimensionId::new(99), pos);
    match category {
        "chunk" => insert_raw(
            store,
            CHUNK_TABLE,
            &codec::chunk_key_bytes(chunk_key),
            &codec::encode_chunk_record(&ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos)))
                .unwrap(),
        ),
        "empty-overlay" | "overlay" => {
            let mut chunk = Chunk::new(pos);
            if category == "overlay" {
                chunk
                    .set_block(BlockPos::new(0, 0, 0), BlockStateId::new(1))
                    .unwrap();
            }
            let overlay =
                ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(1), pos, &chunk, 0).unwrap();
            insert_raw(
                store,
                CHUNK_OVERLAY_TABLE,
                &codec::chunk_key_bytes(chunk_key),
                &codec::encode_chunk_overlay_record(&overlay).unwrap(),
            );
        }
        "entity" => insert_raw(
            store,
            ENTITY_TABLE,
            &codec::entity_key_bytes(EntityKey::new(
                chunk_key.world(),
                chunk_key.dimension(),
                EntityId::new(1),
            )),
            &codec::encode_entity_record(
                &EntityRecord::new(SchemaVersion::new(1), vec![]).unwrap(),
            ),
        ),
        "player" => insert_raw(
            store,
            PLAYER_TABLE,
            &codec::player_key_bytes(PlayerId::offline("existing")),
            &codec::encode_player_record(
                &PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![]).unwrap(),
            ),
        ),
        "journal" | "watermark" => {
            let txn = store.db.begin_write().unwrap();
            txn.open_table(META_TABLE)
                .unwrap()
                .insert(META_LAST_MUTATION_ID_KEY, 0)
                .unwrap();
            if category == "journal" {
                txn.open_table(MUTATION_LOG_TABLE)
                    .unwrap()
                    .insert(
                        0u64.to_be_bytes().as_slice(),
                        codec::encode_mutation_log_record(&mutation()).as_slice(),
                    )
                    .unwrap();
            }
            txn.commit().unwrap();
        }
        "empty-receipt" => {
            let batch = JournalBatchId::from_bytes([1; 16]);
            let receipt = JournalAppendReceipt::from_range(batch, None, 0);
            insert_raw(
                store,
                MUTATION_BATCH_TABLE,
                batch.as_bytes(),
                &encode_journal_receipt(receipt).unwrap(),
            );
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn every_world_table_and_journal_watermark_prevents_missing_scope_initialization() {
    for category in [
        "chunk",
        "empty-overlay",
        "overlay",
        "entity",
        "player",
        "journal",
        "empty-receipt",
        "watermark",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let store =
            RedbStore::open_with_generation_metadata(dir.path().join("freshness.redb")).unwrap();
        seed_world_state(&store, category);
        let before = snapshot_db(&store.db);
        assert_eq!(
            store
                .ensure_generation_identity(key(1), identity(0))
                .await
                .unwrap_err(),
            ServerError::from(StorageError::MissingGenerationIdentity { key: Some(key(1)) }),
            "{category}"
        );
        assert_eq!(snapshot_db(&store.db), before, "{category}");
        assert!(store
            .load_generation_identity(key(1))
            .await
            .unwrap()
            .is_none());
    }
}

#[tokio::test]
async fn bounded_generation_table_rejects_new_identity_but_compares_existing_at_cap() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("limit.redb");
    let before = {
        let store = RedbStore::open_with_generation_metadata(&path).unwrap();
        // Seed the capacity in one transaction; API insertion and boundary are
        // exercised separately without 4096 disk flushes in this unit test.
        let txn = store.db.begin_write().unwrap();
        {
            let mut table = txn.open_table(GENERATION_TABLE).unwrap();
            for scope in 0..u32::try_from(MAX_GENERATION_IDENTITIES).unwrap() {
                table
                    .insert(
                        key(scope).encode().as_slice(),
                        identity(i64::from(scope)).encode().as_slice(),
                    )
                    .unwrap();
            }
        }
        txn.commit().unwrap();
        let before = snapshot_db(&store.db);
        assert_eq!(
            store
                .ensure_generation_identity(key(u32::MAX), identity(0))
                .await
                .unwrap_err(),
            ServerError::from(StorageError::GenerationIdentityLimit {
                max: MAX_GENERATION_IDENTITIES
            })
        );
        assert_eq!(
            store
                .ensure_generation_identity(key(0), identity(0))
                .await
                .unwrap(),
            GenerationInitialization::AlreadyPresent
        );
        assert!(matches!(
            store.ensure_generation_identity(key(0), identity(1)).await,
            Err(ServerError::InvalidState(_))
        ));
        assert_eq!(snapshot_db(&store.db), before);
        before
    };
    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    assert_eq!(snapshot_db(&store.db), before);
    insert_raw(
        &store,
        GENERATION_TABLE,
        &key(u32::MAX).encode(),
        &identity(0).encode(),
    );
    let oversized = snapshot_db(&store.db);
    drop(store);
    assert_eq!(
        RedbStore::open_with_generation_metadata(&path).unwrap_err(),
        ServerError::from(StorageError::GenerationIdentityLimit {
            max: MAX_GENERATION_IDENTITIES
        })
    );
    assert_eq!(snapshot_db(&Database::open(&path).unwrap()), oversized);
}

#[tokio::test]
async fn insert_before_commit_abort_and_drop_preserve_every_logical_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cutpoint.redb");
    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    store
        .put(
            &PluginId::new("cutpoint"),
            StorageKey::new("untouched").unwrap(),
            vec![9],
        )
        .await
        .unwrap();
    let before = snapshot_db(&store.db);
    for abort in [true, false] {
        let txn = store.db.begin_write().unwrap();
        assert_eq!(
            ensure_generation_in_transaction(&txn, key(1), identity(i64::MIN)).unwrap(),
            GenerationInitialization::Initialized
        );
        // Read transactions cannot see the uncommitted identity.
        assert_eq!(store.load_generation_identity(key(1)).await.unwrap(), None);
        if abort {
            txn.abort().unwrap();
        } else {
            drop(txn);
        }
        assert_eq!(snapshot_db(&store.db), before);
    }
    drop(store);
    let reopened = RedbStore::open_with_generation_metadata(&path).unwrap();
    assert_eq!(snapshot_db(&reopened.db), before);
    assert_eq!(
        reopened
            .ensure_generation_identity(key(1), identity(i64::MIN))
            .await
            .unwrap(),
        GenerationInitialization::Initialized
    );
}

#[tokio::test]
async fn commit_then_lost_response_retry_is_idempotent_and_different_seed_still_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("response-cutpoint.redb");
    {
        let store = RedbStore::open_with_generation_metadata(&path).unwrap();
        let txn = store.db.begin_write().unwrap();
        assert_eq!(
            ensure_generation_in_transaction(&txn, key(1), identity(i64::MAX)).unwrap(),
            GenerationInitialization::Initialized
        );
        // Simulate an absent response after the same production helper commits.
        txn.commit().unwrap();
    }
    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    let before = snapshot_db(&store.db);
    assert_eq!(
        store
            .ensure_generation_identity(key(1), identity(i64::MAX))
            .await
            .unwrap(),
        GenerationInitialization::AlreadyPresent
    );
    assert_eq!(
        store
            .ensure_generation_identity(key(1), identity(0))
            .await
            .unwrap_err(),
        ServerError::from(StorageError::GenerationIdentityMismatch {
            key: key(1),
            stored: identity(i64::MAX),
            requested: identity(0)
        })
    );
    assert_eq!(snapshot_db(&store.db), before);
    assert_eq!(before.tables[GENERATION_TABLE.name()].len(), 1);
}

#[test]
fn in_flight_save_transaction_is_visible_to_waiting_initializer() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(
        RedbStore::open_with_generation_metadata(dir.path().join("save-race.redb")).unwrap(),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime
        .block_on(store.ensure_generation_identity(key(1), identity(0)))
        .unwrap();
    let save_txn = store.db.begin_write().unwrap();
    require_generation(&save_txn, GenerationMode::Metadata, Some(key(1))).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let initializer = {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            barrier.wait();
            runtime.block_on(store.ensure_generation_identity(key(2), identity(0)))
        })
    };
    barrier.wait();
    let pos = ChunkPos::new(0, 0);
    let chunk_key = ChunkKey::new(key(1).world(), key(1).dimension(), pos);
    save_txn
        .open_table(CHUNK_TABLE)
        .unwrap()
        .insert(
            codec::chunk_key_bytes(chunk_key).as_slice(),
            codec::encode_chunk_record(&ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos)))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
    save_txn.commit().unwrap();
    assert!(matches!(
        initializer.join().unwrap(),
        Err(ServerError::InvalidState(_))
    ));
    assert!(runtime
        .block_on(store.load_generation_identity(key(2)))
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn rejected_save_batches_and_comparisons_preserve_complete_logical_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let store = RedbStore::open_with_generation_metadata(dir.path().join("snapshot.redb")).unwrap();
    store
        .ensure_generation_identity(key(1), identity(0))
        .await
        .unwrap();
    let pos = ChunkPos::new(0, 0);
    let bound = ChunkKey::new(key(1).world(), key(1).dimension(), pos);
    let unbound = ChunkKey::new(key(2).world(), key(2).dimension(), pos);
    let record = ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos));
    let overlay =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(1), pos, record.chunk(), 0).unwrap();
    let bound_entity = EntityKey::new(bound.world(), bound.dimension(), EntityId::new(1));
    let unbound_entity = EntityKey::new(unbound.world(), unbound.dimension(), EntityId::new(1));
    let entity = EntityRecord::new(SchemaVersion::new(1), vec![3]).unwrap();
    store.save_chunk(bound, record.clone()).await.unwrap();
    store
        .save_chunk_overlays(vec![(bound, overlay.clone())])
        .await
        .unwrap();
    store
        .save_entity(bound_entity, entity.clone())
        .await
        .unwrap();
    store
        .save_player(
            PlayerId::offline("snapshot"),
            PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![2]).unwrap(),
        )
        .await
        .unwrap();
    store
        .put(
            &PluginId::new("snapshot"),
            StorageKey::new("key").unwrap(),
            vec![9],
        )
        .await
        .unwrap();
    store
        .append_block_mutation_batch(JournalBatchId::from_bytes([5; 16]), vec![mutation()])
        .await
        .unwrap();
    let before = snapshot_db(&store.db);
    for result in [
        store
            .save_chunks(vec![(bound, record.clone()), (unbound, record.clone())])
            .await,
        store
            .save_chunk_overlays(vec![(bound, overlay.clone()), (unbound, overlay)])
            .await,
        store
            .save_entities(vec![
                (bound_entity, entity.clone()),
                (unbound_entity, entity.clone()),
            ])
            .await,
        store.save_entity(unbound_entity, entity).await,
        store.save_chunk(unbound, record).await,
    ] {
        assert!(matches!(result, Err(ServerError::InvalidState(_))));
        assert_eq!(snapshot_db(&store.db), before);
    }
    assert!(store
        .ensure_generation_identity(key(1), identity(1))
        .await
        .is_err());
    assert!(store
        .ensure_generation_identity(key(2), identity(0))
        .await
        .is_err());
    assert_eq!(snapshot_db(&store.db), before);
    assert_eq!(
        store
            .ensure_generation_identity(key(1), identity(0))
            .await
            .unwrap(),
        GenerationInitialization::AlreadyPresent
    );
}
