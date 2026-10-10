//! Independent consumer-visible state-machine probes for both real storage producers.

use std::collections::BTreeMap;
use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};

use ferrumc_core::{DimensionId, EntityId, EntityUuid, GameMode, PlayerId, ServerError, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::*;
use ferrumc_world::{GenerationIdentity, WorldSeed};

trait Store: PlayerEntityOwnershipStore + WorldStore + PlayerStore + PersistentEntityStore {}
impl<T: PlayerEntityOwnershipStore + WorldStore + PlayerStore + PersistentEntityStore> Store for T {}
fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(37))
}
fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(4), DimensionId::new(2))
}
fn player() -> ScopedPlayerKey {
    ScopedPlayerKey::new(scope(), PlayerId::offline("OwnershipFixture"))
}
fn entity() -> PersistentEntityKey {
    PersistentEntityKey::new(
        scope().world(),
        scope().dimension(),
        EntityUuid::from_bytes([0xA5; 16]),
    )
}
fn id(n: u16) -> OwnershipTransferId {
    let mut b = [0x6B; 16];
    b[..2].copy_from_slice(&n.to_be_bytes());
    OwnershipTransferId::from_bytes(b)
}
fn config(receipts: u64, revisions: u64) -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            NonZeroUsize::new(MAX_OWNERSHIP_REQUEST_BYTES).unwrap(),
            NonZeroU64::new(receipts).unwrap(),
            NonZeroU64::new(revisions).unwrap(),
        )
        .unwrap(),
        OwnershipOpenLimits::new(
            NonZeroU64::new(1_000_000).unwrap(),
            NonZeroU64::new(128 * 1024 * 1024).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU16::new(8).unwrap(),
        )
        .unwrap(),
    )
}
fn p(bytes: &[u8]) -> PlayerRecord {
    PlayerRecord::new(SchemaVersion::new(19), GameMode::Survival, bytes.to_vec()).unwrap()
}
fn e(chunk: i32, bytes: &[u8]) -> PersistentEntityRecord {
    PersistentEntityRecord::new(
        ChunkPos::new(chunk, -2),
        EntityRecord::new(SchemaVersion::new(7), bytes.to_vec()).unwrap(),
    )
    .unwrap()
}

#[derive(Default)]
struct Model {
    player: Option<Vec<u8>>,
    entity: Option<(i32, Vec<u8>)>,
    player_revision: u64,
    entity_revision: u64,
    epochs: BTreeMap<i32, u64>,
}
impl Model {
    fn put_player(&mut self, value: Option<Vec<u8>>) {
        assert!(value.is_some() || self.player.is_some());
        if self.player != value {
            self.player_revision += 1;
            self.player = value;
        }
    }
    fn put_entity(&mut self, value: Option<(i32, Vec<u8>)>) {
        assert!(value.is_some() || self.entity.is_some());
        if self.entity != value {
            let mut affected = std::collections::BTreeSet::new();
            if let Some((x, _)) = &self.entity {
                affected.insert(*x);
            }
            if let Some((x, _)) = &value {
                affected.insert(*x);
            }
            for x in affected {
                *self.epochs.entry(x).or_default() += 1;
            }
            self.entity_revision += 1;
            self.entity = value;
        }
    }
    async fn assert(&self, store: &dyn Store) {
        let pair = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        assert_eq!(
            pair.player().record().map(PlayerRecord::data),
            self.player.as_deref()
        );
        assert_eq!(
            pair.entity()
                .record()
                .map(|r| (r.chunk().x(), r.record().data())),
            self.entity.as_ref().map(|(x, b)| (*x, b.as_slice()))
        );
        for (stamp, revision, live) in [
            (
                pair.player().stamp(),
                self.player_revision,
                self.player.is_some(),
            ),
            (
                pair.entity().stamp(),
                self.entity_revision,
                self.entity.is_some(),
            ),
        ] {
            assert_eq!(
                stamp.revision().map(OwnershipRevision::get),
                if revision == 0 { None } else { Some(revision) }
            );
            assert_eq!(stamp.is_live(), live);
            assert_eq!(stamp.is_fresh(), revision == 0);
            assert_eq!(stamp.is_tombstone(), revision != 0 && !live);
        }
        for (x, expected) in &self.epochs {
            let page = store
                .load_persistent_entity_page(
                    ChunkKey::new(scope().world(), scope().dimension(), ChunkPos::new(*x, -2)),
                    identity(),
                    None,
                    NonZeroU16::new(1).unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(page.epoch().get(), *expected);
            assert_eq!(
                page.entries().len(),
                usize::from(self.entity.as_ref().is_some_and(|(actual, _)| actual == x))
            );
        }
    }
}
async fn pair_request(
    store: &dyn Store,
    n: u16,
    payload: Option<&[u8]>,
    item: Option<(i32, &[u8])>,
) -> PlayerEntityTransferRequest {
    let before = store
        .load_owned_pair(player(), entity(), identity())
        .await
        .unwrap();
    PlayerEntityTransferRequest::new(
        id(n),
        before.player().expectation().clone(),
        before.entity().expectation().clone(),
        payload.map_or_else(PlayerOwnershipProposal::delete, |b| {
            PlayerOwnershipProposal::put(p(b))
        }),
        item.map_or_else(EntityOwnershipProposal::delete, |(x, b)| {
            EntityOwnershipProposal::put(e(x, b))
        }),
    )
    .unwrap()
}

async fn initial_transfers(
    store: &dyn Store,
) -> (
    Model,
    OwnedPlayerSnapshot,
    PlayerEntityTransferRequest,
    OwnershipCommitOutcome,
) {
    assert!(matches!(
        store.load_owned_pair(player(), entity(), identity()).await,
        Err(OwnershipError::Record(
            StorageError::MissingGenerationIdentity { .. }
        ))
    ));
    store
        .ensure_generation_identity(scope(), identity())
        .await
        .unwrap();
    let mut model = Model::default();
    model.assert(store).await;
    let initial = store.load_owned_player(player(), identity()).await.unwrap();
    let bootstrap = PlayerOwnershipCasRequest::new(
        id(1),
        initial.expectation().clone(),
        PlayerOwnershipProposal::put(p(&[10, 0xEF])),
    )
    .unwrap();
    store
        .compare_exchange_owned_player(bootstrap)
        .await
        .unwrap();
    model.put_player(Some(vec![10, 0xEF]));
    model.assert(store).await;
    let drop = pair_request(store, 2, Some(&[8, 0xEF]), Some((0, &[2, 0xBA]))).await;
    let original = store
        .commit_player_entity_transfer(drop.clone())
        .await
        .unwrap();
    assert_eq!(
        original.disposition(),
        OwnershipCommitDisposition::Committed
    );
    model.put_player(Some(vec![8, 0xEF]));
    model.put_entity(Some((0, vec![2, 0xBA])));
    model.assert(store).await;
    let first = pair_request(store, 3, Some(&[9, 0xEF]), Some((0, &[1, 0xBA]))).await;
    let stale = pair_request(store, 4, Some(&[10, 0xEF]), None).await;
    store.commit_player_entity_transfer(first).await.unwrap();
    model.put_player(Some(vec![9, 0xEF]));
    model.put_entity(Some((0, vec![1, 0xBA])));
    model.assert(store).await;
    assert!(matches!(
        store.commit_player_entity_transfer(stale.clone()).await,
        Err(OwnershipError::CasConflict { .. })
    ));
    assert!(store
        .load_ownership_receipt(stale.receipt_query())
        .await
        .unwrap()
        .is_none());
    model.assert(store).await;
    (model, initial, drop, original)
}
async fn delete_recreate_and_replay(
    store: &dyn Store,
    model: &mut Model,
    initial: &OwnedPlayerSnapshot,
    drop: PlayerEntityTransferRequest,
    original: &OwnershipCommitOutcome,
) {
    let full = pair_request(store, 5, Some(&[10, 0xEF]), None).await;
    store.commit_player_entity_transfer(full).await.unwrap();
    model.put_player(Some(vec![10, 0xEF]));
    model.put_entity(None);
    model.assert(store).await;
    let old_entity = store.load_owned_entity(entity(), identity()).await.unwrap();
    assert!(old_entity.stamp().is_tombstone());
    let recreate = pair_request(store, 6, Some(&[7, 0xEF]), Some((0, &[3, 0xBA]))).await;
    store.commit_player_entity_transfer(recreate).await.unwrap();
    model.put_player(Some(vec![7, 0xEF]));
    model.put_entity(Some((0, vec![3, 0xBA])));
    model.assert(store).await;
    assert!(matches!(
        store
            .compare_exchange_owned_entity(
                EntityOwnershipCasRequest::new(
                    id(7),
                    old_entity.expectation().clone(),
                    EntityOwnershipProposal::put(e(1, &[3, 0xBA]))
                )
                .unwrap()
            )
            .await,
        Err(OwnershipError::CasConflict { .. })
    ));
    let replay = store
        .commit_player_entity_transfer(drop.clone())
        .await
        .unwrap();
    assert_eq!(replay.disposition(), OwnershipCommitDisposition::Replayed);
    assert_eq!(replay.receipt(), original.receipt());
    model.assert(store).await;
    let collision = PlayerEntityTransferRequest::new(
        id(2),
        initial.expectation().clone(),
        old_entity.expectation().clone(),
        PlayerOwnershipProposal::put(p(&[99])),
        EntityOwnershipProposal::put(e(3, &[9])),
    )
    .unwrap();
    assert!(matches!(
        store.commit_player_entity_transfer(collision).await,
        Err(OwnershipError::ReceiptCollision { .. })
    ));
    model.assert(store).await;
}
async fn entity_trace_and_noop(store: &dyn Store, model: &mut Model) {
    // A deterministic independent trace exercises chunk moves, raw no-ops and repeated deletion/recreation.
    for n in 10..34u16 {
        let before = store.load_owned_entity(entity(), identity()).await.unwrap();
        let next = if n % 5 == 0 && model.entity.is_some() {
            None
        } else if n % 3 == 0 && model.entity.is_some() {
            model.entity.clone()
        } else {
            Some((i32::from(n % 4) - 2, vec![n as u8, 0xBA]))
        };
        let proposal = next
            .as_ref()
            .map_or_else(EntityOwnershipProposal::delete, |(x, b)| {
                EntityOwnershipProposal::put(e(*x, b))
            });
        let req =
            EntityOwnershipCasRequest::new(id(n), before.expectation().clone(), proposal).unwrap();
        let outcome = store
            .compare_exchange_owned_entity(req.clone())
            .await
            .unwrap();
        model.put_entity(next);
        model.assert(store).await;
        assert_eq!(
            store
                .compare_exchange_owned_entity(req)
                .await
                .unwrap()
                .receipt(),
            outcome.receipt()
        );
    }
    let before = store
        .load_owned_pair(player(), entity(), identity())
        .await
        .unwrap();
    let no_op = pair_request(
        store,
        40,
        model.player.as_deref(),
        model.entity.as_ref().map(|(x, b)| (*x, b.as_slice())),
    )
    .await;
    let no_op_receipt = store.commit_player_entity_transfer(no_op).await.unwrap();
    assert_eq!(
        no_op_receipt.receipt().player().unwrap().1,
        no_op_receipt.receipt().player().unwrap().2
    );
    assert_eq!(
        before.player().stamp(),
        no_op_receipt.receipt().player().unwrap().2
    );
    model.assert(store).await;
}
async fn old_writers_are_refused(store: &dyn Store, model: &Model) {
    assert!(matches!(
        store.save_player(player().id(), p(&[])).await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store.delete_player(player().id()).await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store.load_player(player().id()).await,
        Err(ServerError::Unsupported(_))
    ));
    let wire = EntityKey::new(scope().world(), scope().dimension(), EntityId::new(17));
    assert!(matches!(
        store.load_entity(wire).await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store
            .save_entity(
                wire,
                EntityRecord::new(SchemaVersion::new(0), vec![]).unwrap()
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
    assert!(matches!(
        store.save_persistent_entities(vec![]).await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store.delete_persistent_entity(entity(), identity()).await,
        Err(ServerError::Unsupported(_))
    ));
    assert!(matches!(
        store
            .save_persistent_entity(PersistentEntityWrite::new(entity(), identity(), e(0, &[])))
            .await,
        Err(ServerError::Unsupported(_))
    ));
    let dest = PersistentEntityKey::new(scope().world(), DimensionId::new(3), entity().uuid());
    assert!(matches!(
        store
            .move_persistent_entity(
                entity(),
                identity(),
                PersistentEntityWrite::new(dest, identity(), e(0, &[]))
            )
            .await,
        Err(ServerError::Unsupported(_))
    ));
    model.assert(store).await;
}
async fn contract(store: &dyn Store) {
    let (mut model, initial, drop, original) = initial_transfers(store).await;
    delete_recreate_and_replay(store, &mut model, &initial, drop, &original).await;
    entity_trace_and_noop(store, &mut model).await;
    old_writers_are_refused(store, &model).await;
}

#[tokio::test]
async fn independent_model_memory() {
    let store = InMemoryStore::with_player_entity_ownership(config(100, 8)).unwrap();
    contract(&store).await;
}
#[tokio::test]
async fn independent_model_redb_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ownership.redb");
    let store = RedbStore::open_with_player_entity_ownership(&path, config(100, 8)).unwrap();
    contract(&store).await;
    let before = store
        .load_owned_pair(player(), entity(), identity())
        .await
        .unwrap();
    drop(store);
    let reopened = RedbStore::open_with_player_entity_ownership(&path, config(100, 8)).unwrap();
    let after = reopened
        .load_owned_pair(player(), entity(), identity())
        .await
        .unwrap();
    assert_eq!(before.player().record(), after.player().record());
    assert_eq!(before.entity().record(), after.entity().record());
    assert_eq!(before.player().stamp(), after.player().stamp());
    assert_eq!(before.entity().stamp(), after.entity().stamp());
}

#[tokio::test]
async fn capacity_and_foreign_owner_refuse_without_consuming_id() {
    for durable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store: Box<dyn Store> = if durable {
            Box::new(
                RedbStore::open_with_player_entity_ownership(
                    dir.path().join("small.redb"),
                    config(2, 2),
                )
                .unwrap(),
            )
        } else {
            Box::new(InMemoryStore::with_player_entity_ownership(config(2, 2)).unwrap())
        };
        store
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let first = pair_request(&*store, 1, Some(&[1]), Some((0, &[1]))).await;
        store
            .commit_player_entity_transfer(first.clone())
            .await
            .unwrap();
        let no_op = pair_request(&*store, 2, Some(&[1]), Some((0, &[1]))).await;
        store
            .commit_player_entity_transfer(no_op.clone())
            .await
            .unwrap();
        let full = pair_request(&*store, 3, Some(&[2]), None).await;
        assert!(matches!(
            store.commit_player_entity_transfer(full.clone()).await,
            Err(OwnershipError::HistoryCapacity {
                kind: OwnershipHistory::Receipts,
                ..
            })
        ));
        assert!(store
            .load_ownership_receipt(full.receipt_query())
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .commit_player_entity_transfer(first)
                .await
                .unwrap()
                .disposition(),
            OwnershipCommitDisposition::Replayed
        );
        assert_eq!(
            store
                .commit_player_entity_transfer(no_op)
                .await
                .unwrap()
                .disposition(),
            OwnershipCommitDisposition::Replayed
        );
        let other = InMemoryStore::with_player_entity_ownership(config(8, 8)).unwrap();
        other
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let foreign = pair_request(&other, 4, Some(&[4]), Some((0, &[4]))).await;
        assert!(matches!(
            store.commit_player_entity_transfer(foreign).await,
            Err(OwnershipError::BackendOwnerMismatch)
        ));
    }
}

#[tokio::test]
async fn positive_configuration_and_revision_capacity() {
    assert!(OwnershipCapacity::new(
        NonZeroUsize::new(148).unwrap(),
        NonZeroU64::new(1).unwrap(),
        NonZeroU64::new(1).unwrap()
    )
    .is_err());
    assert!(OwnershipCapacity::new(
        NonZeroUsize::new(MAX_OWNERSHIP_REQUEST_BYTES + 1).unwrap(),
        NonZeroU64::new(1).unwrap(),
        NonZeroU64::new(1).unwrap()
    )
    .is_err());
    assert!(OwnershipCapacity::new(
        NonZeroUsize::new(149).unwrap(),
        NonZeroU64::new(u64::MAX).unwrap(),
        NonZeroU64::new(1).unwrap()
    )
    .is_err());
    let store = InMemoryStore::with_player_entity_ownership(config(8, 1)).unwrap();
    store
        .ensure_generation_identity(scope(), identity())
        .await
        .unwrap();
    let req = pair_request(&store, 1, Some(&[1]), Some((0, &[1]))).await;
    assert!(matches!(
        store.commit_player_entity_transfer(req.clone()).await,
        Err(OwnershipError::HistoryCapacity {
            kind: OwnershipHistory::Revisions,
            ..
        })
    ));
    assert!(store
        .load_ownership_receipt(req.receipt_query())
        .await
        .unwrap()
        .is_none());
    let pair = store
        .load_owned_pair(player(), entity(), identity())
        .await
        .unwrap();
    assert!(pair.player().stamp().is_fresh() && pair.entity().stamp().is_fresh());
}
