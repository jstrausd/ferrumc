//! Independent storage format for the authoritative item model.
//!
//! Unlike creative input normalization, this codec never clamps a count,
//! drops a component, or replaces a component representation. Schema 1 stores
//! the canonical item resource name, explicit model variant tags, length-framed
//! payloads, and patch order. Component ids and opaque trusted payloads remain
//! pinned to the Minecraft 1.21.8 model; a future registry change requires an
//! explicit schema migration. This is not the Minecraft slot wire format.
//!
//! One record is bounded before parsing. Component lengths and their cumulative
//! total are checked before copying or decoding any payload. Parent inventory
//! and chunk codecs must additionally enforce their own aggregate record caps;
//! the per-stack allowance is not an allocation budget for an entire inventory.

mod nbt_size;

use ferrumc_codec::BoundedReader;
use ferrumc_nbt::{read_named_root, write_named_root, NbtLimits};

use crate::component::{
    ComponentPatch, ComponentTypeId, ComponentValue, OpaqueComponent, CUSTOM_DATA, CUSTOM_NAME,
    DAMAGE, MAX_COMPONENT_TYPE_ID, MAX_DAMAGE, MAX_STACK_SIZE, UNBREAKABLE,
};
use crate::{
    ItemId, ItemStack, ItemValidationError, MAX_COMPONENTS, MAX_COMPONENTS_TOTAL_BYTES,
    MAX_COMPONENT_BYTES,
};

/// Maximum encoded size of one schema-1 stack, including its storage envelope.
///
/// Payload bytes are separately capped at 256 KiB total and 8192 bytes per
/// component. The 1024-byte envelope allowance covers the item resource name
/// and at most 64 additions/removals. Inventory consumers must impose a tighter
/// aggregate budget appropriate to their own persisted record.
pub const MAX_PERSISTED_STACK_BYTES: usize = MAX_COMPONENTS_TOTAL_BYTES + 1024;

const MAGIC: &[u8; 4] = b"FCIS";
const VERSION: u16 = 1;
const MAX_ITEM_NAME_BYTES: usize = 256;

/// A classified failure while encoding or decoding a persisted item stack.
///
/// Storage corruption is rejected rather than normalized. This enum is
/// non-exhaustive so future migrations can add distinct failure modes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ItemPersistenceError {
    /// The record does not start with the item-storage signature.
    #[error("invalid item persistence signature")]
    InvalidMagic,
    /// The stored schema version has no implemented decoder.
    #[error("unsupported item persistence schema {0}")]
    UnsupportedVersion(u16),
    /// The empty/present discriminator is not recognized.
    #[error("unknown persisted item discriminator {0}")]
    InvalidStackKind(u8),
    /// A record exceeded the bounded whole-stack allowance.
    #[error("persisted item record has {size} bytes, exceeding {max}")]
    RecordTooLarge {
        /// Encoded record size.
        size: usize,
        /// Maximum permitted size.
        max: usize,
    },
    /// The stored item name exceeded its bounded resource-name allowance.
    #[error("persisted item name has {size} bytes, exceeding {max}")]
    ItemNameTooLarge {
        /// Declared UTF-8 name size.
        size: usize,
        /// Maximum permitted size.
        max: usize,
    },
    /// The canonical item name does not resolve in the pinned registry.
    #[error("unknown persisted item name {0}")]
    UnknownItemName(String),
    /// A recognized name used an alias instead of the canonical resource name.
    #[error("noncanonical persisted item name {0}")]
    NonCanonicalItemName(String),
    /// A component representation tag is not recognized in schema 1.
    #[error("unknown persisted component representation {0}")]
    InvalidComponentKind(u8),
    /// A typed representation declared the id of a different component type.
    #[error("component representation {kind} declared mismatched type id {type_id}")]
    ComponentTypeMismatch {
        /// Storage representation tag.
        kind: u8,
        /// Declared component type id.
        type_id: i32,
    },
    /// A removal id is outside the pinned component registry, or an opaque
    /// component violates the checked server-authored passthrough contract.
    #[error("invalid persisted component type id {0}")]
    InvalidComponentType(i32),
    /// A patch used an id more than once, including addition/removal conflicts.
    #[error("duplicate persisted component type id {0}")]
    DuplicateComponent(i32),
    /// A typed `max_stack_size` value was outside its supported range.
    #[error("invalid persisted component value for type id {0}")]
    InvalidComponentValue(i32),
    /// A component payload exceeded its per-component allowance.
    #[error("component type {type_id} has {size} bytes, exceeding {max}")]
    ComponentTooLarge {
        /// Component type id.
        type_id: i32,
        /// Declared or preflighted payload size.
        size: usize,
        /// Maximum permitted size.
        max: usize,
    },
    /// File-form component NBT had a nonempty root name.
    #[error("persisted component NBT root must have an empty name")]
    NonEmptyNbtRootName,
    /// The authoritative stack or its patch violated a model count/size limit.
    #[error(transparent)]
    InvalidStack(#[from] ItemValidationError),
    /// A bounded byte read failed, including truncation, invalid UTF-8, and
    /// unexpected trailing bytes.
    #[error(transparent)]
    Codec(#[from] ferrumc_codec::CodecError),
    /// Component NBT violated its structural or resource limits.
    #[error(transparent)]
    Nbt(#[from] ferrumc_nbt::NbtError),
}

/// Encodes one authoritative stack into the independent schema-1 storage form.
///
/// All counts, patch ids, and payload sizes are preflighted before allocating
/// the result. Order, typed versus opaque representation, removed ids, NBT key
/// order and duplicate NBT names, and floating-point bits are preserved.
/// `max_stack_size` is limited to `1..=99` and cannot raise the item's registry
/// count ceiling. Signed damage values retain the current model's semantics.
///
/// # Errors
///
/// Rejects an invalid count, duplicate/conflicting patch ids, unsupported ids,
/// invalid typed payloads, oversized payloads, or invalid/oversized NBT. It does
/// not perform the lossy normalization used for creative network input.
pub fn encode_persisted_stack(stack: &ItemStack) -> Result<Vec<u8>, ItemPersistenceError> {
    let preflight = preflight_stack(stack)?;
    let Some(item) = stack.item() else {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.push(0);
        return Ok(out);
    };
    let patch = stack.components();
    let name = item.name().as_bytes();
    let mut out = Vec::with_capacity(preflight.capacity);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_be_bytes());
    out.push(1);
    out.extend_from_slice(&(name.len() as u16).to_be_bytes());
    out.extend_from_slice(name);
    out.push(stack.count());
    out.push(patch.added().len() as u8);
    out.push(patch.removed().len() as u8);
    for (value, size) in patch.added().iter().zip(preflight.payload_sizes) {
        write_component(&mut out, value, size)?;
    }
    for removed in patch.removed() {
        out.extend_from_slice(&removed.get().to_be_bytes());
    }
    Ok(out)
}

/// Validates the authoritative model without encoding, allocation, or changes.
///
/// Transfer and persistence share these limits so neither clones an unchecked
/// component tree. Encoded byte caps do not imply an equal decoded heap cap.
pub(crate) fn validate_authoritative_stack(stack: &ItemStack) -> Result<(), ItemPersistenceError> {
    preflight_stack(stack).map(|_| ())
}

/// Reuses the lossless preflight to account the exact schema-1 record size.
pub(crate) fn authoritative_encoded_size(stack: &ItemStack) -> Result<usize, ItemPersistenceError> {
    preflight_stack(stack).map(|preflight| preflight.capacity)
}

struct StackPreflight {
    payload_sizes: [usize; MAX_COMPONENTS],
    capacity: usize,
}

fn preflight_stack(stack: &ItemStack) -> Result<StackPreflight, ItemPersistenceError> {
    let mut payload_sizes = [0; MAX_COMPONENTS];
    let Some(item) = stack.item() else {
        return Ok(StackPreflight {
            payload_sizes,
            capacity: 7,
        });
    };
    validate_count(item, stack.count())?;
    let patch = stack.components();
    validate_patch_count(patch.added().len(), patch.removed().len())?;
    let mut seen = ComponentIds::new();
    let mut total = 0;
    for (value, slot) in patch.added().iter().zip(&mut payload_sizes) {
        seen.insert(value.type_id())?;
        let size = component_payload_size(value)?;
        check_component_size(value.type_id(), size)?;
        accumulate_payload(&mut total, size)?;
        *slot = size;
    }
    for removed in patch.removed() {
        seen.insert(removed.get())?;
    }
    let name = item.name().as_bytes();
    check_name_size(name.len())?;
    let capacity = 12 + name.len() + patch.added().len() * 9 + patch.removed().len() * 4 + total;
    Ok(StackPreflight {
        payload_sizes,
        capacity,
    })
}

/// Decodes exactly one persisted stack, rejecting corruption without changes.
///
/// The whole input, entry counts, individual payloads, and cumulative payload
/// sizes are bounded before allocations. The decoder preserves every accepted
/// model variant and patch order; it never invokes creative-slot decoding or
/// normalization. Trailing bytes and unknown schema/representation tags fail.
pub fn decode_persisted_stack(data: &[u8]) -> Result<ItemStack, ItemPersistenceError> {
    if data.len() > MAX_PERSISTED_STACK_BYTES {
        return Err(ItemPersistenceError::RecordTooLarge {
            size: data.len(),
            max: MAX_PERSISTED_STACK_BYTES,
        });
    }
    let mut reader = BoundedReader::new(data);
    if reader.read_bytes(MAGIC.len())? != MAGIC {
        return Err(ItemPersistenceError::InvalidMagic);
    }
    let version = reader.read_u16()?;
    if version != VERSION {
        return Err(ItemPersistenceError::UnsupportedVersion(version));
    }
    match reader.read_u8()? {
        0 => {
            reader.finish()?;
            return Ok(ItemStack::empty());
        }
        1 => {}
        other => return Err(ItemPersistenceError::InvalidStackKind(other)),
    }
    let item = read_item_name(&mut reader)?;
    let count = reader.read_u8()?;
    validate_count(item, count)?;
    let added_count = usize::from(reader.read_u8()?);
    let removed_count = usize::from(reader.read_u8()?);
    validate_patch_count(added_count, removed_count)?;
    let mut seen = ComponentIds::new();
    let mut total = 0;
    let mut added = Vec::with_capacity(added_count);
    for _ in 0..added_count {
        let kind = reader.read_u8()?;
        let type_id = reader.read_i32()?;
        seen.insert(type_id)?;
        let size = reader.read_u32()? as usize;
        check_component_size(type_id, size)?;
        accumulate_payload(&mut total, size)?;
        let raw = reader.read_bytes(size)?;
        added.push(read_component(kind, type_id, raw)?);
    }
    let mut removed = Vec::with_capacity(removed_count);
    for _ in 0..removed_count {
        let type_id = reader.read_i32()?;
        seen.insert(type_id)?;
        removed.push(ComponentTypeId::new(type_id));
    }
    reader.finish()?;
    Ok(ItemStack::try_new(
        item,
        count,
        ComponentPatch::new(added, removed),
    )?)
}

fn read_item_name(reader: &mut BoundedReader<'_>) -> Result<ItemId, ItemPersistenceError> {
    let size = usize::from(reader.read_u16()?);
    check_name_size(size)?;
    let name =
        core::str::from_utf8(reader.read_bytes(size)?).map_err(ferrumc_codec::CodecError::from)?;
    let item = ItemId::from_name(name)
        .ok_or_else(|| ItemPersistenceError::UnknownItemName(name.to_owned()))?;
    if name != item.name() {
        return Err(ItemPersistenceError::NonCanonicalItemName(name.to_owned()));
    }
    Ok(item)
}

fn check_name_size(size: usize) -> Result<(), ItemPersistenceError> {
    if size > MAX_ITEM_NAME_BYTES {
        return Err(ItemPersistenceError::ItemNameTooLarge {
            size,
            max: MAX_ITEM_NAME_BYTES,
        });
    }
    Ok(())
}

fn validate_count(item: ItemId, count: u8) -> Result<(), ItemPersistenceError> {
    if count == 0 || count > item.max_stack() {
        return Err(ItemValidationError::StackCountOutOfRange {
            item_id: item.id(),
            count,
            max: item.max_stack(),
        }
        .into());
    }
    Ok(())
}

fn validate_patch_count(added: usize, removed: usize) -> Result<(), ItemPersistenceError> {
    let count = added.saturating_add(removed);
    if count > MAX_COMPONENTS {
        return Err(ItemValidationError::TooManyComponents {
            count,
            max: MAX_COMPONENTS,
        }
        .into());
    }
    Ok(())
}

fn check_component_size(type_id: i32, size: usize) -> Result<(), ItemPersistenceError> {
    if size > MAX_COMPONENT_BYTES {
        return Err(ItemPersistenceError::ComponentTooLarge {
            type_id,
            size,
            max: MAX_COMPONENT_BYTES,
        });
    }
    Ok(())
}

fn accumulate_payload(total: &mut usize, size: usize) -> Result<(), ItemPersistenceError> {
    *total = total.saturating_add(size);
    if *total > MAX_COMPONENTS_TOTAL_BYTES {
        return Err(ItemValidationError::ComponentsTooLarge {
            total: *total,
            max: MAX_COMPONENTS_TOTAL_BYTES,
        }
        .into());
    }
    Ok(())
}

struct ComponentIds([bool; (MAX_COMPONENT_TYPE_ID + 1) as usize]);

impl ComponentIds {
    fn new() -> Self {
        Self([false; (MAX_COMPONENT_TYPE_ID + 1) as usize])
    }

    fn insert(&mut self, type_id: i32) -> Result<(), ItemPersistenceError> {
        let index = usize::try_from(type_id)
            .ok()
            .and_then(|index| self.0.get_mut(index))
            .ok_or(ItemPersistenceError::InvalidComponentType(type_id))?;
        if std::mem::replace(index, true) {
            return Err(ItemPersistenceError::DuplicateComponent(type_id));
        }
        Ok(())
    }
}

fn component_payload_size(value: &ComponentValue) -> Result<usize, ItemPersistenceError> {
    Ok(match value {
        ComponentValue::MaxStackSize(n) => {
            if !(1..=99).contains(n) {
                return Err(ItemPersistenceError::InvalidComponentValue(MAX_STACK_SIZE));
            }
            1
        }
        ComponentValue::Damage(_) | ComponentValue::MaxDamage(_) => 4,
        ComponentValue::Unbreakable => 0,
        ComponentValue::CustomName(tag) | ComponentValue::CustomData(tag) => {
            nbt_size::file_root_size(tag, &persistence_nbt_limits())?
        }
        ComponentValue::Opaque(component) => component.raw().len(),
    })
}

fn persistence_nbt_limits() -> NbtLimits {
    NbtLimits::default()
        .with_max_bytes(MAX_COMPONENT_BYTES)
        .with_max_list_len(MAX_COMPONENT_BYTES)
        .with_max_string_bytes(MAX_COMPONENT_BYTES)
}

fn write_component(
    out: &mut Vec<u8>,
    value: &ComponentValue,
    size: usize,
) -> Result<(), ItemPersistenceError> {
    out.push(component_kind(value));
    out.extend_from_slice(&value.type_id().to_be_bytes());
    out.extend_from_slice(&(size as u32).to_be_bytes());
    match value {
        ComponentValue::MaxStackSize(n) => out.push(*n),
        ComponentValue::Damage(n) | ComponentValue::MaxDamage(n) => {
            out.extend_from_slice(&n.to_be_bytes());
        }
        ComponentValue::Unbreakable => {}
        ComponentValue::CustomName(tag) | ComponentValue::CustomData(tag) => {
            // The allocation is bounded by the exact-size preflight: the NBT
            // writer itself does not enforce all configured resource limits.
            out.extend_from_slice(&write_named_root("", tag, &persistence_nbt_limits())?);
        }
        ComponentValue::Opaque(component) => out.extend_from_slice(component.raw()),
    }
    Ok(())
}

fn component_kind(value: &ComponentValue) -> u8 {
    match value {
        ComponentValue::MaxStackSize(_) => 1,
        ComponentValue::Damage(_) => 2,
        ComponentValue::MaxDamage(_) => 3,
        ComponentValue::Unbreakable => 4,
        ComponentValue::CustomName(_) => 5,
        ComponentValue::CustomData(_) => 6,
        ComponentValue::Opaque(_) => 7,
    }
}

fn read_component(
    kind: u8,
    type_id: i32,
    raw: &[u8],
) -> Result<ComponentValue, ItemPersistenceError> {
    let expected = match kind {
        1 => MAX_STACK_SIZE,
        2 => DAMAGE,
        3 => MAX_DAMAGE,
        4 => UNBREAKABLE,
        5 => CUSTOM_NAME,
        6 => CUSTOM_DATA,
        7 => {
            let id = ComponentTypeId::new(type_id);
            if id.is_dangerous() || !id.is_in_range() {
                return Err(ItemPersistenceError::InvalidComponentType(type_id));
            }
            let opaque = OpaqueComponent::new(type_id, raw.to_vec())
                .ok_or(ItemPersistenceError::InvalidComponentType(type_id))?;
            return Ok(ComponentValue::Opaque(opaque));
        }
        other => return Err(ItemPersistenceError::InvalidComponentKind(other)),
    };
    if type_id != expected {
        return Err(ItemPersistenceError::ComponentTypeMismatch { kind, type_id });
    }
    let mut reader = BoundedReader::new(raw);
    let value = match kind {
        1 => {
            let value = reader.read_u8()?;
            if !(1..=99).contains(&value) {
                return Err(ItemPersistenceError::InvalidComponentValue(type_id));
            }
            ComponentValue::MaxStackSize(value)
        }
        2 => ComponentValue::Damage(reader.read_i32()?),
        3 => ComponentValue::MaxDamage(reader.read_i32()?),
        4 => ComponentValue::Unbreakable,
        5 | 6 => {
            let (name, tag) = read_named_root(raw, &persistence_nbt_limits())?;
            if !name.is_empty() {
                return Err(ItemPersistenceError::NonEmptyNbtRootName);
            }
            return Ok(if kind == 5 {
                ComponentValue::CustomName(tag)
            } else {
                ComponentValue::CustomData(tag)
            });
        }
        other => return Err(ItemPersistenceError::InvalidComponentKind(other)),
    };
    reader.finish()?;
    Ok(value)
}

#[cfg(test)]
mod tests;
