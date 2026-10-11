//! Actual databases, opaque legacy players and independently derived complete target rows.

use super::*;
use crate::{
    BlockMutationLogRecord, ChunkKey, ChunkOverlayRecord, EntityRecord, GenerationKey,
    JournalBatchId, MutationActor, MutationLogCause, PersistentEntityKey, PersistentEntityRecord,
    PersistentEntityStore, PersistentEntityWrite, PlayerRecord, PlayerStore, PluginStore,
    SchemaVersion, StorageKey, WorldStore,
};
use ferrumc_core::{DimensionId, EntityUuid, GameMode, PlayerId, PluginId, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_world::{BlockStateId, Chunk, GenerationIdentity, WorldSeed};
use redb::TableHandle;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::Ordering;

fn config() -> OwnershipConfig {
    crate::ownership::tests::config()
}
fn scope(world: u32, dimension: u32) -> GenerationKey {
    GenerationKey::new(WorldId::new(world), DimensionId::new(dimension))
}
fn identity(seed: i64) -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(seed))
}
fn players() -> [(PlayerId, GenerationKey, GenerationIdentity); 3] {
    [
        (
            PlayerId::from_uuid(Uuid::from_bytes([1; 16])),
            scope(1, 0),
            identity(11),
        ),
        (
            PlayerId::from_uuid(Uuid::from_bytes([2; 16])),
            scope(1, 7),
            identity(-99),
        ),
        (
            PlayerId::from_uuid(Uuid::from_bytes([3; 16])),
            scope(2, 7),
            identity(i64::MAX),
        ),
    ]
}
fn mapping_bytes(rows: &[(PlayerId, GenerationKey, GenerationIdentity)]) -> Vec<u8> {
    let mut bytes = vec![0; 64];
    bytes[..6].copy_from_slice(b"FCOM\0\x01");
    bytes[8..16].copy_from_slice(&(rows.len() as u64).to_be_bytes());
    for (player, scope, identity) in rows {
        bytes.extend_from_slice(player.as_uuid().as_bytes());
        bytes.extend_from_slice(&scope.encode());
        bytes.extend_from_slice(&identity.encode());
    }
    bytes
}
#[allow(clippy::too_many_lines)] // Keep the complete nonempty evidence fixture visible together.
fn fixture(source: &Path, mapping: &Path) {
    let store = RedbStore::open_with_uuid_entities(source).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        for (_, scope, identity) in players() {
            store
                .ensure_generation_identity(scope, identity)
                .await
                .unwrap();
        }
        for (i, (player, _, _)) in players().into_iter().enumerate() {
            let bytes = vec![b'F', b'C', b'P', b'S', (i as u8) % 2 + 1, 0, 255, i as u8];
            let mode = [GameMode::Survival, GameMode::Creative, GameMode::Adventure][i];
            store
                .save_player(
                    player,
                    PlayerRecord::new(SchemaVersion::new(90 + i as u32), mode, bytes).unwrap(),
                )
                .await
                .unwrap();
        }
        for (world, dimension, id, chunk, seed) in [
            (1, 0, 0x12, ChunkPos::new(2, -3), 11),
            (1, 0, 0x13, ChunkPos::new(2, -3), 11),
            (1, 7, 0x14, ChunkPos::new(-1, -2), -99),
            (2, 7, 0x12, ChunkPos::new(2, -3), i64::MAX),
        ] {
            let key = PersistentEntityKey::new(
                WorldId::new(world),
                DimensionId::new(dimension),
                EntityUuid::from_bytes([id; 16]),
            );
            let record = PersistentEntityRecord::new(
                chunk,
                EntityRecord::new(
                    SchemaVersion::new(700 + u32::from(id)),
                    vec![b'F', b'C', b'I', b'S', 0, id, 255],
                )
                .unwrap(),
            )
            .unwrap();
            store
                .save_persistent_entity(PersistentEntityWrite::new(key, identity(seed), record))
                .await
                .unwrap();
        }
        let historical = PersistentEntityKey::new(
            WorldId::new(1),
            DimensionId::new(0),
            EntityUuid::from_bytes([0x19; 16]),
        );
        store
            .save_persistent_entity(PersistentEntityWrite::new(
                historical,
                identity(11),
                PersistentEntityRecord::new(
                    ChunkPos::new(5, -2),
                    EntityRecord::new(SchemaVersion::new(1), vec![9]).unwrap(),
                )
                .unwrap(),
            ))
            .await
            .unwrap();
        store
            .delete_persistent_entity(historical, identity(11))
            .await
            .unwrap();
        let mut chunk = Chunk::new(ChunkPos::new(2, -3));
        chunk
            .set_block(BlockPos::new(33, 64, -47), BlockStateId::new(9))
            .unwrap();
        let key = ChunkKey::new(WorldId::new(1), DimensionId::new(0), chunk.pos());
        store
            .save_chunk_overlays(vec![(
                key,
                ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(92), chunk.pos(), &chunk, 0)
                    .unwrap(),
            )])
            .await
            .unwrap();
        store
            .put(
                &PluginId::new("migration-plugin"),
                StorageKey::new("opaque").unwrap(),
                vec![9, 0, 255],
            )
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
    });
    drop(store);
    fs::write(mapping, mapping_bytes(&players())).unwrap();
}

struct Fixture {
    dir: tempfile::TempDir,
    source: PathBuf,
    mapping: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.redb");
        let mapping = dir.path().join("mapping.bin");
        fixture(&source, &mapping);
        Self {
            dir,
            source,
            mapping,
        }
    }
    fn output(&self) -> OwnershipMigrationOutputPaths {
        OwnershipMigrationOutputPaths::new(
            self.dir.path().join("backup.redb"),
            self.dir.path().join("plan.bin"),
            self.dir.path().join("stage.redb"),
            self.dir.path().join("checkpoint.bin"),
            self.dir.path(),
        )
        .unwrap()
    }
    fn resume_paths(&self) -> OwnershipMigrationResumePaths {
        OwnershipMigrationResumePaths::new(&self.source, &self.mapping, self.output()).unwrap()
    }
    fn prepare(&self) -> PreparedOwnershipMigration {
        RedbStore::prepare_player_entity_ownership_migration(
            &self.source,
            &self.mapping,
            self.dir.path(),
            config(),
            OwnershipMigrationLimits::defaults(),
        )
        .unwrap()
    }
    fn seal(&self) -> BoundedOwnershipMigration {
        self.prepare().prepare_files(self.output()).unwrap()
    }
    fn resume(&self) -> OwnershipMigrationResume {
        RedbStore::resume_player_entity_ownership_migration(
            self.resume_paths(),
            config(),
            OwnershipMigrationLimits::defaults(),
        )
        .unwrap()
    }
}
type Snapshot = BTreeMap<String, BTreeMap<Vec<u8>, Vec<u8>>>;
fn snapshot(db: &Database) -> Snapshot {
    let txn = db.begin_read().unwrap();
    let mut out = BTreeMap::new();
    let meta = txn.open_table(META_TABLE).unwrap();
    out.insert(
        META_TABLE.name().to_owned(),
        meta.iter()
            .unwrap()
            .map(|row| {
                let (k, v) = row.unwrap();
                (
                    k.value().as_bytes().to_vec(),
                    v.value().to_be_bytes().to_vec(),
                )
            })
            .collect(),
    );
    for name in txn
        .list_tables()
        .unwrap()
        .filter(|n| n.name() != META_TABLE.name())
    {
        let table = txn
            .open_table(TableDefinition::<&[u8], &[u8]>::new(name.name()))
            .unwrap();
        out.insert(
            name.name().to_owned(),
            table
                .iter()
                .unwrap()
                .map(|row| {
                    let (k, v) = row.unwrap();
                    (k.value().to_vec(), v.value().to_vec())
                })
                .collect(),
        );
    }
    out
}
fn snapshot_file(path: &Path) -> Snapshot {
    snapshot(&Database::open(path).unwrap())
}
fn logical_hash(raw: &Snapshot) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"FerrumC ownership storage logical snapshot v1\0");
    for name in std::iter::once("ferrumc:meta").chain([
        "ferrumc:generation",
        "ferrumc:chunk",
        "ferrumc:chunk_overlay",
        "ferrumc:entity",
        "ferrumc:entity_locator",
        "ferrumc:entity_chunk",
        "ferrumc:entity_chunk_epoch",
        "ferrumc:player",
        "ferrumc:plugin",
        "ferrumc:mutation_log",
        "ferrumc:mutation_batch",
        "ferrumc:player_ownership",
        "ferrumc:entity_ownership",
        "ferrumc:ownership_receipt",
    ]) {
        let rows = &raw[name];
        h.update(b"table\0");
        h.update((name.len() as u64).to_be_bytes());
        h.update(name);
        for (key, value) in rows {
            h.update(b"row\0");
            h.update((key.len() as u64).to_be_bytes());
            h.update(key);
            h.update((value.len() as u64).to_be_bytes());
            h.update(value);
        }
        h.update(b"end\0");
        h.update((rows.len() as u64).to_be_bytes());
    }
    h.finalize().into()
}
#[allow(clippy::too_many_lines)] // Independent complete raw target algebra, deliberately not producer encoding helpers.
fn independent_target(mut original: Snapshot, bounded: &BoundedOwnershipMigration) -> Snapshot {
    let n = 7u64;
    let c = config().capacity();
    let meta = original.get_mut("ferrumc:meta").unwrap();
    for (key, value) in [
        ("format_version", 6),
        ("ownership_receipt_count", n),
        ("ownership_revision_count", n),
        ("ownership_request_limit", c.max_request_bytes() as u64),
        ("ownership_receipt_limit", c.max_receipts()),
        ("ownership_revision_limit", c.max_revision_rows()),
    ] {
        meta.insert(key.as_bytes().to_vec(), value.to_be_bytes().to_vec());
    }
    original.insert("ferrumc:player_ownership".into(), BTreeMap::new());
    original.insert("ferrumc:entity_ownership".into(), BTreeMap::new());
    original.insert("ferrumc:ownership_receipt".into(), BTreeMap::new());
    let mut reader = plan::Reader::open(&bounded.plan, bounded.prepared.limits).unwrap();
    while let Some(row) = reader.next().unwrap() {
        let payload = &original[if row.kind == 2 {
            "ferrumc:player"
        } else {
            "ferrumc:entity"
        }][if row.kind == 2 {
            &row.key[..16]
        } else {
            &row.key[..]
        }];
        let mut canonical = b"FerrumC ownership request v1\0".to_vec();
        canonical.push(row.kind);
        canonical.extend_from_slice(&row.id.to_bytes());
        canonical.extend_from_slice(&row.scope.encode());
        canonical.extend_from_slice(&row.identity.encode());
        canonical.extend_from_slice(if row.kind == 2 {
            &row.key[..16]
        } else {
            &[0; 16]
        });
        canonical.extend_from_slice(if row.kind == 3 {
            &row.key[8..]
        } else {
            &[0; 16]
        });
        canonical.extend_from_slice(&[0; 18]);
        if row.kind == 3 {
            canonical.extend_from_slice(&[0; 9]);
        }
        canonical.push(1);
        canonical.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        canonical.extend_from_slice(payload);
        if row.kind == 2 {
            canonical.extend_from_slice(&[0; 9]);
        }
        assert_eq!(row.query, Sha256::digest(&canonical).as_slice());
        let mut receipt = vec![0; 256];
        receipt[..6].copy_from_slice(b"FCTR\0\x01");
        receipt[6] = row.kind;
        receipt[7..23].copy_from_slice(&row.id.to_bytes());
        receipt[23..55].copy_from_slice(&row.query);
        receipt[55..63].copy_from_slice(&row.scope.encode());
        receipt[63..90].copy_from_slice(&row.identity.encode());
        if row.kind == 2 {
            receipt[90..106].copy_from_slice(&row.key[..16]);
            receipt[131] = 1;
            receipt[139] = 1;
            let mut header = b"FCOP\0\x01\x01".to_vec();
            header.extend_from_slice(&row.scope.encode());
            header.extend_from_slice(&1u64.to_be_bytes());
            header.extend_from_slice(&row.id.to_bytes());
            original
                .get_mut("ferrumc:player_ownership")
                .unwrap()
                .insert(row.key[..16].to_vec(), header);
        } else {
            let chunk: [u8; 8] = payload[6..14].try_into().unwrap();
            let mut chunkkey = row.scope.encode().to_vec();
            chunkkey.extend_from_slice(&chunk);
            let epochs = original.get_mut("ferrumc:entity_chunk_epoch").unwrap();
            let before = u64::from_be_bytes(epochs[&chunkkey].as_slice().try_into().unwrap());
            let after = before + 1;
            epochs.insert(chunkkey.clone(), after.to_be_bytes().to_vec());
            receipt[106..122].copy_from_slice(&row.key[8..]);
            receipt[149] = 1;
            receipt[157] = 1;
            receipt[158] = 1;
            receipt[159..175].copy_from_slice(&chunkkey);
            receipt[175..183].copy_from_slice(&before.to_be_bytes());
            receipt[183..191].copy_from_slice(&after.to_be_bytes());
            receipt[232] = 1;
            receipt[233..241].copy_from_slice(&chunk);
            let mut header = b"FCOR\0\x01\x01".to_vec();
            header.extend_from_slice(&row.scope.encode()[4..]);
            header.extend_from_slice(&1u64.to_be_bytes());
            header.extend_from_slice(&row.id.to_bytes());
            let mut key = row.key[..4].to_vec();
            key.extend_from_slice(&row.key[8..]);
            original
                .get_mut("ferrumc:entity_ownership")
                .unwrap()
                .insert(key, header);
        }
        original
            .get_mut("ferrumc:ownership_receipt")
            .unwrap()
            .insert(row.id.to_bytes().to_vec(), receipt);
    }
    original
}

#[test]
fn actual_stage_cas_atomic_import_reopen_and_exact_readonly_resume() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    let expected = independent_target(before, &sealed);
    let expectedhash = logical_hash(&expected);
    assert_eq!(sealed.record.target, expectedhash);
    let immutable: [(PathBuf, Vec<u8>); 4] = std::array::from_fn(|i| {
        let path = sealed.record.evidence[i + 1].path.clone();
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    });
    let outcome = sealed.migrate().unwrap();
    assert_eq!(outcome.target_sha256(), expectedhash);
    assert_eq!(outcome.player_count(), 3);
    assert_eq!(outcome.entity_count(), 4);
    assert_eq!(snapshot_file(&f.source), expected);
    match f.resume() {
        OwnershipMigrationResume::AlreadyCommitted(result) => assert_eq!(result, outcome),
        other @ OwnershipMigrationResume::ReadyToCommit(_) => panic!("{other:?}"),
    }
    for (path, bytes) in immutable {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    let live = RedbStore::open_with_player_entity_ownership(&f.source, config()).unwrap();
    assert_eq!(snapshot(&live.db), expected);
}

#[test]
fn all_actual_source_transaction_cutpoints_abort_or_resolve_uncertainty() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    let expected = independent_target(before.clone(), &sealed);
    let mut points = Vec::new();
    let error = sealed
        .migrate_observed(&mut |point| {
            if matches!(
                point,
                Cutpoint::BeforeRow
                    | Cutpoint::AfterRow
                    | Cutpoint::BeforeMeta
                    | Cutpoint::AfterMeta
                    | Cutpoint::BeforeMarker
                    | Cutpoint::AfterMarker
                    | Cutpoint::BeforeTargetScan
                    | Cutpoint::AfterTargetScan
                    | Cutpoint::BeforeCommit
            ) {
                points.push(point);
            }
            if point == Cutpoint::BeforeCommit {
                Err(Error::SourceDrift)
            } else {
                Ok(())
            }
        })
        .unwrap_err();
    assert!(matches!(error, Error::SourceDrift));
    assert_eq!(snapshot_file(&f.source), before);
    for cut in 0..points.len() {
        let OwnershipMigrationResume::ReadyToCommit(sealed) = f.resume() else {
            panic!("precommit target")
        };
        let mut ordinal = 0;
        let error = sealed
            .migrate_observed(&mut |point| {
                if matches!(
                    point,
                    Cutpoint::BeforeRow
                        | Cutpoint::AfterRow
                        | Cutpoint::BeforeMeta
                        | Cutpoint::AfterMeta
                        | Cutpoint::BeforeMarker
                        | Cutpoint::AfterMarker
                        | Cutpoint::BeforeTargetScan
                        | Cutpoint::AfterTargetScan
                        | Cutpoint::BeforeCommit
                ) {
                    let fail = ordinal == cut;
                    ordinal += 1;
                    if fail {
                        return Err(Error::SourceDrift);
                    }
                }
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(error, Error::SourceDrift));
        assert_eq!(snapshot_file(&f.source), before);
    }
    let OwnershipMigrationResume::ReadyToCommit(sealed) = f.resume() else {
        panic!("ready")
    };
    assert!(matches!(
        sealed.migrate_observed(&mut |point| if point == Cutpoint::AfterCommit {
            Err(Error::SourceDrift)
        } else {
            Ok(())
        }),
        Err(Error::CommitUncertain { .. })
    ));
    assert_eq!(snapshot_file(&f.source), expected);
    assert!(matches!(
        f.resume(),
        OwnershipMigrationResume::AlreadyCommitted(_)
    ));
}

#[test]
fn actual_backend_sync_failure_and_reopen_cutpoints_are_uncertain() {
    for cut in [Cutpoint::BeforeReopen, Cutpoint::AfterReopen] {
        let f = Fixture::new();
        let before = snapshot_file(&f.source);
        let sealed = f.seal();
        let expected = independent_target(before, &sealed);
        assert!(matches!(
            sealed.migrate_observed(&mut |point| if point == cut {
                Err(Error::SourceDrift)
            } else {
                Ok(())
            }),
            Err(Error::CommitUncertain { .. })
        ));
        assert_eq!(snapshot_file(&f.source), expected);
        assert!(matches!(
            f.resume(),
            OwnershipMigrationResume::AlreadyCommitted(_)
        ));
    }
    let f = Fixture::new();
    let sealed = f.seal();
    let flag = Arc::clone(&sealed.prepared.source.fail_sync);
    assert!(matches!(
        sealed.migrate_observed(&mut |point| {
            if point == Cutpoint::BeforeCommit {
                flag.store(true, Ordering::Relaxed);
            }
            Ok(())
        }),
        Err(Error::CommitUncertain { .. })
    ));
    // The backend injection is at the real commit; either exact source/target or
    // explicit recovery-required refusal is permitted, never an automatic retry.
    match RedbStore::resume_player_entity_ownership_migration(
        f.resume_paths(),
        config(),
        OwnershipMigrationLimits::defaults(),
    ) {
        Ok(
            OwnershipMigrationResume::ReadyToCommit(_)
            | OwnershipMigrationResume::AlreadyCommitted(_),
        )
        | Err(Error::RecoveryRequired) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn all_copy_cutpoints_clean_owned_copies_and_preserve_immutable_evidence() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    drop(sealed);
    for cut in [
        Cutpoint::BeforeCopyCreate,
        Cutpoint::AfterCopyCreate,
        Cutpoint::BeforeCopyWrite,
        Cutpoint::AfterCopyWrite,
        Cutpoint::BeforeCopyOpen,
        Cutpoint::AfterCopyOpen,
        Cutpoint::BeforeCopyScan,
        Cutpoint::AfterCopyScan,
        Cutpoint::BeforeCopyClose,
        Cutpoint::AfterCopyClose,
        Cutpoint::BeforeCopyCleanup,
        Cutpoint::AfterCopyCleanup,
    ] {
        let OwnershipMigrationResume::ReadyToCommit(sealed) = f.resume() else {
            panic!("ready")
        };
        let hashes: Vec<_> = sealed.record.evidence[1..]
            .iter()
            .map(|e| (e.path.clone(), fs::read(&e.path).unwrap()))
            .collect();
        let mut once = false;
        assert!(sealed
            .migrate_observed(&mut |point| {
                if point == cut && !once {
                    once = true;
                    Err(Error::SourceDrift)
                } else {
                    Ok(())
                }
            })
            .is_err());
        assert!(once);
        assert_eq!(snapshot_file(&f.source), before);
        for (path, bytes) in hashes {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert!(fs::read_dir(f.dir.path()).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".ferrumc-ownership")));
    }
}

#[test]
fn mapping_malformed_order_missing_unused_and_generation_mismatch_refuse_without_source_changes() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let mut cases = Vec::new();
    let mut truncated = mapping_bytes(&players());
    truncated.pop();
    cases.push(truncated);
    let mut reserved = mapping_bytes(&players());
    reserved[6] = 1;
    cases.push(reserved);
    let mut trailing = mapping_bytes(&players());
    trailing.push(0);
    cases.push(trailing);
    let mut duplicate = players().to_vec();
    duplicate[1] = duplicate[0];
    cases.push(mapping_bytes(&duplicate));
    let mut unsorted = players();
    unsorted.swap(0, 1);
    cases.push(mapping_bytes(&unsorted));
    cases.push(mapping_bytes(&players()[..2]));
    let mut foreign = players();
    foreign[2].0 = PlayerId::from_uuid(Uuid::from_bytes([4; 16]));
    cases.push(mapping_bytes(&foreign));
    let mut wrong = players();
    wrong[0].2 = identity(777);
    cases.push(mapping_bytes(&wrong));
    for bytes in cases {
        fs::write(&f.mapping, bytes).unwrap();
        assert!(RedbStore::prepare_player_entity_ownership_migration(
            &f.source,
            &f.mapping,
            f.dir.path(),
            config(),
            OwnershipMigrationLimits::defaults()
        )
        .is_err());
        assert_eq!(snapshot_file(&f.source), before);
    }
}

#[test]
fn positive_limits_exact_source_and_target_capacity_refuse_one_below() {
    let f = Fixture::new();
    let sealed = f.seal();
    let rows = sealed.record.counts[2];
    let bytes = sealed.record.counts[3];
    drop(sealed);
    for (r, b) in [(rows - 1, bytes), (rows, bytes - 1)] {
        let limits = OwnershipMigrationLimits::new(
            NonZeroU64::new(r).unwrap(),
            NonZeroU64::new(b).unwrap(),
            NonZeroU64::new(3).unwrap(),
            NonZeroU64::new(7).unwrap(),
            NonZeroU64::new(64 * 1024 * 1024).unwrap(),
            NonZeroU64::new(2 * 1024 * 1024 * 1024).unwrap(),
            NonZeroUsize::new(32768).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            RedbStore::prepare_player_entity_ownership_migration(
                &f.source,
                &f.mapping,
                f.dir.path(),
                config(),
                limits
            ),
            Err(Error::ScanCapacity { .. })
        ));
    }
    let limits = OwnershipMigrationLimits::new(
        NonZeroU64::new(rows).unwrap(),
        NonZeroU64::new(bytes).unwrap(),
        NonZeroU64::new(3).unwrap(),
        NonZeroU64::new(7).unwrap(),
        NonZeroU64::new(64 * 1024 * 1024).unwrap(),
        NonZeroU64::new(2 * 1024 * 1024 * 1024).unwrap(),
        NonZeroUsize::new(32768).unwrap(),
    )
    .unwrap();
    assert!(RedbStore::prepare_player_entity_ownership_migration(
        &f.source,
        &f.mapping,
        f.dir.path(),
        config(),
        limits
    )
    .is_ok());
}

#[test]
fn checkpoint_and_plan_malformed_fields_refuse_with_recomputed_checksums() {
    let f = Fixture::new();
    let sealed = f.seal();
    let record = sealed.record.clone();
    let raw = fs::read(&sealed.checkpoint.path).unwrap();
    let path = sealed.checkpoint.path.clone();
    drop(sealed);
    for offset in [0, 4, 6, 7, 14, 304, 336, 368, 370, 376, 377, 378] {
        let mut changed = raw.clone();
        changed[offset] ^= 0xff;
        let end = changed.len() - 32;
        let checksum: [u8; 32] = Sha256::digest(&changed[..end]).into();
        changed[end..].copy_from_slice(&checksum);
        fs::write(&path, &changed).unwrap();
        assert!(
            RedbStore::resume_player_entity_ownership_migration(
                f.resume_paths(),
                config(),
                OwnershipMigrationLimits::defaults()
            )
            .is_err(),
            "offset {offset}"
        );
    }
    for len in [0, 100, 375, raw.len() - 1] {
        fs::write(&path, &raw[..len]).unwrap();
        assert!(RedbStore::resume_player_entity_ownership_migration(
            f.resume_paths(),
            config(),
            OwnershipMigrationLimits::defaults()
        )
        .is_err());
    }
    fs::write(&path, &raw).unwrap();
    let mut planbytes = fs::read(&record.evidence[3].path).unwrap();
    let planpath = record.evidence[3].path.clone();
    let originalplan = planbytes.clone();
    for offset in [
        0,
        4,
        6,
        8,
        112,
        128,
        128 + 17,
        128 + 60,
        128 + 66,
        128 + 140,
    ] {
        planbytes = originalplan.clone();
        planbytes[offset] ^= 0xff;
        fs::write(&planpath, &planbytes).unwrap();
        let mut bound = record.clone();
        bound.evidence[3].physical = (planbytes.len() as u64, Sha256::digest(&planbytes).into());
        bound.plan = bound.evidence[3].physical.1;
        fs::write(&path, bound.encode().unwrap()).unwrap();
        assert!(
            RedbStore::resume_player_entity_ownership_migration(
                f.resume_paths(),
                config(),
                OwnershipMigrationLimits::defaults()
            )
            .is_err(),
            "plan offset {offset}"
        );
    }
    fs::write(&planpath, &originalplan).unwrap();
    fs::write(&path, &raw).unwrap();
    assert!(matches!(
        f.resume(),
        OwnershipMigrationResume::ReadyToCommit(_)
    ));
}

#[test]
fn all_evidence_replacements_preserve_foreign_paths_and_refuse_resume() {
    let f = Fixture::new();
    let sealed = f.seal();
    let paths: Vec<_> = sealed
        .record
        .evidence
        .iter()
        .map(|e| e.path.clone())
        .chain(std::iter::once(sealed.checkpoint.path.clone()))
        .collect();
    drop(sealed);
    for path in paths {
        let own = path.with_extension("kept");
        fs::rename(&path, &own).unwrap();
        let foreign = b"foreign path, must survive";
        fs::write(&path, foreign).unwrap();
        assert!(RedbStore::resume_player_entity_ownership_migration(
            f.resume_paths(),
            config(),
            OwnershipMigrationLimits::defaults()
        )
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), foreign);
        fs::remove_file(&path).unwrap();
        fs::rename(&own, &path).unwrap();
    }
    assert!(matches!(
        f.resume(),
        OwnershipMigrationResume::ReadyToCommit(_)
    ));
}

#[test]
fn checked_epoch_overflow_refuses_before_stage_and_historical_max_has_zero_delta() {
    let f = Fixture::new();
    let epochkey = crate::codec::chunk_key_bytes(ChunkKey::new(
        WorldId::new(1),
        DimensionId::new(0),
        ChunkPos::new(2, -3),
    ));
    {
        let db = Database::open(&f.source).unwrap();
        let txn = db.begin_write().unwrap();
        txn.open_table(uuid_entities::EPOCH_TABLE)
            .unwrap()
            .insert(epochkey.as_slice(), (u64::MAX - 1).to_be_bytes().as_slice())
            .unwrap();
        txn.commit().unwrap();
    }
    let before = snapshot_file(&f.source);
    assert!(matches!(
        RedbStore::prepare_player_entity_ownership_migration(
            &f.source,
            &f.mapping,
            f.dir.path(),
            config(),
            OwnershipMigrationLimits::defaults()
        ),
        Err(Error::EpochExhausted { count: 2, .. })
    ));
    assert_eq!(snapshot_file(&f.source), before);
    assert!(!f.dir.path().join("stage.redb").exists());
    {
        let db = Database::open(&f.source).unwrap();
        let txn = db.begin_write().unwrap();
        txn.open_table(uuid_entities::EPOCH_TABLE)
            .unwrap()
            .insert(epochkey.as_slice(), (u64::MAX - 2).to_be_bytes().as_slice())
            .unwrap();
        txn.commit().unwrap();
    }
    let historical = crate::codec::chunk_key_bytes(ChunkKey::new(
        WorldId::new(1),
        DimensionId::new(0),
        ChunkPos::new(5, -2),
    ));
    {
        let db = Database::open(&f.source).unwrap();
        let txn = db.begin_write().unwrap();
        txn.open_table(uuid_entities::EPOCH_TABLE)
            .unwrap()
            .insert(historical.as_slice(), u64::MAX.to_be_bytes().as_slice())
            .unwrap();
        txn.commit().unwrap();
    }
    let sealed = f.seal();
    sealed.migrate().unwrap();
    let snapshot = snapshot_file(&f.source);
    assert_eq!(
        snapshot["ferrumc:entity_chunk_epoch"][epochkey.as_slice()],
        u64::MAX.to_be_bytes()
    );
    assert_eq!(
        snapshot["ferrumc:entity_chunk_epoch"][historical.as_slice()],
        u64::MAX.to_be_bytes()
    );
}

#[test]
fn capacity_request_history_mapping_plan_scratch_checkpoint_and_physical_bounds() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let defaults = OwnershipMigrationLimits::defaults();
    for field in [2, 3] {
        let mut encoded = defaults.encoded();
        encoded[field] = if field == 2 { 2 } else { 6 };
        let limits = OwnershipMigrationLimits::from_encoded(encoded).unwrap();
        assert!(RedbStore::prepare_player_entity_ownership_migration(
            &f.source,
            &f.mapping,
            f.dir.path(),
            config(),
            limits
        )
        .is_err());
        assert_eq!(snapshot_file(&f.source), before);
    }
    let capacity = crate::OwnershipCapacity::new(
        NonZeroUsize::new(149).unwrap(),
        NonZeroU64::new(7).unwrap(),
        NonZeroU64::new(7).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        RedbStore::prepare_player_entity_ownership_migration(
            &f.source,
            &f.mapping,
            f.dir.path(),
            OwnershipConfig::new(capacity, config().open_limits()),
            defaults
        ),
        Err(Error::RequestCapacity)
    ));
    for (receipts, revisions) in [(6, 7), (7, 6)] {
        let capacity = crate::OwnershipCapacity::new(
            NonZeroUsize::new(crate::MAX_OWNERSHIP_REQUEST_BYTES).unwrap(),
            NonZeroU64::new(receipts).unwrap(),
            NonZeroU64::new(revisions).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            RedbStore::prepare_player_entity_ownership_migration(
                &f.source,
                &f.mapping,
                f.dir.path(),
                OwnershipConfig::new(capacity, config().open_limits()),
                defaults
            ),
            Err(Error::HistoryCapacity)
        ));
    }
    for field in [4, 5, 6] {
        let separate = Fixture::new();
        let mut encoded = defaults.encoded();
        encoded[field] = 1;
        let limits = OwnershipMigrationLimits::from_encoded(encoded).unwrap();
        let prepared = RedbStore::prepare_player_entity_ownership_migration(
            &separate.source,
            &separate.mapping,
            separate.dir.path(),
            config(),
            limits,
        );
        let result = prepared.and_then(|prepared| prepared.prepare_files(separate.output()));
        assert!(result.is_err());
        assert_eq!(snapshot_file(&separate.source), before);
    }
}

#[test]
fn empty_source_has_no_synthetic_resource_and_exact_five_metadata_delta() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("empty.redb");
    drop(RedbStore::open_with_uuid_entities(&source).unwrap());
    let mapping = dir.path().join("empty.mapping");
    fs::write(&mapping, mapping_bytes(&[])).unwrap();
    let before = snapshot_file(&source);
    let output = OwnershipMigrationOutputPaths::new(
        dir.path().join("backup"),
        dir.path().join("plan"),
        dir.path().join("stage"),
        dir.path().join("checkpoint"),
        dir.path(),
    )
    .unwrap();
    let bounded = RedbStore::prepare_player_entity_ownership_migration(
        &source,
        &mapping,
        dir.path(),
        config(),
        OwnershipMigrationLimits::defaults(),
    )
    .unwrap()
    .prepare_files(output)
    .unwrap();
    assert_eq!(bounded.record.counts[4..], [0; 5]);
    bounded.migrate().unwrap();
    let after = snapshot_file(&source);
    for (name, rows) in before {
        if name != "ferrumc:meta" {
            assert_eq!(after[&name], rows);
        }
    }
    assert_eq!(after["ferrumc:meta"].len(), 6);
    assert!(after["ferrumc:ownership_receipt"].is_empty());
    assert!(after["ferrumc:player_ownership"].is_empty());
    assert!(after["ferrumc:entity_ownership"].is_empty());
}

#[test]
fn valid_target_rehash_cannot_hide_original_payload_epoch_receipt_or_count_drift() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    let initial = sealed.record.clone();
    let stagepath = sealed.stage.path.clone();
    let checkpointpath = sealed.checkpoint.path.clone();
    let stagebytes = fs::read(&stagepath).unwrap();
    drop(sealed);
    for kind in 0..5 {
        {
            let db = Database::open(&stagepath).unwrap();
            let txn = db.begin_write().unwrap();
            if kind == 4 {
                let mut meta = txn.open_table(META_TABLE).unwrap();
                meta.insert("ownership_receipt_count", 8).unwrap();
            } else {
                let definition = match kind {
                    0 => PLAYER_TABLE,
                    1 => ENTITY_TABLE,
                    2 => uuid_entities::EPOCH_TABLE,
                    _ => ownership::RECEIPT,
                };
                let mut table = txn.open_table(definition).unwrap();
                let (key, mut value) = {
                    let (k, v) = table.iter().unwrap().next().unwrap().unwrap();
                    (k.value().to_vec(), v.value().to_vec())
                };
                match kind {
                    0 | 1 => {
                        let last = value.len() - 1;
                        value[last] ^= 0x55;
                    }
                    2 => {
                        let epoch = u64::from_be_bytes(value.as_slice().try_into().unwrap());
                        value = (epoch + 10).to_be_bytes().to_vec();
                    }
                    _ => {
                        value[23] ^= 0x55;
                    }
                }
                table.insert(key.as_slice(), value.as_slice()).unwrap();
            }
            txn.commit().unwrap();
        }
        let raw = snapshot_file(&stagepath);
        let bytes = fs::read(&stagepath).unwrap();
        let mut changed = initial.clone();
        changed.target = logical_hash(&raw);
        changed.evidence[4].physical = (bytes.len() as u64, Sha256::digest(&bytes).into());
        fs::write(&checkpointpath, changed.encode().unwrap()).unwrap();
        assert!(
            RedbStore::resume_player_entity_ownership_migration(
                f.resume_paths(),
                config(),
                OwnershipMigrationLimits::defaults()
            )
            .is_err(),
            "target mutation {kind}"
        );
        assert_eq!(snapshot_file(&f.source), before);
        fs::write(&stagepath, &stagebytes).unwrap();
        fs::write(&checkpointpath, initial.encode().unwrap()).unwrap();
    }
    assert!(matches!(
        f.resume(),
        OwnershipMigrationResume::ReadyToCommit(_)
    ));
}

#[test]
fn duplicate_actual_operation_id_in_rebound_plan_is_refused() {
    let f = Fixture::new();
    let sealed = f.seal();
    let mut record = sealed.record.clone();
    let path = sealed.plan.path.clone();
    let checkpointpath = sealed.checkpoint.path.clone();
    let mut bytes = fs::read(&path).unwrap();
    drop(sealed);
    let first: Vec<_> = bytes[128 + 60..128 + 76].to_vec();
    bytes[128 + 148 + 60..128 + 148 + 76].copy_from_slice(&first);
    fs::write(&path, &bytes).unwrap();
    record.plan = Sha256::digest(&bytes).into();
    record.evidence[3].physical = (bytes.len() as u64, record.plan);
    fs::write(&checkpointpath, record.encode().unwrap()).unwrap();
    assert!(matches!(
        RedbStore::resume_player_entity_ownership_migration(
            f.resume_paths(),
            config(),
            OwnershipMigrationLimits::defaults()
        ),
        Err(Error::DuplicateImportId)
    ));
}

#[test]
fn replaced_owned_verification_copy_is_preserved_and_cleanup_is_typed() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    let mut replacement = None;
    let error = sealed
        .migrate_observed(&mut |point| {
            if point == Cutpoint::AfterCopyCreate && replacement.is_none() {
                let path = fs::read_dir(f.dir.path())
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .find(|path| {
                        path.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(".ferrumc-ownership-verify")
                    })
                    .unwrap();
                fs::rename(&path, path.with_extension("own-kept")).unwrap();
                fs::write(&path, b"foreign replacement").unwrap();
                replacement = Some(path);
            }
            Ok(())
        })
        .unwrap_err();
    assert!(
        matches!(error,Error::Producer(ref e) if matches!(**e,super::super::uuid_migration::UuidEntityMigrationError::Cleanup{..}))
    );
    assert_eq!(
        fs::read(replacement.unwrap()).unwrap(),
        b"foreign replacement"
    );
    assert_eq!(snapshot_file(&f.source), before);
}

#[test]
fn exact_physical_copy_admission_and_one_below_are_checked_before_creation() {
    let f = Fixture::new();
    let sealed = f.seal();
    let physical = sealed.record.evidence[4].physical;
    let original = fs::read(&sealed.stage.path).unwrap();
    for (limit, admitted) in [(physical.0 - 1, false), (physical.0, true)] {
        let mut encoded = OwnershipMigrationLimits::defaults().encoded();
        encoded[5] = limit;
        let limits = OwnershipMigrationLimits::from_encoded(encoded).unwrap();
        let mut created = false;
        let result = with_copy(
            &sealed.stage,
            physical,
            f.dir.path(),
            limits,
            &mut |point| {
                if point == Cutpoint::AfterCopyCreate {
                    created = true;
                }
                Ok(())
            },
            |_, _| Ok(()),
        );
        assert_eq!(result.is_ok(), admitted);
        assert_eq!(created, admitted);
        assert_eq!(fs::read(&sealed.stage.path).unwrap(), original);
    }
}

#[test]
fn closed_hash_preserves_valid_bytes_and_refuses_wrong_extent() {
    let dir = tempfile::tempdir().unwrap();
    let file = LockedFile::open(&dir.path().join("hash-evidence"), true, false).unwrap();
    let bytes: Vec<_> = (0..131_073).map(|i| (i % 251) as u8).collect();
    file.write_exact(&bytes).unwrap();
    let expected = (bytes.len() as u64, Sha256::digest(&bytes).into());
    assert_eq!(file.hash_expected_len(expected.0).unwrap(), expected);
    assert_eq!(file.hash().unwrap(), expected);
    assert_eq!(
        checkpoint::Evidence::capture(&file, true, expected.0)
            .unwrap()
            .physical,
        expected
    );
    assert!(
        matches!(checkpoint::Evidence::capture(&file, true, expected.0 - 1), Err(Error::ScanCapacity { kind: "closed evidence bytes", required, limit }) if required == expected.0 && limit == expected.0 - 1)
    );
    // Same inode, externally grown sparse extent: refuse before any hash read.
    file.file.set_len(expected.0 + 1_073_741_824).unwrap();
    assert!(matches!(
        file.hash_expected_len(expected.0),
        Err(super::super::uuid_migration::UuidEntityMigrationError::BackupMismatch)
    ));
    file.file.set_len(expected.0).unwrap();
    assert_eq!(file.hash_expected_len(expected.0).unwrap(), expected);
    file.file.set_len(expected.0 - 1).unwrap();
    assert!(matches!(
        file.hash_expected_len(expected.0),
        Err(super::super::uuid_migration::UuidEntityMigrationError::BackupMismatch)
    ));
    let short_sha: [u8; 32] = Sha256::digest(&bytes[..bytes.len() - 1]).into();
    assert_eq!(file.hash().unwrap().1, short_sha);
}

#[test]
fn grown_closed_evidence_refuses_before_copy_and_post_copy_growth_cleans_up() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    let evidence = [
        (
            &sealed.prepared.mapping,
            &sealed.record.evidence[1],
            Role::Mapping,
        ),
        (&sealed.backup, &sealed.record.evidence[2], Role::Backup),
        (&sealed.plan, &sealed.record.evidence[3], Role::Plan),
        (&sealed.stage, &sealed.record.evidence[4], Role::Stage),
        (
            &sealed.checkpoint,
            &sealed.checkpoint_evidence,
            Role::Checkpoint,
        ),
    ];
    for (file, bound, role) in evidence {
        let writer = fs::OpenOptions::new().write(true).open(&file.path).unwrap();
        writer.set_len(bound.physical.0 + 1_073_741_824).unwrap();
        assert!(
            matches!(bound.verify(file, role, true), Err(Error::EvidenceDrift(actual)) if actual == role)
        );
        if matches!(role, Role::Backup | Role::Stage) {
            let mut created = false;
            assert!(matches!(
                with_copy(file, bound.physical, f.dir.path(), sealed.prepared.limits,
                    &mut |point| {
                        created |= point == Cutpoint::BeforeCopyCreate;
                        Ok(())
                    }, |_, _| Ok(())),
                Err(Error::Producer(error)) if matches!(*error, super::super::uuid_migration::UuidEntityMigrationError::BackupMismatch)
            ));
            assert!(!created);
        }
        writer.set_len(bound.physical.0).unwrap();
        bound.verify(file, role, true).unwrap();
    }
    let physical = sealed.record.evidence[4].physical;
    let result = with_copy(
        &sealed.stage,
        physical,
        f.dir.path(),
        sealed.prepared.limits,
        &mut |point| {
            if point == Cutpoint::AfterCopyWrite {
                sealed
                    .stage
                    .file
                    .set_len(physical.0 + 1_073_741_824)
                    .unwrap();
            }
            Ok(())
        },
        |_, _| Ok(()),
    );
    assert!(
        matches!(result, Err(Error::Producer(error)) if matches!(*error, super::super::uuid_migration::UuidEntityMigrationError::BackupMismatch))
    );
    sealed.stage.file.set_len(physical.0).unwrap();
    sealed.evidence().unwrap();
    assert!(!fs::read_dir(f.dir.path()).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".ferrumc-ownership-verify")));
    drop(sealed);
    assert_eq!(snapshot_file(&f.source), before);
}

#[test]
fn after_committed_stage_copy_cleanup_is_uncertain_and_exactly_resumable() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    let expected = independent_target(before, &sealed);
    let evidence: Vec<_> = sealed.record.evidence[1..]
        .iter()
        .map(|e| &e.path)
        .chain([&sealed.checkpoint.path])
        .map(|path| (path.clone(), fs::read(path).unwrap()))
        .collect();
    let mut committed = false;
    let mut cleanups = 0;
    let error = sealed
        .migrate_observed(&mut |point| {
            committed |= point == Cutpoint::AfterCommit;
            if point == Cutpoint::AfterCopyCleanup {
                cleanups += 1;
                if cleanups == 2 {
                    assert!(committed);
                    return Err(Error::SourceDrift);
                }
            }
            Ok(())
        })
        .unwrap_err();
    assert!(
        matches!(error, Error::CommitUncertain { target, .. } if target == logical_hash(&expected))
    );
    assert_eq!(cleanups, 2);
    assert_eq!(snapshot_file(&f.source), expected);
    for (path, bytes) in evidence {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert!(
        matches!(f.resume(), OwnershipMigrationResume::AlreadyCommitted(result) if result.target_sha256() == logical_hash(&expected))
    );
    assert!(!fs::read_dir(f.dir.path()).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".ferrumc-ownership-verify")));
}

#[test]
fn replaced_committed_stage_copy_preserves_foreign_file_and_resumes_exact_target() {
    let f = Fixture::new();
    let before = snapshot_file(&f.source);
    let sealed = f.seal();
    let expected = independent_target(before, &sealed);
    let evidence: Vec<_> = sealed.record.evidence[1..]
        .iter()
        .map(|e| &e.path)
        .chain([&sealed.checkpoint.path])
        .map(|path| (path.clone(), fs::read(path).unwrap()))
        .collect();
    let mut committed = false;
    let mut cleanups = 0;
    let mut foreign = None;
    let error = sealed
        .migrate_observed(&mut |point| {
            committed |= point == Cutpoint::AfterCommit;
            if point == Cutpoint::BeforeCopyCleanup {
                cleanups += 1;
                if cleanups == 2 {
                    assert!(committed);
                    let path = fs::read_dir(f.dir.path())
                        .unwrap()
                        .map(|entry| entry.unwrap().path())
                        .find(|path| {
                            path.file_name()
                                .unwrap()
                                .to_string_lossy()
                                .starts_with(".ferrumc-ownership-verify")
                        })
                        .unwrap();
                    fs::rename(&path, path.with_extension("own-kept")).unwrap();
                    fs::write(&path, b"foreign postcommit replacement").unwrap();
                    foreign = Some(path);
                }
            }
            Ok(())
        })
        .unwrap_err();
    assert!(
        matches!(error, Error::CommitUncertain { target, .. } if target == logical_hash(&expected))
    );
    assert_eq!(cleanups, 2);
    let foreign = foreign.unwrap();
    assert_eq!(
        fs::read(&foreign).unwrap(),
        b"foreign postcommit replacement"
    );
    assert_eq!(snapshot_file(&f.source), expected);
    for (path, bytes) in evidence {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert!(
        matches!(f.resume(), OwnershipMigrationResume::AlreadyCommitted(result) if result.target_sha256() == logical_hash(&expected))
    );
    assert_eq!(
        fs::read(foreign).unwrap(),
        b"foreign postcommit replacement"
    );
}
