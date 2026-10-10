//! Concurrent consumer intents compete through the actual producer boundaries.

use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};
use std::sync::{Arc, Barrier};

use ferrumc_core::{DimensionId, EntityUuid, GameMode, PlayerId, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::*;
use ferrumc_world::{GenerationIdentity, WorldSeed};

trait Store: PlayerEntityOwnershipStore + WorldStore {}
impl<T: PlayerEntityOwnershipStore + WorldStore> Store for T {}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(4), DimensionId::new(2))
}
fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(37))
}
fn player(n: u8) -> ScopedPlayerKey {
    ScopedPlayerKey::new(scope(), PlayerId::offline(&format!("Collector-{n}")))
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
fn p(n: u8) -> PlayerRecord {
    PlayerRecord::new(SchemaVersion::new(71), GameMode::Survival, vec![n]).unwrap()
}
fn e(n: u8) -> PersistentEntityRecord {
    PersistentEntityRecord::new(
        ChunkPos::new(0, -2),
        EntityRecord::new(SchemaVersion::new(72), vec![n]).unwrap(),
    )
    .unwrap()
}
fn config() -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            NonZeroUsize::new(MAX_OWNERSHIP_REQUEST_BYTES).unwrap(),
            NonZeroU64::new(32).unwrap(),
            NonZeroU64::new(8).unwrap(),
        )
        .unwrap(),
        OwnershipOpenLimits::new(
            NonZeroU64::new(1000).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU16::new(8).unwrap(),
        )
        .unwrap(),
    )
}
fn store(durable: bool, path: &std::path::Path) -> Arc<dyn Store> {
    if durable {
        Arc::new(RedbStore::open_with_player_entity_ownership(path, config()).unwrap())
    } else {
        Arc::new(InMemoryStore::with_player_entity_ownership(config()).unwrap())
    }
}
async fn bootstrap(store: &dyn Store) {
    store
        .ensure_generation_identity(scope(), identity())
        .await
        .unwrap();
    let pair = store
        .load_owned_pair(player(1), entity(), identity())
        .await
        .unwrap();
    store
        .commit_player_entity_transfer(
            PlayerEntityTransferRequest::new(
                id(1),
                pair.player().expectation().clone(),
                pair.entity().expectation().clone(),
                PlayerOwnershipProposal::put(p(0)),
                EntityOwnershipProposal::put(e(1)),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let second = store
        .load_owned_player(player(2), identity())
        .await
        .unwrap();
    store
        .compare_exchange_owned_player(
            PlayerOwnershipCasRequest::new(
                id(2),
                second.expectation().clone(),
                PlayerOwnershipProposal::put(p(0)),
            )
            .unwrap(),
        )
        .await
        .unwrap();
}
enum Intent {
    Transfer(PlayerEntityTransferRequest),
    Autosave(PlayerOwnershipCasRequest),
}
async fn race(
    store: Arc<dyn Store>,
    intents: [Intent; 2],
) -> [OwnershipResult<OwnershipCommitOutcome>; 2] {
    tokio::task::spawn_blocking(move || {
        let barrier = Barrier::new(2);
        std::thread::scope(|threads| {
            let handles = intents.map(|intent| {
                let store = &store;
                let barrier = &barrier;
                threads.spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap();
                    barrier.wait();
                    runtime.block_on(async {
                        match intent {
                            Intent::Transfer(r) => store.commit_player_entity_transfer(r).await,
                            Intent::Autosave(r) => store.compare_exchange_owned_player(r).await,
                        }
                    })
                })
            });
            handles.map(|handle| handle.join().unwrap())
        })
    })
    .await
    .unwrap()
}
async fn pickup(store: &dyn Store, n: u8, operation: u8) -> PlayerEntityTransferRequest {
    let pair = store
        .load_owned_pair(player(n), entity(), identity())
        .await
        .unwrap();
    PlayerEntityTransferRequest::new(
        id(operation),
        pair.player().expectation().clone(),
        pair.entity().expectation().clone(),
        PlayerOwnershipProposal::put(p(1)),
        EntityOwnershipProposal::delete(),
    )
    .unwrap()
}

#[tokio::test]
async fn two_actual_collectors_publish_exactly_one_owner() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = store(durable, &dir.path().join("pickup.redb"));
        bootstrap(&*store).await;
        let a = pickup(&*store, 1, 3).await;
        let b = pickup(&*store, 2, 4).await;
        let queries = [a.receipt_query(), b.receipt_query()];
        let outcomes = race(store.clone(), [Intent::Transfer(a), Intent::Transfer(b)]).await;
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|r| matches!(
                    r,
                    Err(OwnershipError::CasConflict {
                        resource: OwnershipResource::Entity,
                        ..
                    })
                ))
                .count(),
            1
        );
        for (result, query) in outcomes.iter().zip(queries) {
            assert_eq!(
                store.load_ownership_receipt(query).await.unwrap().is_some(),
                result.is_ok()
            );
        }
        let item = store.load_owned_entity(entity(), identity()).await.unwrap();
        assert!(item.stamp().is_tombstone());
        assert_eq!(item.stamp().revision().unwrap().get(), 2);
        let a = store
            .load_owned_player(player(1), identity())
            .await
            .unwrap();
        let b = store
            .load_owned_player(player(2), identity())
            .await
            .unwrap();
        assert_eq!(
            a.record().unwrap().data()[0] + b.record().unwrap().data()[0],
            1
        );
        assert_eq!(
            a.stamp().revision().unwrap().get() + b.stamp().revision().unwrap().get(),
            3
        );
    }
}

#[tokio::test]
async fn player_autosave_and_pickup_cannot_overwrite_each_other() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = store(durable, &dir.path().join("autosave.redb"));
        bootstrap(&*store).await;
        let transfer = pickup(&*store, 1, 3).await;
        let before = store
            .load_owned_player(player(1), identity())
            .await
            .unwrap();
        let autosave = PlayerOwnershipCasRequest::new(
            id(4),
            before.expectation().clone(),
            PlayerOwnershipProposal::put(p(9)),
        )
        .unwrap();
        let outcomes = race(
            store.clone(),
            [Intent::Transfer(transfer), Intent::Autosave(autosave)],
        )
        .await;
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|r| matches!(
                    r,
                    Err(OwnershipError::CasConflict {
                        resource: OwnershipResource::Player,
                        ..
                    })
                ))
                .count(),
            1
        );
        let pair = store
            .load_owned_pair(player(1), entity(), identity())
            .await
            .unwrap();
        assert_eq!(pair.player().stamp().revision().unwrap().get(), 2);
        if outcomes[0].is_ok() {
            assert_eq!(pair.player().record(), Some(&p(1)));
            assert!(pair.entity().stamp().is_tombstone());
        } else {
            assert_eq!(pair.player().record(), Some(&p(9)));
            assert_eq!(pair.entity().record(), Some(&e(1)));
            assert_eq!(pair.entity().stamp().revision().unwrap().get(), 1);
        }
    }
}
