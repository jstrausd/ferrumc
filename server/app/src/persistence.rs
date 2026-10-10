//! One opaque destination shared by bootstrap, capture and the storage writer.

use std::num::NonZeroUsize;
use std::sync::Arc;

use ferrumc_core::{Result as ServerResult, ServerError};
use ferrumc_sim::{GenerationBinding, SimShard, VerifiedGenerationBinding};
use ferrumc_storage::{ChunkKey, ChunkOverlayRecord, WorldStore};

/// Couples one declared baseline with its exact verified backend allocation.
#[derive(Clone)]
pub(crate) struct PersistenceTarget {
    store: Arc<dyn WorldStore>,
    verified: VerifiedGenerationBinding,
}

impl PersistenceTarget {
    /// Reads persisted metadata off-tick and authorizes this exact store.
    pub(crate) async fn verify(
        store: Arc<dyn WorldStore>,
        binding: GenerationBinding,
    ) -> anyhow::Result<Self> {
        let verified = binding.verify_store(&store).await?;
        Ok(Self { store, verified })
    }

    pub(crate) fn store(&self) -> &dyn WorldStore {
        self.store.as_ref()
    }

    pub(crate) fn binding(&self) -> GenerationBinding {
        self.verified.binding()
    }

    /// Checks writer admission afresh, including empty durability barriers.
    pub(crate) async fn validate_write(&self) -> ServerResult<()> {
        self.binding()
            .validate_store(self.store())
            .await
            .map_err(|error| {
                ServerError::invalid_state(format!(
                    "persistence generation binding rejected: {error}"
                ))
            })
    }

    /// Pure synchronous capture after the caller has reserved bounded capacity.
    pub(crate) fn capture(
        &self,
        shard: &mut SimShard,
        tick: u64,
        limit: NonZeroUsize,
    ) -> ServerResult<Vec<(ChunkKey, ChunkOverlayRecord)>> {
        shard
            .loaded_chunks_mut()
            .try_take_persist_dirty_verified(&self.store, &self.verified, tick, limit)
            .map_err(|error| {
                ServerError::invalid_state(format!(
                    "persistence generation capture rejected: {error}"
                ))
            })
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use ferrumc_core::{DimensionId, WorldId};
    use ferrumc_storage::{
        BlockMutationLogRecord, ChunkRecord, EntityKey, EntityRecord, GenerationKey,
        JournalAppendReceipt, JournalBatchId,
    };
    use ferrumc_testkit::FaultInjectingStore;
    use ferrumc_world::{GenerationIdentity, WorldSeed};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    pub(crate) fn binding() -> GenerationBinding {
        GenerationBinding::new(
            GenerationKey::new(WorldId::new(0), DimensionId::new(0)),
            GenerationIdentity::flat_overworld(WorldSeed::new(0)),
        )
    }

    /// Deliberate fault double: existing retry harness with inspectable metadata.
    pub(crate) struct MetadataFaultStore {
        pub(crate) inner: Arc<FaultInjectingStore>,
        pub(crate) reject: AtomicBool,
        pub(crate) reads: AtomicUsize,
    }

    macro_rules! delegate {
        ($name:ident, ($($arg:ident : $ty:ty),*), $out:ty) => {
            fn $name<'life0, 'async_trait>(&'life0 self, $($arg: $ty),*) -> Pin<Box<dyn Future<Output = ServerResult<$out>> + Send + 'async_trait>>
            where 'life0: 'async_trait, Self: 'async_trait {
                self.inner.$name($($arg),*)
            }
        };
    }

    impl WorldStore for MetadataFaultStore {
        fn load_generation_identity<'life0, 'async_trait>(
            &'life0 self,
            _key: GenerationKey,
        ) -> Pin<
            Box<
                dyn Future<Output = ServerResult<Option<GenerationIdentity>>> + Send + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                self.reads.fetch_add(1, Ordering::SeqCst);
                if self.reject.load(Ordering::SeqCst) {
                    Ok(Some(GenerationIdentity::flat_overworld(WorldSeed::new(1))))
                } else {
                    Ok(Some(binding().identity()))
                }
            })
        }
        delegate!(load_chunk, (key: ChunkKey), Option<ChunkRecord>);
        delegate!(save_chunk, (key: ChunkKey, record: ChunkRecord), ());
        delegate!(save_chunks, (chunks: Vec<(ChunkKey, ChunkRecord)>), ());
        delegate!(delete_chunk, (key: ChunkKey), bool);
        delegate!(load_chunk_overlay, (key: ChunkKey), Option<ChunkOverlayRecord>);
        delegate!(save_chunk_overlays, (overlays: Vec<(ChunkKey, ChunkOverlayRecord)>), ());
        delegate!(append_block_mutations, (mutations: Vec<BlockMutationLogRecord>), ());
        delegate!(append_block_mutation_batch, (batch_id: JournalBatchId, mutations: Vec<BlockMutationLogRecord>), JournalAppendReceipt);
        delegate!(load_entity, (key: EntityKey), Option<EntityRecord>);
        delegate!(save_entity, (key: EntityKey, record: EntityRecord), ());
        delegate!(save_entities, (entities: Vec<(EntityKey, EntityRecord)>), ());
        delegate!(delete_entity, (key: EntityKey), bool);
    }

    pub(crate) fn wrong_allocation(
        target: &PersistenceTarget,
        other: PersistenceTarget,
    ) -> PersistenceTarget {
        PersistenceTarget {
            store: other.store,
            verified: target.verified.clone(),
        }
    }

    pub(crate) async fn fault_target(
        inner: Arc<FaultInjectingStore>,
    ) -> (PersistenceTarget, Arc<MetadataFaultStore>) {
        let store = Arc::new(MetadataFaultStore {
            inner,
            reject: AtomicBool::new(false),
            reads: AtomicUsize::new(0),
        });
        let erased: Arc<dyn WorldStore> = store.clone();
        (
            PersistenceTarget::verify(erased, binding())
                .await
                .expect("fixture metadata verifies"),
            store,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_math::{BlockPos, ChunkPos, ShardPos};
    use ferrumc_sim::{ChunkTicket, TicketReason};
    use ferrumc_testkit::FaultInjectingStore;
    use ferrumc_world::{BlockStateId, FlatWorldGenerator};
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn opaque_capture_performs_no_store_read_and_rejects_other_allocation() {
        let (target, metadata) = fixtures::fault_target(Arc::new(FaultInjectingStore::new())).await;
        let mut shard = SimShard::with_generation(ShardPos::new(0, 0), fixtures::binding());
        let pos = ChunkPos::new(0, 0);
        shard
            .loaded_chunks_mut()
            .acquire(
                target.store(),
                &FlatWorldGenerator::new(),
                pos,
                ChunkTicket::of(TicketReason::Player),
            )
            .await
            .expect("bound chunk");
        let block = BlockPos::new(1, 65, 1);
        let chunk = shard.loaded_chunks_mut().get_mut(pos).expect("resident");
        chunk
            .set_block(block, BlockStateId::new(1))
            .expect("valid block");
        chunk.mark_persist_dirty(block);
        let (other, _) = fixtures::fault_target(Arc::new(FaultInjectingStore::new())).await;
        let forged = PersistenceTarget {
            store: other.store,
            verified: target.verified.clone(),
        };
        let before_reads = metadata.reads.load(Ordering::SeqCst);
        assert!(forged.capture(&mut shard, 0, NonZeroUsize::MIN).is_err());
        assert!(shard.loaded_chunks().has_persist_dirty());
        let batch = target
            .capture(&mut shard, 0, NonZeroUsize::MIN)
            .expect("same allocation accepted");
        assert_eq!(batch.len(), 1);
        assert_eq!(metadata.reads.load(Ordering::SeqCst), before_reads);
    }
}
