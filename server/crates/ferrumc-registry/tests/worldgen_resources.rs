//! Producer contracts and exhaustive checks against the unchanged export.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ferrumc_registry::worldgen::{
    self, DensityFunctionResourceId, NoiseResourceId, NoiseSettingsResourceId, OriginalDefinition,
    ResourceDependency,
};
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

#[path = "../build/worldgen_codegen.rs"]
mod worldgen_codegen;

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/fixtures/minecraft-1.21.8-original")
}
fn fixture_bytes() -> Vec<u8> {
    let bytes = std::fs::read(directory().join("worldgen.json")).unwrap();
    assert_eq!(bytes.len(), 2_530_249);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30"
    );
    bytes
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
#[derive(Deserialize)]
struct RawNoise<'a> {
    #[serde(rename = "firstOctave")]
    first_octave: i32,
    #[serde(borrow)]
    amplitudes: Vec<&'a RawValue>,
}

fn definition_contract(original: &OriginalDefinition, expected: &str) {
    assert_eq!(original.json(), expected, "raw export slice changed");
    let stats = original.stats();
    assert_eq!(stats.bytes(), expected.len());
    assert!(stats.bytes() <= worldgen::MAX_DEFINITION_BYTES);
    assert!(stats.nodes() <= worldgen::MAX_DEFINITION_NODES);
    assert!(stats.maximum_depth() <= worldgen::MAX_DEFINITION_DEPTH);
    assert!(stats.dependency_occurrences() <= worldgen::MAX_DEPENDENCY_OCCURRENCES);
    let dependencies = original.dependencies();
    assert!(dependencies.len() <= worldgen::MAX_DISTINCT_DEPENDENCIES);
    assert!(dependencies.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(stats.dependency_occurrences() >= dependencies.len());
    for dependency in dependencies {
        match dependency {
            ResourceDependency::Noise(id) => assert_eq!(
                NoiseResourceId::from_name(id.resource_location()),
                Some(*id)
            ),
            ResourceDependency::DensityFunction(id) => assert_eq!(
                DensityFunctionResourceId::from_name(id.resource_location()),
                Some(*id)
            ),
            ResourceDependency::Block(name)
            | ResourceDependency::Biome(name)
            | ResourceDependency::DensityFunctionType(name)
            | ResourceDependency::SurfaceRuleType(name)
            | ResourceDependency::SurfaceConditionType(name)
            | ResourceDependency::RandomName(name) => {
                assert!(name.starts_with("minecraft:"));
            }
        }
    }
    // Independent census of the hash-pinned original tree. This does not call
    // the generator's parser/schema validator or reuse its dependency records.
    let tree: serde_json::Value = serde_json::from_str(expected).unwrap();
    let mut census = Census::default();
    census.visit(&tree, 0, "", "");
    assert_eq!(stats.nodes(), census.nodes);
    assert_eq!(stats.maximum_depth(), census.depth);
    assert_eq!(stats.dependency_occurrences(), census.occurrences);
    assert_eq!(
        dependencies
            .iter()
            .map(|dep| dependency_name(*dep))
            .collect::<BTreeSet<_>>(),
        census.dependencies
    );
}

fn dependency_name(dependency: ResourceDependency) -> String {
    match dependency {
        ResourceDependency::Noise(id) => format!("Noise:{}", id.resource_location()),
        ResourceDependency::DensityFunction(id) => {
            format!("DensityFunction:{}", id.resource_location())
        }
        ResourceDependency::Block(name) => format!("Block:{name}"),
        ResourceDependency::Biome(name) => format!("Biome:{name}"),
        ResourceDependency::DensityFunctionType(name) => format!("DensityFunctionType:{name}"),
        ResourceDependency::SurfaceRuleType(name) => format!("SurfaceRuleType:{name}"),
        ResourceDependency::SurfaceConditionType(name) => format!("SurfaceConditionType:{name}"),
        ResourceDependency::RandomName(name) => format!("RandomName:{name}"),
    }
}

#[derive(Default)]
struct Census {
    nodes: usize,
    depth: usize,
    occurrences: usize,
    dependencies: BTreeSet<String>,
}
impl Census {
    /// Counts original valid JSON leaves, independently of admission behavior.
    fn visit(&mut self, value: &serde_json::Value, depth: usize, field: &str, parent_type: &str) {
        self.nodes += 1;
        self.depth = self.depth.max(depth);
        match value {
            serde_json::Value::Object(object) => {
                let typ = object
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                for (key, child) in object {
                    self.nodes += 1;
                    self.depth = self.depth.max(depth + 1);
                    self.visit(child, depth + 1, key, typ);
                }
            }
            serde_json::Value::Array(array) => {
                for child in array {
                    self.visit(child, depth + 1, field, parent_type);
                }
            }
            serde_json::Value::String(name) if name.starts_with("minecraft:") => {
                let kind = match field {
                    "type" => match name.as_str() {
                        "minecraft:bandlands"
                        | "minecraft:block"
                        | "minecraft:condition"
                        | "minecraft:sequence" => "SurfaceRuleType",
                        "minecraft:above_preliminary_surface"
                        | "minecraft:biome"
                        | "minecraft:hole"
                        | "minecraft:noise_threshold"
                        | "minecraft:not"
                        | "minecraft:steep"
                        | "minecraft:stone_depth"
                        | "minecraft:temperature"
                        | "minecraft:vertical_gradient"
                        | "minecraft:water"
                        | "minecraft:y_above" => "SurfaceConditionType",
                        _ => "DensityFunctionType",
                    },
                    "noise" => "Noise",
                    "argument"
                        if matches!(parent_type, "minecraft:shift_a" | "minecraft:shift_b") =>
                    {
                        "Noise"
                    }
                    "Name" => "Block",
                    "biome_is" => "Biome",
                    "random_name" => "RandomName",
                    "argument"
                    | "argument1"
                    | "argument2"
                    | "input"
                    | "shift_x"
                    | "shift_y"
                    | "shift_z"
                    | "coordinate"
                    | "when_in_range"
                    | "when_out_of_range"
                    | "barrier"
                    | "fluid_level_floodedness"
                    | "fluid_level_spread"
                    | "lava"
                    | "temperature"
                    | "vegetation"
                    | "continents"
                    | "erosion"
                    | "depth"
                    | "ridges"
                    | "initial_density_without_jaggedness"
                    | "final_density"
                    | "vein_toggle"
                    | "vein_ridged"
                    | "vein_gap" => "DensityFunction",
                    _ => panic!("unclassified namespaced original string: {field}={name}"),
                };
                self.occurrences += 1;
                self.dependencies.insert(format!("{kind}:{name}"));
            }
            _ => {}
        }
    }
}

#[test]
fn all_102_resources_preserve_export_slices_names_and_noise_bits() {
    let bytes = fixture_bytes();
    let export: Export<'_> = serde_json::from_slice(&bytes).unwrap();
    let mut visited = 0;
    let expected = &export.registries["minecraft:worldgen/noise"].entries;
    assert_eq!(worldgen::noise_resources().len(), 60);
    for (id, (name, raw)) in worldgen::noise_resources().zip(expected) {
        assert_eq!(id.resource_location(), name);
        assert_eq!(NoiseResourceId::from_name(name), Some(id));
        assert_eq!(
            NoiseResourceId::from_name(name.strip_prefix("minecraft:").unwrap()),
            Some(id)
        );
        definition_contract(id.definition(), raw.get());
        let noise: RawNoise<'_> = serde_json::from_str(raw.get()).unwrap();
        assert_eq!(id.parameters().first_octave(), noise.first_octave);
        assert_eq!(id.parameters().amplitudes().len(), noise.amplitudes.len());
        for (actual, original) in id.parameters().amplitudes().iter().zip(noise.amplitudes) {
            // std's decimal parser is independent of serde's f64 parser.
            assert_eq!(
                actual.to_bits(),
                original.get().parse::<f64>().unwrap().to_bits()
            );
        }
        visited += 1;
    }
    let expected = &export.registries["minecraft:worldgen/density_function"].entries;
    assert_eq!(worldgen::density_function_resources().len(), 35);
    for (id, (name, raw)) in worldgen::density_function_resources().zip(expected) {
        assert_eq!(id.resource_location(), name);
        assert_eq!(DensityFunctionResourceId::from_name(name), Some(id));
        assert_eq!(
            DensityFunctionResourceId::from_name(name.strip_prefix("minecraft:").unwrap()),
            Some(id)
        );
        definition_contract(id.definition(), raw.get());
        visited += 1;
    }
    let expected = &export.registries["minecraft:worldgen/noise_settings"].entries;
    assert_eq!(worldgen::noise_settings_resources().len(), 7);
    for (id, (name, raw)) in worldgen::noise_settings_resources().zip(expected) {
        assert_eq!(id.resource_location(), name);
        assert_eq!(NoiseSettingsResourceId::from_name(name), Some(id));
        assert_eq!(
            NoiseSettingsResourceId::from_name(name.strip_prefix("minecraft:").unwrap()),
            Some(id)
        );
        definition_contract(id.definition(), raw.get());
        visited += 1;
    }
    assert_eq!(visited, 102);
}

#[test]
fn every_lookup_rejects_unknown_malformed_and_foreign_names() {
    for name in [
        "",
        "minecraft:",
        "minecraft:missing",
        "other:offset",
        "MINECRAFT:offset",
        "OFFSET",
        " offset",
        "offset ",
        "minecraft:minecraft:offset",
        "/offset",
        "../offset",
        "overworld//factor",
        "minecraft:overworld/./factor",
        "minecraft:overworld/../factor",
        "#minecraft:offset",
    ] {
        assert!(NoiseResourceId::from_name(name).is_none(), "noise: {name}");
        assert!(
            DensityFunctionResourceId::from_name(name).is_none(),
            "density: {name}"
        );
        assert!(
            NoiseSettingsResourceId::from_name(name).is_none(),
            "settings: {name}"
        );
    }
    assert!(NoiseResourceId::from_name("overworld").is_none());
    assert!(DensityFunctionResourceId::from_name("overworld").is_none());
    assert!(NoiseSettingsResourceId::from_name("offset").is_none());
}

#[test]
fn iterators_are_double_ended_exact_size_fused_and_paginatable() {
    fn contracts<T: Ord>(
        mut iter: impl ExactSizeIterator<Item = T> + DoubleEndedIterator + std::iter::FusedIterator,
    ) {
        let count = iter.len();
        let first = iter.next().unwrap();
        let last = iter.next_back().unwrap();
        assert!(first < last);
        assert_eq!(iter.len(), count - 2);
        assert_eq!(iter.by_ref().count(), count - 2);
        assert!(iter.next().is_none());
        assert!(iter.next_back().is_none());
        assert_eq!(iter.len(), 0);
    }
    contracts(worldgen::noise_resources());
    contracts(worldgen::density_function_resources());
    contracts(worldgen::noise_settings_resources());
    assert_eq!(
        worldgen::density_function_resources()
            .skip(4)
            .take(3)
            .count(),
        3
    );
    assert_eq!(
        worldgen::noise_resources()
            .map(NoiseResourceId::resource_location)
            .collect::<BTreeSet<_>>()
            .len(),
        60
    );
}

#[test]
fn external_dependencies_are_separate_from_resource_references() {
    let settings = NoiseSettingsResourceId::from_name("overworld").unwrap();
    let deps = settings.definition().dependencies();
    assert!(deps
        .iter()
        .any(|dep| matches!(dep, ResourceDependency::Block("minecraft:water"))));
    assert!(deps
        .iter()
        .any(|dep| matches!(dep, ResourceDependency::Biome("minecraft:desert"))));
    assert!(deps.iter().any(|dep| matches!(
        dep,
        ResourceDependency::RandomName("minecraft:bedrock_floor")
    )));
    assert!(deps
        .iter()
        .any(|dep| matches!(dep, ResourceDependency::Noise(_))));
    assert!(deps
        .iter()
        .any(|dep| matches!(dep, ResourceDependency::DensityFunction(_))));
    assert!(deps.iter().any(|dep| matches!(
        dep,
        ResourceDependency::SurfaceRuleType("minecraft:sequence")
    )));
    assert!(deps.iter().any(|dep| matches!(
        dep,
        ResourceDependency::SurfaceConditionType("minecraft:noise_threshold")
    )));
    let shift = DensityFunctionResourceId::from_name("shift_x").unwrap();
    assert!(shift
        .definition()
        .dependencies()
        .contains(&ResourceDependency::Noise(
            NoiseResourceId::from_name("offset").unwrap()
        )));
    assert!(!shift
        .definition()
        .dependencies()
        .iter()
        .any(|dep| matches!(dep, ResourceDependency::DensityFunction(_))));
}

fn blocks() -> worldgen_codegen::BlockProperties {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/blocks.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|block| {
            let properties = block["states"]
                .as_array()
                .unwrap()
                .iter()
                .map(|property| {
                    let values = if property["type"] == "bool" {
                        vec!["true".to_owned(), "false".to_owned()]
                    } else {
                        property["values"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|value| value.as_str().unwrap().to_owned())
                            .collect()
                    };
                    (property["name"].as_str().unwrap().to_owned(), values)
                })
                .collect();
            (
                format!("minecraft:{}", block["name"].as_str().unwrap()),
                properties,
            )
        })
        .collect()
}

#[test]
fn deterministic_generator_preserves_runtime_dependency_sets_and_stats() {
    let data = worldgen_codegen::load(&directory(), &blocks()).unwrap();
    let first = worldgen_codegen::emit(&data).unwrap();
    assert_eq!(first, worldgen_codegen::emit(&data).unwrap());
    for (definitions, originals) in [
        (
            &data.noise,
            worldgen::noise_resources()
                .map(NoiseResourceId::definition)
                .collect::<Vec<_>>(),
        ),
        (
            &data.density,
            worldgen::density_function_resources()
                .map(DensityFunctionResourceId::definition)
                .collect(),
        ),
        (
            &data.settings,
            worldgen::noise_settings_resources()
                .map(NoiseSettingsResourceId::definition)
                .collect(),
        ),
    ] {
        for (definition, original) in definitions.iter().zip(originals) {
            assert_eq!(definition.json, original.json());
            assert_eq!(definition.stats.nodes, original.stats().nodes());
            assert_eq!(definition.stats.depth, original.stats().maximum_depth());
            assert_eq!(
                definition.occurrences,
                original.stats().dependency_occurrences()
            );
            let generated: Vec<_> = definition
                .dependencies
                .iter()
                .map(|dep| format!("{:?}:{}", dep.kind, dep.name))
                .collect();
            let runtime: Vec<_> = original
                .dependencies()
                .iter()
                .map(|dep| match dep {
                    ResourceDependency::Noise(id) => format!("Noise:{}", id.resource_location()),
                    ResourceDependency::DensityFunction(id) => {
                        format!("DensityFunction:{}", id.resource_location())
                    }
                    ResourceDependency::Block(name) => format!("Block:{name}"),
                    ResourceDependency::Biome(name) => format!("Biome:{name}"),
                    ResourceDependency::DensityFunctionType(name) => {
                        format!("DensityFunctionType:{name}")
                    }
                    ResourceDependency::SurfaceRuleType(name) => format!("SurfaceRuleType:{name}"),
                    ResourceDependency::SurfaceConditionType(name) => {
                        format!("SurfaceConditionType:{name}")
                    }
                    ResourceDependency::RandomName(name) => format!("RandomName:{name}"),
                })
                .collect();
            assert_eq!(generated, runtime);
        }
    }
}

fn mutate_definition(family: &str, name: &str, definition: serde_json::Value) -> Vec<u8> {
    let mut tree: serde_json::Value = serde_json::from_slice(&fixture_bytes()).unwrap();
    tree["registries"][format!("minecraft:worldgen/{family}")]["entries"][name] = definition;
    serde_json::to_vec(&tree).unwrap()
}
fn rejected_density(value: serde_json::Value, message: &str) {
    let bytes = mutate_definition("density_function", "minecraft:zero", value);
    let error = worldgen_codegen::admit(&bytes, &blocks()).err().unwrap();
    assert!(error.to_string().contains(message), "{error}");
}

#[test]
fn generator_separates_clamp_direct_shift_noise_and_unary_density_codecs() {
    use serde_json::json;
    rejected_density(
        json!({"type":"minecraft:clamp","input":"minecraft:y","min":-1,"max":1}),
        "Direct density codec",
    );
    rejected_density(
        json!({"type":"minecraft:shift_a","argument":"minecraft:y"}),
        "unknown noise reference",
    );
    rejected_density(
        json!({"type":"minecraft:abs","argument":"minecraft:offset"}),
        "unknown density reference",
    );
    // Corresponding valid paths are checked separately, then the cycle gate runs.
    for value in [
        json!({"type":"minecraft:clamp","input":0,"min":-1,"max":1}),
        json!({"type":"minecraft:shift_a","argument":"minecraft:offset"}),
        json!({"type":"minecraft:abs","argument":"minecraft:y"}),
    ] {
        assert!(worldgen_codegen::admit(
            &mutate_definition("density_function", "minecraft:zero", value),
            &blocks()
        )
        .is_ok());
    }
}

#[test]
fn generator_rejects_unknown_shape_name_reference_cycle_and_noise_parameters() {
    use serde_json::json;
    rejected_density(
        json!({"type":"minecraft:future_noise"}),
        "unknown builtin density form",
    );
    rejected_density(
        json!({"type":"minecraft:abs","argument":"minecraft:missing"}),
        "unknown density reference",
    );
    rejected_density(
        json!({"type":"minecraft:abs","argument":"other:y"}),
        "malformed/foreign reference",
    );
    rejected_density(
        json!({"type":"minecraft:abs","argument":"#minecraft:y"}),
        "unsupported tag",
    );
    rejected_density(
        json!({"type":"minecraft:abs","argument":"minecraft:zero"}),
        "cyclic density",
    );
    rejected_density(
        json!({"type":"minecraft:abs","argument":0,"extra":false}),
        "unknown or missing schema field",
    );
    for noise in [
        json!({"firstOctave":-17,"amplitudes":[1]}),
        json!({"firstOctave":-1,"amplitudes":[]}),
        json!({"firstOctave":-1,"amplitudes":[3]}),
        json!({"firstOctave":-1,"amplitudes":[0]}),
        json!({"firstOctave":-1,"amplitudes":vec![1;17]}),
    ] {
        assert!(worldgen_codegen::admit(
            &mutate_definition("noise", "minecraft:offset", noise),
            &blocks()
        )
        .is_err());
    }
}

#[test]
fn generator_rejects_duplicate_keys_resource_count_and_foreign_family() {
    let bytes = fixture_bytes();
    let raw = std::str::from_utf8(&bytes).unwrap();
    let duplicate = raw.replacen(
        "\"registries\": {",
        "\"registries\": {}, \"registries\": {",
        1,
    );
    assert!(worldgen_codegen::admit(duplicate.as_bytes(), &blocks())
        .err()
        .unwrap()
        .to_string()
        .contains("duplicate JSON key"));
    let duplicate = raw.replacen(
        "\"firstOctave\":",
        "\"firstOctave\": -3, \"firstOctave\":",
        1,
    );
    assert!(worldgen_codegen::admit(duplicate.as_bytes(), &blocks())
        .err()
        .unwrap()
        .to_string()
        .contains("duplicate JSON key"));
    let duplicate = raw.replacen(
        "\"minecraft:offset\": {",
        "\"minecraft:offset\": {}, \"minecraft:offset\": {",
        1,
    );
    assert!(worldgen_codegen::admit(duplicate.as_bytes(), &blocks())
        .err()
        .unwrap()
        .to_string()
        .contains("duplicate JSON key"));
    let mut tree: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    tree["registries"]["minecraft:worldgen/noise"]["entries"]
        .as_object_mut()
        .unwrap()
        .remove("minecraft:offset");
    assert!(
        worldgen_codegen::admit(&serde_json::to_vec(&tree).unwrap(), &blocks())
            .err()
            .unwrap()
            .to_string()
            .contains("resource count")
    );
    let foreign = raw.replacen("minecraft:worldgen/noise\"", "other:worldgen/noise\"", 1);
    assert!(worldgen_codegen::admit(foreign.as_bytes(), &blocks()).is_err());
    let malformed = raw.replacen("minecraft:offset\": {", "minecraft:../offset\": {", 1);
    assert!(worldgen_codegen::admit(malformed.as_bytes(), &blocks())
        .err()
        .unwrap()
        .to_string()
        .contains("malformed or foreign resource name"));
}

#[test]
fn generator_pins_and_provenance_refuse_content_or_version_drift() {
    let original = fixture_bytes();
    let mut changed = original.clone();
    changed[0] = b' ';
    let pin = (
        2_530_249,
        "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30",
    );
    assert!(worldgen_codegen::verify_pin(&original, pin, "fixture").is_ok());
    assert!(matches!(
        worldgen_codegen::verify_pin(&changed, pin, "fixture"),
        Err(worldgen_codegen::BuildError::Pin(_))
    ));
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory().join("manifest.json")).unwrap()).unwrap();
    let version = std::fs::read(directory().join("version-manifest.json")).unwrap();
    manifest["minecraft"]["data_version"] = serde_json::json!(4438);
    assert!(
        worldgen_codegen::provenance(&serde_json::to_vec(&manifest).unwrap(), &version).is_err()
    );
}

#[test]
fn generator_rejects_whole_fixture_and_selected_raw_byte_oversize() {
    assert!(
        worldgen_codegen::admit(&vec![b' '; 4 * 1024 * 1024 + 1], &blocks())
            .err()
            .unwrap()
            .to_string()
            .contains("byte limit")
    );
    let bytes = fixture_bytes();
    let raw = std::str::from_utf8(&bytes).unwrap();
    let padding = " ".repeat(128 * 1024);
    // Legal whitespace inside a selected noise definition exceeds its raw cap,
    // while the complete valid tree remains below the independent whole cap.
    let noise_start = raw.find("\"minecraft:worldgen/noise\":").unwrap();
    let padded = raw[..noise_start].to_owned()
        + &raw[noise_start..].replacen(
            "\"amplitudes\": [",
            &format!("\"amplitudes\": [{padding}"),
            1,
        );
    assert!(padded.len() < 4 * 1024 * 1024);
    assert!(worldgen_codegen::admit(padded.as_bytes(), &blocks())
        .err()
        .unwrap()
        .to_string()
        .contains("byte limit"));
}
