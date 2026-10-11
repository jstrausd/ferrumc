//! Bounded trusted `DATA_ITEM` metadata, without entity/publication authority.

use std::collections::TryReserveError;

use ferrumc_codec::write_var_int;
use ferrumc_nbt::{network_root_size, write_network_root_into, NbtError, NbtLimits};

use crate::persistence::validate_authoritative_stack;
use crate::{
    ComponentValue, ItemPersistenceError, ItemStack, MAX_COMPONENTS, MAX_COMPONENTS_TOTAL_BYTES,
    MAX_COMPONENT_BYTES,
};

/// Local maximum for the complete `DATA_ITEM` entry and metadata end byte.
///
/// The 256 KiB policy includes index, serializer, slot and terminator, and sits
/// below the protocol envelope. It is not a Minecraft limit, decoded heap cap,
/// aggregate budget, or guarantee of opaque component inner-schema admission.
pub const MAX_ITEM_ENTITY_METADATA_BYTES: usize = MAX_COMPONENTS_TOTAL_BYTES;

/// One complete bounded trusted `DATA_ITEM` entry, followed by the metadata end.
///
/// Bytes are `08 07 <complete trusted slot> ff`, without entity ID or framing.
/// Construction is restricted to [`encode_item_entity_metadata`]. Opaque bytes
/// retain their existing trusted model contract; no original component parser,
/// sanitizer, or confirmed entity/publication authority is supplied by this type.
#[derive(Debug)]
pub struct ItemEntityMetadata {
    bytes: Vec<u8>,
}

impl ItemEntityMetadata {
    /// Borrows the complete immutable metadata bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Moves the already checked buffer without cloning or converting it.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A classified failure before publication of a complete metadata buffer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ItemMetadataError {
    /// The existing lossless model preflight refused the stack.
    #[error(transparent)]
    InvalidStack(#[from] ItemPersistenceError),
    /// Present AIR cannot be emitted without losing its model representation.
    #[error("present air is unsupported in item entity metadata")]
    UnsupportedPresentAir,
    /// The complete metadata exceeds the local encoded-byte policy.
    #[error("item entity metadata of {size} bytes exceeds the maximum of {max}")]
    MetadataTooLarge {
        /// The exact complete encoded metadata size.
        size: usize,
        /// The local maximum complete metadata size.
        max: usize,
    },
    /// Checked size arithmetic or an encoded count conversion overflowed.
    #[error("item entity metadata size arithmetic overflowed")]
    SizeOverflow,
    /// The one bounded fallible output reservation failed.
    #[error("cannot reserve {requested} bytes for item entity metadata: {source}")]
    AllocationFailed {
        /// The already admitted complete output size.
        requested: usize,
        /// The actual allocator or capacity refusal.
        #[source]
        source: TryReserveError,
    },
    /// The checked shared network-NBT producer refused a typed payload.
    #[error(transparent)]
    Nbt(#[from] NbtError),
    /// Direct emission unexpectedly disagreed with its checked size.
    #[error("item metadata output has {actual} bytes but expected {expected}")]
    OutputSizeMismatch {
        /// The admitted payload or complete output size.
        expected: usize,
        /// The observed or attempted byte count.
        actual: usize,
    },
}

/// Encodes one complete bounded trusted `DATA_ITEM` metadata entry.
///
/// Validates the unchanged lossless item model and exact complete wire size
/// before the single fallible output reservation. No stack/component clones,
/// temporary NBT buffers, normalization or silent empty replacement occur.
/// Canonical empty encodes as `08 07 00 ff`. Added/removed order, signed damage,
/// NBT representation and checked trusted opaque payloads remain unchanged.
///
/// # Errors
///
/// Model errors take precedence over present-AIR refusal and metadata size
/// policy. Later failures include reservation, checked NBT or defensive size
/// mismatch errors. Every error discards only private output; source is immutable.
pub fn encode_item_entity_metadata(
    stack: &ItemStack,
) -> Result<ItemEntityMetadata, ItemMetadataError> {
    let plan = preflight(stack)?;
    let mut out = Vec::new();
    out.try_reserve_exact(plan.size)
        .map_err(|source| ItemMetadataError::AllocationFailed {
            requested: plan.size,
            source,
        })?;
    let mut writer = Writer {
        out,
        expected: plan.size,
    };
    writer.append(&[8, 7])?;
    if let Some(item) = stack.item() {
        writer.var_int(i32::from(stack.count()))?;
        writer.var_int(item.id())?;
        writer.var_int(plan.added)?;
        writer.var_int(plan.removed)?;
        let limits = metadata_nbt_limits();
        for (value, size) in stack.components().added().iter().zip(plan.payload_sizes) {
            writer.var_int(value.type_id())?;
            match value {
                ComponentValue::MaxStackSize(value) => writer.var_int(i32::from(*value))?,
                ComponentValue::Damage(value) | ComponentValue::MaxDamage(value) => {
                    writer.var_int(*value)?;
                }
                ComponentValue::Unbreakable => {}
                ComponentValue::CustomData(tag) | ComponentValue::CustomName(tag) => {
                    writer.check_append(size)?;
                    let actual = write_network_root_into(&mut writer.out, tag, &limits)?;
                    if actual != size {
                        return Err(ItemMetadataError::OutputSizeMismatch {
                            expected: size,
                            actual,
                        });
                    }
                }
                ComponentValue::Opaque(value) => writer.append(value.raw())?,
            }
        }
        for removed in stack.components().removed() {
            writer.var_int(removed.get())?;
        }
    } else {
        writer.var_int(0)?;
    }
    writer.append(&[255])?;
    if writer.out.len() != plan.size {
        return Err(ItemMetadataError::OutputSizeMismatch {
            expected: plan.size,
            actual: writer.out.len(),
        });
    }
    Ok(ItemEntityMetadata { bytes: writer.out })
}

struct Plan {
    size: usize,
    added: i32,
    removed: i32,
    payload_sizes: [usize; MAX_COMPONENTS],
}

fn preflight(stack: &ItemStack) -> Result<Plan, ItemMetadataError> {
    validate_authoritative_stack(stack)?;
    let mut plan = Plan {
        size: 4,
        added: 0,
        removed: 0,
        payload_sizes: [0; MAX_COMPONENTS],
    };
    let Some(item) = stack.item() else {
        return Ok(plan);
    };
    if item.id() == 0 {
        return Err(ItemMetadataError::UnsupportedPresentAir);
    }
    let patch = stack.components();
    plan.added = i32::try_from(patch.added().len()).map_err(|_| ItemMetadataError::SizeOverflow)?;
    plan.removed =
        i32::try_from(patch.removed().len()).map_err(|_| ItemMetadataError::SizeOverflow)?;
    // Three bytes for DATA_ITEM index, serializer and the real final terminator.
    plan.size = 3;
    for value in [
        i32::from(stack.count()),
        item.id(),
        plan.added,
        plan.removed,
    ] {
        add_size(&mut plan.size, var_int_size(value)?)?;
    }
    let limits = metadata_nbt_limits();
    for (value, size) in patch.added().iter().zip(&mut plan.payload_sizes) {
        *size = match value {
            ComponentValue::MaxStackSize(value) => var_int_size(i32::from(*value))?,
            ComponentValue::Damage(value) | ComponentValue::MaxDamage(value) => {
                var_int_size(*value)?
            }
            ComponentValue::Unbreakable => 0,
            ComponentValue::CustomName(tag) | ComponentValue::CustomData(tag) => {
                network_root_size(tag, &limits)?
            }
            ComponentValue::Opaque(value) => value.raw().len(),
        };
        add_size(&mut plan.size, var_int_size(value.type_id())?)?;
        add_size(&mut plan.size, *size)?;
    }
    for removed in patch.removed() {
        add_size(&mut plan.size, var_int_size(removed.get())?)?;
    }
    if plan.size > MAX_ITEM_ENTITY_METADATA_BYTES {
        return Err(ItemMetadataError::MetadataTooLarge {
            size: plan.size,
            max: MAX_ITEM_ENTITY_METADATA_BYTES,
        });
    }
    Ok(plan)
}

fn add_size(size: &mut usize, amount: usize) -> Result<(), ItemMetadataError> {
    *size = size
        .checked_add(amount)
        .ok_or(ItemMetadataError::SizeOverflow)?;
    Ok(())
}

fn var_int_size(value: i32) -> Result<usize, ItemMetadataError> {
    // Negative two's-complement values use all 32 bits, hence five wire bytes.
    usize::try_from(value.cast_unsigned().bit_width().max(1).div_ceil(7))
        .map_err(|_| ItemMetadataError::SizeOverflow)
}

fn metadata_nbt_limits() -> NbtLimits {
    NbtLimits::default()
        .with_max_bytes(MAX_COMPONENT_BYTES)
        .with_max_list_len(MAX_COMPONENT_BYTES)
        .with_max_string_bytes(MAX_COMPONENT_BYTES)
}

struct Writer {
    out: Vec<u8>,
    expected: usize,
}

impl Writer {
    fn check_append(&self, amount: usize) -> Result<(), ItemMetadataError> {
        let actual = self
            .out
            .len()
            .checked_add(amount)
            .ok_or(ItemMetadataError::SizeOverflow)?;
        if actual > self.expected {
            return Err(ItemMetadataError::OutputSizeMismatch {
                expected: self.expected,
                actual,
            });
        }
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), ItemMetadataError> {
        self.check_append(bytes.len())?;
        self.out.extend_from_slice(bytes);
        Ok(())
    }

    fn var_int(&mut self, value: i32) -> Result<(), ItemMetadataError> {
        let mut bytes = [0; 5];
        let mut remaining = &mut bytes[..];
        write_var_int(&mut remaining, value);
        let actual = 5 - remaining.len();
        let expected = var_int_size(value)?;
        if actual != expected {
            return Err(ItemMetadataError::OutputSizeMismatch { expected, actual });
        }
        self.append(&bytes[..actual])
    }
}
