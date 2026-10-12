//! Format7 block images publish in the same state lock as Player/Entity rows.
use super::ownership::{Reader, State as OwnedState};
use super::{poisoned, InMemoryStore};
use crate::block_durability::{
    engine::{self, Budget, Table},
    ownership_adapter, BlockCommitOutcome, BlockDurabilityConfig, BlockDurabilityError,
    BlockMutationRequest, BlockReceipt, BlockReceiptQuery, BlockResult, BlockSnapshotSpec,
    ChunkImageCasRequest, OwnedBlockSnapshot, PlayerBlockRequest, Request,
};
use crate::ownership::Runtime;
use crate::persistent_entity::engine::MemoryRead;
use crate::{
    OwnershipCommitDisposition, OwnershipError, OwnershipResult, PlayerBlockDurabilityStore,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) full: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) overlay: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) headers: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) journal: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(super) counts: engine::Counts,
}
impl State {
    pub(super) fn map(&self, t: Table) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        match t {
            Table::Full => &self.full,
            Table::Overlay => &self.overlay,
            Table::Header => &self.headers,
            Table::Journal => &self.journal,
        }
    }
    pub(super) fn is_empty(&self) -> bool {
        self.full.is_empty()
            && self.overlay.is_empty()
            && self.headers.is_empty()
            && self.journal.is_empty()
            && self.counts.chunks == 0
            && self.counts.receipt_bytes == 0
            && self.counts.journal_rows == 0
            && self.counts.journal_bytes == 0
            && self.counts.watermark.is_none()
    }
}
pub(super) fn admit_v1_bytes(current: u64, c: BlockDurabilityConfig) -> OwnershipResult<()> {
    if current
        .checked_add(272)
        .is_none_or(|n| n > c.capacity.receipt_bytes)
    {
        return Err(ownership_adapter(BlockDurabilityError::HistoryCapacity {
            resource: "receipt bytes",
            max: c.capacity.receipt_bytes,
        }));
    }
    Ok(())
}
fn apply(state: &mut OwnedState, plan: engine::Plan, after: &mut impl FnMut()) {
    if let Some(p) = plan.player {
        match p.record {
            Some(b) => {
                state.players.insert(p.key.to_vec(), b);
            }
            None => {
                state.players.remove(p.key.as_slice());
            }
        }
        after();
        state.player_headers.insert(
            p.key.to_vec(),
            crate::ownership::raw::encode_player_header(p.header).to_vec(),
        );
        after();
    }
    for c in plan.chunks {
        let key = crate::codec::chunk_key_bytes(c.key).to_vec();
        for (map, value) in [
            (&mut state.blocks.full, c.full),
            (&mut state.blocks.overlay, c.overlay),
        ] {
            if let Some(value) = value {
                match value {
                    Some(b) => {
                        map.insert(key.clone(), b);
                    }
                    None => {
                        map.remove(&key);
                    }
                }
                after();
            }
        }
        state.blocks.headers.insert(key, c.header.to_vec());
        after();
    }
    for (id, b) in plan.journal {
        state.blocks.journal.insert(id.to_be_bytes().to_vec(), b);
        after();
    }
    state.receipts.insert(
        plan.receipt.query.id.to_bytes().to_vec(),
        plan.receipt.bytes.to_vec(),
    );
    after();
    state.receipt_count = plan.receipts;
    after();
    state.revision_count = plan.revisions;
    after();
    state.blocks.counts = plan.counts;
    after();
}
impl InMemoryStore {
    /// Creates a fresh Format7 owner with explicit capacities and bounded shared admission.
    pub fn with_player_block_durability(config: BlockDurabilityConfig) -> BlockResult<Self> {
        Ok(Self {
            generation_mode: crate::generation::GenerationMode::BlockDurability,
            ownership_runtime: Some(Arc::new(Runtime::new(config.ownership))),
            block_config: Some(config),
            ..Self::default()
        })
    }
    fn block_settings(&self) -> BlockResult<BlockDurabilityConfig> {
        self.block_config
            .filter(|_| self.generation_mode == crate::generation::GenerationMode::BlockDurability)
            .ok_or(BlockDurabilityError::Unsupported)
    }
    async fn block_read<T: Send + 'static>(
        &self,
        f: impl FnOnce(
                &engine::Meter<'_, Reader<'_>>,
                &Arc<()>,
                BlockDurabilityConfig,
                &mut Budget,
            ) -> BlockResult<T>
            + Send
            + 'static,
    ) -> BlockResult<T> {
        let config = self.block_settings()?;
        let permit = self.ownership_runtime()?.permit().map_err(|e| match e {
            OwnershipError::Busy => BlockDurabilityError::Busy,
            other => other.into(),
        })?;
        let store = self.share();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            store.block_read_sync(config, f)
        })
        .await
        .map_err(|e| BlockDurabilityError::Backend(e.to_string()))?
    }
    fn block_read_sync<T>(
        &self,
        config: BlockDurabilityConfig,
        f: impl FnOnce(
            &engine::Meter<'_, Reader<'_>>,
            &Arc<()>,
            BlockDurabilityConfig,
            &mut Budget,
        ) -> BlockResult<T>,
    ) -> BlockResult<T> {
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
            config: Some(config),
            budget: Some(Budget::runtime(config)),
        };
        let mut budget = reader
            .budget
            .clone()
            .ok_or(BlockDurabilityError::Unsupported)?;
        let meter = engine::Meter {
            reader: &reader,
            budget: budget.clone(),
        };
        f(&meter, &self.entity_incarnation, config, &mut budget)
    }
    pub(super) async fn block_load_full(
        &self,
        key: crate::ChunkKey,
    ) -> BlockResult<Option<crate::ChunkRecord>> {
        self.block_read(move |r, _, _, b| {
            let identity = crate::persistent_entity::engine::EntityRead::identity(
                r,
                crate::GenerationKey::new(key.world(), key.dimension()),
            )?
            .ok_or(crate::StorageError::MissingGenerationIdentity {
                key: Some(crate::GenerationKey::new(key.world(), key.dimension())),
            })?;
            let (_, p) = engine::chunk_state(r, key, identity, b)?;
            p.full
                .map(|bytes| {
                    b.cells(98_304)?;
                    b.bytes(bytes.len() as u64)?;
                    crate::codec::decode_chunk_record(&bytes).map_err(Into::into)
                })
                .transpose()
        })
        .await
    }
    pub(super) async fn block_load_overlay(
        &self,
        key: crate::ChunkKey,
    ) -> BlockResult<Option<crate::ChunkOverlayRecord>> {
        self.block_read(move |r, _, _, b| {
            let identity = crate::persistent_entity::engine::EntityRead::identity(
                r,
                crate::GenerationKey::new(key.world(), key.dimension()),
            )?
            .ok_or(crate::StorageError::MissingGenerationIdentity {
                key: Some(crate::GenerationKey::new(key.world(), key.dimension())),
            })?;
            let (_, p) = engine::chunk_state(r, key, identity, b)?;
            p.overlay
                .map(|bytes| {
                    b.cells(98_304)?;
                    b.bytes(bytes.len() as u64)?;
                    crate::codec::decode_chunk_overlay_record(&bytes).map_err(Into::into)
                })
                .transpose()
        })
        .await
    }
    pub(super) async fn block_write(&self, request: Request) -> BlockResult<BlockCommitOutcome> {
        self.block_write_hooks(request, || Ok(()), || {}, || Ok(()))
            .await
    }
    pub(super) async fn block_write_hooks(
        &self,
        request: Request,
        before: impl FnOnce() -> BlockResult<()> + Send + 'static,
        mut after_row: impl FnMut() + Send + 'static,
        after: impl FnOnce() -> BlockResult<()> + Send + 'static,
    ) -> BlockResult<BlockCommitOutcome> {
        let config = self.block_settings()?;
        if request.q.len() > config.capacity.request {
            return Err(BlockDurabilityError::ByteCapacity {
                resource: "request",
                len: request.q.len() as u64,
                max: config.capacity.request as u64,
            });
        }
        let permit = self.ownership_runtime()?.permit().map_err(|e| {
            if e == OwnershipError::Busy {
                BlockDurabilityError::Busy
            } else {
                e.into()
            }
        })?;
        let store = self.share();
        let query = request.query;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _boundary = store.mutation_guard()?;
            let generations = store
                .generations
                .read()
                .map_err(|_| poisoned("generation"))?;
            let mut state = store
                .ownership_state
                .write()
                .map_err(|_| poisoned("ownership"))?;
            let reader = Reader {
                state: &state,
                uuid: MemoryRead {
                    state: &state.uuid,
                    generations: &generations,
                },
                config: Some(config),
                budget: Some(Budget::runtime(config)),
            };
            let mut budget = reader
                .budget
                .clone()
                .ok_or(BlockDurabilityError::Unsupported)?;
            let meter = engine::Meter {
                reader: &reader,
                budget: budget.clone(),
            };
            match engine::prepare(
                &meter,
                &store.entity_incarnation,
                &request,
                config,
                &mut budget,
            )? {
                engine::Prepared::Replay(receipt) => Ok(BlockCommitOutcome {
                    receipt,
                    disposition: OwnershipCommitDisposition::Replayed,
                }),
                engine::Prepared::Commit(plan) => {
                    before()?;
                    let receipt = plan.receipt.clone();
                    apply(&mut state, plan, &mut after_row);
                    after().map_err(|e| BlockDurabilityError::ExecutionUncertain {
                        query,
                        cause: e.to_string(),
                    })?;
                    Ok(BlockCommitOutcome {
                        receipt,
                        disposition: OwnershipCommitDisposition::Committed,
                    })
                }
            }
        })
        .await
        .map_err(|e| BlockDurabilityError::ExecutionUncertain {
            query,
            cause: e.to_string(),
        })?
    }
}
#[async_trait]
impl PlayerBlockDurabilityStore for InMemoryStore {
    async fn load_owned_blocks(&self, spec: BlockSnapshotSpec) -> BlockResult<OwnedBlockSnapshot> {
        self.block_read(move |r, o, c, b| engine::snapshot(r, o, spec, c, b))
            .await
    }
    async fn commit_player_blocks(
        &self,
        request: PlayerBlockRequest,
    ) -> BlockResult<BlockCommitOutcome> {
        self.block_write(request.0).await
    }
    async fn commit_blocks(
        &self,
        request: BlockMutationRequest,
    ) -> BlockResult<BlockCommitOutcome> {
        self.block_write(request.0).await
    }
    async fn compare_exchange_chunk_images(
        &self,
        request: ChunkImageCasRequest,
    ) -> BlockResult<BlockCommitOutcome> {
        self.block_write(request.0).await
    }
    async fn load_block_receipt(
        &self,
        query: BlockReceiptQuery,
    ) -> BlockResult<Option<BlockReceipt>> {
        self.block_read(move |r, _, _, b| engine::lookup(r, query, b))
            .await
    }
}

#[cfg(test)]
mod tests;
