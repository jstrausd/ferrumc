//! Fixed, bounded FCIE schema 1, independent of FCIS and storage envelopes.

use ferrumc_core::{DimensionId, EntityUuid, WorldId};
use ferrumc_items::decode_persisted_stack;
use ferrumc_math::Vec3;
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_storage::{EntityRecord, PersistentEntityKey, PersistentEntityRecord, SchemaVersion};

use super::{
    checked_stack_bytes, EntityData, EntityTransform, EntityTransformComponent, ItemEntityCounters,
    ItemEntityError, ItemEntityProperties, ItemEntityState, ItemNativeKeyField, ItemReferenceField,
    RestoredItemEntity, MAX_NATIVE_ITEM_ENTITY_BYTES, NATIVE_ITEM_ENTITY_HEADER_BYTES,
};

const MAGIC: &[u8; 4] = b"FCIE";
const VERSION: u16 = 1;
const PAYLOAD_SCHEMA: SchemaVersion = SchemaVersion::new(1);

pub(super) fn validate_transform(transform: EntityTransform) -> Result<(), ItemEntityError> {
    let canonical =
        EntityTransform::try_new(transform.position(), transform.yaw(), transform.pitch())?;
    if canonical.yaw().to_bits() != transform.yaw().to_bits() {
        return Err(ItemEntityError::NonCanonicalTransform {
            component: EntityTransformComponent::Yaw,
        });
    }
    if canonical.pitch().to_bits() != transform.pitch().to_bits() {
        return Err(ItemEntityError::NonCanonicalTransform {
            component: EntityTransformComponent::Pitch,
        });
    }
    Ok(())
}

pub(in crate::entity) fn encode_record(
    base: EntityData,
    properties: ItemEntityProperties,
    stack: &[u8],
) -> Result<(PersistentEntityKey, PersistentEntityRecord), ItemEntityError> {
    let size = NATIVE_ITEM_ENTITY_HEADER_BYTES + stack.len();
    if size > MAX_NATIVE_ITEM_ENTITY_BYTES {
        return Err(ItemEntityError::NativeBodyTooLarge {
            size,
            max: MAX_NATIVE_ITEM_ENTITY_BYTES,
        });
    }
    let mut body = Vec::new();
    body.try_reserve_exact(size)
        .map_err(|_| super::EntityStoreError::AllocationFailed)?;
    body.extend_from_slice(MAGIC);
    body.extend_from_slice(&VERSION.to_be_bytes());
    body.extend_from_slice(&base.entity_type().protocol_id().to_be_bytes());
    body.extend_from_slice(&base.owner().world().get().to_be_bytes());
    body.extend_from_slice(&base.owner().dimension().get().to_be_bytes());
    body.extend_from_slice(&base.uuid().to_bytes());
    let position = base.transform().position();
    for value in [position.x, position.y, position.z] {
        body.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    for value in [base.transform().yaw(), base.transform().pitch()] {
        body.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    let motion = properties.motion;
    for value in [motion.x, motion.y, motion.z] {
        body.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    body.extend_from_slice(&properties.bob_offset.to_bits().to_be_bytes());
    for value in [
        properties.counters.age,
        properties.counters.pickup_delay,
        properties.counters.health,
    ] {
        body.extend_from_slice(&value.to_be_bytes());
    }
    for uuid in [properties.owner_target, properties.thrower] {
        body.push(u8::from(uuid.is_some()));
        body.extend_from_slice(&uuid.map_or([0; 16], EntityUuid::to_bytes));
    }
    // Whole-body admission above proves this conversion for every target.
    body.extend_from_slice(&(stack.len() as u32).to_be_bytes());
    body.extend_from_slice(stack);
    let key = PersistentEntityKey::new(base.owner().world(), base.owner().dimension(), base.uuid());
    let record =
        PersistentEntityRecord::new(base.chunk(), EntityRecord::new(PAYLOAD_SCHEMA, body)?)?;
    Ok((key, record))
}

/// Decodes an exact native item record without guessing scope, UUID or wire ID.
///
/// Every header relationship, size, finite property and canonical rotation is
/// checked before the FCIS decoder can allocate a stack. The full FCIS model
/// then re-encodes byte-for-byte, retaining components, patch order and bits.
/// Empty, unsupported or malformed data returns a typed refusal, never a
/// normalized stack. Native motion preserves all finite values, including
/// signed zero and magnitudes above Vanilla NBT load's ten-unit threshold.
pub fn decode_item_entity(
    key: PersistentEntityKey,
    record: &PersistentEntityRecord,
) -> Result<RestoredItemEntity, ItemEntityError> {
    let raw_record = record.record();
    if raw_record.schema_version() != PAYLOAD_SCHEMA {
        return Err(ItemEntityError::UnsupportedPayloadSchema {
            found: raw_record.schema_version().get(),
        });
    }
    let data = raw_record.data();
    if data.len() > MAX_NATIVE_ITEM_ENTITY_BYTES {
        return Err(ItemEntityError::NativeBodyTooLarge {
            size: data.len(),
            max: MAX_NATIVE_ITEM_ENTITY_BYTES,
        });
    }
    if data.len() < NATIVE_ITEM_ENTITY_HEADER_BYTES {
        return Err(ItemEntityError::TruncatedNative {
            offset: data.len(),
            needed: NATIVE_ITEM_ENTITY_HEADER_BYTES - data.len(),
        });
    }
    let mut reader = NativeReader { data, offset: 0 };
    if reader.array::<4>()? != *MAGIC {
        return Err(ItemEntityError::InvalidMagic);
    }
    let version = u16::from_be_bytes(reader.array()?);
    if version != VERSION {
        return Err(ItemEntityError::UnsupportedNativeVersion { found: version });
    }
    let raw_type = i32::from_be_bytes(reader.array()?);
    let entity_type = EntityTypeId::try_from_protocol_id(raw_type)
        .filter(|kind| kind.resource_location() == "minecraft:item")
        .ok_or(ItemEntityError::InvalidNativeEntityType { raw: raw_type })?;
    let world = WorldId::new(u32::from_be_bytes(reader.array()?));
    let dimension = DimensionId::new(u32::from_be_bytes(reader.array()?));
    let uuid = EntityUuid::from_bytes(reader.array()?);
    for (matches, field) in [
        (world == key.world(), ItemNativeKeyField::World),
        (dimension == key.dimension(), ItemNativeKeyField::Dimension),
        (uuid == key.uuid(), ItemNativeKeyField::Uuid),
    ] {
        if !matches {
            return Err(ItemEntityError::NativeKeyMismatch { field });
        }
    }
    let position = Vec3::new(reader.f64()?, reader.f64()?, reader.f64()?);
    let yaw = reader.f32()?;
    let pitch = reader.f32()?;
    let transform = EntityTransform::try_new(position, yaw, pitch)?;
    if transform.yaw().to_bits() != yaw.to_bits() {
        return Err(ItemEntityError::NonCanonicalTransform {
            component: EntityTransformComponent::Yaw,
        });
    }
    if transform.pitch().to_bits() != pitch.to_bits() {
        return Err(ItemEntityError::NonCanonicalTransform {
            component: EntityTransformComponent::Pitch,
        });
    }
    if transform.chunk() != record.chunk() {
        return Err(ItemEntityError::NativeChunkMismatch {
            expected: record.chunk(),
            actual: transform.chunk(),
        });
    }
    let motion = Vec3::new(reader.f64()?, reader.f64()?, reader.f64()?);
    let bob_offset = reader.f32()?;
    let counters = ItemEntityCounters::new(
        i32::from_be_bytes(reader.array()?),
        i32::from_be_bytes(reader.array()?),
        i32::from_be_bytes(reader.array()?),
    );
    let owner_target = reader.reference(ItemReferenceField::OwnerTarget)?;
    let thrower = reader.reference(ItemReferenceField::Thrower)?;
    let properties =
        ItemEntityProperties::try_new(motion, bob_offset, counters, owner_target, thrower)?;
    let declared = u32::from_be_bytes(reader.array()?);
    let raw_stack = &data[reader.offset..];
    if declared as usize != raw_stack.len()
        || raw_stack.len() > MAX_NATIVE_ITEM_ENTITY_BYTES - NATIVE_ITEM_ENTITY_HEADER_BYTES
    {
        return Err(ItemEntityError::InvalidFcisLength {
            declared,
            remaining: raw_stack.len(),
        });
    }
    let stack = decode_persisted_stack(raw_stack)?;
    let checked = checked_stack_bytes(&stack)?;
    if checked != raw_stack {
        return Err(ItemEntityError::NonCanonicalFcis);
    }
    let item = ItemEntityState {
        stack,
        properties,
        fcis_len: checked.len(),
    };
    Ok(RestoredItemEntity {
        key,
        entity_type,
        transform,
        item,
    })
}

struct NativeReader<'a> {
    data: &'a [u8],
    offset: usize,
}
impl NativeReader<'_> {
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ItemEntityError> {
        let Some(end) = self.offset.checked_add(N) else {
            return Err(ItemEntityError::TruncatedNative {
                offset: self.offset,
                needed: N,
            });
        };
        let bytes = self
            .data
            .get(self.offset..end)
            .ok_or(ItemEntityError::TruncatedNative {
                offset: self.offset,
                needed: N,
            })?;
        let mut result = [0; N];
        result.copy_from_slice(bytes);
        self.offset = end;
        Ok(result)
    }
    fn f64(&mut self) -> Result<f64, ItemEntityError> {
        Ok(f64::from_bits(u64::from_be_bytes(self.array()?)))
    }
    fn f32(&mut self) -> Result<f32, ItemEntityError> {
        Ok(f32::from_bits(u32::from_be_bytes(self.array()?)))
    }
    fn reference(
        &mut self,
        field: ItemReferenceField,
    ) -> Result<Option<EntityUuid>, ItemEntityError> {
        let [tag] = self.array()?;
        let bytes = self.array()?;
        match tag {
            0 if bytes == [0; 16] => Ok(None),
            0 => Err(ItemEntityError::NonZeroAbsentUuid { field }),
            1 => Ok(Some(EntityUuid::from_bytes(bytes))),
            value => Err(ItemEntityError::InvalidUuidPresence { field, value }),
        }
    }
}
