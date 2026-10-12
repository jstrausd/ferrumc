//! Real unified-state publication, cancellation and bounded local admission.
use super::*;
use crate::block_test_support::*;
use crate::PlayerEntityOwnershipStore;
use std::sync::mpsc::sync_channel;

async fn setup() -> InMemoryStore {
    let s = InMemoryStore::with_player_block_durability(config()).unwrap();
    initialize(&s).await;
    seed_air(&s).await;
    s
}
fn rows(s: &InMemoryStore) -> String {
    format!("{:?}", s.ownership_state.read().unwrap())
}
fn failure() -> BlockDurabilityError {
    BlockDurabilityError::Backend("actual Memory hook".into())
}
#[tokio::test]
async fn preapply_error_retains_every_row_postapply_loss_retains_exact_receipt() {
    let s = setup().await;
    let before = rows(&s);
    let r = joint(&s, 3).await.0;
    let query = r.query;
    assert!(matches!(
        s.block_write_hooks(r.clone(), || Err(failure()), || {}, || Ok(()))
            .await,
        Err(BlockDurabilityError::Backend(_))
    ));
    assert_eq!(rows(&s), before);
    assert!(s.load_block_receipt(query).await.unwrap().is_none());
    assert!(
        matches!(s.block_write_hooks(r.clone(), || Ok(()), || {}, || Err(failure())).await, Err(BlockDurabilityError::ExecutionUncertain { query: q, .. }) if q == query)
    );
    let after = rows(&s);
    let receipt = s.load_block_receipt(query).await.unwrap().unwrap();
    assert_eq!(s.block_write(r).await.unwrap().receipt(), &receipt);
    assert_eq!(rows(&s), after);
}
#[tokio::test]
async fn real_reader_cannot_capture_midapply_player_and_chunk() {
    let s = Arc::new(setup().await);
    let r = joint(&*s, 3).await.0;
    let writer = s.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = sync_channel(1);
    let mut entered = Some(entered_tx);
    let task = tokio::spawn(async move {
        writer
            .block_write_hooks(
                r,
                || Ok(()),
                move || {
                    if let Some(tx) = entered.take() {
                        tx.send(()).unwrap();
                        release_rx.recv().unwrap();
                    }
                },
                || Ok(()),
            )
            .await
            .unwrap()
    });
    entered_rx.await.unwrap();
    assert!(s.ownership_state.try_read().is_err());
    let reader = s.clone();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let read_task = tokio::spawn(async move {
        started_tx.send(()).unwrap();
        reader
            .load_owned_blocks(spec(true, vec![key(0), key(1)]))
            .await
            .unwrap()
    });
    started_rx.await.unwrap();
    release_tx.send(()).unwrap();
    task.await.unwrap();
    let snap = read_task.await.unwrap();
    assert_eq!(snap.player_record().unwrap().data(), player_bytes(10));
    assert_eq!(
        snap.columns()[0]
            .chunk()
            .get_block(pos(15, 64))
            .unwrap()
            .as_u32(),
        100
    );
    assert_eq!(
        snap.columns()[1]
            .chunk()
            .get_block(pos(16, 64))
            .unwrap()
            .as_u32(),
        300
    );
}
#[tokio::test]
async fn cancelled_future_keeps_real_worker_and_shared_v1_permit() {
    let mut s = setup().await;
    let r = joint(&s, 3).await.0;
    let retry = r.clone();
    let query = r.query;
    let c = config();
    let owned = crate::OwnershipConfig::new(
        c.capacity.ownership,
        crate::OwnershipOpenLimits::new(
            nz(100_000),
            nz(512 * 1024 * 1024),
            nz(16 * 1024 * 1024),
            std::num::NonZeroU16::new(1).unwrap(),
        )
        .unwrap(),
    );
    s.ownership_runtime = Some(Arc::new(Runtime::new(owned)));
    let s = Arc::new(s);
    let worker = s.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = sync_channel(1);
    let task = tokio::spawn(async move {
        worker
            .block_write_hooks(
                r,
                move || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                },
                || {},
                || Ok(()),
            )
            .await
    });
    entered_rx.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
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
    assert_eq!(s.block_write(retry).await.unwrap().receipt(), &receipt);
}
/// Replaces the only occurrence of `from` with `to` (equal length).
fn splice(bytes: &mut [u8], from: &[u8], to: &[u8]) {
    let at: Vec<_> = bytes
        .windows(from.len())
        .enumerate()
        .filter(|(_, w)| *w == from)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(at.len(), 1);
    bytes[at[0]..at[0] + to.len()].copy_from_slice(to);
}
#[tokio::test]
async fn reads_prove_touched_history_and_refuse_forged_or_inconsistent_history() {
    // The joint effect pos(15, 64) air -> 100: position12 + old_state + new_state.
    let mut cell = [0, 0, 0, 15, 0, 0, 0, 64, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0].to_vec();
    let mut forged_cell = cell.clone();
    cell.extend_from_slice(&100u32.to_be_bytes());
    forged_cell.extend_from_slice(&101u32.to_be_bytes());
    let receipt_key = id(3).to_bytes().to_vec();
    let journal_key = |s: &InMemoryStore| {
        let state = s.ownership_state.read().unwrap();
        state
            .blocks
            .journal
            .iter()
            .find(|(_, v)| v.windows(cell.len()).any(|w| w == cell.as_slice()))
            .map(|(k, _)| k.clone())
            .unwrap()
    };
    for consistent_forgery in [true, false] {
        let s = setup().await;
        let r = joint(&s, 3).await.0;
        let query = r.query;
        s.block_write(r).await.unwrap();
        let journal = journal_key(&s);
        {
            let mut state = s.ownership_state.write().unwrap();
            splice(
                state.blocks.journal.get_mut(&journal).unwrap(),
                &cell,
                &forged_cell,
            );
            if consistent_forgery {
                // Same edit in the retained Q, with a recomputed query digest: every relation
                // holds, only column key(0)'s endpoint proof can see the forged chain.
                let fcbr = state.receipts.get_mut(&receipt_key).unwrap();
                splice(fcbr, &cell, &forged_cell);
                let len = u32::from_be_bytes(fcbr[6..10].try_into().unwrap()) as usize;
                let digest = crate::block_durability::raw::digest(&fcbr[10..10 + len]);
                let end = fcbr.len();
                fcbr[end - 40..end - 8].copy_from_slice(&digest);
            }
        }
        let touched = s.load_owned_blocks(spec(false, vec![key(0)])).await;
        let other = s.load_owned_blocks(spec(false, vec![key(1)])).await;
        let both = s.load_owned_blocks(spec(true, vec![key(0), key(1)])).await;
        let player = s.load_owned_player(player(), generation()).await;
        let lookup = s.load_block_receipt(query).await;
        if consistent_forgery {
            // Touched forged history refuses; the untouched column and Player rows still
            // agree with every retained relation. Lookup re-derives the whole receipt.
            assert!(
                matches!(touched, Err(BlockDurabilityError::IncompleteDelta { key: k }) if k == key(0)),
                "{touched:?}"
            );
            assert!(other.is_ok(), "{other:?}");
            assert!(player.is_ok(), "{player:?}");
            assert!(matches!(
                both,
                Err(BlockDurabilityError::IncompleteDelta { .. })
            ));
            assert!(matches!(
                lookup,
                Err(BlockDurabilityError::IncompleteDelta { .. })
            ));
        } else {
            // A journal row that disagrees with its FCBR breaks the receipt's relations,
            // which every reader touching that receipt checks.
            let journal_mismatch = |e: &BlockDurabilityError| {
                matches!(
                    e,
                    BlockDurabilityError::MalformedData {
                        table: "FCBR",
                        reason: "journal exact ordinal disagreement"
                    }
                )
            };
            assert!(
                journal_mismatch(touched.as_ref().unwrap_err()),
                "{touched:?}"
            );
            assert!(journal_mismatch(other.as_ref().unwrap_err()), "{other:?}");
            assert!(journal_mismatch(both.as_ref().unwrap_err()), "{both:?}");
            assert!(journal_mismatch(&lookup.unwrap_err()));
            assert!(
                matches!(&player, Err(crate::OwnershipError::Block(e)) if journal_mismatch(e)),
                "{player:?}"
            );
        }
    }
}
