//! Fixed-capacity admission and original float32 range diagnostics.

use super::{
    Graph, Node, NodeKind, SpecKind, SplineBuildError, SplineCoordinateRange, SplineNodeId,
    SplineNodeSpec, SplinePoint, MAX_SPLINE_COORDINATES, MAX_SPLINE_DEPTH, MAX_SPLINE_NODES,
    MAX_SPLINE_POINTS, MAX_SPLINE_POINTS_PER_NODE,
};

pub(super) fn points(points: &[SplinePoint]) -> Result<(), SplineBuildError> {
    if points.is_empty() {
        return Err(SplineBuildError::EmptyPoints);
    }
    if points.len() > MAX_SPLINE_POINTS_PER_NODE {
        return Err(SplineBuildError::TooManyPointsInNode {
            count: points.len(),
        });
    }
    for (index, pair) in points.windows(2).enumerate() {
        if pair[1].location <= pair[0].location {
            return Err(SplineBuildError::InvalidLocationOrder {
                point: index + 1,
                previous_bits: pair[0].location.to_bits(),
                current_bits: pair[1].location.to_bits(),
            });
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Frame {
    node: usize,
    next_point: usize,
}

pub(super) fn build(
    coordinates: &[SplineCoordinateRange],
    specs: &[SplineNodeSpec<'_>],
    root: SplineNodeId,
) -> Result<Graph, SplineBuildError> {
    let point_count = admit(coordinates, specs, root)?;
    let order = topological(specs)?;
    let (bounds, depth) = metrics(coordinates, specs, &order[..specs.len()])?;

    // No owned graph allocation occurs until every admission check has passed.
    let nodes = specs
        .iter()
        .enumerate()
        .map(|(index, spec)| Node {
            min: bounds[index].0,
            max: bounds[index].1,
            kind: match spec.kind {
                SpecKind::Constant(value) => NodeKind::Constant(value),
                SpecKind::Multipoint { coordinate, points } => NodeKind::Multipoint {
                    coordinate,
                    points: points.to_vec().into_boxed_slice(),
                },
            },
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok(Graph {
        nodes,
        coordinates: coordinates.to_vec().into_boxed_slice(),
        root: root.0 as usize,
        point_count,
        depth,
    })
}

fn admit(
    coordinates: &[SplineCoordinateRange],
    specs: &[SplineNodeSpec<'_>],
    root: SplineNodeId,
) -> Result<usize, SplineBuildError> {
    if specs.is_empty() {
        return Err(SplineBuildError::EmptyGraph);
    }
    if specs.len() > MAX_SPLINE_NODES {
        return Err(SplineBuildError::TooManyNodes { count: specs.len() });
    }
    if coordinates.len() > MAX_SPLINE_COORDINATES {
        return Err(SplineBuildError::TooManyCoordinates {
            count: coordinates.len(),
        });
    }
    if root.0 as usize >= specs.len() {
        return Err(SplineBuildError::UnknownNode {
            node: root,
            parent: None,
        });
    }
    let mut point_count = 0;
    for (index, spec) in specs.iter().enumerate() {
        if let SpecKind::Multipoint { coordinate, points } = spec.kind {
            point_count += points.len();
            if point_count > MAX_SPLINE_POINTS {
                return Err(SplineBuildError::TooManyPoints { count: point_count });
            }
            let node = SplineNodeId::new(index as u32);
            if coordinate.0 as usize >= coordinates.len() {
                return Err(SplineBuildError::UnknownCoordinate { node, coordinate });
            }
            for point in points {
                if point.value.0 as usize >= specs.len() {
                    return Err(SplineBuildError::UnknownNode {
                        node: point.value,
                        parent: Some(node),
                    });
                }
            }
        }
    }

    Ok(point_count)
}

fn topological(
    specs: &[SplineNodeSpec<'_>],
) -> Result<[usize; MAX_SPLINE_NODES], SplineBuildError> {
    // This explicit stack detects even long cycles without recursive calls.
    // Children complete before parents; completed shared nodes are not revisited
    // during construction, unlike sampling's deliberately repeated requests.
    let mut states = [0_u8; MAX_SPLINE_NODES];
    let mut stack = [Frame::default(); MAX_SPLINE_NODES];
    let mut order = [0_usize; MAX_SPLINE_NODES];
    let mut completed = 0;
    for start in 0..specs.len() {
        if states[start] != 0 {
            continue;
        }
        let mut stack_len = 1;
        stack[0] = Frame {
            node: start,
            next_point: 0,
        };
        states[start] = 1;
        while stack_len > 0 {
            let frame = stack[stack_len - 1];
            let child = match specs[frame.node].kind {
                SpecKind::Constant(_) => None,
                SpecKind::Multipoint { points, .. } => points.get(frame.next_point),
            };
            if let Some(point) = child {
                stack[stack_len - 1].next_point += 1;
                let child = point.value.0 as usize;
                match states[child] {
                    0 => {
                        states[child] = 1;
                        stack[stack_len] = Frame {
                            node: child,
                            next_point: 0,
                        };
                        stack_len += 1;
                    }
                    1 => {
                        return Err(SplineBuildError::Cycle {
                            node: SplineNodeId::new(frame.node as u32),
                            child: point.value,
                        })
                    }
                    _ => {}
                }
            } else {
                states[frame.node] = 2;
                order[completed] = frame.node;
                completed += 1;
                stack_len -= 1;
            }
        }
    }

    Ok(order)
}

type Bounds = [(f32, f32); MAX_SPLINE_NODES];

fn metrics(
    coordinates: &[SplineCoordinateRange],
    specs: &[SplineNodeSpec<'_>],
    order: &[usize],
) -> Result<(Bounds, usize), SplineBuildError> {
    let mut depths = [0_usize; MAX_SPLINE_NODES];
    let mut bounds = [(0.0_f32, 0.0_f32); MAX_SPLINE_NODES];
    let mut depth = 0;
    for &index in order {
        let node_depth = match specs[index].kind {
            SpecKind::Constant(_) => 1,
            SpecKind::Multipoint { points, .. } => {
                1 + points
                    .iter()
                    .map(|p| depths[p.value.0 as usize])
                    .max()
                    .unwrap_or(0)
            }
        };
        if node_depth > MAX_SPLINE_DEPTH {
            return Err(SplineBuildError::DepthExceeded {
                node: SplineNodeId::new(index as u32),
                depth: node_depth,
            });
        }
        depths[index] = node_depth;
        depth = depth.max(node_depth);
        bounds[index] = match specs[index].kind {
            SpecKind::Constant(value) => (value, value),
            SpecKind::Multipoint { coordinate, points } => {
                diagnostics(coordinates[coordinate.0 as usize], points, &bounds)
            }
        };
    }

    Ok((bounds, depth))
}

pub(super) fn diagnostics(
    range: SplineCoordinateRange,
    points: &[SplinePoint],
    bounds: &[(f32, f32)],
) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let first = points[0];
    let last = points[points.len() - 1];
    if range.min < first.location {
        let (low, high) = bounds[first.value.0 as usize];
        let low = super::linear_extend(range.min, first, low);
        let high = super::linear_extend(range.min, first, high);
        min = java_min(min, java_min(low, high));
        max = java_max(max, java_max(low, high));
    }
    if range.max > last.location {
        let (low, high) = bounds[last.value.0 as usize];
        let low = super::linear_extend(range.max, last, low);
        let high = super::linear_extend(range.max, last, high);
        min = java_min(min, java_min(low, high));
        max = java_max(max, java_max(low, high));
    }
    for point in points {
        let (low, high) = bounds[point.value.0 as usize];
        min = java_min(min, low);
        max = java_max(max, high);
    }
    for pair in points.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let width = right.location - left.location;
        let (left_min, left_max) = bounds[left.value.0 as usize];
        let (right_min, right_max) = bounds[right.value.0 as usize];
        if left.derivative != 0.0 || right.derivative != 0.0 {
            let left_slope = left.derivative * width;
            let right_slope = right.derivative * width;
            let low = java_min(left_min, right_min);
            let high = java_max(left_max, right_max);
            let low_delta = java_min(
                left_slope - right_max + left_min,
                -right_slope + right_min - left_max,
            );
            let high_delta = java_max(
                left_slope - right_min + left_max,
                -right_slope + right_max - left_min,
            );
            min = java_min(min, low + 0.25 * low_delta);
            max = java_max(max, high + 0.25 * high_delta);
        }
    }
    (min, max)
}

// Unlike Rust's min/max, Java propagates NaN and distinguishes signed zeros.
fn java_min(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        a
    } else if a == 0.0 && b == 0.0 && b.to_bits() == (-0.0_f32).to_bits() {
        b
    } else if a <= b {
        a
    } else {
        b
    }
}

fn java_max(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        a
    } else if a == 0.0 && b == 0.0 && a.to_bits() == (-0.0_f32).to_bits() {
        b
    } else if a >= b {
        a
    } else {
        b
    }
}
