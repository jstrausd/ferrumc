//! Deterministic actual-producer migration and immutable evidence acceptance.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use ferrumc_core::{DimensionId, EntityUuid, GameMode, PlayerId, PluginId, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_world::{BlockStateId, Chunk, GenerationIdentity, WorldSeed};
use redb::{
    Database, MultimapTableHandle, ReadableTable, StorageBackend, TableDefinition, TableHandle,
};
use sha2::{Digest, Sha256};

use super::*;
use crate::redb_store::{
    CHUNK_OVERLAY_TABLE, CHUNK_TABLE, ENTITY_TABLE, GENERATION_TABLE, META_FORMAT_KEY,
    META_LAST_MUTATION_ID_KEY, META_TABLE, MUTATION_BATCH_TABLE, MUTATION_LOG_TABLE, PLAYER_TABLE,
    PLUGIN_TABLE,
};
use crate::{
    BlockMutationLogRecord, ChunkKey, ChunkOverlayRecord, ChunkRecord, EntityRecord,
    JournalBatchId, MutationActor, MutationLogCause, PersistentEntityKey, PersistentEntityRecord,
    PersistentEntityStore, PersistentEntityWrite, PlayerRecord, PlayerStore, PluginStore,
    SchemaVersion, StorageKey, WorldStore,
};

fn limits() -> UuidEntityMigrationLimits {
    UuidEntityMigrationLimits::new(64 * 1024 * 1024, 16 * 1024 * 1024).unwrap()
}
fn scope(world: u32, dimension: u32) -> crate::GenerationKey {
    crate::GenerationKey::new(WorldId::new(world), DimensionId::new(dimension))
}
fn identity(seed: i64) -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(seed))
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn fixture(path: &Path, full: bool) {
    let store = RedbStore::open_with_generation_metadata(path).unwrap();
    runtime().block_on(async {
        store
            .ensure_generation_identity(scope(1, 0), identity(11))
            .await
            .unwrap();
        store
            .ensure_generation_identity(scope(1, 7), identity(-99))
            .await
            .unwrap();
        store
            .ensure_generation_identity(scope(2, 7), identity(i64::MAX))
            .await
            .unwrap();
        if full {
            let mut chunk = Chunk::new(ChunkPos::new(2, -3));
            chunk
                .set_block(BlockPos::new(33, 64, -47), BlockStateId::new(9))
                .unwrap();
            let key = ChunkKey::new(WorldId::new(1), DimensionId::new(0), chunk.pos());
            store
                .save_chunk(key, ChunkRecord::new(SchemaVersion::new(91), chunk.clone()))
                .await
                .unwrap();
            store
                .save_chunk_overlays(vec![(
                    key,
                    ChunkOverlayRecord::try_from_chunk(
                        SchemaVersion::new(92),
                        chunk.pos(),
                        &chunk,
                        0,
                    )
                    .unwrap(),
                )])
                .await
                .unwrap();
            store
                .save_player(
                    PlayerId::offline("uuid-migration"),
                    PlayerRecord::new(SchemaVersion::new(93), GameMode::Survival, vec![3, 4, 5])
                        .unwrap(),
                )
                .await
                .unwrap();
            let plugin = PluginId::new("migration-plugin");
            store
                .put(&plugin, StorageKey::new("opaque").unwrap(), vec![9, 0, 255])
                .await
                .unwrap();
            let log = BlockMutationLogRecord::new(
                SchemaVersion::new(94),
                91,
                17,
                MutationActor::System,
                BlockPos::new(1, 64, 1),
                BlockStateId::new(1),
                BlockStateId::new(2),
                MutationLogCause::Test,
            );
            store
                .append_block_mutation_batch(JournalBatchId::from_bytes([1; 16]), vec![log, log])
                .await
                .unwrap();
            store
                .append_block_mutation_batch(JournalBatchId::from_bytes([2; 16]), vec![])
                .await
                .unwrap();
        }
    });
}

type Snapshot = BTreeMap<String, Vec<(Vec<u8>, Vec<u8>)>>;
fn snapshot_db(db: &Database) -> Snapshot {
    let txn = db.begin_read().unwrap();
    let mut out = BTreeMap::new();
    for name in txn.list_tables().unwrap() {
        let name = name.name().to_owned();
        let mut rows = vec![];
        if name == META_TABLE.name() {
            if let Ok(table) = txn.open_table(META_TABLE) {
                for row in table.iter().unwrap() {
                    let (k, v) = row.unwrap();
                    rows.push((
                        k.value().as_bytes().to_vec(),
                        v.value().to_be_bytes().to_vec(),
                    ));
                }
            } else {
                let table = txn
                    .open_table(TableDefinition::<&[u8], &[u8]>::new(&name))
                    .unwrap();
                for row in table.iter().unwrap() {
                    let (k, v) = row.unwrap();
                    rows.push((k.value().to_vec(), v.value().to_vec()));
                }
            }
        } else {
            let table = txn
                .open_table(TableDefinition::<&[u8], &[u8]>::new(&name))
                .unwrap();
            for row in table.iter().unwrap() {
                let (k, v) = row.unwrap();
                rows.push((k.value().to_vec(), v.value().to_vec()));
            }
        }
        out.insert(name, rows);
    }
    for name in txn.list_multimap_tables().unwrap() {
        out.insert(format!("multimap:{}", name.name()), vec![]);
    }
    out
}
fn snapshot(path: &Path) -> Snapshot {
    snapshot_db(&Database::open(path).unwrap())
}
fn hash(path: &Path) -> [u8; 32] {
    Sha256::digest(fs::read(path).unwrap()).into()
}
fn replace(path: &Path, table: TableDefinition<'_, &[u8], &[u8]>, key: &[u8], value: &[u8]) {
    let db = Database::open(path).unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(table).unwrap().insert(key, value).unwrap();
    txn.commit().unwrap();
}
fn remove(path: &Path, table: TableDefinition<'_, &[u8], &[u8]>, key: &[u8]) {
    let db = Database::open(path).unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(table).unwrap().remove(key).unwrap();
    txn.commit().unwrap();
}
fn injected(stage: Stage) -> UuidEntityMigrationError {
    UuidEntityMigrationError::Io {
        operation: "injected producer cutpoint",
        path: format!("{stage:?}").into(),
        source: io::Error::other("deterministic cutpoint"),
    }
}
fn rechecksum(bytes: &mut [u8]) {
    let checksum: [u8; 32] = Sha256::digest(&bytes[..312]).into();
    bytes[312..344].copy_from_slice(&checksum);
}

#[test]
fn whole_raw_nine_table_baseline_and_exact_twelve_table_delta_survive_commit_and_resume() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    let checkpoint = dir.path().join("checkpoint");
    fixture(&source, true);
    let before = snapshot(&source);
    let prepared = RedbStore::prepare_uuid_entity_migration(&source, limits()).unwrap();
    assert_eq!(
        prepared
            .report()
            .records(UuidEntityMigrationTable::GenerationIdentities),
        3
    );
    let expected = *prepared.report().expected_target_sha256();
    let backed = prepared.create_backup(&backup).unwrap();
    let remembered = *backed.backup().file_sha256();
    let scratch = dir.path().join("backup-inspection");
    fs::copy(&backup, &scratch).unwrap();
    assert_eq!(snapshot(&scratch), before);
    assert_eq!(hash(&backup), remembered);
    let authority = backed.write_checkpoint(&checkpoint).unwrap();
    assert_eq!(fs::metadata(&checkpoint).unwrap().len(), 344);
    let outcome = authority.migrate().unwrap();
    assert_eq!(outcome.kind(), UuidEntityMigrationOutcomeKind::Committed);
    assert_eq!(*outcome.target_snapshot_sha256(), expected);
    assert_eq!(hash(&backup), remembered);
    let mut after = snapshot(&source);
    assert_eq!(after.len(), 12);
    for name in [
        "ferrumc:entity_locator",
        "ferrumc:entity_chunk",
        "ferrumc:entity_chunk_epoch",
    ] {
        assert_eq!(after.remove(name), Some(vec![]));
    }
    let metadata = after.get_mut(META_TABLE.name()).unwrap();
    metadata
        .iter_mut()
        .find(|(k, _)| k == META_FORMAT_KEY.as_bytes())
        .unwrap()
        .1 = 4u64.to_be_bytes().to_vec();
    assert_eq!(after, before);
    let saved_checkpoint = fs::read(&checkpoint).unwrap();
    for _ in 0..2 {
        let UuidEntityMigrationResume::AlreadyCompleted(outcome) =
            RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits())
                .unwrap()
        else {
            panic!("must prove completion")
        };
        assert_eq!(
            outcome.kind(),
            UuidEntityMigrationOutcomeKind::VerifiedCompleted
        );
        assert_eq!(*outcome.target_snapshot_sha256(), expected);
        assert_eq!(hash(&backup), remembered);
        assert_eq!(fs::read(&checkpoint).unwrap(), saved_checkpoint);
    }
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, limits()),
        Err(UuidEntityMigrationError::CompletedVerificationRequiresCheckpoint)
    ));
    drop(RedbStore::open_with_uuid_entities(&source).unwrap());
    assert!(RedbStore::open_with_generation_metadata(&source).is_err());
    assert!(RedbStore::open(&source).is_err());
}

#[test]
fn actual_source_row_cutpoints_abort_whole_transaction_and_lost_reply_is_checked() {
    let cuts = [
        Stage::TableCreated(0),
        Stage::TableCreated(1),
        Stage::TableCreated(2),
        Stage::MarkerWritten,
        Stage::VerificationCreated,
        Stage::VerificationCopied,
        Stage::VerificationValidated,
        Stage::VerificationCleaned,
        Stage::StagedValidated,
        Stage::BeforeCommit,
        Stage::Committed,
    ];
    for cut in cuts {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        fixture(&source, true);
        let before = snapshot(&source);
        let authority = RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint)
            .unwrap();
        let backup_hash = hash(&backup);
        let checkpoint_bytes = fs::read(&checkpoint).unwrap();
        let mut reached = false;
        assert!(authority
            .migrate_observed(&mut |stage| {
                if stage == cut {
                    reached = true;
                    Err(injected(stage))
                } else {
                    Ok(())
                }
            })
            .is_err());
        assert!(reached, "{cut:?}");
        assert_eq!(hash(&backup), backup_hash);
        assert_eq!(fs::read(&checkpoint).unwrap(), checkpoint_bytes);
        if cut == Stage::Committed {
            assert!(matches!(
                RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits())
                    .unwrap(),
                UuidEntityMigrationResume::AlreadyCompleted(_)
            ));
        } else {
            assert_eq!(snapshot(&source), before);
            assert!(matches!(
                RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits())
                    .unwrap(),
                UuidEntityMigrationResume::Ready(_)
            ));
        }
    }
}

#[test]
fn backup_real_producer_cutpoints_never_change_source_and_never_open_recorded_backup() {
    for cut in [
        Stage::BackupCreated,
        Stage::BackupRow,
        Stage::BackupCommitted,
        Stage::BackupClosed,
        Stage::BackupSynced,
        Stage::BackupHashed,
        Stage::VerificationCreated,
        Stage::VerificationCopied,
        Stage::VerificationValidated,
        Stage::VerificationCleaned,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        fixture(&source, true);
        let before = snapshot(&source);
        let prepared = RedbStore::prepare_uuid_entity_migration(&source, limits()).unwrap();
        let mut reached = false;
        let mut recorded = None;
        assert!(prepared
            .create_backup_observed(&backup, &mut |stage| {
                if stage == Stage::BackupHashed {
                    recorded = Some(hash(&backup));
                }
                if stage == cut {
                    reached = true;
                    Err(injected(stage))
                } else {
                    Ok(())
                }
            })
            .is_err());
        assert!(reached);
        assert_eq!(snapshot(&source), before);
        if let Some(recorded) = recorded {
            assert_eq!(hash(&backup), recorded);
        }
        assert!(!fs::read_dir(dir.path()).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".ferrumc-uuid-migration-")
        }));
    }
}

#[test]
fn checkpoint_real_publication_cutpoints_recover_only_known_complete_evidence() {
    for cut in [
        Stage::CheckpointCreated,
        Stage::CheckpointWritten,
        Stage::CheckpointFileSynced,
        Stage::CheckpointLinked,
        Stage::CheckpointDirectorySynced,
        Stage::CheckpointStagingUnlinked,
        Stage::CheckpointReady,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        fixture(&source, true);
        let before = snapshot(&source);
        let backed = RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap();
        let remembered = hash(&backup);
        let mut reached = false;
        assert!(backed
            .write_checkpoint_observed(&checkpoint, &mut |stage| {
                if stage == cut {
                    reached = true;
                    Err(injected(stage))
                } else {
                    Ok(())
                }
            })
            .is_err());
        assert!(reached);
        assert_eq!(snapshot(&source), before);
        assert_eq!(hash(&backup), remembered);
        if checkpoint.exists() {
            let UuidEntityMigrationResume::Ready(ready) =
                RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits())
                    .unwrap()
            else {
                panic!("source remains4")
            };
            ready.checkpoint.verify(true).unwrap();
            drop(ready);
        } else {
            assert!(!matches!(
                RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()),
                Ok(UuidEntityMigrationResume::AlreadyCompleted(_))
            ));
        }
    }
}

#[test]
fn closed_backup_reuse_is_hash_bound_and_copy_only_including_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    fixture(&source, true);
    let backed = RedbStore::prepare_uuid_entity_migration(&source, limits())
        .unwrap()
        .create_backup(&backup)
        .unwrap();
    let remembered = *backed.backup().file_sha256();
    drop(backed);
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .verify_existing_backup(&backup, [0; 32]),
        Err(UuidEntityMigrationError::BackupMismatch)
    ));
    assert_eq!(hash(&backup), remembered);
    drop(
        RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .verify_existing_backup(&backup, remembered)
            .unwrap(),
    );
    assert_eq!(hash(&backup), remembered);
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup),
        Err(UuidEntityMigrationError::EvidenceExists(_))
    ));
    assert_eq!(hash(&backup), remembered);
}

#[test]
fn exact_and_exceeded_scratch_budgets_preserve_original_files() {
    assert!(UuidEntityMigrationLimits::new(0, 1).is_err());
    assert!(UuidEntityMigrationLimits::new(1, 0).is_err());
    assert!(UuidEntityMigrationLimits::new(u64::MAX, 1).is_err());
    assert!(UuidEntityMigrationLimits::new(u64::MAX - 344, 1).is_err());
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    fixture(&source, true);
    let before = snapshot(&source);
    let backed = RedbStore::prepare_uuid_entity_migration(&source, limits())
        .unwrap()
        .create_backup(&backup)
        .unwrap();
    let remembered = hash(&backup);
    let length = backed.backup().file_len();
    drop(backed);
    let small = UuidEntityMigrationLimits::new(length - 1, 16 * 1024 * 1024).unwrap();
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, small)
            .unwrap()
            .verify_existing_backup(&backup, remembered),
        Err(UuidEntityMigrationError::ScratchBudgetExceeded {
            kind: UuidEntityMigrationScratchKind::VerificationCopy,
            ..
        })
    ));
    assert_eq!(hash(&backup), remembered);
    assert_eq!(snapshot(&source), before);
    let small = UuidEntityMigrationLimits::new(64 * 1024 * 1024, 1).unwrap();
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, small),
        Err(UuidEntityMigrationError::ScratchBudgetExceeded {
            kind: UuidEntityMigrationScratchKind::ReceiptIndex,
            ..
        })
    ));
    assert_eq!(snapshot(&source), before);
    let budget = files::DiskBudget::new(UuidEntityMigrationScratchKind::VerificationCopy, 10);
    assert!(budget.check(Some(10)).is_ok());
    assert!(budget.check(Some(11)).is_err());
    assert!(budget.check(u64::MAX.checked_add(1)).is_err());
    let exact = UuidEntityMigrationLimits::new(length, 16 * 1024 * 1024).unwrap();
    drop(
        RedbStore::prepare_uuid_entity_migration(&source, exact)
            .unwrap()
            .verify_existing_backup(&backup, remembered)
            .unwrap(),
    );
    assert_eq!(hash(&backup), remembered);
}

#[test]
fn fixed_checkpoint_parser_refuses_all_lengths_and_rechecks_checksums_constants_counts() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    let checkpoint = dir.path().join("checkpoint");
    fixture(&source, false);
    drop(
        RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint)
            .unwrap(),
    );
    let bytes = fs::read(&checkpoint).unwrap();
    assert!(checkpoint::Data::decode(&bytes).is_ok());
    for length in [0, 1, 8, 311, 312, 343, 345, 512] {
        let input = vec![0; length];
        assert!(matches!(
            checkpoint::Data::decode(&input),
            Err(UuidEntityMigrationError::CheckpointMalformed(_))
        ));
    }
    for offset in [0, 8, 10, 12, 14, 16, 24, 112, 120, 144, 152, 160, 168, 312] {
        let mut wrong = bytes.clone();
        wrong[offset] ^= 1;
        if offset < 312 {
            rechecksum(&mut wrong);
        }
        assert!(checkpoint::Data::decode(&wrong).is_err(), "offset{offset}");
    }
    let mut wrong = bytes.clone();
    wrong[216] ^= 1;
    assert!(checkpoint::Data::decode(&wrong).is_err());
}

#[test]
fn source_backup_checkpoint_locks_fail_fast_and_hardlinks_are_scoped() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    let checkpoint = dir.path().join("checkpoint");
    fixture(&source, false);
    let prepared = RedbStore::prepare_uuid_entity_migration(&source, limits()).unwrap();
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, limits()),
        Err(UuidEntityMigrationError::Busy(_))
    ));
    drop(prepared);
    let backed = RedbStore::prepare_uuid_entity_migration(&source, limits())
        .unwrap()
        .create_backup(&backup)
        .unwrap();
    assert!(matches!(
        files::LockedFile::open(&backup, false, false),
        Err(UuidEntityMigrationError::Busy(_))
    ));
    let ready = backed.write_checkpoint(&checkpoint).unwrap();
    assert!(matches!(
        checkpoint::Authority::load(&checkpoint),
        Err(UuidEntityMigrationError::Busy(_))
    ));
    drop(ready);
    let alias = dir.path().join("source-alias");
    fs::hard_link(&source, &alias).unwrap();
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, limits()),
        Err(UuidEntityMigrationError::UnexpectedFileAlias(_))
    ));
    fs::remove_file(&alias).unwrap();
    let alias = dir.path().join("checkpoint-unrelated");
    fs::hard_link(&checkpoint, &alias).unwrap();
    assert!(matches!(
        RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()),
        Err(UuidEntityMigrationError::UnexpectedFileAlias(_))
    ));
    fs::remove_file(&alias).unwrap();
    let link = dir.path().join("symlink");
    std::os::unix::fs::symlink(&source, &link).unwrap();
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&link, limits()),
        Err(UuidEntityMigrationError::Symlink(_))
    ));
}

#[test]
fn path_replacement_and_late_raw_evidence_change_abort_source_without_foreign_delete() {
    for replace_checkpoint in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        fixture(&source, true);
        let before = snapshot(&source);
        let authority = RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint)
            .unwrap();
        let replaced = if replace_checkpoint {
            &checkpoint
        } else {
            &source
        };
        let moved = dir.path().join("moved");
        assert!(authority
            .migrate_observed(&mut |stage| {
                if stage == Stage::BeforeCommit {
                    fs::rename(replaced, &moved).unwrap();
                    fs::write(replaced, b"foreign replacement").unwrap();
                }
                Ok(())
            })
            .is_err());
        assert_eq!(fs::read(replaced).unwrap(), b"foreign replacement");
        assert_eq!(
            snapshot(if replace_checkpoint { &source } else { &moved }),
            before
        );
    }
}

#[test]
fn own_temporary_cleanup_preserves_replaced_paths_and_original_backup() {
    let dir = tempfile::tempdir().unwrap();
    let temporary = files::TemporaryFile::create(dir.path(), "cleanup").unwrap();
    let path = temporary.file().unwrap().path.clone();
    let moved = dir.path().join("moved");
    fs::rename(&path, &moved).unwrap();
    fs::write(&path, b"foreign").unwrap();
    assert!(matches!(
        temporary.cleanup(),
        Err(UuidEntityMigrationError::Cleanup { .. })
    ));
    assert_eq!(fs::read(&path).unwrap(), b"foreign");
    assert!(moved.exists());
}

#[test]
fn actual_budget_backend_refuses_extent_and_overflow_before_any_file_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let temporary = files::TemporaryFile::create(dir.path(), "budget").unwrap();
    let file = temporary.file().unwrap();
    let path = file.path.clone();
    let budget = files::DiskBudget::new(UuidEntityMigrationScratchKind::ReceiptIndex, 8);
    let backend = files::SharedBackend::new(file.backend(), Some(std::sync::Arc::clone(&budget)));
    backend.set_len(8).unwrap();
    backend.write(0, &[1; 8]).unwrap();
    backend.write(8, &[]).unwrap();
    for extent in [9, u64::MAX] {
        assert!(backend.set_len(extent).is_err());
        assert_eq!(backend.len().unwrap(), 8);
        assert_eq!(fs::read(&path).unwrap(), [1; 8]);
    }
    for (offset, bytes) in [(7, &[2; 2][..]), (u64::MAX, &[2][..]), (9, &[][..])] {
        assert!(backend.write(offset, bytes).is_err());
        assert_eq!(backend.len().unwrap(), 8);
        assert_eq!(fs::read(&path).unwrap(), [1; 8]);
    }
    assert!(matches!(
        budget.error(),
        Some(UuidEntityMigrationError::ScratchBudgetExceeded {
            kind: UuidEntityMigrationScratchKind::ReceiptIndex,
            limit: 8,
        })
    ));
    drop(backend);
    temporary.cleanup().unwrap();
    assert!(!path.exists());
}

#[test]
fn payload_caps_chunk_position_domain_and_valid_old_entity_rows_are_refused_without_changes() {
    for kind in 0..10 {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fixture(&source, false);
        let key = ChunkKey::new(WorldId::new(1), DimensionId::new(0), ChunkPos::new(2, -3));
        let raw_key = crate::codec::chunk_key_bytes(key);
        match kind {
            0 => replace(
                &source,
                CHUNK_TABLE,
                &raw_key,
                &vec![0; crate::MAX_CHUNK_RECORD_LEN + 1],
            ),
            1 => replace(
                &source,
                CHUNK_OVERLAY_TABLE,
                &raw_key,
                &vec![0; crate::MAX_CHUNK_RECORD_LEN + 1],
            ),
            2 => {
                let mut bytes = vec![0; 5 + crate::MAX_PLAYER_DATA_LEN + 1];
                bytes[..4].copy_from_slice(&1u32.to_be_bytes());
                replace(&source, PLAYER_TABLE, &[0; 16], &bytes);
            }
            3 => {
                let bytes = crate::codec::encode_chunk_record(&ChunkRecord::new(
                    SchemaVersion::new(1),
                    Chunk::new(ChunkPos::new(3, -3)),
                ))
                .unwrap();
                replace(&source, CHUNK_TABLE, &raw_key, &bytes);
            }
            4 => {
                let foreign = Chunk::new(ChunkPos::new(3, -3));
                let bytes = crate::codec::encode_chunk_overlay_record(
                    &ChunkOverlayRecord::try_from_chunk(
                        SchemaVersion::new(1),
                        foreign.pos(),
                        &foreign,
                        0,
                    )
                    .unwrap(),
                )
                .unwrap();
                replace(&source, CHUNK_OVERLAY_TABLE, &raw_key, &bytes);
            }
            5 | 6 => {
                let table = if kind == 5 {
                    CHUNK_TABLE
                } else {
                    CHUNK_OVERLAY_TABLE
                };
                let invalid = ChunkKey::new(
                    WorldId::new(1),
                    DimensionId::new(0),
                    ChunkPos::new(i32::MAX, 0),
                );
                replace(&source, table, &crate::codec::chunk_key_bytes(invalid), &[]);
            }
            7 | 8 => {
                let key = crate::EntityKey::new(
                    WorldId::new(1),
                    DimensionId::new(0),
                    ferrumc_core::EntityId::new(42),
                );
                let record = EntityRecord::new(
                    SchemaVersion::new(7),
                    if kind == 7 {
                        vec![]
                    } else {
                        vec![1; crate::MAX_ENTITY_DATA_LEN]
                    },
                )
                .unwrap();
                replace(
                    &source,
                    ENTITY_TABLE,
                    &crate::codec::entity_key_bytes(key),
                    &crate::codec::encode_entity_record(&record),
                );
            }
            9 => replace(&source, PLAYER_TABLE, &[0; 16], &[0, 0, 0, 1, 4]),
            _ => unreachable!(),
        }
        let before = snapshot(&source);
        let result = RedbStore::prepare_uuid_entity_migration(&source, limits());
        if kind == 7 || kind == 8 {
            assert!(matches!(
                result,
                Err(UuidEntityMigrationError::LegacyEntitiesPresent)
            ));
        } else {
            assert!(
                matches!(result, Err(UuidEntityMigrationError::Record { .. })),
                "case{kind}"
            );
        }
        assert_eq!(snapshot(&source), before);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}

#[test]
fn resume_source_inode_is_binding_and_observed_length_is_informational() {
    for replaced in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        fixture(&source, false);
        let authority = RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint)
            .unwrap();
        let before = authority.report().snapshot_sha256().to_owned();
        drop(authority);
        let remembered = hash(&backup);
        let mut bytes = fs::read(&checkpoint).unwrap();
        bytes[48..56].copy_from_slice(&1u64.to_be_bytes());
        rechecksum(&mut bytes);
        fs::write(&checkpoint, bytes).unwrap();
        if replaced {
            let old = dir.path().join("old-source");
            fs::rename(&source, &old).unwrap();
            fs::copy(&old, &source).unwrap();
        } else {
            // Clean source open/close housekeeping is permitted before resume.
            drop(Database::open(&source).unwrap());
        }
        let result =
            RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits());
        if replaced {
            assert!(matches!(
                result,
                Err(UuidEntityMigrationError::SourceChanged)
            ));
        } else {
            let UuidEntityMigrationResume::Ready(value) = result.unwrap() else {
                panic!("format4 authority required")
            };
            assert_eq!(value.report().source_file_len(), 1);
            assert_eq!(*value.report().snapshot_sha256(), before);
            value.migrate().unwrap();
        }
        assert_eq!(hash(&backup), remembered);
    }
}

#[test]
fn changed_new_table_grammar_refuses_completed_resume_without_repair() {
    for table in [
        super::super::uuid_entities::LOCATOR_TABLE,
        super::super::uuid_entities::INDEX_TABLE,
        super::super::uuid_entities::EPOCH_TABLE,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        fixture(&source, false);
        RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint)
            .unwrap()
            .migrate()
            .unwrap();
        replace(&source, table, &[0], &[]);
        let before = snapshot(&source);
        let remembered = hash(&backup);
        let checkpoint_bytes = fs::read(&checkpoint).unwrap();
        assert!(matches!(
            RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()),
            Err(UuidEntityMigrationError::Validation(_))
        ));
        assert_eq!(snapshot(&source), before);
        assert_eq!(hash(&backup), remembered);
        assert_eq!(fs::read(&checkpoint).unwrap(), checkpoint_bytes);
    }
}

#[test]
fn each_existing_wire_entity_and_all_scoped_unscoped_grammar_failures_refuse_without_raw_changes() {
    for kind in 0..18 {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fixture(&source, true);
        match kind {
            0 => replace(&source, ENTITY_TABLE, &[0; 12], &[0; 6]),
            1 => replace(&source, ENTITY_TABLE, &[1; 24], &[]),
            2 => replace(&source, GENERATION_TABLE, &[0; 7], &identity(1).encode()),
            3 => replace(&source, GENERATION_TABLE, &scope(1, 0).encode(), &[0; 26]),
            4 => remove(&source, GENERATION_TABLE, &scope(1, 0).encode()),
            5 => replace(&source, CHUNK_TABLE, &[0; 15], &[]),
            6 => replace(&source, CHUNK_OVERLAY_TABLE, &[0; 17], &[]),
            7 => replace(&source, PLAYER_TABLE, &[0; 15], &[]),
            8 => replace(&source, PLAYER_TABLE, &[0; 16], &[]),
            9 => replace(&source, PLUGIN_TABLE, &[0; 3], &[]),
            10 => replace(&source, PLUGIN_TABLE, &[255; 4], &[]),
            11 => replace(&source, PLUGIN_TABLE, &[0, 0, 0, 1, 255], &[]),
            12 => replace(&source, MUTATION_LOG_TABLE, &[0; 7], &[]),
            13 => remove(&source, MUTATION_LOG_TABLE, &0u64.to_be_bytes()),
            14 => replace(&source, MUTATION_BATCH_TABLE, &[3; 15], &[]),
            15 => replace(&source, MUTATION_BATCH_TABLE, &[3; 16], &[0; 7]),
            16 => {
                let db = Database::open(&source).unwrap();
                let txn = db.begin_write().unwrap();
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert("unknown", 1)
                    .unwrap();
                txn.commit().unwrap();
            }
            17 => {
                let db = Database::open(&source).unwrap();
                let txn = db.begin_write().unwrap();
                txn.open_table(META_TABLE)
                    .unwrap()
                    .insert(META_LAST_MUTATION_ID_KEY, 99)
                    .unwrap();
                txn.commit().unwrap();
            }
            _ => unreachable!(),
        }
        let before = snapshot(&source);
        assert!(
            RedbStore::prepare_uuid_entity_migration(&source, limits()).is_err(),
            "case{kind}"
        );
        assert_eq!(snapshot(&source), before);
    }
}

#[test]
// Keep all nine valid post-migration changes beside their shared retry evidence.
#[allow(clippy::too_many_lines)]
fn format5_gameplay_and_any_existing_table_change_refuse_identical_migration_retry() {
    for kind in 0..9 {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        fixture(&source, true);
        RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint)
            .unwrap()
            .migrate()
            .unwrap();
        let store = RedbStore::open_with_uuid_entities(&source).unwrap();
        runtime().block_on(async {
            match kind {
                0 => {
                    store
                        .put(
                            &PluginId::new("new"),
                            StorageKey::new("key").unwrap(),
                            vec![1],
                        )
                        .await
                        .unwrap();
                }
                1 => {
                    store
                        .save_player(
                            PlayerId::offline("new"),
                            PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![])
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                }
                2 => {
                    store
                        .save_chunk(
                            ChunkKey::new(
                                WorldId::new(1),
                                DimensionId::new(0),
                                ChunkPos::new(3, 3),
                            ),
                            ChunkRecord::new(
                                SchemaVersion::new(1),
                                Chunk::new(ChunkPos::new(3, 3)),
                            ),
                        )
                        .await
                        .unwrap();
                }
                3 => {
                    store
                        .append_block_mutation_batch(JournalBatchId::from_bytes([99; 16]), vec![])
                        .await
                        .unwrap();
                }
                4 | 5 => {
                    let key = PersistentEntityKey::new(
                        WorldId::new(1),
                        DimensionId::new(0),
                        EntityUuid::from_bytes([7; 16]),
                    );
                    let record = PersistentEntityRecord::new(
                        ChunkPos::new(0, 0),
                        EntityRecord::new(SchemaVersion::new(1), vec![]).unwrap(),
                    )
                    .unwrap();
                    store
                        .save_persistent_entity(PersistentEntityWrite::new(
                            key,
                            identity(11),
                            record,
                        ))
                        .await
                        .unwrap();
                    if kind == 5 {
                        store
                            .delete_persistent_entity(key, identity(11))
                            .await
                            .unwrap();
                    }
                }
                6 => {
                    let mut chunk = Chunk::new(ChunkPos::new(2, -3));
                    chunk
                        .set_block(BlockPos::new(33, 64, -47), BlockStateId::new(10))
                        .unwrap();
                    store
                        .save_chunk_overlays(vec![(
                            ChunkKey::new(WorldId::new(1), DimensionId::new(0), chunk.pos()),
                            ChunkOverlayRecord::try_from_chunk(
                                SchemaVersion::new(1),
                                chunk.pos(),
                                &chunk,
                                0,
                            )
                            .unwrap(),
                        )])
                        .await
                        .unwrap();
                }
                7 => {
                    store
                        .append_block_mutation_batch(
                            JournalBatchId::from_bytes([99; 16]),
                            vec![BlockMutationLogRecord::new(
                                SchemaVersion::new(1),
                                91,
                                17,
                                MutationActor::System,
                                BlockPos::new(1, 64, 1),
                                BlockStateId::new(2),
                                BlockStateId::new(3),
                                MutationLogCause::Test,
                            )],
                        )
                        .await
                        .unwrap();
                }
                8 => {}
                _ => unreachable!(),
            }
        });
        drop(store);
        if kind == 8 {
            replace(
                &source,
                GENERATION_TABLE,
                &scope(1, 0).encode(),
                &identity(12).encode(),
            );
        }
        let before = snapshot(&source);
        let remembered = hash(&backup);
        assert!(matches!(
            RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()),
            Err(UuidEntityMigrationError::MigrationDeltaMismatch)
        ));
        assert_eq!(snapshot(&source), before);
        assert_eq!(hash(&backup), remembered);
        drop(RedbStore::open_with_uuid_entities(&source).unwrap());
    }
}

#[test]
fn actual_backend_commit_sync_error_is_uncertain_and_evidence_is_immutable() {
    use std::sync::atomic::Ordering;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    let checkpoint = dir.path().join("checkpoint");
    fixture(&source, true);
    let authority = RedbStore::prepare_uuid_entity_migration(&source, limits())
        .unwrap()
        .create_backup(&backup)
        .unwrap()
        .write_checkpoint(&checkpoint)
        .unwrap();
    let flag = std::sync::Arc::clone(&authority.backed.prepared.source.fail_sync);
    let remembered = hash(&backup);
    let bytes = fs::read(&checkpoint).unwrap();
    let result = authority.migrate_observed(&mut |stage| {
        if stage == Stage::BeforeCommit {
            flag.store(true, Ordering::Relaxed);
        }
        Ok(())
    });
    assert!(matches!(
        result,
        Err(UuidEntityMigrationError::CommitUncertain { .. })
    ));
    assert_eq!(hash(&backup), remembered);
    assert_eq!(fs::read(&checkpoint).unwrap(), bytes);
    match RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()) {
        Ok(
            UuidEntityMigrationResume::Ready(_) | UuidEntityMigrationResume::AlreadyCompleted(_),
        )
        | Err(UuidEntityMigrationError::RecoveryRequired) => {}
        other => panic!("must use checked state or separate recovery: {other:?}"),
    }
}

#[test]
fn blank_and_plugin_only_without_descriptors_migrate_without_inference() {
    for plugin_only in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        let store = RedbStore::open_with_generation_metadata(&source).unwrap();
        if plugin_only {
            runtime()
                .block_on(store.put(
                    &PluginId::new("only-plugin"),
                    StorageKey::new("key").unwrap(),
                    vec![3],
                ))
                .unwrap();
        }
        drop(store);
        let value = RedbStore::prepare_uuid_entity_migration(&source, limits()).unwrap();
        assert_eq!(
            value
                .report()
                .records(UuidEntityMigrationTable::GenerationIdentities),
            0
        );
        assert_ne!(
            value.report().snapshot_sha256(),
            value.report().expected_target_sha256()
        );
        value
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint)
            .unwrap()
            .migrate()
            .unwrap();
        let store = RedbStore::open_with_uuid_entities(&source).unwrap();
        assert_eq!(
            runtime()
                .block_on(store.load_generation_identity(scope(1, 0)))
                .unwrap(),
            None
        );
    }
}

#[test]
fn format3_foreign_catalog_multimap_missing_and_wrong_table_type_refuse_exactly() {
    for kind in 0..6 {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        if kind == 0 {
            drop(RedbStore::open(&source).unwrap());
        } else {
            fixture(&source, false);
            let db = Database::open(&source).unwrap();
            let txn = db.begin_write().unwrap();
            match kind {
                1 => {
                    txn.open_table(TableDefinition::<&[u8], &[u8]>::new("foreign"))
                        .unwrap()
                        .insert(&[1][..], &[2][..])
                        .unwrap();
                }
                2 => {
                    txn.open_multimap_table(redb::MultimapTableDefinition::<&[u8], &[u8]>::new(
                        "foreign_multimap",
                    ))
                    .unwrap()
                    .insert(&[1][..], &[2][..])
                    .unwrap();
                }
                3 => {
                    txn.delete_table(GENERATION_TABLE).unwrap();
                }
                4 => {
                    txn.delete_table(META_TABLE).unwrap();
                    txn.open_table(TableDefinition::<&[u8], &[u8]>::new("ferrumc:meta"))
                        .unwrap()
                        .insert(&[1][..], &[2][..])
                        .unwrap();
                }
                5 => {
                    txn.open_table(super::super::uuid_entities::LOCATOR_TABLE)
                        .unwrap();
                }
                _ => unreachable!(),
            }
            txn.commit().unwrap();
        }
        let before = snapshot(&source);
        assert!(RedbStore::prepare_uuid_entity_migration(&source, limits()).is_err());
        assert_eq!(snapshot(&source), before);
    }
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    assert!(RedbStore::prepare_uuid_entity_migration(&missing, limits()).is_err());
    assert!(!missing.exists());
}

#[test]
fn namespace_utf8_bytes_and_key_value_limits_are_checked_before_backup() {
    for (namespace, key, value, valid) in [
        ("a".repeat(4096), "k".to_owned(), vec![], true),
        ("ä".repeat(2048), "k".to_owned(), vec![], true),
        ("a".repeat(4097), "k".to_owned(), vec![], false),
        (
            format!("{}a", "ä".repeat(2048)),
            "k".to_owned(),
            vec![],
            false,
        ),
        ("p".to_owned(), "k".repeat(512), vec![1; 1024 * 1024], true),
        ("p".to_owned(), "k".repeat(513), vec![], false),
        (
            "p".to_owned(),
            "k".to_owned(),
            vec![1; 1024 * 1024 + 1],
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fixture(&source, false);
        let mut raw = (namespace.len() as u32).to_be_bytes().to_vec();
        raw.extend_from_slice(namespace.as_bytes());
        raw.extend_from_slice(key.as_bytes());
        replace(&source, PLUGIN_TABLE, &raw, &value);
        let before = snapshot(&source);
        assert_eq!(
            RedbStore::prepare_uuid_entity_migration(&source, limits()).is_ok(),
            valid
        );
        assert_eq!(snapshot(&source), before);
    }
}

#[test]
fn actual_descriptor_cap_and_unscoped_presence_require_real_existing_identity() {
    for count in [4096usize, 4097] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fixture(&source, false);
        let db = Database::open(&source).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn.open_table(GENERATION_TABLE).unwrap();
            for index in 0..count - 3 {
                table
                    .insert(
                        scope(100 + index as u32, 0).encode().as_slice(),
                        identity(i64::try_from(index).unwrap()).encode().as_slice(),
                    )
                    .unwrap();
            }
        }
        txn.commit().unwrap();
        drop(db);
        let before = snapshot(&source);
        let result = RedbStore::prepare_uuid_entity_migration(&source, limits());
        if count == 4096 {
            assert_eq!(
                result
                    .unwrap()
                    .report()
                    .records(UuidEntityMigrationTable::GenerationIdentities),
                4096
            );
        } else {
            assert!(matches!(
                result,
                Err(UuidEntityMigrationError::TooManyDescriptors)
            ));
        }
        assert_eq!(snapshot(&source), before);
    }
    for table in [PLAYER_TABLE, MUTATION_BATCH_TABLE] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        drop(RedbStore::open_with_generation_metadata(&source).unwrap());
        let bytes = if table.name() == PLAYER_TABLE.name() {
            crate::codec::encode_player_record(
                &PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![]).unwrap(),
            )
        } else {
            vec![0; 16]
        };
        replace(&source, table, &[1; 16], &bytes);
        let before = snapshot(&source);
        assert!(RedbStore::prepare_uuid_entity_migration(&source, limits()).is_err());
        assert_eq!(snapshot(&source), before);
    }
}

#[test]
fn receipt_overlap_invalid_empty_range_missing_range_and_header_id_mismatch_refuse() {
    for (first, count) in [(0u64, 1u64), (1, 1), (2, 1), (0, 65), (1, 0), (u64::MAX, 2)] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fixture(&source, true);
        let mut receipt = first.to_be_bytes().to_vec();
        receipt.extend_from_slice(&count.to_be_bytes());
        replace(&source, MUTATION_BATCH_TABLE, &[99; 16], &receipt);
        let before = snapshot(&source);
        assert!(RedbStore::prepare_uuid_entity_migration(&source, limits()).is_err());
        assert_eq!(snapshot(&source), before);
    }
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    fixture(&source, true);
    let log = BlockMutationLogRecord::new(
        SchemaVersion::new(1),
        99,
        3,
        MutationActor::System,
        BlockPos::new(0, 64, 0),
        BlockStateId::new(1),
        BlockStateId::new(2),
        MutationLogCause::Test,
    );
    replace(
        &source,
        MUTATION_LOG_TABLE,
        &0u64.to_be_bytes(),
        &crate::codec::encode_mutation_log_record(&log),
    );
    let before = snapshot(&source);
    assert!(RedbStore::prepare_uuid_entity_migration(&source, limits()).is_err());
    assert_eq!(snapshot(&source), before);
}

#[test]
fn recovery_and_geometry_guards_refuse_before_redb_housekeeping() {
    for kind in 0..5 {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fixture(&source, false);
        let mut bytes = fs::read(&source).unwrap();
        match kind {
            0 => bytes[9] |= 2,
            1 => bytes[12..16].copy_from_slice(&8192u32.to_le_bytes()),
            2 => bytes[16..20].copy_from_slice(&0u32.to_le_bytes()),
            3 => bytes[20..24].copy_from_slice(&3u32.to_le_bytes()),
            4 => bytes.push(0),
            _ => unreachable!(),
        }
        fs::write(&source, &bytes).unwrap();
        assert!(matches!(
            RedbStore::prepare_uuid_entity_migration(&source, limits()),
            Err(UuidEntityMigrationError::RecoveryRequired
                | UuidEntityMigrationError::InvalidRedbHeader)
        ));
        assert_eq!(fs::read(&source).unwrap(), bytes);
    }
}

#[test]
fn existing_checkpoint_no_clobber_and_unknown_known_stage_aliases_refuse_without_delete() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    let checkpoint = dir.path().join("checkpoint");
    fixture(&source, false);
    fs::write(&checkpoint, b"foreign checkpoint").unwrap();
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap()
            .write_checkpoint(&checkpoint),
        Err(UuidEntityMigrationError::EvidenceExists(_))
    ));
    assert_eq!(fs::read(&checkpoint).unwrap(), b"foreign checkpoint");
    for kind in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let backup = dir.path().join("backup");
        let checkpoint = dir.path().join("checkpoint");
        fixture(&source, false);
        let backed = RedbStore::prepare_uuid_entity_migration(&source, limits())
            .unwrap()
            .create_backup(&backup)
            .unwrap();
        assert!(backed
            .write_checkpoint_observed(
                &checkpoint,
                &mut |stage| if stage == Stage::CheckpointLinked {
                    Err(injected(stage))
                } else {
                    Ok(())
                }
            )
            .is_err());
        let bytes = fs::read(&checkpoint).unwrap();
        let uuid = uuid::Uuid::from_slice(&bytes[296..312]).unwrap();
        let known = dir
            .path()
            .join(format!(".ferrumc-uuid-checkpoint-{uuid}.tmp"));
        let foreign = dir.path().join("foreign");
        match kind {
            0 => fs::hard_link(&checkpoint, &foreign).unwrap(),
            1 => fs::rename(&known, &foreign).unwrap(),
            2 => {
                fs::rename(&known, &foreign).unwrap();
                fs::write(&known, b"foreign stage").unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits())
                .is_err()
        );
        assert!(foreign.exists());
        assert_eq!(fs::read(&checkpoint).unwrap(), bytes);
        if kind == 2 {
            assert_eq!(fs::read(&known).unwrap(), b"foreign stage");
        }
    }
}
