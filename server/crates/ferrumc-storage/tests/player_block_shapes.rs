//! Public complete pair composition and endpoint rejection on both actual backends.
mod block_support;
use block_support::*;
use ferrumc_math::BlockPos;
use ferrumc_storage::*;
use ferrumc_world::{decode_block_entity, BlockStateId, Chunk};
use std::sync::Arc;
fn image(k: ChunkKey, c: Chunk) -> ChunkImageChange {
    ChunkImageChange::new(
        k,
        ChunkImageAction::Replace(
            ChunkImage::new(
                Some(ChunkRecord::try_new(SchemaVersion::new(89), c).unwrap()),
                None,
            )
            .unwrap(),
        ),
    )
}
async fn put(s: &dyn Store, n: u8, full: Option<ChunkRecord>, overlay: Option<ChunkOverlayRecord>) {
    let snap = s
        .load_owned_blocks(spec(false, vec![key(0)]))
        .await
        .unwrap();
    s.compare_exchange_chunk_images(
        ChunkImageCasRequest::new(
            id(n),
            snap,
            vec![ChunkImageChange::new(
                key(0),
                ChunkImageAction::Replace(ChunkImage::new(full, overlay).unwrap()),
            )],
        )
        .unwrap(),
    )
    .await
    .unwrap();
}
#[allow(clippy::too_many_lines)] // Keep one complete matrix case and its assertions together.
#[tokio::test]
async fn full_overlay_both_neither_section_replaces_entities_zero_section_merges() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("composition");
        let mut s: Arc<dyn Store> = if durable {
            Arc::new(
                RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap(),
            )
        } else {
            Arc::new(InMemoryStore::with_player_block_durability(config()).unwrap())
        };
        initialize(&*s).await;
        let fresh = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        assert!(fresh.columns()[0].stamp().is_fresh());
        assert_eq!(
            fresh.columns()[0]
                .chunk()
                .get_block(pos(0, -64))
                .unwrap()
                .as_u32(),
            85
        );
        let a = pos(0, 64);
        let b = pos(1, 64);
        let mut full = Chunk::new(key(0).pos());
        full.set_block(a, BlockStateId::new(200)).unwrap();
        full.set_block_entity(a, decode_block_entity(&chest_bytes(8)).unwrap())
            .unwrap();
        put(
            &*s,
            2,
            Some(ChunkRecord::try_new(SchemaVersion::new(71), full.clone()).unwrap()),
            None,
        )
        .await;
        let mut ov = Chunk::new(key(0).pos());
        ov.set_block(b, BlockStateId::new(201)).unwrap();
        ov.set_block_entity(b, decode_block_entity(&chest_bytes(9)).unwrap())
            .unwrap();
        // Entity-only capture first: no persist-edited section, so it merges.
        let zero = ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(73), key(0).pos(), &ov, 0)
            .unwrap();
        assert_eq!(zero.section_count(), 0);
        ov.mark_persist_dirty(b);
        let overlay =
            ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(72), key(0).pos(), &ov, 5)
                .unwrap();
        assert_eq!(overlay.section_count(), 1);
        put(
            &*s,
            3,
            Some(ChunkRecord::try_new(SchemaVersion::new(71), full.clone()).unwrap()),
            Some(overlay.clone()),
        )
        .await;
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        assert!(snap.columns()[0].chunk().block_entity(a).is_none());
        assert!(snap.columns()[0].chunk().block_entity(b).is_some());
        assert_eq!(
            snap.columns()[0].chunk().get_block(b).unwrap().as_u32(),
            201
        );
        // The carried section replaces every base cell in it, including `a`.
        assert_eq!(snap.columns()[0].chunk().get_block(a).unwrap().as_u32(), 0);
        assert_eq!(
            s.load_chunk(key(0))
                .await
                .unwrap()
                .unwrap()
                .schema_version()
                .get(),
            71
        );
        assert_eq!(
            s.load_chunk_overlay(key(0))
                .await
                .unwrap()
                .unwrap()
                .schema_version()
                .get(),
            72
        );
        put(
            &*s,
            4,
            Some(ChunkRecord::try_new(SchemaVersion::new(71), full.clone()).unwrap()),
            Some(zero),
        )
        .await;
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        assert!(snap.columns()[0].chunk().block_entity(a).is_some());
        assert!(snap.columns()[0].chunk().block_entity(b).is_some());
        let empty = ChunkOverlayRecord::try_from_chunk(
            SchemaVersion::new(74),
            key(0).pos(),
            &Chunk::new(key(0).pos()),
            0,
        )
        .unwrap();
        put(
            &*s,
            5,
            Some(ChunkRecord::try_new(SchemaVersion::new(71), full).unwrap()),
            Some(empty),
        )
        .await;
        assert!(s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap()
            .columns()[0]
            .chunk()
            .block_entity(a)
            .is_some());
        put(&*s, 6, None, Some(overlay)).await;
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        assert_eq!(
            snap.columns()[0]
                .chunk()
                .get_block(pos(0, -64))
                .unwrap()
                .as_u32(),
            85
        );
        assert!(snap.columns()[0].chunk().block_entity(b).is_some());
        s.compare_exchange_chunk_images(
            ChunkImageCasRequest::new(
                id(7),
                snap,
                vec![ChunkImageChange::new(key(0), ChunkImageAction::Erase)],
            )
            .unwrap(),
        )
        .await
        .unwrap();
        if durable {
            drop(s);
            s = Arc::new(
                RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap(),
            );
        }
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        assert!(snap.columns()[0].stamp().is_tombstone());
        assert!(snap.columns()[0].full_bytes().is_none());
        assert!(snap.columns()[0].overlay_bytes().is_none());
        assert_eq!(
            snap.columns()[0]
                .chunk()
                .get_block(pos(0, -64))
                .unwrap()
                .as_u32(),
            85
        );
    }
}
#[allow(clippy::too_many_lines)] // Keep one complete matrix case and its assertions together.
#[tokio::test]
async fn hidden_last_cell_entity_only_chain_sign_and_shape_refusals() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("endpoints");
        let s: Arc<dyn Store> = if durable {
            Arc::new(
                RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap(),
            )
        } else {
            Arc::new(InMemoryStore::with_player_block_durability(config()).unwrap())
        };
        initialize(&*s).await;
        seed_air(&*s).await;
        let snap = s.load_owned_blocks(spec(true, vec![key(0)])).await.unwrap();
        let last = BlockPos::new(15, 319, -1);
        let mut changed = snap.columns()[0].chunk().clone();
        changed.set_block(last, BlockStateId::new(777)).unwrap();
        let r = PlayerBlockRequest::new(
            id(3),
            snap.clone(),
            p(10),
            vec![image(key(0), changed.clone())],
            vec![],
            vec![],
        )
        .unwrap();
        assert!(matches!(
            s.commit_player_blocks(r).await,
            Err(BlockDurabilityError::IncompleteDelta { .. })
        ));
        assert_eq!(
            s.load_owned_player(player(), generation())
                .await
                .unwrap()
                .record()
                .unwrap()
                .data(),
            player_bytes(11)
        );
        s.commit_player_blocks(
            PlayerBlockRequest::new(
                id(3),
                snap,
                p(10),
                vec![image(key(0), changed)],
                vec![],
                vec![effect(last, 0, 777, None, None)],
            )
            .unwrap(),
        )
        .await
        .unwrap();
        // Entity-only changes still need the complete lossless component endpoint.
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let mut c = snap.columns()[0].chunk().clone();
        let at = pos(14, 64);
        c.set_block_entity(at, decode_block_entity(&chest_bytes(8)).unwrap())
            .unwrap();
        let omitted = BlockMutationRequest::new(
            id(4),
            snap.clone(),
            vec![image(key(0), c.clone())],
            vec![],
            vec![],
        )
        .unwrap();
        assert!(matches!(
            s.commit_blocks(omitted).await,
            Err(BlockDurabilityError::IncompleteDelta { .. })
        ));
        s.commit_blocks(
            BlockMutationRequest::new(
                id(4),
                snap,
                vec![image(key(0), c)],
                vec![],
                vec![effect(at, 0, 0, None, Some(chest_bytes(8)))],
            )
            .unwrap(),
        )
        .await
        .unwrap();
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let mut c = snap.columns()[0].chunk().clone();
        c.set_block_entity(at, decode_block_entity(&chest_bytes(9)).unwrap())
            .unwrap();
        let broken = BlockMutationRequest::new(
            id(5),
            snap.clone(),
            vec![image(key(0), c.clone())],
            vec![],
            vec![effect(at, 0, 0, Some(chest_bytes(7)), Some(chest_bytes(9)))],
        )
        .unwrap();
        assert!(matches!(
            s.commit_blocks(broken).await,
            Err(BlockDurabilityError::InvalidEffect { .. })
        ));
        let chain = vec![
            effect(at, 0, 0, Some(chest_bytes(8)), Some(chest_bytes(10))),
            effect(at, 0, 0, Some(chest_bytes(10)), Some(chest_bytes(9))),
        ];
        s.commit_blocks(
            BlockMutationRequest::new(id(5), snap, vec![image(key(0), c)], vec![], chain).unwrap(),
        )
        .await
        .unwrap();
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let r = BlockMutationRequest::new(
            id(6),
            snap.clone(),
            vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)],
            vec![],
            vec![effect(at, 0, 0, Some(chest_bytes(9)), Some(chest_bytes(9)))],
        )
        .unwrap();
        assert!(matches!(
            s.commit_blocks(r).await,
            Err(BlockDurabilityError::InvalidEffect { .. })
        ));
        assert!(BlockMutationRequest::new(
            id(6),
            snap.clone(),
            vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)],
            vec![at, at],
            vec![]
        )
        .is_err());
        for p0 in [
            BlockPos::new(0, -65, -1),
            BlockPos::new(0, 320, -1),
            pos(16, 64),
        ] {
            let e = ScopedBlockEffect::new(
                scope(),
                SchemaVersion::new(1),
                1,
                MutationActor::System,
                ScopedBlockCause::Test,
                p0,
                BlockStateId::AIR,
                BlockStateId::new(1),
                None,
                None,
            );
            if let Ok(e) = e {
                let r = BlockMutationRequest::new(
                    id(6),
                    snap.clone(),
                    vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)],
                    vec![],
                    vec![e],
                );
                if let Ok(r) = r {
                    assert!(s.commit_blocks(r).await.is_err());
                }
            }
        }
        assert!(BlockSnapshotSpec::new(scope(), generation(), None, vec![key(0), key(0)]).is_err());
        assert!(ChunkImage::new(None, None).is_err());
        for bytes in [vec![], vec![0x7f, 0, 1], vec![0; 16385]] {
            assert!(ScopedBlockEffect::new(
                scope(),
                SchemaVersion::new(1),
                1,
                MutationActor::System,
                ScopedBlockCause::Test,
                at,
                BlockStateId::AIR,
                BlockStateId::AIR,
                None,
                Some(bytes)
            )
            .is_err());
        }
    }
}
