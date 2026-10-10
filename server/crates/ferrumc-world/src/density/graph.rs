//! Private immutable graph and joint cycle/depth admission.

use std::collections::BTreeMap;

use ferrumc_registry::worldgen::{DensityFunctionResourceId, NoiseResourceId};

use super::{
    DensityCompileError, DensityCompileErrorKind as ErrorKind, DensityGraphStats, DensityLimit,
    NoiseGeometry, MAX_DENSITY_DEPTH, MAX_DENSITY_VERTICES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Binary {
    Add,
    Mul,
    Min,
    Max,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unary {
    Abs,
    Square,
    Cube,
    HalfNegative,
    QuarterNegative,
    Squeeze,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Marker {
    Interpolated,
    FlatCache,
    Cache2D,
    CacheOnce,
    CacheAllInCell,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shift {
    All,
    A,
    B,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unit {
    EndIslands,
    BlendAlpha,
    BlendOffset,
    Beardifier,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rarity {
    Type1,
    Type2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HolderOrigin {
    Reference(DensityFunctionResourceId),
    Direct,
}

#[derive(Debug, Clone)]
pub(super) struct Point {
    pub location: f32,
    pub derivative: f32,
    pub value: usize,
}

#[derive(Debug, Clone)]
pub(super) enum NodeKind {
    Constant(f64),
    // Direct HolderHolder is an actual original codec node, even around a
    // constant. Its factory identity must survive until the real visitor pass.
    Holder {
        origin: HolderOrigin,
        target: usize,
    },
    // Spline.Coordinate uses a raw Holder, not HOLDER_HELPER_CODEC. Its identity
    // is kept separate from a density HolderHolder while joining cycle checks.
    Coordinate {
        origin: HolderOrigin,
        target: usize,
    },
    Binary {
        operation: Binary,
        left: usize,
        right: usize,
    },
    Unary {
        operation: Unary,
        input: usize,
    },
    Marker {
        marker: Marker,
        input: usize,
    },
    BlendDensity(usize),
    Clamp {
        input: usize,
        min: f64,
        max: f64,
    },
    RangeChoice {
        input: usize,
        min: f64,
        max: f64,
        inside: usize,
        outside: usize,
    },
    Gradient {
        from_y: i32,
        to_y: i32,
        from_value: f64,
        to_value: f64,
    },
    Noise {
        resource: NoiseResourceId,
        xz: f64,
        y: f64,
    },
    Shift {
        shift: Shift,
        resource: NoiseResourceId,
    },
    ShiftedNoise {
        shift_x: usize,
        shift_y: usize,
        shift_z: usize,
        resource: NoiseResourceId,
        xz: f64,
        y: f64,
    },
    Weird {
        input: usize,
        resource: NoiseResourceId,
        rarity: Rarity,
    },
    Blended {
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear: f64,
    },
    Unit(Unit),
    Spline(usize),
    SplineConstant(f32),
    SplineMultipoint {
        coordinate: usize,
        points: Box<[Point]>,
    },
}
impl NodeKind {
    pub(super) fn is_density(&self) -> bool {
        !matches!(
            self,
            Self::Coordinate { .. } | Self::SplineConstant(_) | Self::SplineMultipoint { .. }
        )
    }
    pub(super) fn valid_child(&self, index: usize, child: &Self) -> bool {
        match self {
            Self::Spline(_) => matches!(
                child,
                Self::SplineConstant(_) | Self::SplineMultipoint { .. }
            ),
            Self::SplineMultipoint { .. } if index == 0 => matches!(child, Self::Coordinate { .. }),
            Self::SplineMultipoint { .. } => matches!(
                child,
                Self::SplineConstant(_) | Self::SplineMultipoint { .. }
            ),
            _ => child.is_density(),
        }
    }
    pub fn children(&self) -> usize {
        match self {
            Self::Holder { .. }
            | Self::Coordinate { .. }
            | Self::Unary { .. }
            | Self::Marker { .. }
            | Self::BlendDensity(_)
            | Self::Clamp { .. }
            | Self::Weird { .. }
            | Self::Spline(_) => 1,
            Self::Binary { .. } => 2,
            Self::RangeChoice { .. } | Self::ShiftedNoise { .. } => 3,
            Self::SplineMultipoint { points, .. } => 1 + points.len(),
            _ => 0,
        }
    }
    pub fn child(&self, index: usize) -> Option<usize> {
        match (self, index) {
            (Self::Holder { target, .. } | Self::Coordinate { target, .. }, 0) => Some(*target),
            (
                Self::Unary { input, .. }
                | Self::Marker { input, .. }
                | Self::Clamp { input, .. }
                | Self::Weird { input, .. }
                | Self::BlendDensity(input)
                | Self::Spline(input)
                | Self::RangeChoice { input, .. },
                0,
            ) => Some(*input),
            (Self::Binary { left, .. }, 0) => Some(*left),
            (Self::Binary { right, .. }, 1) => Some(*right),
            (Self::RangeChoice { inside, .. }, 1) => Some(*inside),
            (Self::RangeChoice { outside, .. }, 2) => Some(*outside),
            (Self::ShiftedNoise { shift_x, .. }, 0) => Some(*shift_x),
            (Self::ShiftedNoise { shift_y, .. }, 1) => Some(*shift_y),
            (Self::ShiftedNoise { shift_z, .. }, 2) => Some(*shift_z),
            (Self::SplineMultipoint { coordinate, .. }, 0) => Some(*coordinate),
            (Self::SplineMultipoint { points, .. }, i) => {
                points.get(i.checked_sub(1)?).map(|point| point.value)
            }
            _ => None,
        }
    }
    pub fn noise(&self) -> Option<NoiseResourceId> {
        match self {
            Self::Noise { resource, .. }
            | Self::Shift { resource, .. }
            | Self::ShiftedNoise { resource, .. }
            | Self::Weird { resource, .. } => Some(*resource),
            _ => None,
        }
    }
    // Admission rereads every descriptor, so no future consumer can receive
    // unchecked parameters even if a parser branch is extended independently.
    pub fn valid_parameters(&self) -> bool {
        match self {
            Self::Constant(value) => value.is_finite() && value.abs() <= 1_000_000.0,
            Self::SplineConstant(value) => value.is_finite(),
            Self::Clamp { min, max, .. } | Self::RangeChoice { min, max, .. } => [*min, *max]
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.0),
            Self::Noise { xz, y, .. } | Self::ShiftedNoise { xz, y, .. } => {
                xz.is_finite() && y.is_finite()
            }
            Self::Gradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => {
                (-4064..=4062).contains(from_y)
                    && (-4064..=4062).contains(to_y)
                    && [*from_value, *to_value]
                        .into_iter()
                        .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
            }
            Self::Blended {
                xz_scale,
                y_scale,
                xz_factor,
                y_factor,
                smear,
            } => {
                [*xz_scale, *y_scale, *xz_factor, *y_factor]
                    .into_iter()
                    .all(|v| v.is_finite() && (0.001..=1000.0).contains(&v))
                    && (1.0..=8.0).contains(smear)
            }
            Self::SplineMultipoint { points, .. } => {
                !points.is_empty()
                    && points
                        .iter()
                        .all(|p| p.location.is_finite() && p.derivative.is_finite())
                    && points
                        .windows(2)
                        .all(|pair| pair[0].location < pair[1].location)
            }
            Self::Holder { origin, .. } | Self::Coordinate { origin, .. } => match origin {
                HolderOrigin::Direct => true,
                HolderOrigin::Reference(id) => !id.resource_location().is_empty(),
            },
            Self::Binary { operation, .. } => matches!(
                operation,
                Binary::Add | Binary::Mul | Binary::Min | Binary::Max
            ),
            Self::Unary { operation, .. } => matches!(
                operation,
                Unary::Abs
                    | Unary::Square
                    | Unary::Cube
                    | Unary::HalfNegative
                    | Unary::QuarterNegative
                    | Unary::Squeeze
            ),
            Self::Marker { marker, .. } => matches!(
                marker,
                Marker::Interpolated
                    | Marker::FlatCache
                    | Marker::Cache2D
                    | Marker::CacheOnce
                    | Marker::CacheAllInCell
            ),
            Self::Shift { shift, .. } => matches!(shift, Shift::All | Shift::A | Shift::B),
            Self::Weird { rarity, .. } => matches!(rarity, Rarity::Type1 | Rarity::Type2),
            Self::Unit(unit) => matches!(
                unit,
                Unit::EndIslands | Unit::BlendAlpha | Unit::BlendOffset | Unit::Beardifier
            ),
            Self::BlendDensity(_) | Self::Spline(_) => true,
        }
    }
}

#[derive(Debug)]
pub(super) struct Node {
    pub resource: &'static str,
    pub kind: NodeKind,
    pub depth: usize,
}
#[derive(Debug)]
pub(super) struct Graph {
    pub nodes: Box<[Node]>,
    pub roots: BTreeMap<DensityFunctionResourceId, usize>,
    pub router_roots: Option<[usize; 15]>,
    pub stats: DensityGraphStats,
    pub source_bytes: usize,
    pub noises: Box<[NoiseResourceId]>,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct RouterMetadata {
    pub geometry: NoiseGeometry,
    pub sea_level: i32,
    pub algorithm: crate::random::RandomAlgorithm,
    pub aquifers: bool,
    pub ore_veins: bool,
    pub disable_mobs: bool,
    // Bounded original source payloads, deliberately not surface/material IR.
    pub default_block: &'static str,
    pub default_fluid: &'static str,
    pub surface_rule: &'static str,
    pub spawn_target: &'static str,
}

pub(super) fn validate(nodes: &mut [Node]) -> Result<DensityGraphStats, DensityCompileError> {
    if nodes.len() > MAX_DENSITY_VERTICES {
        return Err(error(
            "builtin",
            ErrorKind::Limit {
                limit: DensityLimit::Vertices,
                count: nodes.len(),
                maximum: MAX_DENSITY_VERTICES,
            },
        ));
    }
    let mut states = [0u8; MAX_DENSITY_VERTICES];
    let mut depths = vec![0usize; nodes.len()];
    let mut stack = Vec::with_capacity(nodes.len());
    // First finish the complete cycle pass. A long cycle is not mislabeled as
    // a depth error merely because its first traversed arm exceeds the cap.
    for start in 0..nodes.len() {
        if states[start] != 0 {
            continue;
        }
        states[start] = 1;
        stack.push((start, 0usize));
        while let Some((parent, next)) = stack.last_mut() {
            if let Some(child) = nodes[*parent].kind.child(*next) {
                *next += 1;
                if child >= nodes.len()
                    || !nodes[*parent]
                        .kind
                        .valid_child(*next - 1, &nodes[child].kind)
                {
                    return Err(error(nodes[*parent].resource, ErrorKind::ProducerDrift));
                }
                match states[child] {
                    0 => {
                        states[child] = 1;
                        stack.push((child, 0));
                    }
                    1 => {
                        return Err(error(
                            nodes[*parent].resource,
                            ErrorKind::Cycle {
                                from: *parent,
                                to: child,
                            },
                        ))
                    }
                    _ => {}
                }
            } else {
                let id = *parent;
                let mut depth = 1;
                for i in 0..nodes[id].kind.children() {
                    if let Some(child) = nodes[id].kind.child(i) {
                        depth = depth.max(depths[child] + 1);
                    }
                }
                depths[id] = depth;
                states[id] = 2;
                stack.pop();
            }
        }
    }
    for (id, node) in nodes.iter_mut().enumerate() {
        if depths[id] > MAX_DENSITY_DEPTH {
            return Err(error(
                node.resource,
                ErrorKind::Limit {
                    limit: DensityLimit::CombinedDepth,
                    count: depths[id],
                    maximum: MAX_DENSITY_DEPTH,
                },
            ));
        }
        if !node.kind.valid_parameters() {
            return Err(error(node.resource, ErrorKind::NumericRange));
        }
        node.depth = depths[id];
    }
    Ok(measure(nodes, None))
}

pub(super) fn measure(nodes: &[Node], roots: Option<&[usize]>) -> DensityGraphStats {
    let mut included = [false; MAX_DENSITY_VERTICES];
    if let Some(roots) = roots {
        let mut stack = Vec::with_capacity(nodes.len());
        // Mark at admission, so shared edges cannot grow the stack past V.
        for &root in roots {
            if !included[root] {
                included[root] = true;
                stack.push(root);
            }
        }
        while let Some(id) = stack.pop() {
            for i in 0..nodes[id].kind.children() {
                if let Some(child) = nodes[id].kind.child(i) {
                    if !included[child] {
                        included[child] = true;
                        stack.push(child);
                    }
                }
            }
        }
    } else {
        included[..nodes.len()].fill(true);
    }
    let mut stats = DensityGraphStats {
        vertices: 0,
        edges: 0,
        depth: 0,
        splines: 0,
        spline_points: 0,
        markers: 0,
    };
    for (id, node) in nodes.iter().enumerate() {
        if !included[id] {
            continue;
        }
        stats.vertices += 1;
        stats.edges += node.kind.children();
        stats.depth = stats.depth.max(node.depth);
        match &node.kind {
            NodeKind::SplineConstant(_) => stats.splines += 1,
            NodeKind::SplineMultipoint { points, .. } => {
                stats.splines += 1;
                stats.spline_points += points.len();
            }
            NodeKind::Marker { .. } => stats.markers += 1,
            _ => {}
        }
    }
    stats
}
fn error(resource: &'static str, kind: ErrorKind) -> DensityCompileError {
    DensityCompileError {
        resource,
        field: "$",
        kind,
        offset: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_combined_depth_and_unused_cross_kind_cycles() {
        let make = |kind| Node {
            resource: "test",
            kind,
            depth: 0,
        };
        let mut nodes = vec![
            make(NodeKind::Constant(0.0)),
            make(NodeKind::SplineConstant(0.0)),
        ];
        let mut density_root = 0;
        for _ in 0..19 {
            let coordinate = nodes.len();
            nodes.push(make(NodeKind::Coordinate {
                origin: HolderOrigin::Direct,
                target: density_root,
            }));
            let spline = nodes.len();
            nodes.push(make(NodeKind::SplineMultipoint {
                coordinate,
                points: vec![Point {
                    location: 0.0,
                    derivative: 0.0,
                    value: 1,
                }]
                .into_boxed_slice(),
            }));
            let density = nodes.len();
            nodes.push(make(NodeKind::Spline(spline)));
            let holder = nodes.len();
            nodes.push(make(NodeKind::Holder {
                origin: HolderOrigin::Direct,
                target: density,
            }));
            density_root = nodes.len();
            nodes.push(make(NodeKind::BlendDensity(holder)));
        }
        assert_eq!(validate(&mut nodes).unwrap().depth(), 96);
        nodes.push(make(NodeKind::Clamp {
            input: density_root,
            min: -1.0,
            max: 1.0,
        }));
        assert!(matches!(
            validate(&mut nodes).unwrap_err().kind(),
            ErrorKind::Limit {
                limit: DensityLimit::CombinedDepth,
                count: 97,
                maximum: 96
            }
        ));
        nodes[0].kind = NodeKind::Holder {
            origin: HolderOrigin::Direct,
            target: nodes.len() - 1,
        };
        assert!(matches!(
            validate(&mut nodes).unwrap_err().kind(),
            ErrorKind::Cycle { .. }
        ));
    }
}
