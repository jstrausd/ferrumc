use super::*;
use crate::LoadedChunkMap;
use ferrumc_core::{DimensionId, WorldId};
use ferrumc_items::{ComponentPatch, ItemId};
use ferrumc_math::ShardPos;

fn owned_fixture() -> (EntityStore, EntityHandle) {
    let owner =
        ShardId::try_new(WorldId::new(7), DimensionId::new(13), ShardPos::new(0, 0)).unwrap();
    let mut store = EntityStore::with_item_byte_budget(
        owner,
        NonZeroUsize::new(2).unwrap(),
        NonZeroUsize::new(4096).unwrap(),
    )
    .unwrap();
    let base = EntitySpawn::new(
        owner.world(),
        owner.dimension(),
        EntityUuid::from_bytes([1; 16]),
        EntityId::new(1),
        EntityTypeId::from_name("item").unwrap(),
        EntityTransform::try_new(Vec3::new(1.0, 64.0, 1.0), -0.0, -0.0).unwrap(),
    );
    let stack = ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        3,
        ComponentPatch::empty(),
    )
    .unwrap();
    let bytes = checked_stack_bytes(&stack).unwrap();
    let properties = ItemEntityProperties::try_new(
        Vec3::new(-0.0, 11.0, 0.0),
        -0.0,
        ItemEntityCounters::new(i32::MAX, 32768, -3),
        Some(EntityUuid::from_bytes([0; 16])),
        None,
    )
    .unwrap();
    let item = ItemEntityState {
        stack,
        properties,
        fcis_len: bytes.len(),
    };
    let bucket = store.validate_spawn(base).unwrap();
    store.reserve_spawn_slot().unwrap();
    let handle = store.commit_spawn(base, Some(item), bucket);
    store.item_bytes_used = NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len();
    (store, handle)
}

#[test]
fn cached_length_corruption_refuses_snapshot_capture_and_infallible_clone_before_copy() {
    let (mut store, handle) = owned_fixture();
    let index = store.index(&handle).unwrap();
    store.slots[index].item.as_mut().unwrap().fcis_len += 1;
    assert!(matches!(
        store.snapshot_item(&handle),
        Err(ItemEntityError::CachedItemLengthMismatch { .. })
    ));
    assert!(matches!(
        store.capture_item(&handle),
        Err(ItemEntityError::CachedItemLengthMismatch { .. })
    ));
    assert!(matches!(
        store.try_snapshot(),
        Err(ItemEntityError::CachedItemLengthMismatch { .. })
    ));
    assert!(std::panic::catch_unwind(|| store.clone()).is_err());
    assert!(store.get(&handle).is_ok());
}

#[test]
fn aggregate_corruption_refuses_checked_and_infallible_whole_store_snapshot() {
    let (mut store, handle) = owned_fixture();
    let actual = store.item_bytes_used;
    store.item_bytes_used -= 1;
    assert!(
        matches!(store.try_snapshot(), Err(ItemEntityError::AggregateItemBytesMismatch { cached, actual: measured }) if cached == actual-1 && measured == actual)
    );
    assert!(std::panic::catch_unwind(|| store.clone()).is_err());
    assert!(matches!(
        store.snapshot_item(&handle),
        Err(ItemEntityError::AggregateItemBytesMismatch { .. })
    ));
    assert!(matches!(
        store.capture_item(&handle),
        Err(ItemEntityError::AggregateItemBytesMismatch { .. })
    ));
    assert!(store.get(&handle).is_ok());
}

#[test]
fn legacy_guards_precede_every_detach_or_generation_change() {
    let (mut store, handle) = owned_fixture();
    let before = store.capture_item(&handle).unwrap();
    let used = store.item_bytes_used;
    assert_eq!(
        store.remove(&handle),
        Err(EntityStoreError::PayloadRequiresOwnedRemove)
    );
    assert_eq!(
        store.update_transform(
            &handle,
            EntityTransform::try_new(Vec3::new(20.0, 64.0, 1.0), 0.0, 0.0).unwrap()
        ),
        Err(EntityStoreError::ItemMutationRequiresTickContext)
    );
    assert_eq!(store.capture_item(&handle).unwrap(), before);
    assert_eq!(store.item_bytes_used, used);
    assert_eq!(store.handle_for_uuid(before.0.uuid()).unwrap(), handle);
}

#[test]
fn owned_cleanup_at_generation_max_retires_and_returns_payload_without_residency() {
    let (mut store, handle) = owned_fixture();
    let index = store.index(&handle).unwrap();
    let expected = encode_persisted_stack(store.item_state(&handle).unwrap().stack()).unwrap();
    store.slots[index].generation = u64::MAX;
    let max_handle = store.handle(index);
    let map = LoadedChunkMap::new_legacy(store.owner.world(), store.owner.dimension());
    let removed = EntityTickContext::new(&mut store, &map)
        .remove_owned(&max_handle)
        .unwrap();
    assert_eq!(
        encode_persisted_stack(removed.item_state().unwrap().stack()).unwrap(),
        expected
    );
    assert_eq!(store.retired, 1);
    assert_eq!(store.item_bytes_used, 0);
    assert!(store.slots[index].item.is_none());
    assert!(store.free.is_none());
    assert!(matches!(
        store.item_state(&max_handle),
        Err(ItemEntityError::Store(EntityStoreError::StaleHandle))
    ));
    assert!(store.by_uuid.is_empty() && store.by_wire.is_empty());
}

#[test]
fn private_nonfinite_props_and_bad_stack_are_detected_before_snapshot() {
    let (mut store, handle) = owned_fixture();
    let index = store.index(&handle).unwrap();
    store.slots[index]
        .item
        .as_mut()
        .unwrap()
        .properties
        .motion
        .x = f64::NAN;
    assert!(matches!(
        store.try_snapshot(),
        Err(ItemEntityError::NonFiniteProperty {
            component: ItemPropertyComponent::MotionX
        })
    ));
    store.slots[index]
        .item
        .as_mut()
        .unwrap()
        .properties
        .motion
        .x = -0.0;
    store.slots[index].item.as_mut().unwrap().stack = ItemStack::new(
        ItemId::from_name("stone").unwrap(),
        std::num::NonZeroU8::new(65).unwrap(),
        ComponentPatch::empty(),
    );
    assert!(matches!(
        store.snapshot_item(&handle),
        Err(ItemEntityError::Stack(_))
    ));
    assert!(matches!(
        store.capture_item(&handle),
        Err(ItemEntityError::Stack(_))
    ));
    assert!(matches!(
        store.try_snapshot(),
        Err(ItemEntityError::Stack(_))
    ));
}

#[test]
fn free_slot_payload_and_corrupt_base_fail_checked_snapshot() {
    let (mut store, handle) = owned_fixture();
    let index = store.index(&handle).unwrap();
    store.slots[index].entity = None;
    assert!(matches!(
        store.try_snapshot(),
        Err(ItemEntityError::Store(EntityStoreError::StaleHandle))
    ));
}
