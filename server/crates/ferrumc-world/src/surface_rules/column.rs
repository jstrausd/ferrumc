//! Original Minecraft 1.21.8 / Paper build 60 overworld `SurfaceSystem.buildSurface`
//! column driver (the SURFACE step of one centre chunk).
//!
//! The driver copies the GEN-05.4 post-NOISE material into a reusable
//! [`SurfaceChunkBuffer`], then visits the 256 columns in the original order
//! (local X outer, local Z inner). Per column it resolves the column biome
//! through the GEN-05.8 resolver, runs the eroded-badlands prepass, drives one
//! GEN-05.9 [`SurfaceRuleSession`](super::SurfaceRuleSession) over the original
//! stone-depth / water-height loop, writes every rule result and runs the
//! frozen-ocean postpass. `WORLD_SURFACE_WG` is maintained live with the
//! original `Heightmap.update` rule and passed to every `evaluate` call.
//!
//! `FerrumC` policies of this driver (not original behaviour): D1 closed state
//! alphabet, D2 one work admission per chunk, D3 all-or-nothing publication
//! into a reusable buffer, D4 centre domain enforced by the producers, D5
//! `OCEAN_FLOOR_WG` and the post-processing order are not published. The
//! inherited GEN-05.9 policies P1–P4 and P6 apply unchanged.
//!
//! Nothing here installs, persists or activates terrain; carvers, features,
//! structures and lighting are out of scope.

mod bind;
mod buffer;
mod driver;
mod error;

pub use bind::SeededSurfaceColumns;
pub use buffer::SurfaceChunkBuffer;
pub use driver::{SurfaceChunkInput, SurfaceChunkReport};
pub use error::{
    BindingMismatch, MaterialMismatch, SurfaceColumnError, SurfaceColumnErrorKind,
    SurfaceColumnLimit, SurfaceColumnRefusalCategory,
};

/// Largest admitted bind allowance.
pub const MAX_SURFACE_COLUMN_BIND_WORK: u32 = 65_536;
/// Largest admitted driver allowance of one `build_chunk` call.
pub const MAX_SURFACE_COLUMN_WORK: u32 = 1_048_576;
/// Design cap for the measured retained bytes of one [`SeededSurfaceColumns`].
pub const MAX_SURFACE_COLUMN_BINDING_BYTES: usize = 16 * 1024;
/// Design cap for the measured retained bytes of one [`SurfaceChunkBuffer`].
pub const MAX_SURFACE_CHUNK_BUFFER_BYTES: usize = 448 * 1024;
/// Inline capacity of the closed state alphabet (`FerrumC` policy D1).
pub const MAX_SURFACE_COLUMN_ALPHABET: usize = 64;

/// Blocks of one centre chunk: 384 levels × 16 × 16.
pub const SURFACE_CHUNK_STATES: usize = 98_304;
/// 64-bit words of the fluid flag bitmap of one chunk.
pub const SURFACE_CHUNK_FLUID_WORDS: usize = SURFACE_CHUNK_STATES / 64;

/// Driver units of the column biome lookup.
pub const SURFACE_COLUMN_BIOME_WORK: u32 = 1;
/// Driver units bounding the eroded-badlands prepass: three noises, at most
/// 384 scan steps, 384 fill steps and 384 writes (loops start at
/// `min(floor, 319)`).
pub const SURFACE_COLUMN_ERODED_WORK: u32 = 3 + 384 + 384 + 384;
/// Driver units bounding the column loop: at most 385 visits (`i5 ≤ 320` down
/// to −64), 385 stone-below search steps (Σ run length + runs), 384 rule writes
/// and 384 air-top rescan steps (disjoint ranges).
pub const SURFACE_COLUMN_LOOP_WORK: u32 = 385 + 385 + 384 + 384;
/// Driver units bounding the frozen-ocean postpass: three noises, one
/// temperature, two int draws, at most 385 iterations (the loop starts at
/// `min(start, 320)`, review condition A4), 384 double draws and 384 writes.
/// The iteration at 320 reads `VOID_AIR` and never draws: a start above 320 is
/// refused, so `(int) d4 < start ≤ 320`.
pub const SURFACE_COLUMN_FROZEN_WORK: u32 = 3 + 1 + 2 + 385 + 384 + 384;
/// `W_col`: biome + the larger of the two exclusive extensions + loop.
pub const SURFACE_COLUMN_WORK: u32 = SURFACE_COLUMN_BIOME_WORK
    + max_u32(SURFACE_COLUMN_ERODED_WORK, SURFACE_COLUMN_FROZEN_WORK)
    + SURFACE_COLUMN_LOOP_WORK;
/// `C0`: the state copy, the fluid words and the worst-case initial height
/// derivation (one step per block).
pub const SURFACE_CHUNK_FIXED_WORK: u32 = 98_304 + 1_536 + 98_304;
/// `R = C0 + 256 · W_col`: the driver allowance one chunk requires (policy D2).
pub const SURFACE_CHUNK_WORK: u32 = SURFACE_CHUNK_FIXED_WORK + 256 * SURFACE_COLUMN_WORK;
/// Bind census of the six `SurfaceSystem` extension noises in the Paper basis:
/// two stacks × (octave slots + 259 per `ImprovedNoise`).
pub const SURFACE_COLUMN_NOISE_BIND_WORK: usize = 8320;

const fn max_u32(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

/// Checked bind allowance, admitted in `1..=65_536`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceColumnBindBudget(u32);

impl SurfaceColumnBindBudget {
    /// Admits `1..=MAX_SURFACE_COLUMN_BIND_WORK`; anything else is `InvalidBudget`.
    pub fn new(units: u32) -> Result<Self, SurfaceColumnError> {
        admit_budget(units, MAX_SURFACE_COLUMN_BIND_WORK).map(Self)
    }
    /// Admitted allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

/// Checked driver allowance of one chunk, admitted in `1..=1_048_576`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceColumnWorkBudget(u32);

impl SurfaceColumnWorkBudget {
    /// Admits `1..=MAX_SURFACE_COLUMN_WORK`; anything else is `InvalidBudget`.
    pub fn new(units: u32) -> Result<Self, SurfaceColumnError> {
        admit_budget(units, MAX_SURFACE_COLUMN_WORK).map(Self)
    }
    /// Admitted allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

fn admit_budget(units: u32, maximum: u32) -> Result<u32, SurfaceColumnError> {
    if units == 0 || units > maximum {
        return Err(
            SurfaceColumnError::new(SurfaceColumnErrorKind::InvalidBudget)
                .with_counts(units as usize, maximum as usize),
        );
    }
    Ok(units)
}

#[cfg(test)]
mod tests;
