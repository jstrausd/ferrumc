//! The original column loop, the two extensions and the live `WORLD_SURFACE_WG`
//! (`SurfaceSystem.buildSurface`, contract §3–§8).

use ferrumc_math::{BlockPos, ChunkPos, Vec3};
use ferrumc_registry::biome::BiomeResourceId;

use super::super::{
    PreliminarySurfaceCorners, SeededSurfaceRules, SurfaceBlockInput, SurfaceColumnPos,
    SurfaceRuleError, SurfaceRuleErrorKind, SurfaceRuleOutcome, SurfaceRuleSession,
    SurfaceRuleWorkBudget, SURFACE_RULE_LEVEL_HEIGHT, SURFACE_RULE_LEVEL_MIN_Y,
    SURFACE_RULE_OVERWORLD_CHUNK_WORK,
};
use super::bind::{
    StateClass, BADLANDS_PILLAR, BADLANDS_PILLAR_ROOF, BADLANDS_SURFACE, ICEBERG_PILLAR,
    ICEBERG_PILLAR_ROOF, ICEBERG_SURFACE,
};
use super::buffer::{Published, SurfaceChunkBuffer};
use super::{
    BindingMismatch, MaterialMismatch, SeededSurfaceColumns, SurfaceColumnError,
    SurfaceColumnErrorKind as Kind, SurfaceColumnWorkBudget, SURFACE_CHUNK_FLUID_WORDS,
    SURFACE_CHUNK_STATES, SURFACE_CHUNK_WORK,
};
use crate::biome_temperature::BuiltinBiomeTemperatures;
use crate::density::ChunkMaterialSamples;
use crate::stored_biome::SurfaceBiomeResolver;
use crate::BlockStateId;

const MIN_Y: i32 = SURFACE_RULE_LEVEL_MIN_Y;
/// Top block of the build height (`minY + height - 1`).
const TOP_Y: i32 = SURFACE_RULE_LEVEL_MIN_Y + SURFACE_RULE_LEVEL_HEIGHT.cast_signed() - 1;
/// First Y above the build height; `getBlock` there is `VOID_AIR`.
const VOID_Y: i32 = TOP_Y + 1;

/// Borrowed producer outputs for one centre chunk.
///
/// `corners` are caller-trusted (review condition A2): they must be the four
/// GEN-05.3 `preliminary_surface_level` values of the material's chunk, seed
/// and settings (`(16cx, 16cz)`, `(16cx+16, 16cz)`, `(16cx, 16cz+16)`,
/// `(16cx+16, 16cz+16)`). `PreliminarySurfaceCorners` carries no identity, so
/// the driver cannot check them; wrong corners change
/// `above_preliminary_surface` and the frozen-ocean range of a result that is
/// still published under the material's identity.
#[derive(Clone, Copy)]
pub struct SurfaceChunkInput<'a> {
    rules: &'a SeededSurfaceRules,
    resolver: &'a SurfaceBiomeResolver<'a>,
    temperatures: &'a BuiltinBiomeTemperatures,
    corners: PreliminarySurfaceCorners,
    material: &'a ChunkMaterialSamples,
}

impl<'a> SurfaceChunkInput<'a> {
    /// Bundles the rules binding (GEN-05.9), the complete 3×3 resolver
    /// (GEN-05.8), the temperature table (GEN-05.7), the caller-trusted
    /// preliminary corners (GEN-05.3) and the post-NOISE material (GEN-05.4).
    pub const fn new(
        rules: &'a SeededSurfaceRules,
        resolver: &'a SurfaceBiomeResolver<'a>,
        temperatures: &'a BuiltinBiomeTemperatures,
        corners: PreliminarySurfaceCorners,
        material: &'a ChunkMaterialSamples,
    ) -> Self {
        Self {
            rules,
            resolver,
            temperatures,
            corners,
            material,
        }
    }
}

/// Work and write counts of one published chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceChunkReport {
    work_used: u32,
    max_column_work: u32,
    session_work_used: u32,
    eroded_writes: u32,
    rule_writes: u32,
    frozen_writes: u32,
}

impl SurfaceChunkReport {
    /// Driver units spent (bounded by `SURFACE_CHUNK_WORK`).
    pub const fn work_used(self) -> u32 {
        self.work_used
    }
    /// Largest driver units of one column (bounded by `SURFACE_COLUMN_WORK`).
    pub const fn max_column_work(self) -> u32 {
        self.max_column_work
    }
    /// Units of the chunk's GEN-05.9 session (its own allowance).
    pub const fn session_work_used(self) -> u32 {
        self.session_work_used
    }
    /// Blocks written by the eroded-badlands prepass.
    pub const fn eroded_writes(self) -> u32 {
        self.eroded_writes
    }
    /// Blocks written from rule results.
    pub const fn rule_writes(self) -> u32 {
        self.rule_writes
    }
    /// Blocks written by the frozen-ocean postpass.
    pub const fn frozen_writes(self) -> u32 {
        self.frozen_writes
    }
}

/// Source of one block write, in the original write order of a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Eroded,
    Rule,
    Frozen,
}

/// Side-effect-free observation points; the production observer does nothing.
pub(super) trait Observer {
    fn column(&mut self, _local_x: u8, _local_z: u8, _biome: BiomeResourceId, _height_plus_1: i32) {
    }
    fn write(&mut self, _phase: Phase, _y: i32, _state: BlockStateId) {}
    fn loop_start(&mut self, _height_plus_1: i32, _session: &SurfaceRuleSession<'_>) {}
    fn before_evaluate(&mut self, _session: &SurfaceRuleSession<'_>) {}
    fn evaluated(
        &mut self,
        _input: SurfaceBlockInput,
        _outcome: SurfaceRuleOutcome,
        _session: &SurfaceRuleSession<'_>,
    ) {
    }
    fn column_end(&mut self, _session: &SurfaceRuleSession<'_>, _column_work: u32) {}
}

/// Production observer.
pub(super) struct Silent;

impl Observer for Silent {}

impl SeededSurfaceColumns {
    /// Drives the SURFACE step of one centre chunk into `buffer`.
    ///
    /// The buffer is first marked `Empty`. Refusal order, all before any column
    /// work: `InconsistentBinding` (seed, settings of `rules`),
    /// `InconsistentMaterial` (seed, settings, geometry `(-64, 384)`, chunk =
    /// resolver centre), the GEN-05.9 session admission as `Rules(kind)` (seed,
    /// settings, source, geometry, zoom, 3×3 domain), then `WorkBudgetExceeded`
    /// when `budget` is below `SURFACE_CHUNK_WORK` (D2). A defensive failure
    /// inside the loop (`UnclassifiedState`, `Noise`, `Temperature`, `Rules`)
    /// leaves the buffer `Empty` (D3). Only a completely driven chunk is
    /// published. One fresh session per chunk with the contract allowance
    /// `SURFACE_RULE_OVERWORLD_CHUNK_WORK`; no heap allocation.
    pub fn build_chunk(
        &self,
        input: SurfaceChunkInput<'_>,
        budget: SurfaceColumnWorkBudget,
        buffer: &mut SurfaceChunkBuffer,
    ) -> Result<SurfaceChunkReport, SurfaceColumnError> {
        self.build_observed(input, budget, buffer, &mut Silent)
    }

    pub(super) fn build_observed<O: Observer>(
        &self,
        input: SurfaceChunkInput<'_>,
        budget: SurfaceColumnWorkBudget,
        buffer: &mut SurfaceChunkBuffer,
        observer: &mut O,
    ) -> Result<SurfaceChunkReport, SurfaceColumnError> {
        buffer.published = None;
        let chunk = input.material.chunk_pos();
        self.admit(input).map_err(|error| error.with_chunk(chunk))?;
        let rules = |error: SurfaceRuleError| {
            SurfaceColumnError::new(Kind::Rules(error.kind().clone())).with_chunk(chunk)
        };
        let session = SurfaceRuleSession::new(
            input.rules,
            input.resolver,
            input.temperatures,
            input.corners,
            SurfaceRuleWorkBudget::new(SURFACE_RULE_OVERWORLD_CHUNK_WORK).map_err(rules)?,
        )
        .map_err(rules)?;
        if budget.units() < SURFACE_CHUNK_WORK {
            return Err(SurfaceColumnError::new(Kind::WorkBudgetExceeded)
                .with_chunk(chunk)
                .with_counts(SURFACE_CHUNK_WORK as usize, budget.units() as usize));
        }
        let mut driver = Driver {
            columns: self,
            input,
            session,
            buffer,
            observer,
            chunk,
            work: 0,
            max_column_work: 0,
            writes: [0; 3],
        };
        match driver.run() {
            Ok(()) => {
                let report = SurfaceChunkReport {
                    work_used: driver.work,
                    max_column_work: driver.max_column_work,
                    session_work_used: driver.session.work_used(),
                    eroded_writes: driver.writes[Phase::Eroded as usize],
                    rule_writes: driver.writes[Phase::Rule as usize],
                    frozen_writes: driver.writes[Phase::Frozen as usize],
                };
                driver.buffer.published = Some(Published {
                    chunk,
                    seed: self.seed(),
                    resource: self.resource(),
                });
                Ok(report)
            }
            Err(error) => {
                driver.buffer.published = None;
                Err(error.with_chunk(chunk))
            }
        }
    }

    /// Binding, then material consistency (contract §9 steps 1–2).
    fn admit(&self, input: SurfaceChunkInput<'_>) -> Result<(), SurfaceColumnError> {
        let binding = |mismatch| SurfaceColumnError::new(Kind::InconsistentBinding(mismatch));
        if self.seed() != input.rules.seed() {
            return Err(binding(BindingMismatch::Seed));
        }
        if self.resource() != input.rules.template().resource() {
            return Err(binding(BindingMismatch::Settings));
        }
        let material = input.material;
        let mismatch = |mismatch| SurfaceColumnError::new(Kind::InconsistentMaterial(mismatch));
        if material.seed() != self.seed() {
            return Err(mismatch(MaterialMismatch::Seed));
        }
        if material.resource() != self.resource() {
            return Err(mismatch(MaterialMismatch::Settings));
        }
        let geometry = material.geometry();
        if geometry.min_y() != MIN_Y
            || geometry.height() != SURFACE_RULE_LEVEL_HEIGHT
            || material.block_states().len() != SURFACE_CHUNK_STATES
            || material.fluid_update_bitmap().len() != SURFACE_CHUNK_FLUID_WORDS
        {
            return Err(mismatch(MaterialMismatch::Geometry));
        }
        if material.chunk_pos() != input.resolver.center() {
            return Err(mismatch(MaterialMismatch::Chunk));
        }
        Ok(())
    }
}

/// Exclusive per-chunk driver state; lives on the stack.
struct Driver<'d, 'a, O> {
    columns: &'d SeededSurfaceColumns,
    input: SurfaceChunkInput<'a>,
    session: SurfaceRuleSession<'a>,
    buffer: &'d mut SurfaceChunkBuffer,
    observer: &'d mut O,
    chunk: ChunkPos,
    work: u32,
    max_column_work: u32,
    writes: [u32; 3],
}

/// Material-order index `((y + 64) · 16 + lx) · 16 + lz` of an in-height block.
const fn index(local_x: usize, y: i32, local_z: usize) -> usize {
    ((y - MIN_Y) as usize * 16 + local_x) * 16 + local_z
}

impl<O: Observer> Driver<'_, '_, O> {
    fn run(&mut self) -> Result<(), SurfaceColumnError> {
        // Contract §3 step 1: copy the material, then derive the live heights.
        self.buffer
            .states
            .copy_from_slice(self.input.material.block_states());
        self.work += SURFACE_CHUNK_STATES as u32;
        self.buffer
            .fluid
            .copy_from_slice(self.input.material.fluid_update_bitmap());
        self.work += SURFACE_CHUNK_FLUID_WORDS as u32;
        for local_x in 0..16 {
            for local_z in 0..16 {
                let top = self.scan_top(local_x, TOP_Y, local_z)?;
                self.buffer.heights.set(local_x, local_z, top);
            }
        }
        for local_x in 0..16_u8 {
            for local_z in 0..16_u8 {
                let before = self.work;
                self.column(local_x, local_z)
                    .map_err(|error| error.with_column(local_x, local_z))?;
                let used = self.work - before;
                self.max_column_work = self.max_column_work.max(used);
                self.observer.column_end(&self.session, used);
            }
        }
        Ok(())
    }

    /// First non-air Y at or below `from`, or `−65` (one unit per visited Y).
    fn scan_top(
        &mut self,
        local_x: usize,
        from: i32,
        local_z: usize,
    ) -> Result<i32, SurfaceColumnError> {
        let mut y = from;
        while y >= MIN_Y {
            self.work += 1;
            if !self.class_at(local_x, y, local_z)?.air() {
                return Ok(y);
            }
            y -= 1;
        }
        Ok(MIN_Y - 1)
    }

    fn classify(
        &self,
        state: BlockStateId,
        pos: BlockPos,
    ) -> Result<StateClass, SurfaceColumnError> {
        self.columns
            .alphabet
            .classify(state)
            .ok_or_else(|| SurfaceColumnError::new(Kind::UnclassifiedState).with_block(pos))
    }

    /// Original `getBlock`: `VOID_AIR` outside the build height.
    fn class_at(
        &self,
        local_x: usize,
        y: i32,
        local_z: usize,
    ) -> Result<StateClass, SurfaceColumnError> {
        if !(MIN_Y..=TOP_Y).contains(&y) {
            return Ok(StateClass::VOID_AIR);
        }
        let state = self.buffer.states[index(local_x, y, local_z)];
        self.classify(state, self.block(local_x, y, local_z))
    }

    /// Absolute block of a local column with Java `int` wrapping.
    fn block(&self, local_x: usize, y: i32, local_z: usize) -> BlockPos {
        // Local coordinates are below 16; the fallback is unreachable.
        let local = |value: usize| i32::try_from(value).unwrap_or(i32::MAX);
        BlockPos::new(
            self.chunk.x().wrapping_mul(16).wrapping_add(local(local_x)),
            y,
            self.chunk.z().wrapping_mul(16).wrapping_add(local(local_z)),
        )
    }

    fn rules(error: &SurfaceRuleError) -> SurfaceColumnError {
        let failure = SurfaceColumnError::new(Kind::Rules(error.kind().clone()));
        match error.block() {
            Some(block) => failure.with_block(block),
            None => failure,
        }
    }

    /// Contract §3 step 2 for one column.
    fn column(&mut self, local_x: u8, local_z: u8) -> Result<(), SurfaceColumnError> {
        let pos = SurfaceColumnPos::new(self.chunk, local_x, local_z).ok_or_else(|| {
            SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::ColumnOutsideCentre))
        })?;
        let (lx, lz) = (usize::from(local_x), usize::from(local_z));
        let (x, z) = (pos.block_x(), pos.block_z());
        let i4 = self.buffer.heights.at(lx, lz).wrapping_add(1);
        self.work += 1;
        let biome = self
            .input
            .resolver
            .biome_at_block(BlockPos::new(x, i4, z))
            .map_err(|error| {
                SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::Biome(error.kind())))
                    .with_block(BlockPos::new(x, i4, z))
            })?;
        self.observer.column(local_x, local_z, biome, i4);
        if biome == self.columns.eroded_badlands {
            self.eroded(lx, lz, i4)?;
        }
        let i5 = self.buffer.heights.at(lx, lz).wrapping_add(1);
        self.session
            .begin_column(pos)
            .map_err(|error| Self::rules(&error))?;
        self.observer.loop_start(i5, &self.session);
        self.column_loop(lx, lz, i5)?;
        if biome == self.columns.frozen_ocean || biome == self.columns.deep_frozen_ocean {
            let min_surface = self
                .session
                .min_surface_level()
                .map_err(|error| Self::rules(&error))?;
            self.frozen(lx, lz, i4, min_surface, biome)?;
        }
        Ok(())
    }

    /// Original stone-depth / water-height loop from `i5` down to −64.
    fn column_loop(&mut self, lx: usize, lz: usize, i5: i32) -> Result<(), SurfaceColumnError> {
        let mut above = 0_i32;
        let mut water: Option<i32> = None;
        // Original i8 = Integer.MAX_VALUE: the first stone visit searches.
        let mut run_bottom = i32::MAX;
        let mut y = i5;
        while y >= MIN_Y {
            self.work += 1;
            let class = self.class_at(lx, y, lz)?;
            if class.air() {
                above = 0;
                water = None;
            } else if class.fluid() {
                if water.is_none() {
                    water = Some(y.wrapping_add(1));
                }
            } else {
                if run_bottom >= y {
                    // Stone-below search down to minY - 1, which reads VOID_AIR.
                    let mut t = y - 1;
                    loop {
                        self.work += 1;
                        if t < MIN_Y {
                            break;
                        }
                        let below = self.class_at(lx, t, lz)?;
                        if below.air() || below.fluid() {
                            break;
                        }
                        t -= 1;
                    }
                    run_bottom = t + 1;
                }
                above += 1;
                if class.default_state() {
                    let input = SurfaceBlockInput::new(y, above, y - run_bottom + 1, water);
                    self.observer.before_evaluate(&self.session);
                    let outcome = self
                        .session
                        .evaluate(input, &self.buffer.heights)
                        .map_err(|error| Self::rules(&error))?;
                    self.observer.evaluated(input, outcome, &self.session);
                    if let SurfaceRuleOutcome::State(state) = outcome {
                        self.write(lx, y, lz, state, Phase::Rule)?;
                    }
                }
            }
            y -= 1;
        }
        Ok(())
    }

    /// Original `BlockColumn.setBlock` + `ProtoChunk.setBlockState` effects:
    /// inside the build height only, the post-processing flag for a non-empty
    /// fluid, and `Heightmap.update` for `WORLD_SURFACE_WG`.
    fn write(
        &mut self,
        lx: usize,
        y: i32,
        lz: usize,
        state: BlockStateId,
        phase: Phase,
    ) -> Result<(), SurfaceColumnError> {
        if !(MIN_Y..=TOP_Y).contains(&y) {
            return Ok(());
        }
        self.work += 1;
        let class = self.classify(state, self.block(lx, y, lz))?;
        let at = index(lx, y, lz);
        self.buffer.states[at] = state;
        if class.fluid() {
            self.buffer.fluid[at / 64] |= 1_u64 << (at % 64);
        }
        let top = self.buffer.heights.at(lx, lz);
        if !class.air() {
            if y > top {
                self.buffer.heights.set(lx, lz, y);
            }
        } else if y == top {
            // Non-opaque write exactly at the top: rescan downward.
            let new_top = self.scan_top(lx, y - 1, lz)?;
            self.buffer.heights.set(lx, lz, new_top);
        }
        self.writes[phase as usize] += 1;
        self.observer.write(phase, y, state);
        Ok(())
    }

    fn noise(&mut self, slot: usize, x: f64, z: f64) -> Result<f64, SurfaceColumnError> {
        self.work += 1;
        self.columns
            .noises
            .get(slot)
            .ok_or_else(|| SurfaceColumnError::new(Kind::Noise))?
            .sample(Vec3::new(x, 0.0, z))
            .map_err(|_| SurfaceColumnError::new(Kind::Noise))
    }

    /// Original `erodedBadlandsExtension` (contract §6.1).
    fn eroded(&mut self, lx: usize, lz: usize, i4: i32) -> Result<(), SurfaceColumnError> {
        let column = self.block(lx, 0, lz);
        let (bx, bz) = (f64::from(column.x()), f64::from(column.z()));
        let surface = self.noise(BADLANDS_SURFACE, bx, bz)?;
        let pillar = self.noise(BADLANDS_PILLAR, bx * 0.2, bz * 0.2)?;
        let min = java_min((surface * 8.25).abs(), pillar * 15.0);
        if min <= 0.0 {
            return Ok(());
        }
        let roof = self.noise(BADLANDS_PILLAR_ROOF, bx * 0.75, bz * 0.75)?;
        let floor = eroded_floor(min, roof);
        self.eroded_fill(lx, lz, i4, floor)
    }

    /// Scan and fill of the eroded prepass from `min(floor, 319)` (§6.1 steps 2–4).
    fn eroded_fill(
        &mut self,
        lx: usize,
        lz: usize,
        i4: i32,
        floor: i32,
    ) -> Result<(), SurfaceColumnError> {
        if i4 > floor {
            return Ok(());
        }
        // Above 319 the original reads VOID_AIR (neither stone nor water) and
        // drops writes, so both loops start at min(floor, 319) (contract §6.1).
        let start = floor.min(TOP_Y);
        let mut y = start;
        while y >= MIN_Y {
            self.work += 1;
            let class = self.class_at(lx, y, lz)?;
            if class.default_block() {
                break;
            }
            if class.water_block() {
                return Ok(());
            }
            y -= 1;
        }
        let mut y = start;
        while y >= MIN_Y {
            self.work += 1;
            if !self.class_at(lx, y, lz)?.air() {
                break;
            }
            self.write(lx, y, lz, self.columns.default_block, Phase::Eroded)?;
            y -= 1;
        }
        Ok(())
    }

    /// Original `frozenOceanExtension` (contract §6.2) with the stale `i4`.
    fn frozen(
        &mut self,
        lx: usize,
        lz: usize,
        i4: i32,
        min_surface: i32,
        biome: BiomeResourceId,
    ) -> Result<(), SurfaceColumnError> {
        let column = self.block(lx, 0, lz);
        let (x, z) = (column.x(), column.z());
        let (bx, bz) = (f64::from(x), f64::from(z));
        let surface = self.noise(ICEBERG_SURFACE, bx, bz)?;
        let pillar = self.noise(ICEBERG_PILLAR, bx * 1.28, bz * 1.28)?;
        let min = java_min((surface * 8.25).abs(), pillar * 15.0);
        if min <= 1.8 {
            return Ok(());
        }
        let roof = self.noise(ICEBERG_PILLAR_ROOF, bx * 1.17, bz * 1.17)?;
        let sea = self.columns.sea_level;
        self.work += 1;
        let melt = self
            .input
            .temperatures
            .sample(biome, BlockPos::new(x, sea, z), sea)
            .map_err(|error| SurfaceColumnError::new(Kind::Temperature(error.kind().clone())))?
            .should_melt_iceberg_slightly();
        let shape = frozen_shape(min, roof, melt, sea);
        let mut random = self.input.rules.root.at(BlockPos::new(x, 0, z));
        let draw = |random: &mut crate::random::WorldgenRandomSource, bound| {
            random
                .next_int_bounded(bound)
                .map_err(|_| SurfaceColumnError::new(Kind::Noise))
        };
        self.work += 2;
        let snow_limit = 2_i32.wrapping_add(draw(&mut random, 4)?);
        let snow_above = sea.wrapping_add(18).wrapping_add(draw(&mut random, 10)?);
        let start = i4.max((shape.top as i32).wrapping_add(1));
        self.frozen_fill(
            lx,
            lz,
            start,
            min_surface,
            shape,
            snow_limit,
            snow_above,
            &mut random,
        )
    }

    /// Iceberg loop from `min(start, 320)` down to `max(min_surface, −64)`
    /// (review condition A4); Java short-circuit order, one draw per Y at most.
    #[allow(clippy::too_many_arguments)] // the original loop's locals
    fn frozen_fill(
        &mut self,
        lx: usize,
        lz: usize,
        start: i32,
        min_surface: i32,
        shape: FrozenShape,
        snow_limit: i32,
        snow_above: i32,
        random: &mut crate::random::WorldgenRandomSource,
    ) -> Result<(), SurfaceColumnError> {
        // start = max(i4, (int) d4 + 1) with i4 ≤ 320. Iterations above 320 read
        // VOID_AIR and would draw while y < (int) d4; skipping them silently
        // would change the draw sequence, so such a start is a defensive
        // refusal (unreachable through the noises' max_value) rather than a clamp.
        if start > VOID_Y {
            return Err(
                SurfaceColumnError::new(Kind::Noise).with_counts(start as usize, VOID_Y as usize)
            );
        }
        let sea = self.columns.sea_level;
        let (d4, d3) = (shape.top as i32, shape.bottom as i32);
        let mut snow = 0_i32;
        let low = min_surface.max(MIN_Y);
        let mut y = start.min(VOID_Y);
        while y >= low {
            self.work += 1;
            let class = self.class_at(lx, y, lz)?;
            let hit = class.air() && y < d4 && {
                self.work += 1;
                paper_double(random) > 0.01
            } || class.water_block() && y > d3 && y < sea && shape.bottom != 0.0 && {
                self.work += 1;
                paper_double(random) > 0.15
            };
            if hit {
                // The counter advances even when a write above 319 is dropped.
                if snow <= snow_limit && y > snow_above {
                    self.write(lx, y, lz, self.columns.snow_block, Phase::Frozen)?;
                    snow += 1;
                } else {
                    self.write(lx, y, lz, self.columns.packed_ice, Phase::Frozen)?;
                }
            }
            y -= 1;
        }
        Ok(())
    }
}

/// `(d4, d3)` of the frozen-ocean postpass: `top` is `min1` after the sea
/// offset (or 0), `bottom` is `d3` (or 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FrozenShape {
    pub top: f64,
    pub bottom: f64,
}

/// Contract §6.2 steps 2–3 in original IEEE `f64` order.
pub(super) fn frozen_shape(min: f64, roof: f64, melt: bool, sea: i32) -> FrozenShape {
    let abs = (roof * 1.5).abs();
    let mut min1 = java_min(min * min * 1.2, (abs * 40.0).ceil() + 14.0);
    if melt {
        min1 -= 2.0;
    }
    if min1 > 2.0 {
        FrozenShape {
            bottom: f64::from(sea) - min1 - 7.0,
            top: min1 + f64::from(sea),
        }
    } else {
        FrozenShape {
            top: 0.0,
            bottom: 0.0,
        }
    }
}

/// Contract §6.1 step 2: `Mth.floor(64 + min(min² · 2.5, ceil(|roof · 1.5| · 50) + 24))`.
pub(super) fn eroded_floor(min: f64, roof: f64) -> i32 {
    let abs = (roof * 1.5).abs();
    java_floor(64.0 + java_min(min * min * 2.5, (abs * 50.0).ceil() + 24.0))
}

/// Paper build 60 `nextDouble`, recompiled through f32 (`l2f; fmul; f2d`, P1).
pub(super) fn paper_double(random: &mut crate::random::WorldgenRandomSource) -> f64 {
    f64::from(random.next_double() as f32)
}

/// Original `Math.min(double, double)` including NaN and signed-zero rules.
pub(super) fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if a == 0.0 && b == 0.0 && b.to_bits() == (-0.0_f64).to_bits() {
        return b;
    }
    if a <= b {
        a
    } else {
        b
    }
}

/// Original `Mth.floor(double)` with Java `(int)` truncation and saturation.
pub(super) fn java_floor(value: f64) -> i32 {
    let truncated = value as i32;
    if value < f64::from(truncated) {
        truncated.wrapping_sub(1)
    } else {
        truncated
    }
}

/// Private contract-test controls (C3/C5): the driver's own loops and write
/// path on a caller-prepared buffer, outside any original comparison.
#[cfg(test)]
pub(super) mod controls {
    use super::*;

    /// A driver over a prepared buffer; live heights are derived from its states.
    pub(in super::super) struct Synthetic<'s, 'a, O> {
        driver: Driver<'s, 'a, O>,
    }

    impl<'s, 'a, O: Observer> Synthetic<'s, 'a, O> {
        pub fn new(
            columns: &'s SeededSurfaceColumns,
            input: SurfaceChunkInput<'a>,
            session: SurfaceRuleSession<'a>,
            buffer: &'s mut SurfaceChunkBuffer,
            observer: &'s mut O,
        ) -> Result<Self, SurfaceColumnError> {
            let chunk = input.resolver.center();
            let mut driver = Driver {
                columns,
                input,
                session,
                buffer,
                observer,
                chunk,
                work: 0,
                max_column_work: 0,
                writes: [0; 3],
            };
            for local_x in 0..16 {
                for local_z in 0..16 {
                    let top = driver.scan_top(local_x, TOP_Y, local_z)?;
                    driver.buffer.heights.set(local_x, local_z, top);
                }
            }
            Ok(Self { driver })
        }
        pub fn eroded_fill(
            &mut self,
            lx: usize,
            lz: usize,
            i4: i32,
            floor: i32,
        ) -> Result<(), SurfaceColumnError> {
            self.driver.eroded_fill(lx, lz, i4, floor)
        }
        #[allow(clippy::too_many_arguments)] // the original loop's locals
        pub fn frozen_fill(
            &mut self,
            lx: usize,
            lz: usize,
            start: i32,
            min_surface: i32,
            shape: FrozenShape,
            snow_limit: i32,
            snow_above: i32,
            random: &mut crate::random::WorldgenRandomSource,
        ) -> Result<(), SurfaceColumnError> {
            self.driver.frozen_fill(
                lx,
                lz,
                start,
                min_surface,
                shape,
                snow_limit,
                snow_above,
                random,
            )
        }
        pub fn column_loop(
            &mut self,
            lx: usize,
            lz: usize,
            i5: i32,
        ) -> Result<(), SurfaceColumnError> {
            self.driver.column_loop(lx, lz, i5)
        }
        pub fn write(
            &mut self,
            lx: usize,
            y: i32,
            lz: usize,
            state: BlockStateId,
        ) -> Result<(), SurfaceColumnError> {
            self.driver.write(lx, y, lz, state, Phase::Rule)
        }
        pub fn begin_column(&mut self, lx: u8, lz: u8) {
            let pos = SurfaceColumnPos::new(self.driver.chunk, lx, lz).unwrap();
            self.driver.session.begin_column(pos).unwrap();
        }
        pub const fn work(&self) -> u32 {
            self.driver.work
        }
        pub fn reset_work(&mut self) {
            self.driver.work = 0;
        }
        pub const fn height(&self, lx: usize, lz: usize) -> i32 {
            self.driver.buffer.heights.at(lx, lz)
        }
        pub fn state(&self, lx: usize, y: i32, lz: usize) -> BlockStateId {
            self.driver.buffer.states[index(lx, y, lz)]
        }
        pub fn fluid(&self, lx: usize, y: i32, lz: usize) -> bool {
            let at = index(lx, y, lz);
            self.driver.buffer.fluid[at / 64] & (1 << (at % 64)) != 0
        }
        pub const fn writes(&self) -> [u32; 3] {
            self.driver.writes
        }
    }
}
