//! Original point arithmetic and ordered bounded density/spline/noise requests.

use ferrumc_math::{BlockPos, Vec3};

use super::bind::{java_max, java_min, BoundKind, BoundNode, BoundRouter, BoundView};
use super::graph::{Binary, NodeKind, Rarity, Shift, Unary, Unit};
use super::seeded::{DensitySampleError, DensitySampleErrorKind as ErrorKind};
use crate::noise::ConsumerNoiseError;
use crate::sample_work::{SampleWork, SampleWorkExceeded};
use crate::spline::SplineSampleError;

pub(super) fn sample(
    data: &BoundRouter,
    view: &BoundView,
    root: usize,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
) -> Result<f64, DensitySampleError> {
    visit(data, view, root, pos, work, field)
}

#[expect(
    clippy::too_many_lines,
    reason = "One exhaustive dispatch keeps original request and arithmetic order auditable."
)]
fn visit(
    data: &BoundRouter,
    view: &BoundView,
    mut root: usize,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
) -> Result<f64, DensitySampleError> {
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
    work.charge()
        .map_err(|source| budget(node, field, pos, source))?;
    #[cfg(test)]
    work.record_density(node.resource);
    let value = match &node.kind {
        BoundKind::Alias(_) => {
            return Err(error(node.resource, field, pos, ErrorKind::InvalidGraph))
        }
        BoundKind::MulOrAdd {
            operation,
            input,
            argument,
        } => {
            let value = visit(data, view, *input, pos, work, field)?;
            match operation {
                Binary::Add => value + argument,
                Binary::Mul => value * argument,
                _ => return Err(error(node.resource, field, pos, ErrorKind::InvalidGraph)),
            }
        }
        BoundKind::Original(kind) => match kind {
            NodeKind::Constant(value) => *value,
            NodeKind::Holder { target, .. } => visit(data, view, *target, pos, work, field)?,
            NodeKind::Marker { input, .. } | NodeKind::BlendDensity(input) => {
                visit(data, view, *input, pos, work, field)?
            }
            NodeKind::Binary {
                operation,
                left,
                right,
            } => {
                let a = visit(data, view, *left, pos, work, field)?;
                let right_bounds = view.nodes[*right].bounds;
                match operation {
                    Binary::Add => a + visit(data, view, *right, pos, work, field)?,
                    Binary::Mul => {
                        if a == 0.0 {
                            0.0
                        } else {
                            a * visit(data, view, *right, pos, work, field)?
                        }
                    }
                    Binary::Min => {
                        if a < right_bounds.min {
                            a
                        } else {
                            java_min(a, visit(data, view, *right, pos, work, field)?)
                        }
                    }
                    Binary::Max => {
                        if a > right_bounds.max {
                            a
                        } else {
                            java_max(a, visit(data, view, *right, pos, work, field)?)
                        }
                    }
                }
            }
            NodeKind::Unary { operation, input } => {
                unary(*operation, visit(data, view, *input, pos, work, field)?)
            }
            NodeKind::Clamp { input, min, max } => {
                clamp(visit(data, view, *input, pos, work, field)?, *min, *max)
            }
            NodeKind::RangeChoice {
                input,
                min,
                max,
                inside,
                outside,
            } => {
                let value = visit(data, view, *input, pos, work, field)?;
                visit(
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
            NodeKind::Noise { xz, y, .. } => normal(
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
            )?,
            NodeKind::Shift { shift, .. } => {
                let (x, y, z) = match shift {
                    Shift::All => (pos.x(), pos.y(), pos.z()),
                    Shift::A => (pos.x(), 0, pos.z()),
                    Shift::B => (pos.z(), pos.x(), 0),
                };
                normal(
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
                let x = f64::from(pos.x()) * xz + visit(data, view, *shift_x, pos, work, field)?;
                let y = f64::from(pos.y()) * y + visit(data, view, *shift_y, pos, work, field)?;
                let z = f64::from(pos.z()) * xz + visit(data, view, *shift_z, pos, work, field)?;
                normal(data, node, Vec3::new(x, y, z), pos, work, field)?
            }
            NodeKind::Weird { input, rarity, .. } => {
                let value = visit(data, view, *input, pos, work, field)?;
                let rarity = rarity_value(*rarity, value);
                rarity
                    * normal(
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
                    )?
                    .abs()
            }
            NodeKind::Blended { .. } => {
                work.charge()
                    .map_err(|source| budget(node, field, pos, source))?;
                let sampler = node
                    .blended
                    .as_ref()
                    .ok_or_else(|| error(node.resource, field, pos, ErrorKind::InvalidGraph))?;
                sampler
                    .sample_consumer(pos, work)
                    .map_err(|source| noise_error(node, field, pos, source))?
            }
            NodeKind::Unit(unit) => match unit {
                Unit::BlendAlpha => 1.0,
                Unit::BlendOffset | Unit::Beardifier => 0.0,
                Unit::EndIslands => end_islands(data, node, pos, work, field)?,
            },
            NodeKind::Spline(_) => {
                let spline = node
                    .spline
                    .as_ref()
                    .ok_or_else(|| error(node.resource, field, pos, ErrorKind::InvalidGraph))?;
                let value = spline
                    .spline
                    .sample_shared(work, &mut |coordinate, work| {
                        let id = spline
                            .coordinates
                            .get(coordinate.index() as usize)
                            .ok_or_else(|| {
                                error(node.resource, field, pos, ErrorKind::InvalidGraph)
                            })?;
                        Ok(visit(data, view, *id, pos, work, field)? as f32)
                    })
                    .map_err(|source| match source {
                        SplineSampleError::CoordinateSource { source, .. } => source,
                        SplineSampleError::BudgetExceeded { limit } => error(
                            node.resource,
                            field,
                            pos,
                            ErrorKind::WorkExceeded {
                                limit,
                                used: work.used(),
                            },
                        ),
                        SplineSampleError::NonFiniteCoordinate { bits, .. } => error(
                            node.resource,
                            field,
                            pos,
                            ErrorKind::NonFiniteCoordinate { bits },
                        ),
                        SplineSampleError::NonFiniteResult { bits, .. } => error(
                            node.resource,
                            field,
                            pos,
                            ErrorKind::NonFiniteSpline { bits },
                        ),
                        SplineSampleError::InvalidBudget { .. } => {
                            error(node.resource, field, pos, ErrorKind::InvalidGraph)
                        }
                    })?;
                f64::from(value)
            }
            NodeKind::Coordinate { .. }
            | NodeKind::SplineConstant(_)
            | NodeKind::SplineMultipoint { .. } => {
                return Err(error(node.resource, field, pos, ErrorKind::InvalidGraph))
            }
        },
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

fn normal(
    data: &BoundRouter,
    node: &BoundNode,
    position: Vec3,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
) -> Result<f64, DensitySampleError> {
    work.charge()
        .map_err(|source| budget(node, field, pos, source))?;
    let index = node
        .noise
        .ok_or_else(|| error(node.resource, field, pos, ErrorKind::InvalidGraph))?;
    data.noises[index]
        .1
        .sample_consumer(position, work)
        .map_err(|source| noise_error(node, field, pos, source))
}

fn noise_error(
    node: &BoundNode,
    field: &'static str,
    pos: BlockPos,
    source: ConsumerNoiseError,
) -> DensitySampleError {
    match source {
        ConsumerNoiseError::Noise(source) => {
            error(node.resource, field, pos, ErrorKind::Noise(source))
        }
        ConsumerNoiseError::Work(source) => budget(node, field, pos, source),
    }
}
fn budget(
    node: &BoundNode,
    field: &'static str,
    pos: BlockPos,
    source: SampleWorkExceeded,
) -> DensitySampleError {
    error(
        node.resource,
        field,
        pos,
        ErrorKind::WorkExceeded {
            limit: source.limit,
            used: source.used,
        },
    )
}
fn error(
    resource: &'static str,
    field: &'static str,
    pos: BlockPos,
    kind: ErrorKind,
) -> DensitySampleError {
    DensitySampleError {
        resource,
        field,
        position: Some(pos),
        batch_index: None,
        kind,
    }
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

fn clamp(value: f64, min: f64, max: f64) -> f64 {
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

fn rarity_value(rarity: Rarity, value: f64) -> f64 {
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

fn end_islands(
    data: &BoundRouter,
    node: &BoundNode,
    pos: BlockPos,
    work: &mut SampleWork,
    field: &'static str,
) -> Result<f64, DensitySampleError> {
    let x = pos.x() / 8;
    let z = pos.z() / 8;
    let center_x = x / 2;
    let center_z = z / 2;
    let rem_x = x % 2;
    let rem_z = z % 2;
    // Vanilla's first distance wraps i32 before its float sqrt. Paper widens
    // this distance; actual Vanilla goldens cannot prove that separate branch.
    let distance = x.wrapping_mul(x).wrapping_add(z.wrapping_mul(z));
    let mut height = 100.0_f32 - java_sqrt(distance as f32) * 8.0_f32;
    height = clamp_f32(height, -100.0, 80.0);
    for dx in -12..=12 {
        for dz in -12..=12 {
            work.charge()
                .map_err(|source| budget(node, field, pos, source))?;
            let nx = i64::from(center_x + dx);
            let nz = i64::from(center_z + dz);
            if nx * nx + nz * nz > 4096 {
                work.charge()
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
