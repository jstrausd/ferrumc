use ferrumc_math::Vec3;

use super::ConsumerNoiseCoreError;
#[cfg(test)]
use super::ConsumerNoiseError;
use super::{finite, octave_max_value, validate_input, validate_octaves, NoiseError, PerlinNoise};
use crate::random::WorldgenRandomSource;
use crate::sample_work::SampleWork;

const SECOND_INPUT_FACTOR: f64 = 1.018_126_888_217_522_7;

/// Normal-noise caller cap; its scaled second stack must also fit Perlin's cap.
/// This is a `FerrumC` input restriction, not an original Minecraft limit.
pub const MAX_NORMAL_NOISE_COORDINATE: f64 = super::MAX_NOISE_COORDINATE / SECOND_INPUT_FACTOR;

/// The original two-stack normal noise, with explicit modern/legacy selection.
/// No seed, octave, amplitude, or terrain profile is implicitly selected.
#[derive(Debug, Clone)]
pub struct NormalNoise {
    first: PerlinNoise,
    second: PerlinNoise,
    value_factor: f64,
    max_value: f64,
}

impl NormalNoise {
    pub(crate) fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.first.retained_bytes() + self.second.retained_bytes()
            - 2 * std::mem::size_of::<PerlinNoise>()
    }
    pub(crate) fn paper_build60_basis(
        &self,
        work: &mut SampleWork,
    ) -> Result<Self, crate::sample_work::SampleWorkExceeded> {
        self.paper_build60_basis_core(work)
            .map_err(crate::sample_work::ChargeFault::into_public)
    }
    pub(crate) fn paper_build60_basis_core(
        &self,
        work: &mut SampleWork,
    ) -> Result<Self, crate::sample_work::ChargeFault> {
        Ok(Self {
            first: self.first.paper_build60_basis_core(work)?,
            second: self.second.paper_build60_basis_core(work)?,
            value_factor: self.value_factor,
            max_value: self.max_value,
        })
    }
    /// Builds original modern named-octave stacks, consuming two positional forks.
    /// All predictable configuration errors precede allocation and RNG mutation.
    pub fn new(
        random: &mut WorldgenRandomSource,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> Result<Self, NoiseError> {
        Self::build(random, first_octave, amplitudes, false)
    }

    /// Builds the original legacy Nether-biome stacks, including legacy gaps.
    /// Positive last octaves fail even when the corresponding amplitude is zero.
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
        let mut first_nonzero = i32::MAX;
        let mut last_nonzero = i32::MIN;
        for (index, &amplitude) in amplitudes.iter().enumerate() {
            if amplitude != 0.0 {
                first_nonzero = first_nonzero.min(i32::from(index as u8));
                last_nonzero = last_nonzero.max(i32::from(index as u8));
            }
        }
        // An all-zero list intentionally uses Java's overflowing MIN-MAX = 1.
        let span = last_nonzero.wrapping_sub(first_nonzero);
        let deviation = 0.1 * (1.0 + 1.0 / f64::from(span + 1));
        let value_factor = 0.166_666_666_666_666_66 / deviation;
        let stack_max = octave_max_value(amplitudes)?;
        let max_value = finite((stack_max + stack_max) * value_factor)?;
        let mut candidate = *random;
        let build = if legacy {
            PerlinNoise::legacy
        } else {
            PerlinNoise::new
        };
        let first = build(&mut candidate, first_octave, amplitudes)?;
        let second = build(&mut candidate, first_octave, amplitudes)?;
        *random = candidate;
        Ok(Self {
            first,
            second,
            value_factor,
            max_value,
        })
    }

    /// Borrows the first stack, including its exact amplitudes and diagnostic state.
    pub const fn first(&self) -> &PerlinNoise {
        &self.first
    }
    /// Borrows the independently initialized second stack.
    pub const fn second(&self) -> &PerlinNoise {
        &self.second
    }
    /// Returns the original nonzero-octave-span normalization factor.
    pub const fn value_factor(&self) -> f64 {
        self.value_factor
    }
    /// Returns the original signed amplitude-weighted maximum-value diagnostic.
    pub const fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Samples in original evaluation order without allocation or random draws.
    /// Each input magnitude must be at most [`MAX_NORMAL_NOISE_COORDINATE`];
    /// the multiplied second coordinates are checked before either stack runs.
    pub fn sample(&self, position: Vec3) -> Result<f64, NoiseError> {
        if [position.x, position.y, position.z]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > MAX_NORMAL_NOISE_COORDINATE)
        {
            return Err(NoiseError::InvalidInput);
        }
        let second = Vec3::new(
            position.x * SECOND_INPUT_FACTOR,
            position.y * SECOND_INPUT_FACTOR,
            position.z * SECOND_INPUT_FACTOR,
        );
        validate_input(second, 0.0, 0.0)?;
        finite((self.first.sample(position)? + self.second.sample(second)?) * self.value_factor)
    }

    #[cfg(test)]
    pub(crate) fn sample_consumer(
        &self,
        position: Vec3,
        work: &mut SampleWork,
    ) -> Result<f64, ConsumerNoiseError> {
        self.sample_consumer_core(position, work)
            .map_err(ConsumerNoiseCoreError::into_public)
    }
    pub(crate) fn sample_consumer_core(
        &self,
        position: Vec3,
        work: &mut SampleWork,
    ) -> Result<f64, ConsumerNoiseCoreError> {
        let second = Vec3::new(
            position.x * SECOND_INPUT_FACTOR,
            position.y * SECOND_INPUT_FACTOR,
            position.z * SECOND_INPUT_FACTOR,
        );
        if ![
            position.x, position.y, position.z, second.x, second.y, second.z,
        ]
        .into_iter()
        .all(f64::is_finite)
        {
            return Err(NoiseError::InvalidInput.into());
        }
        Ok(finite(
            (self.first.sample_consumer_core(position, work)?
                + self.second.sample_consumer_core(second, work)?)
                * self.value_factor,
        )?)
    }
}
