//! Bounded fallible stable original tree construction.

use std::cmp::Ordering;
use std::mem::size_of;

use super::{
    charge, fail, limit, BiomeBuildBudget, BiomeError, BiomeErrorKind, BiomeLimit,
    BiomeParametersResourceId, BiomeTree, Node, NodeKind, SampleWork,
    MAX_BIOME_BUILD_SCRATCH_BYTES, MAX_BIOME_PARAMETERS, MAX_BIOME_TREE_BYTES,
    MAX_BIOME_TREE_DEPTH, MAX_BIOME_TREE_EDGES, MAX_BIOME_TREE_NODES,
};

struct Builder {
    nodes: Vec<Node>,
    leaf_nodes: Vec<u16>,
    edges: usize,
    depth: usize,
    scratch_bytes: usize,
    scratch_peak: usize,
    work: SampleWork,
}
struct Group {
    axes: [[i64; 2]; 7],
    children: Vec<u16>,
}

pub(super) fn builtin(
    preset: BiomeParametersResourceId,
    budget: BiomeBuildBudget,
) -> Result<BiomeTree, BiomeError> {
    let entries = preset.parameters();
    if entries.is_empty() {
        return Err(fail(BiomeErrorKind::InvalidTree));
    }
    limit(BiomeLimit::Parameters, entries.len(), MAX_BIOME_PARAMETERS)?;
    let mut work = SampleWork::new(budget.0 as usize);
    let planned_nodes = plan_nodes(entries.len(), 1, &mut work)?;
    limit(BiomeLimit::Nodes, planned_nodes, MAX_BIOME_TREE_NODES)?;
    limit(BiomeLimit::Edges, planned_nodes - 1, MAX_BIOME_TREE_EDGES)?;
    let bytes = planned_nodes
        .checked_mul(size_of::<Node>())
        .and_then(|bytes| bytes.checked_add(entries.len() * size_of::<u16>()))
        .and_then(|bytes| bytes.checked_add(size_of::<BiomeTree>()))
        .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
    limit(BiomeLimit::TreeBytes, bytes, MAX_BIOME_TREE_BYTES)?;
    if size_of::<Node>() > 128 {
        return Err(fail(BiomeErrorKind::InvalidTree));
    }
    let mut builder = Builder {
        nodes: reserved(planned_nodes, BiomeLimit::Nodes)?,
        leaf_nodes: reserved(entries.len(), BiomeLimit::Parameters)?,
        edges: 0,
        depth: 0,
        scratch_bytes: 0,
        scratch_peak: 0,
        work,
    };
    let mut children = builder.scratch(entries.len())?;
    for (ordinal, entry) in entries.iter().enumerate() {
        let ordinal = u16::try_from(ordinal).map_err(|_| fail(BiomeErrorKind::InvalidTree))?;
        let mut axes = [[0; 2]; 7];
        for (axis, span) in axes[..6].iter_mut().zip(entry.point().axes()) {
            charge(&mut builder.work)?;
            *axis = [span.minimum(), span.maximum()];
        }
        charge(&mut builder.work)?;
        axes[6] = [entry.point().offset(); 2];
        let node = builder.append(Node {
            axes,
            kind: NodeKind::Leaf(ordinal),
        })?;
        charge(&mut builder.work)?;
        children.push(node);
        builder.leaf_nodes.push(node);
    }
    let root = builder.build(&mut children, 1)?;
    builder.release(children);
    charge(&mut builder.work)?;
    if builder.nodes.len() != planned_nodes {
        return Err(fail(BiomeErrorKind::InvalidTree));
    }
    let retained_bytes = builder.nodes.capacity() * size_of::<Node>()
        + builder.leaf_nodes.capacity() * size_of::<u16>()
        + size_of::<BiomeTree>();
    limit(BiomeLimit::TreeBytes, retained_bytes, MAX_BIOME_TREE_BYTES)?;
    Ok(BiomeTree {
        preset,
        nodes: builder.nodes,
        root,
        leaf_nodes: builder.leaf_nodes,
        edges: builder.edges,
        depth: builder.depth,
        retained_bytes,
        build_work: builder.work.used(),
        build_scratch_peak: builder.scratch_peak,
    })
}

fn reserved<T>(count: usize, resource: BiomeLimit) -> Result<Vec<T>, BiomeError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| fail(BiomeErrorKind::Allocation { limit: resource }))?;
    Ok(values)
}
impl Builder {
    fn scratch<T>(&mut self, count: usize) -> Result<Vec<T>, BiomeError> {
        let bytes = count
            .checked_mul(size_of::<T>())
            .and_then(|n| self.scratch_bytes.checked_add(n))
            .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
        limit(
            BiomeLimit::ScratchBytes,
            bytes,
            MAX_BIOME_BUILD_SCRATCH_BYTES,
        )?;
        charge(&mut self.work)?;
        let values = reserved(count, BiomeLimit::ScratchBytes)?;
        self.scratch_bytes += values.capacity() * size_of::<T>();
        self.scratch_peak = self.scratch_peak.max(self.scratch_bytes);
        limit(
            BiomeLimit::ScratchBytes,
            self.scratch_bytes,
            MAX_BIOME_BUILD_SCRATCH_BYTES,
        )?;
        Ok(values)
    }
    fn release<T>(&mut self, values: Vec<T>) {
        self.scratch_bytes -= values.capacity() * size_of::<T>();
        drop(values);
    }
    fn append(&mut self, node: Node) -> Result<u16, BiomeError> {
        limit(
            BiomeLimit::Nodes,
            self.nodes.len() + 1,
            MAX_BIOME_TREE_NODES,
        )?;
        if self.nodes.len() == self.nodes.capacity() {
            return Err(fail(BiomeErrorKind::InvalidTree));
        }
        charge(&mut self.work)?;
        let id = u16::try_from(self.nodes.len()).map_err(|_| fail(BiomeErrorKind::InvalidTree))?;
        self.nodes.push(node);
        Ok(id)
    }
    fn branch(&mut self, children: &[u16]) -> Result<u16, BiomeError> {
        if children.is_empty() || children.len() > 6 {
            return Err(fail(BiomeErrorKind::InvalidTree));
        }
        limit(
            BiomeLimit::Edges,
            self.edges + children.len(),
            MAX_BIOME_TREE_EDGES,
        )?;
        let axes = self.union(children)?;
        let mut nodes = [0; 6];
        for (destination, &child) in nodes.iter_mut().zip(children) {
            charge(&mut self.work)?;
            *destination = child;
        }
        self.edges += children.len();
        let len = u8::try_from(children.len()).map_err(|_| fail(BiomeErrorKind::InvalidTree))?;
        self.append(Node {
            axes,
            kind: NodeKind::Branch {
                children: nodes,
                len,
            },
        })
    }
    fn union(&mut self, children: &[u16]) -> Result<[[i64; 2]; 7], BiomeError> {
        let Some(&first) = children.first() else {
            return Err(fail(BiomeErrorKind::InvalidTree));
        };
        let mut axes = self.node(first)?.axes;
        for &child in children {
            let child_axes = self.node(child)?.axes;
            for (axis, child) in axes.iter_mut().zip(child_axes) {
                charge(&mut self.work)?;
                axis[0] = axis[0].min(child[0]);
                axis[1] = axis[1].max(child[1]);
            }
        }
        Ok(axes)
    }
    fn node(&self, id: u16) -> Result<&Node, BiomeError> {
        self.nodes
            .get(usize::from(id))
            .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))
    }
    fn build(&mut self, children: &mut [u16], depth: usize) -> Result<u16, BiomeError> {
        limit(BiomeLimit::Depth, depth, MAX_BIOME_TREE_DEPTH)?;
        self.depth = self.depth.max(depth);
        charge(&mut self.work)?;
        match children.len() {
            0 => Err(fail(BiomeErrorKind::InvalidTree)),
            1 => Ok(children[0]),
            2..=6 => {
                limit(BiomeLimit::Depth, depth + 1, MAX_BIOME_TREE_DEPTH)?;
                self.depth = self.depth.max(depth + 1);
                self.sort_nodes(children, None)?;
                self.branch(children)
            }
            count => {
                let width = bucket_width(count)?;
                let mut best = self.scratch::<Group>(6)?;
                let mut best_cost = i64::MAX;
                let mut best_axis = None;
                for axis in 0..7 {
                    self.sort_nodes(children, Some((axis, false)))?;
                    let mut groups = self.scratch::<Group>(6)?;
                    let mut cost = 0_i64;
                    for bucket in children.chunks(width) {
                        let axes = self.union(bucket)?;
                        for [minimum, maximum] in axes {
                            charge(&mut self.work)?;
                            cost = cost.wrapping_add(maximum.wrapping_sub(minimum).wrapping_abs());
                        }
                        let mut copied = self.scratch(bucket.len())?;
                        for &child in bucket {
                            charge(&mut self.work)?;
                            copied.push(child);
                        }
                        charge(&mut self.work)?;
                        groups.push(Group {
                            axes,
                            children: copied,
                        });
                    }
                    charge(&mut self.work)?;
                    if best_cost > cost {
                        for group in best.drain(..) {
                            self.release(group.children);
                        }
                        self.release(best);
                        best = groups;
                        best_cost = cost;
                        best_axis = Some(axis);
                    } else {
                        for group in groups.drain(..) {
                            self.release(group.children);
                        }
                        self.release(groups);
                    }
                }
                let axis = best_axis.ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
                // At most six groups: fallible stable insertion sorting uses no
                // hidden allocation and retains the same original tie order.
                for index in 1..best.len() {
                    let mut cursor = index;
                    while cursor > 0 {
                        if compare_axes(
                            &best[cursor - 1].axes,
                            &best[cursor].axes,
                            axis,
                            true,
                            &mut self.work,
                        )? != Ordering::Greater
                        {
                            break;
                        }
                        charge(&mut self.work)?;
                        best.swap(cursor - 1, cursor);
                        cursor -= 1;
                    }
                }
                let mut roots = [0; 6];
                for (destination, group) in roots.iter_mut().zip(&mut best) {
                    *destination = self.build(&mut group.children, depth + 1)?;
                }
                let root = self.branch(&roots[..best.len()])?;
                for group in best.drain(..) {
                    self.release(group.children);
                }
                self.release(best);
                Ok(root)
            }
        }
    }
    fn sort_nodes(
        &mut self,
        nodes: &mut [u16],
        mode: Option<(usize, bool)>,
    ) -> Result<(), BiomeError> {
        let mut scratch = self.scratch(nodes.len())?;
        scratch.resize(nodes.len(), 0);
        let mut width = 1;
        while width < nodes.len() {
            let mut start = 0;
            while start < nodes.len() {
                let middle = (start + width).min(nodes.len());
                let end = (middle + width).min(nodes.len());
                let (mut left, mut right) = (start, middle);
                for destination in &mut scratch[start..end] {
                    let take_left = if right == end {
                        true
                    } else if left == middle {
                        false
                    } else {
                        let left_axes = self.node(nodes[left])?.axes;
                        let right_axes = self.node(nodes[right])?.axes;
                        let order = if let Some((axis, absolute)) = mode {
                            compare_axes(&left_axes, &right_axes, axis, absolute, &mut self.work)?
                        } else {
                            compare_near(&left_axes, &right_axes, &mut self.work)?
                        };
                        order != Ordering::Greater
                    };
                    charge(&mut self.work)?;
                    *destination = if take_left {
                        let value = nodes[left];
                        left += 1;
                        value
                    } else {
                        let value = nodes[right];
                        right += 1;
                        value
                    };
                }
                start = end;
            }
            for (destination, &source) in nodes.iter_mut().zip(&scratch) {
                charge(&mut self.work)?;
                *destination = source;
            }
            width *= 2;
        }
        self.release(scratch);
        Ok(())
    }
}

// Original bucket lengths depend only on count, so bounded shape planning can
// reserve the exact final tree without retaining a maximum-size Nether buffer.
fn plan_nodes(count: usize, depth: usize, work: &mut SampleWork) -> Result<usize, BiomeError> {
    charge(work)?;
    limit(BiomeLimit::Depth, depth, MAX_BIOME_TREE_DEPTH)?;
    match count {
        0 => Err(fail(BiomeErrorKind::InvalidTree)),
        1 => Ok(1),
        2..=6 => {
            limit(BiomeLimit::Depth, depth + 1, MAX_BIOME_TREE_DEPTH)?;
            Ok(count + 1)
        }
        _ => {
            let width = bucket_width(count)?;
            let mut remaining = count;
            let mut nodes = 1_usize;
            while remaining != 0 {
                charge(work)?;
                let length = remaining.min(width);
                nodes = nodes
                    .checked_add(plan_nodes(length, depth + 1, work)?)
                    .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
                limit(BiomeLimit::Nodes, nodes, MAX_BIOME_TREE_NODES)?;
                remaining -= length;
            }
            Ok(nodes)
        }
    }
}

fn midpoint([minimum, maximum]: [i64; 2], absolute: bool) -> i64 {
    let value = minimum.wrapping_add(maximum) / 2;
    if absolute {
        value.wrapping_abs()
    } else {
        value
    }
}
fn compare_axes(
    left: &[[i64; 2]; 7],
    right: &[[i64; 2]; 7],
    start: usize,
    absolute: bool,
    work: &mut SampleWork,
) -> Result<Ordering, BiomeError> {
    for offset in 0..7 {
        charge(work)?;
        let axis = (start + offset) % 7;
        let order = midpoint(left[axis], absolute).cmp(&midpoint(right[axis], absolute));
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(Ordering::Equal)
}
fn compare_near(
    left: &[[i64; 2]; 7],
    right: &[[i64; 2]; 7],
    work: &mut SampleWork,
) -> Result<Ordering, BiomeError> {
    let (mut left_sum, mut right_sum) = (0_i64, 0_i64);
    for (&left, &right) in left.iter().zip(right) {
        charge(work)?;
        left_sum = left_sum.wrapping_add(midpoint(left, true));
        right_sum = right_sum.wrapping_add(midpoint(right, true));
    }
    Ok(left_sum.cmp(&right_sum))
}
fn bucket_width(count: usize) -> Result<usize, BiomeError> {
    if !(7..=MAX_BIOME_PARAMETERS).contains(&count) {
        return Err(fail(BiomeErrorKind::InvalidTree));
    }
    let mut width = 1;
    while width * 6 < count {
        width *= 6;
    }
    Ok(width)
}
