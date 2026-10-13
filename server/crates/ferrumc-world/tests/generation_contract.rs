use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_world::{
    Chunk, ChunkGenerator, FlatWorldGenerator, GenerationDimension, GenerationError,
    GenerationIdentity, GenerationMinecraftVersion, GeneratorKind, WorldSeed,
    GENERATION_IDENTITY_LEN, SECTION_COUNT,
};

const GOLDEN_SEED: i64 = -0x0102_0304_0506_0708;
const GOLDEN: &str = include_str!("fixtures/generation_identity_v1.hex");

fn golden_bytes() -> [u8; 27] {
    let values = GOLDEN
        .lines()
        .filter(|line| !line.starts_with('#'))
        .flat_map(str::split_whitespace)
        .map(|hex| u8::from_str_radix(hex, 16).expect("declared fixture byte is hex"))
        .collect::<Vec<_>>();
    values.try_into().expect("declared fixture has 27 bytes")
}

#[test]
fn identity_matches_independently_declared_envelope() {
    // This literal additionally pins the textual fixture's field order. Neither
    // expected value comes from the implementation's encoder or constants.
    let expected = [
        0x46, 0x43, 0x47, 0x49, 0x00, 0x01, 0x00, 0x01, 0x00, 0x15, 0x00, 0x08, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x01, 0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf8, 0x00,
    ];
    assert_eq!(golden_bytes(), expected);
    assert_eq!(GENERATION_IDENTITY_LEN, 27);
    let identity = GenerationIdentity::flat_overworld(WorldSeed::new(GOLDEN_SEED));
    assert_eq!(identity.encode(), expected);
    assert_eq!(GenerationIdentity::decode(&expected), Ok(identity));
    assert_eq!(identity.seed().value(), GOLDEN_SEED);
    assert_eq!(identity.generator_kind(), GeneratorKind::FerrumcFlat);
    assert_eq!(identity.generator_revision(), 1);
    assert_eq!(identity.dimension(), GenerationDimension::Overworld);
    assert_eq!(
        identity.minecraft_version(),
        GenerationMinecraftVersion::V1_21_8
    );
    assert_eq!(identity.minecraft_version().components(), [1, 21, 8]);
}

#[test]
fn numeric_seeds_preserve_signed_boundaries_and_byte_order() {
    let cases = [
        (i64::MIN, [0x80, 0, 0, 0, 0, 0, 0, 0]),
        (-1, [0xff; 8]),
        (0, [0; 8]),
        (1, [0, 0, 0, 0, 0, 0, 0, 1]),
        (i64::MAX, [0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
    ];
    for (value, expected_seed_bytes) in cases {
        let identity = GenerationIdentity::flat_overworld(WorldSeed::new(value));
        assert_eq!(identity.seed().value(), value);
        let mut expected = golden_bytes();
        expected[18..26].copy_from_slice(&expected_seed_bytes);
        assert_eq!(identity.encode(), expected);
        assert_eq!(GenerationIdentity::decode(&expected), Ok(identity));
    }
}

#[test]
fn decoder_rejects_every_truncation_and_trailing_data() {
    let golden = golden_bytes();
    for actual in 0..27 {
        assert_eq!(
            GenerationIdentity::decode(&golden[..actual]),
            Err(GenerationError::InvalidLength {
                actual,
                expected: 27,
            })
        );
    }
    for actual in [28, 32, 4096] {
        let mut oversized = vec![0; actual];
        oversized[..27].copy_from_slice(&golden);
        assert_eq!(
            GenerationIdentity::decode(&oversized),
            Err(GenerationError::InvalidLength {
                actual,
                expected: 27,
            })
        );
    }
}

#[test]
fn decoder_rejects_each_magic_byte_and_wrong_record_discriminator() {
    for offset in 0..4 {
        let mut bytes = golden_bytes();
        bytes[offset] ^= 1;
        assert_eq!(
            GenerationIdentity::decode(&bytes),
            Err(GenerationError::InvalidMagic {
                actual: [bytes[0], bytes[1], bytes[2], bytes[3]],
            })
        );
    }
    let mut bytes = golden_bytes();
    bytes[..4].copy_from_slice(b"FCIS");
    assert!(matches!(
        GenerationIdentity::decode(&bytes),
        Err(GenerationError::InvalidMagic { .. })
    ));
}

#[test]
fn envelope_schema_is_rejected_independently_of_generator_revision() {
    for schema in [0_u16, 2, u16::MAX] {
        let mut bytes = golden_bytes();
        bytes[4..6].copy_from_slice(&schema.to_be_bytes());
        assert_eq!(
            GenerationIdentity::decode(&bytes),
            Err(GenerationError::UnsupportedSchema { schema })
        );
    }
}

#[test]
fn every_minecraft_release_component_is_checked() {
    for component in 0..3 {
        for unsupported in [0_u16, 2, u16::MAX] {
            let mut release = [1_u16, 21, 8];
            release[component] = unsupported;
            let mut bytes = golden_bytes();
            let offset = 6 + component * 2;
            bytes[offset..offset + 2].copy_from_slice(&unsupported.to_be_bytes());
            assert_eq!(
                GenerationIdentity::decode(&bytes),
                Err(GenerationError::UnsupportedMinecraftVersion {
                    major: release[0],
                    minor: release[1],
                    patch: release[2],
                })
            );
        }
    }
}

#[test]
fn unknown_algorithm_does_not_fall_back_to_flat() {
    for kind in [0_u16, 3, u16::MAX] {
        let mut bytes = golden_bytes();
        bytes[12..14].copy_from_slice(&kind.to_be_bytes());
        assert_eq!(
            GenerationIdentity::decode(&bytes),
            Err(GenerationError::UnknownGenerator { kind })
        );
    }
}

#[test]
fn known_algorithm_with_unknown_revision_is_not_regenerated() {
    for revision in [0_u32, 2, u32::MAX] {
        let mut bytes = golden_bytes();
        bytes[14..18].copy_from_slice(&revision.to_be_bytes());
        assert_eq!(
            GenerationIdentity::decode(&bytes),
            Err(GenerationError::UnsupportedGeneratorRevision {
                kind: GeneratorKind::FerrumcFlat,
                revision,
            })
        );
    }
}

#[test]
fn unsupported_dimension_does_not_reuse_overworld_geometry() {
    for dimension in [1_u8, 2, u8::MAX] {
        let mut bytes = golden_bytes();
        bytes[26] = dimension;
        assert_eq!(
            GenerationIdentity::decode(&bytes),
            Err(GenerationError::UnsupportedDimension { dimension })
        );
    }
}

#[test]
fn compatibility_requires_equal_supported_identity_including_seed() {
    let expected = GenerationIdentity::flat_overworld(WorldSeed::new(GOLDEN_SEED));
    assert_eq!(expected.ensure_compatible(expected), Ok(()));
    let decoded = GenerationIdentity::decode(&golden_bytes()).expect("known identity");
    assert_eq!(expected.ensure_compatible(decoded), Ok(()));
    // Different release/kind/revision/dimension cannot enter this comparison:
    // their decoder rejection has a separate test above. Only seed varies in
    // the currently closed executable identity set.
    for seed in [i64::MIN, -1, 0, 1, i64::MAX] {
        let actual = GenerationIdentity::flat_overworld(WorldSeed::new(seed));
        assert_eq!(
            expected.ensure_compatible(actual),
            Err(GenerationError::IncompatibleIdentity { expected, actual })
        );
        assert_eq!(
            actual.ensure_compatible(expected),
            Err(GenerationError::IncompatibleIdentity {
                expected: actual,
                actual: expected,
            })
        );
    }
}

#[test]
fn default_generator_has_explicit_zero_seed_without_changing_legacy_profile() {
    let generator = FlatWorldGenerator::new();
    let explicit = FlatWorldGenerator::with_seed(WorldSeed::new(0));
    assert_eq!(generator, explicit);
    assert_eq!(generator, FlatWorldGenerator::default());
    assert_eq!(generator.identity().seed().value(), 0);
    assert_eq!(
        generator.generate(ChunkPos::ORIGIN),
        explicit.generate_chunk(ChunkPos::ORIGIN)
    );
}

#[test]
fn concrete_and_trait_generation_are_reproducible_in_any_chunk_order() {
    let positions = [
        ChunkPos::new(-7, 11),
        ChunkPos::new(4, -9),
        ChunkPos::ORIGIN,
        ChunkPos::new(-1, -1),
        ChunkPos::new(12, 6),
    ];
    for seed in [i64::MIN, GOLDEN_SEED, 0, i64::MAX] {
        let concrete = FlatWorldGenerator::with_seed(WorldSeed::new(seed));
        let generator: &dyn ChunkGenerator = &concrete;
        let identity = generator.identity();
        let forward = positions.map(|pos| generator.generate_chunk(pos));
        for (index, pos) in positions.into_iter().enumerate().rev() {
            let reverse = generator.generate_chunk(pos);
            assert_eq!(reverse, forward[index]);
            assert_eq!(concrete.generate(pos), forward[index]);
            assert_eq!(generator.identity(), identity);
            assert_eq!(reverse.pos(), pos);
            assert_generated_dirty_masks(&reverse);
        }
    }
}

fn assert_generated_dirty_masks(chunk: &Chunk) {
    assert_eq!(chunk.dirty_sections().count(), 8);
    for section in 0..SECTION_COUNT {
        assert_eq!(chunk.is_section_dirty(section), section < 8);
    }
    assert!(!chunk.persist_dirty_sections().any());
    assert!(!chunk.persist_edited_sections().any());
}

#[test]
fn all_flat_blocks_match_the_fixed_revision_one_profile() {
    let position = ChunkPos::new(-2, 3);
    let generator = FlatWorldGenerator::with_seed(WorldSeed::new(GOLDEN_SEED));
    let chunk = generator.generate(position);
    let origin = position.origin_block(0);
    // Numeric expectations are pinned independently of registry lookup and the
    // generator's private constants: bedrock 85, stone 1, dirt 10, grass 9.
    for y in -64..320 {
        let expected = match y {
            -64 => 85,
            -63..=59 => 1,
            60..=62 => 10,
            63 => 9,
            _ => 0,
        };
        for z in 0..16 {
            for x in 0..16 {
                let actual = chunk
                    .get_block(BlockPos::new(origin.x() + x, y, origin.z() + z))
                    .expect("column belongs to generated chunk");
                assert_eq!(actual.as_u32(), expected, "local ({x},{y},{z})");
            }
        }
    }
    assert_generated_dirty_masks(&chunk);
}

#[test]
fn flat_seed_changes_identity_while_leaving_terrain_unchanged() {
    let first = FlatWorldGenerator::with_seed(WorldSeed::new(i64::MIN));
    let second = FlatWorldGenerator::with_seed(WorldSeed::new(i64::MAX));
    assert_ne!(first.identity(), second.identity());
    assert!(matches!(
        first.identity().ensure_compatible(second.identity()),
        Err(GenerationError::IncompatibleIdentity { .. })
    ));
    assert_eq!(
        first.generate(ChunkPos::new(-4, 5)),
        second.generate(ChunkPos::new(-4, 5))
    );
}

#[test]
fn natural_identity_is_distinct_roundtrips_and_rejects_unknown_revisions() {
    for seed in [i64::MIN, 0, i64::MAX] {
        let natural = GenerationIdentity::natural_overworld(WorldSeed::new(seed));
        let flat = GenerationIdentity::flat_overworld(WorldSeed::new(seed));
        let mut expected = flat.encode();
        expected[12..14].copy_from_slice(&2_u16.to_be_bytes());
        assert_eq!(natural.encode(), expected);
        assert_eq!(GenerationIdentity::decode(&expected), Ok(natural));
        assert!(matches!(
            flat.ensure_compatible(natural),
            Err(GenerationError::IncompatibleIdentity { .. })
        ));
        for revision in [0_u32, 2, u32::MAX] {
            expected[14..18].copy_from_slice(&revision.to_be_bytes());
            assert_eq!(
                GenerationIdentity::decode(&expected),
                Err(GenerationError::UnsupportedGeneratorRevision {
                    kind: GeneratorKind::FerrumcNaturalPreview,
                    revision
                })
            );
        }
    }
}
