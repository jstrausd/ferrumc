use std::num::NonZeroU8;

use ferrumc_items::component::{CUSTOM_DATA, DAMAGE, MAX_DAMAGE, MAX_STACK_SIZE};
use ferrumc_items::{
    encode_item_entity_metadata, encode_persisted_stack, persisted_stack_size, ComponentPatch,
    ComponentTypeId, ComponentValue, ItemId, ItemMetadataError, ItemPersistenceError, ItemStack,
    ItemValidationError, OpaqueComponent, MAX_COMPONENTS, MAX_COMPONENTS_TOTAL_BYTES,
    MAX_COMPONENT_BYTES, MAX_ITEM_ENTITY_METADATA_BYTES,
};
use ferrumc_nbt::{write_named_root, NbtCompound, NbtError, NbtLimits, NbtTag};

const PAPER: &str = include_str!("../../ferrumc-proto/tests/fixtures/item_entity_1_21_8/paper.tsv");
const PRISMARINE: &str =
    include_str!("../../ferrumc-proto/tests/fixtures/item_entity_1_21_8/prismarine.tsv");

fn hex(text: &str) -> Vec<u8> {
    let (pairs, remainder) = text.as_bytes().as_chunks::<2>();
    assert_eq!(remainder, []);
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn stack(count: u8, added: Vec<ComponentValue>, removed: Vec<ComponentTypeId>) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::new(added, removed),
    )
    .unwrap()
}

fn compound(name: &str, value: NbtTag) -> NbtTag {
    let mut compound = NbtCompound::new();
    compound.push(name, value);
    NbtTag::Compound(compound)
}

fn data(tag: NbtTag) -> ItemStack {
    stack(1, vec![ComponentValue::CustomData(tag)], Vec::new())
}

fn opaque(id: i32, bytes: Vec<u8>) -> ComponentValue {
    ComponentValue::Opaque(OpaqueComponent::new(id, bytes).unwrap())
}

fn legacy_metadata(stack: &ItemStack) -> Vec<u8> {
    let mut out = vec![8, 7];
    stack.encode_slot(&mut out).unwrap();
    out.push(255);
    out
}

fn assert_emission(stack: &ItemStack) -> Vec<u8> {
    let before = encode_persisted_stack(stack).unwrap();
    let expected = legacy_metadata(stack);
    let result = encode_item_entity_metadata(stack).unwrap();
    assert_eq!(result.as_bytes(), expected);
    let pointer = result.as_bytes().as_ptr();
    let bytes = result.into_bytes();
    assert_eq!(bytes.as_ptr(), pointer);
    assert_eq!(bytes, expected);
    assert_eq!(encode_persisted_stack(stack).unwrap(), before);
    bytes
}

fn assert_model_refusal(stack: &ItemStack, expected: &ItemPersistenceError) {
    let before = format!("{stack:?}");
    let original_error = encode_persisted_stack(stack).unwrap_err();
    let ItemMetadataError::InvalidStack(error) = encode_item_entity_metadata(stack).unwrap_err()
    else {
        panic!("expected the unchanged model error");
    };
    assert_eq!(&error, expected);
    assert_eq!(original_error, error);
    assert_eq!(format!("{stack:?}"), before);
    assert_eq!(encode_persisted_stack(stack).unwrap_err(), error);
}

fn safe_ids() -> impl Iterator<Item = i32> {
    (0..=95).filter(|id| !ComponentTypeId::new(*id).is_dangerous())
}

fn payloads(count: usize, length: usize) -> Vec<ComponentValue> {
    safe_ids()
        .take(count)
        .map(|id| opaque(id, vec![0xff; length]))
        .collect()
}

#[test]
fn five_original_paper_and_prismarine_rows_are_reused_byte_for_byte() {
    let cases = [
        ("metadata_item_empty", ItemStack::empty()),
        ("metadata_item_stone_1", stack(1, Vec::new(), Vec::new())),
        ("metadata_item_stone_64", stack(64, Vec::new(), Vec::new())),
        (
            "metadata_item_components",
            stack(
                3,
                vec![
                    ComponentValue::CustomData(compound(
                        "bytes",
                        NbtTag::ByteArray(vec![0, -1, 1, -1]),
                    )),
                    opaque(5, hex("0800077769726520c3a4")),
                    opaque(16, hex("8001")),
                ],
                Vec::new(),
            ),
        ),
        (
            "metadata_item_removed",
            stack(2, Vec::new(), vec![ComponentTypeId::new(MAX_STACK_SIZE)]),
        ),
    ];
    for (name, stack) in cases {
        let bytes = assert_emission(&stack);
        for fixture in [PAPER, PRISMARINE] {
            let row = fixture
                .lines()
                .find(|line| line.split('\t').next() == Some(name))
                .unwrap();
            assert_eq!(bytes, hex(row.split('\t').nth(6).unwrap()), "{name}");
        }
    }
}

#[test]
fn canonical_empty_is_an_item_entry_and_preserves_moved_buffer() {
    let result = encode_item_entity_metadata(&ItemStack::empty()).unwrap();
    assert_eq!(result.as_bytes(), [8, 7, 0, 255]);
    assert_ne!(result.as_bytes(), [255]);
    let pointer = result.as_bytes().as_ptr();
    let bytes = result.into_bytes();
    assert_eq!(bytes.as_ptr(), pointer);
    assert_eq!(bytes.len(), 4);
}

#[test]
fn all_seven_variants_and_patch_order_match_the_unchanged_slot_encoder() {
    let tag = compound("\0😀", NbtTag::String("ASCII\0ä€😀".to_owned()));
    let stack = stack(
        64,
        vec![
            opaque(16, vec![0x80, 1, 0xff]),
            ComponentValue::CustomName(tag.clone()),
            ComponentValue::MaxDamage(i32::MIN),
            ComponentValue::Unbreakable,
            ComponentValue::Damage(-1),
            ComponentValue::MaxStackSize(99),
            ComponentValue::CustomData(tag),
        ],
        vec![ComponentTypeId::new(95), ComponentTypeId::new(51)],
    );
    let bytes = assert_emission(&stack);
    assert_eq!(&bytes[..10], &[8, 7, 64, 1, 7, 2, 16, 0x80, 1, 0xff]);
    assert_eq!(&bytes[bytes.len() - 3..], &[95, 51, 255]);
}

#[test]
fn signed_damage_and_max_damage_use_real_one_to_five_byte_varints() {
    for (value, payload) in [
        (0, "00"),
        (127, "7f"),
        (128, "8001"),
        (16_383, "ff7f"),
        (16_384, "808001"),
        (2_097_151, "ffff7f"),
        (2_097_152, "80808001"),
        (268_435_455, "ffffff7f"),
        (268_435_456, "8080808001"),
        (i32::MAX, "ffffffff07"),
        (-1, "ffffffff0f"),
        (i32::MIN, "8080808008"),
    ] {
        for (id, component) in [
            (DAMAGE, ComponentValue::Damage(value)),
            (MAX_DAMAGE, ComponentValue::MaxDamage(value)),
        ] {
            let stack = stack(1, vec![component], Vec::new());
            let mut expected = vec![8, 7, 1, 1, 1, 0, u8::try_from(id).unwrap()];
            expected.extend_from_slice(&hex(payload));
            expected.push(255);
            assert_eq!(assert_emission(&stack), expected);
        }
    }
}

#[test]
fn item_ids_above_one_byte_and_present_counts_preserve_the_real_slot() {
    for name in ["diamond_sword", "mace", "waxed_weathered_cut_copper_stairs"] {
        let item = ItemId::from_name(name).unwrap();
        assert!(item.id() > 127);
        for count in [1, item.max_stack()] {
            let stack = ItemStack::try_new(item, count, ComponentPatch::empty()).unwrap();
            assert_emission(&stack);
        }
    }
}

#[test]
fn nested_nbt_preserves_every_family_duplicate_names_and_float_bits() {
    let mut inner = NbtCompound::new();
    inner.push("same", NbtTag::Float(0.0));
    inner.push("same", NbtTag::Float(-0.0));
    inner.push("same", NbtTag::Float(f32::from_bits(0x7fc0_0001)));
    inner.push("same", NbtTag::Float(f32::from_bits(0x7fc0_0002)));
    inner.push(
        "double",
        NbtTag::Double(f64::from_bits(0xfff8_0000_0000_0001)),
    );
    inner.push("byte", NbtTag::Byte(-1));
    inner.push("short", NbtTag::Short(i16::MIN));
    inner.push("int", NbtTag::Int(i32::MIN));
    inner.push("long", NbtTag::Long(i64::MAX));
    inner.push("string\0😀", NbtTag::String("\0ä€😀".to_owned()));
    inner.push("bytes", NbtTag::ByteArray(vec![0, -1]));
    inner.push("ints", NbtTag::IntArray(vec![i32::MIN, -1, 0]));
    inner.push("longs", NbtTag::LongArray(vec![i64::MIN, 0, i64::MAX]));
    inner.push("empty", NbtTag::List(Vec::new()));
    let tag = compound("outer", NbtTag::List(vec![NbtTag::Compound(inner)]));
    let bytes = assert_emission(&data(tag));
    assert!(bytes.windows(4).any(|bytes| bytes == [0x7f, 0xc0, 0, 1]));
    assert!(bytes.windows(4).any(|bytes| bytes == [0x7f, 0xc0, 0, 2]));
}

#[test]
fn present_air_is_explicitly_unsupported_after_lossless_model_validation() {
    let air = ItemId::from_name("air").unwrap();
    let stack = ItemStack::try_new(air, 1, ComponentPatch::empty()).unwrap();
    let before = encode_persisted_stack(&stack).unwrap();
    assert!(matches!(
        encode_item_entity_metadata(&stack),
        Err(ItemMetadataError::UnsupportedPresentAir)
    ));
    assert_eq!(encode_persisted_stack(&stack).unwrap(), before);
    assert_eq!(legacy_metadata(&stack), [8, 7, 1, 0, 0, 0, 255]);

    let invalid = ItemStack::try_new(
        air,
        1,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(0)], Vec::new()),
    )
    .unwrap();
    assert_model_refusal(
        &invalid,
        &ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE),
    );
}

#[test]
fn registry_count_ceiling_wins_before_component_and_envelope_checks() {
    let item = ItemId::from_name("diamond_sword").unwrap();
    let stack = ItemStack::new(
        item,
        NonZeroU8::new(2).unwrap(),
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(99)], Vec::new()),
    );
    assert_model_refusal(
        &stack,
        &ItemValidationError::StackCountOutOfRange {
            item_id: item.id(),
            count: 2,
            max: 1,
        }
        .into(),
    );
}

#[test]
fn invalid_patch_ids_duplicates_conflicts_and_typed_values_are_not_normalized() {
    for id in [-1, 96, i32::MIN, i32::MAX] {
        assert_model_refusal(
            &stack(1, Vec::new(), vec![ComponentTypeId::new(id)]),
            &ItemPersistenceError::InvalidComponentType(id),
        );
    }
    for added in [
        vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
        vec![opaque(DAMAGE, vec![0]), ComponentValue::Damage(0)],
    ] {
        assert_model_refusal(
            &stack(1, added, Vec::new()),
            &ItemPersistenceError::DuplicateComponent(DAMAGE),
        );
    }
    assert_model_refusal(
        &stack(
            1,
            vec![ComponentValue::Damage(1)],
            vec![ComponentTypeId::new(DAMAGE)],
        ),
        &ItemPersistenceError::DuplicateComponent(DAMAGE),
    );
    assert_model_refusal(
        &stack(
            1,
            Vec::new(),
            vec![ComponentTypeId::new(95), ComponentTypeId::new(95)],
        ),
        &ItemPersistenceError::DuplicateComponent(95),
    );
    for value in [0, 100, u8::MAX] {
        assert_model_refusal(
            &stack(1, vec![ComponentValue::MaxStackSize(value)], Vec::new()),
            &ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE),
        );
    }
}

#[test]
fn exact_sixty_four_entries_work_and_sixty_five_refuse() {
    let accepted = stack(1, Vec::new(), (0..64).map(ComponentTypeId::new).collect());
    assert_eq!(assert_emission(&accepted).len(), 7 + MAX_COMPONENTS);
    let rejected = stack(1, Vec::new(), (0..65).map(ComponentTypeId::new).collect());
    assert_model_refusal(
        &rejected,
        &ItemValidationError::TooManyComponents {
            count: 65,
            max: MAX_COMPONENTS,
        }
        .into(),
    );
}

#[test]
fn complete_metadata_n_minus_one_n_n_plus_one_includes_all_overhead() {
    for desired in [
        MAX_ITEM_ENTITY_METADATA_BYTES - 1,
        MAX_ITEM_ENTITY_METADATA_BYTES,
        MAX_ITEM_ENTITY_METADATA_BYTES + 1,
    ] {
        // 7 scalar/header/end bytes + 32 one-byte IDs; no raw payload prefix.
        let last = desired - 39 - 31 * MAX_COMPONENT_BYTES;
        assert!(last <= MAX_COMPONENT_BYTES);
        let added = safe_ids()
            .take(32)
            .enumerate()
            .map(|(index, id)| {
                opaque(
                    id,
                    vec![
                        0xff;
                        if index == 31 {
                            last
                        } else {
                            MAX_COMPONENT_BYTES
                        }
                    ],
                )
            })
            .collect();
        let stack = stack(1, added, Vec::new());
        let before = encode_persisted_stack(&stack).unwrap();
        assert_eq!(persisted_stack_size(&stack).unwrap(), before.len());
        assert_eq!(legacy_metadata(&stack).len(), desired);
        if desired <= MAX_ITEM_ENTITY_METADATA_BYTES {
            assert_eq!(assert_emission(&stack).len(), desired);
        } else {
            assert!(matches!(
                encode_item_entity_metadata(&stack),
                Err(ItemMetadataError::MetadataTooLarge { size, max })
                    if size == desired && max == MAX_ITEM_ENTITY_METADATA_BYTES
            ));
        }
        assert_eq!(encode_persisted_stack(&stack).unwrap(), before);
    }
}

#[test]
fn model_payload_cap_is_distinct_and_precedes_metadata_policy() {
    let exact = stack(1, payloads(32, MAX_COMPONENT_BYTES), Vec::new());
    assert!(persisted_stack_size(&exact).is_ok());
    assert!(matches!(
        encode_item_entity_metadata(&exact),
        Err(ItemMetadataError::MetadataTooLarge { size, max })
            if size == MAX_COMPONENTS_TOTAL_BYTES + 39 && max == MAX_ITEM_ENTITY_METADATA_BYTES
    ));
    let mut added = payloads(32, MAX_COMPONENT_BYTES);
    let id = safe_ids().nth(32).unwrap();
    added.push(opaque(id, vec![0]));
    assert_model_refusal(
        &stack(1, added, Vec::new()),
        &ItemValidationError::ComponentsTooLarge {
            total: MAX_COMPONENTS_TOTAL_BYTES + 1,
            max: MAX_COMPONENTS_TOTAL_BYTES,
        }
        .into(),
    );
}

#[test]
fn file_nbt_8192_boundary_gives_network_8190_without_conflating_slot_size() {
    for file_size in [
        MAX_COMPONENT_BYTES - 1,
        MAX_COMPONENT_BYTES,
        MAX_COMPONENT_BYTES + 1,
    ] {
        let tag = compound("", NbtTag::ByteArray(vec![-1; file_size - 11]));
        assert_eq!(
            write_named_root("", &tag, &NbtLimits::default())
                .unwrap()
                .len(),
            file_size
        );
        let stack = data(tag);
        if file_size <= MAX_COMPONENT_BYTES {
            let bytes = assert_emission(&stack);
            assert_eq!(bytes.len(), 8 + file_size - 2);
            assert_eq!(&bytes[..7], &[8, 7, 1, 1, 1, 0, 0]);
        } else {
            assert_model_refusal(
                &stack,
                &NbtError::MaxBytesExceeded {
                    len: file_size,
                    max: MAX_COMPONENT_BYTES,
                }
                .into(),
            );
        }
    }
}

#[test]
fn typed_nbt_structure_string_and_sequence_refusals_keep_original_errors() {
    assert_model_refusal(
        &data(NbtTag::String(
            "original CustomName stringroot is opaque".to_owned(),
        )),
        &NbtError::UnexpectedRootTag { id: 8 }.into(),
    );
    assert_model_refusal(
        &data(compound(
            "",
            NbtTag::List(vec![NbtTag::Int(1), NbtTag::Byte(2)]),
        )),
        &NbtError::HeterogeneousList {
            expected: 3,
            found: 1,
        }
        .into(),
    );
    assert_model_refusal(
        &data(compound(
            "",
            NbtTag::ByteArray(vec![0; MAX_COMPONENT_BYTES + 1]),
        )),
        &NbtError::ListTooLong {
            len: MAX_COMPONENT_BYTES + 1,
            max: MAX_COMPONENT_BYTES,
        }
        .into(),
    );
    assert_model_refusal(
        &data(compound(
            "",
            NbtTag::String("\0".repeat(MAX_COMPONENT_BYTES / 2 + 1)),
        )),
        &NbtError::StringTooLong {
            len: MAX_COMPONENT_BYTES + 2,
            max: MAX_COMPONENT_BYTES,
        }
        .into(),
    );
}

#[test]
fn typed_nbt_depth_512_passes_and_513_refuses_without_changing_the_source() {
    for depth in [512, 513] {
        let mut tag = NbtTag::Compound(NbtCompound::new());
        for _ in 1..depth {
            tag = compound("", tag);
        }
        let stack = data(tag);
        if depth == 512 {
            assert_emission(&stack);
        } else {
            assert_model_refusal(&stack, &NbtError::DepthExceeded { max: 512 }.into());
        }
    }
}

#[test]
fn trusted_modeled_opaque_bytes_remain_verbatim_without_inner_schema_claims() {
    let stack = stack(
        1,
        vec![
            opaque(CUSTOM_DATA, vec![0xff, 0, 0xff]),
            opaque(DAMAGE, vec![0x80]),
        ],
        Vec::new(),
    );
    assert_eq!(
        assert_emission(&stack),
        [8, 7, 1, 1, 2, 0, 0, 255, 0, 255, 3, 0x80, 255]
    );
}

#[test]
fn old_trusted_slot_acceptance_is_preserved_for_a_stricter_model_refusal() {
    let stack = stack(1, vec![ComponentValue::MaxStackSize(0)], Vec::new());
    let before = legacy_metadata(&stack);
    assert_eq!(before, [8, 7, 1, 1, 1, 0, 1, 0, 255]);
    assert_model_refusal(
        &stack,
        &ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE),
    );
    assert_eq!(legacy_metadata(&stack), before);
}
