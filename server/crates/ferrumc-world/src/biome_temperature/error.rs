use ferrumc_math::BlockPos;
use ferrumc_registry::biome::BiomeResourceId;

use crate::noise::NoiseError;

/// Independent source, structure and native-capacity admission categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeTemperatureLimit {
    /// Bytes in one complete source, at most 128 KiB.
    SourceBytes,
    /// Sum of distinct full source slices, at most 2 MiB.
    AggregateSourceBytes,
    /// Key/value nodes in one source, at most 4096.
    JsonNodes,
    /// Root-zero key/value depth, at most 32.
    JsonDepth,
    /// Object or array entries, at most 256.
    ContainerEntries,
    /// Decoded UTF-8 bytes per key/string, at most 256.
    StringBytes,
    /// Complete checked climate records, at most 65.
    Records,
    /// By-value table and noises, at most 8 KiB.
    RetainedBytes,
    /// Simultaneous native build/parser scratch, at most 1 MiB.
    ScratchBytes,
}

/// Classified failure, without a heap message or invented allocation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiomeTemperatureErrorKind {
    /// Caller allowance is outside 1..=1,048,576.
    InvalidAllowance(u32),
    /// Checked roster identity/order does not match the complete table.
    InvalidResource,
    /// A separately enforced source or native-capacity limit was exceeded.
    Limit {
        /// The independent category that failed admission.
        limit: BiomeTemperatureLimit,
        /// Observed quantity at refusal.
        count: usize,
        /// Inclusive admitted maximum.
        maximum: usize,
    },
    /// Runtime census differs from pinned Registry/original evidence.
    CensusMismatch {
        /// Actual runtime observation.
        observed: usize,
        /// Pinned required quantity.
        expected: usize,
    },
    /// Complete strict JSON syntax or escape validation failed.
    InvalidJson,
    /// Two object keys have the same decoded UTF-8 identity.
    DuplicateField,
    /// A mandatory climate field is absent.
    MissingClimateField,
    /// A climate field has the wrong type, value or nonfinite number.
    InvalidClimateField,
    /// The fixed census is refused before any construction work.
    WorkExhausted {
        /// Exact complete construction requirement.
        required: u32,
        /// Admitted caller allowance.
        allowance: u32,
    },
    /// An unchanged existing Simplex sampler refused its calculation.
    Noise(NoiseError),
    /// Completed temperature is nonfinite; payload preserves raw f32 bits.
    NonFiniteTemperature(u32),
}

/// Structured fixed-size context for construction or temperature failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("biome temperature {kind:?}, resource {resource:?}, field {field:?}, offset {offset:?}")]
pub struct BiomeTemperatureError {
    pub(super) kind: BiomeTemperatureErrorKind,
    resource: Option<BiomeResourceId>,
    field: Option<&'static str>,
    offset: Option<usize>,
    position: Option<BlockPos>,
    sea_level: Option<i32>,
}

impl BiomeTemperatureError {
    /// Borrows the classified failure and its fixed-size payload.
    pub const fn kind(&self) -> &BiomeTemperatureErrorKind {
        &self.kind
    }
    /// Checked biome identity, when the failing operation has one.
    pub const fn resource(&self) -> Option<BiomeResourceId> {
        self.resource
    }
    /// Static climate field name, when field projection failed.
    pub const fn field(&self) -> Option<&'static str> {
        self.field
    }
    /// Byte offset into the complete source, when parsing failed.
    pub const fn offset(&self) -> Option<usize> {
        self.offset
    }
    /// Actual query position, when a query failed.
    pub const fn position(&self) -> Option<BlockPos> {
        self.position
    }
    /// Actual query sea-level bits, when a query failed.
    pub const fn sea_level(&self) -> Option<i32> {
        self.sea_level
    }

    pub(super) fn new(kind: BiomeTemperatureErrorKind) -> Self {
        Self {
            kind,
            resource: None,
            field: None,
            offset: None,
            position: None,
            sea_level: None,
        }
    }
    pub(super) fn with_resource(mut self, resource: BiomeResourceId) -> Self {
        self.resource = Some(resource);
        self
    }
    pub(super) fn at_source(mut self, offset: usize, field: Option<&'static str>) -> Self {
        self.offset = Some(offset);
        self.field = field;
        self
    }
    pub(super) fn at_query(
        mut self,
        resource: BiomeResourceId,
        position: BlockPos,
        sea_level: i32,
    ) -> Self {
        self.resource = Some(resource);
        self.position = Some(position);
        self.sea_level = Some(sea_level);
        self
    }
}
