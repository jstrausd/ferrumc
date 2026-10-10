//! Bounded original seed/mapAll/wrap traversal and Java-record equality.

use std::mem::size_of;

use ferrumc_math::BlockPos;

use super::state::{Cache, ChunkState};
use super::{bytes, charge, fail, limit, NoiseChunkError, NoiseChunkErrorKind, NoiseChunkLimit};
use crate::density::bind::{self, BoundKind, BoundNode, BoundRouter, BoundView, VisitorPass};
use crate::density::graph::{Binary, HolderOrigin, Marker, NodeKind, Unit};
use crate::sample_work::SampleWork;

mod keys;
mod spline;
use keys::{Identity, Keys};

type Binding = (BoundView, ChunkState, [usize; 15], usize, [usize; 6]);
type Roots = ([usize; 15], usize, [usize; 6], [usize; 15]);

pub(super) fn bind(
    data: &BoundRouter,
    state: ChunkState,
    work: &mut SampleWork,
) -> Result<Binding, NoiseChunkError> {
    let mut cold = Cold::new(data, state, work)?;
    let (fields, material, climate, _) = cold.roots()?;
    cold.validate(&fields, material, &climate)?;
    let max_len = cold.state.cell_volume().max(cold.state.count_y + 1);
    let scratch_depth = cold.scratch_depth()?;
    let scratch_len = bytes(
        NoiseChunkLimit::ScratchBytes,
        0,
        scratch_depth,
        max_len,
        4 * 1024 * 1024 / 8,
    )?;
    cold.state.scratch_bytes = bytes(
        NoiseChunkLimit::ScratchBytes,
        0,
        scratch_len,
        8,
        4 * 1024 * 1024,
    )?;
    cold.graph_bytes = bytes(
        NoiseChunkLimit::GraphBytes,
        cold.graph_bytes,
        scratch_depth,
        size_of::<Vec<f64>>(),
        2 * 1024 * 1024,
    )?;
    cold.state.scratch = (0..scratch_depth).map(|_| vec![0.0; max_len]).collect();
    cold.state.graph_bytes = cold.graph_bytes;
    #[cfg(test)]
    {
        cold.state.wrap_entries = cold.wrap_entries;
    }
    Ok((cold.view, cold.state, fields, material, climate))
}

struct Cold<'a> {
    data: &'a BoundRouter,
    state: ChunkState,
    work: &'a mut SampleWork,
    view: BoundView,
    keys: Keys,
    node_keys: Vec<usize>,
    noise_tokens: Vec<u64>,
    spline_arrays: Vec<Option<(u64, u64)>>,
    wrap_map: Box<[Option<usize>]>,
    wrap_entries: usize,
    visits: usize,
    edges: usize,
    graph_bytes: usize,
    #[cfg(test)]
    trace: Option<Vec<(usize, &'static str, usize, &'static str)>>,
}
impl<'a> Cold<'a> {
    #[cfg(test)]
    fn record_wrap(&mut self, key: usize, incoming: usize, value: usize) {
        if self.trace.is_none() {
            return;
        }
        let kind = tests::oracle::class(self, &self.view.nodes[incoming].kind, incoming);
        let value_kind = tests::oracle::class(self, &self.view.nodes[value].kind, value);
        if let Some(trace) = &mut self.trace {
            assert!(trace.len() < 131_072);
            trace.push((key, kind, value, value_kind));
        }
    }
    fn new(
        data: &'a BoundRouter,
        state: ChunkState,
        work: &'a mut SampleWork,
    ) -> Result<Self, NoiseChunkError> {
        let graph_bytes = bytes(
            NoiseChunkLimit::GraphBytes,
            state.graph_bytes,
            4096,
            size_of::<BoundNode>(),
            2 * 1024 * 1024,
        )?;
        // Keys/maps have their own cap. Account their maximum simultaneous size
        // again in the independent cold peak, including bounded recursive clones.
        let temporary = 4096
            * (size_of::<Option<usize>>()
                + size_of::<usize>()
                + size_of::<u64>()
                + size_of::<Option<(u64, u64)>>())
            + 96 * (size_of::<BoundNode>()
                + 65 * size_of::<usize>()
                + 64 * size_of::<crate::density::graph::Point>()
                + 70 * size_of::<u64>())
            + 2 * 4096 * size_of::<usize>()
            + spline::temporary_bytes();
        bytes(
            NoiseChunkLimit::ColdBytes,
            temporary,
            1,
            2 * 1024 * 1024,
            4 * 1024 * 1024,
        )?;
        Ok(Self {
            data,
            state,
            work,
            view: BoundView {
                pass: VisitorPass::SeedRouter,
                nodes: Vec::with_capacity(4096),
            },
            keys: Keys::new()?,
            node_keys: Vec::with_capacity(4096),
            noise_tokens: Vec::with_capacity(4096),
            spline_arrays: Vec::with_capacity(4096),
            wrap_map: vec![None; 4096].into_boxed_slice(),
            wrap_entries: 0,
            visits: 0,
            edges: 0,
            graph_bytes,
            #[cfg(test)]
            trace: None,
        })
    }
    fn roots(&mut self) -> Result<Roots, NoiseChunkError> {
        let mut seed_fields = [0; 15];
        for (slot, &root) in seed_fields.iter_mut().zip(&self.data.fields) {
            *slot = self.seed(root, false, 1)?;
        }
        let mut fields = [0; 15];
        for (slot, &root) in fields.iter_mut().zip(&seed_fields) {
            *slot = self.wrap(root, 1)?;
        }
        let mut beard = self.view.nodes[fields[11]].clone();
        beard.kind = BoundKind::Original(NodeKind::Unit(Unit::Beardifier));
        beard.noise = None;
        beard.blended = None;
        beard.spline = None;
        beard.bounds = bind::pair(0.0, 0.0);
        let zero = self.intern(beard, 0, None, None)?;
        let mut add = self.view.nodes[fields[11]].clone();
        add.kind = BoundKind::Original(NodeKind::Binary {
            operation: Binary::Add,
            left: fields[11],
            right: zero,
        });
        add.spline = None;
        add.noise = None;
        add.blended = None;
        self.refresh(&mut add)?;
        let input = self.intern(add, 0, None, None)?;
        let mut material = self.view.nodes[input].clone();
        material.kind = BoundKind::Original(NodeKind::Marker {
            marker: Marker::CacheAllInCell,
            input,
        });
        let material = self.intern(material, 0, None, None)?;
        let material = self.wrap(material, 1)?;
        let mut climate = [0; 6];
        for (i, slot) in climate.iter_mut().enumerate() {
            *slot = self.wrap(seed_fields[4 + i], 1)?;
        }
        Ok((fields, material, climate, seed_fields))
    }
    fn visit(&mut self, depth: usize) -> Result<(), NoiseChunkError> {
        limit(NoiseChunkLimit::Depth, depth, 96)?;
        limit(NoiseChunkLimit::ColdVisits, self.visits + 1, 131_072)?;
        charge(self.work)?;
        self.visits += 1;
        Ok(())
    }
    fn seed(&mut self, index: usize, native: bool, depth: usize) -> Result<usize, NoiseChunkError> {
        self.visit(depth)?;
        let source = self
            .data
            .source
            .graph
            .nodes
            .get(index)
            .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
        let mut node = self
            .data
            .router
            .nodes
            .get(index)
            .cloned()
            .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
        // A native original MulOrAdd already skips visitor.apply. Decoded Ap2
        // retains both actual codec HolderHolder children before its factory.
        node.kind = if native && matches!(node.kind, BoundKind::MulOrAdd { .. }) {
            node.kind
        } else {
            BoundKind::Original(source.kind.clone())
        };
        match &mut node.kind {
            BoundKind::MulOrAdd { input, .. } => *input = self.seed(*input, native, depth + 1)?,
            BoundKind::Original(
                NodeKind::Holder { origin, target } | NodeKind::Coordinate { origin, target },
            ) => {
                let next_native = native || matches!(origin, HolderOrigin::Reference(_));
                *target = self.seed(*target, next_native, depth + 1)?;
                *origin = HolderOrigin::Direct;
            }
            BoundKind::Original(kind) => {
                let mut children = Vec::with_capacity(kind.children());
                for i in 0..kind.children() {
                    let child = kind
                        .child(i)
                        .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
                    children.push(self.seed(child, native, depth + 1)?);
                }
                remap(kind, &children)?;
            }
            BoundKind::Alias(_) => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
        }
        let noise = if let Some(noise) = node.noise {
            let name = self
                .data
                .noises
                .get(noise)
                .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?
                .0
                .resource_location();
            let legacy = matches!(
                self.data.metadata.algorithm,
                crate::random::RandomAlgorithm::Legacy
            ) && matches!(
                name,
                "minecraft:temperature" | "minecraft:vegetation" | "minecraft:offset"
            );
            self.keys.identity(
                if legacy {
                    Identity::FreshNoise
                } else {
                    Identity::ModernNoise(noise)
                },
                self.work,
            )?
        } else {
            0
        };
        let arrays = if matches!(
            node.kind,
            BoundKind::Original(NodeKind::SplineMultipoint { .. })
        ) {
            Some((
                self.keys
                    .identity(Identity::SplineLocations(index, native), self.work)?,
                self.keys
                    .identity(Identity::SplineDerivatives(index, native), self.work)?,
            ))
        } else {
            None
        };
        let identity = match node.kind {
            BoundKind::Original(NodeKind::Unit(Unit::EndIslands)) => {
                // Codec.unit holds one decoded class instance. Registered
                // factory instances retain their original source identity.
                Some(self.keys.identity(
                    Identity::SeedEnd(if native { index + 1 } else { 0 }),
                    self.work,
                )?)
            }
            BoundKind::Original(NodeKind::Blended { .. }) => Some(
                self.keys
                    .identity(Identity::SeedBlended(index, native), self.work)?,
            ),
            _ => None,
        };
        self.refresh(&mut node)?;
        self.factory(&mut node);
        self.intern(node, noise, arrays, identity)
    }
    fn wrap(&mut self, index: usize, depth: usize) -> Result<usize, NoiseChunkError> {
        self.visit(depth)?;
        let mut node = self
            .view
            .nodes
            .get(index)
            .cloned()
            .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
        let noise = self.noise_tokens[index];
        let arrays = self.spline_arrays[index];
        let identity = self.keys.node_identity(self.node_keys[index]);
        let applies = !matches!(
            node.kind,
            BoundKind::MulOrAdd { .. }
                | BoundKind::Original(
                    NodeKind::Unary { .. }
                        | NodeKind::Clamp { .. }
                        | NodeKind::Coordinate { .. }
                        | NodeKind::SplineConstant(_)
                        | NodeKind::SplineMultipoint { .. }
                )
        );
        match &mut node.kind {
            BoundKind::MulOrAdd { input, .. } => *input = self.wrap(*input, depth + 1)?,
            BoundKind::Original(kind) => {
                let mut children = Vec::with_capacity(kind.children());
                for i in 0..kind.children() {
                    let child = kind
                        .child(i)
                        .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
                    children.push(self.wrap(child, depth + 1)?);
                }
                remap(kind, &children)?;
            }
            BoundKind::Alias(_) => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
        }
        self.refresh(&mut node)?;
        self.factory(&mut node);
        // Mutable wrapper.mapAll creates the original Marker record. Its
        // identity does not participate in that record's actual equality.
        let wrapper = matches!(node.kind, BoundKind::Original(NodeKind::Marker { .. }));
        let incoming = self.intern(node, noise, arrays, if wrapper { None } else { identity })?;
        if !applies {
            return Ok(incoming);
        }
        let key = self.node_keys[incoming];
        charge(self.work)?;
        if let Some(result) = self.wrap_map[key] {
            #[cfg(test)]
            self.record_wrap(key, incoming, result);
            return Ok(result);
        }
        limit(NoiseChunkLimit::VisitorEntries, self.wrap_entries + 1, 4096)?;
        let result = match self.view.nodes[incoming].kind {
            BoundKind::Original(NodeKind::Holder { target, .. }) => target,
            BoundKind::Original(NodeKind::Marker { marker, input }) => {
                let id = self.state.add_wrapper(marker, input)?;
                if matches!(marker, Marker::FlatCache) {
                    for x in 0..5 {
                        for z in 0..5 {
                            let pos = BlockPos::new(
                                self.state.origin.x() + x * 4,
                                0,
                                self.state.origin.z() + z * 4,
                            );
                            let value = self
                                .state
                                .sample(self.data, &self.view, input, pos, false, self.work)?;
                            charge(self.work)?;
                            if let Cache::Flat(values) = &mut self.state.wrappers[id].cache {
                                values[(x * 5 + z) as usize] = value;
                            }
                        }
                    }
                }
                let token = self.keys.identity(Identity::Wrapper(id), self.work)?;
                let node = self.view.nodes[incoming].clone();
                let output = self.intern(node, noise, arrays, Some(token))?;
                self.state.wrapper_nodes[output] = Some(id);
                self.state.wrappers[id].node = output;
                output
            }
            _ => incoming,
        };
        self.wrap_map[key] = Some(result);
        self.wrap_entries += 1;
        #[cfg(test)]
        self.record_wrap(key, incoming, result);
        Ok(result)
    }
    fn factory(&self, node: &mut BoundNode) {
        if let BoundKind::Original(NodeKind::Binary {
            operation: operation @ (Binary::Add | Binary::Mul),
            left,
            right,
        }) = node.kind
        {
            let constant = |index: usize| match self.view.nodes[index].kind {
                BoundKind::Original(NodeKind::Constant(v)) => Some(v),
                _ => None,
            };
            if let Some(argument) = constant(left) {
                node.kind = BoundKind::MulOrAdd {
                    operation,
                    input: right,
                    argument,
                };
            } else if let Some(argument) = constant(right) {
                node.kind = BoundKind::MulOrAdd {
                    operation,
                    input: left,
                    argument,
                };
            }
        }
    }
    fn intern(
        &mut self,
        mut node: BoundNode,
        noise: u64,
        arrays: Option<(u64, u64)>,
        identity: Option<u64>,
    ) -> Result<usize, NoiseChunkError> {
        let words = keys::words(&node, &self.node_keys, noise, arrays, identity)?;
        let key = self.keys.intern(words, identity, self.work)?;
        if let Some(index) = self.keys.node(key) {
            return Ok(index);
        }
        let index = self.view.nodes.len();
        limit(NoiseChunkLimit::Vertices, index + 1, 4096)?;
        let edges = match &node.kind {
            BoundKind::MulOrAdd { .. } | BoundKind::Alias(_) => 1,
            BoundKind::Original(k) => k.children(),
        };
        let total_edges = bytes(NoiseChunkLimit::Edges, self.edges, edges, 1, 8192)?;
        let extra = match &node.kind {
            BoundKind::Original(NodeKind::SplineMultipoint { points, .. }) => {
                points.len() * size_of::<crate::density::graph::Point>()
            }
            _ => 0,
        };
        let mut graph_bytes = bytes(
            NoiseChunkLimit::GraphBytes,
            self.graph_bytes,
            1,
            extra,
            2 * 1024 * 1024,
        )?;
        if let BoundKind::Original(NodeKind::Blended { .. }) = node.kind {
            let original = node
                .blended
                .as_ref()
                .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
            let retained = original.retained_bytes() + 2 * size_of::<usize>();
            graph_bytes = bytes(
                NoiseChunkLimit::GraphBytes,
                graph_bytes,
                1,
                retained,
                2 * 1024 * 1024,
            )?;
            node.blended = Some(std::sync::Arc::new(
                original.paper_build60_basis(self.work)?,
            ));
        }
        if let BoundKind::Original(NodeKind::Spline(root)) = node.kind {
            let plan = spline::plan(&self.view.nodes, root, self.work)?;
            graph_bytes = bytes(
                NoiseChunkLimit::GraphBytes,
                graph_bytes,
                1,
                plan.retained_bytes(),
                2 * 1024 * 1024,
            )?;
            let spline = spline::build(&self.view.nodes, root, &plan, self.work)?;
            node.spline = Some(spline);
        } else {
            node.spline = None;
        }
        self.edges = total_edges;
        self.graph_bytes = graph_bytes;
        self.keys.set_node(key, index);
        self.node_keys.push(key);
        self.noise_tokens.push(noise);
        self.spline_arrays.push(arrays);
        self.state.wrapper_nodes.push(None);
        self.view.nodes.push(node);
        Ok(index)
    }
    fn refresh(&mut self, node: &mut BoundNode) -> Result<(), NoiseChunkError> {
        let child = |id: usize| {
            self.view
                .nodes
                .get(id)
                .map(|n| n.bounds)
                .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))
        };
        node.bounds = match &node.kind {
            BoundKind::MulOrAdd {
                operation,
                input,
                argument,
            } => bind::mul_or_add_bounds(*operation, child(*input)?, *argument),
            BoundKind::Alias(target) => child(*target)?,
            BoundKind::Original(kind) => match kind {
                NodeKind::Holder { target, .. }
                | NodeKind::Marker { input: target, .. }
                | NodeKind::Spline(target) => child(*target)?,
                NodeKind::Coordinate { target, .. } => {
                    let b = child(*target)?;
                    bind::pair(f64::from(b.min as f32), f64::from(b.max as f32))
                }
                NodeKind::Binary {
                    operation,
                    left,
                    right,
                } => bind::binary_bounds(*operation, child(*left)?, child(*right)?),
                NodeKind::Unary { operation, input } => {
                    bind::mapped_bounds(*operation, child(*input)?)
                }
                NodeKind::RangeChoice {
                    inside, outside, ..
                } => {
                    let a = child(*inside)?;
                    let b = child(*outside)?;
                    bind::pair(bind::java_min(a.min, b.min), bind::java_max(a.max, b.max))
                }
                NodeKind::SplineMultipoint { coordinate, points } => {
                    spline::bounds(child(*coordinate)?, points, &self.view.nodes, self.work)?
                }
                _ => node.bounds,
            },
        };
        Ok(())
    }
    fn validate(
        &mut self,
        fields: &[usize; 15],
        material: usize,
        climate: &[usize; 6],
    ) -> Result<(), NoiseChunkError> {
        let mut depths = Vec::with_capacity(self.view.nodes.len());
        for (i, node) in self.view.nodes.iter().enumerate() {
            charge(self.work)?;
            let noise_required = matches!(
                node.kind,
                BoundKind::Original(
                    NodeKind::Noise { .. }
                        | NodeKind::Shift { .. }
                        | NodeKind::ShiftedNoise { .. }
                        | NodeKind::Weird { .. }
                )
            );
            if noise_required != node.noise.is_some()
                || node
                    .noise
                    .is_some_and(|n| n >= self.state.normal_noises.len())
                || matches!(node.kind, BoundKind::Original(NodeKind::Blended { .. }))
                    != node.blended.is_some()
                || matches!(node.kind, BoundKind::Original(NodeKind::Spline(_)))
                    != node.spline.is_some()
            {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
            let mut depth = 1;
            let children = children(&node.kind);
            for (edge, child) in children.into_iter().enumerate() {
                charge(self.work)?;
                if child >= i {
                    return Err(fail(NoiseChunkErrorKind::InvalidGraph));
                }
                if !valid_child(&node.kind, edge, &self.view.nodes[child].kind) {
                    return Err(fail(NoiseChunkErrorKind::InvalidGraph));
                }
                depth = depth.max(depths[child] + 1);
            }
            limit(NoiseChunkLimit::Depth, depth, 96)?;
            depths.push(depth);
        }
        for &root in fields
            .iter()
            .chain(std::iter::once(&material))
            .chain(climate)
        {
            charge(self.work)?;
            if !self
                .view
                .nodes
                .get(root)
                .is_some_and(|n| is_density(&n.kind))
            {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        for (id, wrapper) in self.state.wrappers.iter().enumerate().skip(2) {
            charge(self.work)?;
            if !matches!(self.view.nodes.get(wrapper.node).map(|n|&n.kind),Some(BoundKind::Original(NodeKind::Marker {input,..})) if *input==wrapper.input)
                || self
                    .state
                    .wrapper_nodes
                    .get(wrapper.node)
                    .copied()
                    .flatten()
                    != Some(id)
            {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        Ok(())
    }
    fn scratch_depth(&mut self) -> Result<usize, NoiseChunkError> {
        let mut depths = vec![0_usize; self.view.nodes.len()];
        for (i, node) in self.view.nodes.iter().enumerate() {
            charge(self.work)?;
            let depth = if let BoundKind::Original(NodeKind::Binary {
                operation: Binary::Add,
                left,
                right,
            }) = node.kind
            {
                depths[left].max(depths[right] + 1)
            } else {
                children(&node.kind)
                    .into_iter()
                    .map(|c| depths[c])
                    .max()
                    .unwrap_or(0)
            };
            depths[i] = depth;
        }
        Ok(depths.into_iter().max().unwrap_or(0))
    }
}

fn is_density(kind: &BoundKind) -> bool {
    match kind {
        BoundKind::Original(kind) => kind.is_density(),
        BoundKind::MulOrAdd { operation, .. } => matches!(operation, Binary::Add | Binary::Mul),
        BoundKind::Alias(_) => false,
    }
}
fn valid_child(parent: &BoundKind, edge: usize, child: &BoundKind) -> bool {
    match (parent, child) {
        (BoundKind::Original(parent), BoundKind::Original(child)) => {
            parent.valid_child(edge, child)
        }
        (
            BoundKind::Alias(_)
            | BoundKind::Original(NodeKind::Spline(_) | NodeKind::SplineMultipoint { .. }),
            _,
        )
        | (_, BoundKind::Alias(_)) => false,
        _ => is_density(child),
    }
}

fn children(kind: &BoundKind) -> Vec<usize> {
    match kind {
        BoundKind::Alias(target) | BoundKind::MulOrAdd { input: target, .. } => vec![*target],
        BoundKind::Original(kind) => (0..kind.children()).filter_map(|i| kind.child(i)).collect(),
    }
}
fn remap(kind: &mut NodeKind, children: &[usize]) -> Result<(), NoiseChunkError> {
    if children.len() != kind.children() {
        return Err(fail(NoiseChunkErrorKind::InvalidGraph));
    }
    match kind {
        NodeKind::Holder { target, .. } | NodeKind::Coordinate { target, .. } => {
            *target = children[0];
        }
        NodeKind::Unary { input, .. }
        | NodeKind::Marker { input, .. }
        | NodeKind::Clamp { input, .. }
        | NodeKind::Weird { input, .. }
        | NodeKind::BlendDensity(input)
        | NodeKind::Spline(input) => *input = children[0],
        NodeKind::Binary { left, right, .. } => {
            *left = children[0];
            *right = children[1];
        }
        NodeKind::RangeChoice {
            input,
            inside,
            outside,
            ..
        } => {
            *input = children[0];
            *inside = children[1];
            *outside = children[2];
        }
        NodeKind::ShiftedNoise {
            shift_x,
            shift_y,
            shift_z,
            ..
        } => {
            *shift_x = children[0];
            *shift_y = children[1];
            *shift_z = children[2];
        }
        NodeKind::SplineMultipoint { coordinate, points } => {
            *coordinate = children[0];
            for (point, &child) in points.iter_mut().zip(&children[1..]) {
                point.value = child;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests;
