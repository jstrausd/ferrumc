//! Revision-five `FerrumC` vegetation and surface structures. Source-local plans
//! are evaluated completely against immutable terrain, then projected into the
//! requested chunk. These additions deliberately do not promise Vanilla seeds.
use crate::{
    grass::{GrassPreview, ProductionGates},
    BlockEntity, BlockStateId, ChestInventory, Chunk, HeightmapKind, NaturalGenerationStage,
    NaturalWorldGenerator, WorldError, WorldSeed,
};
use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::block_state::{compute_state_id, state_id_to_block_name};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

const MAX_WRITES: usize = 8192;
const MAX_SOURCE_WORK: usize = 1_048_576;
const CACHE_COUNT: usize = 32;
const CACHE_BYTES: usize = 8 * 1024 * 1024;
fn error(detail: impl std::fmt::Display) -> WorldError {
    WorldError::NaturalGeneration {
        stage: NaturalGenerationStage::Features,
        detail: detail.to_string().chars().take(512).collect(),
    }
}
fn owner(p: BlockPos) -> ChunkPos {
    ChunkPos::new(p.x() >> 4, p.z() >> 4)
}
fn state(name: &str, properties: &[(&str, &str)]) -> Result<BlockStateId, WorldError> {
    compute_state_id(name, &properties.iter().copied().collect())
        .map(BlockStateId::new)
        .ok_or_else(|| error(format!("missing playable state {name}")))
}
fn named(s: BlockStateId, name: &str) -> bool {
    state_id_to_block_name(s.as_u32()) == Some(name)
}
#[derive(Clone, Copy)]
struct Write {
    state: BlockStateId,
    force: bool,
}
#[derive(Default)]
struct Plan {
    blocks: BTreeMap<BlockPos, Write>,
    chest: Option<(BlockPos, ChestInventory)>,
}
impl Plan {
    fn put(&mut self, p: BlockPos, state: BlockStateId, force: bool) -> Result<(), WorldError> {
        if !(-64..320).contains(&p.y()) {
            return Err(error("playable feature outside vertical domain"));
        }
        if self.blocks.len() == MAX_WRITES && !self.blocks.contains_key(&p) {
            return Err(error("playable source write cap"));
        }
        self.blocks.insert(p, Write { state, force });
        Ok(())
    }
    fn bytes(&self) -> usize {
        // Account conservatively for BTree nodes and allocator headers, not only values.
        std::mem::size_of::<Self>()
            + 2 * std::mem::size_of::<usize>()
            + self.blocks.len() * 80
            + usize::from(self.chest.is_some()) * 16_384
    }
}
trait Terrain {
    fn height(&mut self, x: i32, z: i32) -> Result<i32, WorldError>;
    fn read(&mut self, p: BlockPos) -> Result<BlockStateId, WorldError>;
    fn biome(&mut self, p: BlockPos) -> Result<&'static str, WorldError>;
}
struct Raw<'a> {
    generator: &'a NaturalWorldGenerator,
    grass: &'a GrassPreview,
    source: ChunkPos,
    chunks: Vec<(ChunkPos, Arc<Chunk>, [i32; 256])>,
    work: usize,
}
impl Raw<'_> {
    fn charge(&mut self, n: usize) -> Result<(), WorldError> {
        self.work = self
            .work
            .checked_add(n)
            .filter(|n| *n <= MAX_SOURCE_WORK)
            .ok_or_else(|| error("playable source work cap"))?;
        Ok(())
    }
    fn chunk(&mut self, pos: ChunkPos) -> Result<usize, WorldError> {
        if let Some(i) = self.chunks.iter().position(|(p, _, _)| *p == pos) {
            return Ok(i);
        }
        if self.chunks.len() == 9
            || (i64::from(pos.x()) - i64::from(self.source.x())).abs() > 1
            || (i64::from(pos.z()) - i64::from(self.source.z())).abs() > 1
        {
            return Err(error("playable source halo cap"));
        }
        self.charge(16 * 16 * 384)?;
        let raw = self.grass.raw(self.generator, pos)?;
        let map = raw.heightmap(HeightmapKind::WorldSurface);
        let heights =
            std::array::from_fn(|i| map.height((i / 16) as u8, (i % 16) as u8).unwrap_or(-64));
        self.chunks.push((pos, raw, heights));
        Ok(self.chunks.len() - 1)
    }
}
impl Terrain for Raw<'_> {
    fn height(&mut self, x: i32, z: i32) -> Result<i32, WorldError> {
        self.charge(1)?;
        let i = self.chunk(ChunkPos::new(x >> 4, z >> 4))?;
        Ok(self.chunks[i].2[(x & 15) as usize * 16 + (z & 15) as usize])
    }
    fn read(&mut self, p: BlockPos) -> Result<BlockStateId, WorldError> {
        self.charge(1)?;
        if !(-64..320).contains(&p.y()) {
            return Err(error("playable raw read outside vertical domain"));
        }
        let i = self.chunk(owner(p))?;
        self.chunks[i]
            .1
            .get_block(p)
            .ok_or_else(|| error("playable raw block missing"))
    }
    fn biome(&mut self, p: BlockPos) -> Result<&'static str, WorldError> {
        self.charge(1)?;
        let i = self.chunk(owner(p))?;
        let index = ((((p.y().clamp(-64, 319) + 64) / 4) as usize * 4 + (p.x() & 15) as usize / 4)
            * 4)
            + (p.z() & 15) as usize / 4;
        let id = crate::grass::biome_at(&self.chunks[i].1, index)?;
        Ok(id
            .resource_location()
            .strip_prefix("minecraft:")
            .unwrap_or(id.resource_location()))
    }
}
struct Random(u64);
impl Random {
    fn new(seed: WorldSeed, source: ChunkPos, salt: u64) -> Self {
        Self(
            (seed.value() as u64)
                ^ (source.x() as u64).wrapping_mul(0x632b_e59b_d9b4_e019)
                ^ (source.z() as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ salt,
        )
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut v = self.0;
        v = (v ^ (v >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        v = (v ^ (v >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        v ^ (v >> 31)
    }
    fn int(&mut self, bound: i32) -> i32 {
        (self.next() % bound as u64) as i32
    }
}
pub(crate) struct PlayableFeatures {
    cache: Mutex<VecDeque<(ChunkPos, Arc<Plan>)>>,
    gates: ProductionGates,
}
impl PlayableFeatures {
    pub(crate) fn new() -> Result<Self, WorldError> {
        for name in [
            "short_grass",
            "fern",
            "dandelion",
            "poppy",
            "cactus",
            "sugar_cane",
            "sweet_berry_bush",
            "brown_mushroom",
            "sandstone",
            "sandstone_slab",
            "water",
            "spruce_log",
            "jungle_log",
            "acacia_log",
            "oak_log",
            "spruce_planks",
            "oak_planks",
            "cobblestone",
            "chest",
            "crafting_table",
            "cauldron",
            "potted_red_mushroom",
        ] {
            state(name, &[])?;
        }
        Ok(Self {
            cache: Mutex::new(VecDeque::with_capacity(CACHE_COUNT)),
            gates: ProductionGates::new(),
        })
    }
    fn cache_bytes(cache: &VecDeque<(ChunkPos, Arc<Plan>)>) -> usize {
        std::mem::size_of::<Self>()
            + cache.capacity() * std::mem::size_of::<(ChunkPos, Arc<Plan>)>()
            + cache.iter().map(|(_, p)| p.bytes()).sum::<usize>()
    }
    fn get(&self, source: ChunkPos) -> Result<Option<Arc<Plan>>, WorldError> {
        Ok(self
            .cache
            .lock()
            .map_err(|_| error("playable cache poisoned"))?
            .iter()
            .find(|(p, _)| *p == source)
            .map(|(_, p)| Arc::clone(p)))
    }
    fn source(
        &self,
        generator: &NaturalWorldGenerator,
        grass: &GrassPreview,
        seed: WorldSeed,
        source: ChunkPos,
    ) -> Result<Arc<Plan>, WorldError> {
        if let Some(p) = self.get(source)? {
            return Ok(p);
        }
        let _gate = self.gates.lock(source)?;
        if let Some(p) = self.get(source)? {
            return Ok(p);
        }
        let mut terrain = Raw {
            generator,
            grass,
            source,
            chunks: Vec::with_capacity(9),
            work: 0,
        };
        let plan = Arc::new(build(seed, source, &mut terrain)?);
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| error("playable cache poisoned"))?;
        while cache.len() >= CACHE_COUNT || Self::cache_bytes(&cache) + plan.bytes() > CACHE_BYTES {
            if cache.pop_front().is_none() {
                break;
            }
        }
        if Self::cache_bytes(&cache) + plan.bytes() <= CACHE_BYTES {
            cache.push_back((source, Arc::clone(&plan)));
        }
        Ok(plan)
    }
    pub(crate) fn apply(
        &self,
        generator: &NaturalWorldGenerator,
        grass: &GrassPreview,
        seed: WorldSeed,
        target: &mut Chunk,
    ) -> Result<(), WorldError> {
        let center = target.pos();
        // Global coordinate order defines overlap resolution independent of request order.
        for dx in -1..=1 {
            for dz in -1..=1 {
                let source = ChunkPos::new(
                    center
                        .x()
                        .checked_add(dx)
                        .ok_or_else(|| error("playable halo overflow"))?,
                    center
                        .z()
                        .checked_add(dz)
                        .ok_or_else(|| error("playable halo overflow"))?,
                );
                let plan = self.source(generator, grass, seed, source)?;
                project_blocks(&plan, target)?;
                if let Some((p, chest)) = &plan.chest {
                    if owner(*p) == center {
                        target.set_block_entity(*p, BlockEntity::Chest(chest.clone()))?;
                    }
                }
            }
        }
        Ok(())
    }
}
fn project_blocks(plan: &Plan, target: &mut Chunk) -> Result<(), WorldError> {
    for (p, write) in &plan.blocks {
        if owner(*p) != target.pos() {
            continue;
        }
        let current = target
            .get_block(*p)
            .ok_or_else(|| error("playable target block missing"))?;
        let new_log =
            state_id_to_block_name(write.state.as_u32()).is_some_and(|name| name.ends_with("_log"));
        let old_leaf =
            state_id_to_block_name(current.as_u32()).is_some_and(|name| name.ends_with("_leaves"));
        // Independent source crowns cannot erase or obstruct a complete trunk.
        if write.force
            || current.is_air()
            || named(current, "short_grass")
            || named(current, "fern")
            || named(current, "snow")
            || new_log && old_leaf
        {
            target.set_block(*p, write.state)?;
        }
    }
    Ok(())
}
fn replaceable(s: BlockStateId) -> bool {
    s.is_air()
        || ["short_grass", "fern", "snow", "vine"]
            .iter()
            .any(|n| named(s, n))
}
fn grassy(s: BlockStateId) -> bool {
    [
        "grass_block",
        "dirt",
        "podzol",
        "coarse_dirt",
        "mycelium",
        "mud",
        "moss_block",
        "pale_moss_block",
    ]
    .iter()
    .any(|n| named(s, n))
}
fn build(
    seed: WorldSeed,
    source: ChunkPos,
    terrain: &mut impl Terrain,
) -> Result<Plan, WorldError> {
    let mut plan = Plan::default();
    let base_x = source
        .x()
        .checked_mul(16)
        .ok_or_else(|| error("playable source overflow"))?;
    let base_z = source
        .z()
        .checked_mul(16)
        .ok_or_else(|| error("playable source overflow"))?;
    let mut rng = Random::new(seed, source, 0xf005_0005_5645_4745);
    structures(seed, source, terrain, &mut plan)?;
    for _ in 0..6 {
        let x = base_x + rng.int(16);
        let z = base_z + rng.int(16);
        let mut y = terrain.height(x, z)? + 1;
        if named(terrain.read(BlockPos::new(x, y - 1, z))?, "snow") {
            y -= 1;
        }
        if !(0..=300).contains(&y) || !grassy(terrain.read(BlockPos::new(x, y - 1, z))?) {
            continue;
        }
        let biome = terrain.biome(BlockPos::new(x, y, z))?;
        let species = if biome.contains("taiga") || biome == "grove" {
            Some("spruce")
        } else if biome.contains("jungle") {
            Some("jungle")
        } else if biome.contains("savanna") {
            Some("acacia")
        } else if biome.contains("dark_forest") {
            Some("dark_oak")
        } else if biome.contains("pale_garden") {
            Some("pale_oak")
        } else if biome.contains("mangrove_swamp") {
            Some("mangrove")
        } else if biome.contains("cherry_grove") {
            Some("cherry")
        } else if biome.contains("swamp") {
            Some("oak")
        } else {
            None
        };
        if let Some(species) = species {
            if !tree_allowed(seed, BlockPos::new(x, y, z), 2)? {
                continue;
            }
            tree(
                terrain,
                &mut plan,
                BlockPos::new(x, y, z),
                species,
                5 + rng.int(4),
            )?;
        }
    }
    vegetation(&mut rng, terrain, &mut plan, base_x, base_z)?;
    Ok(plan)
}
fn vegetation(
    rng: &mut Random,
    terrain: &mut impl Terrain,
    plan: &mut Plan,
    base_x: i32,
    base_z: i32,
) -> Result<(), WorldError> {
    for _ in 0..192 {
        let x = base_x + rng.int(16);
        let z = base_z + rng.int(16);
        let y = terrain.height(x, z)? + 1;
        if !(-60..=315).contains(&y) {
            continue;
        }
        let p = BlockPos::new(x, y, z);
        if plan.blocks.contains_key(&p) || !replaceable(terrain.read(p)?) {
            continue;
        }
        let ground = terrain.read(BlockPos::new(x, y - 1, z))?;
        let biome = terrain.biome(p)?;
        if (named(ground, "sand") || named(ground, "red_sand"))
            && (biome.contains("desert") || biome.contains("badlands"))
            && rng.int(20) == 0
            && x.rem_euclid(2) == 0
            && z.rem_euclid(2) == 0
        {
            cactus(terrain, plan, p, 1 + rng.int(3))?;
        }
        if (grassy(ground) || named(ground, "sand") || named(ground, "red_sand")) && rng.int(5) == 0
        {
            let mut water = false;
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                water |= named(terrain.read(BlockPos::new(x + dx, y - 1, z + dz))?, "water");
            }
            if water {
                for dy in 0..=rng.int(3) {
                    let p = BlockPos::new(x, y + dy, z);
                    if !terrain.read(p)?.is_air() {
                        break;
                    }
                    plan.put(p, state("sugar_cane", &[])?, false)?;
                }
                continue;
            }
        }
        if !grassy(ground) || biome.contains("snow") || biome.contains("frozen") {
            continue;
        }
        let roll = rng.int(24);
        let name = if biome.contains("taiga") {
            if roll == 0 {
                "sweet_berry_bush"
            } else {
                "fern"
            }
        } else if biome.contains("swamp")
            || biome.contains("dark_forest")
            || biome.contains("mushroom")
        {
            if roll < 3 {
                "brown_mushroom"
            } else {
                "short_grass"
            }
        } else {
            match roll {
                0 => "poppy",
                1 => "dandelion",
                2 if biome.contains("forest") => "lily_of_the_valley",
                _ => "short_grass",
            }
        };
        let properties: &[(&str, &str)] = if name == "sweet_berry_bush" {
            &[("age", "3")]
        } else {
            &[]
        };
        plan.put(p, state(name, properties)?, false)?;
    }
    Ok(())
}
fn cactus(
    terrain: &mut impl Terrain,
    plan: &mut Plan,
    p: BlockPos,
    height: i32,
) -> Result<(), WorldError> {
    // Even coordinate origins cannot create horizontally adjacent columns across
    // independently generated source seams. Every proposed layer is preflighted.
    for dy in 0..height {
        let pos = BlockPos::new(p.x(), p.y() + dy, p.z());
        if plan.blocks.contains_key(&pos) || !terrain.read(pos)?.is_air() {
            return Ok(());
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let neighbor = BlockPos::new(pos.x() + dx, pos.y(), pos.z() + dz);
            if plan
                .blocks
                .get(&neighbor)
                .is_some_and(|w| !w.state.is_air())
                || !terrain.read(neighbor)?.is_air()
            {
                return Ok(());
            }
        }
    }
    if !terrain
        .read(BlockPos::new(p.x(), p.y() + height, p.z()))?
        .is_air()
    {
        return Ok(());
    }
    let cactus = state("cactus", &[])?;
    for dy in 0..height {
        plan.put(BlockPos::new(p.x(), p.y() + dy, p.z()), cactus, false)?;
    }
    Ok(())
}
fn tree(
    terrain: &mut impl Terrain,
    plan: &mut Plan,
    p: BlockPos,
    species: &str,
    height: i32,
) -> Result<(), WorldError> {
    let mut candidate = BTreeMap::new();
    let log = state(&format!("{species}_log"), &[])?;
    let leaf = state(&format!("{species}_leaves"), &[("distance", "1")])?;
    for dy in 0..height {
        candidate.insert(BlockPos::new(p.x(), p.y() + dy, p.z()), log);
    }
    let layers: Vec<(i32, i32)> = if species == "spruce" {
        (2..=height)
            .map(|dy| {
                (
                    dy,
                    if dy == height {
                        0
                    } else {
                        1 + (height - dy) % 2
                    },
                )
            })
            .collect()
    } else {
        (height - 2..=height)
            .map(|dy| (dy, if dy == height { 1 } else { 2 }))
            .collect()
    };
    for (dy, r) in layers {
        for dx in -r..=r {
            for dz in -r..=r {
                if r > 0 && dx.abs() == r && dz.abs() == r {
                    continue;
                }
                candidate
                    .entry(BlockPos::new(p.x() + dx, p.y() + dy, p.z() + dz))
                    .or_insert(leaf);
            }
        }
    }
    // The entire crown is admitted before any write, including its adjacent chunks.
    for pos in candidate.keys() {
        if plan.blocks.contains_key(pos) || !replaceable(terrain.read(*pos)?) {
            return Ok(());
        }
    }
    for (pos, s) in candidate {
        plan.put(pos, s, false)?;
    }
    Ok(())
}
fn structure_source(seed: WorldSeed, source: ChunkPos) -> bool {
    let region = ChunkPos::new(source.x().div_euclid(8), source.z().div_euclid(8));
    let mut rng = Random::new(seed, region, 0xf005_0005_5354_5255);
    source.x().rem_euclid(8) == rng.int(8).clamp(1, 6)
        && source.z().rem_euclid(8) == rng.int(8).clamp(1, 6)
}
fn structure_anchor(seed: WorldSeed, source: ChunkPos) -> Result<BlockPos, WorldError> {
    let mut rng = Random::new(seed, source, 0xf005_0005_484f_5553);
    Ok(BlockPos::new(
        source
            .x()
            .checked_mul(16)
            .and_then(|x| x.checked_add(rng.int(16)))
            .ok_or_else(|| error("structure source overflow"))?,
        0,
        source
            .z()
            .checked_mul(16)
            .and_then(|z| z.checked_add(rng.int(16)))
            .ok_or_else(|| error("structure source overflow"))?,
    ))
}
/// Reserve potential complete structure rectangles, without generating or waiting
/// on another source plan. Failed structures keep this conservative clearing.
pub(crate) fn tree_allowed(
    seed: WorldSeed,
    origin: BlockPos,
    radius: i32,
) -> Result<bool, WorldError> {
    let center = owner(origin);
    for dx in -1..=1 {
        for dz in -1..=1 {
            let source = ChunkPos::new(
                center
                    .x()
                    .checked_add(dx)
                    .ok_or_else(|| error("tree reservation halo overflow"))?,
                center
                    .z()
                    .checked_add(dz)
                    .ok_or_else(|| error("tree reservation halo overflow"))?,
            );
            if !structure_source(seed, source) {
                continue;
            }
            let anchor = structure_anchor(seed, source)?;
            let (x, z) = (anchor.x(), anchor.z());
            if (i64::from(origin.x()) - i64::from(x)).abs() <= i64::from(3 + radius)
                && (i64::from(origin.z()) - i64::from(z)).abs() <= i64::from(4 + radius)
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn structures(
    seed: WorldSeed,
    source: ChunkPos,
    terrain: &mut impl Terrain,
    plan: &mut Plan,
) -> Result<(), WorldError> {
    if !structure_source(seed, source) {
        return Ok(());
    }
    let mut rng = Random::new(seed, source, 0xf005_0005_484f_5553);
    let anchor = structure_anchor(seed, source)?;
    let (x, z) = (anchor.x(), anchor.z());
    // Keep the shape/loot stream independent of the two anchor draws.
    rng.int(16);
    rng.int(16);
    let y = terrain.height(x, z)?;
    if !(1..=300).contains(&y) {
        return Ok(());
    }
    let biome = terrain.biome(BlockPos::new(x, y, z))?;
    if biome.contains("desert") {
        well(terrain, plan, BlockPos::new(x, y, z))?;
    } else if biome == "swamp" {
        hut(terrain, plan, BlockPos::new(x, y, z), true, &mut rng)?;
    } else if [
        "plains",
        "forest",
        "birch_forest",
        "taiga",
        "savanna",
        "meadow",
    ]
    .iter()
    .any(|name| biome.contains(name))
    {
        hut(terrain, plan, BlockPos::new(x, y, z), false, &mut rng)?;
    }
    Ok(())
}
fn well(terrain: &mut impl Terrain, plan: &mut Plan, p: BlockPos) -> Result<(), WorldError> {
    if !named(terrain.read(p)?, "sand") {
        return Ok(());
    }
    for dx in -2..=2 {
        for dz in -2..=2 {
            if terrain
                .read(BlockPos::new(p.x() + dx, p.y() - 1, p.z() + dz))?
                .is_air()
                && terrain
                    .read(BlockPos::new(p.x() + dx, p.y() - 2, p.z() + dz))?
                    .is_air()
            {
                return Ok(());
            }
            for dy in 1..=4 {
                if !replaceable(terrain.read(BlockPos::new(p.x() + dx, p.y() + dy, p.z() + dz))?) {
                    return Ok(());
                }
            }
        }
    }
    let sandstone = state("sandstone", &[])?;
    let slab = state("sandstone_slab", &[])?;
    for dx in -2..=2 {
        for dz in -2..=2 {
            for dy in -2..=0 {
                plan.put(
                    BlockPos::new(p.x() + dx, p.y() + dy, p.z() + dz),
                    sandstone,
                    true,
                )?;
            }
            if dx.abs() == 2 || dz.abs() == 2 {
                plan.put(
                    BlockPos::new(p.x() + dx, p.y() + 1, p.z() + dz),
                    if dx == 0 || dz == 0 { slab } else { sandstone },
                    true,
                )?;
            }
        }
    }
    for (dx, dz) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
        plan.put(
            BlockPos::new(p.x() + dx, p.y(), p.z() + dz),
            state("water", &[])?,
            true,
        )?;
        plan.put(
            BlockPos::new(p.x() + dx, p.y() - 1, p.z() + dz),
            state("sand", &[])?,
            true,
        )?;
    }
    for dx in -1..=1 {
        for dz in -1..=1 {
            plan.put(
                BlockPos::new(p.x() + dx, p.y() + 4, p.z() + dz),
                if dx == 0 && dz == 0 { sandstone } else { slab },
                true,
            )?;
        }
    }
    for dx in [-1, 1] {
        for dz in [-1, 1] {
            for dy in 1..=3 {
                plan.put(
                    BlockPos::new(p.x() + dx, p.y() + dy, p.z() + dz),
                    sandstone,
                    true,
                )?;
            }
        }
    }
    Ok(())
}
fn hut(
    terrain: &mut impl Terrain,
    plan: &mut Plan,
    anchor: BlockPos,
    swamp: bool,
    rng: &mut Random,
) -> Result<(), WorldError> {
    let mut minimum = 320;
    let mut maximum = -64;
    for dx in -3..=3 {
        for dz in -4..=4 {
            let h = terrain.height(anchor.x() + dx, anchor.z() + dz)?;
            minimum = minimum.min(h);
            maximum = maximum.max(h);
            let ground = terrain.read(BlockPos::new(anchor.x() + dx, h, anchor.z() + dz))?;
            if !(grassy(ground) || (swamp && named(ground, "water"))) {
                return Ok(());
            }
        }
    }
    if maximum - minimum > 3 || maximum > 300 {
        return Ok(());
    }
    let base = maximum + 1;
    let floor = if swamp { base + 1 } else { base };
    // All footprint space is validated before admitting this indivisible source plan.
    for dx in -3..=3 {
        for dz in -4..=4 {
            for y in base..=floor + 4 {
                if !replaceable(terrain.read(BlockPos::new(anchor.x() + dx, y, anchor.z() + dz))?) {
                    return Ok(());
                }
            }
        }
    }
    if swamp {
        return swamp_shell(terrain, plan, anchor, maximum);
    }
    let planks = state(if swamp { "spruce_planks" } else { "oak_planks" }, &[])?;
    let stone = state("cobblestone", &[])?;
    let log = state("oak_log", &[])?;
    for dx in -3..=3 {
        for dz in -4..=4 {
            for y in minimum + 1..=floor {
                let s = if swamp { planks } else { stone };
                plan.put(BlockPos::new(anchor.x() + dx, y, anchor.z() + dz), s, true)?;
            }
            for dy in 1..=3 {
                let edge = dx.abs() == 3 || dz.abs() == 4;
                let doorway = dz == -4 && dx == 0 && dy <= 2;
                let window = dy == 2 && (dx.abs() == 3 && dz == 0 || dz == 4 && dx == 0);
                let s = if edge && !doorway && !window {
                    planks
                } else {
                    BlockStateId::AIR
                };
                plan.put(
                    BlockPos::new(anchor.x() + dx, floor + dy, anchor.z() + dz),
                    s,
                    true,
                )?;
            }
            if swamp || rng.int(8) != 0 {
                plan.put(
                    BlockPos::new(anchor.x() + dx, floor + 4, anchor.z() + dz),
                    planks,
                    true,
                )?;
            }
        }
    }
    for dx in [-3, 3] {
        for dz in [-4, 4] {
            for y in minimum..=floor + 3 {
                plan.put(
                    BlockPos::new(anchor.x() + dx, y, anchor.z() + dz),
                    log,
                    true,
                )?;
            }
        }
    }
    plan.put(
        BlockPos::new(anchor.x() + 1, floor + 1, anchor.z() + 2),
        state("crafting_table", &[])?,
        true,
    )?;
    if swamp {
        plan.put(
            BlockPos::new(anchor.x() + 2, floor + 1, anchor.z() + 2),
            state("cauldron", &[])?,
            true,
        )?;
    } else {
        let chest_pos = BlockPos::new(anchor.x() - 1, floor + 1, anchor.z() + 2);
        plan.put(chest_pos, state("chest", &[("facing", "south")])?, true)?;
        let chest = cabin_loot(rng)?;
        plan.chest = Some((chest_pos, chest));
    }
    Ok(())
}

fn swamp_shell(
    terrain: &mut impl Terrain,
    plan: &mut Plan,
    anchor: BlockPos,
    base: i32,
) -> Result<(), WorldError> {
    let at = |x, y, z| BlockPos::new(anchor.x() - 3 + x, base + y, anchor.z() - 4 + z);
    let planks = state("spruce_planks", &[])?;
    let log = state("oak_log", &[])?;
    // Declarative shell: floor, walls and roof overhang of the pinned 7×9 hut.
    for x in 0..=6 {
        for z in 0..=8 {
            for y in 1..=4 {
                let floor = y == 1
                    && ((1..=5).contains(&x) && (1..=7).contains(&z)
                        || (2..=4).contains(&x) && z == 0);
                let roof = y == 4 && (1..=5).contains(&x) && (2..=7).contains(&z);
                let wall = (2..=3).contains(&y)
                    && ((2..=3).contains(&x) && z == 2
                        || (x == 1 || x == 5) && (3..=6).contains(&z)
                        || (2..=4).contains(&x) && z == 7);
                plan.put(
                    at(x, y, z),
                    if floor || roof || wall {
                        planks
                    } else {
                        BlockStateId::AIR
                    },
                    true,
                )?;
            }
        }
    }
    for (x, z) in [(1, 2), (5, 2), (1, 7), (5, 7)] {
        let mut support = base - 1;
        for _ in 0..16 {
            let s = terrain.read(at(x, support - base, z))?;
            if !s.is_air() && !named(s, "water") {
                break;
            }
            support -= 1;
        }
        for y in support - base..=3 {
            plan.put(at(x, y, z), log, true)?;
        }
    }
    for (x, y, z) in [(2, 3, 2), (3, 3, 7), (1, 2, 1), (5, 2, 1)] {
        plan.put(at(x, y, z), state("oak_fence", &[])?, true)?;
    }
    for (x, y, z) in [(1, 3, 4), (5, 3, 4), (5, 3, 5)] {
        plan.put(at(x, y, z), BlockStateId::AIR, true)?;
    }
    plan.put(at(1, 3, 5), state("potted_red_mushroom", &[])?, true)?;
    plan.put(at(3, 2, 6), state("crafting_table", &[])?, true)?;
    plan.put(at(4, 2, 6), state("cauldron", &[])?, true)?;
    for x in 0..=6 {
        plan.put(
            at(x, 4, 1),
            state("spruce_stairs", &[("facing", "north")])?,
            true,
        )?;
        plan.put(
            at(x, 4, 8),
            state("spruce_stairs", &[("facing", "south")])?,
            true,
        )?;
    }
    for z in 2..=7 {
        plan.put(
            at(0, 4, z),
            state("spruce_stairs", &[("facing", "east")])?,
            true,
        )?;
        plan.put(
            at(6, 4, z),
            state("spruce_stairs", &[("facing", "west")])?,
            true,
        )?;
    }
    for (x, z, facing, shape) in [
        (0, 1, "north", "outer_right"),
        (6, 1, "north", "outer_left"),
        (0, 8, "south", "outer_left"),
        (6, 8, "south", "outer_right"),
    ] {
        plan.put(
            at(x, 4, z),
            state("spruce_stairs", &[("facing", facing), ("shape", shape)])?,
            true,
        )?;
    }
    Ok(())
}

fn cabin_loot(rng: &mut Random) -> Result<ChestInventory, WorldError> {
    let mut chest = ChestInventory::new();
    for (slot, (name, maximum)) in [
        ("bread", 5),
        ("coal", 8),
        ("iron_nugget", 12),
        ("wheat", 6),
        ("oak_sapling", 3),
    ]
    .into_iter()
    .enumerate()
    {
        let item = ItemId::from_name(name).ok_or_else(|| error("missing cabin loot item"))?;
        let stack = ItemStack::try_new(item, (1 + rng.int(maximum)) as u8, ComponentPatch::empty())
            .map_err(error)?;
        *chest
            .slot_mut(slot)
            .ok_or_else(|| error("cabin loot slot"))? = stack;
    }
    Ok(chest)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Flat {
        ground: BlockStateId,
        biome: &'static str,
        blocked: Option<BlockPos>,
        bank: bool,
    }
    impl Terrain for Flat {
        fn height(&mut self, _: i32, _: i32) -> Result<i32, WorldError> {
            Ok(64)
        }
        fn read(&mut self, p: BlockPos) -> Result<BlockStateId, WorldError> {
            if self.blocked == Some(p) {
                return state("stone", &[]);
            }
            if self.bank && p.x().rem_euclid(8) == 0 && p.y() == 64 {
                return state("water", &[]);
            }
            Ok(if p.y() <= 64 {
                self.ground
            } else {
                BlockStateId::AIR
            })
        }
        fn biome(&mut self, _: BlockPos) -> Result<&'static str, WorldError> {
            Ok(self.biome)
        }
    }
    fn flat(biome: &'static str, ground: &str) -> Flat {
        Flat {
            ground: state(ground, &[]).unwrap(),
            biome,
            blocked: None,
            bank: false,
        }
    }
    fn selected(seed: WorldSeed, region: ChunkPos) -> ChunkPos {
        let mut random = Random::new(seed, region, 0xf005_0005_5354_5255);
        ChunkPos::new(
            region.x() * 8 + random.int(8).clamp(1, 6),
            region.z() * 8 + random.int(8).clamp(1, 6),
        )
    }
    #[test]
    fn whole_cabin_crosses_signed_seams_with_checked_loot_and_tree_reservations() {
        for seed in [0, -1, i64::MIN, i64::MAX] {
            let seed = WorldSeed::new(seed);
            let source = (-8..=8)
                .map(|x| selected(seed, ChunkPos::new(x, -1)))
                .find(|source| {
                    let anchor = structure_anchor(seed, *source).unwrap();
                    let (x, z) = (anchor.x(), anchor.z());
                    (x & 15) < 3 || (x & 15) > 12 || (z & 15) < 4 || (z & 15) > 11
                })
                .unwrap();
            let mut terrain = flat("plains", "grass_block");
            let plan = build(seed, source, &mut terrain).unwrap();
            let (chest_pos, chest) = plan.chest.as_ref().unwrap();
            assert_eq!(
                chest.slots().iter().filter(|s| s.item().is_some()).count(),
                5
            );
            assert!(chest.slots().iter().all(ItemStack::is_valid));
            let owners: std::collections::BTreeSet<_> = plan
                .blocks
                .iter()
                .filter(|(_, w)| w.force)
                .map(|(p, _)| owner(*p))
                .collect();
            assert!(owners.len() > 1, "{seed:?} {source:?}");
            for p in plan
                .blocks
                .keys()
                .filter(|p| named(plan.blocks[p].state, "chest"))
            {
                assert!(!tree_allowed(seed, *p, 3).unwrap());
            }
            let repeated = build(seed, source, &mut flat("plains", "grass_block")).unwrap();
            assert_eq!(
                plan.blocks.keys().collect::<Vec<_>>(),
                repeated.blocks.keys().collect::<Vec<_>>()
            );
            assert_eq!(plan.chest, repeated.chest);
            assert!(named(plan.blocks[chest_pos].state, "chest"));
        }
    }
    #[test]
    fn competing_signed_source_crowns_never_obstruct_or_erase_complete_trunks() {
        for x in [-17, -1, 15, 31] {
            let base = BlockPos::new(x, 65, -1);
            let mut crown = Plan::default();
            let mut trunk = Plan::default();
            for dy in 0..6 {
                let p = BlockPos::new(base.x(), base.y() + dy, base.z());
                crown
                    .put(p, state("oak_leaves", &[]).unwrap(), false)
                    .unwrap();
                trunk
                    .put(p, state("spruce_log", &[]).unwrap(), false)
                    .unwrap();
            }
            let mut first = Chunk::new(owner(base));
            project_blocks(&crown, &mut first).unwrap();
            project_blocks(&trunk, &mut first).unwrap();
            let mut reversed = Chunk::new(owner(base));
            project_blocks(&trunk, &mut reversed).unwrap();
            project_blocks(&crown, &mut reversed).unwrap();
            for y in -64..320 {
                for local_x in 0..16 {
                    for local_z in 0..16 {
                        let p = BlockPos::new(
                            first.pos().x() * 16 + local_x,
                            y,
                            first.pos().z() * 16 + local_z,
                        );
                        assert_eq!(
                            first.get_block(p),
                            reversed.get_block(p),
                            "semantic projection {p:?}"
                        );
                    }
                }
            }
            assert!(trunk
                .blocks
                .iter()
                .all(|(p, w)| first.get_block(*p) == Some(w.state)));
        }
    }
    #[test]
    fn neighboring_signed_region_structure_rectangles_cannot_overlap() {
        for seed in [0, -1, i64::MIN, i64::MAX] {
            for rx in -3..=3 {
                for rz in -3..=3 {
                    let seed = WorldSeed::new(seed);
                    let origin = selected(seed, ChunkPos::new(rx, rz));
                    assert!(structure_source(seed, origin));
                    let anchor = structure_anchor(seed, origin).unwrap();
                    for (dx, dz) in [(1, 0), (0, 1), (1, 1), (1, -1)] {
                        let next = selected(seed, ChunkPos::new(rx + dx, rz + dz));
                        let other = structure_anchor(seed, next).unwrap();
                        assert!(
                            (anchor.x() - other.x()).abs() > 6
                                || (anchor.z() - other.z()).abs() > 8
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn obstruction_refuses_entire_structure_and_entire_tree_before_writes() {
        let mut plan = Plan::default();
        let mut terrain = flat("plains", "grass_block");
        terrain.blocked = Some(BlockPos::new(16, 67, 16));
        hut(
            &mut terrain,
            &mut plan,
            BlockPos::new(16, 64, 16),
            false,
            &mut Random::new(WorldSeed::new(0), ChunkPos::ORIGIN, 0),
        )
        .unwrap();
        assert!(plan.blocks.is_empty());
        assert!(plan.chest.is_none());
        terrain.blocked = Some(BlockPos::new(17, 70, 16));
        tree(
            &mut terrain,
            &mut plan,
            BlockPos::new(16, 65, 16),
            "jungle",
            6,
        )
        .unwrap();
        assert!(plan.blocks.is_empty());
        terrain.blocked = Some(BlockPos::new(17, 67, 16));
        cactus(&mut terrain, &mut plan, BlockPos::new(16, 65, 16), 3).unwrap();
        assert!(plan.blocks.is_empty());
        terrain.blocked = None;
        plan.put(
            BlockPos::new(17, 66, 16),
            state("cactus", &[]).unwrap(),
            false,
        )
        .unwrap();
        cactus(&mut terrain, &mut plan, BlockPos::new(16, 65, 16), 3).unwrap();
        assert_eq!(plan.blocks.len(), 1);
    }
    #[test]
    fn botanical_routes_supply_real_supported_states_without_dry_sugar_cane() {
        let source = ChunkPos::new(0, 0);
        for (biome, log) in [
            ("taiga", "spruce_log"),
            ("jungle", "jungle_log"),
            ("savanna", "acacia_log"),
            ("dark_forest", "dark_oak_log"),
            ("pale_garden", "pale_oak_log"),
            ("mangrove_swamp", "mangrove_log"),
            ("cherry_grove", "cherry_log"),
        ] {
            let plan = build(WorldSeed::new(0), source, &mut flat(biome, "grass_block")).unwrap();
            assert!(plan.blocks.values().any(|w| named(w.state, log)), "{biome}");
            assert!(!plan.blocks.values().any(|w| named(w.state, "sugar_cane")));
        }
        let dry = build(WorldSeed::new(0), source, &mut flat("desert", "sand")).unwrap();
        assert!(dry.blocks.values().any(|w| named(w.state, "cactus")));
        let mut bank = flat("plains", "grass_block");
        bank.bank = true;
        let wet = build(WorldSeed::new(0), source, &mut bank).unwrap();
        assert!(wet.blocks.values().any(|w| named(w.state, "sugar_cane")));
        for (p, _) in wet
            .blocks
            .iter()
            .filter(|(_, w)| named(w.state, "sugar_cane"))
        {
            assert!([1, 7].contains(&p.x().rem_euclid(8))); // actual adjacent water column
        }
    }
    #[test]
    fn desert_well_and_swamp_shell_have_real_ground_support_and_complete_layouts() {
        let mut well_plan = Plan::default();
        well(
            &mut flat("desert", "sand"),
            &mut well_plan,
            BlockPos::new(-1, 64, -1),
        )
        .unwrap();
        assert_eq!(
            well_plan
                .blocks
                .values()
                .filter(|w| named(w.state, "water"))
                .count(),
            5
        );
        assert!(well_plan.chest.is_none());
        let mut unsupported = flat("desert", "sand");
        unsupported.ground = BlockStateId::AIR;
        let mut refused = Plan::default();
        well(&mut unsupported, &mut refused, BlockPos::new(0, 64, 0)).unwrap();
        assert!(refused.blocks.is_empty());
        let mut swamp = Plan::default();
        hut(
            &mut flat("swamp", "grass_block"),
            &mut swamp,
            BlockPos::new(15, 64, 15),
            true,
            &mut Random(0),
        )
        .unwrap();
        assert!(swamp.blocks.values().any(|w| named(w.state, "cauldron")));
        assert!(swamp
            .blocks
            .values()
            .any(|w| named(w.state, "potted_red_mushroom")));
        assert_eq!(
            swamp
                .blocks
                .values()
                .filter(|w| named(w.state, "spruce_stairs"))
                .count(),
            26
        );
        assert_eq!(
            swamp.blocks[&BlockPos::new(12, 68, 12)].state,
            state(
                "spruce_stairs",
                &[("facing", "north"), ("shape", "outer_right")]
            )
            .unwrap()
        );
        assert!(swamp.chest.is_none());
    }
    #[test]
    fn explicit_work_and_plan_caps_refuse_without_exceeding_retained_limits() {
        let generator = NaturalWorldGenerator::with_playable(WorldSeed::new(0)).unwrap();
        let grass = generator.grass_preview().unwrap();
        let mut raw = Raw {
            generator: &generator,
            grass,
            source: ChunkPos::ORIGIN,
            chunks: Vec::new(),
            work: MAX_SOURCE_WORK,
        };
        assert!(raw.read(BlockPos::new(0, 64, 0)).is_err());
        assert_eq!(raw.chunks.len(), 0);
        let mut plan = Plan::default();
        for n in 0..MAX_WRITES {
            plan.put(
                BlockPos::new(i32::try_from(n).unwrap(), 64, 0),
                BlockStateId::AIR,
                false,
            )
            .unwrap();
        }
        assert!(plan
            .put(
                BlockPos::new(i32::try_from(MAX_WRITES).unwrap(), 64, 0),
                BlockStateId::AIR,
                false
            )
            .is_err());
        assert_eq!(plan.blocks.len(), MAX_WRITES);
        assert!(plan.bytes() < CACHE_BYTES);
    }
    #[test]
    fn real_pipeline_structure_projects_complete_native_chest_with_request_order_and_restart() {
        let seed = WorldSeed::new(0);
        let generator = NaturalWorldGenerator::with_playable(seed).unwrap();
        let features = PlayableFeatures::new().unwrap();
        let grass = generator.grass_preview().unwrap();
        let mut found = None;
        'scan: for region_x in -2..=2 {
            for region_z in -2..=2 {
                let source = selected(seed, ChunkPos::new(region_x, region_z));
                let plan = features.source(&generator, grass, seed, source).unwrap();
                if let Some((p, _)) = &plan.chest {
                    found = Some((*p, source, Arc::clone(&plan)));
                    break 'scan;
                }
            }
        }
        let (p, source, plan) = found.expect("seed0 has an admitted nearby cabin");
        println!(
            "PLAYABLE_STRUCTURE seed=0 chest={p:?} source={:?} worldgen_revision=5",
            owner(p)
        );
        // Instrument only retained unrelated entries to force the genuine source
        // publication path to evict. Recompute uses the real numerical terrain.
        {
            let mut cache = features.cache.lock().unwrap();
            cache.clear();
            cache.push_back((source, Arc::clone(&plan)));
            for i in 1..CACHE_COUNT {
                cache.push_back((
                    ChunkPos::new(-10000 - i32::try_from(i).unwrap(), -10000),
                    Arc::new(Plan::default()),
                ));
            }
        }
        let other = ChunkPos::new(source.x() + 1, source.z());
        features.source(&generator, grass, seed, other).unwrap();
        assert!(features.get(source).unwrap().is_none());
        assert_eq!(features.cache.lock().unwrap().len(), CACHE_COUNT);
        assert!(PlayableFeatures::cache_bytes(&features.cache.lock().unwrap()) <= CACHE_BYTES);
        let recomputed = features.source(&generator, grass, seed, source).unwrap();
        assert!(!Arc::ptr_eq(&plan, &recomputed));
        assert_eq!(plan.chest, recomputed.chest);
        assert_eq!(
            plan.blocks
                .iter()
                .map(|(p, w)| (*p, w.state, w.force))
                .collect::<Vec<_>>(),
            recomputed
                .blocks
                .iter()
                .map(|(p, w)| (*p, w.state, w.force))
                .collect::<Vec<_>>()
        );
        let owners: std::collections::BTreeSet<_> = plan
            .blocks
            .iter()
            .filter(|(_, w)| w.force)
            .map(|(p, _)| owner(*p))
            .collect();
        let mut generated = BTreeMap::new();
        for target in &owners {
            generated.insert(*target, generator.try_generate_chunk(*target).unwrap());
        }
        let fresh = NaturalWorldGenerator::with_playable(seed).unwrap();
        for target in owners.iter().rev() {
            let next = fresh.try_generate_chunk(*target).unwrap();
            assert_eq!(
                generated[target], next,
                "independent request order and process binding {target:?}"
            );
            for (pos, w) in plan
                .blocks
                .iter()
                .filter(|(pos, w)| owner(**pos) == *target && w.force)
            {
                assert_eq!(next.get_block(*pos), Some(w.state));
            }
            assert!(!next.persist_dirty_sections().any());
            assert!(!next.persist_edited_sections().any());
        }
        assert_eq!(
            generated[&owner(p)].block_entity(p),
            Some(&BlockEntity::Chest(plan.chest.as_ref().unwrap().1.clone()))
        );
        assert_eq!(
            generator.identity(),
            crate::GenerationIdentity::natural_overworld_playable(seed)
        );
        assert_ne!(
            generator.identity(),
            NaturalWorldGenerator::with_survival_preview(seed)
                .unwrap()
                .identity()
        );
    }
}
