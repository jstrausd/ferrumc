//! Allocation-free sampling with deterministic original callback ordering.

use super::{
    Graph, NodeKind, SplineCoordinateId, SplineNodeId, SplineSampleError, MAX_SPLINE_SAMPLE_WORK,
};

struct Work {
    left: usize,
    limit: usize,
}

impl Work {
    fn charge<E>(&mut self) -> Result<(), SplineSampleError<E>> {
        if self.left == 0 {
            Err(SplineSampleError::BudgetExceeded { limit: self.limit })
        } else {
            self.left -= 1;
            Ok(())
        }
    }
}

pub(super) fn sample<E>(
    graph: &Graph,
    budget: usize,
    coordinate: &mut impl FnMut(SplineCoordinateId) -> Result<f32, E>,
) -> Result<f32, SplineSampleError<E>> {
    if budget == 0 || budget > MAX_SPLINE_SAMPLE_WORK {
        return Err(SplineSampleError::InvalidBudget { budget });
    }
    let mut work = Work {
        left: budget,
        limit: budget,
    };
    visit(graph, graph.root, &mut work, coordinate)
}

fn visit<E>(
    graph: &Graph,
    index: usize,
    work: &mut Work,
    source: &mut impl FnMut(SplineCoordinateId) -> Result<f32, E>,
) -> Result<f32, SplineSampleError<E>> {
    work.charge()?;
    let result = match &graph.nodes[index].kind {
        NodeKind::Constant(value) => *value,
        NodeKind::Multipoint { coordinate, points } => {
            work.charge()?;
            let x = source(*coordinate).map_err(|source| SplineSampleError::CoordinateSource {
                coordinate: *coordinate,
                source,
            })?;
            if !x.is_finite() {
                return Err(SplineSampleError::NonFiniteCoordinate {
                    coordinate: *coordinate,
                    bits: x.to_bits(),
                });
            }
            // Original Mth.binarySearch finds first location strictly greater
            // than x. Equality therefore enters the interval to its right.
            let mut start = 0;
            let mut count = points.len();
            while count > 0 {
                work.charge()?;
                let half = count / 2;
                let middle = start + half;
                if x < points[middle].location {
                    count = half;
                } else {
                    start = middle + 1;
                    count -= half + 1;
                }
            }
            if start == 0 || start == points.len() {
                let endpoint = if start == 0 {
                    points[0]
                } else {
                    points[start - 1]
                };
                let value = visit(graph, endpoint.value.0 as usize, work, source)?;
                super::linear_extend(x, endpoint, value)
            } else {
                let left = points[start - 1];
                let right = points[start];
                let t = (x - left.location) / (right.location - left.location);
                let low = visit(graph, left.value.0 as usize, work, source)?;
                let high = visit(graph, right.value.0 as usize, work, source)?;
                let a = left.derivative * (right.location - left.location) - (high - low);
                let b = -right.derivative * (right.location - left.location) + (high - low);
                lerp(t, low, high) + t * (1.0 - t) * lerp(t, a, b)
            }
        }
    };
    if result.is_finite() {
        Ok(result)
    } else {
        Err(SplineSampleError::NonFiniteResult {
            node: SplineNodeId::new(index as u32),
            bits: result.to_bits(),
        })
    }
}

fn lerp(t: f32, start: f32, end: f32) -> f32 {
    start + t * (end - start)
}
