use super::*;
use ferrumc_world::FurnaceInventory;

fn furnace_state() -> BlockStateId {
    BlockStateId::new(
        ferrumc_registry::block_state::block_default_state("furnace").expect("furnace"),
    )
}
fn empty_player() -> [ItemStack; 36] {
    std::array::from_fn(|_| ItemStack::empty())
}
async fn furnace_setup(p: PlayerId, cap: usize) -> (SimShard, BlockPos) {
    let mut s = shard_with_player(p).await;
    s.enable_furnaces(cap).expect("enable");
    let pos = BlockPos::new(8, 65, 8);
    assert!(matches!(
        s.apply_block_edit(
            MutationCause::PlayerCreative { player: p },
            pos,
            furnace_state()
        ),
        MutationResult::Applied { .. }
    ));
    (s, pos)
}
fn set_furnace(
    s: &mut SimShard,
    pos: BlockPos,
    slots: [ItemStack; 3],
    burn: u16,
    duration: u16,
    progress: u16,
) {
    let candidate = FurnaceInventory::try_from_parts(slots, burn, duration, progress, 200)
        .expect("valid furnace");
    s.loaded_chunks_mut()
        .get_mut(pos.to_chunk_pos())
        .expect("resident")
        .set_block_entity(pos, BlockEntity::Furnace(candidate))
        .expect("entity");
}

#[tokio::test]
async fn furnace_owner_two_smelt_ticks_lit_persistence_and_result_exchange() {
    let p = player("smelter");
    let (mut s, pos) = furnace_setup(p, 4).await;
    set_furnace(
        &mut s,
        pos,
        [stack("raw_iron", 2), stack("coal", 1), ItemStack::empty()],
        0,
        0,
        0,
    );
    let outputs = s.run_tick();
    assert!(outputs.iter().any(|out| matches!(
        out,
        GameOutput::BlockChanged {
            cause: MutationCause::Furnace,
            ..
        }
    )));
    let current = s.furnace_snapshot(p, pos).expect("snapshot");
    assert_eq!(current.burn_remaining(), 1600);
    assert_eq!(current.cook_progress(), 1);
    assert!(current.slots()[1].item().is_none());
    for _ in 0..399 {
        let _ = s.run_tick();
    }
    let current = s.furnace_snapshot(p, pos).expect("smelted");
    assert!(current.slots()[0].item().is_none());
    assert_eq!(current.slots()[2], stack("iron_ingot", 2));
    assert_eq!(current.burn_remaining(), 1201);
    assert_eq!(current.cook_progress(), 0);
    let (after, inventory, cursor) = s
        .furnace_inventory_click_checked(
            p,
            pos,
            current.slots(),
            empty_player(),
            ItemStack::empty(),
            2,
            0,
            1,
        )
        .expect("shift result");
    assert!(after.slots()[2].item().is_none());
    assert_eq!(inventory[35], stack("iron_ingot", 2));
    assert!(cursor.item().is_none());
    assert!(s.loaded_chunks().has_persist_dirty());
    assert!(s
        .take_mutations()
        .iter()
        .any(|mutation| mutation.cause() == MutationCause::Furnace));
}

#[tokio::test]
async fn furnace_owner_stale_smelt_slots_preserve_player_and_cursor_but_timers_do_not_stale() {
    let p = player("furnace_stale");
    let (mut s, pos) = furnace_setup(p, 4).await;
    set_furnace(
        &mut s,
        pos,
        [stack("raw_iron", 2), stack("coal", 1), ItemStack::empty()],
        0,
        0,
        0,
    );
    let expected = s.furnace_snapshot(p, pos).expect("before");
    let _ = s.run_tick();
    let cursor = stack("diamond", 4);
    let mut inventory = empty_player();
    inventory[27] = stack("dirt", 7);
    let (actual, unchanged, samecursor) = s
        .furnace_inventory_click_checked(
            p,
            pos,
            expected.slots(),
            inventory.clone(),
            cursor.clone(),
            0,
            0,
            0,
        )
        .expect("stale response");
    assert!(actual.slots()[1].item().is_none());
    assert_eq!(unchanged, inventory);
    assert_eq!(samecursor, cursor);
    let expected = actual;
    let _ = s.run_tick();
    let (after, _, cursor) = s
        .furnace_inventory_click_checked(
            p,
            pos,
            expected.slots(),
            empty_player(),
            ItemStack::empty(),
            0,
            1,
            0,
        )
        .expect("timer-independent pickup");
    assert_eq!(after.slots()[0], stack("raw_iron", 1));
    assert_eq!(after.cook_progress(), 2);
    assert_eq!(cursor, stack("raw_iron", 1));
}

#[tokio::test]
async fn furnace_owner_input_identity_reset_and_invalid_click_do_not_mutate() {
    let p = player("furnace_replace");
    let (mut s, pos) = furnace_setup(p, 4).await;
    set_furnace(
        &mut s,
        pos,
        [stack("raw_iron", 2), ItemStack::empty(), ItemStack::empty()],
        8,
        1600,
        67,
    );
    let current = s.furnace_snapshot(p, pos).expect("current");
    assert!(s
        .furnace_inventory_click_checked(
            p,
            pos,
            current.slots(),
            empty_player(),
            stack("dirt", 2),
            2,
            0,
            0
        )
        .is_some());
    assert_eq!(s.furnace_snapshot(p, pos).expect("unchanged"), current);
    assert!(s
        .furnace_inventory_click_checked(
            p,
            pos,
            current.slots(),
            empty_player(),
            stack("dirt", 2),
            999,
            0,
            0
        )
        .is_none());
    let (after, _, cursor) = s
        .furnace_inventory_click_checked(
            p,
            pos,
            current.slots(),
            empty_player(),
            stack("raw_gold", 2),
            0,
            0,
            0,
        )
        .expect("replace input");
    assert_eq!(after.slots()[0], stack("raw_gold", 2));
    assert_eq!(after.cook_progress(), 0);
    assert_eq!(after.burn_remaining(), 8);
    assert_eq!(cursor, stack("raw_iron", 2));
}

#[tokio::test]
async fn furnace_owner_loaded_furnace_discovery_and_unload_pause_preserve_timers() {
    let p = player("furnace_loaded");
    let mut s = shard_with_player(p).await;
    let pos = BlockPos::new(8, 65, 8);
    let _ = s.apply_block_edit(MutationCause::Test, pos, furnace_state());
    set_furnace(
        &mut s,
        pos,
        [stack("raw_copper", 2), stack("coal", 1), ItemStack::empty()],
        0,
        0,
        0,
    );
    s.enable_furnaces(4).expect("enable after imported entity");
    let _ = s.run_tick();
    assert_eq!(
        s.furnace_snapshot(p, pos)
            .expect("discovered")
            .cook_progress(),
        1
    );
    // Reconciliation of a lit state preserves all three original slots/timers.
    assert!(
        matches!(s.chunks.get(pos.to_chunk_pos()).expect("resident").block_entity(pos),Some(BlockEntity::Furnace(f)) if f.slots()[0]==stack("raw_copper",2))
    );
    let saved = s.furnace_snapshot(p, pos).expect("save before unload");
    assert!(s
        .loaded_chunks_mut()
        .release(pos.to_chunk_pos(), ChunkTicket::of(TicketReason::Player))
        .is_unloaded());
    for _ in 0..10 {
        let _ = s.run_tick();
    }
    assert!(s.furnaces.as_ref().expect("runtime").registered.is_empty());
    ground_load(&mut s, Vec3::new(8.0, 65.0, 8.0)).await;
    let _ = s.apply_block_edit(MutationCause::Test, pos, furnace_state());
    s.loaded_chunks_mut()
        .get_mut(pos.to_chunk_pos())
        .expect("resident")
        .set_block_entity(pos, BlockEntity::Furnace(saved))
        .expect("restore native data");
    let _ = s.run_tick();
    assert_eq!(
        s.furnace_snapshot(p, pos)
            .expect("rediscovered")
            .cook_progress(),
        2
    );
}

#[tokio::test]
async fn furnace_owner_capacity_before_world_commit_and_noop_not_dirty() {
    let p = player("furnace_capacity");
    let (mut s, pos) = furnace_setup(p, 1).await;
    let _ = s.take_mutations();
    let _ = s.loaded_chunks_mut().take_dirty();
    let other = BlockPos::new(9, 65, 8);
    assert!(matches!(
        s.apply_block_edit(
            MutationCause::PlayerCreative { player: p },
            other,
            furnace_state()
        ),
        MutationResult::Rejected {
            reason: RejectionReason::FurnaceCapacity,
            ..
        }
    ));
    assert!(s.authoritative_state(other).is_air());
    assert_eq!(s.take_mutations().len(), 0);
    assert_eq!(s.loaded_chunks_mut().take_dirty().len(), 0);
    let _ = s.loaded_chunks_mut().take_persist_dirty(0);
    let expected = s.furnace_snapshot(p, pos).expect("empty");
    s.furnace_inventory_click_checked(
        p,
        pos,
        expected.slots(),
        empty_player(),
        ItemStack::empty(),
        0,
        0,
        0,
    )
    .expect("noop");
    assert!(!s.loaded_chunks().has_persist_dirty());
}

#[tokio::test]
async fn furnace_owner_mining_ground_capacity_refuses_whole_container_then_preserves_contents() {
    let p = player("furnace_miner");
    let (mut s, pos) = furnace_setup(p, 4).await;
    set_furnace(
        &mut s,
        pos,
        [
            stack("raw_iron", 3),
            stack("coal", 4),
            stack("iron_ingot", 5),
        ],
        0,
        0,
        0,
    );
    enable_ground_runtime(&mut s, 3);
    let tool = mining_tool("diamond_pickaxe");
    s.enqueue(GameInput::MiningStart {
        player: p,
        position: pos,
        sequence: 1,
        tool,
    })
    .expect("start");
    let _ = s.run_tick();
    for _ in 0..15 {
        let _ = s.run_tick();
    }
    s.enqueue(GameInput::MiningFinish {
        player: p,
        position: pos,
        sequence: 2,
        tool,
    })
    .expect("finish");
    let outputs = s.run_tick();
    assert!(!s.authoritative_state(pos).is_air());
    assert!(!outputs
        .iter()
        .any(|out| matches!(out, GameOutput::SurvivalBlockBroken { .. })));
    assert_eq!(
        s.furnace_snapshot(p, pos)
            .expect("all contents retained")
            .slots()[2],
        stack("iron_ingot", 5)
    );
    // All contents and the furnace need four distinct reserved payload credits.
    assert!(s.ground_items.as_ref().expect("ground").pending.is_empty());
    s.ground_items = None;
    s.entities = None;
    enable_ground_runtime(&mut s, 8);
    s.enqueue(GameInput::MiningStart {
        player: p,
        position: pos,
        sequence: 3,
        tool,
    })
    .expect("retry start");
    let _ = s.run_tick();
    for _ in 0..15 {
        let _ = s.run_tick();
    }
    s.enqueue(GameInput::MiningFinish {
        player: p,
        position: pos,
        sequence: 4,
        tool,
    })
    .expect("retry finish");
    let _ = s.run_tick();
    assert!(s.authoritative_state(pos).is_air());
    let pending = &s.ground_items.as_ref().expect("ground").pending;
    assert_eq!(pending.len(), 4);
    for name in ["raw_iron", "coal", "iron_ingot", "furnace"] {
        assert!(pending.iter().any(|(_, drop)| drop
            .stack
            .item()
            .is_some_and(|item| item.name().strip_prefix("minecraft:") == Some(name))));
    }
}
