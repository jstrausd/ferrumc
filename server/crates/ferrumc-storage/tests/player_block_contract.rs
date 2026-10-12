//! Independently observed public API and actual17-table original-plus-delta assertions.
mod block_support;
use block_support::*;
use ferrumc_items::decode_persisted_stack;
use ferrumc_math::BlockPos;
use ferrumc_storage::*;
use ferrumc_world::BlockStateId;

async fn verify(s: &dyn Store, receipt: &BlockReceipt) {
    let snap = s
        .load_owned_blocks(spec(true, vec![key(1), key(0)]))
        .await
        .unwrap();
    assert_eq!(snap.player_record().unwrap().data(), player_bytes(10));
    assert_eq!(snap.player_stamp().revision().unwrap().get(), 2);
    let expected = [
        (
            key(0),
            vec![(pos(15, 64), 100), (pos(15, 65), 101), (pos(14, 64), 200)],
            vec![(pos(14, 64), chest_bytes(8))],
        ),
        (key(1), vec![(pos(16, 64), 300)], vec![]),
    ];
    for (column, (key, changes, entities)) in snap.columns().iter().zip(expected) {
        assert_eq!(column.key(), key);
        assert_eq!(column.stamp().revision().unwrap().get(), 2);
        let origin = key.pos().origin_block(-64);
        for y in -64..320 {
            for z in 0..16 {
                for x in 0..16 {
                    let p = BlockPos::new(origin.x() + x, y, origin.z() + z);
                    assert_eq!(
                        column.chunk().get_block(p).unwrap().as_u32(),
                        changes.iter().find(|(q, _)| *q == p).map_or(0, |(_, n)| *n)
                    );
                }
            }
        }
        assert_eq!(column.chunk().block_entity_count(), entities.len());
        for (p, b) in entities {
            let actual = column.chunk().block_entity(p).unwrap();
            let expected = ferrumc_world::decode_block_entity(&b).unwrap();
            assert_eq!(actual, &expected);
        }
    }
    assert_eq!(receipt.journal_range(), Some((0, 4)));
    assert_eq!(
        s.load_block_receipt(receipt.query())
            .await
            .unwrap()
            .as_ref(),
        Some(receipt)
    );
    assert!(matches!(
        s.load_ownership_receipt(OwnershipReceiptQuery::from_bytes(
            receipt.query().id(),
            receipt.query().sha256()
        ))
        .await,
        Err(OwnershipError::ReceiptKindMismatch { .. })
    ));
}
#[tokio::test]
async fn public_memory_complete_joint_components_and_exact_replay() {
    assert_eq!(
        stack(8),
        include_bytes!("fixtures/player_block_format7/stack8.fcis")
    );
    assert_eq!(
        player_bytes(11),
        include_bytes!("fixtures/player_block_format7/before.fcps3")
    );
    assert_eq!(
        player_bytes(10),
        include_bytes!("fixtures/player_block_format7/after.fcps3")
    );
    for count in 1..=64 {
        decode_persisted_stack(&stack(count)).unwrap();
    }
    let s = InMemoryStore::with_player_block_durability(config()).unwrap();
    initialize(&s).await;
    seed_air(&s).await;
    let request = joint(&s, 3).await;
    let query = request.receipt_query();
    let out = s.commit_player_blocks(request.clone()).await.unwrap();
    verify(&s, out.receipt()).await;
    let replay = s.commit_player_blocks(request).await.unwrap();
    assert_eq!(replay.receipt(), out.receipt());
    assert_eq!(replay.disposition(), OwnershipCommitDisposition::Replayed);
    assert_eq!(replay.receipt().query(), query);
}
#[allow(clippy::too_many_lines)] // Keep one complete matrix case and its assertions together.
fn raw_delta(before: &Rows, after: &Rows, receipt: &BlockReceipt) {
    let mut expected = before.clone();
    let pk = player().id().as_uuid().as_bytes().to_vec();
    expected
        .get_mut("ferrumc:player")
        .unwrap()
        .insert(pk.clone(), player_envelope(10));
    let mut player_header = expected["ferrumc:player_ownership"][&pk].clone();
    player_header[15..23].copy_from_slice(&2u64.to_be_bytes());
    player_header[23..39].copy_from_slice(&id(3).to_bytes());
    expected
        .get_mut("ferrumc:player_ownership")
        .unwrap()
        .insert(pk, player_header);
    for (k, schema, changes, entities) in [
        (
            key(0),
            57,
            vec![(pos(15, 64), 100), (pos(15, 65), 101), (pos(14, 64), 200)],
            vec![(pos(14, 64), chest_bytes(8))],
        ),
        (key(1), 56, vec![(pos(16, 64), 300)], vec![]),
    ] {
        let key = chunk_key(k);
        expected
            .get_mut("ferrumc:chunk")
            .unwrap()
            .insert(key.clone(), full(k, schema, &changes, &entities));
        let mut h = b"FCOB\0\x01\x01".to_vec();
        h.extend_from_slice(&2u64.to_be_bytes());
        h.extend_from_slice(&id(3).to_bytes());
        expected
            .get_mut("ferrumc:chunk_ownership")
            .unwrap()
            .insert(key, h);
    }
    // Decode the actual retained body field by field against fixed originals/postimages.
    let mut row = Read::new(receipt.encoded_bytes());
    assert_eq!(row.take(6), b"FCBR\0\x01");
    let n = row.n(4) as usize;
    let qbytes = row.take(n);
    assert_eq!(qbytes, receipt.request_bytes());
    let mut q = Read::new(qbytes);
    assert_eq!(q.take(6), b"FCBQ\0\x01");
    assert_eq!(q.n(1), 1);
    assert_eq!(q.take(16), id(3).to_bytes());
    assert_eq!(q.take(8), &chunk_key(key(0))[..8]);
    assert_eq!(q.take(27), generation().encode());
    assert_eq!(q.take(16), player().id().as_uuid().as_bytes());
    assert_eq!(q.n(1), 1);
    assert_eq!(q.n(8), 1);
    assert_eq!(q.blob(), Some(player_envelope(11).as_slice()));
    assert_eq!(q.blob(), Some(player_envelope(10).as_slice()));
    assert_eq!(q.n(1), 2);
    for (k, schema, changes, entities) in [
        (
            key(0),
            57,
            vec![(pos(15, 64), 100), (pos(15, 65), 101), (pos(14, 64), 200)],
            vec![(pos(14, 64), chest_bytes(8))],
        ),
        (key(1), 56, vec![(pos(16, 64), 300)], vec![]),
    ] {
        assert_eq!(q.take(16), chunk_key(k));
        assert_eq!(q.n(1), 1);
        assert_eq!(q.n(8), 1);
        assert_eq!(q.blob(), Some(full(k, 55, &[], &[]).as_slice()));
        assert_eq!(q.blob(), None);
        assert_eq!(q.n(1), 1);
        assert_eq!(
            q.blob(),
            Some(full(k, schema, &changes, &entities).as_slice())
        );
        assert_eq!(q.blob(), None);
    }
    assert_eq!(q.n(2), 2);
    for p in [pos(15, 63), pos(16, 65)] {
        for n in [p.x(), p.y(), p.z()] {
            assert_eq!(q.n(4) as u32, n as u32);
        }
    }
    assert_eq!(q.n(2), 4);
    for (ordinal, (p, old, new, entity)) in [
        (pos(15, 64), 0, 100, None),
        (pos(15, 65), 0, 101, None),
        (pos(14, 64), 0, 200, Some(chest_bytes(8))),
        (pos(16, 64), 0, 300, None),
    ]
    .into_iter()
    .enumerate()
    {
        let len = q.n(4) as usize;
        let bytes = q.take(len);
        let mut e = Read::new(bytes);
        assert_eq!(e.n(4), 97);
        assert_eq!(e.n(8), 481);
        assert_eq!(e.n(1), 1);
        assert_eq!(e.take(16), player().id().as_uuid().as_bytes());
        assert_eq!(e.n(1), 0);
        for n in [p.x(), p.y(), p.z()] {
            assert_eq!(e.n(4) as u32, n as u32);
        }
        assert_eq!(e.n(4), old);
        assert_eq!(e.n(4), new);
        assert_eq!(e.blob(), None);
        assert_eq!(e.blob(), entity.as_deref());
        e.done();
        let mut journal = b"FCBL\0\x01".to_vec();
        journal.extend_from_slice(&chunk_key(key(0))[..8]);
        journal.extend_from_slice(&id(3).to_bytes());
        journal.extend_from_slice(&(ordinal as u16).to_be_bytes());
        journal.extend_from_slice(&(ordinal as u64).to_be_bytes());
        journal.extend_from_slice(bytes);
        expected
            .get_mut("ferrumc:scoped_mutation_log")
            .unwrap()
            .insert((ordinal as u64).to_be_bytes().to_vec(), journal);
    }
    q.done();
    assert_eq!(row.n(1), 1);
    assert_eq!(row.n(8), 2);
    assert_eq!(row.n(1), 2);
    for _ in 0..2 {
        assert_eq!(row.n(1), 1);
        assert_eq!(row.n(8), 2);
    }
    assert_eq!(row.n(1), 1);
    assert_eq!(row.n(8), 0);
    assert_eq!(row.n(2), 4);
    assert_eq!(row.take(32), receipt.query().sha256());
    assert_eq!(row.take(8), [0; 8]);
    row.done();
    expected
        .get_mut("ferrumc:ownership_receipt")
        .unwrap()
        .insert(id(3).to_bytes().to_vec(), receipt.encoded_bytes().to_vec());
    for (k, v) in [
        ("ownership_receipt_count", 3),
        (
            "operation_receipt_bytes",
            meta(before, "operation_receipt_bytes") + 16 + receipt.encoded_bytes().len() as u64,
        ),
        ("scoped_journal_count", 4),
        (
            "scoped_journal_bytes",
            expected["ferrumc:scoped_mutation_log"]
                .iter()
                .map(|(k, v)| (k.len() + v.len()) as u64)
                .sum(),
        ),
        ("last_scoped_mutation_id", 3),
    ] {
        expected
            .get_mut("ferrumc:meta")
            .unwrap()
            .insert(k.as_bytes().to_vec(), v.to_be_bytes().to_vec());
    }
    assert_eq!(
        after, &expected,
        "whole17-table delta, including unaffected tables/rows"
    );
}
#[tokio::test]
async fn independent_actual_redb17_delta_reopen_and_old_incarnation_replay() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("contract.redb");
    let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    initialize(&s).await;
    seed_air(&s).await;
    let request = joint(&s, 3).await;
    drop(s);
    let before = raw17(&path);
    assert_eq!(
        before,
        fixed_rows(include_str!("fixtures/player_block_format7/before17.tsv"))
    );
    let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    assert!(matches!(
        s.commit_player_blocks(request.clone()).await,
        Err(BlockDurabilityError::BackendOwnerMismatch)
    ));
    let actual = joint(&s, 3).await;
    assert_eq!(actual.receipt_query(), request.receipt_query());
    let out = s.commit_player_blocks(actual).await.unwrap();
    verify(&s, out.receipt()).await;
    drop(s);
    let after = raw17(&path);
    assert_eq!(
        after,
        fixed_rows(include_str!("fixtures/player_block_format7/after17.tsv"))
    );
    assert_eq!(
        out.receipt().encoded_bytes(),
        include_bytes!("fixtures/player_block_format7/joint.fcbr")
    );
    raw_delta(&before, &after, out.receipt());
    let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    let replay = s.commit_player_blocks(request).await.unwrap();
    assert_eq!(replay.receipt(), out.receipt());
    verify(&s, out.receipt()).await;
    drop(s);
    assert_eq!(raw17(&path), after);
}
#[tokio::test]
async fn complete_entity_only_delta_and_hidden_cell_refusal() {
    let s = InMemoryStore::with_player_block_durability(config()).unwrap();
    initialize(&s).await;
    seed_air(&s).await;
    s.commit_player_blocks(joint(&s, 3).await).await.unwrap();
    let snap = s.load_owned_blocks(spec(true, vec![key(0)])).await.unwrap();
    let mut chunk = snap.columns()[0].chunk().clone();
    chunk
        .set_block_entity(
            pos(14, 64),
            ferrumc_world::decode_block_entity(&chest_bytes(9)).unwrap(),
        )
        .unwrap();
    let image = ChunkImage::new(
        Some(ChunkRecord::try_new(SchemaVersion::new(57), chunk.clone()).unwrap()),
        None,
    )
    .unwrap();
    let req = PlayerBlockRequest::new(
        id(4),
        snap.clone(),
        p(9),
        vec![ChunkImageChange::new(
            key(0),
            ChunkImageAction::Replace(image),
        )],
        vec![],
        vec![effect(
            pos(14, 64),
            200,
            200,
            Some(chest_bytes(8)),
            Some(chest_bytes(9)),
        )],
    )
    .unwrap();
    s.commit_player_blocks(req).await.unwrap();
    chunk.set_block(pos(12, 100), BlockStateId::new(1)).unwrap();
    let hidden = PlayerBlockRequest::new(
        id(5),
        s.load_owned_blocks(spec(true, vec![key(0)])).await.unwrap(),
        p(8),
        vec![ChunkImageChange::new(
            key(0),
            ChunkImageAction::Replace(
                ChunkImage::new(
                    Some(ChunkRecord::try_new(SchemaVersion::new(57), chunk).unwrap()),
                    None,
                )
                .unwrap(),
            ),
        )],
        vec![],
        vec![],
    )
    .unwrap();
    assert!(matches!(
        s.commit_player_blocks(hidden).await,
        Err(BlockDurabilityError::IncompleteDelta { .. })
    ));
    assert_eq!(
        s.load_owned_player(player(), generation())
            .await
            .unwrap()
            .record()
            .unwrap()
            .data(),
        player_bytes(9)
    );
}
