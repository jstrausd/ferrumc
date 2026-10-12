//! Public N/N+1 retained capacities, local work/output/scan caps and explicit mode/format refusals.
//! Byte boundaries come from the fixed independent fixtures, never from runtime arithmetic.
mod block_support;
use block_support::*;
use ferrumc_core::ServerError;
use ferrumc_storage::*;
use ferrumc_world::{BlockStateId, Chunk, FlatWorldGenerator};
use redb::{Database, ReadableTable, TableDefinition, TableHandle};
use std::path::Path;
use std::sync::Arc;

const SEED_FCBR: &[u8] = include_bytes!("fixtures/player_block_format7/seed.fcbr");
const JOINT_FCBQ: &[u8] = include_bytes!("fixtures/player_block_format7/joint.fcbq");
const JOINT_FCBR: &[u8] = include_bytes!("fixtures/player_block_format7/joint.fcbr");

fn open(durable: bool, path: &Path, scratch: &Path, caps: Caps) -> Arc<dyn Store> {
    let c = config_with(caps).unwrap();
    if durable {
        Arc::new(RedbStore::open_with_player_block_durability(path, c, scratch).unwrap())
    } else {
        Arc::new(InMemoryStore::with_player_block_durability(c).unwrap())
    }
}
/// v1 bootstrap FCTR (272) + image-CAS seed and joint FCBR rows plus their 16-byte keys.
fn joint_receipt_bytes() -> u64 {
    272 + 16 + SEED_FCBR.len() as u64 + 16 + JOINT_FCBR.len() as u64
}
fn fixed_journal_bytes() -> u64 {
    meta(
        &fixed_rows(include_str!("fixtures/player_block_format7/after17.tsv")),
        "scoped_journal_bytes",
    )
}
/// One more changed cell in an already-live column, with one journal effect.
async fn one_effect(s: &dyn Store, n: u8) -> BlockResult<BlockCommitOutcome> {
    let snap = s
        .load_owned_blocks(spec(false, vec![key(0)]))
        .await
        .unwrap();
    let mut c = snap.columns()[0].chunk().clone();
    let at = pos(2, 90);
    let old = c.get_block(at).unwrap().as_u32();
    c.set_block(at, BlockStateId::new(old + 1)).unwrap();
    s.commit_blocks(
        BlockMutationRequest::new(
            id(n),
            snap,
            vec![replace(key(0), c)],
            vec![],
            vec![effect(at, old, old + 1, None, None)],
        )
        .unwrap(),
    )
    .await
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
async fn stamps(s: &dyn Store) -> Vec<OwnershipStamp> {
    let snap = s
        .load_owned_blocks(spec(true, vec![key(0), key(1), key(2)]))
        .await
        .unwrap();
    let mut out = vec![snap.player_stamp()];
    out.extend(snap.columns().iter().map(OwnedBlockColumn::stamp));
    out
}
fn history(e: &BlockResult<BlockCommitOutcome>, what: &str) -> bool {
    matches!(e, Err(BlockDurabilityError::HistoryCapacity { resource, .. }) if *resource == what)
}
async fn refused_without_write(
    s: &dyn Store,
    attempt: BlockResult<BlockCommitOutcome>,
    what: &str,
    before: &[OwnershipStamp],
    n: u8,
) {
    assert!(history(&attempt, what), "{what}: {attempt:?}");
    assert_eq!(stamps(s).await, before, "{what}");
    assert!(s
        .load_block_receipt(BlockReceiptQuery::from_bytes(id(n), [0; 32]))
        .await
        .unwrap()
        .is_none());
}

#[allow(clippy::too_many_lines)] // Keep one complete matrix case and its assertions together.
#[tokio::test]
async fn every_persisted_row_and_byte_capacity_admits_n_and_refuses_n_plus_one() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let run = |name: &str, caps: Caps| open(durable, &dir.path().join(name), dir.path(), caps);

        // Chunk history rows: two seeded rows are N; a third new column is N+1.
        let s = run(
            "chunks",
            Caps {
                chunks: 2,
                ..Caps::default()
            },
        );
        initialize(&*s).await;
        seed_air(&*s).await;
        let before = stamps(&*s).await;
        let snap = s
            .load_owned_blocks(spec(false, vec![key(2)]))
            .await
            .unwrap();
        let mut c = snap.columns()[0].chunk().clone();
        c.set_block(pos(33, 64), BlockStateId::new(1)).unwrap();
        let attempt = s
            .compare_exchange_chunk_images(
                ChunkImageCasRequest::new(id(9), snap.clone(), vec![replace(key(2), c)]).unwrap(),
            )
            .await;
        refused_without_write(&*s, attempt, "Chunk rows", &before, 9).await;
        // A Fresh no-op creates no history row and remains admitted at the cap.
        s.compare_exchange_chunk_images(
            ChunkImageCasRequest::new(
                id(10),
                snap,
                vec![ChunkImageChange::new(key(2), ChunkImageAction::Keep)],
            )
            .unwrap(),
        )
        .await
        .unwrap();
        s.commit_player_blocks(joint(&*s, 3).await).await.unwrap();

        // Shared receipt rows: bootstrap, seed and joint are N=3.
        let s = run(
            "receipts",
            Caps {
                receipts: 3,
                ..Caps::default()
            },
        );
        initialize(&*s).await;
        seed_air(&*s).await;
        s.commit_player_blocks(joint(&*s, 3).await).await.unwrap();
        let before = stamps(&*s).await;
        refused_without_write(&*s, one_effect(&*s, 4).await, "receipt rows", &before, 4).await;

        // Common FCTR/FCBR bytes, from the fixed fixture rows.
        for (cap, joint_fits) in [
            (joint_receipt_bytes(), true),
            (joint_receipt_bytes() - 1, false),
        ] {
            let s = run(
                &format!("bytes-{cap}"),
                Caps {
                    receipt_bytes: cap,
                    ..Caps::default()
                },
            );
            initialize(&*s).await;
            seed_air(&*s).await;
            let before = stamps(&*s).await;
            let r = joint(&*s, 3).await;
            assert_eq!(r.receipt_query().id(), id(3));
            let out = s.commit_player_blocks(r).await;
            if joint_fits {
                assert_eq!(out.unwrap().receipt().encoded_bytes(), JOINT_FCBR);
                let before = stamps(&*s).await;
                refused_without_write(&*s, one_effect(&*s, 4).await, "receipt bytes", &before, 4)
                    .await;
                // A v1 no-op still needs its 272 shared bytes.
                let current = s.load_owned_player(player(), generation()).await.unwrap();
                let v1 = s
                    .compare_exchange_owned_player(
                        PlayerOwnershipCasRequest::new(
                            id(5),
                            current.expectation().clone(),
                            PlayerOwnershipProposal::put(p(10)),
                        )
                        .unwrap(),
                    )
                    .await;
                assert!(
                    matches!(&v1, Err(OwnershipError::Block(e)) if matches!(**e, BlockDurabilityError::HistoryCapacity { resource: "receipt bytes", .. })),
                    "{v1:?}"
                );
            } else {
                refused_without_write(&*s, out, "receipt bytes", &before, 3).await;
            }
        }

        // Scoped journal rows and bytes: the fixed joint carries four effects.
        for (caps, name) in [
            (
                Caps {
                    journal_rows: 4,
                    ..Caps::default()
                },
                "journal rows",
            ),
            (
                Caps {
                    journal_bytes: fixed_journal_bytes(),
                    ..Caps::default()
                },
                "journal bytes",
            ),
        ] {
            let s = run(name, caps);
            initialize(&*s).await;
            seed_air(&*s).await;
            s.commit_player_blocks(joint(&*s, 3).await).await.unwrap();
            let before = stamps(&*s).await;
            refused_without_write(&*s, one_effect(&*s, 4).await, name, &before, 4).await;
            let tight = if name == "journal rows" {
                Caps {
                    journal_rows: 3,
                    ..Caps::default()
                }
            } else {
                Caps {
                    journal_bytes: fixed_journal_bytes() - 1,
                    ..Caps::default()
                }
            };
            let s = run(&format!("{name}-tight"), tight);
            initialize(&*s).await;
            seed_air(&*s).await;
            let before = stamps(&*s).await;
            let attempt = s.commit_player_blocks(joint(&*s, 3).await).await;
            refused_without_write(&*s, attempt, name, &before, 3).await;
        }

        // Persisted canonical request bytes: exactly the fixed joint Q is N.
        let s = run(
            "request",
            Caps {
                request: JOINT_FCBQ.len(),
                ..Caps::default()
            },
        );
        initialize(&*s).await;
        seed_air(&*s).await;
        let r = joint(&*s, 3).await;
        assert_eq!(
            s.commit_player_blocks(r)
                .await
                .unwrap()
                .receipt()
                .request_bytes(),
            JOINT_FCBQ
        );
        let s = run(
            "request-tight",
            Caps {
                request: JOINT_FCBQ.len() - 1,
                ..Caps::default()
            },
        );
        initialize(&*s).await;
        seed_air(&*s).await;
        let snap = s
            .load_owned_blocks(spec(true, vec![key(0), key(1)]))
            .await
            .unwrap();
        let (images, guards, effects) = joint_parts(&snap, Variant::default());
        assert!(matches!(
            PlayerBlockRequest::new(id(3), snap, p(10), images, guards, effects),
            Err(BlockDurabilityError::ByteCapacity { resource: "request", len, max })
                if len == JOINT_FCBQ.len() as u64 && max == JOINT_FCBQ.len() as u64 - 1
        ));
    }
}

#[tokio::test]
async fn persisted_agreement_is_immutable_and_capacity_constructors_refuse_overflow() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agreement");
    let s = open(true, &path, dir.path(), Caps::default());
    initialize(&*s).await;
    drop(s);
    let before = any_rows(&path);
    for caps in [
        Caps {
            request: MAX_BLOCK_REQUEST_BYTES - 1,
            ..Caps::default()
        },
        Caps {
            chunks: 999,
            ..Caps::default()
        },
        Caps {
            receipt_bytes: 1,
            ..Caps::default()
        },
        Caps {
            journal_rows: 1,
            ..Caps::default()
        },
        Caps {
            journal_bytes: 1,
            ..Caps::default()
        },
        Caps {
            receipts: 999,
            ..Caps::default()
        },
        Caps {
            revisions: 999,
            ..Caps::default()
        },
    ] {
        let opened = RedbStore::open_with_player_block_durability(
            &path,
            config_with(caps).unwrap(),
            dir.path(),
        );
        assert!(
            matches!(opened, Err(BlockDurabilityError::CapacityContractMismatch)),
            "{caps:?}: {:?}",
            opened.err()
        );
        assert_eq!(any_rows(&path), before);
    }
    for caps in [
        Caps {
            request: 143,
            ..Caps::default()
        },
        Caps {
            request: MAX_BLOCK_REQUEST_BYTES + 1,
            ..Caps::default()
        },
        Caps {
            receipt_bytes: u64::MAX,
            ..Caps::default()
        },
        Caps {
            chunks: u64::MAX / 47 + 1,
            ..Caps::default()
        },
        Caps {
            bytes: 512 * 1024 * 1024 + 1,
            ..Caps::default()
        },
        Caps {
            cells: 16_777_217,
            ..Caps::default()
        },
        Caps {
            snapshot: 32 * 1024 * 1024 + 1,
            ..Caps::default()
        },
    ] {
        assert!(
            matches!(
                config_with(caps),
                Err(BlockDurabilityError::InvalidConfiguration(_))
            ),
            "{caps:?}"
        );
    }
}

#[tokio::test]
async fn runtime_work_and_snapshot_output_refuse_typed_at_an_exact_admission_boundary() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("work");
        let prepare = |caps: Caps| {
            let path = path.clone();
            let scratch = dir.path().to_path_buf();
            async move {
                if durable {
                    if !path.exists() {
                        let s = open(true, &path, &scratch, Caps::default());
                        initialize(&*s).await;
                        seed_air(&*s).await;
                        s.commit_player_blocks(joint(&*s, 3).await).await.unwrap();
                    }
                    open(true, &path, &scratch, caps)
                } else {
                    // Memory limits apply from construction, so its identical state per limit
                    // is the real bootstrap Player plus two Fresh flat columns.
                    let s = open(false, &path, &scratch, caps);
                    initialize(&*s).await;
                    s
                }
            }
        };
        let full = || spec(true, vec![key(0), key(1)]);
        if durable {
            for caps in [
                Caps {
                    cells: 98_303,
                    ..Caps::default()
                },
                Caps {
                    bytes: 4096,
                    ..Caps::default()
                },
            ] {
                let s = prepare(caps).await;
                let result = s.load_owned_blocks(full()).await;
                assert!(
                    matches!(result, Err(BlockDurabilityError::ValidationCapacity(_))),
                    "{caps:?}: {:?}",
                    result.err()
                );
                drop(s);
            }
        } else {
            let s = prepare(Caps {
                cells: 98_303,
                ..Caps::default()
            })
            .await;
            assert!(matches!(
                s.load_owned_blocks(full()).await,
                Err(BlockDurabilityError::ValidationCapacity("cells"))
            ));
        }
        // Raise the limit to each reported requirement; the admitted value is exact.
        let mut limit = 1;
        let admitted = loop {
            let s = prepare(Caps {
                snapshot: limit,
                ..Caps::default()
            })
            .await;
            match s.load_owned_blocks(full()).await {
                Ok(_) => break limit,
                Err(BlockDurabilityError::ByteCapacity {
                    resource: "snapshot",
                    len,
                    max,
                }) => {
                    assert_eq!(max, limit);
                    assert!(len > limit);
                    limit = len;
                }
                Err(e) => panic!("{e:?}"),
            }
        };
        let s = prepare(Caps {
            snapshot: admitted - 1,
            ..Caps::default()
        })
        .await;
        assert!(matches!(
            s.load_owned_blocks(full()).await,
            Err(BlockDurabilityError::ByteCapacity { resource: "snapshot", len, .. }) if len == admitted
        ));
    }
}

/// Memory's bootstrap Player plus two Fresh flat columns, with expectations sized from the
/// stored envelope and the World generator, not from Storage's reported requirement.
#[tokio::test]
async fn memory_snapshot_output_and_cells_match_independent_sizes_exactly() {
    let store = |caps: Caps| async move {
        let s = open(false, Path::new("unused"), Path::new("unused"), caps);
        initialize(&*s).await;
        s
    };
    let full = || spec(true, vec![key(0), key(1)]);
    // Output: 35 header, Player 55 + raw row + decoded copy, per column 47 + raw pair
    // (none) + view (49 + 4 bytes per cell of each non-empty section, no entities).
    let flat = FlatWorldGenerator::with_seed(generation().seed()).generate(key(0).pos());
    let sections = flat.sections().iter().filter(|s| !s.is_empty()).count() as u64;
    let view = 49 + 4 * 4096 * sections;
    let output = 35 + 55 + 2 * player_envelope(11).len() as u64 + 2 * (47 + view);
    let snapshot = |snapshot| Caps {
        snapshot,
        ..Caps::default()
    };
    store(snapshot(output))
        .await
        .load_owned_blocks(full())
        .await
        .unwrap();
    assert_eq!(
        store(snapshot(output - 1))
            .await
            .load_owned_blocks(full())
            .await
            .err(),
        Some(BlockDurabilityError::ByteCapacity {
            resource: "snapshot",
            len: output,
            max: output - 1
        })
    );
    // Cells: per Fresh column one generated base plus one column to size its view.
    let need = 2 * (COLUMN + COLUMN);
    let cells = |cells| Caps {
        cells,
        ..Caps::default()
    };
    store(cells(need))
        .await
        .load_owned_blocks(full())
        .await
        .unwrap();
    assert_eq!(
        store(cells(need - 1))
            .await
            .load_owned_blocks(full())
            .await
            .err(),
        Some(BlockDurabilityError::ValidationCapacity("cells"))
    );
}

/// Every table/row of any `FerrumC` redb catalog, read independently after release.
fn any_rows(path: &Path) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let db = Database::open(path).unwrap();
    let t = db.begin_read().unwrap();
    let mut out = Vec::new();
    for d in t.list_tables().unwrap() {
        let name = d.name().to_string();
        if let Ok(table) = t.open_table(TableDefinition::<&[u8], &[u8]>::new(&name)) {
            for row in table.iter().unwrap() {
                let (k, v) = row.unwrap();
                out.push((name.clone(), k.value().to_vec(), v.value().to_vec()));
            }
        } else {
            for row in t
                .open_table(TableDefinition::<&str, u64>::new(&name))
                .unwrap()
                .iter()
                .unwrap()
            {
                let (k, v) = row.unwrap();
                out.push((
                    name.clone(),
                    k.value().as_bytes().to_vec(),
                    v.value().to_be_bytes().to_vec(),
                ));
            }
        }
    }
    assert_eq!(t.list_multimap_tables().unwrap().count(), 0);
    out
}

#[tokio::test]
async fn reopen_scan_rows_bytes_relation_work_and_scratch_extent_refuse_without_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scan");
    let scratch = dir.path().join("scratch");
    std::fs::create_dir(&scratch).unwrap();
    let s = open(true, &path, &scratch, Caps::default());
    initialize(&*s).await;
    seed_air(&*s).await;
    s.commit_player_blocks(joint(&*s, 3).await).await.unwrap();
    drop(s);
    let rows = any_rows(&path);
    let count = rows.len() as u64;
    let bytes: u64 = rows
        .iter()
        .map(|(_, k, v)| (k.len() + v.len()) as u64)
        .sum();
    for (caps, admitted) in [
        (
            Caps {
                scan_rows: count,
                ..Caps::default()
            },
            true,
        ),
        (
            Caps {
                scan_rows: count - 1,
                ..Caps::default()
            },
            false,
        ),
        (
            Caps {
                scan_bytes: bytes,
                ..Caps::default()
            },
            true,
        ),
        (
            Caps {
                scan_bytes: bytes - 1,
                ..Caps::default()
            },
            false,
        ),
        (
            Caps {
                relation_bytes: 4096,
                ..Caps::default()
            },
            false,
        ),
        (
            Caps {
                relation_cells: 98_303,
                ..Caps::default()
            },
            false,
        ),
        (
            Caps {
                scratch: 1,
                ..Caps::default()
            },
            false,
        ),
    ] {
        let opened = RedbStore::open_with_player_block_durability(
            &path,
            config_with(caps).unwrap(),
            &scratch,
        );
        match (admitted, opened) {
            (true, Ok(s)) => drop(s),
            (
                false,
                Err(BlockDurabilityError::ScanCapacity(_) | BlockDurabilityError::ScratchCapacity),
            ) => {}
            (_, other) => panic!("{caps:?}: {:?}", other.err()),
        }
        assert_eq!(any_rows(&path), rows, "{caps:?}");
        assert_eq!(std::fs::read_dir(&scratch).unwrap().count(), 0, "{caps:?}");
    }
}

async fn assert_unsupported(s: &dyn PlayerBlockDurabilityStore, r: &PlayerBlockRequest) {
    assert_eq!(
        s.load_owned_blocks(spec(true, vec![key(0)]))
            .await
            .unwrap_err(),
        BlockDurabilityError::Unsupported
    );
    assert_eq!(
        s.load_block_receipt(r.receipt_query()).await.unwrap_err(),
        BlockDurabilityError::Unsupported
    );
    assert_eq!(
        s.commit_player_blocks(r.clone()).await.unwrap_err(),
        BlockDurabilityError::Unsupported
    );
}

#[tokio::test]
async fn other_modes_and_formats_refuse_every_block_method_and_each_opener_preserves_rows() {
    let dir = tempfile::tempdir().unwrap();
    let source = InMemoryStore::with_player_block_durability(config()).unwrap();
    initialize(&source).await;
    seed_air(&source).await;
    let genuine = joint(&source, 3).await;
    let owned = config().ownership_config();
    assert_unsupported(&InMemoryStore::new(), &genuine).await;
    assert_unsupported(&InMemoryStore::with_generation_metadata(), &genuine).await;
    assert_unsupported(&InMemoryStore::with_uuid_entities(), &genuine).await;
    assert_unsupported(
        &InMemoryStore::with_player_entity_ownership(owned).unwrap(),
        &genuine,
    )
    .await;
    let paths = [3u64, 4, 5, 6].map(|n| dir.path().join(format!("format{n}")));
    {
        let stores: Vec<RedbStore> = vec![
            RedbStore::open(&paths[0]).unwrap(),
            RedbStore::open_with_generation_metadata(&paths[1]).unwrap(),
            RedbStore::open_with_uuid_entities(&paths[2]).unwrap(),
            RedbStore::open_with_player_entity_ownership(&paths[3], owned).unwrap(),
        ];
        for s in &stores {
            assert_unsupported(s, &genuine).await;
        }
    }
    // Empty existing formats 3..=6 need the separate migration, never an upgrade.
    for (n, path) in (3u64..).zip(&paths) {
        let before = any_rows(path);
        assert!(
            matches!(
                RedbStore::open_with_player_block_durability(path, config(), dir.path()),
                Err(BlockDurabilityError::MigrationRequired { actual }) if actual == n
            ),
            "format{n}"
        );
        assert_eq!(any_rows(path), before, "format{n}");
    }
    let seven = dir.path().join("format7");
    let s = open(true, &seven, dir.path(), Caps::default());
    initialize(&*s).await;
    seed_air(&*s).await;
    drop(s);
    let before = any_rows(&seven);
    assert!(RedbStore::open_with_player_entity_ownership(&seven, owned).is_err());
    assert!(RedbStore::open(&seven).is_err());
    assert!(RedbStore::open_with_generation_metadata(&seven).is_err());
    assert!(RedbStore::open_with_uuid_entities(&seven).is_err());
    assert_eq!(any_rows(&seven), before);
    // Format7 refuses every blind legacy writer with a typed Unsupported class.
    let s = open(true, &seven, dir.path(), Caps::default());
    for result in [
        s.save_chunks(vec![]).await,
        s.save_chunk_overlays(vec![]).await,
        s.append_block_mutations(vec![]).await,
        s.delete_chunk(key(0)).await.map(|_| ()),
        s.append_block_mutation_batch(JournalBatchId::from_bytes([1; 16]), vec![])
            .await
            .map(|_| ()),
        s.save_player(player().id(), p(1)).await,
        s.delete_player(player().id()).await.map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(ServerError::Unsupported(_))),
            "{result:?}"
        );
    }
    drop(s);
    assert_eq!(any_rows(&seven), before);
}

/// Cells of one 16×16×384 column.
const COLUMN: u64 = 16 * 16 * 384;
/// Independent runtime cell-work model (INVARIANTS "Runtime cell work"): composing a pair
/// costs one column for its base (full record or flat generator) plus, with an overlay,
/// one column to decode and one to apply it; a gameplay column adds 4 per effect, 2 per
/// guard and a two-sided whole-column comparison. A read proves only the touched column's
/// own entry of its last receipt and reuses that postimage as its view (one more column to
/// size it). Probe 4's autosave history: Fresh before, overlay-only after, image-CAS.
const OVERLAY_HISTORY_PROOF: u64 = COLUMN + 3 * COLUMN;
const SNAPSHOT_COLUMN: u64 = OVERLAY_HISTORY_PROOF + COLUMN;
/// `union_commit`: each column's history proof plus its own gameplay proof
/// (overlay-only before, full-only after, one effect, whole-column comparison).
const UNION_COMMIT: u64 = 8 * (OVERLAY_HISTORY_PROOF + 3 * COLUMN + COLUMN + 4 + 2 * COLUMN);
/// Worst admissible column under the hard shape caps: full base whose 4096 entities a
/// sectioned overlay replaces (3 columns + 2·4096), on both sides of a gameplay proof with
/// 4096 guards, 1024 effects and 4096 entities per side, plus a view of 4096 entities.
const WORST_PAIR: u64 = 3 * COLUMN + 2 * 4096;
const WORST_PROOF: u64 = 2 * WORST_PAIR + 2 * 4096 + 4 * 1024 + 2 * COLUMN + 2 * 4096;
const WORST_SNAPSHOT: u64 = 8 * (WORST_PROOF + COLUMN + 4096);
const WORST_COMMIT: u64 = 8 * WORST_PROOF + 8 * WORST_PROOF;
const HARD_CELLS: u64 = 16_777_216;
const _: () = assert!(WORST_SNAPSHOT <= HARD_CELLS && WORST_COMMIT <= HARD_CELLS);

/// Overlay-only image of an empty chunk: probe 4's ordinary autosave form.
fn overlay_only(k: ChunkKey) -> ChunkImageChange {
    ChunkImageChange::new(
        k,
        ChunkImageAction::Replace(
            ChunkImage::new(
                None,
                Some(
                    ChunkOverlayRecord::try_from_chunk(
                        SchemaVersion::new(3),
                        k.pos(),
                        &Chunk::new(k.pos()),
                        0,
                    )
                    .unwrap(),
                ),
            )
            .unwrap(),
        ),
    )
}
/// Eight 8-column overlay image-CAS writes; columns `8g` then have eight distinct last receipts.
async fn distinct_histories(s: &dyn Store) {
    initialize(s).await;
    for g in 0..8 {
        let keys: Vec<_> = (0..8).map(|i| key(g * 8 + i)).collect();
        let snap = s
            .load_owned_blocks(spec(false, keys.clone()))
            .await
            .unwrap();
        let images = keys.iter().map(|k| overlay_only(*k)).collect();
        s.compare_exchange_chunk_images(
            ChunkImageCasRequest::new(id(10 + g as u8), snap, images).unwrap(),
        )
        .await
        .unwrap();
    }
}
fn distinct(n: i32) -> Vec<ChunkKey> {
    (0..n).map(|g| key(g * 8)).collect()
}
fn union_cell(k: ChunkKey) -> ferrumc_math::BlockPos {
    pos(k.pos().x() * 16 + 3, 70)
}
/// One scoped gameplay commit over the whole distinct-history union, one effect per column.
async fn union_commit(s: &dyn Store, n: u8) -> BlockMutationRequest {
    let keys = distinct(8);
    let snap = s
        .load_owned_blocks(spec(false, keys.clone()))
        .await
        .unwrap();
    let (mut images, mut effects) = (Vec::new(), Vec::new());
    for (i, k) in keys.iter().enumerate() {
        let mut c = snap.columns()[i].chunk().clone();
        let old = c.get_block(union_cell(*k)).unwrap().as_u32();
        c.set_block(union_cell(*k), BlockStateId::new(old + 1 + i as u32))
            .unwrap();
        images.push(replace(*k, c));
        effects.push(effect(union_cell(*k), old, old + 1 + i as u32, None, None));
    }
    BlockMutationRequest::new(id(n), snap, images, vec![], effects).unwrap()
}
/// Probe 4 regression: every 1..8-column union of distinct 8-column histories and a commit
/// over the whole union are admitted at the default (= hard) cell cap, on both backends.
#[tokio::test]
async fn max_union_with_distinct_histories_snapshots_and_commits_under_hard_cell_cap() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("union");
        let s = open(durable, &path, dir.path(), Caps::default());
        distinct_histories(&*s).await;
        for n in 1..=8 {
            let snap = s.load_owned_blocks(spec(false, distinct(n))).await;
            assert!(snap.is_ok(), "durable={durable} n={n}: {:?}", snap.err());
        }
        s.load_owned_blocks(spec(false, (0..8).map(key).collect()))
            .await
            .unwrap();
        let request = union_commit(&*s, 30).await;
        let query = request.receipt_query();
        let before: Vec<_> = s
            .load_owned_blocks(spec(false, distinct(8)))
            .await
            .unwrap()
            .columns()
            .iter()
            .map(|c| c.chunk().get_block(union_cell(c.key())).unwrap().as_u32())
            .collect();
        let outcome = s.commit_blocks(request).await.unwrap();
        assert_eq!(outcome.disposition(), OwnershipCommitDisposition::Committed);
        let check = |s: Arc<dyn Store>| {
            let before = before.clone();
            async move {
                let snap = s.load_owned_blocks(spec(false, distinct(8))).await.unwrap();
                for (i, c) in snap.columns().iter().enumerate() {
                    assert_eq!(
                        c.chunk().get_block(union_cell(c.key())).unwrap().as_u32(),
                        before[i] + 1 + i as u32
                    );
                    assert_eq!(c.stamp().revision().map(OwnershipRevision::get), Some(2));
                }
                assert!(s.load_block_receipt(query).await.unwrap().is_some());
                // The neighbors keep their own (image-CAS) history.
                s.load_owned_blocks(spec(false, (1..8).map(key).collect()))
                    .await
                    .unwrap();
            }
        };
        check(s.clone()).await;
        if durable {
            drop(s);
            check(open(true, &path, dir.path(), Caps::default())).await;
        }
    }
}
/// N/N-1 at the configured cell cap, expectations from the independent model above.
#[tokio::test]
async fn distinct_history_union_cell_work_matches_the_independent_model_exactly() {
    assert_eq!(SNAPSHOT_COLUMN, 491_520);
    assert_eq!(UNION_COMMIT, 7_864_352);
    for durable in [false, true] {
        let prepared = |caps: Caps| async move {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("cells");
            if durable {
                distinct_histories(&*open(true, &path, dir.path(), Caps::default())).await;
            }
            let s = open(durable, &path, dir.path(), caps);
            if !durable {
                distinct_histories(&*s).await;
            }
            (dir, s)
        };
        let cells = |cells| Caps {
            cells,
            ..Caps::default()
        };
        // Memory applies limits from construction, so its 8-write setup (at most
        // 8 * OVERLAY_HISTORY_PROOF cells per call) must itself fit: n >= 7.
        for n in [7u64, 8] {
            let need = n * SNAPSHOT_COLUMN;
            let (_dir, s) = prepared(cells(need)).await;
            s.load_owned_blocks(spec(false, distinct(n as i32)))
                .await
                .unwrap();
            let (_dir, s) = prepared(cells(need - 1)).await;
            assert_eq!(
                s.load_owned_blocks(spec(false, distinct(n as i32)))
                    .await
                    .err(),
                Some(BlockDurabilityError::ValidationCapacity("cells")),
                "durable={durable} n={n}"
            );
        }
        let (_dir, s) = prepared(cells(UNION_COMMIT)).await;
        s.commit_blocks(union_commit(&*s, 30).await).await.unwrap();
        let (_dir, s) = prepared(cells(UNION_COMMIT - 1)).await;
        let request = union_commit(&*s, 30).await;
        let query = request.receipt_query();
        assert_eq!(
            s.commit_blocks(request).await.err(),
            Some(BlockDurabilityError::ValidationCapacity("cells"))
        );
        assert!(s.load_block_receipt(query).await.unwrap().is_none());
    }
}

/// Probe 3 regression: a refused fresh open leaves no database file, its error names the real
/// cause, and the corrected open succeeds. Existing files are never removed.
#[tokio::test]
async fn refused_fresh_open_leaves_nothing_and_the_corrected_retry_opens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fresh.redb");
    let missing = dir.path().join("missing-scratch");
    let not_a_dir = dir.path().join("scratch-file");
    std::fs::write(&not_a_dir, b"x").unwrap();
    for scratch in [&missing, &not_a_dir] {
        match RedbStore::open_with_player_block_durability(&path, config(), scratch) {
            Err(BlockDurabilityError::ScratchUnavailable(reason)) => {
                assert!(!reason.contains("UUID"), "{reason}");
                assert!(reason.contains("scratch"), "{reason}");
            }
            other => panic!("{:?}", other.err()),
        }
        assert!(!path.exists());
    }
    // A scratch extent too small for the disposable index is refused just as early.
    let small = config_with(Caps {
        scratch: 1,
        ..Caps::default()
    })
    .unwrap();
    assert_eq!(
        RedbStore::open_with_player_block_durability(&path, small, dir.path()).err(),
        Some(BlockDurabilityError::ScratchCapacity)
    );
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    let s: Arc<dyn Store> = Arc::new(
        RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap(),
    );
    initialize(&*s).await;
    seed_air(&*s).await;
    drop(s);
    let s = RedbStore::open_with_player_block_durability(&path, config(), dir.path()).unwrap();
    s.load_owned_blocks(spec(true, vec![key(0), key(1)]))
        .await
        .unwrap();
    drop(s);
    // An existing table-free redb file (an older version's refused create) and a foreign
    // file are refused with their exact bytes kept.
    let empty = dir.path().join("empty.redb");
    drop(Database::create(&empty).unwrap());
    let foreign = dir.path().join("foreign.redb");
    std::fs::write(&foreign, b"not a database").unwrap();
    for (file, guided) in [(&empty, true), (&foreign, false)] {
        let before = std::fs::read(file).unwrap();
        let result = RedbStore::open_with_player_block_durability(file, config(), dir.path());
        match result {
            Err(BlockDurabilityError::CatalogMismatch(reason)) if guided => {
                assert!(reason.contains("interrupted fresh create"), "{reason}");
            }
            Err(BlockDurabilityError::Backend(_)) if !guided => {}
            other => panic!("{:?}", other.err()),
        }
        assert_eq!(std::fs::read(file).unwrap(), before);
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 4);
}
/// v1 in Format7 refuses shared receipt bytes before Player/Entity history (contract order).
#[tokio::test]
async fn v1_receipt_bytes_refuse_before_player_history() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let caps = Caps {
            revisions: 1,
            receipt_bytes: 272,
            ..Caps::default()
        };
        let s = open(durable, &dir.path().join("order"), dir.path(), caps);
        initialize(&*s).await;
        let second = ScopedPlayerKey::new(
            scope(),
            ferrumc_core::PlayerId::from_uuid(uuid::Uuid::from_bytes([0x1d; 16])),
        );
        let fresh = s.load_owned_player(second, generation()).await.unwrap();
        // Both the next 272 receipt bytes and a second Player history row exceed their caps.
        let result = s
            .compare_exchange_owned_player(
                PlayerOwnershipCasRequest::new(
                    id(5),
                    fresh.expectation().clone(),
                    PlayerOwnershipProposal::put(p(11)),
                )
                .unwrap(),
            )
            .await;
        assert!(
            matches!(&result, Err(OwnershipError::Block(e)) if matches!(**e, BlockDurabilityError::HistoryCapacity { resource: "receipt bytes", max: 272 })),
            "durable={durable}: {result:?}"
        );
        assert!(s
            .load_owned_player(second, generation())
            .await
            .unwrap()
            .record()
            .is_none());
    }
}
