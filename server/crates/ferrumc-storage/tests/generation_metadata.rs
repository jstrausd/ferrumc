//! Public contract parity for the explicitly enabled generation-metadata mode.

use std::sync::Arc;

use ferrumc_core::{DimensionId, EntityId, GameMode, PlayerId, PluginId, ServerError, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_storage::{
    BlockMutationLogRecord, ChunkKey, ChunkOverlayRecord, ChunkRecord, EntityKey, EntityRecord,
    GenerationInitialization, GenerationKey, InMemoryStore, JournalBatchId, MutationActor,
    MutationLogCause, PlayerRecord, PlayerStore, PluginStore, RedbStore, SchemaVersion, StorageKey,
    WorldStore,
};
use ferrumc_world::{BlockStateId, Chunk, GenerationIdentity, WorldSeed};
use tokio::sync::Barrier;

trait MetadataStore: WorldStore + PlayerStore + PluginStore {}
impl<T: WorldStore + PlayerStore + PluginStore> MetadataStore for T {}

fn key(scope: u32) -> GenerationKey {
    GenerationKey::new(WorldId::new(scope), DimensionId::new(0))
}

fn identity(seed: i64) -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(seed))
}

fn mutation() -> BlockMutationLogRecord {
    BlockMutationLogRecord::new(
        SchemaVersion::new(1),
        20,
        10,
        MutationActor::System,
        BlockPos::new(0, 64, 0),
        BlockStateId::new(1),
        BlockStateId::new(2),
        MutationLogCause::Test,
    )
}

async fn missing_identity_admission(store: &dyn MetadataStore) {
    let pos = ChunkPos::new(0, 0);
    let bound = ChunkKey::new(key(1).world(), key(1).dimension(), pos);
    let record = ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos));
    let overlay =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(1), pos, record.chunk(), 0).unwrap();
    let bound_entity = EntityKey::new(bound.world(), bound.dimension(), EntityId::new(1));
    let entity_record = EntityRecord::new(SchemaVersion::new(1), vec![1]).unwrap();
    let player_id = PlayerId::offline("metadata-contract");
    let player = PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![1]).unwrap();
    assert_eq!(store.load_generation_identity(key(1)).await.unwrap(), None);
    assert!(matches!(
        store.save_chunk(bound, record.clone()).await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store
            .save_chunk_overlays(vec![(bound, overlay.clone())])
            .await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store.save_entity(bound_entity, entity_record.clone()).await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store.save_player(player_id, player.clone()).await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store.append_block_mutations(vec![mutation()]).await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store.append_block_mutations(vec![]).await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store
            .append_block_mutation_batch(JournalBatchId::from_bytes([1; 16]), vec![])
            .await,
        Err(ServerError::InvalidState(_))
    ));
    // Deletes have no admission requirement, including an unbound missing key.
    assert!(!store.delete_chunk(bound).await.unwrap());
    assert!(!store.delete_entity(bound_entity).await.unwrap());
    assert!(!store.delete_player(player_id).await.unwrap());
}

async fn public_contract(store: &dyn MetadataStore) {
    let pos = ChunkPos::new(0, 0);
    let bound = ChunkKey::new(key(1).world(), key(1).dimension(), pos);
    let unbound = ChunkKey::new(key(2).world(), key(2).dimension(), pos);
    let same_world_unbound_dimension = ChunkKey::new(bound.world(), DimensionId::new(1), pos);
    let record = ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos));
    let overlay =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(1), pos, record.chunk(), 0).unwrap();
    let bound_entity = EntityKey::new(bound.world(), bound.dimension(), EntityId::new(1));
    let unbound_entity = EntityKey::new(unbound.world(), unbound.dimension(), EntityId::new(2));
    let entity_record = EntityRecord::new(SchemaVersion::new(1), vec![1]).unwrap();
    let player_id = PlayerId::offline("metadata-contract");
    let player = PlayerRecord::new(SchemaVersion::new(1), GameMode::Survival, vec![1]).unwrap();
    missing_identity_admission(store).await;
    assert_eq!(
        store
            .ensure_generation_identity(key(1), identity(i64::MIN))
            .await
            .unwrap(),
        GenerationInitialization::Initialized
    );
    assert_eq!(
        store
            .ensure_generation_identity(key(1), identity(i64::MIN))
            .await
            .unwrap(),
        GenerationInitialization::AlreadyPresent
    );
    assert!(matches!(
        store
            .ensure_generation_identity(key(1), identity(i64::MAX))
            .await,
        Err(ServerError::InvalidState(_))
    ));
    assert_eq!(
        store.load_generation_identity(key(1)).await.unwrap(),
        Some(identity(i64::MIN))
    );
    assert!(matches!(
        store
            .save_chunk(same_world_unbound_dimension, record.clone())
            .await,
        Err(ServerError::InvalidState(_))
    ));

    assert!(matches!(
        store
            .save_chunk_overlays(vec![(bound, overlay.clone()), (unbound, overlay.clone())])
            .await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(store.load_chunk_overlay(bound).await.unwrap().is_none());
    assert!(matches!(
        store
            .save_entities(vec![
                (bound_entity, entity_record.clone()),
                (unbound_entity, entity_record.clone())
            ])
            .await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(store.load_entity(bound_entity).await.unwrap().is_none());

    // The late unbound entry must not commit the first entry of the same batch.
    assert!(matches!(
        store
            .save_chunks(vec![(bound, record.clone()), (unbound, record.clone())])
            .await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(store.load_chunk(bound).await.unwrap().is_none());
    store.save_chunk(bound, record).await.unwrap();
    assert!(store.load_chunk(bound).await.unwrap().is_some());
    assert!(matches!(
        store.ensure_generation_identity(key(2), identity(0)).await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(store
        .load_generation_identity(key(2))
        .await
        .unwrap()
        .is_none());

    store
        .save_chunk_overlays(vec![(bound, overlay)])
        .await
        .unwrap();
    store
        .save_entity(bound_entity, entity_record)
        .await
        .unwrap();
    store.save_player(player_id, player).await.unwrap();
    journal_admission_contract(store).await;
}

async fn journal_admission_contract(store: &dyn MetadataStore) {
    store
        .append_block_mutations(vec![mutation()])
        .await
        .unwrap();
    let batch = JournalBatchId::from_bytes([2; 16]);
    let receipt = store
        .append_block_mutation_batch(batch, vec![mutation()])
        .await
        .unwrap();
    assert_eq!(
        store
            .append_block_mutation_batch(batch, vec![mutation()])
            .await
            .unwrap(),
        receipt
    );
    let empty = JournalBatchId::from_bytes([3; 16]);
    assert!(store
        .append_block_mutation_batch(empty, vec![])
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn memory_public_generation_contract() {
    public_contract(&InMemoryStore::with_generation_metadata()).await;
}

#[tokio::test]
async fn redb_public_generation_contract_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("generation.redb");
    {
        let store = RedbStore::open_with_generation_metadata(&path).unwrap();
        public_contract(&store).await;
    }
    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    assert_eq!(
        store.load_generation_identity(key(1)).await.unwrap(),
        Some(identity(i64::MIN))
    );
    assert_eq!(
        store
            .ensure_generation_identity(key(1), identity(i64::MIN))
            .await
            .unwrap(),
        GenerationInitialization::AlreadyPresent
    );
}

#[tokio::test]
async fn legacy_metadata_is_unsupported_and_saves_remain_available() {
    let memory = InMemoryStore::new();
    let dir = tempfile::tempdir().unwrap();
    let redb = RedbStore::open(dir.path().join("legacy.redb")).unwrap();
    for store in [&memory as &dyn WorldStore, &redb as &dyn WorldStore] {
        assert!(matches!(
            store.load_generation_identity(key(1)).await,
            Err(ServerError::Unsupported(_))
        ));
        assert!(matches!(
            store.ensure_generation_identity(key(1), identity(0)).await,
            Err(ServerError::Unsupported(_))
        ));
        let pos = ChunkPos::new(0, 0);
        store
            .save_chunk(
                ChunkKey::new(key(1).world(), key(1).dimension(), pos),
                ChunkRecord::new(SchemaVersion::new(1), Chunk::new(pos)),
            )
            .await
            .unwrap();
    }
}

async fn concurrent_initialization(store: Arc<dyn MetadataStore>) {
    let barrier = Arc::new(Barrier::new(8));
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            store
                .ensure_generation_identity(key(1), identity(20))
                .await
                .unwrap()
        }));
    }
    let mut initialized = 0;
    for task in tasks {
        if task.await.unwrap() == GenerationInitialization::Initialized {
            initialized += 1;
        }
    }
    assert_eq!(initialized, 1);

    let barrier = Arc::new(Barrier::new(2));
    let mut tasks = Vec::new();
    for seed in [i64::MIN, i64::MAX] {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            (
                seed,
                store
                    .ensure_generation_identity(key(2), identity(seed))
                    .await,
            )
        }));
    }
    let mut winner = None;
    let mut mismatches = 0;
    for task in tasks {
        let (seed, result) = task.await.unwrap();
        match result {
            Ok(GenerationInitialization::Initialized) => {
                assert!(winner.replace(seed).is_none());
            }
            Err(ServerError::InvalidState(_)) => mismatches += 1,
            other => panic!("unexpected concurrent result {other:?}"),
        }
    }
    assert_eq!(mismatches, 1);
    assert_eq!(
        store.load_generation_identity(key(2)).await.unwrap(),
        Some(identity(winner.unwrap()))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn memory_concurrent_equal_and_different_seeds() {
    concurrent_initialization(Arc::new(InMemoryStore::with_generation_metadata())).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn redb_concurrent_equal_and_different_seeds() {
    let dir = tempfile::tempdir().unwrap();
    concurrent_initialization(Arc::new(
        RedbStore::open_with_generation_metadata(dir.path().join("race.redb")).unwrap(),
    ))
    .await;
}

async fn plugin_only_and_seed_boundaries(store: &dyn MetadataStore) {
    let plugin = PluginId::new("metadata-test");
    let plugin_key = StorageKey::new("only-kv").unwrap();
    store
        .put(&plugin, plugin_key.clone(), vec![5])
        .await
        .unwrap();
    for (scope, seed) in (0..).zip([i64::MIN, -1, 0, 1, i64::MAX]) {
        assert_eq!(
            store
                .ensure_generation_identity(key(scope), identity(seed))
                .await
                .unwrap(),
            GenerationInitialization::Initialized
        );
        assert_eq!(
            store.load_generation_identity(key(scope)).await.unwrap(),
            Some(identity(seed))
        );
    }
    assert_eq!(
        store.get(&plugin, &plugin_key).await.unwrap(),
        Some(vec![5])
    );
}

#[tokio::test]
async fn memory_plugin_only_is_fresh_and_seed_boundaries_round_trip() {
    plugin_only_and_seed_boundaries(&InMemoryStore::with_generation_metadata()).await;
}

#[tokio::test]
async fn redb_plugin_only_is_fresh_and_seed_boundaries_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("seeds.redb");
    {
        plugin_only_and_seed_boundaries(&RedbStore::open_with_generation_metadata(&path).unwrap())
            .await;
    }
    let store = RedbStore::open_with_generation_metadata(&path).unwrap();
    for (scope, seed) in (0..).zip([i64::MIN, -1, 0, 1, i64::MAX]) {
        assert_eq!(
            store.load_generation_identity(key(scope)).await.unwrap(),
            Some(identity(seed))
        );
    }
}
