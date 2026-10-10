//! Admission and generation for the pinned original entity-type fixture.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};
use std::fs;
use std::io::Read as _;
use std::path::Path;

use serde::de::{Error as _, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

const EXPECTED_COUNT: usize = 151;
const MAX_INPUT_BYTES: u64 = 64 * 1024;

#[derive(Debug)]
pub(crate) enum BuildError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Oversized(u64),
    Invalid(String),
    Format(fmt::Error),
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "entity-type input I/O: {error}"),
            Self::Json(error) => write!(formatter, "entity-type JSON schema: {error}"),
            Self::Oversized(bytes) => {
                write!(formatter, "entity-type input exceeds 64 KiB: {bytes}")
            }
            Self::Invalid(detail) => write!(formatter, "entity-type invariant: {detail}"),
            Self::Format(error) => write!(formatter, "entity-type source formatting: {error}"),
        }
    }
}

impl std::error::Error for BuildError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRegistry {
    registry: String,
    protocol_id: i32,
    default: String,
    entries: UniqueEntries,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    protocol_id: i32,
}

struct UniqueEntries(BTreeMap<String, RawEntry>);

impl<'de> Deserialize<'de> for UniqueEntries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = UniqueEntries;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an object with unique entity resource locations")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = BTreeMap::new();
                while let Some((name, entry)) = map.next_entry::<String, RawEntry>()? {
                    if entries.insert(name, entry).is_some() {
                        return Err(A::Error::custom("duplicate entity resource location"));
                    }
                    if entries.len() > EXPECTED_COUNT {
                        return Err(A::Error::custom("entity registry exceeds pinned count"));
                    }
                }
                Ok(UniqueEntries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

fn require(condition: bool, detail: &str) -> Result<(), BuildError> {
    if condition {
        Ok(())
    } else {
        Err(BuildError::Invalid(detail.to_owned()))
    }
}

fn canonical_name(name: &str) -> bool {
    let Some(path) = name.strip_prefix("minecraft:") else {
        return false;
    };
    !path.is_empty()
        && path.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'.' | b'/' | b'-')
        })
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<String>, BuildError> {
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(BuildError::Oversized(bytes.len() as u64));
    }
    let raw: RawRegistry = serde_json::from_slice(bytes).map_err(BuildError::Json)?;
    require(raw.registry == "minecraft:entity_type", "wrong registry")?;
    require(raw.protocol_id == 5, "wrong registry protocol id")?;
    require(raw.default == "minecraft:pig", "wrong registry default")?;
    require(raw.entries.0.len() == EXPECTED_COUNT, "wrong entry count")?;
    require(raw.entries.0.contains_key(&raw.default), "absent default")?;
    require(
        raw.entries.0.contains_key("minecraft:player"),
        "absent player type",
    )?;

    let mut names = vec![None; EXPECTED_COUNT];
    for (name, entry) in raw.entries.0 {
        require(canonical_name(&name), "malformed resource location")?;
        let index = usize::try_from(entry.protocol_id)
            .map_err(|_| BuildError::Invalid("negative entity protocol id".to_owned()))?;
        let slot = names.get_mut(index).ok_or_else(|| {
            BuildError::Invalid("entity protocol id outside dense range".to_owned())
        })?;
        require(slot.is_none(), "duplicate entity protocol id")?;
        *slot = Some(name);
    }
    names
        .into_iter()
        .map(|name| name.ok_or_else(|| BuildError::Invalid("entity protocol id gap".to_owned())))
        .collect()
}

pub(crate) fn load(path: &Path) -> Result<Vec<String>, BuildError> {
    let size = fs::metadata(path).map_err(BuildError::Io)?.len();
    if size > MAX_INPUT_BYTES {
        return Err(BuildError::Oversized(size));
    }
    // Bound the actual read too: an input changed after metadata inspection
    // must not turn the preflight into an unrestricted allocation.
    let file = fs::File::open(path).map_err(BuildError::Io)?;
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(BuildError::Io)?;
    parse(&bytes)
}

pub(crate) fn emit(names: &[String]) -> Result<String, BuildError> {
    require(names.len() == EXPECTED_COUNT, "wrong emission count")?;
    let player_index = names
        .iter()
        .position(|name| name == "minecraft:player")
        .ok_or_else(|| BuildError::Invalid("absent player emission".to_owned()))?;
    let mut by_name = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let bare = name.strip_prefix("minecraft:").unwrap_or(name);
            (bare, index)
        })
        .collect::<Vec<_>>();
    by_name.sort_unstable_by_key(|&(name, _)| name);

    let mut source = String::from(
        "// @generated by build.rs from the pinned original entity-types.json.\n\
         // Edit the generator or re-pin the source; do not edit this output.\n\n",
    );
    let render = (|| -> fmt::Result {
        writeln!(
            source,
            "/// Number of entity types in the pinned Minecraft 1.21.8 registry."
        )?;
        writeln!(
            source,
            "pub const ENTITY_TYPE_COUNT: usize = {};",
            names.len()
        )?;
        writeln!(
            source,
            "const ENTITY_TYPE_COUNT_I32: i32 = {};",
            names.len()
        )?;
        writeln!(source, "const PLAYER_PROTOCOL_ID: i32 = {player_index};")?;
        writeln!(
            source,
            "static ENTITY_TYPE_NAMES: [&str; {}] = [",
            names.len()
        )?;
        for name in names {
            writeln!(source, "    {name:?},")?;
        }
        writeln!(
            source,
            "];\nstatic ENTITY_TYPES_BY_NAME: [(&str, i32); {}] = [",
            names.len()
        )?;
        for (name, index) in by_name {
            writeln!(source, "    ({name:?}, {index}),")?;
        }
        writeln!(source, "];")
    })();
    render.map_err(BuildError::Format)?;
    Ok(source)
}
