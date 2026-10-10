//! Original Minecraft Java 1.21.8 improved/Perlin noise building blocks.
//!
//! These pure samplers do not generate chunks. Explicit configuration and
//! coordinate limits reject unsupported inputs rather than substituting terrain.

use ferrumc_math::Vec3;

use crate::random::WorldgenRandomSource;

mod blended;
mod normal;
mod simplex;

pub use blended::{BlendedNoise, BlendedNoiseParameters};
pub use normal::{NormalNoise, MAX_NORMAL_NOISE_COORDINATE};
pub use simplex::SimplexNoise;

/// Maximum stored octave span, checked before allocation or RNG consumption.
pub const MAX_NOISE_OCTAVES: usize = 64;
/// Lowest supported octave exponent (a configuration bound, not a Vanilla cap).
pub const MIN_NOISE_OCTAVE: i32 = -64;
/// Highest supported octave exponent (a configuration bound, not a Vanilla cap).
pub const MAX_NOISE_OCTAVE: i32 = 32;
/// Coordinate magnitude accepted by public samplers; leaves room for offsets.
pub const MAX_NOISE_COORDINATE: f64 = 2_147_483_136.0;

/// Classified unsupported noise configuration or numerical input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NoiseError {
    /// The octave list must contain between one and the configured maximum.
    #[error("noise octave count {0} must be in 1..={MAX_NOISE_OCTAVES}")]
    OctaveCount(usize),
    /// The first or last exponent exceeds the supported finite factor range.
    #[error("noise octave {0} is outside {MIN_NOISE_OCTAVE}..={MAX_NOISE_OCTAVE}")]
    OctaveOutOfRange(i32),
    /// An amplitude was NaN or infinity.
    #[error("noise amplitude {0} is not finite")]
    NonFiniteAmplitude(usize),
    /// Original legacy initialization disables positive octaves, including gaps.
    #[error("positive legacy noise octaves are disabled")]
    PositiveLegacyOctave,
    /// Coordinates or vertical parameters were NaN, infinite or outside the cap.
    #[error("noise input is nonfinite or exceeds the coordinate magnitude cap")]
    InvalidInput,
    /// A finite caller configuration overflowed a computed bound or sample.
    #[error("noise arithmetic produced a nonfinite result")]
    NonFiniteResult,
    /// A blended-noise parameter falls outside its original codec range.
    #[error("blended-noise parameter {0} is nonfinite or outside its codec range")]
    InvalidBlendedParameter(&'static str),
}

// The final four entries intentionally duplicate particular original gradients.
const GRADIENTS: [[f64; 3]; 16] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0],
    [-1.0, 1.0, 0.0],
    [0.0, -1.0, -1.0],
];

/// A value and its analytic coordinate derivative; no finite-difference estimate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseSample {
    value: f64,
    derivative: Vec3,
}

impl NoiseSample {
    /// Returns the sampled scalar noise value.
    pub const fn value(self) -> f64 {
        self.value
    }
    /// Returns analytic `(dx, dy, dz)`; Java's array accumulation is caller policy.
    pub const fn derivative(self) -> Vec3 {
        self.derivative
    }
}

/// One original improved-noise permutation and its three seeded offsets.
#[derive(Debug, Clone, PartialEq)]
pub struct ImprovedNoise {
    permutation: [u8; 256],
    offsets: Vec3,
}

impl ImprovedNoise {
    /// Consumes three original doubles and the original 256 bounded shuffle draws.
    pub fn new(random: &mut WorldgenRandomSource) -> Self {
        let offsets = Vec3::new(
            random.next_double() * 256.0,
            random.next_double() * 256.0,
            random.next_double() * 256.0,
        );
        let mut permutation = core::array::from_fn(|index| index as u8);
        for index in 0..256 {
            let other = index + random.bounded_positive((256 - index) as u32) as usize;
            permutation.swap(index, other);
        }
        Self {
            permutation,
            offsets,
        }
    }

    /// Returns the seeded offsets without consuming random output.
    pub const fn offsets(&self) -> Vec3 {
        self.offsets
    }
    /// Borrows the complete original permutation for reproducible diagnostics.
    pub const fn permutation(&self) -> &[u8; 256] {
        &self.permutation
    }

    /// Samples ordinary three-dimensional improved noise at finite coordinates.
    pub fn sample(&self, position: Vec3) -> Result<f64, NoiseError> {
        self.sample_with_y(position, 0.0, 0.0)
    }

    /// Samples original vertical quantization: only gradient y is shifted,
    /// while interpolation uses the unshifted fractional y. Negative scales
    /// retain their original meaning; zero disables the shift.
    pub fn sample_with_y(
        &self,
        position: Vec3,
        y_scale: f64,
        y_max: f64,
    ) -> Result<f64, NoiseError> {
        validate_input(position, y_scale, y_max)?;
        finite(self.sample_unchecked(position, y_scale, y_max))
    }

    /// Samples a value and exact analytic derivative, without mutating an array.
    pub fn sample_with_derivative(&self, position: Vec3) -> Result<NoiseSample, NoiseError> {
        validate_input(position, 0.0, 0.0)?;
        let (indices, delta) = self.lattice(position);
        let fades = Vec3::new(fade(delta.x), fade(delta.y), fade(delta.z));
        let values = corner_values(indices, delta);
        let gradients = indices.map(|index| GRADIENTS[usize::from(index & 15)]);
        let dx = lerp3(fades, gradients.map(|gradient| gradient[0]))
            + fade_derivative(delta.x)
                * lerp2(
                    fades.y,
                    fades.z,
                    [
                        values[1] - values[0],
                        values[3] - values[2],
                        values[5] - values[4],
                        values[7] - values[6],
                    ],
                );
        let dy = lerp3(fades, gradients.map(|gradient| gradient[1]))
            + fade_derivative(delta.y)
                * lerp2(
                    fades.z,
                    fades.x,
                    [
                        values[2] - values[0],
                        values[6] - values[4],
                        values[3] - values[1],
                        values[7] - values[5],
                    ],
                );
        let dz = lerp3(fades, gradients.map(|gradient| gradient[2]))
            + fade_derivative(delta.z)
                * lerp2(
                    fades.x,
                    fades.y,
                    [
                        values[4] - values[0],
                        values[5] - values[1],
                        values[6] - values[2],
                        values[7] - values[3],
                    ],
                );
        Ok(NoiseSample {
            value: finite(lerp3(fades, values))?,
            derivative: Vec3::new(finite(dx)?, finite(dy)?, finite(dz)?),
        })
    }

    fn sample_unchecked(&self, position: Vec3, y_scale: f64, y_max: f64) -> f64 {
        let (indices, delta) = self.lattice(position);
        let shift = if y_scale == 0.0 {
            0.0
        } else {
            let capped = if y_max >= 0.0 && y_max < delta.y {
                y_max
            } else {
                delta.y
            };
            f64::from(java_floor(capped / y_scale + f64::from(1.0e-7_f32))) * y_scale
        };
        lerp3(
            Vec3::new(fade(delta.x), fade(delta.y), fade(delta.z)),
            corner_values(indices, Vec3::new(delta.x, delta.y - shift, delta.z)),
        )
    }

    fn lattice(&self, position: Vec3) -> ([u8; 8], Vec3) {
        let shifted = position + self.offsets;
        let grid = [
            java_floor(shifted.x),
            java_floor(shifted.y),
            java_floor(shifted.z),
        ];
        let delta = Vec3::new(
            shifted.x - f64::from(grid[0]),
            shifted.y - f64::from(grid[1]),
            shifted.z - f64::from(grid[2]),
        );
        let lookup = |index: i32| self.permutation[(index & 255) as usize];
        let x0 = i32::from(lookup(grid[0]));
        let x1 = i32::from(lookup(grid[0].wrapping_add(1)));
        let xy = [
            i32::from(lookup(x0.wrapping_add(grid[1]))),
            i32::from(lookup(x1.wrapping_add(grid[1]))),
            i32::from(lookup(x0.wrapping_add(grid[1]).wrapping_add(1))),
            i32::from(lookup(x1.wrapping_add(grid[1]).wrapping_add(1))),
        ];
        let indices = core::array::from_fn(|index| {
            lookup(
                xy[index & 3]
                    .wrapping_add(grid[2])
                    .wrapping_add(i32::from(index >= 4)),
            )
        });
        (indices, delta)
    }
}

#[derive(Debug, Clone)]
struct Octave {
    amplitude: f64,
    noise: Option<ImprovedNoise>,
}

/// A bounded original octave stack, initialized with modern or legacy rules.
#[derive(Debug, Clone)]
pub struct PerlinNoise {
    octaves: Vec<Octave>,
    first_octave: i32,
    input_factor: f64,
    value_factor: f64,
    max_value: f64,
}

impl PerlinNoise {
    /// Builds modern named-octave noise via one original positional fork.
    /// All configuration validation precedes allocation and parent consumption.
    pub fn new(
        random: &mut WorldgenRandomSource,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> Result<Self, NoiseError> {
        Self::build(random, first_octave, amplitudes, false)
    }

    /// Builds original legacy octave noise, including initial octave zero and
    /// 262 next-int steps for every skipped lower octave. Positive octaves fail.
    pub fn legacy(
        random: &mut WorldgenRandomSource,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> Result<Self, NoiseError> {
        Self::build(random, first_octave, amplitudes, true)
    }

    fn build(
        random: &mut WorldgenRandomSource,
        first_octave: i32,
        amplitudes: &[f64],
        legacy: bool,
    ) -> Result<Self, NoiseError> {
        validate_octaves(first_octave, amplitudes, legacy)?;
        let value_factor = octave_value_factor(amplitudes.len());
        let max_value = octave_max_value(amplitudes)?;
        let mut octaves: Vec<_> = amplitudes
            .iter()
            .map(|amplitude| Octave {
                amplitude: *amplitude,
                noise: None,
            })
            .collect();
        if legacy {
            let initial = ImprovedNoise::new(random);
            let zero_index = -first_octave;
            if let Some(octave) = octaves.get_mut(zero_index as usize) {
                if octave.amplitude != 0.0 {
                    octave.noise = Some(initial);
                }
            }
            for index in (0..zero_index as usize).rev() {
                if let Some(octave) = octaves
                    .get_mut(index)
                    .filter(|octave| octave.amplitude != 0.0)
                {
                    octave.noise = Some(ImprovedNoise::new(random));
                } else {
                    // 262 is original consumption, not an approximation of the
                    // possibly rejecting legacy permutation shuffle.
                    for _ in 0..262 {
                        let _ = random.next_int();
                    }
                }
            }
        } else {
            let factory = random.fork_positional();
            for (index, octave) in octaves.iter_mut().enumerate() {
                if octave.amplitude != 0.0 {
                    let name = format!("octave_{}", first_octave + i32::from(index as u8));
                    octave.noise = Some(ImprovedNoise::new(&mut factory.from_hash_of(&name)));
                }
            }
        }
        Ok(Self {
            octaves,
            first_octave,
            input_factor: power_of_two(first_octave),
            value_factor,
            max_value,
        })
    }

    /// Returns the first stored octave exponent.
    pub const fn first_octave(&self) -> i32 {
        self.first_octave
    }
    /// Iterates exact stored amplitudes, including zero gaps and signed zero.
    pub fn amplitudes(&self) -> impl ExactSizeIterator<Item = f64> + '_ {
        self.octaves.iter().map(|octave| octave.amplitude)
    }
    /// Borrows an octave by reverse array index: zero is the highest exponent.
    /// Gaps and out-of-range indices return `None` without panicking.
    pub fn octave_noise(&self, reverse_index: usize) -> Option<&ImprovedNoise> {
        self.octaves
            .get(
                self.octaves
                    .len()
                    .checked_sub(1)?
                    .checked_sub(reverse_index)?,
            )?
            .noise
            .as_ref()
    }
    /// Returns the original amplitude-weighted maximum-value diagnostic.
    pub const fn max_value(&self) -> f64 {
        self.max_value
    }
    /// Returns the original legacy bound for a finite vertical multiplier.
    pub fn max_broken_value(&self, y_multiplier: f64) -> Result<f64, NoiseError> {
        if !y_multiplier.is_finite() {
            return Err(NoiseError::InvalidInput);
        }
        let mut factor = self.value_factor;
        let mut value = 0.0;
        for octave in &self.octaves {
            if octave.noise.is_some() {
                value += octave.amplitude * (y_multiplier + 2.0) * factor;
            }
            factor /= 2.0;
        }
        finite(value)
    }
    /// Samples original Perlin noise with octave wrapping and no vertical shift.
    pub fn sample(&self, position: Vec3) -> Result<f64, NoiseError> {
        self.sample_with_y(position, 0.0, 0.0, false)
    }
    /// Samples original vertical/fixed-y behavior and amplitude accumulation order.
    pub fn sample_with_y(
        &self,
        position: Vec3,
        y_scale: f64,
        y_max: f64,
        fixed_y: bool,
    ) -> Result<f64, NoiseError> {
        validate_input(position, y_scale, y_max)?;
        let mut frequency = self.input_factor;
        let mut factor = self.value_factor;
        let mut value = 0.0;
        for octave in &self.octaves {
            if let Some(noise) = &octave.noise {
                let sample_position = Vec3::new(
                    wrap_unchecked(position.x * frequency),
                    if fixed_y {
                        -noise.offsets.y
                    } else {
                        wrap_unchecked(position.y * frequency)
                    },
                    wrap_unchecked(position.z * frequency),
                );
                value += octave.amplitude
                    * noise.sample_unchecked(
                        sample_position,
                        y_scale * frequency,
                        y_max * frequency,
                    )
                    * factor;
            }
            frequency *= 2.0;
            factor /= 2.0;
        }
        finite(value)
    }
}

/// Applies original Perlin coordinate wrapping to a finite bounded scalar.
pub fn wrap_noise_coordinate(value: f64) -> Result<f64, NoiseError> {
    if !value.is_finite() || value.abs() > MAX_NOISE_COORDINATE {
        return Err(NoiseError::InvalidInput);
    }
    Ok(wrap_unchecked(value))
}

fn validate_octaves(first: i32, amplitudes: &[f64], legacy: bool) -> Result<(), NoiseError> {
    if amplitudes.is_empty() || amplitudes.len() > MAX_NOISE_OCTAVES {
        return Err(NoiseError::OctaveCount(amplitudes.len()));
    }
    if !(MIN_NOISE_OCTAVE..=MAX_NOISE_OCTAVE).contains(&first) {
        return Err(NoiseError::OctaveOutOfRange(first));
    }
    let last = first + i32::from(amplitudes.len() as u8) - 1;
    if last > MAX_NOISE_OCTAVE {
        return Err(NoiseError::OctaveOutOfRange(last));
    }
    for (index, amplitude) in amplitudes.iter().enumerate() {
        if !amplitude.is_finite() {
            return Err(NoiseError::NonFiniteAmplitude(index));
        }
    }
    if legacy && last > 0 {
        return Err(NoiseError::PositiveLegacyOctave);
    }
    Ok(())
}

fn octave_value_factor(count: usize) -> f64 {
    power_of_two(i32::from(count as u8) - 1) / (power_of_two(i32::from(count as u8)) - 1.0)
}

// Both Perlin and Normal preflight the exact original diagnostic before any
// allocation or RNG mutation; Normal must also check the two-stack sum.
fn octave_max_value(amplitudes: &[f64]) -> Result<f64, NoiseError> {
    let mut factor = octave_value_factor(amplitudes.len());
    let mut value = 0.0;
    for &amplitude in amplitudes {
        if amplitude != 0.0 {
            value += amplitude * 2.0 * factor;
        }
        factor /= 2.0;
    }
    finite(value)
}

fn validate_input(position: Vec3, y_scale: f64, y_max: f64) -> Result<(), NoiseError> {
    if [position.x, position.y, position.z, y_scale, y_max]
        .iter()
        .all(|value| value.is_finite() && value.abs() <= MAX_NOISE_COORDINATE)
    {
        Ok(())
    } else {
        Err(NoiseError::InvalidInput)
    }
}
fn finite(value: f64) -> Result<f64, NoiseError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(NoiseError::NonFiniteResult)
    }
}
fn power_of_two(exponent: i32) -> f64 {
    f64::from_bits(((exponent + 1023) as u64) << 52)
}
fn java_floor(value: f64) -> i32 {
    let integer = value as i32;
    if value < f64::from(integer) {
        integer.wrapping_sub(1)
    } else {
        integer
    }
}
fn wrap_unchecked(value: f64) -> f64 {
    value - (value / 33_554_432.0 + 0.5).floor() * 33_554_432.0
}
fn fade(value: f64) -> f64 {
    value * value * value * (value * (value * 6.0 - 15.0) + 10.0)
}
fn fade_derivative(value: f64) -> f64 {
    30.0 * value * value * (value - 1.0) * (value - 1.0)
}
fn lerp(weight: f64, start: f64, end: f64) -> f64 {
    start + weight * (end - start)
}
fn lerp2(x: f64, y: f64, values: [f64; 4]) -> f64 {
    lerp(
        y,
        lerp(x, values[0], values[1]),
        lerp(x, values[2], values[3]),
    )
}
fn lerp3(weights: Vec3, values: [f64; 8]) -> f64 {
    lerp(
        weights.z,
        lerp2(
            weights.x,
            weights.y,
            [values[0], values[1], values[2], values[3]],
        ),
        lerp2(
            weights.x,
            weights.y,
            [values[4], values[5], values[6], values[7]],
        ),
    )
}
fn corner_values(indices: [u8; 8], delta: Vec3) -> [f64; 8] {
    core::array::from_fn(|index| {
        let gradient = GRADIENTS[usize::from(indices[index] & 15)];
        let x = delta.x - (index & 1) as f64;
        let y = delta.y - ((index >> 1) & 1) as f64;
        let z = delta.z - ((index >> 2) & 1) as f64;
        gradient[0] * x + gradient[1] * y + gradient[2] * z
    })
}
