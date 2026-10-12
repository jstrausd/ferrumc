//! Typed column-driver refusals; no message is allocated.

use ferrumc_math::{BlockPos, ChunkPos};

use super::super::{SurfaceRuleErrorKind, SurfaceRuleRefusalCategory};
use crate::biome_temperature::BiomeTemperatureErrorKind;

/// Binding component that differs from the given `SeededSurfaceRules`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingMismatch {
    /// Consistency: the column binding was bound for another world seed.
    Seed,
    /// Consistency: the column binding was bound for another settings resource.
    Settings,
}

/// Material component that differs from the binding or the resolver centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialMismatch {
    /// Consistency: the material was sampled for another world seed.
    Seed,
    /// Consistency: the material was sampled for another settings resource.
    Settings,
    /// Consistency: the material geometry is not `(min_y −64, height 384)` or
    /// its state or fluid arrays do not have one complete chunk's length.
    Geometry,
    /// Consistency: the material chunk is not the resolver's centre chunk.
    Chunk,
}

/// Separately enforced capacity that a `NativeLimit` refusal names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceColumnLimit {
    /// Measured retained bytes of one `SeededSurfaceColumns`.
    BindingBytes,
    /// Measured retained bytes of one `SurfaceChunkBuffer`.
    BufferBytes,
    /// Distinct states of the derived closed alphabet (policy D1).
    AlphabetStates,
}

/// Grouping of refusal kinds for callers and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceColumnRefusalCategory {
    /// A `FerrumC` policy, not original behaviour.
    Policy,
    /// A `FerrumC` capacity or allowance limit.
    Limit,
    /// Caller inputs that do not belong together.
    Consistency,
    /// Defensive failure; unreachable for admitted inputs.
    Defensive,
}

/// Classified column-driver refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceColumnErrorKind {
    /// Limit: an allowance outside its admitted domain.
    InvalidBudget,
    /// Limit / policy D2: the bind census or the chunk requirement `R` exceeds
    /// the given allowance; nothing is built or published.
    WorkBudgetExceeded,
    /// Limit: a native capacity would be exceeded (see `limit()`).
    NativeLimit,
    /// Consistency: the column binding does not belong to the given rules.
    InconsistentBinding(BindingMismatch),
    /// Consistency: the material does not belong to the rules or the resolver.
    InconsistentMaterial(MaterialMismatch),
    /// The GEN-05.9 session refused. Before column work this is the session
    /// admission (consistency or policy, see the inner kind); a refusal that
    /// carries a column is defensive and unreachable for admitted inputs.
    Rules(SurfaceRuleErrorKind),
    /// Defensive: an extension noise sample or random draw failed, or the
    /// frozen-ocean start lies above 320 (review condition A4).
    Noise,
    /// Defensive: the temperature table refused the melt predicate.
    Temperature(BiomeTemperatureErrorKind),
    /// Defensive / policy D1: a state outside the closed alphabet.
    UnclassifiedState,
}

impl SurfaceColumnErrorKind {
    /// Policy, limit, consistency or defensive grouping of this kind.
    pub const fn category(&self) -> SurfaceColumnRefusalCategory {
        match self {
            Self::InvalidBudget | Self::WorkBudgetExceeded | Self::NativeLimit => {
                SurfaceColumnRefusalCategory::Limit
            }
            Self::InconsistentBinding(_) | Self::InconsistentMaterial(_) => {
                SurfaceColumnRefusalCategory::Consistency
            }
            Self::Rules(kind) => match kind.category() {
                SurfaceRuleRefusalCategory::Policy => SurfaceColumnRefusalCategory::Policy,
                SurfaceRuleRefusalCategory::Limit => SurfaceColumnRefusalCategory::Limit,
                SurfaceRuleRefusalCategory::Input => SurfaceColumnRefusalCategory::Consistency,
                SurfaceRuleRefusalCategory::Defensive => SurfaceColumnRefusalCategory::Defensive,
            },
            Self::Noise | Self::Temperature(_) | Self::UnclassifiedState => {
                SurfaceColumnRefusalCategory::Defensive
            }
        }
    }
}

/// Typed refusal with optional chunk, column, block, limit and counts.
///
/// `requested`/`maximum` are zero unless the refusal concerns a count, a budget
/// or an allowance. No message is allocated; the output buffer stays `Empty`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("surface column refused: {kind:?}")]
pub struct SurfaceColumnError {
    kind: SurfaceColumnErrorKind,
    chunk: Option<ChunkPos>,
    column: Option<(u8, u8)>,
    block: Option<BlockPos>,
    limit: Option<SurfaceColumnLimit>,
    requested: usize,
    maximum: usize,
}

impl SurfaceColumnError {
    pub(super) const fn new(kind: SurfaceColumnErrorKind) -> Self {
        Self {
            kind,
            chunk: None,
            column: None,
            block: None,
            limit: None,
            requested: 0,
            maximum: 0,
        }
    }
    pub(super) const fn with_chunk(mut self, chunk: ChunkPos) -> Self {
        self.chunk = Some(chunk);
        self
    }
    pub(super) const fn with_column(mut self, local_x: u8, local_z: u8) -> Self {
        self.column = Some((local_x, local_z));
        self
    }
    pub(super) const fn with_block(mut self, block: BlockPos) -> Self {
        self.block = Some(block);
        self
    }
    pub(super) const fn with_limit(mut self, limit: SurfaceColumnLimit) -> Self {
        self.limit = Some(limit);
        self
    }
    pub(super) const fn with_counts(mut self, requested: usize, maximum: usize) -> Self {
        self.requested = requested;
        self.maximum = maximum;
        self
    }
    /// Classified refusal.
    pub const fn kind(&self) -> &SurfaceColumnErrorKind {
        &self.kind
    }
    /// Centre chunk of a `build_chunk` refusal.
    pub const fn chunk(&self) -> Option<ChunkPos> {
        self.chunk
    }
    /// Local `(x, z)` column of a refusal inside the column loop.
    pub const fn column(&self) -> Option<(u8, u8)> {
        self.column
    }
    /// Block position of a refusal, when one applies.
    pub const fn block(&self) -> Option<BlockPos> {
        self.block
    }
    /// Capacity of a `NativeLimit` refusal.
    pub const fn limit(&self) -> Option<SurfaceColumnLimit> {
        self.limit
    }
    /// Requested count, budget or required allowance.
    pub const fn requested(&self) -> usize {
        self.requested
    }
    /// Admitted maximum or given allowance.
    pub const fn maximum(&self) -> usize {
        self.maximum
    }
}

/// `NativeLimit` refusal when `count` exceeds `maximum`.
pub(super) fn limit(
    limit: SurfaceColumnLimit,
    count: usize,
    maximum: usize,
) -> Result<(), SurfaceColumnError> {
    if count > maximum {
        return Err(SurfaceColumnError::new(SurfaceColumnErrorKind::NativeLimit)
            .with_limit(limit)
            .with_counts(count, maximum));
    }
    Ok(())
}
