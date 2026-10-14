//! Ordinary-furnace ticking and exact checked menu exchange at the shard owner.

use super::{block_change_output, within_reach, MutationResult, SimShard};
use crate::furnace_runtime::FurnaceRuntime;
use crate::{FurnaceError, GameOutput, MutationCause};
use ferrumc_core::{GameMode, PlayerId};
use ferrumc_items::{
    encode_persisted_stack, plan_furnace_click, plan_furnace_tick, same_stack_representation,
    ItemStack,
};
use ferrumc_math::BlockPos;
use ferrumc_world::{
    is_furnace_state, try_encode_block_entity, BlockEntity, BlockStateId, FurnaceInventory,
};
use std::collections::BTreeSet;

impl SimShard {
    /// Selects once-only legacy furnace ticking, exclusive with native ownership.
    /// At most 1024 resident ordinary furnaces may be registered; discovery uses
    /// 64 block entities and four resident columns per logical tick. World remains
    /// the sole slot/timer owner. No unloaded furnace advances or force-loads.
    pub fn enable_furnaces(&mut self, capacity: usize) -> Result<(), FurnaceError> {
        if capacity == 0 || capacity > 1024 {
            return Err(FurnaceError::InvalidCapacity);
        }
        if self.player_ownership.is_some() {
            return Err(FurnaceError::OwnershipRequired);
        }
        if self.furnaces.is_some() {
            return Err(FurnaceError::AlreadyInitialized);
        }
        self.furnaces = Some(FurnaceRuntime {
            live: true,
            world: self.chunks.world(),
            dimension: self.chunks.dimension(),
            capacity,
            registered: BTreeSet::new(),
            scan_chunk: None,
            scan_block: None,
        });
        Ok(())
    }

    /// Reads genuine reachable ordinary-furnace data for an admitted living actor.
    /// The caller owns screen/inventory pinning; this snapshot grants no mutation.
    pub fn furnace_snapshot(
        &self,
        player: PlayerId,
        position: BlockPos,
    ) -> Option<FurnaceInventory> {
        if self.player_ownership.is_some() {
            return None;
        }
        let actor = self.players.get(&player)?;
        if !actor.is_alive()
            || actor.game_mode == GameMode::Spectator
            || !within_reach(actor.position, position)
        {
            return None;
        }
        let chunk = self.chunks.get(position.to_chunk_pos())?;
        if !is_furnace_state(chunk.get_block(position)?.as_u32()) {
            return None;
        }
        let entity @ BlockEntity::Furnace(furnace) = chunk.block_entity(position)? else {
            return None;
        };
        try_encode_block_entity(entity, &mut Vec::new()).ok()?;
        Some(furnace.clone())
    }

    /// Stages all three furnace slots, 36 player slots and cursor together.
    /// Expected slots compare exact lossless FCIS bytes inside this owner; timers
    /// are excluded. Stale returns actual furnace with unchanged player/cursor.
    /// Root must preflight and freeze its complete `PlayerData` before calling and
    /// commit exactly the returned player/cursor postimage only after success.
    #[allow(clippy::too_many_arguments)]
    pub fn furnace_inventory_click_checked(
        &mut self,
        player: PlayerId,
        position: BlockPos,
        expected: &[ItemStack; 3],
        player_slots: [ItemStack; 36],
        cursor: ItemStack,
        slot: usize,
        button: i8,
        mode: i32,
    ) -> Option<(FurnaceInventory, [ItemStack; 36], ItemStack)> {
        let current = self.furnace_snapshot(player, position)?;
        for (actual, expected) in current.slots().iter().zip(expected) {
            if encode_persisted_stack(actual).ok()? != encode_persisted_stack(expected).ok()? {
                return Some((current, player_slots, cursor));
            }
        }
        let plan =
            plan_furnace_click(current.slots(), &player_slots, &cursor, slot, button, mode).ok()?;
        let (slots, player_after, cursor_after) = plan.into_parts();
        let same_input = same_stack_representation(&slots[0], &current.slots()[0]).ok()?;
        let progress = if same_input {
            current.cook_progress()
        } else {
            0
        };
        let after = FurnaceInventory::try_from_parts(
            slots,
            current.burn_remaining(),
            current.burn_duration(),
            progress,
            current.cook_total(),
        )
        .ok()?;
        let before = BlockEntity::Furnace(current);
        let candidate = BlockEntity::Furnace(after.clone());
        let mut old_bytes = Vec::new();
        let mut new_bytes = Vec::new();
        try_encode_block_entity(&before, &mut old_bytes).ok()?;
        try_encode_block_entity(&candidate, &mut new_bytes).ok()?;
        if old_bytes != new_bytes {
            let chunk = self.chunks.get_mut(position.to_chunk_pos())?;
            *chunk.block_entity_mut(position)? = candidate;
            chunk.mark_persist_dirty(position);
        }
        Some((after, player_after, cursor_after))
    }

    pub(super) fn check_furnace_owner(&self) {
        if let Some(runtime) = &self.furnaces {
            assert!(runtime.live, "furnace snapshot has no tick authority");
            assert!(
                runtime.world == self.chunks.world()
                    && runtime.dimension == self.chunks.dimension(),
                "furnace scope changed before input drain"
            );
        }
    }

    pub(super) fn preflight_furnace_block(&self, position: BlockPos, state: BlockStateId) -> bool {
        if !is_furnace_state(state.as_u32()) {
            return true;
        }
        let Some(chunk) = self.chunks.get(position.to_chunk_pos()) else {
            return false;
        };
        if chunk.block_entity(position).is_none()
            && chunk.block_entity_count() >= ferrumc_world::MAX_BLOCK_ENTITIES
        {
            return false;
        }
        self.furnaces.as_ref().is_none_or(|runtime| {
            runtime.registered.contains(&position) || runtime.registered.len() < runtime.capacity
        })
    }

    pub(super) fn reconcile_furnace_registration(
        &mut self,
        position: BlockPos,
        state: BlockStateId,
    ) {
        if let Some(runtime) = self.furnaces.as_mut() {
            if is_furnace_state(state.as_u32()) {
                assert!(
                    runtime.registered.contains(&position)
                        || runtime.registered.len() < runtime.capacity,
                    "furnace placement escaped preflight"
                );
                runtime.registered.insert(position);
            } else {
                runtime.registered.remove(&position);
            }
        }
    }

    pub(super) fn drive_furnaces(&mut self, outputs: &mut Vec<GameOutput>) {
        let Some(mut runtime) = self.furnaces.take() else {
            return;
        };
        runtime.registered.retain(|pos| {
            self.chunks.get(pos.to_chunk_pos()).is_some_and(|chunk| {
                chunk
                    .get_block(*pos)
                    .is_some_and(|state| is_furnace_state(state.as_u32()))
                    && matches!(chunk.block_entity(*pos), Some(BlockEntity::Furnace(_)))
            })
        });
        self.discover_furnaces(&mut runtime, outputs);
        for &position in &runtime.registered {
            if let Err(reason) = self.tick_furnace(position, outputs) {
                outputs.push(GameOutput::FurnaceTickDeferred { position, reason });
            }
        }
        self.furnaces = Some(runtime);
    }

    fn discover_furnaces(&self, runtime: &mut FurnaceRuntime, outputs: &mut Vec<GameOutput>) {
        let mut remaining = 64;
        for _ in 0..4 {
            let column = match runtime.scan_chunk {
                Some(pos) if runtime.scan_block.is_some() && self.chunks.is_loaded(pos) => {
                    Some(pos)
                }
                _ => self
                    .chunks
                    .next_loaded_position(runtime.scan_chunk)
                    .or_else(|| self.chunks.next_loaded_position(None)),
            };
            let Some(column) = column else {
                break;
            };
            if runtime.scan_chunk != Some(column) {
                runtime.scan_block = None;
            }
            runtime.scan_chunk = Some(column);
            let Some(chunk) = self.chunks.get(column) else {
                break;
            };
            let mut entries = chunk
                .block_entities()
                .filter(|(pos, _)| runtime.scan_block.is_none_or(|last| *pos > last));
            let mut consumed = 0;
            for (position, entity) in entries.by_ref().take(remaining) {
                consumed += 1;
                if !matches!(entity, BlockEntity::Furnace(_))
                    || !chunk
                        .get_block(position)
                        .is_some_and(|state| is_furnace_state(state.as_u32()))
                {
                    continue;
                }
                if runtime.registered.contains(&position) {
                    continue;
                }
                if runtime.registered.len() == runtime.capacity {
                    outputs.push(GameOutput::FurnaceTickDeferred {
                        position,
                        reason: FurnaceError::Capacity,
                    });
                } else {
                    runtime.registered.insert(position);
                }
            }
            remaining -= consumed;
            // At most 4096 block entities exist in a column; filtered traversal is
            // therefore bounded even while discovery resumes an imported column.
            if remaining == 0 {
                runtime.scan_block = chunk
                    .block_entities()
                    .filter(|(pos, _)| runtime.scan_block.is_none_or(|last| *pos > last))
                    .nth(consumed - 1)
                    .map(|(pos, _)| pos);
                break;
            }
            runtime.scan_block = None;
        }
    }

    fn tick_furnace(
        &mut self,
        position: BlockPos,
        outputs: &mut Vec<GameOutput>,
    ) -> Result<(), FurnaceError> {
        let chunk = self
            .chunks
            .get(position.to_chunk_pos())
            .ok_or(FurnaceError::BlockRefused)?;
        let state = chunk
            .get_block(position)
            .ok_or(FurnaceError::BlockRefused)?;
        let entity @ BlockEntity::Furnace(current) = chunk
            .block_entity(position)
            .ok_or(FurnaceError::InvalidData)?
        else {
            return Err(FurnaceError::InvalidData);
        };
        if current.burn_remaining() == 0
            && current.cook_progress() == 0
            && (current.slots()[0].item().is_none() || current.slots()[1].item().is_none())
        {
            return Ok(());
        }
        let mut before = Vec::new();
        try_encode_block_entity(entity, &mut before).map_err(|_| FurnaceError::InvalidData)?;
        let plan = plan_furnace_tick(
            current.slots(),
            current.burn_remaining(),
            current.burn_duration(),
            current.cook_progress(),
            current.cook_total(),
        )
        .map_err(|_| FurnaceError::InvalidData)?;
        let candidate = FurnaceInventory::try_from_parts(
            plan.slots().clone(),
            plan.burn_remaining(),
            plan.burn_duration(),
            plan.cook_progress(),
            plan.cook_total(),
        )
        .map_err(|_| FurnaceError::InvalidData)?;
        let lit_state =
            furnace_lit_state(state, plan.is_lit()).ok_or(FurnaceError::BlockRefused)?;
        let candidate = BlockEntity::Furnace(candidate);
        let mut after = Vec::new();
        try_encode_block_entity(&candidate, &mut after).map_err(|_| FurnaceError::InvalidData)?;
        if state != lit_state {
            let result = self.apply_block_edit(MutationCause::Furnace, position, lit_state);
            if !matches!(result, MutationResult::Applied { .. }) {
                return Err(FurnaceError::BlockRefused);
            }
            if let Some(output) =
                block_change_output(MutationCause::Furnace, 0, position, lit_state, result)
            {
                outputs.push(output);
            }
        }
        if before != after {
            let chunk = self
                .chunks
                .get_mut(position.to_chunk_pos())
                .ok_or(FurnaceError::BlockRefused)?;
            *chunk
                .block_entity_mut(position)
                .ok_or(FurnaceError::BlockRefused)? = candidate;
            chunk.mark_persist_dirty(position);
        }
        Ok(())
    }
}

fn furnace_lit_state(state: BlockStateId, lit: bool) -> Option<BlockStateId> {
    let block = ferrumc_registry::block_state::block_metadata("furnace")?;
    let mut properties = std::collections::BTreeMap::new();
    for property in block.properties {
        let value = if property.name == "lit" {
            if lit {
                "true"
            } else {
                "false"
            }
        } else {
            crate::mining::property_value(block, state.as_u32(), property.name)?
        };
        properties.insert(property.name, value);
    }
    Some(BlockStateId::new(
        ferrumc_registry::block_state::compute_state_id("furnace", &properties)?,
    ))
}
