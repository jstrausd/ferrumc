use ferrumc_core::{DimensionId, WorldId};
use ferrumc_storage::{
    GenerationKey, GenerationMigrationError, LegacyGenerationDeclaration, MAX_GENERATION_IDENTITIES,
};
#[cfg(unix)]
use ferrumc_storage::{GenerationMigrationPreparation, RedbStore};

fn scope(id: u32) -> GenerationKey {
    GenerationKey::new(WorldId::new(id), DimensionId::new(0))
}

#[test]
fn declaration_is_explicit_unique_and_bounded() {
    assert!(matches!(
        LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero([]),
        Err(GenerationMigrationError::EmptyDeclaration)
    ));
    assert!(matches!(
        LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero([scope(1), scope(1)]),
        Err(GenerationMigrationError::DuplicateScope { .. })
    ));
    let declaration = LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero(
        (0..MAX_GENERATION_IDENTITIES as u32).map(scope),
    )
    .unwrap();
    assert_eq!(declaration.scopes().len(), MAX_GENERATION_IDENTITIES);
    assert_eq!(declaration.identity().seed().value(), 0);
    assert!(matches!(
        LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero(
            (0..=MAX_GENERATION_IDENTITIES as u32).map(scope)
        ),
        Err(GenerationMigrationError::TooManyScopes)
    ));
}

#[test]
#[cfg(unix)]
fn public_prepare_backup_migrate_retry_contract() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world.redb");
    let backup = dir.path().join("before-v4.redb");
    drop(RedbStore::open(&path).unwrap());
    let declaration = LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero([scope(1)]).unwrap();
    let GenerationMigrationPreparation::Required(prepared) =
        RedbStore::prepare_generation_migration(&path, declaration.clone()).unwrap()
    else {
        panic!("format 3 needs migration");
    };
    assert_eq!(prepared.report().source_format(), 3);
    let backed_up = prepared.create_backup(&backup).unwrap();
    let hash = *backed_up.backup().file_sha256();
    let outcome = backed_up.migrate().unwrap();
    assert_eq!(outcome.backup().file_sha256(), &hash);
    drop(RedbStore::open_with_generation_metadata(&path).unwrap());
    drop(RedbStore::open(&backup).unwrap());
    assert!(matches!(
        RedbStore::prepare_generation_migration(&path, declaration).unwrap(),
        GenerationMigrationPreparation::AlreadyCompatible(_)
    ));
}
