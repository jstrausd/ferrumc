use super::{FurnaceClickError, FurnaceClickPlan};
use crate::{
    exchange_slot, fuel_properties, plan_split, plan_transfer, same_stack_representation,
    smelting_recipe, stack_capacity, transfer_into_slots, ItemStack,
};

fn bucket(stack: &ItemStack) -> bool {
    stack
        .item()
        .is_some_and(|item| item.name() == "minecraft:bucket")
}

fn may_place(index: usize, stack: &ItemStack) -> Result<bool, FurnaceClickError> {
    Ok(match index {
        0 => true,
        1 => bucket(stack) || fuel_properties(stack)?.is_some(),
        _ => false,
    })
}

fn slot_capacity(index: usize, stack: &ItemStack) -> Result<u8, FurnaceClickError> {
    let capacity = stack_capacity(stack)?;
    Ok(if index == 1 && bucket(stack) {
        capacity.min(1)
    } else {
        capacity
    })
}

fn move_stack(
    source: &mut ItemStack,
    destination: &mut ItemStack,
    requested: u8,
    capacity: u8,
) -> Result<bool, FurnaceClickError> {
    // Old native inventories can contain multiple buckets. A click must not
    // silently clamp them just because the GUI insertion ceiling is one.
    if destination.count() >= capacity {
        return Ok(false);
    }
    let plan = plan_transfer(source, destination, requested, capacity)?;
    let changed = plan.moved() != 0;
    (*source, *destination) = plan.into_stacks();
    Ok(changed)
}

pub(super) fn pickup(
    plan: &mut FurnaceClickPlan,
    slot: usize,
    right: bool,
) -> Result<bool, FurnaceClickError> {
    if slot >= 3 {
        return Ok(exchange_slot(
            &mut plan.player[slot - 3],
            &mut plan.cursor,
            right,
        )?);
    }
    let target = &mut plan.slots[slot];
    if plan.cursor.item().is_none() {
        let requested = if right {
            target.count().div_ceil(2)
        } else {
            target.count()
        };
        let capacity = stack_capacity(target)?;
        return move_stack(target, &mut plan.cursor, requested, capacity);
    }
    if may_place(slot, &plan.cursor)? {
        let capacity = slot_capacity(slot, &plan.cursor)?;
        if target.item().is_some() && !same_stack_representation(target, &plan.cursor)? {
            if plan.cursor.count() <= capacity {
                std::mem::swap(target, &mut plan.cursor);
                return Ok(true);
            }
            return Ok(false);
        }
        let requested = if right { 1 } else { plan.cursor.count() };
        return move_stack(&mut plan.cursor, target, requested, capacity);
    }
    // Slot.tryRemove permits extraction into an existing cursor only when
    // enough room exists for all items in a slot whose contents cannot be placed.
    if target.item().is_none() || !same_stack_representation(target, &plan.cursor)? {
        return Ok(false);
    }
    let capacity = stack_capacity(&plan.cursor)?;
    let free = capacity - plan.cursor.count();
    if !may_place(slot, target)? && free < target.count() {
        return Ok(false);
    }
    let requested = target.count();
    move_stack(target, &mut plan.cursor, requested, capacity)
}

pub(super) fn shift(plan: &mut FurnaceClickPlan, slot: usize) -> Result<bool, FurnaceClickError> {
    if slot < 3 {
        return Ok(transfer_into_slots(
            &mut plan.slots[slot],
            &mut plan.player,
            slot == 2,
        )?);
    }
    let index = slot - 3;
    let source = &mut plan.player[index];
    if smelting_recipe(source)?.is_some() {
        let requested = source.count();
        let capacity = stack_capacity(source)?;
        return move_stack(source, &mut plan.slots[0], requested, capacity);
    }
    if fuel_properties(source)?.is_some() {
        let requested = source.count();
        let capacity = slot_capacity(1, source)?;
        return move_stack(source, &mut plan.slots[1], requested, capacity);
    }
    // Disjoint ranges keep the original source separate from every destination.
    let (main, hotbar) = plan.player.split_at_mut(27);
    if index < 27 {
        Ok(transfer_into_slots(&mut main[index], hotbar, false)?)
    } else {
        Ok(transfer_into_slots(&mut hotbar[index - 27], main, false)?)
    }
}

pub(super) fn hotbar(
    plan: &mut FurnaceClickPlan,
    slot: usize,
    button: usize,
) -> Result<bool, FurnaceClickError> {
    let index = 27 + button;
    if slot >= 3 {
        let target = slot - 3;
        if target == index || identical(&plan.player[target], &plan.player[index])? {
            return Ok(false);
        }
        plan.player.swap(target, index);
        return Ok(true);
    }
    let incoming = &mut plan.player[index];
    let target = &mut plan.slots[slot];
    if incoming.item().is_none() {
        if target.item().is_none() {
            return Ok(false);
        }
        std::mem::swap(target, incoming);
        return Ok(true);
    }
    if !may_place(slot, incoming)? {
        return Ok(false);
    }
    let capacity = slot_capacity(slot, incoming)?;
    if incoming.count() > capacity {
        if target.item().is_some() {
            return Err(FurnaceClickError::UnsupportedSwap);
        }
        let (remaining, received) = plan_split(incoming, capacity)?.into_stacks();
        *incoming = remaining;
        *target = received;
        return Ok(capacity != 0);
    }
    if identical(target, incoming)? {
        return Ok(false);
    }
    std::mem::swap(target, incoming);
    Ok(true)
}

fn identical(left: &ItemStack, right: &ItemStack) -> Result<bool, FurnaceClickError> {
    Ok(left.count() == right.count() && same_stack_representation(left, right)?)
}
