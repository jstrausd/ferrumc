//! Exhaustive producer contracts against independent original export parsing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ferrumc_registry::biome::{
    self, BiomeParameterEntry, BiomeParametersResourceId, BiomeResourceId, OriginalBiomeDefinition,
};
use serde::Deserialize;
use serde_json::{value::RawValue, Value};
use sha2::{Digest, Sha256};

#[path = "../build/biome_codegen.rs"]
mod biome_codegen;
#[path = "../build/worldgen_json.rs"]
mod worldgen_json;

fn original_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/fixtures/minecraft-1.21.8-original")
}
fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}
#[derive(Deserialize)]
struct Export<'a> {
    #[serde(borrow)]
    registries: BTreeMap<String, Family<'a>>,
}
#[derive(Deserialize)]
struct Family<'a> {
    #[serde(borrow)]
    entries: BTreeMap<String, &'a RawValue>,
}

fn independent_stats(value: &Value, depth: usize) -> (usize, usize) {
    let mut nodes = 1;
    let mut maximum_depth = depth;
    match value {
        Value::Object(object) => {
            for value in object.values() {
                let (child_nodes, child_depth) = independent_stats(value, depth + 1);
                nodes += 1 + child_nodes;
                maximum_depth = maximum_depth.max(child_depth).max(depth + 1);
            }
        }
        Value::Array(array) => {
            for value in array {
                let (child_nodes, child_depth) = independent_stats(value, depth + 1);
                nodes += child_nodes;
                maximum_depth = maximum_depth.max(child_depth);
            }
        }
        _ => {}
    }
    (nodes, maximum_depth)
}
fn check_definition(definition: &OriginalBiomeDefinition, raw: &str) {
    assert_eq!(definition.json(), raw, "exact export slice changed");
    let value: Value = serde_json::from_str(raw).unwrap();
    let (nodes, depth) = independent_stats(&value, 0);
    let stats = definition.stats();
    assert_eq!(stats.bytes(), raw.len());
    assert_eq!(stats.nodes(), nodes);
    assert_eq!(stats.maximum_depth(), depth);
    assert!(stats.bytes() <= biome::MAX_DEFINITION_BYTES);
    assert!(nodes <= biome::MAX_DEFINITION_NODES);
    assert!(depth <= biome::MAX_DEFINITION_DEPTH);
}

#[test]
fn all_65_biome_and_two_preset_definitions_preserve_exact_export_slices_and_stats() {
    let bytes = std::fs::read(original_dir().join("worldgen.json")).unwrap();
    assert_eq!(bytes.len(), 2_530_249);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30"
    );
    let original: Export<'_> = serde_json::from_slice(&bytes).unwrap();
    let biomes = &original.registries["minecraft:worldgen/biome"].entries;
    let presets =
        &original.registries["minecraft:worldgen/multi_noise_biome_source_parameter_list"].entries;
    assert_eq!(biomes.len(), 65);
    assert_eq!(biome::biome_resources().len(), 65);
    assert_eq!(presets.len(), 2);
    assert_eq!(biome::biome_parameters_resources().len(), 2);
    for (resource, (key, value)) in biome::biome_resources().zip(biomes) {
        assert_eq!(resource.resource_location(), key);
        assert_eq!(BiomeResourceId::from_name(key), Some(resource));
        assert_eq!(BiomeResourceId::from_name(&key[10..]), Some(resource));
        check_definition(resource.definition(), value.get());
        let definition: Value = serde_json::from_str(value.get()).unwrap();
        // Full data must retain generation and spawn settings, not NETWORK data.
        for field in ["carvers", "features", "spawners", "spawn_costs", "effects"] {
            assert!(definition.get(field).is_some(), "missing {field} for {key}");
        }
    }
    for (resource, (key, value)) in biome::biome_parameters_resources().zip(presets) {
        assert_eq!(resource.resource_location(), key);
        assert_eq!(BiomeParametersResourceId::from_name(key), Some(resource));
        assert_eq!(
            BiomeParametersResourceId::from_name(&key[10..]),
            Some(resource)
        );
        check_definition(resource.definition(), value.get());
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalEntry {
    index: usize,
    biome: String,
    axes: [[i64; 2]; 6],
    offset: i64,
}

#[test]
fn every_original_ordered_quantized_entry_and_biome_key_matches() {
    let bytes = std::fs::read(data_dir().join("biome-parameters-1.21.8.json")).unwrap();
    assert_eq!(bytes.len(), 1_072_932);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "4e9ee584071de6822b9e0e8030eb0666814a3c61b77f8e0ce611f9757508f971"
    );
    let expected: BTreeMap<String, Vec<OriginalEntry>> = serde_json::from_slice(&bytes).unwrap();
    let mut checked = 0;
    for (preset, count, unique) in [("overworld", 7593, 54), ("nether", 5, 5)] {
        let resource = BiomeParametersResourceId::from_name(preset).unwrap();
        let original = &expected[resource.resource_location()];
        let actual = resource.parameters();
        assert_eq!(original.len(), count);
        assert_eq!(actual.len(), count);
        assert_eq!(
            actual
                .iter()
                .map(|row| row.biome())
                .collect::<BTreeSet<_>>()
                .len(),
            unique
        );
        for (index, (row, expected)) in actual.iter().zip(original).enumerate() {
            assert_eq!(expected.index, index);
            assert_eq!(row.biome().resource_location(), expected.biome);
            assert_eq!(row.point().offset(), expected.offset);
            for (actual, [minimum, maximum]) in row.point().axes().iter().zip(expected.axes) {
                assert_eq!(actual.minimum(), minimum);
                assert_eq!(actual.maximum(), maximum);
            }
            let point = row.point();
            assert_eq!(
                *point.axes(),
                [
                    point.temperature(),
                    point.humidity(),
                    point.continentalness(),
                    point.erosion(),
                    point.depth(),
                    point.weirdness(),
                ]
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 7598);
    let nether = BiomeParametersResourceId::from_name("nether").unwrap();
    assert_eq!(
        nether
            .parameters()
            .iter()
            .map(|row| row.biome().resource_location())
            .collect::<Vec<_>>(),
        [
            "minecraft:nether_wastes",
            "minecraft:soul_sand_valley",
            "minecraft:crimson_forest",
            "minecraft:warped_forest",
            "minecraft:basalt_deltas"
        ]
    );
}

#[test]
fn immutable_lookup_iteration_and_layout_contracts() {
    let mut biomes = biome::biome_resources();
    assert_eq!(
        biomes.next().unwrap().resource_location(),
        "minecraft:badlands"
    );
    assert_eq!(
        biomes.next_back().unwrap().resource_location(),
        "minecraft:wooded_badlands"
    );
    assert_eq!(biomes.len(), 63);
    let mut presets = biome::biome_parameters_resources();
    assert_eq!(
        presets.next().unwrap().resource_location(),
        "minecraft:nether"
    );
    assert_eq!(
        presets.next_back().unwrap().resource_location(),
        "minecraft:overworld"
    );
    assert_eq!(presets.next(), None);
    assert_eq!(presets.next_back(), None);
    for name in [
        "",
        "minecraft:",
        "PLAINS",
        "minecraft:PLAINS",
        " plains",
        "plains ",
        "notch:plains",
        "minecraft:minecraft:plains",
        "minecraft:../plains",
        "unknown",
    ] {
        assert_eq!(BiomeResourceId::from_name(name), None, "{name}");
    }
    for name in [
        "",
        "minecraft:",
        "OVERWORLD",
        "minecraft:OVERWORLD",
        " overworld",
        "notch:overworld",
        "minecraft:end",
        "end",
        "caves",
        "floating_islands",
    ] {
        assert_eq!(BiomeParametersResourceId::from_name(name), None, "{name}");
    }
    assert_eq!(
        BiomeResourceId::from_name(biome::PLAINS)
            .unwrap()
            .resource_location(),
        biome::PLAINS
    );
    assert!(std::mem::size_of::<BiomeParameterEntry>() <= 112);
    assert!(std::mem::size_of::<BiomeParameterEntry>() * 7598 <= 850_976);
    // Every return path borrows the same static data; no table is rebuilt.
    for preset in biome::biome_parameters_resources() {
        assert!(std::ptr::eq(preset.parameters(), preset.parameters()));
        assert!(std::ptr::eq(preset.definition(), preset.definition()));
    }
}

#[test]
fn build_admission_and_emission_are_deterministic() {
    let first = biome_codegen::load(&original_dir(), &data_dir()).unwrap();
    let second = biome_codegen::load(&original_dir(), &data_dir()).unwrap();
    assert_eq!(
        biome_codegen::emit(&first).unwrap(),
        biome_codegen::emit(&second).unwrap()
    );
}
