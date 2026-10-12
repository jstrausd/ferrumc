//! One chunk's evaluation session with the original lazy per-column/per-block state.

use std::mem::size_of;

use ferrumc_math::{BlockPos, Vec3};
use ferrumc_registry::biome::{BiomeParametersResourceId, BiomeResourceId};

use super::bind::SeededSurfaceRules;
use super::compile::{Gradient, NodeKind, SurfaceRuleTemplate};
use super::error::limit;
use super::{
    PreliminarySurfaceCorners, ResolverMismatch, SurfaceBlockInput, SurfaceColumnPos,
    SurfaceRuleError, SurfaceRuleErrorKind as Kind, SurfaceRuleLimit, SurfaceRuleOutcome,
    SurfaceRuleWorkBudget, WorldSurfaceHeights, MAX_SURFACE_RULE_DEPTH, MAX_SURFACE_RULE_NODES,
    MAX_SURFACE_RULE_SESSION_BYTES, MAX_SURFACE_RULE_STONE_DEPTH, SURFACE_RULE_CLAY_BANDS,
    SURFACE_RULE_COLUMN_WORK, SURFACE_RULE_LEVEL_HEIGHT, SURFACE_RULE_LEVEL_MIN_Y,
    SURFACE_RULE_MIN_SURFACE_WORK,
};
use crate::biome_selection::BiomeSourceIdentity;
use crate::biome_temperature::BuiltinBiomeTemperatures;
use crate::biome_zoom::BiomeZoomSemantics;
use crate::density::admit_chunk;
use crate::stored_biome::{StoredBiomeErrorKind, SurfaceBiomeResolver};
use crate::BlockStateId;

const WORDS: usize = MAX_SURFACE_RULE_NODES / 64;
/// Original `HOW_FAR_BELOW_PRELIMINARY_SURFACE_LEVEL_TO_BUILD_SURFACE`.
const BELOW_PRELIMINARY_SURFACE: i32 = 8;

/// Fixed node bitset; one bit per compiled node (cap 512).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NodeBits([u64; WORDS]);

impl NodeBits {
    const EMPTY: Self = Self([0; WORDS]);
    pub(super) const fn get(&self, node: usize) -> bool {
        self.0[node / 64] & (1 << (node % 64)) != 0
    }
    const fn set(&mut self, node: usize, value: bool) {
        if value {
            self.0[node / 64] |= 1 << (node % 64);
        } else {
            self.0[node / 64] &= !(1 << (node % 64));
        }
    }
}

/// Lazily computed per-column state (original `LazyXZCondition`s and column caches).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Column {
    pub pos: SurfaceColumnPos,
    pub surface_depth: i32,
    pub surface_noise: f64,
    pub draw: f64,
    pub noise_done: NodeBits,
    pub noise_value: NodeBits,
    /// First steep test: result and the four heights it read.
    pub steep: Option<(bool, [i32; 4])>,
    pub hole: Option<bool>,
    pub secondary: Option<f64>,
    pub min_surface: Option<i32>,
}

/// Original per-block (`LazyYCondition`) state of the current call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Block {
    pub done: NodeBits,
    pub value: NodeBits,
    pub biome: Option<BiomeResourceId>,
    pub temperature: Option<bool>,
}

impl Block {
    const EMPTY: Self = Self {
        done: NodeBits::EMPTY,
        value: NodeBits::EMPTY,
        biome: None,
        temperature: None,
    };
}

#[derive(Clone, Copy)]
struct Frame {
    node: u16,
    entered: u16,
}

/// Exclusively owned evaluation state for one centre chunk.
///
/// Borrows the binding, a complete Paper-semantics overworld resolver for the
/// same seed and settings, and the temperature table. Owns no heap memory.
/// `begin_column`, `evaluate` and `min_surface_level` do not allocate.
#[derive(Debug)]
pub struct SurfaceRuleSession<'a> {
    rules: &'a SeededSurfaceRules,
    resolver: &'a SurfaceBiomeResolver<'a>,
    temperatures: &'a BuiltinBiomeTemperatures,
    corners: PreliminarySurfaceCorners,
    allowance: u32,
    used: u32,
    poisoned: bool,
    pub(super) column: Option<Column>,
    pub(super) block: Block,
}

/// Units spent by the current call, checked against the pre-check bound.
struct Meter(u32);

impl Meter {
    const fn charge(&mut self) {
        self.0 += 1;
    }
}

impl<'a> SurfaceRuleSession<'a> {
    /// Admits a session.
    ///
    /// Refusal order: resolver seed, settings, source (P6), geometry (the
    /// settings' `(min_y, height)` = assumed level height `(-64, 384)`), zoom
    /// semantics (P6), then the centre domain and the session-value cap.
    pub fn new(
        rules: &'a SeededSurfaceRules,
        resolver: &'a SurfaceBiomeResolver<'a>,
        temperatures: &'a BuiltinBiomeTemperatures,
        corners: PreliminarySurfaceCorners,
        budget: SurfaceRuleWorkBudget,
    ) -> Result<Self, SurfaceRuleError> {
        Self::new_with(
            rules,
            resolver,
            temperatures,
            corners,
            budget,
            MAX_SURFACE_RULE_SESSION_BYTES,
        )
    }

    pub(super) fn new_with(
        rules: &'a SeededSurfaceRules,
        resolver: &'a SurfaceBiomeResolver<'a>,
        temperatures: &'a BuiltinBiomeTemperatures,
        corners: PreliminarySurfaceCorners,
        budget: SurfaceRuleWorkBudget,
        session_cap: usize,
    ) -> Result<Self, SurfaceRuleError> {
        admit_resolver(rules, resolver)?;
        let center = resolver.center();
        for dx in -1..=1 {
            for dz in -1..=1 {
                let key = ferrumc_math::ChunkPos::new(
                    center.x().wrapping_add(dx),
                    center.z().wrapping_add(dz),
                );
                admit_chunk(key).map_err(|_| {
                    SurfaceRuleError::new(Kind::Biome(StoredBiomeErrorKind::CoordinateDomain))
                })?;
            }
        }
        limit(
            SurfaceRuleLimit::SessionBytes,
            size_of::<Self>(),
            session_cap,
        )?;
        Ok(Self {
            rules,
            resolver,
            temperatures,
            corners,
            allowance: budget.units(),
            used: 0,
            poisoned: false,
            column: None,
            block: Block::EMPTY,
        })
    }

    /// Units charged so far; readable on a poisoned session.
    pub const fn work_used(&self) -> u32 {
        self.used
    }

    const fn remaining(&self) -> u32 {
        self.allowance - self.used
    }

    fn template(&self) -> &'a SurfaceRuleTemplate {
        let rules: &'a SeededSurfaceRules = self.rules;
        rules.template()
    }

    fn admit(&self) -> Result<(), SurfaceRuleError> {
        if self.poisoned {
            return Err(SurfaceRuleError::new(Kind::Poisoned));
        }
        Ok(())
    }

    fn active(&self) -> Result<Column, SurfaceRuleError> {
        self.admit()?;
        self.column
            .ok_or_else(|| SurfaceRuleError::new(Kind::NoActiveColumn))
    }

    fn work_refusal(&self, required: u32) -> Result<(), SurfaceRuleError> {
        if self.remaining() < required {
            return Err(SurfaceRuleError::new(Kind::WorkBudgetExceeded)
                .with_counts(required as usize, self.remaining() as usize));
        }
        Ok(())
    }

    fn poison(&mut self, error: SurfaceRuleError) -> SurfaceRuleError {
        self.poisoned = true;
        error
    }

    /// Starts a column (original `updateXZ`): `Poisoned`, column domain, then
    /// work (2 units). A refusal has no effect; success resets every per-column
    /// cache and computes the surface depth.
    pub fn begin_column(&mut self, pos: SurfaceColumnPos) -> Result<(), SurfaceRuleError> {
        self.admit()?;
        if pos.chunk() != self.resolver.center() {
            return Err(SurfaceRuleError::new(Kind::ColumnOutsideCentre));
        }
        self.work_refusal(SURFACE_RULE_COLUMN_WORK)?;
        let (x, z) = (pos.block_x(), pos.block_z());
        let noise = sample(self.rules.surface_noise(), x, z).map_err(|e| self.poison(e))?;
        // Paper build 60 recompiles nextDouble through f32 (l2f/fmul/f2d).
        let draw = f64::from(self.rules.root.at(BlockPos::new(x, 0, z)).next_double() as f32);
        let column = Column {
            pos,
            surface_depth: (noise * 2.75 + 3.0 + draw * 0.25) as i32,
            surface_noise: noise,
            draw,
            noise_done: NodeBits::EMPTY,
            noise_value: NodeBits::EMPTY,
            steep: None,
            hole: None,
            secondary: None,
            min_surface: None,
        };
        self.column = Some(column);
        self.block = Block::EMPTY;
        self.used += SURFACE_RULE_COLUMN_WORK;
        Ok(())
    }

    /// Original `surfaceDepth` of the active column (0 units).
    pub fn surface_depth(&self) -> Result<i32, SurfaceRuleError> {
        Ok(self.active()?.surface_depth)
    }

    /// Original `getMinSurfaceLevel` for the frozen-ocean driver: `Poisoned`,
    /// `NoActiveColumn`, then (only when uncached) work of 1 unit.
    pub fn min_surface_level(&mut self) -> Result<i32, SurfaceRuleError> {
        let column = self.active()?;
        if let Some(cached) = column.min_surface {
            return Ok(cached);
        }
        self.work_refusal(SURFACE_RULE_MIN_SURFACE_WORK)?;
        let mut meter = Meter(0);
        let value = self.minimum(&mut meter);
        self.used += meter.0;
        Ok(value)
    }

    /// Evaluates one default-block position of the active column (original
    /// `updateY` + `tryApply`).
    ///
    /// Refusal order, before any mutation: `Poisoned`, `NoActiveColumn`, `y`,
    /// stone depth above, below, water (P4), then the remaining allowance
    /// below `467 + m` (`m = 1` iff the column minimum is uncached). A defensive
    /// biome, temperature or noise failure poisons the session.
    pub fn evaluate(
        &mut self,
        input: SurfaceBlockInput,
        heights: &WorldSurfaceHeights,
    ) -> Result<SurfaceRuleOutcome, SurfaceRuleError> {
        let column = self.active()?;
        let x = column.pos.block_x();
        let z = column.pos.block_z();
        let pos = BlockPos::new(x, input.y(), z);
        self.admit_input(input)
            .map_err(|error| error.with_block(pos))?;
        let m = u32::from(column.min_surface.is_none());
        self.work_refusal(self.template().evaluate_work + m)
            .map_err(|error| error.with_block(pos))?;
        self.block = Block::EMPTY;
        let mut meter = Meter(0);
        let result = self.run(input, heights, &mut meter);
        self.used += meter.0;
        match result {
            Ok(Some(state)) => Ok(SurfaceRuleOutcome::State(state)),
            Ok(None) => Ok(SurfaceRuleOutcome::NoRule),
            Err(error) => Err(self.poison(error.with_block(pos))),
        }
    }

    /// `FerrumC` policy P4 in the original `updateY` argument order.
    fn admit_input(&self, input: SurfaceBlockInput) -> Result<(), SurfaceRuleError> {
        let generation = self.template().generation;
        let top = generation
            .min_y
            .wrapping_add(generation.depth)
            .wrapping_sub(1);
        if input.y() < generation.min_y || input.y() > top {
            return Err(SurfaceRuleError::new(Kind::BlockOutsideGenerationHeight));
        }
        let stone = 1..=MAX_SURFACE_RULE_STONE_DEPTH;
        if !stone.contains(&input.stone_depth_above())
            || !stone.contains(&input.stone_depth_below())
        {
            return Err(SurfaceRuleError::new(Kind::InvalidStoneDepth));
        }
        if input.water_height() == Some(i32::MIN) {
            return Err(SurfaceRuleError::new(Kind::InvalidWaterHeight));
        }
        Ok(())
    }

    /// Explicit-stack preorder evaluation; first matching rule wins.
    fn run(
        &mut self,
        input: SurfaceBlockInput,
        heights: &WorldSurfaceHeights,
        meter: &mut Meter,
    ) -> Result<Option<BlockStateId>, SurfaceRuleError> {
        let template = self.template();
        let mut stack = [Frame {
            node: 0,
            entered: 0,
        }; MAX_SURFACE_RULE_DEPTH + 1];
        let mut depth = 1;
        let mut last = None;
        meter.charge();
        while depth > 0 {
            let frame = stack[depth - 1];
            let node = usize::from(frame.node);
            let mut push = None;
            match template.nodes[node].kind {
                NodeKind::Sequence { first, len } => {
                    if (frame.entered > 0 && last.is_some()) || frame.entered == len {
                        depth -= 1;
                    } else {
                        last = None;
                        push = Some(template.children[usize::from(first + frame.entered)]);
                        stack[depth - 1].entered += 1;
                    }
                }
                NodeKind::Condition { test, then } => {
                    if frame.entered > 0 {
                        depth -= 1;
                    } else if self.test(usize::from(test), input, heights, meter)? {
                        stack[depth - 1].entered = 1;
                        push = Some(then);
                    } else {
                        last = None;
                        depth -= 1;
                    }
                }
                NodeKind::Block(state) => {
                    last = Some(template.states[usize::from(state)]);
                    depth -= 1;
                }
                NodeKind::Bandlands => {
                    last = Some(self.band(input.y(), meter)?);
                    depth -= 1;
                }
                _ => return Err(SurfaceRuleError::new(Kind::MalformedDefinition).with_node(node)),
            }
            if let Some(child) = push {
                if depth > MAX_SURFACE_RULE_DEPTH {
                    return Err(SurfaceRuleError::new(Kind::MalformedDefinition).with_node(node));
                }
                stack[depth] = Frame {
                    node: child,
                    entered: 0,
                };
                depth += 1;
                meter.charge();
            }
        }
        Ok(last)
    }

    /// One condition with its `not` chain, iteratively.
    fn test(
        &mut self,
        mut node: usize,
        input: SurfaceBlockInput,
        heights: &WorldSurfaceHeights,
        meter: &mut Meter,
    ) -> Result<bool, SurfaceRuleError> {
        let mut negate = false;
        loop {
            meter.charge();
            let kind = self.template().nodes[node].kind;
            if let NodeKind::Not(target) = kind {
                negate = !negate;
                node = usize::from(target);
                continue;
            }
            return Ok(self.leaf(node, kind, input, heights, meter)? != negate);
        }
    }

    fn leaf(
        &mut self,
        node: usize,
        kind: NodeKind,
        input: SurfaceBlockInput,
        heights: &WorldSurfaceHeights,
        meter: &mut Meter,
    ) -> Result<bool, SurfaceRuleError> {
        let template = self.template();
        let rules: &'a SeededSurfaceRules = self.rules;
        let column = self
            .column
            .ok_or_else(|| SurfaceRuleError::new(Kind::NoActiveColumn))?;
        let depth = column.surface_depth;
        let y = input.y();
        let above = input.stone_depth_above();
        let value = match kind {
            NodeKind::Biome { first, len } => {
                let biome = self.biome(y, meter)?;
                let set = &template.biomes[usize::from(first)..usize::from(first + len)];
                self.per_block(node, set.contains(&biome))
            }
            NodeKind::NoiseThreshold(index) => {
                if column.noise_done.get(node) {
                    return Ok(column.noise_value.get(node));
                }
                let threshold = template.thresholds[usize::from(index)];
                let noise = &rules.noises[usize::from(threshold.noise)];
                meter.charge();
                let sampled = sample(noise, column.pos.block_x(), column.pos.block_z())?;
                let value = sampled >= threshold.min && sampled <= threshold.max;
                if let Some(column) = &mut self.column {
                    column.noise_done.set(node, true);
                    column.noise_value.set(node, value);
                }
                value
            }
            NodeKind::VerticalGradient(index) | NodeKind::PaperFlatBedrock(index) => {
                let value = self.gradient(template.gradients[usize::from(index)], y, meter);
                self.per_block(node, value)
            }
            NodeKind::YAbove(index) => {
                let check = template.y_checks[usize::from(index)];
                let start = y.wrapping_add(if check.add_stone { above } else { 0 });
                self.per_block(
                    node,
                    start >= check.y.wrapping_add(depth.wrapping_mul(check.multiplier)),
                )
            }
            NodeKind::Water(index) => {
                let check = template.water_checks[usize::from(index)];
                let value = input.water_height().is_none_or(|water| {
                    y.wrapping_add(if check.add_stone { above } else { 0 })
                        >= water
                            .wrapping_add(check.offset)
                            .wrapping_add(depth.wrapping_mul(check.multiplier))
                });
                self.per_block(node, value)
            }
            NodeKind::StoneDepth(index) => {
                let check = template.stone_checks[usize::from(index)];
                let value = self.stone_depth(check, input, depth, meter)?;
                self.per_block(node, value)
            }
            NodeKind::Steep => self.steep(column, heights),
            NodeKind::Hole => {
                let hole = column.hole.unwrap_or(depth <= 0);
                if let Some(column) = &mut self.column {
                    column.hole = Some(hole);
                }
                hole
            }
            NodeKind::AbovePreliminarySurface => y >= self.minimum(meter),
            NodeKind::Temperature => self.temperature(y, meter)?,
            _ => return Err(SurfaceRuleError::new(Kind::MalformedDefinition).with_node(node)),
        };
        Ok(value)
    }

    const fn per_block(&mut self, node: usize, value: bool) -> bool {
        self.block.done.set(node, true);
        self.block.value.set(node, value);
        value
    }

    /// The single memoized resolver call of this block (original `Suppliers.memoize`).
    fn biome(&mut self, y: i32, meter: &mut Meter) -> Result<BiomeResourceId, SurfaceRuleError> {
        if let Some(biome) = self.block.biome {
            return Ok(biome);
        }
        let column = self
            .column
            .ok_or_else(|| SurfaceRuleError::new(Kind::NoActiveColumn))?;
        meter.charge();
        let pos = BlockPos::new(column.pos.block_x(), y, column.pos.block_z());
        let biome = self
            .resolver
            .biome_at_block(pos)
            .map_err(|error| SurfaceRuleError::new(Kind::Biome(error.kind())))?;
        self.block.biome = Some(biome);
        Ok(biome)
    }

    /// Original `TemperatureHelperCondition`: `coldEnoughToSnow(pos, seaLevel)`.
    fn temperature(&mut self, y: i32, meter: &mut Meter) -> Result<bool, SurfaceRuleError> {
        if let Some(cold) = self.block.temperature {
            return Ok(cold);
        }
        let biome = self.biome(y, meter)?;
        let column = self
            .column
            .ok_or_else(|| SurfaceRuleError::new(Kind::NoActiveColumn))?;
        meter.charge();
        let pos = BlockPos::new(column.pos.block_x(), y, column.pos.block_z());
        let cold = self
            .temperatures
            .sample(biome, pos, self.template().generation.sea_level)
            .map_err(|error| SurfaceRuleError::new(Kind::Temperature(error.kind().clone())))?
            .cold_enough_to_snow();
        self.block.temperature = Some(cold);
        Ok(cold)
    }

    /// Original `VerticalGradientCondition` (and the Paper flat variant when disabled).
    fn gradient(&self, gradient: Gradient, y: i32, meter: &mut Meter) -> bool {
        if y <= gradient.true_y {
            return true;
        }
        if y >= gradient.false_y {
            return false;
        }
        let (start, end) = (f64::from(gradient.true_y), f64::from(gradient.false_y));
        // Mth.map(y, T, F, 1, 0) = lerp(inverseLerp(y, T, F), 1, 0) in f64.
        let delta = (f64::from(y) - start) / (end - start);
        let chance = 1.0 + delta * (0.0 - 1.0);
        let Some(column) = self.column else {
            return false;
        };
        meter.charge();
        let factory = self.rules.factories[usize::from(gradient.random)];
        let draw = factory
            .at(BlockPos::new(column.pos.block_x(), y, column.pos.block_z()))
            .next_float();
        f64::from(draw) < chance
    }

    /// Original `StoneDepthCondition`; the secondary noise is cached per column.
    fn stone_depth(
        &mut self,
        check: super::compile::StoneCheck,
        input: SurfaceBlockInput,
        depth: i32,
        meter: &mut Meter,
    ) -> Result<bool, SurfaceRuleError> {
        let stone = if check.ceiling {
            input.stone_depth_below()
        } else {
            input.stone_depth_above()
        };
        let surface = if check.add_surface { depth } else { 0 };
        let secondary = if check.range == 0 {
            0
        } else {
            let value = self.secondary(meter)?;
            // Mth.map(value, -1, 1, 0, range); Java (int) truncates and saturates.
            let delta = (value - -1.0) / (1.0 - -1.0);
            (0.0 + delta * (f64::from(check.range) - 0.0)) as i32
        };
        Ok(stone
            <= 1_i32
                .wrapping_add(check.offset)
                .wrapping_add(surface)
                .wrapping_add(secondary))
    }

    fn secondary(&mut self, meter: &mut Meter) -> Result<f64, SurfaceRuleError> {
        let column = self
            .column
            .ok_or_else(|| SurfaceRuleError::new(Kind::NoActiveColumn))?;
        if let Some(value) = column.secondary {
            return Ok(value);
        }
        meter.charge();
        let value = sample(
            self.rules.secondary_noise(),
            column.pos.block_x(),
            column.pos.block_z(),
        )?;
        if let Some(column) = &mut self.column {
            column.secondary = Some(value);
        }
        Ok(value)
    }

    /// Original `SteepMaterialCondition`, read lazily at the column's first test.
    fn steep(&mut self, column: Column, heights: &WorldSurfaceHeights) -> bool {
        if let Some((steep, _)) = column.steep {
            return steep;
        }
        let lx = usize::from(column.pos.local_x());
        let lz = usize::from(column.pos.local_z());
        let read = [
            heights.at(lx, lz.saturating_sub(1)),
            heights.at(lx, (lz + 1).min(15)),
            heights.at(lx.saturating_sub(1), lz),
            heights.at((lx + 1).min(15), lz),
        ];
        // Original asymmetric axes, Java int `+ 4` wrapping.
        let steep = read[1] >= read[0].wrapping_add(4) || read[2] >= read[3].wrapping_add(4);
        if let Some(column) = &mut self.column {
            column.steep = Some((steep, read));
        }
        steep
    }

    /// Original `getMinSurfaceLevel`, computed and charged once per column.
    fn minimum(&mut self, meter: &mut Meter) -> i32 {
        let Some(column) = self.column else {
            return 0;
        };
        if let Some(cached) = column.min_surface {
            return cached;
        }
        meter.charge();
        let [c00, c10, c01, c11] = self.corners.values();
        // (blockX & 15) / 16.0F is a FLOAT division widened to double.
        let dx = f64::from(f32::from(column.pos.local_x()) / 16.0_f32);
        let dz = f64::from(f32::from(column.pos.local_z()) / 16.0_f32);
        let lerp = |delta: f64, start: f64, end: f64| start + delta * (end - start);
        let value = lerp(
            dz,
            lerp(dx, f64::from(c00), f64::from(c10)),
            lerp(dx, f64::from(c01), f64::from(c11)),
        );
        let floor = java_floor(value)
            .wrapping_add(column.surface_depth)
            .wrapping_sub(BELOW_PRELIMINARY_SURFACE);
        if let Some(column) = &mut self.column {
            column.min_surface = Some(floor);
        }
        floor
    }

    /// Original `SurfaceSystem.getBand`; a negative index is a defensive failure.
    fn band(&self, y: i32, meter: &mut Meter) -> Result<BlockStateId, SurfaceRuleError> {
        let column = self
            .column
            .ok_or_else(|| SurfaceRuleError::new(Kind::NoActiveColumn))?;
        meter.charge();
        let clay = sample(
            self.rules.clay_noise(),
            column.pos.block_x(),
            column.pos.block_z(),
        )?;
        let offset = java_round(clay * 4.0) as i32;
        let length = SURFACE_RULE_CLAY_BANDS.cast_signed() as i32;
        // Java `%` keeps the sign of the left operand; never rem_euclid.
        let index = y.wrapping_add(offset).wrapping_add(length) % length;
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rules.bands.get(index).copied())
            .ok_or_else(|| SurfaceRuleError::new(Kind::Noise))
    }
}

/// Resolver identity in `StoredBiomeIdentity` order, then the zoom semantics.
fn admit_resolver(
    rules: &SeededSurfaceRules,
    resolver: &SurfaceBiomeResolver<'_>,
) -> Result<(), SurfaceRuleError> {
    let identity = resolver.identity();
    let template = rules.template();
    let generation = template.generation;
    let overworld = BiomeParametersResourceId::from_name("minecraft:overworld")
        .ok_or_else(|| SurfaceRuleError::new(Kind::MalformedDefinition))?;
    let geometry = identity.geometry();
    let mismatch = if identity.seed() != rules.seed() {
        Some(ResolverMismatch::Seed)
    } else if identity.resource() != template.resource() {
        Some(ResolverMismatch::Settings)
    } else if identity.source() != BiomeSourceIdentity::MultiNoise(overworld) {
        Some(ResolverMismatch::Source)
    } else if geometry.min_y() != generation.settings_min_y
        || i64::from(geometry.height()) != i64::from(generation.settings_height)
        || geometry.min_y() != SURFACE_RULE_LEVEL_MIN_Y
        || geometry.height() != SURFACE_RULE_LEVEL_HEIGHT
    {
        Some(ResolverMismatch::Geometry)
    } else if resolver.semantics() != BiomeZoomSemantics::Paper1_21_8Build60 {
        Some(ResolverMismatch::ZoomSemantics)
    } else {
        None
    };
    match mismatch {
        Some(mismatch) => Err(SurfaceRuleError::new(Kind::InconsistentResolver(mismatch))),
        None => Ok(()),
    }
}

/// Original `NormalNoise.getValue(x, 0.0, z)`; a refusal is a defensive failure.
fn sample(noise: &crate::noise::NormalNoise, x: i32, z: i32) -> Result<f64, SurfaceRuleError> {
    noise
        .sample(Vec3::new(f64::from(x), 0.0, f64::from(z)))
        .map_err(|_| SurfaceRuleError::new(Kind::Noise))
}

/// Original `Mth.floor(double)` with Java `int` conversion and wrapping.
fn java_floor(value: f64) -> i32 {
    let truncated = value as i32;
    if value < f64::from(truncated) {
        truncated.wrapping_sub(1)
    } else {
        truncated
    }
}

/// Original JDK 21 `Math.round(double)`: exact `floor(a + 1/2)` by bit shifting.
pub(super) fn java_round(value: f64) -> i64 {
    let bits = value.to_bits().cast_signed();
    let biased_exponent = (bits & 0x7ff0_0000_0000_0000) >> 52;
    let shift = (53 - 2 + 1023) - biased_exponent;
    if shift & -64 == 0 {
        let mut significand = (bits & 0x000f_ffff_ffff_ffff) | 0x0010_0000_0000_0000;
        if bits < 0 {
            significand = -significand;
        }
        ((significand >> shift) + 1) >> 1
    } else {
        value as i64
    }
}

/// Private test controls: the state projection and direct allowance positioning.
#[cfg(test)]
impl SurfaceRuleSession<'_> {
    /// Active column with every per-column cache, and `work_used`.
    pub(super) const fn projection(&self) -> (Option<Column>, u32, bool) {
        (self.column, self.used, self.poisoned)
    }
    /// Positions the remaining allowance for matrix rows the public API cannot reach.
    pub(super) const fn set_work_used(&mut self, used: u32) {
        self.used = used;
    }
}
