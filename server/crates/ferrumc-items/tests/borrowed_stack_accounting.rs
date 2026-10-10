//! Public borrowed probes checked against the unchanged persistence encoder.
//! Encoding/cloning here builds independent test oracles, never producer paths.

use std::num::NonZeroU8;

use ferrumc_items::component::{BLOCK_ENTITY_DATA, DAMAGE, MAX_COMPONENT_TYPE_ID, MAX_STACK_SIZE};
use ferrumc_items::{
    encode_persisted_stack, persisted_stack_size, same_stack_representation, ComponentPatch,
    ComponentTypeId, ComponentValue, ItemId, ItemPersistenceError, ItemStack, ItemValidationError,
    OpaqueComponent, StackTransferError, MAX_COMPONENTS, MAX_COMPONENTS_TOTAL_BYTES,
    MAX_COMPONENT_BYTES, MAX_PERSISTED_STACK_BYTES,
};
use ferrumc_nbt::{NbtCompound, NbtError, NbtLimits, NbtTag};

fn stack(name: &str, count: u8, patch: ComponentPatch) -> ItemStack {
    ItemStack::try_new(ItemId::from_name(name).expect("fixture item"), count, patch)
        .expect("fixture registry count")
}

fn stone(count: u8, added: Vec<ComponentValue>, removed: Vec<ComponentTypeId>) -> ItemStack {
    stack("stone", count, ComponentPatch::new(added, removed))
}

fn data_stack(tag: NbtTag) -> ItemStack {
    stone(1, vec![ComponentValue::CustomData(tag)], Vec::new())
}

fn compound(entries: impl IntoIterator<Item = (&'static str, NbtTag)>) -> NbtTag {
    let mut root = NbtCompound::new();
    for (name, value) in entries {
        root.push(name, value);
    }
    NbtTag::Compound(root)
}

fn rich_nbt() -> NbtTag {
    compound([
        ("dup", NbtTag::Byte(-1)),
        ("dup", NbtTag::Short(i16::MIN)),
        ("int", NbtTag::Int(i32::MIN)),
        ("long", NbtTag::Long(i64::MAX)),
        ("float", NbtTag::Float(f32::from_bits(0x7fc0_1234))),
        (
            "double",
            NbtTag::Double(f64::from_bits(0x7ff8_0000_0000_5678)),
        ),
        ("bytes", NbtTag::ByteArray(vec![-128, -1, 0, 127])),
        ("ints", NbtTag::IntArray(vec![i32::MIN, 0, i32::MAX])),
        ("longs", NbtTag::LongArray(vec![i64::MIN, 0, i64::MAX])),
        ("\0🦀", NbtTag::String("ASCII\0ä🦀".to_owned())),
        (
            "list",
            NbtTag::List(vec![NbtTag::Float(-0.0), NbtTag::Float(0.0)]),
        ),
        (
            "compound",
            compound([("nested", NbtTag::String("value".to_owned()))]),
        ),
        ("empty-list", NbtTag::List(Vec::new())),
    ])
}

fn opaque(id: i32, bytes: Vec<u8>) -> ComponentValue {
    ComponentValue::Opaque(OpaqueComponent::new(id, bytes).expect("bounded opaque fixture"))
}

fn full_payloads(count: usize, length: usize) -> Vec<ComponentValue> {
    (6..=MAX_COMPONENT_TYPE_ID)
        .filter(|id| !ComponentTypeId::new(*id).is_dangerous())
        .take(count)
        .map(|id| opaque(id, vec![0x5a; length]))
        .collect()
}

fn byte_array_root(length: usize) -> NbtTag {
    compound([("a", NbtTag::ByteArray(vec![0; length]))])
}

fn nested_compound(depth: usize) -> NbtTag {
    let mut tag = compound([]);
    for _ in 1..depth {
        tag = compound([("", tag)]);
    }
    tag
}

fn encoded_identity(stack: &ItemStack) -> Vec<u8> {
    let mut bytes = encode_persisted_stack(stack).expect("valid oracle stack");
    if let Some(item) = stack.item() {
        // Independent schema1 oracle: header7 + namelen2 + canonical name,
        // followed by the single fixed-width count byte. Empty has no count.
        bytes[9 + item.name().len()] = 0;
    }
    bytes
}

fn assert_size(stack: &ItemStack) {
    let before = encode_persisted_stack(stack).expect("valid oracle stack");
    let size = persisted_stack_size(stack).expect("checked size");
    assert_eq!(size, before.len());
    assert!(size <= MAX_PERSISTED_STACK_BYTES);
    assert_eq!(
        encode_persisted_stack(stack).expect("unchanged stack"),
        before
    );
}

fn assert_refused_on_both_sides(invalid: &ItemStack, error: &ItemPersistenceError) {
    assert_eq!(persisted_stack_size(invalid), Err(error.clone()));
    assert_eq!(encode_persisted_stack(invalid), Err(error.clone()));
    for counterpart in [
        ItemStack::empty(),
        stone(1, Vec::new(), Vec::new()),
        stack("glass", 1, ComponentPatch::empty()),
        stone(1, vec![ComponentValue::Unbreakable], Vec::new()),
    ] {
        assert_eq!(
            same_stack_representation(invalid, &counterpart),
            Err(StackTransferError::InvalidSource(error.clone()))
        );
        assert_eq!(
            same_stack_representation(&counterpart, invalid),
            Err(StackTransferError::InvalidDestination(error.clone()))
        );
    }
}

#[test]
fn whole_record_sizes_match_existing_golden_empty_and_stone_bytes() {
    let empty = ItemStack::empty();
    assert_eq!(
        encode_persisted_stack(&empty).expect("empty"),
        b"FCIS\0\x01\0"
    );
    assert_eq!(persisted_stack_size(&empty), Ok(7));
    let stack = stone(3, Vec::new(), Vec::new());
    let golden = b"FCIS\0\x01\x01\0\x0fminecraft:stone\x03\0\0";
    assert_eq!(encode_persisted_stack(&stack).expect("stone"), golden);
    assert_eq!(persisted_stack_size(&stack), Ok(golden.len()));
}

#[test]
fn size_accounts_all_representations_mutf8_duplicates_and_float_bits() {
    let patch = vec![
        ComponentValue::MaxStackSize(64),
        ComponentValue::Damage(-7),
        ComponentValue::MaxDamage(i32::MAX),
        ComponentValue::Unbreakable,
        ComponentValue::CustomName(compound([("text", NbtTag::String("name\0🦀".to_owned()))])),
        ComponentValue::CustomData(rich_nbt()),
        opaque(6, vec![0, 0xff, 0x80]),
    ];
    for value in &patch {
        assert_size(&stone(1, vec![value.clone()], Vec::new()));
    }
    assert_size(&stone(
        64,
        patch,
        vec![ComponentTypeId::new(BLOCK_ENTITY_DATA)],
    ));
    assert_size(&stack("diamond_sword", 1, ComponentPatch::empty()));
    assert_size(&stack("air", 1, ComponentPatch::empty()));
}

#[test]
fn all_valid_present_stone_counts_have_fixed_size_and_equal_representation() {
    let patch = ComponentPatch::new(vec![ComponentValue::CustomData(rich_nbt())], Vec::new());
    let source = stack("stone", 1, patch.clone());
    let expected = persisted_stack_size(&source).expect("source size");
    for count in 1..=64 {
        let destination = stack("stone", count, patch.clone());
        assert_eq!(persisted_stack_size(&destination), Ok(expected));
        assert_eq!(same_stack_representation(&source, &destination), Ok(true));
        assert_eq!(encoded_identity(&source), encoded_identity(&destination));
        assert_eq!(destination.count(), count);
    }
    assert_eq!(source.count(), 1);
}

#[test]
fn empty_and_present_air_keep_distinct_model_identity() {
    let empty = ItemStack::empty();
    let present = stack("air", 1, ComponentPatch::empty());
    let larger = stack("air", 2, ComponentPatch::empty());
    assert_eq!(
        same_stack_representation(&empty, &ItemStack::empty()),
        Ok(true)
    );
    assert_eq!(same_stack_representation(&empty, &present), Ok(false));
    assert_eq!(same_stack_representation(&present, &empty), Ok(false));
    assert_eq!(same_stack_representation(&present, &larger), Ok(true));
    assert_ne!(encoded_identity(&empty), encoded_identity(&present));
    assert_size(&present);
}

#[test]
fn ordered_typed_opaque_and_nbt_identity_matches_independent_fcis_oracle() {
    let corpus = vec![
        ItemStack::empty(),
        stone(1, Vec::new(), Vec::new()),
        stone(64, Vec::new(), Vec::new()),
        stack("glass", 1, ComponentPatch::empty()),
        stack("air", 1, ComponentPatch::empty()),
        stone(1, vec![ComponentValue::MaxStackSize(64)], Vec::new()),
        stone(1, vec![opaque(MAX_STACK_SIZE, vec![64])], Vec::new()),
        stone(1, vec![ComponentValue::Damage(-7)], Vec::new()),
        stone(
            1,
            vec![opaque(DAMAGE, (-7_i32).to_be_bytes().to_vec())],
            Vec::new(),
        ),
        stone(
            1,
            vec![ComponentValue::Damage(-7), ComponentValue::MaxStackSize(64)],
            Vec::new(),
        ),
        stone(
            1,
            vec![ComponentValue::MaxStackSize(64), ComponentValue::Damage(-7)],
            Vec::new(),
        ),
        stone(
            1,
            Vec::new(),
            vec![ComponentTypeId::new(3), ComponentTypeId::new(51)],
        ),
        stone(
            1,
            Vec::new(),
            vec![ComponentTypeId::new(51), ComponentTypeId::new(3)],
        ),
        data_stack(compound([("dup", NbtTag::Int(1)), ("dup", NbtTag::Int(2))])),
        data_stack(compound([("dup", NbtTag::Int(2)), ("dup", NbtTag::Int(1))])),
        data_stack(compound([("dup", NbtTag::Int(2))])),
        data_stack(rich_nbt()),
        data_stack(rich_nbt()),
    ];
    let originals: Vec<Vec<u8>> = corpus
        .iter()
        .map(|s| encode_persisted_stack(s).expect("valid corpus"))
        .collect();
    let identities: Vec<Vec<u8>> = corpus.iter().map(encoded_identity).collect();
    for (left_index, left) in corpus.iter().enumerate() {
        for (right_index, right) in corpus.iter().enumerate() {
            assert_eq!(
                same_stack_representation(left, right),
                Ok(identities[left_index] == identities[right_index]),
                "corpus pair {left_index}/{right_index}"
            );
        }
        assert_size(left);
        assert_eq!(
            encode_persisted_stack(left).expect("unchanged corpus"),
            originals[left_index]
        );
    }
}

fn nested_floats(float_bits: u32, double_bits: u64) -> ItemStack {
    data_stack(compound([(
        "list",
        NbtTag::List(vec![compound([
            ("f32", NbtTag::Float(f32::from_bits(float_bits))),
            ("f64", NbtTag::Double(f64::from_bits(double_bits))),
        ])]),
    )]))
}

#[test]
fn nested_f32_f64_identity_distinguishes_signed_zero_and_nan_payloads() {
    let cases = [
        ((0, 0), (0x8000_0000, 0), false),
        ((0, 0), (0, 0x8000_0000_0000_0000), false),
        (
            (0x7fc0_1234, 0x7ff8_0000_0000_5678),
            (0x7fc0_1234, 0x7ff8_0000_0000_5678),
            true,
        ),
        ((0x7fc0_1234, 0), (0x7fc0_1235, 0), false),
        (
            (0, 0x7ff8_0000_0000_5678),
            (0, 0x7ff8_0000_0000_5679),
            false,
        ),
    ];
    for ((left_float, left_double), (right_float, right_double), expected) in cases {
        let left = nested_floats(left_float, left_double);
        let right = nested_floats(right_float, right_double);
        assert_eq!(same_stack_representation(&left, &right), Ok(expected));
        assert_eq!(same_stack_representation(&right, &left), Ok(expected));
        assert_eq!(
            encoded_identity(&left) == encoded_identity(&right),
            expected
        );
        assert_size(&left);
        assert_size(&right);
    }
}

#[test]
fn registry_count_errors_survive_both_comparison_sides_and_empty_inputs() {
    for (name, count) in [("stone", 65), ("stone", u8::MAX), ("diamond_sword", 2)] {
        let item = ItemId::from_name(name).expect("fixture item");
        let invalid = ItemStack::new(
            item,
            NonZeroU8::new(count).expect("nonzero"),
            ComponentPatch::empty(),
        );
        let error = ItemPersistenceError::InvalidStack(ItemValidationError::StackCountOutOfRange {
            item_id: item.id(),
            count,
            max: item.max_stack(),
        });
        assert_refused_on_both_sides(&invalid, &error);
        assert_eq!(invalid.count(), count);
    }
}

#[test]
fn duplicate_conflicting_out_of_range_and_typed_errors_are_not_short_circuited() {
    let cases = [
        (
            stone(1, vec![ComponentValue::MaxStackSize(0)], Vec::new()),
            ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE),
        ),
        (
            stone(1, vec![ComponentValue::MaxStackSize(100)], Vec::new()),
            ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE),
        ),
        (
            stone(
                1,
                vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
                Vec::new(),
            ),
            ItemPersistenceError::DuplicateComponent(DAMAGE),
        ),
        (
            stone(
                1,
                vec![ComponentValue::Damage(1)],
                vec![ComponentTypeId::new(DAMAGE)],
            ),
            ItemPersistenceError::DuplicateComponent(DAMAGE),
        ),
        (
            stone(
                1,
                Vec::new(),
                vec![ComponentTypeId::new(4), ComponentTypeId::new(4)],
            ),
            ItemPersistenceError::DuplicateComponent(4),
        ),
        (
            stone(1, Vec::new(), vec![ComponentTypeId::new(-1)]),
            ItemPersistenceError::InvalidComponentType(-1),
        ),
        (
            stone(1, Vec::new(), vec![ComponentTypeId::new(96)]),
            ItemPersistenceError::InvalidComponentType(96),
        ),
    ];
    for (invalid, error) in cases {
        assert_refused_on_both_sides(&invalid, &error);
    }
}

#[test]
fn source_error_precedes_a_different_invalid_destination_error() {
    let item = ItemId::from_name("stone").expect("stone");
    let source = ItemStack::new(
        item,
        NonZeroU8::new(65).expect("nonzero"),
        ComponentPatch::empty(),
    );
    let destination = stone(1, vec![ComponentValue::MaxStackSize(0)], Vec::new());
    assert_eq!(
        same_stack_representation(&source, &destination),
        Err(StackTransferError::InvalidSource(
            ItemPersistenceError::InvalidStack(ItemValidationError::StackCountOutOfRange {
                item_id: item.id(),
                count: 65,
                max: 64
            })
        ))
    );
    assert_eq!(
        same_stack_representation(&destination, &source),
        Err(StackTransferError::InvalidSource(
            ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE)
        ))
    );
}

#[test]
fn sixty_four_patch_entries_are_admitted_and_sixty_five_are_refused() {
    let maximum = stone(1, Vec::new(), (0..64).map(ComponentTypeId::new).collect());
    assert_size(&maximum);
    assert_eq!(same_stack_representation(&maximum, &maximum), Ok(true));
    let excessive = stone(1, Vec::new(), (0..65).map(ComponentTypeId::new).collect());
    let error = ItemPersistenceError::InvalidStack(ItemValidationError::TooManyComponents {
        count: MAX_COMPONENTS + 1,
        max: MAX_COMPONENTS,
    });
    assert_refused_on_both_sides(&excessive, &error);
}

#[test]
fn file_nbt_payload_size_boundary_is_exact_and_next_byte_is_refused() {
    // Original FCIS file NBT: empty-name root plus one named array =12+length.
    let maximum = data_stack(byte_array_root(MAX_COMPONENT_BYTES - 12));
    assert_eq!(
        persisted_stack_size(&maximum),
        Ok(12 + "minecraft:stone".len() + 9 + MAX_COMPONENT_BYTES)
    );
    assert_size(&maximum);
    assert_eq!(same_stack_representation(&maximum, &maximum), Ok(true));
    let excessive = data_stack(byte_array_root(MAX_COMPONENT_BYTES - 11));
    let error = ItemPersistenceError::Nbt(NbtError::MaxBytesExceeded {
        len: MAX_COMPONENT_BYTES + 1,
        max: MAX_COMPONENT_BYTES,
    });
    assert_refused_on_both_sides(&excessive, &error);
}

#[test]
fn total_payload_byte_boundary_is_lossless_and_plus_one_is_refused() {
    let maximum = stone(1, full_payloads(32, MAX_COMPONENT_BYTES), Vec::new());
    assert_eq!(
        persisted_stack_size(&maximum),
        Ok(12 + "minecraft:stone".len() + 32 * 9 + MAX_COMPONENTS_TOTAL_BYTES)
    );
    assert_size(&maximum);
    assert_eq!(same_stack_representation(&maximum, &maximum), Ok(true));
    let mut components = full_payloads(33, MAX_COMPONENT_BYTES);
    let last_id = components.last().expect("extra component").type_id();
    *components.last_mut().expect("extra component") = opaque(last_id, vec![0x5a]);
    let excessive = stone(1, components, Vec::new());
    let error = ItemPersistenceError::InvalidStack(ItemValidationError::ComponentsTooLarge {
        total: MAX_COMPONENTS_TOTAL_BYTES + 1,
        max: MAX_COMPONENTS_TOTAL_BYTES,
    });
    assert_refused_on_both_sides(&excessive, &error);
}

#[test]
fn nbt_depth_boundary_and_malformed_structure_are_checked_before_identity() {
    let maximum = data_stack(nested_compound(NbtLimits::DEFAULT_MAX_DEPTH));
    assert_size(&maximum);
    assert_eq!(same_stack_representation(&maximum, &maximum), Ok(true));
    let excessive = data_stack(nested_compound(NbtLimits::DEFAULT_MAX_DEPTH + 1));
    let error = ItemPersistenceError::Nbt(NbtError::DepthExceeded {
        max: NbtLimits::DEFAULT_MAX_DEPTH,
    });
    assert_refused_on_both_sides(&excessive, &error);
    let malformed = data_stack(compound([(
        "list",
        NbtTag::List(vec![NbtTag::Int(1), NbtTag::Byte(1)]),
    )]));
    assert_refused_on_both_sides(
        &malformed,
        &ItemPersistenceError::Nbt(NbtError::HeterogeneousList {
            expected: 3,
            found: 1,
        }),
    );
    let wrong_root = data_stack(NbtTag::Int(1));
    assert_refused_on_both_sides(
        &wrong_root,
        &ItemPersistenceError::Nbt(NbtError::UnexpectedRootTag { id: 3 }),
    );
    let mutf8_overflow = data_stack(compound([(
        "text",
        NbtTag::String("🦀".repeat(MAX_COMPONENT_BYTES / 6 + 1)),
    )]));
    assert_refused_on_both_sides(
        &mutf8_overflow,
        &ItemPersistenceError::Nbt(NbtError::StringTooLong {
            len: 8196,
            max: MAX_COMPONENT_BYTES,
        }),
    );
}
