//! Original wrapper state; mutable identity is strictly session-local.

#![expect(
    clippy::cast_possible_wrap,
    reason = "Private geometry and provider indices are admitted at no more than 512 or 131072 before signed coordinate conversion."
)]

use ferrumc_math::{BlockPos, ChunkPos};

use super::{
    bytes, charge, fail, limit, pack_column, NoiseChunkError, NoiseChunkErrorKind, NoiseChunkLimit,
};
use crate::density::bind::{BoundRouter, BoundView};
use crate::density::evaluate::{self, EndRadialReference, MarkerEvaluator};
use crate::density::graph::Marker;
use crate::density::seeded::{DensitySampleError, DensitySampleErrorKind};
use crate::density::NoiseGeometry;
use crate::sample_work::SampleWork;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Lifecycle {
    Fresh,
    Running,
    Complete,
    Poisoned,
}
impl Lifecycle {
    fn name(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Poisoned => "poisoned",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DriverPhase {
    Dormant,
    FirstSlice,
    NeedAdvance,
    SlicesReady,
    CellSelected,
    YUpdated,
    XUpdated,
    ZUpdated,
    Complete,
}
impl DriverPhase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dormant => "driver_dormant",
            Self::FirstSlice => "driver_first_slice",
            Self::NeedAdvance => "driver_need_advance",
            Self::SlicesReady => "driver_slices_ready",
            Self::CellSelected => "driver_cell_selected",
            Self::YUpdated => "driver_y_updated",
            Self::XUpdated => "driver_x_updated",
            Self::ZUpdated => "driver_z_updated",
            Self::Complete => "driver_complete",
        }
    }
}

pub(super) struct Interpolator {
    pub slice0: Vec<f64>,
    pub slice1: Vec<f64>,
    pub corners: [f64; 8],
    pub y_values: [f64; 4],
    pub x_values: [f64; 2],
    pub value: f64,
}
pub(super) enum Cache {
    Interpolated(Interpolator),
    Flat(Vec<f64>),
    TwoD {
        key: u64,
        value: f64,
    },
    Once {
        counter: i64,
        array_counter: i64,
        value: f64,
        array: Vec<f64>,
        valid_len: Option<usize>,
    },
    Cell(Vec<f64>),
}
pub(super) struct Wrapper {
    pub node: usize,
    pub input: usize,
    pub cache: Cache,
}
pub(super) struct ChunkState {
    #[cfg(test)]
    pub requests: Option<super::tests::requests::Requests>,
    pub normal_noises: Box<[crate::noise::NormalNoise]>,
    #[cfg(test)]
    pub sampler_bytes: usize,
    pub graph_bytes: usize,
    pub scratch_bytes: usize,
    pub lifecycle: Lifecycle,
    pub driver_phase: DriverPhase,
    pub driver_x: usize,
    pub origin: BlockPos,
    pub geometry: NoiseGeometry,
    pub width: usize,
    pub height: usize,
    pub count_xz: usize,
    pub count_y: usize,
    pub min_cell_y: i32,
    pub wrappers: Vec<Wrapper>,
    pub wrapper_nodes: Vec<Option<usize>>,
    pub interpolators: Vec<usize>,
    pub cells: Vec<usize>,
    pub cache_bytes: usize,
    #[cfg(test)]
    pub wrap_entries: usize,
    pub scratch: Vec<Vec<f64>>,
    pub interpolating: bool,
    pub filling_cell: bool,
    pub start: BlockPos,
    pub in_x: i32,
    pub in_y: i32,
    pub in_z: i32,
    pub scalar_counter: i64,
    pub array_counter: i64,
    pub array_index: usize,
    own_context: bool,
    marker_error: Option<NoiseChunkError>,
}
impl ChunkState {
    pub fn new(
        data: &BoundRouter,
        chunk: ChunkPos,
        geometry: NoiseGeometry,
        work: &mut SampleWork,
    ) -> Result<Self, NoiseChunkError> {
        let width = geometry.cell_width() as usize;
        let height = geometry.cell_height() as usize;
        if width == 0
            || width > 16
            || height == 0
            || height > 512
            || 16 % width != 0
            || geometry.height() == 0
            || !(geometry.height() as usize).is_multiple_of(height)
            || geometry.min_y() % height as i32 != 0
        {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        let origin = chunk.origin_block(0);
        let mut sampler_bytes = 0;
        for (_, noise) in &data.noises {
            sampler_bytes = bytes(
                NoiseChunkLimit::GraphBytes,
                sampler_bytes,
                1,
                noise.retained_bytes(),
                2 * 1024 * 1024,
            )?;
        }
        let metadata = std::mem::size_of::<Self>()
            + 128 * (std::mem::size_of::<Wrapper>() + 2 * std::mem::size_of::<usize>())
            + 4096 * std::mem::size_of::<Option<usize>>();
        let graph_bytes = bytes(
            NoiseChunkLimit::GraphBytes,
            metadata,
            1,
            sampler_bytes,
            2 * 1024 * 1024,
        )?;
        let normal_noises = data
            .noises
            .iter()
            .map(|(_, noise)| noise.paper_build60_basis(work))
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();
        let mut state = Self {
            normal_noises,
            graph_bytes,
            scratch_bytes: 0,
            #[cfg(test)]
            sampler_bytes,
            lifecycle: Lifecycle::Fresh,
            origin,
            geometry,
            width,
            height,
            count_xz: 16 / width,
            count_y: geometry.height() as usize / height,
            min_cell_y: geometry.min_y().div_euclid(height as i32),
            wrappers: Vec::with_capacity(128),
            wrapper_nodes: Vec::with_capacity(4096),
            interpolators: Vec::with_capacity(128),
            cells: Vec::with_capacity(128),
            cache_bytes: 0,
            #[cfg(test)]
            wrap_entries: 0,
            scratch: Vec::new(),
            #[cfg(test)]
            requests: None,
            driver_phase: DriverPhase::Dormant,
            driver_x: 0,
            interpolating: false,
            filling_cell: false,
            start: origin,
            in_x: 0,
            in_y: 0,
            in_z: 0,
            scalar_counter: 0,
            array_counter: 0,
            array_index: 0,
            own_context: false,
            marker_error: None,
        };
        // Empty Blender still owns and fills both constructor grids.
        for value in [1.0, 0.0] {
            let id = state.add_wrapper(Marker::FlatCache, usize::MAX)?;
            if let Cache::Flat(values) = &mut state.wrappers[id].cache {
                for slot in values {
                    charge(work)?;
                    *slot = value;
                }
            }
        }
        Ok(state)
    }
    pub fn require(
        &self,
        phase: Lifecycle,
        operation: &'static str,
    ) -> Result<(), NoiseChunkError> {
        if self.lifecycle == phase {
            Ok(())
        } else {
            Err(fail(NoiseChunkErrorKind::Lifecycle {
                operation,
                phase: self.lifecycle.name(),
            }))
        }
    }
    pub fn require_query(&self, operation: &'static str) -> Result<(), NoiseChunkError> {
        if matches!(self.lifecycle, Lifecycle::Fresh | Lifecycle::Complete) {
            Ok(())
        } else {
            Err(fail(NoiseChunkErrorKind::Lifecycle {
                operation,
                phase: self.lifecycle.name(),
            }))
        }
    }
    fn array(&mut self, len: usize) -> Result<Vec<f64>, NoiseChunkError> {
        self.cache_bytes = bytes(
            NoiseChunkLimit::CacheBytes,
            self.cache_bytes,
            len,
            8,
            8 * 1024 * 1024,
        )?;
        Ok(vec![0.0; len])
    }
    pub fn add_wrapper(&mut self, marker: Marker, input: usize) -> Result<usize, NoiseChunkError> {
        limit(NoiseChunkLimit::Wrappers, self.wrappers.len() + 1, 128)?;
        let id = self.wrappers.len();
        let cache = match marker {
            Marker::Interpolated => {
                let len = (self.count_xz + 1) * (self.count_y + 1);
                let result = Cache::Interpolated(Interpolator {
                    slice0: self.array(len)?,
                    slice1: self.array(len)?,
                    corners: [0.0; 8],
                    y_values: [0.0; 4],
                    x_values: [0.0; 2],
                    value: 0.0,
                });
                self.interpolators.push(id);
                result
            }
            Marker::FlatCache => Cache::Flat(self.array(25)?),
            Marker::Cache2D => Cache::TwoD {
                key: pack_column(1_875_066, 1_875_066),
                value: 0.0,
            },
            Marker::CacheOnce => Cache::Once {
                counter: 0,
                array_counter: 0,
                value: 0.0,
                array: self.array(self.cell_volume().max(self.count_y + 1))?,
                valid_len: None,
            },
            Marker::CacheAllInCell => {
                let array = self.array(self.cell_volume())?;
                self.cells.push(id);
                Cache::Cell(array)
            }
        };
        self.wrappers.push(Wrapper {
            node: usize::MAX,
            input,
            cache,
        });
        Ok(id)
    }
    pub fn cell_volume(&self) -> usize {
        self.width * self.width * self.height
    }
    pub fn pos(&self) -> BlockPos {
        BlockPos::new(
            self.start.x() + self.in_x,
            self.start.y() + self.in_y,
            self.start.z() + self.in_z,
        )
    }
    pub fn sample(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        root: usize,
        pos: BlockPos,
        own: bool,
        work: &mut SampleWork,
    ) -> Result<f64, NoiseChunkError> {
        let prior = self.own_context;
        self.own_context = own;
        let value = evaluate::visit_with(data, view, root, pos, work, "noise_chunk", self);
        self.own_context = prior;
        if let Some(error) = self.marker_error.take() {
            return Err(error);
        }
        value.map_err(Into::into)
    }
    fn lifecycle_error(&mut self, pos: BlockPos) -> DensitySampleError {
        self.marker_error = Some(fail(NoiseChunkErrorKind::Lifecycle {
            operation: "compute_owned_wrapper",
            phase: "outside_interpolation",
        }));
        invalid(pos)
    }
}

impl MarkerEvaluator for ChunkState {
    #[cfg(test)]
    fn declared_sample(
        &mut self,
        root: usize,
        pos: BlockPos,
    ) -> Option<Result<f64, DensitySampleError>> {
        self.requests.as_ref()?;
        let context = super::tests::requests::context(self, pos, self.own_context);
        self.requests.as_mut()?.sample(root, pos, &context)
    }
    fn end_reference(&self) -> EndRadialReference {
        EndRadialReference::Paper1_21_8Build60
    }
    fn normal_sampler<'a>(
        &'a self,
        _data: &'a BoundRouter,
        index: usize,
    ) -> Option<&'a crate::noise::NormalNoise> {
        self.normal_noises.get(index)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Original five-wrapper dispatch preserves distinct own/foreign cache and failure timing."
    )]
    fn sample_marker(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        root: usize,
        input: usize,
        pos: BlockPos,
        work: &mut SampleWork,
        field: &'static str,
    ) -> Result<f64, DensitySampleError> {
        let id = self
            .wrapper_nodes
            .get(root)
            .copied()
            .flatten()
            .ok_or_else(|| invalid(pos))?;
        let own = self.own_context;
        match &mut self.wrappers[id].cache {
            Cache::Interpolated(interp) => {
                if !own {
                    return evaluate::visit_with(data, view, input, pos, work, field, self);
                }
                if !self.interpolating {
                    return Err(self.lifecycle_error(pos));
                }
                if self.filling_cell {
                    let corners = interp.corners;
                    let x = f64::from(self.in_x) / self.width as f64;
                    let y = f64::from(self.in_y) / self.height as f64;
                    let z = f64::from(self.in_z) / self.width as f64;
                    for _ in 0..7 {
                        work.charge().map_err(|e| density_budget(pos, e))?;
                    }
                    Ok(lerp(
                        z,
                        lerp(
                            y,
                            lerp(x, corners[0], corners[1]),
                            lerp(x, corners[2], corners[3]),
                        ),
                        lerp(
                            y,
                            lerp(x, corners[4], corners[5]),
                            lerp(x, corners[6], corners[7]),
                        ),
                    ))
                } else {
                    Ok(interp.value)
                }
            }
            Cache::Flat(values) => {
                let x = (pos.x() >> 2) - (self.origin.x() >> 2);
                let z = (pos.z() >> 2) - (self.origin.z() >> 2);
                if (0..5).contains(&x) && (0..5).contains(&z) {
                    Ok(values[(x * 5 + z) as usize])
                } else {
                    evaluate::visit_with(data, view, input, pos, work, field, self)
                }
            }
            Cache::TwoD { key, value } => {
                let current = pack_column(pos.x(), pos.z());
                if *key == current {
                    return Ok(*value);
                }
                *key = current;
                let result = evaluate::visit_with(data, view, input, pos, work, field, self)?;
                if let Cache::TwoD { value, .. } = &mut self.wrappers[id].cache {
                    *value = result;
                }
                Ok(result)
            }
            Cache::Once {
                counter,
                array_counter,
                value,
                array,
                valid_len,
            } => {
                if !own {
                    return evaluate::visit_with(data, view, input, pos, work, field, self);
                }
                if let Some(len) = valid_len {
                    if *array_counter == self.array_counter {
                        return array
                            .get(self.array_index)
                            .filter(|_| self.array_index < *len)
                            .copied()
                            .ok_or_else(|| invalid(pos));
                    }
                }
                if *counter == self.scalar_counter {
                    return Ok(*value);
                }
                *counter = self.scalar_counter;
                let result = evaluate::visit_with(data, view, input, pos, work, field, self)?;
                if let Cache::Once { value, .. } = &mut self.wrappers[id].cache {
                    *value = result;
                }
                Ok(result)
            }
            Cache::Cell(values) => {
                if !own {
                    return evaluate::visit_with(data, view, input, pos, work, field, self);
                }
                if !self.interpolating {
                    return Err(self.lifecycle_error(pos));
                }
                if self.in_x >= 0
                    && self.in_y >= 0
                    && self.in_z >= 0
                    && self.in_x < self.width as i32
                    && self.in_y < self.height as i32
                    && self.in_z < self.width as i32
                {
                    Ok(values[((self.height - 1 - self.in_y as usize) * self.width
                        + self.in_x as usize)
                        * self.width
                        + self.in_z as usize])
                } else {
                    evaluate::visit_with(data, view, input, pos, work, field, self)
                }
            }
        }
    }
}
pub(super) fn lerp(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}
fn invalid(pos: BlockPos) -> DensitySampleError {
    DensitySampleError {
        resource: "noise_chunk",
        field: "$",
        position: Some(pos),
        batch_index: None,
        kind: DensitySampleErrorKind::InvalidGraph,
    }
}
fn density_budget(pos: BlockPos, e: crate::sample_work::SampleWorkExceeded) -> DensitySampleError {
    DensitySampleError {
        kind: DensitySampleErrorKind::WorkExceeded {
            limit: e.limit,
            used: e.used,
        },
        ..invalid(pos)
    }
}
