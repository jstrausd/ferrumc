//! Actual close/reopen refusals preserve the entire existing catalog and logical rows.
use super::*;
use crate::block_test_support::*;
use crate::{
    ChunkImageAction, ChunkImageCasRequest, ChunkImageChange, PlayerBlockDurabilityStore,
    WorldStore,
};
use redb::{MultimapTableHandle, ReadableMultimapTable};
use std::collections::BTreeMap;
use std::fs;

/// Every table's raw rows, keyed by table name.
type Fingerprint = BTreeMap<String, Vec<(Vec<u8>, Vec<u8>)>>;

fn fingerprint(path: &Path) -> Fingerprint {
    let db = Database::open(path).unwrap();
    let t = db.begin_read().unwrap();
    let mut out = BTreeMap::new();
    for d in t.list_tables().unwrap() {
        let name = d.name();
        let rows = if let Ok(table) = t.open_table(TableDefinition::<&[u8], &[u8]>::new(name)) {
            table
                .iter()
                .unwrap()
                .map(|row| {
                    let (k, v) = row.unwrap();
                    (k.value().to_vec(), v.value().to_vec())
                })
                .collect()
        } else {
            t.open_table(TableDefinition::<&str, u64>::new(name))
                .unwrap()
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
        };
        out.insert(name.into(), rows);
    }
    for d in t.list_multimap_tables().unwrap() {
        let rows = t
            .open_multimap_table(redb::MultimapTableDefinition::<&[u8], &[u8]>::new(d.name()))
            .unwrap()
            .iter()
            .unwrap()
            .flat_map(|row| {
                let (k, values) = row.unwrap();
                values.map(move |v| (k.value().to_vec(), v.unwrap().value().to_vec()))
            })
            .collect();
        out.insert(format!("multimap:{}", d.name()), rows);
    }
    out
}
fn data(
    txn: &redb::WriteTransaction,
    name: &'static str,
    key: &[u8],
    mutate: impl FnOnce(&mut Vec<u8>),
) {
    let mut t = txn
        .open_table(TableDefinition::<&[u8], &[u8]>::new(name))
        .unwrap();
    let mut v = t.get(key).unwrap().unwrap().value().to_vec();
    mutate(&mut v);
    t.insert(key, v.as_slice()).unwrap();
}
#[tokio::test]
#[allow(clippy::too_many_lines)] // Every corruption case stays visible beside its refusal check.
async fn actual_catalog_metadata_receipt_history_journal_corruption_refuses_without_repair() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original");
    let s = crate::RedbStore::open_with_player_block_durability(&original, config(), dir.path())
        .unwrap();
    crate::block_test_support::initialize(&s).await;
    seed_air(&s).await;
    s.commit_player_blocks(joint(&s, 3).await).await.unwrap();
    drop(s);
    let cases = 45;
    for case in 0..cases {
        let path = dir.path().join(format!("bad-{case}"));
        fs::copy(&original, &path).unwrap();
        {
            let db = Database::open(&path).unwrap();
            let tx = db.begin_write().unwrap();
            match case {
                0 => {
                    tx.delete_table(HEADER).unwrap();
                }
                1 => {
                    tx.open_table(TableDefinition::<&[u8], &[u8]>::new("foreign"))
                        .unwrap()
                        .insert(&[1][..], &[2][..])
                        .unwrap();
                }
                2 => {
                    tx.delete_table(HEADER).unwrap();
                    tx.open_table(TableDefinition::<&str, u64>::new(raw::CHUNK_OWNER))
                        .unwrap()
                        .insert("foreign", 41)
                        .unwrap();
                }
                3 => {
                    tx.open_multimap_table(redb::MultimapTableDefinition::<&[u8], &[u8]>::new(
                        "foreign",
                    ))
                    .unwrap()
                    .insert(&[1][..], &[2][..])
                    .unwrap();
                }
                4 => {
                    tx.open_table(META_TABLE)
                        .unwrap()
                        .remove(CHUNK_COUNT)
                        .unwrap();
                }
                5 => {
                    tx.open_table(META_TABLE)
                        .unwrap()
                        .insert("unknown", 17)
                        .unwrap();
                }
                6 => {
                    tx.open_table(META_TABLE)
                        .unwrap()
                        .insert(META_FORMAT_KEY, 99)
                        .unwrap();
                }
                7 => {
                    tx.open_table(META_TABLE)
                        .unwrap()
                        .insert(REQUEST_LIMIT, 144)
                        .unwrap();
                }
                8..=13 => {
                    let name = [
                        ownership::RECEIPT_COUNT,
                        ownership::REVISION_COUNT,
                        RECEIPT_BYTES,
                        CHUNK_COUNT,
                        JOURNAL_COUNT,
                        JOURNAL_BYTES,
                    ][case - 8];
                    tx.open_table(META_TABLE)
                        .unwrap()
                        .insert(name, 999)
                        .unwrap();
                }
                14 => {
                    tx.open_table(META_TABLE)
                        .unwrap()
                        .insert(WATERMARK, u64::MAX)
                        .unwrap();
                }
                15 => {
                    tx.open_table(super::super::GENERATION_TABLE)
                        .unwrap()
                        .remove(scope().encode().as_slice())
                        .unwrap();
                }
                16 => data(&tx, "ferrumc:generation", &scope().encode(), |v| v[0] = 0),
                17 => {
                    tx.open_table(HEADER)
                        .unwrap()
                        .remove(chunk_key(key(0)).as_slice())
                        .unwrap();
                }
                18 => data(&tx, raw::CHUNK_OWNER, &chunk_key(key(0)), |v| v[6] = 0),
                19 => data(&tx, raw::CHUNK_OWNER, &chunk_key(key(0)), |v| {
                    v[7..15].fill(0);
                }),
                20 => data(&tx, raw::CHUNK_OWNER, &chunk_key(key(0)), |v| {
                    v[15..31].fill(0xff);
                }),
                21 => data(&tx, "ferrumc:chunk", &chunk_key(key(0)), |v| v[3] ^= 1),
                22 => {
                    tx.open_table(super::super::CHUNK_TABLE)
                        .unwrap()
                        .remove(chunk_key(key(0)).as_slice())
                        .unwrap();
                }
                23 => {
                    tx.open_table(ownership::RECEIPT)
                        .unwrap()
                        .remove(id(3).to_bytes().as_slice())
                        .unwrap();
                }
                24 => data(
                    &tx,
                    crate::ownership::raw::RECEIPT,
                    &id(3).to_bytes(),
                    |v| v.push(0),
                ),
                25 => data(
                    &tx,
                    crate::ownership::raw::RECEIPT,
                    &id(3).to_bytes(),
                    |v| {
                        v.pop();
                    },
                ),
                26 => data(
                    &tx,
                    crate::ownership::raw::RECEIPT,
                    &id(3).to_bytes(),
                    |v| v[5] = 2,
                ),
                27 => data(
                    &tx,
                    crate::ownership::raw::RECEIPT,
                    &id(3).to_bytes(),
                    |v| *v.last_mut().unwrap() = 1,
                ),
                28 => data(
                    &tx,
                    crate::ownership::raw::RECEIPT,
                    &id(3).to_bytes(),
                    |v| {
                        let n = v.len();
                        v[n - 40] ^= 1;
                    },
                ),
                29 => data(
                    &tx,
                    crate::ownership::raw::RECEIPT,
                    &id(3).to_bytes(),
                    |v| {
                        let n = u32::from_be_bytes(v[6..10].try_into().unwrap()) as usize;
                        v[10 + n + 8] ^= 1;
                    },
                ),
                30 => data(
                    &tx,
                    crate::ownership::raw::PLAYER_OWNER,
                    player().id().as_uuid().as_bytes(),
                    |v| v[6..14].fill(0),
                ),
                31 => {
                    tx.open_table(ownership::PLAYER_OWNER)
                        .unwrap()
                        .remove(player().id().as_uuid().as_bytes().as_slice())
                        .unwrap();
                }
                32 => data(
                    &tx,
                    "ferrumc:player",
                    player().id().as_uuid().as_bytes(),
                    |v| v[0] ^= 1,
                ),
                33 => {
                    tx.open_table(JOURNAL)
                        .unwrap()
                        .remove(0u64.to_be_bytes().as_slice())
                        .unwrap();
                }
                34 => data(&tx, raw::JOURNAL, &0u64.to_be_bytes(), |v| {
                    v[30..32].copy_from_slice(&1u16.to_be_bytes());
                }),
                35 => data(&tx, raw::JOURNAL, &0u64.to_be_bytes(), |v| {
                    v[32..40].copy_from_slice(&1u64.to_be_bytes());
                }),
                36 => data(&tx, raw::JOURNAL, &0u64.to_be_bytes(), |v| {
                    v[14..30].fill(0xff);
                }),
                37 => data(&tx, raw::JOURNAL, &0u64.to_be_bytes(), |v| v[6..14].fill(0)),
                38 => data(&tx, raw::JOURNAL, &0u64.to_be_bytes(), |v| v[40] ^= 1),
                39 => {
                    tx.open_table(JOURNAL)
                        .unwrap()
                        .insert(9u64.to_be_bytes().as_slice(), &[1][..])
                        .unwrap();
                }
                40 => {
                    tx.open_table(HEADER)
                        .unwrap()
                        .insert(
                            chunk_key(key(3)).as_slice(),
                            raw::encode_header(raw::Header {
                                stamp: crate::OwnershipStamp::checked(2, 1).unwrap(),
                                id: id(3),
                            })
                            .as_slice(),
                        )
                        .unwrap();
                }
                41 => {
                    tx.open_table(TableDefinition::<&[u8], &[u8]>::new(
                        "ferrumc:entity_locator",
                    ))
                    .unwrap()
                    .insert(&[1; 24][..], &[1; 8][..])
                    .unwrap();
                }
                42 => {
                    tx.open_table(TableDefinition::<&[u8], &[u8]>::new("ferrumc:mutation_log"))
                        .unwrap()
                        .insert(0u64.to_be_bytes().as_slice(), &[1][..])
                        .unwrap();
                }
                43 => {
                    tx.open_table(TableDefinition::<&[u8], &[u8]>::new(
                        "ferrumc:mutation_batch",
                    ))
                    .unwrap()
                    .insert(&[1; 16][..], &[1][..])
                    .unwrap();
                }
                44 => {
                    tx.open_table(ownership::RECEIPT)
                        .unwrap()
                        .insert(
                            id(99).to_bytes().as_slice(),
                            include_bytes!(
                                "../../../tests/fixtures/player_block_format7/joint.fcbr"
                            )
                            .as_slice(),
                        )
                        .unwrap();
                }
                _ => unreachable!(),
            }
            tx.commit().unwrap();
        }
        let before = fingerprint(&path);
        assert!(
            crate::RedbStore::open_with_player_block_durability(&path, config(), dir.path())
                .is_err(),
            "case{case}"
        );
        assert_eq!(
            fingerprint(&path),
            before,
            "refused open mutated case{case}"
        );
    }
}
#[tokio::test]
async fn over128_retained_fresh_noop_receipts_reopen_without_unbounded_cache_or_fake_cap() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many");
    let s =
        crate::RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    s.ensure_generation_identity(scope(), generation())
        .await
        .unwrap();
    for n in 1..=130 {
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let outcome = s
            .compare_exchange_chunk_images(
                ChunkImageCasRequest::new(
                    id(n),
                    snap,
                    vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)],
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert!(outcome.receipt().chunks()[0].1.is_fresh());
    }
    drop(s);
    let before = fingerprint(&path);
    let s =
        crate::RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    drop(s);
    assert_eq!(fingerprint(&path), before);
}
#[test]
fn actual_disk_index_extent_ranges_cleanup_and_foreign_replacements() {
    let dir = tempfile::tempdir().unwrap();
    let index = Index::create(dir.path(), 16 * 1024 * 1024).unwrap();
    let extent = index.file.as_ref().unwrap().file().unwrap().len().unwrap();
    assert!(extent > 0 && extent <= 16 * 1024 * 1024);
    index.insert(0, 0, 3).unwrap();
    index.insert(1, 0, 3).unwrap();
    index.insert(1, 4, 8).unwrap();
    index.validate(&mut Budget::hard()).unwrap();
    index.insert(1, 7, 10).unwrap();
    assert!(index.validate(&mut Budget::hard()).is_err());
    index.finish().unwrap();
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    // redb's transient creation and abort writes exceed the final length, so search
    // the real whole-lifecycle boundary (create, abort, cleanup): `admitted` passes,
    // `admitted - 1` refuses with ScratchCapacity, and both leave no owned file.
    let lifecycle = |limit: u64| -> BlockResult<u64> {
        let index = Index::create(dir.path(), limit)?;
        let len = index.file.as_ref().unwrap().file().unwrap().len().unwrap();
        index.finish()?;
        Ok(len)
    };
    let (mut refused, mut admitted) = (1u64, 16 * 1024 * 1024);
    assert_eq!(
        lifecycle(refused),
        Err(BlockDurabilityError::ScratchCapacity)
    );
    while admitted - refused > 1 {
        let limit = refused + (admitted - refused) / 2;
        match lifecycle(limit) {
            Ok(len) => {
                assert!(len <= limit);
                admitted = limit;
            }
            Err(BlockDurabilityError::ScratchCapacity) => refused = limit,
            Err(e) => panic!("{e:?}"),
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
    assert!(extent <= admitted);
    lifecycle(admitted).unwrap();
    assert_eq!(
        lifecycle(admitted - 1),
        Err(BlockDurabilityError::ScratchCapacity)
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    for symlink in [false, true] {
        let index = Index::create(dir.path(), 16 * 1024 * 1024).unwrap();
        let name = fs::read_dir(dir.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let moved = dir.path().join("owned-renamed");
        fs::rename(&name, &moved).unwrap();
        if symlink {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&moved, &name).unwrap();
        } else {
            fs::write(&name, b"foreign replacement").unwrap();
        }
        assert!(matches!(
            index.finish(),
            Err(BlockDurabilityError::ScratchCleanup(_))
        ));
        assert!(fs::symlink_metadata(&name).is_ok());
        if !symlink {
            assert_eq!(fs::read(&name).unwrap(), b"foreign replacement");
        }
        fs::remove_file(&name).unwrap();
        fs::remove_file(&moved).unwrap();
    }
}
/// Replaces the index's owned scratch file with a foreign one, so its cleanup must fail.
fn sabotage(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let name = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".ferrumc-block-ranges")
        })
        .unwrap();
    let moved = dir.join("owned-renamed");
    fs::rename(&name, &moved).unwrap();
    fs::write(&name, b"foreign replacement").unwrap();
    (name, moved)
}
/// A failed initialization or scan stays the reported cause when the scratch cleanup fails
/// too (the cleanup failure is logged); a cleanup failure alone is still reported. The
/// foreign file at the scratch path is never removed.
#[test]
fn original_failure_is_reported_when_scratch_cleanup_also_fails() {
    let scratch = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let check = |(name, moved): (std::path::PathBuf, std::path::PathBuf)| {
        assert_eq!(fs::read(&name).unwrap(), b"foreign replacement");
        fs::remove_file(&name).unwrap();
        fs::remove_file(&moved).unwrap();
        assert_eq!(fs::read_dir(scratch.path()).unwrap().count(), 0);
    };
    // Initialization fails before the scan takes the index (empty redb file opened as existing).
    let empty = Database::create(dir.path().join("empty.redb")).unwrap();
    let index = Index::create(scratch.path(), 16 * 1024 * 1024).unwrap();
    let paths = sabotage(scratch.path());
    assert!(matches!(
        super::initialize(&empty, StoreOrigin::Existing, config(), index),
        Err(BlockDurabilityError::CatalogMismatch(reason)) if reason.starts_with("empty redb file")
    ));
    check(paths);
    // The scan itself fails (a foreign table in an otherwise valid Format7 catalog).
    let path = dir.path().join("seven.redb");
    drop(
        crate::RedbStore::open_with_player_block_durability(&path, config(), scratch.path())
            .unwrap(),
    );
    let db = Database::open(&path).unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(TableDefinition::<&[u8], &[u8]>::new("foreign"))
        .unwrap();
    txn.commit().unwrap();
    let index = Index::create(scratch.path(), 16 * 1024 * 1024).unwrap();
    let paths = sabotage(scratch.path());
    assert!(matches!(
        super::initialize(&db, StoreOrigin::Existing, config(), index),
        Err(BlockDurabilityError::CatalogMismatch(_))
    ));
    check(paths);
    // Without an earlier failure the cleanup failure is the reported error; nothing commits.
    let fresh = Database::create(dir.path().join("fresh.redb")).unwrap();
    let index = Index::create(scratch.path(), 16 * 1024 * 1024).unwrap();
    let paths = sabotage(scratch.path());
    assert!(matches!(
        super::initialize(&fresh, StoreOrigin::Fresh, config(), index),
        Err(BlockDurabilityError::ScratchCleanup(_))
    ));
    check(paths);
    assert!(fresh
        .begin_read()
        .unwrap()
        .list_tables()
        .unwrap()
        .next()
        .is_none());
}

// These counters are local to a real scan; wrapping its actual redb guard observes both
// acquisition and Drop, without process-global state or a copied receipt.
#[derive(Default)]
pub(super) struct JournalObservation {
    counts: std::rc::Rc<std::cell::Cell<(usize, usize, usize, usize)>>,
    sizes: std::rc::Rc<std::cell::RefCell<Vec<usize>>>,
}
pub(super) struct ObservedGuard<B> {
    row: B,
    observation: std::rc::Rc<std::cell::Cell<(usize, usize, usize, usize)>>,
}
impl JournalObservation {
    pub(super) fn retain<B: AsRef<[u8]>>(&self, row: B) -> ObservedGuard<B> {
        let (reads, live, peak, max_bytes) = self.counts.get();
        let len = row.as_ref().len();
        self.counts
            .set((reads + 1, live + 1, peak.max(live + 1), max_bytes.max(len)));
        self.sizes.borrow_mut().push(len);
        ObservedGuard {
            row,
            observation: self.counts.clone(),
        }
    }
}
impl<B: AsRef<[u8]>> AsRef<[u8]> for ObservedGuard<B> {
    fn as_ref(&self) -> &[u8] {
        self.row.as_ref()
    }
}
impl<B> Drop for ObservedGuard<B> {
    fn drop(&mut self) {
        let (reads, live, peak, max_bytes) = self.observation.get();
        self.observation.set((reads, live - 1, peak, max_bytes));
    }
}
#[tokio::test]
async fn scoped_scan_reads_parses_hashes_once_per_operation_and_releases_each_borrowed_guard() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observed");
    let store =
        crate::RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    let other_scope =
        crate::GenerationKey::new(ferrumc_core::WorldId::new(18), scope().dimension());
    store
        .ensure_generation_identity(other_scope, generation())
        .await
        .unwrap();
    crate::block_test_support::initialize(&store).await;
    seed_air(&store).await;
    store
        .commit_player_blocks(joint(&store, 3).await)
        .await
        .unwrap();
    let other_key =
        crate::ChunkKey::new(other_scope.world(), other_scope.dimension(), key(0).pos());
    let snapshot = store
        .load_owned_blocks(
            crate::BlockSnapshotSpec::new(other_scope, generation(), None, vec![other_key])
                .unwrap(),
        )
        .await
        .unwrap();
    let at = pos(1, 300);
    let old = snapshot.columns()[0]
        .chunk()
        .get_block(at)
        .unwrap()
        .as_u32();
    let request = crate::BlockMutationRequest::new(
        id(4),
        snapshot,
        vec![ChunkImageChange::new(other_key, ChunkImageAction::Keep)],
        vec![],
        vec![
            effect_in(other_scope, Meta::default(), at, old, old + 1, None, None),
            effect_in(other_scope, Meta::default(), at, old + 1, old, None, None),
        ],
    )
    .unwrap();
    store.commit_blocks(request).await.unwrap();
    drop(store);
    let db = Database::open(&path).unwrap();
    let txn = db.begin_write().unwrap();
    let mut reader = reader(&txn, config()).unwrap();
    let mut relation = Budget::startup(config());
    reader.budget = Some(relation.clone());
    let table = &reader.blocks.as_ref().unwrap()[3];
    let observation = JournalObservation::default();
    let mut raw_budget = RawBudget {
        rows: 0,
        bytes: 0,
        config: config(),
    };
    let (rows, _) =
        scoped_journal(table, &reader, &mut raw_budget, &mut relation, &observation).unwrap();
    assert_eq!(rows, 6);
    assert_eq!(relation.frame_work.get(), (2, 2));
    let (reads, live, peak, bytes) = observation.counts.get();
    assert_eq!((reads, live, peak), (2, 0, 1));
    println!("Journal observation: reads={reads}, live_after={live}, peak_receipts={peak}, max_receipt_bytes={bytes}, frame_parse_hash={:?}", relation.frame_work.get());
    assert_ne!(observation.sizes.borrow()[0], observation.sizes.borrow()[1]);
    assert_eq!(bytes, *observation.sizes.borrow().iter().max().unwrap());
    assert_eq!(raw_budget.rows, 6);
    drop(reader);
    drop(txn);
    drop(db);
    let before = fingerprint(&path);
    drop(crate::RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap());
    assert_eq!(fingerprint(&path), before);
}

async fn cycle(store: &crate::RedbStore, operation: u8, effects: usize) {
    let snapshot = store
        .load_owned_blocks(spec(false, vec![key(0)]))
        .await
        .unwrap();
    let at = pos(1, 300);
    let old = snapshot.columns()[0]
        .chunk()
        .get_block(at)
        .unwrap()
        .as_u32();
    let changes = vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)];
    let effects = (0..effects)
        .map(|i| {
            if i % 2 == 0 {
                effect(at, old, old + 1, None, None)
            } else {
                effect(at, old + 1, old, None, None)
            }
        })
        .collect();
    store
        .commit_blocks(
            crate::BlockMutationRequest::new(id(operation), snapshot, changes, vec![], effects)
                .unwrap(),
        )
        .await
        .unwrap();
}
#[tokio::test]
#[allow(clippy::too_many_lines)] // Each physical corruption and its full/directional refusal are adjacent.
async fn contiguous_operation_corruptions_refuse_in_both_directions_and_preserve_all_rows() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original-ranges");
    let store =
        crate::RedbStore::open_with_player_block_durability(&original, config(), dir.path())
            .unwrap();
    store
        .ensure_generation_identity(scope(), generation())
        .await
        .unwrap();
    cycle(&store, 10, 2).await;
    // A zero-effect receipt between different-sized nonempty ranges must not enter the cache.
    let snap = store
        .load_owned_blocks(spec(false, vec![key(0)]))
        .await
        .unwrap();
    store
        .compare_exchange_chunk_images(
            ChunkImageCasRequest::new(
                id(11),
                snap,
                vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)],
            )
            .unwrap(),
        )
        .await
        .unwrap();
    cycle(&store, 12, 4).await;
    cycle(&store, 13, 2).await;
    drop(store);
    for case in 0..17 {
        let path = dir.path().join(format!("ranges-{case}"));
        fs::copy(&original, &path).unwrap();
        {
            let db = Database::open(&path).unwrap();
            let txn = db.begin_write().unwrap();
            match case {
                0 => data(&txn, raw::JOURNAL, &0u64.to_be_bytes(), |v| v[6] ^= 1), // scope
                1 => data(&txn, raw::JOURNAL, &0u64.to_be_bytes(), |v| {
                    v[14..30].fill(99);
                }), // orphan operation
                2 => data(&txn, raw::JOURNAL, &1u64.to_be_bytes(), |v| {
                    v[30..32].copy_from_slice(&0u16.to_be_bytes());
                }),
                3 => data(&txn, raw::JOURNAL, &1u64.to_be_bytes(), |v| {
                    v[32..40].copy_from_slice(&7u64.to_be_bytes());
                }),
                4 => data(&txn, raw::JOURNAL, &1u64.to_be_bytes(), |v| v[40] ^= 1), // exact effect schema
                5 => data(&txn, raw::JOURNAL, &1u64.to_be_bytes(), |v| {
                    v[14..30].copy_from_slice(&id(12).to_bytes());
                }), // interleaving
                6 => data(&txn, raw::JOURNAL, &2u64.to_be_bytes(), |v| {
                    v[14..30].copy_from_slice(&id(10).to_bytes());
                }), // old operation reuse
                7 => data(&txn, raw::JOURNAL, &2u64.to_be_bytes(), |v| {
                    v[30..32].copy_from_slice(&1u16.to_be_bytes());
                }), // missing start
                8 => {
                    txn.open_table(JOURNAL)
                        .unwrap()
                        .remove(7u64.to_be_bytes().as_slice())
                        .unwrap();
                }
                9 => {
                    let mut table = txn.open_table(JOURNAL).unwrap();
                    let mut value = table
                        .get(7u64.to_be_bytes().as_slice())
                        .unwrap()
                        .unwrap()
                        .value()
                        .to_vec();
                    value[32..40].copy_from_slice(&8u64.to_be_bytes());
                    value[30..32].copy_from_slice(&2u16.to_be_bytes());
                    table
                        .insert(8u64.to_be_bytes().as_slice(), value.as_slice())
                        .unwrap();
                }
                10 => data(&txn, owned_raw::RECEIPT, &id(12).to_bytes(), |v| {
                    let at = v.len() - 50;
                    v[at..at + 8].copy_from_slice(&3u64.to_be_bytes());
                }), // wrong first
                11 => data(&txn, owned_raw::RECEIPT, &id(12).to_bytes(), |v| {
                    let at = v.len() - 42;
                    v[at..at + 2].copy_from_slice(&3u16.to_be_bytes());
                }), // range truncation
                12 => data(&txn, owned_raw::RECEIPT, &id(12).to_bytes(), |v| {
                    let at = v.len() - 42;
                    v[at..at + 2].copy_from_slice(&5u16.to_be_bytes());
                }), // overrun
                13 => data(&txn, raw::JOURNAL, &1u64.to_be_bytes(), |v| {
                    v[14..30].copy_from_slice(&id(11).to_bytes());
                }), // zero-effect receipt has no row
                14 => data(&txn, owned_raw::RECEIPT, &id(10).to_bytes(), |v| {
                    let at = v.len() - 50;
                    v[at..at + 8].copy_from_slice(&2u64.to_be_bytes());
                }), // overlapping ranges
                15 => {
                    // Forge the complete two-effect relation and digest consistently. Only the
                    // full endpoint proof can reject the changed final state against Keep.
                    data(&txn, raw::JOURNAL, &1u64.to_be_bytes(), |v| {
                        v[86..90].copy_from_slice(&99u32.to_be_bytes());
                    });
                    data(&txn, owned_raw::RECEIPT, &id(10).to_bytes(), |v| {
                        let qlen = u32::from_be_bytes(v[6..10].try_into().unwrap()) as usize;
                        // This exact Fresh/Keep fixture has Q=144+2*64: effect tail at Q-10.
                        assert_eq!(qlen, 272);
                        let at = 10 + qlen - 14;
                        v[at..at + 4].copy_from_slice(&99u32.to_be_bytes());
                        let digest = Sha256::new()
                            .chain_update(b"FerrumC block operation v1\0")
                            .chain_update(&v[10..10 + qlen])
                            .finalize();
                        let at = v.len() - 40;
                        v[at..at + 32].copy_from_slice(&digest);
                    });
                }
                _ => {
                    txn.open_table(ownership::RECEIPT)
                        .unwrap()
                        .remove(id(10).to_bytes().as_slice())
                        .unwrap();
                }
            }
            txn.commit().unwrap();
        }
        let before = fingerprint(&path);
        assert_eq!(before.len(), 17);
        // Execute the changed Journal direction itself, even when the earlier FCBR direction
        // would already reject. A consistent endpoint forgery intentionally passes this branch.
        {
            let db = Database::open(&path).unwrap();
            let txn = db.begin_write().unwrap();
            let mut reader = reader(&txn, config()).unwrap();
            let mut budget = Budget::startup(config());
            reader.budget = Some(budget.clone());
            let observation = JournalObservation::default();
            let mut raw_budget = RawBudget {
                rows: 0,
                bytes: 0,
                config: config(),
            };
            let out = scoped_journal(
                &reader.blocks.as_ref().unwrap()[3],
                &reader,
                &mut raw_budget,
                &mut budget,
                &observation,
            );
            assert_eq!(out.is_ok(), case == 15, "Journal case {case}: {out:?}");
            assert_eq!(observation.counts.get().1, 0, "guard leaked on case {case}");
            assert!(observation.counts.get().2 <= 1);
        }
        let scratch = dir.path().join(format!("scratch-{case}"));
        fs::create_dir(&scratch).unwrap();
        let error = crate::RedbStore::open_with_player_block_durability(&path, config(), &scratch)
            .err()
            .unwrap();
        if case == 15 {
            assert!(
                matches!(error, BlockDurabilityError::IncompleteDelta { .. }),
                "{error:?}"
            );
        }
        assert_eq!(fingerprint(&path), before, "case {case}");
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
    }
}
