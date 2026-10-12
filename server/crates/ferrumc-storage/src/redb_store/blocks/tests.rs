//! Actual producer failures, with independent complete logical snapshots.
use super::*;
use crate::block_test_support::*;
use crate::{
    ChunkImage, ChunkImageAction, ChunkImageCasRequest, ChunkImageChange, ChunkRecord,
    PlayerBlockDurabilityStore, PlayerBlockRequest, PlayerEntityOwnershipStore, SchemaVersion,
};
use redb::backends::FileBackend;
use redb::StorageBackend;
use std::collections::BTreeMap;
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::sync_channel,
};

async fn setup(path: &Path, scratch: &Path) -> RedbStore {
    let s = RedbStore::open_with_player_block_durability(path, config(), scratch).unwrap();
    initialize(&s).await;
    seed_air(&s).await;
    s
}
/// Every table's raw rows, keyed by table name.
type Rows = BTreeMap<String, Vec<(Vec<u8>, Vec<u8>)>>;

fn rows(s: &RedbStore) -> Rows {
    super::super::ownership::tests::snapshot(s)
}
fn fail() -> BlockDurabilityError {
    BlockDurabilityError::Backend("injected actual cutpoint".into())
}

#[tokio::test]
async fn every_real_joint_row_and_precommit_failure_retains_all17_tables_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let s = setup(&dir.path().join("count"), dir.path()).await;
    let r = joint(&s, 3).await.0;
    let mut count = 0;
    commit(&s.db, &s.entity_incarnation, config(), &r, |point| {
        if point != Cutpoint::AfterCommit {
            count += 1;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        count, 38,
        "18 actual insert/remove/meta steps, plan and commit"
    );
    drop(s);
    for stop in 0..count {
        let path = dir.path().join(format!("cut-{stop}"));
        let s = setup(&path, dir.path()).await;
        let before = rows(&s);
        let r = joint(&s, 3).await.0;
        let query = r.query;
        let mut visited = 0;
        let result = commit(&s.db, &s.entity_incarnation, config(), &r, |_| {
            let at = visited;
            visited += 1;
            if at == stop {
                Err(fail())
            } else {
                Ok(())
            }
        });
        assert!(
            matches!(result, Err(BlockDurabilityError::Backend(_))),
            "cutpoint{stop}"
        );
        assert_eq!(rows(&s), before, "cutpoint{stop}");
        assert!(s.load_block_receipt(query).await.unwrap().is_none());
        drop(s);
        let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
        assert_eq!(rows(&s), before, "reopen{stop}");
    }
}
#[tokio::test]
async fn actual_pair_removes_and_overlay_inserts_each_abort_whole_transaction() {
    let dir = tempfile::tempdir().unwrap();
    for erase in [false, true] {
        let path = dir.path().join(format!("pair-{erase}"));
        let s = setup(&path, dir.path()).await;
        if erase {
            let snap = s
                .load_owned_blocks(spec(false, vec![key(0)]))
                .await
                .unwrap();
            let view = snap.columns()[0].chunk().clone();
            let overlay = crate::ChunkOverlayRecord::try_from_chunk(
                SchemaVersion::new(80),
                key(0).pos(),
                &view,
                7,
            )
            .unwrap();
            let image = ChunkImage::new(
                Some(ChunkRecord::try_new(SchemaVersion::new(55), view).unwrap()),
                Some(overlay),
            )
            .unwrap();
            s.compare_exchange_chunk_images(
                ChunkImageCasRequest::new(
                    id(4),
                    snap,
                    vec![ChunkImageChange::new(
                        key(0),
                        ChunkImageAction::Replace(image),
                    )],
                )
                .unwrap(),
            )
            .await
            .unwrap();
        }
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let action = if erase {
            ChunkImageAction::Erase
        } else {
            ChunkImageAction::Replace(
                ChunkImage::new(
                    None,
                    Some(
                        crate::ChunkOverlayRecord::try_from_chunk(
                            SchemaVersion::new(91),
                            key(0).pos(),
                            snap.columns()[0].chunk(),
                            5,
                        )
                        .unwrap(),
                    ),
                )
                .unwrap(),
            )
        };
        let r = ChunkImageCasRequest::new(id(5), snap, vec![ChunkImageChange::new(key(0), action)])
            .unwrap()
            .0;
        let before = rows(&s);
        let mut points = 0;
        let result = commit(&s.db, &s.entity_incarnation, config(), &r, |point| {
            if point == Cutpoint::BeforeCommit {
                Err(fail())
            } else {
                points += usize::from(point == Cutpoint::AfterRow);
                Ok(())
            }
        });
        assert!(result.is_err());
        assert_eq!(points, 10);
        for stop in 0..(2 * points + 2) {
            let mut n = 0;
            assert!(commit(&s.db, &s.entity_incarnation, config(), &r, |_| {
                let at = n;
                n += 1;
                if at == stop {
                    Err(fail())
                } else {
                    Ok(())
                }
            })
            .is_err());
            assert_eq!(rows(&s), before);
        }
        s.block_write(r).await.unwrap();
        drop(s);
        RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    }
}
#[tokio::test]
async fn real_aftercommit_loss_and_worker_panic_replay_exact_old_incarnation() {
    for panic in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lost");
        let s = setup(&path, dir.path()).await;
        let r = joint(&s, 3).await.0;
        let query = r.query;
        let result = s
            .block_write_hooks(r.clone(), move |point| {
                if point == Cutpoint::AfterCommit {
                    assert!(!panic, "actual started worker panic");
                    return Err(fail());
                }
                Ok(())
            })
            .await;
        assert!(
            matches!(result,Err(BlockDurabilityError::ExecutionUncertain{query:q,..}) if q==query)
        );
        let receipt = s.load_block_receipt(query).await.unwrap().unwrap();
        let after = rows(&s);
        drop(s);
        let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
        let out = s.block_write(r).await.unwrap();
        assert_eq!(out.receipt(), &receipt);
        assert_eq!(out.disposition(), OwnershipCommitDisposition::Replayed);
        assert_eq!(rows(&s), after);
    }
}
#[derive(Debug)]
struct CommitFault {
    inner: FileBackend,
    fail: Arc<AtomicBool>,
    write_fault: bool,
}
impl StorageBackend for CommitFault {
    fn len(&self) -> io::Result<u64> {
        self.inner.len()
    }
    fn read(&self, o: u64, n: usize) -> io::Result<Vec<u8>> {
        self.inner.read(o, n)
    }
    fn set_len(&self, n: u64) -> io::Result<()> {
        self.inner.set_len(n)
    }
    fn write(&self, o: u64, b: &[u8]) -> io::Result<()> {
        if self.write_fault && self.fail.swap(false, Ordering::SeqCst) {
            return Err(io::Error::other("actual commit write fault"));
        }
        self.inner.write(o, b)
    }
    fn sync_data(&self, eventual: bool) -> io::Result<()> {
        if !self.write_fault && self.fail.swap(false, Ordering::SeqCst) {
            return Err(io::Error::other("actual immediate commit fsync fault"));
        }
        self.inner.sync_data(eventual)
    }
}
#[tokio::test]
async fn real_commit_write_and_fsync_failure_is_uncertain_until_exclusive_reopen() {
    for write_fault in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("io");
        let fail_flag = Arc::new(AtomicBool::new(false));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let db = Database::builder()
            .set_cache_size(64 * 1024 * 1024)
            .create_with_backend(CommitFault {
                inner: FileBackend::new(file).unwrap(),
                fail: fail_flag.clone(),
                write_fault,
            })
            .unwrap();
        let index = super::super::block_validation::Index::create(
            dir.path(),
            config().ownership.open_limits().max_journal_index_bytes(),
        )
        .unwrap();
        super::super::block_validation::initialize(&db, StoreOrigin::Fresh, config(), index)
            .unwrap();
        let s = RedbStore {
            db: Arc::new(db),
            generation_mode: crate::generation::GenerationMode::BlockDurability,
            entity_incarnation: Arc::new(()),
            ownership_runtime: Some(Arc::new(Runtime::new(config().ownership))),
            block_config: Some(config()),
        };
        initialize(&s).await;
        seed_air(&s).await;
        let before = rows(&s);
        let r = joint(&s, 3).await.0;
        let query = r.query;
        let result = s
            .block_write_hooks(r.clone(), move |point| {
                if point == Cutpoint::BeforeCommit {
                    fail_flag.store(true, Ordering::SeqCst);
                }
                Ok(())
            })
            .await;
        assert!(
            matches!(result,Err(BlockDurabilityError::CommitUncertain{query:q,..}) if q==query)
        );
        drop(s);
        let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
        let prior = s.load_block_receipt(query).await.unwrap();
        let retry = if prior.is_some() {
            r.clone()
        } else {
            assert_eq!(rows(&s), before);
            joint(&s, 3).await.0
        };
        assert_eq!(retry.q, r.q);
        assert_eq!(retry.query, query);
        let out = s.block_write(retry.clone()).await.unwrap();
        assert_eq!(
            out.disposition(),
            if prior.is_some() {
                OwnershipCommitDisposition::Replayed
            } else {
                OwnershipCommitDisposition::Committed
            }
        );
        let after = rows(&s);
        assert_eq!(s.block_write(retry).await.unwrap().receipt(), out.receipt());
        assert_eq!(rows(&s), after);
    }
}
#[tokio::test]
async fn cancellation_retains_shared_worker_permit_and_busy_starts_no_worker() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = setup(&dir.path().join("cancel"), dir.path()).await;
    let r = joint(&s, 3).await.0;
    let query = r.query;
    let retry = r.clone();
    let mut c = config();
    c.ownership = crate::OwnershipConfig::new(
        c.capacity.ownership,
        crate::OwnershipOpenLimits::new(
            nz(100_000),
            nz(512 * 1024 * 1024),
            nz(16 * 1024 * 1024),
            std::num::NonZeroU16::new(1).unwrap(),
        )
        .unwrap(),
    );
    s.ownership_runtime = Some(Arc::new(Runtime::new(c.ownership)));
    let s = Arc::new(s);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = sync_channel(1);
    let worker = s.clone();
    let mut entered_tx = Some(entered_tx);
    let caller = tokio::spawn(async move {
        worker
            .block_write_hooks(r, move |point| {
                if point == Cutpoint::BeforePlan {
                    entered_tx.take().unwrap().send(()).ok();
                    release_rx.recv().unwrap();
                }
                Ok(())
            })
            .await
    });
    entered_rx.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert_eq!(
        s.load_block_receipt(query).await.unwrap_err(),
        BlockDurabilityError::Busy
    );
    assert_eq!(
        s.load_owned_player(player(), generation())
            .await
            .unwrap_err(),
        OwnershipError::Busy
    );
    assert_eq!(
        s.block_write(retry.clone()).await.unwrap_err(),
        BlockDurabilityError::Busy
    );
    release_tx.send(()).unwrap();
    crate::ownership::tests::wait_for_permit(s.ownership_runtime.as_ref().unwrap()).await;
    let receipt = s.load_block_receipt(query).await.unwrap().unwrap();
    let out = s.block_write(retry).await.unwrap();
    assert_eq!(out.receipt(), &receipt);
}
/// Rewrites real bootstrap/seed evidence into a consistent revision-MAX history.
/// The reopen validator, not this helper, decides whether that history is admissible.
#[allow(clippy::too_many_lines)] // One transaction rewrites all coupled evidence rows.
async fn force_max(s: &RedbStore) {
    let max = crate::OwnershipStamp::checked(1, u64::MAX).unwrap();
    let below = crate::OwnershipStamp::checked(1, u64::MAX - 1).unwrap();
    // The genuine seed images, now claiming a Live(MAX-1) original for key(0).
    let mut fresh = s
        .load_owned_blocks(spec(false, vec![key(0), key(1)]))
        .await
        .unwrap();
    assert!(fresh.columns[0].stamp.is_live());
    let previous = crate::codec::encode_chunk_record(
        &ChunkRecord::try_new(
            SchemaVersion::new(54),
            ferrumc_world::Chunk::new(key(0).pos()),
        )
        .unwrap(),
    )
    .unwrap();
    fresh.columns[0].stamp = below;
    fresh.columns[0].pair = crate::block_durability::Pair {
        full: Some(Arc::from(previous)),
        overlay: None,
    };
    fresh.columns[1].stamp = crate::OwnershipStamp::FRESH;
    fresh.columns[1].pair = crate::block_durability::Pair::default();
    let images = [key(0), key(1)]
        .into_iter()
        .map(|k| {
            ChunkImageChange::new(
                k,
                ChunkImageAction::Replace(
                    ChunkImage::new(
                        Some(
                            ChunkRecord::try_new(
                                SchemaVersion::new(55),
                                ferrumc_world::Chunk::new(k.pos()),
                            )
                            .unwrap(),
                        ),
                        None,
                    )
                    .unwrap(),
                ),
            )
        })
        .collect();
    let seed = ChunkImageCasRequest::new(id(2), fresh, images).unwrap().0;
    let q = raw::q(&seed.q).unwrap();
    let (inactive, chunks) = engine::after_stamps(&q).unwrap();
    assert_eq!(chunks[0].1, max);
    let receipt = raw::encode_receipt(&seed.q, inactive, &chunks, None, 0);
    let txn = s.db.begin_write().unwrap();
    {
        let mut receipts = txn.open_table(ownership::RECEIPT).unwrap();
        let old = receipts
            .get(id(2).to_bytes().as_slice())
            .unwrap()
            .unwrap()
            .value()
            .len() as u64;
        receipts
            .insert(id(2).to_bytes().as_slice(), receipt.as_slice())
            .unwrap();
        let k = id(1).to_bytes();
        let mut v1 = crate::ownership::raw::receipt(
            &k,
            receipts.get(k.as_slice()).unwrap().unwrap().value(),
        )
        .unwrap();
        v1.before_player = below;
        v1.after_player = max;
        receipts
            .insert(
                k.as_slice(),
                crate::ownership::raw::encode_receipt(&v1).as_slice(),
            )
            .unwrap();
        let mut meta = txn.open_table(META_TABLE).unwrap();
        let bytes = meta.get(RECEIPT_BYTES).unwrap().unwrap().value();
        meta.insert(RECEIPT_BYTES, bytes - old + receipt.len() as u64)
            .unwrap();
    }
    txn.open_table(HEADER)
        .unwrap()
        .insert(
            crate::codec::chunk_key_bytes(key(0)).as_slice(),
            raw::encode_header(raw::Header {
                stamp: max,
                id: id(2),
            })
            .as_slice(),
        )
        .unwrap();
    {
        let mut owners = txn.open_table(ownership::PLAYER_OWNER).unwrap();
        let k = crate::codec::player_key_bytes(player().id());
        let mut h = crate::ownership::raw::player_header(
            owners.get(k.as_slice()).unwrap().unwrap().value(),
        )
        .unwrap();
        h.stamp = max;
        owners
            .insert(
                k.as_slice(),
                crate::ownership::raw::encode_player_header(h).as_slice(),
            )
            .unwrap();
    }
    txn.commit().unwrap();
}
#[allow(clippy::too_many_lines)] // Keep one complete matrix case and its assertions together.
#[tokio::test]
async fn persisted_player_and_chunk_max_noop_preserves_pointers_and_real_change_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("max");
    let s = setup(&path, dir.path()).await;
    force_max(&s).await;
    drop(s);
    let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    let max = crate::OwnershipStamp::checked(1, u64::MAX).unwrap();
    let snap = s.load_owned_blocks(spec(true, vec![key(0)])).await.unwrap();
    assert_eq!(snap.player_stamp(), max);
    assert_eq!(snap.columns()[0].stamp(), max);
    let keep = || vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)];
    let before = rows(&s);
    let out = s
        .commit_player_blocks(
            PlayerBlockRequest::new(id(3), snap.clone(), p(11), keep(), vec![pos(0, 64)], vec![])
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out.receipt().after_player_stamp(), max);
    assert_eq!(out.receipt().chunks()[0].1, max);
    let after = rows(&s);
    // Only the receipt row and the shared receipt count/bytes changed: pointers stay at id1/id2.
    for table in [
        "ferrumc:player",
        "ferrumc:player_ownership",
        "ferrumc:chunk",
        "ferrumc:chunk_ownership",
        "ferrumc:scoped_mutation_log",
    ] {
        assert_eq!(after[table], before[table], "{table}");
    }
    let meta = |rows: &Rows, k: &str| {
        rows["ferrumc:meta"]
            .iter()
            .find(|(key, _)| key == k.as_bytes())
            .map(|(_, v)| u64::from_be_bytes(v.as_slice().try_into().unwrap()))
            .unwrap()
    };
    assert_eq!(
        meta(&after, RECEIPT_BYTES),
        meta(&before, RECEIPT_BYTES) + 16 + out.receipt().encoded_bytes().len() as u64
    );
    assert_eq!(
        meta(&after, ownership::RECEIPT_COUNT),
        meta(&before, ownership::RECEIPT_COUNT) + 1
    );
    let mut changed = snap.columns()[0].chunk().clone();
    changed
        .set_block(pos(1, 64), ferrumc_world::BlockStateId::new(5))
        .unwrap();
    let chunk_change = PlayerBlockRequest::new(
        id(5),
        snap.clone(),
        p(11),
        vec![ChunkImageChange::new(
            key(0),
            ChunkImageAction::Replace(
                ChunkImage::new(
                    Some(ChunkRecord::try_new(SchemaVersion::new(55), changed).unwrap()),
                    None,
                )
                .unwrap(),
            ),
        )],
        vec![],
        vec![effect(pos(1, 64), 0, 5, None, None)],
    )
    .unwrap();
    for (request, resource) in [
        (
            PlayerBlockRequest::new(id(4), snap, p(12), keep(), vec![pos(0, 64)], vec![]).unwrap(),
            "Player",
        ),
        (chunk_change, "Chunk"),
    ] {
        let query = request.receipt_query();
        assert!(
            matches!(s.commit_player_blocks(request).await, Err(BlockDurabilityError::RevisionExhausted { resource: r, .. }) if r == resource)
        );
        assert_eq!(rows(&s), after);
        assert!(s.load_block_receipt(query).await.unwrap().is_none());
    }
    drop(s);
    let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    assert_eq!(rows(&s), after);
}
/// Removal after a refused fresh initialization needs this call's inode and proof of an
/// empty catalog; anything else is left in place with the original typed error.
#[test]
fn abandoned_fresh_file_is_removed_only_with_identity_and_empty_catalog() {
    let dir = tempfile::tempdir().unwrap();
    let created = |name: &str| {
        let path = dir.path().join(name);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let identity = FileIdentity::of(&file.metadata().unwrap()).unwrap();
        (path, file, identity)
    };
    let error = || BlockDurabilityError::Backend("refused".into());
    // Table-free database through the open handle: removed.
    let (path, file, identity) = created("empty");
    let db = Database::builder().create_file(file).unwrap();
    assert_eq!(
        abandon_fresh(&path, Some(identity), Some(db), error()),
        error()
    );
    assert!(!path.exists());
    // Any table means possibly committed data: kept.
    let (path, file, identity) = created("table");
    let db = Database::builder().create_file(file).unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(HEADER).unwrap();
    txn.commit().unwrap();
    abandon_fresh(&path, Some(identity), Some(db), error());
    assert!(path.exists());
    // Unknown identity, or the path now naming another inode: kept.
    let (path, file, _) = created("unknown");
    let db = Database::builder().create_file(file).unwrap();
    abandon_fresh(&path, None, Some(db), error());
    assert!(path.exists());
    let (path, file, identity) = created("replaced");
    let db = Database::builder().create_file(file).unwrap();
    std::fs::rename(&path, dir.path().join("moved")).unwrap();
    std::fs::write(&path, b"foreign").unwrap();
    abandon_fresh(&path, Some(identity), Some(db), error());
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign");
    // Without a database handle only a zero-length file proves no content.
    let (path, file, identity) = created("zero");
    drop(file);
    abandon_fresh(&path, Some(identity), None, error());
    assert!(!path.exists());
    let (path, mut file, identity) = created("partial");
    std::io::Write::write_all(&mut file, b"partial header").unwrap();
    drop(file);
    abandon_fresh(&path, Some(identity), None, error());
    assert!(path.exists());
}
/// redb twin of Memory's forged-history read test: rows are rewritten under the open store,
/// so the runtime readers (not Reopen, which re-derives every receipt) must refuse.
#[tokio::test]
async fn reads_prove_touched_history_and_refuse_forged_or_inconsistent_history() {
    let mut cell = [0, 0, 0, 15, 0, 0, 0, 64, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0].to_vec();
    let mut forged_cell = cell.clone();
    cell.extend_from_slice(&100u32.to_be_bytes());
    forged_cell.extend_from_slice(&101u32.to_be_bytes());
    let splice = |bytes: &mut Vec<u8>| {
        let at = bytes
            .windows(cell.len())
            .position(|w| w == cell.as_slice())
            .unwrap();
        bytes[at..at + cell.len()].copy_from_slice(&forged_cell);
    };
    for consistent_forgery in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let s = setup(&dir.path().join("forged"), dir.path()).await;
        let r = joint(&s, 3).await.0;
        let query = r.query;
        s.block_write(r).await.unwrap();
        let txn = s.db.begin_write().unwrap();
        {
            let mut journal = txn.open_table(JOURNAL).unwrap();
            let (k, mut v) = journal
                .iter()
                .unwrap()
                .map(Result::unwrap)
                .map(|(k, v)| (k.value().to_vec(), v.value().to_vec()))
                .find(|(_, v)| v.windows(cell.len()).any(|w| w == cell.as_slice()))
                .unwrap();
            splice(&mut v);
            journal.insert(k.as_slice(), v.as_slice()).unwrap();
            if consistent_forgery {
                let mut receipts = txn.open_table(super::super::ownership::RECEIPT).unwrap();
                let key = id(3).to_bytes();
                let mut fcbr = receipts
                    .get(key.as_slice())
                    .unwrap()
                    .unwrap()
                    .value()
                    .to_vec();
                splice(&mut fcbr);
                let len = u32::from_be_bytes(fcbr[6..10].try_into().unwrap()) as usize;
                let digest = crate::block_durability::raw::digest(&fcbr[10..10 + len]);
                let end = fcbr.len();
                fcbr[end - 40..end - 8].copy_from_slice(&digest);
                receipts.insert(key.as_slice(), fcbr.as_slice()).unwrap();
            }
        }
        txn.commit().unwrap();
        let touched = s.load_owned_blocks(spec(false, vec![key(0)])).await;
        let other = s.load_owned_blocks(spec(false, vec![key(1)])).await;
        let player = s.load_owned_player(player(), generation()).await;
        let lookup = s.load_block_receipt(query).await;
        if consistent_forgery {
            assert!(
                matches!(touched, Err(BlockDurabilityError::IncompleteDelta { key: k }) if k == key(0)),
                "{touched:?}"
            );
            assert!(other.is_ok() && player.is_ok(), "{other:?} {player:?}");
            assert!(matches!(
                lookup,
                Err(BlockDurabilityError::IncompleteDelta { .. })
            ));
        } else {
            let mismatch = BlockDurabilityError::MalformedData {
                table: "FCBR",
                reason: "journal exact ordinal disagreement",
            };
            assert_eq!(touched.err(), Some(mismatch.clone()));
            assert_eq!(other.err(), Some(mismatch.clone()));
            assert_eq!(lookup.err(), Some(mismatch.clone()));
            assert!(
                matches!(&player, Err(OwnershipError::Block(e)) if **e == mismatch),
                "{player:?}"
            );
        }
        drop(s);
        // Reopen re-derives every receipt and refuses either forgery without repair.
        assert!(RedbStore::open_with_player_block_durability(
            dir.path().join("forged"),
            config(),
            dir.path()
        )
        .is_err());
    }
}
