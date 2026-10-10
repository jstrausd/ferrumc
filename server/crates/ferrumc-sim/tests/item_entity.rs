//! Independent native byte fixtures and real tick-capability item contracts.

use std::num::{NonZeroU8, NonZeroUsize};

use ferrumc_core::{DimensionId, EntityId, EntityUuid, WorldId};
use ferrumc_items::{
    decode_persisted_stack, encode_persisted_stack, ComponentPatch, ComponentTypeId,
    ComponentValue, ItemId, ItemStack, OpaqueComponent,
};
use ferrumc_math::{ChunkPos, ShardPos, Vec3};
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_sim::{
    decode_item_entity, step_active_item_counters, step_inactive_item_counters, ChunkTicket,
    EntitySpawn, EntityStore, EntityStoreError, EntityTransform, ItemCounterMode,
    ItemDespawnDecision, ItemEntityCounters, ItemEntityError, ItemEntityProperties,
    ItemEntitySpawn, ItemNativeKeyField, ItemPropertyComponent, ItemReferenceField, ShardId,
    SimShard, TicketReason, MAX_ITEM_ENTITY_BYTES_PER_SHARD, MAX_NATIVE_ITEM_ENTITY_BYTES,
    NATIVE_ITEM_ENTITY_HEADER_BYTES,
};
use ferrumc_storage::{
    EntityRecord, InMemoryStore, PersistentEntityKey, PersistentEntityRecord, SchemaVersion,
};
use ferrumc_world::FlatWorldGenerator;

fn nz(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
fn uuid(id: u16) -> EntityUuid {
    EntityUuid::from_bytes(u128::from(id).to_be_bytes())
}
fn owner() -> ShardId {
    ShardId::try_new(WorldId::new(7), DimensionId::new(13), ShardPos::new(-1, 0)).unwrap()
}
fn base(id: u16, x: f64, kind: &str) -> EntitySpawn {
    EntitySpawn::new(
        owner().world(),
        owner().dimension(),
        uuid(id),
        EntityId::new(i32::from(id)),
        EntityTypeId::from_name(kind).unwrap(),
        EntityTransform::try_new(Vec3::new(x, 64.0, 1.0), -0.0, -0.0).unwrap(),
    )
}
fn props() -> ItemEntityProperties {
    ItemEntityProperties::try_new(
        Vec3::new(-0.0, 11.5, f64::from_bits(1)),
        -0.0,
        ItemEntityCounters::new(i32::MAX, 32768, -5),
        Some(uuid(0)),
        Some(uuid(99)),
    )
    .unwrap()
}
fn stone(count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}
fn ticket() -> ChunkTicket {
    ChunkTicket::of(TicketReason::Spawn)
}

async fn shard(capacity: usize, budget: usize) -> SimShard {
    let mut shard =
        SimShard::in_dimension(owner().position(), owner().world(), owner().dimension());
    shard
        .initialize_entities_with_item_budget(nz(capacity), nz(budget))
        .unwrap();
    let storage = InMemoryStore::new();
    for chunk in [ChunkPos::new(-1, 0), ChunkPos::new(-2, 0)] {
        shard
            .loaded_chunks_mut()
            .acquire(&storage, &FlatWorldGenerator::new(), chunk, ticket())
            .await
            .unwrap();
    }
    shard
}

fn component(out: &mut Vec<u8>, kind: u8, id: i32, payload: &[u8]) {
    out.push(kind);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
}

// Independently framed FCIS fixture: patch order, opaque/typed distinction,
// removal, duplicate NBT names and exact IEEE bits must all survive the owner.
fn stack_fixture() -> Vec<u8> {
    let mut nbt = vec![10, 0, 0];
    for bits in [0x8000_0000_0000_0000_u64, 0x7ff8_1234_5678_9abc] {
        nbt.extend_from_slice(&[6, 0, 3, b'd', b'u', b'p']);
        nbt.extend_from_slice(&bits.to_be_bytes());
    }
    nbt.extend_from_slice(&[5, 0, 3, b'b', b'o', b'b']);
    nbt.extend_from_slice(&0x8000_0000_u32.to_be_bytes());
    nbt.push(0);
    let name = b"minecraft:stone";
    let mut out = b"FCIS\0\x01\x01".to_vec();
    out.extend_from_slice(&(name.len() as u16).to_be_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(&[17, 5, 1]);
    component(&mut out, 7, 6, &[0xfe, 0, 0xfd]);
    component(&mut out, 6, 0, &nbt);
    component(&mut out, 2, 3, &i32::MIN.to_be_bytes());
    component(&mut out, 4, 4, &[]);
    component(&mut out, 1, 1, &[32]);
    out.extend_from_slice(&12_i32.to_be_bytes());
    assert_eq!(
        encode_persisted_stack(&decode_persisted_stack(&out).unwrap()).unwrap(),
        out
    );
    out
}

fn record(body: Vec<u8>, chunk: ChunkPos, schema: u32) -> PersistentEntityRecord {
    PersistentEntityRecord::new(
        chunk,
        EntityRecord::new(SchemaVersion::new(schema), body).unwrap(),
    )
    .unwrap()
}

// Native fixture uses the published absolute offsets, not the FCIE encoder.
fn native_fixture(stack: &[u8]) -> (PersistentEntityKey, PersistentEntityRecord) {
    let key = PersistentEntityKey::new(WorldId::new(7), DimensionId::new(13), uuid(1));
    let mut bytes = vec![0; 144];
    bytes[0..4].copy_from_slice(b"FCIE");
    bytes[4..6].copy_from_slice(&1_u16.to_be_bytes());
    bytes[6..10].copy_from_slice(&69_i32.to_be_bytes());
    bytes[10..14].copy_from_slice(&7_u32.to_be_bytes());
    bytes[14..18].copy_from_slice(&13_u32.to_be_bytes());
    bytes[18..34].copy_from_slice(&uuid(1).to_bytes());
    for (offset, value) in [
        (34, -0.5_f64),
        (42, 64.0),
        (50, 1.0),
        (66, -0.0),
        (74, 11.5),
        (82, f64::from_bits(1)),
    ] {
        bytes[offset..offset + 8].copy_from_slice(&value.to_bits().to_be_bytes());
    }
    for offset in [58, 62, 90] {
        bytes[offset..offset + 4].copy_from_slice(&0x8000_0000_u32.to_be_bytes());
    }
    for (offset, value) in [(94, i32::MAX), (98, 32768), (102, -5)] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
    bytes[106] = 1;
    bytes[123] = 1;
    bytes[124..140].copy_from_slice(&uuid(99).to_bytes());
    bytes[140..144].copy_from_slice(&(stack.len() as u32).to_be_bytes());
    bytes.extend_from_slice(stack);
    (key, record(bytes, ChunkPos::new(-1, 0), 1))
}

#[test]
fn golden_native_header_preserves_all_runtime_bits_and_separate_nil_reference() {
    let stack = stack_fixture();
    let (key, row) = native_fixture(&stack);
    let restored = decode_item_entity(key, &row).unwrap();
    assert_eq!(restored.key(), key);
    assert_eq!(restored.entity_type().resource_location(), "minecraft:item");
    assert_eq!(restored.transform().chunk(), ChunkPos::new(-1, 0));
    let properties = restored.item_state().properties();
    assert_eq!(properties.motion().x.to_bits(), 0x8000_0000_0000_0000);
    assert_eq!(properties.motion().y.to_bits(), 11.5_f64.to_bits());
    assert_eq!(properties.motion().z.to_bits(), 1);
    assert_eq!(properties.bob_offset().to_bits(), 0x8000_0000);
    assert_eq!(
        properties.counters(),
        ItemEntityCounters::new(i32::MAX, 32768, -5)
    );
    assert_eq!(properties.owner_target(), Some(uuid(0)));
    assert_eq!(properties.thrower(), Some(uuid(99)));
    assert_eq!(
        encode_persisted_stack(restored.item_state().stack()).unwrap(),
        stack
    );
    assert_eq!(
        restored.item_state().native_body_len(),
        row.record().data().len()
    );
}

#[test]
fn properties_reject_each_nonfinite_component_without_clamping_finite_values() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for (motion, component) in [
            (Vec3::new(invalid, 0.0, 0.0), ItemPropertyComponent::MotionX),
            (Vec3::new(0.0, invalid, 0.0), ItemPropertyComponent::MotionY),
            (Vec3::new(0.0, 0.0, invalid), ItemPropertyComponent::MotionZ),
        ] {
            assert!(
                matches!(ItemEntityProperties::try_new(motion,0.0,ItemEntityCounters::new(0,0,5),None,None), Err(ItemEntityError::NonFiniteProperty { component: actual }) if actual == component)
            );
        }
    }
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(ItemEntityProperties::try_new(
            Vec3::ZERO,
            invalid,
            ItemEntityCounters::new(0, 0, 5),
            None,
            None
        )
        .is_err());
    }
    let extreme = ItemEntityProperties::try_new(
        Vec3::new(f64::MAX, -f64::MAX, -0.0),
        -f32::MAX,
        ItemEntityCounters::new(i32::MIN, i32::MAX, i32::MIN),
        None,
        None,
    )
    .unwrap();
    assert_eq!(extreme.motion().z.to_bits(), (-0.0_f64).to_bits());
    assert_eq!(extreme.counters().health(), i32::MIN);
}

#[test]
fn native_parser_rejects_every_cutpoint_and_outer_identity_mismatch() {
    let (key, row) = native_fixture(&stack_fixture());
    let bytes = row.record().data();
    for cut in 0..bytes.len() {
        assert!(
            decode_item_entity(key, &record(bytes[..cut].to_vec(), row.chunk(), 1)).is_err(),
            "cut {cut}"
        );
    }
    for (other, field) in [
        (
            PersistentEntityKey::new(WorldId::new(8), key.dimension(), key.uuid()),
            ItemNativeKeyField::World,
        ),
        (
            PersistentEntityKey::new(key.world(), DimensionId::new(14), key.uuid()),
            ItemNativeKeyField::Dimension,
        ),
        (
            PersistentEntityKey::new(key.world(), key.dimension(), uuid(2)),
            ItemNativeKeyField::Uuid,
        ),
    ] {
        assert!(
            matches!(decode_item_entity(other,&row),Err(ItemEntityError::NativeKeyMismatch { field: actual }) if actual == field)
        );
    }
    assert!(matches!(
        decode_item_entity(key, &record(bytes.to_vec(), ChunkPos::new(0, 0), 1)),
        Err(ItemEntityError::NativeChunkMismatch { .. })
    ));
    assert!(matches!(
        decode_item_entity(key, &record(bytes.to_vec(), row.chunk(), 0)),
        Err(ItemEntityError::UnsupportedPayloadSchema { found: 0 })
    ));
    assert!(matches!(
        decode_item_entity(key, &record(bytes.to_vec(), row.chunk(), u32::MAX)),
        Err(ItemEntityError::UnsupportedPayloadSchema { found: u32::MAX })
    ));
}

#[test]
fn native_parser_checks_magic_type_and_canonical_transform_before_admission() {
    let (key, row) = native_fixture(&stack_fixture());
    let bytes = row.record().data();
    let mutate = |offset: usize, replacement: &[u8]| {
        let mut body = bytes.to_vec();
        body[offset..offset + replacement.len()].copy_from_slice(replacement);
        decode_item_entity(key, &record(body, row.chunk(), 1))
    };
    assert!(matches!(
        mutate(0, b"FXXE"),
        Err(ItemEntityError::InvalidMagic)
    ));
    assert!(matches!(
        mutate(4, &2_u16.to_be_bytes()),
        Err(ItemEntityError::UnsupportedNativeVersion { found: 2 })
    ));
    for value in [-1, 0, 151, i32::MAX] {
        assert!(matches!(
            mutate(6, &value.to_be_bytes()),
            Err(ItemEntityError::InvalidNativeEntityType { .. })
        ));
    }
    for offset in [34, 42, 50] {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(mutate(offset, &value.to_bits().to_be_bytes()).is_err());
        }
    }
    assert!(mutate(34, &30_000_001_f64.to_bits().to_be_bytes()).is_err());
    assert!(mutate(42, &20_000_001_f64.to_bits().to_be_bytes()).is_err());
    for (offset, value) in [
        (58, 360_f32),
        (58, -360_f32),
        (62, 91_f32),
        (62, -91_f32),
        (58, f32::NAN),
        (62, f32::INFINITY),
    ] {
        assert!(mutate(offset, &value.to_bits().to_be_bytes()).is_err());
    }
}

#[test]
fn native_parser_rejects_nonfinite_properties_and_malformed_optional_references() {
    let (key, row) = native_fixture(&stack_fixture());
    let bytes = row.record().data();
    let mutate = |offset: usize, replacement: &[u8]| {
        let mut body = bytes.to_vec();
        body[offset..offset + replacement.len()].copy_from_slice(replacement);
        decode_item_entity(key, &record(body, row.chunk(), 1))
    };
    for offset in [66, 74, 82] {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                mutate(offset, &value.to_bits().to_be_bytes()),
                Err(ItemEntityError::NonFiniteProperty { .. })
            ));
        }
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            mutate(90, &value.to_bits().to_be_bytes()),
            Err(ItemEntityError::NonFiniteProperty {
                component: ItemPropertyComponent::BobOffset
            })
        ));
    }
    for (offset, field) in [
        (106, ItemReferenceField::OwnerTarget),
        (123, ItemReferenceField::Thrower),
    ] {
        assert!(
            matches!(mutate(offset,&[2]),Err(ItemEntityError::InvalidUuidPresence { field: actual, value: 2 }) if actual == field)
        );
        let mut body = bytes.to_vec();
        body[offset] = 0;
        body[offset + 1] = 1;
        assert!(
            matches!(decode_item_entity(key,&record(body,row.chunk(),1)),Err(ItemEntityError::NonZeroAbsentUuid { field: actual }) if actual == field)
        );
    }
}

#[test]
fn native_parser_checks_exact_fcis_length_and_semantics_before_admission() {
    let (key, row) = native_fixture(&stack_fixture());
    let bytes = row.record().data();
    let mutate = |offset: usize, replacement: &[u8]| {
        let mut body = bytes.to_vec();
        body[offset..offset + replacement.len()].copy_from_slice(replacement);
        decode_item_entity(key, &record(body, row.chunk(), 1))
    };
    for value in [0, u32::MAX] {
        assert!(matches!(
            mutate(140, &value.to_be_bytes()),
            Err(ItemEntityError::InvalidFcisLength { .. })
        ));
    }
    assert!(matches!(
        mutate(144, b"XXXX"),
        Err(ItemEntityError::Stack(_))
    ));
    let mut trailing = bytes.to_vec();
    trailing.push(0);
    assert!(matches!(
        decode_item_entity(key, &record(trailing, row.chunk(), 1)),
        Err(ItemEntityError::InvalidFcisLength { .. })
    ));
    let (key, empty) = native_fixture(b"FCIS\0\x01\0");
    assert!(matches!(
        decode_item_entity(key, &empty),
        Err(ItemEntityError::EmptyStack)
    ));
    let mut internal_trailing = bytes.to_vec();
    internal_trailing.push(0);
    let remaining = (internal_trailing.len() - 144) as u32;
    internal_trailing[140..144].copy_from_slice(&remaining.to_be_bytes());
    assert!(matches!(
        decode_item_entity(key, &record(internal_trailing, row.chunk(), 1)),
        Err(ItemEntityError::Stack(_))
    ));
    let count_offset = 144 + 9 + b"minecraft:stone".len();
    for count in [0, 65, 255] {
        assert!(matches!(
            mutate(count_offset, &[count]),
            Err(ItemEntityError::Stack(_))
        ));
    }
}

#[test]
fn hand_checked_counter_cases_preserve_sentinels_wrap_and_exact_signed_thresholds() {
    use ItemDespawnDecision::{Due, Keep};
    for (age, delay, rate, next_age, next_delay, decision) in [
        (0, 1, 6000, 1, 0, Keep),
        (5999, 10, 6000, 6000, 9, Due),
        (-32769, 32768, 6000, -32768, 32767, Keep),
        (-32768, 32767, 6000, -32768, 32767, Keep),
        (-32767, 32766, 6000, -32766, 32765, Keep),
        (i32::MAX, i32::MAX, 6000, i32::MIN, i32::MAX - 1, Keep),
        (i32::MIN, i32::MIN, 6000, i32::MIN + 1, i32::MIN, Keep),
        (0, -1, 6000, 1, -1, Keep),
        (-1, 0, 0, 0, 0, Due),
        (-32768, 0, -32768, -32768, 0, Due),
        (-32768, 0, -32767, -32768, 0, Keep),
        (i32::MAX, 0, i32::MIN, i32::MIN, 0, Due),
        (-2, -1, -1, -1, -1, Due),
    ] {
        for health in [i32::MIN, -1, 0, 5, i32::MAX] {
            let before = ItemEntityCounters::new(age, delay, health);
            for step in [
                step_active_item_counters(before, rate),
                step_inactive_item_counters(before, rate),
            ] {
                assert_eq!(
                    step.counters(),
                    ItemEntityCounters::new(next_age, next_delay, health)
                );
                assert_eq!(step.decision(), decision);
            }
        }
    }
}

#[tokio::test]
async fn checked_snapshot_and_shard_clone_preserve_payload_with_fresh_incarnation() {
    let expected = stack_fixture();
    let stack = decode_persisted_stack(&expected).unwrap();
    let mut shard = shard(3, 8192).await;
    let handle = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(base(1, -0.5, "item"), stack, props()))
                .unwrap()
        })
        .unwrap()
        .0;
    let store = shard.entities().unwrap();
    let captured = store.capture_item(&handle).unwrap();
    assert_eq!(captured, native_fixture(&expected));
    assert_eq!(
        store
            .snapshot_item(&handle)
            .unwrap()
            .item_state()
            .properties()
            .motion()
            .x
            .to_bits(),
        (-0.0_f64).to_bits()
    );
    let copied = store.try_snapshot().unwrap();
    assert!(matches!(
        copied.item_state(&handle),
        Err(ItemEntityError::Store(EntityStoreError::ForeignHandle))
    ));
    let copy_handle = copied.handle_for_uuid(uuid(1)).unwrap();
    assert_eq!(copied.capture_item(&copy_handle).unwrap(), captured);
    let clone = shard.clone();
    assert_eq!(
        clone.entities().unwrap().item_bytes_used(),
        store.item_bytes_used()
    );
    assert!(matches!(
        clone.entities().unwrap().item_state(&handle),
        Err(ItemEntityError::Store(EntityStoreError::ForeignHandle))
    ));
}

#[tokio::test]
async fn real_context_owns_payload_move_remove_reuse_and_new_wire_restore() {
    let expected = stack_fixture();
    let stack = decode_persisted_stack(&expected).unwrap();
    let mut shard = shard(3, 8192).await;
    let handle = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(base(1, -0.5, "item"), stack, props()))
                .unwrap()
        })
        .unwrap()
        .0;
    let captured = shard.entities().unwrap().capture_item(&handle).unwrap();
    shard
        .run_tick_with_entity_work(|ctx| {
            assert_eq!(
                ctx.remove(&handle),
                Err(EntityStoreError::PayloadRequiresOwnedRemove)
            );
            ctx.update_transform(
                &handle,
                EntityTransform::try_new(Vec3::new(-17.0, 65.0, 1.0), -0.0, -0.0).unwrap(),
            )
            .unwrap();
        })
        .unwrap();
    assert_eq!(
        shard
            .entities()
            .unwrap()
            .in_chunk(ChunkPos::new(-1, 0))
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        shard
            .entities()
            .unwrap()
            .in_chunk(ChunkPos::new(-2, 0))
            .unwrap()
            .len(),
        1
    );
    let bytes_used = shard.entities().unwrap().item_bytes_used();
    assert!(shard
        .loaded_chunks_mut()
        .release(ChunkPos::new(-2, 0), ticket())
        .is_unloaded());
    assert!(shard.entities().unwrap().capture_item(&handle).is_ok());
    shard
        .run_tick_with_entity_work(|ctx| {
            assert!(matches!(
                ctx.step_item_counters(&handle, ItemCounterMode::Active, 6000),
                Err(ItemEntityError::Store(
                    EntityStoreError::ChunkNotResident { .. }
                ))
            ));
            let removed = ctx.remove_owned(&handle).unwrap();
            assert_eq!(
                encode_persisted_stack(removed.item_state().unwrap().stack()).unwrap(),
                expected
            );
        })
        .unwrap();
    assert!(bytes_used > 0);
    assert_eq!(shard.entities().unwrap().item_bytes_used(), 0);
    assert!(matches!(
        shard.entities().unwrap().item_state(&handle),
        Err(ItemEntityError::Store(EntityStoreError::StaleHandle))
    ));
    let restored = decode_item_entity(captured.0, &captured.1).unwrap();
    let new_handle = shard
        .run_tick_with_entity_work(|ctx| ctx.restore_item(restored, EntityId::new(9876)).unwrap())
        .unwrap()
        .0;
    assert_ne!(handle, new_handle);
    let store = shard.entities().unwrap();
    assert_eq!(
        store.get(&new_handle).unwrap().wire_id(),
        EntityId::new(9876)
    );
    assert!(store.handle_for_wire_id(EntityId::new(1)).is_none());
    assert_eq!(store.capture_item(&new_handle).unwrap(), captured);
    assert_eq!(store.item_bytes_used(), bytes_used);
}

#[tokio::test]
async fn due_reset_and_checked_props_do_not_remove_or_touch_other_payload_fields() {
    let mut shard = shard(1, 1024).await;
    let initial = ItemEntityProperties::try_new(
        props().motion(),
        props().bob_offset(),
        ItemEntityCounters::new(5999, 2, -1),
        props().owner_target(),
        props().thrower(),
    )
    .unwrap();
    let handle = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(
                base(1, -0.5, "item"),
                stone(1),
                initial,
            ))
            .unwrap()
        })
        .unwrap()
        .0;
    let before = shard.entities().unwrap().item_bytes_used();
    shard
        .run_tick_with_entity_work(|ctx| {
            assert_eq!(
                ctx.step_item_counters(&handle, ItemCounterMode::Inactive, 6000)
                    .unwrap(),
                ItemDespawnDecision::Due
            );
            assert_eq!(ctx.entities().len(), 1);
            let state = ctx.entities().item_state(&handle).unwrap();
            assert_eq!(
                state.properties().counters(),
                ItemEntityCounters::new(6000, 1, -1)
            );
            assert_eq!(
                state.properties().motion().x.to_bits(),
                initial.motion().x.to_bits()
            );
            ctx.reset_item_despawn_age(&handle).unwrap();
            assert_eq!(
                ctx.entities()
                    .item_state(&handle)
                    .unwrap()
                    .properties()
                    .counters(),
                ItemEntityCounters::new(0, 1, -1)
            );
            assert_eq!(
                ctx.set_item_properties(&handle, props())
                    .unwrap()
                    .counters(),
                ItemEntityCounters::new(0, 1, -1)
            );
            assert_eq!(
                ctx.step_item_counters(&handle, ItemCounterMode::Active, 6000)
                    .unwrap(),
                ItemDespawnDecision::Keep
            );
            assert_eq!(
                ctx.entities()
                    .item_state(&handle)
                    .unwrap()
                    .properties()
                    .counters(),
                ItemEntityCounters::new(i32::MIN, 32767, -5)
            );
        })
        .unwrap();
    assert_eq!(shard.entities().unwrap().item_bytes_used(), before);
}

#[tokio::test]
async fn legacy_placeholder_and_non_item_consumers_fail_typed_and_zero_budget_preserves_input() {
    let mut shard = shard(3, 1024).await;
    shard
        .run_tick_with_entity_work(|ctx| {
            let placeholder = ctx.spawn(base(1, -0.5, "item")).unwrap();
            let cow = ctx.spawn(base(2, -0.5, "cow")).unwrap();
            for handle in [&placeholder, &cow] {
                assert!(ctx.entities().snapshot_item(handle).is_err());
                assert!(ctx.entities().capture_item(handle).is_err());
                assert!(ctx
                    .step_item_counters(handle, ItemCounterMode::Active, 6000)
                    .is_err());
                let refusal = ctx.replace_item_stack(handle, stone(7)).unwrap_err();
                assert_eq!(refusal.into_parts().1.count(), 7);
            }
            assert!(matches!(
                ctx.entities().item_state(&placeholder),
                Err(ItemEntityError::MissingItemState)
            ));
            assert!(matches!(
                ctx.entities().item_state(&cow),
                Err(ItemEntityError::WrongEntityType { .. })
            ));
            assert!(ctx.remove_owned(&cow).unwrap().item_state().is_none());
            assert!(ctx
                .remove_owned(&placeholder)
                .unwrap()
                .item_state()
                .is_none());
        })
        .unwrap();
    let mut legacy =
        SimShard::in_dimension(owner().position(), owner().world(), owner().dimension());
    legacy.initialize_entities(nz(1)).unwrap();
    let storage = InMemoryStore::new();
    legacy
        .loaded_chunks_mut()
        .acquire(
            &storage,
            &FlatWorldGenerator::new(),
            ChunkPos::new(-1, 0),
            ticket(),
        )
        .await
        .unwrap();
    legacy
        .run_tick_with_entity_work(|ctx| {
            let refusal = ctx
                .spawn_item(ItemEntitySpawn::new(
                    base(1, -0.5, "item"),
                    stone(9),
                    props(),
                ))
                .unwrap_err();
            assert!(matches!(
                refusal.reason(),
                ItemEntityError::ItemByteBudgetExceeded { limit: 0, .. }
            ));
            let (_, proposal) = refusal.into_parts();
            assert_eq!(proposal.stack().count(), 9);
            assert_eq!(
                proposal.properties().motion().x.to_bits(),
                (-0.0_f64).to_bits()
            );
            assert_eq!(ctx.entities().len(), 0);
        })
        .unwrap();
}

fn opaque_body(target: usize) -> ItemStack {
    let ids: Vec<_> = (0..=95)
        .filter(|&id| OpaqueComponent::new(id, vec![]).is_some())
        .take(32)
        .collect();
    let build = |last_size| {
        ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            1,
            ComponentPatch::new(
                ids.iter()
                    .enumerate()
                    .map(|(i, &id)| {
                        ComponentValue::Opaque(
                            OpaqueComponent::new(
                                id,
                                vec![0xa5; if i == 31 { last_size } else { 8192 }],
                            )
                            .unwrap(),
                        )
                    })
                    .collect(),
                vec![],
            ),
        )
        .unwrap()
    };
    let full = build(8192);
    let encoded = encode_persisted_stack(&full).unwrap().len() + 144;
    assert!(encoded >= target);
    let stack = build(8192 - (encoded - target));
    assert_eq!(encode_persisted_stack(&stack).unwrap().len() + 144, target);
    stack
}

#[tokio::test]
async fn exact_native_body_and_aggregate_boundaries_refuse_without_losing_valid_components() {
    assert_eq!(NATIVE_ITEM_ENTITY_HEADER_BYTES, 144);
    assert_eq!(MAX_NATIVE_ITEM_ENTITY_BYTES, 262_144);
    let mut shard = shard(2, MAX_NATIVE_ITEM_ENTITY_BYTES).await;
    let exact = opaque_body(MAX_NATIVE_ITEM_ENTITY_BYTES);
    let expected = encode_persisted_stack(&exact).unwrap();
    let handle = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(base(1, -0.5, "item"), exact, props()))
                .unwrap()
        })
        .unwrap()
        .0;
    assert_eq!(
        shard.entities().unwrap().item_bytes_used(),
        MAX_NATIVE_ITEM_ENTITY_BYTES
    );
    let captured = shard.entities().unwrap().capture_item(&handle).unwrap();
    assert_eq!(
        captured.1.record().data().len(),
        MAX_NATIVE_ITEM_ENTITY_BYTES
    );
    assert_eq!(
        encode_persisted_stack(
            decode_item_entity(captured.0, &captured.1)
                .unwrap()
                .item_state()
                .stack()
        )
        .unwrap(),
        expected
    );
    shard
        .run_tick_with_entity_work(|ctx| {
            let oversized = opaque_body(MAX_NATIVE_ITEM_ENTITY_BYTES + 1);
            let wanted = encode_persisted_stack(&oversized).unwrap();
            let refused = ctx.replace_item_stack(&handle, oversized).unwrap_err();
            assert!(matches!(
                refused.reason(),
                ItemEntityError::NativeBodyTooLarge { size: 262_145, .. }
            ));
            assert_eq!(
                encode_persisted_stack(&refused.into_parts().1).unwrap(),
                wanted
            );
            let refused = ctx
                .spawn_item(ItemEntitySpawn::new(
                    base(2, -0.5, "item"),
                    stone(1),
                    props(),
                ))
                .unwrap_err();
            assert!(matches!(
                refused.reason(),
                ItemEntityError::ItemByteBudgetExceeded { .. }
            ));
            assert_eq!(
                ctx.entities().item_bytes_used(),
                MAX_NATIVE_ITEM_ENTITY_BYTES
            );
            assert_eq!(
                encode_persisted_stack(ctx.entities().item_state(&handle).unwrap().stack())
                    .unwrap(),
                expected
            );
            let old = ctx.replace_item_stack(&handle, stone(4)).unwrap();
            assert_eq!(encode_persisted_stack(&old).unwrap(), expected);
            let small = ctx.entities().item_bytes_used();
            assert!(small < MAX_NATIVE_ITEM_ENTITY_BYTES);
            let old = ctx
                .replace_item_stack(&handle, opaque_body(MAX_NATIVE_ITEM_ENTITY_BYTES))
                .unwrap();
            assert_eq!(old.count(), 4);
            assert_eq!(
                ctx.entities().item_bytes_used(),
                MAX_NATIVE_ITEM_ENTITY_BYTES
            );
        })
        .unwrap();
}

#[tokio::test]
async fn malformed_replacements_and_foreign_stale_handles_return_all_input_ownership() {
    let mut shard = shard(2, 2048).await;
    let handle = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(
                base(1, -0.5, "item"),
                stone(2),
                props(),
            ))
            .unwrap()
        })
        .unwrap()
        .0;
    let before = shard.entities().unwrap().capture_item(&handle).unwrap();
    let used = shard.entities().unwrap().item_bytes_used();
    let mut foreign = EntityStore::new(owner(), nz(1)).unwrap();
    let foreign_handle = foreign.spawn(base(99, -0.5, "item")).unwrap();
    shard
        .run_tick_with_entity_work(|ctx| {
            let invalid_count = ItemStack::new(
                ItemId::from_name("stone").unwrap(),
                NonZeroU8::new(65).unwrap(),
                ComponentPatch::empty(),
            );
            let duplicate = ItemStack::try_new(
                ItemId::from_name("stone").unwrap(),
                1,
                ComponentPatch::new(
                    vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
                    vec![],
                ),
            )
            .unwrap();
            let conflict = ItemStack::try_new(
                ItemId::from_name("stone").unwrap(),
                1,
                ComponentPatch::new(
                    vec![ComponentValue::Damage(1)],
                    vec![ComponentTypeId::new(3)],
                ),
            )
            .unwrap();
            let invalid_component = ItemStack::try_new(
                ItemId::from_name("stone").unwrap(),
                1,
                ComponentPatch::new(vec![ComponentValue::MaxStackSize(0)], vec![]),
            )
            .unwrap();
            for input in [
                invalid_count,
                duplicate,
                conflict,
                invalid_component,
                ItemStack::empty(),
            ] {
                let wanted = input.clone();
                let refused = ctx.replace_item_stack(&handle, input).unwrap_err();
                let (_, returned) = refused.into_parts();
                assert_eq!(returned, wanted);
                assert_eq!(ctx.entities().capture_item(&handle).unwrap(), before);
                assert_eq!(ctx.entities().item_bytes_used(), used);
            }
            let refused = ctx
                .replace_item_stack(&foreign_handle, stone(7))
                .unwrap_err();
            assert!(matches!(
                refused.reason(),
                ItemEntityError::Store(EntityStoreError::ForeignHandle)
            ));
            assert_eq!(refused.into_parts().1.count(), 7);
            ctx.remove_owned(&handle).unwrap();
            let replacement = ctx
                .spawn_item(ItemEntitySpawn::new(
                    base(2, -0.5, "item"),
                    stone(3),
                    props(),
                ))
                .unwrap();
            assert_ne!(replacement, handle);
            let refused = ctx.replace_item_stack(&handle, stone(8)).unwrap_err();
            assert!(matches!(
                refused.reason(),
                ItemEntityError::Store(EntityStoreError::StaleHandle)
            ));
            assert_eq!(refused.into_parts().1.count(), 8);
        })
        .unwrap();
}

#[test]
fn invalid_initialization_never_publishes_a_partial_store() {
    let mut shard =
        SimShard::in_dimension(owner().position(), owner().world(), owner().dimension());
    assert!(matches!(
        shard.initialize_entities_with_item_budget(nz(1), nz(MAX_ITEM_ENTITY_BYTES_PER_SHARD + 1)),
        Err(ItemEntityError::InvalidItemByteBudget { .. })
    ));
    assert!(shard.entities().is_none());
    shard
        .initialize_entities_with_item_budget(nz(1), nz(1))
        .unwrap();
    assert_eq!(shard.entities().unwrap().item_byte_budget(), 1);
    assert!(matches!(
        shard.initialize_entities_with_item_budget(nz(1), nz(2048)),
        Err(ItemEntityError::Store(EntityStoreError::AlreadyInitialized))
    ));
    assert_eq!(shard.entities().unwrap().item_byte_budget(), 1);
}

#[tokio::test]
async fn every_spawn_refusal_returns_complete_proposal_and_keeps_all_owner_indexes() {
    let mut shard = shard(3, 2048).await;
    let handle = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(
                base(1, -0.5, "item"),
                stone(2),
                props(),
            ))
            .unwrap()
        })
        .unwrap()
        .0;
    let before = shard.entities().unwrap().capture_item(&handle).unwrap();
    let used = shard.entities().unwrap().item_bytes_used();
    let wrong_scope = EntitySpawn::new(
        WorldId::new(8),
        owner().dimension(),
        uuid(2),
        EntityId::new(2),
        EntityTypeId::from_name("item").unwrap(),
        EntityTransform::try_new(Vec3::new(-0.5, 64.0, 1.0), 0.0, 0.0).unwrap(),
    );
    let duplicate_wire = EntitySpawn::new(
        owner().world(),
        owner().dimension(),
        uuid(2),
        EntityId::new(1),
        EntityTypeId::from_name("item").unwrap(),
        EntityTransform::try_new(Vec3::new(-0.5, 64.0, 1.0), 0.0, 0.0).unwrap(),
    );
    let cases = [
        (
            wrong_scope,
            ItemEntityError::Store(EntityStoreError::ScopeMismatch {
                owner: owner(),
                world: WorldId::new(8),
                dimension: owner().dimension(),
            }),
        ),
        (
            base(2, 0.5, "item"),
            ItemEntityError::Store(EntityStoreError::OutsideOwner {
                owner: owner(),
                chunk: ChunkPos::new(0, 0),
            }),
        ),
        (
            base(2, -33.0, "item"),
            ItemEntityError::Store(EntityStoreError::ChunkNotResident {
                chunk: ChunkPos::new(-3, 0),
            }),
        ),
        (
            base(2, -0.5, "cow"),
            ItemEntityError::WrongEntityType {
                actual: EntityTypeId::from_name("cow").unwrap(),
            },
        ),
        (
            base(1, -0.5, "item"),
            ItemEntityError::Store(EntityStoreError::DuplicateUuid { uuid: uuid(1) }),
        ),
        (
            duplicate_wire,
            ItemEntityError::Store(EntityStoreError::DuplicateWireId {
                wire_id: EntityId::new(1),
            }),
        ),
    ];
    shard
        .run_tick_with_entity_work(|ctx| {
            for (request, reason) in cases {
                let refusal = ctx
                    .spawn_item(ItemEntitySpawn::new(request, stone(9), props()))
                    .unwrap_err();
                assert_eq!(refusal.reason(), &reason);
                let (_, returned) = refusal.into_parts();
                assert_eq!(returned.base(), request);
                assert_eq!(returned.stack().count(), 9);
                assert_eq!(
                    returned.properties().motion().x.to_bits(),
                    (-0.0_f64).to_bits()
                );
                assert_eq!(returned.properties().owner_target(), Some(uuid(0)));
                assert_eq!(returned.properties().thrower(), Some(uuid(99)));
                assert_eq!(ctx.entities().capture_item(&handle).unwrap(), before);
                assert_eq!(ctx.entities().item_bytes_used(), used);
                assert_eq!(ctx.entities().handle_for_uuid(uuid(1)).unwrap(), handle);
                assert_eq!(
                    ctx.entities().handle_for_wire_id(EntityId::new(1)).unwrap(),
                    handle
                );
                assert_eq!(
                    ctx.entities().in_chunk(ChunkPos::new(-1, 0)).unwrap().len(),
                    1
                );
            }
        })
        .unwrap();
}

#[tokio::test]
async fn invalid_stack_and_slot_full_spawn_refusals_preserve_input_and_existing_payload() {
    let mut shard = shard(3, 2048).await;
    let handle = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(
                base(1, -0.5, "item"),
                stone(2),
                props(),
            ))
            .unwrap()
        })
        .unwrap()
        .0;
    let before = shard.entities().unwrap().capture_item(&handle).unwrap();
    shard
        .run_tick_with_entity_work(|ctx| {
            let bad = ItemStack::new(
                ItemId::from_name("stone").unwrap(),
                NonZeroU8::new(65).unwrap(),
                ComponentPatch::empty(),
            );
            let refused = ctx
                .spawn_item(ItemEntitySpawn::new(base(2, -0.5, "item"), bad, props()))
                .unwrap_err();
            assert!(matches!(refused.reason(), ItemEntityError::Stack(_)));
            assert_eq!(refused.into_parts().1.stack().count(), 65);
            for id in [2, 3] {
                ctx.spawn_item(ItemEntitySpawn::new(
                    base(id, -0.5, "item"),
                    stone(1),
                    props(),
                ))
                .unwrap();
            }
            let refused = ctx
                .spawn_item(ItemEntitySpawn::new(
                    base(4, -0.5, "item"),
                    stone(8),
                    props(),
                ))
                .unwrap_err();
            assert_eq!(
                refused.reason(),
                &ItemEntityError::Store(EntityStoreError::Full { capacity: 3 })
            );
            assert_eq!(refused.into_parts().1.stack().count(), 8);
            assert_eq!(ctx.entities().len(), 3);
            assert_eq!(ctx.entities().capture_item(&handle).unwrap(), before);
        })
        .unwrap();
}

#[tokio::test]
async fn restore_refusals_keep_uuid_scope_stack_properties_and_supplied_wire_value() {
    let expected = stack_fixture();
    let (key, row) = native_fixture(&expected);
    let mut shard = shard(2, 2048).await;
    let resident = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn_item(ItemEntitySpawn::new(
                base(2, -0.5, "item"),
                stone(3),
                props(),
            ))
            .unwrap()
        })
        .unwrap()
        .0;
    let before = shard.entities().unwrap().capture_item(&resident).unwrap();
    shard
        .run_tick_with_entity_work(|ctx| {
            let refusal = ctx
                .restore_item(decode_item_entity(key, &row).unwrap(), EntityId::new(2))
                .unwrap_err();
            assert_eq!(
                refusal.reason(),
                &ItemEntityError::Store(EntityStoreError::DuplicateWireId {
                    wire_id: EntityId::new(2)
                })
            );
            let (_, proposal, wire) = refusal.into_parts();
            assert_eq!(wire, EntityId::new(2));
            assert_eq!(proposal.key(), key);
            assert_eq!(
                encode_persisted_stack(proposal.item_state().stack()).unwrap(),
                expected
            );
            assert_eq!(proposal.item_state().properties().counters().health(), -5);
            for (x, chunk) in [
                (0.5_f64, ChunkPos::new(0, 0)),
                (-33.5, ChunkPos::new(-3, 0)),
            ] {
                let mut bytes = row.record().data().to_vec();
                bytes[34..42].copy_from_slice(&x.to_bits().to_be_bytes());
                let different_column = record(bytes, chunk, 1);
                let proposed = decode_item_entity(key, &different_column).unwrap();
                let refused = ctx.restore_item(proposed, EntityId::new(-987)).unwrap_err();
                assert!(matches!(
                    refused.reason(),
                    ItemEntityError::Store(
                        EntityStoreError::OutsideOwner { .. }
                            | EntityStoreError::ChunkNotResident { .. }
                    )
                ));
                let (_, proposal, wire) = refused.into_parts();
                assert_eq!(wire, EntityId::new(-987));
                assert_eq!(proposal.transform().chunk(), chunk);
                assert_eq!(
                    encode_persisted_stack(proposal.item_state().stack()).unwrap(),
                    expected
                );
            }
            let mut body = row.record().data().to_vec();
            body[10..14].copy_from_slice(&8_u32.to_be_bytes());
            let other_key = PersistentEntityKey::new(WorldId::new(8), key.dimension(), key.uuid());
            let proposal = decode_item_entity(other_key, &record(body, row.chunk(), 1)).unwrap();
            let refused = ctx.restore_item(proposal, EntityId::new(3)).unwrap_err();
            assert!(matches!(
                refused.reason(),
                ItemEntityError::Store(EntityStoreError::ScopeMismatch { .. })
            ));
            let (_, proposal, wire) = refused.into_parts();
            assert_eq!(wire, EntityId::new(3));
            assert_eq!(proposal.key(), other_key);
            assert_eq!(
                encode_persisted_stack(proposal.item_state().stack()).unwrap(),
                expected
            );
            let accepted = ctx
                .restore_item(decode_item_entity(key, &row).unwrap(), EntityId::new(3))
                .unwrap();
            let refused = ctx
                .restore_item(decode_item_entity(key, &row).unwrap(), EntityId::new(4))
                .unwrap_err();
            assert!(matches!(
                refused.reason(),
                ItemEntityError::Store(EntityStoreError::DuplicateUuid { .. })
            ));
            assert_eq!(ctx.entities().capture_item(&resident).unwrap(), before);
            assert_eq!(
                ctx.entities().capture_item(&accepted).unwrap(),
                (key, row.clone())
            );
        })
        .unwrap();
}

#[tokio::test]
async fn restore_budget_refusal_preserves_complete_proposal_and_supplied_wire_value() {
    let expected = stack_fixture();
    let (key, row) = native_fixture(&expected);
    let mut too_small = shard(1, 1).await;
    too_small
        .run_tick_with_entity_work(|ctx| {
            let refused = ctx
                .restore_item(decode_item_entity(key, &row).unwrap(), EntityId::new(5))
                .unwrap_err();
            assert!(matches!(
                refused.reason(),
                ItemEntityError::ItemByteBudgetExceeded { limit: 1, .. }
            ));
            let (_, proposal, wire) = refused.into_parts();
            assert_eq!(wire, EntityId::new(5));
            assert_eq!(
                encode_persisted_stack(proposal.item_state().stack()).unwrap(),
                expected
            );
            assert_eq!(ctx.entities().len(), 0);
        })
        .unwrap();
}
