//! Typed, allocation-free admission and defensive failures.

use crate::biome_selection::BiomeErrorKind;
use crate::density::NoiseChunkErrorKind;
use crate::surface_rules::SurfaceRuleErrorKind;

/// Independent native capacity category, before its corresponding allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarverLimit {
    /// Immutable binding value, excluding static SIN.
    Binding,
    /// Independent owned output including all three boxes.
    Buffer,
    /// Run value and its bounded rule context.
    Run,
}
/// Immutable binding identity comparison order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingMismatch {
    /// World seed.
    Seed,
    /// Noise settings.
    Settings,
}
/// Published Surface identity comparison order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceMismatch {
    /// Input is Empty.
    NotPublished,
    /// World seed.
    Seed,
    /// Noise settings.
    Settings,
    /// Target chunk.
    Chunk,
}
/// Closed producer identity and lifecycle comparison order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerMismatch {
    /// World seed.
    Seed,
    /// Noise settings.
    Settings,
    /// Explicit original multinoise source.
    Source,
    /// Complete original build geometry.
    Geometry,
    /// Original Paper zoom.
    ZoomSemantics,
    /// Target or privately derived source halo.
    Chunk,
    /// One-shot run must be Ready.
    Lifecycle,
}
/// Independently admitted cumulative producer allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarverProducer {
    /// Shared Noise/Aquifer constructor and sampling work.
    Noise,
    /// Original history-bearing selector search.
    Selector,
    /// Existing surface-rule context work.
    Surface,
}
/// Concrete admission or defensive failure; none publishes partial output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarverErrorKind {
    /// A budget is outside its closed public domain.
    InvalidBudget { units: u64, maximum: u64 },
    /// Structural Carver work was refused before execution.
    WorkBudgetExceeded { required: u64, available: u64 },
    /// Native capacity was refused before allocation.
    NativeLimit {
        category: CarverLimit,
        required: usize,
        available: usize,
    },
    /// Binding and prepared rule identity disagree.
    InconsistentBinding(BindingMismatch),
    /// The independently published input does not belong to this run.
    InconsistentSurface(SurfaceMismatch),
    /// A closed producer's identity, domain or one-shot lifecycle disagrees.
    InconsistentProducer(ProducerMismatch),
    /// Exact cumulative admission or after-admission fault, little-endian limbs.
    ProducerWorkExceeded {
        producer: CarverProducer,
        required: [u64; 3],
        limit: [u64; 3],
        used: [u64; 3],
    },
    /// Existing checked Noise computation refused.
    Producer(NoiseChunkErrorKind),
    /// Existing original selector refused.
    Biome(BiomeErrorKind),
    /// Existing rule context refused.
    Rules(SurfaceRuleErrorKind),
    /// A state escaped the accepted closed alphabet.
    UnclassifiedState,
}
/// Typed refusal with no heap message construction.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("cold Carver runtime failed: {kind:?}")]
pub struct CarverError {
    kind: CarverErrorKind,
}
impl CarverError {
    pub(super) const fn new(kind: CarverErrorKind) -> Self {
        Self { kind }
    }
    /// Classified refusal or defensive fault.
    pub const fn kind(&self) -> &CarverErrorKind {
        &self.kind
    }
}
