//! Distinguish decoded codec objects from original registered factory objects.

use super::{
    error, invalid, DensityBindError, ErrorKind, Graph, HolderOrigin, NodeKind,
    MAX_DENSITY_VERTICES,
};
use crate::density::graph::{self, Node};
use crate::density::{DensityCompileErrorKind, MAX_DENSITY_EDGES};
use ferrumc_registry::worldgen::DensityFunctionResourceId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::density) enum SourceConstruction {
    Decoded,
    Registered,
    Shared,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::density) struct SourceOrigin {
    pub index: usize,
    pub construction: SourceConstruction,
    pub holder: Option<HolderOrigin>,
}
#[derive(Debug)]
pub(in crate::density) struct PreparedGraph {
    pub graph: Graph,
    pub origins: Box<[SourceOrigin]>,
    pub registered_roots: BTreeMap<DensityFunctionResourceId, usize>,
}
impl PreparedGraph {
    #[expect(
        clippy::too_many_lines,
        reason = "Single child-first pass preserves both original construction origins without expanding resource trees."
    )]
    pub fn new(source: &Graph) -> Result<Self, DensityBindError> {
        let resource = "registered_density_origins";
        if source.nodes.len() > MAX_DENSITY_VERTICES {
            return Err(error(
                resource,
                ErrorKind::Limit {
                    count: source.nodes.len(),
                    maximum: MAX_DENSITY_VERTICES,
                },
            ));
        }
        let mut edges: usize = source.nodes.iter().map(|node| node.kind.children()).sum();
        if edges > MAX_DENSITY_EDGES {
            return Err(error(
                resource,
                ErrorKind::Limit {
                    count: edges,
                    maximum: MAX_DENSITY_EDGES,
                },
            ));
        }
        let mut order: Vec<_> = (0..source.nodes.len()).collect();
        order.sort_by_key(|&i| source.nodes[i].depth);
        let registered = registered_closure(source, &order)?;
        let mut nodes: Vec<_> = source
            .nodes
            .iter()
            .map(|node| Node {
                resource: node.resource,
                kind: node.kind.clone(),
                depth: node.depth,
            })
            .collect();
        let mut origins: Vec<_> = source
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| SourceOrigin {
                index,
                construction: SourceConstruction::Decoded,
                holder: match node.kind {
                    NodeKind::Holder { origin, .. } | NodeKind::Coordinate { origin, .. } => {
                        Some(origin)
                    }
                    _ => None,
                },
            })
            .collect();
        let mut native = vec![usize::MAX; source.nodes.len()];
        for index in order {
            let node = &source.nodes[index];
            let mut decoded = node.kind.clone();
            if let NodeKind::Holder {
                origin: HolderOrigin::Reference(_),
                target,
            }
            | NodeKind::Coordinate {
                origin: HolderOrigin::Reference(_),
                target,
            } = &mut decoded
            {
                *target = mapped(&native, *target, node.resource)?;
            }
            nodes[index].kind = decoded.clone();
            if !registered[index] {
                continue;
            }
            // Only the original *native* factory representation omits the
            // JSON codec's inline HolderHolder. Decoded roots keep theirs.
            if let NodeKind::Holder {
                origin: HolderOrigin::Direct,
                target,
            } = node.kind
            {
                native[index] = mapped(&native, target, node.resource)?;
                continue;
            }
            let mut actual = node.kind.clone();
            remap_children(&mut actual, &native, node.resource)?;
            if (0..actual.children()).all(|i| actual.child(i) == decoded.child(i)) {
                native[index] = index;
                origins[index].construction = SourceConstruction::Shared;
            } else {
                let origin = SourceOrigin {
                    construction: SourceConstruction::Registered,
                    ..origins[index]
                };
                native[index] = append_variant(
                    &mut nodes,
                    &mut origins,
                    &mut edges,
                    actual,
                    origin,
                    node.resource,
                )?;
            }
        }
        let stats = graph::validate(&mut nodes).map_err(|source| match source.kind() {
            DensityCompileErrorKind::Limit { count, maximum, .. } => error(
                resource,
                ErrorKind::Limit {
                    count: *count,
                    maximum: *maximum,
                },
            ),
            _ => invalid(resource),
        })?;
        let registered_roots = source
            .roots
            .iter()
            .map(|(&id, &index)| Ok((id, mapped(&native, index, resource)?)))
            .collect::<Result<BTreeMap<_, _>, DensityBindError>>()?;
        Ok(Self {
            registered_roots,
            graph: Graph {
                nodes: nodes.into_boxed_slice(),
                roots: source.roots.clone(),
                router_roots: source.router_roots,
                stats,
                source_bytes: source.source_bytes,
                noises: source.noises.clone(),
            },
            origins: origins.into_boxed_slice(),
        })
    }
}
fn registered_closure(source: &Graph, order: &[usize]) -> Result<Vec<bool>, DensityBindError> {
    let resource = "registered_density_origins";
    let mut registered = vec![false; source.nodes.len()];
    for &index in source.roots.values() {
        *registered.get_mut(index).ok_or_else(|| invalid(resource))? = true;
    }
    // Descending admitted depth marks the complete resource closure without
    // an edge-sized pending stack or recursive expansion of references.
    for &index in order.iter().rev() {
        if registered[index] {
            let node = &source.nodes[index];
            for child in 0..node.kind.children() {
                let child = node
                    .kind
                    .child(child)
                    .ok_or_else(|| invalid(node.resource))?;
                *registered
                    .get_mut(child)
                    .ok_or_else(|| invalid(node.resource))? = true;
            }
        }
    }
    Ok(registered)
}
fn append_variant(
    nodes: &mut Vec<Node>,
    origins: &mut Vec<SourceOrigin>,
    edges: &mut usize,
    kind: NodeKind,
    origin: SourceOrigin,
    resource: &'static str,
) -> Result<usize, DensityBindError> {
    let count = nodes.len() + 1;
    if count > MAX_DENSITY_VERTICES {
        return Err(error(
            resource,
            ErrorKind::Limit {
                count,
                maximum: MAX_DENSITY_VERTICES,
            },
        ));
    }
    let count_edges = *edges + kind.children();
    if count_edges > MAX_DENSITY_EDGES {
        return Err(error(
            resource,
            ErrorKind::Limit {
                count: count_edges,
                maximum: MAX_DENSITY_EDGES,
            },
        ));
    }
    let index = nodes.len();
    nodes.push(Node {
        resource,
        kind,
        depth: 0,
    });
    origins.push(origin);
    *edges = count_edges;
    Ok(index)
}
fn mapped(
    mapping: &[usize],
    index: usize,
    resource: &'static str,
) -> Result<usize, DensityBindError> {
    mapping
        .get(index)
        .copied()
        .filter(|&i| i != usize::MAX)
        .ok_or_else(|| invalid(resource))
}
fn remap_children(
    kind: &mut NodeKind,
    mapping: &[usize],
    resource: &'static str,
) -> Result<(), DensityBindError> {
    let map = |index| mapped(mapping, index, resource);
    match kind {
        NodeKind::Holder { target, .. } | NodeKind::Coordinate { target, .. } => {
            *target = map(*target)?;
        }
        NodeKind::Unary { input, .. }
        | NodeKind::Marker { input, .. }
        | NodeKind::Clamp { input, .. }
        | NodeKind::Weird { input, .. }
        | NodeKind::BlendDensity(input)
        | NodeKind::Spline(input) => *input = map(*input)?,
        NodeKind::Binary { left, right, .. } => {
            *left = map(*left)?;
            *right = map(*right)?;
        }
        NodeKind::RangeChoice {
            input,
            inside,
            outside,
            ..
        } => {
            *input = map(*input)?;
            *inside = map(*inside)?;
            *outside = map(*outside)?;
        }
        NodeKind::ShiftedNoise {
            shift_x,
            shift_y,
            shift_z,
            ..
        } => {
            *shift_x = map(*shift_x)?;
            *shift_y = map(*shift_y)?;
            *shift_z = map(*shift_z)?;
        }
        NodeKind::SplineMultipoint { coordinate, points } => {
            *coordinate = map(*coordinate)?;
            for point in points {
                point.value = map(point.value)?;
            }
        }
        _ => {}
    }
    Ok(())
}
