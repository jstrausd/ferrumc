//! Checked original configured-tree kernel on a complete, closed local region.
//!
//! This is independent of biome tree selection and natural generator revisions.
//! Region admission accepts only a closed original terrain/log/leaf state set,
//! including short grass and dry/waterlogged leaves. Scheduled leaf ticks are published as a set; duplicate/order
//! history and subsequent gameplay ticking are outside this kernel contract.
use crate::{
    random::WorldgenRandomSource, BlockStateId, NaturalGenerationStage, WorldError, WorldSeed,
};
use ferrumc_math::BlockPos;
use ferrumc_registry::block_state::{block_metadata, compute_state_id};
use std::collections::BTreeMap;
const WIDTH: usize = 7;
const HEIGHT: usize = 12;
/// Complete volume size; order is relative Y (-2..9), X (-3..3), Z (-3..3).
pub(crate) const TREE_REGION_STATES: usize = WIDTH * WIDTH * HEIGHT;
const DIRECTIONS: [[i32; 3]; 6] = [
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];
fn error(detail: impl std::fmt::Display) -> WorldError {
    WorldError::NaturalGeneration {
        stage: NaturalGenerationStage::Features,
        detail: detail.to_string().chars().take(512).collect(),
    }
}
fn state(name: &str) -> Result<BlockStateId, WorldError> {
    block_metadata(name)
        .map(|m| BlockStateId::new(m.default_state))
        .ok_or_else(|| error("missing tree state"))
}
/// Complete admitted local volume for one original configured-tree attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TreeRegion {
    origin: BlockPos,
    states: Box<[BlockStateId]>,
}
impl TreeRegion {
    /// Admits a complete original terrain/log/leaf volume; unknown/reactive states refuse.
    ///
    /// Genuine JVM state facts preserve replaceability, dirt, leaf persistence and
    /// source-water semantics. Gameplay leaf/fluid/falling-block ticks are not activated.
    pub(crate) fn new(origin: BlockPos, states: &[BlockStateId]) -> Result<Self, WorldError> {
        if states.len() != TREE_REGION_STATES {
            return Err(error("incomplete tree region"));
        }
        if !(-29_999_996..=29_999_996).contains(&origin.x())
            || !(-29_999_996..=29_999_996).contains(&origin.z())
            || !(-64..320).contains(&origin.y())
        {
            return Err(error("tree origin outside admitted geometry"));
        }
        if states.iter().any(|s| !Facts::admits(*s)) {
            return Err(error("unsupported tree region state"));
        }
        Ok(Self {
            origin,
            states: states.to_vec().into_boxed_slice(),
        })
    }
    /// Complete final or input region in the documented local order.
    #[must_use]
    pub(crate) fn states(&self) -> &[BlockStateId] {
        &self.states
    }
    /// Checked block query within the complete local volume.
    #[must_use]
    pub(crate) fn block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.index(pos).and_then(|i| self.states.get(i)).copied()
    }
    fn index(&self, pos: BlockPos) -> Option<usize> {
        let x = i64::from(pos.x()) - i64::from(self.origin.x()) + 3;
        let y = i64::from(pos.y()) - i64::from(self.origin.y()) + 2;
        let z = i64::from(pos.z()) - i64::from(self.origin.z()) + 3;
        if !(0..7).contains(&x) || !(0..12).contains(&y) || !(0..7).contains(&z) {
            return None;
        }
        Some((y as usize * WIDTH + x as usize) * WIDTH + z as usize)
    }
    fn pos(&self, i: usize) -> BlockPos {
        BlockPos::new(
            self.origin.x() + i32::from(((i / WIDTH) % WIDTH) as u8) - 3,
            self.origin.y() + i32::from((i / (WIDTH * WIDTH)) as u8) - 2,
            self.origin.z() + i32::from((i % WIDTH) as u8) - 3,
        )
    }
}
struct Alphabet {
    dirt: BlockStateId,
    log: BlockStateId,
    leaves: [[BlockStateId; 2]; 7],
    base_height: i32,
}
impl Alphabet {
    fn compile(species: &str) -> Result<Self, WorldError> {
        let mut leaves = [[BlockStateId::AIR; 2]; 7];
        for (distance, values) in leaves.iter_mut().enumerate() {
            for (wet, value) in values.iter_mut().enumerate() {
                let d = (distance + 1).to_string();
                let properties = BTreeMap::from([
                    ("distance", d.as_str()),
                    ("persistent", "false"),
                    ("waterlogged", if wet == 0 { "false" } else { "true" }),
                ]);
                *value = BlockStateId::new(
                    compute_state_id(&format!("{species}_leaves"), &properties)
                        .ok_or_else(|| error("missing tree leaf state"))?,
                );
            }
        }
        Ok(Self {
            dirt: state("dirt")?,
            log: state(&format!("{species}_log"))?,
            leaves,
            base_height: if species == "oak" { 4 } else { 5 },
        })
    }
    fn log(s: BlockStateId) -> bool {
        Facts::flag(s, 3)
    }
    fn leaf(s: BlockStateId) -> Option<(usize, bool)> {
        Facts::leaf(s)
    }
    fn replaceable(s: BlockStateId) -> bool {
        Facts::flag(s, 0) || Facts::flag(s, 2)
    }
    fn distance(s: BlockStateId) -> Option<usize> {
        if Self::log(s) {
            Some(0)
        } else {
            Self::leaf(s).map(|p| p.0)
        }
    }
}
/// Original JVM-exported state predicates, bound to the complete pinned registry.
#[derive(Clone, Copy)]
pub(crate) struct Facts;
impl Facts {
    fn bits(s: BlockStateId) -> Option<u16> {
        let i = usize::try_from(s.as_u32()).ok()?.checked_mul(2)?;
        let b = include_bytes!("tree_resources/state_facts.bin").get(i..i + 2)?;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }
    pub(crate) fn flag(s: BlockStateId, bit: u32) -> bool {
        Self::bits(s).is_some_and(|v| v & (1 << bit) != 0)
    }
    fn admits(s: BlockStateId) -> bool {
        Self::flag(s, 14)
    }
    fn leaf(s: BlockStateId) -> Option<(usize, bool)> {
        let b = Self::bits(s)?;
        (b & (1 << 4) != 0).then_some((usize::from((b >> 11) & 7) + 1, b & (1 << 9) != 0))
    }
    fn with_distance(s: BlockStateId, distance: usize) -> Result<BlockStateId, WorldError> {
        let (old, _) = Self::leaf(s).ok_or_else(|| error("nonleaf distance work item"))?;
        let delta = (i32::from(distance as u8) - i32::from(old as u8)) * 4;
        Ok(BlockStateId::new(s.as_u32().wrapping_add_signed(delta)))
    }
}
/// Complete successful/refused original kernel result, without live-world mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeOutput {
    region: TreeRegion,
    placed: bool,
    leaf_ticks: Vec<BlockPos>,
}
impl TreeOutput {
    /// Original ConfiguredFeature.place boolean, including geometric failure.
    #[must_use]
    pub(crate) const fn placed(&self) -> bool {
        self.placed
    }
    /// Complete output region; input remains unchanged on every path.
    #[must_use]
    pub(crate) const fn region(&self) -> &TreeRegion {
        &self.region
    }
    /// Deduplicated leaf block ticks with original delay 1, in local volume order.
    ///
    /// This set does not claim original `HashSet` iteration or duplicate tick calls.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn leaf_ticks(&self) -> &[BlockPos] {
        &self.leaf_ticks
    }
}
/// Immutable original configured `minecraft:tree` kernel binding.
pub(crate) struct TreeKernel {
    alphabet: Alphabet,
}
impl TreeKernel {
    pub(crate) fn compile(species: &str) -> Result<Self, WorldError> {
        let source = match species {
            "oak" => include_str!("tree_resources/oak.json"),
            "birch" => include_str!("tree_resources/birch.json"),
            _ => return Err(error("unsupported tree species")),
        };
        let descriptor: serde_json::Value = serde_json::from_str(source).map_err(error)?;
        let c = &descriptor["config"];
        let base = if species == "oak" { 4 } else { 5 };
        if descriptor["type"] != "minecraft:tree"
            || c["trunk_placer"]["type"] != "minecraft:straight_trunk_placer"
            || c["trunk_placer"]["base_height"] != base
            || c["trunk_placer"]["height_rand_a"] != 2
            || c["trunk_placer"]["height_rand_b"] != 0
            || c["foliage_placer"]["type"] != "minecraft:blob_foliage_placer"
            || c["foliage_placer"]["height"] != 3
            || c["foliage_placer"]["radius"] != 2
            || c["foliage_placer"]["offset"] != 0
            || c["minimum_size"]["limit"] != 1
            || c["minimum_size"]["lower_size"] != 0
            || c["minimum_size"]["upper_size"] != 1
            || c["force_dirt"] != false
            || c["ignore_vines"] != true
            || c["decorators"] != serde_json::json!([])
            || c.get("root_placer").is_some()
            || c["minimum_size"].get("min_clipped_height").is_some()
            || c["trunk_provider"]["state"]["Name"] != format!("minecraft:{species}_log")
            || c["foliage_provider"]["state"]["Name"] != format!("minecraft:{species}_leaves")
        {
            return Err(error("unsupported original tree descriptor"));
        }
        if c["minimum_size"]["type"] != "minecraft:two_layers_feature_size" {
            return Err(error("unsupported original tree size provider"));
        }
        for (field, block, properties) in [
            (
                "trunk_provider",
                format!("minecraft:{species}_log"),
                serde_json::json!({"axis":"y"}),
            ),
            (
                "foliage_provider",
                format!("minecraft:{species}_leaves"),
                serde_json::json!({"distance":"7","persistent":"false","waterlogged":"false"}),
            ),
            (
                "dirt_provider",
                "minecraft:dirt".to_owned(),
                serde_json::Value::Null,
            ),
        ] {
            if c[field]["type"] != "minecraft:simple_state_provider"
                || c[field]["state"]["Name"] != block
                || c[field]["state"]["Properties"] != properties
            {
                return Err(error("unsupported original tree state provider"));
            }
        }
        Ok(Self {
            alphabet: Alphabet::compile(species)?,
        })
    }
    /// Executes the original configured tree kernel with a fresh `WorldgenRandom`.
    ///
    /// This seed initializes the feature RNG directly. Decoration seeding, biome
    /// filters, weighted selectors, fallen trees and bees belong to later routes.
    /// The complete bounded input is cloned; no partial output is returned on error.
    #[cfg(test)]
    pub(crate) fn run(
        &self,
        input: &TreeRegion,
        seed: WorldSeed,
    ) -> Result<TreeOutput, WorldError> {
        self.execute(input, &mut Random::new(seed))
    }
    pub(crate) fn execute(
        &self,
        input: &TreeRegion,
        random: &mut Random,
    ) -> Result<TreeOutput, WorldError> {
        let mut output = TreeOutput {
            region: input.clone(),
            placed: false,
            leaf_ticks: Vec::new(),
        };
        let region = &mut output.region;
        let a = &self.alphabet;
        let origin = region.origin;
        let height = a.base_height + random.int(3)? + random.int(1)?;
        if origin.y() < -63 || origin.y() + height + 1 > 320 {
            return Ok(output);
        }
        for y in 0..=height + 1 {
            let radius = i32::from(y >= 1);
            for x in -radius..=radius {
                for z in -radius..=radius {
                    let pos = offset(origin, [x, y, z]);
                    let s = read(region, pos)?;
                    if !Alphabet::replaceable(s) && !Alphabet::log(s) {
                        return Ok(output);
                    }
                }
            }
        }
        let mut trunk = JavaSet::new();
        let below = offset(origin, [0, -1, 0]);
        if !Facts::flag(read(region, below)?, 5) || Facts::flag(read(region, below)?, 7) {
            write(region, below, a.dirt)?;
            trunk.add(
                region,
                region
                    .index(below)
                    .ok_or_else(|| error("tree trunk index"))?,
            )?;
        }
        for y in 0..height {
            let pos = offset(origin, [0, y, 0]);
            if Alphabet::replaceable(read(region, pos)?) {
                write(region, pos, a.log)?;
                trunk.add(
                    region,
                    region.index(pos).ok_or_else(|| error("tree trunk index"))?,
                )?;
            }
        }
        let mut foliage = [false; TREE_REGION_STATES];
        for y in (-3_i32..=0).rev() {
            let radius = (1 - y / 2).max(0);
            for x in -radius..=radius {
                for z in -radius..=radius {
                    if x.abs() == radius && z.abs() == radius && (random.int(2)? == 0 || y == 0) {
                        continue;
                    }
                    let pos = offset(origin, [x, height + y, z]);
                    let s = read(region, pos)?;
                    if !Alphabet::leaf(s).is_some_and(|(_, persistent)| persistent)
                        && Alphabet::replaceable(s)
                    {
                        write(region, pos, a.leaves[6][usize::from(Facts::flag(s, 10))])?;
                        foliage[region
                            .index(pos)
                            .ok_or_else(|| error("tree foliage index"))?] = true;
                    }
                }
            }
        }
        if trunk.entries.is_empty() && !foliage.iter().any(|v| *v) {
            return Ok(output);
        }
        let mut bounds = [i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN];
        for (i, was_foliage) in foliage.iter().enumerate() {
            if *was_foliage || trunk.entries.contains(&i) {
                let p = region.pos(i);
                bounds[0] = bounds[0].min(p.x());
                bounds[1] = bounds[1].max(p.x());
                bounds[2] = bounds[2].min(p.y());
                bounds[3] = bounds[3].max(p.y());
                bounds[4] = bounds[4].min(p.z());
                bounds[5] = bounds[5].max(p.z());
            }
        }
        let inside = |p: BlockPos| {
            p.x() >= bounds[0]
                && p.x() <= bounds[1]
                && p.y() >= bounds[2]
                && p.y() <= bounds[3]
                && p.z() >= bounds[4]
                && p.z() <= bounds[5]
        };
        let full = update_distances(region, &trunk, &inside)?;
        output.leaf_ticks = shape_ticks(region, &full)?;
        output.placed = true;
        Ok(output)
    }
}
// Exact Java HashMap bucket/chain iteration for this closed BlockPos set.
// Resizing splits buckets preserving insertion order within each chain.
struct JavaSet {
    entries: Vec<usize>,
    capacity: usize,
}
impl JavaSet {
    fn new() -> Self {
        Self {
            entries: Vec::with_capacity(TREE_REGION_STATES),
            capacity: 16,
        }
    }
    fn bucket(&self, region: &TreeRegion, key: usize) -> u32 {
        let p = region.pos(key);
        let h = p
            .y()
            .wrapping_add(p.z().wrapping_mul(31))
            .wrapping_mul(31)
            .wrapping_add(p.x()) as u32;
        (h ^ (h >> 16)) & (self.capacity as u32 - 1)
    }
    fn add(&mut self, region: &TreeRegion, key: usize) -> Result<(), WorldError> {
        if self.entries.contains(&key) {
            return Ok(());
        }
        if self.entries.len() == TREE_REGION_STATES {
            return Err(error("tree Java set cap exhausted"));
        }
        let bucket = self.bucket(region, key);
        let chain = self
            .entries
            .iter()
            .filter(|k| self.bucket(region, **k) == bucket)
            .count();
        // HashMap treeifies on the ninth colliding node. TreeNode order is a
        // separate policy: refuse before insertion, never substitute an order.
        if chain >= 8 && self.capacity >= 64 {
            return Err(error("tree Java tree-bin policy refusal"));
        }
        self.entries.push(key);
        if chain >= 8 || self.entries.len() > self.capacity * 3 / 4 {
            self.capacity *= 2;
        }
        Ok(())
    }
    fn first(&self, region: &TreeRegion) -> Option<usize> {
        self.entries
            .iter()
            .copied()
            .min_by_key(|key| self.bucket(region, *key))
    }
    fn pop(&mut self, region: &TreeRegion) -> Option<usize> {
        let key = self.first(region)?;
        let i = self.entries.iter().position(|k| *k == key)?;
        self.entries.remove(i);
        Some(key)
    }
}
fn update_distances(
    region: &mut TreeRegion,
    trunk: &JavaSet,
    inside: &impl Fn(BlockPos) -> bool,
) -> Result<[bool; TREE_REGION_STATES], WorldError> {
    let mut queues: [JavaSet; 7] = std::array::from_fn(|_| JavaSet::new());
    let mut source = JavaSet {
        entries: trunk.entries.clone(),
        capacity: trunk.capacity,
    };
    while let Some(i) = source.pop(region) {
        queues[0].add(region, i)?;
    }
    let mut full = [false; TREE_REGION_STATES];
    let mut distance = 0;
    // Every original insertion has one of seven fixed distances. The complete
    // bounded worklist admits duplicate positions across separate buckets.
    for _ in 0..TREE_REGION_STATES * 7 {
        while distance < 7 && queues[distance].entries.is_empty() {
            distance += 1;
        }
        if distance == 7 {
            return Ok(full);
        }
        let i = queues[distance]
            .pop(region)
            .ok_or_else(|| error("tree Java iterator inconsistent"))?;
        if distance != 0 {
            region.states[i] = Facts::with_distance(region.states[i], distance)?;
        }
        full[i] = true;
        for direction in DIRECTIONS {
            let next = offset(region.pos(i), direction);
            if !inside(next) {
                continue;
            }
            let j = region
                .index(next)
                .ok_or_else(|| error("tree distance outside region"))?;
            if full[j] {
                continue;
            }
            if let Some(current) = Alphabet::distance(region.states[j]) {
                let value = current.min(distance + 1);
                if value < 7 {
                    queues[value].add(region, j)?;
                    distance = distance.min(value);
                }
            }
        }
    }
    Err(error("tree distance allowance exhausted"))
}
fn shape_ticks(
    region: &mut TreeRegion,
    full: &[bool; TREE_REGION_STATES],
) -> Result<Vec<BlockPos>, WorldError> {
    let mut ticks = [false; TREE_REGION_STATES];
    for i in 0..TREE_REGION_STATES {
        if !full[i] {
            continue;
        }
        for direction in DIRECTIONS {
            let neighbor = offset(region.pos(i), direction);
            let j = region
                .index(neighbor)
                .ok_or_else(|| error("tree shape edge outside region"))?;
            if full[j] {
                continue;
            }
            let inside_pos = region.pos(i);
            let outside_pos = region.pos(j);
            let outside_state = region.states[j];
            stable_edge(region, inside_pos, direction, outside_state)?;
            let inside_state = region.states[i];
            stable_edge(
                region,
                outside_pos,
                [-direction[0], -direction[1], -direction[2]],
                inside_state,
            )?;
            leaf_edge(region.states[i], region.states[j], i, &mut ticks);
            leaf_edge(region.states[j], region.states[i], j, &mut ticks);
        }
    }
    Ok((0..TREE_REGION_STATES)
        .filter(|i| ticks[*i])
        .map(|i| region.pos(i))
        .collect())
}
// The closed input alphabet still includes terrain classes with conditional
// shape changes. Short-grass survival and snowy-soil flags are independent of
// face order. Unsupported snow removal can cascade and therefore refuses the
// complete result instead of inventing an edge iteration order.
fn stable_edge(
    region: &mut TreeRegion,
    pos: BlockPos,
    direction: [i32; 3],
    neighbor: BlockStateId,
) -> Result<(), WorldError> {
    let state = read(region, pos)?;
    let belongs = |name: &str| {
        block_metadata(name).is_some_and(|m| (m.min_state..=m.max_state).contains(&state.as_u32()))
    };
    if belongs("snow") {
        let below = read(region, offset(pos, [0, -1, 0]))?;
        if !Facts::flag(below, 8) {
            return Err(error("snow edge requires unsupported shape mutation"));
        }
    }
    if belongs("short_grass") {
        let below = read(region, offset(pos, [0, -1, 0]))?;
        if !Facts::flag(below, 6) {
            write(region, pos, BlockStateId::AIR)?;
        }
    }
    if direction == [0, 1, 0] {
        for name in ["grass_block", "mycelium", "podzol"] {
            if belongs(name) {
                let m = block_metadata(name).ok_or_else(|| error("missing snowy terrain"))?;
                let snowy = state.as_u32() != m.default_state;
                if snowy != Facts::flag(neighbor, 15) {
                    let raw = if Facts::flag(neighbor, 15) {
                        m.min_state
                    } else {
                        m.default_state
                    };
                    write(region, pos, BlockStateId::new(raw))?;
                }
            }
        }
    }
    Ok(())
}
fn leaf_edge(
    state: BlockStateId,
    neighbor: BlockStateId,
    i: usize,
    ticks: &mut [bool; TREE_REGION_STATES],
) {
    if let Some((distance, _)) = Alphabet::leaf(state) {
        let value = Alphabet::distance(neighbor).unwrap_or(7) + 1;
        if value != 1 || distance != value {
            ticks[i] = true;
        }
    }
}
fn offset(pos: BlockPos, delta: [i32; 3]) -> BlockPos {
    BlockPos::new(pos.x() + delta[0], pos.y() + delta[1], pos.z() + delta[2])
}
fn read(region: &TreeRegion, pos: BlockPos) -> Result<BlockStateId, WorldError> {
    region
        .block(pos)
        .ok_or_else(|| error("tree read outside complete region"))
}
fn write(region: &mut TreeRegion, pos: BlockPos, state: BlockStateId) -> Result<(), WorldError> {
    let i = region
        .index(pos)
        .ok_or_else(|| error("tree write outside complete region"))?;
    region.states[i] = state;
    Ok(())
}
pub(crate) struct Random {
    core: WorldgenRandomSource,
    remaining: u32,
    #[cfg(test)]
    pub(crate) draws: Vec<String>,
}
impl Random {
    pub(crate) fn new(seed: WorldSeed) -> Self {
        Self {
            core: WorldgenRandomSource::xoroshiro(seed),
            remaining: 65_536,
            #[cfg(test)]
            draws: Vec::new(),
        }
    }
    fn bits_count(&mut self, count: u32) -> Result<i32, WorldError> {
        if self.remaining == 0 {
            return Err(error("tree RNG allowance exhausted"));
        }
        self.remaining -= 1;
        Ok(((self.core.next_long() as u64) >> (64 - count)) as i32)
    }
    pub(crate) fn int(&mut self, bound: i32) -> Result<i32, WorldError> {
        if !(1..=20).contains(&bound) {
            return Err(error("unsupported tree random bound"));
        }
        let value = if bound & (bound - 1) == 0 {
            ((i64::from(bound) * i64::from(self.bits_count(31)?)) >> 31) as i32
        } else {
            loop {
                let b = self.bits_count(31)?;
                let v = b % bound;
                if b.wrapping_sub(v).wrapping_add(bound - 1) >= 0 {
                    break v;
                }
            }
        };
        #[cfg(test)]
        self.draws.push(format!("{bound}:{value}"));
        Ok(value)
    }
    pub(crate) fn float(&mut self) -> Result<f32, WorldError> {
        let value = self.bits_count(24)? as f32 / 16_777_216.0;
        #[cfg(test)]
        self.draws.push(format!("F:{}", value.to_bits()));
        Ok(value)
    }
    fn long(&mut self) -> Result<i64, WorldError> {
        let high = i64::from(self.bits_count(32)?);
        let low = i64::from(self.bits_count(32)?);
        Ok(high.wrapping_shl(32).wrapping_add(low))
    }
    pub(crate) fn feature(
        &mut self,
        seed: WorldSeed,
        source: ferrumc_math::ChunkPos,
        index: i64,
    ) -> Result<(), WorldError> {
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
#[cfg(test)]
mod tests {
    use super::*;
    fn initial(origin: BlockPos, name: &str, a: &Alphabet) -> TreeRegion {
        let mut states = vec![BlockStateId::AIR; TREE_REGION_STATES];
        let shape = TreeRegion {
            origin,
            states: states.clone().into_boxed_slice(),
        };
        for (i, s) in states.iter_mut().enumerate() {
            let p = shape.pos(i);
            *s = if p.y() < origin.y() {
                state(match name {
                    "dirt" => "dirt",
                    "stone_floor" => "stone",
                    "coarse_dirt" => "coarse_dirt",
                    "podzol" => "podzol",
                    "sand" => "sand",
                    "farmland" => "farmland",
                    "mycelium" | "snowy_mycelium" => "mycelium",
                    _ => "grass_block",
                })
                .unwrap()
            } else if name == "blocked" && p == offset(origin, [0, 3, 0]) {
                state("stone").unwrap()
            } else if name == "adjacent_log"
                && p.x() == origin.x() + 1
                && p.z() == origin.z()
                && p.y() > origin.y()
                && p.y() <= origin.y() + 6
            {
                a.log
            } else if name == "persistent_leaf" && p == offset(origin, [2, 4, 2]) {
                BlockStateId::new(a.leaves[6][0].as_u32() - 2)
            } else if name == "leaf_distance" && p == offset(origin, [1, 4, 0]) {
                a.leaves[1][0]
            } else if name == "mixed_leaf" && p == offset(origin, [1, 4, 0]) {
                BlockStateId::new(
                    compute_state_id("oak_leaves", &BTreeMap::from([("distance", "2")])).unwrap(),
                )
            } else if name == "floating_grass" && p == offset(origin, [3, 3, 0]) {
                state("short_grass").unwrap()
            } else if name == "mixed_log" && p == offset(origin, [1, 3, 0]) {
                state("oak_log").unwrap()
            } else if ((name == "water_in_trunk"
                || name == "snow_in_trunk"
                || name == "grass_in_trunk")
                && p == origin)
                || (name == "water_in_crown" && p == offset(origin, [1, 4, 0]))
            {
                state(match name {
                    "snow_in_trunk" => "snow",
                    "grass_in_trunk" => "short_grass",
                    _ => "water",
                })
                .unwrap()
            } else {
                BlockStateId::AIR
            };
        }
        if name == "snowy_grass" || name == "snowy_mycelium" {
            let block = if name == "snowy_grass" {
                "grass_block"
            } else {
                "mycelium"
            };
            let snowy = BlockStateId::new(
                compute_state_id(block, &BTreeMap::from([("snowy", "true")])).unwrap(),
            );
            for (i, state) in states.iter_mut().enumerate() {
                if shape.pos(i).y() < origin.y() {
                    *state = snowy;
                }
            }
        }
        TreeRegion::new(origin, &states).unwrap()
    }
    fn position(s: &str) -> BlockPos {
        let a: Vec<i32> = s.split(',').map(|v| v.parse().unwrap()).collect();
        BlockPos::new(a[0], a[1], a[2])
    }
    #[test]
    fn genuine_oak_and_birch_states_draws_boolean_and_leaf_ticks() {
        let mut cases = 0;
        for (species, fixture) in [
            (
                "oak",
                include_str!("../tests/fixtures/tree-original-1.21.8/original.tsv.oak"),
            ),
            (
                "birch",
                include_str!("../tests/fixtures/tree-original-1.21.8/original.tsv.birch"),
            ),
        ] {
            let kernel = TreeKernel::compile(species).unwrap();
            for line in fixture.lines().skip(1) {
                let row: Vec<_> = line.split('\t').collect();
                let seed = WorldSeed::new(row[0].parse().unwrap());
                let origin = BlockPos::new(
                    row[1].parse().unwrap(),
                    row[2].parse().unwrap(),
                    row[3].parse().unwrap(),
                );
                let input = initial(origin, row[4], &kernel.alphabet);
                let mut expected_random = Random::new(seed);
                for draw in row[6].split(';').filter(|v| !v.is_empty()) {
                    let (bound, value) = draw.split_once(':').unwrap();
                    assert_eq!(
                        expected_random.int(bound.parse().unwrap()).unwrap(),
                        value.parse::<i32>().unwrap(),
                        "{species} {seed:?} {} RNG",
                        row[4]
                    );
                }
                let mut random = Random::new(seed);
                let output = kernel.execute(&input, &mut random).unwrap();
                assert_eq!(
                    random.core, expected_random.core,
                    "{species} {seed:?} {} final RNG",
                    row[4]
                );
                assert_eq!(
                    output.placed(),
                    row[5] == "true",
                    "{species} {seed:?} {origin:?} {} boolean",
                    row[4]
                );
                let mut expected = input.clone();
                for change in row[8].split(';').filter(|v| !v.is_empty()) {
                    let (pos, raw) = change.split_once('=').unwrap();
                    write(
                        &mut expected,
                        position(pos),
                        BlockStateId::new(raw.parse().unwrap()),
                    )
                    .unwrap();
                }
                for i in 0..TREE_REGION_STATES {
                    assert_eq!(
                        output.region().states[i],
                        expected.states[i],
                        "{species} {seed:?} {origin:?} {:?} {} state",
                        expected.pos(i),
                        row[4]
                    );
                }
                let ticks: std::collections::BTreeSet<_> = row[9]
                    .split(';')
                    .filter(|v| !v.is_empty())
                    .map(|v| position(v.split_once('=').unwrap().0))
                    .collect();
                assert_eq!(
                    output
                        .leaf_ticks()
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>(),
                    ticks,
                    "{species} {seed:?} {origin:?} {} ticks",
                    row[4]
                );
                cases += 1;
            }
        }
        assert_eq!(cases, 1260);
    }
    #[test]
    fn exact_allstate_census_and_leaf_distance_mapping() {
        for line in include_str!("../tests/fixtures/tree-original-1.21.8/original.tsv.facts")
            .lines()
            .skip(1)
        {
            let row: Vec<_> = line.split('\t').collect();
            let state = BlockStateId::new(row[0].parse().unwrap());
            let metadata = block_metadata(row[1]).unwrap();
            assert!((metadata.min_state..=metadata.max_state).contains(&state.as_u32()));
            assert_eq!(
                row[8], row[13],
                "original oak/birch sapling support equality"
            );
            assert_eq!(
                Facts::flag(state, 7),
                ["minecraft:grass_block", "minecraft:mycelium"].contains(&row[1])
            );
            assert_eq!(Facts::flag(state, 8), row[14] == "true");
            assert_eq!(Facts::flag(state, 15), row[15] == "true");
            for (bit, col) in [
                (0, 2),
                (1, 3),
                (2, 4),
                (3, 5),
                (4, 6),
                (5, 7),
                (6, 8),
                (9, 9),
                (10, 12),
            ] {
                assert_eq!(
                    Facts::flag(state, bit),
                    row[col] == "true",
                    "{} bit{bit}",
                    row[0]
                );
            }
            if row[6] == "true" {
                for (d, value) in row[11].split(',').enumerate() {
                    assert_eq!(
                        Facts::with_distance(state, d + 1).unwrap().as_u32(),
                        value.parse::<u32>().unwrap()
                    );
                }
            }
        }
    }
    #[test]
    fn closed_region_unknown_and_budget_refusals_publish_no_partial() {
        let kernel = TreeKernel::compile("oak").unwrap();
        let input = initial(BlockPos::new(0, 64, 0), "grass", &kernel.alphabet);
        let before = input.clone();
        let mut states = input.states().to_vec();
        states[0] = state("redstone_wire").unwrap();
        assert!(TreeRegion::new(input.origin, &states).is_err());
        states[0] = BlockStateId::new(u32::MAX);
        assert!(TreeRegion::new(input.origin, &states).is_err());
        assert!(TreeRegion::new(input.origin, &states[..TREE_REGION_STATES - 1]).is_err());
        let mut random = Random::new(WorldSeed::new(0));
        random.remaining = 1;
        assert!(kernel.execute(&input, &mut random).is_err());
        assert_eq!(input, before);
        assert!(kernel.run(&input, WorldSeed::new(0)).unwrap().placed());
    }
}

#[cfg(test)]
mod edge_refusals {
    use super::*;
    #[test]
    fn original_floating_snow_shape_witness_refuses_without_publication() {
        let kernel = TreeKernel::compile("oak").unwrap();
        let origin = BlockPos::new(0, 64, 0);
        let mut states = vec![BlockStateId::AIR; TREE_REGION_STATES];
        let geometry = TreeRegion {
            origin,
            states: states.clone().into_boxed_slice(),
        };
        for (i, s) in states.iter_mut().enumerate() {
            if geometry.pos(i).y() < 64 {
                *s = state("grass_block").unwrap();
            }
        }
        let i = geometry.index(BlockPos::new(3, 67, 0)).unwrap();
        states[i] = state("snow").unwrap();
        let input = TreeRegion::new(origin, &states).unwrap();
        let before = input.clone();
        let err = kernel.run(&input, WorldSeed::new(0)).unwrap_err();
        assert!(err.to_string().contains("snow edge"));
        assert_eq!(input, before);
    }
}
