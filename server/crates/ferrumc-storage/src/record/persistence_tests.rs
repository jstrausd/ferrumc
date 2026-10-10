use ferrumc_core::{DimensionId, WorldId};
use ferrumc_items::{
    ComponentPatch, ComponentTypeId, ComponentValue, ItemId, ItemStack, OpaqueComponent,
};
use ferrumc_nbt::{NbtCompound, NbtTag};
use ferrumc_world::{BlockEntity, ChestInventory, Sign, SignKind};

use super::*;
use crate::{codec, ChunkKey, InMemoryStore, RedbStore, WorldStore};

fn key(pos: ChunkPos) -> ChunkKey {
    ChunkKey::new(WorldId::new(0), DimensionId::new(0), pos)
}

fn component_item() -> ItemStack {
    let mut name = NbtCompound::new();
    name.push("text", NbtTag::String("Named 🦀\0".to_owned()));
    let mut data = NbtCompound::new();
    data.push("same", NbtTag::Int(1));
    data.push("same", NbtTag::Int(2));
    data.push("float", NbtTag::Float(f32::from_bits(0x7fc0_1234)));
    data.push("zero", NbtTag::Double(-0.0));
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        13,
        ComponentPatch::new(
            vec![
                ComponentValue::MaxStackSize(1),
                ComponentValue::Damage(-7),
                ComponentValue::MaxDamage(i32::MAX),
                ComponentValue::Unbreakable,
                ComponentValue::CustomName(NbtTag::Compound(name)),
                ComponentValue::CustomData(NbtTag::Compound(data)),
                ComponentValue::Opaque(OpaqueComponent::new(6, vec![0, 0xff, 7]).unwrap()),
            ],
            vec![ComponentTypeId::new(95), ComponentTypeId::new(51)],
        ),
    )
    .unwrap()
}

fn component_chunk(pos: ChunkPos) -> Chunk {
    let mut chest = ChestInventory::new();
    *chest.slot_mut(7).unwrap() = component_item();
    let mut chunk = Chunk::new(pos);
    let at = pos.origin_block(64);
    chunk.set_block(at, BlockStateId::new(1)).unwrap();
    chunk.mark_persist_dirty(at);
    chunk
        .set_block_entity(at, BlockEntity::Chest(chest))
        .unwrap();
    chunk
}

fn entity_payload(entity: &BlockEntity) -> Vec<u8> {
    let mut out = Vec::new();
    try_encode_block_entity(entity, &mut out).unwrap();
    out
}

fn blank_payload() -> Vec<u8> {
    entity_payload(&BlockEntity::Sign(Sign::new(SignKind::Sign)))
}

#[test]
fn checked_capture_full_codec_and_apply_preserve_components_and_nan_bits() {
    let source = component_chunk(ChunkPos::ORIGIN);
    let overlay =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), source.pos(), &source, 17)
            .unwrap();
    assert!(source.persist_dirty_sections().any());
    let bytes = codec::encode_chunk_overlay_record(&overlay).unwrap();
    assert_eq!(overlay.encoded_len(), bytes.len());
    let loaded = codec::decode_chunk_overlay_record(&bytes).unwrap();
    let mut target = Chunk::new(source.pos());
    loaded.try_apply_to_chunk(&mut target).unwrap();
    let persisted_target =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), target.pos(), &target, 17)
            .unwrap();
    assert_eq!(persisted_target.block_entities, overlay.block_entities);
    let full =
        codec::encode_chunk_record(&ChunkRecord::try_new(SchemaVersion::new(3), source).unwrap())
            .unwrap();
    let loaded = codec::decode_chunk_record(&full).unwrap();
    assert_eq!(codec::encode_chunk_record(&loaded).unwrap(), full);
}

#[test]
fn checked_capture_rejects_late_bad_entity_without_clearing_dirty_masks() {
    let mut source = Chunk::new(ChunkPos::ORIGIN);
    source
        .set_block_entity(
            BlockPos::new(0, 64, 0),
            BlockEntity::Sign(Sign::new(SignKind::Sign)),
        )
        .unwrap();
    let late = BlockPos::new(15, 64, 15);
    let mut bad = Sign::new(SignKind::Sign);
    bad.set_face_lines(true, std::array::from_fn(|_| "x".repeat(1025)));
    source
        .set_block_entity(late, BlockEntity::Sign(bad))
        .unwrap();
    source.mark_persist_dirty(late);
    let before = source.clone();
    assert!(
        matches!(ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), source.pos(), &source, 1), Err(StorageError::BlockEntityCodec { pos, .. }) if pos == late)
    );
    assert_eq!(source, before);
    assert!(source.persist_dirty_sections().any());
    assert!(source.persist_edited_sections().any());
}

fn malformed_overlay(late: bool) -> ChunkOverlayRecord {
    let mut record = ChunkOverlayRecord::from_parts(
        SchemaVersion::new(3),
        ChunkPos::ORIGIN,
        1 << 8,
        vec![OverlaySection::new(8, vec![BlockStateId::new(1); SECTION_VOLUME]).unwrap()],
        0,
        vec![
            (BlockPos::new(0, 64, 0), blank_payload()),
            (BlockPos::new(15, 64, 15), blank_payload()),
        ],
    )
    .unwrap();
    record.block_entities[usize::from(late)].1 = vec![255];
    record
}

#[test]
fn strict_apply_rejects_before_and_late_entity_without_any_target_changes() {
    for late in [false, true] {
        let mut target = Chunk::new(ChunkPos::ORIGIN);
        let existing = BlockPos::new(1, 70, 1);
        target.set_block(existing, BlockStateId::new(7)).unwrap();
        target
            .set_block_entity(existing, BlockEntity::Sign(Sign::new(SignKind::Hanging)))
            .unwrap();
        target.mark_persist_dirty(existing);
        let before = target.clone();
        assert!(matches!(
            malformed_overlay(late).try_apply_to_chunk(&mut target),
            Err(StorageError::BlockEntityCodec { .. })
        ));
        assert_eq!(target, before);
        assert!(target.persist_dirty_sections().any());
    }
}

#[test]
fn duplicate_offcolumn_height_and_domain_errors_are_atomic() {
    let mut record = ChunkOverlayRecord::from_parts(
        SchemaVersion::new(3),
        ChunkPos::ORIGIN,
        0,
        Vec::new(),
        0,
        vec![(BlockPos::new(0, 64, 0), blank_payload())],
    )
    .unwrap();
    for bad_position in [
        BlockPos::new(16, 64, 0),
        BlockPos::new(0, 320, 0),
        BlockPos::new(0, -65, 0),
    ] {
        record.block_entities[0].0 = bad_position;
        let mut target = Chunk::new(ChunkPos::ORIGIN);
        let before = target.clone();
        assert!(matches!(
            record.try_apply_to_chunk(&mut target),
            Err(StorageError::BlockEntityOutsideChunk { .. })
        ));
        assert_eq!(target, before);
    }
    record.block_entities[0].0 = BlockPos::new(0, 64, 0);
    record.block_entities.push(record.block_entities[0].clone());
    let mut target = Chunk::new(ChunkPos::ORIGIN);
    let before = target.clone();
    assert!(matches!(
        record.try_apply_to_chunk(&mut target),
        Err(StorageError::DuplicateBlockEntity { .. })
    ));
    assert_eq!(target, before);
    let outside = ChunkPos::new(i32::MAX, 0);
    assert!(matches!(
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), outside, &Chunk::new(outside), 0),
        Err(StorageError::ChunkPositionOutsideDomain { .. })
    ));
}

#[test]
fn mismatched_capture_apply_and_constructor_reject_columns() {
    let source = Chunk::new(ChunkPos::ORIGIN);
    assert!(matches!(
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), ChunkPos::new(1, 0), &source, 0),
        Err(StorageError::ChunkPositionMismatch { .. })
    ));
    let overlay =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), source.pos(), &source, 0)
            .unwrap();
    let mut target = Chunk::new(ChunkPos::new(1, 0));
    let before = target.clone();
    assert!(matches!(
        overlay.try_apply_to_chunk(&mut target),
        Err(StorageError::ChunkPositionMismatch { .. })
    ));
    assert_eq!(target, before);
    assert!(ChunkRecord::try_new(
        SchemaVersion::new(3),
        Chunk::new(ChunkPos::new(i32::MIN, 0))
    )
    .is_err());
}

fn sized_chest_payload(length: usize) -> Vec<u8> {
    // Versioned entity/item envelope with two opaque values costs 339 bytes.
    let mut chest = ChestInventory::new();
    *chest.slot_mut(26).unwrap() = ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        13,
        ComponentPatch::new(
            vec![
                ComponentValue::Opaque(OpaqueComponent::new(6, vec![0; 8192]).unwrap()),
                ComponentValue::Opaque(
                    OpaqueComponent::new(7, vec![0; length - 339 - 8192]).unwrap(),
                ),
            ],
            Vec::new(),
        ),
    )
    .unwrap();
    let payload = entity_payload(&BlockEntity::Chest(chest));
    assert_eq!(payload.len(), length);
    payload
}

fn aggregate_boundary() -> ChunkOverlayRecord {
    let mut entities = Vec::new();
    let full = sized_chest_payload(MAX_BLOCK_ENTITY_PAYLOAD_LEN);
    for index in 0..127_i32 {
        entities.push((BlockPos::new(index % 16, 64, index / 16), full.clone()));
    }
    let remaining = MAX_CHUNK_BLOCK_ENTITY_DATA_LEN - 127 * (16 + MAX_BLOCK_ENTITY_PAYLOAD_LEN);
    entities.push((
        BlockPos::new(15, 64, 15),
        sized_chest_payload(remaining - 16),
    ));
    ChunkOverlayRecord::from_parts(
        SchemaVersion::new(3),
        ChunkPos::ORIGIN,
        0,
        Vec::new(),
        0,
        entities,
    )
    .unwrap()
}

#[test]
fn aggregate_two_mib_boundary_and_next_byte_reject_before_copy() {
    let boundary = aggregate_boundary();
    assert_eq!(boundary.encoded_len(), MAX_CHUNK_BLOCK_ENTITY_DATA_LEN + 36);
    let bytes = codec::encode_chunk_overlay_record(&boundary).unwrap();
    assert_eq!(
        codec::decode_chunk_overlay_record(&bytes).unwrap(),
        boundary
    );
    let mut excessive = boundary.clone();
    excessive.block_entities.last_mut().unwrap().1.push(0);
    assert!(matches!(
        excessive.validate(),
        Err(StorageError::RecordTooLarge {
            max: MAX_CHUNK_BLOCK_ENTITY_DATA_LEN,
            ..
        })
    ));
    let mut forged = bytes;
    let last_length = 36 + 127 * (16 + MAX_BLOCK_ENTITY_PAYLOAD_LEN) + 12;
    let declared = u32::from_be_bytes(forged[last_length..last_length + 4].try_into().unwrap());
    forged[last_length..last_length + 4].copy_from_slice(&(declared + 1).to_be_bytes());
    assert_eq!(
        codec::decode_chunk_overlay_record(&forged),
        Err(StorageError::RecordTooLarge {
            len: MAX_CHUNK_BLOCK_ENTITY_DATA_LEN + 1,
            max: MAX_CHUNK_BLOCK_ENTITY_DATA_LEN
        })
    );
}

#[test]
fn constructors_and_whole_decoders_reject_aggregate_caps() {
    let mut boundary = aggregate_boundary();
    boundary.block_entities.last_mut().unwrap().1.push(0);
    assert!(matches!(
        ChunkOverlayRecord::from_parts(
            boundary.schema_version,
            boundary.pos,
            0,
            Vec::new(),
            0,
            boundary.block_entities
        ),
        Err(StorageError::RecordTooLarge { .. })
    ));
    let oversized = vec![0; MAX_CHUNK_RECORD_LEN + 1];
    for result in [
        codec::decode_chunk_overlay_record(&oversized).map(|_| ()),
        codec::decode_chunk_record(&oversized).map(|_| ()),
    ] {
        assert_eq!(
            result,
            Err(StorageError::RecordTooLarge {
                len: oversized.len(),
                max: MAX_CHUNK_RECORD_LEN
            })
        );
    }
}

#[test]
fn all_4096_empty_versioned_chests_remain_admissible() {
    let mut chunk = Chunk::new(ChunkPos::ORIGIN);
    for index in 0..MAX_BLOCK_ENTITIES {
        let position = BlockPos::new(
            i32::try_from(index % 16).unwrap(),
            -64 + i32::try_from(index / 256).unwrap(),
            i32::try_from((index / 16) % 16).unwrap(),
        );
        chunk
            .set_block_entity(position, BlockEntity::Chest(ChestInventory::new()))
            .unwrap();
    }
    let record =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), chunk.pos(), &chunk, 0).unwrap();
    assert_eq!(record.block_entity_count(), MAX_BLOCK_ENTITIES);
    assert!(record.encoded_len() < MAX_CHUNK_BLOCK_ENTITY_DATA_LEN);
}

#[test]
fn merged_entity_count_failure_leaves_full_target_unchanged() {
    let mut target = Chunk::new(ChunkPos::ORIGIN);
    for index in 0..MAX_BLOCK_ENTITIES {
        let position = BlockPos::new(
            i32::try_from(index % 16).unwrap(),
            -64 + i32::try_from(index / 256).unwrap(),
            i32::try_from((index / 16) % 16).unwrap(),
        );
        target
            .set_block_entity(position, BlockEntity::Sign(Sign::new(SignKind::Sign)))
            .unwrap();
    }
    let overlay = ChunkOverlayRecord::from_parts(
        SchemaVersion::new(3),
        target.pos(),
        0,
        Vec::new(),
        0,
        vec![(BlockPos::new(0, 64, 0), blank_payload())],
    )
    .unwrap();
    let before = target.clone();
    assert!(overlay.try_apply_to_chunk(&mut target).is_err());
    assert_eq!(target, before);
}

#[tokio::test]
async fn redb_reopen_preserves_component_overlays_and_full_chunk_nan_bits() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("components.redb");
    let chunk = component_chunk(ChunkPos::ORIGIN);
    let overlay =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), chunk.pos(), &chunk, 9).unwrap();
    let full = ChunkRecord::try_new(SchemaVersion::new(3), chunk).unwrap();
    let full_bytes = codec::encode_chunk_record(&full).unwrap();
    let overlay_bytes = codec::encode_chunk_overlay_record(&overlay).unwrap();
    {
        let store = RedbStore::open(&path).unwrap();
        store.save_chunk(key(ChunkPos::ORIGIN), full).await.unwrap();
        store
            .save_chunk_overlays(vec![(key(ChunkPos::ORIGIN), overlay)])
            .await
            .unwrap();
    }
    let reopened = RedbStore::open(&path).unwrap();
    let full = reopened
        .load_chunk(key(ChunkPos::ORIGIN))
        .await
        .unwrap()
        .unwrap();
    let overlay = reopened
        .load_chunk_overlay(key(ChunkPos::ORIGIN))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(codec::encode_chunk_record(&full).unwrap(), full_bytes);
    assert_eq!(
        codec::encode_chunk_overlay_record(&overlay).unwrap(),
        overlay_bytes
    );
    let mut restored = Chunk::new(ChunkPos::ORIGIN);
    overlay.try_apply_to_chunk(&mut restored).unwrap();
    assert_eq!(
        capture_block_entities(&restored).unwrap(),
        capture_block_entities(full.chunk()).unwrap()
    );
}

async fn assert_atomic_batches(store: &dyn WorldStore) {
    let origin = ChunkPos::ORIGIN;
    let original = ChunkRecord::new(SchemaVersion::new(7), Chunk::new(origin));
    store
        .save_chunk(key(origin), original.clone())
        .await
        .unwrap();
    let mut bad_chunk = Chunk::new(ChunkPos::new(1, 0));
    let mut bad = Sign::new(SignKind::Sign);
    bad.set_face_lines(true, std::array::from_fn(|_| "x".repeat(1025)));
    bad_chunk
        .set_block_entity(bad_chunk.pos().origin_block(64), BlockEntity::Sign(bad))
        .unwrap();
    assert!(store
        .save_chunks(vec![
            (
                key(origin),
                ChunkRecord::new(SchemaVersion::new(8), Chunk::new(origin))
            ),
            (
                key(bad_chunk.pos()),
                ChunkRecord::new(SchemaVersion::new(8), bad_chunk)
            )
        ])
        .await
        .is_err());
    assert_eq!(
        store
            .load_chunk(key(origin))
            .await
            .unwrap()
            .unwrap()
            .schema_version(),
        SchemaVersion::new(7)
    );

    let original =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), origin, &Chunk::new(origin), 7)
            .unwrap();
    store
        .save_chunk_overlays(vec![(key(origin), original.clone())])
        .await
        .unwrap();
    let replacement =
        ChunkOverlayRecord::try_from_chunk(SchemaVersion::new(3), origin, &Chunk::new(origin), 8)
            .unwrap();
    let mut bad_overlay = malformed_overlay(true);
    bad_overlay.pos = ChunkPos::new(1, 0);
    for (pos, _) in &mut bad_overlay.block_entities {
        *pos = BlockPos::new(pos.x() + 16, pos.y(), pos.z());
    }
    assert!(store
        .save_chunk_overlays(vec![
            (key(origin), replacement),
            (key(bad_overlay.pos), bad_overlay)
        ])
        .await
        .is_err());
    assert_eq!(
        store
            .load_chunk_overlay(key(origin))
            .await
            .unwrap()
            .unwrap(),
        original
    );
    assert!(store
        .save_chunk(
            key(ChunkPos::new(1, 0)),
            ChunkRecord::new(SchemaVersion::new(3), Chunk::new(origin))
        )
        .await
        .is_err());
}

#[tokio::test]
async fn both_backends_reject_late_bad_batch_members_without_prior_changes() {
    assert_atomic_batches(&InMemoryStore::new()).await;
    let directory = tempfile::tempdir().unwrap();
    let redb = RedbStore::open(directory.path().join("atomic.redb")).unwrap();
    assert_atomic_batches(&redb).await;
}

#[test]
fn duplicate_and_foreign_overlay_records_are_rejected_by_decoder() {
    let mut overlay = ChunkOverlayRecord::from_parts(
        SchemaVersion::new(3),
        ChunkPos::ORIGIN,
        0,
        Vec::new(),
        0,
        vec![
            (BlockPos::new(0, 64, 0), blank_payload()),
            (BlockPos::new(1, 64, 0), blank_payload()),
        ],
    )
    .unwrap();
    let mut bytes = codec::encode_chunk_overlay_record(&overlay).unwrap();
    let second = 36 + 16 + overlay.block_entities[0].1.len();
    let first_position: [u8; 12] = bytes[36..48].try_into().unwrap();
    bytes[second..second + 12].copy_from_slice(&first_position);
    assert!(matches!(
        codec::decode_chunk_overlay_record(&bytes),
        Err(StorageError::DuplicateBlockEntity { .. })
    ));
    overlay.block_entities.pop();
    let mut bytes = codec::encode_chunk_overlay_record(&overlay).unwrap();
    bytes[36..40].copy_from_slice(&16_i32.to_be_bytes());
    assert!(matches!(
        codec::decode_chunk_overlay_record(&bytes),
        Err(StorageError::BlockEntityOutsideChunk { .. })
    ));
}
