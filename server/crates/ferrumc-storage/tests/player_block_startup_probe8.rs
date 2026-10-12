//! Independent Public API reopen amplifier probe; no private storage engine/codec calls.
mod block_support;
use block_support::*;
use ferrumc_math::BlockPos;
use ferrumc_storage::*;
use ferrumc_world::{BlockStateId, Chunk};

fn noisy(k: ChunkKey) -> Chunk {
    let mut c = Chunk::new(k.pos());
    let o = k.pos().origin_block(-64);
    for y in -64..320 {
        for z in 0..16 {
            for x in 0..16 {
                let n = ((x * 7 + y * 13 + z * 31) as u32).wrapping_mul(2_654_435_761) % 20_000 + 1;
                c.set_block(BlockPos::new(o.x() + x, y, o.z() + z), BlockStateId::new(n))
                    .unwrap();
            }
        }
    }
    c
}
fn replace(k: ChunkKey, c: Chunk) -> ChunkImageChange {
    ChunkImageChange::new(
        k,
        ChunkImageAction::Replace(
            ChunkImage::new(
                Some(ChunkRecord::try_new(SchemaVersion::new(55), c).unwrap()),
                None,
            )
            .unwrap(),
        ),
    )
}
#[tokio::test]
async fn probe8_valid_commit_reopens_under_its_original_one_gib_allowance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scan.redb");
    let scratch = dir.path().join("scratch");
    std::fs::create_dir(&scratch).unwrap();
    let low = 1024 * 1024 * 1024;
    let cfg = config_with(Caps {
        relation_bytes: low,
        ..Caps::default()
    })
    .unwrap();
    let s = RedbStore::open_with_player_block_durability(&path, cfg, &scratch).unwrap();
    initialize(&s).await;
    // Three complete images, one operation; journal effects touch only column zero. Exact union
    // still covers the other two through raw-equal Replace proposals.
    let ks = vec![key(0), key(1), key(2)];
    let snap = s.load_owned_blocks(spec(false, ks.clone())).await.unwrap();
    s.compare_exchange_chunk_images(
        ChunkImageCasRequest::new(
            id(80),
            snap,
            ks.iter().map(|k| replace(*k, noisy(*k))).collect(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let snap = s.load_owned_blocks(spec(false, ks.clone())).await.unwrap();
    let f: u64 = snap
        .columns()
        .iter()
        .map(|c| c.full_bytes().unwrap().len() as u64)
        .sum();
    let at = BlockPos::new(1, 300, -15);
    let old = snap.columns()[0].chunk().get_block(at).unwrap().as_u32();
    let new = if old == 1 { 2 } else { 1 };
    let effects = (0..1024)
        .map(|i| {
            if i % 2 == 0 {
                effect(at, old, new, None, None)
            } else {
                effect(at, new, old, None, None)
            }
        })
        .collect();
    let req = BlockMutationRequest::new(
        id(81),
        snap.clone(),
        snap.columns()
            .iter()
            .map(|c| replace(c.key(), c.chunk().clone()))
            .collect(),
        vec![],
        effects,
    )
    .unwrap();
    let query = req.receipt_query();
    let out = s.commit_blocks(req.clone()).await.unwrap();
    let q = out.receipt().request_bytes().len() as u64;
    let r = out.receipt().encoded_bytes().len() as u64;
    assert_eq!(q, 98 + 46 * 3 + 2 * f + 64 * 1024);
    assert_eq!(r, q + 71 + 9 * 3);
    let journal_cost = 1024 * (16 + r + q);
    println!("probe8 Q={q} R={r} full_sum={f} journal_receipt_read_hash_lower_bound={journal_cost} same_limit={low}");
    assert!(journal_cost > 2 * low);
    assert_eq!(
        s.commit_blocks(req.clone()).await.unwrap().disposition(),
        OwnershipCommitDisposition::Replayed
    );
    assert_eq!(
        s.load_block_receipt(query)
            .await
            .unwrap()
            .unwrap()
            .encoded_bytes(),
        out.receipt().encoded_bytes()
    );
    drop(s);
    let before = raw17(&path);
    assert_eq!(before.len(), 17);
    let s = RedbStore::open_with_player_block_durability(&path, cfg, &scratch).unwrap();
    assert_eq!(std::fs::read_dir(&scratch).unwrap().count(), 0);
    assert_eq!(
        s.load_block_receipt(query)
            .await
            .unwrap()
            .unwrap()
            .encoded_bytes(),
        out.receipt().encoded_bytes()
    );
    assert_eq!(
        s.commit_blocks(req).await.unwrap().disposition(),
        OwnershipCommitDisposition::Replayed
    );
    let now = s.load_owned_blocks(spec(false, ks)).await.unwrap();
    for (a, b) in now.columns().iter().zip(snap.columns()) {
        assert_eq!(a.chunk(), b.chunk());
        assert_eq!(a.full_bytes(), b.full_bytes());
    }
    drop(s);
    assert_eq!(raw17(&path), before);
    assert_eq!(std::fs::read_dir(&scratch).unwrap().count(), 0);
}
