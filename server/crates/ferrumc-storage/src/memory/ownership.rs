//! All ownership rows publish under one state lock, also used by pair and UUID reads.

use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::Arc;

use async_trait::async_trait;
use ferrumc_world::GenerationIdentity;

use super::{poisoned, InMemoryStore};
use crate::ownership::{engine, raw, Request, Runtime};
use crate::persistent_entity::engine::{EntityRead, MemoryRead, RawRows, RawTable};
use crate::{
    ChunkKey, EntityOwnershipCasRequest, GenerationKey, OwnedEntitySnapshot,
    OwnedPlayerEntitySnapshot, OwnedPlayerSnapshot, OwnershipCommitDisposition,
    OwnershipCommitOutcome, OwnershipConfig, OwnershipError, OwnershipReceipt,
    OwnershipReceiptQuery, OwnershipResult, PersistentEntityCursor, PersistentEntityKey,
    PersistentEntityPage, PlayerEntityOwnershipStore, PlayerEntityTransferRequest,
    PlayerOwnershipCasRequest, ScopedPlayerKey, StorageError,
};

#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) uuid: crate::persistent_entity::engine::MemoryState,
    pub(super) players: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) player_headers: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) entity_headers: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) receipts: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) receipt_count: u64,
    pub(super) revision_count: u64,
}
impl State {
    pub(super) fn is_empty(&self) -> bool {
        self.uuid.is_empty()
            && self.players.is_empty()
            && self.player_headers.is_empty()
            && self.entity_headers.is_empty()
            && self.receipts.is_empty()
            && self.receipt_count == 0
            && self.revision_count == 0
    }
    /// Hooks are infallible during application: readers hold this same lock boundary.
    fn apply(&mut self, plan: engine::Plan, mut after: impl FnMut()) {
        if let Some(p) = plan.player {
            match p.record {
                Some(b) => {
                    self.players.insert(p.key.to_vec(), b);
                }
                None => {
                    self.players.remove(p.key.as_slice());
                }
            }
            after();
            self.player_headers
                .insert(p.key.to_vec(), raw::encode_player_header(p.header).to_vec());
            after();
        }
        self.uuid.apply(plan.entity);
        after();
        if let Some((key, h)) = plan.entity_header {
            self.entity_headers
                .insert(key.to_vec(), raw::encode_entity_header(h).to_vec());
            after();
        }
        self.receipts.insert(
            plan.receipt.query.id.to_bytes().to_vec(),
            raw::encode_receipt(&plan.receipt).to_vec(),
        );
        after();
        self.receipt_count = plan.receipt_count;
        after();
        self.revision_count = plan.revision_count;
        after();
    }
}
pub(super) struct Reader<'a> {
    pub(super) state: &'a State,
    pub(super) uuid: MemoryRead<'a>,
}
impl EntityRead for Reader<'_> {
    type Bytes<'a>
        = &'a [u8]
    where
        Self: 'a;
    fn get(
        &self,
        table: RawTable,
        key: &[u8],
    ) -> std::result::Result<Option<Self::Bytes<'_>>, StorageError> {
        self.uuid.get(table, key)
    }
    fn identity(
        &self,
        key: GenerationKey,
    ) -> std::result::Result<Option<GenerationIdentity>, StorageError> {
        self.uuid.identity(key)
    }
    fn index_rows(
        &self,
        lower: Bound<Vec<u8>>,
        upper: Bound<Vec<u8>>,
    ) -> std::result::Result<RawRows<'_, Self::Bytes<'_>>, StorageError> {
        self.uuid.index_rows(lower, upper)
    }
}
impl engine::Read for Reader<'_> {
    fn owned(&self, table: engine::Table, key: &[u8]) -> OwnershipResult<Option<Self::Bytes<'_>>> {
        let map = match table {
            engine::Table::Player => &self.state.players,
            engine::Table::PlayerHeader => &self.state.player_headers,
            engine::Table::EntityHeader => &self.state.entity_headers,
            engine::Table::Receipt => &self.state.receipts,
        };
        Ok(map.get(key).map(Vec::as_slice))
    }
    fn counts(&self) -> OwnershipResult<(u64, u64)> {
        Ok((self.state.receipt_count, self.state.revision_count))
    }
}
impl InMemoryStore {
    /// Creates a fresh owner with explicit permanent-history and local-work limits.
    pub fn with_player_entity_ownership(config: OwnershipConfig) -> OwnershipResult<Self> {
        Ok(Self {
            generation_mode: crate::generation::GenerationMode::Ownership,
            ownership_runtime: Some(Arc::new(Runtime::new(config))),
            ..Self::default()
        })
    }
    pub(super) fn ownership_runtime(&self) -> OwnershipResult<&Arc<Runtime>> {
        self.ownership_runtime
            .as_ref()
            .filter(|_| self.generation_mode == crate::generation::GenerationMode::Ownership)
            .ok_or(OwnershipError::Unsupported)
    }
    fn read_owned<T>(
        &self,
        f: impl FnOnce(&Reader<'_>) -> OwnershipResult<T>,
    ) -> OwnershipResult<T> {
        let runtime = self.ownership_runtime()?;
        let _permit = runtime.permit()?;
        let _boundary = self.mutation_guard()?;
        let generations = self
            .generations
            .read()
            .map_err(|_| poisoned("generation"))?;
        let state = self
            .ownership_state
            .read()
            .map_err(|_| poisoned("ownership"))?;
        let reader = Reader {
            state: &state,
            uuid: MemoryRead {
                state: &state.uuid,
                generations: &generations,
            },
        };
        f(&reader)
    }
    fn write_owned(&self, request: Request) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned_hooks(request, || Ok(()), || {}, || Ok(()))
    }
    fn write_owned_hooks(
        &self,
        request: Request,
        before: impl FnOnce() -> OwnershipResult<()>,
        after_row: impl FnMut(),
        after: impl FnOnce() -> OwnershipResult<()>,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        let runtime = self.ownership_runtime()?;
        request.admission(runtime.config.capacity().max_request_bytes())?;
        let _permit = runtime.permit()?;
        let _boundary = self.mutation_guard()?;
        let generations = self
            .generations
            .read()
            .map_err(|_| poisoned("generation"))?;
        let mut state = self
            .ownership_state
            .write()
            .map_err(|_| poisoned("ownership"))?;
        let reader = Reader {
            state: &state,
            uuid: MemoryRead {
                state: &state.uuid,
                generations: &generations,
            },
        };
        match engine::prepare(
            &reader,
            &self.entity_incarnation,
            runtime.config.capacity(),
            request,
        )? {
            engine::Prepared::Replay(r) => Ok(engine::outcome(
                *r,
                &self.entity_incarnation,
                OwnershipCommitDisposition::Replayed,
            )),
            engine::Prepared::Commit(plan) => {
                before()?;
                let receipt = plan.receipt.clone();
                state.apply(*plan, after_row);
                after().map_err(|e| OwnershipError::ExecutionUncertain {
                    query: receipt.query(),
                    cause: e.to_string(),
                })?;
                Ok(engine::outcome(
                    receipt,
                    &self.entity_incarnation,
                    OwnershipCommitDisposition::Committed,
                ))
            }
        }
    }
    pub(super) fn owned_uuid_page(
        &self,
        chunk: ChunkKey,
        expected: GenerationIdentity,
        cursor: Option<&PersistentEntityCursor>,
        limit: std::num::NonZeroU16,
    ) -> OwnershipResult<PersistentEntityPage> {
        self.read_owned(|r| {
            Ok(crate::persistent_entity::engine::page(
                &engine::EntityAdapter(r),
                &self.entity_incarnation,
                chunk,
                expected,
                cursor,
                limit,
            )?)
        })
    }
}
#[async_trait]
impl PlayerEntityOwnershipStore for InMemoryStore {
    async fn load_owned_player(
        &self,
        key: ScopedPlayerKey,
        expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerSnapshot> {
        self.read_owned(|r| engine::load_player(r, &self.entity_incarnation, key, expected))
    }
    async fn load_owned_entity(
        &self,
        key: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedEntitySnapshot> {
        self.read_owned(|r| engine::load_entity(r, &self.entity_incarnation, key, expected))
    }
    async fn load_owned_pair(
        &self,
        player: ScopedPlayerKey,
        entity: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerEntitySnapshot> {
        self.read_owned(|r| {
            engine::load_pair(r, &self.entity_incarnation, player, entity, expected)
        })
    }
    async fn commit_player_entity_transfer(
        &self,
        request: PlayerEntityTransferRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned(request.0)
    }
    async fn compare_exchange_owned_player(
        &self,
        request: PlayerOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned(request.0)
    }
    async fn compare_exchange_owned_entity(
        &self,
        request: EntityOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        self.write_owned(request.0)
    }
    async fn load_ownership_receipt(
        &self,
        query: OwnershipReceiptQuery,
    ) -> OwnershipResult<Option<OwnershipReceipt>> {
        self.read_owned(|r| engine::lookup_receipt(r, query))
    }
}

#[cfg(test)]
mod tests {
    use ferrumc_math::ChunkPos;
    use std::sync::{mpsc::sync_channel, Barrier};

    use super::*;
    use crate::ownership::tests::{config, e, entity, id, identity, p, player, scope};
    use crate::{
        EntityOwnershipProposal, OwnershipResource, OwnershipStamp, PlayerOwnershipProposal,
        WorldStore,
    };
    #[tokio::test]
    async fn busy_admission_is_shared_by_clones_during_actual_memory_application() {
        let mut store = setup().await;
        let request = request(&store, 2, 2).await;
        let query = request.query();
        store.ownership_runtime = Some(Arc::new(Runtime::new(
            crate::ownership::tests::config_inflight(1),
        )));
        let store = Arc::new(store);
        let writer_store = store.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        // A single bounded release signal holds the real synchronous producer.
        let (release_tx, release_rx) = sync_channel(1);
        let writer = tokio::task::spawn_blocking(move || {
            writer_store.write_owned_hooks(
                request,
                || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                },
                || {},
                || Ok(()),
            )
        });
        entered_rx.await.unwrap();
        assert!(matches!(
            store.load_owned_pair(player(), entity(), identity()).await,
            Err(OwnershipError::Busy)
        ));
        assert!(matches!(
            store.load_ownership_receipt(query).await,
            Err(OwnershipError::Busy)
        ));
        release_tx.send(()).unwrap();
        let outcome = writer.await.unwrap().unwrap();
        assert_eq!(
            store.load_ownership_receipt(query).await.unwrap().as_ref(),
            Some(outcome.receipt())
        );
    }
    type Maps = [BTreeMap<Vec<u8>, Vec<u8>>; 8];
    fn snapshot(store: &InMemoryStore) -> (Maps, u64, u64) {
        let s = store.ownership_state.read().unwrap();
        (
            [
                s.players.clone(),
                s.player_headers.clone(),
                s.entity_headers.clone(),
                s.receipts.clone(),
                s.uuid.entities.clone(),
                s.uuid.locators.clone(),
                s.uuid.indexes.clone(),
                s.uuid.epochs.clone(),
            ],
            s.receipt_count,
            s.revision_count,
        )
    }
    async fn setup() -> InMemoryStore {
        let store = InMemoryStore::with_player_entity_ownership(config()).unwrap();
        store
            .ensure_generation_identity(scope(), identity())
            .await
            .unwrap();
        let r = request(&store, 1, 1).await;
        store.write_owned(r).unwrap();
        store
    }
    async fn request(store: &InMemoryStore, n: u8, body: u8) -> Request {
        let pair = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        PlayerEntityTransferRequest::new(
            id(n),
            pair.player().expectation().clone(),
            pair.entity().expectation().clone(),
            PlayerOwnershipProposal::put(p(body)),
            EntityOwnershipProposal::put(e(0, body)),
        )
        .unwrap()
        .0
    }
    #[tokio::test]
    async fn real_preapply_refusal_and_postpublish_lost_reply_have_distinct_evidence() {
        let store = setup().await;
        let before = snapshot(&store);
        let r = request(&store, 2, 2).await;
        assert!(matches!(
            store.write_owned_hooks(
                r.clone(),
                || Err(OwnershipError::Backend("preapply".into())),
                || {},
                || Ok(())
            ),
            Err(OwnershipError::Backend(_))
        ));
        assert_eq!(snapshot(&store), before);
        assert!(store
            .load_ownership_receipt(r.query())
            .await
            .unwrap()
            .is_none());
        assert!(matches!(
            store.write_owned_hooks(
                r.clone(),
                || Ok(()),
                || {},
                || Err(OwnershipError::Backend("lost reply".into()))
            ),
            Err(OwnershipError::ExecutionUncertain { .. })
        ));
        assert_ne!(snapshot(&store), before);
        let receipt = store
            .load_ownership_receipt(r.query())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(store.write_owned(r).unwrap().receipt(), &receipt);
    }
    #[test]
    fn actual_mid_apply_reader_cannot_capture_a_partial_pair() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let store = Arc::new(runtime.block_on(setup()));
        let r = runtime.block_on(request(&store, 2, 2));
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let writer_store = store.clone();
        let writer_entered = entered.clone();
        let writer_release = release.clone();
        let writer = std::thread::spawn(move || {
            let mut first = true;
            writer_store
                .write_owned_hooks(
                    r,
                    || Ok(()),
                    || {
                        if first {
                            first = false;
                            writer_entered.wait();
                            writer_release.wait();
                        }
                    },
                    || Ok(()),
                )
                .unwrap()
        });
        entered.wait();
        assert!(
            store.ownership_state.try_read().is_err(),
            "first real player write is protected by the pair read boundary"
        );
        // One bounded test signal announces the reader attempt, not successful lock acquisition.
        let (started_tx, started_rx) = sync_channel(1);
        let reader_store = store.clone();
        let reader = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            reader_store
                .read_owned(|r| {
                    engine::load_pair(
                        r,
                        &reader_store.entity_incarnation,
                        player(),
                        entity(),
                        identity(),
                    )
                })
                .unwrap()
        });
        started_rx.recv().unwrap();
        release.wait();
        writer.join().unwrap();
        let pair = reader.join().unwrap();
        assert_eq!(pair.player().record(), Some(&p(2)));
        assert_eq!(pair.entity().record(), Some(&e(0, 2)));
        assert_eq!(pair.player().stamp().revision().unwrap().get(), 2);
        assert_eq!(pair.entity().stamp().revision().unwrap().get(), 2);
    }
    #[tokio::test]
    async fn retained_revision_and_epoch_max_allow_only_raw_no_op() {
        let store = setup().await;
        {
            let mut state = store.ownership_state.write().unwrap();
            let key = raw::player_key(player());
            let mut header =
                raw::player_header(state.player_headers.get(key.as_slice()).unwrap()).unwrap();
            header.stamp = OwnershipStamp::checked(1, u64::MAX).unwrap();
            state
                .player_headers
                .insert(key.to_vec(), raw::encode_player_header(header).to_vec());
            let mut receipt = raw::receipt(
                &id(1).to_bytes(),
                state.receipts.get(id(1).to_bytes().as_slice()).unwrap(),
            )
            .unwrap();
            receipt.before_player = OwnershipStamp::checked(1, u64::MAX - 1).unwrap();
            receipt.after_player = header.stamp;
            receipt.epochs[0].before = u64::MAX - 1;
            receipt.epochs[0].after = u64::MAX;
            state.receipts.insert(
                id(1).to_bytes().to_vec(),
                raw::encode_receipt(&receipt).to_vec(),
            );
            state.uuid.epochs.insert(
                crate::codec::chunk_key_bytes(ChunkKey::new(
                    scope().world(),
                    scope().dimension(),
                    ChunkPos::new(0, -2),
                ))
                .to_vec(),
                u64::MAX.to_be_bytes().to_vec(),
            );
        }
        let no_op = request(&store, 2, 1).await;
        store.write_owned(no_op.clone()).unwrap();
        let before = snapshot(&store);
        let changed = request(&store, 3, 2).await;
        assert!(matches!(
            store.write_owned(changed),
            Err(OwnershipError::RevisionExhausted {
                resource: OwnershipResource::Player
            })
        ));
        assert_eq!(snapshot(&store), before);
        let pair = store
            .load_owned_pair(player(), entity(), identity())
            .await
            .unwrap();
        let entity_only = EntityOwnershipCasRequest::new(
            id(4),
            pair.entity().expectation().clone(),
            EntityOwnershipProposal::put(e(1, 2)),
        )
        .unwrap();
        assert!(matches!(
            store.write_owned(entity_only.0),
            Err(OwnershipError::Record(
                StorageError::PersistentEntityEpochExhausted { .. }
            ))
        ));
        assert_eq!(snapshot(&store), before);
        assert_eq!(
            store.write_owned(no_op).unwrap().disposition(),
            OwnershipCommitDisposition::Replayed
        );
    }
    #[tokio::test]
    async fn encountered_owner_and_receipt_corruption_never_overwrites_or_repairs() {
        for case in 0..7 {
            let store = setup().await;
            let r = request(&store, 2, 2).await;
            {
                let mut s = store.ownership_state.write().unwrap();
                match case {
                    0 => {
                        s.player_headers
                            .remove(raw::player_key(player()).as_slice());
                    }
                    1 => {
                        s.entity_headers
                            .remove(raw::entity_key(entity()).as_slice());
                    }
                    2 => {
                        s.receipts.remove(id(1).to_bytes().as_slice());
                    }
                    3 => {
                        s.player_headers
                            .get_mut(raw::player_key(player()).as_slice())
                            .unwrap()[6] = 2;
                    }
                    4 => {
                        s.entity_headers
                            .get_mut(raw::entity_key(entity()).as_slice())
                            .unwrap()[18] = 0;
                    }
                    5 => {
                        s.receipts.get_mut(id(1).to_bytes().as_slice()).unwrap()[241] = 1;
                    }
                    6 => {
                        s.players
                            .get_mut(raw::player_key(player()).as_slice())
                            .unwrap()[4] = 255;
                    }
                    _ => unreachable!(),
                }
            }
            let before = snapshot(&store);
            assert!(store.write_owned(r).is_err(), "case{case}");
            assert_eq!(snapshot(&store), before);
            assert!(
                store
                    .load_owned_pair(player(), entity(), identity())
                    .await
                    .is_err(),
                "case{case}"
            );
            assert_eq!(snapshot(&store), before);
        }
    }
}
