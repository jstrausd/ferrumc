//! Original f32 geometry and target-only ordered mutations.

use ferrumc_math::{BlockPos, ChunkPos};

use crate::biome_selection::BiomeClimateTarget;
use crate::random::WorldgenRandomSource;
use crate::surface_rules::SurfaceChunkBuffer;
use crate::surface_rules::{
    RuleCoreError, SurfaceBlockInput, SurfaceColumnPos, SurfaceRuleOutcome,
};
use crate::{BlockStateId, WorldSeed};

use super::buffer::{mask_index, state_index};
use super::sin::{cos, sin};
use super::{
    biome_error, noise_error, rules_error, surface_mismatch, CarvedChunkBuffer, CarverClimateQuart,
    CarverError, CarverErrorKind, CarverReport, CarverWorkBudget, DirectCarverBiomes, GeometryWork,
    PreparedCarverRun, SeededCarvers, SurfaceMismatch,
};

#[expect(
    clippy::too_many_lines,
    reason = "The closed source and carver loops retain original order and trace projections."
)]
pub(super) fn execute(
    binding: &SeededCarvers,
    surface: &SurfaceChunkBuffer,
    run: &mut PreparedCarverRun<'_>,
    budget: CarverWorkBudget,
    output: &mut CarvedChunkBuffer,
) -> Result<CarverReport, CarverError> {
    let mut work = GeometryWork {
        limit: budget.units(),
        used: 0,
    };
    // Empty transition is charged here; admission already removed publication.
    work.charge()?;
    let (states, fluid, heights) = surface
        .carver_contents()
        .ok_or_else(|| surface_mismatch(SurfaceMismatch::NotPublished))?;
    for (destination, source) in output.states.iter_mut().zip(states) {
        work.charge()?;
        *destination = *source;
    }
    for (destination, source) in output.fluid.iter_mut().zip(fluid) {
        work.charge()?;
        *destination = *source;
    }
    for x in 0..16 {
        for z in 0..16 {
            work.charge()?;
            output.heights.set(x, z, heights.at(x, z));
        }
    }
    for word in &mut output.mask {
        work.charge()?;
        *word = 0;
    }
    #[cfg(test)]
    if let Some(trace) = &mut run.trace {
        trace.shadow = states.to_vec();
    }
    let mut engine = Engine {
        binding,
        run,
        output,
        work,
        report: CarverReport::default(),
    };
    for dx in -8..=8 {
        for dz in -8..=8 {
            engine.work.charge()?;
            let request = CarverClimateQuart::source(engine.run.chunk, dx, dz)?;
            let climate = engine
                .run
                .noise
                .sample_carver_climate(request)
                .map_err(noise_error)?;
            let selected = engine
                .run
                .selector
                .select_core(BiomeClimateTarget::from_climate(climate))
                .map_err(biome_error)?;
            #[cfg(test)]
            if let Some(trace) = &mut engine.run.trace {
                let quart = BlockPos::new(
                    (engine.run.chunk.x() + dx) * 4,
                    0,
                    (engine.run.chunk.z() + dz) * 4,
                );
                trace.queries.push((
                    false,
                    BlockPos::new(quart.x() * 4, 0, quart.z() * 4),
                    quart,
                    selected.biome(),
                ));
            }
            #[cfg(not(test))]
            let _ = selected;
            let source = ChunkPos::new(engine.run.chunk.x() + dx, engine.run.chunk.z() + dz);
            for carver in 0..3 {
                let mut random = large_feature(binding.seed, carver, source);
                engine.work.charge()?;
                let start = random.next_float() <= [0.15_f32, 0.07_f32, 0.01_f32][carver];
                #[cfg(test)]
                if let Some(trace) = &mut engine.run.trace {
                    trace.starts.push(if start { '1' } else { '0' });
                }
                if start {
                    #[cfg(test)]
                    let before = engine.report;
                    #[cfg(test)]
                    let before_mask = engine
                        .run
                        .trace
                        .as_ref()
                        .map(|_| engine.output.mask.to_vec());
                    engine.report.starts += 1;
                    if carver == 2 {
                        engine.canyon(source, &mut random)?;
                    } else {
                        engine.cave(source, carver == 1, &mut random)?;
                    }
                    #[cfg(test)]
                    if let Some(trace) = &mut engine.run.trace {
                        let mut counts = [0_u64; 10];
                        let before_mask = before_mask.as_ref().unwrap();
                        for (word, after) in engine.output.mask.iter().enumerate() {
                            let mut bits = after & !before_mask[word];
                            while bits != 0 {
                                let bit = word * 64 + bits.trailing_zeros() as usize;
                                bits &= bits - 1;
                                counts[0] += 1;
                                let lx = bit & 15;
                                let lz = bit >> 4 & 15;
                                let y = i32::try_from(bit >> 8).unwrap() - 64;
                                for yy in [y, y - 1] {
                                    if yy < -64 {
                                        continue;
                                    }
                                    let index = ((yy + 64) as usize * 16 + lx) * 16 + lz;
                                    let now = engine.output.states[index];
                                    if trace.shadow[index] == now {
                                        continue;
                                    }
                                    trace.shadow[index] = now;
                                    counts[1] += 1;
                                    let name =
                                        ferrumc_registry::block_state::state_id_to_block_name(
                                            now.as_u32(),
                                        );
                                    let category = if yy == y {
                                        match name {
                                            Some("air") => 2,
                                            Some("water") => 3,
                                            Some("lava") => 4,
                                            _ => 6,
                                        }
                                    } else {
                                        5
                                    };
                                    counts[category] += 1;
                                }
                            }
                        }
                        counts[7] = engine.report.aquifer_calls - before.aquifer_calls;
                        counts[8] = engine.report.schedule_reads - before.schedule_reads;
                        counts[9] = engine.report.fixups - before.fixups;
                        trace.carves.push((source, carver, counts));
                    }
                }
            }
        }
    }
    engine.work.charge()?;
    engine.report.work = engine.work.used;
    Ok(engine.report)
}

pub(super) fn large_feature(
    seed: WorldSeed,
    carver: usize,
    source: ChunkPos,
) -> WorldgenRandomSource {
    let base = seed.value().wrapping_add((carver as u64).cast_signed());
    let mut random = WorldgenRandomSource::legacy(WorldSeed::new(base));
    let a = random.next_long();
    let b = random.next_long();
    let selected =
        i64::from(source.x()).wrapping_mul(a) ^ i64::from(source.z()).wrapping_mul(b) ^ base;
    random.set_seed(WorldSeed::new(selected));
    random
}
fn int(random: &mut WorldgenRandomSource, bound: u32) -> i32 {
    random.bounded_positive(bound).cast_signed()
}
fn uniform(random: &mut WorldgenRandomSource, lo: f32, hi: f32) -> f32 {
    uniform_draw(random.next_float(), lo, hi)
}
pub(super) fn uniform_draw(draw: f32, lo: f32, hi: f32) -> f32 {
    draw * (hi - lo) + lo
}
pub(super) fn trapezoid_draw(first: f32, second: f32) -> f32 {
    (0.0_f32 + first * 4.0_f32) + second * 2.0_f32
}
pub(super) fn thickness_base(first: f32, second: f32) -> f32 {
    first * 2.0_f32 + second
}
pub(super) fn thickness_expand(base: f32, first: f32, second: f32) -> f32 {
    base * (first * second * 3.0_f32 + 1.0_f32)
}
fn floor(value: f64) -> i32 {
    let i = value as i32;
    if value < f64::from(i) {
        i.wrapping_sub(1)
    } else {
        i
    }
}

struct Engine<'call, 'prepared> {
    binding: &'call SeededCarvers,
    run: &'call mut PreparedCarverRun<'prepared>,
    output: &'call mut CarvedChunkBuffer,
    work: GeometryWork,
    report: CarverReport,
}
#[derive(Clone, Copy)]
struct Tunnel {
    x: f64,
    y: f64,
    z: f64,
    hm: f64,
    vm: f64,
    thickness: f32,
    yaw: f32,
    pitch: f32,
    start: i32,
    count: i32,
    ratio: f64,
    floor: f64,
}
#[derive(Clone, Copy)]
enum Skip<'a> {
    Cave(f64),
    Canyon(&'a [f32; 384]),
}
impl Skip<'_> {
    fn excludes(&self, x: f64, y: f64, z: f64, block_y: i32) -> bool {
        match self {
            Self::Cave(min) => y <= *min || x * x + y * y + z * z >= 1.0,
            Self::Canyon(widths) => {
                (x * x + z * z) * f64::from(widths[(block_y + 63) as usize]) + y * y / 6.0 >= 1.0
            }
        }
    }
}
impl Engine<'_, '_> {
    fn cave(
        &mut self,
        source: ChunkPos,
        extra: bool,
        random: &mut WorldgenRandomSource,
    ) -> Result<(), CarverError> {
        let first = int(random, 15) + 1;
        let second = int(random, first as u32) + 1;
        let caves = int(random, second as u32);
        for _ in 0..caves {
            let x = f64::from(source.x() * 16 + int(random, 16));
            let y = f64::from(int(random, if extra { 104 } else { 237 }) - 56);
            let z = f64::from(source.z() * 16 + int(random, 16));
            let hm = f64::from(uniform(random, 0.7, 1.4));
            let vm = f64::from(uniform(random, 0.8, 1.3));
            let bottom = f64::from(uniform(random, -1.0, -0.4));
            let mut tunnels = 1;
            if int(random, 4) == 0 {
                let ratio = f64::from(uniform(random, 0.1, 0.9));
                let radius = 1.0_f32 + random.next_float() * 6.0_f32;
                let horizontal = 1.5 + f64::from(sin(std::f32::consts::FRAC_PI_2) * radius);
                self.ellipsoid(
                    x + 1.0,
                    y,
                    z,
                    horizontal,
                    horizontal * ratio,
                    Skip::Cave(bottom),
                )?;
                tunnels += int(random, 4);
            }
            for _ in 0..tunnels {
                let yaw = random.next_float() * std::f32::consts::TAU;
                let pitch = (random.next_float() - 0.5_f32) / 4.0_f32;
                let mut thickness = thickness_base(random.next_float(), random.next_float());
                if int(random, 10) == 0 {
                    thickness =
                        thickness_expand(thickness, random.next_float(), random.next_float());
                }
                let count = 112 - int(random, 28);
                self.tunnel(
                    random.next_long(),
                    Tunnel {
                        x,
                        y,
                        z,
                        hm,
                        vm,
                        thickness,
                        yaw,
                        pitch,
                        start: 0,
                        count,
                        ratio: 1.0,
                        floor: bottom,
                    },
                )?;
            }
        }
        Ok(())
    }
    fn tunnel(&mut self, seed: i64, mut t: Tunnel) -> Result<(), CarverError> {
        #[cfg(test)]
        if let Some(trace) = &mut self.run.trace {
            trace.tunnels.push(t.thickness);
        }
        let mut random = WorldgenRandomSource::legacy(WorldSeed::new(seed));
        let branch = int(&mut random, (t.count / 2) as u32) + t.count / 4;
        let steep = int(&mut random, 6) == 0;
        let mut yaw_delta = 0.0_f32;
        let mut pitch_delta = 0.0_f32;
        for i in t.start..t.count {
            self.work.charge()?;
            let horizontal = 1.5
                + f64::from(sin(std::f32::consts::PI * i as f32 / t.count as f32) * t.thickness);
            let vertical = horizontal * t.ratio;
            let c = cos(t.pitch);
            t.x += f64::from(cos(t.yaw) * c);
            t.y += f64::from(sin(t.pitch));
            t.z += f64::from(sin(t.yaw) * c);
            t.pitch *= if steep { 0.92_f32 } else { 0.7_f32 };
            t.pitch += pitch_delta * 0.1_f32;
            t.yaw += yaw_delta * 0.1_f32;
            pitch_delta *= 0.9_f32;
            yaw_delta *= 0.75_f32;
            pitch_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 2.0_f32;
            yaw_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 4.0_f32;
            if i == branch && t.thickness > 1.0_f32 {
                let seed = random.next_long();
                let thickness = random.next_float() * 0.5_f32 + 0.5_f32;
                self.tunnel(
                    seed,
                    Tunnel {
                        thickness,
                        yaw: t.yaw - std::f32::consts::FRAC_PI_2,
                        pitch: t.pitch / 3.0_f32,
                        start: i,
                        ratio: 1.0,
                        ..t
                    },
                )?;
                let seed = random.next_long();
                let thickness = random.next_float() * 0.5_f32 + 0.5_f32;
                self.tunnel(
                    seed,
                    Tunnel {
                        thickness,
                        yaw: t.yaw + std::f32::consts::FRAC_PI_2,
                        pitch: t.pitch / 3.0_f32,
                        start: i,
                        ratio: 1.0,
                        ..t
                    },
                )?;
                return Ok(());
            }
            if int(&mut random, 4) != 0 {
                if !self.can_reach(t.x, t.z, i, t.count, t.thickness) {
                    return Ok(());
                }
                self.ellipsoid(
                    t.x,
                    t.y,
                    t.z,
                    horizontal * t.hm,
                    vertical * t.vm,
                    Skip::Cave(t.floor),
                )?;
            }
        }
        Ok(())
    }
    #[expect(
        clippy::many_single_char_names,
        reason = "Pinned Canyon arithmetic uses coordinate and trigonometric names."
    )]
    fn canyon(
        &mut self,
        source: ChunkPos,
        random: &mut WorldgenRandomSource,
    ) -> Result<(), CarverError> {
        let mut x = f64::from(source.x() * 16 + int(random, 16));
        let mut y = f64::from(int(random, 58) + 10);
        let mut z = f64::from(source.z() * 16 + int(random, 16));
        let mut yaw = random.next_float() * std::f32::consts::TAU;
        let mut pitch = uniform(random, -0.125, 0.125);
        // Original trapezoid(0,6,2): f1=2, f2=4, left-associated.
        let thickness = trapezoid_draw(random.next_float(), random.next_float());
        let count = (112.0_f32 * uniform(random, 0.75, 1.0)) as i32;
        let mut random = WorldgenRandomSource::legacy(WorldSeed::new(random.next_long()));
        let mut widths = [0.0_f32; 384];
        let mut width = 1.0_f32;
        for (i, slot) in widths.iter_mut().enumerate() {
            self.work.charge()?;
            if i == 0 || int(&mut random, 3) == 0 {
                width = 1.0_f32 + random.next_float() * random.next_float();
            }
            *slot = width * width;
        }
        let mut yaw_delta = 0.0_f32;
        let mut pitch_delta = 0.0_f32;
        for i in 0..count {
            self.work.charge()?;
            let mut horizontal =
                1.5 + f64::from(sin(i as f32 * std::f32::consts::PI / count as f32) * thickness);
            let mut vertical = horizontal * 3.0;
            horizontal *= f64::from(uniform(&mut random, 0.75, 1.0));
            let center = 1.0_f32 - (0.5_f32 - i as f32 / count as f32).abs() * 2.0_f32;
            let factor = 1.0_f32 + 0.0_f32 * center;
            vertical = f64::from(factor) * vertical * f64::from(uniform(&mut random, 0.75, 1.0));
            let c = cos(pitch);
            let s = sin(pitch);
            x += f64::from(cos(yaw) * c);
            y += f64::from(s);
            z += f64::from(sin(yaw) * c);
            pitch *= 0.7_f32;
            pitch += pitch_delta * 0.05_f32;
            yaw += yaw_delta * 0.05_f32;
            pitch_delta *= 0.8_f32;
            yaw_delta *= 0.5_f32;
            pitch_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 2.0_f32;
            yaw_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 4.0_f32;
            if int(&mut random, 4) != 0 {
                if !self.can_reach(x, z, i, count, thickness) {
                    return Ok(());
                }
                self.ellipsoid(x, y, z, horizontal, vertical, Skip::Canyon(&widths))?;
            }
        }
        Ok(())
    }
    fn can_reach(&self, x: f64, z: f64, i: i32, count: i32, width: f32) -> bool {
        let dx = x - f64::from(self.run.chunk.x() * 16 + 8);
        let dz = z - f64::from(self.run.chunk.z() * 16 + 8);
        let remaining = f64::from(count - i);
        let limit = f64::from(width + 2.0_f32 + 16.0_f32);
        dx * dx + dz * dz - remaining * remaining <= limit * limit
    }
    fn ellipsoid(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        hr: f64,
        vr: f64,
        skip: Skip<'_>,
    ) -> Result<(), CarverError> {
        self.work.charge()?;
        let origin_x = self.run.chunk.x() * 16;
        let origin_z = self.run.chunk.z() * 16;
        let extent = 16.0 + hr * 2.0;
        if (x - f64::from(origin_x + 8)).abs() > extent
            || (z - f64::from(origin_z + 8)).abs() > extent
        {
            return Ok(());
        }
        let x0 = (floor(x - hr) - origin_x - 1).max(0);
        let x1 = (floor(x + hr) - origin_x).min(15);
        let z0 = (floor(z - hr) - origin_z - 1).max(0);
        let z1 = (floor(z + hr) - origin_z).min(15);
        let y0 = (floor(y - vr) - 1).max(-63);
        let y1 = (floor(y + vr) + 1).min(312);
        for lx in x0..=x1 {
            let rx = (f64::from(origin_x + lx) + 0.5 - x) / hr;
            for lz in z0..=z1 {
                self.work.charge()?;
                let rz = (f64::from(origin_z + lz) + 0.5 - z) / hr;
                if rx * rx + rz * rz >= 1.0 {
                    continue;
                }
                let mut reached = false;
                for by in (y0 + 1..=y1).rev() {
                    self.work.charge()?;
                    let ry = (f64::from(by) - 0.5 - y) / vr;
                    if skip.excludes(rx, ry, rz, by) {
                        continue;
                    }
                    let pos = BlockPos::new(origin_x + lx, by, origin_z + lz);
                    let i = mask_index(pos);
                    if self.output.mask[i / 64] & (1 << (i % 64)) != 0 {
                        continue;
                    }
                    self.output.mask[i / 64] |= 1 << (i % 64);
                    self.report.mask_bits += 1;
                    self.block(pos, &mut reached)?;
                }
            }
        }
        Ok(())
    }
    fn state(&self, pos: BlockPos) -> Result<BlockStateId, CarverError> {
        state_index(pos)
            .and_then(|i| self.output.states.get(i))
            .copied()
            .ok_or_else(|| CarverError::new(CarverErrorKind::UnclassifiedState))
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Original mask, Aquifer, write and lazy fix-up order is kept together."
    )]
    fn block(&mut self, pos: BlockPos, reached: &mut bool) -> Result<(), CarverError> {
        self.work.charge()?;
        let old = self.binding.classify(self.state(pos)?)?;
        if old.surface {
            *reached = true;
        }
        if !old.replaceable {
            return Ok(());
        }
        let state = if pos.y() <= -56 {
            self.run.aquifer.lava()
        } else {
            self.work.charge()?;
            self.report.aquifer_calls += 1;
            let result = self
                .run
                .aquifer
                .compute(&mut self.run.noise, pos)
                .map_err(noise_error)?;
            #[cfg(test)]
            if let Some(trace) = &mut self.run.trace {
                trace.events.push(format!(
                    "A{},{},{},{},{}",
                    pos.x(),
                    pos.y(),
                    pos.z(),
                    result.map_or(-1, |s| i64::from(s.as_u32())),
                    u8::from(self.run.aquifer.schedule())
                ));
            }
            let Some(state) = result else {
                return Ok(());
            };
            state
        };
        self.write(pos, state)?;
        self.work.charge()?;
        #[cfg(test)]
        {
            self.report.schedule_reads += 1;
            if let Some(trace) = &mut self.run.trace {
                trace
                    .events
                    .push(format!("S{}", u8::from(self.run.aquifer.schedule())));
            }
        }
        let fluid = self.binding.classify(state)?.fluid;
        if self.run.aquifer.schedule() && fluid {
            self.mark(pos);
        }
        if *reached {
            let below = BlockPos::new(pos.x(), pos.y() - 1, pos.z());
            if self.binding.classify(self.state(below)?)?.dirt {
                self.work.charge()?;
                self.report.fixups += 1;
                self.run
                    .context
                    .begin_column(SurfaceColumnPos::from_block(below))
                    .map_err(rules_error)?;
                let mut direct = DirectCarverBiomes {
                    center: self.run.chunk,
                    noise: &mut self.run.noise,
                    selector: &mut self.run.selector,
                    zoom: self.run.zoom,
                    work: &mut self.work,
                    #[cfg(test)]
                    queries: self.run.trace.as_mut().map(|t| &mut t.queries),
                };
                let outcome = self
                    .run
                    .context
                    .evaluate_carver(
                        SurfaceBlockInput::new(
                            below.y(),
                            1,
                            1,
                            if fluid { Some(pos.y()) } else { None },
                        ),
                        &self.output.heights,
                        &mut direct,
                    )
                    .map_err(|error| match error {
                        RuleCoreError::Rule(error) => rules_error(error),
                        RuleCoreError::Direct(error) => error,
                    })?;
                #[cfg(test)]
                if let Some(trace) = &mut self.run.trace {
                    trace.events.push(format!(
                        "T{},{},{},{},{},{}",
                        below.x(),
                        below.y(),
                        below.z(),
                        if fluid { pos.y() } else { i32::MIN },
                        self.run.context.surface_depth().map_err(rules_error)?,
                        match outcome {
                            SurfaceRuleOutcome::State(s) => i64::from(s.as_u32()),
                            SurfaceRuleOutcome::NoRule => -1,
                        }
                    ));
                }
                if let SurfaceRuleOutcome::State(state) = outcome {
                    #[cfg(test)]
                    {
                        self.report.fixup_writes += 1;
                    }
                    self.write(below, state)?;
                    if self.binding.classify(state)?.fluid {
                        self.mark(below);
                    }
                }
            }
        }
        Ok(())
    }
    fn mark(&mut self, pos: BlockPos) {
        if let Some(i) = state_index(pos) {
            self.output.fluid[i / 64] |= 1 << (i % 64);
        }
    }
    fn write(&mut self, pos: BlockPos, state: BlockStateId) -> Result<(), CarverError> {
        self.work.charge()?;
        let index =
            state_index(pos).ok_or_else(|| CarverError::new(CarverErrorKind::UnclassifiedState))?;
        let entry = self.binding.classify(state)?;
        self.output.states[index] = state;
        self.report.writes += 1;
        #[cfg(test)]
        if let Some(trace) = &mut self.run.trace {
            trace.writes.push((pos, state));
        }
        let lx = (pos.x() & 15) as usize;
        let lz = (pos.z() & 15) as usize;
        let top = self.output.heights.at(lx, lz);
        if !entry.air && pos.y() > top {
            self.output.heights.set(lx, lz, pos.y());
        } else if entry.air && pos.y() == top {
            let mut replacement = -65;
            for y in (-64..pos.y()).rev() {
                self.work.charge()?;
                if !self
                    .binding
                    .classify(self.state(BlockPos::new(pos.x(), y, pos.z()))?)?
                    .air
                {
                    replacement = y;
                    break;
                }
            }
            self.output.heights.set(lx, lz, replacement);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
