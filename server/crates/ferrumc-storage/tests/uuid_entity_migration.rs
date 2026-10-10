//! Public offline migration and fresh UUID-mode restart contract.
#![cfg(unix)]

use ferrumc_core::{DimensionId, EntityUuid, PluginId, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::{
    EntityRecord, GenerationKey, GenerationMigrationPreparation, LegacyGenerationDeclaration,
    PersistentEntityKey, PersistentEntityRecord, PersistentEntityStore, PersistentEntityWrite,
    PluginStore, RedbStore, SchemaVersion, StorageKey, UuidEntityMigrationError,
    UuidEntityMigrationLimits, UuidEntityMigrationOutcomeKind, UuidEntityMigrationResume,
    UuidEntityMigrationTable, WorldStore,
};
use ferrumc_world::{GenerationIdentity, WorldSeed};

fn limits() -> UuidEntityMigrationLimits {
    UuidEntityMigrationLimits::new(64 * 1024 * 1024, 16 * 1024 * 1024).unwrap()
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(17), DimensionId::new(2))
}
fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(-42))
}

#[test]
fn public_checkpoint_resume_before_commit_and_fresh_uuid_writes_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("world.redb");
    let backup = dir.path().join("before-v5.redb");
    let checkpoint = dir.path().join("migration.checkpoint");
    let store = RedbStore::open_with_generation_metadata(&source).unwrap();
    runtime()
        .block_on(store.ensure_generation_identity(scope(), identity()))
        .unwrap();
    runtime()
        .block_on(store.put(
            &PluginId::new("fixture"),
            StorageKey::new("opaque").unwrap(),
            vec![0, 255],
        ))
        .unwrap();
    drop(store);
    let prepared = RedbStore::prepare_uuid_entity_migration(&source, limits()).unwrap();
    assert_eq!(prepared.report().source_format(), 4);
    assert_eq!(prepared.report().target_format(), 5);
    assert_eq!(
        prepared
            .report()
            .records(UuidEntityMigrationTable::GenerationIdentities),
        1
    );
    let backed = prepared.create_backup(&backup).unwrap();
    let recorded = *backed.backup().file_sha256();
    drop(backed.write_checkpoint(&checkpoint).unwrap());
    let UuidEntityMigrationResume::Ready(ready) =
        RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()).unwrap()
    else {
        panic!("source4 must return authority")
    };
    let outcome = ready.migrate().unwrap();
    assert_eq!(outcome.kind(), UuidEntityMigrationOutcomeKind::Committed);
    assert_eq!(*outcome.backup().file_sha256(), recorded);
    let UuidEntityMigrationResume::AlreadyCompleted(outcome) =
        RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()).unwrap()
    else {
        panic!("source5 must be verified")
    };
    assert_eq!(
        outcome.kind(),
        UuidEntityMigrationOutcomeKind::VerifiedCompleted
    );
    let store = RedbStore::open_with_uuid_entities(&source).unwrap();
    let key = PersistentEntityKey::new(
        scope().world(),
        scope().dimension(),
        EntityUuid::from_bytes([9; 16]),
    );
    let record = PersistentEntityRecord::new(
        ChunkPos::new(-2, 3),
        EntityRecord::new(SchemaVersion::new(27), vec![1, 0, 255]).unwrap(),
    )
    .unwrap();
    runtime()
        .block_on(store.save_persistent_entity(PersistentEntityWrite::new(
            key,
            identity(),
            record.clone(),
        )))
        .unwrap();
    drop(store);
    let store = RedbStore::open_with_uuid_entities(&source).unwrap();
    assert_eq!(
        runtime()
            .block_on(store.load_persistent_entity(key, identity()))
            .unwrap(),
        Some(record)
    );
    assert_eq!(
        runtime()
            .block_on(store.load_generation_identity(scope()))
            .unwrap(),
        Some(identity())
    );
    drop(store);
    assert!(matches!(
        RedbStore::resume_uuid_entity_migration(&source, &checkpoint, &backup, limits()),
        Err(UuidEntityMigrationError::MigrationDeltaMismatch)
    ));
}

#[test]
fn public_prepared_backup_typestates_refuse_unrecorded_evidence_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("world");
    let backup = dir.path().join("backup");
    drop(RedbStore::open_with_generation_metadata(&source).unwrap());
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
    let backed = RedbStore::prepare_uuid_entity_migration(&source, limits())
        .unwrap()
        .verify_existing_backup(&backup, remembered)
        .unwrap();
    assert_eq!(backed.backup().file_sha256(), &remembered);
    assert_eq!(
        backed
            .report()
            .records(UuidEntityMigrationTable::GenerationIdentities),
        0
    );
}

#[test]
fn format3_first_requires_its_separate_unchanged_generation_migration() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("world");
    drop(RedbStore::open(&source).unwrap());
    assert!(matches!(
        RedbStore::prepare_uuid_entity_migration(&source, limits()),
        Err(UuidEntityMigrationError::WrongFormat(Some(3)))
    ));
    let declaration = LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero([scope()]).unwrap();
    let GenerationMigrationPreparation::Required(prepared) =
        RedbStore::prepare_generation_migration(&source, declaration).unwrap()
    else {
        panic!("separate3→4 required")
    };
    prepared
        .create_backup(dir.path().join("before4"))
        .unwrap()
        .migrate()
        .unwrap();
    let outcome = RedbStore::prepare_uuid_entity_migration(&source, limits())
        .unwrap()
        .create_backup(dir.path().join("before5"))
        .unwrap()
        .write_checkpoint(dir.path().join("checkpoint"))
        .unwrap()
        .migrate()
        .unwrap();
    assert_eq!(
        outcome
            .report()
            .records(UuidEntityMigrationTable::GenerationIdentities),
        1
    );
    drop(RedbStore::open_with_uuid_entities(&source).unwrap());
}
