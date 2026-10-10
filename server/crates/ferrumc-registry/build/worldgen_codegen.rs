//! Pinned builtin worldgen producer. All parsing and allocation is build-only.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::fs::File;
use std::io::Read as _;
use std::path::Path;

use serde::Deserialize;
use serde_json::{value::RawValue, Map, Value};
use sha2::{Digest, Sha256};

use crate::worldgen_json as bounded_json;
#[path = "worldgen_schema.rs"]
mod schema;

pub(crate) const WHOLE_LIMITS: bounded_json::Limits = bounded_json::Limits {
    bytes: 4 * 1024 * 1024,
    nodes: 131_072,
    depth: 32,
    entries: 1024,
    string_bytes: 256,
};
pub(crate) const DEFINITION_LIMITS: bounded_json::Limits = bounded_json::Limits {
    bytes: 128 * 1024,
    nodes: 4096,
    depth: 32,
    entries: 256,
    string_bytes: 256,
};
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
const FAMILIES: &[(&str, usize)] = &[
    ("biome", 65),
    ("configured_carver", 4),
    ("configured_feature", 224),
    ("density_function", 35),
    ("flat_level_generator_preset", 9),
    ("multi_noise_biome_source_parameter_list", 2),
    ("noise", 60),
    ("noise_settings", 7),
    ("placed_feature", 258),
    ("processor_list", 40),
    ("structure", 34),
    ("structure_set", 20),
    ("template_pool", 188),
    ("world_preset", 6),
];

#[derive(Debug)]
pub(crate) enum BuildError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Bounds(&'static str),
    Pin(&'static str),
    Invalid(String),
    Format(fmt::Error),
}
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "worldgen input I/O: {e}"),
            Self::Json(e) => write!(f, "worldgen JSON admission: {e}"),
            Self::Bounds(e) => write!(f, "worldgen limit: {e}"),
            Self::Pin(e) => write!(f, "worldgen provenance pin: {e}"),
            Self::Invalid(e) => write!(f, "worldgen schema/reference: {e}"),
            Self::Format(e) => write!(f, "worldgen source formatting: {e}"),
        }
    }
}
impl std::error::Error for BuildError {}

pub(super) fn require(condition: bool, detail: &str) -> Result<(), BuildError> {
    if condition {
        Ok(())
    } else {
        Err(BuildError::Invalid(detail.to_owned()))
    }
}

pub(super) fn canonical_name(name: &str) -> bool {
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

/// Already validated block/property domains from the existing block producer.
pub(crate) type BlockProperties = BTreeMap<String, BTreeMap<String, Vec<String>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Kind {
    Noise,
    DensityFunction,
    Block,
    Biome,
    DensityFunctionType,
    SurfaceRuleType,
    SurfaceConditionType,
    RandomName,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Dependency {
    pub kind: Kind,
    pub name: String,
}
pub(crate) struct Definition {
    pub name: String,
    pub json: String,
    pub stats: bounded_json::Stats,
    pub occurrences: usize,
    pub dependencies: BTreeSet<Dependency>,
    pub noise: Option<(i32, Vec<f64>)>,
}
pub(crate) struct Data {
    pub noise: Vec<Definition>,
    pub density: Vec<Definition>,
    pub settings: Vec<Definition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExport<'a> {
    #[serde(borrow)]
    registries: BTreeMap<String, RawFamily<'a>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFamily<'a> {
    #[serde(borrow)]
    entries: BTreeMap<String, &'a RawValue>,
}

fn read_bounded(path: &Path, cap: usize) -> Result<Vec<u8>, BuildError> {
    let file = File::open(path).map_err(BuildError::Io)?;
    if file.metadata().map_err(BuildError::Io)?.len() > cap as u64 {
        return Err(BuildError::Bounds("input file bytes"));
    }
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(BuildError::Io)?;
    if bytes.len() > cap {
        return Err(BuildError::Bounds("input grew beyond byte cap"));
    }
    Ok(bytes)
}
pub(crate) fn verify_pin(
    bytes: &[u8],
    expected: (usize, &str),
    name: &'static str,
) -> Result<(), BuildError> {
    if bytes.len() != expected.0 || format!("{:x}", Sha256::digest(bytes)) != expected.1 {
        return Err(BuildError::Pin(name));
    }
    Ok(())
}
pub(crate) fn load(directory: &Path, blocks: &BlockProperties) -> Result<Data, BuildError> {
    let bytes = read_bounded(&directory.join("worldgen.json"), WHOLE_LIMITS.bytes)?;
    let manifest = read_bounded(&directory.join("manifest.json"), 16 * 1024)?;
    let version = read_bounded(&directory.join("version-manifest.json"), 64 * 1024)?;
    verify_pin(&bytes, WORLDGEN_PIN, "worldgen.json")?;
    verify_pin(&manifest, MANIFEST_PIN, "manifest.json")?;
    verify_pin(&version, VERSION_PIN, "version-manifest.json")?;
    provenance(&manifest, &version)?;
    admit(&bytes, blocks)
}

pub(crate) fn provenance(manifest: &[u8], version: &[u8]) -> Result<(), BuildError> {
    let (manifest, _) = bounded_json::parse(
        manifest,
        bounded_json::Limits {
            bytes: 16 * 1024,
            ..WHOLE_LIMITS
        },
    )
    .map_err(BuildError::Json)?;
    let (version, _) = bounded_json::parse(
        version,
        bounded_json::Limits {
            bytes: 64 * 1024,
            string_bytes: 4096,
            ..WHOLE_LIMITS
        },
    )
    .map_err(BuildError::Json)?;
    require(
        manifest["schema_version"] == 1
            && manifest["minecraft"]["version"] == "1.21.8"
            && manifest["minecraft"]["protocol_version"] == 772
            && manifest["minecraft"]["data_version"] == 4440,
        "Minecraft/schema version drift",
    )?;
    require(version["id"] == "1.21.8", "version manifest release drift")?;
    require(
        manifest["fixtures"]["worldgen.json"]["bytes"] == WORLDGEN_PIN.0
            && manifest["fixtures"]["worldgen.json"]["sha256"] == WORLDGEN_PIN.1
            && manifest["fixtures"]["version-manifest.json"]["bytes"] == VERSION_PIN.0
            && manifest["fixtures"]["version-manifest.json"]["sha256"] == VERSION_PIN.1,
        "manifest fixture pins drift",
    )?;
    for (family, count, bytes, index) in [
        (
            "noise",
            60,
            4516,
            "8cac804ec775f881419d42f1534c93f25551ac9af56e61a6090817a920101cf7",
        ),
        (
            "density_function",
            35,
            335_487,
            "5ad0c9966a00e304b6f09b336c06b4797279f140822e0949bdab091412dafab7",
        ),
        (
            "noise_settings",
            7,
            546_768,
            "19fc82360b409a12ead0736d28712910664d9ba4447b394c7054481f154e9ca8",
        ),
    ] {
        let receipt = &manifest["source"]["datapack_categories"][format!("worldgen/{family}")];
        require(
            receipt["count"] == count
                && receipt["bytes"] == bytes
                && receipt["path_bytes_sha256_index"] == index,
            "manifest resource receipt drift",
        )?;
    }
    Ok(())
}

/// Structural admission is separately testable; production always checks pins first.
pub(crate) fn admit(bytes: &[u8], blocks: &BlockProperties) -> Result<Data, BuildError> {
    let (tree, _) = bounded_json::parse(bytes, WHOLE_LIMITS).map_err(BuildError::Json)?;
    let root = object(&tree, &["registries"])?;
    let registries = root["registries"]
        .as_object()
        .ok_or_else(|| invalid("registries must be an object"))?;
    require(
        registries.len() == FAMILIES.len(),
        "unexpected worldgen family count",
    )?;
    for (family, count) in FAMILIES {
        let key = format!("minecraft:worldgen/{family}");
        let bucket = registries
            .get(&key)
            .ok_or_else(|| invalid("missing or foreign worldgen family"))?;
        let wrapper = object(bucket, &["entries"])?;
        let entries = wrapper["entries"]
            .as_object()
            .ok_or_else(|| invalid("resource entries must be an object"))?;
        require(
            entries.len() == *count,
            "pinned family resource count drift",
        )?;
        for name in entries.keys() {
            require(canonical_name(name), "malformed or foreign resource name")?;
        }
    }
    // RawValue is used only after bounded, duplicate-free whole-tree admission.
    let raw: RawExport<'_> = serde_json::from_slice(bytes).map_err(BuildError::Json)?;
    let noise_names = &raw.registries["minecraft:worldgen/noise"].entries;
    let density_names = &raw.registries["minecraft:worldgen/density_function"].entries;
    let biome_names = &raw.registries["minecraft:worldgen/biome"].entries;
    let mut data = Data {
        noise: Vec::new(),
        density: Vec::new(),
        settings: Vec::new(),
    };
    for (family, destination) in [
        ("noise", &mut data.noise),
        ("density_function", &mut data.density),
        ("noise_settings", &mut data.settings),
    ] {
        for (name, original) in &raw.registries[&format!("minecraft:worldgen/{family}")].entries {
            let (value, stats) = bounded_json::parse(original.get().as_bytes(), DEFINITION_LIMITS)
                .map_err(BuildError::Json)?;
            let mut checker = schema::Checker::new(noise_names, density_names, biome_names, blocks);
            let noise = match family {
                "noise" => Some(schema::noise(&value)?),
                "density_function" => {
                    checker.density(&value, false)?;
                    None
                }
                "noise_settings" => {
                    checker.settings(&value)?;
                    None
                }
                _ => unreachable!("selected families are fixed"),
            };
            // Definition ownership starts only after shape/reference validation.
            destination.push(Definition {
                name: name.clone(),
                json: original.get().to_owned(),
                stats,
                occurrences: checker.occurrences,
                dependencies: checker.dependencies,
                noise,
            });
        }
    }
    check_density_cycles(&data.density)?;
    Ok(data)
}

fn check_density_cycles(definitions: &[Definition]) -> Result<(), BuildError> {
    fn visit(
        index: usize,
        definitions: &[Definition],
        states: &mut [u8],
    ) -> Result<(), BuildError> {
        require(states[index] != 1, "cyclic density resource references")?;
        if states[index] == 2 {
            return Ok(());
        }
        states[index] = 1;
        for dependency in &definitions[index].dependencies {
            if dependency.kind == Kind::DensityFunction {
                let next = definitions
                    .binary_search_by(|entry| entry.name.cmp(&dependency.name))
                    .map_err(|_| invalid("unresolved density reference"))?;
                visit(next, definitions, states)?;
            }
        }
        states[index] = 2;
        Ok(())
    }
    let mut states = vec![0; definitions.len()];
    for index in 0..definitions.len() {
        visit(index, definitions, &mut states)?;
    }
    Ok(())
}

pub(super) fn invalid(detail: &str) -> BuildError {
    BuildError::Invalid(detail.to_owned())
}
pub(super) fn object<'a>(
    value: &'a Value,
    fields: &[&str],
) -> Result<&'a Map<String, Value>, BuildError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("expected object"))?;
    require(
        object.len() == fields.len() && fields.iter().all(|key| object.contains_key(*key)),
        "unknown or missing schema field",
    )?;
    Ok(object)
}
pub(super) fn text(value: &Value) -> Result<&str, BuildError> {
    value.as_str().ok_or_else(|| invalid("expected string"))
}
pub(super) fn array(value: &Value) -> Result<&[Value], BuildError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| invalid("expected array"))
}
pub(super) fn number(value: &Value, min: f64, max: f64) -> Result<f64, BuildError> {
    let number = value
        .as_f64()
        .ok_or_else(|| invalid("expected finite number"))?;
    require(
        number.is_finite() && (min..=max).contains(&number),
        "number outside finite codec range",
    )?;
    Ok(number)
}
pub(super) fn integer(value: &Value, min: i64, max: i64) -> Result<i64, BuildError> {
    let value = value.as_i64().ok_or_else(|| invalid("expected integer"))?;
    require((min..=max).contains(&value), "integer outside codec range")?;
    Ok(value)
}
pub(super) fn boolean(value: &Value) -> Result<(), BuildError> {
    require(value.is_boolean(), "expected boolean")
}

pub(crate) fn emit(data: &Data) -> Result<String, BuildError> {
    let mut output =
        String::from("// Generated from the pinned original builtin export. Do not edit.\n");
    for (label, definitions) in [
        ("NOISE", &data.noise),
        ("DENSITY", &data.density),
        ("SETTINGS", &data.settings),
    ] {
        writeln!(
            output,
            "static {label}: [Resource; {}] = [",
            definitions.len()
        )
        .map_err(BuildError::Format)?;
        for definition in definitions {
            write!(output,"Resource {{ name: {:?}, original: OriginalDefinition {{ json: {:?}, stats: DefinitionStats {{ bytes: {}, nodes: {}, maximum_depth: {}, dependency_occurrences: {} }}, dependencies: &[",definition.name,definition.json,decimal_literal(definition.json.len()),definition.stats.nodes,definition.stats.depth,definition.occurrences).map_err(BuildError::Format)?;
            for dependency in &definition.dependencies {
                let kind = format!("{:?}", dependency.kind);
                match dependency.kind {
                    Kind::Noise | Kind::DensityFunction => {
                        let (collection, identifier) = if dependency.kind == Kind::Noise {
                            (&data.noise, "NoiseResourceId")
                        } else {
                            (&data.density, "DensityFunctionResourceId")
                        };
                        let slot = collection
                            .binary_search_by(|entry| entry.name.cmp(&dependency.name))
                            .map_err(|_| invalid("unresolved emitted resource"))?;
                        write!(output, "ResourceDependency::{kind}({identifier}({slot})),")
                            .map_err(BuildError::Format)?;
                    }
                    _ => write!(output, "ResourceDependency::{kind}({:?}),", dependency.name)
                        .map_err(BuildError::Format)?,
                }
            }
            output.push_str("] } },\n");
        }
        output.push_str("];\n");
    }
    writeln!(
        output,
        "static NOISE_PARAMETERS: [NoiseParameters; {}] = [",
        data.noise.len()
    )
    .map_err(BuildError::Format)?;
    for definition in &data.noise {
        let Some((octave, amplitudes)) = &definition.noise else {
            return Err(invalid("noise-family parameters missing during emission"));
        };
        write!(
            output,
            "NoiseParameters {{ first_octave: {octave}, amplitudes: &["
        )
        .map_err(BuildError::Format)?;
        for amplitude in amplitudes {
            let bits = amplitude.to_bits();
            write!(
                output,
                "f64::from_bits(0x{:04x}_{:04x}_{:04x}_{:04x}),",
                bits >> 48,
                (bits >> 32) & 0xffff,
                (bits >> 16) & 0xffff,
                bits & 0xffff
            )
            .map_err(BuildError::Format)?;
        }
        output.push_str("] },\n");
    }
    output.push_str("];\n");
    Ok(output)
}

fn decimal_literal(value: usize) -> String {
    let digits = value.to_string();
    let mut output = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push('_');
        }
        output.push(digit);
    }
    output
}
