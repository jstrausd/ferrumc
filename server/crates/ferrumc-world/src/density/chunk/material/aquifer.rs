//! Run-owned original grid aquifer, with lazy location and fluid-status caches.

use ferrumc_math::{BlockPos, ChunkPos};

use super::super::{
    bytes, core_charge as charge, core_fail as fail, limit, NoiseChunkErrorKind, NoiseChunkLimit,
};
use super::{
    clamped_map, finite, random_at, random_int, sample, MaterialBlocks, MaterialDomain,
    NoiseChunkError, NoiseChunkSession,
};
use crate::BlockStateId;

const MAX_SLOTS: usize = 512;
const MAX_CACHE_BYTES: usize = 12 * 1024;
const WAY_BELOW_MIN_Y: i32 = -32_512;
const SURFACE_OFFSETS: [ChunkPos; 13] = [
    ChunkPos::new(0, 0),
    ChunkPos::new(-2, -1),
    ChunkPos::new(-1, -1),
    ChunkPos::new(0, -1),
    ChunkPos::new(1, -1),
    ChunkPos::new(-3, 0),
    ChunkPos::new(-2, 0),
    ChunkPos::new(-1, 0),
    ChunkPos::new(1, 0),
    ChunkPos::new(-2, 1),
    ChunkPos::new(-1, 1),
    ChunkPos::new(0, 1),
    ChunkPos::new(1, 1),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct FluidStatus {
    level: i32,
    kind: BlockStateId,
}
impl FluidStatus {
    fn at(self, y: i32, air: BlockStateId) -> BlockStateId {
        if y < self.level {
            self.kind
        } else {
            air
        }
    }
}

#[cfg_attr(test, derive(Debug))]
pub(super) struct Aquifer {
    pub schedule: bool,
    enabled: bool,
    sea_level: i32,
    min_grid: BlockPos,
    size: [usize; 3],
    locations: Box<[i64]>,
    statuses: Box<[Option<FluidStatus>]>,
}
impl Aquifer {
    #[cfg(test)]
    pub(super) fn locations_len(&self) -> usize {
        self.locations.len()
    }
    #[cfg(test)]
    pub(super) fn statuses_len(&self) -> usize {
        self.statuses.len()
    }
    pub fn new(session: &mut NoiseChunkSession) -> Result<Self, NoiseChunkError> {
        charge(&mut session.work)?;
        let enabled = session.data.metadata.aquifers;
        let sea_level = session.data.metadata.sea_level;
        let origin = session.state.origin;
        let geometry = session.geometry();
        let min_grid = BlockPos::new(
            origin.x().div_euclid(16) - 1,
            geometry.min_y().div_euclid(12) - 1,
            origin.z().div_euclid(16) - 1,
        );
        let top = i64::from(geometry.min_y()) + i64::from(geometry.height());
        let max_y = i32::try_from(top)
            .map_err(|_| fail(NoiseChunkErrorKind::InvalidGraph))?
            .div_euclid(12)
            + 1;
        let size = [
            3,
            usize::try_from(max_y - min_grid.y() + 1)
                .map_err(|_| fail(NoiseChunkErrorKind::InvalidGraph))?,
            3,
        ];
        let slots = if enabled {
            size[0]
                .checked_mul(size[1])
                .and_then(|n| n.checked_mul(size[2]))
                .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?
        } else {
            0
        };
        limit(NoiseChunkLimit::AquiferSlots, slots, MAX_SLOTS)?;
        let total = bytes(
            NoiseChunkLimit::AquiferBytes,
            0,
            slots,
            std::mem::size_of::<i64>(),
            MAX_CACHE_BYTES,
        )?;
        bytes(
            NoiseChunkLimit::AquiferBytes,
            total,
            slots,
            std::mem::size_of::<Option<FluidStatus>>(),
            MAX_CACHE_BYTES,
        )?;
        for _ in 0..slots {
            charge(&mut session.work)?;
        }
        let statuses = vec![None; slots].into_boxed_slice();
        for _ in 0..slots {
            charge(&mut session.work)?;
        }
        let locations = vec![i64::MAX; slots].into_boxed_slice();
        Ok(Self {
            schedule: false,
            enabled,
            sea_level,
            min_grid,
            size,
            locations,
            statuses,
        })
    }
    fn index(&self, grid: BlockPos) -> Result<usize, NoiseChunkError> {
        let offsets = [
            i64::from(grid.x()) - i64::from(self.min_grid.x()),
            i64::from(grid.y()) - i64::from(self.min_grid.y()),
            i64::from(grid.z()) - i64::from(self.min_grid.z()),
        ];
        for (value, maximum) in offsets.iter().zip(self.size) {
            if *value < 0 || *value as u64 >= maximum as u64 {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        Ok(
            (offsets[1] as usize * self.size[2] + offsets[2] as usize) * self.size[0]
                + offsets[0] as usize,
        )
    }
    fn global(
        &self,
        session: &mut NoiseChunkSession,
        blocks: &MaterialBlocks,
        pos: BlockPos,
    ) -> Result<FluidStatus, NoiseChunkError> {
        charge(&mut session.work)?;
        let value = if pos.y() < (-54).min(self.sea_level) {
            FluidStatus {
                level: -54,
                kind: blocks.lava,
            }
        } else {
            FluidStatus {
                level: self.sea_level,
                kind: blocks.default_fluid,
            }
        };
        #[cfg(test)]
        let value = if let Some(picker) = session.state.material_picker.as_ref() {
            let (level, kind) = picker.at(pos);
            if let Some(requests) = session.state.requests.as_mut() {
                requests.record(serde_json::json!({"kind":"global_fluid","x":pos.x(),"y":pos.y(),"z":pos.z(),"height":level,"fluid_raw_state_id":kind.as_u32()}));
            }
            FluidStatus { level, kind }
        } else {
            value
        };
        Ok(value)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One ordered aquifer dispatch preserves the original lazy cache, pressure and fluid-flag branches."
    )]
    pub fn compute(
        &mut self,
        session: &mut NoiseChunkSession,
        blocks: &MaterialBlocks,
        domain: MaterialDomain,
        substance: f64,
    ) -> Result<Option<BlockStateId>, NoiseChunkError> {
        charge(&mut session.work)?;
        let pos = session.state.pos();
        finite(session, "aquifer_substance", substance)?;
        if !self.enabled {
            return if substance > 0.0 {
                Ok(None)
            } else {
                Ok(Some(
                    self.global(session, blocks, pos)?.at(pos.y(), blocks.air),
                ))
            };
        }
        if substance > 0.0 {
            self.schedule = false;
            return Ok(None);
        }
        if blocks.is_lava(self.global(session, blocks, pos)?.at(pos.y(), blocks.air)) {
            self.schedule = false;
            return Ok(Some(blocks.lava));
        }
        let base = BlockPos::new(
            (pos.x() - 5).div_euclid(16),
            (pos.y() + 1).div_euclid(12),
            (pos.z() - 5).div_euclid(16),
        );
        let mut distances = [i32::MAX; 4];
        let mut centers = [0_i64; 4];
        for dx in 0..=1 {
            for dy in -1..=1 {
                for dz in 0..=1 {
                    charge(&mut session.work)?;
                    let grid = BlockPos::new(base.x() + dx, base.y() + dy, base.z() + dz);
                    let index = self.index(grid)?;
                    charge(&mut session.work)?;
                    let mut packed = self.locations[index];
                    if packed == i64::MAX {
                        random_at(session, grid, "aquifer")?;
                        let mut random = session.data.aquifer_random.at(grid);
                        let x = random_int(session, &mut random, 10)?;
                        let y = random_int(session, &mut random, 9)?;
                        let z = random_int(session, &mut random, 10)?;
                        let center =
                            BlockPos::new(grid.x() * 16 + x, grid.y() * 12 + y, grid.z() * 16 + z);
                        domain.center(center)?;
                        packed = pack(center);
                        charge(&mut session.work)?;
                        self.locations[index] = packed;
                    }
                    let center = unpack(packed);
                    domain.center(center)?;
                    let x = center.x() - pos.x();
                    let y = center.y() - pos.y();
                    let z = center.z() - pos.z();
                    let distance = x * x + y * y + z * z;
                    for rank in 0..4 {
                        if distances[rank] >= distance {
                            for j in (rank + 1..4).rev() {
                                distances[j] = distances[j - 1];
                                centers[j] = centers[j - 1];
                            }
                            distances[rank] = distance;
                            centers[rank] = packed;
                            break;
                        }
                    }
                }
            }
        }
        let nearest = self.status(session, blocks, domain, centers[0])?;
        let first_similarity = similarity(distances[0], distances[1]);
        let state = nearest.at(pos.y(), blocks.air);
        let flowing = similarity(100, 144);
        if first_similarity <= 0.0 {
            self.schedule = first_similarity >= flowing
                && nearest != self.status(session, blocks, domain, centers[1])?;
            return Ok(Some(state));
        }
        if blocks.is_water(state)
            && blocks.is_lava(
                self.global(
                    session,
                    blocks,
                    BlockPos::new(pos.x(), pos.y() - 1, pos.z()),
                )?
                .at(pos.y() - 1, blocks.air),
            )
        {
            self.schedule = true;
            return Ok(Some(state));
        }
        let mut barrier = None;
        let second = self.status(session, blocks, domain, centers[1])?;
        let pressure =
            first_similarity * Self::pressure(session, blocks, &mut barrier, nearest, second)?;
        if Self::positive(session, substance, pressure)? {
            self.schedule = false;
            return Ok(None);
        }
        let third = self.status(session, blocks, domain, centers[2])?;
        let first_third = similarity(distances[0], distances[2]);
        if first_third > 0.0 {
            let pressure = first_similarity
                * first_third
                * Self::pressure(session, blocks, &mut barrier, nearest, third)?;
            if Self::positive(session, substance, pressure)? {
                self.schedule = false;
                return Ok(None);
            }
        }
        let second_third = similarity(distances[1], distances[2]);
        if second_third > 0.0 {
            let pressure = first_similarity
                * second_third
                * Self::pressure(session, blocks, &mut barrier, second, third)?;
            if Self::positive(session, substance, pressure)? {
                self.schedule = false;
                return Ok(None);
            }
        }
        let first_different = nearest != second;
        let second_different = second_third >= flowing && second != third;
        let third_different = first_third >= flowing && nearest != third;
        self.schedule = if !first_different && !second_different && !third_different {
            first_third >= flowing
                && similarity(distances[0], distances[3]) >= flowing
                && nearest != self.status(session, blocks, domain, centers[3])?
        } else {
            true
        };
        Ok(Some(state))
    }
    fn pressure(
        session: &mut NoiseChunkSession,
        blocks: &MaterialBlocks,
        barrier: &mut Option<f64>,
        first: FluidStatus,
        second: FluidStatus,
    ) -> Result<f64, NoiseChunkError> {
        charge(&mut session.work)?;
        let pos = session.state.pos();
        let a = first.at(pos.y(), blocks.air);
        let b = second.at(pos.y(), blocks.air);
        if (blocks.is_lava(a) && blocks.is_water(b)) || (blocks.is_water(a) && blocks.is_lava(b)) {
            return Ok(2.0);
        }
        let difference = first.level.wrapping_sub(second.level).wrapping_abs();
        if difference == 0 {
            return Ok(0.0);
        }
        let midpoint = 0.5 * f64::from(first.level.wrapping_add(second.level));
        let relative = f64::from(pos.y()) + 0.5 - midpoint;
        let gap = f64::from(difference) / 2.0 - relative.abs();
        let shape = if relative > 0.0 {
            let value = 0.0 + gap;
            if value > 0.0 {
                value / 1.5
            } else {
                value / 2.5
            }
        } else {
            let value = 3.0 + gap;
            if value > 0.0 {
                value / 3.0
            } else {
                value / 10.0
            }
        };
        let noise = if !(-2.0..=2.0).contains(&shape) {
            0.0
        } else if let Some(value) = *barrier {
            value
        } else {
            let value = sample(session, 0, pos, true)?;
            *barrier = Some(value);
            value
        };
        finite(session, "aquifer_pressure", 2.0 * (noise + shape))
    }
    fn positive(
        session: &NoiseChunkSession,
        substance: f64,
        pressure: f64,
    ) -> Result<bool, NoiseChunkError> {
        finite(session, "aquifer_pressure_product", pressure)?;
        Ok(finite(session, "aquifer_pressure_sum", substance + pressure)? > 0.0)
    }
    fn status(
        &mut self,
        session: &mut NoiseChunkSession,
        blocks: &MaterialBlocks,
        domain: MaterialDomain,
        packed: i64,
    ) -> Result<FluidStatus, NoiseChunkError> {
        let pos = unpack(packed);
        domain.center(pos)?;
        let grid = BlockPos::new(
            pos.x().div_euclid(16),
            pos.y().div_euclid(12),
            pos.z().div_euclid(16),
        );
        let index = self.index(grid)?;
        charge(&mut session.work)?;
        if let Some(value) = self.statuses[index] {
            return Ok(value);
        }
        let value = self.compute_fluid(session, blocks, domain, pos)?;
        charge(&mut session.work)?;
        self.statuses[index] = Some(value);
        Ok(value)
    }
    fn compute_fluid(
        &self,
        session: &mut NoiseChunkSession,
        blocks: &MaterialBlocks,
        domain: MaterialDomain,
        pos: BlockPos,
    ) -> Result<FluidStatus, NoiseChunkError> {
        charge(&mut session.work)?;
        let global = self.global(session, blocks, pos)?;
        let mut minimum = i32::MAX;
        let upper = pos.y() + 12;
        let lower = pos.y() - 12;
        let mut present = false;
        for offset in SURFACE_OFFSETS {
            charge(&mut session.work)?;
            let x = pos.x() + offset.x() * 16;
            let z = pos.z() + offset.z() * 16;
            let column = BlockPos::new(x, 0, z);
            MaterialDomain::horizontal(column)?;
            MaterialDomain::horizontal(BlockPos::new((x >> 2) * 4, 0, (z >> 2) * 4))?;
            let surface = session.preliminary(column)?;
            let fluid_y = surface.wrapping_add(8);
            let center = offset == ChunkPos::ORIGIN;
            if center && lower > fluid_y {
                return Ok(global);
            }
            let below = upper > fluid_y;
            if below || center {
                let fluid = self.global(session, blocks, BlockPos::new(x, fluid_y, z))?;
                if fluid.at(fluid_y, blocks.air) != blocks.air {
                    if center {
                        present = true;
                    }
                    if below {
                        return Ok(fluid);
                    }
                }
            }
            minimum = minimum.min(surface);
        }
        let level = Self::surface_level(session, domain, pos, global, minimum, present)?;
        let kind = Self::fluid_type(session, blocks, domain, pos, global, level)?;
        Ok(FluidStatus { level, kind })
    }
    fn surface_level(
        session: &mut NoiseChunkSession,
        domain: MaterialDomain,
        pos: BlockPos,
        global: FluidStatus,
        minimum: i32,
        present: bool,
    ) -> Result<i32, NoiseChunkError> {
        charge(&mut session.work)?;
        domain.center(pos)?;
        let (flooded, global_flooded) = if sample(session, 7, pos, false)? < f64::from(-0.225_f32)
            && sample(session, 8, pos, false)? > f64::from(0.9_f32)
        {
            (-1.0, -1.0)
        } else {
            let relative = minimum.wrapping_add(8).wrapping_sub(pos.y());
            let near_surface = if present {
                clamped_map(f64::from(relative), 0.0, 64.0, 1.0, 0.0)
            } else {
                0.0
            };
            let noise = sample(session, 1, pos, false)?.clamp(-1.0, 1.0);
            let global_threshold = -0.3 + ((near_surface - 1.0) / (0.0 - 1.0)) * (0.8 - -0.3);
            let random_threshold = -0.8 + ((near_surface - 1.0) / (0.0 - 1.0)) * (0.4 - -0.8);
            (noise - random_threshold, noise - global_threshold)
        };
        Ok(if global_flooded > 0.0 {
            global.level
        } else if flooded > 0.0 {
            Self::random_surface(session, domain, pos, minimum)?
        } else {
            WAY_BELOW_MIN_Y
        })
    }
    fn random_surface(
        session: &mut NoiseChunkSession,
        domain: MaterialDomain,
        pos: BlockPos,
        minimum: i32,
    ) -> Result<i32, NoiseChunkError> {
        charge(&mut session.work)?;
        let scaled = BlockPos::new(
            pos.x().div_euclid(16),
            pos.y().div_euclid(40),
            pos.z().div_euclid(16),
        );
        domain.scaled(scaled, 16)?;
        let spread = sample(session, 2, scaled, false)?;
        let value = finite(session, "aquifer_spread", spread * 10.0)?;
        // Java double-to-int saturation followed by its wrapping floor adjust.
        let divided = value / 3.0;
        let integer = divided as i32;
        let quantized = (if divided < f64::from(integer) {
            integer.wrapping_sub(1)
        } else {
            integer
        })
        .wrapping_mul(3);
        Ok(minimum.min((scaled.y() * 40 + 20).wrapping_add(quantized)))
    }
    fn fluid_type(
        session: &mut NoiseChunkSession,
        blocks: &MaterialBlocks,
        domain: MaterialDomain,
        pos: BlockPos,
        global: FluidStatus,
        level: i32,
    ) -> Result<BlockStateId, NoiseChunkError> {
        charge(&mut session.work)?;
        if level <= -10 && level != WAY_BELOW_MIN_Y && global.kind != blocks.lava {
            let scaled = BlockPos::new(
                pos.x().div_euclid(64),
                pos.y().div_euclid(40),
                pos.z().div_euclid(64),
            );
            domain.scaled(scaled, 64)?;
            if sample(session, 3, scaled, false)?.abs() > 0.3 {
                return Ok(blocks.lava);
            }
        }
        Ok(global.kind)
    }
}

fn similarity(first: i32, second: i32) -> f64 {
    1.0 - f64::from(second.wrapping_sub(first).wrapping_abs()) / 25.0
}
fn pack(pos: BlockPos) -> i64 {
    (((pos.x() as u64 & 67_108_863) << 38)
        | (pos.y() as u64 & 4095)
        | ((pos.z() as u64 & 67_108_863) << 12))
        .cast_signed()
}
fn unpack(value: i64) -> BlockPos {
    BlockPos::new(
        (value >> 38) as i32,
        (value.wrapping_shl(52) >> 52) as i32,
        (value.wrapping_shl(26) >> 38) as i32,
    )
}

#[cfg(test)]
mod oracle;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::density::chunk::tests::{budget, router};
    use crate::sample_work::SampleWork;

    fn fresh() -> (NoiseChunkSession, MaterialBlocks, Aquifer, MaterialDomain) {
        let r = router("minecraft:overworld", 0);
        let mut s = r
            .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
            .unwrap();
        let blocks = MaterialBlocks::new(
            s.data.metadata.default_block,
            s.data.metadata.default_fluid,
            &mut s.work,
        )
        .unwrap();
        let aquifer = Aquifer::new(&mut s).unwrap();
        let domain = MaterialDomain::new(&s).unwrap();
        s.state.start = BlockPos::new(8, 40, 8);
        s.state.in_x = 0;
        s.state.in_y = 0;
        s.state.in_z = 0;
        s.work = SampleWork::new(1_000_000);
        (s, blocks, aquifer, domain)
    }

    fn nonfinite(error: &NoiseChunkError, operation: &'static str, bits: u64) {
        let NoiseChunkErrorKind::Density(error) = error.kind() else {
            panic!("expected density refusal, got {:?}", error.kind());
        };
        assert_eq!(error.field(), operation);
        assert!(matches!(
            error.kind(),
            crate::density::DensitySampleErrorKind::NonFiniteDensity { bits: actual }
            if *actual == bits
        ));
    }

    #[test]
    fn checked_nonfinite_substance_and_arithmetic_have_no_fallback() {
        for value in [
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from_bits(0x7ff8_1234_5678_9abc),
        ] {
            let (mut session, blocks, mut aquifer, domain) = fresh();
            nonfinite(
                &aquifer
                    .compute(&mut session, &blocks, domain, value)
                    .unwrap_err(),
                "aquifer_substance",
                value.to_bits(),
            );
            assert_eq!(session.work.used(), 1);
            assert!(aquifer.locations.iter().all(|&v| v == i64::MAX));
            assert!(aquifer.statuses.iter().all(Option::is_none));
        }
        let (session, _, _, _) = fresh();
        nonfinite(
            &Aquifer::positive(&session, 0.0, f64::INFINITY).unwrap_err(),
            "aquifer_pressure_product",
            f64::INFINITY.to_bits(),
        );
        nonfinite(
            &Aquifer::positive(&session, f64::MAX, f64::MAX).unwrap_err(),
            "aquifer_pressure_sum",
            f64::INFINITY.to_bits(),
        );
        let mut session = crate::density::chunk::tests::requests::fresh(false);
        let blocks = MaterialBlocks::new(
            session.data.metadata.default_block,
            session.data.metadata.default_fluid,
            &mut session.work,
        )
        .unwrap();
        session.state.start = BlockPos::new(0, 0, 0);
        session.fields[0] = crate::density::chunk::tests::requests::leaf(
            &mut session,
            "barrier",
            &[f64::MAX],
            (f64::MAX, f64::MAX),
        );
        let mut barrier = None;
        nonfinite(
            &Aquifer::pressure(
                &mut session,
                &blocks,
                &mut barrier,
                FluidStatus {
                    level: 0,
                    kind: blocks.default_fluid,
                },
                FluidStatus {
                    level: 1,
                    kind: blocks.default_fluid,
                },
            )
            .unwrap_err(),
            "aquifer_pressure",
            f64::INFINITY.to_bits(),
        );
        session.fields[2] = crate::density::chunk::tests::requests::leaf(
            &mut session,
            "spread",
            &[f64::MAX],
            (f64::MAX, f64::MAX),
        );
        let domain = MaterialDomain::new(&session).unwrap();
        nonfinite(
            &Aquifer::random_surface(&mut session, domain, BlockPos::new(0, 0, 0), 0).unwrap_err(),
            "aquifer_spread",
            f64::INFINITY.to_bits(),
        );
    }
    #[test]
    fn lazy_aquifer_miss_hit_budget_and_failed_cache_publication() {
        let (mut session, blocks, mut aquifer, domain) = fresh();
        let expected = aquifer
            .compute(&mut session, &blocks, domain, -100.0)
            .unwrap();
        let miss_units = session.work.used();
        assert_eq!(
            aquifer.locations.iter().filter(|&&v| v != i64::MAX).count(),
            12
        );
        assert!(aquifer.statuses.iter().filter(|v| v.is_some()).count() >= 1);
        let locations = aquifer.locations.to_vec();
        let statuses = aquifer.statuses.to_vec();
        let schedule = aquifer.schedule;
        let (mut exact, other_blocks, mut other_aquifer, other_domain) = fresh();
        exact.work = SampleWork::new(miss_units);
        assert_eq!(
            other_aquifer
                .compute(&mut exact, &other_blocks, other_domain, -100.0)
                .unwrap(),
            expected
        );
        assert_eq!(other_aquifer.schedule, schedule);
        assert_eq!(&*other_aquifer.locations, &locations);
        assert_eq!(&*other_aquifer.statuses, &statuses);
        let (mut short, other_blocks, mut other_aquifer, other_domain) = fresh();
        short.work = SampleWork::new(miss_units - 1);
        assert!(other_aquifer
            .compute(&mut short, &other_blocks, other_domain, -100.0)
            .is_err());
        assert_eq!(short.work.used(), miss_units - 1);
        assert_eq!(
            other_aquifer
                .locations
                .iter()
                .filter(|&&v| v != i64::MAX)
                .count(),
            12
        );
        session.work = SampleWork::new(1_000_000);
        assert_eq!(
            aquifer
                .compute(&mut session, &blocks, domain, -100.0)
                .unwrap(),
            expected
        );
        let hit_n = session.work.used();
        assert!(hit_n < miss_units);
        assert_eq!(&*aquifer.locations, &locations);
        assert_eq!(&*aquifer.statuses, &statuses);
        assert_eq!(aquifer.schedule, schedule);
        session.work = SampleWork::new(hit_n);
        assert_eq!(
            aquifer
                .compute(&mut session, &blocks, domain, -100.0)
                .unwrap(),
            expected
        );
        session.work = SampleWork::new(hit_n - 1);
        assert!(aquifer
            .compute(&mut session, &blocks, domain, -100.0)
            .is_err());
        assert_eq!(&*aquifer.locations, &locations);
        assert_eq!(&*aquifer.statuses, &statuses);
        let (mut status_session, other_blocks, mut other_aquifer, other_domain) = fresh();
        let center = BlockPos::new(0, 36, 0);
        let value = other_aquifer
            .status(
                &mut status_session,
                &other_blocks,
                other_domain,
                pack(center),
            )
            .unwrap();
        let miss_units = status_session.work.used();
        let (mut short, other_blocks, mut other_aquifer, other_domain) = fresh();
        short.work = SampleWork::new(miss_units - 1);
        assert!(other_aquifer
            .status(&mut short, &other_blocks, other_domain, pack(center))
            .is_err());
        assert!(other_aquifer.statuses.iter().all(Option::is_none));
        let (mut exact, other_blocks, mut other_aquifer, other_domain) = fresh();
        exact.work = SampleWork::new(miss_units);
        assert_eq!(
            other_aquifer
                .status(&mut exact, &other_blocks, other_domain, pack(center))
                .unwrap(),
            value
        );
        assert_eq!(
            other_aquifer
                .statuses
                .iter()
                .filter(|v| v.is_some())
                .count(),
            1
        );
    }
    #[test]
    fn cache_grid_bounds_packed_positions_and_separate_byte_caps() {
        let (mut s, _, a, _) = fresh();
        assert_eq!(a.index(a.min_grid).unwrap(), 0);
        assert_eq!(
            a.index(BlockPos::new(
                a.min_grid.x() + 2,
                a.min_grid.y() + 34,
                a.min_grid.z() + 2
            ))
            .unwrap(),
            314
        );
        for grid in [
            BlockPos::new(a.min_grid.x() - 1, a.min_grid.y(), a.min_grid.z()),
            BlockPos::new(a.min_grid.x() + 3, a.min_grid.y(), a.min_grid.z()),
            BlockPos::new(a.min_grid.x(), a.min_grid.y() - 1, a.min_grid.z()),
            BlockPos::new(a.min_grid.x(), a.min_grid.y() + 35, a.min_grid.z()),
            BlockPos::new(a.min_grid.x(), a.min_grid.y(), a.min_grid.z() + 3),
        ] {
            assert!(a.index(grid).is_err());
        }
        for pos in [
            BlockPos::new(-30_000_128, -2048, 30_000_128),
            BlockPos::new(30_000_128, 2047, -30_000_128),
            BlockPos::new(-1, -1, -1),
            BlockPos::new(0, 0, 0),
        ] {
            assert_eq!(unpack(pack(pos)), pos);
            assert_ne!(pack(pos), i64::MAX);
        }
        let total = bytes(
            NoiseChunkLimit::AquiferBytes,
            0,
            512,
            std::mem::size_of::<i64>(),
            MAX_CACHE_BYTES,
        )
        .unwrap();
        assert!(bytes(
            NoiseChunkLimit::AquiferBytes,
            total,
            512,
            std::mem::size_of::<Option<FluidStatus>>(),
            MAX_CACHE_BYTES
        )
        .is_ok());
        assert!(limit(NoiseChunkLimit::AquiferSlots, 512, MAX_SLOTS).is_ok());
        assert!(limit(NoiseChunkLimit::AquiferSlots, 513, MAX_SLOTS).is_err());
        assert!(bytes(
            NoiseChunkLimit::AquiferBytes,
            MAX_CACHE_BYTES,
            1,
            1,
            MAX_CACHE_BYTES
        )
        .is_err());
        s.state.geometry.height = 1024;
        assert!(matches!(
            Aquifer::new(&mut s).err().unwrap().kind(),
            NoiseChunkErrorKind::Limit {
                limit: NoiseChunkLimit::AquiferSlots,
                ..
            }
        ));
    }
}
