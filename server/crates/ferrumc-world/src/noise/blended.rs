use ferrumc_math::{BlockPos, Vec3};

use super::ConsumerNoiseCoreError;
use super::{finite, lerp, wrap_unchecked, NoiseError, PerlinNoise};
use crate::random::WorldgenRandomSource;
use crate::sample_work::SampleWork;

/// Explicit original blended-noise codec parameters; no default terrain profile.
/// The original Java testing constructor accepts values outside these ranges;
/// this checked Rust API deliberately supports the serialized codec domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendedNoiseParameters {
    xz_scale: f64,
    y_scale: f64,
    xz_factor: f64,
    y_factor: f64,
    smear_scale_multiplier: f64,
}

impl BlendedNoiseParameters {
    /// Validates four finite parameters in `0.001..=1000` and smear in `1..=8`.
    /// Validation allocates nothing and never consumes a random stream.
    pub fn new(
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear_scale_multiplier: f64,
    ) -> Result<Self, NoiseError> {
        for (name, value) in [
            ("xz_scale", xz_scale),
            ("y_scale", y_scale),
            ("xz_factor", xz_factor),
            ("y_factor", y_factor),
        ] {
            if !value.is_finite() || !(0.001..=1000.0).contains(&value) {
                return Err(NoiseError::InvalidBlendedParameter(name));
            }
        }
        if !smear_scale_multiplier.is_finite() || !(1.0..=8.0).contains(&smear_scale_multiplier) {
            return Err(NoiseError::InvalidBlendedParameter(
                "smear_scale_multiplier",
            ));
        }
        Ok(Self {
            xz_scale,
            y_scale,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
        })
    }
    /// Returns the horizontal limit-stack scale.
    pub const fn xz_scale(self) -> f64 {
        self.xz_scale
    }
    /// Returns the vertical limit-stack scale.
    pub const fn y_scale(self) -> f64 {
        self.y_scale
    }
    /// Returns the horizontal main-stack divisor.
    pub const fn xz_factor(self) -> f64 {
        self.xz_factor
    }
    /// Returns the vertical main-stack divisor.
    pub const fn y_factor(self) -> f64 {
        self.y_factor
    }
    /// Returns the original vertical smear multiplier.
    pub const fn smear_scale_multiplier(self) -> f64 {
        self.smear_scale_multiplier
    }
}

/// Original min/max/main legacy stacks used by old blended density noise.
/// This is a pure sampler, not a density router or a terrain generator.
#[derive(Debug, Clone)]
pub struct BlendedNoise {
    min_limit_noise: PerlinNoise,
    max_limit_noise: PerlinNoise,
    main_noise: PerlinNoise,
    parameters: BlendedNoiseParameters,
    xz_multiplier: f64,
    y_multiplier: f64,
    max_value: f64,
}

impl BlendedNoise {
    pub(crate) fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.min_limit_noise.retained_bytes()
            + self.max_limit_noise.retained_bytes()
            + self.main_noise.retained_bytes()
            - 3 * std::mem::size_of::<PerlinNoise>()
    }
    #[cfg(test)]
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
            min_limit_noise: self.min_limit_noise.paper_build60_basis_core(work)?,
            max_limit_noise: self.max_limit_noise.paper_build60_basis_core(work)?,
            main_noise: self.main_noise.paper_build60_basis_core(work)?,
            parameters: self.parameters,
            xz_multiplier: self.xz_multiplier,
            y_multiplier: self.y_multiplier,
            max_value: self.max_value,
        })
    }
    /// Builds original 16/16/8-octave legacy stacks from an explicit RNG.
    /// The validated codec domain guarantees finite derived factors; returned
    /// constructor errors preserve the caller's entire random state.
    pub fn new(
        random: &mut WorldgenRandomSource,
        parameters: BlendedNoiseParameters,
    ) -> Result<Self, NoiseError> {
        let xz_multiplier = 684.412 * parameters.xz_scale;
        let y_multiplier = 684.412 * parameters.y_scale;
        let mut candidate = *random;
        let min_limit_noise = PerlinNoise::legacy(&mut candidate, -15, &[1.0; 16])?;
        let max_limit_noise = PerlinNoise::legacy(&mut candidate, -15, &[1.0; 16])?;
        let main_noise = PerlinNoise::legacy(&mut candidate, -7, &[1.0; 8])?;
        let max_value = min_limit_noise.max_broken_value(y_multiplier)?;
        *random = candidate;
        Ok(Self {
            min_limit_noise,
            max_limit_noise,
            main_noise,
            parameters,
            xz_multiplier,
            y_multiplier,
            max_value,
        })
    }

    /// Constructs new stacks with these parameters and an explicitly supplied RNG.
    /// Existing stacks stay untouched; only the supplied parent is consumed.
    pub fn with_new_random(&self, random: &mut WorldgenRandomSource) -> Result<Self, NoiseError> {
        Self::new(random, self.parameters)
    }
    /// Returns the checked codec parameters retained for reseeding.
    pub const fn parameters(&self) -> BlendedNoiseParameters {
        self.parameters
    }
    /// Borrows the sixteen-octave minimum-limit stack for exact diagnostics.
    pub const fn min_limit_noise(&self) -> &PerlinNoise {
        &self.min_limit_noise
    }
    /// Borrows the sixteen-octave maximum-limit stack for exact diagnostics.
    pub const fn max_limit_noise(&self) -> &PerlinNoise {
        &self.max_limit_noise
    }
    /// Borrows the eight-octave interpolation stack for exact diagnostics.
    pub const fn main_noise(&self) -> &PerlinNoise {
        &self.main_noise
    }
    /// Returns the original minimum-value diagnostic.
    pub const fn min_value(&self) -> f64 {
        -self.max_value
    }
    /// Returns the original maximum-value diagnostic, including its legacy units.
    pub const fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Samples the original block-coordinate algorithm across the full i32 domain.
    /// Internal scaled vertical values intentionally bypass the public Improved
    /// input cap, preserving the original consumer's wrapping and quantization.
    /// Sampling allocates nothing, consumes no RNG, and performs no I/O.
    pub fn sample(&self, position: BlockPos) -> Result<f64, NoiseError> {
        finite(self.sample_with_blend(position).0)
    }

    pub(crate) fn sample_consumer_core(
        &self,
        position: BlockPos,
        work: &mut SampleWork,
    ) -> Result<f64, ConsumerNoiseCoreError> {
        let result = self.sample_with_blend_metered(position, &mut || work.charge_core())?;
        Ok(finite(result.0)?)
    }

    fn sample_with_blend(&self, position: BlockPos) -> (f64, f64) {
        match self
            .sample_with_blend_metered(position, &mut || Ok::<(), std::convert::Infallible>(()))
        {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }

    fn sample_with_blend_metered<E>(
        &self,
        position: BlockPos,
        charge: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(f64, f64), E> {
        let x = f64::from(position.x()) * self.xz_multiplier;
        let y = f64::from(position.y()) * self.y_multiplier;
        let z = f64::from(position.z()) * self.xz_multiplier;
        let main_x = x / self.parameters.xz_factor;
        let main_y = y / self.parameters.y_factor;
        let main_z = z / self.parameters.xz_factor;
        let smear = self.y_multiplier * self.parameters.smear_scale_multiplier;
        let main_smear = smear / self.parameters.y_factor;
        let mut interpolation_sum = 0.0;
        let mut frequency = 1.0;
        for index in 0..8 {
            charge()?;
            if let Some(noise) = self.main_noise.octave_noise(index) {
                interpolation_sum += noise.sample_unchecked(
                    Vec3::new(
                        wrap_unchecked(main_x * frequency),
                        wrap_unchecked(main_y * frequency),
                        wrap_unchecked(main_z * frequency),
                    ),
                    main_smear * frequency,
                    main_y * frequency,
                ) / frequency;
            }
            frequency /= 2.0;
        }
        let scaled_sum = interpolation_sum / 10.0;
        let shifted_sum = scaled_sum + 1.0;
        let blend = shifted_sum / 2.0;
        let use_max_only = blend >= 1.0;
        let use_min_only = blend <= 0.0;
        let mut min_value = 0.0;
        let mut max_value = 0.0;
        frequency = 1.0;
        for index in 0..16 {
            charge()?;
            let p = Vec3::new(
                wrap_unchecked(x * frequency),
                wrap_unchecked(y * frequency),
                wrap_unchecked(z * frequency),
            );
            let y_scale = smear * frequency;
            if !use_max_only {
                if let Some(noise) = self.min_limit_noise.octave_noise(index) {
                    min_value += noise.sample_unchecked(p, y_scale, y * frequency) / frequency;
                }
            }
            if !use_min_only {
                if let Some(noise) = self.max_limit_noise.octave_noise(index) {
                    max_value += noise.sample_unchecked(p, y_scale, y * frequency) / frequency;
                }
            }
            frequency /= 2.0;
        }
        let min = min_value / 512.0;
        let max = max_value / 512.0;
        Ok((clamped_lerp(min, max, blend) / 128.0, blend))
    }
}

fn clamped_lerp(min: f64, max: f64, blend: f64) -> f64 {
    if blend < 0.0 {
        min
    } else if blend > 1.0 {
        max
    } else {
        lerp(blend, min, max)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{random::RandomAlgorithm, WorldSeed};

    const ORIGINAL: &str = include_str!("../../tests/fixtures/composite_noise_1_21_8.tsv");

    fn float(bits: &str) -> f64 {
        f64::from_bits(u64::from_str_radix(bits, 16).unwrap())
    }

    #[test]
    fn original_goldens_exercise_both_clamps_and_interior_interpolation() {
        let mut samplers = BTreeMap::new();
        let mut branches = [0_usize; 3];
        for row in ORIGINAL.lines() {
            let f: Vec<_> = row.split('|').collect();
            if f[0] == "B" {
                let algorithm = if f[2] == "L" {
                    RandomAlgorithm::Legacy
                } else {
                    RandomAlgorithm::Xoroshiro128PlusPlus
                };
                let mut random =
                    WorldgenRandomSource::new(algorithm, WorldSeed::new(f[3].parse().unwrap()));
                let p: Vec<_> = f[4].split(',').map(float).collect();
                let parameters = BlendedNoiseParameters::new(p[0], p[1], p[2], p[3], p[4]).unwrap();
                samplers.insert(f[1], BlendedNoise::new(&mut random, parameters).unwrap());
            } else if f[0] == "BV" {
                let position = BlockPos::new(
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                    f[4].parse().unwrap(),
                );
                let (sample, blend) = samplers[f[1]].sample_with_blend(position);
                assert_eq!(sample.to_bits(), float(f[5]).to_bits());
                branches[if blend <= 0.0 {
                    0
                } else if blend >= 1.0 {
                    2
                } else {
                    1
                }] += 1;
            }
        }
        assert!(
            branches.iter().all(|count| *count > 0),
            "uncovered original clamp branches: {branches:?}"
        );
    }

    #[test]
    fn original_clamped_lerp_goldens_cover_signed_zero_and_adjacent_boundaries() {
        let mut records = 0;
        for row in ORIGINAL.lines() {
            let f: Vec<_> = row.split('|').collect();
            if f[0] == "CL" {
                assert_eq!(
                    clamped_lerp(float(f[1]), float(f[2]), float(f[3])).to_bits(),
                    float(f[4]).to_bits()
                );
                records += 1;
            }
        }
        assert_eq!(records, 30);
    }
}
