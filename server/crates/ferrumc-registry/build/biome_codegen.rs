//! Build-only immutable biome definitions and ordered original climate data.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};
use std::fs::File;
use std::io::Read as _;
use std::path::Path;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

use crate::worldgen_json as bounded_json;

const WORLDGEN_PIN: (usize, &str) = (
    2_530_249,
    "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30",
);
const MANIFEST_PIN: (usize, &str) = (
    7061,
    "d5b4506f5749d114d7b54c50231ee00f6f15d14b2826f8c39489df06932ed9ee",
);
const VERSION_PIN: (usize, &str) = (
    43_719,
    "3bf5cdd444dfa89199fe4f8a92b6c79f4e4fa8977ff52029c3acd99378506fad",
);
const PARAMETERS_PIN: (usize, &str) = (
    1_072_932,
    "4e9ee584071de6822b9e0e8030eb0666814a3c61b77f8e0ce611f9757508f971",
);
const PROVENANCE_PIN: (usize, &str) = (
    1389,
    "f24ff1017170e1623904e3570dfa4371328edd895b5f94b09ea717a22fe1464f",
);
const WHOLE_LIMITS: bounded_json::Limits = bounded_json::Limits {
    bytes: 4 * 1024 * 1024,
    nodes: 131_072,
    depth: 32,
    entries: 1024,
    string_bytes: 256,
};
const DEFINITION_LIMITS: bounded_json::Limits = bounded_json::Limits {
    bytes: 128 * 1024,
    nodes: 4096,
    depth: 32,
    entries: 256,
    string_bytes: 256,
};
const CENSUS_BYTES: usize = 2 * 1024 * 1024;
const CENSUS_NODES: usize = 262_144;
const CENSUS_DEPTH: usize = 8;
const CENSUS_STRING_BYTES: usize = 64;
const PRESETS: &[(&str, usize)] = &[("minecraft:nether", 5), ("minecraft:overworld", 7593)];

#[derive(Debug)]
pub(crate) enum BuildError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Bounds(&'static str),
    Pin(&'static str),
    Invalid(&'static str),
    Format(fmt::Error),
}
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "biome input I/O: {e}"),
            Self::Json(e) => write!(f, "biome JSON admission: {e}"),
            Self::Bounds(e) => write!(f, "biome input bound: {e}"),
            Self::Pin(e) => write!(f, "biome provenance pin: {e}"),
            Self::Invalid(e) => write!(f, "biome resource admission: {e}"),
            Self::Format(e) => write!(f, "biome source formatting: {e}"),
        }
    }
}
impl std::error::Error for BuildError {}

pub(crate) struct Definition {
    name: String,
    json: String,
    stats: bounded_json::Stats,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Entry {
    pub axes: [[i64; 2]; 6],
    pub offset: i64,
    pub biome: u8,
}
pub(crate) struct Data {
    biomes: Vec<Definition>,
    presets: Vec<Definition>,
    parameters: BTreeMap<String, Vec<Entry>>,
}

fn require(condition: bool, detail: &'static str) -> Result<(), BuildError> {
    if condition {
        Ok(())
    } else {
        Err(BuildError::Invalid(detail))
    }
}
fn canonical_name(name: &str) -> bool {
    name.len() <= 256
        && name.strip_prefix("minecraft:").is_some_and(|path| {
            !path.is_empty()
                && path.bytes().all(|b| {
                    b.is_ascii_lowercase()
                        || b.is_ascii_digit()
                        || matches!(b, b'_' | b'.' | b'/' | b'-')
                })
                && path
                    .split('/')
                    .all(|segment| !matches!(segment, "" | "." | ".."))
        })
}
fn read_bounded(path: &Path, cap: usize) -> Result<Vec<u8>, BuildError> {
    let file = File::open(path).map_err(BuildError::Io)?;
    if file.metadata().map_err(BuildError::Io)?.len() > cap as u64 {
        return Err(BuildError::Bounds("file bytes"));
    }
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(BuildError::Io)?;
    if bytes.len() > cap {
        return Err(BuildError::Bounds("file grew beyond cap"));
    }
    Ok(bytes)
}
fn verify_pin(bytes: &[u8], pin: (usize, &str), name: &'static str) -> Result<(), BuildError> {
    if bytes.len() != pin.0 || format!("{:x}", Sha256::digest(bytes)) != pin.1 {
        return Err(BuildError::Pin(name));
    }
    Ok(())
}

pub(crate) fn load(original: &Path, data: &Path) -> Result<Data, BuildError> {
    let export = read_bounded(&original.join("worldgen.json"), WHOLE_LIMITS.bytes)?;
    let manifest = read_bounded(&original.join("manifest.json"), 16 * 1024)?;
    let version = read_bounded(&original.join("version-manifest.json"), 64 * 1024)?;
    let parameters = read_bounded(&data.join("biome-parameters-1.21.8.json"), CENSUS_BYTES)?;
    let provenance = read_bounded(
        &data.join("biome-parameters-1.21.8.provenance.json"),
        16 * 1024,
    )?;
    verify_pin(&export, WORLDGEN_PIN, "worldgen.json")?;
    verify_pin(&manifest, MANIFEST_PIN, "manifest.json")?;
    verify_pin(&version, VERSION_PIN, "version-manifest.json")?;
    verify_pin(&parameters, PARAMETERS_PIN, "biome parameters")?;
    verify_pin(&provenance, PROVENANCE_PIN, "biome parameter provenance")?;
    // Fixed pins admit the independently reviewed full definitions and census.
    // Structural tests call admission separately; production never skips pins.
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest).map_err(BuildError::Json)?;
    let receipts = &manifest["source"]["datapack_categories"];
    require(
        receipts["worldgen/biome"]["count"] == 65
            && receipts["worldgen/biome"]["bytes"] == 248_743
            && receipts["worldgen/biome"]["path_bytes_sha256_index"]
                == "f0184350ddabcda7f49af499b1e9177dd8629641c0bd1360334edf65a5dc5b21"
            && receipts["worldgen/multi_noise_biome_source_parameter_list"]["count"] == 2
            && receipts["worldgen/multi_noise_biome_source_parameter_list"]["bytes"] == 71
            && receipts["worldgen/multi_noise_biome_source_parameter_list"]
                ["path_bytes_sha256_index"]
                == "5d34ab0c3ee9f42616182eaac5d1483a53d36b56c5d4ccff1976b3507851f68e",
        "original resource receipts",
    )?;
    admit(&export, &parameters)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export<'a> {
    #[serde(borrow)]
    registries: BTreeMap<String, Family<'a>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Family<'a> {
    #[serde(borrow)]
    entries: BTreeMap<String, &'a RawValue>,
}

pub(crate) fn admit(export: &[u8], parameters: &[u8]) -> Result<Data, BuildError> {
    bounded_json::parse(export, WHOLE_LIMITS).map_err(BuildError::Json)?;
    let export: Export<'_> = serde_json::from_slice(export).map_err(BuildError::Json)?;
    let mut biomes = Vec::new();
    let mut presets = Vec::new();
    for (family, count, destination) in [
        ("minecraft:worldgen/biome", 65, &mut biomes),
        (
            "minecraft:worldgen/multi_noise_biome_source_parameter_list",
            2,
            &mut presets,
        ),
    ] {
        let Some(entries) = export.registries.get(family).map(|family| &family.entries) else {
            return Err(BuildError::Invalid("missing original resource family"));
        };
        require(entries.len() == count, "original resource count")?;
        for (name, raw) in entries {
            require(canonical_name(name), "resource name")?;
            let (tree, stats) = bounded_json::parse(raw.get().as_bytes(), DEFINITION_LIMITS)
                .map_err(BuildError::Json)?;
            require(tree.is_object(), "definition object")?;
            if count == 2 {
                require(
                    PRESETS.iter().any(|(preset, _)| *preset == name)
                        && tree.as_object().is_some_and(|object| object.len() == 1)
                        && tree["preset"].as_str() == Some(name),
                    "original preset definition",
                )?;
            }
            destination.push(Definition {
                name: name.clone(),
                json: raw.get().to_owned(),
                stats,
            });
        }
    }
    let keys = biomes
        .iter()
        .enumerate()
        .map(|(index, definition)| (definition.name.clone(), index as u8))
        .collect();
    let parameters = admit_parameters(parameters, &keys)?;
    Ok(Data {
        biomes,
        presets,
        parameters,
    })
}

#[derive(Default)]
struct CensusBudget {
    nodes: usize,
}
impl CensusBudget {
    fn charge<E: serde::de::Error>(&mut self, depth: usize) -> Result<(), E> {
        if depth > CENSUS_DEPTH || self.nodes == CENSUS_NODES {
            return Err(E::custom("census node/depth cap"));
        }
        self.nodes += 1;
        Ok(())
    }
}

pub(crate) fn admit_parameters(
    bytes: &[u8],
    keys: &BTreeMap<String, u8>,
) -> Result<BTreeMap<String, Vec<Entry>>, BuildError> {
    if bytes.len() > CENSUS_BYTES {
        return Err(BuildError::Bounds("census bytes"));
    }
    let mut budget = CensusBudget::default();
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let result = RootSeed {
        budget: &mut budget,
        keys,
    }
    .deserialize(&mut decoder)
    .map_err(BuildError::Json)?;
    decoder.end().map_err(BuildError::Json)?;
    Ok(result)
}

struct RootSeed<'a> {
    budget: &'a mut CensusBudget,
    keys: &'a BTreeMap<String, u8>,
}
impl<'de> DeserializeSeed<'de> for RootSeed<'_> {
    type Value = BTreeMap<String, Vec<Entry>>;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.budget.charge::<D::Error>(0)?;
        decoder.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for RootSeed<'_> {
    type Value = BTreeMap<String, Vec<Entry>>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("exactly the two original biome parameter presets")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut result = BTreeMap::new();
        while let Some(name) = map.next_key_seed(TextSeed {
            budget: self.budget,
            depth: 1,
        })? {
            let Some((_, count)) = PRESETS.iter().find(|(preset, _)| *preset == name) else {
                return Err(A::Error::custom("unknown census preset"));
            };
            if result.contains_key(&name) {
                return Err(A::Error::custom("duplicate decoded census preset"));
            }
            let rows = map.next_value_seed(RowsSeed {
                budget: self.budget,
                keys: self.keys,
                count: *count,
            })?;
            result.insert(name, rows);
        }
        if result.len() != 2 {
            return Err(A::Error::custom("missing census preset"));
        }
        Ok(result)
    }
}

struct RowsSeed<'a> {
    budget: &'a mut CensusBudget,
    keys: &'a BTreeMap<String, u8>,
    count: usize,
}
impl<'de> DeserializeSeed<'de> for RowsSeed<'_> {
    type Value = Vec<Entry>;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.budget.charge::<D::Error>(1)?;
        decoder.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for RowsSeed<'_> {
    type Value = Vec<Entry>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the original fixed-length ordered parameter array")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut rows = Vec::new();
        for index in 0..self.count {
            let Some(row) = sequence.next_element_seed(RowSeed {
                budget: self.budget,
                keys: self.keys,
                index,
            })?
            else {
                return Err(A::Error::custom("census row count too small"));
            };
            rows.push(row);
        }
        // The refusal seed cannot materialize an extra row or nested payload.
        sequence.next_element_seed(EndSeed("census row count too large"))?;
        Ok(rows)
    }
}

struct RowSeed<'a> {
    budget: &'a mut CensusBudget,
    keys: &'a BTreeMap<String, u8>,
    index: usize,
}
impl<'de> DeserializeSeed<'de> for RowSeed<'_> {
    type Value = Entry;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.budget.charge::<D::Error>(2)?;
        decoder.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for RowSeed<'_> {
    type Value = Entry;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a four-field original quantized parameter entry")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut seen = 0_u8;
        let mut biome = None;
        let mut axes = None;
        let mut offset = None;
        while let Some(field) = map.next_key_seed(TextSeed {
            budget: self.budget,
            depth: 3,
        })? {
            let bit = match field.as_str() {
                "index" => 1,
                "biome" => 2,
                "axes" => 4,
                "offset" => 8,
                _ => return Err(A::Error::custom("unknown census row field")),
            };
            if seen & bit != 0 {
                return Err(A::Error::custom("duplicate decoded census row field"));
            }
            seen |= bit;
            match field.as_str() {
                "index" => {
                    let value = map.next_value_seed(IntegerSeed {
                        budget: self.budget,
                        depth: 3,
                    })?;
                    if usize::try_from(value).ok() != Some(self.index) {
                        return Err(A::Error::custom("census source index is not contiguous"));
                    }
                }
                "biome" => {
                    let name = map.next_value_seed(TextSeed {
                        budget: self.budget,
                        depth: 3,
                    })?;
                    biome =
                        Some(*self.keys.get(&name).ok_or_else(|| {
                            A::Error::custom("unknown or noncanonical census biome")
                        })?);
                }
                "axes" => {
                    axes = Some(map.next_value_seed(AxesSeed {
                        budget: self.budget,
                    })?);
                }
                "offset" => {
                    let value = map.next_value_seed(IntegerSeed {
                        budget: self.budget,
                        depth: 3,
                    })?;
                    if !(0..=10_000).contains(&value) {
                        return Err(A::Error::custom(
                            "census offset outside original codec domain",
                        ));
                    }
                    offset = Some(value);
                }
                _ => return Err(A::Error::custom("unknown census row field")),
            }
        }
        match (seen, biome, axes, offset) {
            (15, Some(biome), Some(axes), Some(offset)) => Ok(Entry {
                axes,
                offset,
                biome,
            }),
            _ => Err(A::Error::custom("missing census row field")),
        }
    }
}

struct AxesSeed<'a> {
    budget: &'a mut CensusBudget,
}
impl<'de> DeserializeSeed<'de> for AxesSeed<'_> {
    type Value = [[i64; 2]; 6];
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.budget.charge::<D::Error>(3)?;
        decoder.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for AxesSeed<'_> {
    type Value = [[i64; 2]; 6];
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("exactly six quantized climate spans")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut axes = [[0; 2]; 6];
        for axis in &mut axes {
            *axis = sequence
                .next_element_seed(SpanSeed {
                    budget: self.budget,
                })?
                .ok_or_else(|| A::Error::custom("too few census axes"))?;
        }
        sequence.next_element_seed(EndSeed("too many census axes"))?;
        Ok(axes)
    }
}

struct SpanSeed<'a> {
    budget: &'a mut CensusBudget,
}
impl<'de> DeserializeSeed<'de> for SpanSeed<'_> {
    type Value = [i64; 2];
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.budget.charge::<D::Error>(4)?;
        decoder.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for SpanSeed<'_> {
    type Value = [i64; 2];
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a two-integer ordered climate interval")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut span = [0; 2];
        for endpoint in &mut span {
            *endpoint = sequence
                .next_element_seed(IntegerSeed {
                    budget: self.budget,
                    depth: 5,
                })?
                .ok_or_else(|| A::Error::custom("too few census span endpoints"))?;
            if !(-20_000..=20_000).contains(endpoint) {
                return Err(A::Error::custom(
                    "census span outside original codec domain",
                ));
            }
        }
        sequence.next_element_seed(EndSeed("too many census span endpoints"))?;
        if span[0] > span[1] {
            return Err(A::Error::custom("reversed census span"));
        }
        Ok(span)
    }
}

struct IntegerSeed<'a> {
    budget: &'a mut CensusBudget,
    depth: usize,
}
impl<'de> DeserializeSeed<'de> for IntegerSeed<'_> {
    type Value = i64;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.budget.charge::<D::Error>(self.depth)?;
        decoder.deserialize_i64(self)
    }
}
impl Visitor<'_> for IntegerSeed<'_> {
    type Value = i64;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a quantized signed integer, without float coercion")
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<i64, E> {
        Ok(value)
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<i64, E> {
        i64::try_from(value).map_err(|_| E::custom("census integer overflow"))
    }
}

struct TextSeed<'a> {
    budget: &'a mut CensusBudget,
    depth: usize,
}
impl<'de> DeserializeSeed<'de> for TextSeed<'_> {
    type Value = String;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.budget.charge::<D::Error>(self.depth)?;
        decoder.deserialize_str(self)
    }
}
impl Visitor<'_> for TextSeed<'_> {
    type Value = String;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a bounded census field or canonical resource name")
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
        if value.len() > CENSUS_STRING_BYTES {
            return Err(E::custom("census decoded string cap"));
        }
        Ok(value.to_owned())
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<String, E> {
        if value.len() > CENSUS_STRING_BYTES {
            return Err(E::custom("census decoded string cap"));
        }
        Ok(value)
    }
}

struct EndSeed(&'static str);
impl<'de> DeserializeSeed<'de> for EndSeed {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, _decoder: D) -> Result<(), D::Error> {
        Err(D::Error::custom(self.0))
    }
}

pub(crate) fn emit(data: &Data) -> Result<String, BuildError> {
    let mut output = String::from("// Generated by biome_codegen.rs; do not edit.\n");
    for (label, definitions) in [("BIOMES", &data.biomes), ("PRESETS", &data.presets)] {
        writeln!(
            output,
            "static {label}: [Resource; {}] = [",
            definitions.len()
        )
        .map_err(BuildError::Format)?;
        for definition in definitions {
            writeln!(
                output,
                "Resource {{ name: {:?}, original: OriginalBiomeDefinition {{ json: {:?}, stats: BiomeDefinitionStats {{ bytes: {}, nodes: {}, maximum_depth: {} }} }} }},",
                definition.name, definition.json, definition.json.len(), definition.stats.nodes, definition.stats.depth,
            ).map_err(BuildError::Format)?;
        }
        output.push_str("];\n");
    }
    for (slot, definition) in data.presets.iter().enumerate() {
        let Some(entries) = data.parameters.get(&definition.name) else {
            return Err(BuildError::Invalid("missing admitted preset"));
        };
        writeln!(
            output,
            "static PARAMETERS_{slot}: [BiomeParameterEntry; {}] = [",
            entries.len(),
        )
        .map_err(BuildError::Format)?;
        for entry in entries {
            output.push_str("BiomeParameterEntry { point: ClimateParameterPoint { axes: [");
            for [minimum, maximum] in entry.axes {
                write!(
                    output,
                    "ClimateParameterSpan {{ minimum: {minimum}, maximum: {maximum} }},"
                )
                .map_err(BuildError::Format)?;
            }
            writeln!(
                output,
                "], offset: {} }}, biome: BiomeResourceId({}) }},",
                entry.offset, entry.biome,
            )
            .map_err(BuildError::Format)?;
        }
        output.push_str("];\n");
    }
    output.push_str(
        "static PARAMETERS: [&[BiomeParameterEntry]; 2] = [&PARAMETERS_0, &PARAMETERS_1];\n",
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn original_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/fixtures/minecraft-1.21.8-original")
    }
    fn data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
    }
    fn input() -> (Value, BTreeMap<String, u8>) {
        let bytes = std::fs::read(original_dir().join("worldgen.json")).unwrap();
        let original: Export<'_> = serde_json::from_slice(&bytes).unwrap();
        let keys = original.registries["minecraft:worldgen/biome"]
            .entries
            .keys()
            .enumerate()
            .map(|(index, name)| (name.clone(), index as u8))
            .collect();
        let census = serde_json::from_slice(
            &std::fs::read(data_dir().join("biome-parameters-1.21.8.json")).unwrap(),
        )
        .unwrap();
        (census, keys)
    }
    fn refusal(value: &Value, keys: &BTreeMap<String, u8>, detail: &str) {
        let bytes = serde_json::to_vec(value).unwrap();
        let error = admit_parameters(&bytes, keys).unwrap_err().to_string();
        assert!(error.contains(detail), "expected {detail:?}, got {error}");
    }

    #[test]
    fn census_missing_extra_duplicate_unknown_and_wrong_container_inputs_refuse() {
        let (original, keys) = input();
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove("minecraft:nether");
        refusal(&value, &keys, "missing census preset");
        value = original.clone();
        value["minecraft:end"] = json!([]);
        refusal(&value, &keys, "unknown census preset");
        value = original.clone();
        value["minecraft:nether"] = json!({});
        refusal(&value, &keys, "array");
        value = original.clone();
        value["minecraft:nether"].as_array_mut().unwrap().pop();
        refusal(&value, &keys, "row count too small");
        value = original.clone();
        value["minecraft:nether"]
            .as_array_mut()
            .unwrap()
            .push(json!({"extra": vec![0; 20]}));
        refusal(&value, &keys, "row count too large");
        value = original.clone();
        value["minecraft:overworld"].as_array_mut().unwrap().pop();
        refusal(&value, &keys, "row count too small");
        value = original.clone();
        value["minecraft:overworld"]
            .as_array_mut()
            .unwrap()
            .push(json!({"extra": {"nested": []}}));
        refusal(&value, &keys, "row count too large");
        value = original.clone();
        value["minecraft:nether"][0] = json!([]);
        refusal(&value, &keys, "invalid type");
        let bytes = serde_json::to_string(&original).unwrap();
        let escaped_duplicate = bytes.replacen(
            "\"minecraft:nether\":",
            "\"minecraft:nether\":[],\"minecraft:nethe\\u0072\":",
            1,
        );
        // An empty first array itself refuses before the duplicate can be parsed.
        assert!(admit_parameters(escaped_duplicate.as_bytes(), &keys).is_err());
        let nether = serde_json::to_string(&original["minecraft:nether"]).unwrap();
        let duplicate =
            format!("{{\"minecraft:nether\":{nether},\"minecraft:nethe\\u0072\":{nether}}}");
        assert!(admit_parameters(duplicate.as_bytes(), &keys)
            .unwrap_err()
            .to_string()
            .contains("duplicate decoded census preset"));
        let duplicate = bytes.replacen("\"index\":0", "\"index\":0,\"\\u0069ndex\":0", 1);
        assert!(admit_parameters(duplicate.as_bytes(), &keys)
            .unwrap_err()
            .to_string()
            .contains("duplicate decoded census row field"));
    }

    #[test]
    fn census_row_names_fields_indices_and_exact_integer_domains_refuse() {
        let (original, keys) = input();
        for (field, changed, message) in [
            ("index", json!(1), "not contiguous"),
            ("index", json!(-1), "not contiguous"),
            ("index", json!(0.0), "integer"),
            ("index", json!(u64::MAX), "overflow"),
            (
                "biome",
                json!("minecraft:unknown"),
                "unknown or noncanonical",
            ),
            ("biome", json!("nether_wastes"), "unknown or noncanonical"),
            (
                "biome",
                json!("notch:nether_wastes"),
                "unknown or noncanonical",
            ),
            (
                "biome",
                json!("MINECRAFT:nether_wastes"),
                "unknown or noncanonical",
            ),
            ("biome", json!("x".repeat(65)), "string cap"),
            ("offset", json!(-1), "offset outside"),
            ("offset", json!(10001), "offset outside"),
            ("offset", json!(0.0), "integer"),
            ("axes", json!([]), "too few census axes"),
            ("axes", json!(vec![[0, 0]; 7]), "too many census axes"),
            ("axes", json!(vec![[0, 0]; 5]), "too few census axes"),
            ("axes", json!({}), "invalid type"),
        ] {
            let mut value = original.clone();
            value["minecraft:nether"][0][field] = changed;
            refusal(&value, &keys, message);
        }
        for field in ["index", "biome", "axes", "offset"] {
            let mut value = original.clone();
            value["minecraft:nether"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            refusal(&value, &keys, "missing census row field");
        }
        let mut value = original.clone();
        value["minecraft:nether"][0]["unknown"] = json!({"nested": [0]});
        refusal(&value, &keys, "unknown census row field");
        for (span, message) in [
            (json!([]), "too few census span endpoints"),
            (json!([0]), "too few census span endpoints"),
            (json!([0, 0, 0]), "too many census span endpoints"),
            (json!([1, 0]), "reversed census span"),
            (json!([-20001, 0]), "outside original codec domain"),
            (json!([0, 20001]), "outside original codec domain"),
            (json!([0.0, 1]), "integer"),
            (json!([0, u64::MAX]), "overflow"),
            (json!([[0], 0]), "integer"),
        ] {
            let mut value = original.clone();
            value["minecraft:nether"][0]["axes"][0] = span;
            refusal(&value, &keys, message);
        }
    }

    #[test]
    fn census_ordered_duplicates_and_inclusive_codec_boundaries_are_retained() {
        let (mut value, keys) = input();
        let first = value["minecraft:nether"][0].clone();
        value["minecraft:nether"][1] = first;
        value["minecraft:nether"][1]["index"] = json!(1);
        value["minecraft:nether"][2]["axes"][0] = json!([-20000, 20000]);
        value["minecraft:nether"][2]["offset"] = json!(10000);
        let admitted = admit_parameters(&serde_json::to_vec(&value).unwrap(), &keys).unwrap();
        assert_eq!(admitted["minecraft:nether"].len(), 5);
        assert_eq!(
            admitted["minecraft:nether"][0],
            admitted["minecraft:nether"][1]
        );
        assert_eq!(admitted["minecraft:nether"][2].axes[0], [-20000, 20000]);
        assert_eq!(admitted["minecraft:nether"][2].offset, 10000);
    }

    #[test]
    fn census_caps_trailing_truncated_and_nonfinite_inputs_refuse() {
        let (value, keys) = input();
        let mut bytes = serde_json::to_vec(&value).unwrap();
        bytes.push(b'0');
        assert!(admit_parameters(&bytes, &keys)
            .unwrap_err()
            .to_string()
            .contains("trailing"));
        bytes.truncate(100);
        assert!(admit_parameters(&bytes, &keys).is_err());
        let too_large = vec![b' '; CENSUS_BYTES + 1];
        assert!(matches!(
            admit_parameters(&too_large, &keys),
            Err(BuildError::Bounds("census bytes"))
        ));
        let raw = serde_json::to_string(&value).unwrap();
        for number in ["1e999", "NaN", "Infinity", "9223372036854775808"] {
            let changed = raw.replacen("\"offset\":0", &format!("\"offset\":{number}"), 1);
            assert!(
                admit_parameters(changed.as_bytes(), &keys).is_err(),
                "{number}"
            );
        }
        let mut budget = CensusBudget {
            nodes: CENSUS_NODES - 1,
        };
        budget.charge::<serde_json::Error>(CENSUS_DEPTH).unwrap();
        assert!(budget.charge::<serde_json::Error>(0).is_err());
        let mut budget = CensusBudget::default();
        assert!(budget
            .charge::<serde_json::Error>(CENSUS_DEPTH + 1)
            .is_err());
        assert_eq!(budget.nodes, 0);
    }

    #[test]
    fn every_production_pin_rejects_byte_size_and_digest_changes() {
        for (path, pin, label) in [
            (
                original_dir().join("worldgen.json"),
                WORLDGEN_PIN,
                "worldgen",
            ),
            (
                original_dir().join("manifest.json"),
                MANIFEST_PIN,
                "manifest",
            ),
            (
                original_dir().join("version-manifest.json"),
                VERSION_PIN,
                "version",
            ),
            (
                data_dir().join("biome-parameters-1.21.8.json"),
                PARAMETERS_PIN,
                "parameters",
            ),
            (
                data_dir().join("biome-parameters-1.21.8.provenance.json"),
                PROVENANCE_PIN,
                "provenance",
            ),
        ] {
            let mut bytes = std::fs::read(&path).unwrap();
            verify_pin(&bytes, pin, label).unwrap();
            bytes[0] ^= 1;
            assert!(matches!(
                verify_pin(&bytes, pin, label),
                Err(BuildError::Pin(_))
            ));
            bytes.push(0);
            assert!(matches!(
                verify_pin(&bytes, pin, label),
                Err(BuildError::Pin(_))
            ));
            assert!(matches!(
                read_bounded(&path, pin.0 - 1),
                Err(BuildError::Bounds(_))
            ));
        }
    }

    #[test]
    fn original_family_and_preset_structural_drift_refuses() {
        let raw = std::fs::read(original_dir().join("worldgen.json")).unwrap();
        let parameters = std::fs::read(data_dir().join("biome-parameters-1.21.8.json")).unwrap();
        let original: Value = serde_json::from_slice(&raw).unwrap();
        let mut value = original.clone();
        value["registries"]
            .as_object_mut()
            .unwrap()
            .remove("minecraft:worldgen/biome");
        assert!(admit(&serde_json::to_vec(&value).unwrap(), &parameters)
            .err()
            .unwrap()
            .to_string()
            .contains("missing original resource family"));
        value = original.clone();
        value["registries"]["minecraft:worldgen/biome"]["entries"]
            .as_object_mut()
            .unwrap()
            .remove("minecraft:plains");
        assert!(admit(&serde_json::to_vec(&value).unwrap(), &parameters)
            .err()
            .unwrap()
            .to_string()
            .contains("resource count"));
        value = original.clone();
        value["registries"]["minecraft:worldgen/multi_noise_biome_source_parameter_list"]
            ["entries"]["minecraft:nether"]["preset"] = json!("minecraft:overworld");
        assert!(admit(&serde_json::to_vec(&value).unwrap(), &parameters)
            .err()
            .unwrap()
            .to_string()
            .contains("original preset definition"));
        value = original;
        value["registries"]["minecraft:worldgen/multi_noise_biome_source_parameter_list"]
            ["entries"]["minecraft:nether"]["extra"] = json!(0);
        assert!(admit(&serde_json::to_vec(&value).unwrap(), &parameters)
            .err()
            .unwrap()
            .to_string()
            .contains("original preset definition"));
    }
}
