//! Bounded, immutable original Minecraft 1.21.8 float32 cubic splines.
//!
//! IDs address positions in one construction's specification slices. They are
//! not persistent or globally unique handles. The complete graph is checked
//! before any owned graph allocation; a clone shares only immutable data.
//! Coordinate ranges reproduce the original diagnostics, not sampling clamps.
//! Nonfinite sampled results fail at the first child or root node that produces
//! them. This checked policy can stop callbacks earlier than Java; successful
//! finite samples retain the original callback order.
//! Codecs, coordinate visitors, value transformers and density/terrain consumers
//! are separate contracts. This module activates no terrain generator.

mod evaluate;
mod validate;

use std::sync::Arc;

/// Maximum number of nodes, including constants and unreachable specifications.
pub const MAX_SPLINE_NODES: usize = 1024;
/// Maximum number of graph-local coordinate declarations.
pub const MAX_SPLINE_COORDINATES: usize = 64;
/// Maximum number of strictly ordered points in any multipoint node.
pub const MAX_SPLINE_POINTS_PER_NODE: usize = 64;
/// Maximum aggregate number of points across all node specifications.
pub const MAX_SPLINE_POINTS: usize = 16_384;
/// Maximum graph depth, counting a constant leaf as depth one.
pub const MAX_SPLINE_DEPTH: usize = 32;
/// Maximum work units in one sample: node entries, coordinate callbacks and
/// binary-search comparisons each consume one unit before being performed.
pub const MAX_SPLINE_SAMPLE_WORK: usize = 1_048_576;

/// A node specification's graph-local slice index; construction validates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SplineNodeId(u32);

impl SplineNodeId {
    /// Declares an index; this does not authorize it in any other graph.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the graph-local specification index.
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// A coordinate declaration's graph-local slice index; construction validates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SplineCoordinateId(u32);

impl SplineCoordinateId {
    /// Declares an index; equal indices in different graphs are separate bindings.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the graph-local declaration index delivered to the callback.
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// A coordinate's reported diagnostic range, never an enforced sample clamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineCoordinateRange {
    min: f32,
    max: f32,
}

impl SplineCoordinateRange {
    /// Declares finite ordered diagnostic endpoints, preserving signed zeros.
    /// Samples outside this range remain allowed if finite. One-sided infinite
    /// ranges are rejected; use [`Self::unlimited`] for the full infinite range.
    pub fn bounded(min: f32, max: f32) -> Result<Self, SplineBuildError> {
        if !min.is_finite() || !max.is_finite() || min > max {
            return Err(SplineBuildError::InvalidCoordinateRange {
                min_bits: min.to_bits(),
                max_bits: max.to_bits(),
            });
        }
        Ok(Self { min, max })
    }

    /// Declares the original unlimited diagnostic range, negative to positive
    /// infinity. Actual callback values must still be finite in this Rust API.
    pub const fn unlimited() -> Self {
        Self {
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }

    /// Returns the reported minimum, possibly negative infinity.
    pub const fn min_value(self) -> f32 {
        self.min
    }

    /// Returns the reported maximum, possibly positive infinity.
    pub const fn max_value(self) -> f32 {
        self.max
    }
}

/// A finite location, graph-local child value and finite endpoint derivative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplinePoint {
    location: f32,
    value: SplineNodeId,
    derivative: f32,
}

impl SplinePoint {
    /// Checks both float fields before creating an immutable point. Child IDs
    /// are validated against the complete graph by [`CubicSpline::new`].
    pub fn new(
        location: f32,
        value: SplineNodeId,
        derivative: f32,
    ) -> Result<Self, SplineBuildError> {
        finite(location, SplineFloatField::Location)?;
        finite(derivative, SplineFloatField::Derivative)?;
        Ok(Self {
            location,
            value,
            derivative,
        })
    }

    /// Returns the original float32 location.
    pub const fn location(self) -> f32 {
        self.location
    }

    /// Returns the graph-local child specification index.
    pub const fn value(self) -> SplineNodeId {
        self.value
    }

    /// Returns the original float32 endpoint derivative, preserving signed zero.
    pub const fn derivative(self) -> f32 {
        self.derivative
    }
}

/// An allocation-free borrowed specification, never an unchecked runtime node.
#[derive(Debug, Clone, Copy)]
pub struct SplineNodeSpec<'a> {
    kind: SpecKind<'a>,
}

#[derive(Debug, Clone, Copy)]
enum SpecKind<'a> {
    Constant(f32),
    Multipoint {
        coordinate: SplineCoordinateId,
        points: &'a [SplinePoint],
    },
}

impl<'a> SplineNodeSpec<'a> {
    /// Checks a constant before creating its allocation-free specification.
    pub fn constant(value: f32) -> Result<Self, SplineBuildError> {
        finite(value, SplineFloatField::Value)?;
        Ok(Self {
            kind: SpecKind::Constant(value),
        })
    }

    /// Borrows 1..=64 strictly ascending points without allocating. Duplicate
    /// locations, including positive/negative zero, are rejected. Coordinate and
    /// child IDs, aggregate size, cycles and depth are checked at graph creation.
    pub fn multipoint(
        coordinate: SplineCoordinateId,
        points: &'a [SplinePoint],
    ) -> Result<Self, SplineBuildError> {
        validate::points(points)?;
        Ok(Self {
            kind: SpecKind::Multipoint { coordinate, points },
        })
    }
}

/// The finite input field responsible for a construction rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineFloatField {
    /// Constant node value.
    Value,
    /// Point location.
    Location,
    /// Point derivative.
    Derivative,
}

/// Complete construction failure; no partial runtime graph escapes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SplineBuildError {
    /// Empty node specification list.
    #[error("a spline graph requires at least one node")]
    EmptyGraph,
    /// Too many node specifications.
    #[error("spline node count {count} exceeds {MAX_SPLINE_NODES}")]
    TooManyNodes { count: usize },
    /// Too many coordinate declarations.
    #[error("spline coordinate count {count} exceeds {MAX_SPLINE_COORDINATES}")]
    TooManyCoordinates { count: usize },
    /// A multipoint specification has no points.
    #[error("a multipoint spline requires at least one point")]
    EmptyPoints,
    /// One node exceeds its point allowance.
    #[error("spline point count {count} exceeds {MAX_SPLINE_POINTS_PER_NODE}")]
    TooManyPointsInNode { count: usize },
    /// The complete graph exceeds its aggregate point allowance.
    #[error("spline aggregate point count {count} exceeds {MAX_SPLINE_POINTS}")]
    TooManyPoints { count: usize },
    /// A finite point/value contract was violated; raw bits preserve the input.
    #[error("nonfinite spline {field:?} with float bits {bits:#010x}")]
    NonFiniteInput { field: SplineFloatField, bits: u32 },
    /// Diagnostic endpoints are nonfinite or reversed; unlimited is explicit.
    #[error("invalid spline coordinate range {min_bits:#010x}..{max_bits:#010x}")]
    InvalidCoordinateRange { min_bits: u32, max_bits: u32 },
    /// A location is equal to or less than its predecessor.
    #[error("spline point {point} is not strictly ascending")]
    InvalidLocationOrder {
        point: usize,
        previous_bits: u32,
        current_bits: u32,
    },
    /// Root or child index does not refer to a specification in this graph.
    #[error("unknown graph-local spline node {node:?}, referred by {parent:?}")]
    UnknownNode {
        node: SplineNodeId,
        parent: Option<SplineNodeId>,
    },
    /// A node references an undeclared graph-local coordinate.
    #[error("spline node {node:?} references unknown coordinate {coordinate:?}")]
    UnknownCoordinate {
        node: SplineNodeId,
        coordinate: SplineCoordinateId,
    },
    /// A child edge returns to a node being visited, including unused subgraphs.
    #[error("cyclic spline edge {node:?} -> {child:?}")]
    Cycle {
        node: SplineNodeId,
        child: SplineNodeId,
    },
    /// A node exceeds the depth allowance; constant leaves have depth one.
    #[error("spline node {node:?} depth {depth} exceeds {MAX_SPLINE_DEPTH}")]
    DepthExceeded { node: SplineNodeId, depth: usize },
}

/// Sampling rejection. Graph data always remain unchanged, but previously
/// invoked callback effects are not rolled back. Source errors remain typed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SplineSampleError<E> {
    /// Budget admission failed before invoking any callback.
    #[error("spline work budget {budget} must be in 1..={MAX_SPLINE_SAMPLE_WORK}")]
    InvalidBudget { budget: usize },
    /// No more work is performed after this bounded budget is exhausted.
    #[error("spline sample exhausted its {limit} work units")]
    BudgetExceeded { limit: usize },
    /// The coordinate callback failed, after any earlier ordered requests.
    #[error("spline coordinate {coordinate:?} failed: {source}")]
    CoordinateSource {
        coordinate: SplineCoordinateId,
        source: E,
    },
    /// The source returned a nonfinite float; diagnostics do not permit it.
    #[error("spline coordinate {coordinate:?} returned nonfinite bits {bits:#010x}")]
    NonFiniteCoordinate {
        coordinate: SplineCoordinateId,
        bits: u32,
    },
    /// Original float32 arithmetic produced a nonfinite child or root result.
    /// Sampling stops at that node, before any remaining right-child callbacks.
    /// Java may continue evaluating and produce a different nonfinite root value
    /// and callback trace. This early checked error is additional Rust policy.
    #[error("spline node {node:?} produced nonfinite bits {bits:#010x}")]
    NonFiniteResult { node: SplineNodeId, bits: u32 },
}

/// Immutable owned constant/multipoint DAG with original float32 diagnostics.
/// Cloning shares one immutable graph; input slices are never retained.
#[derive(Debug, Clone)]
pub struct CubicSpline {
    graph: Arc<Graph>,
}

#[derive(Debug)]
struct Graph {
    nodes: Box<[Node]>,
    coordinates: Box<[SplineCoordinateRange]>,
    root: usize,
    point_count: usize,
    depth: usize,
}

#[derive(Debug)]
struct Node {
    kind: NodeKind,
    min: f32,
    max: f32,
}

#[derive(Debug)]
enum NodeKind {
    Constant(f32),
    Multipoint {
        coordinate: SplineCoordinateId,
        points: Box<[SplinePoint]>,
    },
}

impl CubicSpline {
    /// Checks every specification, including unused nodes, before allocating
    /// the owned graph. Limits are 1024 nodes, 64 coordinates, 64 points per node,
    /// 16384 total points and depth 32 (constant leaf = 1). Cycles and unknown
    /// indices fail. Equal numeric indices from different inputs cannot carry
    /// provenance: IDs are explicitly graph-local declaration indices.
    ///
    /// Min/max use the original operation order and Java float min/max semantics.
    /// They may be infinite or NaN due to unlimited ranges or arithmetic overflow;
    /// they are diagnostics, never additional acceptance bounds for callbacks.
    pub fn new(
        coordinates: &[SplineCoordinateRange],
        nodes: &[SplineNodeSpec<'_>],
        root: SplineNodeId,
    ) -> Result<Self, SplineBuildError> {
        validate::build(coordinates, nodes, root).map(|graph| Self {
            graph: Arc::new(graph),
        })
    }

    /// Samples without internal heap allocation, I/O, mutation or random draws.
    /// A budget in 1..=1048576 is required; node entry, coordinate callback and
    /// each binary-search comparison cost one unit before that operation.
    ///
    /// Each multipoint requests its coordinate once. Lower/upper extension
    /// evaluates only the endpoint child; interiors evaluate left then right,
    /// including exact interior locations. Shared children and coordinates are
    /// requested repeatedly, never memoized. Callback errors stop immediately;
    /// earlier callback effects are not rolled back. Actual coordinates and
    /// sampled child/root results must be finite, an explicit difference from
    /// Java. A nonfinite child stops evaluation before remaining right-child
    /// callbacks; no parity is claimed for Java's continued nonfinite traces.
    ///
    /// All interpolation/extension arithmetic is f32 in original order, without
    /// deliberate FMA. A future density consumer must cast f64 coordinates to
    /// f32 before this call and widen the result afterwards, as Minecraft does.
    pub fn sample_with<E>(
        &self,
        budget: usize,
        coordinate: &mut impl FnMut(SplineCoordinateId) -> Result<f32, E>,
    ) -> Result<f32, SplineSampleError<E>> {
        evaluate::sample(&self.graph, budget, coordinate)
    }

    /// Returns the original root minimum diagnostic, including signed zero,
    /// infinity or NaN. This is not a bound for arbitrary coordinate callbacks.
    pub fn min_value(&self) -> f32 {
        self.graph.nodes[self.graph.root].min
    }

    /// Returns the original root maximum diagnostic, including signed zero,
    /// infinity or NaN. This is not a bound for arbitrary coordinate callbacks.
    pub fn max_value(&self) -> f32 {
        self.graph.nodes[self.graph.root].max
    }

    /// Returns all owned nodes, including unused but validated specifications.
    pub fn node_count(&self) -> usize {
        self.graph.nodes.len()
    }

    /// Returns the number of owned coordinate declarations.
    pub fn coordinate_count(&self) -> usize {
        self.graph.coordinates.len()
    }

    /// Returns the aggregate points in every owned node.
    pub fn point_count(&self) -> usize {
        self.graph.point_count
    }

    /// Returns maximum depth of all nodes, counting constants as depth one.
    pub fn depth(&self) -> usize {
        self.graph.depth
    }

    /// Returns this graph's reported coordinate range, or `None` for an unknown
    /// local index. This lookup performs no coordinate callback.
    pub fn coordinate_range(&self, id: SplineCoordinateId) -> Option<SplineCoordinateRange> {
        self.graph.coordinates.get(id.0 as usize).copied()
    }
}

fn finite(value: f32, field: SplineFloatField) -> Result<(), SplineBuildError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SplineBuildError::NonFiniteInput {
            field,
            bits: value.to_bits(),
        })
    }
}

fn linear_extend(coordinate: f32, point: SplinePoint, value: f32) -> f32 {
    if point.derivative == 0.0 {
        value
    } else {
        value + point.derivative * (coordinate - point.location)
    }
}
