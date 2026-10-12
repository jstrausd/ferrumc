//! Original point arithmetic and ordered bounded density/spline/noise requests.

use ferrumc_math::{BlockPos, Vec3};

use super::bind::{java_max, java_min, BoundKind, BoundNode, BoundRouter, BoundView};
use super::graph::{Binary, NodeKind, Rarity, Shift, Unary, Unit};
use super::seeded::{DensitySampleError, DensitySampleErrorKind as ErrorKind};
use crate::noise::ConsumerNoiseCoreError;
use crate::sample_work::{ChargeFault, CoreError, SampleWork};
use crate::spline::SplineCoreError;

pub(super) type DensityCoreError = CoreError<DensitySampleError>;
impl From<DensitySampleError> for DensityCoreError {
    fn from(error: DensitySampleError) -> Self {
        Self::Error(error)
    }
}

/// Numerical reference belongs to the consumer context, never a global option.
#[derive(Clone, Copy)]
pub(super) enum EndRadialReference {
    Vanilla1_21_8,
    Paper1_21_8Build60,
}

/// Private hook for exclusively owned `NoiseChunk` caches. Point evaluation keeps
/// its original identity-marker behavior and unchanged meter charges.
pub(super) trait MarkerEvaluator {
    #[cfg(test)]
    fn declared_sample(
        &mut self,
        _root: usize,
        _pos: BlockPos,
    ) -> Option<Result<f64, DensitySampleError>> {
        None
    }
    fn end_reference(&self) -> EndRadialReference;
    fn normal_sampler<'a>(
        &'a self,
        data: &'a BoundRouter,
        index: usize,
    ) -> Option<&'a crate::noise::NormalNoise> {
        data.noises.get(index).map(|(_, noise)| noise)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The hook preserves the existing checked graph, position and shared-meter context."
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
    ) -> Result<f64, DensitySampleError>;
    #[expect(
        clippy::too_many_arguments,
        reason = "Preserves the complete existing marker context."
    )]
    fn sample_marker_core(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        root: usize,
        input: usize,
        pos: BlockPos,
        work: &mut SampleWork,
        field: &'static str,
    ) -> Result<f64, DensityCoreError> {
        self.sample_marker(data, view, root, input, pos, work, field)
            .map_err(CoreError::Error)
    }
}
struct PointEvaluator;
impl MarkerEvaluator for PointEvaluator {
    fn end_reference(&self) -> EndRadialReference {
        EndRadialReference::Vanilla1_21_8
    }
    fn sample_marker(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        _root: usize,
        input: usize,
        pos: BlockPos,
        work: &mut SampleWork,
        field: &'static str,
    ) -> Result<f64, DensitySampleError> {
        visit_with(data, view, input, pos, work, field, self)
    }
    fn sample_marker_core(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        _root: usize,
        input: usize,
        pos: BlockPos,
        work: &mut SampleWork,
        field: &'static str,
    ) -> Result<f64, DensityCoreError> {
        visit_with_core(data, view, input, pos, work, field, self)
    }
}

pub(super) fn sample(
    data: &BoundRouter,
    view: &BoundView,
    root: usize,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
) -> Result<f64, DensitySampleError> {
    visit_with(data, view, root, pos, work, field, &mut PointEvaluator)
}

pub(super) fn visit_with(
    data: &BoundRouter,
    view: &BoundView,
    root: usize,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
    evaluator: &mut impl MarkerEvaluator,
) -> Result<f64, DensitySampleError> {
    visit_with_core(data, view, root, pos, work, field, evaluator).map_err(CoreError::into_public)
}

#[expect(
    clippy::too_many_lines,
    reason = "One exhaustive dispatch keeps original request and arithmetic order auditable."
)]
pub(super) fn visit_with_core(
    data: &BoundRouter,
    view: &BoundView,
    mut root: usize,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
    evaluator: &mut impl MarkerEvaluator,
) -> Result<f64, DensityCoreError> {
    // Actual removed holders/markers are identity aliases, not compute calls.
    // Combined depth admission bounds this traversal without recursion.
    let mut aliases = 0;
    let node = loop {
        let node = view.nodes.get(root).ok_or_else(|| {
            error(
                data.resource.resource_location(),
                field,
                pos,
                ErrorKind::InvalidGraph,
            )
        })?;
        if let BoundKind::Alias(target) = node.kind {
            aliases += 1;
            if aliases > super::MAX_DENSITY_DEPTH {
                return Err(error(node.resource, field, pos, ErrorKind::InvalidGraph));
            }
            root = target;
        } else {
            break node;
        }
    };
    work.charge_core()
        .map_err(|source| budget(node, field, pos, source))?;
    #[cfg(test)]
    work.record_density(node.resource);
    #[cfg(test)]
    let declared = evaluator.declared_sample(root, pos).transpose()?;
    #[cfg(not(test))]
    let declared: Option<f64> = None;
    let value = if let Some(value) = declared {
        value
    } else {
        match &node.kind {
            BoundKind::Alias(_) => {
                return Err(error(node.resource, field, pos, ErrorKind::InvalidGraph))
            }
            BoundKind::MulOrAdd {
                operation,
                input,
                argument,
            } => {
                let value = visit_with_core(data, view, *input, pos, work, field, evaluator)?;
                match operation {
                    Binary::Add => value + argument,
                    Binary::Mul => value * argument,
                    _ => return Err(error(node.resource, field, pos, ErrorKind::InvalidGraph)),
                }
            }
            BoundKind::Original(kind) => match kind {
                NodeKind::Constant(value) => *value,
                NodeKind::Holder { target, .. } => {
                    visit_with_core(data, view, *target, pos, work, field, evaluator)?
                }
                NodeKind::Marker { input, .. } => {
                    evaluator.sample_marker_core(data, view, root, *input, pos, work, field)?
                }
                NodeKind::BlendDensity(input) => {
                    visit_with_core(data, view, *input, pos, work, field, evaluator)?
                }
                NodeKind::Binary {
                    operation,
                    left,
                    right,
                } => {
                    let a = visit_with_core(data, view, *left, pos, work, field, evaluator)?;
                    let right_bounds = view.nodes[*right].bounds;
                    match operation {
                        Binary::Add => {
                            a + visit_with_core(data, view, *right, pos, work, field, evaluator)?
                        }
                        Binary::Mul => {
                            if a == 0.0 {
                                0.0
                            } else {
                                a * visit_with_core(
                                    data, view, *right, pos, work, field, evaluator,
                                )?
                            }
                        }
                        Binary::Min => {
                            if a < right_bounds.min {
                                a
                            } else {
                                java_min(
                                    a,
                                    visit_with_core(
                                        data, view, *right, pos, work, field, evaluator,
                                    )?,
                                )
                            }
                        }
                        Binary::Max => {
                            if a > right_bounds.max {
                                a
                            } else {
                                java_max(
                                    a,
                                    visit_with_core(
                                        data, view, *right, pos, work, field, evaluator,
                                    )?,
                                )
                            }
                        }
                    }
                }
                NodeKind::Unary { operation, input } => unary(
                    *operation,
                    visit_with_core(data, view, *input, pos, work, field, evaluator)?,
                ),
                NodeKind::Clamp { input, min, max } => clamp(
                    visit_with_core(data, view, *input, pos, work, field, evaluator)?,
                    *min,
                    *max,
                ),
                NodeKind::RangeChoice {
                    input,
                    min,
                    max,
                    inside,
                    outside,
                } => {
                    let value = visit_with_core(data, view, *input, pos, work, field, evaluator)?;
                    visit_with_core(
                        data,
                        view,
                        if value >= *min && value < *max {
                            *inside
                        } else {
                            *outside
                        },
                        pos,
                        work,
                        field,
                        evaluator,
                    )?
                }
                NodeKind::Gradient {
                    from_y,
                    to_y,
                    from_value,
                    to_value,
                } => {
                    let delta = (f64::from(pos.y()) - f64::from(*from_y))
                        / (f64::from(*to_y) - f64::from(*from_y));
                    clamped_lerp(*from_value, *to_value, delta)
                }
                NodeKind::Noise { xz, y, .. } => normal_core(
                    data,
                    node,
                    Vec3::new(
                        f64::from(pos.x()) * xz,
                        f64::from(pos.y()) * y,
                        f64::from(pos.z()) * xz,
                    ),
                    pos,
                    work,
                    field,
                    evaluator,
                )?,
                NodeKind::Shift { shift, .. } => {
                    let (x, y, z) = match shift {
                        Shift::All => (pos.x(), pos.y(), pos.z()),
                        Shift::A => (pos.x(), 0, pos.z()),
                        Shift::B => (pos.z(), pos.x(), 0),
                    };
                    normal_core(
                        data,
                        node,
                        Vec3::new(
                            f64::from(x) * 0.25,
                            f64::from(y) * 0.25,
                            f64::from(z) * 0.25,
                        ),
                        pos,
                        work,
                        field,
                        evaluator,
                    )? * 4.0
                }
                NodeKind::ShiftedNoise {
                    shift_x,
                    shift_y,
                    shift_z,
                    xz,
                    y,
                    ..
                } => {
                    let x = f64::from(pos.x()) * xz
                        + visit_with_core(data, view, *shift_x, pos, work, field, evaluator)?;
                    let y = f64::from(pos.y()) * y
                        + visit_with_core(data, view, *shift_y, pos, work, field, evaluator)?;
                    let z = f64::from(pos.z()) * xz
                        + visit_with_core(data, view, *shift_z, pos, work, field, evaluator)?;
                    normal_core(data, node, Vec3::new(x, y, z), pos, work, field, evaluator)?
                }
                NodeKind::Weird { input, rarity, .. } => {
                    let value = visit_with_core(data, view, *input, pos, work, field, evaluator)?;
                    let rarity = rarity_value(*rarity, value);
                    rarity
                        * normal_core(
                            data,
                            node,
                            Vec3::new(
                                f64::from(pos.x()) / rarity,
                                f64::from(pos.y()) / rarity,
                                f64::from(pos.z()) / rarity,
                            ),
                            pos,
                            work,
                            field,
                            evaluator,
                        )?
                        .abs()
                }
                NodeKind::Blended { .. } => {
                    work.charge_core()
                        .map_err(|source| budget(node, field, pos, source))?;
                    let sampler = node
                        .blended
                        .as_ref()
                        .ok_or_else(|| error(node.resource, field, pos, ErrorKind::InvalidGraph))?;
                    sampler
                        .sample_consumer_core(pos, work)
                        .map_err(|source| noise_error(node, field, pos, source))?
                }
                NodeKind::Unit(unit) => match unit {
                    Unit::BlendAlpha => 1.0,
                    Unit::BlendOffset | Unit::Beardifier => 0.0,
                    Unit::EndIslands => end_islands_with_reference(
                        data,
                        node,
                        pos,
                        work,
                        field,
                        evaluator.end_reference(),
                    )?,
                },
                NodeKind::Spline(_) => {
                    let spline = node
                        .spline
                        .as_ref()
                        .ok_or_else(|| error(node.resource, field, pos, ErrorKind::InvalidGraph))?;
                    let value = spline
                        .spline
                        .sample_shared_core(work, &mut |coordinate, work| {
                            let id = spline
                                .coordinates
                                .get(coordinate.index() as usize)
                                .ok_or_else(|| {
                                    error(node.resource, field, pos, ErrorKind::InvalidGraph)
                                })?;
                            Ok(
                                visit_with_core(data, view, *id, pos, work, field, evaluator)?
                                    as f32,
                            )
                        })
                        .map_err(|source| match source {
                            SplineCoreError::CoordinateSource { source, .. } => source,
                            SplineCoreError::Work(fault) => budget(node, field, pos, fault),
                            SplineCoreError::NonFiniteCoordinate { bits, .. } => error(
                                node.resource,
                                field,
                                pos,
                                ErrorKind::NonFiniteCoordinate { bits },
                            ),
                            SplineCoreError::NonFiniteResult { bits, .. } => error(
                                node.resource,
                                field,
                                pos,
                                ErrorKind::NonFiniteSpline { bits },
                            ),
                        })?;
                    f64::from(value)
                }
                NodeKind::Coordinate { .. }
                | NodeKind::SplineConstant(_)
                | NodeKind::SplineMultipoint { .. } => {
                    return Err(error(node.resource, field, pos, ErrorKind::InvalidGraph))
                }
            },
        }
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error(
            node.resource,
            field,
            pos,
            ErrorKind::NonFiniteDensity {
                bits: value.to_bits(),
            },
        ))
    }
}

pub(super) fn normal_core(
    data: &BoundRouter,
    node: &BoundNode,
    position: Vec3,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
    evaluator: &impl MarkerEvaluator,
) -> Result<f64, DensityCoreError> {
    work.charge_core()
        .map_err(|source| budget(node, field, pos, source))?;
    let index = node
        .noise
        .ok_or_else(|| error(node.resource, field, pos, ErrorKind::InvalidGraph))?;
    evaluator
        .normal_sampler(data, index)
        .ok_or_else(|| error(node.resource, field, pos, ErrorKind::InvalidGraph))?
        .sample_consumer_core(position, work)
        .map_err(|source| noise_error(node, field, pos, source))
}

fn noise_error(
    node: &BoundNode,
    field: &'static str,
    pos: BlockPos,
    source: ConsumerNoiseCoreError,
) -> DensityCoreError {
    match source {
        ConsumerNoiseCoreError::Noise(source) => {
            error(node.resource, field, pos, ErrorKind::Noise(source))
        }
        ConsumerNoiseCoreError::Work(source) => budget(node, field, pos, source),
    }
}
fn budget(
    node: &BoundNode,
    field: &'static str,
    pos: BlockPos,
    source: ChargeFault,
) -> DensityCoreError {
    match source {
        ChargeFault::Public { limit, used } => error(
            node.resource,
            field,
            pos,
            ErrorKind::WorkExceeded { limit, used },
        ),
        fault @ ChargeFault::Carver { .. } => CoreError::Work(fault),
    }
}
fn error(
    resource: &'static str,
    field: &'static str,
    pos: BlockPos,
    kind: ErrorKind,
) -> DensityCoreError {
    DensitySampleError {
        resource,
        field,
        position: Some(pos),
        batch_index: None,
        kind,
    }
    .into()
}

pub(super) fn unary(operation: Unary, value: f64) -> f64 {
    match operation {
        Unary::Abs => value.abs(),
        Unary::Square => value * value,
        Unary::Cube => value * value * value,
        Unary::HalfNegative => {
            if value > 0.0 {
                value
            } else {
                value * 0.5
            }
        }
        Unary::QuarterNegative => {
            if value > 0.0 {
                value
            } else {
                value * 0.25
            }
        }
        Unary::Squeeze => {
            let value = clamp(value, -1.0, 1.0);
            value / 2.0 - value * value * value / 24.0
        }
    }
}

pub(super) fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else {
        java_min(value, max)
    }
}
fn clamped_lerp(start: f64, end: f64, delta: f64) -> f64 {
    if delta < 0.0 {
        start
    } else if delta > 1.0 {
        end
    } else {
        start + delta * (end - start)
    }
}

pub(super) fn rarity_value(rarity: Rarity, value: f64) -> f64 {
    match rarity {
        Rarity::Type1 => {
            if value < -0.5 {
                0.75
            } else if value < 0.0 {
                1.0
            } else if value < 0.5 {
                1.5
            } else {
                2.0
            }
        }
        Rarity::Type2 => {
            if value < -0.75 {
                0.5
            } else if value < -0.5 {
                0.75
            } else if value < 0.5 {
                1.0
            } else if value < 0.75 {
                2.0
            } else {
                3.0
            }
        }
    }
}

fn end_islands_with_reference(
    data: &BoundRouter,
    node: &BoundNode,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
    reference: EndRadialReference,
) -> Result<f64, DensityCoreError> {
    let x = pos.x() / 8;
    let z = pos.z() / 8;
    let center_x = x / 2;
    let center_z = z / 2;
    let rem_x = x % 2;
    let rem_z = z % 2;
    // Vanilla's first distance wraps i32 before its float sqrt. Paper widens
    // this distance; actual Vanilla goldens cannot prove that separate branch.
    let distance = match reference {
        EndRadialReference::Vanilla1_21_8 => {
            x.wrapping_mul(x).wrapping_add(z.wrapping_mul(z)) as f32
        }
        EndRadialReference::Paper1_21_8Build60 => {
            (i64::from(x) * i64::from(x) + i64::from(z) * i64::from(z)) as f32
        }
    };
    let mut height = 100.0_f32 - java_sqrt(distance) * 8.0_f32;
    height = clamp_f32(height, -100.0, 80.0);
    for dx in -12..=12 {
        for dz in -12..=12 {
            work.charge_core()
                .map_err(|source| budget(node, field, pos, source))?;
            let nx = i64::from(center_x + dx);
            let nz = i64::from(center_z + dz);
            if nx * nx + nz * nz > 4096 {
                work.charge_core()
                    .map_err(|source| budget(node, field, pos, source))?;
                let sample = data
                    .end
                    .sample_2d(nx as f64, nz as f64)
                    .map_err(|source| error(node.resource, field, pos, ErrorKind::Noise(source)))?;
                if sample < f64::from(-0.9_f32) {
                    let scale = ((nx as f32).abs() * 3439.0_f32 + (nz as f32).abs() * 147.0_f32)
                        % 13.0_f32
                        + 9.0_f32;
                    let delta_x = (rem_x - dx * 2) as f32;
                    let delta_z = (rem_z - dz * 2) as f32;
                    let candidate =
                        100.0_f32 - java_sqrt(delta_x * delta_x + delta_z * delta_z) * scale;
                    height = java_max_f32(height, clamp_f32(candidate, -100.0, 80.0));
                }
            }
        }
    }
    Ok((f64::from(height) - 8.0) / 128.0)
}

fn java_sqrt(value: f32) -> f32 {
    f64::from(value).sqrt() as f32
}
fn java_min_f32(a: f32, b: f32) -> f32 {
    java_min(f64::from(a), f64::from(b)) as f32
}
fn java_max_f32(a: f32, b: f32) -> f32 {
    java_max(f64::from(a), f64::from(b)) as f32
}
fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else {
        java_min_f32(value, max)
    }
}
