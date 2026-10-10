//! Checked, pure item-count transfers over the supported authoritative model.
//!
//! These plans conserve item quantities and component representation. They do
//! not implement inventory slot policy, item-entity eligibility, plugin events,
//! or a persistent transaction. Callers must retain exclusive authority while
//! applying both returned stacks; a plan has no revision/CAS or replay guard.

use ferrumc_nbt::NbtTag;

use crate::component::{ComponentPatch, ComponentValue};
use crate::persistence::validate_authoritative_stack;
use crate::{ItemPersistenceError, ItemStack};

/// A classified failure before a split or transfer produces an owned plan.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StackTransferError {
    /// The source violates the supported authoritative count/component bounds.
    #[error("invalid transfer source: {0}")]
    InvalidSource(ItemPersistenceError),
    /// The destination violates the authoritative count/component bounds.
    #[error("invalid transfer destination: {0}")]
    InvalidDestination(ItemPersistenceError),
    /// The existing destination already exceeds the supplied total capacity.
    #[error("destination count {count} exceeds supplied total capacity {capacity}")]
    DestinationOverCapacity {
        /// Existing quantity, which is not silently reduced.
        count: u8,
        /// Caller-supplied total destination capacity.
        capacity: u8,
    },
}

/// Owned source/destination results of one checked, in-memory transfer decision.
///
/// Inputs remain unchanged. Applying only one returned stack, applying a stale
/// plan, or persisting the pair in separate transactions does not preserve the
/// contract; the caller owns synchronization, publication, and crash safety.
#[derive(Debug, Clone)]
pub struct StackTransferPlan {
    source_after: ItemStack,
    destination_after: ItemStack,
    moved: u8,
}

impl StackTransferPlan {
    /// The untransferred source remainder; a fully drained source is canonical empty.
    #[must_use]
    pub fn source_after(&self) -> &ItemStack {
        &self.source_after
    }

    /// The resulting destination, or the separated stack for [`plan_split`].
    #[must_use]
    pub fn destination_after(&self) -> &ItemStack {
        &self.destination_after
    }

    /// The exact quantity removed from the source and added to the destination.
    #[must_use]
    pub fn moved(&self) -> u8 {
        self.moved
    }

    /// Consumes the plan into `(source_after, destination_after)` for publication.
    #[must_use]
    pub fn into_stacks(self) -> (ItemStack, ItemStack) {
        (self.source_after, self.destination_after)
    }
}

/// Plans separation of up to `requested` items without mutating the source.
///
/// `requested` is an upper bound, not an authoritative count: zero separates
/// nothing and a request larger than the source separates the whole stack.
/// Both results retain the source's exact component representation whenever
/// present. Empty outputs have count zero and an empty patch. Invalid sources
/// fail even for a zero request, before cloning their component tree.
///
/// # Errors
///
/// Returns [`StackTransferError::InvalidSource`] for an invalid registry count,
/// patch id/count/value, or component byte/NBT structural limit.
pub fn plan_split(
    source: &ItemStack,
    requested: u8,
) -> Result<StackTransferPlan, StackTransferError> {
    validate_authoritative_stack(source).map_err(StackTransferError::InvalidSource)?;
    let moved = requested.min(source.count());
    Ok(StackTransferPlan {
        source_after: copy_at_count(source, source.count() - moved)
            .map_err(StackTransferError::InvalidSource)?,
        destination_after: copy_at_count(source, moved)
            .map_err(StackTransferError::InvalidSource)?,
        moved,
    })
}

/// Plans a non-swapping transfer of up to `requested` items into a destination.
///
/// All `u8` requests/capacities are valid bounded quantities. `requested` is an
/// upper bound and `destination_capacity` is the destination's **total** allowed
/// count, not its free space. A capacity above the registry maximum is safely
/// limited by that maximum; it cannot enlarge the item ceiling. Zero permits
/// only an empty destination. Existing counts are validated, never clamped.
///
/// An empty destination receives the source patch verbatim. Present stacks
/// merge only if both item ids and the ordered component representation match,
/// including typed/opaque variants, removals, NBT duplicates/order, and exact
/// floating-point bits. Incompatible stacks yield `moved() == 0`; they never
/// swap or overwrite. Identical NaN bits match; distinct NaN payloads and signed
/// zeros do not. This is deliberately stricter than Paper's normalized maps.
///
/// Caller-supplied capacity must include gameplay policy such as resolved
/// `max_stack_size`, damage/stackability, and slot-specific limits. This
/// primitive does not resolve item defaults or interpret opaque components.
/// Both inputs are fully preflighted, even for an empty/no-op/incompatible
/// transfer. No mutation occurs and no result is cloned before validation.
///
/// # Errors
///
/// Returns a classified source/destination model error or
/// [`StackTransferError::DestinationOverCapacity`] if an existing destination
/// is already above the supplied total capacity.
pub fn plan_transfer(
    source: &ItemStack,
    destination: &ItemStack,
    requested: u8,
    destination_capacity: u8,
) -> Result<StackTransferPlan, StackTransferError> {
    validate_authoritative_stack(source).map_err(StackTransferError::InvalidSource)?;
    validate_authoritative_stack(destination).map_err(StackTransferError::InvalidDestination)?;
    if destination.count() > destination_capacity {
        return Err(StackTransferError::DestinationOverCapacity {
            count: destination.count(),
            capacity: destination_capacity,
        });
    }
    let compatible = destination.item().is_none()
        || source.item() == destination.item()
            && same_patch_representation(source.components(), destination.components());
    let ceiling = source
        .item()
        .map_or(0, |item| item.max_stack().min(destination_capacity));
    // Compatible present stacks have the same registry ceiling; validation
    // proves both that ceiling and the supplied capacity cover the old count.
    let moved = if compatible {
        requested
            .min(source.count())
            .min(ceiling - destination.count())
    } else {
        0
    };
    let destination_after = if moved == 0 {
        destination.clone()
    } else {
        let template = if destination.item().is_some() {
            destination
        } else {
            source
        };
        copy_at_count(template, destination.count() + moved)
            .map_err(StackTransferError::InvalidDestination)?
    };
    Ok(StackTransferPlan {
        source_after: copy_at_count(source, source.count() - moved)
            .map_err(StackTransferError::InvalidSource)?,
        destination_after,
        moved,
    })
}

/// Compares two validated stack representations without allocation or encoding.
///
/// Both inputs pass the lossless persistence preflight before any equality
/// decision, including for empty or incompatible stacks. Valid counts are then
/// ignored: identity is the item id and exact ordered component representation,
/// including typed/opaque variants, removals, NBT duplicates/order, and float
/// bits. Identical NaN bits match; different payloads and signed zeros do not.
/// Two canonical empty stacks match; empty versus present does not. Present
/// air remains present. Inputs are neither normalized nor cloned.
///
/// This checks model representation, not Paper's resolved component equality,
/// gameplay stackability, slot capacity, or permission to commit a transfer.
///
/// # Errors
///
/// Returns [`StackTransferError::InvalidSource`] or
/// [`StackTransferError::InvalidDestination`] for the corresponding preflight
/// failure. Source is checked first, so its error wins when both are invalid.
pub fn same_stack_representation(
    source: &ItemStack,
    destination: &ItemStack,
) -> Result<bool, StackTransferError> {
    validate_authoritative_stack(source).map_err(StackTransferError::InvalidSource)?;
    validate_authoritative_stack(destination).map_err(StackTransferError::InvalidDestination)?;
    Ok(source.item() == destination.item()
        && same_patch_representation(source.components(), destination.components()))
}

pub(crate) fn copy_at_count(
    stack: &ItemStack,
    count: u8,
) -> Result<ItemStack, ItemPersistenceError> {
    match (stack.item(), count) {
        (_, 0) | (None, _) => Ok(ItemStack::empty()),
        (Some(item), _) => Ok(ItemStack::try_new(item, count, stack.components().clone())?),
    }
}

fn same_patch_representation(left: &ComponentPatch, right: &ComponentPatch) -> bool {
    left.removed() == right.removed()
        && left.added().len() == right.added().len()
        && left
            .added()
            .iter()
            .zip(right.added())
            .all(|(left, right)| same_component_representation(left, right))
}

fn same_component_representation(left: &ComponentValue, right: &ComponentValue) -> bool {
    match (left, right) {
        (ComponentValue::MaxStackSize(left), ComponentValue::MaxStackSize(right)) => left == right,
        (ComponentValue::Damage(left), ComponentValue::Damage(right))
        | (ComponentValue::MaxDamage(left), ComponentValue::MaxDamage(right)) => left == right,
        (ComponentValue::Unbreakable, ComponentValue::Unbreakable) => true,
        (ComponentValue::CustomName(left), ComponentValue::CustomName(right))
        | (ComponentValue::CustomData(left), ComponentValue::CustomData(right)) => {
            same_nbt_representation(left, right)
        }
        (ComponentValue::Opaque(left), ComponentValue::Opaque(right)) => left == right,
        _ => false,
    }
}

fn same_nbt_representation(left: &NbtTag, right: &NbtTag) -> bool {
    match (left, right) {
        (NbtTag::Float(left), NbtTag::Float(right)) => left.to_bits() == right.to_bits(),
        (NbtTag::Double(left), NbtTag::Double(right)) => left.to_bits() == right.to_bits(),
        (NbtTag::List(left), NbtTag::List(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| same_nbt_representation(left, right))
        }
        (NbtTag::Compound(left), NbtTag::Compound(right)) => {
            left.len() == right.len()
                && left.iter().zip(right.iter()).all(
                    |((left_name, left_value), (right_name, right_value))| {
                        left_name == right_name && same_nbt_representation(left_value, right_value)
                    },
                )
        }
        // Scalars/arrays contain no floats; float-bearing descendants must use
        // the explicit recursive branches above, not derived IEEE equality.
        _ => left == right,
    }
}

#[cfg(test)]
mod tests;
