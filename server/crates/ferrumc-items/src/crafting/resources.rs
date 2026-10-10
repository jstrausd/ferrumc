//! Allocation-free logical clone accounting after the lossless stack preflight.

use core::mem::size_of;

use ferrumc_nbt::NbtTag;

use crate::{ComponentTypeId, ComponentValue, ItemStack};

use super::CraftingError;

pub(super) fn clone_bytes(stack: &ItemStack) -> Result<usize, CraftingError> {
    let mut total = size_of::<ItemStack>();
    add(
        &mut total,
        stack.components().added().len(),
        size_of::<ComponentValue>(),
    )?;
    add(
        &mut total,
        stack.components().removed().len(),
        size_of::<ComponentTypeId>(),
    )?;
    for component in stack.components().added() {
        match component {
            ComponentValue::CustomData(tag) | ComponentValue::CustomName(tag) => {
                nbt_heap(tag, &mut total)?;
            }
            ComponentValue::Opaque(value) => add(&mut total, value.raw().len(), 1)?,
            _ => {}
        }
    }
    Ok(total)
}

fn add(total: &mut usize, count: usize, unit: usize) -> Result<(), CraftingError> {
    *total = count
        .checked_mul(unit)
        .and_then(|value| total.checked_add(value))
        .ok_or(CraftingError::ResourceOverflow)?;
    Ok(())
}

fn nbt_heap(tag: &NbtTag, total: &mut usize) -> Result<(), CraftingError> {
    match tag {
        NbtTag::ByteArray(values) => add(total, values.len(), size_of::<i8>())?,
        NbtTag::IntArray(values) => add(total, values.len(), size_of::<i32>())?,
        NbtTag::LongArray(values) => add(total, values.len(), size_of::<i64>())?,
        NbtTag::String(value) => add(total, value.len(), 1)?,
        NbtTag::List(values) => {
            add(total, values.len(), size_of::<NbtTag>())?;
            for value in values {
                nbt_heap(value, total)?;
            }
        }
        NbtTag::Compound(values) => {
            add(total, values.len(), size_of::<(String, NbtTag)>())?;
            for (name, value) in values.iter() {
                add(total, name.len(), 1)?;
                nbt_heap(value, total)?;
            }
        }
        _ => {}
    }
    Ok(())
}
