//! Public API races and complete cell/image endpoints. No forged expectations.
mod block_support;
use block_support::*;
use ferrumc_core::{DimensionId, EntityUuid, PlayerId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_storage::*;
use ferrumc_world::{decode_block_entity, BlockStateId, Chunk, GenerationIdentity, WorldSeed};
use std::path::Path;
use std::sync::{Arc, Barrier};

fn store(durable: bool, path: &Path, scratch: &Path) -> Arc<dyn Store> {
    if durable {
        Arc::new(RedbStore::open_with_player_block_durability(path, config(), scratch).unwrap())
    } else {
        Arc::new(InMemoryStore::with_player_block_durability(config()).unwrap())
    }
}
fn keep(keys: impl IntoIterator<Item = ChunkKey>) -> Vec<ChunkImageChange> {
    keys.into_iter()
        .map(|k| ChunkImageChange::new(k, ChunkImageAction::Keep))
        .collect()
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
async fn noop(s: &dyn Store, n: u8, k: ChunkKey) -> PlayerBlockRequest {
    PlayerBlockRequest::new(
        id(n),
        s.load_owned_blocks(spec(true, vec![k])).await.unwrap(),
        p(11),
        keep([k]),
        vec![pos(k.pos().x() * 16, 64)],
        vec![],
    )
    .unwrap()
}
async fn block_change(
    s: &dyn Store,
    n: u8,
    keys: Vec<ChunkKey>,
    player_present: bool,
) -> BlockMutationRequest {
    let snap = s
        .load_owned_blocks(spec(player_present, keys.clone()))
        .await
        .unwrap();
    let mut c = snap.columns()[0].chunk().clone();
    let p0 = pos(keys[0].pos().x() * 16, 64);
    let old = c.get_block(p0).unwrap().as_u32();
    c.set_block(p0, BlockStateId::new(old + 1)).unwrap();
    let images = keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            if i == 0 {
                image(*k, c.clone())
            } else {
                ChunkImageChange::new(*k, ChunkImageAction::Keep)
            }
        })
        .collect();
    let guards = keys
        .iter()
        .skip(1)
        .map(|k| pos(k.pos().x() * 16, 64))
        .collect();
    BlockMutationRequest::new(
        id(n),
        snap,
        images,
        guards,
        vec![effect(p0, old, old + 1, None, None)],
    )
    .unwrap()
}
enum Intent {
    Joint(PlayerBlockRequest),
    Blocks(BlockMutationRequest),
    Player(PlayerOwnershipCasRequest),
    Transfer(PlayerEntityTransferRequest),
    Image(ChunkImageCasRequest),
}
async fn race(
    s: Arc<dyn Store>,
    intents: [Intent; 2],
) -> [BlockResult<OwnershipCommitDisposition>; 2] {
    tokio::task::spawn_blocking(move || {
        let barrier = Barrier::new(2);
        std::thread::scope(|threads| {
            let handles = intents.map(|intent| {
                let s = &s;
                let barrier = &barrier;
                threads.spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap();
                    barrier.wait();
                    rt.block_on(async {
                        match intent {
                            Intent::Joint(r) => {
                                s.commit_player_blocks(r).await.map(|o| o.disposition())
                            }
                            Intent::Blocks(r) => s.commit_blocks(r).await.map(|o| o.disposition()),
                            Intent::Image(r) => s
                                .compare_exchange_chunk_images(r)
                                .await
                                .map(|o| o.disposition()),
                            Intent::Player(r) => s
                                .compare_exchange_owned_player(r)
                                .await
                                .map(|o| o.disposition())
                                .map_err(Into::into),
                            Intent::Transfer(r) => s
                                .commit_player_entity_transfer(r)
                                .await
                                .map(|o| o.disposition())
                                .map_err(Into::into),
                        }
                    })
                })
            });
            handles.map(|h| h.join().unwrap())
        })
    })
    .await
    .unwrap()
}
fn conflict(e: &BlockDurabilityError) -> bool {
    matches!(
        e,
        BlockDurabilityError::CasConflict { .. }
            | BlockDurabilityError::Ownership(OwnershipError::CasConflict { .. })
    )
}
fn one_winner(results: &[BlockResult<OwnershipCommitDisposition>; 2]) {
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert!(
        results
            .iter()
            .filter_map(|r| r.as_ref().err())
            .all(conflict),
        "{results:?}"
    );
}
#[tokio::test]
async fn joint_competes_with_shared_v1_autosave_and_transfer_without_partial_loser() {
    for durable in [false, true] {
        for transfer in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let s = store(durable, &dir.path().join("race"), dir.path());
            initialize(&*s).await;
            seed_air(&*s).await;
            let player_before = s.load_owned_player(player(), generation()).await.unwrap();
            let a = joint(&*s, 3).await;
            let b = if transfer {
                let entity = PersistentEntityKey::new(
                    scope().world(),
                    scope().dimension(),
                    EntityUuid::from_bytes([0xfa; 16]),
                );
                let before = s.load_owned_entity(entity, generation()).await.unwrap();
                Intent::Transfer(
                    PlayerEntityTransferRequest::new(
                        id(4),
                        player_before.expectation().clone(),
                        before.expectation().clone(),
                        PlayerOwnershipProposal::put(p(9)),
                        EntityOwnershipProposal::put(
                            PersistentEntityRecord::new(
                                ChunkPos::new(0, -1),
                                EntityRecord::new(SchemaVersion::new(7), vec![41]).unwrap(),
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap(),
                )
            } else {
                Intent::Player(
                    PlayerOwnershipCasRequest::new(
                        id(4),
                        player_before.expectation().clone(),
                        PlayerOwnershipProposal::put(p(9)),
                    )
                    .unwrap(),
                )
            };
            let joint_query = a.receipt_query();
            let column_stamps: Vec<_> = s
                .load_owned_blocks(spec(false, vec![key(0), key(1)]))
                .await
                .unwrap()
                .columns()
                .iter()
                .map(OwnedBlockColumn::stamp)
                .collect();
            let results = race(s.clone(), [Intent::Joint(a), b]).await;
            one_winner(&results);
            let snap = s
                .load_owned_blocks(spec(true, vec![key(0), key(1)]))
                .await
                .unwrap();
            let joint_won = results[0].is_ok();
            assert_eq!(
                snap.player_record().unwrap().data(),
                player_bytes(if joint_won { 10 } else { 9 })
            );
            let cell = |i: usize, x| {
                snap.columns()[i]
                    .chunk()
                    .get_block(pos(x, 64))
                    .unwrap()
                    .as_u32()
            };
            // The losing side is whole: no cell, entity, stamp or receipt of it survives.
            assert_eq!(cell(0, 15), if joint_won { 100 } else { 0 });
            assert_eq!(cell(0, 14), if joint_won { 200 } else { 0 });
            assert_eq!(cell(1, 16), if joint_won { 300 } else { 0 });
            assert_eq!(
                snap.columns()[0].chunk().block_entity_count(),
                usize::from(joint_won)
            );
            for (column, before) in snap.columns().iter().zip(&column_stamps) {
                assert_eq!(column.stamp() == *before, !joint_won);
            }
            assert_eq!(
                s.load_block_receipt(joint_query).await.unwrap().is_some(),
                joint_won
            );
            if transfer {
                let entity = PersistentEntityKey::new(
                    scope().world(),
                    scope().dimension(),
                    EntityUuid::from_bytes([0xfa; 16]),
                );
                let after = s.load_owned_entity(entity, generation()).await.unwrap();
                assert_eq!(
                    after.record().map(|r| r.record().data().to_vec()),
                    (!joint_won).then(|| vec![41])
                );
                assert_eq!(after.stamp().is_fresh(), joint_won);
            }
        }
    }
}
/// Different Players on one Chunk: only the winner's cells and receipt exist.
async fn different_player_loser_left_no_trace(
    s: &dyn Store,
    [a_query, b_query]: [BlockReceiptQuery; 2],
    results: &[BlockResult<OwnershipCommitDisposition>; 2],
) {
    let (a_won, b_won) = (results[0].is_ok(), results[1].is_ok());
    let cells = s
        .load_owned_blocks(spec(false, vec![key(0), key(1)]))
        .await
        .unwrap();
    let cell = |i: usize, x| {
        cells.columns()[i]
            .chunk()
            .get_block(pos(x, 64))
            .unwrap()
            .as_u32()
    };
    assert_eq!(cell(0, 15), if a_won { 100 } else { 0 });
    assert_eq!(cell(1, 16), if a_won { 300 } else { 0 });
    assert_eq!(cell(0, 0), u32::from(b_won));
    assert_eq!(
        s.load_block_receipt(a_query).await.unwrap().is_some(),
        a_won
    );
    assert_eq!(
        s.load_block_receipt(b_query).await.unwrap().is_some(),
        b_won
    );
}
/// Overlapping multi-Chunk unions: exactly the winner's single increment and receipt.
async fn overlapping_union_race_leaves_only_the_winner(s: Arc<dyn Store>) {
    let overlapping_a = block_change(&*s, 10, vec![key(0), key(1)], false).await;
    let overlapping_b = block_change(&*s, 11, vec![key(0)], false).await;
    let queries = [overlapping_a.receipt_query(), overlapping_b.receipt_query()];
    let before = s
        .load_owned_blocks(spec(false, vec![key(0), key(1)]))
        .await
        .unwrap();
    let results = race(
        s.clone(),
        [Intent::Blocks(overlapping_a), Intent::Blocks(overlapping_b)],
    )
    .await;
    one_winner(&results);
    // Exactly the winner's single increment and receipt; key(1) is only guarded.
    let after = s
        .load_owned_blocks(spec(false, vec![key(0), key(1)]))
        .await
        .unwrap();
    let at = |snap: &OwnedBlockSnapshot| {
        snap.columns()[0]
            .chunk()
            .get_block(pos(0, 64))
            .unwrap()
            .as_u32()
    };
    let revision = |snap: &OwnedBlockSnapshot| {
        snap.columns()[0]
            .stamp()
            .revision()
            .map(OwnershipRevision::get)
    };
    assert_eq!(at(&after), at(&before) + 1);
    assert_eq!(revision(&after), revision(&before).map(|r| r + 1));
    assert_eq!(after.columns()[1].stamp(), before.columns()[1].stamp());
    for (query, result) in queries.into_iter().zip(&results) {
        assert_eq!(
            s.load_block_receipt(query).await.unwrap().is_some(),
            result.is_ok()
        );
    }
}
#[tokio::test]
async fn same_chunk_different_players_overlap_disjoint_unions_and_guard_drift() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = store(durable, &dir.path().join("overlap"), dir.path());
        initialize(&*s).await;
        seed_air(&*s).await;
        let second = ScopedPlayerKey::new(
            scope(),
            PlayerId::from_uuid(uuid::Uuid::from_bytes([0x1d; 16])),
        );
        let fresh = s.load_owned_player(second, generation()).await.unwrap();
        s.compare_exchange_owned_player(
            PlayerOwnershipCasRequest::new(
                id(5),
                fresh.expectation().clone(),
                PlayerOwnershipProposal::put(p(11)),
            )
            .unwrap(),
        )
        .await
        .unwrap();
        let a = joint(&*s, 3).await;
        let b_snap = s
            .load_owned_blocks(
                BlockSnapshotSpec::new(
                    scope(),
                    generation(),
                    Some(second.id()),
                    vec![key(0), key(1)],
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let mut b = b_snap.columns()[0].chunk().clone();
        b.set_block(pos(0, 64), BlockStateId::new(1)).unwrap();
        let b = PlayerBlockRequest::new(
            id(4),
            b_snap,
            p(9),
            vec![
                image(key(0), b),
                ChunkImageChange::new(key(1), ChunkImageAction::Keep),
            ],
            vec![pos(16, 64)],
            vec![effect(pos(0, 64), 0, 1, None, None)],
        )
        .unwrap();
        let (a_query, b_query) = (a.receipt_query(), b.receipt_query());
        let results = race(s.clone(), [Intent::Joint(a), Intent::Joint(b)]).await;
        one_winner(&results);
        different_player_loser_left_no_trace(&*s, [a_query, b_query], &results).await;
        let first = s.load_owned_player(player(), generation()).await.unwrap();
        let other = s.load_owned_player(second, generation()).await.unwrap();
        assert_eq!(
            first.record().unwrap().data(),
            player_bytes(if results[0].is_ok() { 10 } else { 11 })
        );
        assert_eq!(
            other.record().unwrap().data(),
            player_bytes(if results[1].is_ok() { 9 } else { 11 })
        );
        let a = block_change(&*s, 6, vec![key(0)], false).await;
        let b = block_change(&*s, 7, vec![key(1)], false).await;
        assert!(race(s.clone(), [Intent::Blocks(a), Intent::Blocks(b)])
            .await
            .iter()
            .all(std::result::Result::is_ok));
        let guarded = block_change(&*s, 8, vec![key(0), key(1)], false).await;
        let neighbor = block_change(&*s, 9, vec![key(1)], false).await;
        s.commit_blocks(neighbor).await.unwrap();
        assert!(
            matches!(s.commit_blocks(guarded).await,Err(BlockDurabilityError::CasConflict{key:Some(k),..}) if k==key(1))
        );
        // Both genuine snapshots predate the guard-only neighbor observation.
        let guarded_success = block_change(&*s, 12, vec![key(0), key(1)], false).await;
        let neighbor_success = block_change(&*s, 13, vec![key(1)], false).await;
        let neighbor_before = s
            .load_owned_blocks(spec(false, vec![key(1)]))
            .await
            .unwrap();
        s.commit_blocks(guarded_success).await.unwrap();
        let neighbor_after = s
            .load_owned_blocks(spec(false, vec![key(1)]))
            .await
            .unwrap();
        assert_eq!(
            neighbor_before.columns()[0].stamp(),
            neighbor_after.columns()[0].stamp()
        );
        s.commit_blocks(neighbor_success).await.unwrap();
        overlapping_union_race_leaves_only_the_winner(s).await;
    }
}
#[tokio::test]
async fn byte_exact_same_id_race_collision_namespace_and_foreign_authority() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = store(durable, &dir.path().join("replay"), dir.path());
        initialize(&*s).await;
        seed_air(&*s).await;
        let r = joint(&*s, 3).await;
        let query = r.receipt_query();
        let results = race(
            s.clone(),
            [Intent::Joint(r.clone()), Intent::Joint(r.clone())],
        )
        .await;
        assert!(results.iter().all(std::result::Result::is_ok));
        assert!(results.contains(&Ok(OwnershipCommitDisposition::Committed)));
        assert!(results.contains(&Ok(OwnershipCommitDisposition::Replayed)));
        assert_eq!(
            s.commit_player_blocks(r).await.unwrap().disposition(),
            OwnershipCommitDisposition::Replayed
        );
        let changed = noop(&*s, 3, key(0)).await;
        assert!(matches!(
            s.commit_player_blocks(changed).await,
            Err(BlockDurabilityError::ReceiptCollision { .. })
        ));
        let current = s.load_owned_player(player(), generation()).await.unwrap();
        let v1 = PlayerOwnershipCasRequest::new(
            id(3),
            current.expectation().clone(),
            PlayerOwnershipProposal::put(p(10)),
        )
        .unwrap();
        assert!(matches!(
            s.compare_exchange_owned_player(v1).await,
            Err(OwnershipError::ReceiptCollision { .. })
        ));
        assert!(matches!(
            s.load_block_receipt(BlockReceiptQuery::from_bytes(query.id(), [0xff; 32]))
                .await,
            Err(BlockDurabilityError::ReceiptCollision { .. })
        ));
        assert!(matches!(
            s.commit_player_blocks(noop(&*s, 1, key(0)).await).await,
            Err(BlockDurabilityError::ReceiptCollision { .. })
        ));
        let foreign = store(durable, &dir.path().join("foreign"), dir.path());
        initialize(&*foreign).await;
        seed_air(&*foreign).await;
        let r = noop(&*foreign, 77, key(0)).await;
        assert!(matches!(
            s.commit_player_blocks(r).await,
            Err(BlockDurabilityError::BackendOwnerMismatch)
        ));
        let other_seed = GenerationIdentity::flat_overworld(WorldSeed::new(-828));
        assert!(matches!(
            s.load_owned_blocks(
                BlockSnapshotSpec::new(scope(), other_seed, Some(player().id()), vec![key(0)])
                    .unwrap()
            )
            .await,
            Err(BlockDurabilityError::GenerationMismatch)
        ));
    }
}
#[allow(clippy::many_single_char_names)] // Store/request/chunk names stay short in this race fixture.
#[tokio::test]
async fn image_cas_uses_same_history_and_all_blind_block_writers_refuse_before_shape() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = store(durable, &dir.path().join("bypass"), dir.path());
        initialize(&*s).await;
        seed_air(&*s).await;
        let r = joint(&*s, 3).await;
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let mut c = snap.columns()[0].chunk().clone();
        c.set_block(pos(5, 64), BlockStateId::new(1)).unwrap();
        let cas = ChunkImageCasRequest::new(id(4), snap, vec![image(key(0), c)]).unwrap();
        s.compare_exchange_chunk_images(cas).await.unwrap();
        assert!(matches!(
            s.commit_player_blocks(r).await,
            Err(BlockDurabilityError::CasConflict { .. })
        ));
        assert!(s
            .save_chunk(
                key(0),
                ChunkRecord::new(SchemaVersion::new(1), Chunk::new(key(1).pos()))
            )
            .await
            .is_err());
        assert!(s.save_chunks(vec![]).await.is_err());
        assert!(s
            .save_chunks(vec![
                (
                    key(0),
                    ChunkRecord::new(SchemaVersion::new(1), Chunk::new(key(0).pos()))
                );
                4097
            ])
            .await
            .is_err());
        assert!(s.delete_chunk(key(0)).await.is_err());
        assert!(s.save_chunk_overlays(vec![]).await.is_err());
        assert!(s
            .save_chunk_overlays(vec![(
                key(0),
                ChunkOverlayRecord::try_from_chunk(
                    SchemaVersion::new(1),
                    key(0).pos(),
                    &Chunk::new(key(0).pos()),
                    0
                )
                .unwrap()
            )])
            .await
            .is_err());
        assert!(s.append_block_mutations(vec![]).await.is_err());
        assert!(s
            .append_block_mutation_batch(JournalBatchId::from_bytes([0xaa; 16]), vec![])
            .await
            .is_err());
        assert!(s.save_player(player().id(), p(5)).await.is_err());
        assert!(s.delete_player(player().id()).await.is_err());
        assert!(s.save_persistent_entities(vec![]).await.is_err());
        assert_eq!(
            s.load_owned_player(player(), generation())
                .await
                .unwrap()
                .record()
                .unwrap()
                .data(),
            player_bytes(11)
        );
        let a = joint(&*s, 8).await;
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let mut c = snap.columns()[0].chunk().clone();
        c.set_block(pos(5, 64), BlockStateId::new(2)).unwrap();
        let b = ChunkImageCasRequest::new(id(9), snap, vec![image(key(0), c)]).unwrap();
        one_winner(&race(s.clone(), [Intent::Joint(a), Intent::Image(b)]).await);
    }
}
#[tokio::test]
async fn hard_chunk_guard_effect_shape_boundaries_and_fresh_journal_cycle() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = store(durable, &dir.path().join("shape"), dir.path());
        initialize(&*s).await;
        let keys = (0..8).map(key).collect::<Vec<_>>();
        let snap = s
            .load_owned_blocks(spec(false, keys.clone()))
            .await
            .unwrap();
        s.compare_exchange_chunk_images(
            ChunkImageCasRequest::new(id(2), snap, keep(keys)).unwrap(),
        )
        .await
        .unwrap();
        assert!(
            BlockSnapshotSpec::new(scope(), generation(), None, (0..9).map(key).collect()).is_err()
        );
        assert!(BlockSnapshotSpec::new(scope(), generation(), None, vec![key(0), key(0)]).is_err());
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let guards = (0..4096)
            .map(|i| BlockPos::new(i % 16, 64 + i / 256, -16 + (i / 16) % 16))
            .collect::<Vec<_>>();
        let req =
            BlockMutationRequest::new(id(3), snap.clone(), keep([key(0)]), guards.clone(), vec![])
                .unwrap();
        s.commit_blocks(req).await.unwrap();
        assert!(s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap()
            .columns()[0]
            .stamp()
            .is_fresh());
        let mut too_many = guards;
        too_many.push(pos(1, 81));
        assert!(matches!(
            BlockMutationRequest::new(id(4), snap.clone(), keep([key(0)]), too_many, vec![]),
            Err(BlockDurabilityError::BatchCapacity { .. })
        ));
        let effects = (0..1024)
            .map(|i| effect(pos(1, 64), (i % 2) as u32, ((i + 1) % 2) as u32, None, None))
            .collect::<Vec<_>>();
        let mut excess = effects.clone();
        excess.push(effect(pos(1, 64), 0, 1, None, None));
        assert!(matches!(
            BlockMutationRequest::new(id(4), snap.clone(), keep([key(0)]), vec![], excess),
            Err(BlockDurabilityError::BatchCapacity { .. })
        ));
        let out = s
            .commit_blocks(
                BlockMutationRequest::new(id(5), snap, keep([key(0)]), vec![], effects).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(out.receipt().journal_range(), Some((0, 1024)));
        assert!(out.receipt().chunks()[0].1.is_tombstone());
        assert_eq!(out.receipt().chunks()[0].1.revision().unwrap().get(), 1);
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        let erased = ChunkImageCasRequest::new(
            id(6),
            snap,
            vec![ChunkImageChange::new(key(0), ChunkImageAction::Erase)],
        )
        .unwrap();
        assert!(matches!(
            s.compare_exchange_chunk_images(erased).await,
            Err(BlockDurabilityError::AlreadyAbsent { .. })
        ));
    }
}
fn collides(e: &BlockResult<BlockCommitOutcome>) -> bool {
    matches!(e, Err(BlockDurabilityError::ReceiptCollision { id: i }) if *i == id(3))
}
#[allow(clippy::too_many_lines)] // Keep one complete matrix case and its assertions together.
#[tokio::test]
async fn every_retained_field_variant_of_one_id_collides_without_any_write() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("variants");
        let s = store(durable, &path, dir.path());
        // A second real descriptor exists before any world row, so scope can vary.
        let other = GenerationKey::new(scope().world(), DimensionId::new(4));
        s.ensure_generation_identity(other, generation())
            .await
            .unwrap();
        initialize(&*s).await;
        // An earlier genuine original: Fresh flat terrain instead of the seeded air base.
        let fresh = s
            .load_owned_blocks(spec(true, vec![key(0), key(1)]))
            .await
            .unwrap();
        let fresh_original = joint_variant(id(3), fresh, Variant::default());
        seed_air(&*s).await;
        let snap = s
            .load_owned_blocks(spec(true, vec![key(0), key(1)]))
            .await
            .unwrap();
        let original = joint_variant(id(3), snap.clone(), Variant::default());
        let base = Variant::default();
        let mut variants = vec![fresh_original];
        for v in [
            Variant {
                post_schema: 58,
                ..base
            },
            Variant { chest: 9, ..base },
            Variant { player: 12, ..base },
            Variant {
                meta: Meta {
                    schema: 98,
                    ..base.meta
                },
                ..base
            },
            Variant {
                meta: Meta {
                    tick: 482,
                    ..base.meta
                },
                ..base
            },
            Variant {
                meta: Meta {
                    cause: ScopedBlockCause::Command,
                    ..base.meta
                },
                ..base
            },
            Variant { swap: true, ..base },
        ] {
            variants.push(joint_variant(id(3), snap.clone(), v));
        }
        for r in &variants {
            assert_ne!(r.receipt_query(), original.receipt_query());
            assert_eq!(r.receipt_query().id(), id(3));
        }
        // Same images/guards/effects, kind 2 instead of joint kind 1.
        let kind_snap = s
            .load_owned_blocks(spec(false, vec![key(0), key(1)]))
            .await
            .unwrap();
        let (images, guards, effects) = joint_parts(&kind_snap, base);
        let kind = BlockMutationRequest::new(id(3), kind_snap, images, guards, effects).unwrap();
        let image_kind = ChunkImageCasRequest::new(
            id(3),
            s.load_owned_blocks(spec(false, vec![key(0)]))
                .await
                .unwrap(),
            vec![ChunkImageChange::new(key(0), ChunkImageAction::Keep)],
        )
        .unwrap();
        let other_key = ChunkKey::new(other.world(), other.dimension(), ChunkPos::new(0, -1));
        let other_snap = s
            .load_owned_blocks(
                BlockSnapshotSpec::new(other, generation(), None, vec![other_key]).unwrap(),
            )
            .await
            .unwrap();
        let mut c = other_snap.columns()[0].chunk().clone();
        let at = pos(1, 64);
        let old = c.get_block(at).unwrap().as_u32();
        c.set_block(at, BlockStateId::new(old + 1)).unwrap();
        let scoped = BlockMutationRequest::new(
            id(3),
            other_snap,
            vec![ChunkImageChange::new(
                other_key,
                ChunkImageAction::Replace(
                    ChunkImage::new(
                        Some(ChunkRecord::try_new(SchemaVersion::new(55), c).unwrap()),
                        None,
                    )
                    .unwrap(),
                ),
            )],
            vec![],
            vec![effect_in(
                other,
                Meta::default(),
                at,
                old,
                old + 1,
                None,
                None,
            )],
        )
        .unwrap();
        let committed = s.commit_player_blocks(original.clone()).await.unwrap();
        let before = s
            .load_owned_blocks(spec(true, vec![key(0), key(1)]))
            .await
            .unwrap();
        for r in variants {
            assert!(collides(&s.commit_player_blocks(r).await));
        }
        assert!(collides(&s.commit_blocks(kind).await));
        assert!(collides(&s.compare_exchange_chunk_images(image_kind).await));
        assert!(collides(&s.commit_blocks(scoped).await));
        let after = s
            .load_owned_blocks(spec(true, vec![key(0), key(1)]))
            .await
            .unwrap();
        assert_eq!(after.player_stamp(), before.player_stamp());
        for (a, b) in after.columns().iter().zip(before.columns()) {
            assert_eq!(a.stamp(), b.stamp());
            assert_eq!(a.full_bytes(), b.full_bytes());
            assert_eq!(a.overlay_bytes(), b.overlay_bytes());
        }
        assert_eq!(
            s.load_block_receipt(original.receipt_query())
                .await
                .unwrap()
                .as_ref(),
            Some(committed.receipt())
        );
        let replay = s.commit_player_blocks(original).await.unwrap();
        assert_eq!(replay.disposition(), OwnershipCommitDisposition::Replayed);
        // The shared namespace also refuses the other direction: FCTR via the block lookup.
        assert_eq!(
            s.load_block_receipt(BlockReceiptQuery::from_bytes(id(1), [0; 32]))
                .await
                .unwrap_err(),
            BlockDurabilityError::ReceiptKindMismatch { id: id(1) }
        );
        if durable {
            drop(s);
            let reopened =
                RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
            assert_eq!(
                reopened
                    .load_block_receipt(committed.receipt().query())
                    .await
                    .unwrap()
                    .as_ref(),
                Some(committed.receipt())
            );
        }
    }
}
#[tokio::test]
async fn stale_snapshot_and_historical_receipt_cannot_authorize_a_new_id() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = store(durable, &dir.path().join("stale"), dir.path());
        initialize(&*s).await;
        seed_air(&*s).await;
        let stale = s
            .load_owned_blocks(spec(true, vec![key(0), key(1)]))
            .await
            .unwrap();
        let committed = s
            .commit_player_blocks(joint_variant(id(3), stale.clone(), Variant::default()))
            .await
            .unwrap();
        // The receipt names historical stamps/images; only a new coherent snapshot authorizes.
        assert_eq!(
            committed
                .receipt()
                .after_player_stamp()
                .revision()
                .unwrap()
                .get(),
            2
        );
        let replayed_shape = joint_variant(
            id(20),
            stale.clone(),
            Variant {
                chest: 9,
                ..Variant::default()
            },
        );
        assert!(matches!(
            s.commit_player_blocks(replayed_shape).await,
            Err(BlockDurabilityError::CasConflict {
                resource: "Player",
                ..
            })
        ));
        // Without the Player, the same stale original fails on the sorted Chunk stamp.
        let chunk_only = s
            .load_owned_blocks(spec(false, vec![key(0)]))
            .await
            .unwrap();
        s.compare_exchange_chunk_images(
            ChunkImageCasRequest::new(id(21), chunk_only.clone(), keep([key(0)])).unwrap(),
        )
        .await
        .unwrap();
        let mut c = chunk_only.columns()[0].chunk().clone();
        c.set_block(pos(3, 64), BlockStateId::new(7)).unwrap();
        s.compare_exchange_chunk_images(
            ChunkImageCasRequest::new(id(22), chunk_only.clone(), vec![image(key(0), c.clone())])
                .unwrap(),
        )
        .await
        .unwrap();
        assert!(matches!(
            s.compare_exchange_chunk_images(
                ChunkImageCasRequest::new(id(23), chunk_only, vec![image(key(0), c)]).unwrap()
            )
            .await,
            Err(BlockDurabilityError::CasConflict {
                resource: "Chunk",
                ..
            })
        ));
        assert!(s
            .load_block_receipt(BlockReceiptQuery::from_bytes(id(20), [0; 32]))
            .await
            .unwrap()
            .is_none());
    }
}
#[allow(clippy::too_many_lines)] // Keep one complete matrix case and its assertions together.
#[tokio::test]
async fn union_requires_every_key_to_be_written_guarded_or_effected() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = store(durable, &dir.path().join("union"), dir.path());
        initialize(&*s).await;
        seed_air(&*s).await;
        let snap = s
            .load_owned_blocks(spec(false, vec![key(0), key(1)]))
            .await
            .unwrap();
        let keep_only = BlockMutationRequest::new(
            id(3),
            snap.clone(),
            keep([key(0), key(1)]),
            vec![pos(1, 64)],
            vec![],
        )
        .unwrap();
        assert!(matches!(
            s.commit_blocks(keep_only).await,
            Err(BlockDurabilityError::MalformedData { table: "union", .. })
        ));
        let uncovered_guard = BlockMutationRequest::new(
            id(4),
            snap.clone(),
            keep([key(0), key(1)]),
            vec![pos(1, 64), pos(17, 64), pos(33, 64)],
            vec![],
        )
        .unwrap();
        assert!(matches!(
            s.commit_blocks(uncovered_guard).await,
            Err(BlockDurabilityError::InvalidEffect {
                reason: "uncovered guard",
                ..
            })
        ));
        let uncovered_effect = BlockMutationRequest::new(
            id(5),
            snap.clone(),
            keep([key(0), key(1)]),
            vec![pos(1, 64), pos(17, 64)],
            vec![effect(pos(33, 64), 0, 1, None, None)],
        )
        .unwrap();
        assert!(matches!(
            s.commit_blocks(uncovered_effect).await,
            Err(BlockDurabilityError::InvalidEffect {
                reason: "uncovered effect",
                ..
            })
        ));
        // Proposals must name exactly the snapshot keys, once each.
        assert!(
            BlockMutationRequest::new(id(6), snap.clone(), keep([key(0)]), vec![], vec![]).is_err()
        );
        assert!(BlockMutationRequest::new(
            id(6),
            snap.clone(),
            keep([key(0), key(0)]),
            vec![],
            vec![]
        )
        .is_err());
        assert!(BlockMutationRequest::new(
            id(6),
            snap.clone(),
            keep([key(0), key(2)]),
            vec![],
            vec![]
        )
        .is_err());
        // A Player snapshot cannot drive the no-Player kinds, and vice versa.
        let with_player = s.load_owned_blocks(spec(true, vec![key(0)])).await.unwrap();
        assert!(matches!(
            BlockMutationRequest::new(
                id(6),
                with_player.clone(),
                keep([key(0)]),
                vec![pos(1, 64)],
                vec![]
            ),
            Err(BlockDurabilityError::ScopeMismatch)
        ));
        assert!(matches!(
            ChunkImageCasRequest::new(id(6), with_player, keep([key(0)])),
            Err(BlockDurabilityError::ScopeMismatch)
        ));
        assert!(matches!(
            PlayerBlockRequest::new(
                id(6),
                snap,
                p(1),
                keep([key(0), key(1)]),
                vec![pos(1, 64), pos(17, 64)],
                vec![]
            ),
            Err(BlockDurabilityError::ScopeMismatch)
        ));
        for n in 3..=6 {
            assert!(s
                .load_block_receipt(BlockReceiptQuery::from_bytes(id(n), [0; 32]))
                .await
                .unwrap()
                .is_none());
        }
    }
}
/// Fixed versioned sign payload: waxed, glowing front, four lines per face.
fn sign_bytes(line: &str) -> Vec<u8> {
    let mut b = vec![0x7f, 0, 1, 0, 0, 1];
    for (color, glowing, first) in [("black", 1u8, line), ("white", 0, "")] {
        b.extend_from_slice(&(color.len() as u16).to_be_bytes());
        b.extend_from_slice(color.as_bytes());
        b.push(glowing);
        for text in [first, "", "two", ""] {
            b.extend_from_slice(&(text.len() as u16).to_be_bytes());
            b.extend_from_slice(text.as_bytes());
        }
    }
    b
}
#[tokio::test]
async fn sign_text_edit_is_an_exact_entity_only_delta_with_retained_journal_bytes() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let s = store(durable, &dir.path().join("sign"), dir.path());
        initialize(&*s).await;
        seed_air(&*s).await;
        let at = pos(3, 70);
        let mut put = Vec::new();
        for (n, old_entity, new_entity, old_state, new_state) in [
            (3u8, None, Some(sign_bytes("Hi")), 0u32, 4300u32),
            (
                4,
                Some(sign_bytes("Hi")),
                Some(sign_bytes("Bye")),
                4300,
                4300,
            ),
        ] {
            let snap = s.load_owned_blocks(spec(true, vec![key(0)])).await.unwrap();
            let mut c = snap.columns()[0].chunk().clone();
            c.set_block(at, BlockStateId::new(new_state)).unwrap();
            c.set_block_entity(
                at,
                decode_block_entity(new_entity.as_ref().unwrap()).unwrap(),
            )
            .unwrap();
            let out = s
                .commit_player_blocks(
                    PlayerBlockRequest::new(
                        id(n),
                        snap,
                        p(11),
                        vec![image(key(0), c)],
                        vec![],
                        vec![effect(
                            at,
                            old_state,
                            new_state,
                            old_entity,
                            new_entity.clone(),
                        )],
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            // Player bytes are unchanged, so only the Chunk advances.
            assert_eq!(
                out.receipt().after_player_stamp().revision().unwrap().get(),
                1
            );
            put.push(out);
        }
        let snap = s.load_owned_blocks(spec(true, vec![key(0)])).await.unwrap();
        let mut encoded = Vec::new();
        ferrumc_world::try_encode_block_entity(
            snap.columns()[0].chunk().block_entity(at).unwrap(),
            &mut encoded,
        )
        .unwrap();
        assert_eq!(encoded, sign_bytes("Bye"));
        assert_eq!(snap.columns()[0].stamp().revision().unwrap().get(), 3);
        assert_eq!(put[1].receipt().journal_range(), Some((1, 1)));
        // The retained Q ends with exactly this effect: old/new sign blobs verbatim.
        let q = put[1].receipt().request_bytes();
        let mut tail = Vec::new();
        for b in [sign_bytes("Hi"), sign_bytes("Bye")] {
            tail.push(1);
            tail.extend_from_slice(&(b.len() as u32).to_be_bytes());
            tail.extend_from_slice(&b);
        }
        assert!(q.ends_with(&tail));
    }
}
