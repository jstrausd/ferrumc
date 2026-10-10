use std::collections::BTreeMap;
use std::num::NonZeroU8;

use ferrumc_nbt::{NbtCompound, NbtError, NbtTag};

use super::*;
use crate::component::{ComponentTypeId, OpaqueComponent, DAMAGE, MAX_STACK_SIZE};
use crate::{
    encode_persisted_stack, ItemId, ItemValidationError, MAX_COMPONENTS,
    MAX_COMPONENTS_TOTAL_BYTES, MAX_COMPONENT_BYTES,
};

fn stack(name: &str, count: u8, components: ComponentPatch) -> ItemStack {
    ItemStack::try_new(ItemId::from_name(name).unwrap(), count, components).unwrap()
}

fn stone(count: u8) -> ItemStack {
    if count == 0 {
        ItemStack::empty()
    } else {
        stack("stone", count, ComponentPatch::empty())
    }
}

fn encoded(stack: &ItemStack) -> Vec<u8> {
    encode_persisted_stack(stack).unwrap()
}

// The oracle totals by exact persistence representation with only the count
// field normalized; it does not call the transfer's compatibility comparison.
fn quantities(stacks: &[&ItemStack]) -> BTreeMap<Vec<u8>, u32> {
    let mut totals = BTreeMap::new();
    for stack in stacks {
        if let Some(item) = stack.item() {
            let mut identity = encoded(stack);
            identity[9 + item.name().len()] = 1;
            *totals.entry(identity).or_default() += u32::from(stack.count());
        }
    }
    totals
}

fn assert_conserved(source: &ItemStack, destination: &ItemStack, plan: &StackTransferPlan) {
    assert_eq!(
        quantities(&[source, destination]),
        quantities(&[plan.source_after(), plan.destination_after()])
    );
    assert_eq!(source.count() - plan.source_after().count(), plan.moved());
    assert_eq!(
        plan.destination_after().count() - destination.count(),
        plan.moved()
    );
    assert!(plan.source_after().is_valid());
    assert!(plan.destination_after().is_valid());
    for output in [plan.source_after(), plan.destination_after()] {
        if output.item().is_none() {
            assert_eq!(output.count(), 0);
            assert!(output.components().is_empty());
        }
    }
}

#[test]
fn split_covers_every_stone_count_and_u8_request_without_input_mutation() {
    for count in 0..=64 {
        let source = stone(count);
        let before = encoded(&source);
        for requested in 0..=u8::MAX {
            let plan = plan_split(&source, requested).unwrap();
            assert_eq!(plan.moved(), requested.min(count));
            assert_eq!(plan.source_after().count(), count - plan.moved());
            assert_eq!(plan.destination_after().count(), plan.moved());
            assert_conserved(&source, &ItemStack::empty(), &plan);
            assert_eq!(encoded(&source), before);
        }
    }
}

#[test]
fn transfer_boundary_matrix_conserves_and_never_clamps_existing_destination() {
    for source_count in [0, 1, 2, 32, 63, 64] {
        for destination_count in [0, 1, 2, 32, 63, 64] {
            for capacity in [0, 1, 2, 16, 63, 64, 65, 99, u8::MAX] {
                let source = stone(source_count);
                let destination = stone(destination_count);
                let before = [encoded(&source), encoded(&destination)];
                for requested in 0..=u8::MAX {
                    let result = plan_transfer(&source, &destination, requested, capacity);
                    if destination_count > capacity {
                        assert_eq!(
                            result.unwrap_err(),
                            StackTransferError::DestinationOverCapacity {
                                count: destination_count,
                                capacity,
                            }
                        );
                    } else {
                        let plan = result.unwrap();
                        let space = u16::from(capacity.min(64)) - u16::from(destination_count);
                        let expected = u16::from(source_count).min(u16::from(requested)).min(space);
                        assert_eq!(u16::from(plan.moved()), expected);
                        assert_conserved(&source, &destination, &plan);
                        assert!(plan.destination_after().count() <= capacity.min(64));
                    }
                    assert_eq!([encoded(&source), encoded(&destination)], before);
                }
            }
        }
    }
}

#[test]
fn every_registered_item_keeps_its_ceiling_even_with_u8_max_capacity() {
    for id in 0..ferrumc_registry::item::ITEM_COUNT {
        let item = ItemId::new(i32::try_from(id).unwrap()).unwrap();
        let source = ItemStack::try_new(item, item.max_stack(), ComponentPatch::empty()).unwrap();
        let destination = ItemStack::try_new(item, 1, ComponentPatch::empty()).unwrap();
        let plan = plan_transfer(&source, &destination, u8::MAX, u8::MAX).unwrap();
        assert_eq!(plan.destination_after().count(), item.max_stack());
        assert_eq!(plan.source_after().count(), 1);
        assert_eq!(plan.moved(), item.max_stack() - 1);
        assert_conserved(&source, &destination, &plan);

        let split = plan_split(&source, u8::MAX).unwrap();
        assert_eq!(split.destination_after().count(), item.max_stack());
        assert!(split.source_after().item().is_none());
    }
}

#[test]
fn incompatible_items_components_and_representation_never_swap_or_overwrite() {
    let source = stack(
        "stone",
        12,
        ComponentPatch::new(vec![ComponentValue::Damage(-3)], vec![]),
    );
    let destinations = [
        stack("glass", 10, source.components().clone()),
        stone(10),
        stack(
            "stone",
            10,
            ComponentPatch::new(vec![ComponentValue::Damage(-2)], vec![]),
        ),
        // Same wire id/value, different authoritative representation.
        stack(
            "stone",
            10,
            ComponentPatch::new(
                vec![ComponentValue::Opaque(
                    OpaqueComponent::new(DAMAGE, vec![0xfd, 0xff, 0xff, 0xff, 0x0f]).unwrap(),
                )],
                vec![],
            ),
        ),
    ];
    for destination in destinations {
        let plan = plan_transfer(&source, &destination, 12, 64).unwrap();
        assert_eq!(plan.moved(), 0);
        assert_eq!(encoded(plan.source_after()), encoded(&source));
        assert_eq!(encoded(plan.destination_after()), encoded(&destination));
        assert_conserved(&source, &destination, &plan);
    }
}

fn rich_patch() -> ComponentPatch {
    let mut data = NbtCompound::new();
    data.push("same", NbtTag::Float(f32::from_bits(0x7fc0_0042)));
    data.push(
        "same",
        NbtTag::Double(f64::from_bits(0xfff8_0000_0000_0042)),
    );
    data.push("zero", NbtTag::Float(-0.0));
    data.push("other_zero", NbtTag::Double(-0.0));
    data.push("nul", NbtTag::String("\0💎".to_owned()));
    data.push("array", NbtTag::ByteArray(vec![-128, 0, 127]));
    ComponentPatch::new(
        vec![
            ComponentValue::CustomData(NbtTag::Compound(data)),
            ComponentValue::Opaque(OpaqueComponent::new(42, vec![0xff, 0x00, 0x80]).unwrap()),
            ComponentValue::Damage(i32::MIN),
            ComponentValue::MaxDamage(i32::MAX),
            ComponentValue::Unbreakable,
        ],
        vec![ComponentTypeId::new(51), ComponentTypeId::new(66)],
    )
}

#[test]
fn split_and_merge_retain_nan_bits_duplicates_opaque_bytes_and_signed_values() {
    let source = stack("stone", 17, rich_patch());
    let before = encoded(&source);
    let split = plan_split(&source, 6).unwrap();
    assert_eq!(split.moved(), 6);
    assert_conserved(&source, &ItemStack::empty(), &split);
    let merged =
        plan_transfer(split.destination_after(), split.source_after(), u8::MAX, 64).unwrap();
    assert_eq!(merged.moved(), 6, "identical NaN bits must merge");
    assert!(merged.source_after().item().is_none());
    assert_eq!(encoded(merged.destination_after()), before);
    assert_eq!(encoded(&source), before);
    let (remainder, result) = merged.into_stacks();
    assert!(remainder.item().is_none());
    assert_eq!(encoded(&result), before);
}

fn nbt_stack(value: NbtTag) -> ItemStack {
    let mut data = NbtCompound::new();
    data.push("value", value);
    stack(
        "stone",
        3,
        ComponentPatch::new(
            vec![ComponentValue::CustomData(NbtTag::Compound(data))],
            vec![],
        ),
    )
}

#[test]
fn float_representation_comparison_is_recursive_and_bit_exact() {
    let cases = [
        (NbtTag::Float(0.0), NbtTag::Float(-0.0)),
        (NbtTag::Double(0.0), NbtTag::Double(-0.0)),
        (
            NbtTag::Float(f32::from_bits(0x7fc0_0001)),
            NbtTag::Float(f32::from_bits(0x7fc0_0002)),
        ),
        (
            NbtTag::Double(f64::from_bits(0x7ff8_0000_0000_0001)),
            NbtTag::Double(f64::from_bits(0x7ff8_0000_0000_0002)),
        ),
    ];
    for (left, right) in cases {
        // Descendants of both a list and a compound need the bit comparison.
        let source = nbt_stack(NbtTag::List(vec![left]));
        let destination = nbt_stack(NbtTag::List(vec![right]));
        let plan = plan_transfer(&source, &destination, 3, 64).unwrap();
        assert_eq!(plan.moved(), 0);
        assert_conserved(&source, &destination, &plan);
        let identical = plan_transfer(&source, &source, 3, 64).unwrap();
        assert_eq!(identical.moved(), 3);
        assert_conserved(&source, &source, &identical);
    }
}

#[test]
fn patch_order_removal_order_defaults_and_nbt_key_order_are_not_normalized() {
    let added = vec![ComponentValue::Damage(1), ComponentValue::Unbreakable];
    let removed = vec![ComponentTypeId::new(6), ComponentTypeId::new(7)];
    let source = stack(
        "stone",
        5,
        ComponentPatch::new(added.clone(), removed.clone()),
    );
    let cases = [
        ComponentPatch::new(added.into_iter().rev().collect(), removed.clone()),
        ComponentPatch::new(
            source.components().added().to_vec(),
            removed.into_iter().rev().collect(),
        ),
    ];
    for patch in cases {
        let destination = stack("stone", 4, patch);
        let plan = plan_transfer(&source, &destination, 5, 64).unwrap();
        assert_eq!(plan.moved(), 0);
        assert_conserved(&source, &destination, &plan);
    }
    let explicit_default = stack(
        "stone",
        4,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(64)], vec![]),
    );
    assert_eq!(
        plan_transfer(&stone(5), &explicit_default, 5, 64)
            .unwrap()
            .moved(),
        0
    );

    let mut left = NbtCompound::new();
    left.push("a", NbtTag::Int(1));
    left.push("b", NbtTag::Int(2));
    let mut right = NbtCompound::new();
    right.push("b", NbtTag::Int(2));
    right.push("a", NbtTag::Int(1));
    let source = nbt_stack(NbtTag::Compound(left));
    let destination = nbt_stack(NbtTag::Compound(right));
    let plan = plan_transfer(&source, &destination, 3, 64).unwrap();
    assert_eq!(plan.moved(), 0);
    assert_conserved(&source, &destination, &plan);
}

fn assert_invalid_source_and_destination(stack: &ItemStack, error: &ItemPersistenceError) {
    let original = format!("{stack:?}");
    let valid = stone(1);
    for requested in [0, 1, u8::MAX] {
        assert_eq!(
            plan_split(stack, requested).unwrap_err(),
            StackTransferError::InvalidSource(error.clone())
        );
        assert_eq!(
            plan_transfer(stack, &valid, requested, 64).unwrap_err(),
            StackTransferError::InvalidSource(error.clone())
        );
        // The different source item cannot hide an invalid destination.
        let incompatible = stack_for_incompatible_source();
        assert_eq!(
            plan_transfer(&incompatible, stack, requested, 64).unwrap_err(),
            StackTransferError::InvalidDestination(error.clone())
        );
        assert_eq!(
            plan_transfer(&ItemStack::empty(), stack, requested, 64).unwrap_err(),
            StackTransferError::InvalidDestination(error.clone())
        );
        assert_eq!(format!("{stack:?}"), original);
        assert_eq!(valid.count(), 1);
    }
}

fn stack_for_incompatible_source() -> ItemStack {
    stack("glass", 1, ComponentPatch::empty())
}

#[test]
fn invalid_authoritative_counts_fail_even_for_noop_or_incompatible_transfers() {
    for (name, count) in [("stone", 65), ("stone", u8::MAX), ("diamond_sword", 2)] {
        let item = ItemId::from_name(name).unwrap();
        let source = ItemStack::new(
            item,
            NonZeroU8::new(count).unwrap(),
            ComponentPatch::empty(),
        );
        let error = ItemPersistenceError::InvalidStack(ItemValidationError::StackCountOutOfRange {
            item_id: item.id(),
            count,
            max: item.max_stack(),
        });
        assert_invalid_source_and_destination(&source, &error);
    }
}

#[test]
fn invalid_duplicate_conflicting_out_of_range_and_typed_patches_fail_unchanged() {
    let patches = [
        (
            ComponentPatch::new(
                vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
                vec![],
            ),
            ItemPersistenceError::DuplicateComponent(DAMAGE),
        ),
        (
            ComponentPatch::new(
                vec![ComponentValue::Damage(1)],
                vec![ComponentTypeId::new(DAMAGE)],
            ),
            ItemPersistenceError::DuplicateComponent(DAMAGE),
        ),
        (
            ComponentPatch::new(
                vec![],
                vec![ComponentTypeId::new(6), ComponentTypeId::new(6)],
            ),
            ItemPersistenceError::DuplicateComponent(6),
        ),
        (
            ComponentPatch::new(vec![], vec![ComponentTypeId::new(-1)]),
            ItemPersistenceError::InvalidComponentType(-1),
        ),
        (
            ComponentPatch::new(vec![], vec![ComponentTypeId::new(96)]),
            ItemPersistenceError::InvalidComponentType(96),
        ),
        (
            ComponentPatch::new(vec![ComponentValue::MaxStackSize(0)], vec![]),
            ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE),
        ),
        (
            ComponentPatch::new(vec![ComponentValue::MaxStackSize(100)], vec![]),
            ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE),
        ),
    ];
    for (patch, error) in patches {
        assert_invalid_source_and_destination(&stack("stone", 1, patch), &error);
    }
}

#[test]
fn component_count_and_payload_bounds_are_checked_before_planning() {
    let too_many = stack(
        "stone",
        1,
        ComponentPatch::new(vec![], (0..=64).map(ComponentTypeId::new).collect()),
    );
    assert_invalid_source_and_destination(
        &too_many,
        &ItemPersistenceError::InvalidStack(ItemValidationError::TooManyComponents {
            count: MAX_COMPONENTS + 1,
            max: MAX_COMPONENTS,
        }),
    );
    let mut data = NbtCompound::new();
    data.push("long", NbtTag::String("x".repeat(MAX_COMPONENT_BYTES + 1)));
    let source = nbt_stack(NbtTag::Compound(data));
    let error = encode_persisted_stack(&source).unwrap_err();
    assert!(matches!(
        error,
        ItemPersistenceError::Nbt(NbtError::StringTooLong { .. })
    ));
    assert_invalid_source_and_destination(&source, &error);

    let maximum_opaque = stack(
        "stone",
        64,
        ComponentPatch::new(
            vec![ComponentValue::Opaque(
                OpaqueComponent::new(42, vec![0xaa; MAX_COMPONENT_BYTES]).unwrap(),
            )],
            vec![],
        ),
    );
    let plan = plan_split(&maximum_opaque, 32).unwrap();
    assert_conserved(&maximum_opaque, &ItemStack::empty(), &plan);
}

#[test]
fn combined_payload_boundary_and_maximum_entry_count_are_checked() {
    let safe_ids: Vec<_> = (0..=95)
        .filter(|id| !ComponentTypeId::new(*id).is_dangerous())
        .collect();
    let maximum_added: Vec<_> = safe_ids
        .iter()
        .take(MAX_COMPONENTS_TOTAL_BYTES / MAX_COMPONENT_BYTES)
        .map(|id| {
            ComponentValue::Opaque(
                OpaqueComponent::new(*id, vec![0xab; MAX_COMPONENT_BYTES]).unwrap(),
            )
        })
        .collect();
    let maximum = stack(
        "stone",
        2,
        ComponentPatch::new(maximum_added.clone(), vec![]),
    );
    let plan = plan_split(&maximum, 1).unwrap();
    assert_conserved(&maximum, &ItemStack::empty(), &plan);

    let mut exceeded = maximum_added;
    exceeded.push(ComponentValue::Opaque(
        OpaqueComponent::new(safe_ids[exceeded.len()], vec![0xcd]).unwrap(),
    ));
    let exceeded = stack("stone", 2, ComponentPatch::new(exceeded, vec![]));
    assert_invalid_source_and_destination(
        &exceeded,
        &ItemPersistenceError::InvalidStack(ItemValidationError::ComponentsTooLarge {
            total: MAX_COMPONENTS_TOTAL_BYTES + 1,
            max: MAX_COMPONENTS_TOTAL_BYTES,
        }),
    );

    let maximum_removals = stack(
        "stone",
        2,
        ComponentPatch::new(vec![], (0..64).map(ComponentTypeId::new).collect()),
    );
    let plan = plan_transfer(&maximum_removals, &ItemStack::empty(), 1, 1).unwrap();
    assert_conserved(&maximum_removals, &ItemStack::empty(), &plan);
}

#[test]
fn malformed_nbt_and_depth_budget_fail_without_outputs() {
    let heterogeneous = nbt_stack(NbtTag::List(vec![NbtTag::Int(1), NbtTag::Byte(1)]));
    let error = ItemPersistenceError::Nbt(NbtError::HeterogeneousList {
        expected: 3,
        found: 1,
    });
    assert_invalid_source_and_destination(&heterogeneous, &error);
    let mut nested = NbtTag::Compound(NbtCompound::new());
    for _ in 0..=ferrumc_nbt::NbtLimits::default().max_depth() {
        let mut parent = NbtCompound::new();
        parent.push("next", nested);
        nested = NbtTag::Compound(parent);
    }
    let source = nbt_stack(nested);
    let error = encode_persisted_stack(&source).unwrap_err();
    assert!(matches!(
        error,
        ItemPersistenceError::Nbt(NbtError::DepthExceeded { .. })
    ));
    assert_invalid_source_and_destination(&source, &error);
}

#[test]
fn supplied_capacity_controls_lower_component_policy_without_changing_patch() {
    let patch = ComponentPatch::new(vec![ComponentValue::MaxStackSize(4)], vec![]);
    let source = stack("stone", 3, patch.clone());
    let destination = stack("stone", 2, patch);
    let plan = plan_transfer(&source, &destination, 3, 4).unwrap();
    assert_eq!(plan.moved(), 2);
    assert_eq!(plan.source_after().count(), 1);
    assert_eq!(plan.destination_after().count(), 4);
    assert_conserved(&source, &destination, &plan);
}
