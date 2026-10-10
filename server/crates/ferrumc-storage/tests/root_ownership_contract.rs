//! Independent public-consumer checks for replay precedence and historical CAS authority.

use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};

use ferrumc_core::{DimensionId, EntityUuid, GameMode, PlayerId, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::*;
use ferrumc_world::{GenerationIdentity, WorldSeed};

trait Store: PlayerEntityOwnershipStore + WorldStore + PersistentEntityStore {}
impl<T: PlayerEntityOwnershipStore + WorldStore + PersistentEntityStore> Store for T {}

fn config() -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            NonZeroUsize::new(MAX_OWNERSHIP_REQUEST_BYTES).unwrap(),
            NonZeroU64::new(3).unwrap(),
            NonZeroU64::new(2).unwrap(),
        )
        .unwrap(),
        OwnershipOpenLimits::new(
            NonZeroU64::new(1_000).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU16::new(2).unwrap(),
        )
        .unwrap(),
    )
}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(11), DimensionId::new(3))
}
fn generation() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(-713))
}
fn player() -> ScopedPlayerKey {
    ScopedPlayerKey::new(scope(), PlayerId::offline("RootReceiptAuthority"))
}
fn entity() -> PersistentEntityKey {
    PersistentEntityKey::new(
        scope().world(),
        scope().dimension(),
        EntityUuid::from_bytes([0xED; 16]),
    )
}
fn id(value: u8) -> OwnershipTransferId {
    let mut bytes = [0x1D; 16];
    bytes[15] = value;
    OwnershipTransferId::from_bytes(bytes)
}
fn p(count: u8) -> PlayerRecord {
    PlayerRecord::new(
        SchemaVersion::new(42),
        GameMode::Survival,
        vec![count, 0xFA],
    )
    .unwrap()
}
fn e(count: u8) -> PersistentEntityRecord {
    PersistentEntityRecord::new(
        ChunkPos::new(-2, 7),
        EntityRecord::new(SchemaVersion::new(53), vec![count, 0xCE]).unwrap(),
    )
    .unwrap()
}
async fn request(
    store: &dyn Store,
    operation: u8,
    player_count: u8,
    entity_count: u8,
) -> PlayerEntityTransferRequest {
    let before = store
        .load_owned_pair(player(), entity(), generation())
        .await
        .unwrap();
    PlayerEntityTransferRequest::new(
        id(operation),
        before.player().expectation().clone(),
        before.entity().expectation().clone(),
        PlayerOwnershipProposal::put(p(player_count)),
        EntityOwnershipProposal::put(e(entity_count)),
    )
    .unwrap()
}
async fn verify_current(store: &dyn Store, player_count: u8, player_revision: u64) {
    let current = store
        .load_owned_pair(player(), entity(), generation())
        .await
        .unwrap();
    assert_eq!(
        current.player().record().unwrap().data(),
        &[player_count, 0xFA]
    );
    assert_eq!(
        current.entity().record().unwrap().record().data(),
        &[3, 0xCE]
    );
    assert_eq!(
        current.player().stamp().revision().unwrap().get(),
        player_revision
    );
    assert_eq!(current.entity().stamp().revision().unwrap().get(), 2);
    let page = store
        .load_persistent_entity_page(
            ChunkKey::new(scope().world(), scope().dimension(), ChunkPos::new(-2, 7)),
            generation(),
            None,
            NonZeroU16::new(1).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.epoch().get(), 2);
    assert_eq!(page.entries().len(), 1);
}

async fn dead_owner_sequence(
    store: &dyn Store,
) -> (
    PlayerEntityTransferRequest,
    PlayerEntityTransferRequest,
    OwnershipCommitOutcome,
) {
    let donor = InMemoryStore::with_player_entity_ownership(config()).unwrap();
    donor
        .ensure_generation_identity(scope(), generation())
        .await
        .unwrap();
    let original = request(store, 1, 10, 2).await;
    let foreign_identical = request(&donor, 1, 10, 2).await;
    let collision = request(&donor, 1, 11, 2).await;
    let unused_foreign = request(&donor, 3, 9, 3).await;
    assert_eq!(original.receipt_query(), foreign_identical.receipt_query());
    assert_ne!(original.receipt_query(), collision.receipt_query());
    drop(donor);

    let committed = store.commit_player_entity_transfer(original).await.unwrap();
    assert_eq!(
        committed.disposition(),
        OwnershipCommitDisposition::Committed
    );
    assert!(matches!(
        store.commit_player_entity_transfer(collision.clone()).await,
        Err(OwnershipError::ReceiptCollision { .. })
    ));
    assert!(matches!(
        store.commit_player_entity_transfer(unused_foreign).await,
        Err(OwnershipError::BackendOwnerMismatch)
    ));
    store
        .commit_player_entity_transfer(request(store, 3, 9, 3).await)
        .await
        .unwrap();
    verify_current(store, 9, 2).await;

    (foreign_identical, collision, committed)
}

async fn stale_replay_and_capacity(
    store: &dyn Store,
    foreign_identical: &PlayerEntityTransferRequest,
    committed: &OwnershipCommitOutcome,
) {
    let replay = store
        .commit_player_entity_transfer(foreign_identical.clone())
        .await
        .unwrap();
    assert_eq!(replay.disposition(), OwnershipCommitDisposition::Replayed);
    assert_eq!(replay.receipt(), committed.receipt());
    assert_eq!(
        replay
            .player_expectation()
            .unwrap()
            .stamp()
            .revision()
            .unwrap()
            .get(),
        1
    );
    let stale = PlayerOwnershipCasRequest::new(
        id(4),
        replay.player_expectation().unwrap().clone(),
        PlayerOwnershipProposal::put(p(8)),
    )
    .unwrap();
    assert!(matches!(
        store.compare_exchange_owned_player(stale.clone()).await,
        Err(OwnershipError::CasConflict {
            resource: OwnershipResource::Player,
            ..
        })
    ));
    assert!(store
        .load_ownership_receipt(stale.receipt_query())
        .await
        .unwrap()
        .is_none());
    verify_current(store, 9, 2).await;
    let fresh = store
        .load_owned_player(player(), generation())
        .await
        .unwrap();
    store
        .compare_exchange_owned_player(
            PlayerOwnershipCasRequest::new(
                id(4),
                fresh.expectation().clone(),
                PlayerOwnershipProposal::put(p(8)),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    verify_current(store, 8, 3).await;

    let full = request(store, 5, 8, 3).await;
    assert!(matches!(
        store.commit_player_entity_transfer(full.clone()).await,
        Err(OwnershipError::HistoryCapacity {
            kind: OwnershipHistory::Receipts,
            max: 3
        })
    ));
    assert!(store
        .load_ownership_receipt(full.receipt_query())
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .commit_player_entity_transfer(foreign_identical.clone())
            .await
            .unwrap()
            .receipt(),
        committed.receipt()
    );
    verify_current(store, 8, 3).await;
}

async fn check(durable: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root-receipts.redb");
    let store: Box<dyn Store> = if durable {
        Box::new(RedbStore::open_with_player_entity_ownership(&path, config()).unwrap())
    } else {
        Box::new(InMemoryStore::with_player_entity_ownership(config()).unwrap())
    };
    store
        .ensure_generation_identity(scope(), generation())
        .await
        .unwrap();
    let (foreign_identical, collision, committed) = dead_owner_sequence(&*store).await;
    stale_replay_and_capacity(&*store, &foreign_identical, &committed).await;

    if durable {
        drop(store);
        let reopened = RedbStore::open_with_player_entity_ownership(&path, config()).unwrap();
        assert_eq!(
            reopened
                .load_ownership_receipt(foreign_identical.receipt_query())
                .await
                .unwrap()
                .as_ref(),
            Some(committed.receipt())
        );
        let replay = reopened
            .commit_player_entity_transfer(foreign_identical)
            .await
            .unwrap();
        assert_eq!(replay.disposition(), OwnershipCommitDisposition::Replayed);
        assert_eq!(replay.receipt(), committed.receipt());
        assert!(matches!(
            reopened.commit_player_entity_transfer(collision).await,
            Err(OwnershipError::ReceiptCollision { .. })
        ));
        verify_current(&reopened, 8, 3).await;
    }
}

#[tokio::test]
async fn memory_replay_precedes_dead_owner_and_stale_authority() {
    check(false).await;
}
#[tokio::test]
async fn redb_replay_precedes_dead_owner_and_preserves_reopen_state() {
    check(true).await;
}
