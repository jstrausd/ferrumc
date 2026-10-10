//! Checked handoffs keep dirty ownership until a complete batch is representable.

use std::num::{NonZeroU8, NonZeroUsize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use ferrumc_core::{DimensionId, Result as CoreResult, ServerError, WorldId};
use ferrumc_items::{ComponentPatch, ComponentValue, ItemId, ItemStack, OpaqueComponent};
use ferrumc_math::BlockPos;
use ferrumc_storage::{BlockMutationLogRecord, EntityKey, EntityRecord, InMemoryStore};
use ferrumc_world::{BlockEntity, BlockStateId, ChestInventory};

use super::*;

fn map_with_dirty_chunks(count: usize) -> LoadedChunkMap {
    let mut map = LoadedChunkMap::new(WorldId::new(0), DimensionId::new(0));
    for x in 0..count {
        let pos = ChunkPos::new(i32::try_from(x).expect("fixture domain"), 0);
        let mut chunk = Chunk::new(pos);
        let block = pos.origin_block(0);
        chunk.set_block(block, BlockStateId::new(1)).unwrap();
        chunk.mark_persist_dirty(block);
        map.resident.insert(
            pos,
            ResidentChunk {
                chunk,
                tickets: BTreeMap::from([(ChunkTicket::of(TicketReason::Spawn), 1)]),
            },
        );
    }
    map
}

fn component_stack() -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("diamond").unwrap(),
        7,
        ComponentPatch::new(
            vec![ComponentValue::Opaque(
                OpaqueComponent::new(6, vec![10, 0]).unwrap(),
            )],
            vec![ferrumc_items::ComponentTypeId::new(51)],
        ),
    )
    .unwrap()
}

fn seed_chest(chunk: &mut Chunk, stack: ItemStack) -> BlockPos {
    let pos = chunk.pos().origin_block(0);
    let chest_state = ferrumc_registry::block_state::block_default_state("chest").unwrap();
    chunk
        .set_block(pos, BlockStateId::new(chest_state))
        .unwrap();
    let mut chest = ChestInventory::new();
    *chest.slot_mut(0).unwrap() = stack;
    chunk
        .set_block_entity(pos, BlockEntity::Chest(chest))
        .unwrap();
    chunk.mark_persist_dirty(pos);
    pos
}

fn invalid_count_stack() -> ItemStack {
    ItemStack::new(
        ItemId::from_name("diamond_sword").unwrap(),
        NonZeroU8::new(2).unwrap(),
        ComponentPatch::empty(),
    )
}

#[test]
fn checked_capture_late_failure_preserves_selected_chunks_masks_and_fairness() {
    let mut map = map_with_dirty_chunks(3);
    let first = ChunkPos::new(0, 0);
    let second = ChunkPos::new(1, 0);
    let third = ChunkPos::new(2, 0);
    let one = NonZeroUsize::new(1).unwrap();
    assert_eq!(
        map.try_take_persist_dirty_bounded(1, one).unwrap()[0]
            .0
            .pos(),
        first
    );
    assert_eq!(map.persist_scan_after, Some(first));
    map.get_mut(first)
        .unwrap()
        .mark_persist_dirty(first.origin_block(0));
    seed_chest(map.get_mut(second).unwrap(), component_stack());
    seed_chest(map.get_mut(third).unwrap(), invalid_count_stack());
    let before: Vec<_> = map.resident.values().map(|r| r.chunk.clone()).collect();
    let continuation = map.persist_scan_after;

    assert!(matches!(
        map.try_take_persist_dirty_bounded(2, NonZeroUsize::new(2).unwrap()),
        Err(StorageError::BlockEntityCodec { .. })
    ));
    assert_eq!(map.persist_scan_after, continuation);
    assert_eq!(
        map.resident
            .values()
            .map(|r| r.chunk.clone())
            .collect::<Vec<_>>(),
        before
    );
    assert!(map
        .resident
        .values()
        .all(|r| r.chunk.persist_dirty_sections().any()));

    seed_chest(map.get_mut(third).unwrap(), component_stack());
    let batch = map
        .try_take_persist_dirty_bounded(3, NonZeroUsize::new(2).unwrap())
        .unwrap();
    assert_eq!(
        batch.iter().map(|(key, _)| key.pos()).collect::<Vec<_>>(),
        [second, third]
    );
    assert!(map.get(first).unwrap().persist_dirty_sections().any());
    assert!(!map.get(second).unwrap().persist_dirty_sections().any());
    assert!(!map.get(third).unwrap().persist_dirty_sections().any());
    assert!(map.get(second).unwrap().persist_edited_sections().any());
    assert_eq!(map.persist_scan_after, Some(third));
    for (key, record) in batch {
        let mut restored = Chunk::new(key.pos());
        record.try_apply_to_chunk(&mut restored).unwrap();
        let Some(BlockEntity::Chest(chest)) = restored.block_entity(key.pos().origin_block(0))
        else {
            panic!("captured chest missing");
        };
        assert_eq!(chest.slot(0), Some(&component_stack()));
        assert_eq!(record.updated_at_tick(), 3);
    }
    assert_eq!(
        map.try_take_persist_dirty_bounded(4, one).unwrap()[0]
            .0
            .pos(),
        first
    );
    assert_eq!(map.persist_scan_after, None);
    assert!(!map.has_persist_dirty());
}

#[test]
fn checked_capture_clamps_requested_capacity_and_keeps_the_tail_dirty() {
    let mut map = map_with_dirty_chunks(MAX_SAVE_BATCH + 1);
    let batch = map
        .try_take_persist_dirty_bounded(1, NonZeroUsize::new(usize::MAX).unwrap())
        .unwrap();
    assert_eq!(batch.len(), MAX_SAVE_BATCH);
    let tail = ChunkPos::new(i32::try_from(MAX_SAVE_BATCH).unwrap(), 0);
    assert!(map.get(tail).unwrap().persist_dirty_sections().any());
    // Re-dirty the lowest key; the older deferred tail must still be selected first.
    map.get_mut(ChunkPos::ORIGIN)
        .unwrap()
        .mark_persist_dirty(BlockPos::new(0, 0, 0));
    let next = map
        .try_take_persist_dirty_bounded(2, NonZeroUsize::new(1).unwrap())
        .unwrap();
    assert_eq!(next[0].0.pos(), tail);
    assert!(map
        .get(ChunkPos::ORIGIN)
        .unwrap()
        .persist_dirty_sections()
        .any());
}

/// A corrupt durable fixture bypasses backend admission validation deliberately.
/// Writes are observable and refused so a failed load cannot repair/default data.
struct CorruptStore {
    base: ChunkRecord,
    overlay: ChunkOverlayRecord,
    writes: AtomicUsize,
    reads: AtomicUsize,
}

impl CorruptStore {
    fn refuse_write(&self) -> ServerError {
        self.writes.fetch_add(1, Ordering::Relaxed);
        ServerError::unsupported("fixture refuses writes")
    }
}

#[async_trait]
impl WorldStore for CorruptStore {
    async fn load_chunk(&self, _key: ChunkKey) -> CoreResult<Option<ChunkRecord>> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok(Some(self.base.clone()))
    }
    async fn load_chunk_overlay(&self, _key: ChunkKey) -> CoreResult<Option<ChunkOverlayRecord>> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok(Some(self.overlay.clone()))
    }
    async fn save_chunk(&self, _key: ChunkKey, _record: ChunkRecord) -> CoreResult<()> {
        Err(self.refuse_write())
    }
    async fn save_chunks(&self, _chunks: Vec<(ChunkKey, ChunkRecord)>) -> CoreResult<()> {
        Err(self.refuse_write())
    }
    async fn delete_chunk(&self, _key: ChunkKey) -> CoreResult<bool> {
        Err(self.refuse_write())
    }
    async fn save_chunk_overlays(
        &self,
        _overlays: Vec<(ChunkKey, ChunkOverlayRecord)>,
    ) -> CoreResult<()> {
        Err(self.refuse_write())
    }
    async fn append_block_mutations(
        &self,
        _mutations: Vec<BlockMutationLogRecord>,
    ) -> CoreResult<()> {
        Err(self.refuse_write())
    }
    async fn load_entity(&self, _key: EntityKey) -> CoreResult<Option<EntityRecord>> {
        Ok(None)
    }
    async fn save_entity(&self, _key: EntityKey, _record: EntityRecord) -> CoreResult<()> {
        Err(self.refuse_write())
    }
    async fn save_entities(&self, _entities: Vec<(EntityKey, EntityRecord)>) -> CoreResult<()> {
        Err(self.refuse_write())
    }
    async fn delete_entity(&self, _key: EntityKey) -> CoreResult<bool> {
        Err(self.refuse_write())
    }
}

#[tokio::test]
async fn corrupt_overlay_load_admits_no_ticket_or_defaulted_chest_and_never_writes() {
    let pos = ChunkPos::ORIGIN;
    let mut edited = Chunk::new(pos);
    seed_chest(&mut edited, invalid_count_stack());
    // Legacy capture retains the invalid count on disk; strict apply must refuse
    // it instead of the old skip-corrupt-entity behavior admitting an empty chest.
    let overlay = ChunkOverlayRecord::from_chunk(OVERLAY_SCHEMA_VERSION, pos, &edited, 7);
    let store = CorruptStore {
        base: ChunkRecord::new(CHUNK_SCHEMA_VERSION, Chunk::new(pos)),
        overlay: overlay.clone(),
        writes: AtomicUsize::new(0),
        reads: AtomicUsize::new(0),
    };
    let mut map = LoadedChunkMap::new(WorldId::new(0), DimensionId::new(0));
    let error = map
        .acquire(
            &store,
            &FlatWorldGenerator::new(),
            pos,
            ChunkTicket::of(TicketReason::Spawn),
        )
        .await
        .expect_err("corrupt chest must reject the whole load");
    assert!(
        matches!(error, SimError::ChunkLoad { pos: failed, source: ServerError::Internal { .. } } if failed == pos)
    );
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(map.ticket_count(pos), 0);
    assert!(!map.has_persist_dirty());
    assert_eq!(store.writes.load(Ordering::Relaxed), 0);
    assert_eq!(store.overlay, overlay);
}

fn fixture_store(base: Chunk, overlay: ChunkOverlayRecord) -> CorruptStore {
    CorruptStore {
        base: ChunkRecord::new(CHUNK_SCHEMA_VERSION, base),
        overlay,
        writes: AtomicUsize::new(0),
        reads: AtomicUsize::new(0),
    }
}

async fn bound_map_with_dirty_chunks(
    count: usize,
) -> (
    LoadedChunkMap,
    Arc<dyn WorldStore>,
    VerifiedGenerationBinding,
) {
    let mut map = map_with_dirty_chunks(count);
    let binding = GenerationBinding::new(
        GenerationKey::new(map.world(), map.dimension()),
        FlatWorldGenerator::new().identity(),
    );
    map.generation = Some(binding);
    let store: Arc<dyn WorldStore> = Arc::new(InMemoryStore::with_generation_metadata());
    store
        .ensure_generation_identity(binding.key(), binding.identity())
        .await
        .unwrap();
    let verified = binding.verify_store(&store).await.unwrap();
    (map, store, verified)
}

#[tokio::test]
async fn bound_capture_metadata_and_late_codec_failures_preserve_every_fact_and_cursor() {
    let (mut map, store, verified) = bound_map_with_dirty_chunks(3).await;
    let first = ChunkPos::new(0, 0);
    let second = ChunkPos::new(1, 0);
    let third = ChunkPos::new(2, 0);
    map.try_take_persist_dirty_verified(&store, &verified, 1, NonZeroUsize::new(1).unwrap())
        .unwrap();
    assert_eq!(map.persist_scan_after, Some(first));
    map.get_mut(first)
        .unwrap()
        .mark_persist_dirty(first.origin_block(0));
    seed_chest(map.get_mut(second).unwrap(), component_stack());
    seed_chest(map.get_mut(third).unwrap(), invalid_count_stack());
    let before: Vec<_> = map.resident.values().map(|r| r.chunk.clone()).collect();
    let tickets: Vec<_> = map
        .loaded_positions()
        .map(|pos| map.canonical_ticket_state_record(pos))
        .collect();
    let continuation = map.persist_scan_after;
    let capacity = NonZeroUsize::new(2).unwrap();
    let binding = map.generation_binding().unwrap();
    let wrong = InMemoryStore::with_generation_metadata();
    wrong
        .ensure_generation_identity(binding.key(), identity_for_seed(77))
        .await
        .unwrap();
    let empty = InMemoryStore::with_generation_metadata();
    let legacy = InMemoryStore::new();
    for target in [&wrong as &dyn WorldStore, &empty, &legacy] {
        assert!(matches!(
            map.try_take_persist_dirty_bound(target, 2, capacity).await,
            Err(SimError::GenerationIdentityMismatch { .. }
                | SimError::GenerationIdentityMissing { .. }
                | SimError::GenerationStore { .. })
        ));
        assert_eq!(map.persist_scan_after, continuation);
        assert_eq!(
            map.resident
                .values()
                .map(|r| r.chunk.clone())
                .collect::<Vec<_>>(),
            before
        );
    }
    for async_capture in [false, true] {
        let result = if async_capture {
            map.try_take_persist_dirty_bound(store.as_ref(), 3, capacity)
                .await
        } else {
            map.try_take_persist_dirty_verified(&store, &verified, 3, capacity)
        };
        assert!(matches!(
            result,
            Err(SimError::PersistCapture {
                source: StorageError::BlockEntityCodec { .. }
            })
        ));
        assert_eq!(map.persist_scan_after, continuation);
        assert_eq!(
            map.resident
                .values()
                .map(|r| r.chunk.clone())
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            map.loaded_positions()
                .map(|pos| map.canonical_ticket_state_record(pos))
                .collect::<Vec<_>>(),
            tickets
        );
    }
    seed_chest(map.get_mut(third).unwrap(), component_stack());
    let batch = map
        .try_take_persist_dirty_verified(&store, &verified, 4, capacity)
        .unwrap();
    assert_eq!(
        batch.iter().map(|(key, _)| key.pos()).collect::<Vec<_>>(),
        [second, third]
    );
    assert_eq!(map.persist_scan_after, Some(third));
    assert!(map.get(first).unwrap().persist_dirty_sections().any());
    assert!(map.get(second).unwrap().persist_edited_sections().any());
    assert_eq!(
        map.try_take_persist_dirty_bound(store.as_ref(), 5, NonZeroUsize::new(1).unwrap())
            .await
            .unwrap()[0]
            .0
            .pos(),
        first
    );
    assert!(!map.has_persist_dirty());
    assert_eq!(map.persist_scan_after, None);
}

fn identity_for_seed(seed: i64) -> ferrumc_world::GenerationIdentity {
    ferrumc_world::GenerationIdentity::flat_overworld(ferrumc_world::WorldSeed::new(seed))
}

#[tokio::test]
async fn metadata_maps_refuse_all_unverified_legacy_drains_without_any_dirtyclear() {
    let (mut map, store, verified) = bound_map_with_dirty_chunks(2).await;
    map.try_take_persist_dirty_verified(&store, &verified, 1, NonZeroUsize::new(1).unwrap())
        .unwrap();
    let before: Vec<_> = map.resident.values().map(|r| r.chunk.clone()).collect();
    let continuation = map.persist_scan_after;
    assert_eq!(map.take_dirty(), Vec::new());
    assert_eq!(map.take_persist_dirty(2), Vec::new());
    assert_eq!(
        map.take_persist_dirty_bounded(2, NonZeroUsize::new(1).unwrap()),
        Vec::new()
    );
    assert_eq!(
        map.try_take_persist_dirty_bounded(2, NonZeroUsize::new(1).unwrap()),
        Err(StorageError::GenerationMetadataUnsupported)
    );
    assert_eq!(map.persist_scan_after, continuation);
    assert_eq!(
        map.resident
            .values()
            .map(|r| r.chunk.clone())
            .collect::<Vec<_>>(),
        before
    );
    assert!(map.has_persist_dirty());
}

#[tokio::test]
async fn verified_capture_clamps_before_allocation_and_keeps_the_deferred_tail() {
    let (mut map, store, verified) = bound_map_with_dirty_chunks(MAX_SAVE_BATCH + 1).await;
    let batch = map
        .try_take_persist_dirty_verified(
            &store,
            &verified,
            1,
            NonZeroUsize::new(usize::MAX).unwrap(),
        )
        .unwrap();
    assert_eq!(batch.len(), MAX_SAVE_BATCH);
    assert_eq!(batch.last().unwrap().0.pos(), ChunkPos::new(4095, 0));
    assert_eq!(map.persist_scan_after, Some(ChunkPos::new(4095, 0)));
    assert!(map
        .get(ChunkPos::new(4096, 0))
        .unwrap()
        .persist_dirty_sections()
        .any());
    map.get_mut(ChunkPos::ORIGIN)
        .unwrap()
        .mark_persist_dirty(ChunkPos::ORIGIN.origin_block(0));
    let next = map
        .try_take_persist_dirty_verified(&store, &verified, 2, NonZeroUsize::new(1).unwrap())
        .unwrap();
    assert_eq!(next[0].0.pos(), ChunkPos::new(4096, 0));
    assert!(map
        .get(ChunkPos::ORIGIN)
        .unwrap()
        .persist_dirty_sections()
        .any());
}

#[tokio::test]
async fn custom_store_mismatched_base_or_overlay_column_never_becomes_resident() {
    let requested = ChunkPos::ORIGIN;
    let other = ChunkPos::new(1, 0);
    for mismatch_base in [true, false] {
        let base_pos = if mismatch_base { other } else { requested };
        let overlay_pos = if mismatch_base { requested } else { other };
        let overlay = ChunkOverlayRecord::try_from_chunk(
            OVERLAY_SCHEMA_VERSION,
            overlay_pos,
            &Chunk::new(overlay_pos),
            0,
        )
        .unwrap();
        let store = fixture_store(Chunk::new(base_pos), overlay);
        let mut map = LoadedChunkMap::new(WorldId::new(0), DimensionId::new(0));
        let error = map
            .acquire(
                &store,
                &FlatWorldGenerator::new(),
                requested,
                ChunkTicket::of(TicketReason::Spawn),
            )
            .await
            .expect_err("another column must reject admission");
        assert!(matches!(error, SimError::ChunkLoad { .. }));
        assert_eq!(map.loaded_count(), 0);
        assert_eq!(map.ticket_count(requested), 0);
        assert_eq!(store.writes.load(Ordering::Relaxed), 0);
    }
}

#[tokio::test]
async fn out_of_block_domain_keys_are_rejected_before_reading_or_generating() {
    let overlay = ChunkOverlayRecord::try_from_chunk(
        OVERLAY_SCHEMA_VERSION,
        ChunkPos::ORIGIN,
        &Chunk::new(ChunkPos::ORIGIN),
        0,
    )
    .unwrap();
    let store = fixture_store(Chunk::new(ChunkPos::ORIGIN), overlay);
    let mut map = LoadedChunkMap::new(WorldId::new(0), DimensionId::new(0));
    for pos in [
        ChunkPos::new(i32::MIN, 0),
        ChunkPos::new(i32::MAX, 0),
        ChunkPos::new(0, -134_217_729),
        ChunkPos::new(0, 134_217_728),
    ] {
        assert!(matches!(
            map.acquire(&store, &FlatWorldGenerator::new(), pos, ChunkTicket::of(TicketReason::Spawn)).await,
            Err(SimError::ChunkLoad { pos: failed, .. }) if failed == pos
        ));
    }
    assert_eq!(map.loaded_count(), 0);
    assert_eq!(store.reads.load(Ordering::Relaxed), 0);
    assert_eq!(store.writes.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn checked_component_capture_survives_store_and_strict_reload() {
    let store = InMemoryStore::new();
    let mut map = map_with_dirty_chunks(2);
    let expected = component_stack();
    let positions = [ChunkPos::ORIGIN, ChunkPos::new(1, 0)];
    for pos in positions {
        seed_chest(map.get_mut(pos).unwrap(), expected.clone());
    }
    while map.has_persist_dirty() {
        let batch = map
            .try_take_persist_dirty_bounded(4, NonZeroUsize::new(1).unwrap())
            .unwrap();
        store.save_chunk_overlays(batch).await.unwrap();
    }
    let mut reloaded = LoadedChunkMap::new(WorldId::new(0), DimensionId::new(0));
    for pos in positions {
        reloaded
            .acquire(
                &store,
                &FlatWorldGenerator::new(),
                pos,
                ChunkTicket::of(TicketReason::Spawn),
            )
            .await
            .unwrap();
        let chunk = reloaded.get(pos).unwrap();
        let Some(BlockEntity::Chest(chest)) = chunk.block_entity(pos.origin_block(0)) else {
            panic!("saved chest missing");
        };
        assert_eq!(chest.slot(0), Some(&expected));
        assert!(chunk.persist_edited_sections().any());
        assert!(!chunk.persist_dirty_sections().any());
    }
}
