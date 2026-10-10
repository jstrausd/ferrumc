use std::num::NonZeroU8;

use ferrumc_codec::CodecError;
use ferrumc_nbt::{NbtCompound, NbtError, NbtTag};

use super::*;
use crate::component::{BLOCK_ENTITY_DATA, ITEM_NAME};

fn stack(added: Vec<ComponentValue>, removed: Vec<ComponentTypeId>) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        3,
        ComponentPatch::new(added, removed),
    )
    .unwrap()
}

fn record(name: &[u8], count: u8, added: &[(u8, i32, &[u8])], removed: &[i32]) -> Vec<u8> {
    let mut out = b"FCIS\0\x01\x01".to_vec();
    out.extend_from_slice(&(name.len() as u16).to_be_bytes());
    out.extend_from_slice(name);
    out.push(count);
    out.push(added.len() as u8);
    out.push(removed.len() as u8);
    for (kind, type_id, raw) in added {
        out.push(*kind);
        out.extend_from_slice(&type_id.to_be_bytes());
        out.extend_from_slice(&(raw.len() as u32).to_be_bytes());
        out.extend_from_slice(raw);
    }
    for type_id in removed {
        out.extend_from_slice(&type_id.to_be_bytes());
    }
    out
}

fn stone_record(added: &[(u8, i32, &[u8])], removed: &[i32]) -> Vec<u8> {
    record(b"minecraft:stone", 3, added, removed)
}

fn custom_data() -> NbtTag {
    let mut nested = NbtCompound::new();
    nested.push("same", NbtTag::Int(1));
    nested.push("same", NbtTag::Int(2));
    let mut root = NbtCompound::new();
    root.push("byte", NbtTag::Byte(i8::MIN));
    root.push("short", NbtTag::Short(i16::MIN));
    root.push("int", NbtTag::Int(i32::MIN));
    root.push("long", NbtTag::Long(i64::MIN));
    root.push("float", NbtTag::Float(f32::from_bits(0x7fc0_1234)));
    root.push(
        "double",
        NbtTag::Double(f64::from_bits(0xfff8_1234_5678_1234)),
    );
    root.push("negative-zero", NbtTag::Double(-0.0));
    root.push("bytes", NbtTag::ByteArray(vec![i8::MIN, 0, i8::MAX]));
    root.push("text", NbtTag::String("\0Grüße 🦀".to_owned()));
    root.push("list", NbtTag::List(vec![NbtTag::Int(3), NbtTag::Int(1)]));
    root.push("empty-list", NbtTag::List(Vec::new()));
    root.push("nested", NbtTag::Compound(nested));
    root.push("ints", NbtTag::IntArray(vec![i32::MIN, i32::MAX]));
    root.push("longs", NbtTag::LongArray(vec![i64::MIN, i64::MAX]));
    NbtTag::Compound(root)
}

fn all_variants() -> ItemStack {
    let mut name = NbtCompound::new();
    name.push("text", NbtTag::String("Named 🦀\0".to_owned()));
    stack(
        vec![
            ComponentValue::MaxStackSize(1),
            ComponentValue::Damage(i32::MIN),
            ComponentValue::MaxDamage(i32::MAX),
            ComponentValue::Unbreakable,
            ComponentValue::CustomName(NbtTag::Compound(name)),
            ComponentValue::CustomData(custom_data()),
            ComponentValue::Opaque(OpaqueComponent::new(ITEM_NAME, vec![0, 0xff, 7]).unwrap()),
        ],
        vec![
            ComponentTypeId::new(MAX_COMPONENT_TYPE_ID),
            ComponentTypeId::new(BLOCK_ENTITY_DATA),
        ],
    )
}

#[test]
fn storage_golden_empty_and_componentless_stacks() {
    let empty = b"FCIS\0\x01\0";
    assert_eq!(encode_persisted_stack(&ItemStack::empty()).unwrap(), empty);
    assert_eq!(decode_persisted_stack(empty).unwrap(), ItemStack::empty());

    let stone = b"FCIS\0\x01\x01\0\x0fminecraft:stone\x03\0\0";
    let expected = stack(Vec::new(), Vec::new());
    assert_eq!(encode_persisted_stack(&expected).unwrap(), stone);
    assert_eq!(decode_persisted_stack(stone).unwrap(), expected);
}

#[test]
fn storage_golden_typed_and_opaque_components_are_distinct() {
    // An opaque payload for a modeled id retains its representation and bytes.
    let typed = stone_record(
        &[(2, DAMAGE, &[0xff, 0xff, 0xff, 0xf9])],
        &[BLOCK_ENTITY_DATA],
    );
    assert_eq!(
        decode_persisted_stack(&typed).unwrap(),
        stack(
            vec![ComponentValue::Damage(-7)],
            vec![ComponentTypeId::new(BLOCK_ENTITY_DATA)]
        )
    );
    let opaque = stone_record(
        &[(7, DAMAGE, &[0xff, 0xff, 0xff, 0xf9])],
        &[BLOCK_ENTITY_DATA],
    );
    let decoded = decode_persisted_stack(&opaque).unwrap();
    assert!(matches!(
        decoded.components().added(),
        [ComponentValue::Opaque(component)] if component.raw() == [0xff, 0xff, 0xff, 0xf9]
    ));
    assert_eq!(encode_persisted_stack(&decoded).unwrap(), opaque);
    assert_ne!(typed, opaque);
}

#[test]
fn all_component_and_nbt_variants_preserve_bits_and_order() {
    let expected = all_variants();
    let bytes = encode_persisted_stack(&expected).unwrap();
    let decoded = decode_persisted_stack(&bytes).unwrap();
    assert_eq!(encode_persisted_stack(&decoded).unwrap(), bytes);
    assert_eq!(decoded.count(), expected.count());
    assert_eq!(
        decoded.components().removed(),
        expected.components().removed()
    );
    assert_eq!(decoded.components().added().len(), 7);
    let ComponentValue::CustomData(NbtTag::Compound(data)) = &decoded.components().added()[5]
    else {
        panic!("custom_data representation changed");
    };
    let Some(NbtTag::Float(value)) = data.get("float") else {
        panic!("float missing");
    };
    assert_eq!(value.to_bits(), 0x7fc0_1234);
    let Some(NbtTag::Double(value)) = data.get("double") else {
        panic!("double missing");
    };
    assert_eq!(value.to_bits(), 0xfff8_1234_5678_1234);
    let Some(NbtTag::Double(value)) = data.get("negative-zero") else {
        panic!("negative zero missing");
    };
    assert_eq!(value.to_bits(), (-0.0_f64).to_bits());
    let Some(NbtTag::Compound(nested)) = data.get("nested") else {
        panic!("nested compound missing");
    };
    assert_eq!(
        nested.iter().collect::<Vec<_>>(),
        vec![("same", &NbtTag::Int(1)), ("same", &NbtTag::Int(2))]
    );
}

#[test]
fn every_truncation_and_trailing_byte_is_rejected() {
    for complete in [
        encode_persisted_stack(&ItemStack::empty()).unwrap(),
        encode_persisted_stack(&all_variants()).unwrap(),
    ] {
        for length in 0..complete.len() {
            assert!(
                decode_persisted_stack(&complete[..length]).is_err(),
                "length={length}"
            );
        }
        let mut trailing = complete;
        trailing.push(0);
        assert!(matches!(
            decode_persisted_stack(&trailing),
            Err(ItemPersistenceError::Codec(CodecError::TrailingBytes {
                remaining: 1
            }))
        ));
    }
}

#[test]
fn unknown_magic_version_stack_and_component_discriminators_fail() {
    assert_eq!(
        decode_persisted_stack(b"nope\0\x01\0"),
        Err(ItemPersistenceError::InvalidMagic)
    );
    for version in [0_u16, 2, u16::MAX] {
        let mut bytes = b"FCIS".to_vec();
        bytes.extend_from_slice(&version.to_be_bytes());
        bytes.push(0);
        assert_eq!(
            decode_persisted_stack(&bytes),
            Err(ItemPersistenceError::UnsupportedVersion(version))
        );
    }
    for kind in [2, 255] {
        let bytes = [b'F', b'C', b'I', b'S', 0, 1, kind];
        assert_eq!(
            decode_persisted_stack(&bytes),
            Err(ItemPersistenceError::InvalidStackKind(kind))
        );
    }
    for kind in [0, 8, 255] {
        assert_eq!(
            decode_persisted_stack(&stone_record(&[(kind, DAMAGE, &[])], &[])),
            Err(ItemPersistenceError::InvalidComponentKind(kind))
        );
    }
}

#[test]
fn invalid_resource_names_and_declared_name_limits_fail() {
    assert_eq!(
        decode_persisted_stack(&record(b"stone", 1, &[], &[])),
        Err(ItemPersistenceError::NonCanonicalItemName(
            "stone".to_owned()
        ))
    );
    assert_eq!(
        decode_persisted_stack(&record(b"minecraft:absent", 1, &[], &[])),
        Err(ItemPersistenceError::UnknownItemName(
            "minecraft:absent".to_owned()
        ))
    );
    assert!(matches!(
        decode_persisted_stack(&record(&[0xff], 1, &[], &[])),
        Err(ItemPersistenceError::Codec(CodecError::InvalidUtf8(_)))
    ));
    let mut declared = b"FCIS\0\x01\x01".to_vec();
    declared.extend_from_slice(&257_u16.to_be_bytes());
    assert_eq!(
        decode_persisted_stack(&declared),
        Err(ItemPersistenceError::ItemNameTooLarge {
            size: 257,
            max: MAX_ITEM_NAME_BYTES
        })
    );
}

#[test]
fn counts_are_rejected_without_clamping_or_component_override() {
    let sword = ItemId::from_name("diamond_sword").unwrap();
    for (item, count) in [(ItemId::from_name("stone").unwrap(), 0), (sword, 2)] {
        let expected =
            ItemPersistenceError::InvalidStack(ItemValidationError::StackCountOutOfRange {
                item_id: item.id(),
                count,
                max: item.max_stack(),
            });
        assert_eq!(
            decode_persisted_stack(&record(item.name().as_bytes(), count, &[], &[])),
            Err(expected.clone())
        );
        if let Some(nonzero) = NonZeroU8::new(count) {
            let invalid = ItemStack::new(
                item,
                nonzero,
                ComponentPatch::new(vec![ComponentValue::MaxStackSize(99)], Vec::new()),
            );
            assert_eq!(encode_persisted_stack(&invalid), Err(expected));
        }
    }
    let valid = stack(vec![ComponentValue::MaxStackSize(1)], Vec::new());
    assert_eq!(
        decode_persisted_stack(&encode_persisted_stack(&valid).unwrap()).unwrap(),
        valid
    );
}

#[test]
fn invalid_max_stack_sizes_and_typed_payloads_fail() {
    for value in [0, 100, 255] {
        assert_eq!(
            encode_persisted_stack(&stack(
                vec![ComponentValue::MaxStackSize(value)],
                Vec::new()
            )),
            Err(ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE))
        );
        assert_eq!(
            decode_persisted_stack(&stone_record(&[(1, MAX_STACK_SIZE, &[value])], &[])),
            Err(ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE))
        );
    }
    for value in [1, 99] {
        let expected = stack(vec![ComponentValue::MaxStackSize(value)], Vec::new());
        assert_eq!(
            decode_persisted_stack(&encode_persisted_stack(&expected).unwrap()).unwrap(),
            expected
        );
    }
    for (kind, type_id, raw) in [
        (1, MAX_STACK_SIZE, &[][..]),
        (2, DAMAGE, &[0, 0, 0][..]),
        (3, MAX_DAMAGE, &[0, 0, 0, 0, 0][..]),
        (4, UNBREAKABLE, &[0][..]),
    ] {
        assert!(matches!(
            decode_persisted_stack(&stone_record(&[(kind, type_id, raw)], &[])),
            Err(ItemPersistenceError::Codec(_))
        ));
    }
    assert_eq!(
        decode_persisted_stack(&stone_record(&[(2, MAX_DAMAGE, &[0; 4])], &[])),
        Err(ItemPersistenceError::ComponentTypeMismatch {
            kind: 2,
            type_id: MAX_DAMAGE
        })
    );
}

#[test]
fn duplicate_and_conflicting_patch_ids_fail_in_both_directions() {
    let duplicate_added = stack(
        vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
        Vec::new(),
    );
    let duplicate_removed = stack(
        Vec::new(),
        vec![ComponentTypeId::new(DAMAGE), ComponentTypeId::new(DAMAGE)],
    );
    let conflict = stack(
        vec![ComponentValue::Damage(1)],
        vec![ComponentTypeId::new(DAMAGE)],
    );
    for invalid in [duplicate_added, duplicate_removed, conflict] {
        assert_eq!(
            encode_persisted_stack(&invalid),
            Err(ItemPersistenceError::DuplicateComponent(DAMAGE))
        );
    }
    for malformed in [
        stone_record(&[(2, DAMAGE, &[0; 4]), (2, DAMAGE, &[0; 4])], &[]),
        stone_record(&[], &[DAMAGE, DAMAGE]),
        stone_record(&[(2, DAMAGE, &[0; 4])], &[DAMAGE]),
    ] {
        assert_eq!(
            decode_persisted_stack(&malformed),
            Err(ItemPersistenceError::DuplicateComponent(DAMAGE))
        );
    }
}

#[test]
fn removal_range_and_opaque_safety_contract_fail_closed() {
    for type_id in [-1, 96, i32::MAX] {
        assert_eq!(
            encode_persisted_stack(&stack(Vec::new(), vec![ComponentTypeId::new(type_id)])),
            Err(ItemPersistenceError::InvalidComponentType(type_id))
        );
        assert_eq!(
            decode_persisted_stack(&stone_record(&[], &[type_id])),
            Err(ItemPersistenceError::InvalidComponentType(type_id))
        );
    }
    for type_id in [BLOCK_ENTITY_DATA, crate::component::CONTAINER] {
        assert_eq!(
            decode_persisted_stack(&stone_record(&[(7, type_id, &[1, 2])], &[])),
            Err(ItemPersistenceError::InvalidComponentType(type_id))
        );
    }
}

#[test]
fn patch_entry_count_accepts_64_and_rejects_65() {
    let removals = (0..64).map(ComponentTypeId::new).collect();
    let boundary = stack(Vec::new(), removals);
    assert_eq!(
        decode_persisted_stack(&encode_persisted_stack(&boundary).unwrap()).unwrap(),
        boundary
    );
    let excessive = stack(Vec::new(), (0..65).map(ComponentTypeId::new).collect());
    let expected = ItemPersistenceError::InvalidStack(ItemValidationError::TooManyComponents {
        count: 65,
        max: MAX_COMPONENTS,
    });
    assert_eq!(encode_persisted_stack(&excessive), Err(expected.clone()));
    // Deliberately omit the declared entries: the count must fail before reads.
    let mut bytes = stone_record(&[], &[]);
    *bytes.last_mut().unwrap() = 65;
    assert_eq!(decode_persisted_stack(&bytes), Err(expected));
}

#[test]
fn opaque_payload_and_declared_payload_limits_are_bounded() {
    let expected = stack(
        vec![ComponentValue::Opaque(
            OpaqueComponent::new(ITEM_NAME, vec![0x81; MAX_COMPONENT_BYTES]).unwrap(),
        )],
        Vec::new(),
    );
    assert_eq!(
        decode_persisted_stack(&encode_persisted_stack(&expected).unwrap()).unwrap(),
        expected
    );
    for size in [MAX_COMPONENT_BYTES + 1, u32::MAX as usize] {
        let mut bytes = stone_record(&[], &[]);
        let added_position = bytes.len() - 2;
        bytes[added_position] = 1;
        bytes.push(7);
        bytes.extend_from_slice(&ITEM_NAME.to_be_bytes());
        bytes.extend_from_slice(&(size as u32).to_be_bytes());
        assert_eq!(
            decode_persisted_stack(&bytes),
            Err(ItemPersistenceError::ComponentTooLarge {
                type_id: ITEM_NAME,
                size,
                max: MAX_COMPONENT_BYTES
            })
        );
    }
}

fn opaque_full_payloads(count: usize) -> Vec<ComponentValue> {
    (6..=MAX_COMPONENT_TYPE_ID)
        .filter(|id| !ComponentTypeId::new(*id).is_dangerous())
        .take(count)
        .map(|id| {
            ComponentValue::Opaque(OpaqueComponent::new(id, vec![0; MAX_COMPONENT_BYTES]).unwrap())
        })
        .collect()
}

#[test]
fn cumulative_payload_limit_accepts_256_kib_and_rejects_next_payload() {
    let boundary = stack(opaque_full_payloads(32), Vec::new());
    let bytes = encode_persisted_stack(&boundary).unwrap();
    assert!(bytes.len() <= MAX_PERSISTED_STACK_BYTES);
    assert_eq!(decode_persisted_stack(&bytes).unwrap(), boundary);
    let excessive = stack(opaque_full_payloads(33), Vec::new());
    assert!(matches!(
        encode_persisted_stack(&excessive),
        Err(ItemPersistenceError::InvalidStack(
            ItemValidationError::ComponentsTooLarge { .. }
        ))
    ));

    // Keep the whole record below its envelope cap, but declare one extra
    // byte after exactly 256 KiB of payload. No extra payload is available;
    // the total limit must win over EOF and before another allocation.
    let mut forged = bytes;
    let added_position = 9 + "minecraft:stone".len() + 1;
    forged[added_position] = 33;
    forged.push(7);
    forged.extend_from_slice(&95_i32.to_be_bytes());
    forged.extend_from_slice(&1_u32.to_be_bytes());
    assert_eq!(
        decode_persisted_stack(&forged),
        Err(ItemPersistenceError::InvalidStack(
            ItemValidationError::ComponentsTooLarge {
                total: MAX_COMPONENTS_TOTAL_BYTES + 1,
                max: MAX_COMPONENTS_TOTAL_BYTES
            }
        ))
    );
}

fn byte_array_root(length: usize) -> NbtTag {
    let mut root = NbtCompound::new();
    root.push("a", NbtTag::ByteArray(vec![0; length]));
    NbtTag::Compound(root)
}

#[test]
fn nbt_exact_byte_limit_is_preflighted_before_encoding() {
    // Empty-name file root + named one-character byte-array field = 12 bytes.
    let boundary = stack(
        vec![ComponentValue::CustomData(byte_array_root(
            MAX_COMPONENT_BYTES - 12,
        ))],
        Vec::new(),
    );
    assert_eq!(
        decode_persisted_stack(&encode_persisted_stack(&boundary).unwrap()).unwrap(),
        boundary
    );
    let excessive = stack(
        vec![ComponentValue::CustomData(byte_array_root(
            MAX_COMPONENT_BYTES - 11,
        ))],
        Vec::new(),
    );
    assert!(matches!(
        encode_persisted_stack(&excessive),
        Err(ItemPersistenceError::Nbt(NbtError::MaxBytesExceeded { .. }))
    ));
}

#[test]
fn nbt_structure_sequence_string_and_depth_limits_match_both_directions() {
    let bad_sequence = stack(
        vec![ComponentValue::CustomData(byte_array_root(
            MAX_COMPONENT_BYTES + 1,
        ))],
        Vec::new(),
    );
    assert!(matches!(
        encode_persisted_stack(&bad_sequence),
        Err(ItemPersistenceError::Nbt(NbtError::ListTooLong { .. }))
    ));
    let mut mixed = NbtCompound::new();
    mixed.push("x", NbtTag::List(vec![NbtTag::Int(1), NbtTag::Byte(1)]));
    assert!(matches!(
        encode_persisted_stack(&stack(
            vec![ComponentValue::CustomData(NbtTag::Compound(mixed))],
            Vec::new()
        )),
        Err(ItemPersistenceError::Nbt(
            NbtError::HeterogeneousList { .. }
        ))
    ));
    for text in [
        "a".repeat(MAX_COMPONENT_BYTES + 1),
        "🦀".repeat(MAX_COMPONENT_BYTES / 6 + 1),
    ] {
        let mut root = NbtCompound::new();
        root.push("text", NbtTag::String(text));
        assert!(matches!(
            encode_persisted_stack(&stack(
                vec![ComponentValue::CustomData(NbtTag::Compound(root))],
                Vec::new()
            )),
            Err(ItemPersistenceError::Nbt(NbtError::StringTooLong { .. }))
        ));
    }
    let mut too_deep = NbtTag::Compound(NbtCompound::new());
    for _ in 0..NbtLimits::DEFAULT_MAX_DEPTH {
        let mut outer = NbtCompound::new();
        outer.push("", too_deep);
        too_deep = NbtTag::Compound(outer);
    }
    assert!(matches!(
        encode_persisted_stack(&stack(
            vec![ComponentValue::CustomData(too_deep)],
            Vec::new()
        )),
        Err(ItemPersistenceError::Nbt(NbtError::DepthExceeded { .. }))
    ));
    // Valid root, then array declaration greater than cap, without its payload.
    let malformed = [10, 0, 0, 7, 0, 1, b'a', 0, 0, 0x20, 1];
    assert!(matches!(
        decode_persisted_stack(&stone_record(&[(6, CUSTOM_DATA, &malformed)], &[])),
        Err(ItemPersistenceError::Nbt(NbtError::ListTooLong { .. }))
    ));
}

#[test]
fn malformed_nbt_roots_and_nonempty_names_fail() {
    assert!(matches!(
        encode_persisted_stack(&stack(
            vec![ComponentValue::CustomData(NbtTag::Int(1))],
            Vec::new()
        )),
        Err(ItemPersistenceError::Nbt(NbtError::UnexpectedRootTag {
            id: 3
        }))
    ));
    for raw in [
        &[3, 0, 0][..],
        &[10, 0, 0, 0, 0][..],
        &[10, 0, 0, 255][..],
        &[10, 0, 0, 8, 0, 0, 0, 1, 255, 0][..],
    ] {
        assert!(matches!(
            decode_persisted_stack(&stone_record(&[(6, CUSTOM_DATA, raw)], &[])),
            Err(ItemPersistenceError::Nbt(_))
        ));
    }
    assert_eq!(
        decode_persisted_stack(&stone_record(
            &[(5, CUSTOM_NAME, &[10, 0, 1, b'x', 0])],
            &[]
        )),
        Err(ItemPersistenceError::NonEmptyNbtRootName)
    );
}

#[test]
fn independent_file_nbt_fixture_retains_custom_name() {
    // File-form compound root, then TAG_String named "text" holding "hello".
    // This fixture is specified independently of the item and NBT encoders.
    let raw = [
        10, 0, 0, 8, 0, 4, b't', b'e', b'x', b't', 0, 5, b'h', b'e', b'l', b'l', b'o', 0,
    ];
    let fixture = stone_record(&[(5, CUSTOM_NAME, &raw)], &[]);
    let mut text = NbtCompound::new();
    text.push("text", NbtTag::String("hello".to_owned()));
    let expected = stack(
        vec![ComponentValue::CustomName(NbtTag::Compound(text))],
        Vec::new(),
    );
    assert_eq!(decode_persisted_stack(&fixture).unwrap(), expected);
    assert_eq!(encode_persisted_stack(&expected).unwrap(), fixture);
}

fn nested_compound_fixture(depth: usize) -> Vec<u8> {
    let mut raw = vec![10, 0, 0];
    for _ in 1..depth {
        raw.extend_from_slice(&[10, 0, 0]);
    }
    raw.extend(std::iter::repeat_n(0, depth));
    raw
}

#[test]
fn nbt_depth_boundary_agrees_between_preflight_and_decoder() {
    let raw = nested_compound_fixture(NbtLimits::DEFAULT_MAX_DEPTH);
    let fixture = stone_record(&[(6, CUSTOM_DATA, &raw)], &[]);
    let decoded = decode_persisted_stack(&fixture).unwrap();
    assert_eq!(encode_persisted_stack(&decoded).unwrap(), fixture);
    let too_deep = nested_compound_fixture(NbtLimits::DEFAULT_MAX_DEPTH + 1);
    assert_eq!(
        decode_persisted_stack(&stone_record(&[(6, CUSTOM_DATA, &too_deep)], &[])),
        Err(ItemPersistenceError::Nbt(NbtError::DepthExceeded {
            max: NbtLimits::DEFAULT_MAX_DEPTH
        }))
    );
}

#[test]
fn whole_record_limit_is_checked_before_magic_or_allocation() {
    let oversized = vec![0; MAX_PERSISTED_STACK_BYTES + 1];
    assert_eq!(
        decode_persisted_stack(&oversized),
        Err(ItemPersistenceError::RecordTooLarge {
            size: oversized.len(),
            max: MAX_PERSISTED_STACK_BYTES
        })
    );
}

#[test]
fn persistence_retains_present_air_instead_of_lossy_normalization() {
    let present_air = ItemStack::try_new(
        ItemId::from_name("air").unwrap(),
        1,
        ComponentPatch::empty(),
    )
    .unwrap();
    assert_eq!(
        decode_persisted_stack(&encode_persisted_stack(&present_air).unwrap()).unwrap(),
        present_air
    );
    assert_ne!(present_air, ItemStack::empty());
}
