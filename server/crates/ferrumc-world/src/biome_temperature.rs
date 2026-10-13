//! Fixed builtin biome temperature and pure Paper 1.21.8 build 60 predicates.
//!
//! Construction validates all 65 complete Registry definitions on the stack.
//! The returned immutable value owns five fixed Simplex objects and no heap.
//! Queries support every signed block coordinate and sea level, with no world
//! seed, cache, RNG advancement or world mutation. Vanilla's retained public
//! temperature cache is deliberately outside this stateless contract.

use ferrumc_math::BlockPos;
use ferrumc_registry::biome::{biome_resources, BiomeResourceId};

use crate::noise::{NoiseError, SimplexNoise};
use crate::random::WorldgenRandomSource;
use crate::WorldSeed;

mod error;
mod json;
pub use error::{BiomeTemperatureError, BiomeTemperatureErrorKind, BiomeTemperatureLimit};
use BiomeTemperatureErrorKind as Kind;

const RECORDS: usize = 65;
const SOURCE_BYTES: usize = 338_695;
const JSON_NODES: usize = 14_466;
const CORE_DRAWS: u32 = 1310;
const CONSTRUCTION_WORK: u32 = 354_536;
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_RETAINED_BYTES: usize = 8 * 1024;
const MAX_SCRATCH_BYTES: usize = 1024 * 1024;

/// Checked admission allowance for the complete fixed construction census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeTemperatureBuildBudget(u32);

impl BiomeTemperatureBuildBudget {
    /// Admits 1..=1,048,576 units, before parsing or noise construction.
    pub fn new(allowance: u32) -> Result<Self, BiomeTemperatureError> {
        if !(1..=1_048_576).contains(&allowance) {
            return Err(BiomeTemperatureError::new(Kind::InvalidAllowance(
                allowance,
            )));
        }
        Ok(Self(allowance))
    }

    /// Returns the declared construction allowance, which is not a CPU meter.
    pub const fn allowance(self) -> u32 {
        self.0
    }
}

/// Original precipitation classification, including a biome's dry flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomePrecipitation {
    /// The selected biome has no precipitation, regardless of temperature.
    None,
    /// A precipitating biome is at least 0.15F.
    Rain,
    /// A precipitating biome is below 0.15F.
    Snow,
}

/// One finite original f32 temperature and its pure precipitation predicates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeTemperatureSample {
    temperature: f32,
    has_precipitation: bool,
}

impl BiomeTemperatureSample {
    /// Returns the original f32 value; its raw bits remain observable.
    pub const fn temperature(self) -> f32 {
        self.temperature
    }
    /// Original inclusive 0.15F rain threshold, independent of the dry flag.
    pub fn warm_enough_to_rain(self) -> bool {
        self.temperature >= 0.15_f32
    }
    /// Negation of the original rain threshold, independent of the dry flag.
    pub fn cold_enough_to_snow(self) -> bool {
        !self.warm_enough_to_rain()
    }
    /// Original strictly greater than 0.1F iceberg melt predicate.
    pub fn should_melt_iceberg_slightly(self) -> bool {
        self.temperature > 0.1_f32
    }
    /// Classifies the actual selected biome's precipitation and temperature.
    pub fn precipitation(self) -> BiomePrecipitation {
        if !self.has_precipitation {
            BiomePrecipitation::None
        } else if self.cold_enough_to_snow() {
            BiomePrecipitation::Snow
        } else {
            BiomePrecipitation::Rain
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Climate {
    temperature: f32,
    has_precipitation: bool,
    frozen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Record {
    biome: BiomeResourceId,
    climate: Climate,
}

/// Complete checked builtin climate table with immutable fixed temperature noises.
///
/// Compile, clone and query allocate nothing. Every sample uses false noise
/// offsets. Existing Simplex retains Vanilla offsets privately; Paper's 15
/// offset differences are unobservable here, and Paper constructor bit parity
/// is not claimed. No world seed or generic noise configuration is accepted.
///
/// ```compile_fail
/// use ferrumc_math::BlockPos;
/// use ferrumc_registry::biome::BiomeParametersResourceId;
/// use ferrumc_world::biome_temperature::BuiltinBiomeTemperatures;
/// fn wrong(table: &BuiltinBiomeTemperatures, preset: BiomeParametersResourceId) {
///     table.sample(preset, BlockPos::new(0, 0, 0), 63);
/// }
/// ```
/// ```compile_fail
/// use ferrumc_world::{WorldSeed, biome_temperature::BuiltinBiomeTemperatures};
/// BuiltinBiomeTemperatures::compile(WorldSeed::new(1234));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinBiomeTemperatures {
    records: [Record; RECORDS],
    temperature: FixedNoise<1>,
    frozen: FixedNoise<3>,
    info: FixedNoise<1>,
    construction_work: u32,
}

impl BuiltinBiomeTemperatures {
    /// Validates the complete fixed roster, census and native capacity before
    /// publishing an immutable table. Allowances below 354,536 fail before work.
    pub fn compile(budget: BiomeTemperatureBuildBudget) -> Result<Self, BiomeTemperatureError> {
        if budget.0 < CONSTRUCTION_WORK {
            return Err(BiomeTemperatureError::new(Kind::WorkExhausted {
                required: CONSTRUCTION_WORK,
                allowance: budget.0,
            }));
        }
        check_limit(
            std::mem::size_of::<Self>(),
            MAX_RETAINED_BYTES,
            BiomeTemperatureLimit::RetainedBytes,
        )?;
        check_limit(
            json::SCRATCH_BYTES,
            MAX_SCRATCH_BYTES,
            BiomeTemperatureLimit::ScratchBytes,
        )?;
        let mut resources = biome_resources();
        check_census(resources.len(), RECORDS)?;
        let first = resources.next().ok_or_else(|| {
            BiomeTemperatureError::new(Kind::CensusMismatch {
                observed: 0,
                expected: RECORDS,
            })
        })?;
        // Only an already checked ID initializes the staging table. Every slot
        // is overwritten before publication; this is never a Plains fallback.
        let mut records = [Record {
            biome: first,
            climate: Climate {
                temperature: 0.0,
                has_precipitation: false,
                frozen: false,
            },
        }; RECORDS];
        let mut bytes = 0;
        for (index, biome) in std::iter::once(first).chain(resources).enumerate() {
            check_limit(index + 1, RECORDS, BiomeTemperatureLimit::Records)?;
            if index > 0 && records[index - 1].biome >= biome {
                return Err(BiomeTemperatureError::new(Kind::InvalidResource).with_resource(biome));
            }
            let definition = biome.definition();
            json::check_source_bytes(definition.json().len())
                .map_err(|e| e.with_resource(biome))?;
            bytes += definition.json().len();
            check_limit(
                bytes,
                MAX_SOURCE_BYTES,
                BiomeTemperatureLimit::AggregateSourceBytes,
            )?;
            records[index].biome = biome;
        }
        check_census(bytes, SOURCE_BYTES)?;
        let mut nodes = 0;
        for record in &mut records {
            let source = record.biome.definition();
            let (climate, stats) =
                json::parse(source.json()).map_err(|e| e.with_resource(record.biome))?;
            check_census(stats.nodes, source.stats().nodes())
                .map_err(|e| e.with_resource(record.biome))?;
            check_census(stats.depth, source.stats().maximum_depth())
                .map_err(|e| e.with_resource(record.biome))?;
            check_census(source.json().len(), source.stats().bytes())
                .map_err(|e| e.with_resource(record.biome))?;
            record.climate = climate;
            nodes += stats.nodes;
        }
        check_census(nodes, JSON_NODES)?;
        check_census(
            bytes + nodes + RECORDS + CORE_DRAWS as usize,
            CONSTRUCTION_WORK as usize,
        )?;
        Ok(Self {
            records,
            temperature: FixedNoise::temperature(),
            frozen: FixedNoise::frozen(),
            info: FixedNoise::info(),
            construction_work: CONSTRUCTION_WORK,
        })
    }

    /// Declared spent admission census: bytes + nodes + records + original draws.
    /// The draw term is pinned original evidence, not runtime instrumentation.
    pub const fn construction_work(&self) -> u32 {
        self.construction_work
    }

    /// Computes stateless temperature for an existing checked biome identity.
    /// Every i32 axis and sea level is supported; sea + 17 uses Java wrapping.
    /// At most six `Simplex2D` calls occur, with no allocation or mutable state.
    pub fn sample(
        &self,
        biome: BiomeResourceId,
        position: BlockPos,
        sea_level: i32,
    ) -> Result<BiomeTemperatureSample, BiomeTemperatureError> {
        let index = self
            .records
            .binary_search_by_key(&biome, |r| r.biome)
            .map_err(|_| {
                BiomeTemperatureError::new(Kind::InvalidResource)
                    .at_query(biome, position, sea_level)
            })?;
        let climate = self.records[index].climate;
        let calculated = self
            .adjusted(climate, position, sea_level)
            .map_err(|error| {
                BiomeTemperatureError::new(Kind::Noise(error)).at_query(biome, position, sea_level)
            })?;
        if !calculated.is_finite() {
            return Err(BiomeTemperatureError::new(Kind::NonFiniteTemperature(
                calculated.to_bits(),
            ))
            .at_query(biome, position, sea_level));
        }
        Ok(BiomeTemperatureSample {
            temperature: calculated,
            has_precipitation: climate.has_precipitation,
        })
    }

    pub(crate) fn info_noise(&self, x: f64, z: f64) -> Result<f64, NoiseError> {
        self.info.sample(x, z)
    }

    fn modified(&self, climate: Climate, position: BlockPos) -> Result<f32, NoiseError> {
        if climate.frozen {
            let x = f64::from(position.x());
            let z = f64::from(position.z());
            let frozen = self.frozen.sample(x * 0.05, z * 0.05)? * 7.0;
            let info = self.info.sample(x * 0.2, z * 0.2)?;
            if frozen + info < 0.3 && self.info.sample(x * 0.09, z * 0.09)? < 0.8 {
                return Ok(0.2_f32);
            }
        }
        Ok(climate.temperature)
    }

    fn adjusted(
        &self,
        climate: Climate,
        position: BlockPos,
        sea_level: i32,
    ) -> Result<f32, NoiseError> {
        let base = self.modified(climate, position)?;
        let height = sea_level.wrapping_add(17);
        if position.y() > height {
            // Java converts and divides as FLOAT before widening. Likewise Y
            // and height enter the FLOAT expression separately, never Y-height.
            let noise = (self.temperature.sample(
                f64::from(position.x() as f32 / 8.0_f32),
                f64::from(position.z() as f32 / 8.0_f32),
            )? * 8.0) as f32;
            Ok(base - (noise + position.y() as f32 - height as f32) * 0.05_f32 / 40.0_f32)
        } else {
            Ok(base)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct FixedNoise<const N: usize> {
    levels: [SimplexNoise; N],
    input_factor: f64,
    value_factor: f64,
}

impl FixedNoise<1> {
    fn temperature() -> Self {
        Self::single(1234)
    }
    fn info() -> Self {
        Self::single(2345)
    }
    fn single(seed: i64) -> Self {
        Self::single_with_state(seed).0
    }
    fn single_with_state(seed: i64) -> (Self, WorldgenRandomSource) {
        let mut random = WorldgenRandomSource::legacy(WorldSeed::new(seed));
        let noise = Self {
            levels: [SimplexNoise::new(&mut random)],
            input_factor: 1.0,
            value_factor: 1.0,
        };
        (noise, random)
    }
}

impl FixedNoise<3> {
    fn frozen() -> Self {
        Self::frozen_with_state().0
    }
    fn frozen_with_state() -> (Self, WorldgenRandomSource) {
        let mut random = WorldgenRandomSource::legacy(WorldSeed::new(3456));
        let noise = Self {
            levels: std::array::from_fn(|_| SimplexNoise::new(&mut random)),
            input_factor: 1.0,
            value_factor: 1.0 / 7.0,
        };
        (noise, random)
    }
}

impl<const N: usize> FixedNoise<N> {
    fn sample(&self, x: f64, z: f64) -> Result<f64, NoiseError> {
        let mut result = 0.0;
        let mut input = self.input_factor;
        let mut value = self.value_factor;
        for level in &self.levels {
            result += level.sample_2d(x * input + 0.0, z * input + 0.0)? * value;
            input /= 2.0;
            value *= 2.0;
        }
        Ok(result)
    }
}

fn check_census(observed: usize, expected: usize) -> Result<(), BiomeTemperatureError> {
    if observed != expected {
        return Err(BiomeTemperatureError::new(Kind::CensusMismatch {
            observed,
            expected,
        }));
    }
    Ok(())
}

fn check_limit(
    count: usize,
    maximum: usize,
    limit: BiomeTemperatureLimit,
) -> Result<(), BiomeTemperatureError> {
    if count > maximum {
        return Err(BiomeTemperatureError::new(Kind::Limit {
            limit,
            count,
            maximum,
        }));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
