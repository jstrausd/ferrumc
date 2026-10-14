//! Resident-only bucket targeting from actual pinned outline and fluid shapes.
//! The result selects one cell; exclusive runtime owners still check identity,
//! mode, reach policy, revision, fluid admission, events and accepted mutations.

use ferrumc_math::{BlockPos, Direction, Vec3};
use ferrumc_world::BlockStateId;

const CATALOG: &[u8] = include_bytes!("bucket_raycast/data/catalog.bin");
const SPANS: &[u8] = include_bytes!("bucket_raycast/data/shape-spans.bin");
const BOXES: &[u8] = include_bytes!("bucket_raycast/data/boxes.bin");
const SIN: &[u8] = include_bytes!("bucket_raycast/data/sin.bin");
const STATE_BYTES: usize = 20;
const MAX_READS: usize = 64;
const MAX_SHAPE_CHECKS: usize = 8192;
const MAX_BOXES: usize = 16;
const EPSILON: f64 = 1.0e-7;

/// Explicit refusal before a bucket target can reach its exclusive owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BucketRayError {
    /// Eye, look or original0..64 interaction range is not admitted.
    InvalidRay,
    /// A required ray/above/target cell is absent from the resident reader.
    Unavailable(BlockPos),
    /// Invalid state id or an outline requiring absent entity/block-entity facts.
    UnsupportedState(BlockStateId),
    /// The ray would exceed64 reader calls or8192 bounded shape-box checks.
    BudgetExceeded,
    /// Compiled pinned shape data does not satisfy its internal bounded contract.
    InvalidCatalog,
}

impl std::fmt::Display for BucketRayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRay => formatter.write_str("invalid bucket ray"),
            Self::Unavailable(position) => {
                write!(formatter, "unavailable bucket ray cell {position:?}")
            }
            Self::UnsupportedState(state) => write!(
                formatter,
                "unsupported bucket outline state {}",
                state.as_u32()
            ),
            Self::BudgetExceeded => formatter.write_str("bucket ray budget exceeded"),
            Self::InvalidCatalog => formatter.write_str("invalid pinned bucket shape catalog"),
        }
    }
}
impl std::error::Error for BucketRayError {}

type Result<T> = std::result::Result<T, BucketRayError>;

/// Select the actual first `OUTLINE/SOURCE_ONLY` hit cell for an empty bucket, or
/// the clicked water-container cell / hit-face neighbor for a WATER bucket.
/// Caller supplies authoritative eye/look/reach and resident-only state reads.
/// Original view-vector LUT, boundary ties, inside hits and interaction-direction
/// overrides are preserved. Source fluids use actual dry-/same-fluid-above shape
/// variants. Light/scaffolding main-hand contexts and shulker block entities
/// explicitly refuse; this context-free helper cannot infer those facts.
/// Every read, including above-fluid and selected pour target, shares one64-call
/// bound. Shape checks share a separate8192 bound. No hot-path allocation, force
/// load, inventory/vitals copy, fluid/property mutation or event is performed.
/// `fill == false` means water pouring, not lava or another filled bucket type.
pub(crate) fn ray_target(
    eye: Vec3,
    yaw: f32,
    pitch: f32,
    reach: f64,
    fill: bool,
    mut read: impl FnMut(BlockPos) -> Option<BlockStateId>,
) -> Result<Option<BlockPos>> {
    let ray = Ray::new(eye, yaw, pitch, reach)?;
    let mut budget = Budget::new();
    let Some(cell_hit) = trace(ray, fill, &mut read, &mut budget)? else {
        return Ok(None);
    };
    if fill || cell_hit.state.flags & 4 != 0 {
        return Ok(Some(cell_hit.hit.position));
    }
    let target = checked_neighbor(cell_hit.hit.position, cell_hit.hit.face)?;
    // Selection does not force-load or pretend an absent destination is air.
    read_state(target, &mut read, &mut budget)?;
    Ok(Some(target))
}

#[derive(Clone, Copy)]
struct Ray {
    from: Vec3,
    to: Vec3,
    delta: Vec3,
}
impl Ray {
    fn new(eye: Vec3, yaw: f32, pitch: f32, reach: f64) -> Result<Self> {
        if !yaw.is_finite()
            || !pitch.is_finite()
            || !reach.is_finite()
            || !(0.0..=64.0).contains(&reach)
            || !admitted_point(eye)
        {
            return Err(BucketRayError::InvalidRay);
        }
        let view = view_vector(yaw, pitch)?;
        let to = Vec3::new(
            eye.x + view.x * reach,
            eye.y + view.y * reach,
            eye.z + view.z * reach,
        );
        if !admitted_point(to) {
            return Err(BucketRayError::InvalidRay);
        }
        Ok(Self {
            from: eye,
            to,
            delta: Vec3::new(to.x - eye.x, to.y - eye.y, to.z - eye.z),
        })
    }
    fn point(self, fraction: f64) -> Vec3 {
        Vec3::new(
            self.from.x + fraction * self.delta.x,
            self.from.y + fraction * self.delta.y,
            self.from.z + fraction * self.delta.z,
        )
    }
}

fn admitted_point(point: Vec3) -> bool {
    [point.x, point.y, point.z].into_iter().all(|value| {
        value.is_finite()
            && value >= f64::from(i32::MIN) + 2.0
            && value <= f64::from(i32::MAX) - 2.0
    })
}

fn view_vector(yaw: f32, pitch: f32) -> Result<Vec3> {
    let radians = (std::f64::consts::PI / 180.0) as f32;
    let x_rotation = pitch * radians;
    let y_rotation = -yaw * radians;
    let cos_y = trig(y_rotation, true)?;
    let sin_y = trig(y_rotation, false)?;
    let cos_x = trig(x_rotation, true)?;
    let sin_x = trig(x_rotation, false)?;
    Ok(Vec3::new(
        f64::from(sin_y * cos_x),
        f64::from(-sin_x),
        f64::from(cos_y * cos_x),
    ))
}

fn trig(value: f32, cosine: bool) -> Result<f32> {
    let scaled = value * 10430.378_f32;
    let scaled = if cosine { scaled + 16384.0 } else { scaled };
    // Float-to-int saturation and the mask match Java casts, including huge
    // finite rotations whose scaled value overflows to infinity.
    let index = ((scaled as i32) & 65535) as usize;
    Ok(f32::from_bits(read_u32(SIN, index * 4)?))
}

struct Budget {
    reads: usize,
    checks: usize,
}
impl Budget {
    const fn new() -> Self {
        Self {
            reads: MAX_READS,
            checks: MAX_SHAPE_CHECKS,
        }
    }
    fn read(&mut self) -> Result<()> {
        self.reads = self
            .reads
            .checked_sub(1)
            .ok_or(BucketRayError::BudgetExceeded)?;
        Ok(())
    }
    fn check(&mut self) -> Result<()> {
        self.checks = self
            .checks
            .checked_sub(1)
            .ok_or(BucketRayError::BudgetExceeded)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct State {
    id: BlockStateId,
    outline: u16,
    interaction: u16,
    fluid_low: u16,
    fluid_high: u16,
    flags: u8,
    fluid_kind: u8,
    offset_kind: u8,
    horizontal: f64,
    vertical: f64,
}

fn state_record(id: BlockStateId) -> Result<State> {
    if id.as_u32() >= read_u32(CATALOG, 0)? {
        return Err(BucketRayError::UnsupportedState(id));
    }
    let start = (id.as_u32() as usize)
        .checked_mul(STATE_BYTES)
        .and_then(|value| value.checked_add(4))
        .ok_or(BucketRayError::InvalidCatalog)?;
    let row = CATALOG
        .get(start..start + STATE_BYTES)
        .ok_or(BucketRayError::InvalidCatalog)?;
    Ok(State {
        id,
        outline: read_u16(row, 0)?,
        interaction: read_u16(row, 2)?,
        fluid_low: read_u16(row, 4)?,
        fluid_high: read_u16(row, 6)?,
        flags: row[8],
        fluid_kind: row[9],
        offset_kind: row[10],
        horizontal: f64::from(f32::from_bits(read_u32(row, 12)?)),
        vertical: f64::from(f32::from_bits(read_u32(row, 16)?)),
    })
}

fn read_state(
    position: BlockPos,
    read: &mut impl FnMut(BlockPos) -> Option<BlockStateId>,
    budget: &mut Budget,
) -> Result<State> {
    budget.read()?;
    state_record(read(position).ok_or(BucketRayError::Unavailable(position))?)
}

fn checked_neighbor(position: BlockPos, face: Direction) -> Result<BlockPos> {
    let offset = face.offset();
    Ok(BlockPos::new(
        position
            .x()
            .checked_add(offset.x())
            .ok_or(BucketRayError::InvalidRay)?,
        position
            .y()
            .checked_add(offset.y())
            .ok_or(BucketRayError::InvalidRay)?,
        position
            .z()
            .checked_add(offset.z())
            .ok_or(BucketRayError::InvalidRay)?,
    ))
}

#[derive(Debug, Clone, Copy)]
struct Hit {
    position: BlockPos,
    face: Direction,
    location: Vec3,
    #[cfg(test)]
    inside: bool,
}
#[derive(Clone, Copy)]
struct CellHit {
    hit: Hit,
    state: State,
}

fn trace(
    ray: Ray,
    fill: bool,
    read: &mut impl FnMut(BlockPos) -> Option<BlockStateId>,
    budget: &mut Budget,
) -> Result<Option<CellHit>> {
    if ray.from == ray.to {
        return Ok(None);
    }
    let from = components(ray.from);
    let to = components(ray.to);
    // Original Mth.lerp(-1e-7, from, to) expands each end, including exact
    // integer boundaries. Strict comparisons below preserve Z/Y/X tie order.
    let start: [f64; 3] =
        std::array::from_fn(|axis| from[axis] + -EPSILON * (to[axis] - from[axis]));
    let end: [f64; 3] = std::array::from_fn(|axis| to[axis] + -EPSILON * (from[axis] - to[axis]));
    let mut cell: [i32; 3] = start.map(|value| value.floor() as i32);
    let delta: [f64; 3] = std::array::from_fn(|axis| end[axis] - start[axis]);
    let sign: [i32; 3] = delta.map(|value| {
        if value == 0.0 {
            0
        } else if value < 0.0 {
            -1
        } else {
            1
        }
    });
    let increment: [f64; 3] = std::array::from_fn(|axis| {
        if sign[axis] == 0 {
            f64::MAX
        } else {
            f64::from(sign[axis]) / delta[axis]
        }
    });
    let mut next: [f64; 3] = std::array::from_fn(|axis| {
        let fraction = start[axis] - start[axis].floor();
        increment[axis]
            * if sign[axis] > 0 {
                1.0 - fraction
            } else {
                fraction
            }
    });
    loop {
        let position = BlockPos::new(cell[0], cell[1], cell[2]);
        let state = read_state(position, read, budget)?;
        if let Some(hit) = clip_cell(ray, position, state, fill, read, budget)? {
            return Ok(Some(CellHit { hit, state }));
        }
        if !next.into_iter().any(|value| value <= 1.0) {
            return Ok(None);
        }
        let axis = if next[0] < next[1] {
            if next[0] < next[2] {
                0
            } else {
                2
            }
        } else if next[1] < next[2] {
            1
        } else {
            2
        };
        cell[axis] = cell[axis]
            .checked_add(sign[axis])
            .ok_or(BucketRayError::InvalidRay)?;
        next[axis] += increment[axis];
    }
}

fn clip_cell(
    ray: Ray,
    position: BlockPos,
    state: State,
    fill: bool,
    read: &mut impl FnMut(BlockPos) -> Option<BlockStateId>,
    budget: &mut Budget,
) -> Result<Option<Hit>> {
    if state.flags & 1 == 0 {
        return Err(BucketRayError::UnsupportedState(state.id));
    }
    if state.flags & 2 != 0 {
        return Ok(None);
    }
    let offset = shape_offset(state, position)?;
    let mut block = clip_shape(
        ray,
        position,
        state.outline,
        if state.flags & 16 != 0 {
            offset
        } else {
            [0.0; 3]
        },
        budget,
    )?;
    if let Some(ref mut hit) = block {
        if let Some(interaction) = clip_shape(
            ray,
            position,
            state.interaction,
            if state.flags & 32 != 0 {
                offset
            } else {
                [0.0; 3]
            },
            budget,
        )? {
            if distance_squared(ray.from, interaction.location)
                < distance_squared(ray.from, hit.location)
            {
                hit.face = interaction.face;
            }
        }
    }
    let fluid = if fill && state.flags & 8 != 0 {
        let above = read_state(checked_neighbor(position, Direction::Up)?, read, budget)?;
        let shape = if above.fluid_kind == state.fluid_kind {
            state.fluid_high
        } else {
            state.fluid_low
        };
        clip_shape(ray, position, shape, [0.0; 3], budget)?
    } else {
        None
    };
    Ok(match (block, fluid) {
        (Some(block), Some(fluid)) => Some(
            if distance_squared(ray.from, block.location)
                <= distance_squared(ray.from, fluid.location)
            {
                block
            } else {
                fluid
            },
        ),
        (Some(hit), None) | (None, Some(hit)) => Some(hit),
        (None, None) => None,
    })
}

fn shape_offset(state: State, position: BlockPos) -> Result<[f64; 3]> {
    if state.offset_kind == 0 {
        return Ok([0.0; 3]);
    }
    if state.offset_kind > 2 {
        return Err(BucketRayError::InvalidCatalog);
    }
    let hash = i64::from(position.x().wrapping_mul(3_129_871))
        ^ i64::from(position.z()).wrapping_mul(116_129_781);
    let seed = hash
        .wrapping_mul(hash)
        .wrapping_mul(42_317_861)
        .wrapping_add(hash.wrapping_mul(11))
        >> 16;
    let horizontal = |bits: i64| {
        ((f64::from((bits as f32) / 15.0) - 0.5) * 0.5).clamp(-state.horizontal, state.horizontal)
    };
    let y = if state.offset_kind == 2 {
        (f64::from(((seed >> 4) & 15) as f32 / 15.0) - 1.0) * state.vertical
    } else {
        0.0
    };
    Ok([horizontal(seed & 15), y, horizontal((seed >> 8) & 15)])
}

#[derive(Clone, Copy)]
struct ShapeSpan {
    start: usize,
    count: usize,
}
fn shape_span(id: u16) -> Result<ShapeSpan> {
    let row = usize::from(id) * 6;
    let start = read_u32(SPANS, row)? as usize;
    let count = usize::from(read_u16(SPANS, row + 4)?);
    if count > MAX_BOXES
        || start
            .checked_add(count)
            .and_then(|end| end.checked_mul(48))
            .is_none_or(|end| end > BOXES.len())
    {
        return Err(BucketRayError::InvalidCatalog);
    }
    Ok(ShapeSpan { start, count })
}

#[derive(Clone, Copy)]
struct Bounds {
    min: [f64; 3],
    max: [f64; 3],
}
fn shape_box(index: usize, offset: [f64; 3]) -> Result<Bounds> {
    let bytes = BOXES
        .get(index * 48..index * 48 + 48)
        .ok_or(BucketRayError::InvalidCatalog)?;
    Ok(Bounds {
        min: [
            read_f64(bytes, 0)? + offset[0],
            read_f64(bytes, 8)? + offset[1],
            read_f64(bytes, 16)? + offset[2],
        ],
        max: [
            read_f64(bytes, 24)? + offset[0],
            read_f64(bytes, 32)? + offset[1],
            read_f64(bytes, 40)? + offset[2],
        ],
    })
}

fn clip_shape(
    ray: Ray,
    position: BlockPos,
    id: u16,
    offset: [f64; 3],
    budget: &mut Budget,
) -> Result<Option<Hit>> {
    let shape = shape_span(id)?;
    if shape.count == 0 || distance_squared(ray.from, ray.to) < EPSILON {
        return Ok(None);
    }
    let behind = ray.point(0.001);
    let cell = [
        f64::from(position.x()),
        f64::from(position.y()),
        f64::from(position.z()),
    ];
    let point = [behind.x - cell[0], behind.y - cell[1], behind.z - cell[2]];
    for index in shape.start..shape.start + shape.count {
        budget.check()?;
        let bounds = shape_box(index, offset)?;
        if (0..3).all(|axis| point[axis] >= bounds.min[axis] && point[axis] < bounds.max[axis]) {
            return Ok(Some(Hit {
                position,
                face: approximate_nearest(ray.delta).opposite(),
                location: behind,
                #[cfg(test)]
                inside: true,
            }));
        }
    }
    let mut closest = 1.0;
    let mut face = None;
    for index in shape.start..shape.start + shape.count {
        budget.check()?;
        let mut bounds = shape_box(index, offset)?;
        for (axis, coordinate) in cell.into_iter().enumerate() {
            bounds.min[axis] += coordinate;
            bounds.max[axis] += coordinate;
        }
        clip_box(ray, bounds, &mut closest, &mut face);
    }
    Ok(face.map(|face| Hit {
        position,
        face,
        location: ray.point(closest),
        #[cfg(test)]
        inside: false,
    }))
}

fn clip_box(ray: Ray, bounds: Bounds, closest: &mut f64, face: &mut Option<Direction>) {
    let from = components(ray.from);
    let delta = components(ray.delta);
    for axis in 0..3 {
        let (plane, candidate) = if delta[axis] > EPSILON {
            (
                bounds.min[axis],
                [Direction::West, Direction::Down, Direction::North][axis],
            )
        } else if delta[axis] < -EPSILON {
            (
                bounds.max[axis],
                [Direction::East, Direction::Up, Direction::South][axis],
            )
        } else {
            continue;
        };
        let distance = (plane - from[axis]) / delta[axis];
        let a = (axis + 1) % 3;
        let b = (axis + 2) % 3;
        let point_a = from[a] + distance * delta[a];
        let point_b = from[b] + distance * delta[b];
        if 0.0 < distance
            && distance < *closest
            && bounds.min[a] - EPSILON < point_a
            && point_a < bounds.max[a] + EPSILON
            && bounds.min[b] - EPSILON < point_b
            && point_b < bounds.max[b] + EPSILON
        {
            *closest = distance;
            *face = Some(candidate);
        }
    }
}

fn approximate_nearest(vector: Vec3) -> Direction {
    let (x, y, z) = (vector.x as f32, vector.y as f32, vector.z as f32);
    let mut direction = Direction::North;
    let mut best = f32::from_bits(1);
    for (face, dot) in [
        (Direction::Down, -y),
        (Direction::Up, y),
        (Direction::North, -z),
        (Direction::South, z),
        (Direction::West, -x),
        (Direction::East, x),
    ] {
        if dot > best {
            best = dot;
            direction = face;
        }
    }
    direction
}

fn components(point: Vec3) -> [f64; 3] {
    [point.x, point.y, point.z]
}
fn distance_squared(from: Vec3, to: Vec3) -> f64 {
    let (x, y, z) = (from.x - to.x, from.y - to.y, from.z - to.z);
    x * x + y * y + z * z
}
fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(BucketRayError::InvalidCatalog)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}
fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(BucketRayError::InvalidCatalog)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}
fn read_f64(bytes: &[u8], offset: usize) -> Result<f64> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(BucketRayError::InvalidCatalog)?;
    Ok(f64::from_le_bytes(
        raw.try_into().map_err(|_| BucketRayError::InvalidCatalog)?,
    ))
}

#[cfg(test)]
mod tests {
    use ferrumc_registry::block_state::block_default_state;

    use super::*;

    #[derive(serde::Deserialize)]
    struct BlockFixture {
        pos: [i32; 3],
        state: u32,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        name: String,
        eye: [f64; 3],
        yaw: f32,
        pitch: f32,
        reach: f64,
        fill: bool,
        blocks: Vec<BlockFixture>,
        view_bits: [String; 3],
        to_bits: [String; 3],
        target: Option<[i32; 3]>,
        hit_pos: Option<[i32; 3]>,
        face: Option<usize>,
        inside: Option<bool>,
        location_bits: Option<[String; 3]>,
    }
    impl Fixture {
        fn eye(&self) -> Vec3 {
            Vec3::new(self.eye[0], self.eye[1], self.eye[2])
        }
        fn read(&self, position: BlockPos) -> BlockStateId {
            self.blocks
                .iter()
                .find(|block| block.pos == coordinates(position))
                .map_or(BlockStateId::AIR, |block| BlockStateId::new(block.state))
        }
    }
    fn named(name: &str) -> BlockStateId {
        BlockStateId::new(block_default_state(name).unwrap())
    }
    fn coordinates(position: BlockPos) -> [i32; 3] {
        [position.x(), position.y(), position.z()]
    }
    fn point_bits(point: Vec3) -> [u64; 3] {
        [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()]
    }
    fn bits(value: &[String; 3]) -> [u64; 3] {
        std::array::from_fn(|index| u64::from_str_radix(&value[index], 16).unwrap())
    }

    #[test]
    fn actual_original_jvm_ray_fixtures_match_target_face_inside_and_bits() {
        let fixtures: Vec<Fixture> =
            serde_json::from_str(include_str!("bucket_raycast/reference/jvm-fixtures.json"))
                .unwrap();
        assert_eq!(fixtures.len(), 37);
        for fixture in fixtures {
            let view = view_vector(fixture.yaw, fixture.pitch).unwrap();
            assert_eq!(
                point_bits(view),
                bits(&fixture.view_bits),
                "view {}",
                fixture.name
            );
            let ray = Ray::new(fixture.eye(), fixture.yaw, fixture.pitch, fixture.reach).unwrap();
            assert_eq!(
                point_bits(ray.to),
                bits(&fixture.to_bits),
                "to {}",
                fixture.name
            );
            let mut budget = Budget::new();
            let hit = trace(
                ray,
                fixture.fill,
                &mut |pos| Some(fixture.read(pos)),
                &mut budget,
            )
            .unwrap();
            assert_eq!(
                hit.map(|value| coordinates(value.hit.position)),
                fixture.hit_pos,
                "hit {}",
                fixture.name
            );
            if let Some(value) = hit {
                assert_eq!(
                    value.hit.face,
                    Direction::ALL[fixture.face.unwrap()],
                    "face {}",
                    fixture.name
                );
                assert_eq!(
                    value.hit.inside,
                    fixture.inside.unwrap(),
                    "inside {}",
                    fixture.name
                );
                assert_eq!(
                    point_bits(value.hit.location),
                    bits(fixture.location_bits.as_ref().unwrap()),
                    "location {}",
                    fixture.name
                );
            }
            let selected = ray_target(
                fixture.eye(),
                fixture.yaw,
                fixture.pitch,
                fixture.reach,
                fixture.fill,
                |pos| Some(fixture.read(pos)),
            )
            .unwrap();
            assert_eq!(
                selected.map(coordinates),
                fixture.target,
                "target {}",
                fixture.name
            );
        }
    }

    #[test]
    fn every_pinned_state_and_shape_satisfies_bounded_catalog_contract() {
        let states = read_u32(CATALOG, 0).unwrap();
        assert_eq!(states, 27946);
        assert_eq!(CATALOG.len(), 4 + states as usize * STATE_BYTES);
        assert_eq!(SPANS.len(), 853 * 6);
        assert_eq!(BOXES.len(), 2376 * 48);
        assert_eq!(SIN.len(), 65536 * 4);
        let mut contextual = 0;
        let mut positional = 0;
        for raw in 0..states {
            let state = state_record(BlockStateId::new(raw)).unwrap();
            assert_eq!(state.flags & !63, 0);
            assert!(state.fluid_kind <= 2);
            assert!(state.offset_kind <= 2);
            assert!(state.horizontal.is_finite() && (0.0..=0.25).contains(&state.horizontal));
            assert!(state.vertical.is_finite() && (0.0..=0.25).contains(&state.vertical));
            assert_eq!(state.flags & 0x30 != 0, state.offset_kind != 0);
            contextual += usize::from(state.flags & 1 == 0);
            positional += usize::from(state.offset_kind != 0);
            for shape in [
                state.outline,
                state.interaction,
                state.fluid_low,
                state.fluid_high,
            ] {
                shape_span(shape).unwrap();
            }
        }
        assert_eq!(positional, 89);
        assert!(contextual > 100);
        for shape in 0..853 {
            let span = shape_span(shape).unwrap();
            assert!(span.count <= 15);
            for index in span.start..span.start + span.count {
                let bounds = shape_box(index, [0.0; 3]).unwrap();
                for axis in 0..3 {
                    assert!(bounds.min[axis].is_finite() && bounds.max[axis].is_finite());
                    assert!(bounds.min[axis] < bounds.max[axis]);
                    assert!(bounds.min[axis] >= -1.0 && bounds.max[axis] <= 2.0);
                }
            }
        }
        for index in 0..65536 {
            let value = f32::from_bits(read_u32(SIN, index * 4).unwrap());
            assert!(value.is_finite() && (-1.0..=1.0).contains(&value));
        }
    }

    #[test]
    fn malformed_rays_refuse_before_any_resident_read() {
        let eye = Vec3::new(0.5, 0.5, 0.5);
        let mut calls = 0;
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 64.0001] {
            assert_eq!(
                ray_target(eye, 0.0, 0.0, bad, true, |_| {
                    calls += 1;
                    Some(BlockStateId::AIR)
                }),
                Err(BucketRayError::InvalidRay)
            );
        }
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for (yaw, pitch) in [(bad, 0.0), (0.0, bad)] {
                assert_eq!(
                    ray_target(eye, yaw, pitch, 4.5, true, |_| {
                        calls += 1;
                        Some(BlockStateId::AIR)
                    }),
                    Err(BucketRayError::InvalidRay)
                );
            }
        }
        for axis in 0..3 {
            for bad in [
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::from(i32::MAX),
                f64::from(i32::MIN),
            ] {
                let mut point = [0.5; 3];
                point[axis] = bad;
                assert_eq!(
                    ray_target(
                        Vec3::new(point[0], point[1], point[2]),
                        0.0,
                        0.0,
                        4.5,
                        true,
                        |_| {
                            calls += 1;
                            Some(BlockStateId::AIR)
                        }
                    ),
                    Err(BucketRayError::InvalidRay)
                );
            }
        }
        assert_eq!(calls, 0);
    }

    #[test]
    fn zero_length_and_original_tiny_shape_rays_do_not_pick() {
        let eye = Vec3::new(0.5, 0.5, 0.5);
        let mut calls = 0;
        assert_eq!(
            ray_target(eye, 0.0, 0.0, 0.0, false, |_| {
                calls += 1;
                Some(named("stone"))
            }),
            Ok(None)
        );
        assert_eq!(calls, 0);
        assert_eq!(
            ray_target(eye, 0.0, 0.0, 0.0001, false, |_| Some(named("stone"))),
            Ok(None)
        );
    }

    #[test]
    fn absent_ray_above_or_pour_destination_refuses_without_force_load() {
        let eye = Vec3::new(0.5, 0.5, 0.5);
        let mut calls = 0;
        assert_eq!(
            ray_target(eye, 0.0, 0.0, 4.5, true, |_| {
                calls += 1;
                None
            }),
            Err(BucketRayError::Unavailable(BlockPos::ORIGIN))
        );
        assert_eq!(calls, 1);
        assert_eq!(
            ray_target(eye, 0.0, 0.0, 4.5, true, |pos| (pos == BlockPos::ORIGIN)
                .then_some(named("water"))),
            Err(BucketRayError::Unavailable(BlockPos::new(0, 1, 0)))
        );
        assert_eq!(
            ray_target(eye, 0.0, 0.0, 4.5, false, |pos| (pos == BlockPos::ORIGIN)
                .then_some(named("stone"))),
            Err(BucketRayError::Unavailable(BlockPos::new(0, 0, -1)))
        );
    }

    #[test]
    fn unknown_ids_and_absent_contextual_outline_facts_refuse() {
        let eye = Vec3::new(0.5, 0.5, 0.5);
        for state in [
            BlockStateId::new(27946),
            BlockStateId::new(u32::MAX),
            named("light"),
            named("scaffolding"),
            named("shulker_box"),
            named("blue_shulker_box"),
        ] {
            assert_eq!(
                ray_target(eye, 0.0, 0.0, 4.5, false, |_| Some(state)),
                Err(BucketRayError::UnsupportedState(state))
            );
        }
    }

    #[test]
    fn exact64_read_budget_and_next_cell_boundary_are_enforced() {
        let eye = Vec3::new(0.5, 0.5, 0.5);
        let mut calls = 0;
        assert_eq!(
            ray_target(eye, 0.0, 0.0, 63.0, false, |_| {
                calls += 1;
                Some(BlockStateId::AIR)
            }),
            Ok(None)
        );
        assert_eq!(calls, 64);
        calls = 0;
        assert_eq!(
            ray_target(eye, 0.0, 0.0, 64.0, false, |_| {
                calls += 1;
                Some(BlockStateId::AIR)
            }),
            Err(BucketRayError::BudgetExceeded)
        );
        assert_eq!(calls, 64);
    }

    #[test]
    fn above_fluid_reads_share_the_same64_call_budget() {
        let mut calls = 0;
        assert_eq!(
            ray_target(Vec3::new(0.5, 0.95, 0.5), 0.0, 0.0, 64.0, true, |pos| {
                calls += 1;
                Some(if pos.y() == 0 {
                    named("water")
                } else {
                    BlockStateId::AIR
                })
            }),
            Err(BucketRayError::BudgetExceeded)
        );
        assert_eq!(calls, 64);
    }

    #[test]
    fn exhausted_shape_budget_refuses_and_invalid_compiled_data_is_bounded() {
        let ray = Ray::new(Vec3::new(0.5, 0.5, 0.5), 0.0, 0.0, 4.5).unwrap();
        let mut budget = Budget {
            reads: MAX_READS,
            checks: 0,
        };
        assert!(matches!(
            trace(ray, false, &mut |_| Some(named("stone")), &mut budget),
            Err(BucketRayError::BudgetExceeded)
        ));
        assert!(matches!(
            shape_span(u16::MAX),
            Err(BucketRayError::InvalidCatalog)
        ));
        assert!(matches!(
            shape_box(2376, [0.0; 3]),
            Err(BucketRayError::InvalidCatalog)
        ));
        assert_eq!(read_u16(&[0], 0), Err(BucketRayError::InvalidCatalog));
        assert_eq!(read_u32(&[0; 3], 0), Err(BucketRayError::InvalidCatalog));
        assert!(matches!(
            read_f64(&[0; 7], 0),
            Err(BucketRayError::InvalidCatalog)
        ));
    }
}
