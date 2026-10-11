use ferrumc_nbt::{
    network_root_size, read_network_root_with_consumed, write_named_root, write_network_root,
    write_network_root_into, NbtCompound, NbtError, NbtLimits, NbtTag,
};

const PAPER: &str = include_str!("../../ferrumc-proto/tests/fixtures/item_entity_1_21_8/paper.tsv");

fn root(name: &str, value: NbtTag) -> NbtTag {
    let mut compound = NbtCompound::new();
    compound.push(name, value);
    NbtTag::Compound(compound)
}

fn hex(text: &str) -> Vec<u8> {
    let (pairs, remainder) = text.as_bytes().as_chunks::<2>();
    assert_eq!(remainder, []);
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn buffer_with_room(room: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(room + 3);
    // Actual free capacity, independent of any allocator rounding.
    out.resize(out.capacity() - room, 0xa5);
    let last = out.len() - 3;
    out[last..].copy_from_slice(&[0xff, 0, 0x80]);
    assert_eq!(out.capacity() - out.len(), room);
    out
}

fn assert_emission(tag: &NbtTag, limits: &NbtLimits, expected: &[u8]) {
    assert_eq!(network_root_size(tag, limits).unwrap(), expected.len());
    let mut out = buffer_with_room(expected.len());
    let prefix = out.clone();
    let pointer = out.as_ptr();
    let capacity = out.capacity();
    assert_eq!(
        write_network_root_into(&mut out, tag, limits).unwrap(),
        expected.len()
    );
    assert_eq!(out.as_ptr(), pointer);
    assert_eq!(out.capacity(), capacity);
    assert_eq!(&out[..prefix.len()], prefix);
    assert_eq!(&out[prefix.len()..], expected);
    assert_eq!(write_network_root(tag, limits).unwrap(), expected);
}

fn assert_refusal(tag: &NbtTag, limits: &NbtLimits, expected: NbtError) {
    let legacy_before = write_network_root(tag, limits);
    let size_result = network_root_size(tag, limits);
    for room in [0, 100_000] {
        let mut out = buffer_with_room(room);
        let prefix = out.clone();
        let pointer = out.as_ptr();
        let capacity = out.capacity();
        assert_eq!(
            write_network_root_into(&mut out, tag, limits),
            Err(expected.clone())
        );
        assert_eq!(out, prefix);
        assert_eq!(out.as_ptr(), pointer);
        assert_eq!(out.capacity(), capacity);
    }
    assert_eq!(size_result, Err(expected));
    assert_eq!(write_network_root(tag, limits), legacy_before);
}

fn all_tags() -> NbtTag {
    let mut inner = NbtCompound::new();
    inner.push("x", NbtTag::Float(0.0));
    inner.push("x", NbtTag::Float(-0.0));
    let mut compound = NbtCompound::new();
    compound.push("a", NbtTag::Byte(-1));
    compound.push("b", NbtTag::Short(0x1234));
    compound.push("c", NbtTag::Int(-2));
    compound.push("d", NbtTag::Long(i64::MIN));
    compound.push("e", NbtTag::Float(f32::from_bits(0x7fc0_0001)));
    compound.push("f", NbtTag::Double(-0.0));
    compound.push("g", NbtTag::ByteArray(vec![-1, 0, 127]));
    compound.push("h", NbtTag::String("\0a😀".to_owned()));
    compound.push("i", NbtTag::List(vec![NbtTag::Int(0), NbtTag::Int(-1)]));
    compound.push("j", NbtTag::Compound(inner));
    compound.push("k", NbtTag::IntArray(vec![1, -2]));
    compound.push("l", NbtTag::LongArray(vec![3, -4]));
    NbtTag::Compound(compound)
}

fn all_tags_golden() -> Vec<u8> {
    hex(concat!(
        "0a",
        "01000161ff",
        "020001621234",
        "03000163fffffffe",
        "040001648000000000000000",
        "050001657fc00001",
        "060001668000000000000000",
        "0700016700000003ff007f",
        "080001680009c08061eda0bdedb880",
        "09000169030000000200000000ffffffff",
        "0a00016a0500017800000000050001788000000000",
        "0b00016b0000000200000001fffffffe",
        "0c00016c000000020000000000000003fffffffffffffffc",
        "00",
    ))
}

#[test]
fn empty_root_has_two_network_bytes_and_no_file_name() {
    let tag = NbtTag::Compound(NbtCompound::new());
    let limits = NbtLimits::default();
    assert_emission(&tag, &limits, &[10, 0]);
    assert_eq!(write_named_root("", &tag, &limits).unwrap(), [10, 0, 0, 0]);
    let mut no_room = Vec::new();
    assert_eq!(
        write_network_root_into(&mut no_room, &tag, &limits),
        Err(NbtError::OutputCapacityExceeded {
            needed: 2,
            remaining: 0,
        })
    );
    assert_eq!(no_room.len(), 0);
    assert_eq!(no_room.capacity(), 0);
}

#[test]
fn real_paper_custom_data_preserves_embedded_end_marker_bytes() {
    let row = PAPER
        .lines()
        .find(|line| line.starts_with("metadata_item_components\t"))
        .unwrap();
    let metadata = hex(row.split('\t').nth(6).unwrap());
    assert_eq!(&metadata[..7], &[8, 7, 3, 1, 3, 0, 0]);
    let limits = NbtLimits::default();
    let (decoded, consumed) = read_network_root_with_consumed(&metadata[7..], &limits).unwrap();
    let tag = root("bytes", NbtTag::ByteArray(vec![0, -1, 1, -1]));
    assert_eq!(decoded, tag);
    assert_eq!(consumed, 18);
    assert_eq!(
        &metadata[7 + consumed..],
        &hex("050800077769726520c3a4108001ff")
    );
    assert_emission(&tag, &limits, &metadata[7..7 + consumed]);
}

#[test]
fn all_tag_families_match_fixed_golden_without_normalizing() {
    let tag = all_tags();
    let golden = all_tags_golden();
    let limits = NbtLimits::default();
    assert_emission(&tag, &limits, &golden);
    let file = write_named_root("", &tag, &limits).unwrap();
    assert_eq!(&file[..3], &[10, 0, 0]);
    assert_eq!(&file[3..], &golden[1..]);
    assert_eq!(file.len(), golden.len() + 2);
}

#[test]
fn nested_float_bits_empty_lists_and_duplicate_names_survive() {
    let mut entries = Vec::new();
    for (f, d) in [
        (0x0000_0000, 0x0000_0000_0000_0000),
        (0x8000_0000, 0x8000_0000_0000_0000),
        (0x7fc0_0001, 0x7ff8_0000_0000_0001),
        (0x7fc0_0002, 0x7ff8_0000_0000_0002),
        (0xffc0_0001, 0xfff8_0000_0000_0001),
    ] {
        let mut compound = NbtCompound::new();
        compound.push("same", NbtTag::List(vec![NbtTag::Float(f32::from_bits(f))]));
        compound.push(
            "same",
            NbtTag::List(vec![NbtTag::Double(f64::from_bits(d))]),
        );
        compound.push("empty", NbtTag::List(Vec::new()));
        entries.push(NbtTag::Compound(compound));
    }
    let tag = root("floats", NbtTag::List(entries));
    let limits = NbtLimits::default();
    let bytes = write_network_root(&tag, &limits).unwrap();
    assert_emission(&tag, &limits, &bytes);
    let (decoded, consumed) = read_network_root_with_consumed(&bytes, &limits).unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(write_network_root(&decoded, &limits).unwrap(), bytes);
}

#[test]
fn actual_free_capacity_n_minus_one_n_n_plus_one_preserves_prefix() {
    let tag = all_tags();
    let limits = NbtLimits::default();
    let expected = all_tags_golden();
    for room in [expected.len() - 1, expected.len(), expected.len() + 1] {
        let mut out = buffer_with_room(room);
        let prefix = out.clone();
        let pointer = out.as_ptr();
        let capacity = out.capacity();
        let result = write_network_root_into(&mut out, &tag, &limits);
        if room < expected.len() {
            assert_eq!(
                result,
                Err(NbtError::OutputCapacityExceeded {
                    needed: expected.len(),
                    remaining: room,
                })
            );
            assert_eq!(out, prefix);
        } else {
            assert_eq!(result, Ok(expected.len()));
            assert_eq!(&out[..prefix.len()], prefix);
            assert_eq!(&out[prefix.len()..], expected);
            assert_eq!(out.capacity() - out.len(), room - expected.len());
        }
        assert_eq!(out.capacity(), capacity);
        assert_eq!(out.as_ptr(), pointer);
    }
}

#[test]
fn consecutive_appends_count_only_the_root_and_return_appended_length() {
    let tag = NbtTag::Compound(NbtCompound::new());
    let limits = NbtLimits::default().with_max_bytes(2);
    let mut out = Vec::with_capacity(65_536 + 4);
    out.resize(out.capacity() - 4, 0xfe);
    let prefix = out.clone();
    let pointer = out.as_ptr();
    let capacity = out.capacity();
    assert_eq!(write_network_root_into(&mut out, &tag, &limits), Ok(2));
    assert_eq!(write_network_root_into(&mut out, &tag, &limits), Ok(2));
    assert_eq!(&out[..prefix.len()], prefix);
    assert_eq!(&out[prefix.len()..], &[10, 0, 10, 0]);
    let complete = out.clone();
    assert_eq!(
        write_network_root_into(&mut out, &tag, &limits),
        Err(NbtError::OutputCapacityExceeded {
            needed: 2,
            remaining: 0,
        })
    );
    assert_eq!(out, complete);
    assert_eq!(out.as_ptr(), pointer);
    assert_eq!(out.capacity(), capacity);
}

#[test]
fn whole_root_byte_caps_include_type_prefixes_and_end() {
    let tag = root("a", NbtTag::ByteArray(vec![0, -1, 1, -1]));
    let bytes = write_network_root(&tag, &NbtLimits::default()).unwrap();
    assert_eq!(bytes.len(), 14);
    assert_refusal(
        &tag,
        &NbtLimits::default().with_max_bytes(bytes.len() - 1),
        NbtError::MaxBytesExceeded {
            len: bytes.len(),
            max: bytes.len() - 1,
        },
    );
    for max in [bytes.len(), bytes.len() + 1] {
        assert_emission(&tag, &NbtLimits::default().with_max_bytes(max), &bytes);
    }
    let empty = NbtTag::Compound(NbtCompound::new());
    for (max, len) in [(0, 1), (1, 2)] {
        assert_refusal(
            &empty,
            &NbtLimits::default().with_max_bytes(max),
            NbtError::MaxBytesExceeded { len, max },
        );
    }
}

#[test]
fn every_sequence_family_checks_configured_count_before_traversal() {
    for value in [
        NbtTag::ByteArray(vec![1, 2, 3]),
        NbtTag::IntArray(vec![1, 2, 3]),
        NbtTag::LongArray(vec![1, 2, 3]),
        NbtTag::List(vec![NbtTag::Int(1), NbtTag::Int(2), NbtTag::Int(3)]),
    ] {
        let tag = root("", value);
        assert_refusal(
            &tag,
            &NbtLimits::default().with_max_list_len(2),
            NbtError::ListTooLong { len: 3, max: 2 },
        );
        let expected = write_network_root(&tag, &NbtLimits::default()).unwrap();
        for max in [3, 4] {
            assert_emission(
                &tag,
                &NbtLimits::default().with_max_list_len(max),
                &expected,
            );
        }
    }
    for value in [
        NbtTag::ByteArray(Vec::new()),
        NbtTag::IntArray(Vec::new()),
        NbtTag::LongArray(Vec::new()),
        NbtTag::List(Vec::new()),
    ] {
        let tag = root("", value);
        let limits = NbtLimits::default().with_max_list_len(0);
        let expected = write_network_root(&tag, &limits).unwrap();
        assert_emission(&tag, &limits, &expected);
    }
}

#[test]
fn modified_utf8_payload_caps_count_nul_bmp_and_surrogates() {
    let tag = root("", NbtTag::String("\0ä€😀".to_owned()));
    let expected = write_network_root(&tag, &NbtLimits::default()).unwrap();
    assert_refusal(
        &tag,
        &NbtLimits::default().with_max_string_bytes(12),
        NbtError::StringTooLong { len: 13, max: 12 },
    );
    for max in [13, 14] {
        assert_emission(
            &tag,
            &NbtLimits::default().with_max_string_bytes(max),
            &expected,
        );
    }
    let long = root("", NbtTag::String("a".repeat(100_000)));
    assert_refusal(
        &long,
        &NbtLimits::default().with_max_string_bytes(8),
        NbtError::StringTooLong { len: 9, max: 8 },
    );
}

#[test]
fn compound_names_share_the_same_modified_utf8_limit() {
    let tag = root("\0😀", NbtTag::Byte(1));
    assert_refusal(
        &tag,
        &NbtLimits::default().with_max_string_bytes(7),
        NbtError::StringTooLong { len: 8, max: 7 },
    );
    let expected = write_network_root(&tag, &NbtLimits::default()).unwrap();
    for max in [8, 9] {
        assert_emission(
            &tag,
            &NbtLimits::default().with_max_string_bytes(max),
            &expected,
        );
    }
}

#[test]
fn u16_string_prefix_bound_remains_when_configured_limit_is_larger() {
    let limits = NbtLimits::default().with_max_string_bytes(usize::MAX);
    let tag = root("", NbtTag::String("a".repeat(65_535)));
    let expected = write_network_root(&tag, &limits).unwrap();
    assert_eq!(expected.len(), 65_542);
    assert_emission(&tag, &limits, &expected);
    assert_refusal(
        &root("", NbtTag::String("a".repeat(65_536))),
        &limits,
        NbtError::StringTooLong {
            len: 65_536,
            max: 65_535,
        },
    );
    assert_refusal(
        &root("", NbtTag::String("😀".repeat(10_923))),
        &limits,
        NbtError::StringTooLong {
            len: 65_538,
            max: 65_535,
        },
    );
}

fn compound_depth(depth: usize) -> NbtTag {
    let mut tag = NbtTag::Compound(NbtCompound::new());
    for _ in 1..depth {
        tag = root("", tag);
    }
    tag
}

fn list_depth(depth: usize) -> NbtTag {
    let mut tag = NbtTag::List(Vec::new());
    for _ in 2..depth {
        tag = NbtTag::List(vec![tag]);
    }
    root("", tag)
}

#[test]
fn compound_depth_obeys_configuration_and_the_absolute_checked_ceiling() {
    for max in [3, 512] {
        let limits = NbtLimits::default().with_max_depth(max);
        for depth in [max - 1, max] {
            let tag = compound_depth(depth);
            let expected = write_network_root(&tag, &limits).unwrap();
            assert_emission(&tag, &limits, &expected);
        }
        assert_refusal(
            &compound_depth(max + 1),
            &limits,
            NbtError::DepthExceeded { max },
        );
    }
    let limits = NbtLimits::default().with_max_depth(1_024);
    let deep = compound_depth(513);
    assert!(write_network_root(&deep, &limits).is_ok());
    assert_refusal(&deep, &limits, NbtError::DepthExceeded { max: 512 });
    assert_refusal(
        &compound_depth(1),
        &NbtLimits::default().with_max_depth(0).with_max_bytes(0),
        NbtError::DepthExceeded { max: 0 },
    );
}

#[test]
fn list_depth_obeys_configuration_and_the_absolute_checked_ceiling() {
    for max in [3, 512] {
        let limits = NbtLimits::default().with_max_depth(max);
        for depth in [max - 1, max] {
            let tag = list_depth(depth);
            let expected = write_network_root(&tag, &limits).unwrap();
            assert_emission(&tag, &limits, &expected);
        }
        assert_refusal(
            &list_depth(max + 1),
            &limits,
            NbtError::DepthExceeded { max },
        );
    }
    let limits = NbtLimits::default().with_max_depth(usize::MAX);
    let deep = list_depth(513);
    assert!(write_network_root(&deep, &limits).is_ok());
    assert_refusal(&deep, &limits, NbtError::DepthExceeded { max: 512 });
}

#[test]
fn invalid_roots_precede_depth_resource_and_capacity_errors() {
    let limits = NbtLimits::default().with_max_depth(0).with_max_bytes(0);
    for (tag, id) in [
        (NbtTag::Byte(1), 1),
        (NbtTag::Short(1), 2),
        (NbtTag::Int(1), 3),
        (NbtTag::Long(1), 4),
        (NbtTag::Float(f32::from_bits(0x7fc0_0001)), 5),
        (NbtTag::Double(-0.0), 6),
        (NbtTag::ByteArray(vec![1]), 7),
        (NbtTag::String("wire ä".to_owned()), 8),
        (NbtTag::List(vec![NbtTag::Byte(1)]), 9),
        (NbtTag::IntArray(vec![1]), 11),
        (NbtTag::LongArray(vec![1]), 12),
    ] {
        assert_refusal(&tag, &limits, NbtError::UnexpectedRootTag { id });
    }
}

#[test]
fn bounded_list_count_precedes_children_and_model_errors_precede_capacity() {
    let tag = root("", NbtTag::List(vec![NbtTag::Int(1), NbtTag::Byte(1)]));
    assert_refusal(
        &tag,
        &NbtLimits::default().with_max_list_len(1),
        NbtError::ListTooLong { len: 2, max: 1 },
    );
    assert_refusal(
        &tag,
        &NbtLimits::default().with_max_list_len(2),
        NbtError::HeterogeneousList {
            expected: 3,
            found: 1,
        },
    );
}

#[test]
fn stricter_probes_leave_all_legacy_owned_writer_limits_and_bytes_intact() {
    let tag = all_tags();
    let golden = all_tags_golden();
    for (limits, expected_error) in [
        (
            NbtLimits::default().with_max_bytes(1),
            NbtError::MaxBytesExceeded { len: 2, max: 1 },
        ),
        (
            NbtLimits::default().with_max_list_len(0),
            NbtError::ListTooLong { len: 3, max: 0 },
        ),
        (
            NbtLimits::default().with_max_string_bytes(0),
            NbtError::StringTooLong { len: 1, max: 0 },
        ),
    ] {
        assert_eq!(write_network_root(&tag, &limits).unwrap(), golden);
        let file = write_named_root("", &tag, &limits).unwrap();
        assert_eq!(&file[..3], &[10, 0, 0]);
        assert_eq!(&file[3..], &golden[1..]);
        assert_refusal(&tag, &limits, expected_error);
    }
}
