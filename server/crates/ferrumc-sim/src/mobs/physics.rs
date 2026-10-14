//! Conservative bounded ground motion and exact static shape obstruction.
use super::{
    facts::{Box3, MobFacts, State},
    MobAttributes, MobError, MAX_MOB_BOX_WORK, MAX_MOB_READS,
};
use ferrumc_math::{BlockPos, Vec3};
use ferrumc_world::BlockStateId;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Terrain<'a, F> {
    facts: &'a MobFacts,
    read: &'a mut F,
    cache: BTreeMap<BlockPos, Result<State, MobError>>,
    box_work: usize,
}
impl<'a, F: FnMut(BlockPos) -> Option<BlockStateId>> Terrain<'a, F> {
    pub(super) fn new(facts: &'a MobFacts, read: &'a mut F) -> Self {
        Self {
            facts,
            read,
            cache: BTreeMap::new(),
            box_work: 0,
        }
    }
    pub(super) fn state(&mut self, pos: BlockPos) -> Result<State, MobError> {
        if let Some(value) = self.cache.get(&pos) {
            return *value;
        }
        if self.cache.len() >= MAX_MOB_READS {
            return Err(MobError::WorkLimit);
        }
        let value = (self.read)(pos)
            .ok_or(MobError::IncompleteTerrain)
            .and_then(|state| {
                self.facts
                    .states
                    .get(state.as_u32() as usize)
                    .copied()
                    .ok_or(MobError::UnsupportedState)
            })
            .and_then(|state| {
                if state.flags & 1 == 0 {
                    Err(MobError::UnsupportedState)
                } else {
                    Ok(state)
                }
            });
        self.cache.insert(pos, value);
        value
    }
    fn boxes(&mut self, pos: BlockPos) -> Result<Vec<Box3>, MobError> {
        let state = self.state(pos)?;
        let source = &self.facts.shapes[state.shape];
        self.box_work = self
            .box_work
            .checked_add(source.len())
            .ok_or(MobError::WorkLimit)?;
        if self.box_work > MAX_MOB_BOX_WORK {
            return Err(MobError::WorkLimit);
        }
        let delta = [f64::from(pos.x()), f64::from(pos.y()), f64::from(pos.z())];
        Ok(source
            .iter()
            .map(|b| {
                let mut b = *b;
                for (axis, d) in delta.into_iter().enumerate() {
                    b.min[axis] += d;
                    b.max[axis] += d;
                }
                b
            })
            .collect())
    }
    fn clear_box(&mut self, body: Box3) -> Result<bool, MobError> {
        for pos in padded_cells(body)? {
            let state = self.state(pos)?;
            let cell = Box3 {
                min: [f64::from(pos.x()), f64::from(pos.y()), f64::from(pos.z())],
                max: [
                    f64::from(pos.x() + 1),
                    f64::from(pos.y() + 1),
                    f64::from(pos.z() + 1),
                ],
            };
            if intersects(body, cell) && (state.flags & 2 == 0 || state.flags & 8192 == 0) {
                return Ok(false);
            }
            for shape in self.boxes(pos)? {
                if intersects(body, shape) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    pub(super) fn clear_body(
        &mut self,
        pos: Vec3,
        attributes: MobAttributes,
    ) -> Result<bool, MobError> {
        self.clear_box(body(pos, attributes))
    }
    pub(super) fn supported(
        &mut self,
        pos: Vec3,
        attributes: MobAttributes,
    ) -> Result<bool, MobError> {
        Ok(self.floor_height(pos, attributes, pos.y, pos.y)?.is_some())
    }
    fn floor_height(
        &mut self,
        pos: Vec3,
        attributes: MobAttributes,
        low: f64,
        high: f64,
    ) -> Result<Option<f64>, MobError> {
        let half = attributes.width() / 2.0;
        let probe = Box3 {
            min: [pos.x - half, low - 0.000_001, pos.z - half],
            max: [pos.x + half, high + 0.000_001, pos.z + half],
        };
        let mut highest: Option<f64> = None;
        for cell in padded_cells(probe)? {
            for shape in self.boxes(cell)? {
                let top = shape.max[1];
                if top >= low - 0.000_001
                    && top <= high + 0.000_001
                    && shape.max[0] > probe.min[0] + 0.000_001
                    && shape.min[0] < probe.max[0] - 0.000_001
                    && shape.max[2] > probe.min[2] + 0.000_001
                    && shape.min[2] < probe.max[2] - 0.000_001
                {
                    highest = Some(highest.map_or(top, |value| value.max(top)));
                }
            }
        }
        Ok(highest)
    }
    pub(super) fn grounded_move(
        &mut self,
        before: Vec3,
        proposal: Vec3,
        attributes: MobAttributes,
    ) -> Result<Option<Vec3>, MobError> {
        // Never repair a floating/out-of-collision actor by teleporting it onto ground.
        if !self.supported(before, attributes)? || !self.clear_body(before, attributes)? {
            return Ok(None);
        }
        let Some(height) = self.floor_height(
            proposal,
            attributes,
            before.y - 1.0,
            before.y + attributes.step_height(),
        )?
        else {
            return Ok(None);
        };
        let after = Vec3::new(proposal.x, height, proposal.z);
        let corner = if height > before.y {
            Vec3::new(before.x, height, before.z)
        } else {
            Vec3::new(proposal.x, before.y, proposal.z)
        };
        // Swept bodies conservatively close thin/diagonal obstacles. Separate
        // vertical/horizontal segments permit 0.6-step slabs without tunnelling.
        if !self.clear_box(union(body(before, attributes), body(corner, attributes)))?
            || !self.clear_box(union(body(corner, attributes), body(after, attributes)))?
        {
            return Ok(None);
        }
        Ok(Some(after))
    }
    pub(super) fn line_of_sight(&mut self, from: Vec3, to: Vec3) -> Result<bool, MobError> {
        let delta = to - from;
        let length = delta.length();
        if length > 48.0 {
            return Ok(false);
        }
        let steps = (length * 2.0).ceil().max(1.0) as usize;
        if steps > 96 {
            return Err(MobError::WorkLimit);
        }
        let mut visited = BTreeSet::new();
        for step in 0..=steps {
            let point = from + delta * (step as f64 / steps as f64);
            let cell = BlockPos::new(
                point.x.floor() as i32,
                point.y.floor() as i32,
                point.z.floor() as i32,
            );
            for x in cell.x() - 1..=cell.x() + 1 {
                for y in cell.y() - 1..=cell.y() + 1 {
                    for z in cell.z() - 1..=cell.z() + 1 {
                        let pos = BlockPos::new(x, y, z);
                        if !visited.insert(pos) {
                            continue;
                        }
                        let state = self.state(pos)?;
                        if state.flags & 2 == 0 {
                            return Ok(false);
                        }
                        for shape in self.boxes(pos)? {
                            if segment_intersects(from, to, shape) {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }
        Ok(true)
    }
}
fn body(pos: Vec3, attributes: MobAttributes) -> Box3 {
    let half = attributes.width() / 2.0;
    Box3 {
        min: [pos.x - half, pos.y, pos.z - half],
        max: [pos.x + half, pos.y + attributes.height(), pos.z + half],
    }
}
fn union(a: Box3, b: Box3) -> Box3 {
    let mut out = a;
    for axis in 0..3 {
        out.min[axis] = a.min[axis].min(b.min[axis]);
        out.max[axis] = a.max[axis].max(b.max[axis]);
    }
    out
}
fn intersects(a: Box3, b: Box3) -> bool {
    (0..3).all(|axis| a.max[axis] > b.min[axis] + 1e-9 && a.min[axis] < b.max[axis] - 1e-9)
}
fn padded_cells(bounds: Box3) -> Result<Vec<BlockPos>, MobError> {
    let mut start = [0; 3];
    let mut end = [0; 3];
    let mut count = 1usize;
    for axis in 0..3 {
        start[axis] = bounds.min[axis].floor() as i32 - 1;
        end[axis] = bounds.max[axis].floor() as i32 + 1;
        count = count
            .checked_mul((end[axis] - start[axis] + 1) as usize)
            .ok_or(MobError::WorkLimit)?;
    }
    if count > MAX_MOB_READS {
        return Err(MobError::WorkLimit);
    }
    let mut cells = Vec::with_capacity(count);
    for x in start[0]..=end[0] {
        for y in start[1]..=end[1] {
            for z in start[2]..=end[2] {
                cells.push(BlockPos::new(x, y, z));
            }
        }
    }
    Ok(cells)
}
fn segment_intersects(from: Vec3, to: Vec3, b: Box3) -> bool {
    let origin = [from.x, from.y, from.z];
    let delta = [to.x - from.x, to.y - from.y, to.z - from.z];
    let mut low: f64 = 0.0;
    let mut high: f64 = 1.0;
    for axis in 0..3 {
        if delta[axis].abs() < 1e-12 {
            if origin[axis] <= b.min[axis] || origin[axis] >= b.max[axis] {
                return false;
            }
        } else {
            let t1 = (b.min[axis] - origin[axis]) / delta[axis];
            let t2 = (b.max[axis] - origin[axis]) / delta[axis];
            low = low.max(t1.min(t2));
            high = high.min(t1.max(t2));
            if low >= high {
                return false;
            }
        }
    }
    low < 1.0 && high > 0.0
}
pub(super) fn in_attack_range(
    mob: Vec3,
    attributes: MobAttributes,
    target: Vec3,
    reach: f64,
) -> bool {
    let mut bounds = body(mob, attributes);
    for axis in [0, 2] {
        bounds.min[axis] -= reach;
        bounds.max[axis] += reach;
    }
    let player_half = f64::from(0.6_f32) / 2.0;
    let player = Box3 {
        min: [target.x - player_half, target.y, target.z - player_half],
        max: [
            target.x + player_half,
            target.y + f64::from(1.8_f32),
            target.z + player_half,
        ],
    };
    (0..3).all(|axis| bounds.max[axis] > player.min[axis] && bounds.min[axis] < player.max[axis])
}
