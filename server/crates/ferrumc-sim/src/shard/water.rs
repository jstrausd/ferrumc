//! Bounded ordinary water ticks committed by the actual shard edit owner.

use super::{block_change_output, within_reach, MutationCause, MutationResult, SimShard};
use crate::ground_items::MAX_GROUND_REGIONS;
use crate::water_runtime::WaterRuntime;
use crate::{
    BlockStateId, GameOutput, GroundItemDrop, WaterError, WaterRules, WaterTickQueue,
    WATER_TICK_DELAY,
};
use ferrumc_core::{GameMode, PlayerId};
use ferrumc_items::ItemStack;
use ferrumc_math::{BlockPos, Vec3};
use std::collections::BTreeSet;

impl SimShard {
    /// Selects bounded legacy water gameplay using pinned static collision facts.
    /// Existing natural sources wake through edits or explicit `WaterSchedule`
    /// inputs; no chunk is force-loaded. Native durable ownership is exclusive.
    pub fn enable_water(
        &mut self,
        capacity: usize,
        source_conversion: bool,
    ) -> Result<(), WaterError> {
        if self.player_ownership.is_some() {
            return Err(WaterError::OwnershipRequired);
        }
        if self.water.is_some() {
            return Err(WaterError::AlreadyInitialized);
        }
        let queue = WaterTickQueue::new(capacity)?;
        let rules = WaterRules::builtin()?;
        self.water = Some(WaterRuntime {
            live: true,
            world: self.chunks.world(),
            dimension: self.chunks.dimension(),
            now: 0,
            conversion: source_conversion,
            rules,
            queue,
        });
        Ok(())
    }

    /// Reads current unique scheduled cells, including due work retained for retry.
    pub fn water_queue_len(&self) -> Option<usize> {
        Some(self.water.as_ref()?.queue.len())
    }

    pub(super) fn bucket_use(
        &mut self,
        player: PlayerId,
        position: BlockPos,
        sequence: i32,
        fill: bool,
        token: u64,
        outputs: &mut Vec<GameOutput>,
    ) {
        let state = self.apply_bucket(player, position, sequence, fill, token, outputs);
        if state.is_none() {
            outputs.push(GameOutput::BlockChangeRejected {
                player,
                position,
                sequence,
                requested_state: BlockStateId::AIR,
                authoritative_state: self.authoritative_state(position),
            });
        }
        outputs.push(GameOutput::BucketUseResolved {
            player,
            position,
            sequence,
            fill,
            token,
            state,
        });
    }

    fn apply_bucket(
        &mut self,
        player: PlayerId,
        position: BlockPos,
        sequence: i32,
        fill: bool,
        token: u64,
        outputs: &mut Vec<GameOutput>,
    ) -> Option<BlockStateId> {
        let actor = self.players.get_mut(&player)?;
        if !actor.is_alive()
            || matches!(actor.game_mode, GameMode::Adventure | GameMode::Spectator)
            || !within_reach(actor.position, position)
        {
            return None;
        }
        let state = actor.survival.as_mut()?;
        if token == 0 || token <= state.last_bucket_token {
            return None;
        }
        state.last_bucket_token = token;
        let before = self
            .chunks
            .get(position.to_chunk_pos())?
            .get_block(position)?;
        let after = self
            .water
            .as_ref()?
            .rules
            .bucket_after(before, fill)
            .ok()??;
        let cause = MutationCause::PlayerCreative { player };
        let result = self.apply_block_edit(cause, position, after);
        if !matches!(result, MutationResult::Applied { .. }) {
            return None;
        }
        if let Some(output) = block_change_output(cause, sequence, position, after, result) {
            outputs.push(output);
        }
        self.update_fence_neighbors(outputs, position);
        Some(after)
    }

    pub(super) fn begin_water_tick(&mut self) {
        let Some(water) = self.water.as_mut() else {
            return;
        };
        assert!(water.live, "water snapshot has no tick authority");
        assert!(
            water.world == self.chunks.world() && water.dimension == self.chunks.dimension(),
            "water runtime scope changed before input drain"
        );
        water.now = water
            .now
            .checked_add(1)
            .unwrap_or_else(|| panic!("logical water tick overflow"));
    }

    pub(super) fn schedule_water(&mut self, position: BlockPos, outputs: &mut Vec<GameOutput>) {
        let Some(water) = self.water.as_mut() else {
            return;
        };
        let checked = (|| {
            let state = self
                .chunks
                .get(position.to_chunk_pos())
                .and_then(|chunk| chunk.get_block(position))
                .ok_or(WaterError::ChunkNotLoaded)?;
            if !water.rules.is_water(state)? {
                return Ok(());
            }
            let due = water
                .now
                .checked_add(WATER_TICK_DELAY)
                .ok_or(WaterError::TickOverflow)?;
            water.queue.schedule_at(due, position)
        })();
        if let Err(reason) = checked {
            outputs.push(GameOutput::WaterTickDeferred { position, reason });
        }
    }

    pub(super) fn prepare_water_changes(
        &mut self,
        changes: &[(BlockPos, BlockStateId)],
    ) -> Result<(), WaterError> {
        let Some(water) = self.water.as_mut() else {
            return Ok(());
        };
        if !water.live {
            return Err(WaterError::SnapshotOnly);
        }
        let mut positions = Vec::with_capacity(changes.len().min(6));
        for &(position, state) in changes {
            let before = self
                .chunks
                .get(position.to_chunk_pos())
                .and_then(|chunk| chunk.get_block(position))
                .ok_or(WaterError::ChunkNotLoaded)?;
            if before != state {
                positions.push(position);
            }
        }
        water
            .queue
            .schedule_after_changes(&water.rules, water.now, &positions, |position| {
                changes
                    .iter()
                    .rev()
                    .find(|(pos, _)| *pos == position)
                    .map(|(_, state)| *state)
                    .or_else(|| {
                        self.chunks
                            .get(position.to_chunk_pos())
                            .and_then(|chunk| chunk.get_block(position))
                    })
            })?;
        Ok(())
    }

    // Whole-plan state/drop/scheduling admission precedes its first world edit.
    #[allow(clippy::too_many_lines)]
    pub(super) fn drive_water(&mut self, outputs: &mut Vec<GameOutput>) {
        let Some(mut water) = self.water.take() else {
            return;
        };
        for _ in 0..64 {
            let position = match water.queue.pop_due(water.now, 1) {
                Ok(mut due) => match due.pop() {
                    Some(pos) => pos,
                    None => break,
                },
                Err(error) => panic!("fixed water drain refused: {error}"),
            };
            let result = (|| {
                let source = self
                    .chunks
                    .get(position.to_chunk_pos())
                    .and_then(|chunk| chunk.get_block(position))
                    .ok_or(WaterError::ChunkNotLoaded)?;
                if !water.rules.is_water(source)? {
                    return Ok(());
                }
                let plan = water.rules.plan_tick(position, water.conversion, |pos| {
                    self.chunks
                        .get(pos.to_chunk_pos())
                        .and_then(|chunk| chunk.get_block(pos))
                })?;
                if plan.writes().is_empty() {
                    return Ok(());
                }
                let mut changes = Vec::with_capacity(plan.writes().len());
                let mut drops = Vec::new();
                let mut random = self.mining_random;
                for write in plan.writes() {
                    let chunk = self
                        .chunks
                        .get(write.pos().to_chunk_pos())
                        .ok_or(WaterError::ChunkNotLoaded)?;
                    if chunk.get_block(write.pos()) != Some(write.before()) {
                        return Err(WaterError::UnknownState);
                    }
                    changes.push((write.pos(), write.after()));
                    let before_name = ferrumc_registry::block_state::state_id_to_block_name(
                        write.before().as_u32(),
                    );
                    let after_name = ferrumc_registry::block_state::state_id_to_block_name(
                        write.after().as_u32(),
                    );
                    if before_name != after_name
                        && !write.before().is_air()
                        && !water.rules.is_water(write.before())?
                    {
                        // Static waterlogged containers preserve their block data.
                        // Unsupported destructive block entities remain untouched.
                        if chunk.block_entity(write.pos()).is_some() {
                            return Err(WaterError::DropRefused);
                        }
                        if let Some(stack) =
                            crate::mining::ordinary_drop(write.before(), &mut random)
                        {
                            drops.push((write.pos(), stack));
                        }
                    }
                }
                self.preflight_water_drops(&drops)?;
                let positions: Vec<_> = changes.iter().map(|(pos, _)| *pos).collect();
                water
                    .queue
                    .schedule_after_changes(&water.rules, water.now, &positions, |pos| {
                        changes
                            .iter()
                            .rev()
                            .find(|(changed, _)| *changed == pos)
                            .map(|(_, state)| *state)
                            .or_else(|| {
                                self.chunks
                                    .get(pos.to_chunk_pos())
                                    .and_then(|chunk| chunk.get_block(pos))
                            })
                    })?;
                for &(pos, state) in &changes {
                    let result =
                        self.apply_block_edit_prepared(MutationCause::Fluid, pos, state, true);
                    assert!(
                        matches!(result, MutationResult::Applied { .. }),
                        "prevalidated water commit refused: {result:?}"
                    );
                    if let Some(output) =
                        block_change_output(MutationCause::Fluid, 0, pos, state, result)
                    {
                        outputs.push(output);
                    }
                }
                if let Some(ground) = self.ground_items.as_mut() {
                    for (pos, stack) in drops {
                        let size =
                            ferrumc_items::persisted_stack_size(&stack).unwrap_or_else(|error| {
                                panic!("prevalidated water drop failed: {error}")
                            }) + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES;
                        ground.admit_drop(
                            size,
                            GroundItemDrop::new(
                                Vec3::new(
                                    f64::from(pos.x()) + 0.5,
                                    f64::from(pos.y()) + 0.5,
                                    f64::from(pos.z()) + 0.5,
                                ),
                                stack,
                                10,
                                None,
                                None,
                            ),
                        );
                    }
                }
                self.mining_random = random;
                for (pos, _) in changes {
                    self.update_fence_neighbors(outputs, pos);
                }
                Ok(())
            })();
            if let Err(reason) = result {
                let retry = water
                    .now
                    .checked_add(WATER_TICK_DELAY)
                    .unwrap_or_else(|| panic!("water retry tick overflow"));
                if let Err(error) = water.queue.schedule_at(retry, position) {
                    panic!("released water due slot could not retain its refusal: {error}");
                }
                outputs.push(GameOutput::WaterTickDeferred { position, reason });
            }
        }
        self.water = Some(water);
    }

    fn preflight_water_drops(&self, drops: &[(BlockPos, ItemStack)]) -> Result<(), WaterError> {
        if drops.is_empty() {
            return Ok(());
        }
        let ground = self.ground_items.as_ref().ok_or(WaterError::DropRefused)?;
        let stacks: Vec<_> = drops.iter().map(|(_, stack)| stack.clone()).collect();
        ground
            .reserve_drop_bytes(&stacks)
            .map_err(|_| WaterError::DropRefused)?;
        let owners: BTreeSet<_> = drops
            .iter()
            .map(|(pos, _)| {
                crate::ShardPartitioner::for_block(
                    ground.owner.world(),
                    ground.owner.dimension(),
                    *pos,
                )
            })
            .collect();
        let added = owners
            .iter()
            .filter(|owner| !ground.region_claims.contains(owner))
            .count();
        if ground.region_claims.len() + added > MAX_GROUND_REGIONS {
            return Err(WaterError::DropRefused);
        }
        Ok(())
    }
}
