//! Immutable original builtin worldgen resources for Minecraft 1.21.8.
//!
//! This is a data producer. Density compilation, sampler creation, seed binding,
//! surface/biome evaluation and terrain generation belong to checked consumers.
//! The static JSON preserves the vendored original export's exact number tokens
//! and whitespace. No JSON parser, allocation, I/O or mutable global runs here.

use std::iter::FusedIterator;

/// Maximum bytes in a selected builtin definition, before any consumer parses it.
pub const MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum JSON nodes, counting both object keys and values, with the root included.
pub const MAX_DEFINITION_NODES: usize = 4096;
/// Maximum JSON depth; root is zero, object keys and children add one.
pub const MAX_DEFINITION_DEPTH: usize = 32;
/// Maximum elements in any definition array or object.
pub const MAX_DEFINITION_CONTAINER_ENTRIES: usize = 256;
/// Maximum decoded string/key UTF-8 bytes in any definition.
pub const MAX_DEFINITION_STRING_BYTES: usize = 256;
/// Maximum dependency occurrences per definition, including repeated codec types.
pub const MAX_DEPENDENCY_OCCURRENCES: usize = 2048;
/// Maximum distinct dependencies in a single definition.
pub const MAX_DISTINCT_DEPENDENCIES: usize = 512;

/// Measured, build-checked statistics for exactly this JSON definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionStats {
    bytes: usize,
    nodes: usize,
    maximum_depth: usize,
    dependency_occurrences: usize,
}
impl DefinitionStats {
    /// UTF-8 byte length of the exact static JSON slice.
    pub const fn bytes(self) -> usize {
        self.bytes
    }
    /// Object keys plus JSON values; root is included.
    pub const fn nodes(self) -> usize {
        self.nodes
    }
    /// Greatest key/value depth, relative to a root at zero.
    pub const fn maximum_depth(self) -> usize {
        self.maximum_depth
    }
    /// Occurrences of all typed dependencies, before deduplication.
    pub const fn dependency_occurrences(self) -> usize {
        self.dependency_occurrences
    }
}

/// Typed dependencies found through the original codec paths at build time.
///
/// Noise/density references resolve to this producer's family-specific ids.
/// External categories are names, not implemented behavior or registry handles.
/// This builtin export contains no tag dependencies; unsupported tags fail the
/// producer build instead of being silently discarded or replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceDependency {
    /// Fully resolved builtin normal-noise resource.
    Noise(NoiseResourceId),
    /// Fully resolved builtin density resource.
    DensityFunction(DensityFunctionResourceId),
    /// External block registry dependency; properties remain in the JSON.
    Block(&'static str),
    /// External biome registry dependency.
    Biome(&'static str),
    /// External density function codec-type registry dependency.
    DensityFunctionType(&'static str),
    /// External surface rule codec-type registry dependency.
    SurfaceRuleType(&'static str),
    /// External surface condition codec-type registry dependency.
    SurfaceConditionType(&'static str),
    /// A positional RNG factory name, not a registry resource/holder.
    RandomName(&'static str),
}

/// Immutable, bounded definition from the unchanged original export.
///
/// Fields cannot be fabricated or replaced. Consumers must compile this JSON
/// with checked original codec semantics, their own cumulative graph/work bounds
/// and explicit handling of external categories. These per-definition limits
/// do not bound an expanded graph or provide runtime evaluation.
#[derive(Debug)]
pub struct OriginalDefinition {
    json: &'static str,
    stats: DefinitionStats,
    dependencies: &'static [ResourceDependency],
}
impl OriginalDefinition {
    /// Exact JSON slice of the pinned export, with original decimal tokens intact.
    pub const fn json(&self) -> &'static str {
        self.json
    }
    /// Build-checked size, tree and reference statistics.
    pub const fn stats(&self) -> DefinitionStats {
        self.stats
    }
    /// Distinct typed dependencies in deterministic enum/name order.
    pub const fn dependencies(&self) -> &'static [ResourceDependency] {
        self.dependencies
    }
}

/// Original normal-noise octave/amplitude parameters, without a seed or sampler.
#[derive(Debug)]
pub struct NoiseParameters {
    first_octave: i32,
    amplitudes: &'static [f64],
}
impl NoiseParameters {
    /// Original first octave. Builtin scope is -16..=-1.
    pub const fn first_octave(&self) -> i32 {
        self.first_octave
    }
    /// Original finite amplitudes with exact rounded f64 bits, including zeros.
    pub const fn amplitudes(&self) -> &'static [f64] {
        self.amplitudes
    }
}

struct Resource {
    name: &'static str,
    original: OriginalDefinition,
}
include!(concat!(env!("OUT_DIR"), "/worldgen_generated.rs"));

macro_rules! resource_id {
    ($identifier:ident, $table:ident, $docs:literal) => {
        #[doc = $docs]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $identifier(u8);
        impl $identifier {
            /// Exact canonical `minecraft:` or bare name lookup; alloc-free.
            /// Other namespaces, whitespace, case changes and unknown names return `None`.
            pub fn from_name(name: &str) -> Option<Self> {
                lookup(&$table, name).map(Self)
            }
            /// Canonical original name. The private local slot is not a wire id.
            pub fn resource_location(self) -> &'static str {
                $table[usize::from(self.0)].name
            }
            /// Original checked builtin definition; no runtime compiler is supplied.
            pub fn definition(self) -> &'static OriginalDefinition {
                &$table[usize::from(self.0)].original
            }
        }
    };
}
resource_id!(NoiseResourceId,NOISE,"A validated builtin noise resource; private slot, no unchecked construction.\n```compile_fail\nuse ferrumc_registry::worldgen::NoiseResourceId;\nlet invalid = NoiseResourceId(255);\n```");
resource_id!(DensityFunctionResourceId,DENSITY,"A validated builtin density resource, distinct from a noise id.\n```compile_fail\nuse ferrumc_registry::worldgen::{NoiseResourceId,DensityFunctionResourceId};\nlet noise = NoiseResourceId::from_name(\"minecraft:offset\").unwrap();\nlet density: DensityFunctionResourceId = noise;\n```");
resource_id!(NoiseSettingsResourceId,SETTINGS,"A validated builtin noise-settings resource, distinct from density ids.\n```compile_fail\nuse ferrumc_registry::worldgen::{DensityFunctionResourceId,NoiseSettingsResourceId};\nlet density = DensityFunctionResourceId::from_name(\"minecraft:zero\").unwrap();\nlet settings: NoiseSettingsResourceId = density;\n```");

impl NoiseResourceId {
    /// Seed-free original parameters. All constructors enforce this family.
    pub fn parameters(self) -> &'static NoiseParameters {
        &NOISE_PARAMETERS[usize::from(self.0)]
    }
}

fn lookup(resources: &[Resource], name: &str) -> Option<u8> {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    resources
        .binary_search_by(|resource| resource.name["minecraft:".len()..].cmp(bare))
        .ok()
        .map(|index| index as u8)
}

/// All 60 builtin noise resources in canonical-name order, without allocation.
pub fn noise_resources(
) -> impl ExactSizeIterator<Item = NoiseResourceId> + DoubleEndedIterator + FusedIterator {
    (0..NOISE.len() as u8).map(NoiseResourceId)
}
/// All 35 builtin density resources in canonical-name order, without allocation.
pub fn density_function_resources(
) -> impl ExactSizeIterator<Item = DensityFunctionResourceId> + DoubleEndedIterator + FusedIterator
{
    (0..DENSITY.len() as u8).map(DensityFunctionResourceId)
}
/// All seven builtin noise-settings resources in canonical-name order, without allocation.
pub fn noise_settings_resources(
) -> impl ExactSizeIterator<Item = NoiseSettingsResourceId> + DoubleEndedIterator + FusedIterator {
    (0..SETTINGS.len() as u8).map(NoiseSettingsResourceId)
}
