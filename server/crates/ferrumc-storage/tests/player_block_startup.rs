//! Independent complete-scan byte oracle and real N-1/N/N+1 exclusive reopen checks.
mod block_support;
use block_support::*;
use ferrumc_items::{
    ComponentPatch, ComponentValue, ItemId, ItemStack, OpaqueComponent, MAX_COMPONENT_BYTES,
};
use ferrumc_math::BlockPos;
use ferrumc_storage::*;
use ferrumc_world::{
    decode_block_entity, try_encode_block_entity, BlockEntity, BlockStateId, ChestInventory, Chunk,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
struct Entry {
    key: Vec<u8>,
    before: u64,
    after: u64,
    revision: u64,
    entity_total: u64,
}
struct Operation {
    id: Vec<u8>,
    receipt: BlockReceipt,
    entries: Vec<Entry>,
    effects: u64,
    entity_each_side: u64,
    image: bool,
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
fn encoded_key(k: ChunkKey) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&k.world().get().to_be_bytes());
    out.extend_from_slice(&k.dimension().get().to_be_bytes());
    out.extend_from_slice(&k.pos().x().to_be_bytes());
    out.extend_from_slice(&k.pos().z().to_be_bytes());
    out
}
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
fn form_bound(fill: u8) -> Vec<u8> {
    let opaque =
        |id, len| ComponentValue::Opaque(OpaqueComponent::new(id, vec![fill; len]).unwrap());
    let item = ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        13,
        ComponentPatch::new(
            vec![
                opaque(6, MAX_COMPONENT_BYTES),
                opaque(7, 16_384 - 339 - MAX_COMPONENT_BYTES),
            ],
            vec![],
        ),
    )
    .unwrap();
    let mut chest = ChestInventory::new();
    *chest.slot_mut(26).unwrap() = item;
    let mut bytes = Vec::new();
    try_encode_block_entity(&BlockEntity::Chest(chest), &mut bytes).unwrap();
    assert_eq!(bytes.len(), 16_384);
    bytes
}
fn proof(
    op: &Operation,
    fulls: &BTreeMap<Vec<u8>, u64>,
    headers: &BTreeMap<Vec<u8>, Vec<u8>>,
) -> u64 {
    let mut n = 35;
    for e in &op.entries {
        n += 47;
        let rev = u64::from_be_bytes(headers[&e.key][7..15].try_into().unwrap());
        if rev == e.revision {
            n += 32 + fulls[&e.key];
        }
        n += e.before + e.after;
        if !op.image {
            n += 2 * e.entity_total;
        }
    }
    let journal = 100 + 2 * op.entity_each_side;
    n += op.effects * (8 + 2 * journal + 6 * op.entity_each_side);
    n
}
/// The whole scan's raw inventory, every physical relation read, parse/hash, decode/copy,
/// canonical entity encoding, exact journal encoding and scratch-index row. No candidate
/// budget or private parser is queried. Table/header offsets follow the immutable contract.
fn scan_need(rows: &Rows, ops: &[Operation]) -> u64 {
    let fulls: BTreeMap<_, _> = rows["ferrumc:chunk"]
        .iter()
        .map(|(k, v)| (k.clone(), v.len() as u64))
        .collect();
    let headers = &rows["ferrumc:chunk_ownership"];
    let by_id: BTreeMap<_, _> = ops.iter().map(|op| (op.id.clone(), op)).collect();
    let mut n = 128
        + rows
            .iter()
            .filter(|(name, _)| name.as_str() != "ferrumc:meta")
            .flat_map(|(_, r)| r)
            .map(|(k, v)| (k.len() + v.len()) as u64)
            .sum::<u64>();
    let mut proved = BTreeSet::new();
    for name in ["ferrumc:chunk", "ferrumc:chunk_ownership"] {
        for key in rows[name].keys() {
            let op = by_id[&headers[key][15..31]];
            let q = op.receipt.request_bytes().len() as u64;
            let r = op.receipt.encoded_bytes().len() as u64;
            n += 35 + 79 + 2 * fulls[key] + 16 + r + q;
            if name == "ferrumc:chunk" {
                n += 35;
            }
            if proved.insert(op.id.clone()) {
                n += proof(op, &fulls, headers);
            }
        }
    }
    for id in rows["ferrumc:ownership_receipt"].keys() {
        let op = by_id[id];
        let q = op.receipt.request_bytes().len() as u64;
        let blob_total: u64 = op.entries.iter().map(|e| e.before + e.after).sum();
        assert_eq!(
            q,
            98 + 46 * op.entries.len() as u64
                + blob_total
                + op.effects * (64 + 2 * op.entity_each_side)
        );
        assert_eq!(
            op.receipt.encoded_bytes().len() as u64,
            q + 71 + 9 * op.entries.len() as u64
        );
        n += q;
        if proved.insert(id.clone()) {
            n += q + proof(op, &fulls, headers);
        }
        if op.effects != 0 {
            n += 16 + op.receipt.encoded_bytes().len() as u64 + q + 17;
        }
    }
    n
}
#[allow(clippy::too_many_lines)] // Keep the three admitted fixture constructions beside their independent model inputs.
async fn history(s: &RedbStore, dense: bool, multiple: bool) -> Vec<Operation> {
    s.ensure_generation_identity(scope(), generation())
        .await
        .unwrap();
    let keys = if dense {
        vec![key(0)]
    } else {
        vec![key(0), key(1), key(2)]
    };
    let (from, to) = if dense {
        (form_bound(0x5a), form_bound(0xa5))
    } else {
        (vec![], vec![])
    };
    let at = BlockPos::new(1, 300, -15);
    let mut chunks: Vec<_> = keys
        .iter()
        .map(|k| {
            if dense {
                Chunk::new(k.pos())
            } else {
                noisy(*k)
            }
        })
        .collect();
    if dense {
        chunks[0]
            .set_block_entity(at, decode_block_entity(&from).unwrap())
            .unwrap();
    }
    let snap = s
        .load_owned_blocks(spec(false, keys.clone()))
        .await
        .unwrap();
    let out = s
        .compare_exchange_chunk_images(
            ChunkImageCasRequest::new(
                id(80),
                snap,
                keys.iter()
                    .zip(chunks)
                    .map(|(k, c)| replace(*k, c))
                    .collect(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let snap = s
        .load_owned_blocks(spec(false, keys.clone()))
        .await
        .unwrap();
    let entries: Vec<_> = snap
        .columns()
        .iter()
        .map(|c| Entry {
            key: encoded_key(c.key()),
            before: 0,
            after: c.full_bytes().unwrap().len() as u64,
            revision: 1,
            entity_total: 0,
        })
        .collect();
    let mut ops = vec![Operation {
        id: id(80).to_bytes().to_vec(),
        receipt: out.receipt().clone(),
        entries,
        effects: 0,
        entity_each_side: 0,
        image: true,
    }];
    let passes = if multiple { 3 } else { 1 };
    for pass in 0..passes {
        let ks = if pass == 1 {
            vec![key(0)]
        } else {
            keys.clone()
        };
        let snap = s.load_owned_blocks(spec(false, ks)).await.unwrap();
        let fulls: usize = snap
            .columns()
            .iter()
            .map(|c| c.full_bytes().unwrap().len())
            .sum();
        let count = if dense {
            1019
        } else if multiple {
            2 + 2 * pass
        } else {
            2
        };
        if dense {
            assert!(98 + 46 + 2 * fulls + count * (64 + 2 * from.len()) <= MAX_BLOCK_REQUEST_BYTES);
        }
        let old = snap.columns()[0].chunk().get_block(at).unwrap().as_u32();
        let new = if old == 1 { 2 } else { 1 };
        let effects = (0..count)
            .map(|i| {
                if dense {
                    let (a, b) = if i % 2 == 0 {
                        (&from, &to)
                    } else {
                        (&to, &from)
                    };
                    effect(at, old, old, Some(a.clone()), Some(b.clone()))
                } else if i % 2 == 0 {
                    effect(at, old, new, None, None)
                } else {
                    effect(at, new, old, None, None)
                }
            })
            .collect();
        let mut images = Vec::new();
        let mut entries = Vec::new();
        for c in snap.columns() {
            let mut chunk = c.chunk().clone();
            if dense {
                chunk
                    .set_block_entity(at, decode_block_entity(&to).unwrap())
                    .unwrap();
            }
            entries.push(Entry {
                key: encoded_key(c.key()),
                before: c.full_bytes().unwrap().len() as u64,
                after: c.full_bytes().unwrap().len() as u64,
                revision: c.stamp().revision().unwrap().get() + u64::from(c.key() == key(0)),
                entity_total: if dense { 16_384 } else { 0 },
            });
            images.push(replace(c.key(), chunk));
        }
        let op_id = id(81 + pass as u8);
        let request = BlockMutationRequest::new(op_id, snap, images, vec![], effects).unwrap();
        let out = s.commit_blocks(request).await.unwrap();
        ops.push(Operation {
            id: op_id.to_bytes().to_vec(),
            receipt: out.receipt().clone(),
            entries,
            effects: count as u64,
            entity_each_side: if dense { 16_384 } else { 0 },
            image: false,
        });
    }
    ops
}
async fn boundaries(dense: bool, multiple: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scan");
    let scratch = dir.path().join("scratch");
    std::fs::create_dir(&scratch).unwrap();
    let cfg = config_with(Caps {
        relation_bytes: 1024 * 1024 * 1024,
        ..Caps::default()
    })
    .unwrap();
    let s = RedbStore::open_with_player_block_durability(&path, cfg, &scratch).unwrap();
    let ops = history(&s, dense, multiple).await;
    drop(s);
    let before = raw17(&path);
    let n = scan_need(&before, &ops);
    println!("full scan dense={dense} multiple={multiple} N={n}");
    assert_eq!(before.len(), 17);
    for cap in [n - 1, n, n + 1] {
        let out = RedbStore::open_with_player_block_durability(
            &path,
            config_with(Caps {
                relation_bytes: cap,
                ..Caps::default()
            })
            .unwrap(),
            &scratch,
        );
        if cap < n {
            assert_eq!(out.err(), Some(BlockDurabilityError::ScanCapacity("bytes")));
        } else {
            drop(out.unwrap());
        }
        assert_eq!(raw17(&path), before);
        assert_eq!(std::fs::read_dir(&scratch).unwrap().count(), 0);
    }
}
#[tokio::test]
async fn small_effect_large_q_reopen_exact_scan_bytes() {
    boundaries(false, false).await;
}
#[tokio::test]
async fn dense_near_32_mib_entity_reopen_exact_scan_bytes() {
    boundaries(true, false).await;
}
#[tokio::test]
async fn multiple_operation_reopen_exact_scan_bytes() {
    boundaries(false, true).await;
}
