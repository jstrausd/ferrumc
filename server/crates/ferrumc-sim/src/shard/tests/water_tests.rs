use super::*;
use crate::WaterError;

fn water_state() -> BlockStateId {
    BlockStateId::new(ferrumc_registry::block_state::block_default_state("water").expect("source"))
}

async fn water_floor(s: &mut SimShard, from_x: i32, to_x: i32) {
    for x in [from_x, to_x] {
        ground_load(s, Vec3::new(f64::from(x), 65.0, 8.0)).await;
    }
    for x in from_x..=to_x {
        for z in 0..16 {
            assert!(matches!(
                s.apply_block_edit(
                    MutationCause::Test,
                    BlockPos::new(x, 64, z),
                    BlockStateId::new(1)
                ),
                MutationResult::Applied { .. }
            ));
        }
    }
}

#[tokio::test]
async fn water_owner_five_tick_delay_cross_region_and_real_journal() {
    let mut s = shard();
    water_floor(&mut s, 120, 136).await;
    let source = BlockPos::new(127, 65, 8);
    assert!(matches!(
        s.apply_block_edit(MutationCause::Test, source, water_state()),
        MutationResult::Applied { .. }
    ));
    s.enable_water(4096, true).expect("enable");
    s.enqueue(GameInput::WaterSchedule { position: source })
        .expect("schedule");
    for _ in 0..5 {
        assert!(!s.run_tick().iter().any(|output| matches!(
            output,
            GameOutput::BlockChanged {
                cause: MutationCause::Fluid,
                ..
            }
        )));
    }
    let outputs = s.run_tick();
    assert!(outputs.iter().any(|output| matches!(output,GameOutput::BlockChanged {position,cause:MutationCause::Fluid,..} if *position==BlockPos::new(128,65,8))));
    assert_eq!(
        state_id_to_block_name(s.authoritative_state(BlockPos::new(128, 65, 8)).as_u32()),
        Some("water")
    );
    let mutations = s.take_mutations();
    assert!(mutations
        .iter()
        .any(|mutation| matches!(mutation.cause(), MutationCause::Fluid)));
    assert!(s.loaded_chunks().has_persist_dirty());
}

#[tokio::test]
async fn water_owner_full_schedule_refuses_world_write_without_dirty_or_journal() {
    let p = player("water_full");
    let mut s = shard_with_player(p).await;
    let occupied = BlockPos::new(12, 65, 8);
    assert!(matches!(
        s.apply_block_edit(MutationCause::Test, occupied, water_state()),
        MutationResult::Applied { .. }
    ));
    let _ = s.loaded_chunks_mut().take_dirty();
    s.enable_water(1, true).expect("enable");
    s.enqueue(GameInput::WaterSchedule { position: occupied })
        .expect("schedule");
    let _ = s.run_tick();
    let target = BlockPos::new(8, 65, 8);
    let result = s.apply_block_edit(
        MutationCause::PlayerCreative { player: p },
        target,
        water_state(),
    );
    assert!(matches!(
        result,
        MutationResult::Rejected {
            reason: RejectionReason::FluidScheduling,
            ..
        }
    ));
    assert!(s.authoritative_state(target).is_air());
    assert_eq!(s.water_queue_len(), Some(1));
    assert_eq!(s.take_mutations().len(), 0);
    assert_eq!(s.loaded_chunks_mut().take_dirty().len(), 0);
}

#[tokio::test]
async fn water_owner_source_removal_decays_without_wall_clock_or_chunk_loads() {
    let p = player("water_decay");
    let mut s = shard_with_player(p).await;
    water_floor(&mut s, 0, 15).await;
    let source = BlockPos::new(8, 65, 8);
    assert!(matches!(
        s.apply_block_edit(MutationCause::Test, source, water_state()),
        MutationResult::Applied { .. }
    ));
    s.enable_water(4096, true).expect("enable");
    s.enqueue(GameInput::WaterSchedule { position: source })
        .expect("schedule");
    for _ in 0..100 {
        let _ = s.run_tick();
    }
    assert_eq!(
        state_id_to_block_name(s.authoritative_state(BlockPos::new(9, 65, 8)).as_u32()),
        Some("water")
    );
    assert!(matches!(
        s.apply_block_edit(
            MutationCause::PlayerCreative { player: p },
            source,
            BlockStateId::AIR
        ),
        MutationResult::Applied { .. }
    ));
    for _ in 0..240 {
        let _ = s.run_tick();
    }
    for x in 0..16 {
        for z in 0..16 {
            assert!(
                s.authoritative_state(BlockPos::new(x, 65, z)).is_air(),
                "remaining water {x},{z}"
            );
        }
    }
    assert_eq!(s.loaded_chunks().loaded_count(), 1);
}

#[tokio::test]
async fn water_owner_destroyed_torch_uses_real_ground_payload_owner() {
    let mut s = shard();
    water_floor(&mut s, 0, 15).await;
    enable_ground_runtime(&mut s, 8);
    s.refill_ground_item_identities(vec![ground_identity(90)])
        .expect("identity");
    let source = BlockPos::new(8, 65, 8);
    let torch = BlockPos::new(9, 65, 8);
    let torch_state = BlockStateId::new(
        ferrumc_registry::block_state::block_default_state("torch").expect("torch"),
    );
    let _ = s.apply_block_edit(MutationCause::Test, source, water_state());
    let _ = s.apply_block_edit(MutationCause::Test, torch, torch_state);
    s.enable_water(4096, true).expect("enable");
    s.enqueue(GameInput::WaterSchedule { position: source })
        .expect("schedule");
    for _ in 0..6 {
        let _ = s.run_tick();
    }
    assert_eq!(
        state_id_to_block_name(s.authoritative_state(torch).as_u32()),
        Some("water")
    );
    let snapshots = s.ground_item_snapshots(None, 32).expect("items");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].item_state().stack(), &stack("torch", 1));
}

#[tokio::test]
async fn water_owner_drop_capacity_refusal_retains_entire_flow_preimage_and_due_work() {
    let mut s = shard();
    water_floor(&mut s, 0, 15).await;
    enable_ground_runtime(&mut s, 1);
    s.enqueue_ground_item_drop(crate::GroundItemDrop::new(
        Vec3::new(6.0, 65.0, 8.0),
        stack("diamond", 5),
        10,
        None,
        None,
    ))
    .expect("reserved");
    let source = BlockPos::new(8, 65, 8);
    let torch = BlockPos::new(9, 65, 8);
    let torch_state = BlockStateId::new(
        ferrumc_registry::block_state::block_default_state("torch").expect("torch"),
    );
    let _ = s.apply_block_edit(MutationCause::Test, source, water_state());
    let _ = s.apply_block_edit(MutationCause::Test, torch, torch_state);
    s.enable_water(4096, true).expect("enable");
    s.enqueue(GameInput::WaterSchedule { position: source })
        .expect("schedule");
    let mut outputs = Vec::new();
    for _ in 0..6 {
        outputs.extend(s.run_tick());
    }
    assert!(outputs.iter().any(|output| matches!(
        output,
        GameOutput::WaterTickDeferred {
            reason: WaterError::DropRefused,
            ..
        }
    )));
    assert_eq!(s.authoritative_state(torch), torch_state);
    assert!(s.authoritative_state(BlockPos::new(8, 65, 7)).is_air());
    assert_eq!(s.water_queue_len(), Some(1));
    assert_eq!(
        s.ground_items
            .as_ref()
            .expect("owner")
            .pending
            .front()
            .expect("retained")
            .1
            .stack,
        stack("diamond", 5)
    );
    assert_eq!(s.take_mutations().len(), 0);
}

#[tokio::test]
async fn water_owner_bucket_pour_fill_replay_and_flowing_pickup_rules() {
    let p = player("bucket_owner");
    let mut s = shard_with_player(p).await;
    s.enable_survival(crate::Difficulty::Normal, false)
        .expect("health owner");
    s.enable_water(4096, true).expect("water owner");
    let position = BlockPos::new(8, 65, 8);
    s.enqueue(GameInput::BucketUse {
        player: p,
        position,
        sequence: 1,
        fill: false,
        token: 1,
    })
    .expect("pour");
    assert!(s.authoritative_state(position).is_air());
    assert!(s.run_tick().iter().any(|output| matches!(output,GameOutput::BucketUseResolved {token:1,state:Some(state),..} if *state==water_state())));
    assert_eq!(s.authoritative_state(position), water_state());
    s.enqueue(GameInput::BucketUse {
        player: p,
        position,
        sequence: 2,
        fill: true,
        token: 2,
    })
    .expect("fill");
    assert!(s.run_tick().iter().any(|output| matches!(output,GameOutput::BucketUseResolved {token:2,state:Some(state),..} if state.is_air())));
    assert!(s.authoritative_state(position).is_air());
    s.enqueue(GameInput::BucketUse {
        player: p,
        position,
        sequence: 3,
        fill: false,
        token: 2,
    })
    .expect("replayed plan");
    assert!(s.run_tick().iter().any(|output| matches!(
        output,
        GameOutput::BucketUseResolved {
            token: 2,
            state: None,
            ..
        }
    )));
    assert!(s.authoritative_state(position).is_air());
    let flowing = BlockStateId::new(
        ferrumc_registry::block_state::compute_state_id(
            "water",
            &[("level", "1")].into_iter().collect(),
        )
        .expect("flow"),
    );
    let _ = s.apply_block_edit(MutationCause::Test, position, flowing);
    s.enqueue(GameInput::BucketUse {
        player: p,
        position,
        sequence: 4,
        fill: true,
        token: 3,
    })
    .expect("flowing pickup");
    assert!(s.run_tick().iter().any(|output| matches!(
        output,
        GameOutput::BucketUseResolved {
            token: 3,
            state: None,
            ..
        }
    )));
    assert_eq!(s.authoritative_state(position), flowing);
}

#[tokio::test]
async fn water_owner_bucket_static_waterlogging_and_full_queue_preserve_world() {
    let p = player("bucket_container");
    let mut s = shard_with_player(p).await;
    s.enable_survival(crate::Difficulty::Normal, false)
        .expect("health owner");
    s.enable_water(1, true).expect("water owner");
    let occupied = BlockPos::new(12, 65, 8);
    let _ = s.apply_block_edit(MutationCause::Test, occupied, water_state());
    s.enqueue(GameInput::WaterSchedule { position: occupied })
        .expect("schedule");
    let _ = s.run_tick();
    let position = BlockPos::new(8, 65, 8);
    s.enqueue(GameInput::BucketUse {
        player: p,
        position,
        sequence: 1,
        fill: false,
        token: 1,
    })
    .expect("pour");
    assert!(s.run_tick().iter().any(|output| matches!(
        output,
        GameOutput::BucketUseResolved {
            token: 1,
            state: None,
            ..
        }
    )));
    assert!(s.authoritative_state(position).is_air());
    assert_eq!(s.water_queue_len(), Some(1));
    let rules = crate::WaterRules::builtin().expect("rules");
    let dry = BlockStateId::new(
        ferrumc_registry::block_state::compute_state_id(
            "oak_sign",
            &[("rotation", "3"), ("waterlogged", "false")]
                .into_iter()
                .collect(),
        )
        .expect("dry"),
    );
    let wet = rules
        .bucket_after(dry, false)
        .expect("pour static")
        .expect("waterlogged");
    assert_eq!(
        rules.bucket_after(wet, true).expect("fill static"),
        Some(dry)
    );
    assert_eq!(state_id_to_block_name(wet.as_u32()), Some("oak_sign"));
}
