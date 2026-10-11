//! Original biome zoom seed and allocation-free selected quart coordinates.
//!
//! Selection supports every signed32 block axis independently of density's
//! sampling admission. It does not resolve a source, fetch a neighbor or supply
//! a fallback biome. Vanilla and Paper's distinct f64 evaluations are explicit.

use ferrumc_math::BlockPos;
use sha2::{Digest, Sha256};

use crate::WorldSeed;

/// Original SHA-256-obfuscated biome zoom seed, distinct from a world seed.
///
/// ```compile_fail
/// use ferrumc_world::{WorldSeed, biome_zoom::BiomeZoomSeed};
/// let seed: BiomeZoomSeed = WorldSeed::new(0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BiomeZoomSeed(i64);

impl BiomeZoomSeed {
    /// Hashes exactly the world's eight little-endian seed bytes once.
    ///
    /// The first eight SHA-256 digest bytes are read little-endian, preserving
    /// Guava's original `hashLong(seed).asLong()` interpretation.
    #[must_use]
    pub fn from_world_seed(seed: WorldSeed) -> Self {
        let digest = Sha256::digest(seed.value().to_le_bytes());
        let mut first = [0u8; 8];
        first.copy_from_slice(&digest[..8]);
        Self(i64::from_le_bytes(first))
    }

    /// Accepts explicitly already-obfuscated bits without hashing them again.
    #[must_use]
    pub const fn from_obfuscated_bits(bits: i64) -> Self {
        Self(bits)
    }

    /// Preserved signed64 obfuscated bits, not the original world seed.
    #[must_use]
    pub const fn bits(self) -> i64 {
        self.0
    }
}

/// Exact original numerical evaluation used for biome zoom displacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeZoomSemantics {
    /// Vanilla Java 1.21.8: divide bucket, subtract half, multiply displacement.
    Vanilla1_21_8,
    /// Paper Java 1.21.8 build60: subtract the integer midpoint before scaling.
    Paper1_21_8Build60,
}

/// Selected signed32 quart coordinates, independent of density admission.
///
/// These are source coordinates, not block origins or a checked climate query.
/// A later density consumer must explicitly admit its quart-to-block conversion.
///
/// ```compile_fail
/// use ferrumc_world::{biome_zoom::BiomeQuartPos, density::ClimateQuartPos};
/// fn climate(pos: BiomeQuartPos) -> ClimateQuartPos { pos }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BiomeQuartPos {
    x: i32,
    y: i32,
    z: i32,
}

impl BiomeQuartPos {
    /// Original selected quart X, without a density/world-border restriction.
    #[must_use]
    pub const fn x(self) -> i32 {
        self.x
    }

    /// Original selected quart Y, without height-accessor clamping.
    #[must_use]
    pub const fn y(self) -> i32 {
        self.y
    }

    /// Original selected quart Z, without a density/world-border restriction.
    #[must_use]
    pub const fn z(self) -> i32 {
        self.z
    }
}

/// Immutable original biome zoom coordinate producer; no mutable source history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BiomeZoom {
    seed: BiomeZoomSeed,
    semantics: BiomeZoomSemantics,
}

impl BiomeZoom {
    /// Chooses an explicit obfuscated seed and original numerical contract.
    #[must_use]
    pub const fn new(seed: BiomeZoomSeed, semantics: BiomeZoomSemantics) -> Self {
        Self { seed, semantics }
    }

    /// Explicitly selected obfuscated seed, without another hash operation.
    #[must_use]
    pub const fn seed(self) -> BiomeZoomSeed {
        self.seed
    }

    /// Explicit Vanilla or Paper numerical evaluation.
    #[must_use]
    pub const fn semantics(self) -> BiomeZoomSemantics {
        self.semantics
    }

    /// Selects the original first minimum across exactly eight quart corners.
    ///
    /// All signed32 block axes are supported. Subtraction wraps before arithmetic
    /// shifts, including the two lowest block values. This performs 64 bounded
    /// LCG steps on stack data and never samples or resolves a biome source.
    #[must_use]
    pub fn select_quart(self, pos: BlockPos) -> BiomeQuartPos {
        let shifted = [
            pos.x().wrapping_sub(2),
            pos.y().wrapping_sub(2),
            pos.z().wrapping_sub(2),
        ];
        let base = shifted.map(|axis| axis >> 2);
        let fraction = shifted.map(|axis| f64::from(axis & 3) / 4.0);
        let mut best = f64::INFINITY;
        let mut selected = 0;
        for corner in 0..8 {
            let upper = [corner & 4 != 0, corner & 2 != 0, corner & 1 != 0];
            let quart = [
                base[0] + i32::from(upper[0]),
                base[1] + i32::from(upper[1]),
                base[2] + i32::from(upper[2]),
            ];
            let offset = [
                if upper[0] {
                    fraction[0] - 1.0
                } else {
                    fraction[0]
                },
                if upper[1] {
                    fraction[1] - 1.0
                } else {
                    fraction[1]
                },
                if upper[2] {
                    fraction[2] - 1.0
                } else {
                    fraction[2]
                },
            ];
            let candidate = distance(self.seed.0, quart, offset, self.semantics);
            if prefer_candidate(best, candidate) {
                best = candidate;
                selected = corner;
            }
        }
        BiomeQuartPos {
            x: base[0] + i32::from(selected & 4 != 0),
            y: base[1] + i32::from(selected & 2 != 0),
            z: base[2] + i32::from(selected & 1 != 0),
        }
    }
}

fn prefer_candidate(best: f64, candidate: f64) -> bool {
    best > candidate
}

fn next(left: i64, right: i64) -> i64 {
    left.wrapping_mul(
        left.wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407),
    )
    .wrapping_add(right)
}

fn fiddle(value: i64, semantics: BiomeZoomSemantics) -> f64 {
    match semantics {
        BiomeZoomSemantics::Vanilla1_21_8 => {
            let bucket = (value >> 24).rem_euclid(1024) as f64 / 1024.0;
            (bucket - 0.5) * 0.9
        }
        BiomeZoomSemantics::Paper1_21_8Build60 => {
            (((value >> 24) & 1023) - 512) as f64 * (0.9 / 1024.0)
        }
    }
}

fn distance(seed: i64, quart: [i32; 3], offset: [f64; 3], semantics: BiomeZoomSemantics) -> f64 {
    let mut mixed = seed;
    for coordinate in quart.into_iter().chain(quart) {
        mixed = next(mixed, i64::from(coordinate));
    }
    let x = offset[0] + fiddle(mixed, semantics);
    mixed = next(mixed, seed);
    let y = offset[1] + fiddle(mixed, semantics);
    mixed = next(mixed, seed);
    let z = offset[2] + fiddle(mixed, semantics);
    z * z + y * y + x * x
}

#[cfg(test)]
mod tests;
