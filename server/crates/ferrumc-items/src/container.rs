//! Bounded inventory transfers for ordinary player and chest slots.
use crate::{
    persisted_stack_size, plan_transfer, ComponentValue, ItemPersistenceError, ItemStack,
    StackTransferError,
};

/// Refusal before a container plan can publish any partial mutation.
#[derive(Debug, thiserror::Error)]
pub enum ContainerTransferError {
    /// A slot array or action does not describe a single chest/player window.
    #[error("invalid container shape or action")]
    InvalidShape,
    /// The aggregate persisted representation exceeds one MiB.
    #[error("container stack budget exceeded")]
    BudgetExceeded,
    /// A stack violates lossless persistence validation.
    #[error(transparent)]
    InvalidStack(#[from] ItemPersistenceError),
    /// A transfer violates its component capacity or model bounds.
    #[error(transparent)]
    Transfer(#[from] StackTransferError),
    /// A max-stack component has unsupported opaque semantics.
    #[error("invalid max-stack component")]
    InvalidCapacity,
}

/// Resolves the component maximum, bounded by `FerrumC`'s registry ceiling.
///
/// Removing `max_stack_size` uses Minecraft's fallback of one. A trusted opaque
/// value must be exactly one legal `VarInt`; unsupported bytes refuse mutation.
pub fn stack_capacity(stack: &ItemStack) -> Result<u8, ContainerTransferError> {
    persisted_stack_size(stack)?;
    let Some(item) = stack.item() else {
        return Ok(0);
    };
    let mut capacity = item.max_stack();
    if stack
        .components()
        .removed()
        .iter()
        .any(|id| id.get() == crate::component::MAX_STACK_SIZE)
    {
        capacity = 1;
    }
    for value in stack.components().added() {
        match value {
            ComponentValue::MaxStackSize(value) => capacity = capacity.min(*value),
            ComponentValue::Opaque(value)
                if value.type_id() == crate::component::MAX_STACK_SIZE =>
            {
                let mut reader = ferrumc_codec::BoundedReader::new(value.raw());
                let decoded = reader
                    .read_var_int()
                    .map_err(|_| ContainerTransferError::InvalidCapacity)?;
                if !(1..=99).contains(&decoded) || reader.remaining() != 0 {
                    return Err(ContainerTransferError::InvalidCapacity);
                }
                capacity = capacity.min(decoded as u8);
            }
            _ => {}
        }
    }
    if stack.count() > capacity {
        return Err(ContainerTransferError::InvalidCapacity);
    }
    Ok(capacity)
}

/// Moves a source into slots: matching stacks first, then empty slots.
///
/// Callers must stage their complete operation before publication. Traversal is
/// reversed for the chest-to-player hotbar-first order used by Paper.
pub fn transfer_into_slots(
    source: &mut ItemStack,
    slots: &mut [ItemStack],
    reverse: bool,
) -> Result<bool, ContainerTransferError> {
    let mut changed = false;
    for empty in [false, true] {
        for offset in 0..slots.len() {
            let index = if reverse {
                slots.len() - 1 - offset
            } else {
                offset
            };
            if source.item().is_none() {
                return Ok(changed);
            }
            if slots[index].item().is_none() != empty {
                continue;
            }
            let capacity = if empty {
                stack_capacity(source)?
            } else {
                stack_capacity(&slots[index])?
            };
            let plan = plan_transfer(source, &slots[index], source.count(), capacity)?;
            changed |= plan.moved() != 0;
            let (remaining, destination) = plan.into_stacks();
            *source = remaining;
            slots[index] = destination;
        }
    }
    Ok(changed)
}

/// Plans a shift transfer or number-key swap in a 27+36-slot chest window.
///
/// The returned tuple is `(chest_after, player_after)`. No cursor participates.
/// All input models/capacities and the one-MiB aggregate are checked before any
/// clone. This pure plan has no revision, world or durable commit authority.
pub fn plan_chest_inventory_click(
    chest: &[ItemStack],
    player: &[ItemStack],
    slot: usize,
    hotbar: Option<u8>,
) -> Result<(Vec<ItemStack>, Vec<ItemStack>), ContainerTransferError> {
    if chest.len() != 27
        || player.len() != 36
        || slot >= 63
        || hotbar.is_some_and(|value| value >= 9)
    {
        return Err(ContainerTransferError::InvalidShape);
    }
    let mut bytes = 0usize;
    for stack in chest.iter().chain(player) {
        bytes = bytes
            .checked_add(persisted_stack_size(stack)?)
            .ok_or(ContainerTransferError::BudgetExceeded)?;
        if bytes > 1024 * 1024 {
            return Err(ContainerTransferError::BudgetExceeded);
        }
        stack_capacity(stack)?;
    }
    let mut chest = chest.to_vec();
    let mut player = player.to_vec();
    if let Some(hotbar) = hotbar {
        let target = 27 + usize::from(hotbar);
        if slot < 27 {
            std::mem::swap(&mut chest[slot], &mut player[target]);
        } else {
            player.swap(slot - 27, target);
        }
    } else if slot < 27 {
        transfer_into_slots(&mut chest[slot], &mut player, true)?;
    } else {
        transfer_into_slots(&mut player[slot - 27], &mut chest, false)?;
    }
    Ok((chest, player))
}

/// Complete bounded chest/player/cursor postimages for one drag operation.
/// This pure snapshot grants no owner, revision, persistence or commit authority.
pub struct ChestDragPlan {
    chest: Vec<ItemStack>,
    player: Vec<ItemStack>,
    cursor: ItemStack,
}

impl ChestDragPlan {
    /// Borrows the complete player-window postimage for consumer admission.
    pub fn player(&self) -> &[ItemStack] {
        &self.player
    }

    /// Borrows the carried-stack postimage for consumer admission.
    pub fn cursor(&self) -> &ItemStack {
        &self.cursor
    }

    /// Transfers all three postimages to the exclusive consumer together.
    pub fn into_parts(self) -> (Vec<ItemStack>, Vec<ItemStack>, ItemStack) {
        (self.chest, self.player, self.cursor)
    }
}

/// Plans ordinary left/right drag distribution in a single 27+36 chest screen.
///
/// Selection contains at most 63 unique valid window indices. Right drag places
/// one per selected compatible slot; left drag distributes the original cursor
/// count evenly. Full or incompatible slots retain their contents and share.
/// All models/caps and the one-MiB complete encoded input/output budgets pass before any
/// clone. A complete staged postimage preserves exact components and quantities.
pub fn plan_chest_drag(
    chest: &[ItemStack],
    player: &[ItemStack],
    cursor: &ItemStack,
    selection: &[usize],
    right: bool,
) -> Result<ChestDragPlan, ContainerTransferError> {
    if chest.len() != 27 || player.len() != 36 || selection.len() > 63 {
        return Err(ContainerTransferError::InvalidShape);
    }
    let mut selected = [false; 63];
    for &index in selection {
        let Some(previous) = selected.get_mut(index) else {
            return Err(ContainerTransferError::InvalidShape);
        };
        if *previous {
            return Err(ContainerTransferError::InvalidShape);
        }
        *previous = true;
    }
    let mut bytes = 0usize;
    for stack in chest.iter().chain(player).chain(std::iter::once(cursor)) {
        bytes = bytes
            .checked_add(persisted_stack_size(stack)?)
            .ok_or(ContainerTransferError::BudgetExceeded)?;
        if bytes > 1024 * 1024 {
            return Err(ContainerTransferError::BudgetExceeded);
        }
        stack_capacity(stack)?;
    }
    let per_slot = if right {
        1
    } else if selection.is_empty() {
        0
    } else {
        cursor.count() / selection.len() as u8
    };
    // Splitting into empty slots repeats the entire component representation.
    // Forecast the complete postimage without allocating those copies first.
    let cursor_bytes = persisted_stack_size(cursor)?;
    let mut remaining = cursor.count();
    for &index in selection {
        let destination = if index < 27 {
            &chest[index]
        } else {
            &player[index - 27]
        };
        if remaining == 0
            || (destination.item().is_some()
                && !crate::same_stack_representation(cursor, destination)?)
        {
            continue;
        }
        let capacity = if destination.item().is_none() {
            stack_capacity(cursor)?
        } else {
            stack_capacity(destination)?
        };
        let moved = per_slot.min(remaining).min(capacity - destination.count());
        if moved != 0 && destination.item().is_none() {
            bytes = bytes
                .checked_add(cursor_bytes - persisted_stack_size(destination)?)
                .ok_or(ContainerTransferError::BudgetExceeded)?;
        }
        remaining -= moved;
    }
    if cursor.item().is_some() && remaining == 0 {
        bytes -= cursor_bytes - persisted_stack_size(&ItemStack::empty())?;
    }
    if bytes > 1024 * 1024 {
        return Err(ContainerTransferError::BudgetExceeded);
    }
    let mut plan = ChestDragPlan {
        chest: chest.to_vec(),
        player: player.to_vec(),
        cursor: cursor.clone(),
    };
    for &index in selection {
        let destination = if index < 27 {
            &mut plan.chest[index]
        } else {
            &mut plan.player[index - 27]
        };
        let capacity = if destination.item().is_none() {
            stack_capacity(cursor)?
        } else {
            stack_capacity(destination)?
        };
        let transfer = plan_transfer(&plan.cursor, destination, per_slot, capacity)?;
        let (remaining, after) = transfer.into_stacks();
        plan.cursor = remaining;
        *destination = after;
    }
    Ok(plan)
}

/// Plans an ordinary left/right slot exchange using exact components and caps.
/// Both postimages publish together; refused or malformed models stay unchanged.
pub fn exchange_slot(
    slot: &mut ItemStack,
    cursor: &mut ItemStack,
    right: bool,
) -> Result<bool, ContainerTransferError> {
    stack_capacity(slot)?;
    stack_capacity(cursor)?;
    let (source, destination, pick_up) = if cursor.item().is_none() {
        (&*slot, &*cursor, true)
    } else {
        (&*cursor, &*slot, false)
    };
    if source.item().is_none() {
        return Ok(false);
    }
    if destination.item().is_some() && !crate::same_stack_representation(source, destination)? {
        std::mem::swap(slot, cursor);
        return Ok(true);
    }
    let requested = if right {
        if pick_up {
            source.count().div_ceil(2)
        } else {
            1
        }
    } else {
        source.count()
    };
    let capacity = if destination.item().is_none() {
        stack_capacity(source)?
    } else {
        stack_capacity(destination)?
    };
    let plan = plan_transfer(source, destination, requested, capacity)?;
    if plan.moved() == 0 {
        return Ok(false);
    }
    let (remaining, received) = plan.into_stacks();
    if pick_up {
        *slot = remaining;
        *cursor = received;
    } else {
        *cursor = remaining;
        *slot = received;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentPatch, ItemId};
    fn stack(count: u8, limit: u8) -> ItemStack {
        ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            count,
            ComponentPatch::new(vec![ComponentValue::MaxStackSize(limit)], vec![]),
        )
        .unwrap()
    }
    #[test]
    fn chest_drag_preserves_component_caps_and_every_postimage() {
        let mut chest = vec![ItemStack::empty(); 27];
        let player = vec![ItemStack::empty(); 36];
        chest[0] = stack(15, 16);
        chest[1] = stack(16, 16);
        chest[2] = stack(1, 8); // incompatible exact max-stack component
        let cursor = stack(10, 16);
        let plan = plan_chest_drag(&chest, &player, &cursor, &[0, 1, 2, 3, 27], true).unwrap();
        let (after, player_after, carried) = plan.into_parts();
        assert_eq!(after[0], stack(16, 16));
        assert_eq!(after[1], chest[1]);
        assert_eq!(after[2], chest[2]);
        assert_eq!(after[3], stack(1, 16));
        assert_eq!(player_after[0], stack(1, 16));
        assert_eq!(carried, stack(7, 16));
        let (after, player_after, carried) =
            plan_chest_drag(&chest, &player, &cursor, &[0, 3, 27], false)
                .unwrap()
                .into_parts();
        assert_eq!(after[0], stack(16, 16)); // clipped share remains carried
        assert_eq!(after[3], stack(3, 16));
        assert_eq!(player_after[0], stack(3, 16));
        assert_eq!(carried, stack(3, 16));
        assert_eq!(chest[0], stack(15, 16));
        for selection in [vec![0, 0], vec![63], vec![0; 64]] {
            assert!(plan_chest_drag(&chest, &player, &cursor, &selection, true).is_err());
        }
        assert!(plan_chest_drag(&chest[..26], &player, &cursor, &[0], true).is_err());
    }

    #[test]
    fn chest_drag_component_replication_exceeds_budget_before_any_output_clone() {
        let components = [6, 7, 8, 9]
            .into_iter()
            .map(|id| {
                crate::ComponentValue::Opaque(
                    crate::OpaqueComponent::new(id, vec![0; 8192]).unwrap(),
                )
            })
            .collect();
        let cursor = ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            63,
            ComponentPatch::new(components, vec![]),
        )
        .unwrap();
        let chest = vec![ItemStack::empty(); 27];
        let player = vec![ItemStack::empty(); 36];
        let indices: Vec<_> = (0..63).collect();
        assert!(matches!(
            plan_chest_drag(&chest, &player, &cursor, &indices, true),
            Err(ContainerTransferError::BudgetExceeded)
        ));
        assert_eq!(cursor.count(), 63);
        assert!(chest
            .iter()
            .chain(&player)
            .all(|stack| stack.item().is_none()));
    }

    #[test]
    fn shift_moves_both_directions_with_component_limits_and_reverse_order() {
        let mut chest = vec![ItemStack::empty(); 27];
        let mut player = vec![ItemStack::empty(); 36];
        chest[0] = stack(16, 16);
        player[35] = stack(12, 16);
        let (after_chest, after_player) =
            plan_chest_inventory_click(&chest, &player, 0, None).unwrap();
        assert!(after_chest[0].item().is_none());
        assert_eq!(after_player[35].count(), 16);
        assert_eq!(after_player[34].count(), 12);
        let (after_chest, after_player) =
            plan_chest_inventory_click(&after_chest, &after_player, 61, None).unwrap();
        assert_eq!(after_chest[0], stack(12, 16));
        assert!(after_player[34].item().is_none());
        assert_eq!(chest[0], stack(16, 16));
    }
    #[test]
    fn full_and_incompatible_destinations_conserve_source() {
        let chest = vec![stack(16, 16); 27];
        let player = vec![stack(16, 16); 36];
        let after = plan_chest_inventory_click(&chest, &player, 27, None).unwrap();
        assert_eq!(after, (chest.clone(), player.clone()));
        assert!(plan_chest_inventory_click(&chest, &player, 63, None).is_err());
        assert!(plan_chest_inventory_click(&chest, &player, 0, Some(9)).is_err());
    }
    #[test]
    fn number_swap_preserves_components_and_limits() {
        let mut chest = vec![ItemStack::empty(); 27];
        let mut player = vec![ItemStack::empty(); 36];
        chest[0] = stack(3, 16);
        player[27] = stack(7, 8);
        let (chest, player) = plan_chest_inventory_click(&chest, &player, 0, Some(0)).unwrap();
        assert_eq!(chest[0], stack(7, 8));
        assert_eq!(player[27], stack(3, 16));
    }
}
