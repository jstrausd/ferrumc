//! Public configuration keeps declarations distinct from runtime resolution.

use std::fmt::Write;
use std::path::Path;

use ferrumc_config::{WorldConfig, WorldGenerationPreset};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Configuration {
    world: WorldConfig,
}

#[test]
fn absent_table_and_fields_do_not_choose_a_seed_or_preset() {
    for input in ["", "[world]", "[world]\nanvil_import_dir = '/maps/region'"] {
        let parsed: Configuration = toml::from_str(input).unwrap();
        assert_eq!(parsed.world.seed(), None);
        assert_eq!(parsed.world.preset(), None);
    }
    let default = WorldConfig::default();
    assert_eq!(default.seed(), None);
    assert_eq!(default.preset(), None);
    assert_eq!(default.anvil_import_dir(), None);
}

#[test]
fn explicit_zero_differs_from_omission_and_signed_integer_extremes_are_exact() {
    for (literal, expected) in [
        ("0", 0),
        ("+0", 0),
        ("-0", 0),
        ("1", 1),
        ("-1", -1),
        ("9223372036854775807", i64::MAX),
        ("-9223372036854775808", i64::MIN),
    ] {
        let config: WorldConfig = toml::from_str(&format!("seed = {literal}")).unwrap();
        assert_eq!(config.seed(), Some(expected));
        assert_eq!(config.preset(), None);
        assert_eq!(config.anvil_import_dir(), None);
    }
    let zero: WorldConfig = toml::from_str("seed = 0").unwrap();
    assert_ne!(zero, WorldConfig::default());
}

#[test]
fn numeric_integer_literals_follow_toml_typing_without_text_seed_parsing() {
    for (literal, expected) in [
        ("0x2a", 42),
        ("0o52", 42),
        ("0b101010", 42),
        ("1_000", 1000),
    ] {
        let config: WorldConfig = toml::from_str(&format!("seed = {literal}")).unwrap();
        assert_eq!(config.seed(), Some(expected));
    }
}

#[test]
fn seeds_refuse_strings_floats_random_tokens_other_types_and_overflow() {
    for literal in [
        "'0'",
        "'123'",
        "'-1'",
        "'random'",
        "'North Carolina'",
        "''",
        "0.0",
        "1.0",
        "1e0",
        "-1.0",
        "nan",
        "inf",
        "-inf",
        "true",
        "false",
        "[]",
        "{}",
        "[1]",
        "{ value = 1 }",
        "9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
    ] {
        assert!(
            toml::from_str::<WorldConfig>(&format!("seed = {literal}")).is_err(),
            "invalid seed accepted: {literal}"
        );
    }
}

#[test]
fn only_supported_profiles_are_accepted() {
    let config: WorldConfig = toml::from_str("preset = 'ferrumc_flat'").unwrap();
    assert_eq!(config.preset(), Some(WorldGenerationPreset::FerrumcFlat));
    assert_eq!(config.seed(), None);
    let natural: WorldConfig =
        toml::from_str("preset = 'ferrumc_natural_preview'\nseed = -42").unwrap();
    assert_eq!(
        natural.preset(),
        Some(WorldGenerationPreset::FerrumcNaturalPreview)
    );
    assert_eq!(natural.seed(), Some(-42));
    let grass: WorldConfig =
        toml::from_str("preset = 'ferrumc_natural_grass_preview'\nseed = -42").unwrap();
    assert_eq!(
        grass.preset(),
        Some(WorldGenerationPreset::FerrumcNaturalGrassPreview)
    );
    assert_eq!(grass.seed(), Some(-42));
    let trees: WorldConfig =
        toml::from_str("preset = 'ferrumc_natural_tree_preview'\nseed = -42").unwrap();
    assert_eq!(
        trees.preset(),
        Some(WorldGenerationPreset::FerrumcNaturalTreePreview)
    );
    assert_eq!(trees.seed(), Some(-42));
    for literal in [
        "'flat'",
        "'normal'",
        "'noise'",
        "'ferrumc_noise'",
        "'minecraft:flat'",
        "'minecraft:normal'",
        "'FerrumcFlat'",
        "'FERRUMC_FLAT'",
        "'ferrumc-flat'",
        "' ferrumc_flat'",
        "'ferrumc_flat '",
        "'ferrumc_natural_trees_preview'",
        "'ferrumc_natural_tree_preview '",
        "''",
        "0",
        "true",
        "[]",
        "{}",
    ] {
        assert!(
            toml::from_str::<WorldConfig>(&format!("preset = {literal}")).is_err(),
            "unimplemented/invalid preset accepted: {literal}"
        );
    }
}

#[test]
fn import_seed_and_preset_are_independent_optional_declarations() {
    for imported in [false, true] {
        for seed in [None, Some(0), Some(i64::MIN), Some(i64::MAX)] {
            for (preset, literal) in [
                (None, None),
                (
                    Some(WorldGenerationPreset::FerrumcFlat),
                    Some("ferrumc_flat"),
                ),
                (
                    Some(WorldGenerationPreset::FerrumcNaturalPreview),
                    Some("ferrumc_natural_preview"),
                ),
                (
                    Some(WorldGenerationPreset::FerrumcNaturalGrassPreview),
                    Some("ferrumc_natural_grass_preview"),
                ),
                (
                    Some(WorldGenerationPreset::FerrumcNaturalTreePreview),
                    Some("ferrumc_natural_tree_preview"),
                ),
            ] {
                let mut input = String::from("[world]\n");
                if imported {
                    input.push_str("anvil_import_dir = '/maps/region'\n");
                }
                if let Some(seed) = seed {
                    writeln!(input, "seed = {seed}").unwrap();
                }
                if let Some(literal) = literal {
                    writeln!(input, "preset = '{literal}'").unwrap();
                }
                let config: Configuration = toml::from_str(&input).unwrap();
                assert_eq!(config.world.seed(), seed);
                assert_eq!(config.world.preset(), preset);
                assert_eq!(
                    config.world.anvil_import_dir(),
                    imported.then(|| Path::new("/maps/region"))
                );
            }
        }
    }
}

#[test]
fn typed_construction_preserves_import_and_does_not_resolve_omission() {
    let mut imported = WorldConfig::default();
    // Preserve historical public import-field access while new fields are private.
    imported.anvil_import_dir = Some("/maps/region".into());
    let configured =
        imported.with_generation(Some(i64::MIN), Some(WorldGenerationPreset::FerrumcFlat));
    assert_eq!(configured.seed(), Some(i64::MIN));
    assert_eq!(
        configured.preset(),
        Some(WorldGenerationPreset::FerrumcFlat)
    );
    assert_eq!(
        configured.anvil_import_dir(),
        Some(Path::new("/maps/region"))
    );
    let omitted = configured.with_generation(None, None);
    assert_eq!(omitted.seed(), None);
    assert_eq!(omitted.preset(), None);
    assert_eq!(omitted.anvil_import_dir(), Some(Path::new("/maps/region")));
    let explicit =
        WorldConfig::default().with_generation(Some(0), Some(WorldGenerationPreset::FerrumcFlat));
    let parsed: WorldConfig = toml::from_str("seed = 0\npreset = 'ferrumc_flat'").unwrap();
    assert_eq!(explicit, parsed);
}

#[test]
fn unknown_keys_typos_and_duplicate_choices_refuse_config() {
    for input in [
        "sead = 0",
        "Seed = 0",
        "world_seed = 0",
        "seed_text = '0'",
        "generation = {}",
        "presett = 'ferrumc_flat'",
        "generate_structures = true",
        "seed = 1\nseed = 2",
        "preset = 'ferrumc_flat'\npreset = 'ferrumc_flat'",
        "anvil_import_dirr = '/maps/region'",
    ] {
        assert!(
            toml::from_str::<WorldConfig>(input).is_err(),
            "invalid config accepted: {input}"
        );
    }
}
