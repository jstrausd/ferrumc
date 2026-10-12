//! Rebuild actual mapped spline diagnostics with retained source-array identity.

use super::super::{
    bytes, core_charge as charge, core_fail as fail, limit, NoiseChunkCoreError as NoiseChunkError,
    NoiseChunkErrorKind, NoiseChunkLimit,
};
use crate::density::bind::{BoundKind, BoundNode, BoundSpline};
use crate::density::graph::{NodeKind, Point};
use crate::density::DensityBounds;
use crate::sample_work::SampleWork;
use crate::spline::{
    CubicSpline, SplineCoordinateId, SplineNodeId, SplineNodeSpec, SplinePoint,
    MAX_SPLINE_COORDINATES, MAX_SPLINE_NODES, MAX_SPLINE_POINTS, MAX_SPLINE_POINTS_PER_NODE,
};

fn invalid() -> NoiseChunkError {
    fail(NoiseChunkErrorKind::InvalidGraph)
}
pub(super) fn bounds(
    range: DensityBounds,
    points: &[Point],
    nodes: &[BoundNode],
    work: &mut SampleWork,
) -> Result<DensityBounds, NoiseChunkError> {
    limit(
        NoiseChunkLimit::Vertices,
        points.len(),
        MAX_SPLINE_POINTS_PER_NODE,
    )?;
    let mut local = Vec::with_capacity(points.len());
    let mut values = Vec::with_capacity(points.len());
    for (i, point) in points.iter().enumerate() {
        charge(work)?;
        local.push(
            SplinePoint::new(
                point.location,
                SplineNodeId::new(i as u32),
                point.derivative,
            )
            .map_err(|_| invalid())?,
        );
        let b = nodes.get(point.value).ok_or_else(invalid)?.bounds;
        values.push((b.min as f32, b.max as f32));
    }
    let (min, max) = CubicSpline::original_multipoint_bounds(
        range.min as f32,
        range.max as f32,
        &local,
        &values,
    );
    Ok(crate::density::bind::pair(f64::from(min), f64::from(max)))
}
pub(super) struct Plan {
    ids: Box<[usize]>,
    ordered: Vec<usize>,
    coordinates: Vec<usize>,
    point_count: usize,
}
impl Plan {
    pub fn retained_bytes(&self) -> usize {
        CubicSpline::retained_bytes_for_shape(
            self.ordered.len(),
            self.coordinates.len(),
            self.point_count,
        ) + self.coordinates.len() * std::mem::size_of::<usize>()
    }
}
// One bounded plan/build is live at a time. Fixed ID arrays avoid BTreeMap
// allocation overhead or growth hidden from cold-construction admission.
pub(super) fn temporary_bytes() -> usize {
    2 * 4096 * std::mem::size_of::<usize>()
        + 2 * MAX_SPLINE_POINTS * std::mem::size_of::<usize>()
        + MAX_SPLINE_POINTS * std::mem::size_of::<SplinePoint>()
        + MAX_SPLINE_NODES
            * (std::mem::size_of::<Vec<SplinePoint>>()
                + std::mem::size_of::<SplineNodeSpec<'_>>()
                + 64)
        + MAX_SPLINE_COORDINATES
            * (std::mem::size_of::<usize>()
                + std::mem::size_of::<crate::spline::SplineCoordinateRange>())
}
pub(super) fn plan(
    nodes: &[BoundNode],
    root: usize,
    work: &mut SampleWork,
) -> Result<Plan, NoiseChunkError> {
    let mut ids = vec![usize::MAX; 4096].into_boxed_slice();
    let mut pending = Vec::with_capacity(MAX_SPLINE_POINTS);
    pending.push(root);
    let mut ordered = Vec::with_capacity(MAX_SPLINE_NODES);
    let mut coordinates = Vec::with_capacity(MAX_SPLINE_COORDINATES);
    let mut point_count = 0;
    while let Some(id) = pending.pop() {
        charge(work)?;
        let slot = ids.get_mut(id).ok_or_else(invalid)?;
        if *slot != usize::MAX {
            continue;
        }
        limit(
            NoiseChunkLimit::Vertices,
            ordered.len() + 1,
            MAX_SPLINE_NODES,
        )?;
        *slot = ordered.len();
        ordered.push(id);
        match &nodes.get(id).ok_or_else(invalid)?.kind {
            BoundKind::Original(NodeKind::SplineConstant(_)) => {}
            BoundKind::Original(NodeKind::SplineMultipoint { coordinate, points }) => {
                if points.is_empty() {
                    return Err(invalid());
                }
                limit(
                    NoiseChunkLimit::Edges,
                    points.len(),
                    MAX_SPLINE_POINTS_PER_NODE,
                )?;
                point_count = bytes(
                    NoiseChunkLimit::Edges,
                    point_count,
                    points.len(),
                    1,
                    MAX_SPLINE_POINTS,
                )?;
                let mut found = false;
                for &prior in &coordinates {
                    charge(work)?;
                    if prior == *coordinate {
                        found = true;
                        break;
                    }
                }
                if !found {
                    limit(
                        NoiseChunkLimit::Vertices,
                        coordinates.len() + 1,
                        MAX_SPLINE_COORDINATES,
                    )?;
                    coordinates.push(*coordinate);
                }
                limit(
                    NoiseChunkLimit::Edges,
                    pending.len() + points.len(),
                    MAX_SPLINE_POINTS,
                )?;
                for point in points {
                    charge(work)?;
                    pending.push(point.value);
                }
            }
            _ => return Err(invalid()),
        }
    }
    // Coordinate order in CubicSpline.mapAll follows the node list; the
    // preflight DFS above retains exactly that order.
    Ok(Plan {
        ids,
        ordered,
        coordinates,
        point_count,
    })
}
pub(super) fn build(
    nodes: &[BoundNode],
    root: usize,
    plan: &Plan,
    work: &mut SampleWork,
) -> Result<BoundSpline, NoiseChunkError> {
    let mut coordinate_ids = vec![usize::MAX; 4096].into_boxed_slice();
    let mut coordinates = Vec::with_capacity(plan.coordinates.len());
    let mut ranges = Vec::with_capacity(plan.coordinates.len());
    for (i, &coordinate) in plan.coordinates.iter().enumerate() {
        charge(work)?;
        let BoundKind::Original(NodeKind::Coordinate { target, .. }) =
            nodes.get(coordinate).ok_or_else(invalid)?.kind
        else {
            return Err(invalid());
        };
        *coordinate_ids.get_mut(coordinate).ok_or_else(invalid)? = i;
        coordinates.push(target);
        let b = nodes[coordinate].bounds;
        ranges.push(CubicSpline::original_coordinate_range(
            b.min as f32,
            b.max as f32,
        ));
    }
    let mut lists = Vec::with_capacity(plan.ordered.len());
    for &id in &plan.ordered {
        charge(work)?;
        let points = match &nodes[id].kind {
            BoundKind::Original(NodeKind::SplineConstant(_)) => Vec::new(),
            BoundKind::Original(NodeKind::SplineMultipoint { points, .. }) => {
                let mut list = Vec::with_capacity(points.len());
                for point in points {
                    charge(work)?;
                    let child = *plan
                        .ids
                        .get(point.value)
                        .filter(|&&id| id != usize::MAX)
                        .ok_or_else(invalid)?;
                    list.push(
                        SplinePoint::new(
                            point.location,
                            SplineNodeId::new(child as u32),
                            point.derivative,
                        )
                        .map_err(|_| invalid())?,
                    );
                }
                list
            }
            _ => return Err(invalid()),
        };
        lists.push(points);
    }
    let mut specs = Vec::with_capacity(plan.ordered.len());
    for (i, &id) in plan.ordered.iter().enumerate() {
        charge(work)?;
        specs.push(
            match nodes[id].kind {
                BoundKind::Original(NodeKind::SplineConstant(value)) => {
                    SplineNodeSpec::constant(value)
                }
                BoundKind::Original(NodeKind::SplineMultipoint { coordinate, .. }) => {
                    SplineNodeSpec::multipoint(
                        SplineCoordinateId::new(coordinate_ids[coordinate] as u32),
                        &lists[i],
                    )
                }
                _ => return Err(invalid()),
            }
            .map_err(|_| invalid())?,
        );
    }
    let spline = CubicSpline::new(&ranges, &specs, SplineNodeId::new(plan.ids[root] as u32))
        .map_err(|_| invalid())?;
    if spline.retained_bytes() + coordinates.len() * std::mem::size_of::<usize>()
        != plan.retained_bytes()
    {
        return Err(invalid());
    }
    Ok(BoundSpline {
        spline,
        coordinates: coordinates.into_boxed_slice(),
    })
}
