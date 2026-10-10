//! Checked scalar samples in the original empty-Blender point context.

use std::sync::Arc;

use ferrumc_math::BlockPos;
use ferrumc_registry::worldgen::{DensityFunctionResourceId, NoiseSettingsResourceId};

use super::{bind::BoundRouter, evaluate, RouterField, RouterTemplate};
use crate::{noise::NoiseError, sample_work::SampleWork, spline::SplineBuildError, WorldSeed};

/// Largest allowance shared by one density point or all six climate axes.
pub const MAX_DENSITY_SAMPLE_WORK: usize = 1_048_576;
/// Largest allowance shared across one ordered bounded batch.
pub const MAX_DENSITY_BATCH_WORK: usize = 67_108_864;
/// Maximum number of requests published atomically by one scalar batch.
pub const MAX_DENSITY_BATCH_REQUESTS: usize = 4096;
/// Inclusive horizontal block domain of the checked builtin consumer.
pub const MAX_DENSITY_BLOCK_COORDINATE: i32 = 30_000_000;

/// Original diagnostic endpoints, never sample clamps or a guaranteed hull.
/// Signed zeros, nonfinite and reversed diagnostic values are retained.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityBounds {
    pub(super) min: f64,
    pub(super) max: f64,
}
impl DensityBounds {
    /// Original minimum-value diagnostic, including its exact float bits.
    pub const fn min_value(self) -> f64 {
        self.min
    }
    /// Original maximum-value diagnostic, including its exact float bits.
    pub const fn max_value(self) -> f64 {
        self.max
    }
}

/// An explicitly validated allowance for one point or six-axis climate sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityWorkBudget(usize);
impl DensityWorkBudget {
    /// Checks the closed `1..=MAX_DENSITY_SAMPLE_WORK` domain.
    pub fn new(units: usize) -> Result<Self, DensitySampleError> {
        check_budget(units, MAX_DENSITY_SAMPLE_WORK)?;
        Ok(Self(units))
    }
    /// Chosen allowance; no mutable counter is exposed.
    pub const fn units(self) -> usize {
        self.0
    }
}

/// One explicitly validated allowance shared across an entire scalar batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityBatchBudget(usize);
impl DensityBatchBudget {
    /// Checks the closed `1..=MAX_DENSITY_BATCH_WORK` domain.
    pub fn new(units: usize) -> Result<Self, DensitySampleError> {
        check_budget(units, MAX_DENSITY_BATCH_WORK)?;
        Ok(Self(units))
    }
    /// Chosen aggregate allowance.
    pub const fn units(self) -> usize {
        self.0
    }
}

fn check_budget(units: usize, maximum: usize) -> Result<(), DensitySampleError> {
    if units == 0 || units > maximum {
        return Err(DensitySampleError::admission(
            DensitySampleErrorKind::InvalidBudget { units, maximum },
        ));
    }
    Ok(())
}

/// Six climate dimensions in the original `Climate.Sampler` evaluation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ClimateAxis {
    /// Temperature.
    Temperature,
    /// Humidity, originally the vegetation router field.
    Humidity,
    /// Continentalness, originally the continents router field.
    Continentalness,
    /// Erosion.
    Erosion,
    /// Depth.
    Depth,
    /// Weirdness, originally the ridges router field.
    Weirdness,
}
impl ClimateAxis {
    /// Original ordered climate axes.
    pub const ALL: [Self; 6] = [
        Self::Temperature,
        Self::Humidity,
        Self::Continentalness,
        Self::Erosion,
        Self::Depth,
        Self::Weirdness,
    ];
    /// Original climate dimension name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Temperature => "temperature",
            Self::Humidity => "humidity",
            Self::Continentalness => "continentalness",
            Self::Erosion => "erosion",
            Self::Depth => "depth",
            Self::Weirdness => "weirdness",
        }
    }
}

/// Original six f32 climate values and their Java-long quantizations.
/// This is not a biome lookup, spawn finder or climate parameter search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClimateSample {
    values: [f32; 6],
    quantized: [i64; 6],
}
impl ClimateSample {
    /// The six f32 values in [`ClimateAxis::ALL`] order.
    pub const fn values(self) -> [f32; 6] {
        self.values
    }
    /// Java `(long)(value * 10000.0F)` results in the same order.
    pub const fn quantized(self) -> [i64; 6] {
        self.quantized
    }
    /// One original f32 climate value.
    pub const fn value(self, axis: ClimateAxis) -> f32 {
        self.values[axis as usize]
    }
    /// One original quantized climate coordinate.
    pub const fn quantized_value(self, axis: ClimateAxis) -> i64 {
        self.quantized[axis as usize]
    }
}

/// Checked quart coordinates for the actual original `Climate.Sampler` API.
/// Each axis is multiplied by four before sampling; no shift wrap is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimateQuartPos {
    x: i32,
    y: i32,
    z: i32,
}
impl ClimateQuartPos {
    /// Checks quart-to-block conversion before any i32 cast or sample work.
    pub fn new(x: i32, y: i32, z: i32) -> Result<Self, DensitySampleError> {
        for (axis, coordinate, min, max) in [
            (
                "x",
                x,
                -MAX_DENSITY_BLOCK_COORDINATE,
                MAX_DENSITY_BLOCK_COORDINATE,
            ),
            ("y", y, i32::MIN, i32::MAX),
            (
                "z",
                z,
                -MAX_DENSITY_BLOCK_COORDINATE,
                MAX_DENSITY_BLOCK_COORDINATE,
            ),
        ] {
            let value = i64::from(coordinate) * 4;
            if value < i64::from(min) || value > i64::from(max) {
                return Err(DensitySampleError::admission(
                    DensitySampleErrorKind::PositionOutOfRange { axis, value },
                ));
            }
        }
        Ok(Self { x, y, z })
    }
    /// Floors each checked block axis with arithmetic shift, including negatives.
    /// Sampling this position uses the enclosing quart's origin block.
    pub fn from_block(pos: BlockPos) -> Result<Self, DensitySampleError> {
        check_position(pos)?;
        Self::new(pos.x() >> 2, pos.y() >> 2, pos.z() >> 2)
    }
    /// Original quart-aligned absolute sample block.
    pub const fn block_origin(self) -> BlockPos {
        BlockPos::new(self.x << 2, self.y << 2, self.z << 2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Field(RouterField),
    Definition(DensityFunctionResourceId),
}

/// A checked-resource scalar request in an explicitly bounded ordered batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DensityRequest {
    pub(super) target: Target,
    pub(super) position: BlockPos,
}
impl DensityRequest {
    /// Requests one original router field at an absolute block position.
    pub const fn field(field: RouterField, pos: BlockPos) -> Self {
        Self {
            target: Target::Field(field),
            position: pos,
        }
    }
    /// Requests a builtin density in this router's explicit settings/seed context.
    pub const fn definition(id: DensityFunctionResourceId, pos: BlockPos) -> Self {
        Self {
            target: Target::Definition(id),
            position: pos,
        }
    }
}

/// Classified binding error; no incomplete seed graph becomes publicly visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DensityBindErrorKind {
    /// Original noise configuration failed.
    Noise(NoiseError),
    /// The fully checked local spline could not be reconstructed.
    Spline(SplineBuildError),
    /// Seed preparation encountered an unsupported random request.
    Random(crate::random::RandomError),
    /// A private transformed graph violated its checked template contract.
    InvalidGraph,
    /// An immutable transformed view exceeded an original graph cap.
    Limit { count: usize, maximum: usize },
}

/// Original resource and classified seed binding rejection.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("density seed binding of {resource} failed: {kind:?}")]
pub struct DensityBindError {
    pub(super) resource: &'static str,
    pub(super) kind: DensityBindErrorKind,
}
impl DensityBindError {
    /// Original resource whose construction failed.
    pub const fn resource(&self) -> &'static str {
        self.resource
    }
    /// Classified construction failure.
    pub const fn kind(&self) -> &DensityBindErrorKind {
        &self.kind
    }
}

/// Classified scalar rejection; a replacement terrain density is never supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DensitySampleErrorKind {
    /// The explicit work allowance is zero or exceeds its API cap.
    InvalidBudget { units: usize, maximum: usize },
    /// A batch is empty or exceeds the bounded request count.
    InvalidBatchSize { count: usize, maximum: usize },
    /// A coordinate is outside the explicit point/quarter conversion domain.
    PositionOutOfRange { axis: &'static str, value: i64 },
    /// The next operation was stopped before execution.
    WorkExceeded { limit: usize, used: usize },
    /// The concrete checked noise consumer rejected its numerical inputs/result.
    Noise(NoiseError),
    /// An actually requested density node completed with NaN or infinity.
    NonFiniteDensity { bits: u64 },
    /// An actual spline/climate f64-to-f32 conversion was nonfinite.
    NonFiniteCoordinate { bits: u32 },
    /// An actually requested local spline node completed with NaN or infinity.
    NonFiniteSpline { bits: u32 },
    /// A private bound graph violated its construction invariant.
    InvalidGraph,
}

/// Source, target, position and optional batch index of a scalar failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("density sample of {resource}/{field} failed: {kind:?}")]
pub struct DensitySampleError {
    pub(super) resource: &'static str,
    pub(super) field: &'static str,
    pub(super) position: Option<BlockPos>,
    pub(super) batch_index: Option<usize>,
    pub(super) kind: DensitySampleErrorKind,
}
impl DensitySampleError {
    fn admission(kind: DensitySampleErrorKind) -> Self {
        Self {
            resource: "density_admission",
            field: "$",
            position: None,
            batch_index: None,
            kind,
        }
    }
    /// Original source resource, or `density_admission` for pre-sample checks.
    pub const fn resource(&self) -> &'static str {
        self.resource
    }
    /// Requested router/climate field or density name.
    pub const fn field(&self) -> &'static str {
        self.field
    }
    /// Absolute sample position when execution had a selected target.
    pub const fn position(&self) -> Option<BlockPos> {
        self.position
    }
    /// First failing ordered batch request; later requests were not sampled.
    pub const fn batch_index(&self) -> Option<usize> {
        self.batch_index
    }
    /// Classified rejection.
    pub const fn kind(&self) -> &DensitySampleErrorKind {
        &self.kind
    }
}

pub(super) fn check_position(pos: BlockPos) -> Result<(), DensitySampleError> {
    for (axis, value) in [("x", pos.x()), ("z", pos.z())] {
        if !(-MAX_DENSITY_BLOCK_COORDINATE..=MAX_DENSITY_BLOCK_COORDINATE).contains(&value) {
            let mut error =
                DensitySampleError::admission(DensitySampleErrorKind::PositionOutOfRange {
                    axis,
                    value: i64::from(value),
                });
            error.position = Some(pos);
            return Err(error);
        }
    }
    Ok(())
}

/// Immutable original seed bindings and separate seed-router/climate views.
/// Point markers use the empty-Blender single-point context, not chunk caches.
#[derive(Debug, Clone)]
pub struct SeededRouter {
    pub(super) data: Arc<BoundRouter>,
}
impl RouterTemplate {
    /// Binds actual original noises and reconstructs both original visitor views.
    pub fn bind_seed(&self, seed: WorldSeed) -> Result<SeededRouter, DensityBindError> {
        super::bind::bind(self, seed)
    }
}
impl SeededRouter {
    /// Explicitly chosen unchanged signed64 seed bits.
    pub fn seed(&self) -> WorldSeed {
        self.data.seed
    }
    /// Original settings selecting this router's modern/legacy wiring context.
    pub fn resource(&self) -> NoiseSettingsResourceId {
        self.data.resource
    }
    /// Original seeded router-field diagnostics, never enforced sample clamps.
    pub fn field_bounds(&self, field: RouterField) -> DensityBounds {
        self.data.router.nodes[self.data.fields[field as usize]].bounds
    }
    /// Original named-density diagnostics in this explicit settings context.
    pub fn definition_bounds(&self, id: DensityFunctionResourceId) -> DensityBounds {
        self.data.router.nodes[self.data.definitions[&id]].bounds
    }
    /// Reconstructed diagnostics after the actual separate climate visitor.
    pub fn climate_bounds(&self, axis: ClimateAxis) -> DensityBounds {
        self.data.climate.nodes[self.data.climate_fields[axis as usize]].bounds
    }
    /// Samples exactly the selected absolute block, sharing one checked allowance.
    pub fn sample_field(
        &self,
        field: RouterField,
        pos: BlockPos,
        work: DensityWorkBudget,
    ) -> Result<f64, DensitySampleError> {
        self.sample_request(
            DensityRequest::field(field, pos),
            &mut SampleWork::new(work.0),
        )
    }
    /// Samples a named builtin in this router's explicit settings/seed context.
    pub fn sample_definition(
        &self,
        id: DensityFunctionResourceId,
        pos: BlockPos,
        work: DensityWorkBudget,
    ) -> Result<f64, DensitySampleError> {
        self.sample_request(
            DensityRequest::definition(id, pos),
            &mut SampleWork::new(work.0),
        )
    }
    /// Samples the actual climate view at this exact, potentially unaligned block.
    pub fn sample_climate_at_block(
        &self,
        pos: BlockPos,
        work: DensityWorkBudget,
    ) -> Result<ClimateSample, DensitySampleError> {
        check_position(pos)?;
        let mut meter = SampleWork::new(work.0);
        let mut values = [0.0; 6];
        let mut quantized = [0; 6];
        for axis in ClimateAxis::ALL {
            let value = evaluate::sample(
                &self.data,
                &self.data.climate,
                self.data.climate_fields[axis as usize],
                pos,
                &mut meter,
                axis.name(),
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
                });
            }
            values[axis as usize] = value;
            quantized[axis as usize] = quantize(value);
        }
        Ok(ClimateSample { values, quantized })
    }
    /// Samples the original quart-aligned block context of `Climate.Sampler`.
    pub fn sample_climate(
        &self,
        pos: ClimateQuartPos,
        work: DensityWorkBudget,
    ) -> Result<ClimateSample, DensitySampleError> {
        self.sample_climate_at_block(pos.block_origin(), work)
    }
    /// Evaluates ordered bounded requests under one aggregate meter. Results
    /// become observable only after every request succeeds; no partial output.
    pub fn sample_batch(
        &self,
        requests: &[DensityRequest],
        work: DensityBatchBudget,
    ) -> Result<Box<[f64]>, DensitySampleError> {
        if requests.is_empty() || requests.len() > MAX_DENSITY_BATCH_REQUESTS {
            return Err(DensitySampleError::admission(
                DensitySampleErrorKind::InvalidBatchSize {
                    count: requests.len(),
                    maximum: MAX_DENSITY_BATCH_REQUESTS,
                },
            ));
        }
        for (index, request) in requests.iter().enumerate() {
            check_position(request.position).map_err(|mut error| {
                error.batch_index = Some(index);
                error
            })?;
        }
        let mut meter = SampleWork::new(work.0);
        let mut result = Vec::with_capacity(requests.len());
        for (index, request) in requests.iter().enumerate() {
            let value = self
                .sample_request(*request, &mut meter)
                .map_err(|mut error| {
                    error.batch_index = Some(index);
                    error
                })?;
            result.push(value);
        }
        Ok(result.into_boxed_slice())
    }

    fn sample_request(
        &self,
        request: DensityRequest,
        work: &mut SampleWork,
    ) -> Result<f64, DensitySampleError> {
        check_position(request.position)?;
        let (root, field) = match request.target {
            Target::Field(field) => (self.data.fields[field as usize], field.name()),
            Target::Definition(id) => (self.data.definitions[&id], id.resource_location()),
        };
        evaluate::sample(
            &self.data,
            &self.data.router,
            root,
            request.position,
            work,
            field,
        )
    }
}

pub(super) fn quantize(value: f32) -> i64 {
    (value * 10_000.0_f32) as i64
}

#[cfg(test)]
mod tests;
