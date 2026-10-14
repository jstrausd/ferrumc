//! Explicit isolated-source oak/birch shape subset, before the accepted grass patches.
use crate::{
    biome_zoom::{BiomeZoom, BiomeZoomSeed, BiomeZoomSemantics},
    grass::{biome_at, biome_location, GrassPreview, ProductionGates, Region},
    tree_kernel::{Facts, Random, TreeKernel, TreeRegion, TREE_REGION_STATES},
    BlockStateId, Chunk, NaturalGenerationStage, NaturalWorldGenerator, WorldError, WorldSeed,
};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::biome::{biome_resources, BiomeResourceId};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};
fn error(detail: impl std::fmt::Display) -> WorldError {
    WorldError::NaturalGeneration {
        stage: NaturalGenerationStage::Features,
        detail: detail.to_string().chars().take(512).collect(),
    }
}
fn owning(p: BlockPos) -> ChunkPos {
    ChunkPos::new(p.x() >> 4, p.z() >> 4)
}
const MAX_WRITES: usize = 4096;
const SOURCE_CACHE: usize = 32;
type Writes = Arc<[(BlockPos, BlockStateId)]>;
pub(crate) struct TreePreview {
    kernels: [TreeKernel; 2],
    members: Vec<(BiomeResourceId, [bool; 3])>,
    sources: Mutex<VecDeque<(ChunkPos, Writes)>>,
    source_gates: ProductionGates,
    #[cfg(test)]
    source_computations: std::sync::atomic::AtomicUsize,
}
impl TreePreview {
    pub(crate) fn new() -> Result<Self, WorldError> {
        for (feature, source) in [
            include_str!("tree_resources/trees_birch.json"),
            include_str!("tree_resources/trees_birch_and_oak_leaf_litter.json"),
            include_str!("tree_resources/trees_plains.json"),
        ]
        .into_iter()
        .enumerate()
        {
            let descriptor: serde_json::Value = serde_json::from_str(source).map_err(error)?;
            let name = [
                "minecraft:trees_birch",
                "minecraft:trees_birch_and_oak_leaf_litter",
                "minecraft:trees_plains",
            ][feature];
            let p = descriptor["placement"]
                .as_array()
                .ok_or_else(|| error("missing tree placement"))?;
            let (base, weight) = if feature == 2 { (0, 19) } else { (10, 9) };
            if descriptor["feature"] != name
                || p.len() != if feature == 1 { 5 } else { 6 }
                || p[0]["type"] != "minecraft:count"
                || p[0]["count"]["type"] != "minecraft:weighted_list"
                || p[0]["count"]["distribution"]
                    != serde_json::json!([{"data":base,"weight":weight},{"data":base+1,"weight":1}])
                || p[1]["type"] != "minecraft:in_square"
                || p[2]["type"] != "minecraft:surface_water_depth_filter"
                || p[2]["max_water_depth"] != 0
                || p[3]["type"] != "minecraft:heightmap"
                || p[3]["heightmap"] != "OCEAN_FLOOR"
                || p[if feature == 2 { 5 } else { 4 }]["type"] != "minecraft:biome"
            {
                return Err(error("unsupported original tree placement"));
            }
            if feature != 1 {
                let guard = &p[if feature == 2 { 4 } else { 5 }];
                if guard["type"] != "minecraft:block_predicate_filter"
                    || guard["predicate"]["type"] != "minecraft:would_survive"
                    || guard["predicate"]["state"]["Name"]
                        != if feature == 0 {
                            "minecraft:birch_sapling"
                        } else {
                            "minecraft:oak_sapling"
                        }
                {
                    return Err(error("unsupported tree sapling placement"));
                }
            }
        }
        let mut members = Vec::with_capacity(65);
        for biome in biome_resources() {
            let def: serde_json::Value =
                serde_json::from_str(biome.definition().json()).map_err(error)?;
            let steps = def["features"]
                .as_array()
                .ok_or_else(|| error("missing tree biome features"))?;
            members.push((
                biome,
                [
                    "minecraft:trees_birch",
                    "minecraft:trees_birch_and_oak_leaf_litter",
                    "minecraft:trees_plains",
                ]
                .map(|name| {
                    steps.iter().any(|step| {
                        step.as_array()
                            .is_some_and(|features| features.iter().any(|f| f == name))
                    })
                }),
            ));
        }
        Ok(Self {
            kernels: [TreeKernel::compile("oak")?, TreeKernel::compile("birch")?],
            members,
            sources: Mutex::new(VecDeque::with_capacity(SOURCE_CACHE)),
            source_gates: ProductionGates::new(),
            #[cfg(test)]
            source_computations: std::sync::atomic::AtomicUsize::new(0),
        })
    }
    fn member(&self, biome: BiomeResourceId, feature: usize) -> bool {
        self.members
            .iter()
            .any(|(id, flags)| *id == biome && flags[feature])
    }
    fn source(
        &self,
        generator: &NaturalWorldGenerator,
        grass: &GrassPreview,
        seed: WorldSeed,
        source: ChunkPos,
    ) -> Result<Writes, WorldError> {
        if let Some(v) = self
            .sources
            .lock()
            .map_err(|_| error("tree source cache poisoned"))?
            .iter()
            .find(|(p, _)| *p == source)
            .map(|(_, v)| Arc::clone(v))
        {
            return Ok(v);
        }
        let _production = self.source_gates.lock(source)?;
        if let Some((_, writes)) = self
            .sources
            .lock()
            .map_err(|_| error("tree source cache poisoned"))?
            .iter()
            .find(|(key, _)| *key == source)
        {
            return Ok(Arc::clone(writes));
        }
        #[cfg(test)]
        self.source_computations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let zoom = BiomeZoom::new(
            BiomeZoomSeed::from_world_seed(seed),
            BiomeZoomSemantics::Paper1_21_8Build60,
        );
        // All attempts retain a complete source-local halo. No target-dependent
        // predicate/read/write pruning can change the following attempt's history.
        let mut region = SourceRegion {
            generator,
            grass,
            source,
            zoom,
            raw: Vec::with_capacity(9),
            writes: BTreeMap::new(),
        };
        self.place(seed, source, &mut region)?;
        grass.apply_source(generator, seed, source, &mut region)?;
        let writes: Writes = region.writes.into_iter().collect::<Vec<_>>().into();
        let mut cache = self
            .sources
            .lock()
            .map_err(|_| error("tree source cache poisoned"))?;
        if let Some((_, existing)) = cache.iter().find(|(p, _)| *p == source) {
            return Ok(Arc::clone(existing));
        }
        if cache.len() == SOURCE_CACHE {
            cache.pop_front();
        }
        cache.push_back((source, Arc::clone(&writes)));
        Ok(writes)
    }
    fn place(
        &self,
        seed: WorldSeed,
        source: ChunkPos,
        region: &mut impl TreeWorld,
    ) -> Result<(), WorldError> {
        let mut random = Random::new(seed);
        // Original global FeatureSorter step 9 ordering: 24,25,52, then grass54/59.
        for (feature, index) in [24, 25, 52].into_iter().enumerate() {
            random.feature(seed, source, index)?;
            let count = if feature == 2 {
                usize::from(random.int(20)? == 19)
            } else {
                10 + usize::from(random.int(10)? == 9)
            };
            for _ in 0..count {
                let x = source.x().wrapping_mul(16).wrapping_add(random.int(16)?);
                let z = source.z().wrapping_mul(16).wrapping_add(random.int(16)?);
                let (surface, ocean) = region.heights(x, z)?;
                if surface - ocean > 0 || ocean <= -64 {
                    continue;
                }
                let origin = BlockPos::new(x, ocean, z);
                if !region.tree_allowed(origin)? {
                    continue;
                }
                let supports = Facts::flag(region.state(BlockPos::new(x, ocean - 1, z))?, 6);
                // Plains has outer sapling before biome; birch's outer sapling is
                // after biome, mixed forest uses the selected nested sapling guard.
                if feature == 2 && !supports {
                    continue;
                }
                if !self.member(region.biome(origin)?, feature) {
                    continue;
                }
                if feature == 0 && !supports {
                    continue;
                }
                let species = match feature {
                    0 => {
                        if random.float()? < 0.0125 {
                            None
                        } else {
                            Some(1)
                        }
                    }
                    1 => {
                        if random.float()? < 0.0025 {
                            None
                        } else if random.float()? < 0.2 {
                            Some(1)
                        } else if random.float()? < 0.1 || random.float()? < 0.0125 {
                            None
                        } else {
                            Some(0)
                        }
                    }
                    _ => {
                        if random.float()? < 0.333_333_34 || random.float()? < 0.0125 {
                            None
                        } else {
                            Some(0)
                        }
                    }
                };
                let Some(species) = species else { continue };
                if feature == 1 && !supports {
                    continue;
                }
                let mut states = Vec::with_capacity(TREE_REGION_STATES);
                for y in -2..10 {
                    for x in -3..4 {
                        for z in -3..4 {
                            states.push(region.state(BlockPos::new(
                                origin.x() + x,
                                origin.y() + y,
                                origin.z() + z,
                            ))?);
                        }
                    }
                }
                let input = TreeRegion::new(origin, &states)?;
                let output = self.kernels[species].execute(&input, &mut random)?;
                if !output.placed() {
                    continue;
                }
                for (i, old) in states.into_iter().enumerate() {
                    let pos = BlockPos::new(
                        origin.x() + i32::from(((i / 7) % 7) as u8) - 3,
                        origin.y() + i32::from((i / 49) as u8) - 2,
                        origin.z() + i32::from((i % 7) as u8) - 3,
                    );
                    let state = output.region().states()[i];
                    if old != state {
                        region.put(pos, state)?;
                    }
                }
            }
        }
        #[cfg(test)]
        region.record(&random.draws);
        Ok(())
    }
    pub(crate) fn generate(
        &self,
        generator: &NaturalWorldGenerator,
        seed: WorldSeed,
        target: ChunkPos,
        grass: &GrassPreview,
    ) -> Result<Chunk, WorldError> {
        let mut output = grass.raw(generator, target)?.as_ref().clone();
        let mut union = BTreeMap::new();
        for dx in -1..=1 {
            for dz in -1..=1 {
                let source = ChunkPos::new(
                    target
                        .x()
                        .checked_add(dx)
                        .ok_or_else(|| error("tree source overflow"))?,
                    target
                        .z()
                        .checked_add(dz)
                        .ok_or_else(|| error("tree source overflow"))?,
                );
                for &(pos, state) in self.source(generator, grass, seed, source)?.iter() {
                    if owning(pos) != target {
                        continue;
                    }
                    // Independent source crowns cannot erase another source's trunk.
                    // Equal-priority conflicts resolve by canonical source-coordinate order.
                    let priority = rank(state, grass.grass_state());
                    let entry = union.entry(pos).or_insert((priority, state));
                    if priority >= entry.0 {
                        *entry = (priority, state);
                    }
                }
            }
        }
        for (pos, (_, state)) in union {
            output.set_block(pos, state)?;
        }
        Ok(output)
    }
}
fn rank(state: BlockStateId, grass: BlockStateId) -> u8 {
    if Facts::flag(state, 3) {
        3
    } else if Facts::flag(state, 4) {
        1
    } else if state == grass {
        0
    } else {
        2
    }
}
trait TreeWorld: Region {
    fn tree_allowed(&self, _origin: BlockPos) -> Result<bool, WorldError> {
        Ok(true)
    }
    #[cfg(test)]
    fn record(&mut self, _draws: &[String]) {}
    fn heights(&mut self, x: i32, z: i32) -> Result<(i32, i32), WorldError>;
    fn put(&mut self, pos: BlockPos, state: BlockStateId) -> Result<(), WorldError>;
}
impl TreeWorld for SourceRegion<'_> {
    fn tree_allowed(&self, origin: BlockPos) -> Result<bool, WorldError> {
        self.generator.tree_allowed(origin)
    }
    fn heights(&mut self, x: i32, z: i32) -> Result<(i32, i32), WorldError> {
        self.source_heights(x, z)
    }
    fn put(&mut self, pos: BlockPos, state: BlockStateId) -> Result<(), WorldError> {
        self.put_state(pos, state)
    }
}
struct SourceRegion<'a> {
    generator: &'a NaturalWorldGenerator,
    grass: &'a GrassPreview,
    source: ChunkPos,
    zoom: BiomeZoom,
    raw: Vec<(ChunkPos, Arc<Chunk>)>,
    writes: BTreeMap<BlockPos, BlockStateId>,
}
impl SourceRegion<'_> {
    fn source_heights(&mut self, x: i32, z: i32) -> Result<(i32, i32), WorldError> {
        let mut surface = -64;
        let mut ocean = -64;
        for y in (-64..320).rev() {
            let state = self.state(BlockPos::new(x, y, z))?;
            if surface == -64 && !Facts::flag(state, 0) {
                surface = y + 1;
            }
            if ocean == -64 && Facts::flag(state, 1) {
                ocean = y + 1;
            }
            if surface > -64 && ocean > -64 {
                break;
            }
        }
        Ok((surface, ocean))
    }
    fn put_state(&mut self, pos: BlockPos, state: BlockStateId) -> Result<(), WorldError> {
        if !(-64..320).contains(&pos.y()) {
            return Err(error("tree write outside vertical geometry"));
        }
        if !self.relevant(pos) {
            return Err(error("tree write outside complete source halo"));
        }
        if !self.writes.contains_key(&pos) && self.writes.len() == MAX_WRITES {
            return Err(error("tree source write cap exceeded"));
        }
        self.writes.insert(pos, state);
        Ok(())
    }
}
impl Region for SourceRegion<'_> {
    fn height(&mut self, x: i32, z: i32) -> Result<i32, WorldError> {
        self.source_heights(x, z).map(|h| h.0)
    }
    fn biome(&mut self, pos: BlockPos) -> Result<BiomeResourceId, WorldError> {
        let (owner, index) = biome_location(self.zoom, pos);
        // A source owns its raw halo for the full attempt history. Global-cache
        // eviction must not cause this already-held biome array to be sampled again.
        if let Some((_, raw)) = self.raw.iter().find(|(key, _)| *key == owner) {
            return biome_at(raw, index);
        }
        self.grass.biome(self.generator, self.zoom, pos)
    }
    fn relevant(&self, pos: BlockPos) -> bool {
        let owner = owning(pos);
        (i64::from(owner.x()) - i64::from(self.source.x())).abs() <= 1
            && (i64::from(owner.z()) - i64::from(self.source.z())).abs() <= 1
    }
    fn state(&mut self, pos: BlockPos) -> Result<BlockStateId, WorldError> {
        if !(-64..320).contains(&pos.y()) {
            return Ok(BlockStateId::AIR);
        }
        if !self.relevant(pos) {
            return Err(error("tree read outside complete source halo"));
        }
        if let Some(state) = self.writes.get(&pos) {
            return Ok(*state);
        }
        let owner = owning(pos);
        if !self.raw.iter().any(|(p, _)| *p == owner) {
            if self.raw.len() == 9 {
                return Err(error("tree source raw halo cap exceeded"));
            }
            self.raw
                .push((owner, self.grass.raw(self.generator, owner)?));
        }
        self.raw
            .iter()
            .find(|(p, _)| *p == owner)
            .and_then(|(_, c)| c.get_block(pos))
            .ok_or_else(|| error("tree raw state absent"))
    }
    fn write(&mut self, pos: BlockPos) -> Result<(), WorldError> {
        self.put_state(pos, self.grass.grass_state())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_sources_coalesce_one_complete_isolated_history() {
        use std::sync::atomic::Ordering;
        let generator = NaturalWorldGenerator::with_tree_preview(WorldSeed::new(0)).unwrap();
        let trees = generator.tree_preview().unwrap();
        let grass = generator.grass_preview().unwrap();
        let barrier = std::sync::Barrier::new(2);
        let values = std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..2)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        trees
                            .source(&generator, grass, WorldSeed::new(0), ChunkPos::ORIGIN)
                            .unwrap()
                    })
                })
                .collect();
            jobs.into_iter()
                .map(|job| job.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(Arc::ptr_eq(&values[0], &values[1]));
        assert_eq!(trees.source_computations.load(Ordering::Relaxed), 1);
        let cache = trees.sources.lock().unwrap();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].0, ChunkPos::ORIGIN);
        assert_eq!(cache[0].1.as_ref(), values[0].as_ref());
    }

    #[test]
    fn refused_source_releases_gate_without_publishing_partial_writes() {
        use std::sync::atomic::Ordering;
        let generator = NaturalWorldGenerator::with_tree_preview(WorldSeed::new(0)).unwrap();
        let trees = generator.tree_preview().unwrap();
        let grass = generator.grass_preview().unwrap();
        let invalid = ChunkPos::new(i32::MAX, 0);
        for _ in 0..2 {
            assert!(trees
                .source(&generator, grass, WorldSeed::new(0), invalid)
                .is_err());
        }
        assert_eq!(trees.source_computations.load(Ordering::Relaxed), 2);
        assert!(trees.sources.lock().unwrap().is_empty());
    }

    #[test]
    fn source_biomes_use_retained_raw_arrays_after_global_eviction() {
        let generator = NaturalWorldGenerator::with_tree_preview(WorldSeed::new(0)).unwrap();
        let grass = generator.grass_preview().unwrap();
        let zoom = BiomeZoom::new(
            BiomeZoomSeed::from_world_seed(WorldSeed::new(0)),
            BiomeZoomSemantics::Paper1_21_8Build60,
        );
        let sentinel = BiomeResourceId::from_name("minecraft:the_void").unwrap();
        let mut region = SourceRegion {
            generator: &generator,
            grass,
            source: ChunkPos::new(-1, -1),
            zoom,
            raw: Vec::with_capacity(9),
            writes: BTreeMap::new(),
        };
        for pos in [
            BlockPos::new(-1, -1000, -1),
            BlockPos::new(-17, 1000, -17),
            BlockPos::new(0, 64, 0),
        ] {
            let (owner, index) = biome_location(zoom, pos);
            let mut raw = Chunk::new(owner);
            raw.install_generated_biomes(vec![sentinel; 1536].into_boxed_slice())
                .unwrap();
            region.raw.clear();
            region.raw.push((owner, Arc::new(raw)));
            grass.clear_raw_caches();
            assert_eq!(
                region.biome(pos).unwrap(),
                sentinel,
                "retained owner {owner:?}, index{index}"
            );
            assert!(grass.cache_is_empty_for_test());
        }
        region.raw.clear();
        let pos = BlockPos::new(-1, 64, -1);
        assert_eq!(
            region.biome(pos).unwrap(),
            grass.biome(&generator, zoom, pos).unwrap()
        );
    }

    struct Controlled {
        floor: BlockStateId,
        slope: bool,
        grass: bool,
        biome: BiomeResourceId,
        writes: BTreeMap<BlockPos, BlockStateId>,
        draws: Vec<String>,
    }
    impl Controlled {
        fn initial(&self, pos: BlockPos) -> BlockStateId {
            let y = 64
                + if self.slope {
                    (pos.x() + pos.z()).rem_euclid(5)
                } else {
                    0
                };
            if pos.y() <= y {
                self.floor
            } else if self.grass && pos.y() == y + 1 {
                BlockStateId::new(
                    ferrumc_registry::block_state::block_metadata("short_grass")
                        .unwrap()
                        .default_state,
                )
            } else {
                BlockStateId::AIR
            }
        }
    }
    impl Region for Controlled {
        fn height(&mut self, x: i32, z: i32) -> Result<i32, WorldError> {
            self.heights(x, z).map(|h| h.0)
        }
        fn biome(&mut self, _pos: BlockPos) -> Result<BiomeResourceId, WorldError> {
            Ok(self.biome)
        }
        fn relevant(&self, _pos: BlockPos) -> bool {
            true
        }
        fn state(&mut self, pos: BlockPos) -> Result<BlockStateId, WorldError> {
            Ok(self
                .writes
                .get(&pos)
                .copied()
                .unwrap_or_else(|| self.initial(pos)))
        }
        fn write(&mut self, _pos: BlockPos) -> Result<(), WorldError> {
            Err(error("unexpected test grass"))
        }
    }
    impl TreeWorld for Controlled {
        fn heights(&mut self, x: i32, z: i32) -> Result<(i32, i32), WorldError> {
            let mut surface = -64;
            let mut ocean = -64;
            for y in (-64..320).rev() {
                let state = self.state(BlockPos::new(x, y, z))?;
                if surface == -64 && !Facts::flag(state, 0) {
                    surface = y + 1;
                }
                if ocean == -64 && Facts::flag(state, 1) {
                    ocean = y + 1;
                }
                if surface > -64 && ocean > -64 {
                    break;
                }
            }
            Ok((surface, ocean))
        }
        fn put(&mut self, pos: BlockPos, state: BlockStateId) -> Result<(), WorldError> {
            self.writes.insert(pos, state);
            Ok(())
        }
        fn record(&mut self, draws: &[String]) {
            self.draws = draws.to_vec();
        }
    }
    #[test]
    fn genuine_original_methods_execute_explicit_subset_and_full_source_history() {
        let preview = TreePreview::new().unwrap();
        let mut cases = 0;
        for line in include_str!("../tests/fixtures/tree-original-1.21.8/original.tsv.placement")
            .lines()
            .skip(1)
        {
            let row: Vec<_> = line.split('\t').collect();
            let seed = WorldSeed::new(row[0].parse().unwrap());
            let source = ChunkPos::new(row[1].parse().unwrap(), row[2].parse().unwrap());
            let floor = if row[4] == "short_grass" {
                "grass_block"
            } else {
                row[4]
            };
            let mut region = Controlled {
                floor: BlockStateId::new(
                    ferrumc_registry::block_state::block_metadata(floor)
                        .unwrap()
                        .default_state,
                ),
                slope: row[5] == "true",
                grass: row[4] == "short_grass",
                biome: BiomeResourceId::from_name(&format!("minecraft:{}", row[3])).unwrap(),
                writes: BTreeMap::new(),
                draws: Vec::new(),
            };
            preview.place(seed, source, &mut region).unwrap();
            assert_eq!(
                region.draws.join(";"),
                row[6],
                "{seed:?} {source:?} {} {} slope{} original RNG",
                row[3],
                row[4],
                row[5]
            );
            let final_states: std::collections::BTreeSet<_> = region
                .writes
                .iter()
                .filter(|(pos, state)| region.initial(**pos) != **state)
                .map(|(pos, state)| {
                    format!("{},{},{}={}", pos.x(), pos.y(), pos.z(), state.as_u32())
                })
                .collect();
            let expected: std::collections::BTreeSet<_> = row[7]
                .split(';')
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect();
            assert_eq!(
                final_states, expected,
                "{seed:?} {source:?} {} {} slope{} original final source",
                row[3], row[4], row[5]
            );
            cases += 1;
        }
        assert_eq!(cases, 576);
    }
    #[test]
    fn real_seed_zero_trees_are_visible_deterministic_and_preserve_generation_masks() {
        let generator = NaturalWorldGenerator::with_tree_preview(WorldSeed::new(0)).unwrap();
        let mut wood = 0;
        let mut leaf = 0;
        for cx in -1..=1 {
            for cz in -1..=1 {
                let chunk = generator.try_generate_chunk(ChunkPos::new(cx, cz)).unwrap();
                for x in 0..16 {
                    for z in 0..16 {
                        for y in 0..160 {
                            let pos = BlockPos::new(cx * 16 + x, y, cz * 16 + z);
                            let state = chunk.get_block(pos).unwrap();
                            if Facts::flag(state, 3) {
                                wood += 1;
                                println!("TREE_WITNESS log {pos:?} state={state:?}");
                            }
                            if Facts::flag(state, 4) {
                                leaf += 1;
                            }
                        }
                    }
                }
                assert_eq!(
                    chunk,
                    generator.try_generate_chunk(ChunkPos::new(cx, cz)).unwrap()
                );
                assert!(!chunk.persist_dirty_sections().any());
                assert!(!chunk.persist_edited_sections().any());
            }
        }
        let mut first = Vec::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                first.push(generator.try_generate_chunk(ChunkPos::new(cx, cz)).unwrap());
            }
        }
        generator
            .tree_preview()
            .unwrap()
            .sources
            .lock()
            .unwrap()
            .clear();
        generator.grass_preview().unwrap().clear_raw_caches();
        for chunk in first.iter().rev() {
            assert_eq!(
                *chunk,
                generator.try_generate_chunk(chunk.pos()).unwrap(),
                "opposite target order after cache eviction"
            );
        }
        println!("TREE_TOTAL wood={wood} leaf={leaf}");
        assert!(wood > 0 && leaf > 0);
        assert_eq!(
            generator.identity(),
            crate::GenerationIdentity::natural_overworld_with_trees(WorldSeed::new(0))
        );
    }
}
