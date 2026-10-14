//! Explicit revision-4 isolated-source subset of normal Overworld placed ores.
//!
//! Original counts/heights/targets/biome memberships and feature seeds are pinned.
//! Source-local history includes every candidate, including writes outside the
//! requested target. Other placed geology/structures and whole-FEATURES history
//! remain omitted: this is reproducible Survival terrain, not Vanilla seed parity.
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

use ferrumc_math::{BlockPos, ChunkPos, LocalBlockPos};
use ferrumc_registry::{
    biome::BiomeResourceId,
    block_state::{block_metadata, compute_state_id, state_id_to_block_name},
};

use crate::{
    biome_zoom::{BiomeZoom, BiomeZoomSeed, BiomeZoomSemantics},
    grass::{GrassPreview, ProductionGates},
    random::WorldgenRandomSource,
    BlockStateId, Chunk, NaturalGenerationStage, NaturalWorldGenerator, WorldError, WorldSeed,
};

#[path = "survival_ores/catalog.rs"]
mod catalog;
mod trigonometry {
    include!("carver/sin_table.rs");
    pub(super) fn sin(value: f32) -> f32 {
        f32::from_bits(ORIGINAL_SIN_BITS[((value * 10430.378_f32) as i32 & 65535) as usize])
    }
}
const MOTION: &[u8] = include_bytes!("survival_ores/blocks_motion.bin");
const MAX_WRITES: usize = 16_384;
const SOURCE_CACHE_COUNT: usize = 32;
const SOURCE_CACHE_BYTES: usize = 8 * 1024 * 1024;
const MAX_SOURCE_WORK: u32 = 1_048_576;
type Writes = Arc<[(BlockPos, BlockStateId)]>;
fn error(detail: impl std::fmt::Display) -> WorldError {
    WorldError::NaturalGeneration {
        stage: NaturalGenerationStage::Features,
        detail: detail.to_string().chars().take(512).collect(),
    }
}
#[derive(Clone, Copy)]
enum Count {
    Fixed(u32),
    Uniform(u32, u32),
    Rare(u32),
}
#[derive(Clone, Copy)]
struct Spec {
    key: &'static str,
    index: i64,
    count: Count,
    min_y: i32,
    max_y: i32,
    triangle: bool,
    size: usize,
    discard: f32,
    targets: [(&'static str, &'static [(&'static str, &'static str)]); 2],
}
pub(crate) struct SurvivalOrePreview {
    targets: [[BlockStateId; 2]; 18],
    replaceable: [(u32, u32); 6],
    members: [(BiomeResourceId, u32); 54],
    sources: Mutex<VecDeque<(ChunkPos, Writes)>>,
    source_gates: ProductionGates,
}
impl SurvivalOrePreview {
    pub(crate) fn new() -> Result<Self, WorldError> {
        let mut targets = [[BlockStateId::AIR; 2]; 18];
        for (i, spec) in catalog::SPECS.iter().enumerate() {
            for (slot, (name, props)) in spec.targets.iter().enumerate() {
                let properties = props.iter().copied().collect();
                targets[i][slot] =
                    BlockStateId::new(compute_state_id(name, &properties).ok_or_else(|| {
                        error(format!(
                            "missing original ore target state for {}",
                            spec.key
                        ))
                    })?);
            }
        }
        let mut replaceable = [(0, 0); 6];
        for (slot, name) in [
            "stone",
            "granite",
            "diorite",
            "andesite",
            "deepslate",
            "tuff",
        ]
        .iter()
        .enumerate()
        {
            let metadata = block_metadata(name).ok_or_else(|| error("missing ore tag block"))?;
            replaceable[slot] = (metadata.min_state, metadata.max_state);
        }
        let first = BiomeResourceId::from_name(catalog::MEMBERS[0].0)
            .ok_or_else(|| error("missing ore biome"))?;
        let mut members = [(first, 0); 54];
        for (slot, (name, flags)) in catalog::MEMBERS.iter().enumerate() {
            members[slot] = (
                BiomeResourceId::from_name(name)
                    .ok_or_else(|| error("missing original ore biome"))?,
                *flags,
            );
        }
        Ok(Self {
            targets,
            replaceable,
            members,
            sources: Mutex::new(VecDeque::with_capacity(SOURCE_CACHE_COUNT)),
            source_gates: ProductionGates::new(),
        })
    }
    fn member(&self, biome: BiomeResourceId, feature: usize) -> bool {
        self.members
            .iter()
            .any(|(id, flags)| *id == biome && flags & (1 << feature) != 0)
    }
    fn target(&self, state: BlockStateId, feature: usize) -> Option<BlockStateId> {
        self.replaceable
            .iter()
            .position(|(min, max)| (*min..=*max).contains(&state.as_u32()))
            .map(|index| self.targets[feature][usize::from(index >= 4)])
    }
    fn cached(&self, source: ChunkPos) -> Result<Option<Writes>, WorldError> {
        Ok(self
            .sources
            .lock()
            .map_err(|_| error("ore source cache poisoned"))?
            .iter()
            .find(|(key, _)| *key == source)
            .map(|(_, writes)| Arc::clone(writes)))
    }
    fn source(
        &self,
        generator: &NaturalWorldGenerator,
        grass: &GrassPreview,
        seed: WorldSeed,
        source: ChunkPos,
    ) -> Result<Writes, WorldError> {
        if let Some(value) = self.cached(source)? {
            return Ok(value);
        }
        let _gate = self.source_gates.lock(source)?;
        if let Some(value) = self.cached(source)? {
            return Ok(value);
        }
        let mut region = SourceRegion::new(generator, grass, source)?;
        let zoom = BiomeZoom::new(
            BiomeZoomSeed::from_world_seed(seed),
            BiomeZoomSemantics::Paper1_21_8Build60,
        );
        for (feature, spec) in catalog::SPECS.iter().enumerate() {
            let mut random = Random::feature(seed, source, spec.index)?;
            let count = match spec.count {
                Count::Fixed(count) => count,
                Count::Uniform(min, max) => min + random.int(max - min + 1)? as u32,
                Count::Rare(chance) => u32::from(random.float()? < 1.0 / chance as f32),
            };
            for _ in 0..count {
                let x = source.x().wrapping_mul(16) + random.int(16)?;
                let z = source.z().wrapping_mul(16) + random.int(16)?;
                let span = spec.max_y - spec.min_y;
                let y = if spec.triangle {
                    let half = span / 2;
                    spec.min_y
                        + random.int((span - half + 1) as u32)?
                        + random.int((half + 1) as u32)?
                } else {
                    spec.min_y + random.int((span + 1) as u32)?
                };
                let origin = BlockPos::new(x, y, z);
                if !self.member(grass.biome(generator, zoom, origin)?, feature) {
                    continue;
                }
                place(self, feature, spec, origin, &mut random, &mut region)?;
            }
        }
        let writes: Writes = region.writes.into_iter().collect::<Vec<_>>().into();
        let bytes = writes.len() * std::mem::size_of::<(BlockPos, BlockStateId)>();
        let mut cache = self
            .sources
            .lock()
            .map_err(|_| error("ore source cache poisoned"))?;
        while cache.len() >= SOURCE_CACHE_COUNT
            || cache
                .iter()
                .map(|(_, v)| v.len() * std::mem::size_of::<(BlockPos, BlockStateId)>())
                .sum::<usize>()
                + bytes
                > SOURCE_CACHE_BYTES
        {
            cache.pop_front();
        }
        cache.push_back((source, Arc::clone(&writes)));
        Ok(writes)
    }
    pub(crate) fn apply(
        &self,
        generator: &NaturalWorldGenerator,
        grass: &GrassPreview,
        seed: WorldSeed,
        target: &mut Chunk,
    ) -> Result<(), WorldError> {
        let center = target.pos();
        for dx in -1..=1 {
            for dz in -1..=1 {
                let source = ChunkPos::new(
                    center
                        .x()
                        .checked_add(dx)
                        .ok_or_else(|| error("ore source coordinate overflow"))?,
                    center
                        .z()
                        .checked_add(dz)
                        .ok_or_else(|| error("ore source coordinate overflow"))?,
                );
                for &(pos, state) in self.source(generator, grass, seed, source)?.iter() {
                    if pos.to_chunk_pos() == center {
                        target.set_block(pos, state)?;
                    }
                }
            }
        }
        Ok(())
    }
}
struct SourceRegion {
    source: ChunkPos,
    raw: Vec<Arc<Chunk>>,
    heights: [[i32; 256]; 9],
    writes: BTreeMap<BlockPos, BlockStateId>,
    work: u32,
}
impl SourceRegion {
    fn new(
        generator: &NaturalWorldGenerator,
        grass: &GrassPreview,
        source: ChunkPos,
    ) -> Result<Self, WorldError> {
        let mut raw = Vec::with_capacity(9);
        let mut heights = [[-64; 256]; 9];
        for dz in -1..=1 {
            for dx in -1..=1 {
                let pos = ChunkPos::new(
                    source
                        .x()
                        .checked_add(dx)
                        .ok_or_else(|| error("ore halo overflow"))?,
                    source
                        .z()
                        .checked_add(dz)
                        .ok_or_else(|| error("ore halo overflow"))?,
                );
                let chunk = grass.raw(generator, pos)?;
                let slot = raw.len();
                for z in 0..16_u8 {
                    for x in 0..16_u8 {
                        'column: for (section, blocks) in chunk.sections().iter().enumerate().rev()
                        {
                            for y in (0..16_u8).rev() {
                                let Some(local) = LocalBlockPos::new(x, y, z) else {
                                    continue;
                                };
                                let id = blocks.get(local).as_u32() as usize;
                                let motion = *MOTION
                                    .get(id)
                                    .ok_or_else(|| error("unknown ore terrain state"))?;
                                if motion != 0 {
                                    heights[slot][usize::from(z) * 16 + usize::from(x)] = -64
                                        + i32::try_from(section * 16)
                                            .map_err(|_| error("ore section offset overflow"))?
                                        + i32::from(y)
                                        + 1;
                                    break 'column;
                                }
                            }
                        }
                    }
                }
                raw.push(chunk);
            }
        }
        Ok(Self {
            source,
            raw,
            heights,
            writes: BTreeMap::new(),
            work: 0,
        })
    }
    fn slot(&self, pos: BlockPos) -> Result<usize, WorldError> {
        let owner = pos.to_chunk_pos();
        let dx = i64::from(owner.x()) - i64::from(self.source.x());
        let dz = i64::from(owner.z()) - i64::from(self.source.z());
        if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) {
            return Err(error("ore read escaped source halo"));
        }
        Ok(((dz + 1) * 3 + dx + 1) as usize)
    }
    fn get(&self, pos: BlockPos) -> Result<BlockStateId, WorldError> {
        if !(-64..=319).contains(&pos.y()) {
            return Ok(BlockStateId::AIR);
        }
        if let Some(value) = self.writes.get(&pos) {
            return Ok(*value);
        }
        self.raw[self.slot(pos)?]
            .get_block(pos)
            .ok_or_else(|| error("ore raw read missing"))
    }
    fn height(&self, x: i32, z: i32) -> Result<i32, WorldError> {
        let pos = BlockPos::new(x, 0, z);
        Ok(self.heights[self.slot(pos)?][(z & 15) as usize * 16 + (x & 15) as usize])
    }
    fn put(&mut self, pos: BlockPos, state: BlockStateId) -> Result<(), WorldError> {
        self.slot(pos)?;
        if !self.writes.contains_key(&pos) && self.writes.len() == MAX_WRITES {
            return Err(error("ore source write cap exhausted"));
        }
        self.writes.insert(pos, state);
        Ok(())
    }
    fn charge(&mut self) -> Result<(), WorldError> {
        if self.work == MAX_SOURCE_WORK {
            return Err(error("ore source work allowance exhausted"));
        }
        self.work += 1;
        Ok(())
    }
    fn adjacent_air(&self, pos: BlockPos) -> Result<bool, WorldError> {
        for (dx, dy, dz) in [
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
            (-1, 0, 0),
            (1, 0, 0),
        ] {
            let state = self.get(BlockPos::new(pos.x() + dx, pos.y() + dy, pos.z() + dz))?;
            if matches!(
                state_id_to_block_name(state.as_u32()),
                Some("air" | "cave_air" | "void_air")
            ) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
// Original WorldgenRandom over xoroshiro: inherited nextInt/nextDouble/nextLong
// consume the high bits of separate xoroshiro longs. Direct xoroshiro methods
// have different consumption and must not be substituted here.
struct Random {
    core: WorldgenRandomSource,
    remaining: u32,
}
impl Random {
    fn bits(&mut self, count: u32) -> Result<u32, WorldError> {
        if self.remaining == 0 {
            return Err(error("ore RNG allowance exhausted"));
        }
        self.remaining -= 1;
        Ok((self.core.next_long() as u64 >> (64 - count)) as u32)
    }
    fn int(&mut self, bound: u32) -> Result<i32, WorldError> {
        if !(1..=385).contains(&bound) {
            return Err(error("unsupported original ore RNG bound"));
        }
        if bound.is_power_of_two() {
            return Ok(((u64::from(bound) * u64::from(self.bits(31)?)) >> 31) as i32);
        }
        loop {
            let bits = self.bits(31)?;
            let value = bits % bound;
            if bits
                .wrapping_sub(value)
                .wrapping_add(bound - 1)
                .cast_signed()
                >= 0
            {
                return Ok(value.cast_signed());
            }
        }
    }
    fn float(&mut self) -> Result<f32, WorldError> {
        Ok(self.bits(24)? as f32 / 16_777_216.0)
    }
    fn double(&mut self) -> Result<f64, WorldError> {
        Ok(f64::from(
            ((u64::from(self.bits(26)?) << 27) + u64::from(self.bits(27)?)) as f32
                * 1.110_223E-16_f32,
        ))
    }
    fn long(&mut self) -> Result<i64, WorldError> {
        Ok(i64::from(self.bits(32)?.cast_signed())
            .wrapping_shl(32)
            .wrapping_add(i64::from(self.bits(32)?.cast_signed())))
    }
    fn feature(seed: WorldSeed, source: ChunkPos, index: i64) -> Result<Self, WorldError> {
        let mut random = Self {
            core: WorldgenRandomSource::xoroshiro(seed),
            remaining: MAX_SOURCE_WORK,
        };
        let x = random.long()? | 1;
        let z = random.long()? | 1;
        let decoration = i64::from(source.x().wrapping_mul(16))
            .wrapping_mul(x)
            .wrapping_add(i64::from(source.z().wrapping_mul(16)).wrapping_mul(z))
            ^ seed.value();
        random.core = WorldgenRandomSource::xoroshiro(WorldSeed::new(
            decoration.wrapping_add(index).wrapping_add(60_000),
        ));
        Ok(random)
    }
}
// Keep the original complete OreFeature arithmetic and chronology contiguous.
#[allow(clippy::too_many_lines, clippy::manual_midpoint)]
fn place(
    catalog: &SurvivalOrePreview,
    feature: usize,
    spec: &Spec,
    origin: BlockPos,
    random: &mut Random,
    region: &mut SourceRegion,
) -> Result<(), WorldError> {
    let angle = random.float()? * std::f32::consts::PI;
    let spread = spec.size as f32 / 8.0;
    let pad = ((spec.size as f32 / 16.0 * 2.0 + 1.0) / 2.0).ceil() as i32;
    let x0 = f64::from(origin.x()) + f64::from(angle).sin() * f64::from(spread);
    let x1 = f64::from(origin.x()) - f64::from(angle).sin() * f64::from(spread);
    let z0 = f64::from(origin.z()) + f64::from(angle).cos() * f64::from(spread);
    let z1 = f64::from(origin.z()) - f64::from(angle).cos() * f64::from(spread);
    let y0 = f64::from(origin.y() + random.int(3)? - 2);
    let y1 = f64::from(origin.y() + random.int(3)? - 2);
    let base = BlockPos::new(
        origin.x() - spread.ceil() as i32 - pad,
        origin.y() - 2 - pad,
        origin.z() - spread.ceil() as i32 - pad,
    );
    let width = 2 * (spread.ceil() as i32 + pad);
    let height = 2 * (2 + pad);
    let mut intersects = false;
    'height: for x in base.x()..=base.x() + width {
        for z in base.z()..=base.z() + width {
            if base.y() <= region.height(x, z)? {
                intersects = true;
                break 'height;
            }
        }
    }
    if !intersects {
        return Ok(());
    }
    let mut spheres = [[0_f64; 4]; 20];
    for (index, sphere) in spheres.iter_mut().enumerate().take(spec.size) {
        let fraction = index as f32 / spec.size as f32;
        let t = f64::from(fraction);
        let radius = random.double()? * spec.size as f64 / 16.0;
        *sphere = [
            x0 + t * (x1 - x0),
            y0 + t * (y1 - y0),
            z0 + t * (z1 - z0),
            ((f64::from(trigonometry::sin(std::f32::consts::PI * fraction) + 1.0) * radius) + 1.0)
                / 2.0,
        ];
    }
    for i in 0..spec.size - 1 {
        if spheres[i][3] <= 0.0 {
            continue;
        }
        for j in i + 1..spec.size {
            if spheres[j][3] <= 0.0 {
                continue;
            }
            let a = spheres[i];
            let b = spheres[j];
            let dx = a[0] - b[0];
            let dy = a[1] - b[1];
            let dz = a[2] - b[2];
            let dr = a[3] - b[3];
            if dr * dr > dx * dx + dy * dy + dz * dz {
                spheres[if dr > 0.0 { j } else { i }][3] = -1.0;
            }
        }
    }
    let mut visited = vec![false; (width * height * width) as usize];
    for sphere in spheres.iter().take(spec.size) {
        let [cx, cy, cz, r] = *sphere;
        if r < 0.0 {
            continue;
        }
        let min_x = ((cx - r).floor() as i32).max(base.x());
        let min_y = ((cy - r).floor() as i32).max(base.y());
        let min_z = ((cz - r).floor() as i32).max(base.z());
        let max_x = ((cx + r).floor() as i32).max(min_x);
        let max_y = ((cy + r).floor() as i32).max(min_y);
        let max_z = ((cz + r).floor() as i32).max(min_z);
        for x in min_x..=max_x {
            let dx = (f64::from(x) + 0.5 - cx) / r;
            if dx * dx >= 1.0 {
                continue;
            }
            for y in min_y..=max_y {
                let dy = (f64::from(y) + 0.5 - cy) / r;
                if dx * dx + dy * dy >= 1.0 {
                    continue;
                }
                for z in min_z..=max_z {
                    region.charge()?;
                    let dz = (f64::from(z) + 0.5 - cz) / r;
                    if dx * dx + dy * dy + dz * dz >= 1.0 || !(-64..=319).contains(&y) {
                        continue;
                    }
                    let index = (x - base.x()
                        + (y - base.y()) * width
                        + (z - base.z()) * width * height) as usize;
                    let flag = visited
                        .get_mut(index)
                        .ok_or_else(|| error("ore visited geometry escaped bound"))?;
                    if *flag {
                        continue;
                    }
                    *flag = true;
                    let pos = BlockPos::new(x, y, z);
                    let state = region.get(pos)?;
                    let Some(target) = catalog.target(state, feature) else {
                        continue;
                    };
                    let skip = spec.discard <= 0.0
                        || (spec.discard < 1.0 && random.float()? >= spec.discard);
                    if skip || !region.adjacent_air(pos)? {
                        region.put(pos, target)?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(name: &str) -> BlockStateId {
        BlockStateId::new(ferrumc_registry::block_state::block_default_state(name).unwrap())
    }
    fn solid_region(kind: u32) -> SourceRegion {
        let substrate = state(match kind {
            1 => "deepslate",
            3 => "bedrock",
            _ => "stone",
        });
        let mut raw = Vec::new();
        for dz in -1..=1 {
            for dx in -1..=1 {
                let key = ChunkPos::new(dx, dz);
                let mut chunk = Chunk::new(key);
                for section in 0..24 {
                    for y in 0..16_u8 {
                        for z in 0..16_u8 {
                            for x in 0..16_u8 {
                                if kind == 2 && dx * 16 + i32::from(x) == 0 {
                                    continue;
                                }
                                chunk.set_local(
                                    section,
                                    LocalBlockPos::new(x, y, z).unwrap(),
                                    substrate,
                                );
                            }
                        }
                    }
                }
                raw.push(Arc::new(chunk));
            }
        }
        SourceRegion {
            source: ChunkPos::ORIGIN,
            raw,
            heights: [[320; 256]; 9],
            writes: BTreeMap::new(),
            work: 0,
        }
    }
    #[test]
    fn actual_paper_rng_chronology_and_float_double_bits() {
        let mut rows = 0;
        for line in include_str!("../tests/fixtures/survival_ores/paper-random.tsv").lines() {
            let values: Vec<_> = line.split('\t').collect();
            let mut random = Random::feature(
                WorldSeed::new(values[0].parse().unwrap()),
                ChunkPos::new(values[1].parse().unwrap(), values[2].parse().unwrap()),
                values[3].parse().unwrap(),
            )
            .unwrap();
            for (column, bound) in [16, 3, 2, 9, 193, 305, 184, 161, 385].iter().enumerate() {
                assert_eq!(
                    random.int(*bound).unwrap(),
                    values[4 + column].parse::<i32>().unwrap(),
                    "{line}"
                );
            }
            assert_eq!(
                random.float().unwrap().to_bits(),
                values[13].parse::<u32>().unwrap()
            );
            assert_eq!(
                random.double().unwrap().to_bits(),
                values[14].parse::<u64>().unwrap()
            );
            rows += 1;
        }
        assert_eq!(rows, 80);
    }
    #[test]
    fn actual_paper_ore_kernel_solid_air_boundary_and_draws() {
        let catalog = SurvivalOrePreview::new().unwrap();
        let templates: Vec<_> = (0..4).map(solid_region).collect();
        let mut rows = 0;
        for line in include_str!("../tests/fixtures/survival_ores/paper-kernel.tsv").lines() {
            let columns: Vec<_> = line.split('\t').collect();
            let kind: usize = columns[2].parse().unwrap();
            let mut region = SourceRegion {
                source: ChunkPos::ORIGIN,
                raw: templates[kind].raw.clone(),
                heights: [[320; 256]; 9],
                writes: BTreeMap::new(),
                work: 0,
            };
            let spec = Spec {
                size: columns[1].parse().unwrap(),
                discard: columns[3].parse().unwrap(),
                ..catalog::SPECS[9]
            };
            let mut random = Random::feature(
                WorldSeed::new(columns[0].parse().unwrap()),
                ChunkPos::ORIGIN,
                18,
            )
            .unwrap();
            place(
                &catalog,
                9,
                &spec,
                BlockPos::new(0, 8, 0),
                &mut random,
                &mut region,
            )
            .unwrap();
            let expected: BTreeMap<_, _> = columns[6]
                .split(';')
                .filter(|s| !s.is_empty())
                .map(|entry| {
                    let xyz: Vec<i32> = entry.split(',').map(|n| n.parse().unwrap()).collect();
                    (
                        BlockPos::new(xyz[0], xyz[1], xyz[2]),
                        state(if kind == 1 {
                            "deepslate_diamond_ore"
                        } else {
                            "diamond_ore"
                        }),
                    )
                })
                .collect();
            assert_eq!(region.writes, expected, "case {line}");
            assert_eq!(
                !region.writes.is_empty(),
                columns[4].parse::<bool>().unwrap()
            );
            assert_eq!(
                MAX_SOURCE_WORK - random.remaining,
                columns[5].parse::<u32>().unwrap(),
                "draws {line}"
            );
            rows += 1;
        }
        assert_eq!(rows, 192);
    }
    #[test]
    fn ore_tags_preserve_unrelated_blocks_and_all_deepslate_variants() {
        let catalog = SurvivalOrePreview::new().unwrap();
        for (feature, bare) in [
            (0, "coal"),
            (2, "iron"),
            (5, "gold"),
            (7, "redstone"),
            (9, "diamond"),
            (13, "lapis"),
            (15, "copper"),
        ] {
            assert_eq!(catalog.target(state("water"), feature), None);
            assert_eq!(catalog.target(state("bedrock"), feature), None);
            assert_eq!(catalog.target(state("diamond_ore"), feature), None);
            for substrate in ["stone", "granite", "diorite", "andesite"] {
                assert_eq!(
                    catalog.target(state(substrate), feature),
                    Some(state(&format!("{bare}_ore")))
                );
            }
            for substrate in ["deepslate", "tuff"] {
                assert_eq!(
                    catalog.target(state(substrate), feature),
                    Some(state(&format!("deepslate_{bare}_ore")))
                );
            }
        }
        let mut random = Random::feature(WorldSeed::new(0), ChunkPos::ORIGIN, 18).unwrap();
        let before = random.remaining;
        assert!(random.int(0).is_err());
        assert_eq!(before, random.remaining);
        let mut region = solid_region(0);
        region.work = MAX_SOURCE_WORK;
        assert!(region.charge().is_err());
        assert_eq!(region.work, MAX_SOURCE_WORK);
        assert!(region.writes.is_empty());
    }
    #[test]
    #[ignore = "explicit actual natural terrain smoke, run in release on the bounded generation lane"]
    fn natural_survival_contains_the_seven_ore_families() {
        let generator = NaturalWorldGenerator::with_survival_preview(WorldSeed::new(0)).unwrap();
        let mut totals = [0_usize; 7];
        let mut deep = [0_usize; 7];
        let mut samples = [None; 7];
        let families = [
            "coal", "iron", "copper", "gold", "redstone", "lapis", "diamond",
        ];
        let ids = families.map(|name| {
            [
                state(&format!("{name}_ore")),
                state(&format!("deepslate_{name}_ore")),
            ]
        });
        for key in [
            ChunkPos::ORIGIN,
            ChunkPos::new(1, 0),
            ChunkPos::new(0, 1),
            ChunkPos::new(1, 1),
        ] {
            let chunk = generator.try_generate_chunk(key).unwrap();
            for (section_index, section) in chunk.sections().iter().enumerate() {
                for y in 0..16_u8 {
                    for z in 0..16_u8 {
                        for x in 0..16_u8 {
                            let id = section.get(LocalBlockPos::new(x, y, z).unwrap());
                            for (index, family) in ids.iter().enumerate() {
                                if family.contains(&id) {
                                    totals[index] += 1;
                                    if family[1] == id {
                                        deep[index] += 1;
                                    }
                                    if samples[index].is_none() {
                                        samples[index] = Some(BlockPos::new(
                                            key.x() * 16 + i32::from(x),
                                            -64 + i32::try_from(section_index * 16).unwrap()
                                                + i32::from(y),
                                            key.z() * 16 + i32::from(z),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            eprintln!("natural seed0 {key:?}: cumulative {totals:?}");
        }
        eprintln!("natural totals={totals:?} deepslate={deep:?} samples={samples:?}");
        assert!(
            totals.into_iter().all(|n| n > 0),
            "missing families {totals:?}"
        );
        let first = generator.try_generate_chunk(ChunkPos::ORIGIN).unwrap();
        let repeat = generator
            .clone()
            .try_generate_chunk(ChunkPos::ORIGIN)
            .unwrap();
        assert_eq!(first, repeat);
        assert!(!first.persist_dirty_sections().any());
    }
}
