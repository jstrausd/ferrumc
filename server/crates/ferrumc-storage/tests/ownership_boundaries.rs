//! Limits, retained history and explicit authority admission through public APIs.

use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};

use async_trait::async_trait;
use ferrumc_core::{DimensionId, EntityUuid, GameMode, PlayerId, PluginId, ServerError, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::*;
use ferrumc_world::{GenerationIdentity, WorldSeed};

trait Store: PlayerEntityOwnershipStore + WorldStore + PluginStore {}
impl<T: PlayerEntityOwnershipStore + WorldStore + PluginStore> Store for T {}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(4), DimensionId::new(2))
}
fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(37))
}
fn player() -> ScopedPlayerKey {
    ScopedPlayerKey::new(scope(), PlayerId::offline("Boundary"))
}
fn entity() -> PersistentEntityKey {
    PersistentEntityKey::new(
        scope().world(),
        scope().dimension(),
        EntityUuid::from_bytes([7; 16]),
    )
}
fn id(n: u8) -> OwnershipTransferId {
    OwnershipTransferId::from_bytes([n; 16])
}
fn config(bytes: usize) -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            NonZeroUsize::new(bytes).unwrap(),
            NonZeroU64::new(8).unwrap(),
            NonZeroU64::new(8).unwrap(),
        )
        .unwrap(),
        OwnershipOpenLimits::new(
            NonZeroU64::new(1000).unwrap(),
            NonZeroU64::new(16 * 1024 * 1024).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU16::new(8).unwrap(),
        )
        .unwrap(),
    )
}
fn store(durable: bool, path: &std::path::Path, config: OwnershipConfig) -> Box<dyn Store> {
    if durable {
        Box::new(RedbStore::open_with_player_entity_ownership(path, config).unwrap())
    } else {
        Box::new(InMemoryStore::with_player_entity_ownership(config).unwrap())
    }
}
fn p(bytes: Vec<u8>) -> PlayerRecord {
    PlayerRecord::new(SchemaVersion::new(u32::MAX), GameMode::Survival, bytes).unwrap()
}
fn e(bytes: Vec<u8>) -> PersistentEntityRecord {
    PersistentEntityRecord::new(
        ChunkPos::new(0, -2),
        EntityRecord::new(SchemaVersion::new(u32::MAX - 1), bytes).unwrap(),
    )
    .unwrap()
}
async fn fresh(store: &dyn Store) -> OwnedPlayerEntitySnapshot {
    store
        .ensure_generation_identity(scope(), identity())
        .await
        .unwrap();
    store
        .load_owned_pair(player(), entity(), identity())
        .await
        .unwrap()
}
fn put_pair(
    n: u8,
    pair: &OwnedPlayerEntitySnapshot,
    p: PlayerRecord,
    e: PersistentEntityRecord,
) -> PlayerEntityTransferRequest {
    PlayerEntityTransferRequest::new(
        id(n),
        pair.player().expectation().clone(),
        pair.entity().expectation().clone(),
        PlayerOwnershipProposal::put(p),
        EntityOwnershipProposal::put(e),
    )
    .unwrap()
}

#[tokio::test]
async fn maximum_opaque_payloads_exact_admission_noop_and_reopen() {
    // Independent envelope arithmetic: fixed149 + player5+1MiB + entity22+256KiB.
    let exact = 1_310_896;
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("max.redb");
        let store = store(durable, &path, config(exact));
        let before = fresh(&*store).await;
        let player_record = p(vec![0xEF; MAX_PLAYER_DATA_LEN]);
        let entity_record = e(vec![0xBA; MAX_ENTITY_DATA_LEN]);
        let request = put_pair(1, &before, player_record.clone(), entity_record.clone());
        let query = request.receipt_query();
        let committed = store
            .commit_player_entity_transfer(request.clone())
            .await
            .unwrap();
        let current = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        assert_eq!(current.player().record(), Some(&player_record));
        assert_eq!(current.entity().record(), Some(&entity_record));
        let noop = put_pair(2, &current, player_record.clone(), entity_record.clone());
        let receipt = store.commit_player_entity_transfer(noop).await.unwrap();
        assert_eq!(
            receipt.receipt().player().unwrap().1,
            receipt.receipt().player().unwrap().2
        );
        assert_eq!(
            receipt.receipt().entity().unwrap().1,
            receipt.receipt().entity().unwrap().2
        );
        assert_eq!(receipt.receipt().epochs(), []);
        assert_eq!(
            store
                .commit_player_entity_transfer(request.clone())
                .await
                .unwrap()
                .receipt(),
            committed.receipt()
        );
        drop(store);
        if durable {
            let reopened =
                RedbStore::open_with_player_entity_ownership(&path, config(exact)).unwrap();
            let pair = reopened
                .load_owned_pair(player(), entity(), identity())
                .await
                .unwrap();
            assert_eq!(pair.player().record(), Some(&player_record));
            assert_eq!(pair.entity().record(), Some(&entity_record));
            assert_eq!(
                reopened
                    .load_ownership_receipt(query)
                    .await
                    .unwrap()
                    .as_ref(),
                Some(committed.receipt())
            );
            assert_eq!(
                reopened
                    .commit_player_entity_transfer(request)
                    .await
                    .unwrap()
                    .disposition(),
                OwnershipCommitDisposition::Replayed
            );
        }
    }
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = store(
            durable,
            &dir.path().join("one-over.redb"),
            config(exact - 1),
        );
        let before = fresh(&*store).await;
        let request = put_pair(
            1,
            &before,
            p(vec![0xEF; MAX_PLAYER_DATA_LEN]),
            e(vec![0xBA; MAX_ENTITY_DATA_LEN]),
        );
        let query = request.receipt_query();
        assert!(
            matches!(store.commit_player_entity_transfer(request).await, Err(OwnershipError::RequestBytesExceeded { len, max }) if len==exact && max==exact-1)
        );
        assert!(store.load_ownership_receipt(query).await.unwrap().is_none());
        let after = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        assert!(after.player().stamp().is_fresh() && after.entity().stamp().is_fresh());
    }
}

async fn refuse_absent_delete(store: &dyn Store, pair: &OwnedPlayerEntitySnapshot, n: u8) {
    let p = PlayerOwnershipCasRequest::new(
        id(n),
        pair.player().expectation().clone(),
        PlayerOwnershipProposal::delete(),
    )
    .unwrap();
    let e = EntityOwnershipCasRequest::new(
        id(n + 1),
        pair.entity().expectation().clone(),
        EntityOwnershipProposal::delete(),
    )
    .unwrap();
    let queries = [p.receipt_query(), e.receipt_query()];
    assert!(matches!(
        store.compare_exchange_owned_player(p).await,
        Err(OwnershipError::AlreadyAbsent {
            resource: OwnershipResource::Player,
            ..
        })
    ));
    assert!(matches!(
        store.compare_exchange_owned_entity(e).await,
        Err(OwnershipError::AlreadyAbsent {
            resource: OwnershipResource::Entity,
            ..
        })
    ));
    for query in queries {
        assert!(store.load_ownership_receipt(query).await.unwrap().is_none());
    }
}
#[tokio::test]
async fn absence_and_complete_deletion_retain_tombstones_and_global_history() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.redb");
        let store = store(durable, &path, config(MAX_OWNERSHIP_REQUEST_BYTES));
        let before = fresh(&*store).await;
        refuse_absent_delete(&*store, &before, 1).await;
        let first = put_pair(1, &before, p(vec![1]), e(vec![1]));
        store
            .commit_player_entity_transfer(first.clone())
            .await
            .unwrap();
        let live = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        let delete = PlayerEntityTransferRequest::new(
            id(2),
            live.player().expectation().clone(),
            live.entity().expectation().clone(),
            PlayerOwnershipProposal::delete(),
            EntityOwnershipProposal::delete(),
        )
        .unwrap();
        store.commit_player_entity_transfer(delete).await.unwrap();
        let absent = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        assert!(absent.player().stamp().is_tombstone() && absent.entity().stamp().is_tombstone());
        refuse_absent_delete(&*store, &absent, 3).await;
        assert!(matches!(
            store
                .ensure_generation_identity(
                    GenerationKey::new(WorldId::new(5), DimensionId::new(2)),
                    identity()
                )
                .await,
            Err(ServerError::InvalidState(_))
        ));
        assert_eq!(
            store
                .commit_player_entity_transfer(first.clone())
                .await
                .unwrap()
                .disposition(),
            OwnershipCommitDisposition::Replayed
        );
        drop(store);
        if durable {
            let reopened = RedbStore::open_with_player_entity_ownership(
                &path,
                config(MAX_OWNERSHIP_REQUEST_BYTES),
            )
            .unwrap();
            let absent = reopened
                .load_owned_pair(player(), entity(), identity())
                .await
                .unwrap();
            assert_eq!(absent.player().stamp().revision().unwrap().get(), 2);
            assert_eq!(absent.entity().stamp().revision().unwrap().get(), 2);
            assert!(matches!(
                reopened
                    .ensure_generation_identity(
                        GenerationKey::new(WorldId::new(5), DimensionId::new(2)),
                        identity()
                    )
                    .await,
                Err(ServerError::InvalidState(_))
            ));
            assert_eq!(
                reopened
                    .commit_player_entity_transfer(first)
                    .await
                    .unwrap()
                    .disposition(),
                OwnershipCommitDisposition::Replayed
            );
        }
    }
}

#[tokio::test]
async fn ownership_mode_preserves_bounded_plugin_namespaces() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.redb");
        let store = store(durable, &path, config(MAX_OWNERSHIP_REQUEST_BYTES));
        let a = PluginId::new("a".repeat(4096));
        let b = PluginId::new("b".repeat(4096));
        let oversized = PluginId::new("a".repeat(4097));
        let key = StorageKey::new("boundary").unwrap();
        store.put(&a, key.clone(), vec![7]).await.unwrap();
        store.put(&b, key.clone(), vec![8]).await.unwrap();
        assert_eq!(store.get(&a, &key).await.unwrap(), Some(vec![7]));
        assert_eq!(store.get(&b, &key).await.unwrap(), Some(vec![8]));
        assert!(store.put(&oversized, key.clone(), vec![9]).await.is_err());
        assert!(store.get(&oversized, &key).await.is_err());
        assert!(store.delete(&oversized, &key).await.is_err());
        assert!(store.keys(&oversized).await.is_err());
        assert!(store.delete(&a, &key).await.unwrap());
        assert!(store.get(&a, &key).await.unwrap().is_none());
        assert_eq!(store.get(&b, &key).await.unwrap(), Some(vec![8]));
        drop(store);
        if durable {
            let reopened = RedbStore::open_with_player_entity_ownership(
                &path,
                config(MAX_OWNERSHIP_REQUEST_BYTES),
            )
            .unwrap();
            assert_eq!(reopened.get(&b, &key).await.unwrap(), Some(vec![8]));
        }
    }
}
struct Unsupported;
#[async_trait]
impl PlayerEntityOwnershipStore for Unsupported {}
async fn unsupported(store: &dyn PlayerEntityOwnershipStore, pair: &OwnedPlayerEntitySnapshot) {
    let player_request = PlayerOwnershipCasRequest::new(
        id(1),
        pair.player().expectation().clone(),
        PlayerOwnershipProposal::put(p(vec![])),
    )
    .unwrap();
    let entity_request = EntityOwnershipCasRequest::new(
        id(2),
        pair.entity().expectation().clone(),
        EntityOwnershipProposal::put(e(vec![])),
    )
    .unwrap();
    let r = put_pair(3, pair, p(vec![]), e(vec![]));
    let query = r.receipt_query();
    assert!(matches!(
        store.load_owned_player(player(), identity()).await,
        Err(OwnershipError::Unsupported)
    ));
    assert!(matches!(
        store.load_owned_entity(entity(), identity()).await,
        Err(OwnershipError::Unsupported)
    ));
    assert!(matches!(
        store.load_owned_pair(player(), entity(), identity()).await,
        Err(OwnershipError::Unsupported)
    ));
    assert!(matches!(
        store.compare_exchange_owned_player(player_request).await,
        Err(OwnershipError::Unsupported)
    ));
    assert!(matches!(
        store.compare_exchange_owned_entity(entity_request).await,
        Err(OwnershipError::Unsupported)
    ));
    assert!(matches!(
        store.commit_player_entity_transfer(r).await,
        Err(OwnershipError::Unsupported)
    ));
    assert!(matches!(
        store.load_ownership_receipt(query).await,
        Err(OwnershipError::Unsupported)
    ));
}
#[tokio::test]
async fn all_seven_calls_refuse_default_and_every_older_mode() {
    let source =
        InMemoryStore::with_player_entity_ownership(config(MAX_OWNERSHIP_REQUEST_BYTES)).unwrap();
    let pair = fresh(&source).await;
    unsupported(&Unsupported, &pair).await;
    for store in [
        InMemoryStore::new(),
        InMemoryStore::with_generation_metadata(),
        InMemoryStore::with_uuid_entities(),
    ] {
        unsupported(&store, &pair).await;
    }
    let dir = tempfile::tempdir().unwrap();
    for mode in 3..=5 {
        let path = dir.path().join(format!("old-{mode}.redb"));
        let store = match mode {
            3 => RedbStore::open(&path),
            4 => RedbStore::open_with_generation_metadata(&path),
            _ => RedbStore::open_with_uuid_entities(&path),
        }
        .unwrap();
        unsupported(&store, &pair).await;
    }
}

fn collision_variant(pair: &OwnedPlayerEntitySnapshot, case: u8) -> PlayerEntityTransferRequest {
    let mut player_record = Some(p(vec![1]));
    let mut entity_record = Some(e(vec![1]));
    match case {
        0 => player_record = Some(p(vec![2])),
        1 => {
            player_record = Some(
                PlayerRecord::new(SchemaVersion::new(0), GameMode::Survival, vec![1]).unwrap(),
            );
        }
        2 => {
            player_record = Some(
                PlayerRecord::new(SchemaVersion::new(u32::MAX), GameMode::Creative, vec![1])
                    .unwrap(),
            );
        }
        3 => {
            entity_record = Some(
                PersistentEntityRecord::new(
                    ChunkPos::new(0, -2),
                    EntityRecord::new(SchemaVersion::new(0), vec![1]).unwrap(),
                )
                .unwrap(),
            );
        }
        4 => {
            entity_record = Some(
                PersistentEntityRecord::new(
                    ChunkPos::new(1, -2),
                    EntityRecord::new(SchemaVersion::new(u32::MAX - 1), vec![1]).unwrap(),
                )
                .unwrap(),
            );
        }
        5 => entity_record = Some(e(vec![2])),
        6 => player_record = None,
        7 => entity_record = None,
        _ => unreachable!(),
    }
    PlayerEntityTransferRequest::new(
        id(1),
        pair.player().expectation().clone(),
        pair.entity().expectation().clone(),
        player_record.map_or_else(
            PlayerOwnershipProposal::delete,
            PlayerOwnershipProposal::put,
        ),
        entity_record.map_or_else(
            EntityOwnershipProposal::delete,
            EntityOwnershipProposal::put,
        ),
    )
    .unwrap()
}
async fn alternate_pair(
    alternate: GenerationKey,
    generation: GenerationIdentity,
) -> OwnedPlayerEntitySnapshot {
    let store =
        InMemoryStore::with_player_entity_ownership(config(MAX_OWNERSHIP_REQUEST_BYTES)).unwrap();
    store
        .ensure_generation_identity(alternate, generation)
        .await
        .unwrap();
    store
        .load_owned_pair(
            ScopedPlayerKey::new(alternate, player().id()),
            PersistentEntityKey::new(alternate.world(), alternate.dimension(), entity().uuid()),
            generation,
        )
        .await
        .unwrap()
}
#[tokio::test]
async fn receipt_id_collision_checks_all_semantic_request_fields_before_cas() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = store(
            durable,
            &dir.path().join("collision.redb"),
            config(MAX_OWNERSHIP_REQUEST_BYTES),
        );
        let before = fresh(&*store).await;
        let original = put_pair(1, &before, p(vec![1]), e(vec![1]));
        let outcome = store
            .commit_player_entity_transfer(original.clone())
            .await
            .unwrap();
        let after = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        for case in 0..8 {
            assert!(
                matches!(
                    store
                        .commit_player_entity_transfer(collision_variant(&before, case))
                        .await,
                    Err(OwnershipError::ReceiptCollision { .. })
                ),
                "collision case{case}"
            );
        }
        let changed_expectation = put_pair(1, &after, p(vec![1]), e(vec![1]));
        assert!(matches!(
            store
                .commit_player_entity_transfer(changed_expectation)
                .await,
            Err(OwnershipError::ReceiptCollision { .. })
        ));
        let player_only = PlayerOwnershipCasRequest::new(
            id(1),
            before.player().expectation().clone(),
            PlayerOwnershipProposal::put(p(vec![1])),
        )
        .unwrap();
        assert!(matches!(
            store.compare_exchange_owned_player(player_only).await,
            Err(OwnershipError::ReceiptCollision { .. })
        ));
        let entity_only = EntityOwnershipCasRequest::new(
            id(1),
            before.entity().expectation().clone(),
            EntityOwnershipProposal::put(e(vec![1])),
        )
        .unwrap();
        assert!(matches!(
            store.compare_exchange_owned_entity(entity_only).await,
            Err(OwnershipError::ReceiptCollision { .. })
        ));
        for pair in [
            alternate_pair(
                GenerationKey::new(WorldId::new(5), DimensionId::new(2)),
                identity(),
            )
            .await,
            alternate_pair(
                scope(),
                GenerationIdentity::flat_overworld(WorldSeed::new(38)),
            )
            .await,
        ] {
            assert!(matches!(
                store
                    .commit_player_entity_transfer(put_pair(1, &pair, p(vec![1]), e(vec![1])))
                    .await,
                Err(OwnershipError::ReceiptCollision { .. })
            ));
        }
        let current = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        assert_eq!(current.player().record(), after.player().record());
        assert_eq!(current.entity().record(), after.entity().record());
        assert_eq!(current.player().stamp(), after.player().stamp());
        assert_eq!(current.entity().stamp(), after.entity().stamp());
        assert_eq!(
            store
                .commit_player_entity_transfer(original)
                .await
                .unwrap()
                .receipt(),
            outcome.receipt()
        );
    }
}
