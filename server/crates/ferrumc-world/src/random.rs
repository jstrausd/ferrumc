//! Owned, deterministic Minecraft Java 1.21.8 random primitives.
//!
//! Integer overflow, signed widening and generator consumption are part of the
//! worldgen contract. This module does not select seeds or implement Gaussian
//! sampling. Copies are independent snapshots, not shared random streams.

use ferrumc_math::BlockPos;
use md5::{Digest, Md5};

use crate::WorldSeed;

const GOLDEN_RATIO: u64 = 0x9e37_79b9_7f4a_7c15;
const SILVER_RATIO: u64 = 0x6a09_e667_f3bc_c909;
const LEGACY_MULTIPLIER: u64 = 25_214_903_917;
const LEGACY_MASK: u64 = (1 << 48) - 1;
/// Maximum explicit consumption per call; larger budgets fail before mutation.
pub const MAX_RANDOM_CONSUME_COUNT: u32 = 1 << 20;

/// The two original random algorithms used by Minecraft world generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomAlgorithm {
    /// Java-compatible 48-bit linear congruential source.
    Legacy,
    /// Minecraft's xoroshiro128++ source and seed-upgrade rules.
    Xoroshiro128PlusPlus,
}

/// Exact low/high halves of a 128-bit seed, preserving every input bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed128 {
    lo: u64,
    hi: u64,
}

impl Seed128 {
    /// Constructs seed halves from their original signed Java long values.
    pub const fn new(lo: i64, hi: i64) -> Self {
        Self {
            lo: lo as u64,
            hi: hi as u64,
        }
    }

    /// Upgrades one numeric seed using Minecraft's Stafford13 mixing.
    pub const fn upgrade(seed: WorldSeed) -> Self {
        let lo = seed.value() as u64 ^ SILVER_RATIO;
        Self {
            lo: mix_stafford13(lo),
            hi: mix_stafford13(lo.wrapping_add(GOLDEN_RATIO)),
        }
    }

    /// Returns the low seed half as a signed Java long.
    pub const fn low(self) -> i64 {
        self.lo.cast_signed()
    }

    /// Returns the high seed half as a signed Java long.
    pub const fn high(self) -> i64 {
        self.hi.cast_signed()
    }

    fn from_name(name: &str) -> Self {
        let digest = Md5::digest(name.as_bytes());
        let mut lo = [0; 8];
        let mut hi = [0; 8];
        lo.copy_from_slice(&digest[..8]);
        hi.copy_from_slice(&digest[8..]);
        Self {
            lo: u64::from_be_bytes(lo),
            hi: u64::from_be_bytes(hi),
        }
    }

    const fn xor(self, other: Self) -> Self {
        Self {
            lo: self.lo ^ other.lo,
            hi: self.hi ^ other.hi,
        }
    }
}

const fn mix_stafford13(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Legacy(u64),
    Xoroshiro(Seed128),
}

/// An exclusively owned original random stream; reseeding resets its state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldgenRandomSource {
    state: State,
}

/// Invalid caller quantity, rejected without consuming random output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RandomError {
    /// Java bounded-int sampling requires a strictly positive signed bound.
    #[error("random bound must be positive, received {0}")]
    InvalidBound(i32),
    /// Explicit consumption exceeds the per-call CPU budget.
    #[error("random consumption {0} exceeds {MAX_RANDOM_CONSUME_COUNT}")]
    ConsumeCountTooLarge(u32),
}

impl WorldgenRandomSource {
    /// Creates an explicitly selected algorithm with a numeric world seed.
    pub const fn new(algorithm: RandomAlgorithm, seed: WorldSeed) -> Self {
        match algorithm {
            RandomAlgorithm::Legacy => Self::legacy(seed),
            RandomAlgorithm::Xoroshiro128PlusPlus => Self::xoroshiro(seed),
        }
    }

    /// Creates the original 48-bit Java-compatible source.
    pub const fn legacy(seed: WorldSeed) -> Self {
        Self {
            state: State::Legacy((seed.value() as u64 ^ LEGACY_MULTIPLIER) & LEGACY_MASK),
        }
    }

    /// Creates xoroshiro128++ using Minecraft's numeric-seed upgrade.
    pub const fn xoroshiro(seed: WorldSeed) -> Self {
        Self::xoroshiro_128(Seed128::upgrade(seed))
    }

    /// Creates xoroshiro128++; all-zero halves select the original fallback.
    pub const fn xoroshiro_128(mut seed: Seed128) -> Self {
        if seed.lo | seed.hi == 0 {
            seed = Seed128 {
                lo: GOLDEN_RATIO,
                hi: SILVER_RATIO,
            };
        }
        Self {
            state: State::Xoroshiro(seed),
        }
    }

    /// Returns the selected algorithm without advancing the stream.
    pub const fn algorithm(self) -> RandomAlgorithm {
        match self.state {
            State::Legacy(_) => RandomAlgorithm::Legacy,
            State::Xoroshiro(_) => RandomAlgorithm::Xoroshiro128PlusPlus,
        }
    }

    /// Restarts this algorithm from an explicitly selected numeric seed.
    pub fn set_seed(&mut self, seed: WorldSeed) {
        *self = Self::new(self.algorithm(), seed);
    }

    /// Draws a signed 32-bit integer using the original algorithm's bit selection.
    pub fn next_int(&mut self) -> i32 {
        match &mut self.state {
            State::Legacy(seed) => legacy_bits(seed, 32).cast_signed(),
            State::Xoroshiro(seed) => (xoroshiro_long(seed) as u32).cast_signed(),
        }
    }

    /// Draws an unbiased value in `0..bound`; invalid bounds preserve the stream.
    pub fn next_int_bounded(&mut self, bound: i32) -> Result<i32, RandomError> {
        if bound <= 0 {
            return Err(RandomError::InvalidBound(bound));
        }
        Ok(self.bounded_positive(bound as u32).cast_signed())
    }

    // Callers in this crate supply 1..=i32::MAX. In particular, permutation
    // construction supplies 256..=1, so an impossible public error is avoided.
    pub(crate) fn bounded_positive(&mut self, bound: u32) -> u32 {
        match &mut self.state {
            State::Legacy(seed) => {
                if bound.is_power_of_two() {
                    return ((u64::from(bound) * u64::from(legacy_bits(seed, 31))) >> 31) as u32;
                }
                loop {
                    let bits = legacy_bits(seed, 31);
                    let value = bits % bound;
                    if bits
                        .wrapping_sub(value)
                        .wrapping_add(bound - 1)
                        .cast_signed()
                        >= 0
                    {
                        return value;
                    }
                }
            }
            State::Xoroshiro(seed) => {
                let threshold = bound.wrapping_neg() % bound;
                loop {
                    let product = u64::from(xoroshiro_long(seed) as u32) * u64::from(bound);
                    if product as u32 >= threshold {
                        return (product >> 32) as u32;
                    }
                }
            }
        }
    }

    /// Draws a signed Java long, including signed widening of both legacy ints.
    pub fn next_long(&mut self) -> i64 {
        match &mut self.state {
            State::Legacy(seed) => {
                let high = i64::from(legacy_bits(seed, 32).cast_signed());
                let low = i64::from(legacy_bits(seed, 32).cast_signed());
                high.wrapping_shl(32).wrapping_add(low)
            }
            State::Xoroshiro(seed) => xoroshiro_long(seed).cast_signed(),
        }
    }

    /// Draws the original boolean (legacy high bit; xoroshiro low bit).
    pub fn next_boolean(&mut self) -> bool {
        match &mut self.state {
            State::Legacy(seed) => legacy_bits(seed, 1) != 0,
            State::Xoroshiro(seed) => xoroshiro_long(seed) & 1 != 0,
        }
    }

    /// Draws an exactly representable 24-bit fraction in `[0, 1)`.
    pub fn next_float(&mut self) -> f32 {
        let bits = match &mut self.state {
            State::Legacy(seed) => legacy_bits(seed, 24),
            State::Xoroshiro(seed) => (xoroshiro_long(seed) >> 40) as u32,
        };
        bits as f32 * (1.0 / 16_777_216.0)
    }

    /// Draws the original 53-bit fraction in `[0, 1)`.
    pub fn next_double(&mut self) -> f64 {
        let bits = match &mut self.state {
            State::Legacy(seed) => {
                (u64::from(legacy_bits(seed, 26)) << 27) + u64::from(legacy_bits(seed, 27))
            }
            State::Xoroshiro(seed) => xoroshiro_long(seed) >> 11,
        };
        bits as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    /// Consumes the original `count` next-int steps (one core step per algorithm).
    /// Counts above the CPU cap fail before mutation; zero is a no-op.
    pub fn consume_count(&mut self, count: u32) -> Result<(), RandomError> {
        if count > MAX_RANDOM_CONSUME_COUNT {
            return Err(RandomError::ConsumeCountTooLarge(count));
        }
        for _ in 0..count {
            let _ = self.next_int();
        }
        Ok(())
    }

    /// Forks using one legacy long or two xoroshiro longs, consuming the parent.
    #[must_use]
    pub fn fork(&mut self) -> Self {
        match self.state {
            State::Legacy(_) => Self::legacy(WorldSeed::new(self.next_long())),
            State::Xoroshiro(_) => {
                let lo = self.next_long();
                let hi = self.next_long();
                Self::xoroshiro_128(Seed128::new(lo, hi))
            }
        }
    }

    /// Captures an immutable positional factory using the original fork ordering.
    pub fn fork_positional(&mut self) -> PositionalRandomFactory {
        match self.state {
            State::Legacy(_) => PositionalRandomFactory::legacy(WorldSeed::new(self.next_long())),
            State::Xoroshiro(_) => {
                let lo = self.next_long();
                let hi = self.next_long();
                PositionalRandomFactory::xoroshiro(Seed128::new(lo, hi))
            }
        }
    }
}

fn legacy_bits(seed: &mut u64, bits: u32) -> u32 {
    *seed = seed.wrapping_mul(LEGACY_MULTIPLIER).wrapping_add(11) & LEGACY_MASK;
    (*seed >> (48 - bits)) as u32
}

fn xoroshiro_long(seed: &mut Seed128) -> u64 {
    let output = seed
        .lo
        .wrapping_add(seed.hi)
        .rotate_left(17)
        .wrapping_add(seed.lo);
    let mixed = seed.hi ^ seed.lo;
    seed.lo = seed.lo.rotate_left(49) ^ mixed ^ mixed.wrapping_shl(21);
    seed.hi = mixed.rotate_left(28);
    output
}

/// Immutable original coordinate/name factory; request order cannot mutate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionalRandomFactory {
    state: State,
}

impl PositionalRandomFactory {
    /// Constructs an original legacy positional factory from its raw seed.
    pub const fn legacy(seed: WorldSeed) -> Self {
        Self {
            state: State::Legacy(seed.value() as u64),
        }
    }

    /// Constructs an original xoroshiro positional factory from its raw halves.
    pub const fn xoroshiro(seed: Seed128) -> Self {
        Self {
            state: State::Xoroshiro(seed),
        }
    }

    /// Derives a stream at a typed block coordinate with original Java overflow.
    pub fn at(self, position: BlockPos) -> WorldgenRandomSource {
        let coordinate = coordinate_seed(position) as u64;
        match self.state {
            State::Legacy(seed) => {
                WorldgenRandomSource::legacy(WorldSeed::new((coordinate ^ seed).cast_signed()))
            }
            State::Xoroshiro(seed) => WorldgenRandomSource::xoroshiro_128(Seed128 {
                lo: coordinate ^ seed.lo,
                hi: seed.hi,
            }),
        }
    }

    /// Derives a named stream using Java UTF16 hash or original MD5 UTF-8 halves.
    /// Rust strings do not represent Java's isolated surrogate characters.
    pub fn from_hash_of(self, name: &str) -> WorldgenRandomSource {
        match self.state {
            State::Legacy(seed) => {
                let hash = name.encode_utf16().fold(0_i32, |hash, unit| {
                    hash.wrapping_mul(31).wrapping_add(i32::from(unit))
                });
                WorldgenRandomSource::legacy(WorldSeed::new(
                    (i64::from(hash) as u64 ^ seed).cast_signed(),
                ))
            }
            State::Xoroshiro(seed) => {
                WorldgenRandomSource::xoroshiro_128(Seed128::from_name(name).xor(seed))
            }
        }
    }

    /// Derives from a numeric seed; legacy deliberately ignores the factory seed.
    pub const fn from_seed(self, seed: WorldSeed) -> WorldgenRandomSource {
        match self.state {
            State::Legacy(_) => WorldgenRandomSource::legacy(seed),
            State::Xoroshiro(factory) => WorldgenRandomSource::xoroshiro_128(Seed128 {
                lo: seed.value() as u64 ^ factory.lo,
                hi: seed.value() as u64 ^ factory.hi,
            }),
        }
    }
}

fn coordinate_seed(position: BlockPos) -> i64 {
    // The x multiply is an int overflow BEFORE Java widens to long; z's
    // multiply is already long. Widening x first changes distant-world seeds.
    let value = i64::from(position.x().wrapping_mul(3_129_871))
        ^ i64::from(position.z()).wrapping_mul(116_129_781)
        ^ i64::from(position.y());
    value
        .wrapping_mul(value)
        .wrapping_mul(42_317_861)
        .wrapping_add(value.wrapping_mul(11))
        >> 16
}
