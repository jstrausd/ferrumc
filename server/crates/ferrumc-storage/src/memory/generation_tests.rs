use std::collections::BTreeMap;
use std::sync::{Arc, Barrier};

use ferrumc_core::{DimensionId, EntityId, GameMode, ServerError, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_world::{BlockStateId, Chunk, WorldSeed};

use super::*;
use crate::{codec, MutationActor, MutationLogCause, SchemaVersion};

type Rows = BTreeMap<Vec<u8>, Vec<u8>>;
type Snapshot = BTreeMap<&'static str, Rows>;

fn key(scope: u32) -> GenerationKey {
    GenerationKey::new(WorldId::new(scope), DimensionId::new(0))
}

fn identity(seed: i64) -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(seed))
}

fn mutation() -> BlockMutationLogRecord {
    BlockMutationLogRecord::new(
        SchemaVersion::new(1),
        0,
        10,
        MutationActor::System,
        BlockPos::new(0, 64, 0),
        BlockStateId::new(1),
        BlockStateId::new(2),
        MutationLogCause::Test,
    )
}

fn snapshot(store: &InMemoryStore) -> Snapshot {
    let mut result = Snapshot::new();
    result.insert(
        "generation",
        store
            .generations
            .read()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.encode().to_vec(), value.encode().to_vec()))
            .collect(),
    );
    result.insert(
        "chunks",
        store
            .chunks
            .read()
            .unwrap()
            .iter()
            .map(|(key, value)| {
                (
                    codec::chunk_key_bytes(*key).to_vec(),
                    codec::encode_chunk_record(value).unwrap(),
                )
            })
            .collect(),
    );
    result.insert(
        "overlays",
        store
            .overlays
            .read()
            .unwrap()
            .iter()
            .map(|(key, value)| {
                (
                    codec::chunk_key_bytes(*key).to_vec(),
                    codec::encode_chunk_overlay_record(value).unwrap(),
                )
            })
            .collect(),
    );
    result.insert(
        "entities",
        store
            .entities
            .read()
            .unwrap()
            .iter()
            .map(|(key, value)| {
                (
                    codec::entity_key_bytes(*key).to_vec(),
                    codec::encode_entity_record(value),
                )
            })
            .collect(),
    );
    result.insert(
        "players",
        store
            .players
            .read()
            .unwrap()
            .iter()
            .map(|(key, value)| {
                (
                    codec::player_key_bytes(*key).to_vec(),
                    codec::encode_player_record(value),
                )
            })
            .collect(),
    );
    result.insert(
        "plugins",
        store
            .plugins
            .read()
            .unwrap()
            .iter()
            .flat_map(|(plugin, namespace)| {
                namespace
                    .iter()
                    .map(|(key, value)| (codec::plugin_key_bytes(plugin, key), value.clone()))
            })
            .collect(),
    );
    snapshot_journal(store, &mut result);
    result
}

fn snapshot_journal(store: &InMemoryStore, result: &mut Snapshot) {
    let journal = store.mutation_log.read().unwrap();
    result.insert(
        "journal",
        journal
            .records
            .iter()
            .map(|record| {
                (
                    record.id().to_be_bytes().to_vec(),
                    codec::encode_mutation_log_record(record),
                )
            })
            .collect(),
    );
    result.insert(
        "journal-meta",
        journal
            .last_id
            .into_iter()
            .map(|id| (vec![], id.to_be_bytes().to_vec()))
            .collect(),
    );
    result.insert(
        "receipts",
        journal
            .receipts
            .iter()
            .map(|(batch, committed)| {
                let mut encoded = Vec::new();
                encoded.extend_from_slice(&committed.receipt.first_id().unwrap_or(0).to_be_bytes());
                encoded.extend_from_slice(&committed.receipt.len().to_be_bytes());
                for record in &committed.records {
                    encoded.extend_from_slice(&codec::encode_mutation_log_record(record));
                }
                (batch.as_bytes().to_vec(), encoded)
            })
            .collect(),
    );
}

#[tokio::test]
async fn every_world_bearing_category_blocks_missing_identity_without_mutation() {
    for category in [
        "chunk",
        "empty-overlay",
        "overlay",
        "entity",
        "player",
        "journal",
        "empty-receipt",
        "watermark",
    ] {
        let store = InMemoryStore::with_generation_metadata();
        let pos = ChunkPos::new(0, 0);
        let chunk_key = ChunkKey::new(WorldId::new(99), DimensionId::new(99), pos);
        match category {
            "chunk" => {
                store.chunks.write().unwrap().insert(
                    chunk_key,
                    ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos)),
                );
            }
            "empty-overlay" | "overlay" => {
                let mut chunk = Chunk::new(pos);
                if category == "overlay" {
                    chunk
                        .set_block(BlockPos::new(0, 0, 0), BlockStateId::new(1))
                        .unwrap();
                }
                let overlay =
                    ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(1), pos, &chunk, 0)
                        .unwrap();
                store.overlays.write().unwrap().insert(chunk_key, overlay);
            }
            "entity" => {
                store.entities.write().unwrap().insert(
                    EntityKey::new(chunk_key.world(), chunk_key.dimension(), EntityId::new(1)),
                    EntityRecord::new(SchemaVersion::new(1), vec![]).unwrap(),
                );
            }
            "player" => {
                store.players.write().unwrap().insert(
                    PlayerId::offline("existing"),
                    PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![]).unwrap(),
                );
            }
            "journal" => {
                let mut journal = store.mutation_log.write().unwrap();
                journal.records.push(mutation());
                journal.last_id = Some(0);
            }
            "empty-receipt" => {
                let batch = JournalBatchId::from_bytes([1; 16]);
                let receipt = JournalAppendReceipt::from_range(batch, None, 0);
                store.mutation_log.write().unwrap().receipts.insert(
                    batch,
                    CommittedJournalBatch {
                        receipt,
                        records: vec![],
                    },
                );
            }
            "watermark" => {
                store.mutation_log.write().unwrap().last_id = Some(0);
            }
            _ => unreachable!(),
        }
        let before = snapshot(&store);
        assert!(
            matches!(
                store.ensure_generation_identity(key(1), identity(0)).await,
                Err(ServerError::InvalidState(_))
            ),
            "{category}"
        );
        assert_eq!(snapshot(&store), before, "{category}");
        assert_eq!(store.load_generation_identity(key(1)).await.unwrap(), None);
    }
}

#[tokio::test]
async fn identity_limit_refuses_insert_but_keeps_existing_compare_and_mismatch() {
    let store = InMemoryStore::with_generation_metadata();
    for scope in 0..u32::try_from(MAX_GENERATION_IDENTITIES).unwrap() {
        store
            .ensure_generation_identity(key(scope), identity(i64::from(scope)))
            .await
            .unwrap();
    }
    let before = snapshot(&store);
    assert!(matches!(
        store
            .ensure_generation_identity(key(u32::MAX), identity(0))
            .await,
        Err(ServerError::Capacity(_))
    ));
    assert_eq!(
        store
            .ensure_generation_identity(key(0), identity(0))
            .await
            .unwrap(),
        GenerationInitialization::AlreadyPresent
    );
    assert!(matches!(
        store.ensure_generation_identity(key(0), identity(1)).await,
        Err(ServerError::InvalidState(_))
    ));
    assert_eq!(snapshot(&store), before);
}

#[test]
fn in_flight_save_is_visible_to_exclusive_freshness_scan() {
    let store = Arc::new(InMemoryStore::with_generation_metadata());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime
        .block_on(store.ensure_generation_identity(key(1), identity(0)))
        .unwrap();
    let boundary = store.mutation_guard().unwrap();
    assert!(store.mutation_boundary.try_write().is_err());
    let barrier = Arc::new(Barrier::new(2));
    let initializer = {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            barrier.wait();
            runtime.block_on(store.ensure_generation_identity(key(2), identity(0)))
        })
    };
    barrier.wait();
    let pos = ChunkPos::new(0, 0);
    // Model the production save after it has acquired its shared boundary.
    // Reacquiring a read lock while a writer is queued would itself deadlock
    // on writer-preferring platforms, so retain this one existing save guard.
    store.require_generation(Some(key(1))).unwrap();
    store.chunks.write().unwrap().insert(
        ChunkKey::new(key(1).world(), key(1).dimension(), pos),
        ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos)),
    );
    drop(boundary);
    assert!(matches!(
        initializer.join().unwrap(),
        Err(ServerError::InvalidState(_))
    ));
    assert_eq!(
        runtime
            .block_on(store.load_generation_identity(key(2)))
            .unwrap(),
        None
    );
}

#[test]
fn exclusive_initialization_boundary_is_shared_by_all_mutation_guards() {
    let store = InMemoryStore::with_generation_metadata();
    let boundary = store.mutation_boundary.write().unwrap();
    assert!(store.mutation_boundary.try_read().is_err());
    drop(boundary);
    assert!(store.mutation_guard().unwrap().is_some());
    assert!(InMemoryStore::new().mutation_guard().unwrap().is_none());
}

#[tokio::test]
async fn all_rejected_saves_and_compare_preserve_complete_logical_snapshot() {
    let store = InMemoryStore::with_generation_metadata();
    store
        .ensure_generation_identity(key(1), identity(0))
        .await
        .unwrap();
    let pos = ChunkPos::new(0, 0);
    let bound = ChunkKey::new(key(1).world(), key(1).dimension(), pos);
    let unbound = ChunkKey::new(key(2).world(), key(2).dimension(), pos);
    let record = ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos));
    let overlay =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(1), pos, record.chunk(), 0).unwrap();
    let bound_entity = EntityKey::new(bound.world(), bound.dimension(), EntityId::new(1));
    let unbound_entity = EntityKey::new(unbound.world(), unbound.dimension(), EntityId::new(1));
    let entity = EntityRecord::new(SchemaVersion::new(1), vec![3]).unwrap();
    store.save_chunk(bound, record.clone()).await.unwrap();
    store
        .save_chunk_overlays(vec![(bound, overlay.clone())])
        .await
        .unwrap();
    store
        .save_entity(bound_entity, entity.clone())
        .await
        .unwrap();
    store
        .save_player(
            PlayerId::offline("snapshot"),
            PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![2]).unwrap(),
        )
        .await
        .unwrap();
    store
        .put(
            &PluginId::new("snapshot"),
            StorageKey::new("key").unwrap(),
            vec![9],
        )
        .await
        .unwrap();
    store
        .append_block_mutation_batch(JournalBatchId::from_bytes([5; 16]), vec![mutation()])
        .await
        .unwrap();
    let before = snapshot(&store);
    for result in [
        store
            .save_chunks(vec![(bound, record.clone()), (unbound, record.clone())])
            .await,
        store
            .save_chunk_overlays(vec![(bound, overlay.clone()), (unbound, overlay)])
            .await,
        store
            .save_entities(vec![
                (bound_entity, entity.clone()),
                (unbound_entity, entity.clone()),
            ])
            .await,
        store.save_entity(unbound_entity, entity).await,
        store.save_chunk(unbound, record).await,
    ] {
        assert!(matches!(result, Err(ServerError::InvalidState(_))));
        assert_eq!(snapshot(&store), before);
    }
    assert!(store
        .ensure_generation_identity(key(1), identity(1))
        .await
        .is_err());
    assert!(store
        .ensure_generation_identity(key(2), identity(0))
        .await
        .is_err());
    assert_eq!(snapshot(&store), before);
    assert_eq!(
        store
            .ensure_generation_identity(key(1), identity(0))
            .await
            .unwrap(),
        GenerationInitialization::AlreadyPresent
    );
}
