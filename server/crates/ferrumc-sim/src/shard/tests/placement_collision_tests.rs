//! Real authoritative placement regressions, including FIFO movement and exact replacements.
use super::*;
use ferrumc_registry::block_state::block_default_state;

async fn owner(feet: Vec3) -> (SimShard, PlayerId) {
    let mut shard = shard_with_loaded_chunk(ChunkPos::ORIGIN).await;
    let actor = player("collision-actor");
    shard
        .enqueue(GameInput::PlayerJoin {
            player: actor,
            position: feet,
        })
        .unwrap();
    shard.run_tick();
    (shard, actor)
}

fn unchanged(shard: &SimShard, before: &ferrumc_world::Chunk) {
    assert_eq!(shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap(), before);
    assert!(
        !shard.has_pending_mutations(),
        "refusal has no journal mutation"
    );
}

fn rejected(outputs: &[GameOutput], actor: PlayerId, position: BlockPos, sequence: i32) {
    assert!(outputs.iter().any(|output| matches!(output,
        GameOutput::BlockChangeRejected { player, position: target, sequence: actual, authoritative_state, .. }
        if *player == actor && *target == position && *actual == sequence && authoritative_state.is_air()
    )), "refusal carries authoritative air correction and ACK sequence: {outputs:?}");
    assert!(
        !outputs
            .iter()
            .any(|output| matches!(output, GameOutput::BlockChanged { .. })),
        "refusal never broadcasts accepted placement"
    );
}

#[tokio::test]
async fn stone_at_feet_or_head_refuses_before_any_dirty_or_journal_change() {
    for y in [64, 65] {
        let (mut shard, actor) = owner(Vec3::new(8.5, 64.0, 8.5)).await;
        let target = BlockPos::new(8, y, 8);
        let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
        let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 17);
        rejected(&outputs, actor, target, 17);
        unchanged(&shard, &before);
    }
}

#[tokio::test]
async fn touching_player_face_is_clear_and_stone_is_accepted() {
    let feet = Vec3::new(9.0 + f64::from(0.6_f32 / 2.0), 64.0, 8.5);
    let (mut shard, actor) = owner(feet).await;
    let target = BlockPos::new(8, 64, 8);
    let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 18);
    assert!(outputs.iter().any(
        |output| matches!(output, GameOutput::BlockChanged { position, .. } if *position == target)
    ));
    assert_eq!(block_at(&shard, target), Some(BlockStateId::new(1)));
}

#[tokio::test]
async fn another_player_obstructs_but_a_spectator_does_not() {
    for spectator in [false, true] {
        let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
        let other = player("collision-other");
        shard
            .enqueue(GameInput::PlayerJoin {
                player: other,
                position: Vec3::new(8.5, 64.0, 8.5),
            })
            .unwrap();
        if spectator {
            shard
                .enqueue(GameInput::SetGameMode {
                    player: other,
                    mode: GameMode::Spectator,
                })
                .unwrap();
        }
        shard.run_tick();
        let target = BlockPos::new(8, 64, 8);
        let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
        let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 19);
        if spectator {
            assert_eq!(block_at(&shard, target), Some(BlockStateId::new(1)));
        } else {
            rejected(&outputs, actor, target, 19);
            unchanged(&shard, &before);
        }
    }
}

#[tokio::test]
async fn spectator_actor_cannot_place_even_in_an_unobstructed_cell() {
    let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    shard
        .enqueue(GameInput::SetGameMode {
            player: actor,
            mode: GameMode::Spectator,
        })
        .unwrap();
    let target = BlockPos::new(8, 64, 8);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 20);
    rejected(&outputs, actor, target, 20);
    unchanged(&shard, &before);
}

#[tokio::test]
async fn torch_and_grass_inside_body_use_empty_collision_shapes() {
    for name in ["torch", "short_grass"] {
        let (mut shard, actor) = owner(Vec3::new(8.5, 64.0, 8.5)).await;
        let target = BlockPos::new(8, 64, 8);
        let state = block_default_state(name).unwrap();
        let outputs = place_block(
            &mut shard,
            actor,
            target,
            state,
            Direction::Up,
            0.5,
            0.0,
            21,
        );
        assert!(outputs.iter().any(|output| matches!(output, GameOutput::BlockChanged { position, .. } if *position == target)), "{name}");
        assert_eq!(block_at(&shard, target), Some(BlockStateId::new(state)));
    }
}

#[tokio::test]
async fn slab_refinement_checks_actual_top_or_bottom_shape() {
    for (face, cursor, accepted) in [(Direction::Up, 0.5, true), (Direction::North, 0.8, false)] {
        let (mut shard, actor) = owner(Vec3::new(8.5, 64.5, 8.5)).await;
        let target = BlockPos::new(8, 64, 8);
        let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
        let outputs = place_block(&mut shard, actor, target, OAK_SLAB, face, cursor, 0.0, 22);
        if accepted {
            assert_eq!(block_at(&shard, target), Some(BlockStateId::new(OAK_SLAB)));
        } else {
            rejected(&outputs, actor, target, 22);
            unchanged(&shard, &before);
        }
    }
}

#[tokio::test]
async fn slab_merge_checks_relocated_double_before_replacing_existing_half() {
    let (mut shard, actor) = owner(Vec3::new(8.5, 64.5, 8.5)).await;
    let clicked = BlockPos::new(8, 64, 8);
    let stepped = BlockPos::new(8, 65, 8);
    set_world_block(&mut shard, clicked, OAK_SLAB);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    let outputs = place_block(
        &mut shard,
        actor,
        stepped,
        OAK_SLAB,
        Direction::Up,
        0.5,
        0.0,
        23,
    );
    assert!(outputs.contains(&GameOutput::BlockChangeRejected {
        player: actor,
        position: clicked,
        sequence: 23,
        requested_state: BlockStateId::new(OAK_SLAB),
        authoritative_state: BlockStateId::new(OAK_SLAB)
    }));
    unchanged(&shard, &before);
}

#[tokio::test]
async fn bed_head_collision_refuses_both_cells_and_corrects_both_predictions() {
    let (mut shard, actor) = owner(Vec3::new(8.5, 65.0, 9.5)).await;
    let foot = BlockPos::new(8, 65, 8);
    let head = BlockPos::new(8, 65, 9);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    let outputs = place_block(
        &mut shard,
        actor,
        foot,
        WHITE_BED,
        Direction::Up,
        0.5,
        0.0,
        24,
    );
    rejected(&outputs, actor, foot, 24);
    rejected(&outputs, actor, head, 24);
    unchanged(&shard, &before);
}

#[tokio::test]
async fn exact_plugin_replacement_uses_final_shape_but_nonplayer_force_edits_remain_exact() {
    let (mut shard, actor) = owner(Vec3::new(8.5, 64.0, 8.5)).await;
    let target = BlockPos::new(8, 64, 8);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    shard
        .enqueue(GameInput::SetBlockExact {
            player: actor,
            position: target,
            sequence: 25,
            state: BlockStateId::new(1),
        })
        .unwrap();
    let outputs = shard.run_tick();
    rejected(&outputs, actor, target, 25);
    unchanged(&shard, &before);
    shard
        .enqueue(GameInput::SetBlockExact {
            player: actor,
            position: target,
            sequence: 26,
            state: BlockStateId::new(TORCH),
        })
        .unwrap();
    assert!(shard
        .run_tick()
        .iter()
        .any(|output| matches!(output, GameOutput::BlockChanged { .. })));
    assert!(matches!(
        shard.apply_block_edit(MutationCause::Command, target, BlockStateId::new(1)),
        MutationResult::Applied { .. }
    ));
}

#[tokio::test]
async fn earlier_valid_move_is_used_without_applying_a_later_move_early() {
    for move_first in [true, false] {
        let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
        let target = BlockPos::new(8, 64, 8);
        let movement = GameInput::PlayerMove {
            player: actor,
            position: Some(Vec3::new(8.5, 64.0, 8.5)),
            yaw: None,
            pitch: None,
        };
        let placement = GameInput::BlockPlace {
            player: actor,
            position: target,
            sequence: 27,
            state: BlockStateId::new(1),
            clicked_face: Direction::Up,
            cursor_position: Vec3::new(0.5, 0.5, 0.5),
            player_yaw: 0.0,
        };
        let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
        if move_first {
            shard.enqueue(movement).unwrap();
            shard.enqueue(placement).unwrap();
        } else {
            shard.enqueue(placement).unwrap();
            shard.enqueue(movement).unwrap();
        }
        let outputs = shard.run_tick();
        if move_first {
            rejected(&outputs, actor, target, 27);
            unchanged(&shard, &before);
        } else {
            assert_eq!(block_at(&shard, target), Some(BlockStateId::new(1)));
        }
        assert_eq!(
            shard.player_position(actor),
            Some(Vec3::new(8.5, 64.0, 8.5))
        );
    }
}

#[tokio::test]
async fn upper_door_collision_rejects_whole_placement() {
    // South-facing closed door occupies its thin north strip; only the upper
    // cell reaches this player's feet, leaving the lower cell vertically clear.
    let (mut shard, actor) = owner(Vec3::new(8.5, 66.0, 8.1)).await;
    let lower = BlockPos::new(8, 65, 8);
    let upper = BlockPos::new(8, 66, 8);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    let outputs = place_block(
        &mut shard,
        actor,
        lower,
        OAK_DOOR,
        Direction::Up,
        0.5,
        0.0,
        28,
    );
    rejected(&outputs, actor, lower, 28);
    rejected(&outputs, actor, upper, 28);
    unchanged(&shard, &before);
}

#[tokio::test]
async fn another_players_valid_prefix_survives_a_later_invalid_move() {
    let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    let other = player("moving-obstruction");
    shard
        .enqueue(GameInput::PlayerJoin {
            player: other,
            position: Vec3::new(10.5, 64.0, 8.5),
        })
        .unwrap();
    shard.run_tick();
    for position in [Vec3::new(8.5, 64.0, 8.5), Vec3::new(f64::NAN, 64.0, 8.5)] {
        shard
            .enqueue(GameInput::PlayerMove {
                player: other,
                position: Some(position),
                yaw: None,
                pitch: None,
            })
            .unwrap();
    }
    let target = BlockPos::new(8, 64, 8);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 29);
    rejected(&outputs, actor, target, 29);
    unchanged(&shard, &before);
}

fn completion(
    outputs: &[GameOutput],
    actor: PlayerId,
    position: BlockPos,
    sequence: i32,
    state: Option<BlockStateId>,
) {
    let actual: Vec<_> = outputs
        .iter()
        .filter(|output| matches!(output, GameOutput::PlayerPlacementResolved { .. }))
        .collect();
    assert_eq!(
        actual,
        vec![&GameOutput::PlayerPlacementResolved {
            player: actor,
            position,
            sequence,
            state
        }]
    );
    assert!(matches!(
        outputs.last(),
        Some(GameOutput::PlayerPlacementResolved { .. })
    ));
}

#[tokio::test]
async fn opt_in_completion_reports_committed_shape_and_each_refusal_once() {
    let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    shard.enable_placement_completions();
    let target = BlockPos::new(8, 64, 8);
    let outputs = place_block(
        &mut shard,
        actor,
        target,
        OAK_SLAB,
        Direction::North,
        0.8,
        0.0,
        30,
    );
    completion(&outputs, actor, target, 30, block_at(&shard, target));
    assert_ne!(block_at(&shard, target), Some(BlockStateId::new(OAK_SLAB)));
    let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 30);
    completion(&outputs, actor, target, 30, None); // Same sequence, occupied cell.
    let absent = player("absent-placer");
    for exact in [false, true] {
        let input = if exact {
            GameInput::SetBlockExactTracked {
                player: absent,
                position: target,
                sequence: 31,
                state: BlockStateId::new(1),
            }
        } else {
            GameInput::BlockPlace {
                player: absent,
                position: target,
                sequence: 31,
                state: BlockStateId::new(1),
                clicked_face: Direction::Up,
                cursor_position: Vec3::new(0.5, 0.5, 0.5),
                player_yaw: 0.0,
            }
        };
        shard.enqueue(input).unwrap();
        completion(&shard.run_tick(), absent, target, 31, None);
    }
}

#[tokio::test]
async fn completion_covers_body_refusal_exact_acceptance_and_multicell_acceptance() {
    let (mut shard, actor) = owner(Vec3::new(8.5, 64.0, 8.5)).await;
    shard.enable_placement_completions();
    let target = BlockPos::new(8, 64, 8);
    for (state, accepted) in [(1, false), (TORCH, true)] {
        shard
            .enqueue(GameInput::SetBlockExactTracked {
                player: actor,
                position: target,
                sequence: 32,
                state: BlockStateId::new(state),
            })
            .unwrap();
        completion(
            &shard.run_tick(),
            actor,
            target,
            32,
            accepted.then_some(BlockStateId::new(state)),
        );
    }
    let foot = BlockPos::new(10, 64, 8);
    let outputs = place_block(
        &mut shard,
        actor,
        foot,
        WHITE_BED,
        Direction::Up,
        0.5,
        0.0,
        33,
    );
    completion(&outputs, actor, foot, 33, block_at(&shard, foot));
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(output, GameOutput::BlockChanged { .. }))
            .count(),
        2
    );
}

#[tokio::test]
async fn known_other_held_block_replaces_short_grass_and_journals_the_previous_state() {
    let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    let target = BlockPos::new(8, 64, 8);
    let grass = block_default_state("short_grass").unwrap();
    set_world_block(&mut shard, target, grass);
    let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 34);
    assert!(outputs.iter().any(
        |output| matches!(output, GameOutput::BlockChanged { position, .. } if *position == target)
    ));
    assert_eq!(block_at(&shard, target), Some(BlockStateId::new(1)));
    let mutations = shard.take_mutations();
    assert_eq!(mutations.len(), 1);
    assert_eq!(mutations[0].old_state(), BlockStateId::new(grass));
    assert_eq!(mutations[0].new_state(), BlockStateId::new(1));
}

#[tokio::test]
async fn short_grass_body_replacement_refusal_preserves_the_existing_plant() {
    let (mut shard, actor) = owner(Vec3::new(8.5, 64.0, 8.5)).await;
    let target = BlockPos::new(8, 64, 8);
    let grass = block_default_state("short_grass").unwrap();
    set_world_block(&mut shard, target, grass);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 35);
    assert!(outputs.contains(&GameOutput::BlockChangeRejected {
        player: actor,
        position: target,
        sequence: 35,
        requested_state: BlockStateId::new(1),
        authoritative_state: BlockStateId::new(grass)
    }));
    unchanged(&shard, &before);
}

#[tokio::test]
async fn short_grass_does_not_replace_its_own_item() {
    let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    let target = BlockPos::new(8, 64, 8);
    let grass = block_default_state("short_grass").unwrap();
    set_world_block(&mut shard, target, grass);
    let before = shard.loaded_chunks().get(ChunkPos::ORIGIN).unwrap().clone();
    let outputs = place_block(
        &mut shard,
        actor,
        target,
        grass,
        Direction::Up,
        0.5,
        0.0,
        36,
    );
    assert!(outputs.contains(&GameOutput::BlockChangeRejected {
        player: actor,
        position: target,
        sequence: 36,
        requested_state: BlockStateId::new(grass),
        authoritative_state: BlockStateId::new(grass)
    }));
    unchanged(&shard, &before);
}

#[tokio::test]
async fn bed_primary_and_extra_cells_can_atomically_replace_short_grass() {
    let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    let foot = BlockPos::new(8, 64, 8);
    let head = BlockPos::new(8, 64, 9);
    let grass = block_default_state("short_grass").unwrap();
    set_world_block(&mut shard, foot, grass);
    set_world_block(&mut shard, head, grass);
    let outputs = place_block(
        &mut shard,
        actor,
        foot,
        WHITE_BED,
        Direction::Up,
        0.5,
        0.0,
        37,
    );
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(output, GameOutput::BlockChanged { .. }))
            .count(),
        2
    );
    assert_ne!(block_at(&shard, foot), Some(BlockStateId::new(grass)));
    assert_ne!(block_at(&shard, head), Some(BlockStateId::new(grass)));
    let mutations = shard.take_mutations();
    assert_eq!(mutations.len(), 2);
    assert!(mutations
        .iter()
        .all(|mutation| mutation.old_state() == BlockStateId::new(grass)));
}

#[tokio::test]
async fn placement_target_uses_resident_grass_with_other_item_and_defaults_safely() {
    let (mut shard, _) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    let clicked = BlockPos::new(8, 64, 8);
    let adjacent = BlockPos::new(8, 65, 8);
    let grass = BlockStateId::new(block_default_state("short_grass").unwrap());
    let stone = BlockStateId::new(1);
    set_world_block(&mut shard, clicked, grass.as_u32());
    assert_eq!(shard.placement_target(clicked, adjacent, stone), clicked);
    assert_eq!(shard.placement_target(clicked, adjacent, grass), adjacent);
    set_world_block(&mut shard, clicked, 1);
    assert_eq!(shard.placement_target(clicked, adjacent, stone), adjacent);
    assert_eq!(
        shard.placement_target(BlockPos::new(32, 64, 8), adjacent, stone),
        adjacent
    );
    assert_eq!(
        shard.placement_target(BlockPos::new(8, 10_000, 8), adjacent, stone),
        adjacent
    );
}

#[tokio::test]
async fn generic_exact_world_intent_with_same_sequence_cannot_complete_a_placement() {
    let (mut shard, actor) = owner(Vec3::new(8.5, 64.0, 8.5)).await;
    shard.enable_placement_completions();
    let target = BlockPos::new(8, 64, 8);
    // A before_interact hook's world intent precedes the explicit placement.
    shard
        .enqueue(GameInput::SetBlockExact {
            player: actor,
            position: target,
            sequence: 38,
            state: BlockStateId::new(TORCH),
        })
        .unwrap();
    shard
        .enqueue(GameInput::BlockPlace {
            player: actor,
            position: target,
            sequence: 38,
            state: BlockStateId::new(1),
            clicked_face: Direction::Up,
            cursor_position: Vec3::new(0.5, 0.5, 0.5),
            player_yaw: 0.0,
        })
        .unwrap();
    let outputs = shard.run_tick();
    completion(&outputs, actor, target, 38, None);
    assert_eq!(block_at(&shard, target), Some(BlockStateId::new(TORCH)));
}

#[tokio::test]
async fn generic_exact_writes_never_emit_placement_completion_on_accept_or_refusal() {
    for (feet, state) in [
        (Vec3::new(8.5, 64.0, 8.5), 1),
        (Vec3::new(6.5, 64.0, 8.5), 1),
    ] {
        let (mut shard, actor) = owner(feet).await;
        shard.enable_placement_completions();
        for player in [actor, super::player("missing-world-intent")] {
            shard
                .enqueue(GameInput::SetBlockExact {
                    player,
                    position: BlockPos::new(8, 64, 8),
                    sequence: 39,
                    state: BlockStateId::new(state),
                })
                .unwrap();
            assert!(!shard
                .run_tick()
                .iter()
                .any(|output| matches!(output, GameOutput::PlayerPlacementResolved { .. })));
        }
    }
}

#[tokio::test]
async fn relocated_merge_completion_reports_actual_primary_position() {
    let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
    shard.enable_placement_completions();
    let clicked = BlockPos::new(8, 64, 8);
    let adjacent = BlockPos::new(8, 65, 8);
    set_world_block(&mut shard, clicked, OAK_SLAB);
    let outputs = place_block(
        &mut shard,
        actor,
        adjacent,
        OAK_SLAB,
        Direction::Up,
        0.5,
        0.0,
        40,
    );
    completion(&outputs, actor, clicked, 40, block_at(&shard, clicked));
    assert_ne!(block_at(&shard, clicked), Some(BlockStateId::new(OAK_SLAB)));
}

#[tokio::test]
async fn fluid_replacement_owner_flowing_water_and_lava_are_replaceable_without_body_bypass() {
    for name in ["water", "lava"] {
        let (mut shard, actor) = owner(Vec3::new(6.5, 64.0, 8.5)).await;
        let liquid = BlockStateId::new(
            ferrumc_registry::block_state::compute_state_id(
                name,
                &[("level", "3")].into_iter().collect(),
            )
            .expect("flowing state"),
        );
        let target = BlockPos::new(8, 65, 8);
        let _ = shard.apply_block_edit(MutationCause::Test, target, liquid);
        let outputs = place_block(&mut shard, actor, target, 1, Direction::Up, 0.5, 0.0, 21);
        assert_eq!(shard.authoritative_state(target), BlockStateId::new(1));
        assert!(outputs.iter().any(
            |output| matches!(output,GameOutput::BlockChanged {position,..} if *position==target)
        ));
        let feet = BlockPos::new(6, 64, 8);
        let _ = shard.apply_block_edit(MutationCause::Test, feet, liquid);
        let outputs = place_block(&mut shard, actor, feet, 1, Direction::Up, 0.5, 0.0, 22);
        assert_eq!(shard.authoritative_state(feet), liquid);
        assert!(outputs
            .iter()
            .any(|output| matches!(output, GameOutput::BlockChangeRejected { .. })));
    }
}
