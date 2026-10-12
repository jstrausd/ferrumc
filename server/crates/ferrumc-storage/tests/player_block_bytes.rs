//! Runtime validation bytes: a receipt shared by a union's columns is read, digested and parsed
//! once per call, and the 512 MiB cap refuses typed without a partial result or mutation.
//! Expectations come from the independent byte model below (INVARIANTS "Runtime validation
//! bytes") and the fixed FCBQ/FCBR layouts, never from Storage's reported requirement.
mod block_support;
use block_support::*;
use ferrumc_math::BlockPos;
use ferrumc_storage::*;
use ferrumc_world::{decode_block_entity, try_encode_block_entity, BlockStateId, Chunk};
use std::path::Path;
use std::sync::Arc;

const MIB: u64 = 1024 * 1024;
/// Per call: the generation identity row and the two counter reads.
const CALL: u64 = 35 + 128 + 256;
/// One column's own row reads, without its Full bytes: Full key, absent Overlay key, Header.
const READS: u64 = 16 + 16 + (16 + 31);

/// A distinct last receipt is read (16-byte key + row) and its FCBQ body digested once.
const fn receipt_once(r: u64, q: u64) -> u64 {
    16 + r + q
}
/// Encountered relations of an image-CAS receipt: identity, then per entry Header, Full, Overlay.
fn relations(fulls: &[u64]) -> u64 {
    35 + fulls.iter().map(|f| READS + f).sum::<u64>()
}
/// A column's own reads, its proved postimage decode (`before` is the decoded preimage) and
/// the canonical encodings of the view's entities, each charged its real length (`entities`,
/// from the World codec). Snapshot output copies are charged only to the output cap.
/// Round 6: formerly a flat 16 KiB per view entity.
const fn column(full: u64, before: u64, entities: u64) -> u64 {
    READS + full + before + full + entities
}
/// The World codec's encoded length of every entity of a view: the oracle for its charge.
fn entities(chunk: &Chunk) -> u64 {
    chunk
        .block_entities()
        .map(|(_, e)| {
            let mut b = Vec::new();
            try_encode_block_entity(e, &mut b).unwrap();
            b.len() as u64
        })
        .sum()
}

/// Residual of the hard caps: eight columns whose last receipts are eight distinct maximal
/// rows (Q = 32 MiB, FCBR = Q + 143) need at least their reads and digests, which alone
/// exceed 512 MiB. Such a union refuses with `ValidationCapacity("bytes")`; the contract's
/// "maximum shape does not guarantee admission" covers it.
const _: () = assert!(8 * receipt_once(32 * MIB + 143, 32 * MIB) > 512 * MIB);

fn open(durable: bool, path: &Path, scratch: &Path, caps: Caps) -> Arc<dyn Store> {
    let c = config_with(caps).unwrap();
    if durable {
        Arc::new(RedbStore::open_with_player_block_durability(path, c, scratch).unwrap())
    } else {
        Arc::new(InMemoryStore::with_player_block_durability(c).unwrap())
    }
}
fn bytes(n: u64) -> Caps {
    Caps {
        bytes: n,
        ..Caps::default()
    }
}
/// A column with `sections` sections of unshared states and `chests` chests.
fn noisy(k: ChunkKey, salt: u32, sections: i32, chests: u16) -> Chunk {
    let mut c = Chunk::new(k.pos());
    let o = k.pos().origin_block(-64);
    for y in -64..(-64 + 16 * sections) {
        for z in 0..16 {
            for x in 0..16 {
                let n = ((x * 7 + y * 13 + z * 31) as u32)
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(salt)
                    % 20_000
                    + 1;
                c.set_block(BlockPos::new(o.x() + x, y, o.z() + z), BlockStateId::new(n))
                    .unwrap();
            }
        }
    }
    for i in 0..i32::from(chests) {
        let at = BlockPos::new(o.x() + i % 16, 100 + i / 256, o.z() + i / 16 % 16);
        c.set_block_entity(
            at,
            decode_block_entity(&chest_bytes((i % 60 + 1) as u8)).unwrap(),
        )
        .unwrap();
    }
    c
}
fn image(k: ChunkKey, c: Chunk) -> ChunkImageChange {
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
/// One image-CAS writes `keys`; afterwards they share its receipt as their last receipt.
async fn write(
    s: &dyn Store,
    n: u8,
    keys: &[ChunkKey],
    salt: u32,
    sections: i32,
    chests: u16,
) -> BlockReceipt {
    let snap = s
        .load_owned_blocks(spec(false, keys.to_vec()))
        .await
        .unwrap();
    let images = keys
        .iter()
        .enumerate()
        .map(|(i, k)| image(*k, noisy(*k, salt + i as u32, sections, chests)))
        .collect();
    s.compare_exchange_chunk_images(ChunkImageCasRequest::new(id(n), snap, images).unwrap())
        .await
        .unwrap()
        .receipt()
        .clone()
}
/// Exact FCBR row and FCBQ body lengths, checked against the fixed layouts: FCBR is Q + 71
/// plus 9 per column; an image-CAS Q is 98 + 46 per column plus its pair bytes.
fn sizes(receipt: &BlockReceipt, columns: u64, pairs: u64) -> (u64, u64) {
    let r = receipt.encoded_bytes().len() as u64;
    let q = receipt.request_bytes().len() as u64;
    assert_eq!(r, q + 71 + 9 * columns);
    assert_eq!(q, 98 + 46 * columns + pairs);
    (r, q)
}
fn fulls(snap: &OwnedBlockSnapshot) -> Vec<u64> {
    snap.columns()
        .iter()
        .map(|c| c.full_bytes().unwrap().len() as u64)
        .collect()
}
fn eight() -> Vec<ChunkKey> {
    (0..8).map(key).collect()
}
fn distinct() -> Vec<ChunkKey> {
    (0..8).map(|g| key(g * 8)).collect()
}

const SHARED_SECTIONS: i32 = 24;
const SHARED_CHESTS: u16 = 64;
async fn shared(s: &dyn Store) -> BlockReceipt {
    initialize(s).await;
    write(s, 10, &eight(), 0, SHARED_SECTIONS, SHARED_CHESTS).await
}
/// N/N+1 for one receipt shared by an 8-column union. Re-reading and digesting it per column
/// would alone exceed N, so only a once-per-call receipt admits the union at N.
/// Round 6: the exact boundary runs on redb, which reopens under the cap. Memory fixes its
/// cap at construction, so its set-up write would run under N too, and that write needs more
/// than this snapshot: per column it parses, decodes, encodes, copies, frames and retains the
/// image (6 A) where the snapshot reads, digests, relates and decodes it (5 A) plus encodes
/// the view's entities (sum L < A). Only the former flat 16 KiB per view entity lifted N above
/// the write. Memory checks this union's admission here and the exact once-per-call boundary
/// in `half_shared_union_admits_its_exact_byte_need_and_refuses_one_less`.
#[tokio::test]
async fn shared_receipt_union_admits_its_exact_byte_need_and_refuses_one_less() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let sizing = open(
            durable,
            &dir.path().join("size"),
            dir.path(),
            Caps::default(),
        );
        let receipt = shared(&*sizing).await;
        let reference = sizing
            .load_owned_blocks(spec(false, eight()))
            .await
            .unwrap();
        let f = fulls(&reference);
        let (r, q) = sizes(&receipt, 8, f.iter().sum());
        let need = CALL
            + receipt_once(r, q)
            + relations(&f)
            + f.iter()
                .zip(reference.columns())
                .map(|(f, c)| column(*f, 0, entities(c.chunk())))
                .sum::<u64>();
        assert!(8 * receipt_once(r, q) > need, "durable={durable}");
        if !durable {
            // Memory: `reference`, taken above at the default cap, is this union's admission.
            continue;
        }
        for (cap, admitted) in [(need, true), (need - 1, false)] {
            let path = dir.path().join(format!("cap-{cap}"));
            shared(&*open(true, &path, dir.path(), Caps::default())).await;
            let s = open(true, &path, dir.path(), bytes(cap));
            let out = s.load_owned_blocks(spec(false, eight())).await;
            if admitted {
                let snap = out.unwrap();
                assert_eq!(fulls(&snap), f);
                for (c, expected) in snap.columns().iter().zip(reference.columns()) {
                    assert_eq!(c.stamp(), expected.stamp());
                    assert_eq!(c.chunk(), expected.chunk());
                }
            } else {
                assert_eq!(
                    out.err(),
                    Some(BlockDurabilityError::ValidationCapacity("bytes")),
                    "durable={durable}"
                );
            }
        }
    }
}

/// Columns 0..4 share one image-CAS receipt; columns 4..8 each have their own.
async fn half_shared(s: &dyn Store) -> Vec<BlockReceipt> {
    initialize(s).await;
    let keys = eight();
    let mut out = vec![write(s, 10, &keys[..4], 0, SHARED_SECTIONS, SHARED_CHESTS).await];
    for i in 4..8 {
        let receipt = write(
            s,
            10 + i as u8,
            &keys[i..=i],
            i as u32,
            SHARED_SECTIONS,
            SHARED_CHESTS,
        )
        .await;
        out.push(receipt);
    }
    out
}
/// N/N+1 for a union where four columns share one receipt and four have their own, on Memory
/// and redb. Each set-up write needs less than this snapshot, so Memory's construction-time
/// cap checks the exact boundary too. Reading the shared receipt once per column would add
/// three more reads and digests, far above N - 1.
#[tokio::test]
async fn half_shared_union_admits_its_exact_byte_need_and_refuses_one_less() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let sizing = open(
            durable,
            &dir.path().join("size"),
            dir.path(),
            Caps::default(),
        );
        let receipts = half_shared(&*sizing).await;
        let reference = sizing
            .load_owned_blocks(spec(false, eight()))
            .await
            .unwrap();
        let f = fulls(&reference);
        let (r, q) = sizes(&receipts[0], 4, f[..4].iter().sum());
        let mut need = CALL + receipt_once(r, q) + relations(&f[..4]);
        for (i, receipt) in (4..8).zip(&receipts[1..]) {
            let (r, q) = sizes(receipt, 1, f[i]);
            need += receipt_once(r, q) + relations(&f[i..=i]);
        }
        need += f
            .iter()
            .zip(reference.columns())
            .map(|(f, c)| column(*f, 0, entities(c.chunk())))
            .sum::<u64>();
        for (cap, admitted) in [(need, true), (need - 1, false)] {
            let path = dir.path().join(format!("cap-{cap}"));
            let s = if durable {
                half_shared(&*open(true, &path, dir.path(), Caps::default())).await;
                open(true, &path, dir.path(), bytes(cap))
            } else {
                let s = open(false, &path, dir.path(), bytes(cap));
                half_shared(&*s).await;
                s
            };
            let out = s.load_owned_blocks(spec(false, eight())).await;
            if admitted {
                let snap = out.unwrap();
                assert_eq!(fulls(&snap), f);
                for (c, expected) in snap.columns().iter().zip(reference.columns()) {
                    assert_eq!(c.stamp(), expected.stamp());
                    assert_eq!(c.chunk(), expected.chunk());
                }
            } else {
                assert_eq!(
                    out.err(),
                    Some(BlockDurabilityError::ValidationCapacity("bytes")),
                    "durable={durable}"
                );
            }
        }
    }
}

/// Second 8-column image-CAS over full images: its Q carries both pairs per column, near the
/// 32 MiB request bound, and the union's snapshot output stays under the 32 MiB output cap.
const NEAR_SECTIONS: i32 = 24;
const NEAR_CHESTS: u16 = 900;
/// A union whose eight columns share one near-32 MiB receipt is admitted at the hard default
/// 512 MiB cap, also after reopen. Round 4 charged that receipt per column and refused it.
#[tokio::test]
async fn shared_near_cap_receipt_union_is_admitted_at_the_hard_default_byte_cap() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("near");
        let s = open(durable, &path, dir.path(), Caps::default());
        initialize(&*s).await;
        write(&*s, 10, &eight(), 0, NEAR_SECTIONS, NEAR_CHESTS).await;
        let before = fulls(&s.load_owned_blocks(spec(false, eight())).await.unwrap());
        let receipt = write(&*s, 11, &eight(), 100, NEAR_SECTIONS, NEAR_CHESTS).await;
        let check = |s: Arc<dyn Store>| {
            let receipt = receipt.clone();
            let before = before.clone();
            async move {
                let snap = s.load_owned_blocks(spec(false, eight())).await.unwrap();
                let f = fulls(&snap);
                let (r, q) = sizes(
                    &receipt,
                    8,
                    before.iter().sum::<u64>() + f.iter().sum::<u64>(),
                );
                assert!(q > 28 * MIB && q <= MAX_BLOCK_REQUEST_BYTES as u64, "{q}");
                let need = CALL
                    + receipt_once(r, q)
                    + relations(&f)
                    + f.iter()
                        .zip(&before)
                        .zip(snap.columns())
                        .map(|((f, b), c)| column(*f, *b, entities(c.chunk())))
                        .sum::<u64>();
                assert!(need <= 512 * MIB, "{need}");
                for c in snap.columns() {
                    assert_eq!(c.stamp().revision().map(OwnershipRevision::get), Some(2));
                }
            }
        };
        check(s.clone()).await;
        if durable {
            drop(s);
            check(open(true, &path, dir.path(), Caps::default())).await;
        }
    }
}

const DISTINCT_SECTIONS: i32 = 4;
/// Eight 8-column image-CAS writes; columns `8g` then have eight distinct last receipts.
async fn distinct_receipts(s: &dyn Store) -> Vec<BlockReceipt> {
    initialize(s).await;
    let mut out = Vec::new();
    for g in 0..8 {
        let keys: Vec<_> = (0..8).map(|i| key(g * 8 + i)).collect();
        out.push(write(s, 10 + g as u8, &keys, g as u32 * 8, DISTINCT_SECTIONS, 0).await);
    }
    out
}
/// Gameplay commit over the distinct union, one changed cell per column.
fn union_commit(snap: OwnedBlockSnapshot) -> BlockMutationRequest {
    let (mut images, mut effects) = (Vec::new(), Vec::new());
    for c in snap.columns() {
        let mut chunk = c.chunk().clone();
        let at = BlockPos::new(c.key().pos().x() * 16 + 3, 300, -16);
        chunk.set_block(at, BlockStateId::new(7)).unwrap();
        images.push(image(c.key(), chunk));
        effects.push(effect(at, 0, 7, None, None));
    }
    BlockMutationRequest::new(id(30), snap, images, vec![], effects).unwrap()
}
/// Distinct receipts are each read and digested by the union: N/N+1 from the model, and at
/// exactly N a commit over the same union refuses typed with no partial result or mutation.
#[tokio::test]
async fn distinct_receipts_refuse_typed_at_the_byte_cap_without_mutation() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let sizing = open(
            durable,
            &dir.path().join("size"),
            dir.path(),
            Caps::default(),
        );
        let receipts = distinct_receipts(&*sizing).await;
        let mut need = CALL;
        for (g, receipt) in (0..8).zip(&receipts) {
            let keys: Vec<_> = (0..8).map(|i| key(g * 8 + i)).collect();
            let f = fulls(&sizing.load_owned_blocks(spec(false, keys)).await.unwrap());
            let (r, q) = sizes(receipt, 8, f.iter().sum());
            need += receipt_once(r, q) + relations(&f) + column(f[0], 0, 0);
        }
        let reference = sizing
            .load_owned_blocks(spec(false, distinct()))
            .await
            .unwrap();
        let prepared = |cap: u64| {
            let path = dir.path().join(format!("cap-{cap}"));
            let scratch = dir.path().to_path_buf();
            async move {
                if durable {
                    distinct_receipts(&*open(true, &path, &scratch, Caps::default())).await;
                    open(true, &path, &scratch, bytes(cap))
                } else {
                    let s = open(false, &path, &scratch, bytes(cap));
                    distinct_receipts(&*s).await;
                    s
                }
            }
        };
        let s = prepared(need - 1).await;
        assert_eq!(
            s.load_owned_blocks(spec(false, distinct())).await.err(),
            Some(BlockDurabilityError::ValidationCapacity("bytes")),
            "durable={durable}"
        );
        let s = prepared(need).await;
        let snap = s.load_owned_blocks(spec(false, distinct())).await.unwrap();
        let request = union_commit(snap);
        let query = request.receipt_query();
        assert_eq!(
            s.commit_blocks(request).await.err(),
            Some(BlockDurabilityError::ValidationCapacity("bytes")),
            "durable={durable}"
        );
        let after = s.load_owned_blocks(spec(false, distinct())).await.unwrap();
        for (a, b) in after.columns().iter().zip(reference.columns()) {
            assert_eq!(a.stamp(), b.stamp());
            assert_eq!(a.full_bytes(), b.full_bytes());
        }
        assert!(s.load_block_receipt(query).await.unwrap().is_none());
    }
}
