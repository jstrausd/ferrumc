//! Checked Minecraft 1.21.8 builtin compilation and seeded scalar/climate sampling.
//!
//! Templates own immutable private graphs. They expose metadata only: there is
//! no evaluation API, unbound-noise sampler, generated chunk or natural-world
//! identity. Seed binding and `NoiseChunk` contexts are subsequent consumers.

mod bind;
mod chunk;
mod compile;
mod evaluate;
mod graph;
mod json;
mod seeded;

// GEN-05.8 stored lookup reuses the single accepted chunk-domain predicate.
pub(crate) use chunk::{admit_chunk, NaturalNoisePlan};
// GEN-05.10 column driver: the closed GEN-05.4 material alphabet.
pub(crate) use chunk::{material_alphabet, MATERIAL_ALPHABET_STATES};
pub(crate) use chunk::{CarverAquifer, NoiseChunkCoreError};
// GEN-05.9 surface rules reuse the single checked definition JSON admission and
// this compiler's original Codec.DOUBLE / Gson INT token decoding.
pub use chunk::{
    ChunkDensitySamples, ChunkMaterialSamples, NoiseChunkError, NoiseChunkErrorKind,
    NoiseChunkLimit, NoiseChunkSession, NoiseChunkWorkBudget, MAX_NOISE_CHUNK_WORK,
};
pub(crate) use compile::{integer as decode_int, number as decode_double};
pub(crate) use json::{parse as parse_definition, Value as JsonValue, ValueKind as JsonValueKind};

pub use seeded::{
    ClimateAxis, ClimateQuartPos, ClimateSample, DensityBatchBudget, DensityBindError,
    DensityBindErrorKind, DensityBounds, DensityRequest, DensitySampleError,
    DensitySampleErrorKind, DensityWorkBudget, SeededRouter, MAX_DENSITY_BATCH_REQUESTS,
    MAX_DENSITY_BATCH_WORK, MAX_DENSITY_BLOCK_COORDINATE, MAX_DENSITY_SAMPLE_WORK,
};

use std::sync::Arc;

use ferrumc_registry::worldgen::{
    DensityFunctionResourceId, NoiseResourceId, NoiseSettingsResourceId,
};

use graph::Graph;

/// Maximum distinct original source bytes admitted by one compilation.
pub const MAX_DENSITY_SOURCE_BYTES: usize = 2 * 1024 * 1024;
/// Maximum private vertices, including direct/reference holders and splines.
pub const MAX_DENSITY_VERTICES: usize = 4096;
/// Maximum private edges, including density/spline coordinate transitions.
pub const MAX_DENSITY_EDGES: usize = 8192;
/// Maximum combined density/holder/spline depth, with a leaf at depth one.
pub const MAX_DENSITY_DEPTH: usize = 96;

/// The fifteen original noise-router fields, in the original codec order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RouterField {
    /// Aquifer barrier density.
    Barrier,
    /// Aquifer fluid-level floodedness.
    FluidLevelFloodedness,
    /// Aquifer fluid-level spread.
    FluidLevelSpread,
    /// Aquifer lava density.
    Lava,
    /// Climate temperature.
    Temperature,
    /// Climate humidity (originally named vegetation).
    Vegetation,
    /// Climate continentalness.
    Continents,
    /// Climate erosion.
    Erosion,
    /// Climate depth.
    Depth,
    /// Climate weirdness (originally named ridges).
    Ridges,
    /// Preliminary surface density.
    InitialDensityWithoutJaggedness,
    /// Final terrain density, before material selection.
    FinalDensity,
    /// Ore-vein selection noise.
    VeinToggle,
    /// Ore-vein ridge noise.
    VeinRidged,
    /// Ore-vein gap noise.
    VeinGap,
}
impl RouterField {
    /// All fields in original codec order; no allocation or mutable registry.
    pub const ALL: [Self; 15] = [
        Self::Barrier,
        Self::FluidLevelFloodedness,
        Self::FluidLevelSpread,
        Self::Lava,
        Self::Temperature,
        Self::Vegetation,
        Self::Continents,
        Self::Erosion,
        Self::Depth,
        Self::Ridges,
        Self::InitialDensityWithoutJaggedness,
        Self::FinalDensity,
        Self::VeinToggle,
        Self::VeinRidged,
        Self::VeinGap,
    ];
    /// Original serialized field name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Barrier => "barrier",
            Self::FluidLevelFloodedness => "fluid_level_floodedness",
            Self::FluidLevelSpread => "fluid_level_spread",
            Self::Lava => "lava",
            Self::Temperature => "temperature",
            Self::Vegetation => "vegetation",
            Self::Continents => "continents",
            Self::Erosion => "erosion",
            Self::Depth => "depth",
            Self::Ridges => "ridges",
            Self::InitialDensityWithoutJaggedness => "initial_density_without_jaggedness",
            Self::FinalDensity => "final_density",
            Self::VeinToggle => "vein_toggle",
            Self::VeinRidged => "vein_ridged",
            Self::VeinGap => "vein_gap",
        }
    }
}

/// Checked original noise-cell geometry. This is not a build-height decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoiseGeometry {
    min_y: i32,
    height: u32,
    horizontal: u8,
    vertical: u8,
}
impl NoiseGeometry {
    /// Original minimum noise Y.
    pub const fn min_y(self) -> i32 {
        self.min_y
    }
    /// Original noise height in blocks, including the original permitted zero.
    pub const fn height(self) -> u32 {
        self.height
    }
    /// Horizontal cell width in blocks (four times the noise size).
    pub const fn cell_width(self) -> u32 {
        self.horizontal as u32 * 4
    }
    /// Vertical cell height in blocks (four times the noise size).
    pub const fn cell_height(self) -> u32 {
        self.vertical as u32 * 4
    }
}

/// Immutable graph measurements. Counts include explicitly retained holders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityGraphStats {
    vertices: usize,
    edges: usize,
    depth: usize,
    splines: usize,
    spline_points: usize,
    markers: usize,
}
impl DensityGraphStats {
    /// Vertices, including spline values and holder identities.
    pub const fn vertices(self) -> usize {
        self.vertices
    }
    /// Edges, including holder and spline-coordinate transitions.
    pub const fn edges(self) -> usize {
        self.edges
    }
    /// Greatest combined depth, counting a constant leaf as one.
    pub const fn depth(self) -> usize {
        self.depth
    }
    /// Spline constant and multipoint vertices.
    pub const fn spline_vertices(self) -> usize {
        self.splines
    }
    /// Aggregate points in the measured spline multipoints.
    pub const fn spline_points(self) -> usize {
        self.spline_points
    }
    /// Original cache/interpolation marker occurrences.
    pub const fn markers(self) -> usize {
        self.markers
    }
}

/// Which bounded admission allowance was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityLimit {
    /// A single original JSON definition's UTF-8 bytes.
    DefinitionBytes,
    /// Distinct cumulative original resources, charged before parsing.
    SourceBytes,
    /// JSON values and object keys.
    JsonNodes,
    /// JSON key/value depth, root zero.
    JsonDepth,
    /// Array or object entries.
    ContainerEntries,
    /// Decoded UTF-8 string or key bytes.
    StringBytes,
    /// Original bundled Gson INT fallback number-token bytes.
    IntegerTokenBytes,
    /// Total IR vertices.
    Vertices,
    /// Total IR edges.
    Edges,
    /// Combined density/spline/holder depth.
    CombinedDepth,
    /// Spline-local vertices.
    SplineVertices,
    /// Points in a spline multipoint.
    SplinePointsPerNode,
    /// Total spline-local points.
    SplinePoints,
    /// Distinct spline-local coordinates.
    SplineCoordinates,
    /// Spline-local depth, without following density coordinates.
    SplineDepth,
}

/// Classified compiler rejection; no partial public template is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DensityCompileErrorKind {
    /// Full JSON grammar, trailing input, or escaped-string decoding failed.
    InvalidJson,
    /// Duplicate object key after escape decoding.
    DuplicateField,
    /// A required schema field is absent.
    MissingField,
    /// Unknown object field (additional `FerrumC` admission policy).
    UnknownField,
    /// Wrong JSON value shape for the original codec path.
    WrongShape,
    /// Unknown density codec type.
    UnknownType,
    /// Unknown resource or a known name in the wrong reference family.
    UnknownReference,
    /// Numeric input is nonfinite or outside the original codec bounds.
    NumericRange,
    /// Original noise geometry constraints fail.
    InvalidGeometry,
    /// Spline locations are not strictly increasing as decoded f32 values.
    InvalidSplineOrder,
    /// Producer byte/tree/dependency/parameter metadata differ from admission.
    ProducerDrift,
    /// A joint density/spline/holder edge forms a cycle, including unused nodes.
    Cycle { from: usize, to: usize },
    /// An explicit bounded allocation or graph allowance failed.
    Limit {
        limit: DensityLimit,
        count: usize,
        maximum: usize,
    },
}

/// Resource and field context plus a typed failure; caller inputs remain unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("density compilation of {resource} at {field} failed: {kind:?}")]
pub struct DensityCompileError {
    resource: &'static str,
    field: &'static str,
    kind: DensityCompileErrorKind,
    offset: Option<usize>,
}
impl DensityCompileError {
    /// Canonical original resource currently being admitted.
    pub const fn resource(&self) -> &'static str {
        self.resource
    }
    /// Original schema field, or `$` for syntax/whole-resource checks.
    pub const fn field(&self) -> &'static str {
        self.field
    }
    /// Classified rejection, without requiring message matching.
    pub const fn kind(&self) -> &DensityCompileErrorKind {
        &self.kind
    }
    /// Byte offset when a JSON syntax/admission rejection supplies one.
    pub const fn offset(&self) -> Option<usize> {
        self.offset
    }
}

/// An immutable seed-free builtin density graph. Clones share owned checked IR.
#[derive(Debug, Clone)]
pub struct DensityTemplate {
    resource: DensityFunctionResourceId,
    graph: Arc<Graph>,
    root_stats: DensityGraphStats,
}
impl DensityTemplate {
    /// Compiles all 35 pinned density definitions, including unused graphs,
    /// and selects this named root. No raw/dynamic JSON API or sampling exists.
    pub fn compile_definition(
        resource: DensityFunctionResourceId,
    ) -> Result<Self, DensityCompileError> {
        compile::density(resource)
    }
    /// Selected original density resource.
    pub const fn resource(&self) -> DensityFunctionResourceId {
        self.resource
    }
    /// Reachable graph measurements for the selected root.
    pub const fn root_stats(&self) -> DensityGraphStats {
        self.root_stats
    }
    /// All retained/validated definitions, including unused graphs.
    pub fn owned_stats(&self) -> DensityGraphStats {
        self.graph.stats
    }
    /// Distinct admitted source bytes, before parsing, including referenced noise.
    pub fn source_bytes(&self) -> usize {
        self.graph.source_bytes
    }
    /// Referenced noise resources in deterministic canonical resource order.
    pub fn noise_resources(&self) -> &[NoiseResourceId] {
        &self.graph.noises
    }
}

/// An immutable seed-free builtin router; deferred material/surface payloads
/// are retained privately but are not compiled or executed by this contract.
#[derive(Debug, Clone)]
pub struct RouterTemplate {
    resource: NoiseSettingsResourceId,
    graph: Arc<Graph>,
    metadata: graph::RouterMetadata,
    field_stats: [DensityGraphStats; 15],
}
impl RouterTemplate {
    /// Compiles all 35 density definitions plus the selected original settings
    /// router, retaining all original holder and marker identities.
    pub fn compile_builtin(resource: NoiseSettingsResourceId) -> Result<Self, DensityCompileError> {
        compile::router(resource)
    }
    /// Selected original settings resource.
    pub const fn resource(&self) -> NoiseSettingsResourceId {
        self.resource
    }
    /// Checked original noise-cell geometry.
    pub const fn geometry(&self) -> NoiseGeometry {
        self.metadata.geometry
    }
    /// Original sea level; no material selection is implemented here.
    pub const fn sea_level(&self) -> i32 {
        self.metadata.sea_level
    }
    /// Original random-source flag (true means legacy; false means xoroshiro).
    pub const fn uses_legacy_random_source(&self) -> bool {
        matches!(
            self.metadata.algorithm,
            crate::random::RandomAlgorithm::Legacy
        )
    }
    /// Original aquifer-enable flag; no aquifer implementation is activated.
    pub const fn aquifers_enabled(&self) -> bool {
        self.metadata.aquifers
    }
    /// Original ore-vein flag; no ore implementation is activated.
    pub const fn ore_veins_enabled(&self) -> bool {
        self.metadata.ore_veins
    }
    /// Original mob-generation flag; this compiler creates no entities.
    pub const fn mob_generation_disabled(&self) -> bool {
        self.metadata.disable_mobs
    }
    /// Reachable graph measurements for an explicitly named router field.
    pub fn field_stats(&self, field: RouterField) -> DensityGraphStats {
        self.field_stats[field as usize]
    }
    /// All retained/validated definitions and the selected router.
    pub fn owned_stats(&self) -> DensityGraphStats {
        self.graph.stats
    }
    /// Distinct admitted source bytes, charged before any resource is parsed.
    pub fn source_bytes(&self) -> usize {
        self.graph.source_bytes
    }
    /// Referenced noise resources in deterministic canonical resource order.
    pub fn noise_resources(&self) -> &[NoiseResourceId] {
        &self.graph.noises
    }
}
