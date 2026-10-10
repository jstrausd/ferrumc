//! Exclusively owned Paper 1.21.8 build 60 fresh-noise-chunk evaluation.

use std::sync::Arc;

use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;

use super::bind::{BoundRouter, BoundView};
use super::seeded::{check_position, quantize};
use super::{
    ClimateAxis, ClimateQuartPos, ClimateSample, DensitySampleError, DensitySampleErrorKind,
    NoiseGeometry, SeededRouter,
};
use crate::sample_work::SampleWork;
use crate::WorldSeed;

mod cold;
mod driver;
mod fill;
mod state;

use state::{ChunkState, Lifecycle};

/// Largest aggregate allowance for construction and all queries until reset.
pub const MAX_NOISE_CHUNK_WORK: u32 = 268_435_456;
const MAX_SURFACE_COLUMNS: usize = 2048;

/// Separately admitted storage or graph resource; these are not an RSS limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseChunkLimit {
    /// Runtime density/spline vertices.
    Vertices,
    /// Runtime child edges.
    Edges,
    /// Combined runtime density/spline depth.
    Depth,
    /// Actual recursive cold visits.
    ColdVisits,
    /// Java-equality keys.
    EqualityKeys,
    /// Actual visitor map entries.
    VisitorEntries,
    /// Identity-bearing original and wrapper objects.
    IdentityTokens,
    /// Mutable wrappers, including both empty-blending grids.
    Wrappers,
    /// Additional equality/identity metadata bytes.
    KeyBytes,
    /// Retained runtime graph metadata bytes.
    GraphBytes,
    /// Temporary cold-construction bytes.
    ColdBytes,
    /// Owned cache/sample-array bytes.
    CacheBytes,
    /// Simultaneously leased ADD-right scratch bytes.
    ScratchBytes,
    /// Complete output samples/bytes.
    Output,
    /// Distinct preliminary-surface quart columns.
    SurfaceColumns,
}

/// Classified construction, admission or evaluation rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoiseChunkErrorKind {
    /// Work allowance is outside the closed supported range.
    InvalidBudget { units: u32, maximum: u32 },
    /// An interior block or the positive interpolation corner is outside ±30M.
    ChunkOutOfRange { axis: &'static str, coordinate: i32 },
    /// Existing checked scalar/coordinate/noise arithmetic rejected execution.
    Density(DensitySampleError),
    /// The next metered operation was refused before execution.
    WorkExceeded { limit: usize, used: usize },
    /// A separately bounded resource would exceed its admitted capacity.
    Limit {
        limit: NoiseChunkLimit,
        count: usize,
        maximum: usize,
    },
    /// A public or private request violates the session's current phase.
    Lifecycle {
        operation: &'static str,
        phase: &'static str,
    },
    /// A private producer violated its checked shape or child-index contract.
    InvalidGraph,
}

/// Typed failure; no partial chunk-density output is published.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("fresh Paper noise chunk failed: {kind:?}")]
pub struct NoiseChunkError {
    kind: NoiseChunkErrorKind,
}
impl NoiseChunkError {
    /// Classified failure without message matching.
    pub const fn kind(&self) -> &NoiseChunkErrorKind {
        &self.kind
    }
}
impl From<DensitySampleError> for NoiseChunkError {
    fn from(value: DensitySampleError) -> Self {
        fail(NoiseChunkErrorKind::Density(value))
    }
}
impl From<crate::sample_work::SampleWorkExceeded> for NoiseChunkError {
    fn from(value: crate::sample_work::SampleWorkExceeded) -> Self {
        fail(NoiseChunkErrorKind::WorkExceeded {
            limit: value.limit,
            used: value.used,
        })
    }
}

/// An explicitly admitted aggregate budget shared until successful reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoiseChunkWorkBudget(u32);
impl NoiseChunkWorkBudget {
    /// Admits exactly `1..=MAX_NOISE_CHUNK_WORK` units.
    pub fn new(units: u32) -> Result<Self, NoiseChunkError> {
        if units == 0 || units > MAX_NOISE_CHUNK_WORK {
            return Err(fail(NoiseChunkErrorKind::InvalidBudget {
                units,
                maximum: MAX_NOISE_CHUNK_WORK,
            }));
        }
        Ok(Self(units))
    }
    /// Chosen aggregate work allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

/// One successful owned material-density array, before aquifer/material rules.
#[derive(Debug)]
pub struct ChunkDensitySamples {
    seed: WorldSeed,
    resource: NoiseSettingsResourceId,
    chunk: ChunkPos,
    geometry: NoiseGeometry,
    values: Box<[f64]>,
}
impl ChunkDensitySamples {
    /// Full original signed seed bits.
    pub const fn seed(&self) -> WorldSeed {
        self.seed
    }
    /// Original settings resource.
    pub const fn resource(&self) -> NoiseSettingsResourceId {
        self.resource
    }
    /// Admitted complete chunk column.
    pub const fn chunk_pos(&self) -> ChunkPos {
        self.chunk
    }
    /// Complete original settings noise geometry, without height clamping.
    pub const fn geometry(&self) -> NoiseGeometry {
        self.geometry
    }
    /// Ascending-Y/X/Z values; index `((y-minY)*16+localX)*16+localZ`.
    pub fn values(&self) -> &[f64] {
        &self.values
    }
    /// Returns a value only inside this chunk and its complete noise height.
    pub fn density_at(&self, pos: BlockPos) -> Option<f64> {
        let index = output_index(self.chunk, self.geometry, pos)?;
        self.values.get(index).copied()
    }
}

/// Owned synchronous fresh-chunk consumer with the original five cache types.
///
/// Uses Paper 1.21.8 build 60 numerical policy, empty blending, no retrogen or
/// structure contributions, and complete settings geometry. This produces
/// densities, preliminary surfaces and climate; it does not generate blocks.
/// Mutable arrays and counters belong exclusively to this session. Reset may
/// temporarily retain both the old session and a fully admitted candidate.
pub struct NoiseChunkSession {
    data: Arc<BoundRouter>,
    chunk: ChunkPos,
    view: BoundView,
    state: ChunkState,
    fields: [usize; 15],
    material: usize,
    climate: [usize; 6],
    work: SampleWork,
    surface: SurfaceCache,
}
impl SeededRouter {
    /// Binds a complete fresh 16×16 Paper-noise chunk, without blending,
    /// structures, retrogen or height-accessor clamping. Construction and every
    /// later query share this single work allowance until successful reset.
    pub fn fresh_paper_noise_chunk(
        &self,
        chunk: ChunkPos,
        budget: NoiseChunkWorkBudget,
    ) -> Result<NoiseChunkSession, NoiseChunkError> {
        NoiseChunkSession::new(Arc::clone(&self.data), chunk, budget)
    }
}
impl NoiseChunkSession {
    fn new(
        data: Arc<BoundRouter>,
        chunk: ChunkPos,
        budget: NoiseChunkWorkBudget,
    ) -> Result<Self, NoiseChunkError> {
        admit_chunk(chunk)?;
        let geometry = data.metadata.geometry;
        let mut work = SampleWork::new(budget.0 as usize);
        let state = ChunkState::new(&data, chunk, geometry, &mut work)?;
        let (view, state, fields, material, climate) = cold::bind(&data, state, &mut work)?;
        Ok(Self {
            data,
            chunk,
            view,
            state,
            fields,
            material,
            climate,
            work,
            surface: SurfaceCache::new(),
        })
    }
    /// Full original signed seed bits.
    pub fn seed(&self) -> WorldSeed {
        self.data.seed
    }
    /// Original settings resource.
    pub fn resource(&self) -> NoiseSettingsResourceId {
        self.data.resource
    }
    /// Admitted complete chunk column.
    pub const fn chunk_pos(&self) -> ChunkPos {
        self.chunk
    }
    /// Complete original settings noise geometry.
    pub const fn geometry(&self) -> NoiseGeometry {
        self.state.geometry
    }
    /// Aggregate construction/query work already charged.
    pub const fn work_used(&self) -> usize {
        self.work.used()
    }
    /// Runs exactly one original interpolation/material-density loop per reset.
    /// A failure poisons this session and publishes no partial density array.
    pub fn sample_chunk_densities(&mut self) -> Result<ChunkDensitySamples, NoiseChunkError> {
        self.state
            .require(Lifecycle::Fresh, "sample_chunk_densities")?;
        self.state.lifecycle = Lifecycle::Running;
        let result = self.drive();
        self.state.lifecycle = if result.is_ok() {
            Lifecycle::Complete
        } else {
            Lifecycle::Poisoned
        };
        result
    }
    /// Original quart-aligned preliminary level; Y is ignored. Returns
    /// `i32::MAX` when no density exceeds the original strict threshold.
    pub fn preliminary_surface_level(&mut self, pos: BlockPos) -> Result<i32, NoiseChunkError> {
        check_position(pos)?;
        self.state.require_query("preliminary_surface_level")?;
        let result = self.preliminary(pos);
        if result.is_err() {
            self.state.lifecycle = Lifecycle::Poisoned;
        }
        result
    }
    /// Uses the six original cached-Climate roots in their original order.
    pub fn sample_climate(
        &mut self,
        pos: ClimateQuartPos,
    ) -> Result<ClimateSample, NoiseChunkError> {
        self.state.require_query("sample_climate")?;
        let result = self.climate_sample(pos.block_origin());
        if result.is_err() {
            self.state.lifecycle = Lifecycle::Poisoned;
        }
        result
    }
    /// Builds a complete new candidate and swaps only after construction
    /// succeeds. A rejected reset leaves every old cache/counter unchanged.
    pub fn reset(&mut self, budget: NoiseChunkWorkBudget) -> Result<(), NoiseChunkError> {
        let candidate = Self::new(Arc::clone(&self.data), self.chunk, budget)?;
        *self = candidate;
        Ok(())
    }
    fn preliminary(&mut self, pos: BlockPos) -> Result<i32, NoiseChunkError> {
        let x = (pos.x() >> 2) * 4;
        let z = (pos.z() >> 2) * 4;
        let key = pack_column(x, z);
        if let Some(value) = self.surface.get(key, &mut self.work)? {
            return Ok(value);
        }
        self.surface.admit()?;
        let geometry = self.geometry();
        let mut y = geometry.min_y() + geometry.height().cast_signed();
        let mut level = i32::MAX;
        while y >= geometry.min_y() {
            charge(&mut self.work)?;
            if self.state.sample(
                &self.data,
                &self.view,
                self.fields[10],
                BlockPos::new(x, y, z),
                false,
                &mut self.work,
            )? > 0.390_625
            {
                level = y;
                break;
            }
            y -= geometry.cell_height().cast_signed();
        }
        charge(&mut self.work)?;
        self.surface.insert(key, level)?;
        Ok(level)
    }
    fn climate_sample(&mut self, pos: BlockPos) -> Result<ClimateSample, NoiseChunkError> {
        let mut values = [0.0_f32; 6];
        for axis in ClimateAxis::ALL {
            let value = &mut values[axis as usize];
            *value = self.state.sample(
                &self.data,
                &self.view,
                self.climate[axis as usize],
                pos,
                false,
                &mut self.work,
            )? as f32;
            if !value.is_finite() {
                return Err(DensitySampleError {
                    resource: self.resource().resource_location(),
                    field: axis.name(),
                    position: Some(pos),
                    batch_index: None,
                    kind: DensitySampleErrorKind::NonFiniteCoordinate {
                        bits: value.to_bits(),
                    },
                }
                .into());
            }
            charge(&mut self.work)?;
        }
        Ok(ClimateSample {
            values,
            quantized: values.map(quantize),
        })
    }
}

struct SurfaceCache {
    keys: Box<[u64; MAX_SURFACE_COLUMNS]>,
    values: Box<[i32; MAX_SURFACE_COLUMNS]>,
    len: usize,
}
impl SurfaceCache {
    fn new() -> Self {
        Self {
            keys: Box::new([0; MAX_SURFACE_COLUMNS]),
            values: Box::new([0; MAX_SURFACE_COLUMNS]),
            len: 0,
        }
    }
    fn get(&self, key: u64, work: &mut SampleWork) -> Result<Option<i32>, NoiseChunkError> {
        for (i, &prior) in self.keys[..self.len].iter().enumerate() {
            charge(work)?;
            if prior == key {
                return Ok(Some(self.values[i]));
            }
        }
        Ok(None)
    }
    fn admit(&self) -> Result<(), NoiseChunkError> {
        limit(
            NoiseChunkLimit::SurfaceColumns,
            self.len + 1,
            MAX_SURFACE_COLUMNS,
        )
    }
    fn insert(&mut self, key: u64, value: i32) -> Result<(), NoiseChunkError> {
        self.admit()?;
        self.keys[self.len] = key;
        self.values[self.len] = value;
        self.len += 1;
        Ok(())
    }
}
fn fail(kind: NoiseChunkErrorKind) -> NoiseChunkError {
    NoiseChunkError { kind }
}
fn limit(kind: NoiseChunkLimit, count: usize, maximum: usize) -> Result<(), NoiseChunkError> {
    if count > maximum {
        Err(fail(NoiseChunkErrorKind::Limit {
            limit: kind,
            count,
            maximum,
        }))
    } else {
        Ok(())
    }
}
// Check aggregate bytes before allocation, including integer overflow.
fn bytes(
    kind: NoiseChunkLimit,
    base: usize,
    count: usize,
    size: usize,
    maximum: usize,
) -> Result<usize, NoiseChunkError> {
    let total = count
        .checked_mul(size)
        .and_then(|n| base.checked_add(n))
        .ok_or_else(|| {
            fail(NoiseChunkErrorKind::Limit {
                limit: kind,
                count: usize::MAX,
                maximum,
            })
        })?;
    limit(kind, total, maximum)?;
    Ok(total)
}
fn charge(work: &mut SampleWork) -> Result<(), NoiseChunkError> {
    work.charge().map_err(|e| {
        fail(NoiseChunkErrorKind::WorkExceeded {
            limit: e.limit,
            used: e.used,
        })
    })
}
fn pack_column(x: i32, z: i32) -> u64 {
    u64::from(x as u32) | (u64::from(z as u32) << 32)
}
fn admit_chunk(chunk: ChunkPos) -> Result<(), NoiseChunkError> {
    for (axis, coordinate) in [("x", chunk.x()), ("z", chunk.z())] {
        let origin = i64::from(coordinate) * 16;
        if origin < -30_000_000 || origin + 16 > 30_000_000 {
            return Err(fail(NoiseChunkErrorKind::ChunkOutOfRange {
                axis,
                coordinate,
            }));
        }
    }
    Ok(())
}
fn output_index(chunk: ChunkPos, geometry: NoiseGeometry, pos: BlockPos) -> Option<usize> {
    if pos.to_chunk_pos() != chunk {
        return None;
    }
    let y = i64::from(pos.y()) - i64::from(geometry.min_y());
    if y < 0 || y >= i64::from(geometry.height()) {
        return None;
    }
    Some(((y as usize * 16) + (pos.x() & 15) as usize) * 16 + (pos.z() & 15) as usize)
}

#[cfg(test)]
mod tests;
