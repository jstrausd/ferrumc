//! Bounded, deterministic ordinary water ticks from pinned Paper 1.21.8 build 60.
//!
//! A plan reads resident blocks only and publishes no edits. The owner commits
//! its checked writes through the normal journal/persistence/output path, then
//! schedules actual committed water cells. Dynamic/off-grid collisions, other
//! fluid interactions, custom liquid containers, Java events and block drops
//! remain explicit caller responsibilities or typed refusals.
use std::collections::{BTreeMap, BTreeSet};

use ferrumc_math::BlockPos;
use ferrumc_registry::block_state::compute_state_id;
use ferrumc_world::BlockStateId;

const STATES: &[u8] = include_bytes!("fluids/data/states.bin");
const FACES: &[u8] = include_bytes!("fluids/data/faces.bin");
const RECORD: usize = 20;
const STATIC: u16 = 1;
const SOLID: u16 = 2;
const HOLD_ANY: u16 = 4;
const CONTAINER: u16 = 8;
const HOLD_SOURCE: u16 = 16;
const HOLD_FLOW: u16 = 32;
const WATER: u16 = 64;
const SOURCE: u16 = 128;
const FALLING: u16 = 256;
const OTHER_FLUID: u16 = 512;
const FULL_COLLISION: u16 = 1024;
const MAX_READS: usize = 512;
const MAX_WRITES: usize = 6;
const HORIZONTAL: [usize; 4] = [2, 5, 3, 4]; // Paper NORTH,EAST,SOUTH,WEST.
const ENUM_HORIZONTAL: [usize; 4] = [2, 3, 4, 5]; // EnumMap publication order.
const OFFSETS: [(i32, i32, i32); 6] = [
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    (-1, 0, 0),
    (1, 0, 0),
];

/// Normal ordinary-water delay in logical ticks.
pub const WATER_TICK_DELAY: u64 = 5;
/// Hard owner-local scheduled-cell ceiling.
pub const MAX_WATER_TICKS: usize = 65_536;
/// Maximum due cells transferred by one queue drain.
pub const MAX_WATER_DUE_BATCH: usize = 256;

/// Classified refusal; pure planning and failed scheduling publish no edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterError {
    /// Invalid queue or batch limit.
    InvalidLimit,
    /// The complete scheduling admission exceeds the owner cap.
    QueueFull,
    /// Logical tick addition would overflow.
    TickOverflow,
    /// Typed coordinates cannot represent a requested neighbor.
    CoordinateOverflow,
    /// A registry state or frozen record is missing or malformed.
    UnknownState,
    /// Collision geometry is dynamic or outside the exact raster domain.
    UnsupportedCollision,
    /// A relevant predicate encounters lava or another unsupported fluid.
    UnsupportedFluid,
    /// A custom liquid container has no admitted static write operation.
    UnsupportedContainer,
    /// The fixed pure-plan read/write allowance is exhausted.
    PlanLimit,
    /// Explicit legacy water gameplay is once-only.
    AlreadyInitialized,
    /// Native durable ownership cannot share this legacy producer.
    OwnershipRequired,
    /// A copied consumer cannot mutate its queue or world.
    SnapshotOnly,
    /// Ground payload/metadata/region capacity refused the whole destruction drop set.
    DropRefused,
    /// A retained due cell currently has no ticket-resident column.
    ChunkNotLoaded,
}
impl std::fmt::Display for WaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "water operation refused: {self:?}")
    }
}
impl std::error::Error for WaterError {}

/// One original-order block write; the caller validates its before-state at commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterWrite {
    pos: BlockPos,
    before: BlockStateId,
    after: BlockStateId,
}
impl WaterWrite {
    /// Resident target coordinate.
    pub const fn pos(&self) -> BlockPos {
        self.pos
    }
    /// Actual state observed before this staged write.
    pub const fn before(&self) -> BlockStateId {
        self.before
    }
    /// Exact final registry state, including water level/waterlogging.
    pub const fn after(&self) -> BlockStateId {
        self.after
    }
}

/// A private bounded sequence of at most six original-order writes.
#[derive(Debug, Default)]
pub struct WaterPlan {
    writes: Vec<WaterWrite>,
}
impl WaterPlan {
    /// Read the complete sequence before atomically admitting the owner commit.
    pub fn writes(&self) -> &[WaterWrite] {
        &self.writes
    }
}

/// Earliest-due scheduling with one live index entry per cell and explicit refusal.
#[derive(Debug, Clone)]
pub struct WaterTickQueue {
    cap: usize,
    by_pos: BTreeMap<BlockPos, u64>,
    by_due: BTreeSet<(u64, BlockPos)>,
}
impl WaterTickQueue {
    /// Create an empty queue with a positive cap no greater than 65536.
    pub fn new(cap: usize) -> Result<Self, WaterError> {
        if !(1..=MAX_WATER_TICKS).contains(&cap) {
            return Err(WaterError::InvalidLimit);
        }
        Ok(Self {
            cap,
            by_pos: BTreeMap::new(),
            by_due: BTreeSet::new(),
        })
    }
    /// Number of unique scheduled cells; rescheduling never creates stale entries.
    pub fn len(&self) -> usize {
        self.by_pos.len()
    }
    /// Whether no cell remains scheduled.
    pub fn is_empty(&self) -> bool {
        self.by_pos.is_empty()
    }
    /// Schedule exactly one cell; retain an earlier due time on duplicate admission.
    pub fn schedule_at(&mut self, due: u64, pos: BlockPos) -> Result<(), WaterError> {
        let old = self.by_pos.get(&pos).copied();
        if old.is_none() && self.by_pos.len() == self.cap {
            return Err(WaterError::QueueFull);
        }
        if old.is_some_and(|old| old <= due) {
            return Ok(());
        }
        if let Some(old) = old {
            self.by_due.remove(&(old, pos));
        }
        self.by_pos.insert(pos, due);
        self.by_due.insert((due, pos));
        Ok(())
    }
    /// Transfer a canonical due prefix; future cells and overflow tails remain owned.
    pub fn pop_due(&mut self, now: u64, limit: usize) -> Result<Vec<BlockPos>, WaterError> {
        if !(1..=MAX_WATER_DUE_BATCH).contains(&limit) {
            return Err(WaterError::InvalidLimit);
        }
        let mut due = Vec::with_capacity(limit.min(self.by_pos.len()));
        while due.len() < limit {
            let Some(&(tick, pos)) = self.by_due.first() else {
                break;
            };
            if tick > now {
                break;
            }
            self.by_due.remove(&(tick, pos));
            self.by_pos.remove(&pos);
            due.push(pos);
        }
        Ok(due)
    }
    /// Atomically admits the water neighborhood of at most six planned changes.
    /// The reader supplies the complete postimage before any world write. Every
    /// unique due cell passes capacity/state/coordinate checks before mutation.
    pub fn schedule_after_changes(
        &mut self,
        rules: &WaterRules,
        now: u64,
        changed: &[BlockPos],
        mut read: impl FnMut(BlockPos) -> Option<BlockStateId>,
    ) -> Result<usize, WaterError> {
        if changed.len() > MAX_WRITES {
            return Err(WaterError::PlanLimit);
        }
        let due = now
            .checked_add(WATER_TICK_DELAY)
            .ok_or(WaterError::TickOverflow)?;
        let mut cells = BTreeSet::new();
        for &changed in changed {
            for direction in 0..7 {
                let pos = if direction == 6 {
                    changed
                } else {
                    offset(changed, direction)?
                };
                if let Some(state) = read(pos) {
                    if rules.is_water(state)? {
                        cells.insert(pos);
                    }
                }
            }
        }
        let added = cells
            .iter()
            .filter(|pos| !self.by_pos.contains_key(pos))
            .count();
        if self.by_pos.len() + added > self.cap {
            return Err(WaterError::QueueFull);
        }
        let count = cells.len();
        for pos in cells {
            self.schedule_at(due, pos)?;
        }
        Ok(count)
    }

    /// Schedule the exact committed cell and its six resident water neighbors.
    ///
    /// This is the placement/removal/neighbor-update hook. The complete seven-cell
    /// admission is preflighted: QueueFull/overflow/unknown state changes nothing.
    /// Missing chunks are never loaded. The owner invokes this at tick boundaries.
    pub fn schedule_after_change(
        &mut self,
        rules: &WaterRules,
        now: u64,
        changed: BlockPos,
        mut read: impl FnMut(BlockPos) -> Option<BlockStateId>,
    ) -> Result<usize, WaterError> {
        let due = now
            .checked_add(WATER_TICK_DELAY)
            .ok_or(WaterError::TickOverflow)?;
        let mut cells = [None; 7];
        let mut added = 0;
        for (i, cell) in cells.iter_mut().enumerate() {
            let pos = if i == 0 {
                changed
            } else {
                offset(changed, i - 1)?
            };
            if let Some(state) = read(pos) {
                if rules.is_water(state)? {
                    *cell = Some(pos);
                    added += usize::from(!self.by_pos.contains_key(&pos));
                }
            }
        }
        if self.by_pos.len() + added > self.cap {
            return Err(WaterError::QueueFull);
        }
        let count = cells.iter().flatten().count();
        for pos in cells.into_iter().flatten() {
            self.schedule_at(due, pos)?;
        }
        Ok(count)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Liquid {
    amount: u8,
    source: bool,
    falling: bool,
}
impl Liquid {
    fn empty(self) -> bool {
        self.amount == 0
    }
}
#[derive(Clone, Copy)]
struct Facts {
    state: BlockStateId,
    flags: u16,
    liquid: Liquid,
    waterlog: Option<BlockStateId>,
    faces: [u16; 6],
}
impl Facts {
    fn has(self, flag: u16) -> bool {
        self.flags & flag != 0
    }
    fn specific(self, incoming: Liquid) -> bool {
        self.has(if incoming.source {
            HOLD_SOURCE
        } else {
            HOLD_FLOW
        })
    }
}

/// Immutable native rules backed by actual pinned-JVM state and collision facts.
#[derive(Debug, Clone)]
pub struct WaterRules {
    levels: [BlockStateId; 16],
}
impl WaterRules {
    /// Bind original water state IDs and validate the frozen scalar domains.
    pub fn builtin() -> Result<Self, WaterError> {
        if STATES.len() != 27_946 * RECORD || !FACES.len().is_multiple_of(32) {
            return Err(WaterError::UnknownState);
        }
        let mut levels = [BlockStateId::AIR; 16];
        for (level, state) in levels.iter_mut().enumerate() {
            let value = level.to_string();
            let properties = [("level", value.as_str())].into_iter().collect();
            *state = BlockStateId::new(
                compute_state_id("minecraft:water", &properties).ok_or(WaterError::UnknownState)?,
            );
        }
        Ok(Self { levels })
    }
    /// Recognize source/flowing/falling and waterlogged cells without guessing IDs.
    pub fn is_water(&self, state: BlockStateId) -> Result<bool, WaterError> {
        Ok(Self::facts(state)?.has(WATER))
    }
    /// Resolves one ordinary water bucket target without mutating the block.
    /// Filling requires exact source water; pouring admits air, short grass,
    /// existing water and the exported static simple waterlogged containers.
    /// Other fluid/custom-container policies remain refused.
    pub fn bucket_after(
        &self,
        state: BlockStateId,
        fill: bool,
    ) -> Result<Option<BlockStateId>, WaterError> {
        let facts = Self::facts(state)?;
        if !facts.has(STATIC) {
            return Err(WaterError::UnsupportedCollision);
        }
        if facts.has(OTHER_FLUID) {
            return Err(WaterError::UnsupportedFluid);
        }
        let name = ferrumc_registry::block_state::state_id_to_block_name(state.as_u32())
            .ok_or(WaterError::UnknownState)?;
        if fill {
            if !facts.has(WATER) || !facts.has(SOURCE) {
                return Ok(None);
            }
            if name == "water" {
                return Ok(Some(BlockStateId::AIR));
            }
            let block = ferrumc_registry::block_state::block_metadata(name)
                .ok_or(WaterError::UnknownState)?;
            if crate::mining::property_value(block, state.as_u32(), "waterlogged") != Some("true") {
                return Ok(None);
            }
            let mut properties = BTreeMap::new();
            for property in block.properties {
                let value = if property.name == "waterlogged" {
                    "false"
                } else {
                    crate::mining::property_value(block, state.as_u32(), property.name)
                        .ok_or(WaterError::UnknownState)?
                };
                properties.insert(property.name, value);
            }
            Ok(Some(BlockStateId::new(
                compute_state_id(name, &properties).ok_or(WaterError::UnknownState)?,
            )))
        } else if state.is_air() || name == "short_grass" || name == "water" {
            Ok(Some(self.levels[0]))
        } else if facts.has(CONTAINER) && facts.has(HOLD_SOURCE) {
            Ok(facts.waterlog)
        } else {
            Ok(None)
        }
    }

    /// Compute one ordinary-water tick against resident blocks, with no mutation.
    ///
    /// Source conversion is the explicit original game rule. Read returns None
    /// for unavailable neighbors, treated closed. At most 512 distinct reads and
    /// six writes are admitted; later reads see earlier staged writes. The owner
    /// must apply all writes through its normal checked commit path and schedule
    /// each changed cell with `schedule_after_change` using committed states.
    pub fn plan_tick(
        &self,
        pos: BlockPos,
        source_conversion: bool,
        read: impl FnMut(BlockPos) -> Option<BlockStateId>,
    ) -> Result<WaterPlan, WaterError> {
        let mut planner = Planner {
            rules: self,
            conversion: source_conversion,
            read,
            cache: BTreeMap::new(),
            holes: BTreeMap::new(),
            plan: WaterPlan::default(),
        };
        planner.tick(pos)?;
        Ok(planner.plan)
    }
    fn facts(state: BlockStateId) -> Result<Facts, WaterError> {
        let index = usize::try_from(state.as_u32()).map_err(|_| WaterError::UnknownState)?;
        let start = index.checked_mul(RECORD).ok_or(WaterError::UnknownState)?;
        let end = start.checked_add(RECORD).ok_or(WaterError::UnknownState)?;
        let bytes = STATES.get(start..end).ok_or(WaterError::UnknownState)?;
        let flags = u16::from_be_bytes([bytes[0], bytes[1]]);
        let waterlog = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let mut faces = [0; 6];
        for (i, face) in faces.iter_mut().enumerate() {
            *face = u16::from_be_bytes([bytes[8 + i * 2], bytes[9 + i * 2]]);
            if usize::from(*face) >= FACES.len() / 32 {
                return Err(WaterError::UnknownState);
            }
        }
        Ok(Facts {
            state,
            flags,
            liquid: Liquid {
                amount: if flags & WATER == 0 { 0 } else { bytes[2] },
                source: flags & SOURCE != 0,
                falling: flags & FALLING != 0,
            },
            waterlog: (waterlog != u32::MAX).then(|| BlockStateId::new(waterlog)),
            faces,
        })
    }
    fn legacy(&self, liquid: Liquid) -> BlockStateId {
        if liquid.empty() {
            BlockStateId::AIR
        } else if liquid.source {
            self.levels[0]
        } else {
            self.levels[usize::from(8 - liquid.amount.min(8) + u8::from(liquid.falling) * 8)]
        }
    }
    fn wall(from: Facts, to: Facts, direction: usize) -> Result<bool, WaterError> {
        if !from.has(STATIC) || !to.has(STATIC) {
            return Err(WaterError::UnsupportedCollision);
        }
        if from.has(FULL_COLLISION) || to.has(FULL_COLLISION) {
            return Ok(false);
        }
        let first = usize::from(from.faces[direction]) * 32;
        let second = usize::from(to.faces[direction ^ 1]) * 32;
        Ok((0..32).any(|i| FACES[first + i] | FACES[second + i] != 255))
    }
}
fn offset(pos: BlockPos, direction: usize) -> Result<BlockPos, WaterError> {
    let (dx, dy, dz) = OFFSETS[direction];
    Ok(BlockPos::new(
        pos.x()
            .checked_add(dx)
            .ok_or(WaterError::CoordinateOverflow)?,
        pos.y()
            .checked_add(dy)
            .ok_or(WaterError::CoordinateOverflow)?,
        pos.z()
            .checked_add(dz)
            .ok_or(WaterError::CoordinateOverflow)?,
    ))
}
struct Planner<'a, R> {
    rules: &'a WaterRules,
    conversion: bool,
    read: R,
    cache: BTreeMap<BlockPos, Option<Facts>>,
    holes: BTreeMap<BlockPos, bool>,
    plan: WaterPlan,
}
impl<R: FnMut(BlockPos) -> Option<BlockStateId>> Planner<'_, R> {
    fn get(&mut self, pos: BlockPos) -> Result<Option<Facts>, WaterError> {
        if let Some(write) = self.plan.writes.iter().rev().find(|write| write.pos == pos) {
            return WaterRules::facts(write.after).map(Some);
        }
        if let Some(state) = self.cache.get(&pos) {
            return Ok(*state);
        }
        if self.cache.len() == MAX_READS {
            return Err(WaterError::PlanLimit);
        }
        let state = (self.read)(pos).map(WaterRules::facts).transpose()?;
        self.cache.insert(pos, state);
        Ok(state)
    }
    fn write(&mut self, pos: BlockPos, before: Facts, liquid: Liquid) -> Result<(), WaterError> {
        let after = if before.has(CONTAINER) {
            if !liquid.source {
                return Err(WaterError::UnsupportedContainer);
            }
            before.waterlog.ok_or(WaterError::UnsupportedContainer)?
        } else {
            self.rules.legacy(liquid)
        };
        if after != before.state {
            if self.plan.writes.len() == MAX_WRITES {
                return Err(WaterError::PlanLimit);
            }
            self.plan.writes.push(WaterWrite {
                pos,
                before: before.state,
                after,
            });
            self.holes.clear();
        }
        Ok(())
    }
    fn new_liquid(&mut self, pos: BlockPos, current: Facts) -> Result<Liquid, WaterError> {
        let mut maximum = 0;
        let mut sources = 0;
        for direction in HORIZONTAL {
            if let Some(neighbor) = self.get(offset(pos, direction)?)? {
                if neighbor.has(WATER) && WaterRules::wall(current, neighbor, direction)? {
                    sources += u8::from(neighbor.liquid.source);
                    maximum = maximum.max(neighbor.liquid.amount);
                }
            }
        }
        if sources >= 2 && self.conversion {
            if let Some(below) = self.get(offset(pos, 0)?)? {
                if below.has(SOLID) || below.liquid.source {
                    return Ok(Liquid {
                        amount: 8,
                        source: true,
                        falling: false,
                    });
                }
            }
        }
        if let Some(above) = self.get(offset(pos, 1)?)? {
            if above.has(WATER) && WaterRules::wall(current, above, 1)? {
                return Ok(Liquid {
                    amount: 8,
                    source: false,
                    falling: true,
                });
            }
        }
        Ok(Liquid {
            amount: maximum.saturating_sub(1),
            source: false,
            falling: false,
        })
    }
    fn maybe(from: Facts, to: Facts, direction: usize) -> Result<bool, WaterError> {
        if to.liquid.source || !to.has(HOLD_ANY) {
            return Ok(false);
        }
        if to.has(OTHER_FLUID) {
            return Err(WaterError::UnsupportedFluid);
        }
        WaterRules::wall(from, to, direction)
    }
    fn hole(&mut self, pos: BlockPos, state: Facts) -> Result<bool, WaterError> {
        if let Some(hole) = self.holes.get(&pos) {
            return Ok(*hole);
        }
        let result = if let Some(below) = self.get(offset(pos, 0)?)? {
            let held = below.has(WATER) || below.has(HOLD_ANY) && below.has(HOLD_FLOW);
            if held && below.has(OTHER_FLUID) {
                return Err(WaterError::UnsupportedFluid);
            }
            held && WaterRules::wall(state, below, 0)?
        } else {
            false
        };
        self.holes.insert(pos, result);
        Ok(result)
    }
    fn slope(
        &mut self,
        pos: BlockPos,
        state: Facts,
        depth: u8,
        blocked: usize,
    ) -> Result<u16, WaterError> {
        let mut best = 1000;
        for direction in HORIZONTAL {
            if direction == blocked {
                continue;
            }
            let next = offset(pos, direction)?;
            if let Some(target) = self.get(next)? {
                if Self::maybe(state, target, direction)? && target.has(HOLD_FLOW) {
                    if self.hole(next, target)? {
                        return Ok(u16::from(depth));
                    }
                    if depth < 4 {
                        best = best.min(self.slope(next, target, depth + 1, direction ^ 1)?);
                    }
                }
            }
        }
        Ok(best)
    }
    fn sides(&mut self, pos: BlockPos, current: Facts, liquid: Liquid) -> Result<(), WaterError> {
        if !liquid.falling && liquid.amount <= 1 {
            return Ok(());
        }
        let mut best = 1000;
        let mut targets = [None; 6];
        for direction in HORIZONTAL {
            let next = offset(pos, direction)?;
            if let Some(target) = self.get(next)? {
                if Self::maybe(current, target, direction)? {
                    let new = self.new_liquid(next, target)?;
                    if target.specific(new) {
                        let distance = if self.hole(next, target)? {
                            0
                        } else {
                            self.slope(next, target, 1, direction ^ 1)?
                        };
                        if distance < best {
                            targets.fill(None);
                        }
                        if distance <= best {
                            if !target.has(WATER) {
                                targets[direction] = Some((next, target, new));
                            }
                            best = distance;
                        }
                    }
                }
            }
        }
        for direction in ENUM_HORIZONTAL {
            if let Some((next, target, new)) = targets[direction] {
                self.write(next, target, new)?;
            }
        }
        Ok(())
    }
    fn tick(&mut self, pos: BlockPos) -> Result<(), WaterError> {
        let Some(mut current) = self.get(pos)? else {
            return Ok(());
        };
        if !current.has(WATER) {
            return Ok(());
        }
        let mut liquid = current.liquid;
        if !liquid.source {
            let new = self.new_liquid(pos, current)?;
            if new != liquid {
                self.write(pos, current, new)?;
                current = WaterRules::facts(self.rules.legacy(new))?;
                liquid = new;
            }
        }
        if liquid.empty() {
            return Ok(());
        }
        let down = offset(pos, 0)?;
        if let Some(below) = self.get(down)? {
            if Self::maybe(current, below, 0)? {
                let new = self.new_liquid(down, below)?;
                if !below.has(WATER) && below.specific(new) {
                    self.write(down, below, new)?;
                    let mut sources = 0;
                    for direction in HORIZONTAL {
                        if let Some(neighbor) = self.get(offset(pos, direction)?)? {
                            sources += u8::from(neighbor.liquid.source);
                        }
                    }
                    if sources >= 3 {
                        self.sides(pos, current, liquid)?;
                    }
                    return Ok(());
                }
            }
        }
        if liquid.source || !self.hole(pos, current)? {
            self.sides(pos, current, liquid)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn named(name: &str, props: &[(&str, &str)]) -> BlockStateId {
        BlockStateId::new(compute_state_id(name, &props.iter().copied().collect()).unwrap())
    }
    // Share the production resident-read signature with all controlled fixtures.
    #[allow(clippy::unnecessary_wraps)]
    fn ground(cells: &BTreeMap<BlockPos, BlockStateId>, pos: BlockPos) -> Option<BlockStateId> {
        Some(cells.get(&pos).copied().unwrap_or_else(|| {
            if pos.y() < 0 {
                named("stone", &[])
            } else {
                BlockStateId::AIR
            }
        }))
    }
    #[test]
    fn actual_paper_new_liquid_and_slope_spread() {
        let rules = WaterRules::builtin().unwrap();
        let mut cases = 0;
        for line in include_str!("fluids/reference/paper-rules.tsv").lines() {
            let fields: Vec<_> = line.split('\t').collect();
            let kind: usize = fields[0].parse().unwrap();
            let conversion: bool = fields[1].parse().unwrap();
            let level: usize = fields[2].parse().unwrap();
            let mut cells = BTreeMap::new();
            for value in fields[3].split(';').filter(|value| !value.is_empty()) {
                let parts: Vec<i32> = value.split(',').map(|part| part.parse().unwrap()).collect();
                cells.insert(
                    BlockPos::new(parts[0], parts[1], parts[2]),
                    BlockStateId::new(parts[3].cast_unsigned()),
                );
            }
            let origin = BlockPos::new(0, 0, 0);
            let state = WaterRules::facts(rules.levels[level]).unwrap();
            let read = |pos: BlockPos| {
                if kind == 11 && pos.x() < 0 && pos.z() >= 0 {
                    None
                } else {
                    ground(&cells, pos)
                }
            };
            let mut planner = Planner {
                rules: &rules,
                conversion,
                read,
                cache: BTreeMap::new(),
                holes: BTreeMap::new(),
                plan: WaterPlan::default(),
            };
            let actual = planner.new_liquid(origin, state).unwrap();
            assert_eq!(
                rules.legacy(actual).as_u32(),
                fields[4].parse::<u32>().unwrap(),
                "new-liquid {kind}/{conversion}/{level}"
            );
            if level != 7 {
                planner.sides(origin, state, state.liquid).unwrap();
                let actual: Vec<_> = planner
                    .plan
                    .writes()
                    .iter()
                    .map(|write| {
                        let direction = OFFSETS
                            .iter()
                            .position(|&(x, y, z)| write.pos == BlockPos::new(x, y, z))
                            .unwrap();
                        (
                            direction,
                            rules
                                .legacy(WaterRules::facts(write.after).unwrap().liquid)
                                .as_u32(),
                        )
                    })
                    .collect();
                let expected: Vec<_> = fields[5]
                    .split(';')
                    .filter(|value| !value.is_empty())
                    .map(|value| {
                        let (direction, state) = value.split_once(',').unwrap();
                        (
                            direction.parse::<usize>().unwrap(),
                            state.parse::<u32>().unwrap(),
                        )
                    })
                    .collect();
                assert_eq!(actual, expected, "spread {kind}/{conversion}/{level}");
            }
            cases += 1;
        }
        assert_eq!(cases, 144);
    }
    #[test]
    fn queue_coalesces_and_refuses_complete_hook_without_loss() {
        let rules = WaterRules::builtin().unwrap();
        let p = BlockPos::new(0, 0, 0);
        let west = BlockPos::new(-1, 0, 0);
        let east = BlockPos::new(1, 0, 0);
        let mut queue = WaterTickQueue::new(2).unwrap();
        queue.schedule_at(30, p).unwrap();
        for tick in (10..30).rev() {
            queue.schedule_at(tick, p).unwrap();
        }
        queue.schedule_at(100, p).unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.by_due.len(), 1);
        let read = |pos| {
            if pos == west || pos == east {
                Some(rules.levels[0])
            } else {
                Some(BlockStateId::AIR)
            }
        };
        assert_eq!(
            queue.schedule_after_change(&rules, 0, p, read),
            Err(WaterError::QueueFull)
        );
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop_due(9, 2).unwrap().len(), 0);
        assert_eq!(queue.pop_due(10, 2).unwrap(), vec![p]);
        assert_eq!(queue.schedule_after_change(&rules, 10, p, read).unwrap(), 2);
        assert_eq!(queue.pop_due(15, 1).unwrap(), vec![west]);
        assert_eq!(queue.pop_due(15, 1).unwrap(), vec![east]);
        assert!(queue.is_empty());
        assert_eq!(
            queue.schedule_after_change(&rules, u64::MAX, p, read),
            Err(WaterError::TickOverflow)
        );
        assert!(queue.is_empty());
    }
    fn settle(
        rules: &WaterRules,
        queue: &mut WaterTickQueue,
        cells: &mut BTreeMap<BlockPos, BlockStateId>,
        ticks: u64,
    ) {
        for now in 0..ticks {
            for pos in queue.pop_due(now, 256).unwrap() {
                let plan = rules
                    .plan_tick(pos, true, |pos| ground(cells, pos))
                    .unwrap();
                for write in plan.writes() {
                    assert_eq!(ground(cells, write.pos), Some(write.before));
                    cells.insert(write.pos, write.after);
                }
                for write in plan.writes() {
                    queue
                        .schedule_after_change(rules, now, write.pos, |pos| ground(cells, pos))
                        .unwrap();
                }
            }
        }
    }
    #[test]
    fn placed_water_flows_across_border_and_removed_source_decays() {
        let rules = WaterRules::builtin().unwrap();
        let source = BlockPos::new(15, 0, 15);
        let mut cells = BTreeMap::from([(source, rules.levels[0])]);
        let mut queue = WaterTickQueue::new(4096).unwrap();
        queue
            .schedule_after_change(&rules, 0, source, |pos| ground(&cells, pos))
            .unwrap();
        settle(&rules, &mut queue, &mut cells, 500);
        assert!(queue.is_empty());
        assert_eq!(
            ground(&cells, BlockPos::new(22, 0, 15)),
            Some(rules.levels[7])
        );
        assert_eq!(
            ground(&cells, BlockPos::new(23, 0, 15)),
            Some(BlockStateId::AIR)
        );
        assert_eq!(
            ground(&cells, BlockPos::new(16, 0, 16)),
            Some(rules.levels[2])
        );
        cells.insert(source, BlockStateId::AIR);
        queue
            .schedule_after_change(&rules, 0, source, |pos| ground(&cells, pos))
            .unwrap();
        settle(&rules, &mut queue, &mut cells, 500);
        assert!(queue.is_empty());
        assert!(cells.values().all(|state| !rules.is_water(*state).unwrap()));
    }
    #[test]
    fn down_source_merging_static_containers_and_typed_refusals() {
        let rules = WaterRules::builtin().unwrap();
        let p = BlockPos::new(0, 0, 0);
        let source = BlockPos::new(0, 2, 0);
        let cells = BTreeMap::from([(source, rules.levels[0])]);
        let plan = rules
            .plan_tick(source, true, |pos| ground(&cells, pos))
            .unwrap();
        assert_eq!(
            plan.writes(),
            [WaterWrite {
                pos: BlockPos::new(0, 1, 0),
                before: BlockStateId::AIR,
                after: rules.levels[8]
            }]
        );
        let mut cells = BTreeMap::from([
            (p, rules.levels[1]),
            (BlockPos::new(-1, 0, 0), rules.levels[0]),
            (BlockPos::new(1, 0, 0), rules.levels[0]),
        ]);
        let plan = rules.plan_tick(p, true, |pos| ground(&cells, pos)).unwrap();
        assert_eq!(plan.writes()[0].after, rules.levels[0]);
        let plan = rules
            .plan_tick(p, false, |pos| ground(&cells, pos))
            .unwrap();
        assert!(plan.writes().iter().all(|write| write.pos != p));
        cells.insert(BlockPos::new(0, -1, 0), BlockStateId::AIR);
        let plan = rules.plan_tick(p, true, |pos| ground(&cells, pos)).unwrap();
        assert!(plan.writes().iter().all(|write| write.pos != p));
        let dry = named("oak_sign", &[("rotation", "0"), ("waterlogged", "false")]);
        cells.insert(p, dry);
        cells.insert(BlockPos::new(0, -1, 0), named("stone", &[]));
        let plan = rules
            .plan_tick(BlockPos::new(-1, 0, 0), true, |pos| ground(&cells, pos))
            .unwrap();
        assert!(plan.writes().iter().any(|write| write.pos == p
            && write.after == named("oak_sign", &[("rotation", "0"), ("waterlogged", "true")])));
        cells.insert(p, rules.levels[0]);
        cells.insert(BlockPos::new(0, -1, 0), named("lava", &[("level", "0")]));
        assert!(matches!(
            rules.plan_tick(p, true, |pos| ground(&cells, pos)),
            Err(WaterError::UnsupportedFluid)
        ));
        cells.insert(BlockPos::new(0, -1, 0), named("stone", &[]));
        cells.insert(BlockPos::new(0, 0, -1), BlockStateId::new(u32::MAX));
        assert!(matches!(
            rules.plan_tick(p, true, |pos| ground(&cells, pos)),
            Err(WaterError::UnknownState)
        ));
    }
}
