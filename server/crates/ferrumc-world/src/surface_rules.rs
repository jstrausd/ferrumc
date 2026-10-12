//! Original Minecraft 1.21.8 / Paper build 60 overworld surface rules.
//!
//! Profile: Paper build 60 numerics (the Paper `ImprovedNoise` offset basis and
//! the f32-rounded `nextDouble` column draw), Paper zoom semantics through the
//! GEN-05.8 stored-biome resolver and the Paper default flat-bedrock mode
//! (`generateFlatBedrock = false`). Any other profile is refused typed.
//!
//! The module compiles the original overworld `surface_rule` of the pinned
//! Registry settings export into an immutable preorder node table
//! ([`SurfaceRuleTemplate`]), binds it to one world seed
//! ([`SeededSurfaceRules`]) and evaluates one block position at a time against
//! an explicit per-chunk, per-column and per-block context
//! ([`SurfaceRuleSession`]). The column driver that produces stone depths,
//! water heights and the heights view, and that writes blocks into chunks, is a
//! separate later consumer. Nothing here generates, installs, persists or
//! activates terrain.
//!
//! Every lookup refusal is typed ([`SurfaceRuleErrorKind`]); there is no default
//! block, Plains, nearest-chunk or fallback-temperature substitution.

mod bind;
mod compile;
mod error;
mod session;

use ferrumc_math::{BlockPos, ChunkPos};

pub use bind::SeededSurfaceRules;
pub use compile::SurfaceRuleTemplate;
pub use error::{
    ResolverMismatch, SurfaceRuleError, SurfaceRuleErrorKind, SurfaceRuleLimit,
    SurfaceRuleRefusalCategory,
};
pub use session::SurfaceRuleSession;

use crate::BlockStateId;

/// Maximum compiled rule and condition nodes of one template.
pub const MAX_SURFACE_RULE_NODES: usize = 512;
/// Maximum node depth, root zero; also the explicit evaluation stack bound.
pub const MAX_SURFACE_RULE_DEPTH: usize = 32;
/// Maximum children of one `sequence` rule.
pub const MAX_SURFACE_RULE_SEQUENCE_CHILDREN: usize = 64;
/// Maximum biome names over all `biome` conditions.
pub const MAX_SURFACE_RULE_BIOME_REFERENCES: usize = 256;
/// Maximum biome names of one `biome` condition.
pub const MAX_SURFACE_RULE_BIOMES_PER_CONDITION: usize = 16;
/// Maximum distinct bound noises, including the three `SurfaceSystem` noises.
pub const MAX_SURFACE_RULE_NOISES: usize = 16;
/// Maximum distinct gradient random names.
pub const MAX_SURFACE_RULE_RANDOM_NAMES: usize = 8;
/// Maximum distinct result block states.
pub const MAX_SURFACE_RULE_STATES: usize = 128;
/// Design cap for the measured retained bytes of one template.
pub const MAX_SURFACE_RULE_TEMPLATE_BYTES: usize = 16 * 1024;
/// Design cap for the measured retained bytes of one seeded binding.
pub const MAX_SURFACE_RULE_BINDING_BYTES: usize = 32 * 1024;
/// Design cap for one session value; a session owns no heap memory.
pub const MAX_SURFACE_RULE_SESSION_BYTES: usize = 4 * 1024;
/// Largest admitted template compilation allowance.
pub const MAX_SURFACE_RULE_BUILD_WORK: u32 = 262_144;
/// Largest admitted seed-binding allowance.
pub const MAX_SURFACE_RULE_BIND_WORK: u32 = 65_536;
/// Largest admitted session allowance.
pub const MAX_SURFACE_RULE_SESSION_WORK: u32 = 67_108_864;
/// Units charged by [`SurfaceRuleSession::begin_column`]: surface noise and draw.
pub const SURFACE_RULE_COLUMN_WORK: u32 = 2;
/// Units charged once per column when the minimum surface level is computed.
pub const SURFACE_RULE_MIN_SURFACE_WORK: u32 = 1;
/// Session allowance that suffices for every block of one overworld chunk:
/// `256 × (2 + 1 + 384 × 467)`.
pub const SURFACE_RULE_OVERWORLD_CHUNK_WORK: u32 = 45_908_736;

/// Number of original clay bands (`SurfaceSystem.generateBands`).
pub const SURFACE_RULE_CLAY_BANDS: usize = 192;

/// Level minimum block Y every admitted overworld setting assumes.
///
/// The original resolves the generation context from the chunk's level height
/// (`minY = max(level, settings)`, `genDepth = min(level, settings)`); the
/// overworld level height is `(-64, 384)`, which equals `noise.min_y` and
/// `noise.height` of the three admitted settings. The resolver geometry check
/// (`ResolverMismatch::Geometry`) rests on this assumption.
pub const SURFACE_RULE_LEVEL_MIN_Y: i32 = -64;
/// Level height every admitted overworld setting assumes; see
/// [`SURFACE_RULE_LEVEL_MIN_Y`].
pub const SURFACE_RULE_LEVEL_HEIGHT: u32 = 384;
/// Inclusive admitted stone depth above and below (`FerrumC` policy P4).
pub const MAX_SURFACE_RULE_STONE_DEPTH: i32 = 4096;

/// Checked template compilation allowance, admitted in `1..=262_144`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceRuleBuildBudget(u32);

impl SurfaceRuleBuildBudget {
    /// Admits `1..=MAX_SURFACE_RULE_BUILD_WORK`; anything else is `InvalidBudget`.
    pub fn new(units: u32) -> Result<Self, SurfaceRuleError> {
        admit_budget(units, MAX_SURFACE_RULE_BUILD_WORK).map(Self)
    }
    /// Admitted allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

/// Checked seed-binding allowance, admitted in `1..=65_536`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceRuleBindBudget(u32);

impl SurfaceRuleBindBudget {
    /// Admits `1..=MAX_SURFACE_RULE_BIND_WORK`; anything else is `InvalidBudget`.
    pub fn new(units: u32) -> Result<Self, SurfaceRuleError> {
        admit_budget(units, MAX_SURFACE_RULE_BIND_WORK).map(Self)
    }
    /// Admitted allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

/// Checked session allowance, admitted in `1..=67_108_864`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceRuleWorkBudget(u32);

impl SurfaceRuleWorkBudget {
    /// Admits `1..=MAX_SURFACE_RULE_SESSION_WORK`; anything else is `InvalidBudget`.
    pub fn new(units: u32) -> Result<Self, SurfaceRuleError> {
        admit_budget(units, MAX_SURFACE_RULE_SESSION_WORK).map(Self)
    }
    /// Admitted allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

fn admit_budget(units: u32, maximum: u32) -> Result<u32, SurfaceRuleError> {
    if units == 0 || units > maximum {
        return Err(SurfaceRuleError::new(SurfaceRuleErrorKind::InvalidBudget)
            .with_counts(units as usize, maximum as usize));
    }
    Ok(units)
}

/// Numerical profile of the bound samplers (`FerrumC` policy P1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceNumericProfile {
    /// Paper build 60: f32-rounded `ImprovedNoise` offsets and column draw.
    /// The only admitted profile; the upstream terrain producers are Paper.
    PaperBuild60,
    /// Vanilla 1.21.8 numerics; refused typed as `UnsupportedNumericProfile`
    /// and never silently computed as Paper.
    Vanilla,
}

/// Paper `generateFlatBedrock` world configuration (`FerrumC` policy P2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperFlatBedrock {
    /// Paper default: the bedrock condition evaluates as the original gradient.
    Disabled,
    /// Flat bedrock; refused typed as `UnsupportedFlatBedrock` until a separate
    /// world-configuration ticket provides valid evidence.
    Enabled,
}

/// Selected rule state, or no rule matched (the caller keeps its block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRuleOutcome {
    /// The original rule tree selected this block state.
    State(BlockStateId),
    /// No rule produced a state (original `null`).
    NoRule,
}

/// Block column of one chunk: chunk position plus local X/Z in `0..16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceColumnPos {
    chunk: ChunkPos,
    local_x: u8,
    local_z: u8,
}

impl SurfaceColumnPos {
    /// Returns `None` unless both local coordinates are below 16.
    pub const fn new(chunk: ChunkPos, local_x: u8, local_z: u8) -> Option<Self> {
        if local_x < 16 && local_z < 16 {
            Some(Self {
                chunk,
                local_x,
                local_z,
            })
        } else {
            None
        }
    }
    /// Column containing a block; its Y is ignored.
    pub const fn from_block(pos: BlockPos) -> Self {
        Self {
            chunk: pos.to_chunk_pos(),
            local_x: (pos.x() & 15) as u8,
            local_z: (pos.z() & 15) as u8,
        }
    }
    /// Chunk of this column.
    pub const fn chunk(self) -> ChunkPos {
        self.chunk
    }
    /// Local X in `0..16`.
    pub const fn local_x(self) -> u8 {
        self.local_x
    }
    /// Local Z in `0..16`.
    pub const fn local_z(self) -> u8 {
        self.local_z
    }
    /// Absolute block X with Java `int` wrapping.
    pub const fn block_x(self) -> i32 {
        self.chunk
            .x()
            .wrapping_mul(16)
            .wrapping_add(self.local_x as i32)
    }
    /// Absolute block Z with Java `int` wrapping.
    pub const fn block_z(self) -> i32 {
        self.chunk
            .z()
            .wrapping_mul(16)
            .wrapping_add(self.local_z as i32)
    }
}

/// The four original preliminary surface levels of the centre surface cell.
///
/// They are `NoiseChunk.preliminarySurfaceLevel` at block `(16cx, 16cz)`,
/// `(16cx+16, 16cz)`, `(16cx, 16cz+16)` and `(16cx+16, 16cz+16)`, produced by
/// the column driver through GEN-05.3. Every `i32` is admitted, including
/// `i32::MAX` ("no surface").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreliminarySurfaceCorners([i32; 4]);

impl PreliminarySurfaceCorners {
    /// Corners in the original cache order: `(0,0)`, `(+16,0)`, `(0,+16)`, `(+16,+16)`.
    pub const fn new(
        min_x_min_z: i32,
        max_x_min_z: i32,
        min_x_max_z: i32,
        max_x_max_z: i32,
    ) -> Self {
        Self([min_x_min_z, max_x_min_z, min_x_max_z, max_x_max_z])
    }
    /// Corners in the original cache order.
    pub const fn values(self) -> [i32; 4] {
        self.0
    }
}

/// Borrowed original `WORLD_SURFACE_WG` heights of the centre chunk.
///
/// Each value is the original `getHeight` result (top block Y, or `minY - 1`).
/// The view is passed on every [`SurfaceRuleSession::evaluate`] call because the
/// original reads it lazily at a column's first `steep` test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSurfaceHeights([[i32; 16]; 16]);

impl WorldSurfaceHeights {
    /// Heights indexed `[local_x][local_z]`.
    pub const fn new(heights: [[i32; 16]; 16]) -> Self {
        Self(heights)
    }
    /// Height of one local column, or `None` outside `0..16`.
    pub fn height(&self, local_x: u8, local_z: u8) -> Option<i32> {
        self.0
            .get(usize::from(local_x))?
            .get(usize::from(local_z))
            .copied()
    }
    const fn at(&self, local_x: usize, local_z: usize) -> i32 {
        self.0[local_x][local_z]
    }
}

/// Per-block inputs the column driver produces for one `updateY`.
///
/// Values are checked by [`SurfaceRuleSession::evaluate`] against the `FerrumC`
/// policy domains (P4): `y` inside the generation height, both stone depths in
/// `1..=4096`, and a water height other than `i32::MIN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceBlockInput {
    y: i32,
    stone_depth_above: i32,
    stone_depth_below: i32,
    water_height: Option<i32>,
}

impl SurfaceBlockInput {
    /// Block Y, stone depths above/below and water height (`None` = no water,
    /// the original `Integer.MIN_VALUE`).
    pub const fn new(
        y: i32,
        stone_depth_above: i32,
        stone_depth_below: i32,
        water_height: Option<i32>,
    ) -> Self {
        Self {
            y,
            stone_depth_above,
            stone_depth_below,
            water_height,
        }
    }
    /// Block Y.
    pub const fn y(self) -> i32 {
        self.y
    }
    /// Original `stoneDepthAbove`.
    pub const fn stone_depth_above(self) -> i32 {
        self.stone_depth_above
    }
    /// Original `stoneDepthBelow`.
    pub const fn stone_depth_below(self) -> i32 {
        self.stone_depth_below
    }
    /// Original `waterHeight`; `None` means no water above.
    pub const fn water_height(self) -> Option<i32> {
        self.water_height
    }
}

#[cfg(test)]
mod tests;
