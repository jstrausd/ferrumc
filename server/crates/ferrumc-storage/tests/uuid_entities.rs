//! Backend parity and public contract tests; UUID mode is never activated in App.

use std::num::NonZeroU16;
use std::sync::Arc;

use async_trait::async_trait;
use ferrumc_core::{DimensionId, EntityId, EntityUuid, ServerError, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::{
    ChunkKey, EntityKey, EntityRecord, GenerationKey, InMemoryStore, PersistentEntityKey as Key,
    PersistentEntityRecord as Record, PersistentEntityStore, PersistentEntityWrite as Write,
    RedbStore, SchemaVersion, WorldStore, MAX_ENTITY_DATA_LEN, MAX_PERSISTENT_ENTITY_BATCH_BYTES,
};
use ferrumc_world::{GenerationIdentity, WorldSeed};

trait Store: PersistentEntityStore + WorldStore {}
impl<T: PersistentEntityStore + WorldStore> Store for T {}

fn identity(seed: i64) -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(seed))
}
fn key(world: u32, dimension: u32, uuid: u128) -> Key {
    Key::new(
        WorldId::new(world),
        DimensionId::new(dimension),
        EntityUuid::from_bytes(uuid.to_be_bytes()),
    )
}
fn chunk(world: u32, dimension: u32, x: i32, z: i32) -> ChunkKey {
    ChunkKey::new(
        WorldId::new(world),
        DimensionId::new(dimension),
        ChunkPos::new(x, z),
    )
}
fn write(key: Key, expected: GenerationIdentity, pos: ChunkPos, body: Vec<u8>) -> Write {
    Write::new(
        key,
        expected,
        Record::new(
            pos,
            EntityRecord::new(SchemaVersion::new(u32::MAX), body).unwrap(),
        )
        .unwrap(),
    )
}
fn limit(count: u16) -> NonZeroU16 {
    NonZeroU16::new(count).unwrap()
}
async fn initialize(store: &dyn Store, world: u32, dimension: u32, seed: i64) {
    store
        .ensure_generation_identity(
            GenerationKey::new(WorldId::new(world), DimensionId::new(dimension)),
            identity(seed),
        )
        .await
        .unwrap();
}
async fn epoch(store: &dyn Store, chunk: ChunkKey, expected: GenerationIdentity) -> u64 {
    store
        .load_persistent_entity_page(chunk, expected, None, limit(64))
        .await
        .unwrap()
        .epoch()
        .get()
}

async fn initial_behavior(store: &dyn Store) -> (Key, Write, Key) {
    // Supported empty request is a no-op even before any descriptor exists.
    store.save_persistent_entities(vec![]).await.unwrap();
    initialize(store, 0, 0, 0).await;
    initialize(store, 0, 1, 1).await;
    initialize(store, 1, 0, 2).await;
    let origin = ChunkPos::new(-1, 2);
    let first = key(0, 0, 0);
    assert!(store
        .load_persistent_entity(first, identity(0))
        .await
        .unwrap()
        .is_none());
    assert!(!store
        .delete_persistent_entity(first, identity(0))
        .await
        .unwrap());
    assert_eq!(epoch(store, chunk(0, 0, -1, 2), identity(0)).await, 0);
    let proposal = write(first, identity(0), origin, vec![1, 2, 3]);
    store
        .save_persistent_entity(proposal.clone())
        .await
        .unwrap();
    assert_eq!(
        store
            .load_persistent_entity(first, identity(0))
            .await
            .unwrap()
            .as_ref(),
        Some(proposal.record())
    );
    assert_eq!(epoch(store, chunk(0, 0, -1, 2), identity(0)).await, 1);
    store
        .save_persistent_entity(proposal.clone())
        .await
        .unwrap();
    assert_eq!(epoch(store, chunk(0, 0, -1, 2), identity(0)).await, 1);
    assert!(matches!(
        store.load_persistent_entity(first, identity(3)).await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store
            .save_persistent_entities(vec![proposal.clone(), proposal.clone()])
            .await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(matches!(
        store
            .save_persistent_entity(write(key(0, 1, 0), identity(1), origin, vec![3]))
            .await,
        Err(ServerError::InvalidState(_))
    ));
    assert!(store
        .load_persistent_entity(key(0, 1, 0), identity(1))
        .await
        .unwrap()
        .is_none());
    assert!(!store
        .delete_persistent_entity(key(0, 1, 0), identity(1))
        .await
        .unwrap());
    // Same UUID bits in another world are independent.
    store
        .save_persistent_entity(write(key(1, 0, 0), identity(2), origin, vec![4]))
        .await
        .unwrap();
    let second = key(0, 0, u128::MAX);
    store
        .save_persistent_entities(vec![
            write(first, identity(0), origin, vec![5]),
            write(second, identity(0), origin, vec![6]),
        ])
        .await
        .unwrap();
    assert_eq!(epoch(store, chunk(0, 0, -1, 2), identity(0)).await, 2);
    (first, proposal, second)
}

async fn behavior(store: &dyn Store) {
    let (first, proposal, second) = initial_behavior(store).await;
    let origin = proposal.record().chunk();
    // Chunk change increments source and target once, preserving empty history.
    store
        .save_persistent_entity(write(second, identity(0), ChunkPos::new(3, -4), vec![7]))
        .await
        .unwrap();
    assert_eq!(epoch(store, chunk(0, 0, -1, 2), identity(0)).await, 3);
    assert_eq!(epoch(store, chunk(0, 0, 3, -4), identity(0)).await, 1);
    let moved = write(key(0, 1, 0), identity(1), ChunkPos::new(3, -4), vec![8]);
    for destination in [
        proposal.clone(),
        write(key(1, 0, 0), identity(2), origin, vec![9]),
        write(key(0, 1, 7), identity(1), origin, vec![9]),
    ] {
        assert!(matches!(
            store
                .move_persistent_entity(first, identity(0), destination)
                .await,
            Err(ServerError::InvalidState(_))
        ));
    }
    assert!(store
        .move_persistent_entity(first, identity(9), moved.clone())
        .await
        .is_err());
    store
        .move_persistent_entity(first, identity(0), moved.clone())
        .await
        .unwrap();
    assert!(store
        .load_persistent_entity(first, identity(0))
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .load_persistent_entity(moved.key(), identity(1))
            .await
            .unwrap()
            .as_ref(),
        Some(moved.record())
    );
    assert_eq!(epoch(store, chunk(0, 0, -1, 2), identity(0)).await, 4);
    assert_eq!(epoch(store, chunk(0, 1, 3, -4), identity(1)).await, 1);
    assert!(store
        .delete_persistent_entity(moved.key(), identity(1))
        .await
        .unwrap());
    assert_eq!(epoch(store, chunk(0, 1, 3, -4), identity(1)).await, 2);
    assert!(!store
        .delete_persistent_entity(moved.key(), identity(1))
        .await
        .unwrap());
    assert_eq!(epoch(store, chunk(0, 1, 3, -4), identity(1)).await, 2);
    assert!(store
        .ensure_generation_identity(
            GenerationKey::new(WorldId::new(3), DimensionId::new(3)),
            identity(0)
        )
        .await
        .is_err());
    // Atomic admission: an earlier valid proposal is not applied when a later baseline fails.
    let valid = write(key(0, 0, 12), identity(0), origin, vec![12]);
    let invalid = write(key(0, 0, 13), identity(8), origin, vec![13]);
    assert!(store
        .save_persistent_entities(vec![valid.clone(), invalid])
        .await
        .is_err());
    assert!(store
        .load_persistent_entity(valid.key(), identity(0))
        .await
        .unwrap()
        .is_none());
    // All old wire-keyed entity methods are refused, including an empty batch.
    let wire = EntityKey::new(WorldId::new(0), DimensionId::new(0), EntityId::new(1));
    assert!(matches!(
        store.load_entity(wire).await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store
            .save_entity(
                wire,
                EntityRecord::new(SchemaVersion::new(1), vec![]).unwrap()
            )
            .await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store.save_entities(vec![]).await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store.delete_entity(wire).await,
        Err(ServerError::Unsupported(_))
    ));
}

#[tokio::test]
async fn memory_and_redb_have_identical_save_delete_move_contracts() {
    behavior(&InMemoryStore::with_uuid_entities()).await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store.redb");
    behavior(&RedbStore::open_with_uuid_entities(&path).unwrap()).await;
    let reopened = RedbStore::open_with_uuid_entities(&path).unwrap();
    assert_eq!(
        reopened
            .load_persistent_entity(key(0, 0, u128::MAX), identity(0))
            .await
            .unwrap()
            .unwrap()
            .record()
            .data(),
        &[7]
    );
}

async fn paging(store: &dyn Store) {
    initialize(store, u32::MAX, u32::MAX, 0).await;
    let world = u32::MAX;
    let dimension = u32::MAX;
    // Prefix is allFF, requiring an unbounded upper bound without wraparound.
    let column = chunk(world, dimension, -1, -1);
    for ids in [vec![0, 1, 2], vec![u128::MAX - 1, u128::MAX]] {
        store
            .save_persistent_entities(
                ids.into_iter()
                    .map(|uuid| {
                        write(
                            key(world, dimension, uuid),
                            identity(0),
                            column.pos(),
                            vec![],
                        )
                    })
                    .collect(),
            )
            .await
            .unwrap();
    }
    let first = store
        .load_persistent_entity_page(column, identity(0), None, limit(2))
        .await
        .unwrap();
    assert_eq!(
        first
            .entries()
            .iter()
            .map(|entry| entry.key().uuid().to_bytes())
            .collect::<Vec<_>>(),
        vec![0u128.to_be_bytes(), 1u128.to_be_bytes()]
    );
    let cursor = first.next().unwrap().clone();
    let middle = store
        .load_persistent_entity_page(column, identity(0), Some(cursor.clone()), limit(2))
        .await
        .unwrap();
    assert_eq!(middle.entries().len(), 2);
    let last = store
        .load_persistent_entity_page(column, identity(0), middle.next().cloned(), limit(2))
        .await
        .unwrap();
    assert_eq!(
        last.entries()[0].key().uuid().to_bytes(),
        u128::MAX.to_be_bytes()
    );
    assert!(last.next().is_none());
    assert!(matches!(
        store
            .load_persistent_entity_page(column, identity(0), None, limit(65))
            .await,
        Err(ServerError::Capacity(_))
    ));
    assert!(store
        .load_persistent_entity_page(
            chunk(world, dimension, 0, 0),
            identity(0),
            Some(cursor.clone()),
            limit(2)
        )
        .await
        .is_err());
    assert!(store
        .load_persistent_entity_page(column, identity(1), Some(cursor.clone()), limit(2))
        .await
        .is_err());
    store
        .delete_persistent_entity(key(world, dimension, 2), identity(0))
        .await
        .unwrap();
    assert!(matches!(
        store
            .load_persistent_entity_page(column, identity(0), Some(cursor), limit(2))
            .await,
        Err(ServerError::InvalidState(_))
    ));
}
#[tokio::test]
async fn nil_max_uuid_and_allff_prefix_cursor_paging_matches_both_backends() {
    paging(&InMemoryStore::with_uuid_entities()).await;
    let directory = tempfile::tempdir().unwrap();
    paging(&RedbStore::open_with_uuid_entities(directory.path().join("p.redb")).unwrap()).await;
}

async fn limits(store: &dyn Store) {
    initialize(store, 0, 0, 0).await;
    let pos = ChunkPos::new(0, 0);
    let too_many = (0..65)
        .map(|id| write(key(0, 0, id), identity(0), pos, vec![]))
        .collect();
    assert!(matches!(
        store.save_persistent_entities(too_many).await,
        Err(ServerError::Capacity(_))
    ));
    assert_eq!(epoch(store, chunk(0, 0, 0, 0), identity(0)).await, 0);
    store
        .save_persistent_entities(
            (0..64)
                .map(|id| write(key(0, 0, id), identity(0), pos, vec![]))
                .collect(),
        )
        .await
        .unwrap();
    let page = store
        .load_persistent_entity_page(chunk(0, 0, 0, 0), identity(0), None, limit(64))
        .await
        .unwrap();
    assert_eq!(page.entries().len(), 64);
    assert!(page.next().is_none());
    let final_len = MAX_PERSISTENT_ENTITY_BATCH_BYTES - 16 * 46 - 15 * MAX_ENTITY_DATA_LEN;
    let mut proposals: Vec<_> = (0..15)
        .map(|id| {
            write(
                key(0, 0, id),
                identity(0),
                pos,
                vec![1; MAX_ENTITY_DATA_LEN],
            )
        })
        .collect();
    proposals.push(write(
        key(0, 0, 15),
        identity(0),
        pos,
        vec![2; final_len + 1],
    ));
    assert!(matches!(
        store.save_persistent_entities(proposals).await,
        Err(ServerError::Capacity(_))
    ));
    assert_eq!(
        store
            .load_persistent_entity(key(0, 0, 0), identity(0))
            .await
            .unwrap()
            .unwrap()
            .record()
            .data(),
        &[]
    );
    let mut proposals: Vec<_> = (0..15)
        .map(|id| {
            write(
                key(0, 0, id),
                identity(0),
                pos,
                vec![1; MAX_ENTITY_DATA_LEN],
            )
        })
        .collect();
    proposals.push(write(key(0, 0, 15), identity(0), pos, vec![2; final_len]));
    store.save_persistent_entities(proposals).await.unwrap();
    // Four maximum bodies exceed1MiB by envelope+key bytes; only three are copied.
    let page = store
        .load_persistent_entity_page(chunk(0, 0, 0, 0), identity(0), None, limit(64))
        .await
        .unwrap();
    assert_eq!(page.entries().len(), 3);
    assert!(page.next().is_some());
    assert_eq!(epoch(store, chunk(0, 0, 0, 0), identity(0)).await, 2);
}
#[tokio::test]
async fn atomic_write_count_encoded_byte_and_page_byte_bounds() {
    limits(&InMemoryStore::with_uuid_entities()).await;
    let directory = tempfile::tempdir().unwrap();
    limits(&RedbStore::open_with_uuid_entities(directory.path().join("p.redb")).unwrap()).await;
}

#[tokio::test]
async fn cursors_accept_arc_clones_reject_foreign_backend_and_reopen_without_retaining_owner() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("p.redb");
    let store = Arc::new(RedbStore::open_with_uuid_entities(&path).unwrap());
    initialize(store.as_ref(), 0, 0, 0).await;
    store
        .save_persistent_entities(
            (0..2)
                .map(|id| write(key(0, 0, id), identity(0), ChunkPos::new(0, 0), vec![]))
                .collect(),
        )
        .await
        .unwrap();
    let cursor = store
        .load_persistent_entity_page(chunk(0, 0, 0, 0), identity(0), None, limit(1))
        .await
        .unwrap()
        .next()
        .unwrap()
        .clone();
    store
        .clone()
        .load_persistent_entity_page(
            chunk(0, 0, 0, 0),
            identity(0),
            Some(cursor.clone()),
            limit(1),
        )
        .await
        .unwrap();
    let other = InMemoryStore::with_uuid_entities();
    initialize(&other, 0, 0, 0).await;
    assert!(other
        .load_persistent_entity_page(
            chunk(0, 0, 0, 0),
            identity(0),
            Some(cursor.clone()),
            limit(1)
        )
        .await
        .is_err());
    let weak = Arc::downgrade(&store);
    drop(store);
    assert!(weak.upgrade().is_none());
    let reopened = RedbStore::open_with_uuid_entities(&path).unwrap();
    assert!(reopened
        .load_persistent_entity_page(chunk(0, 0, 0, 0), identity(0), Some(cursor), limit(1))
        .await
        .is_err());
}

struct DefaultOnly;
#[async_trait]
impl PersistentEntityStore for DefaultOnly {}
async fn unsupported(store: &dyn PersistentEntityStore) {
    let k = key(0, 0, 0);
    let proposal = write(k, identity(0), ChunkPos::new(0, 0), vec![]);
    for result in [
        store
            .load_persistent_entity(k, identity(0))
            .await
            .map(|_| ()),
        store.save_persistent_entity(proposal.clone()).await,
        store.save_persistent_entities(vec![]).await,
        store
            .delete_persistent_entity(k, identity(0))
            .await
            .map(|_| ()),
        store.move_persistent_entity(k, identity(0), proposal).await,
        store
            .load_persistent_entity_page(chunk(0, 0, 0, 0), identity(0), None, limit(65))
            .await
            .map(|_| ()),
    ] {
        assert!(matches!(result, Err(ServerError::Unsupported(_))));
    }
}
#[tokio::test]
async fn legacy_metadata_and_unadopted_traits_explicitly_refuse_even_empty_requests() {
    unsupported(&DefaultOnly).await;
    unsupported(&InMemoryStore::new()).await;
    unsupported(&InMemoryStore::with_generation_metadata()).await;
    let directory = tempfile::tempdir().unwrap();
    unsupported(&RedbStore::open(directory.path().join("three.redb")).unwrap()).await;
    unsupported(
        &RedbStore::open_with_generation_metadata(directory.path().join("four.redb")).unwrap(),
    )
    .await;
}

#[test]
fn complete_chunk_origin_and_local_offsets_must_fit_block_domain() {
    for value in [-134_217_728, 134_217_727] {
        assert!(Record::new(
            ChunkPos::new(value, value),
            EntityRecord::new(SchemaVersion::new(0), vec![]).unwrap()
        )
        .is_ok());
    }
    for value in [i32::MIN, -134_217_729, 134_217_728, i32::MAX] {
        assert!(Record::new(
            ChunkPos::new(value, 0),
            EntityRecord::new(SchemaVersion::new(0), vec![]).unwrap()
        )
        .is_err());
        assert!(Record::new(
            ChunkPos::new(0, value),
            EntityRecord::new(SchemaVersion::new(0), vec![]).unwrap()
        )
        .is_err());
    }
}

async fn plugin_boundaries(store: &dyn ferrumc_storage::PluginStore) {
    use ferrumc_core::PluginId;
    use ferrumc_storage::StorageKey;
    let key = StorageKey::new("key").unwrap();
    for valid in ["a".repeat(4096), "ä".repeat(2048)] {
        let plugin = PluginId::new(valid.clone());
        store.put(&plugin, key.clone(), vec![7]).await.unwrap();
        assert_eq!(store.get(&plugin, &key).await.unwrap(), Some(vec![7]));
        assert_eq!(store.keys(&plugin).await.unwrap(), vec![key.clone()]);
        let invalid = PluginId::new(format!("{valid}x"));
        for result in [
            store.put(&invalid, key.clone(), vec![8]).await,
            store.get(&invalid, &key).await.map(|_| ()),
            store.delete(&invalid, &key).await.map(|_| ()),
            store.keys(&invalid).await.map(|_| ()),
        ] {
            assert!(matches!(result, Err(ServerError::Capacity(_))));
        }
        assert_eq!(store.get(&plugin, &key).await.unwrap(), Some(vec![7]));
    }
}
#[tokio::test]
async fn format5_plugin_namespace_byte_admission_and_strict_reopen_match_both_backends() {
    use ferrumc_core::PluginId;
    use ferrumc_storage::{PluginStore, StorageKey};
    plugin_boundaries(&InMemoryStore::with_uuid_entities()).await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("p.redb");
    plugin_boundaries(&RedbStore::open_with_uuid_entities(&path).unwrap()).await;
    let reopened = RedbStore::open_with_uuid_entities(&path).unwrap();
    for valid in ["a".repeat(4096), "ä".repeat(2048)] {
        let plugin = PluginId::new(valid);
        let key = StorageKey::new("key").unwrap();
        assert_eq!(reopened.get(&plugin, &key).await.unwrap(), Some(vec![7]));
        assert!(reopened.delete(&plugin, &key).await.unwrap());
    }
    // This is a Format5 policy, not a new generic PluginId restriction.
    let invalid = PluginId::new("a".repeat(4097));
    let key = StorageKey::new("key").unwrap();
    for store in [
        &InMemoryStore::new(),
        &InMemoryStore::with_generation_metadata(),
    ] {
        store.put(&invalid, key.clone(), vec![1]).await.unwrap();
        assert_eq!(store.get(&invalid, &key).await.unwrap(), Some(vec![1]));
    }
    for store in [
        RedbStore::open(directory.path().join("old3.redb")).unwrap(),
        RedbStore::open_with_generation_metadata(directory.path().join("old4.redb")).unwrap(),
    ] {
        store.put(&invalid, key.clone(), vec![1]).await.unwrap();
        assert_eq!(store.get(&invalid, &key).await.unwrap(), Some(vec![1]));
    }
}
