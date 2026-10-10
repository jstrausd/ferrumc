use super::*;

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Barrier;

use ferrumc_core::{DimensionId, GameMode, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_world::{BlockStateId, Chunk};
use redb::{TableDefinition, TableHandle};

use crate::codec;
use crate::redb_store::{encode_journal_receipt, META_LAST_MUTATION_ID_KEY};
use crate::{
    BlockMutationLogRecord, ChunkKey, ChunkOverlayRecord, ChunkRecord, EntityKey, EntityRecord,
    JournalAppendReceipt, MutationActor, MutationLogCause, PlayerRecord, SchemaVersion,
    MAX_ENTITY_DATA_LEN, MAX_PLAYER_DATA_LEN, MAX_PLUGIN_KEY_LEN, MAX_PLUGIN_VALUE_LEN,
    MAX_SAVE_BATCH,
};

type Snapshot = BTreeMap<String, Vec<(Vec<u8>, Vec<u8>)>>;

fn scope(id: u32) -> GenerationKey {
    GenerationKey::new(WorldId::new(id), DimensionId::new(0))
}

fn declaration(scopes: &[u32]) -> LegacyGenerationDeclaration {
    LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero(scopes.iter().copied().map(scope))
        .unwrap()
}

fn fresh(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("world.redb");
    drop(RedbStore::open(&path).unwrap());
    path
}

fn prepare(path: &Path, scopes: &[u32]) -> PreparedGenerationMigration {
    let GenerationMigrationPreparation::Required(prepared) =
        RedbStore::prepare_generation_migration(path, declaration(scopes)).unwrap()
    else {
        panic!("expected legacy source");
    };
    prepared
}

fn raw_edit(path: &Path, edit: impl FnOnce(&redb::WriteTransaction)) {
    let db = Database::open(path).unwrap();
    let txn = db.begin_write().unwrap();
    edit(&txn);
    txn.commit().unwrap();
}

fn snapshot(path: &Path) -> Snapshot {
    let db = Database::open(path).unwrap();
    let txn = db.begin_read().unwrap();
    let mut snapshot = Snapshot::new();
    for handle in txn.list_tables().unwrap() {
        let name = handle.name();
        let mut rows = Vec::new();
        if name == META_TABLE.name() {
            for row in txn.open_table(META_TABLE).unwrap().iter().unwrap() {
                let (key, value) = row.unwrap();
                rows.push((
                    key.value().as_bytes().to_vec(),
                    value.value().to_be_bytes().to_vec(),
                ));
            }
        } else if let Ok(table) = txn.open_table(TableDefinition::<&[u8], &[u8]>::new(name)) {
            for row in table.iter().unwrap() {
                let (key, value) = row.unwrap();
                rows.push((key.value().to_vec(), value.value().to_vec()));
            }
        } else {
            // Catalog/type-specific tests preserve a stable shape marker; no
            // incompatible value is interpreted by a guessed byte grammar.
            rows.push((b"incompatible table type".to_vec(), Vec::new()));
        }
        snapshot.insert(name.to_owned(), rows);
    }
    snapshot
}

fn mutation(id: u64) -> BlockMutationLogRecord {
    BlockMutationLogRecord::new(
        SchemaVersion::new(0x3456),
        id,
        17 + id,
        MutationActor::System,
        BlockPos::new(32, 64, -48),
        BlockStateId::AIR,
        BlockStateId::new(1),
        MutationLogCause::Test,
    )
}

fn journal_rows(txn: &redb::WriteTransaction, count: u64) {
    {
        let mut table = txn.open_table(MUTATION_LOG_TABLE).unwrap();
        for id in 0..count {
            table
                .insert(
                    id.to_be_bytes().as_slice(),
                    codec::encode_mutation_log_record(&mutation(id)).as_slice(),
                )
                .unwrap();
        }
    }
    if count > 0 {
        txn.open_table(META_TABLE)
            .unwrap()
            .insert(META_LAST_MUTATION_ID_KEY, count - 1)
            .unwrap();
    }
}

fn receipt(txn: &redb::WriteTransaction, token: u8, first: u64, count: usize) {
    let batch = JournalBatchId::from_bytes([token; 16]);
    let range = (count > 0).then(|| (first, first + count as u64 - 1));
    let bytes =
        encode_journal_receipt(JournalAppendReceipt::from_range(batch, range, count)).unwrap();
    txn.open_table(MUTATION_BATCH_TABLE)
        .unwrap()
        .insert(batch.as_bytes().as_slice(), bytes.as_slice())
        .unwrap();
}

fn all_categories(path: &Path) {
    raw_edit(path, |txn| {
        let pos = ChunkPos::new(2, -3);
        let key = ChunkKey::new(WorldId::new(1), DimensionId::new(0), pos);
        let record = ChunkRecord::new(SchemaVersion::new(0x1234_5678), Chunk::new(pos));
        let canonical = codec::encode_chunk_record(&record).unwrap();
        // A valid dense all-air section decodes to the canonical empty palette.
        // Reencoding would lose this representation, so exact bytes catch any
        // tempting decode/reencode migration or backup implementation.
        let mut noncanonical = canonical[..21].to_vec();
        noncanonical.push(1);
        noncanonical.extend_from_slice(&vec![0; 4096 * 4]);
        noncanonical.extend_from_slice(&canonical[22..]);
        assert_ne!(
            noncanonical,
            codec::encode_chunk_record(&codec::decode_chunk_record(&noncanonical).unwrap())
                .unwrap()
        );
        txn.open_table(CHUNK_TABLE)
            .unwrap()
            .insert(
                codec::chunk_key_bytes(key).as_slice(),
                noncanonical.as_slice(),
            )
            .unwrap();
        let overlay =
            ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(17), pos, record.chunk(), 999)
                .unwrap();
        txn.open_table(CHUNK_OVERLAY_TABLE)
            .unwrap()
            .insert(
                codec::chunk_key_bytes(key).as_slice(),
                codec::encode_chunk_overlay_record(&overlay)
                    .unwrap()
                    .as_slice(),
            )
            .unwrap();
        let entity_key = EntityKey::new(
            WorldId::new(1),
            DimensionId::new(0),
            ferrumc_core::EntityId::new(-1),
        );
        let entity = EntityRecord::new(SchemaVersion::new(u32::MAX), vec![0, 255, 17]).unwrap();
        txn.open_table(ENTITY_TABLE)
            .unwrap()
            .insert(
                codec::entity_key_bytes(entity_key).as_slice(),
                codec::encode_entity_record(&entity).as_slice(),
            )
            .unwrap();
        let player =
            PlayerRecord::new(SchemaVersion::new(0), GameMode::Survival, vec![7, 4, 1]).unwrap();
        txn.open_table(PLAYER_TABLE)
            .unwrap()
            .insert(
                [9; 16].as_slice(),
                codec::encode_player_record(&player).as_slice(),
            )
            .unwrap();
        txn.open_table(PLUGIN_TABLE)
            .unwrap()
            .insert([0, 0, 0, 1, b'a', b'k'].as_slice(), [255, 0, 7].as_slice())
            .unwrap();
        journal_rows(txn, 3);
        receipt(txn, 3, 0, 2);
        receipt(txn, 2, 2, 1);
        receipt(txn, 1, 0, 0);
    });
}

fn refuse_unchanged(path: &Path, scopes: &[u32], check: impl FnOnce(&GenerationMigrationError)) {
    let before = snapshot(path);
    let error = RedbStore::prepare_generation_migration(path, declaration(scopes)).unwrap_err();
    check(&error);
    assert_eq!(snapshot(path), before);
}

#[test]
fn declaration_constructor_stops_an_unbounded_iterator_at_the_cap() {
    let mut calls = 0;
    let input = std::iter::from_fn(|| {
        let id = calls;
        calls += 1;
        Some(GenerationKey::new(
            ferrumc_core::WorldId::new(id),
            ferrumc_core::DimensionId::new(0),
        ))
    });
    assert!(matches!(
        LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero(input),
        Err(GenerationMigrationError::TooManyScopes)
    ));
    assert_eq!(calls, MAX_GENERATION_IDENTITIES as u32 + 1);
}

#[test]
fn every_raw_category_backup_and_journal_order_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    all_categories(&path);
    let before = snapshot(&path);
    let prepared = prepare(&path, &[1]);
    assert_eq!(
        prepared.report().records(GenerationMigrationTable::Chunks),
        1
    );
    assert_eq!(
        prepared
            .report()
            .records(GenerationMigrationTable::ChunkOverlays),
        1
    );
    assert_eq!(
        prepared
            .report()
            .records(GenerationMigrationTable::JournalRecords),
        3
    );
    assert_eq!(
        prepared
            .report()
            .records(GenerationMigrationTable::JournalReceipts),
        3
    );
    let expected_hash = *prepared.report().snapshot_sha256();
    let backup_path = dir.path().join("backup.redb");
    let backed = prepared.create_backup(&backup_path).unwrap();
    assert_eq!(backed.backup().snapshot_sha256(), &expected_hash);
    let outcome = backed.migrate().unwrap();
    assert_eq!(snapshot(&backup_path), before);
    let mut after = snapshot(&path);
    let descriptors = after.remove(GENERATION_TABLE.name()).unwrap();
    assert_eq!(
        descriptors,
        vec![(
            scope(1).encode().to_vec(),
            declaration(&[1]).identity().encode().to_vec()
        )]
    );
    let meta = after.get_mut(META_TABLE.name()).unwrap();
    let marker = meta
        .iter_mut()
        .find(|(key, _)| key == META_FORMAT_KEY.as_bytes())
        .unwrap();
    assert_eq!(marker.1, 4_u64.to_be_bytes());
    marker.1 = 3_u64.to_be_bytes().to_vec();
    assert_eq!(after, before);
    assert_eq!(outcome.report().snapshot_sha256(), &expected_hash);
    assert!(RedbStore::open(&path).is_err());
    assert!(matches!(
        RedbStore::prepare_generation_migration(&path, declaration(&[1])).unwrap(),
        GenerationMigrationPreparation::AlreadyCompatible(_)
    ));
}

#[test]
fn source_remains_exclusive_through_report_backup_and_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let before = snapshot(&path);
    let prepared = prepare(&path, &[1]);
    let barrier = Arc::new(Barrier::new(2));
    let gate = Arc::clone(&barrier);
    let other_path = path.clone();
    let other = std::thread::spawn(move || {
        gate.wait();
        RedbStore::open(other_path)
    });
    barrier.wait();
    assert!(other.join().unwrap().is_err());
    assert!(RedbStore::open_with_generation_metadata(&path).is_err());
    let backup_path = dir.path().join("backup.redb");
    let backed = prepared.create_backup(&backup_path).unwrap();
    assert!(RedbStore::open(&path).is_err());
    assert!(RedbStore::open(&backup_path).is_err());
    drop(backed);
    assert_eq!(snapshot(&path), before);
    assert_eq!(snapshot(&backup_path), before);
}

#[test]
fn scoped_categories_require_complete_declaration() {
    for category in [CHUNK_TABLE, CHUNK_OVERLAY_TABLE, ENTITY_TABLE] {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        all_categories(&path);
        raw_edit(&path, |txn| {
            for other in [
                CHUNK_TABLE,
                CHUNK_OVERLAY_TABLE,
                ENTITY_TABLE,
                PLAYER_TABLE,
                PLUGIN_TABLE,
                MUTATION_LOG_TABLE,
                MUTATION_BATCH_TABLE,
            ] {
                if other.name() != category.name() {
                    txn.open_table(other).unwrap().retain(|_, _| false).unwrap();
                }
            }
            txn.open_table(META_TABLE)
                .unwrap()
                .remove(META_LAST_MUTATION_ID_KEY)
                .unwrap();
        });
        refuse_unchanged(&path, &[2], |error| {
            assert!(matches!(error,
            GenerationMigrationError::UndeclaredScope { key } if *key == scope(1)));
        });
    }
}

#[test]
fn each_unscoped_category_is_conservatively_ambiguous() {
    for kind in [
        GenerationMigrationTable::Players,
        GenerationMigrationTable::JournalRecords,
        GenerationMigrationTable::JournalReceipts,
        GenerationMigrationTable::Metadata,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        raw_edit(&path, |txn| match kind {
            GenerationMigrationTable::Players => {
                let value = codec::encode_player_record(
                    &PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![]).unwrap(),
                );
                txn.open_table(PLAYER_TABLE)
                    .unwrap()
                    .insert([0; 16].as_slice(), value.as_slice())
                    .unwrap();
            }
            GenerationMigrationTable::JournalRecords => journal_rows(txn, 1),
            GenerationMigrationTable::JournalReceipts => receipt(txn, 1, 0, 0),
            _ => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert(META_LAST_MUTATION_ID_KEY, 0)
                    .unwrap();
            }
        });
        refuse_unchanged(&path, &[1, 2], |error| {
            assert!(matches!(
                error,
                GenerationMigrationError::AmbiguousUnscopedState
            ));
        });
    }
}

#[test]
fn multiple_scoped_worlds_and_empty_declared_scopes_can_migrate() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    raw_edit(&path, |txn| {
        for id in [1, 2] {
            let key = EntityKey::new(
                WorldId::new(id),
                DimensionId::new(0),
                ferrumc_core::EntityId::new(5),
            );
            let value = codec::encode_entity_record(
                &EntityRecord::new(SchemaVersion::new(1), vec![]).unwrap(),
            );
            txn.open_table(ENTITY_TABLE)
                .unwrap()
                .insert(codec::entity_key_bytes(key).as_slice(), value.as_slice())
                .unwrap();
        }
    });
    prepare(&path, &[1, 2, 3])
        .create_backup(dir.path().join("backup.redb"))
        .unwrap()
        .migrate()
        .unwrap();
    assert!(matches!(
        RedbStore::prepare_generation_migration(&path, declaration(&[1, 2, 3])).unwrap(),
        GenerationMigrationPreparation::AlreadyCompatible(_)
    ));
    refuse_unchanged(&path, &[1, 2], |error| {
        assert!(matches!(
            error,
            GenerationMigrationError::UndeclaredScope { .. }
                | GenerationMigrationError::DescriptorSetMismatch
        ));
    });
    refuse_unchanged(&path, &[1, 2, 3, 4], |error| {
        assert!(matches!(
            error,
            GenerationMigrationError::DescriptorSetMismatch
        ));
    });
}

#[test]
fn catalog_format_metadata_and_table_shape_refusals() {
    for case in 0..7 {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        raw_edit(&path, |txn| match case {
            0 => {
                txn.delete_table(CHUNK_TABLE).unwrap();
            }
            1 => {
                txn.open_table(TableDefinition::<&[u8], &[u8]>::new("unknown"))
                    .unwrap();
            }
            2 => {
                txn.open_multimap_table(redb::MultimapTableDefinition::<u64, u64>::new("multimap"))
                    .unwrap();
            }
            3 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .remove(META_FORMAT_KEY)
                    .unwrap();
            }
            4 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert(META_FORMAT_KEY, 2)
                    .unwrap();
            }
            5 => {
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert("unrecognized", 1)
                    .unwrap();
            }
            _ => {
                txn.delete_table(ENTITY_TABLE).unwrap();
                txn.open_table(TableDefinition::<u64, &[u8]>::new("ferrumc:entity"))
                    .unwrap()
                    .insert(1, [0; 4].as_slice())
                    .unwrap();
            }
        });
        refuse_unchanged(&path, &[1], |_| {});
    }
}

#[test]
fn malformed_fixed_width_keys_are_refused_before_record_decode() {
    for (table, expected) in [
        (CHUNK_TABLE, 16),
        (CHUNK_OVERLAY_TABLE, 16),
        (ENTITY_TABLE, 12),
        (PLAYER_TABLE, 16),
        (MUTATION_LOG_TABLE, 8),
        (MUTATION_BATCH_TABLE, 16),
    ] {
        for len in [0, expected - 1, expected + 1] {
            let dir = tempfile::tempdir().unwrap();
            let path = fresh(&dir);
            raw_edit(&path, |txn| {
                txn.open_table(table)
                    .unwrap()
                    .insert(vec![0; len].as_slice(), [0].as_slice())
                    .unwrap();
            });
            refuse_unchanged(&path, &[1], |error| {
                assert!(matches!(
                    error,
                    GenerationMigrationError::MalformedKey { .. }
                        | GenerationMigrationError::Record { .. }
                ));
            });
        }
    }
}

#[test]
fn malformed_bounded_record_payloads_and_position_mismatch_are_refused() {
    for case in 0..9 {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        raw_edit(&path, |txn| {
            let pos = ChunkPos::new(2, -3);
            let chunk_key =
                codec::chunk_key_bytes(ChunkKey::new(WorldId::new(1), DimensionId::new(0), pos));
            let (table, key, value) = match case {
                0 => (CHUNK_TABLE, chunk_key.to_vec(), vec![]),
                1 => {
                    let value = codec::encode_chunk_record(&ChunkRecord::new(
                        SchemaVersion::new(1),
                        Chunk::new(ChunkPos::new(3, -3)),
                    ))
                    .unwrap();
                    (CHUNK_TABLE, chunk_key.to_vec(), value)
                }
                2 => (CHUNK_OVERLAY_TABLE, chunk_key.to_vec(), vec![0; 19]),
                3 => {
                    let chunk = Chunk::new(ChunkPos::new(3, -3));
                    let overlay = ChunkOverlayRecord::try_from_chunk(
                        SchemaVersion::new(1),
                        chunk.pos(),
                        &chunk,
                        0,
                    )
                    .unwrap();
                    (
                        CHUNK_OVERLAY_TABLE,
                        chunk_key.to_vec(),
                        codec::encode_chunk_overlay_record(&overlay).unwrap(),
                    )
                }
                4 => (ENTITY_TABLE, chunk_key[..12].to_vec(), vec![0; 3]),
                5 => (
                    ENTITY_TABLE,
                    chunk_key[..12].to_vec(),
                    vec![0; 4 + MAX_ENTITY_DATA_LEN + 1],
                ),
                6 => (PLAYER_TABLE, vec![0; 16], vec![0; 4]),
                7 => (
                    PLAYER_TABLE,
                    vec![0; 16],
                    vec![0; 5 + MAX_PLAYER_DATA_LEN + 1],
                ),
                _ => (PLAYER_TABLE, vec![0; 16], vec![0, 0, 0, 0, 4]),
            };
            txn.open_table(table)
                .unwrap()
                .insert(key.as_slice(), value.as_slice())
                .unwrap();
        });
        refuse_unchanged(&path, &[1], |error| {
            assert!(matches!(error, GenerationMigrationError::Record { .. }));
        });
    }
}

#[test]
fn malformed_journal_sequence_record_and_watermark_refusals() {
    for case in 0..8 {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        raw_edit(&path, |txn| {
            journal_rows(txn, 2);
            match case {
                0 => {
                    txn.open_table(MUTATION_LOG_TABLE)
                        .unwrap()
                        .remove(0_u64.to_be_bytes().as_slice())
                        .unwrap();
                }
                1 => {
                    txn.open_table(MUTATION_LOG_TABLE)
                        .unwrap()
                        .insert(
                            1_u64.to_be_bytes().as_slice(),
                            codec::encode_mutation_log_record(&mutation(0)).as_slice(),
                        )
                        .unwrap();
                }
                2 => {
                    txn.open_table(META_TABLE)
                        .unwrap()
                        .remove(META_LAST_MUTATION_ID_KEY)
                        .unwrap();
                }
                3 => {
                    txn.open_table(META_TABLE)
                        .unwrap()
                        .insert(META_LAST_MUTATION_ID_KEY, 0)
                        .unwrap();
                }
                4 => {
                    txn.open_table(MUTATION_LOG_TABLE)
                        .unwrap()
                        .retain(|_, _| false)
                        .unwrap();
                }
                5 => {
                    txn.open_table(MUTATION_LOG_TABLE)
                        .unwrap()
                        .insert(1_u64.to_be_bytes().as_slice(), [0].as_slice())
                        .unwrap();
                }
                6 => {
                    let mut value = codec::encode_mutation_log_record(&mutation(1));
                    value[20] = 2; // unsupported actor tag
                    txn.open_table(MUTATION_LOG_TABLE)
                        .unwrap()
                        .insert(1_u64.to_be_bytes().as_slice(), value.as_slice())
                        .unwrap();
                }
                _ => {
                    let mut value = codec::encode_mutation_log_record(&mutation(1));
                    value.push(0);
                    txn.open_table(MUTATION_LOG_TABLE)
                        .unwrap()
                        .insert(1_u64.to_be_bytes().as_slice(), value.as_slice())
                        .unwrap();
                }
            }
        });
        refuse_unchanged(&path, &[1], |error| {
            assert!(matches!(
                error,
                GenerationMigrationError::JournalSequence { .. }
                    | GenerationMigrationError::JournalRecordId { .. }
                    | GenerationMigrationError::JournalWatermark { .. }
                    | GenerationMigrationError::Record { .. }
            ));
        });
    }
}

#[test]
fn receipt_malformed_missing_range_overflow_and_overlaps_are_refused() {
    for case in 0..8 {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        raw_edit(&path, |txn| {
            journal_rows(txn, 3);
            match case {
                0 => {
                    txn.open_table(MUTATION_BATCH_TABLE)
                        .unwrap()
                        .insert([1; 16].as_slice(), [0; 15].as_slice())
                        .unwrap();
                }
                1 => {
                    txn.open_table(MUTATION_BATCH_TABLE)
                        .unwrap()
                        .insert([1; 16].as_slice(), [0; 17].as_slice())
                        .unwrap();
                }
                2..=4 => {
                    let (first, count): (u64, u64) = match case {
                        2 => (1, 0),
                        3 => (0, MAX_SAVE_BATCH as u64 + 1),
                        _ => (u64::MAX, 2),
                    };
                    let mut bytes = first.to_be_bytes().to_vec();
                    bytes.extend_from_slice(&count.to_be_bytes());
                    txn.open_table(MUTATION_BATCH_TABLE)
                        .unwrap()
                        .insert([1; 16].as_slice(), bytes.as_slice())
                        .unwrap();
                }
                5 => receipt(txn, 1, 2, 2),
                6 => {
                    receipt(txn, 3, 0, 2);
                    receipt(txn, 1, 0, 1);
                }
                _ => {
                    receipt(txn, 3, 0, 3);
                    receipt(txn, 1, 1, 1);
                }
            }
        });
        refuse_unchanged(&path, &[1], |error| {
            assert!(matches!(
                error,
                GenerationMigrationError::Record { .. }
                    | GenerationMigrationError::OverlappingReceipts { .. }
            ));
        });
        assert!(!fs::read_dir(dir.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".ferrumc-migration-receipts-")));
    }
}

#[test]
fn plugin_byte_boundaries_and_malformed_keys() {
    for case in 0..7 {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        let (key, value) = match case {
            0 => (vec![0; 3], vec![]),
            1 => (vec![0, 0, 0, 2, b'a'], vec![]),
            2 => (vec![0, 0, 0, 1, 255], vec![]),
            3 => (vec![0, 0, 0, 1, b'a', 255], vec![]),
            4 => {
                let mut key = 1_u32.to_be_bytes().to_vec();
                key.push(b'a');
                key.extend_from_slice(&vec![b'k'; MAX_PLUGIN_KEY_LEN + 1]);
                (key, vec![])
            }
            5 => (vec![0, 0, 0, 1, b'a'], vec![0; MAX_PLUGIN_VALUE_LEN + 1]),
            _ => {
                let mut key = (MAX_MIGRATION_PLUGIN_ID_LEN as u32 + 1)
                    .to_be_bytes()
                    .to_vec();
                key.extend_from_slice(&vec![b'a'; MAX_MIGRATION_PLUGIN_ID_LEN + 1]);
                (key, vec![])
            }
        };
        raw_edit(&path, |txn| {
            txn.open_table(PLUGIN_TABLE)
                .unwrap()
                .insert(key.as_slice(), value.as_slice())
                .unwrap();
        });
        refuse_unchanged(&path, &[1], |error| {
            assert!(matches!(
                error,
                GenerationMigrationError::InvalidPluginKey
                    | GenerationMigrationError::Record { .. }
            ));
        });
    }
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    raw_edit(&path, |txn| {
        let mut key = (MAX_MIGRATION_PLUGIN_ID_LEN as u32).to_be_bytes().to_vec();
        key.extend_from_slice("ä".repeat(MAX_MIGRATION_PLUGIN_ID_LEN / 2).as_bytes());
        key.extend_from_slice("ä".repeat(MAX_PLUGIN_KEY_LEN / 2).as_bytes());
        txn.open_table(PLUGIN_TABLE)
            .unwrap()
            .insert(key.as_slice(), vec![0; MAX_PLUGIN_VALUE_LEN].as_slice())
            .unwrap();
    });
    let prepared = prepare(&path, &[1, 2]);
    assert_eq!(
        prepared
            .report()
            .records(GenerationMigrationTable::PluginValues),
        1
    );
    drop(prepared);
}

#[test]
fn malformed_or_incomplete_format_four_never_becomes_retry_success() {
    for case in 0..6 {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        raw_edit(&path, |txn| {
            txn.open_table(META_TABLE)
                .unwrap()
                .insert(META_FORMAT_KEY, 4)
                .unwrap();
            if case == 0 {
                return;
            }
            let mut table = txn.open_table(GENERATION_TABLE).unwrap();
            match case {
                1 => {}
                2 => {
                    table
                        .insert(
                            [0; 7].as_slice(),
                            declaration(&[1]).identity().encode().as_slice(),
                        )
                        .unwrap();
                }
                3 => {
                    table
                        .insert(scope(1).encode().as_slice(), [0; 26].as_slice())
                        .unwrap();
                }
                4 => {
                    table
                        .insert(
                            scope(1).encode().as_slice(),
                            GenerationIdentity::flat_overworld(WorldSeed::new(17))
                                .encode()
                                .as_slice(),
                        )
                        .unwrap();
                }
                _ => {
                    let mut value = declaration(&[1]).identity().encode();
                    value[17] = 2;
                    table
                        .insert(scope(1).encode().as_slice(), value.as_slice())
                        .unwrap();
                }
            }
        });
        refuse_unchanged(&path, &[1], |_| {});
    }
}

#[test]
fn exact_4096_descriptor_boundary_can_commit_and_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let scopes: Vec<u32> = (0..MAX_GENERATION_IDENTITIES as u32).collect();
    let prepared = prepare(&path, &scopes);
    prepared
        .create_backup(dir.path().join("backup.redb"))
        .unwrap()
        .migrate()
        .unwrap();
    let GenerationMigrationPreparation::AlreadyCompatible(report) =
        RedbStore::prepare_generation_migration(&path, declaration(&scopes)).unwrap()
    else {
        panic!("complete format 4 must retry");
    };
    assert_eq!(
        report.records(GenerationMigrationTable::GenerationIdentities),
        MAX_GENERATION_IDENTITIES as u64
    );
}

#[test]
fn backup_existing_path_hash_and_snapshot_conflicts_are_safe() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let before = snapshot(&path);
    let backup_path = dir.path().join("backup.redb");
    fs::write(&backup_path, b"unrelated operator file").unwrap();
    assert!(matches!(
        prepare(&path, &[1]).create_backup(&backup_path),
        Err(GenerationMigrationError::BackupExists(_))
    ));
    assert_eq!(fs::read(&backup_path).unwrap(), b"unrelated operator file");
    assert_eq!(snapshot(&path), before);
    fs::remove_file(&backup_path).unwrap();
    let backed = prepare(&path, &[1]).create_backup(&backup_path).unwrap();
    let hash = *backed.backup().file_sha256();
    drop(backed);
    assert!(matches!(
        prepare(&path, &[1]).verify_existing_backup(&backup_path, [0; 32]),
        Err(GenerationMigrationError::BackupMismatch)
    ));
    let backed = prepare(&path, &[1])
        .verify_existing_backup(&backup_path, hash)
        .unwrap();
    drop(backed);
    assert_eq!(snapshot(&path), before);
    raw_edit(&backup_path, |txn| {
        txn.open_table(PLUGIN_TABLE)
            .unwrap()
            .insert([0; 4].as_slice(), [1].as_slice())
            .unwrap();
    });
    let file = LockedFile::open(&backup_path, false).unwrap();
    let changed_hash = file.file_hash().unwrap().1;
    drop(file);
    assert!(matches!(
        prepare(&path, &[1]).verify_existing_backup(&backup_path, changed_hash),
        Err(GenerationMigrationError::BackupMismatch)
    ));
    assert_eq!(snapshot(&path), before);
}

#[test]
fn tampered_backup_file_prevents_source_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let before = snapshot(&path);
    let backup_path = dir.path().join("backup.redb");
    let backed = prepare(&path, &[1]).create_backup(&backup_path).unwrap();
    // A raw writer intentionally bypasses advisory redb locking. The final
    // backup hash catches this independent of the standard writer lock.
    OpenOptions::new()
        .append(true)
        .open(&backup_path)
        .unwrap()
        .write_all(&[17])
        .unwrap();
    assert!(matches!(
        backed.migrate(),
        Err(GenerationMigrationError::BackupMismatch)
    ));
    assert_eq!(snapshot(&path), before);
}

#[test]
fn source_fingerprint_change_is_refused_in_actual_writer_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let before = snapshot(&path);
    let backed = prepare(&path, &[1])
        .create_backup(dir.path().join("backup.redb"))
        .unwrap();
    let txn = backed.prepared.db.begin_write().unwrap();
    txn.open_table(PLUGIN_TABLE)
        .unwrap()
        .insert([0; 4].as_slice(), [1].as_slice())
        .unwrap();
    assert!(matches!(
        stage_migration(&txn, &backed.prepared.report),
        Err(GenerationMigrationError::SourceChanged)
    ));
    txn.abort().unwrap();
    drop(backed);
    assert_eq!(snapshot(&path), before);
}

#[test]
fn each_transaction_cutpoint_rolls_back_complete_schema_and_records() {
    for cutpoint in [
        MigrationStage::TableCreated,
        MigrationStage::DescriptorInserted(0),
        MigrationStage::DescriptorInserted(1),
        MigrationStage::MarkerWritten,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        let before = snapshot(&path);
        let backed = prepare(&path, &[1, 2])
            .create_backup(dir.path().join("backup.redb"))
            .unwrap();
        let txn = backed.prepared.db.begin_write().unwrap();
        let result = stage_migration_with_observer(&txn, &backed.prepared.report, |stage| {
            if stage == cutpoint {
                Err(GenerationMigrationError::CounterOverflow)
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        drop(txn);
        drop(backed);
        assert_eq!(snapshot(&path), before);
        drop(RedbStore::open(&path).unwrap());
    }
}

#[test]
fn committed_but_lost_answer_retry_has_no_duplicate_identity_or_journal() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    all_categories(&path);
    let backed = prepare(&path, &[1])
        .create_backup(dir.path().join("backup.redb"))
        .unwrap();
    let txn = backed.prepared.db.begin_write().unwrap();
    stage_migration(&txn, &backed.prepared.report).unwrap();
    txn.commit().unwrap();
    // Simulate a lost API response after the same production stage/commit.
    drop(backed);
    let before_retry = snapshot(&path);
    assert!(matches!(
        RedbStore::prepare_generation_migration(&path, declaration(&[1])).unwrap(),
        GenerationMigrationPreparation::AlreadyCompatible(_)
    ));
    assert_eq!(snapshot(&path), before_retry);
    assert_eq!(before_retry.get(GENERATION_TABLE.name()).unwrap().len(), 1);
    assert_eq!(
        before_retry.get(MUTATION_LOG_TABLE.name()).unwrap().len(),
        3
    );
}

#[test]
fn missing_empty_and_invalid_sources_are_never_created_or_normalized() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.redb");
    assert!(RedbStore::prepare_generation_migration(&missing, declaration(&[1])).is_err());
    assert!(!missing.exists());
    for bytes in [vec![], vec![0; 9], b"unrelated file".to_vec()] {
        let path = dir.path().join("invalid.redb");
        fs::write(&path, &bytes).unwrap();
        assert!(RedbStore::prepare_generation_migration(&path, declaration(&[1])).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn recovery_flag_is_refused_without_header_repair() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let mut bytes = fs::read(&path).unwrap();
    bytes[9] |= 2;
    fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        RedbStore::prepare_generation_migration(&path, declaration(&[1])),
        Err(GenerationMigrationError::RecoveryRequired)
    ));
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn bounded_physical_header_and_geometry_refuse_before_redb_repair() {
    for case in 0..9 {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(&dir);
        let mut bytes = fs::read(&path).unwrap();
        match case {
            0 => bytes.truncate(319),
            1 => bytes[9] |= 8,
            2 => bytes[12..16].copy_from_slice(&0_u32.to_le_bytes()),
            3 => bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes()),
            4 => bytes[20..24].copy_from_slice(&0_u32.to_le_bytes()),
            5 => bytes[20..24].copy_from_slice(&3_u32.to_le_bytes()),
            6 => {
                bytes[24..32].fill(0);
            }
            7 => bytes.push(0),
            _ => {
                bytes.pop();
            }
        }
        fs::write(&path, &bytes).unwrap();
        assert!(RedbStore::prepare_generation_migration(&path, declaration(&[1])).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn backup_tamper_during_staging_is_rechecked_immediately_before_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let before = snapshot(&path);
    let backup_path = dir.path().join("backup.redb");
    let backed = prepare(&path, &[1]).create_backup(&backup_path).unwrap();
    let result = backed.migrate_with_observer(|stage| {
        if stage == MigrationStage::MarkerWritten {
            OpenOptions::new()
                .append(true)
                .open(&backup_path)
                .unwrap()
                .write_all(&[7])
                .unwrap();
        }
        Ok(())
    });
    assert!(matches!(
        result,
        Err(GenerationMigrationError::BackupMismatch)
    ));
    assert_eq!(snapshot(&path), before);
}

#[test]
fn source_path_replacement_during_staging_cannot_report_committed_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let before = snapshot(&path);
    let moved = dir.path().join("moved.redb");
    let backed = prepare(&path, &[1])
        .create_backup(dir.path().join("backup.redb"))
        .unwrap();
    let result = backed.migrate_with_observer(|stage| {
        if stage == MigrationStage::MarkerWritten {
            fs::rename(&path, &moved).unwrap();
            fs::write(&path, b"replacement").unwrap();
        }
        Ok(())
    });
    assert!(matches!(
        result,
        Err(GenerationMigrationError::PathChanged(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), b"replacement");
    assert_eq!(snapshot(&moved), before);
}

#[cfg(unix)]
#[test]
fn symlinks_hardlinks_and_replaced_paths_are_refused() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let path = fresh(&dir);
    let link = dir.path().join("source-link.redb");
    symlink(&path, &link).unwrap();
    assert!(matches!(
        RedbStore::prepare_generation_migration(&link, declaration(&[1])),
        Err(GenerationMigrationError::Symlink(_))
    ));
    let hardlink = dir.path().join("backup-hardlink.redb");
    fs::hard_link(&path, &hardlink).unwrap();
    assert!(prepare(&path, &[1]).create_backup(&hardlink).is_err());
    assert!(prepare(&path, &[1])
        .verify_existing_backup(&hardlink, [0; 32])
        .is_err());
    let prepared = prepare(&path, &[1]);
    let moved = dir.path().join("moved.redb");
    fs::rename(&path, &moved).unwrap();
    fs::write(&path, b"replacement").unwrap();
    assert!(matches!(
        prepared.create_backup(dir.path().join("backup.redb")),
        Err(GenerationMigrationError::PathChanged(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), b"replacement");
    drop(RedbStore::open(&moved).unwrap());
}
