//! Original short-grass kernels under an explicit isolated-source preview policy.
use crate::{
    biome_zoom::{BiomeZoom, BiomeZoomSeed, BiomeZoomSemantics},
    random::WorldgenRandomSource,
    BlockStateId, Chunk, HeightmapKind, NaturalGenerationStage, NaturalWorldGenerator, WorldError,
    WorldSeed,
};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::{
    biome::{biome_resources, BiomeResourceId},
    block_state::block_metadata,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

fn error(detail: impl std::fmt::Display) -> WorldError {
    WorldError::NaturalGeneration {
        stage: NaturalGenerationStage::Features,
        detail: detail.to_string().chars().take(512).collect(),
    }
}
fn resolve_state(name: &str) -> Result<BlockStateId, WorldError> {
    block_metadata(name)
        .map(|block| BlockStateId::new(block.default_state))
        .ok_or_else(|| error("unknown grass state resource"))
}
fn json(source: &str) -> Result<serde_json::Value, WorldError> {
    serde_json::from_str(source).map_err(error)
}
struct Catalog {
    memberships: Vec<(BiomeResourceId, [bool; 2])>,
    support: Vec<(u32, u32)>,
    grass: BlockStateId,
    tries: usize,
    spread: [u32; 3],
    indices: [i64; 2],
}
impl Catalog {
    fn compile() -> Result<Self, WorldError> {
        let patch = json(include_str!("grass_resources/patch.json"))?;
        let config = &patch["config"];
        if patch["type"] != "minecraft:random_patch"
            || config["feature"]["feature"]["type"] != "minecraft:simple_block"
            || config["feature"]["feature"]["config"]["to_place"]["type"]
                != "minecraft:simple_state_provider"
            || config["feature"]["placement"][0]["predicate"]["blocks"] != "minecraft:air"
        {
            return Err(error("unsupported original grass descriptor"));
        }
        let provider = &config["feature"]["feature"]["config"]["to_place"]["state"]["Name"];
        let grass = resolve_state(
            provider
                .as_str()
                .ok_or_else(|| error("grass state missing"))?,
        )?;
        let integer = |key: &str| {
            config[key]
                .as_u64()
                .ok_or_else(|| error("grass integer missing"))
        };
        let tries = usize::try_from(integer("tries")?).map_err(error)?;
        let xz = u32::try_from(integer("xz_spread")?).map_err(error)?;
        let y = u32::try_from(integer("y_spread")?).map_err(error)?;
        if tries != 32 || xz != 7 || y != 3 {
            return Err(error("grass projection bounds changed"));
        }
        let plain = json(include_str!("grass_resources/plain.json"))?;
        let forest = json(include_str!("grass_resources/forest.json"))?;
        for placed in [&plain, &forest] {
            if placed["feature"] != "minecraft:patch_grass"
                || placed["placement"][1]["type"] != "minecraft:in_square"
                || placed["placement"][2]["heightmap"] != "WORLD_SURFACE_WG"
                || placed["placement"][3]["type"] != "minecraft:biome"
            {
                return Err(error("unsupported grass placement"));
            }
        }
        if plain["placement"][0]["noise_level"] != -0.8
            || plain["placement"][0]["below_noise"] != 5
            || plain["placement"][0]["above_noise"] != 10
            || forest["placement"][0]["count"] != 2
        {
            return Err(error("grass counts changed"));
        }
        let mut memberships = Vec::with_capacity(65);
        for biome in biome_resources() {
            let def = json(biome.definition().json())?;
            let steps = def["features"]
                .as_array()
                .ok_or_else(|| error("biome feature list missing"))?;
            if steps.iter().any(|step| !step.is_array()) {
                return Err(error("invalid biome feature step"));
            }
            memberships.push((
                biome,
                [
                    "minecraft:patch_grass_plain",
                    "minecraft:patch_grass_forest",
                ]
                .map(|name| {
                    steps.iter().any(|step| {
                        step.as_array()
                            .is_some_and(|features| features.iter().any(|value| value == name))
                    })
                }),
            ));
        }
        let support = support_ranges()?;
        let mut indices = [0; 2];
        for (slot, line) in include_str!("grass_resources/indices.tsv")
            .lines()
            .enumerate()
        {
            let step_index = line
                .split('\t')
                .nth(1)
                .ok_or_else(|| error("missing original grass index"))?;
            let (step, index) = step_index
                .split_once(',')
                .ok_or_else(|| error("invalid original grass index"))?;
            if step != "9" || slot >= 2 {
                return Err(error("grass index step changed"));
            }
            indices[slot] = index.parse::<i64>().map_err(error)?;
        }
        if indices != [54, 59] {
            return Err(error("original grass indices changed"));
        }
        Ok(Self {
            memberships,
            support,
            grass,
            tries,
            spread: [xz + 1, y + 1, xz + 1],
            indices,
        })
    }
    fn supports(&self, state: BlockStateId) -> bool {
        self.support
            .iter()
            .any(|(min, max)| (*min..=*max).contains(&state.as_u32()))
    }
    fn member(&self, biome: BiomeResourceId, feature: usize) -> bool {
        self.memberships
            .iter()
            .any(|(id, flags)| *id == biome && flags[feature])
    }
}
fn support_ranges() -> Result<Vec<(u32, u32)>, WorldError> {
    let dirt = json(include_str!("grass_resources/dirt.json"))?;
    let mut support = Vec::with_capacity(11);
    for name in dirt["values"]
        .as_array()
        .ok_or_else(|| error("dirt tag missing"))?
        .iter()
        .map(|v| v.as_str())
        .chain([Some("minecraft:farmland")])
    {
        let block = block_metadata(name.ok_or_else(|| error("invalid dirt tag member"))?)
            .ok_or_else(|| error("unknown dirt support"))?;
        support.push((block.min_state, block.max_state));
    }
    if support.len() != 11 {
        return Err(error("original dirt support census changed"));
    }
    Ok(support)
}
// Adapter, not the direct Xoroshiro API: Java longs consume TWO next(32)s.
struct Random {
    core: WorldgenRandomSource,
    remaining: u32,
}
impl Random {
    fn new(seed: WorldSeed) -> Self {
        Self {
            core: WorldgenRandomSource::xoroshiro(seed),
            remaining: 65_536,
        }
    }
    fn bits(&mut self, count: u32) -> Result<u32, WorldError> {
        if self.remaining == 0 {
            return Err(error("grass RNG allowance exhausted"));
        }
        self.remaining -= 1;
        Ok((self.core.next_long() as u64 >> (64 - count)) as u32)
    }
    fn long(&mut self) -> Result<i64, WorldError> {
        let high = i64::from(self.bits(32)?.cast_signed());
        let low = i64::from(self.bits(32)?.cast_signed());
        Ok(high.wrapping_shl(32).wrapping_add(low))
    }
    fn int(&mut self, bound: u32) -> Result<i32, WorldError> {
        // The admitted descriptors use only power-of-two bounds. Refuse any
        // extension before drawing; no rejection loop belongs to this subset.
        if !matches!(bound, 4 | 8 | 16) {
            return Err(error("unsupported grass RNG bound"));
        }
        Ok(((u64::from(bound) * u64::from(self.bits(31)?)) >> 31) as i32)
    }
    fn feature(&mut self, seed: WorldSeed, source: ChunkPos, index: i64) -> Result<(), WorldError> {
        self.core = WorldgenRandomSource::xoroshiro(seed);
        let x = self.long()? | 1;
        let z = self.long()? | 1;
        let deco = i64::from(source.x().wrapping_mul(16))
            .wrapping_mul(x)
            .wrapping_add(i64::from(source.z().wrapping_mul(16)).wrapping_mul(z))
            ^ seed.value();
        self.core = WorldgenRandomSource::xoroshiro(WorldSeed::new(
            deco.wrapping_add(index).wrapping_add(90_000),
        ));
        Ok(())
    }
}
pub(crate) trait Region {
    fn height(&mut self, x: i32, z: i32) -> Result<i32, WorldError>;
    fn biome(&mut self, pos: BlockPos) -> Result<BiomeResourceId, WorldError>;
    fn relevant(&self, pos: BlockPos) -> bool;
    fn state(&mut self, pos: BlockPos) -> Result<BlockStateId, WorldError>;
    fn write(&mut self, pos: BlockPos) -> Result<(), WorldError>;
}
fn place(
    catalog: &Catalog,
    seed: WorldSeed,
    source: ChunkPos,
    counts: [usize; 2],
    region: &mut impl Region,
) -> Result<(), WorldError> {
    let mut random = Random::new(seed);
    for (feature, count) in counts.into_iter().enumerate() {
        random.feature(seed, source, catalog.indices[feature])?;
        for _ in 0..count {
            let x = source.x().wrapping_mul(16).wrapping_add(random.int(16)?);
            let z = source.z().wrapping_mul(16).wrapping_add(random.int(16)?);
            let y = region.height(x, z)?;
            // HeightmapPlacement yields no position at/below the world minimum.
            if y <= -64 {
                continue;
            }
            let origin = BlockPos::new(x, y, z);
            if !catalog.member(region.biome(origin)?, feature) {
                continue;
            }
            for _ in 0..catalog.tries {
                let dx = random.int(catalog.spread[0])? - random.int(catalog.spread[0])?;
                let dy = random.int(catalog.spread[1])? - random.int(catalog.spread[1])?;
                let dz = random.int(catalog.spread[2])? - random.int(catalog.spread[2])?;
                let pos = BlockPos::new(x.wrapping_add(dx), y.wrapping_add(dy), z.wrapping_add(dz));
                // No later RNG depends on predicates. Outside source/target cannot
                // affect a later source height or a retained support/occupancy read.
                if !region.relevant(pos) {
                    continue;
                }
                if region.state(pos)? != BlockStateId::AIR {
                    continue;
                }
                if !catalog.supports(region.state(BlockPos::new(pos.x(), pos.y() - 1, pos.z()))?) {
                    continue;
                }
                if (-64..320).contains(&pos.y()) {
                    region.write(pos)?;
                }
            }
        }
    }
    Ok(())
}
pub(super) fn biome_at(raw: &Chunk, index: usize) -> Result<BiomeResourceId, WorldError> {
    raw.biomes()
        .and_then(|values| values.get(index))
        .copied()
        .ok_or_else(|| error("raw biomes missing"))
}

pub(super) fn biome_location(zoom: BiomeZoom, pos: BlockPos) -> (ChunkPos, usize) {
    let quart = zoom.select_quart(pos);
    let owner = ChunkPos::new(quart.x() >> 2, quart.z() >> 2);
    let index = (((quart.y().clamp(-16, 79) + 16) as usize * 4 + (quart.x() & 3) as usize) * 4)
        + (quart.z() & 3) as usize;
    (owner, index)
}

const PRODUCTION_GATE_COUNT: usize = 32;

/// Fixed key stripes coalesce misses without retaining in-flight terrain or keys.
/// A collision can wait, but generation never holds a cache mutex. SOURCE gates
/// may call RAW/BIOME gates; RAW and BIOME producers never call SOURCE or each
/// other. A thread holds at most one gate of each kind, so this order is acyclic.
/// Poisoning refuses new production; successful immutable cache hits stay usable.
pub(super) struct ProductionGates([Mutex<()>; PRODUCTION_GATE_COUNT]);
impl ProductionGates {
    /// Creates 32 bounded stripes without retained keys or producer results.
    pub(super) fn new() -> Self {
        Self(std::array::from_fn(|_| Mutex::new(())))
    }
    fn stripe(pos: ChunkPos) -> usize {
        let mut key =
            (u64::from(pos.x().cast_unsigned()) << 32) | u64::from(pos.z().cast_unsigned());
        key = (key ^ (key >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        key = (key ^ (key >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        key ^= key >> 31;
        usize::from(key.to_le_bytes()[0] & 31)
    }
    /// Owns production for this key stripe, refusing a poisoned producer.
    pub(super) fn lock(&self, pos: ChunkPos) -> Result<std::sync::MutexGuard<'_, ()>, WorldError> {
        self.0[Self::stripe(pos)]
            .lock()
            .map_err(|_| error("natural producer gate poisoned"))
    }
}

#[derive(Default)]
struct Cache {
    raw: VecDeque<(ChunkPos, Arc<Chunk>)>,
    biomes: VecDeque<(ChunkPos, Arc<[BiomeResourceId]>)>,
}
pub(crate) struct GrassPreview {
    catalog: Catalog,
    cache: Mutex<Cache>,
    raw_gates: ProductionGates,
    biome_gates: ProductionGates,
    #[cfg(test)]
    raw_computations: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    biome_computations: std::sync::atomic::AtomicUsize,
}
impl GrassPreview {
    pub(crate) fn new() -> Result<Self, WorldError> {
        Ok(Self {
            catalog: Catalog::compile()?,
            cache: Mutex::new(Cache::default()),
            raw_gates: ProductionGates::new(),
            biome_gates: ProductionGates::new(),
            #[cfg(test)]
            raw_computations: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            biome_computations: std::sync::atomic::AtomicUsize::new(0),
        })
    }
    pub(crate) fn raw(
        &self,
        generator: &NaturalWorldGenerator,
        pos: ChunkPos,
    ) -> Result<Arc<Chunk>, WorldError> {
        {
            let cache = self
                .cache
                .lock()
                .map_err(|_| error("raw baseline cache poisoned"))?;
            if let Some(chunk) = cache
                .raw
                .iter()
                .find(|(key, _)| *key == pos)
                .map(|(_, chunk)| Arc::clone(chunk))
            {
                return Ok(chunk);
            }
        }
        let _production = self.raw_gates.lock(pos)?;
        // A waiter must observe the owner's publication before rebuilding. This
        // lookup owns only the short cache lock, not the numerical computation.
        {
            let cache = self
                .cache
                .lock()
                .map_err(|_| error("raw baseline cache poisoned"))?;
            if let Some((_, chunk)) = cache.raw.iter().find(|(key, _)| *key == pos) {
                return Ok(Arc::clone(chunk));
            }
        }
        #[cfg(test)]
        self.raw_computations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let chunk = Arc::new(generator.generate_raw_chunk(pos)?);
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| error("raw baseline cache poisoned"))?;
        // Preserve the defensive one-slot publication check and FIFO policy.
        // Same-key producers are gated; no cache lock covers numeric work.
        if let Some(existing) = cache
            .raw
            .iter()
            .find(|(key, _)| *key == pos)
            .map(|(_, chunk)| Arc::clone(chunk))
        {
            return Ok(existing);
        }
        if cache.raw.len() == 32 {
            cache.raw.pop_front();
        }
        cache.raw.push_back((pos, Arc::clone(&chunk)));
        Ok(chunk)
    }
    pub(crate) fn biome(
        &self,
        generator: &NaturalWorldGenerator,
        zoom: BiomeZoom,
        pos: BlockPos,
    ) -> Result<BiomeResourceId, WorldError> {
        let (chunk, index) = biome_location(zoom, pos);
        {
            let cache = self
                .cache
                .lock()
                .map_err(|_| error("raw baseline cache poisoned"))?;
            if let Some(raw) = cache
                .raw
                .iter()
                .find(|(key, _)| *key == chunk)
                .map(|(_, raw)| Arc::clone(raw))
            {
                return biome_at(&raw, index);
            }
            if let Some(array) = cache
                .biomes
                .iter()
                .find(|(key, _)| *key == chunk)
                .map(|(_, array)| Arc::clone(array))
            {
                return array
                    .get(index)
                    .copied()
                    .ok_or_else(|| error("biome projection invalid"));
            }
        }
        let _production = self.biome_gates.lock(chunk)?;
        {
            let cache = self
                .cache
                .lock()
                .map_err(|_| error("raw baseline cache poisoned"))?;
            if let Some((_, raw)) = cache.raw.iter().find(|(key, _)| *key == chunk) {
                return biome_at(raw, index);
            }
            if let Some((_, array)) = cache.biomes.iter().find(|(key, _)| *key == chunk) {
                return array
                    .get(index)
                    .copied()
                    .ok_or_else(|| error("biome projection invalid"));
            }
        }
        #[cfg(test)]
        self.biome_computations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let array: Arc<[BiomeResourceId]> = generator.sample_center_biomes(chunk)?.into();
        let result = array
            .get(index)
            .copied()
            .ok_or_else(|| error("biome projection invalid"))?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| error("raw baseline cache poisoned"))?;
        if let Some(existing) = cache
            .biomes
            .iter()
            .find(|(key, _)| *key == chunk)
            .map(|(_, array)| Arc::clone(array))
        {
            return existing
                .get(index)
                .copied()
                .ok_or_else(|| error("biome projection invalid"));
        }
        if cache.biomes.len() == 64 {
            cache.biomes.pop_front();
        }
        cache.biomes.push_back((chunk, array));
        Ok(result)
    }
    #[cfg(test)]
    pub(crate) fn cache_is_empty_for_test(&self) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.raw.is_empty() && cache.biomes.is_empty()
    }
    #[cfg(test)]
    pub(crate) fn clear_raw_caches(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.raw.clear();
        cache.biomes.clear();
    }
    pub(crate) fn apply_source(
        &self,
        generator: &NaturalWorldGenerator,
        seed: WorldSeed,
        source: ChunkPos,
        region: &mut impl Region,
    ) -> Result<(), WorldError> {
        place(
            &self.catalog,
            seed,
            source,
            [generator.grass_plain_count(source)?, 2],
            region,
        )
    }
    pub(crate) const fn grass_state(&self) -> BlockStateId {
        self.catalog.grass
    }
    pub(crate) fn generate(
        &self,
        generator: &NaturalWorldGenerator,
        seed: WorldSeed,
        target: ChunkPos,
    ) -> Result<Chunk, WorldError> {
        let baseline = self.raw(generator, target)?;
        let mut output = baseline.as_ref().clone();
        let zoom = BiomeZoom::new(
            BiomeZoomSeed::from_world_seed(seed),
            BiomeZoomSemantics::Paper1_21_8Build60,
        );
        for dx in -1..=1 {
            for dz in -1..=1 {
                let source = ChunkPos::new(
                    target
                        .x()
                        .checked_add(dx)
                        .ok_or_else(|| error("grass source coordinate overflow"))?,
                    target
                        .z()
                        .checked_add(dz)
                        .ok_or_else(|| error("grass source coordinate overflow"))?,
                );
                let raw = self.raw(generator, source)?;
                let heightmap = raw.heightmap(HeightmapKind::WorldSurface);
                let mut heights = [-64; 256];
                for x in 0..16 {
                    for z in 0..16 {
                        heights[x * 16 + z] = heightmap
                            .height(x as u8, z as u8)
                            .ok_or_else(|| error("raw height missing"))?
                            + 1;
                    }
                }
                let count = generator.grass_plain_count(source)?;
                let mut region = Projected {
                    preview: self,
                    generator,
                    source,
                    target,
                    raw: &raw,
                    baseline: &baseline,
                    zoom,
                    heights,
                    writes: Vec::with_capacity(384),
                };
                place(&self.catalog, seed, source, [count, 2], &mut region)?;
                for pos in region.writes {
                    if owning(pos) == target {
                        output.set_block(pos, self.catalog.grass)?;
                    }
                }
            }
        }
        Ok(output)
    }
}
fn owning(pos: BlockPos) -> ChunkPos {
    ChunkPos::new(pos.x() >> 4, pos.z() >> 4)
}
struct Projected<'a> {
    preview: &'a GrassPreview,
    generator: &'a NaturalWorldGenerator,
    source: ChunkPos,
    target: ChunkPos,
    raw: &'a Chunk,
    baseline: &'a Chunk,
    zoom: BiomeZoom,
    heights: [i32; 256],
    writes: Vec<BlockPos>,
}
impl Region for Projected<'_> {
    fn height(&mut self, x: i32, z: i32) -> Result<i32, WorldError> {
        Ok(self.heights[(x & 15) as usize * 16 + (z & 15) as usize])
    }
    fn biome(&mut self, pos: BlockPos) -> Result<BiomeResourceId, WorldError> {
        self.preview.biome(self.generator, self.zoom, pos)
    }
    fn relevant(&self, pos: BlockPos) -> bool {
        owning(pos) == self.source || owning(pos) == self.target
    }
    fn state(&mut self, pos: BlockPos) -> Result<BlockStateId, WorldError> {
        if !(-64..320).contains(&pos.y()) {
            return resolve_state("void_air");
        }
        if self.writes.contains(&pos) {
            return Ok(self.preview.catalog.grass);
        }
        let chunk = if owning(pos) == self.source {
            self.raw
        } else {
            self.baseline
        };
        chunk
            .get_block(pos)
            .ok_or_else(|| error("retained grass read outside raw baseline"))
    }
    fn write(&mut self, pos: BlockPos) -> Result<(), WorldError> {
        if self.writes.len() == 384 {
            return Err(error("grass write projection cap exceeded"));
        }
        self.writes.push(pos);
        if owning(pos) == self.source {
            let h = &mut self.heights[(pos.x() & 15) as usize * 16 + (pos.z() & 15) as usize];
            *h = (*h).max(pos.y() + 1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_gates_serialize_same_key_and_allow_distinct_stripes() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let gates = ProductionGates::new();
        let key = ChunkPos::ORIGIN;
        let active = AtomicUsize::new(0);
        let barrier = std::sync::Barrier::new(4);
        std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        for _ in 0..16 {
                            let _guard = gates.lock(key).unwrap();
                            assert_eq!(active.fetch_add(1, Ordering::SeqCst), 0);
                            std::thread::yield_now();
                            assert_eq!(active.fetch_sub(1, Ordering::SeqCst), 1);
                        }
                    })
                })
                .collect();
            for job in jobs {
                job.join().unwrap();
            }
        });
        let other = (1..128)
            .map(|x| ChunkPos::new(x, 0))
            .find(|pos| ProductionGates::stripe(*pos) != ProductionGates::stripe(key))
            .unwrap();
        let entered = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                let _guard = gates.lock(key).unwrap();
                entered.wait();
            });
            let second = scope.spawn(|| {
                let _guard = gates.lock(other).unwrap();
                entered.wait();
            });
            first.join().unwrap();
            second.join().unwrap();
        });
        assert_eq!(gates.0.len(), 32);
        println!(
            "PRODUCTION_GATES_NATIVE one={} three={}",
            std::mem::size_of::<ProductionGates>(),
            3 * std::mem::size_of::<ProductionGates>()
        );
        for pos in [ChunkPos::new(i32::MIN, i32::MAX), ChunkPos::new(-1, -1)] {
            assert!(ProductionGates::stripe(pos) < 32);
            drop(gates.lock(pos).unwrap());
        }
    }

    #[test]
    fn producer_errors_release_gate_and_poisoning_refuses_new_production() {
        let gates = ProductionGates::new();
        let key = ChunkPos::ORIGIN;
        let failed = || -> Result<(), WorldError> {
            let _guard = gates.lock(key)?;
            Err(error("injected numerical refusal"))
        };
        assert!(failed().is_err());
        drop(gates.lock(key).unwrap());
        assert!(std::panic::catch_unwind(|| {
            let _guard = gates.lock(key).unwrap();
            panic!("injected producer panic");
        })
        .is_err());
        let refusal = gates.lock(key).unwrap_err();
        assert!(matches!(refusal, WorldError::NaturalGeneration {
            stage: NaturalGenerationStage::Features, detail,
        } if detail == "natural producer gate poisoned"));
    }

    #[test]
    fn refused_raw_production_retries_without_a_retained_gate_or_cache_entry() {
        use std::sync::atomic::Ordering;
        let generator = NaturalWorldGenerator::with_tree_preview(WorldSeed::new(0)).unwrap();
        let grass = generator.grass_preview().unwrap();
        let invalid = ChunkPos::new(i32::MAX, 0);
        for _ in 0..2 {
            assert!(grass.raw(&generator, invalid).is_err());
        }
        assert_eq!(grass.raw_computations.load(Ordering::Relaxed), 2);
        assert!(grass.cache.lock().unwrap().raw.is_empty());
    }

    #[test]
    fn poisoned_production_stripe_keeps_completed_raw_hits_but_refuses_new_keys() {
        use std::sync::atomic::Ordering;
        let generator = NaturalWorldGenerator::with_tree_preview(WorldSeed::new(0)).unwrap();
        let grass = generator.grass_preview().unwrap();
        let key = ChunkPos::ORIGIN;
        let original = grass.raw(&generator, key).unwrap();
        assert!(std::panic::catch_unwind(|| {
            let _guard = grass.raw_gates.lock(key).unwrap();
            panic!("injected producer panic");
        })
        .is_err());
        assert!(Arc::ptr_eq(&original, &grass.raw(&generator, key).unwrap()));
        let collision = (1..512)
            .map(|x| ChunkPos::new(x, 0))
            .find(|pos| ProductionGates::stripe(*pos) == ProductionGates::stripe(key))
            .unwrap();
        let refused = grass.raw(&generator, collision).unwrap_err();
        assert!(matches!(refused, WorldError::NaturalGeneration {
            stage: NaturalGenerationStage::Features, detail,
        } if detail == "natural producer gate poisoned"));
        assert_eq!(grass.raw_computations.load(Ordering::Relaxed), 1);
        assert_eq!(grass.cache.lock().unwrap().raw.len(), 1);
    }

    #[test]
    fn concurrent_raw_and_biome_publications_keep_one_slot_per_key() {
        let generator = NaturalWorldGenerator::with_tree_preview(WorldSeed::new(0)).unwrap();
        let barrier = std::sync::Barrier::new(4);
        let values = std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        generator
                            .grass_preview()
                            .unwrap()
                            .raw(&generator, ChunkPos::ORIGIN)
                            .unwrap()
                    })
                })
                .collect();
            jobs.into_iter()
                .map(|job| job.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            generator
                .grass_preview()
                .unwrap()
                .raw_computations
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        for value in &values {
            assert!(Arc::ptr_eq(value, &values[0]));
            assert!(!value.persist_dirty_sections().any());
            assert!(!value.persist_edited_sections().any());
        }
        assert_eq!(
            generator
                .grass_preview()
                .unwrap()
                .cache
                .lock()
                .unwrap()
                .raw
                .len(),
            1
        );
        let zoom = BiomeZoom::new(
            BiomeZoomSeed::from_world_seed(WorldSeed::new(0)),
            BiomeZoomSemantics::Paper1_21_8Build60,
        );
        let pos = BlockPos::new(-2000, 64, 2000);
        let (owner, _) = biome_location(zoom, pos);
        assert_ne!(owner, ChunkPos::ORIGIN);
        let biomes = std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        generator
                            .grass_preview()
                            .unwrap()
                            .biome(&generator, zoom, pos)
                            .unwrap()
                    })
                })
                .collect();
            jobs.into_iter()
                .map(|job| job.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(biomes.iter().all(|biome| *biome == biomes[0]));
        assert_eq!(
            generator
                .grass_preview()
                .unwrap()
                .biome_computations
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        let cache = generator.grass_preview().unwrap().cache.lock().unwrap();
        assert_eq!(cache.biomes.len(), 1);
        assert_eq!(cache.biomes.front().unwrap().0, owner);
    }

    struct Probe {
        support: BlockStateId,
        slope: bool,
        biome: BiomeResourceId,
        source: ChunkPos,
        target: Option<ChunkPos>,
        writes: Vec<BlockPos>,
        grass: BlockStateId,
    }
    impl Region for Probe {
        fn height(&mut self, x: i32, z: i32) -> Result<i32, WorldError> {
            let mut height = 65 + if self.slope { (x + z).rem_euclid(5) } else { 0 };
            for pos in &self.writes {
                if pos.x() == x && pos.z() == z {
                    height = height.max(pos.y() + 1);
                }
            }
            Ok(height)
        }
        fn biome(&mut self, _: BlockPos) -> Result<BiomeResourceId, WorldError> {
            Ok(self.biome)
        }
        fn relevant(&self, pos: BlockPos) -> bool {
            self.target.is_none() || owning(pos) == self.source || Some(owning(pos)) == self.target
        }
        fn state(&mut self, pos: BlockPos) -> Result<BlockStateId, WorldError> {
            if self.writes.contains(&pos) {
                return Ok(self.grass);
            }
            let ground = 64
                + if self.slope {
                    (pos.x() + pos.z()).rem_euclid(5)
                } else {
                    0
                };
            Ok(if pos.y() <= ground {
                self.support
            } else {
                BlockStateId::AIR
            })
        }
        fn write(&mut self, pos: BlockPos) -> Result<(), WorldError> {
            self.writes.push(pos);
            Ok(())
        }
    }
    #[test]
    fn original_placed_grass_and_rng_match_for_all_signed_seeds_supports_and_slopes() {
        let catalog = Catalog::compile().unwrap();
        let temperatures = crate::biome_temperature::BuiltinBiomeTemperatures::compile(
            crate::biome_temperature::BiomeTemperatureBuildBudget::new(1_048_576).unwrap(),
        )
        .unwrap();
        for line in include_str!("../tests/fixtures/grass-original-1.21.8/original.tsv")
            .lines()
            .skip(1)
        {
            let row: Vec<_> = line.split('\t').collect();
            let feature = usize::from(row[1] == "patch_grass_forest");
            let seed = WorldSeed::new(row[2].parse().unwrap());
            let source = ChunkPos::new(row[3].parse().unwrap(), row[4].parse().unwrap());
            let biome = BiomeResourceId::from_name(if row[5].contains('@') {
                "desert"
            } else if feature == 0 {
                "plains"
            } else {
                "forest"
            })
            .unwrap();
            let count: usize = row[7].parse().unwrap();
            if feature == 0 {
                let noise = temperatures
                    .info_noise(
                        f64::from(source.x() * 16) / 200.0,
                        f64::from(source.z() * 16) / 200.0,
                    )
                    .unwrap();
                assert_eq!(count, if noise < -0.8 { 5 } else { 10 }, "{} noise", row[0]);
            }
            let mut random = Random::new(seed);
            random
                .feature(seed, source, catalog.indices[feature])
                .unwrap();
            for draw in row[10].split(';').filter(|v| !v.is_empty()) {
                let (bound, value) = draw.split_once(':').unwrap();
                assert_eq!(
                    random.int(bound.parse().unwrap()).unwrap(),
                    value.parse::<i32>().unwrap(),
                    "{} RNG",
                    row[0]
                );
            }
            let expected: Vec<_> = row[9]
                .split(';')
                .filter(|v| !v.is_empty())
                .map(|v| {
                    let a: Vec<i32> = v.split(',').map(|v| v.parse().unwrap()).collect();
                    BlockPos::new(a[0], a[1], a[2])
                })
                .collect();
            let mut counts = [0; 2];
            counts[feature] = count;
            let mut full = Probe {
                support: resolve_state(row[5].split('@').next().unwrap()).unwrap(),
                slope: row[6] == "true",
                biome,
                source,
                target: None,
                writes: Vec::new(),
                grass: catalog.grass,
            };
            place(&catalog, seed, source, counts, &mut full).unwrap();
            assert_eq!(full.writes, expected, "{} full original writes", row[0]);
            assert_eq!(
                !full.writes.is_empty(),
                row[8] == "true",
                "{} original result",
                row[0]
            );
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let target = ChunkPos::new(source.x() + dx, source.z() + dz);
                    let mut projected = Probe {
                        target: Some(target),
                        writes: Vec::new(),
                        support: full.support,
                        slope: full.slope,
                        biome,
                        source,
                        grass: catalog.grass,
                    };
                    place(&catalog, seed, source, counts, &mut projected).unwrap();
                    let filtered: Vec<_> = expected
                        .iter()
                        .copied()
                        .filter(|pos| owning(*pos) == source || owning(*pos) == target)
                        .collect();
                    assert_eq!(
                        projected.writes, filtered,
                        "{} exact source/target projection {target:?}",
                        row[0]
                    );
                }
            }
        }
    }
    #[test]
    fn rng_refusal_changes_no_core_and_no_work() {
        let mut random = Random::new(WorldSeed::new(0));
        random.remaining = 0;
        let before = random.core;
        assert!(random.int(8).is_err());
        assert_eq!(random.core, before);
        assert_eq!(random.remaining, 0);
    }
    #[test]
    fn preview_runtime_is_optional_deterministic_and_has_real_grass() {
        let start = std::time::Instant::now();
        let generator = NaturalWorldGenerator::with_grass_preview(WorldSeed::new(0)).unwrap();
        let origin = generator.try_generate_chunk(ChunkPos::ORIGIN).unwrap();
        let cold = start.elapsed();
        let start = std::time::Instant::now();
        let repeated = generator
            .clone()
            .try_generate_chunk(ChunkPos::ORIGIN)
            .unwrap();
        let warm = start.elapsed();
        assert_eq!(origin, repeated);
        let start = std::time::Instant::now();
        let next = generator.try_generate_chunk(ChunkPos::new(1, 0)).unwrap();
        let adjacent = start.elapsed();
        assert_eq!(
            origin,
            generator.try_generate_chunk(ChunkPos::ORIGIN).unwrap()
        );
        let grass = resolve_state("short_grass").unwrap();
        let count = |c: &Chunk| {
            let mut count = 0;
            for x in 0..16 {
                for z in 0..16 {
                    for y in -64..320 {
                        if c.get_block(BlockPos::new(c.pos().x() * 16 + x, y, c.pos().z() * 16 + z))
                            .unwrap()
                            == grass
                        {
                            count += 1;
                        }
                    }
                }
            }
            count
        };
        println!("grass preview seed0 cold={cold:?} warm={warm:?} adjacent={adjacent:?} origin_grass={} next_grass={}",count(&origin),count(&next));
        assert!(count(&origin) + count(&next) > 0);
        assert!(!origin.persist_dirty_sections().any());
        assert!(!origin.persist_edited_sections().any());
        assert_eq!(
            generator.identity(),
            crate::GenerationIdentity::natural_overworld_with_grass(WorldSeed::new(0))
        );
        assert_ne!(
            generator.identity(),
            crate::GenerationIdentity::natural_overworld(WorldSeed::new(0))
        );
        let cache = generator.grass_preview().unwrap().cache.lock().unwrap();
        assert!(cache.raw.len() <= 32);
        assert!(cache.biomes.len() <= 64);
    }
}
