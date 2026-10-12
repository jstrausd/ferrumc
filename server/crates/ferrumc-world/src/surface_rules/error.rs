//! Typed surface-rule refusals; no message is allocated.

use ferrumc_math::BlockPos;

use crate::biome_temperature::BiomeTemperatureErrorKind;
use crate::stored_biome::StoredBiomeErrorKind;

/// Resolver identity component that differs from the session's binding.
///
/// Checked in this order, following `StoredBiomeIdentity` and then the zoom getter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverMismatch {
    /// Consistency: the resolver seed differs from the binding seed.
    Seed,
    /// Consistency: the resolver settings differ from the template settings.
    Settings,
    /// `FerrumC` policy P6: the source is not `MultiNoise(minecraft:overworld)`.
    Source,
    /// Consistency: the geometry is not the settings' `(min_y, height)`, i.e.
    /// the assumed level height `(-64, 384)`.
    Geometry,
    /// `FerrumC` policy P6: the zoom semantics are not Paper build 60.
    ZoomSemantics,
}

/// Separately enforced capacity that a refusal names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRuleLimit {
    /// UTF-8 bytes of the settings definition.
    DefinitionBytes,
    /// JSON values and object keys.
    JsonNodes,
    /// JSON depth.
    JsonDepth,
    /// JSON array or object entries.
    ContainerEntries,
    /// Decoded JSON string bytes.
    StringBytes,
    /// Original Gson INT token bytes.
    IntegerTokenBytes,
    /// Compiled nodes.
    Nodes,
    /// Compiled node depth (root zero).
    Depth,
    /// Children of one sequence.
    SequenceChildren,
    /// Biome names over all biome conditions.
    BiomeReferences,
    /// Biome names of one condition.
    BiomesPerCondition,
    /// Distinct bound noises.
    Noises,
    /// Distinct gradient random names.
    RandomNames,
    /// Distinct result states.
    States,
    /// Measured retained template bytes.
    TemplateBytes,
    /// Measured retained binding bytes.
    BindingBytes,
    /// Session value bytes.
    SessionBytes,
}

/// Grouping of refusal kinds for callers and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRuleRefusalCategory {
    /// A `FerrumC` policy (P1–P6), not original behaviour.
    Policy,
    /// A `FerrumC` capacity or allowance-domain limit.
    Limit,
    /// Malformed or inconsistent caller input or definition data.
    Input,
    /// Defensive evaluation failure; unreachable for admitted inputs, poisons.
    Defensive,
}

/// Classified surface-rule refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceRuleErrorKind {
    /// Limit: an allowance outside its admitted domain.
    InvalidBudget,
    /// Policy P3: only `overworld`, `large_biomes` and `amplified` with
    /// `legacy_random_source = false` are admitted.
    UnsupportedSettings,
    /// Policy P1: only `SurfaceNumericProfile::PaperBuild60` is admitted.
    UnsupportedNumericProfile,
    /// Policy P2: only `PaperFlatBedrock::Disabled` is admitted.
    UnsupportedFlatBedrock,
    /// Input: JSON syntax, codec shape, unknown field, number range or a
    /// definition that drifted from the pinned census.
    MalformedDefinition,
    /// Input: unknown rule or condition codec type.
    UnknownCodecType,
    /// Input: result state name or properties do not resolve.
    InvalidBlockState,
    /// Input: unknown biome name.
    UnknownBiome,
    /// Input: unknown noise name.
    UnknownNoise,
    /// Input: vertical anchor shape or range.
    InvalidAnchor,
    /// Limit: a native capacity would be exceeded.
    NativeLimit,
    /// Input/policy: the resolver does not match the session's binding.
    InconsistentResolver(ResolverMismatch),
    /// Input: the column is not inside the resolver's centre chunk.
    ColumnOutsideCentre,
    /// Input: no column has been begun in this session.
    NoActiveColumn,
    /// Policy P4: `y` outside the generation height `[-64, 319]`.
    BlockOutsideGenerationHeight,
    /// Policy P4: stone depth above or below outside `1..=4096`.
    InvalidStoneDepth,
    /// Policy P4: water height `Some(i32::MIN)` (the original "no water" code).
    InvalidWaterHeight,
    /// Policy P5: the remaining allowance is below the call's requirement;
    /// the call has no effect and the session is not poisoned.
    WorkBudgetExceeded,
    /// Defensive: the stored-biome resolver refused; poisons the session.
    Biome(StoredBiomeErrorKind),
    /// Defensive: the temperature table refused; poisons the session.
    Temperature(BiomeTemperatureErrorKind),
    /// Defensive: a noise sample or band index failed; poisons the session.
    Noise,
    /// Policy P5: an earlier defensive failure poisoned this session.
    Poisoned,
}

impl SurfaceRuleErrorKind {
    /// Policy, limit, input or defensive grouping of this kind.
    pub const fn category(&self) -> SurfaceRuleRefusalCategory {
        match self {
            Self::UnsupportedSettings
            | Self::UnsupportedNumericProfile
            | Self::UnsupportedFlatBedrock
            | Self::InconsistentResolver(
                ResolverMismatch::Source | ResolverMismatch::ZoomSemantics,
            )
            | Self::BlockOutsideGenerationHeight
            | Self::InvalidStoneDepth
            | Self::InvalidWaterHeight
            | Self::WorkBudgetExceeded
            | Self::Poisoned => SurfaceRuleRefusalCategory::Policy,
            Self::InvalidBudget | Self::NativeLimit => SurfaceRuleRefusalCategory::Limit,
            Self::MalformedDefinition
            | Self::UnknownCodecType
            | Self::InvalidBlockState
            | Self::UnknownBiome
            | Self::UnknownNoise
            | Self::InvalidAnchor
            | Self::InconsistentResolver(_)
            | Self::ColumnOutsideCentre
            | Self::NoActiveColumn => SurfaceRuleRefusalCategory::Input,
            Self::Biome(_) | Self::Temperature(_) | Self::Noise => {
                SurfaceRuleRefusalCategory::Defensive
            }
        }
    }
}

/// Typed refusal with optional block, node, limit and counts.
///
/// `requested`/`maximum` are zero unless the refusal concerns a count, a budget
/// or an allowance. No message is allocated; nothing is published on failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("surface rule refused: {kind:?}")]
pub struct SurfaceRuleError {
    kind: SurfaceRuleErrorKind,
    block: Option<BlockPos>,
    node: Option<usize>,
    limit: Option<SurfaceRuleLimit>,
    requested: usize,
    maximum: usize,
}

impl SurfaceRuleError {
    pub(crate) const fn new(kind: SurfaceRuleErrorKind) -> Self {
        Self {
            kind,
            block: None,
            node: None,
            limit: None,
            requested: 0,
            maximum: 0,
        }
    }
    pub(crate) const fn with_block(mut self, block: BlockPos) -> Self {
        self.block = Some(block);
        self
    }
    pub(crate) const fn with_node(mut self, node: usize) -> Self {
        self.node = Some(node);
        self
    }
    pub(crate) const fn with_limit(mut self, limit: SurfaceRuleLimit) -> Self {
        self.limit = Some(limit);
        self
    }
    pub(crate) const fn with_counts(mut self, requested: usize, maximum: usize) -> Self {
        self.requested = requested;
        self.maximum = maximum;
        self
    }
    /// Classified refusal.
    pub const fn kind(&self) -> &SurfaceRuleErrorKind {
        &self.kind
    }
    /// Block position of an evaluation refusal.
    pub const fn block(&self) -> Option<BlockPos> {
        self.block
    }
    /// Compiled (preorder) node of a definition or evaluation refusal.
    pub const fn node(&self) -> Option<usize> {
        self.node
    }
    /// Capacity of a `NativeLimit` refusal.
    pub const fn limit(&self) -> Option<SurfaceRuleLimit> {
        self.limit
    }
    /// Requested count, budget or required allowance.
    pub const fn requested(&self) -> usize {
        self.requested
    }
    /// Admitted maximum or remaining allowance.
    pub const fn maximum(&self) -> usize {
        self.maximum
    }
}

pub(crate) fn limit(
    limit: SurfaceRuleLimit,
    count: usize,
    maximum: usize,
) -> Result<(), SurfaceRuleError> {
    if count > maximum {
        return Err(SurfaceRuleError::new(SurfaceRuleErrorKind::NativeLimit)
            .with_limit(limit)
            .with_counts(count, maximum));
    }
    Ok(())
}
