//! Selected-mainhand Q/CtrlQ split plans and actual owned-ground admission.
//! No GUI/cursor drop, client position/velocity or independent entity owner.

use crate::inventory::{PlayerInventory, HOTBAR_START, SLOT_COUNT};
use ferrumc_core::{EntityUuid, GameMode, PlayerId};
use ferrumc_items::{encode_persisted_stack, plan_split, ItemStack};
use ferrumc_sim::{EntityTransform, GroundItemDrop, SimShard};

pub(crate) struct HeldDropPlan {
    dropped: ItemStack,
    commit: HeldDropCommit,
}

pub(crate) struct HeldDropCommit {
    slot: usize,
    selected: u8,
    mode: GameMode,
    before: Vec<Vec<u8>>,
    after: ItemStack,
}

impl HeldDropPlan {
    /// Prepare only status 3 whole or status 4 one; retain complete FCIS beforeimages
    /// and admit the full projected-save postimage before any owner request.
    pub(crate) fn prepare(
        inventory: &PlayerInventory,
        status: i32,
    ) -> anyhow::Result<Option<Self>> {
        if !matches!(status, 3 | 4) || inventory.game_mode() == GameMode::Spectator {
            return Ok(None);
        }
        let slot = HOTBAR_START + usize::from(inventory.selected());
        let selected = inventory
            .slot(slot)
            .ok_or_else(|| anyhow::anyhow!("invalid held slot"))?;
        if selected.item().is_none() {
            return Ok(None);
        }
        let count = if status == 3 { selected.count() } else { 1 };
        let (after, dropped) = plan_split(selected, count)?.into_stacks();
        let image = stacks(inventory)?;
        crate::inventory_budget::admit(image.iter().enumerate().map(|(index, stack)| {
            if index == slot {
                &after
            } else {
                *stack
            }
        }))?;
        let mut bytes = 41 + 56 * 4 + 21;
        let mut before = Vec::with_capacity(SLOT_COUNT + 10);
        for stack in image {
            let frame = encode_persisted_stack(stack)?;
            bytes += frame.len();
            anyhow::ensure!(
                bytes <= ferrumc_storage::MAX_PLAYER_DATA_LEN,
                "held drop beforeimage exceeds player record budget"
            );
            before.push(frame);
        }
        Ok(Some(Self {
            dropped,
            commit: HeldDropCommit {
                slot,
                selected: inventory.selected(),
                mode: inventory.game_mode(),
                before,
                after,
            },
        }))
    }

    /// Move the proposed payload to the driver while keeping inventory evidence.
    pub(crate) fn into_parts(self) -> (ItemStack, HeldDropCommit) {
        (self.dropped, self.commit)
    }
}

impl HeldDropCommit {
    /// Exact comparison includes component/NBT order and float bits in all 56
    /// frames, selected index and mode; no partial item equality can authorize it.
    pub(crate) fn matches(&self, inventory: &PlayerInventory) -> anyhow::Result<bool> {
        let current = stacks(inventory)?;
        let mut matches =
            inventory.selected() == self.selected && inventory.game_mode() == self.mode;
        for (stack, before) in current.into_iter().zip(&self.before) {
            matches &= encode_persisted_stack(stack)? == *before;
        }
        Ok(matches)
    }

    /// Caller must hold the non-canceled serial transaction and have actual true
    /// driver acceptance. Mismatch is fatal/uncertain, never a denial or retry.
    pub(crate) fn commit(self, inventory: &mut PlayerInventory) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.matches(inventory)?,
            "held drop beforeimage changed after owner acceptance"
        );
        inventory.commit_held_drop(self.slot, self.after);
        Ok(())
    }
}

fn stacks(inventory: &PlayerInventory) -> anyhow::Result<Vec<&ItemStack>> {
    let mut image = Vec::with_capacity(SLOT_COUNT + 10);
    for index in 0..SLOT_COUNT {
        image.push(
            inventory
                .slot(index)
                .ok_or_else(|| anyhow::anyhow!("invalid player image"))?,
        );
    }
    image.push(inventory.carried());
    image.extend(inventory.table_inputs());
    Ok(image)
}

/// Sole driver checks the real actor and current resident position. The generic
/// owner reserves native/metadata/pending budgets before accepting this payload.
/// Existing ground physics uses feet position and velocity (0, 0.2, 0); original
/// eye-position/pitch-dependent horizontal throws are not supported here.
pub(crate) fn admit(shard: &mut SimShard, player: PlayerId, stack: ItemStack) -> bool {
    if shard
        .player_game_mode(player)
        .is_none_or(|mode| mode == GameMode::Spectator)
        || shard
            .player_vitals(player)
            .is_none_or(|vitals| !vitals.health().is_finite() || vitals.health() <= 0.0)
    {
        return false;
    }
    let Some(position) = shard.player_position(player) else {
        return false;
    };
    let Ok(transform) = EntityTransform::try_new(position, 0.0, 0.0) else {
        return false;
    };
    if shard.loaded_chunks().get(transform.chunk()).is_none() {
        return false;
    }
    shard
        .enqueue_ground_item_drop(GroundItemDrop::new(
            position,
            stack,
            40,
            None,
            Some(EntityUuid::from_uuid(player.as_uuid())),
        ))
        .is_ok()
}

#[cfg(test)]
mod tests;
