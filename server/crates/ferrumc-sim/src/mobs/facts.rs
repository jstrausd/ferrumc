//! Validated embedded executable facts.
use super::{MobAttributes, MobError, MobKind, MobMetadataAccessor};
use ferrumc_registry::entity_type::EntityTypeId;
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, Copy)]
pub(super) struct State {
    pub(super) flags: u16,
    pub(super) shape: usize,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Box3 {
    pub(super) min: [f64; 3],
    pub(super) max: [f64; 3],
}
#[derive(Debug)]
pub(super) struct MobFacts {
    pub(super) attributes: [MobAttributes; 5],
    pub(super) metadata: [MobMetadataAccessor; 8],
    pub(super) attack_reach: f64,
    pub(super) states: Vec<State>,
    pub(super) shapes: Vec<Vec<Box3>>,
}
static FACTS: OnceLock<Result<Arc<MobFacts>, MobError>> = OnceLock::new();
pub(super) fn builtin() -> Result<Arc<MobFacts>, MobError> {
    FACTS.get_or_init(|| load().map(Arc::new)).clone()
}
fn load() -> Result<MobFacts, MobError> {
    let attributes = load_attributes()?;
    let mut metadata = Vec::with_capacity(8);
    for line in include_str!("data/metadata.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(MobError::InvalidFacts);
        }
        metadata.push(MobMetadataAccessor {
            index: fields[1].parse().map_err(|_| MobError::InvalidFacts)?,
            serializer: fields[2].parse().map_err(|_| MobError::InvalidFacts)?,
        });
    }
    let metadata = metadata.try_into().map_err(|_| MobError::InvalidFacts)?;
    let attack_reach: f64 = include_str!("data/attack-reach.txt")
        .trim()
        .parse()
        .map_err(|_| MobError::InvalidFacts)?;
    if !attack_reach.is_finite() || !(0.8..0.9).contains(&attack_reach) {
        return Err(MobError::InvalidFacts);
    }
    let mut cursor = Cursor {
        bytes: include_bytes!("data/shapes.bin"),
        position: 0,
    };
    let shape_count = usize::from(cursor.short()?);
    if shape_count == 0 || shape_count > 4096 {
        return Err(MobError::InvalidFacts);
    }
    let mut shapes = Vec::with_capacity(shape_count);
    for _ in 0..shape_count {
        let count = usize::from(cursor.short()?);
        // Unadmitted dynamic shapes can have larger box counts; never used.
        if count > 256 {
            return Err(MobError::InvalidFacts);
        }
        let mut boxes = Vec::with_capacity(count);
        for _ in 0..count {
            let mut values = [0.0; 6];
            for value in &mut values {
                *value = cursor.double()?;
                if !value.is_finite() {
                    return Err(MobError::InvalidFacts);
                }
            }
            for axis in 0..3 {
                if values[axis] >= values[axis + 3] {
                    return Err(MobError::InvalidFacts);
                }
            }
            boxes.push(Box3 {
                min: [values[0], values[1], values[2]],
                max: [values[3], values[4], values[5]],
            });
        }
        shapes.push(boxes);
    }
    if cursor.position != cursor.bytes.len() {
        return Err(MobError::InvalidFacts);
    }
    let bytes = include_bytes!("data/states.bin");
    if bytes.len() != 27946 * 4 {
        return Err(MobError::InvalidFacts);
    }
    let mut states = Vec::with_capacity(27946);
    for bytes in bytes.as_chunks::<4>().0 {
        let flags = u16::from_be_bytes([bytes[0], bytes[1]]);
        let shape = usize::from(u16::from_be_bytes([bytes[2], bytes[3]]));
        let boxes = shapes.get(shape).ok_or(MobError::InvalidFacts)?;
        if flags & 1 != 0
            && (boxes.len() > 64
                || boxes.iter().any(|b| {
                    b.min
                        .into_iter()
                        .chain(b.max)
                        .any(|v| !(-1.0..=2.0).contains(&v))
                }))
        {
            return Err(MobError::InvalidFacts);
        }
        states.push(State { flags, shape });
    }
    Ok(MobFacts {
        attributes,
        metadata,
        attack_reach,
        states,
        shapes,
    })
}
fn load_attributes() -> Result<[MobAttributes; 5], MobError> {
    if include_str!("data/attributes.tsv").lines().count() != 5 {
        return Err(MobError::InvalidFacts);
    }
    let mut attributes = Vec::with_capacity(5);
    for (kind, line) in MobKind::ALL
        .into_iter()
        .zip(include_str!("data/attributes.tsv").lines())
    {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 11 || fields[0] != kind.name() {
            return Err(MobError::InvalidFacts);
        }
        let entity_type = EntityTypeId::from_name(kind.name()).ok_or(MobError::InvalidFacts)?;
        if entity_type.protocol_id()
            != fields[1]
                .parse::<i32>()
                .map_err(|_| MobError::InvalidFacts)?
        {
            return Err(MobError::InvalidFacts);
        }
        let mut values = [0.0_f64; 9];
        for (target, source) in values.iter_mut().zip(&fields[2..]) {
            *target = source.parse().map_err(|_| MobError::InvalidFacts)?;
            if !target.is_finite() || *target < 0.0 {
                return Err(MobError::InvalidFacts);
            }
        }
        if values[0] <= 0.0
            || values[0] > 1.0
            || values[1] <= 0.0
            || values[1] > 2.0
            || values[3] <= 0.0
            || values[3] > 20.0
            || values[4] > 0.25
            || values[8] > 0.6
        {
            return Err(MobError::InvalidFacts);
        }
        attributes.push(MobAttributes {
            entity_type,
            width: values[0],
            height: values[1],
            eye_height: values[2],
            max_health: values[3] as f32,
            movement_speed: values[4],
            follow_range: values[5],
            attack_damage: values[6] as f32,
            armor: values[7] as f32,
            step_height: values[8],
        });
    }
    attributes.try_into().map_err(|_| MobError::InvalidFacts)
}
struct Cursor {
    bytes: &'static [u8],
    position: usize,
}
impl Cursor {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], MobError> {
        let end = self.position.checked_add(N).ok_or(MobError::InvalidFacts)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(MobError::InvalidFacts)?
            .try_into()
            .map_err(|_| MobError::InvalidFacts)?;
        self.position = end;
        Ok(value)
    }
    fn short(&mut self) -> Result<u16, MobError> {
        Ok(u16::from_be_bytes(self.take()?))
    }
    fn double(&mut self) -> Result<f64, MobError> {
        Ok(f64::from_be_bytes(self.take()?))
    }
}
