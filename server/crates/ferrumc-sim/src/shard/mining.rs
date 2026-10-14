//! Tick-boundary mining; the application remains the legacy inventory owner.

use super::{
    block_change_output, try_encode_block_entity, within_reach, BlockEntity, BlockPos,
    BlockStateId, GameMode, GameOutput, MutationCause, MutationResult, PlayerId, SimShard, Vec3,
};
use crate::mining::{ordinary_drop, ore_experience};
use crate::MiningTool;

const MAX_MINING_ACTIONS: usize = 128;

#[derive(Debug, Clone, Copy)]
pub(super) struct MiningWork {
    position: BlockPos,
    previous_state: BlockStateId,
    sequence: i32,
    tool: MiningTool,
    elapsed: u32,
    delayed: bool,
    fresh: bool,
    stage: i8,
}

impl SimShard {
    pub(super) fn start_mining(
        &mut self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        position: BlockPos,
        sequence: i32,
        tool: MiningTool,
    ) {
        self.start_mining_inner(outputs, player, position, sequence, tool);
        self.release_unused_tool(outputs, player, tool.token());
    }

    fn start_mining_inner(
        &mut self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        position: BlockPos,
        sequence: i32,
        tool: MiningTool,
    ) {
        self.cancel_mining(outputs, player);
        let Some(actor) = self.players.get(&player) else {
            return;
        };
        if !self.chunks.is_loaded(position.to_chunk_pos()) {
            self.reject_mining(outputs, player, position, sequence);
            return;
        }
        if !actor.is_alive()
            || !within_reach(actor.position, position)
            || matches!(actor.game_mode, GameMode::Adventure | GameMode::Spectator)
        {
            self.reject_mining(outputs, player, position, sequence);
            return;
        }
        let previous_state = self.authoritative_state(position);
        if actor.game_mode == GameMode::Creative {
            let cause = MutationCause::PlayerCreative { player };
            let result = self.apply_block_edit(cause, position, BlockStateId::AIR);
            if let Some(output) =
                block_change_output(cause, sequence, position, BlockStateId::AIR, result)
            {
                outputs.push(output);
            }
            if !previous_state.is_air() && matches!(result, MutationResult::Applied { .. }) {
                self.update_fence_neighbors(outputs, position);
            }
            return;
        }
        if previous_state.is_air() {
            outputs.push(GameOutput::BlockActionAcknowledged { player, sequence });
            return;
        }
        let Some(profile) = tool.profile(previous_state) else {
            self.reject_mining(outputs, player, position, sequence);
            return;
        };
        if self.mining_work.len() >= MAX_MINING_ACTIONS {
            self.reject_mining(outputs, player, position, sequence);
            return;
        }
        let work = MiningWork {
            position,
            previous_state,
            sequence,
            tool,
            elapsed: 1,
            delayed: false,
            fresh: true,
            stage: ((profile.progress * 10.0) as i8).clamp(0, 9),
        };
        if profile.progress >= 1.0 {
            self.complete_mining(outputs, player, work);
        } else {
            outputs.push(GameOutput::MiningProgress {
                player,
                position,
                stage: work.stage,
            });
            outputs.push(GameOutput::BlockActionAcknowledged { player, sequence });
            self.mining_work.insert(player, work);
        }
    }

    pub(super) fn finish_mining(
        &mut self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        position: BlockPos,
        sequence: i32,
        tool: MiningTool,
    ) {
        self.finish_mining_inner(outputs, player, position, sequence, tool);
        self.release_unused_tool(outputs, player, tool.token());
    }

    fn finish_mining_inner(
        &mut self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        position: BlockPos,
        sequence: i32,
        tool: MiningTool,
    ) {
        let Some(mut work) = self.mining_work.remove(&player) else {
            self.reject_mining(outputs, player, position, sequence);
            return;
        };
        if work.position != position
            || work.tool.held_item() != tool.held_item()
            || work.tool.token() != tool.token()
            || !self.mining_is_valid(player, work)
        {
            outputs.push(GameOutput::MiningProgress {
                player,
                position: work.position,
                stage: -1,
            });
            self.reject_mining(outputs, player, position, sequence);
            self.release_unused_tool(outputs, player, work.tool.token());
            return;
        }
        work.sequence = sequence;
        work.tool = tool;
        let progress = tool
            .profile(work.previous_state)
            .map_or(0.0, |profile| profile.progress * work.elapsed as f32);
        if progress >= 0.7 {
            self.complete_mining(outputs, player, work);
        } else {
            work.delayed = true;
            self.mining_work.insert(player, work);
            outputs.push(GameOutput::BlockActionAcknowledged { player, sequence });
        }
    }

    pub(super) fn cancel_mining(&mut self, outputs: &mut Vec<GameOutput>, player: PlayerId) {
        if let Some(work) = self.mining_work.remove(&player) {
            outputs.push(GameOutput::MiningProgress {
                player,
                position: work.position,
                stage: -1,
            });
            self.release_unused_tool(outputs, player, work.tool.token());
        }
    }

    pub(super) fn update_mining_tool(
        &mut self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        tool: MiningTool,
    ) {
        let _ = tool;
        self.cancel_mining(outputs, player);
    }

    fn mining_is_valid(&self, player: PlayerId, work: MiningWork) -> bool {
        self.players.get(&player).is_some_and(|actor| {
            actor.is_alive()
                && actor.game_mode == GameMode::Survival
                && within_reach(actor.position, work.position)
        }) && self.chunks.is_loaded(work.position.to_chunk_pos())
            && self.authoritative_state(work.position) == work.previous_state
    }

    pub(super) fn drive_mining(&mut self, outputs: &mut Vec<GameOutput>) {
        // At most 128 copy-only keys; no scan of unrelated players/chunks.
        let players: Vec<_> = self.mining_work.keys().copied().collect();
        for player in players {
            let Some(mut work) = self.mining_work.remove(&player) else {
                continue;
            };
            if !self.mining_is_valid(player, work) {
                outputs.push(GameOutput::MiningProgress {
                    player,
                    position: work.position,
                    stage: -1,
                });
                self.reject_mining(outputs, player, work.position, work.sequence);
                self.release_unused_tool(outputs, player, work.tool.token());
                continue;
            }
            if work.fresh {
                work.fresh = false;
                self.mining_work.insert(player, work);
                continue;
            }
            work.elapsed = work.elapsed.saturating_add(1);
            let progress = work
                .tool
                .profile(work.previous_state)
                .map_or(0.0, |profile| profile.progress * work.elapsed as f32);
            if work.delayed && progress >= 1.0 {
                self.complete_mining(outputs, player, work);
                continue;
            }
            let stage = ((progress * 10.0) as i8).clamp(0, 9);
            if stage != work.stage {
                work.stage = stage;
                outputs.push(GameOutput::MiningProgress {
                    player,
                    position: work.position,
                    stage,
                });
            }
            self.mining_work.insert(player, work);
        }
    }

    fn reject_mining(
        &self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        position: BlockPos,
        sequence: i32,
    ) {
        outputs.push(GameOutput::BlockChangeRejected {
            player,
            position,
            sequence,
            requested_state: BlockStateId::AIR,
            authoritative_state: self.authoritative_state(position),
        });
    }

    fn release_unused_tool(&self, outputs: &mut Vec<GameOutput>, player: PlayerId, token: u64) {
        if token == 0
            || self
                .mining_work
                .get(&player)
                .is_some_and(|work| work.tool.token() == token)
        {
            return;
        }
        if outputs.iter().any(|output| match output {
            GameOutput::MiningReleased {
                player: actor,
                tool_token,
            }
            | GameOutput::SurvivalBlockBroken {
                player: actor,
                tool_token,
                ..
            } => *actor == player && *tool_token == token,
            _ => false,
        }) {
            return;
        }
        outputs.push(GameOutput::MiningReleased {
            player,
            tool_token: token,
        });
    }

    fn complete_mining(
        &mut self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        work: MiningWork,
    ) {
        self.complete_mining_inner(outputs, player, work);
        self.release_unused_tool(outputs, player, work.tool.token());
    }

    // All drop/chest preflights precede the single irreversible world edit.
    #[allow(clippy::too_many_lines)]
    fn complete_mining_inner(
        &mut self,
        outputs: &mut Vec<GameOutput>,
        player: PlayerId,
        work: MiningWork,
    ) {
        let Some(profile) = work.tool.profile(work.previous_state) else {
            self.reject_mining(outputs, player, work.position, work.sequence);
            return;
        };
        // Container contents must survive the block-entity cleanup. Complete
        // original codec admission precedes cloning and the ground reservation.
        let mut drops = Vec::new();
        if let Some(entity) = self
            .chunks
            .get(work.position.to_chunk_pos())
            .and_then(|chunk| chunk.block_entity(work.position))
        {
            let slots: &[ferrumc_items::ItemStack] = match entity {
                BlockEntity::Chest(chest) => chest.slots(),
                BlockEntity::Furnace(furnace) => furnace.slots(),
                _ => &[],
            };
            if !slots.is_empty() {
                if try_encode_block_entity(entity, &mut Vec::new()).is_err() {
                    self.reject_mining(outputs, player, work.position, work.sequence);
                    return;
                }
                drops.extend(slots.iter().filter(|stack| stack.item().is_some()).cloned());
            }
        }
        let mut random = self.mining_random;
        let experience = if profile.correct {
            if let Some(drop) = ordinary_drop(work.previous_state, &mut random) {
                drops.push(drop);
            }
            ore_experience(work.previous_state, &mut random)
        } else {
            0
        };
        if let Some(ground) = &self.ground_items {
            let owner = crate::ShardPartitioner::for_block(
                ground.owner.world(),
                ground.owner.dimension(),
                work.position,
            );
            if ground.check_region(owner).is_err() || ground.reserve_drop_bytes(&drops).is_err() {
                self.reject_mining(outputs, player, work.position, work.sequence);
                outputs.push(GameOutput::MiningProgress {
                    player,
                    position: work.position,
                    stage: -1,
                });
                return;
            }
        }
        let cause = MutationCause::PlayerCreative { player };
        let result = self.apply_block_edit(cause, work.position, BlockStateId::AIR);
        if let Some(output) = block_change_output(
            cause,
            work.sequence,
            work.position,
            BlockStateId::AIR,
            result,
        ) {
            outputs.push(output);
        }
        outputs.push(GameOutput::MiningProgress {
            player,
            position: work.position,
            stage: -1,
        });
        if matches!(result, MutationResult::Applied { .. }) {
            self.mining_random = random;
            if let Some(ground) = self.ground_items.as_mut() {
                let position = Vec3::new(
                    f64::from(work.position.x()) + 0.5,
                    f64::from(work.position.y()) + 0.5,
                    f64::from(work.position.z()) + 0.5,
                );
                for stack in drops.drain(..) {
                    let size = match ferrumc_items::persisted_stack_size(&stack) {
                        Ok(size) => size + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES,
                        Err(error) => panic!("checked ground drop failed: {error}"),
                    };
                    ground.admit_drop(
                        size,
                        crate::GroundItemDrop::new(
                            position,
                            stack,
                            10,
                            None,
                            Some(ferrumc_core::EntityUuid::from_uuid(player.as_uuid())),
                        ),
                    );
                }
            }
            outputs.push(GameOutput::SurvivalBlockBroken {
                player,
                position: work.position,
                sequence: work.sequence,
                previous_state: work.previous_state,
                held_item: work.tool.held_item(),
                tool_token: work.tool.token(),
                drops,
                experience,
                tool_damage: profile.tool_damage,
            });
            self.update_fence_neighbors(outputs, work.position);
        }
    }
}
