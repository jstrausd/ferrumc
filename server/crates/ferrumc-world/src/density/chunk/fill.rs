//! Original fillArray dispatch and context-provider request order.

#![expect(
    clippy::cast_possible_wrap,
    reason = "Private geometry and provider indices are admitted at no more than 512 or 131072 before signed coordinate conversion."
)]

use ferrumc_math::{BlockPos, Vec3};

use super::state::{Cache, ChunkState};
use super::{
    core_charge as charge, core_fail as fail, NoiseChunkCoreError as NoiseChunkError,
    NoiseChunkErrorKind,
};
use crate::density::bind::{java_max, java_min, BoundKind, BoundRouter, BoundView};
use crate::density::evaluate;
use crate::density::graph::{Binary, NodeKind, Unit};
use crate::density::{DensitySampleError, DensitySampleErrorKind};
use crate::sample_work::SampleWork;

#[derive(Clone, Copy)]
pub(super) enum Provider {
    Slice,
    Cell,
    #[cfg(test)]
    Foreign(usize),
}
impl Provider {
    fn own(self) -> bool {
        match self {
            Self::Slice | Self::Cell => true,
            #[cfg(test)]
            Self::Foreign(_) => false,
        }
    }
}
impl ChunkState {
    pub fn for_index(
        &mut self,
        provider: Provider,
        index: usize,
        work: &mut SampleWork,
    ) -> Result<BlockPos, NoiseChunkError> {
        let maximum = match provider {
            Provider::Slice => self.count_y + 1,
            Provider::Cell => self.cell_volume(),
            #[cfg(test)]
            Provider::Foreign(length) if length <= 128 => length,
            #[cfg(test)]
            Provider::Foreign(_) => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
        };
        if index >= maximum {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        charge(work)?;
        match provider {
            #[cfg(test)]
            Provider::Foreign(_) => {
                if let Some(requests) = &mut self.requests {
                    requests.record(
                        serde_json::json!({"kind":"provider","case":"foreign","index":index}),
                    );
                }
                return Ok(BlockPos::new(
                    index as i32,
                    100 + index as i32,
                    -(index as i32),
                ));
            }
            Provider::Slice => {
                self.start = BlockPos::new(
                    self.start.x(),
                    (index as i32 + self.min_cell_y) * self.height as i32,
                    self.start.z(),
                );
                self.scalar_counter = self.scalar_counter.wrapping_add(1);
                self.in_y = 0;
            }
            Provider::Cell => {
                self.in_z = (index % self.width) as i32;
                self.in_x = ((index / self.width) % self.width) as i32;
                self.in_y = (self.height - 1 - index / (self.width * self.width)) as i32;
            }
        }
        self.array_index = index;
        Ok(self.pos())
    }
    pub(super) fn fill_direct(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        root: usize,
        values: &mut [f64],
        provider: Provider,
        work: &mut SampleWork,
    ) -> Result<(), NoiseChunkError> {
        let expected = match provider {
            Provider::Slice => self.count_y + 1,
            Provider::Cell => self.cell_volume(),
            #[cfg(test)]
            Provider::Foreign(length) if length <= 128 => length,
            #[cfg(test)]
            Provider::Foreign(_) => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
        };
        if values.len() != expected || view.nodes.get(root).is_none() {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        if provider.own() {
            self.array_index = 0;
        }
        #[cfg(test)]
        if !provider.own() {
            if let Some(requests) = &mut self.requests {
                requests.record(serde_json::json!({"kind":"provider","case":"foreign","direct_length":values.len()}));
            }
        }
        for (i, slot) in values.iter_mut().enumerate() {
            let pos = self.for_index(provider, i, work)?;
            // Java evaluates values[arrayIndex++] before the RHS compute.
            // Cell direct fill therefore exposes i+1, while forIndex exposes i.
            if matches!(provider, Provider::Cell) {
                self.array_index += 1;
            }
            let value = self.sample(data, view, root, pos, provider.own(), work)?;
            write(slot, value, pos, work)?;
        }
        Ok(())
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Exhaustive original array dispatch keeps branch-specific request order visible."
    )]
    pub fn fill(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        root: usize,
        values: &mut [f64],
        provider: Provider,
        work: &mut SampleWork,
    ) -> Result<(), NoiseChunkError> {
        let expected = match provider {
            Provider::Slice => self.count_y + 1,
            Provider::Cell => self.cell_volume(),
            #[cfg(test)]
            Provider::Foreign(length) if length <= 128 => length,
            #[cfg(test)]
            Provider::Foreign(_) => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
        };
        if values.len() != expected || expected > 131_072 {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        let node = view
            .nodes
            .get(root)
            .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
        charge(work)?;
        #[cfg(test)]
        if let Some(result) = super::tests::requests::fill_declared(self, root, values, work) {
            return result.map_err(Into::into);
        }
        match &node.kind {
            BoundKind::Alias(target) => self.fill(data, view, *target, values, provider, work)?,
            BoundKind::MulOrAdd {
                operation,
                input,
                argument,
            } => {
                self.fill(data, view, *input, values, provider, work)?;
                for slot in values {
                    let value = match operation {
                        Binary::Add => *slot + argument,
                        Binary::Mul => *slot * argument,
                        _ => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
                    };
                    write(slot, value, self.pos(), work)?;
                }
            }
            BoundKind::Original(kind) => match kind {
                NodeKind::Constant(value) => {
                    for slot in values {
                        write(slot, *value, self.pos(), work)?;
                    }
                }
                NodeKind::Unit(Unit::BlendAlpha | Unit::BlendOffset | Unit::Beardifier) => {
                    let value = if matches!(kind, NodeKind::Unit(Unit::BlendAlpha)) {
                        1.0
                    } else {
                        0.0
                    };
                    for slot in values {
                        write(slot, value, self.pos(), work)?;
                    }
                }
                NodeKind::Holder { target, .. } => {
                    self.fill(data, view, *target, values, provider, work)?;
                }
                NodeKind::Marker { input, .. } => {
                    let id = self
                        .wrapper_nodes
                        .get(root)
                        .copied()
                        .flatten()
                        .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
                    match &self.wrappers[id].cache {
                        Cache::TwoD { .. } => {
                            self.fill(data, view, *input, values, provider, work)?;
                        }
                        Cache::Interpolated(_) if !self.filling_cell => {
                            self.fill(data, view, *input, values, provider, work)?;
                        }
                        Cache::Once {
                            array_counter,
                            array,
                            valid_len,
                            ..
                        } if valid_len.is_some() && *array_counter == self.array_counter => {
                            if values.len() > valid_len.unwrap_or(0) {
                                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
                            }
                            for (slot, &value) in values.iter_mut().zip(array) {
                                write(slot, value, self.pos(), work)?;
                            }
                        }
                        Cache::Once { .. } => {
                            self.fill(data, view, *input, values, provider, work)?;
                            if let Cache::Once {
                                array,
                                valid_len,
                                array_counter,
                                ..
                            } = &mut self.wrappers[id].cache
                            {
                                if values.len() > array.len() {
                                    return Err(fail(NoiseChunkErrorKind::InvalidGraph));
                                }
                                for (slot, &value) in array.iter_mut().zip(values.iter()) {
                                    charge(work)?;
                                    *slot = value;
                                }
                                *valid_len = Some(values.len());
                                *array_counter = self.array_counter;
                            }
                        }
                        _ => self.fill_direct(data, view, root, values, provider, work)?,
                    }
                }
                NodeKind::Unary { operation, input } => {
                    self.fill(data, view, *input, values, provider, work)?;
                    for slot in values {
                        write(slot, evaluate::unary(*operation, *slot), self.pos(), work)?;
                    }
                }
                NodeKind::Clamp { input, min, max } => {
                    self.fill(data, view, *input, values, provider, work)?;
                    for slot in values {
                        write(slot, evaluate::clamp(*slot, *min, *max), self.pos(), work)?;
                    }
                }
                NodeKind::Binary {
                    operation: Binary::Add,
                    left,
                    right,
                } => {
                    self.fill(data, view, *left, values, provider, work)?;
                    let mut scratch = self
                        .scratch
                        .pop()
                        .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
                    let result = self.fill(
                        data,
                        view,
                        *right,
                        &mut scratch[..values.len()],
                        provider,
                        work,
                    );
                    let result = result.and_then(|()| {
                        for (slot, &b) in values.iter_mut().zip(&scratch) {
                            write(slot, *slot + b, self.pos(), work)?;
                        }
                        Ok(())
                    });
                    self.scratch.push(scratch);
                    result?;
                }
                NodeKind::Binary {
                    operation,
                    left,
                    right,
                } => {
                    self.fill(data, view, *left, values, provider, work)?;
                    let bounds = view.nodes[*right].bounds;
                    for (i, slot) in values.iter_mut().enumerate() {
                        let a = *slot;
                        let skip = match operation {
                            Binary::Mul => a == 0.0,
                            Binary::Min => a < bounds.min,
                            Binary::Max => a > bounds.max,
                            Binary::Add => false,
                        };
                        let value = if skip {
                            if matches!(operation, Binary::Mul) {
                                0.0
                            } else {
                                a
                            }
                        } else {
                            let pos = self.for_index(provider, i, work)?;
                            let b = self.sample(data, view, *right, pos, provider.own(), work)?;
                            match operation {
                                Binary::Mul => a * b,
                                Binary::Min => java_min(a, b),
                                Binary::Max => java_max(a, b),
                                Binary::Add => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
                            }
                        };
                        write(slot, value, self.pos(), work)?;
                    }
                }
                NodeKind::RangeChoice {
                    input,
                    min,
                    max,
                    inside,
                    outside,
                } => {
                    self.fill(data, view, *input, values, provider, work)?;
                    for (i, slot) in values.iter_mut().enumerate() {
                        let child = if *slot >= *min && *slot < *max {
                            *inside
                        } else {
                            *outside
                        };
                        let pos = self.for_index(provider, i, work)?;
                        let value = self.sample(data, view, child, pos, provider.own(), work)?;
                        write(slot, value, pos, work)?;
                    }
                }
                NodeKind::BlendDensity(input) => {
                    self.fill(data, view, *input, values, provider, work)?;
                    for (i, slot) in values.iter_mut().enumerate() {
                        let pos = self.for_index(provider, i, work)?;
                        write(slot, *slot, pos, work)?;
                    }
                }
                NodeKind::Weird { input, rarity, .. } => {
                    self.fill(data, view, *input, values, provider, work)?;
                    for (i, slot) in values.iter_mut().enumerate() {
                        let pos = self.for_index(provider, i, work)?;
                        let rarity = evaluate::rarity_value(*rarity, *slot);
                        let value = rarity
                            * evaluate::normal_core(
                                data,
                                node,
                                Vec3::new(
                                    f64::from(pos.x()) / rarity,
                                    f64::from(pos.y()) / rarity,
                                    f64::from(pos.z()) / rarity,
                                ),
                                pos,
                                work,
                                "noise_chunk",
                                self,
                            )?
                            .abs();
                        write(slot, value, pos, work)?;
                    }
                }
                NodeKind::Coordinate { .. }
                | NodeKind::SplineConstant(_)
                | NodeKind::SplineMultipoint { .. } => {
                    return Err(fail(NoiseChunkErrorKind::InvalidGraph))
                }
                _ => self.fill_direct(data, view, root, values, provider, work)?,
            },
        }
        Ok(())
    }
}
pub(super) fn write(
    slot: &mut f64,
    value: f64,
    pos: BlockPos,
    work: &mut SampleWork,
) -> Result<(), NoiseChunkError> {
    if !value.is_finite() {
        return Err(DensitySampleError {
            resource: "noise_chunk",
            field: "$",
            position: Some(pos),
            batch_index: None,
            kind: DensitySampleErrorKind::NonFiniteDensity {
                bits: value.to_bits(),
            },
        }
        .into());
    }
    charge(work)?;
    *slot = value;
    Ok(())
}
