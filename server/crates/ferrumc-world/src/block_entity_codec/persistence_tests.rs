use ferrumc_items::{
    ComponentTypeId, ComponentValue, ItemValidationError, OpaqueComponent, MAX_COMPONENT_BYTES,
};
use ferrumc_nbt::{NbtCompound, NbtTag};

use super::*;

fn stored(entity: &BlockEntity) -> Vec<u8> {
    let mut bytes = Vec::new();
    try_encode_block_entity(entity, &mut bytes).unwrap();
    bytes
}

fn chest_with_item(item: ItemStack) -> BlockEntity {
    let mut chest = ChestInventory::new();
    *chest.slot_mut(CHEST_SLOTS - 1).unwrap() = item;
    BlockEntity::Chest(chest)
}

fn stone(components: ComponentPatch) -> ItemStack {
    ItemStack::try_new(ItemId::from_name("stone").unwrap(), 13, components).unwrap()
}

fn custom_data() -> NbtTag {
    let mut root = NbtCompound::new();
    root.push("duplicate", NbtTag::Int(1));
    root.push("duplicate", NbtTag::Int(2));
    root.push("float", NbtTag::Float(f32::from_bits(0x7fc0_1234)));
    root.push("zero", NbtTag::Double(-0.0));
    root.push("text", NbtTag::String("NUL\0 und 🦀".to_owned()));
    NbtTag::Compound(root)
}

fn all_variants() -> ItemStack {
    let mut name = NbtCompound::new();
    name.push("text", NbtTag::String("Named 🦀".to_owned()));
    stone(ComponentPatch::new(
        vec![
            ComponentValue::MaxStackSize(1),
            ComponentValue::Damage(-7),
            ComponentValue::MaxDamage(i32::MAX),
            ComponentValue::Unbreakable,
            ComponentValue::CustomName(NbtTag::Compound(name)),
            ComponentValue::CustomData(custom_data()),
            ComponentValue::Opaque(OpaqueComponent::new(6, vec![0, 0xff, 7]).unwrap()),
        ],
        vec![ComponentTypeId::new(95), ComponentTypeId::new(51)],
    ))
}

#[test]
fn versioned_empty_chest_matches_independent_golden_grammar() {
    let mut fixture = vec![0x7f, 0, 1, 1];
    // Each of the 27 empty FCIS records is exactly seven bytes, framed by u32.
    for _ in 0..CHEST_SLOTS {
        fixture.extend_from_slice(b"\0\0\0\x07FCIS\0\x01\0");
    }
    let empty = BlockEntity::Chest(ChestInventory::new());
    assert_eq!(stored(&empty), fixture);
    assert_eq!(decode_block_entity(&fixture).unwrap(), empty);
}

#[test]
fn old_componentless_chest_golden_migrates_without_item_count_changes() {
    let mut fixture = vec![1];
    // Legacy first slot contains 64 stone, followed by 26 empty-slot bytes.
    fixture.extend_from_slice(&[1, 0, 0, 0, 1, 64]);
    fixture.extend(std::iter::repeat_n(0, CHEST_SLOTS - 1));
    let decoded = decode_block_entity(&fixture).unwrap();
    let BlockEntity::Chest(chest) = &decoded else {
        panic!("legacy chest changed entity kind");
    };
    assert_eq!(chest.slot(0).unwrap().item(), ItemId::from_name("stone"));
    assert_eq!(chest.slot(0).unwrap().count(), 64);
    assert!(chest
        .slots()
        .iter()
        .all(|slot| slot.components().is_empty()));
    assert!(chest.slots()[1..].iter().all(|slot| slot.item().is_none()));
    let migrated = stored(&decoded);
    assert_eq!(&migrated[..4], &[0x7f, 0, 1, 1]);
    assert_eq!(decode_block_entity(&migrated).unwrap(), decoded);
    let mut reproduced_legacy = Vec::new();
    encode_block_entity(&decoded, &mut reproduced_legacy);
    assert_eq!(reproduced_legacy, fixture);
}

#[test]
fn old_blank_sign_golden_migrates_preserving_all_fields() {
    // Legacy plain, unwaxed sign, black on both faces, no glow or text.
    let fixture = b"\0\0\0\0\x05black\0\0\0\0\0\0\0\0\0\0\x05black\0\0\0\0\0\0\0\0\0";
    let expected = BlockEntity::Sign(Sign::new(SignKind::Sign));
    assert_eq!(decode_block_entity(fixture).unwrap(), expected);
    let migrated = stored(&expected);
    assert_eq!(&migrated[..4], &[0x7f, 0, 1, 0]);
    assert_eq!(&migrated[4..], &fixture[1..]);
    assert_eq!(decode_block_entity(&migrated).unwrap(), expected);
}

#[test]
fn chest_components_nbt_bits_and_order_survive_versioned_storage() {
    let expected = chest_with_item(all_variants());
    let bytes = stored(&expected);
    let decoded = decode_block_entity(&bytes).unwrap();
    // NaN has no reflexive model PartialEq, so the encoding is the fidelity check.
    assert_eq!(stored(&decoded), bytes);
    let BlockEntity::Chest(chest) = decoded else {
        panic!("chest changed entity kind");
    };
    let item = chest.slot(CHEST_SLOTS - 1).unwrap();
    assert_eq!(item.count(), 13);
    assert_eq!(item.components().added().len(), 7);
    assert_eq!(
        item.components().removed(),
        &[ComponentTypeId::new(95), ComponentTypeId::new(51)]
    );
    let ComponentValue::CustomData(NbtTag::Compound(data)) = &item.components().added()[5] else {
        panic!("custom_data representation changed");
    };
    assert_eq!(
        data.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        vec!["duplicate", "duplicate", "float", "zero", "text"]
    );
    let Some(NbtTag::Float(value)) = data.get("float") else {
        panic!("float missing");
    };
    assert_eq!(value.to_bits(), 0x7fc0_1234);
    let Some(NbtTag::Double(value)) = data.get("zero") else {
        panic!("zero missing");
    };
    assert_eq!(value.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn filled_chest_preserves_slot_positions_and_total_item_count() {
    let mut chest = ChestInventory::new();
    for index in 0..CHEST_SLOTS {
        let mut data = NbtCompound::new();
        data.push("slot", NbtTag::Int(i32::try_from(index).unwrap()));
        *chest.slot_mut(index).unwrap() = ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            u8::try_from(index + 1).unwrap(),
            ComponentPatch::new(
                vec![ComponentValue::CustomData(NbtTag::Compound(data))],
                Vec::new(),
            ),
        )
        .unwrap();
    }
    let expected = BlockEntity::Chest(chest);
    let decoded = decode_block_entity(&stored(&expected)).unwrap();
    assert_eq!(decoded, expected);
    let BlockEntity::Chest(chest) = decoded else {
        panic!("chest changed entity kind");
    };
    assert_eq!(
        chest
            .slots()
            .iter()
            .map(|slot| u32::from(slot.count()))
            .sum::<u32>(),
        378
    );
}

#[test]
fn versioned_sign_preserves_hanging_waxed_styling_and_unicode() {
    let front = SignFace::from_parts(
        [
            "Unicode 🦀\0".to_owned(),
            "äöü".to_owned(),
            String::new(),
            "last".to_owned(),
        ],
        "red".to_owned(),
        true,
    );
    let back = SignFace::from_parts(
        std::array::from_fn(|_| "back".to_owned()),
        "lime".to_owned(),
        false,
    );
    let sign = BlockEntity::Sign(Sign::from_parts(SignKind::Hanging, true, front, back));
    assert_eq!(decode_block_entity(&stored(&sign)).unwrap(), sign);
}

#[test]
fn every_truncated_versioned_payload_and_trailing_byte_fails() {
    for bytes in [
        stored(&chest_with_item(all_variants())),
        stored(&BlockEntity::Sign(Sign::new(SignKind::Sign))),
    ] {
        for end in 0..bytes.len() {
            assert!(
                decode_block_entity(&bytes[..end]).is_err(),
                "accepted prefix {end}"
            );
        }
        let mut extra = bytes;
        extra.push(0);
        assert_eq!(
            decode_block_entity(&extra),
            Err(BlockEntityCodecError::TrailingBytes)
        );
    }
}

#[test]
fn unsupported_schema_and_entity_types_are_rejected_explicitly() {
    for schema in [0_u16, 2, u16::MAX] {
        let mut fixture = vec![0x7f];
        fixture.extend_from_slice(&schema.to_be_bytes());
        assert_eq!(
            decode_block_entity(&fixture),
            Err(BlockEntityCodecError::UnsupportedSchema(schema))
        );
    }
    assert_eq!(
        decode_block_entity(&[0x7f, 0, 1, 7]),
        Err(BlockEntityCodecError::UnknownTag(7))
    );
}

#[test]
fn aggregate_cap_and_hostile_slot_declarations_fail_before_child_decoding() {
    let oversized = vec![0; MAX_BLOCK_ENTITY_PAYLOAD_LEN + 1];
    assert_eq!(
        decode_block_entity(&oversized),
        Err(BlockEntityCodecError::PayloadTooLarge {
            len: oversized.len(),
            max: MAX_BLOCK_ENTITY_PAYLOAD_LEN
        })
    );
    for length in [(MAX_BLOCK_ENTITY_PAYLOAD_LEN + 1) as u32, u32::MAX] {
        let mut declared = vec![0x7f, 0, 1, 1];
        declared.extend_from_slice(&length.to_be_bytes());
        assert_eq!(
            decode_block_entity(&declared),
            Err(BlockEntityCodecError::StackTooLarge {
                slot: 0,
                len: length as usize,
                max: MAX_BLOCK_ENTITY_PAYLOAD_LEN
            })
        );
    }
    let declared_missing = [0x7f, 0, 1, 1, 0, 0, 0, 7];
    assert_eq!(
        decode_block_entity(&declared_missing),
        Err(BlockEntityCodecError::Truncated)
    );
}

fn child_record_fixture(item: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x7f, 0, 1, 1];
    bytes.extend_from_slice(&(item.len() as u32).to_be_bytes());
    bytes.extend_from_slice(item);
    for _ in 1..CHEST_SLOTS {
        bytes.extend_from_slice(b"\0\0\0\x07FCIS\0\x01\0");
    }
    bytes
}

#[test]
fn malformed_child_records_fail_whole_entity_without_partial_success() {
    let item = encode_persisted_stack(&stone(ComponentPatch::empty())).unwrap();
    for malformed in [b"FAIL\0\x01\0".as_slice(), &item[..item.len() - 1]] {
        assert!(matches!(
            decode_block_entity(&child_record_fixture(malformed)),
            Err(BlockEntityCodecError::Item { slot: 0, .. })
        ));
    }
    let mut unknown_schema = b"FCIS\0\x02\0".to_vec();
    assert_eq!(
        decode_block_entity(&child_record_fixture(&unknown_schema)),
        Err(BlockEntityCodecError::Item {
            slot: 0,
            source: ItemPersistenceError::UnsupportedVersion(2)
        })
    );
    unknown_schema[5] = 1;
    unknown_schema.push(0);
    assert!(matches!(
        decode_block_entity(&child_record_fixture(&unknown_schema)),
        Err(BlockEntityCodecError::Item { slot: 0, .. })
    ));
}

fn chest_at_aggregate_boundary(extra_byte: bool) -> BlockEntity {
    // Golden layout: four entity header bytes, 26*(u32 + 7-byte empty FCIS),
    // then u32 + 27-byte present FCIS header + two 9-byte descriptors + payload.
    // Total overhead = 339 bytes. Both opaque values remain under 8192 each.
    let second_size =
        MAX_BLOCK_ENTITY_PAYLOAD_LEN - 339 - MAX_COMPONENT_BYTES + usize::from(extra_byte);
    chest_with_item(stone(ComponentPatch::new(
        vec![
            ComponentValue::Opaque(OpaqueComponent::new(6, vec![0; MAX_COMPONENT_BYTES]).unwrap()),
            ComponentValue::Opaque(OpaqueComponent::new(7, vec![0xff; second_size]).unwrap()),
        ],
        Vec::new(),
    )))
}

#[test]
fn exact_16_kib_chest_round_trips_and_next_byte_is_atomic_failure() {
    let boundary = chest_at_aggregate_boundary(false);
    let bytes = stored(&boundary);
    assert_eq!(bytes.len(), MAX_BLOCK_ENTITY_PAYLOAD_LEN);
    assert_eq!(decode_block_entity(&bytes).unwrap(), boundary);
    let mut destination = vec![0x51, 0x52];
    assert_eq!(
        try_encode_block_entity(&chest_at_aggregate_boundary(true), &mut destination),
        Err(BlockEntityCodecError::PayloadTooLarge {
            len: MAX_BLOCK_ENTITY_PAYLOAD_LEN + 1,
            max: MAX_BLOCK_ENTITY_PAYLOAD_LEN
        })
    );
    assert_eq!(destination, vec![0x51, 0x52]);
}

#[test]
fn invalid_late_chest_stack_fails_atomically_with_slot_and_source() {
    let invalid = ItemStack::new(
        ItemId::from_name("diamond_sword").unwrap(),
        NonZeroU8::new(2).unwrap(),
        ComponentPatch::empty(),
    );
    let mut destination = vec![0x51, 0x52];
    assert!(
        matches!(try_encode_block_entity(&chest_with_item(invalid), &mut destination), Err(BlockEntityCodecError::Item { slot, source: ItemPersistenceError::InvalidStack(ItemValidationError::StackCountOutOfRange { count: 2, max: 1, .. }) }) if slot == CHEST_SLOTS - 1)
    );
    assert_eq!(destination, vec![0x51, 0x52]);
}

#[test]
fn duplicate_patch_and_invalid_nbt_reject_new_entity_encoding() {
    let malformed_patch = stone(ComponentPatch::new(
        vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
        Vec::new(),
    ));
    let malformed_nbt = stone(ComponentPatch::new(
        vec![ComponentValue::CustomData(NbtTag::Int(1))],
        Vec::new(),
    ));
    for item in [malformed_patch, malformed_nbt] {
        let mut destination = vec![0x51];
        assert!(matches!(
            try_encode_block_entity(&chest_with_item(item), &mut destination),
            Err(BlockEntityCodecError::Item { .. })
        ));
        assert_eq!(destination, vec![0x51]);
    }
}

#[test]
fn signs_accept_maximum_utf8_fields_and_reject_oversize_atomically() {
    let mut sign = Sign::new(SignKind::Sign);
    let line = "🦀".repeat(MAX_SIGN_LINE_BYTES / 4);
    sign.set_face_lines(true, std::array::from_fn(|_| line.clone()));
    sign.set_face_lines(false, std::array::from_fn(|_| line.clone()));
    let entity = BlockEntity::Sign(sign.clone());
    assert_eq!(decode_block_entity(&stored(&entity)).unwrap(), entity);
    sign.set_face_lines(
        false,
        std::array::from_fn(|_| "x".repeat(MAX_SIGN_LINE_BYTES + 1)),
    );
    let mut destination = vec![0x51, 0x52];
    assert_eq!(
        try_encode_block_entity(&BlockEntity::Sign(sign), &mut destination),
        Err(BlockEntityCodecError::StringTooLong {
            len: MAX_SIGN_LINE_BYTES + 1,
            max: MAX_SIGN_LINE_BYTES
        })
    );
    assert_eq!(destination, vec![0x51, 0x52]);
}

#[test]
fn invalid_legacy_counts_and_boolean_fields_are_not_normalized() {
    assert_eq!(
        decode_block_entity(&[1, 1, 0, 0, 0, 1, 65]),
        Err(BlockEntityCodecError::CountTooLarge {
            item_id: 1,
            count: 65,
            max: 64
        })
    );
    assert_eq!(
        decode_block_entity(&[0, 0, 2]),
        Err(BlockEntityCodecError::InvalidBoolean(2))
    );
    assert_eq!(
        decode_block_entity(&[0x7f, 0, 1, 0, 0, 2]),
        Err(BlockEntityCodecError::InvalidBoolean(2))
    );
}

#[test]
fn successful_append_preserves_existing_caller_bytes() {
    let entity = BlockEntity::Chest(ChestInventory::new());
    let expected = stored(&entity);
    let mut destination = vec![0x51, 0x52];
    try_encode_block_entity(&entity, &mut destination).unwrap();
    assert_eq!(&destination[..2], &[0x51, 0x52]);
    assert_eq!(&destination[2..], expected);
}
