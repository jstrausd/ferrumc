//! Closed normal smelting/fuel facts and an allocation-bounded one-furnace tick.
//! Consumers own exclusive authority, residency, input-change timer resets,
//! lit block state, screens, events, recipe awards/XP and durable publication.

mod data;
#[cfg(test)]
mod tests;

use std::num::NonZeroU8;

use crate::{
    persisted_stack_size, plan_split, plan_transfer, same_stack_representation, stack_capacity,
    ComponentPatch, ContainerTransferError, ItemId, ItemPersistenceError, ItemStack,
    StackTransferError,
};

/// Three slots and timer framing together may occupy at most sixteen KiB.
/// This encoded-byte bound matches World's complete furnace entity allowance.
pub const MAX_FURNACE_BYTES: usize = 16 * 1024;
/// Versioned entity header, three u32 item frames and four u16 timers.
pub const FURNACE_FRAMING_BYTES: usize = 4 + 3 * 4 + 4 * 2;

/// Refusal before a furnace plan publishes state or consumes a source item.
#[derive(Debug, thiserror::Error)]
pub enum SmeltingError {
    /// A source stack failed authoritative lossless validation.
    #[error(transparent)]
    Stack(#[from] ItemPersistenceError),
    /// A source stack has an unsupported or exceeded effective capacity.
    #[error(transparent)]
    Capacity(#[from] ContainerTransferError),
    /// A checked split/transfer failed.
    #[error(transparent)]
    Transfer(#[from] StackTransferError),
    /// Remaining burn must not exceed duration; cooking total is 1..32767,
    /// progress is strictly below total and duration is at most 32767.
    #[error("invalid furnace timers")]
    InvalidTimers,
    /// Complete persisted furnace bytes exceed sixteen KiB.
    #[error("furnace item budget exceeded")]
    BudgetExceeded,
    /// A present air stack has no ordinary furnace semantics.
    #[error("present air in furnace")]
    PresentAir,
    /// A generated pinned fact references an absent registry item.
    #[error("invalid pinned smelting catalog")]
    InvalidCatalog,
    /// Sponge processing cannot safely replace a stack of multiple buckets.
    #[error("unsupported furnace bucket remainder")]
    UnsupportedRemainder,
}

/// A resolved fixed-output normal furnace recipe from the pinned resources.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmeltingRecipe {
    name: &'static str,
    result: ItemId,
    cooking_ticks: u16,
    experience: f32,
}

impl SmeltingRecipe {
    /// Stable original namespaced recipe key.
    pub const fn name(self) -> &'static str {
        self.name
    }
    /// Fixed result item identity (one item, original defaults).
    pub const fn result(self) -> ItemId {
        self.result
    }
    /// Original cook duration; all seventy-two builtin normal recipes use 200.
    pub const fn cooking_ticks(self) -> u16 {
        self.cooking_ticks
    }
    /// Original experience per smelt; this descriptor does not spawn or award XP.
    pub const fn experience(self) -> f32 {
        self.experience
    }
    /// Build the fixed componentless single-item output.
    pub fn output(self) -> ItemStack {
        ItemStack::new(self.result, NonZeroU8::MIN, ComponentPatch::empty())
    }
}

/// Original fuel duration and crafting remainder, independent of a live furnace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuelProperties {
    burn_ticks: u16,
    remainder: Option<ItemId>,
}

impl FuelProperties {
    /// Normal-furnace burn duration at twenty TPS (coal1600, lava20000).
    pub const fn burn_ticks(self) -> u16 {
        self.burn_ticks
    }
    /// Original item crafting remainder after the last fuel item is consumed.
    pub const fn remainder(self) -> Option<ItemId> {
        self.remainder
    }
}

/// Resolve ordinary smelting by item membership; surviving input patches remain
/// unchanged. Fixed outputs do not inherit input names, damage or other patches.
pub fn smelting_recipe(input: &ItemStack) -> Result<Option<SmeltingRecipe>, SmeltingError> {
    validate_stack(input)?;
    let Some(item) = input.item() else {
        return Ok(None);
    };
    // Last resource key wins as in Paper's uncached normal recipe selection.
    let Some((_, name, result, cooking_ticks, experience)) = data::RECIPES
        .iter()
        .rev()
        .find(|(ingredient, _, _, _, _)| *ingredient == item.name())
    else {
        return Ok(None);
    };
    Ok(Some(SmeltingRecipe {
        name,
        result: ItemId::from_name(result).ok_or(SmeltingError::InvalidCatalog)?,
        cooking_ticks: *cooking_ticks,
        experience: f32::from_bits(*experience),
    }))
}

/// Resolve the pinned 269-fuel catalog, including exact original tag expansion
/// and nonflammable-wood removal. Patches do not change item-based fuel duration.
pub fn fuel_properties(input: &ItemStack) -> Result<Option<FuelProperties>, SmeltingError> {
    validate_stack(input)?;
    let Some(item) = input.item() else {
        return Ok(None);
    };
    let Ok(index) = data::FUELS.binary_search_by_key(&item.name(), |(name, _)| *name) else {
        return Ok(None);
    };
    let remainder = if item.name() == "minecraft:lava_bucket" {
        Some(ItemId::from_name("bucket").ok_or(SmeltingError::InvalidCatalog)?)
    } else {
        None
    };
    Ok(Some(FuelProperties {
        burn_ticks: data::FUELS[index].1,
        remainder,
    }))
}

/// Owned postimage of one furnace tick; every field must commit together.
/// The plan grants no revision, chunk ownership, replay or persistence authority.
#[derive(Debug, Clone)]
pub struct FurnaceTickPlan {
    slots: [ItemStack; 3],
    burn_remaining: u16,
    burn_duration: u16,
    cook_progress: u16,
    cook_total: u16,
    smelted: bool,
    fuel_consumed: bool,
}

impl FurnaceTickPlan {
    /// Complete input/fuel/output postimage, in indices zero/one/two.
    pub fn slots(&self) -> &[ItemStack; 3] {
        &self.slots
    }
    /// Move all three postimage slots into the exclusive World owner.
    pub fn into_slots(self) -> [ItemStack; 3] {
        self.slots
    }
    /// Remaining burn after this tick.
    pub const fn burn_remaining(&self) -> u16 {
        self.burn_remaining
    }
    /// Duration of the most recently started fuel item.
    pub const fn burn_duration(&self) -> u16 {
        self.burn_duration
    }
    /// Progress toward the next completed smelt.
    pub const fn cook_progress(&self) -> u16 {
        self.cook_progress
    }
    /// Current recipe's cook duration.
    pub const fn cook_total(&self) -> u16 {
        self.cook_total
    }
    /// Whether this tick consumed one input and produced one result.
    pub const fn smelted(&self) -> bool {
        self.smelted
    }
    /// Whether this tick consumed one new fuel item.
    pub const fn fuel_consumed(&self) -> bool {
        self.fuel_consumed
    }
    /// Whether the owning furnace block must currently have its lit property set.
    pub const fn is_lit(&self) -> bool {
        self.burn_remaining > 0
    }
}

/// Plan exactly one loaded ordinary-furnace tick without mutating any input.
/// Burn decrements first; fresh fuel starts only with compatible available output
/// space. Cooking advances once, smelts at the recipe threshold, and cools by
/// two when idle without both input and fuel. An already burning blocked furnace
/// spends its remaining burn and resets cooking, matching the normal reference.
/// Output merging conservatively requires exact ordered component representation.
/// Input replacement must reset progress in the consumer before calling this.
pub fn plan_furnace_tick(
    slots: &[ItemStack; 3],
    burn_remaining: u16,
    burn_duration: u16,
    cook_progress: u16,
    cook_total: u16,
) -> Result<FurnaceTickPlan, SmeltingError> {
    validate_furnace_parts(
        slots,
        burn_remaining,
        burn_duration,
        cook_progress,
        cook_total,
    )?;
    let recipe = smelting_recipe(&slots[0])?;
    let output = recipe.map(SmeltingRecipe::output);
    let can_burn = match &output {
        Some(output) => {
            slots[2].item().is_none()
                || (same_stack_representation(output, &slots[2])?
                    && slots[2].count() < stack_capacity(&slots[2])?)
        }
        None => false,
    };
    let mut plan = FurnaceTickPlan {
        slots: slots.clone(),
        burn_remaining: burn_remaining.saturating_sub(1),
        burn_duration,
        cook_progress,
        cook_total,
        smelted: false,
        fuel_consumed: false,
    };
    let active = plan.is_lit() || (slots[0].item().is_some() && slots[1].item().is_some());
    if !active {
        plan.cook_progress = plan.cook_progress.saturating_sub(2);
        return Ok(plan);
    }
    if !plan.is_lit() && can_burn {
        if let Some(fuel) = fuel_properties(&slots[1])? {
            plan.burn_remaining = fuel.burn_ticks();
            plan.burn_duration = fuel.burn_ticks();
            let (remaining, _) = plan_split(&slots[1], 1)?.into_stacks();
            plan.slots[1] = if remaining.item().is_none() {
                match fuel.remainder() {
                    Some(item) => ItemStack::new(item, NonZeroU8::MIN, ComponentPatch::empty()),
                    None => remaining,
                }
            } else {
                remaining
            };
            plan.fuel_consumed = true;
        } else {
            plan.burn_remaining = 0;
            plan.burn_duration = 0;
        }
    }
    if plan.is_lit() && can_burn {
        let recipe = recipe.ok_or(SmeltingError::InvalidCatalog)?;
        if plan.cook_progress == 0 {
            plan.cook_total = recipe.cooking_ticks();
        }
        plan.cook_progress += 1;
        if plan.cook_progress >= plan.cook_total {
            let output = output.ok_or(SmeltingError::InvalidCatalog)?;
            let capacity = if slots[2].item().is_none() {
                stack_capacity(&output)?
            } else {
                stack_capacity(&slots[2])?
            };
            let (_, result) = plan_transfer(&output, &slots[2], 1, capacity)?.into_stacks();
            let (remaining, _) = plan_split(&slots[0], 1)?.into_stacks();
            if slots[0]
                .item()
                .is_some_and(|item| item.name() == "minecraft:wet_sponge")
                && plan.slots[1]
                    .item()
                    .is_some_and(|item| item.name() == "minecraft:bucket")
            {
                if plan.slots[1].count() != 1 {
                    return Err(SmeltingError::UnsupportedRemainder);
                }
                let water =
                    ItemId::from_name("water_bucket").ok_or(SmeltingError::InvalidCatalog)?;
                plan.slots[1] = ItemStack::new(water, NonZeroU8::MIN, ComponentPatch::empty());
            }
            plan.slots[0] = remaining;
            plan.slots[2] = result;
            plan.cook_progress = 0;
            plan.cook_total = recipe.cooking_ticks();
            plan.smelted = true;
        }
    } else {
        plan.cook_progress = 0;
    }
    validate_furnace_parts(
        &plan.slots,
        plan.burn_remaining,
        plan.burn_duration,
        plan.cook_progress,
        plan.cook_total,
    )?;
    Ok(plan)
}

/// Validate complete native furnace parts without cloning or allocating.
/// World data admission and the tick planner share these same bounds.
pub fn validate_furnace_parts(
    slots: &[ItemStack; 3],
    burn_remaining: u16,
    burn_duration: u16,
    cook_progress: u16,
    cook_total: u16,
) -> Result<(), SmeltingError> {
    if burn_remaining > burn_duration
        || burn_duration > 32767
        || cook_total == 0
        || cook_total > 32767
        || cook_progress >= cook_total
    {
        return Err(SmeltingError::InvalidTimers);
    }
    let mut bytes = FURNACE_FRAMING_BYTES;
    for stack in slots {
        bytes = bytes
            .checked_add(persisted_stack_size(stack)?)
            .ok_or(SmeltingError::BudgetExceeded)?;
        if bytes > MAX_FURNACE_BYTES {
            return Err(SmeltingError::BudgetExceeded);
        }
        validate_stack(stack)?;
    }
    Ok(())
}

fn validate_stack(stack: &ItemStack) -> Result<(), SmeltingError> {
    stack_capacity(stack)?;
    if stack
        .item()
        .is_some_and(|item| item.name() == "minecraft:air")
    {
        return Err(SmeltingError::PresentAir);
    }
    Ok(())
}
