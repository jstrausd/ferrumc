//! Complete immutable builtin biome resources and ordered climate parameters.
//!
//! The 65 full definitions and two preset definitions retain exact slices of the
//! pinned original Minecraft 1.21.8 export. Expanded climate entries retain the
//! actual original preset order and quantized integers. This data producer does
//! no parsing, allocation, I/O, selection or generation at runtime.
//!
//! Resource IDs below are opaque local identities, not dynamic registry/wire or
//! persisted IDs. Consumers must explicitly choose and consistently transmit
//! configuration registry entry order and map biome palettes to that order.

use std::iter::FusedIterator;

/// Resource location of the plains biome — the single biome the MVP flat world
/// places everywhere.
pub const PLAINS: &str = "minecraft:plains";

/// Maximum bytes in one preserved original biome or preset definition.
pub const MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum keys plus values in a definition, including its root.
pub const MAX_DEFINITION_NODES: usize = 4096;
/// Maximum definition key/value depth, starting at root zero.
pub const MAX_DEFINITION_DEPTH: usize = 32;
/// Maximum entries in a definition's object or array.
pub const MAX_DEFINITION_CONTAINER_ENTRIES: usize = 256;
/// Maximum decoded UTF-8 bytes in a definition's string or key.
pub const MAX_DEFINITION_STRING_BYTES: usize = 256;

/// Build-checked statistics of exactly one preserved export slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeDefinitionStats {
    bytes: usize,
    nodes: usize,
    maximum_depth: usize,
}
impl BiomeDefinitionStats {
    /// UTF-8 length of the exact static JSON slice.
    pub const fn bytes(self) -> usize {
        self.bytes
    }
    /// JSON values and object keys, including the root.
    pub const fn nodes(self) -> usize {
        self.nodes
    }
    /// Greatest key/value depth relative to root zero.
    pub const fn maximum_depth(self) -> usize {
        self.maximum_depth
    }
}

/// Preserved full biome or minimal preset definition from the original export.
///
/// Full biome definitions include climate, effects, generation and spawn data.
/// This is not the smaller biome NETWORK codec payload or original JAR resource
/// formatting. Immutable fields cannot be fabricated by consumers. Actual
/// biome codec evaluation and external reference resolution need consumers.
#[derive(Debug)]
pub struct OriginalBiomeDefinition {
    json: &'static str,
    stats: BiomeDefinitionStats,
}
impl OriginalBiomeDefinition {
    /// Exact original export slice, with number tokens and whitespace retained.
    pub const fn json(&self) -> &'static str {
        self.json
    }
    /// Measured syntax statistics admitted by the build-only producer.
    pub const fn stats(&self) -> BiomeDefinitionStats {
        self.stats
    }
}

/// A checked original biome resource, with a private local slot.
///
/// No numeric wire ID, unchecked constructor or default identity is provided.
/// ```compile_fail
/// use ferrumc_registry::biome::BiomeResourceId;
/// let invalid = BiomeResourceId(255);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BiomeResourceId(u8);
impl BiomeResourceId {
    /// Exact canonical or bare name lookup, without allocation.
    /// Other namespaces, malformed/unknown names and case changes return `None`.
    pub fn from_name(name: &str) -> Option<Self> {
        lookup(&BIOMES, name).map(Self)
    }
    /// Canonical resource location, independent of configuration entry order.
    pub fn resource_location(self) -> &'static str {
        BIOMES[usize::from(self.0)].name
    }
    /// The complete original definition; no runtime parser or evaluator.
    pub fn definition(self) -> &'static OriginalBiomeDefinition {
        &BIOMES[usize::from(self.0)].original
    }
}

/// A checked builtin multinoise preset, distinct from a biome resource.
///
/// The local slot is not a wire ID or a parameter row index.
/// ```compile_fail
/// use ferrumc_registry::biome::{BiomeResourceId, BiomeParametersResourceId};
/// let biome = BiomeResourceId::from_name("plains").unwrap();
/// let preset: BiomeParametersResourceId = biome;
/// ```
/// ```compile_fail
/// use ferrumc_registry::biome::BiomeParametersResourceId;
/// let invalid = BiomeParametersResourceId(255);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BiomeParametersResourceId(u8);
impl BiomeParametersResourceId {
    /// Exact canonical or bare name lookup, without allocation.
    /// Unsupported namespaces, spellings and presets return `None`.
    pub fn from_name(name: &str) -> Option<Self> {
        lookup(&PRESETS, name).map(Self)
    }
    /// Original canonical preset resource location.
    pub fn resource_location(self) -> &'static str {
        PRESETS[usize::from(self.0)].name
    }
    /// Exact minimal preset definition, which contains only the preset key.
    pub fn definition(self) -> &'static OriginalBiomeDefinition {
        &PRESETS[usize::from(self.0)].original
    }
    /// All original quantized entries in their original order, including repeats.
    ///
    /// Overworld has 7593 entries, Nether five. Slice position is a parameter
    /// ordinal, never a biome registry ID. Selection/history and `RTree` behavior
    /// belong to checked consumers; this method just borrows immutable data.
    pub fn parameters(self) -> &'static [BiomeParameterEntry] {
        PARAMETERS[usize::from(self.0)]
    }
}

/// Original inclusive climate interval after Java's float-to-long quantization.
///
/// Coordinates are scaled by 10000. Build admission checks ordered integer
/// endpoints in the original codec domain -20000..=20000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimateParameterSpan {
    minimum: i64,
    maximum: i64,
}
impl ClimateParameterSpan {
    /// Inclusive quantized minimum.
    pub const fn minimum(self) -> i64 {
        self.minimum
    }
    /// Inclusive quantized maximum.
    pub const fn maximum(self) -> i64 {
        self.maximum
    }
}

/// Original six climate spans plus quantized offset, without a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimateParameterPoint {
    axes: [ClimateParameterSpan; 6],
    offset: i64,
}
impl ClimateParameterPoint {
    /// Temperature, humidity, continentalness, erosion, depth, weirdness order.
    pub const fn axes(&self) -> &[ClimateParameterSpan; 6] {
        &self.axes
    }
    /// Original quantized temperature span.
    pub const fn temperature(self) -> ClimateParameterSpan {
        self.axes[0]
    }
    /// Original quantized humidity span.
    pub const fn humidity(self) -> ClimateParameterSpan {
        self.axes[1]
    }
    /// Original quantized continentalness span.
    pub const fn continentalness(self) -> ClimateParameterSpan {
        self.axes[2]
    }
    /// Original quantized erosion span.
    pub const fn erosion(self) -> ClimateParameterSpan {
        self.axes[3]
    }
    /// Original quantized depth span.
    pub const fn depth(self) -> ClimateParameterSpan {
        self.axes[4]
    }
    /// Original quantized weirdness span.
    pub const fn weirdness(self) -> ClimateParameterSpan {
        self.axes[5]
    }
    /// Original quantized offset in the original codec domain 0..=10000.
    pub const fn offset(self) -> i64 {
        self.offset
    }
}

/// One immutable original parameter point and its checked biome resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeParameterEntry {
    point: ClimateParameterPoint,
    biome: BiomeResourceId,
}
impl BiomeParameterEntry {
    /// Original spans and offset; no float re-quantization occurs.
    pub const fn point(self) -> ClimateParameterPoint {
        self.point
    }
    /// Original checked biome key identity, not a client registry index.
    pub const fn biome(self) -> BiomeResourceId {
        self.biome
    }
}

struct Resource {
    name: &'static str,
    original: OriginalBiomeDefinition,
}
include!(concat!(env!("OUT_DIR"), "/biomes_generated.rs"));

fn lookup(resources: &[Resource], name: &str) -> Option<u8> {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    resources
        .binary_search_by(|resource| resource.name["minecraft:".len()..].cmp(bare))
        .ok()
        .map(|slot| slot as u8)
}

/// All 65 builtin biomes in canonical-name order, without allocation.
/// This deterministic resource order does not assign wire or persisted IDs.
pub fn biome_resources(
) -> impl ExactSizeIterator<Item = BiomeResourceId> + DoubleEndedIterator + FusedIterator {
    (0..BIOMES.len() as u8).map(BiomeResourceId)
}

/// Both builtin multinoise presets in canonical-name order, without allocation.
pub fn biome_parameters_resources(
) -> impl ExactSizeIterator<Item = BiomeParametersResourceId> + DoubleEndedIterator + FusedIterator
{
    (0..PRESETS.len() as u8).map(BiomeParametersResourceId)
}
