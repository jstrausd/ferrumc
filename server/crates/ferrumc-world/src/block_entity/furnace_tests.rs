use super::*;
use crate::{decode_block_entity, try_encode_block_entity, BlockEntityCodecError};
use ferrumc_items::{
    encode_persisted_stack, plan_furnace_tick, ComponentPatch, ComponentTypeId, ComponentValue,
    ItemId, OpaqueComponent,
};
use ferrumc_nbt::{NbtCompound, NbtTag};
use ferrumc_registry::block_state::block_default_state;

fn stack(name: &str, count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}

fn encoded(furnace: FurnaceInventory) -> Vec<u8> {
    let mut out = Vec::new();
    try_encode_block_entity(&BlockEntity::Furnace(furnace), &mut out).unwrap();
    out
}

fn parsed(bytes: &[u8]) -> FurnaceInventory {
    let BlockEntity::Furnace(furnace) = decode_block_entity(bytes).unwrap() else {
        panic!("furnace kind changed");
    };
    furnace
}

#[test]
fn ordinary_furnace_classifier_and_defaults_are_closed() {
    let fresh = FurnaceInventory::new();
    assert_eq!(fresh.slots().len(), 3);
    assert!(fresh.slots().iter().all(|slot| slot.item().is_none()));
    assert_eq!(
        (
            fresh.burn_remaining(),
            fresh.burn_duration(),
            fresh.cook_progress(),
            fresh.cook_total()
        ),
        (0, 0, 0, 200)
    );
    assert!(fresh.slot(3).is_none());
    assert!(is_furnace_state(block_default_state("furnace").unwrap()));
    for name in ["blast_furnace", "smoker", "stone"] {
        assert!(!is_furnace_state(block_default_state(name).unwrap()));
    }
    assert!(!is_furnace_state(u32::MAX));
}

#[test]
fn empty_versioned_furnace_matches_independent_fixed_grammar() {
    let mut fixture = vec![0x7f, 0, 1, 2];
    for _ in 0..3 {
        fixture.extend_from_slice(b"\0\0\0\x07FCIS\0\x01\0");
    }
    fixture.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 200]);
    assert_eq!(encoded(FurnaceInventory::new()), fixture);
    assert_eq!(parsed(&fixture), FurnaceInventory::new());
}

#[test]
fn furnace_storage_preserves_component_order_nbt_bits_and_all_four_timers() {
    let mut custom = NbtCompound::new();
    custom.push("same", NbtTag::Float(f32::from_bits(0x7fc0_1234)));
    custom.push("same", NbtTag::Double(-0.0));
    let input = ItemStack::try_new(
        ItemId::from_name("raw_iron").unwrap(),
        1,
        ComponentPatch::new(
            vec![
                ComponentValue::Damage(-7),
                ComponentValue::CustomData(NbtTag::Compound(custom)),
                ComponentValue::Opaque(OpaqueComponent::new(6, vec![0xff, 7]).unwrap()),
            ],
            vec![ComponentTypeId::new(95), ComponentTypeId::new(51)],
        ),
    )
    .unwrap();
    let furnace = FurnaceInventory::try_from_parts(
        [input, stack("bucket", 1), stack("iron_ingot", 63)],
        19801,
        20000,
        199,
        200,
    )
    .unwrap();
    let bytes = encoded(furnace);
    let restored = parsed(&bytes);
    assert_eq!(
        (
            restored.burn_remaining(),
            restored.burn_duration(),
            restored.cook_progress(),
            restored.cook_total()
        ),
        (19801, 20000, 199, 200)
    );
    assert_eq!(
        restored.slots()[0].components().removed(),
        &[ComponentTypeId::new(95), ComponentTypeId::new(51)]
    );
    assert_eq!(encoded(restored), bytes);
}

#[test]
fn checked_replacement_is_atomic_for_bad_timers_capacity_and_total_budget() {
    let mut furnace = FurnaceInventory::try_from_parts(
        [stack("raw_iron", 2), stack("coal", 2), ItemStack::empty()],
        1000,
        1600,
        77,
        200,
    )
    .unwrap();
    let before = encoded(furnace.clone());
    assert!(furnace
        .try_replace(furnace.slots().clone(), 1601, 1600, 77, 200)
        .is_err());
    assert_eq!(encoded(furnace.clone()), before);
    let invalid = ItemStack::try_new(
        ItemId::from_name("raw_iron").unwrap(),
        2,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(1)], vec![]),
    )
    .unwrap();
    assert!(furnace
        .try_replace(
            [invalid, stack("coal", 2), ItemStack::empty()],
            1000,
            1600,
            77,
            200
        )
        .is_err());
    assert_eq!(encoded(furnace.clone()), before);
    let patch = ComponentPatch::new(
        vec![ComponentValue::Opaque(
            OpaqueComponent::new(6, vec![0; 8192]).unwrap(),
        )],
        vec![],
    );
    let large = [
        ItemStack::try_new(ItemId::from_name("raw_iron").unwrap(), 1, patch.clone()).unwrap(),
        ItemStack::try_new(ItemId::from_name("coal").unwrap(), 1, patch).unwrap(),
        ItemStack::empty(),
    ];
    assert!(furnace.try_replace(large, 0, 0, 0, 200).is_err());
    assert_eq!(encoded(furnace), before);
}

#[test]
fn decoder_refuses_every_truncation_overlarge_frame_trailing_bytes_and_timer_corruption() {
    let valid = encoded(FurnaceInventory::new());
    for end in 0..valid.len() {
        assert!(
            decode_block_entity(&valid[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(
        decode_block_entity(&trailing),
        Err(BlockEntityCodecError::TrailingBytes)
    );
    let mut frame = valid.clone();
    frame[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(matches!(
        decode_block_entity(&frame),
        Err(BlockEntityCodecError::StackTooLarge { slot: 0, .. })
    ));
    for timers in [
        [0, 1, 0, 0, 0, 0, 0, 200],
        [0, 0, 0, 0, 0, 200, 0, 200],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 128, 0, 0, 0, 0, 200],
    ] {
        let mut invalid = valid.clone();
        let offset = invalid.len() - 8;
        invalid[offset..].copy_from_slice(&timers);
        assert_eq!(
            decode_block_entity(&invalid),
            Err(BlockEntityCodecError::InvalidFurnaceState)
        );
    }
    // FCIS permits a registry-valid count with a low component capacity; native
    // furnace admission must independently refuse it rather than clamp it.
    let invalid = ItemStack::try_new(
        ItemId::from_name("raw_iron").unwrap(),
        2,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(1)], vec![]),
    )
    .unwrap();
    let mut malformed = vec![0x7f, 0, 1, 2];
    for slot in [invalid, ItemStack::empty(), ItemStack::empty()] {
        let record = encode_persisted_stack(&slot).unwrap();
        malformed.extend_from_slice(&(record.len() as u32).to_be_bytes());
        malformed.extend_from_slice(&record);
    }
    malformed.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 200]);
    assert_eq!(
        decode_block_entity(&malformed),
        Err(BlockEntityCodecError::InvalidFurnaceState)
    );
}

#[test]
fn persist_restore_midcook_reaches_same_iron_and_bucket_postimage() {
    let mut live = FurnaceInventory::try_from_parts(
        [
            stack("raw_iron", 2),
            stack("lava_bucket", 1),
            ItemStack::empty(),
        ],
        0,
        0,
        0,
        200,
    )
    .unwrap();
    for _ in 0..113 {
        let plan = plan_furnace_tick(
            live.slots(),
            live.burn_remaining(),
            live.burn_duration(),
            live.cook_progress(),
            live.cook_total(),
        )
        .unwrap();
        live.try_replace(
            plan.slots().clone(),
            plan.burn_remaining(),
            plan.burn_duration(),
            plan.cook_progress(),
            plan.cook_total(),
        )
        .unwrap();
    }
    let mut restored = parsed(&encoded(live.clone()));
    for _ in 0..287 {
        for furnace in [&mut live, &mut restored] {
            let plan = plan_furnace_tick(
                furnace.slots(),
                furnace.burn_remaining(),
                furnace.burn_duration(),
                furnace.cook_progress(),
                furnace.cook_total(),
            )
            .unwrap();
            furnace
                .try_replace(
                    plan.slots().clone(),
                    plan.burn_remaining(),
                    plan.burn_duration(),
                    plan.cook_progress(),
                    plan.cook_total(),
                )
                .unwrap();
        }
        assert_eq!(encoded(live.clone()), encoded(restored.clone()));
    }
    assert_eq!(live.slots()[2], stack("iron_ingot", 2));
    assert_eq!(live.slots()[1], stack("bucket", 1));
    assert!(live.slots()[0].item().is_none());
}

#[test]
fn furnace_exact_sixteen_kib_boundary_is_admitted_and_next_byte_refused() {
    fn payload(a: usize, b: usize) -> ItemStack {
        ItemStack::try_new(
            ItemId::from_name("raw_iron").unwrap(),
            1,
            ComponentPatch::new(
                vec![
                    ComponentValue::Opaque(OpaqueComponent::new(6, vec![0; a]).unwrap()),
                    ComponentValue::Opaque(OpaqueComponent::new(7, vec![0; b]).unwrap()),
                ],
                vec![],
            ),
        )
        .unwrap()
    }
    let empty_len = ferrumc_items::persisted_stack_size(&ItemStack::empty()).unwrap();
    let base = ferrumc_items::persisted_stack_size(&payload(0, 0)).unwrap();
    let remaining = ferrumc_items::MAX_FURNACE_BYTES
        - ferrumc_items::FURNACE_FRAMING_BYTES
        - 2 * empty_len
        - base;
    let second = remaining - 8192;
    let exact = FurnaceInventory::try_from_parts(
        [
            payload(8192, second),
            ItemStack::empty(),
            ItemStack::empty(),
        ],
        0,
        0,
        0,
        200,
    )
    .unwrap();
    let bytes = encoded(exact);
    assert_eq!(bytes.len(), 16384);
    assert_eq!(encoded(parsed(&bytes)), bytes);
    assert!(FurnaceInventory::try_from_parts(
        [
            payload(8192, second + 1),
            ItemStack::empty(),
            ItemStack::empty()
        ],
        0,
        0,
        0,
        200
    )
    .is_err());
}
