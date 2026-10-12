//! Block entities are charged their real canonical encoding length before each encoding; the
//! 16 KiB entity bound stays a form bound, not a flat per-entity charge. Expectations come
//! from the FCBQ/FCBR/FCBL layouts and the World codec's own encoded lengths, never from
//! Storage's reported requirement.
mod block_support;
use block_support::*;
use ferrumc_items::{
    ComponentPatch, ComponentValue, ItemId, ItemStack, OpaqueComponent, MAX_COMPONENT_BYTES,
};
use ferrumc_math::BlockPos;
use ferrumc_storage::*;
use ferrumc_world::{
    decode_block_entity, try_encode_block_entity, BlockEntity, BlockStateId, ChestInventory, Chunk,
    MAX_BLOCK_ENTITY_PAYLOAD_LEN,
};
use std::path::Path;
use std::sync::Arc;

/// Per call: the generation identity row and the two counter reads.
const CALL: u64 = 35 + 128 + 256;
/// One column's own row reads, without its Full bytes: Full key, absent Overlay key, Header.
const READS: u64 = 16 + 16 + (16 + 31);

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
/// The oracle for entity charges: the World codec's encoded length of every entity of `chunk`.
fn encoded(chunk: &Chunk) -> u64 {
    chunk
        .block_entities()
        .map(|(_, e)| {
            let mut b = Vec::new();
            try_encode_block_entity(e, &mut b).unwrap();
            b.len() as u64
        })
        .sum()
}
/// A column holding `n` copies of `entity` at y >= 100.
fn stocked(k: ChunkKey, n: u16, entity: &BlockEntity) -> Chunk {
    let mut c = Chunk::new(k.pos());
    let o = k.pos().origin_block(-64);
    for i in 0..i32::from(n) {
        let at = BlockPos::new(o.x() + i % 16, 100 + i / 256, o.z() + i / 16 % 16);
        c.set_block_entity(at, entity.clone()).unwrap();
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
fn keys(n: i32) -> Vec<ChunkKey> {
    (0..n).map(key).collect()
}
async fn image_cas(s: &dyn Store, n: u8, keys: &[ChunkKey], chunk: impl Fn(ChunkKey) -> Chunk) {
    let snap = s
        .load_owned_blocks(spec(false, keys.to_vec()))
        .await
        .unwrap();
    let images = keys.iter().map(|k| image(*k, chunk(*k))).collect();
    s.compare_exchange_chunk_images(ChunkImageCasRequest::new(id(n), snap, images).unwrap())
        .await
        .unwrap();
}
fn revisions(snap: &OwnedBlockSnapshot) -> Vec<Option<u64>> {
    snap.columns()
        .iter()
        .map(|c| c.stamp().revision().map(OwnershipRevision::get))
        .collect()
}

// ---- Root-review round-2 probe 7, now a regression test: realistic entity-dense unions at the
// hard default caps (512 MiB bytes, 16,777,216 cells, 32 MiB snapshot output) are admitted.

/// Gameplay over a snapshot: one air->state cell per column (no entity touched).
fn gameplay(snap: OwnedBlockSnapshot, n: u8, state: u32) -> BlockMutationRequest {
    let (mut images, mut effects) = (Vec::new(), Vec::new());
    for c in snap.columns() {
        let mut chunk = c.chunk().clone();
        let at = BlockPos::new(c.key().pos().x() * 16 + 5, 300, -16);
        let old = chunk.get_block(at).unwrap().as_u32();
        chunk.set_block(at, BlockStateId::new(state)).unwrap();
        images.push(image(c.key(), chunk));
        effects.push(effect(at, old, state, None, None));
    }
    BlockMutationRequest::new(id(n), snap, images, vec![], effects).unwrap()
}
/// Image-CAS of `n` empty chests per column, then a first gameplay over the whole union, so
/// every column's last receipt is a gameplay receipt whose proof encodes both entity sides.
async fn dense(s: &dyn Store, columns: i32, n: u16) {
    initialize(s).await;
    let empty = BlockEntity::Chest(ChestInventory::new());
    image_cas(s, 10, &keys(columns), |k| stocked(k, n, &empty)).await;
    let snap = s
        .load_owned_blocks(spec(false, keys(columns)))
        .await
        .unwrap();
    s.commit_blocks(gameplay(snap, 20, 9)).await.unwrap();
}
/// Snapshot and gameplay commit over `columns`; the commit's receipt is stored and only the
/// committed columns advance.
async fn admitted(s: &dyn Store, columns: i32, total: i32, n: u8, entities: usize) {
    let snap = s
        .load_owned_blocks(spec(false, keys(columns)))
        .await
        .unwrap();
    for c in snap.columns() {
        assert_eq!(c.chunk().block_entity_count(), entities);
    }
    let before = revisions(&s.load_owned_blocks(spec(false, keys(total))).await.unwrap());
    let request = gameplay(snap, n, u32::from(n));
    let query = request.receipt_query();
    let out = s.commit_blocks(request).await.unwrap();
    assert_eq!(
        s.load_block_receipt(query).await.unwrap().as_ref(),
        Some(out.receipt())
    );
    let after = revisions(&s.load_owned_blocks(spec(false, keys(total))).await.unwrap());
    let committed = usize::try_from(columns).unwrap();
    for (i, (b, a)) in before.iter().zip(&after).enumerate() {
        let expected = if i < committed { b.map(|r| r + 1) } else { *b };
        assert_eq!(*a, expected, "column {i}");
    }
}
/// 7a: World-maximum entity density (4096 empty chests per column). Under the former flat
/// 16 KiB charge the 3-column snapshot and the 2-column gameplay commit refused.
#[tokio::test]
async fn probe7a_max_entity_density_columns_are_admitted_at_the_hard_default_caps() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = open(durable, &dir.path().join("a"), dir.path(), Caps::default());
        dense(&*s, 3, 4096).await;
        let snap = s.load_owned_blocks(spec(false, keys(3))).await.unwrap();
        assert_eq!(revisions(&snap), vec![Some(2); 3]);
        for c in snap.columns() {
            assert_eq!(c.chunk().block_entity_count(), 4096);
        }
        admitted(&*s, 2, 3, 30, 4096).await;
        admitted(&*s, 1, 3, 31, 4096).await;
    }
}
/// 7b: 1100 empty chests per column over an 8-column union. Formerly the 8-column gameplay
/// commit refused.
#[tokio::test]
async fn probe7b_eight_columns_with_1100_entities_are_admitted_at_the_hard_default_caps() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = open(durable, &dir.path().join("b"), dir.path(), Caps::default());
        dense(&*s, 8, 1100).await;
        admitted(&*s, 8, 8, 30, 1100).await;
        admitted(&*s, 4, 8, 31, 1100).await;
    }
}

// ---- Exact N/N-1 with entity data.

const COLUMNS: i32 = 4;
const CHESTS: u16 = 200;
/// The cell that receives one new chest per column; air before, distinct from the stock.
fn slot(k: ChunkKey) -> BlockPos {
    let o = k.pos().origin_block(-64);
    BlockPos::new(o.x() + 7, 90, o.z() + 7)
}
/// Places one new chest of `new` bytes at `slot` (an entity-only effect, cell 0 -> 0).
fn place(snap: OwnedBlockSnapshot, n: u8, new: &[u8]) -> BlockMutationRequest {
    let (mut images, mut effects) = (Vec::new(), Vec::new());
    for c in snap.columns() {
        let mut chunk = c.chunk().clone();
        chunk
            .set_block_entity(slot(c.key()), decode_block_entity(new).unwrap())
            .unwrap();
        images.push(image(c.key(), chunk));
        effects.push(effect(slot(c.key()), 0, 0, None, Some(new.to_vec())));
    }
    BlockMutationRequest::new(id(n), snap, images, vec![], effects).unwrap()
}
/// One image-CAS stocks every column, then one gameplay per column: the union's columns have
/// distinct gameplay last receipts. Returns each column's pre-gameplay Full length and receipt.
async fn histories(s: &dyn Store) -> Vec<(u64, BlockReceipt)> {
    initialize(s).await;
    let stock = decode_block_entity(&chest_bytes(3)).unwrap();
    image_cas(s, 10, &keys(COLUMNS), |k| stocked(k, CHESTS, &stock)).await;
    let mut out = Vec::new();
    for (i, k) in keys(COLUMNS).into_iter().enumerate() {
        let snap = s.load_owned_blocks(spec(false, vec![k])).await.unwrap();
        let before = snap.columns()[0].full_bytes().unwrap().len() as u64;
        let out_i = s
            .commit_blocks(place(snap, 20 + i as u8, &chest_bytes(8)))
            .await
            .unwrap();
        out.push((before, out_i.receipt().clone()));
    }
    out
}
/// Exact snapshot need of the union, from the layouts and the World codec's lengths:
/// - per call: identity row and counters;
/// - per column: its own reads; its distinct last receipt read (16-byte key + FCBR row) and
///   its Q digested once; that receipt's relations (identity, the entry's Header/Full/Overlay
///   reads, the journal row read with its 8-byte key and its exact re-encoding); the endpoint
///   proof decoding the before and after Full images; the effect's new entity checked
///   canonical (decode + encode) and decoded once more when applied; and the canonical
///   encodings of the before and after entity sets plus the view's entities.
///
/// Snapshot output copies are charged only to the output cap.
fn need(histories: &[(u64, BlockReceipt)], reference: &OwnedBlockSnapshot, new: u64) -> u64 {
    let mut need = CALL;
    for ((before, receipt), column) in histories.iter().zip(reference.columns()) {
        let full = column.full_bytes().unwrap().len() as u64;
        let r = receipt.encoded_bytes().len() as u64;
        let q = receipt.request_bytes().len() as u64;
        // FCBR = Q + 71 + 9 per column; a one-column gameplay Q is 98 + 46 per column, both
        // pairs, and 64 + entity bytes per effect.
        assert_eq!(r, q + 71 + 9);
        assert_eq!(q, 98 + 46 + before + full + 64 + new);
        // FCBL = 40-byte header + effect (60 + entity bytes).
        let journal = 40 + 60 + new;
        let entities = encoded(column.chunk());
        need += READS + full;
        need += 16 + r + q;
        need += 35 + READS + full + 8 + 2 * journal;
        need += before + full + 3 * new + 3 * entities;
    }
    need
}
#[tokio::test]
async fn realistic_entity_union_admits_its_exact_byte_need_and_refuses_one_less() {
    let new = chest_bytes(8).len() as u64;
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let sizing = open(
            durable,
            &dir.path().join("size"),
            dir.path(),
            Caps::default(),
        );
        let h = histories(&*sizing).await;
        let reference = sizing
            .load_owned_blocks(spec(false, keys(COLUMNS)))
            .await
            .unwrap();
        for c in reference.columns() {
            assert_eq!(c.chunk().block_entity_count(), usize::from(CHESTS) + 1);
            assert_eq!(encoded(c.chunk()), (u64::from(CHESTS) + 1) * new);
        }
        let need = need(&h, &reference, new);
        // The former flat 16 KiB per encoded entity alone would exceed this need.
        assert!(3 * COLUMNS as u64 * (u64::from(CHESTS) + 1) * 16 * 1024 > need);
        for (cap, admitted) in [(need, true), (need - 1, false)] {
            let path = dir.path().join(format!("cap-{cap}"));
            let s = if durable {
                histories(&*open(true, &path, dir.path(), Caps::default())).await;
                open(true, &path, dir.path(), bytes(cap))
            } else {
                let s = open(false, &path, dir.path(), bytes(cap));
                histories(&*s).await;
                s
            };
            let out = s.load_owned_blocks(spec(false, keys(COLUMNS))).await;
            if admitted {
                let snap = out.unwrap();
                for (c, expected) in snap.columns().iter().zip(reference.columns()) {
                    assert_eq!(c.stamp(), expected.stamp());
                    assert_eq!(c.full_bytes(), expected.full_bytes());
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

// ---- Entities at the 16 KiB form bound are still charged in full.

/// The World codec's exact 16 KiB chest (its golden layout: 339 overhead bytes).
fn chest_at_form_bound() -> Vec<u8> {
    form_bound_chest(0x5a)
}
fn form_bound_chest(fill: u8) -> Vec<u8> {
    let opaque = |id: i32, len: usize| {
        ComponentValue::Opaque(OpaqueComponent::new(id, vec![fill; len]).unwrap())
    };
    let stone = ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        13,
        ComponentPatch::new(
            vec![
                opaque(6, MAX_COMPONENT_BYTES),
                opaque(7, MAX_BLOCK_ENTITY_PAYLOAD_LEN - 339 - MAX_COMPONENT_BYTES),
            ],
            Vec::new(),
        ),
    )
    .unwrap();
    let mut chest = ChestInventory::new();
    *chest.slot_mut(26).unwrap() = stone;
    let mut b = Vec::new();
    try_encode_block_entity(&BlockEntity::Chest(chest), &mut b).unwrap();
    assert_eq!(b.len(), MAX_BLOCK_ENTITY_PAYLOAD_LEN);
    b
}
const PLACED: i32 = 64;
/// One gameplay placing `PLACED` chests of `new` bytes on air cells of one column.
fn place_many(snap: OwnedBlockSnapshot, n: u8, new: &[u8]) -> BlockMutationRequest {
    let c = &snap.columns()[0];
    let mut chunk = c.chunk().clone();
    let o = c.key().pos().origin_block(-64);
    let mut effects = Vec::new();
    for i in 0..PLACED {
        let at = BlockPos::new(o.x() + i % 16, 80 + i / 16, o.z() + 3);
        chunk
            .set_block_entity(at, decode_block_entity(new).unwrap())
            .unwrap();
        effects.push(effect(at, 0, 0, None, Some(new.to_vec())));
    }
    let images = vec![image(c.key(), chunk)];
    BlockMutationRequest::new(id(n), snap, images, vec![], effects).unwrap()
}
/// A cap below the entity-proportional charges of 64 placed form-bound chests refuses that
/// commit typed, without a write, and admits the same commit with realistic chests.
/// Lower bound of those charges per placed entity of N bytes, each charged before its work:
/// the Q parse and the FCBQ digest of the receipt frame each see it twice (effect and after
/// image), the FCBR encode and its retained copy each twice; the effect's canonical check twice
/// (decode, encode); once each for the after-image decode, the apply decode, the canonical
/// before and after entity sets, the FCBL encode and the after-image copy. 16·N in total.
#[tokio::test]
async fn form_bound_entities_exceed_a_byte_cap_typed_without_mutation() {
    let big = chest_at_form_bound();
    let small = chest_bytes(8);
    let cap = 16 * PLACED as u64 * big.len() as u64 - 1;
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = open(durable, &dir.path().join("f"), dir.path(), bytes(cap));
        initialize(&*s).await;
        let stock = decode_block_entity(&small).unwrap();
        image_cas(&*s, 10, &keys(1), |k| stocked(k, 8, &stock)).await;
        let snap = s.load_owned_blocks(spec(false, keys(1))).await.unwrap();
        let refused = place_many(snap.clone(), 20, &big);
        let query = refused.receipt_query();
        assert_eq!(
            s.commit_blocks(refused).await.err(),
            Some(BlockDurabilityError::ValidationCapacity("bytes")),
            "durable={durable}"
        );
        let after = s.load_owned_blocks(spec(false, keys(1))).await.unwrap();
        assert_eq!(after.columns()[0].stamp(), snap.columns()[0].stamp());
        assert_eq!(
            after.columns()[0].full_bytes(),
            snap.columns()[0].full_bytes()
        );
        assert!(s.load_block_receipt(query).await.unwrap().is_none());
        let out = s.commit_blocks(place_many(snap, 21, &small)).await.unwrap();
        let committed = s.load_owned_blocks(spec(false, keys(1))).await.unwrap();
        assert_eq!(
            committed.columns()[0].chunk().block_entity_count(),
            8 + PLACED as usize
        );
        assert_eq!(
            s.load_block_receipt(out.receipt().query())
                .await
                .unwrap()
                .as_ref(),
            Some(out.receipt())
        );
    }
}

// ---- The remaining hard-cap residual: distinct maximal, effect-dense receipts.

/// Toggles one form-bound chest at `slot` of every snapshot column between two contents, with
/// as many effects as the 32 MiB request bound admits, split evenly and odd per column (so
/// every column really changes). Returns the request and its effect count.
fn toggles(snap: OwnedBlockSnapshot, n: u8, from: &[u8], to: &[u8]) -> (BlockMutationRequest, u64) {
    let columns = snap.columns().len();
    let fulls: usize = snap
        .columns()
        .iter()
        .map(|c| c.full_bytes().unwrap().len())
        .sum();
    let per = 64 + from.len() + to.len();
    let mut count = (MAX_BLOCK_REQUEST_BYTES - 98 - 46 * columns - 2 * fulls) / per / columns;
    count -= 1 - count % 2;
    let (mut images, mut effects) = (Vec::new(), Vec::new());
    for c in snap.columns() {
        let mut chunk = c.chunk().clone();
        chunk
            .set_block_entity(slot(c.key()), decode_block_entity(to).unwrap())
            .unwrap();
        images.push(image(c.key(), chunk));
        for i in 0..count {
            let (old, new) = if i % 2 == 0 { (from, to) } else { (to, from) };
            effects.push(effect(
                slot(c.key()),
                0,
                0,
                Some(old.to_vec()),
                Some(new.to_vec()),
            ));
        }
    }
    let request = BlockMutationRequest::new(id(n), snap, images, vec![], effects).unwrap();
    (request, (count * columns) as u64)
}
fn contents() -> (Vec<u8>, Vec<u8>) {
    (form_bound_chest(0x5a), form_bound_chest(0xa5))
}
/// Stocks `columns` with one form-bound chest each, then one maximal toggle gameplay per
/// column, so each column's last receipt is a distinct near-32 MiB, effect-dense receipt.
async fn effect_dense(s: &dyn Store, columns: i32) -> Vec<(u64, BlockReceipt, u64)> {
    initialize(s).await;
    let (x, y) = contents();
    let stock = decode_block_entity(&x).unwrap();
    image_cas(s, 10, &keys(columns), |k| {
        let mut c = Chunk::new(k.pos());
        c.set_block_entity(slot(k), stock.clone()).unwrap();
        c
    })
    .await;
    let mut out = Vec::new();
    for (i, k) in keys(columns).into_iter().enumerate() {
        let snap = s.load_owned_blocks(spec(false, vec![k])).await.unwrap();
        let full = snap.columns()[0].full_bytes().unwrap().len() as u64;
        let (request, m) = toggles(snap, 20 + i as u8, &x, &y);
        let receipt = s.commit_blocks(request).await.unwrap().receipt().clone();
        out.push((full, receipt, m));
    }
    out
}
/// Exact snapshot need over the first `k` effect-dense columns (layouts as in `need`): per
/// column its own reads, its receipt read and digest, the relations with `m` journal rows
/// (read with key, re-encoded), both decoded images, per effect the canonical check of both
/// entities, the encoding of the replaced entity and the decode of the new one, and the three
/// canonical one-entity sets (before, after, view).
fn effect_dense_need(dense: &[(u64, BlockReceipt, u64)], columns: usize) -> u64 {
    let entity = MAX_BLOCK_ENTITY_PAYLOAD_LEN as u64;
    let max = MAX_BLOCK_REQUEST_BYTES as u64;
    let mut need = CALL;
    for (full, receipt, m) in &dense[..columns] {
        let r = receipt.encoded_bytes().len() as u64;
        let q = receipt.request_bytes().len() as u64;
        assert_eq!(r, q + 71 + 9);
        assert_eq!(q, 98 + 46 + 2 * full + m * (64 + 2 * entity));
        assert!(
            q <= max && q + 2 * (64 + 2 * entity) > max,
            "maximal odd effect count"
        );
        let journal = 40 + 60 + 2 * entity;
        need += READS + full;
        need += 16 + r + q;
        need += 35 + READS + full + m * (8 + 2 * journal);
        need += 2 * full + m * (2 * 2 * entity + entity + entity) + 3 * entity;
    }
    need
}
fn dense_caps(bytes: u64) -> Caps {
    Caps {
        bytes,
        journal_bytes: 256 * 1024 * 1024,
        ..Caps::default()
    }
}
/// Snapshots over two and three effect-dense columns.
async fn snapshots(s: &dyn Store, admits_two: bool) {
    let two = s.load_owned_blocks(spec(false, keys(2))).await;
    if admits_two {
        assert_eq!(revisions(&two.unwrap()), vec![Some(2); 2]);
    } else {
        assert_eq!(
            two.err(),
            Some(BlockDurabilityError::ValidationCapacity("bytes"))
        );
    }
    assert_eq!(
        s.load_owned_blocks(spec(false, keys(3))).await.err(),
        Some(BlockDurabilityError::ValidationCapacity("bytes"))
    );
}
/// At the hard default cap: a maximal effect-dense commit over two columns with distinct
/// effect-dense histories refuses typed without a write; over one column it is admitted.
async fn commits(s: &dyn Store) {
    let (x, y) = contents();
    let (y, x) = (x, y); // every column now holds the second content
    let snap = s.load_owned_blocks(spec(false, keys(2))).await.unwrap();
    let (request, _) = toggles(snap.clone(), 40, &x, &y);
    let query = request.receipt_query();
    assert_eq!(
        s.commit_blocks(request).await.err(),
        Some(BlockDurabilityError::ValidationCapacity("bytes"))
    );
    assert!(s.load_block_receipt(query).await.unwrap().is_none());
    for k in keys(2) {
        let one = s.load_owned_blocks(spec(false, vec![k])).await.unwrap();
        let at = snap.columns().iter().position(|c| c.key() == k).unwrap();
        assert_eq!(one.columns()[0].stamp(), snap.columns()[at].stamp());
        assert_eq!(
            one.columns()[0].full_bytes(),
            snap.columns()[at].full_bytes()
        );
    }
    let one = s.load_owned_blocks(spec(false, keys(1))).await.unwrap();
    let (request, _) = toggles(one, 41, &x, &y);
    let out = s.commit_blocks(request).await.unwrap();
    assert_eq!(
        s.load_block_receipt(out.receipt().query())
            .await
            .unwrap()
            .as_ref(),
        Some(out.receipt())
    );
}
/// Documented residual of the hard 512 MiB cap. A near-32 MiB receipt whose effects carry
/// form-bound entities costs about seven times its Q to validate, so three such distinct last
/// receipts refuse a snapshot typed while two are admitted at exactly their need; a commit
/// adding its own such request refuses with two of them and is admitted with one.
/// Every store is set up fresh under its cap (the set-up commits need less than the
/// two-column snapshot); redb is never reopened here, because its startup scan re-reads and
/// re-digests a receipt per journal row (documented separately as a startup cost).
#[tokio::test]
async fn distinct_effect_dense_form_bound_receipts_are_the_hard_cap_residual() {
    let hard = 512 * 1024 * 1024;
    let dir = tempfile::tempdir().unwrap();
    let sizing = open(
        false,
        &dir.path().join("sizing"),
        dir.path(),
        dense_caps(hard),
    );
    let h = effect_dense(&*sizing, 3).await;
    let (two, three) = (effect_dense_need(&h, 2), effect_dense_need(&h, 3));
    let q = h[0].1.request_bytes().len() as u64;
    assert!(two <= hard && three > hard, "{two} {three}");
    // About seven Q per receipt: FCBR row and digest (2 Q), journal rows read and re-encoded
    // (about 2 Q), and per effect canonical checks, replaced-entity encode and apply decode
    // (about 3 Q).
    assert!(6 * q < two / 2 && two / 2 < 8 * q, "{q} {two}");
    snapshots(&*sizing, true).await;
    commits(&*sizing).await;
    drop(sizing);
    for durable in [false, true] {
        for (cap, admits_two) in [(two, true), (two - 1, false)] {
            let path = dir.path().join(format!("{durable}-{cap}"));
            let s = open(durable, &path, dir.path(), dense_caps(cap));
            let got = effect_dense(&*s, 3).await;
            for (a, b) in got.iter().zip(&h) {
                assert_eq!(
                    (a.0, a.1.encoded_bytes(), a.2),
                    (b.0, b.1.encoded_bytes(), b.2)
                );
            }
            snapshots(&*s, admits_two).await;
        }
    }
    let s = open(true, &dir.path().join("hard"), dir.path(), dense_caps(hard));
    effect_dense(&*s, 3).await;
    snapshots(&*s, true).await;
    commits(&*s).await;
}
