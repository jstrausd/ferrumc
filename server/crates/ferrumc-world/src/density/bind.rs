//! Original `RandomState` binding and the two actual factory-reconstruction views.

use std::collections::BTreeMap;
use std::sync::Arc;

use ferrumc_registry::worldgen::{
    DensityFunctionResourceId, NoiseResourceId, NoiseSettingsResourceId,
};

use super::graph::{Binary, Graph, HolderOrigin, NodeKind, Rarity, Unit};

mod origin;
use super::seeded::{
    DensityBindError, DensityBindErrorKind as ErrorKind, DensityBounds, SeededRouter,
};
use super::{RouterTemplate, MAX_DENSITY_VERTICES};
use crate::noise::{BlendedNoise, BlendedNoiseParameters, NormalNoise, SimplexNoise};
use crate::random::{PositionalRandomFactory, WorldgenRandomSource};
use crate::spline::{CubicSpline, SplineCoordinateId, SplineNodeId, SplineNodeSpec, SplinePoint};
use crate::WorldSeed;
pub(super) use origin::{PreparedGraph, SourceConstruction, SourceOrigin};

#[derive(Debug, Clone)]
pub(super) enum BoundKind {
    Original(NodeKind),
    // An actual visitor removed a holder/marker; this is not a compute entry.
    Alias(usize),
    MulOrAdd {
        operation: Binary,
        input: usize,
        argument: f64,
    },
}

#[derive(Debug, Clone)]
pub(super) struct BoundSpline {
    pub spline: CubicSpline,
    pub coordinates: Box<[usize]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VisitorPass {
    SeedRouter,
    Climate,
}

#[derive(Debug, Clone)]
pub(super) struct BoundNode {
    pub origin: SourceOrigin,
    pub resource: &'static str,
    pub kind: BoundKind,
    pub bounds: DensityBounds,
    pub noise: Option<usize>,
    pub blended: Option<Arc<BlendedNoise>>,
    pub spline: Option<BoundSpline>,
}

#[derive(Debug)]
pub(super) struct BoundView {
    pub pass: VisitorPass,
    pub nodes: Vec<BoundNode>,
}

#[derive(Debug)]
pub(super) struct BoundRouter {
    pub source: Arc<PreparedGraph>,
    pub metadata: super::graph::RouterMetadata,
    pub resource: NoiseSettingsResourceId,
    pub seed: WorldSeed,
    pub router: BoundView,
    pub climate: BoundView,
    pub fields: [usize; 15],
    pub definitions: BTreeMap<DensityFunctionResourceId, usize>,
    pub climate_fields: [usize; 6],
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Original registered roots retain independently audited native factory provenance."
        )
    )]
    pub registered_definitions: BTreeMap<DensityFunctionResourceId, usize>,
    pub noises: Box<[(NoiseResourceId, NormalNoise)]>,
    pub end: SimplexNoise,
    pub aquifer_random: PositionalRandomFactory,
    pub ore_random: PositionalRandomFactory,
}

pub(super) fn bind(
    template: &RouterTemplate,
    seed: WorldSeed,
) -> Result<SeededRouter, DensityBindError> {
    let prepared = PreparedGraph::new(&template.graph)?;
    let graph = &prepared.graph;
    let resource = template.resource.resource_location();
    let fields = graph.router_roots.ok_or_else(|| invalid(resource))?;
    if graph.nodes.len() > MAX_DENSITY_VERTICES {
        return Err(error(
            resource,
            ErrorKind::Limit {
                count: graph.nodes.len(),
                maximum: MAX_DENSITY_VERTICES,
            },
        ));
    }
    let mut initial = WorldgenRandomSource::new(template.metadata.algorithm, seed);
    let positional = initial.fork_positional();
    let aquifer_random = positional
        .from_hash_of("minecraft:aquifer")
        .fork_positional();
    let ore_random = positional.from_hash_of("minecraft:ore").fork_positional();
    let legacy = template.uses_legacy_random_source();
    let mut noises = Vec::with_capacity(graph.noises.len());
    for &id in &graph.noises {
        let name = id.resource_location();
        let sample = bind_noise(legacy, seed, positional, id)
            .map_err(|source| error(name, ErrorKind::Noise(source)))?;
        noises.push((id, sample));
    }
    let mut end_random = WorldgenRandomSource::legacy(seed);
    end_random
        .consume_count(17292)
        .map_err(|source| error(resource, ErrorKind::Random(source)))?;
    let end = SimplexNoise::new(&mut end_random);
    let mut blended = BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        if let NodeKind::Blended {
            xz_scale,
            y_scale,
            xz_factor,
            y_factor,
            smear,
        } = node.kind
        {
            let parameters =
                BlendedNoiseParameters::new(xz_scale, y_scale, xz_factor, y_factor, smear)
                    .map_err(|source| error(node.resource, ErrorKind::Noise(source)))?;
            let mut random = if legacy {
                WorldgenRandomSource::legacy(seed)
            } else {
                positional.from_hash_of("minecraft:terrain")
            };
            let noise = BlendedNoise::new(&mut random, parameters)
                .map_err(|source| error(node.resource, ErrorKind::Noise(source)))?;
            blended.insert(index, Arc::new(noise));
        }
    }
    let router = lower(&prepared, &noises, &blended, None)?;
    let climate = lower(&prepared, &noises, &blended, Some(&router))?;
    let climate_fields = [
        fields[4], fields[5], fields[6], fields[7], fields[8], fields[9],
    ];
    Ok(SeededRouter {
        data: Arc::new(BoundRouter {
            resource: template.resource,
            seed,
            router,
            climate,
            fields,
            definitions: graph.roots.clone(),
            climate_fields,
            registered_definitions: prepared.registered_roots.clone(),
            noises: noises.into_boxed_slice(),
            end,
            aquifer_random,
            ore_random,
            metadata: template.metadata,
            source: Arc::new(prepared),
        }),
    })
}

pub(super) fn bind_noise(
    legacy: bool,
    seed: WorldSeed,
    positional: PositionalRandomFactory,
    id: NoiseResourceId,
) -> Result<NormalNoise, crate::noise::NoiseError> {
    let name = id.resource_location();
    let parameters = id.parameters();
    match (legacy, name) {
        (true, "minecraft:temperature" | "minecraft:vegetation") => {
            let addition = i64::from(name == "minecraft:vegetation");
            let mut random =
                WorldgenRandomSource::legacy(WorldSeed::new(seed.value().wrapping_add(addition)));
            NormalNoise::legacy(&mut random, -7, &[1.0, 1.0])
        }
        (true, "minecraft:offset") => {
            NormalNoise::new(&mut positional.from_hash_of(name), 0, &[0.0])
        }
        _ => NormalNoise::new(
            &mut positional.from_hash_of(name),
            parameters.first_octave(),
            parameters.amplitudes(),
        ),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "One exhaustive dispatch keeps actual factory reconstruction order auditable."
)]
fn lower(
    prepared: &PreparedGraph,
    noises: &[(NoiseResourceId, NormalNoise)],
    blended: &BTreeMap<usize, Arc<BlendedNoise>>,
    prior: Option<&BoundView>,
) -> Result<BoundView, DensityBindError> {
    let graph = &prepared.graph;
    let climate = prior.is_some();
    // The admitted combined depth provides a child-first DAG order, including
    // raw coordinate and density/spline transitions. No tree expansion.
    let mut order: Vec<_> = (0..graph.nodes.len()).collect();
    order.sort_by_key(|&index| graph.nodes[index].depth);
    let mut nodes: Vec<Option<BoundNode>> = (0..graph.nodes.len()).map(|_| None).collect();
    for index in order {
        let node = &graph.nodes[index];
        let mut kind = node.kind.clone();
        let noise = kind
            .noise()
            .map(|id| {
                noises
                    .iter()
                    .position(|(resource, _)| *resource == id)
                    .ok_or_else(|| invalid(node.resource))
            })
            .transpose()?;
        let noise_bound = noise.map(|i| noises[i].1.max_value());
        let mut lowered = None;
        let mut spline = None;
        let mut bound_blended = None;
        let child = |id| bounds(&nodes, id, node.resource);
        let mut diagnostic = match &mut kind {
            NodeKind::Constant(value) => pair(*value, *value),
            NodeKind::Holder { origin, target } => {
                *origin = HolderOrigin::Direct;
                if climate {
                    lowered = Some(BoundKind::Alias(*target));
                }
                child(*target)?
            }
            NodeKind::Coordinate { origin, target } => {
                *origin = HolderOrigin::Direct;
                let range = child(*target)?;
                pair(f64::from(range.min as f32), f64::from(range.max as f32))
            }
            NodeKind::Marker { input, .. } => {
                if climate {
                    lowered = Some(BoundKind::Alias(*input));
                }
                child(*input)?
            }
            NodeKind::Binary {
                operation,
                left,
                right,
            } => {
                let diagnostic = binary_bounds(*operation, child(*left)?, child(*right)?);
                if matches!(operation, Binary::Add | Binary::Mul) {
                    if let Some(argument) = constant(&nodes, *left, node.resource)? {
                        lowered = Some(BoundKind::MulOrAdd {
                            operation: *operation,
                            input: *right,
                            argument,
                        });
                    } else if let Some(argument) = constant(&nodes, *right, node.resource)? {
                        lowered = Some(BoundKind::MulOrAdd {
                            operation: *operation,
                            input: *left,
                            argument,
                        });
                    }
                }
                diagnostic
            }
            NodeKind::Unary { operation, input } => {
                let range = child(*input)?;
                mapped_bounds(*operation, range)
            }
            NodeKind::BlendDensity(_) => pair(f64::NEG_INFINITY, f64::INFINITY),
            NodeKind::Clamp { min, max, .. } => pair(*min, *max),
            NodeKind::RangeChoice {
                inside, outside, ..
            } => {
                let a = child(*inside)?;
                let b = child(*outside)?;
                pair(java_min(a.min, b.min), java_max(a.max, b.max))
            }
            NodeKind::Gradient {
                from_value,
                to_value,
                ..
            } => pair(
                java_min(*from_value, *to_value),
                java_max(*from_value, *to_value),
            ),
            NodeKind::Noise { .. } | NodeKind::ShiftedNoise { .. } => {
                let max = noise_bound.ok_or_else(|| invalid(node.resource))?;
                pair(-max, max)
            }
            NodeKind::Shift { .. } => {
                let max = noise_bound.ok_or_else(|| invalid(node.resource))? * 4.0;
                pair(-max, max)
            }
            NodeKind::Weird { rarity, .. } => {
                let rarity = match rarity {
                    Rarity::Type1 => 2.0,
                    Rarity::Type2 => 3.0,
                };
                pair(
                    0.0,
                    rarity * noise_bound.ok_or_else(|| invalid(node.resource))?,
                )
            }
            NodeKind::Blended { .. } => {
                let sampler = blended.get(&index).ok_or_else(|| invalid(node.resource))?;
                bound_blended = Some(Arc::clone(sampler));
                pair(sampler.min_value(), sampler.max_value())
            }
            NodeKind::Unit(unit) => match unit {
                Unit::EndIslands => pair(-0.84375, 0.5625),
                Unit::BlendAlpha => pair(1.0, 1.0),
                Unit::BlendOffset | Unit::Beardifier => pair(0.0, 0.0),
            },
            NodeKind::Spline(root) => {
                spline = Some(build_spline(graph, *root, &nodes, node.resource)?);
                child(*root)?
            }
            NodeKind::SplineConstant(value) => pair(f64::from(*value), f64::from(*value)),
            NodeKind::SplineMultipoint { coordinate, points } => {
                let range = child(*coordinate)?;
                let mut local_points = Vec::with_capacity(points.len());
                let mut local_bounds = Vec::with_capacity(points.len());
                for (i, point) in points.iter().enumerate() {
                    local_points.push(
                        SplinePoint::new(
                            point.location,
                            SplineNodeId::new(i as u32),
                            point.derivative,
                        )
                        .map_err(|source| error(node.resource, ErrorKind::Spline(source)))?,
                    );
                    let bounds = child(point.value)?;
                    local_bounds.push((bounds.min as f32, bounds.max as f32));
                }
                let (min, max) = CubicSpline::original_multipoint_bounds(
                    range.min as f32,
                    range.max as f32,
                    &local_points,
                    &local_bounds,
                );
                pair(f64::from(min), f64::from(max))
            }
        };
        // MulOrAdd.mapAll has its own reconstruction contract. The climate
        // visitor must consume the actual first-pass factory, not replay Ap2
        // construction from the seed-free JSON descriptor.
        let original_mul_or_add =
            if prepared.origins[index].construction == SourceConstruction::Registered {
                lowered.as_ref()
            } else {
                None
            };
        let prior_mul_or_add = prior.map(|view| &view.nodes[index].kind);
        if let Some(BoundKind::MulOrAdd {
            operation,
            input,
            argument,
        }) = prior_mul_or_add.or(original_mul_or_add)
        {
            let range = child(*input)?;
            diagnostic = mul_or_add_bounds(*operation, range, *argument);
            lowered = Some(BoundKind::MulOrAdd {
                operation: *operation,
                input: *input,
                argument: *argument,
            });
        }
        nodes[index] = Some(BoundNode {
            origin: prepared.origins[index],
            resource: node.resource,
            kind: lowered.unwrap_or(BoundKind::Original(kind)),
            bounds: diagnostic,
            noise,
            blended: bound_blended,
            spline,
        });
    }
    Ok(BoundView {
        pass: if climate {
            VisitorPass::Climate
        } else {
            VisitorPass::SeedRouter
        },
        nodes: nodes
            .into_iter()
            .map(|node| node.ok_or_else(|| invalid("bound_view")))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn bounds(
    nodes: &[Option<BoundNode>],
    index: usize,
    resource: &'static str,
) -> Result<DensityBounds, DensityBindError> {
    nodes
        .get(index)
        .and_then(Option::as_ref)
        .map(|node| node.bounds)
        .ok_or_else(|| invalid(resource))
}

fn constant(
    nodes: &[Option<BoundNode>],
    mut index: usize,
    resource: &'static str,
) -> Result<Option<f64>, DensityBindError> {
    // Aliases only exist after an actual visitor removed the corresponding node.
    for _ in 0..=super::MAX_DENSITY_DEPTH {
        let node = nodes
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid(resource))?;
        match node.kind {
            BoundKind::Alias(target) => index = target,
            BoundKind::Original(NodeKind::Constant(value)) => return Ok(Some(value)),
            _ => return Ok(None),
        }
    }
    Err(invalid(resource))
}

fn build_spline(
    graph: &Graph,
    root: usize,
    nodes: &[Option<BoundNode>],
    resource: &'static str,
) -> Result<BoundSpline, DensityBindError> {
    let mut indices = BTreeMap::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if indices.contains_key(&id) {
            continue;
        }
        let index = indices.len();
        if index >= crate::spline::MAX_SPLINE_NODES {
            return Err(error(
                resource,
                ErrorKind::Limit {
                    count: index + 1,
                    maximum: crate::spline::MAX_SPLINE_NODES,
                },
            ));
        }
        indices.insert(id, index);
        match &graph.nodes[id].kind {
            NodeKind::SplineConstant(_) => {}
            NodeKind::SplineMultipoint { points, .. } => {
                pending.extend(points.iter().map(|point| point.value));
            }
            _ => return Err(invalid(resource)),
        }
    }
    let mut ordered = vec![0; indices.len()];
    for (&global, &local) in &indices {
        ordered[local] = global;
    }
    let mut coordinate_ids = BTreeMap::new();
    let mut coordinates = Vec::new();
    let mut ranges = Vec::new();
    let mut point_lists = Vec::with_capacity(indices.len());
    for &global in &ordered {
        let points = match &graph.nodes[global].kind {
            NodeKind::SplineConstant(_) => Vec::new(),
            NodeKind::SplineMultipoint { coordinate, points } => {
                if !coordinate_ids.contains_key(coordinate) {
                    let id = coordinates.len();
                    if id >= crate::spline::MAX_SPLINE_COORDINATES {
                        return Err(error(
                            resource,
                            ErrorKind::Limit {
                                count: id + 1,
                                maximum: crate::spline::MAX_SPLINE_COORDINATES,
                            },
                        ));
                    }
                    let NodeKind::Coordinate { target, .. } = graph.nodes[*coordinate].kind else {
                        return Err(invalid(resource));
                    };
                    coordinate_ids.insert(*coordinate, id);
                    coordinates.push(target);
                    let bound = bounds(nodes, *coordinate, resource)?;
                    ranges.push(CubicSpline::original_coordinate_range(
                        bound.min as f32,
                        bound.max as f32,
                    ));
                }
                points
                    .iter()
                    .map(|point| {
                        SplinePoint::new(
                            point.location,
                            SplineNodeId::new(indices[&point.value] as u32),
                            point.derivative,
                        )
                        .map_err(|source| error(resource, ErrorKind::Spline(source)))
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => return Err(invalid(resource)),
        };
        point_lists.push(points);
    }
    let mut specs = Vec::with_capacity(indices.len());
    for (index, &global) in ordered.iter().enumerate() {
        specs.push(
            match graph.nodes[global].kind {
                NodeKind::SplineConstant(value) => SplineNodeSpec::constant(value),
                NodeKind::SplineMultipoint { coordinate, .. } => SplineNodeSpec::multipoint(
                    SplineCoordinateId::new(coordinate_ids[&coordinate] as u32),
                    &point_lists[index],
                ),
                _ => return Err(invalid(resource)),
            }
            .map_err(|source| error(resource, ErrorKind::Spline(source)))?,
        );
    }
    let spline = CubicSpline::new(&ranges, &specs, SplineNodeId::new(indices[&root] as u32))
        .map_err(|source| error(resource, ErrorKind::Spline(source)))?;
    Ok(BoundSpline {
        spline,
        coordinates: coordinates.into_boxed_slice(),
    })
}

pub(super) const fn pair(min: f64, max: f64) -> DensityBounds {
    DensityBounds { min, max }
}

pub(super) fn binary_bounds(
    operation: Binary,
    a: DensityBounds,
    b: DensityBounds,
) -> DensityBounds {
    let min = match operation {
        Binary::Add => a.min + b.min,
        Binary::Mul => {
            if a.min > 0.0 && b.min > 0.0 {
                a.min * b.min
            } else if a.max < 0.0 && b.max < 0.0 {
                a.max * b.max
            } else {
                java_min(a.min * b.max, a.max * b.min)
            }
        }
        Binary::Min => java_min(a.min, b.min),
        Binary::Max => java_max(a.min, b.min),
    };
    let max = match operation {
        Binary::Add => a.max + b.max,
        Binary::Mul => {
            if a.min > 0.0 && b.min > 0.0 {
                a.max * b.max
            } else if a.max < 0.0 && b.max < 0.0 {
                a.min * b.min
            } else {
                java_max(a.min * b.min, a.max * b.max)
            }
        }
        Binary::Min => java_min(a.max, b.max),
        Binary::Max => java_max(a.max, b.max),
    };
    pair(min, max)
}

pub(super) fn mapped_bounds(operation: super::graph::Unary, range: DensityBounds) -> DensityBounds {
    let min = super::evaluate::unary(operation, range.min);
    let max = super::evaluate::unary(operation, range.max);
    if matches!(
        operation,
        super::graph::Unary::Abs | super::graph::Unary::Square
    ) {
        pair(java_max(0.0, range.min), java_max(min, max))
    } else {
        pair(min, max)
    }
}
pub(super) fn mul_or_add_bounds(
    operation: Binary,
    range: DensityBounds,
    argument: f64,
) -> DensityBounds {
    if matches!(operation, Binary::Add) {
        pair(range.min + argument, range.max + argument)
    } else if argument >= 0.0 {
        pair(range.min * argument, range.max * argument)
    } else {
        pair(range.max * argument, range.min * argument)
    }
}
pub(super) fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if a == 0.0 && b == 0.0 && b.to_bits() == (-0.0_f64).to_bits() {
        b
    } else if a <= b {
        a
    } else {
        b
    }
}
pub(super) fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if a == 0.0 && b == 0.0 && a.to_bits() == (-0.0_f64).to_bits() {
        b
    } else if a >= b {
        a
    } else {
        b
    }
}
fn error(resource: &'static str, kind: ErrorKind) -> DensityBindError {
    DensityBindError { resource, kind }
}
fn invalid(resource: &'static str) -> DensityBindError {
    error(resource, ErrorKind::InvalidGraph)
}
