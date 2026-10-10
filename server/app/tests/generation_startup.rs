//! Startup and explicit offline generation migration contracts.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use clap::Parser;
use redb::{Database, ReadableTable, TableDefinition, TableHandle};

use ferrumc_app::{AppConfig, Cli};
use ferrumc_core::{DimensionId, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::{ChunkKey, ChunkRecord, GenerationKey, RedbStore, SchemaVersion, WorldStore};
use ferrumc_world::{FlatWorldGenerator, GenerationIdentity, WorldSeed};

fn key(world: u32) -> GenerationKey {
    GenerationKey::new(WorldId::new(world), DimensionId::new(0))
}

fn config(dir: &Path, seed: Option<i64>) -> AppConfig {
    let text = format!(
        "bind = \"127.0.0.1:0\"\n{}",
        seed.map_or_else(String::new, |seed| format!("[world]\nseed = {seed}\n"))
    );
    AppConfig::from_toml_str(&text)
        .expect("configuration")
        .with_world_dir(Some(dir.to_path_buf()))
        .expect("world directory")
}

/// Compares every table/catalog/record byte, independent of redb file housekeeping.
type LogicalSnapshot = BTreeMap<String, Vec<(Vec<u8>, Vec<u8>)>>;

fn snapshot(path: &Path) -> LogicalSnapshot {
    let db = Database::open(path).expect("snapshot backend");
    let txn = db.begin_read().expect("snapshot read");
    txn.list_tables()
        .expect("catalog")
        .map(|handle| {
            let name = handle.name().to_owned();
            let records = if name == "ferrumc:meta" {
                let table = txn
                    .open_table(TableDefinition::<&str, u64>::new(&name))
                    .expect("metadata");
                table
                    .iter()
                    .expect("metadata rows")
                    .map(|row| {
                        let (key, value) = row.expect("metadata row");
                        (
                            key.value().as_bytes().to_vec(),
                            value.value().to_be_bytes().to_vec(),
                        )
                    })
                    .collect()
            } else {
                let table = txn
                    .open_table(TableDefinition::<&[u8], &[u8]>::new(&name))
                    .expect("byte table");
                table
                    .iter()
                    .expect("byte rows")
                    .map(|row| {
                        let (key, value) = row.expect("byte row");
                        (key.value().to_vec(), value.value().to_vec())
                    })
                    .collect()
            };
            (name, records)
        })
        .collect()
}

fn migrate_args(source: &Path, backup: Option<&Path>) -> Vec<String> {
    let mut args = vec![
        "ferrumc".to_owned(),
        "migrate-generation".to_owned(),
        "--source".to_owned(),
        source.display().to_string(),
        "--scope".to_owned(),
        "0:0".to_owned(),
        "--declare-legacy-flat-v1-seed-zero".to_owned(),
    ];
    if let Some(backup) = backup {
        args.extend(["--backup".to_owned(), backup.display().to_string()]);
    } else {
        args.push("--prepare-only".to_owned());
    }
    args
}

#[tokio::test]
async fn startup_seed_mismatch_preserves_complete_logical_snapshot() {
    let dir = tempfile::tempdir().expect("world");
    let path = dir.path().join("world.redb");
    let store = RedbStore::open_with_generation_metadata(&path).expect("format4");
    let identity = GenerationIdentity::flat_overworld(WorldSeed::new(123));
    store
        .ensure_generation_identity(key(0), identity)
        .await
        .expect("identity");
    let chunk = FlatWorldGenerator::with_seed(identity.seed()).generate(ChunkPos::new(0, 0));
    store
        .save_chunk(
            ChunkKey::new(key(0).world(), key(0).dimension(), chunk.pos()),
            ChunkRecord::new(SchemaVersion::new(1), chunk),
        )
        .await
        .expect("original chunk");
    drop(store);
    let before = snapshot(&path);
    let mismatch = config(dir.path(), Some(124));
    let error = ferrumc_app::run(&mismatch)
        .await
        .expect_err("explicit seed mismatch");
    assert!(error
        .to_string()
        .contains("incompatible generation identity"));
    assert_eq!(snapshot(&path), before);
}

#[tokio::test]
async fn foreign_scope_data_refuses_default_scope_without_new_metadata_or_import() {
    let dir = tempfile::tempdir().expect("world");
    let path = dir.path().join("world.redb");
    let store = RedbStore::open_with_generation_metadata(&path).expect("format4");
    let identity = GenerationIdentity::flat_overworld(WorldSeed::new(0));
    store
        .ensure_generation_identity(key(7), identity)
        .await
        .expect("foreign scope");
    let chunk = FlatWorldGenerator::new().generate(ChunkPos::new(0, 0));
    store
        .save_chunk(
            ChunkKey::new(key(7).world(), key(7).dimension(), chunk.pos()),
            ChunkRecord::new(SchemaVersion::new(1), chunk),
        )
        .await
        .expect("foreign chunk");
    drop(store);
    let before = snapshot(&path);
    ferrumc_app::run(&config(dir.path(), None))
        .await
        .expect_err("scope0 cannot invent foreign baseline");
    assert_eq!(snapshot(&path), before);
}

#[tokio::test]
async fn unsupported_version_generator_and_revision_refuse_without_mutation() {
    for (offset, bytes) in [
        (10, vec![0, 9]),
        (12, vec![255, 255]),
        (14, vec![0, 0, 0, 2]),
        (0, vec![0, 0, 0, 0]),
    ] {
        let dir = tempfile::tempdir().expect("world");
        let path = dir.path().join("world.redb");
        let store = RedbStore::open_with_generation_metadata(&path).expect("format4");
        let identity = GenerationIdentity::flat_overworld(WorldSeed::new(0));
        store
            .ensure_generation_identity(key(0), identity)
            .await
            .expect("identity");
        drop(store);
        let db = Database::open(&path).expect("raw backend");
        let txn = db.begin_write().expect("raw write");
        {
            let mut table = txn
                .open_table(TableDefinition::<&[u8], &[u8]>::new("ferrumc:generation"))
                .expect("descriptor table");
            let mut value = identity.encode();
            value[offset..offset + bytes.len()].copy_from_slice(&bytes);
            table
                .insert([0_u8; 8].as_slice(), value.as_slice())
                .expect("unsupported descriptor");
        }
        txn.commit().expect("raw fixture commit");
        drop(db);
        let before = snapshot(&path);
        ferrumc_app::run(&config(dir.path(), None))
            .await
            .expect_err("unsupported descriptor");
        assert_eq!(snapshot(&path), before);
    }
}

#[tokio::test]
async fn format3_refuses_normal_start_then_offline_backup_migration_and_restart_work() {
    let dir = tempfile::tempdir().expect("world");
    let path = dir.path().join("world.redb");
    let backup = dir.path().join("explicit-format3-backup.redb");
    let legacy = RedbStore::open(&path).expect("legacy fixture");
    let pos = ChunkPos::new(0, 0);
    let chunk = FlatWorldGenerator::new().generate(pos);
    legacy
        .save_chunk(
            ChunkKey::new(WorldId::new(0), DimensionId::new(0), pos),
            ChunkRecord::new(SchemaVersion::new(1), chunk),
        )
        .await
        .expect("legacy chunk");
    drop(legacy);
    let before = snapshot(&path);
    let error = ferrumc_app::run(&config(dir.path(), None))
        .await
        .expect_err("legacy normal startup must refuse");
    assert!(error.to_string().contains("migrate-generation"));
    assert_eq!(snapshot(&path), before);
    let report = Cli::try_parse_from(migrate_args(&path, None))
        .expect("prepare CLI")
        .run_offline()
        .await
        .expect("preflight")
        .expect("offline report");
    assert!(report.contains("prepare-only") && report.contains("Chunks=1"));
    assert_eq!(snapshot(&path), before);
    let cli = Cli::try_parse_from(migrate_args(&path, Some(&backup))).expect("migration CLI");
    let report = cli.run_offline().await.expect("migration").expect("report");
    assert!(report.contains("migration committed: format4") && report.contains("file_sha256="));
    assert_eq!(
        snapshot(&backup),
        before,
        "backup preserves every original record byte"
    );
    let migrated = snapshot(&path);
    let retry = cli
        .run_offline()
        .await
        .expect("idempotent retry")
        .expect("report");
    assert!(retry.contains("already compatible"));
    assert_eq!(snapshot(&path), migrated);
    let server = ferrumc_app::run(&config(dir.path(), None))
        .await
        .expect("format4 starts");
    server.shutdown().await.expect("final verified drain");
}

#[tokio::test]
async fn offline_active_source_and_existing_backup_fail_without_record_changes() {
    let dir = tempfile::tempdir().expect("world");
    let path = dir.path().join("world.redb");
    let backup = dir.path().join("backup.redb");
    let legacy = RedbStore::open(&path).expect("legacy fixture");
    let cli = Cli::try_parse_from(migrate_args(&path, Some(&backup))).expect("CLI");
    cli.run_offline()
        .await
        .expect_err("active source lock refuses migration");
    assert!(!backup.exists());
    drop(legacy);
    let before = snapshot(&path);
    std::fs::write(&backup, b"operator file").expect("existing destination");
    cli.run_offline()
        .await
        .expect_err("backup overwrite refused");
    assert_eq!(
        std::fs::read(&backup).expect("original backup path"),
        b"operator file"
    );
    assert_eq!(snapshot(&path), before);
}

#[test]
fn offline_parser_rejects_missing_declaration_bad_scopes_hashes_and_conflicts() {
    let basic = [
        "ferrumc",
        "migrate-generation",
        "--source",
        "world.redb",
        "--prepare-only",
    ];
    assert!(Cli::try_parse_from(basic).is_err());
    for scope in [
        "",
        "0",
        "-1:0",
        "0:-1",
        "4294967296:0",
        "0:4294967296",
        "0:0:0",
        "x:0",
    ] {
        let mut args = basic.to_vec();
        args.extend(["--scope", scope, "--declare-legacy-flat-v1-seed-zero"]);
        assert!(Cli::try_parse_from(args).is_err(), "scope {scope:?}");
    }
    for hash in [
        "",
        "00",
        "gg",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "éééééééééééééééééééééééééééééééé",
    ] {
        assert!(Cli::try_parse_from([
            "ferrumc",
            "migrate-generation",
            "--source",
            "world.redb",
            "--backup",
            "backup.redb",
            "--scope",
            "0:0",
            "--declare-legacy-flat-v1-seed-zero",
            "--backup-sha256",
            hash
        ])
        .is_err());
    }
    assert!(Cli::try_parse_from([
        "ferrumc",
        "migrate-generation",
        "--source",
        "world.redb",
        "--backup",
        "backup.redb",
        "--prepare-only",
        "--scope",
        "0:0",
        "--declare-legacy-flat-v1-seed-zero"
    ])
    .is_err());
}

#[tokio::test]
async fn offline_reuse_requires_remembered_hash_and_preserves_backup() {
    use ferrumc_storage::{GenerationMigrationPreparation, LegacyGenerationDeclaration};
    let dir = tempfile::tempdir().expect("world");
    let path = dir.path().join("world.redb");
    let backup = dir.path().join("backup.redb");
    drop(RedbStore::open(&path).expect("legacy fixture"));
    let declaration =
        LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero([key(0)]).expect("declaration");
    let GenerationMigrationPreparation::Required(prepared) =
        RedbStore::prepare_generation_migration(&path, declaration).expect("prepare")
    else {
        panic!("legacy migration required");
    };
    let backed_up = prepared
        .create_backup(&backup)
        .expect("exclusive validated backup");
    let hash = backed_up.backup().file_sha256().iter().fold(
        String::with_capacity(64),
        |mut hash, byte| {
            write!(hash, "{byte:02x}").expect("format hash");
            hash
        },
    );
    drop(backed_up);
    let before = snapshot(&path);
    let backup_before = std::fs::read(&backup).expect("closed backup");
    let mut args = migrate_args(&path, Some(&backup));
    args.extend(["--backup-sha256".to_owned(), "0".repeat(64)]);
    Cli::try_parse_from(&args)
        .expect("wrong hash CLI")
        .run_offline()
        .await
        .expect_err("wrong remembered hash");
    assert_eq!(snapshot(&path), before);
    assert_eq!(
        std::fs::read(&backup).expect("unchanged backup"),
        backup_before
    );
    args.pop();
    args.push(hash);
    let report = Cli::try_parse_from(args)
        .expect("remembered hash CLI")
        .run_offline()
        .await
        .expect("verified reuse migration")
        .expect("report");
    assert!(report.contains("migration committed: format4"));
    assert_eq!(
        std::fs::read(&backup).expect("preserved backup"),
        backup_before
    );
}

#[test]
fn binary_offline_dispatch_never_bootstraps_normal_config() {
    let dir = tempfile::tempdir().expect("offline directory");
    let source = dir.path().join("source.redb");
    let missing_config = dir.path().join("should-not-be-written.toml");
    drop(RedbStore::open(&source).expect("legacy fixture"));
    let before = snapshot(&source);
    let mut args = migrate_args(&source, None);
    args.remove(0);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ferrumc"))
        .current_dir(dir.path())
        .arg("--config")
        .arg(&missing_config)
        .args(args)
        .output()
        .expect("invoke actual offline binary");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("prepare-only"));
    assert!(!missing_config.exists());
    assert!(!dir.path().join("world").exists());
    assert_eq!(snapshot(&source), before);
}
