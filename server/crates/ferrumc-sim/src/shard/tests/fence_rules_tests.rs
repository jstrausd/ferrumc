use super::*;
use ferrumc_registry::block_state::{block_default_state, compute_state_id};

const SIDES: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
];

fn side_name(side: Direction) -> &'static str {
    match side {
        Direction::North => "north",
        Direction::South => "south",
        Direction::East => "east",
        Direction::West => "west",
        other => panic!("expected horizontal test direction {other:?}"),
    }
}

fn reference_state(name: &str, properties: &BTreeMap<&str, &str>) -> u32 {
    compute_state_id(name, properties).expect("pinned fixture state")
}

fn assert_fence_arm(s: &SimShard, pos: BlockPos, name: &str, side: Direction, connected: bool) {
    let mut properties = BTreeMap::new();
    properties.insert(side_name(side), if connected { "true" } else { "false" });
    assert_eq!(
        block_at(s, pos),
        Some(BlockStateId::new(reference_state(name, &properties)))
    );
}

fn stale_wet_fence(name: &str, side: Direction, wanted: bool) -> (u32, u32) {
    let mut properties = BTreeMap::new();
    properties.insert("waterlogged", "true");
    // An unrelated connection must survive the notification, even though this
    // scaffolding has not loaded a real connecting neighbour for that arm.
    properties.insert(side_name(side.opposite()), "true");
    properties.insert(side_name(side), if wanted { "false" } else { "true" });
    let old = reference_state(name, &properties);
    properties.insert(side_name(side), if wanted { "true" } else { "false" });
    (old, reference_state(name, &properties))
}

fn assert_secondary_edit(s: &mut SimShard, outputs: &[GameOutput], pos: BlockPos, expected: u32) {
    assert_eq!(
        outputs.len(),
        2,
        "one primary and one bounded secondary update"
    );
    assert!(outputs.contains(&GameOutput::BlockChanged {
        position: pos,
        state: BlockStateId::new(expected),
        sequence: 0,
        cause: MutationCause::Command,
    }));
    let mutations = s.take_mutations();
    assert_eq!(mutations.len(), 2);
    assert_eq!(mutations[1].position(), pos);
    assert_eq!(mutations[1].new_state(), BlockStateId::new(expected));
    assert!(s.loaded_chunks().has_persist_dirty());
}

#[tokio::test]
async fn consumer_fence_families_apply_to_fresh_placement_and_secondary_refresh() {
    let p = player("fence-family-contract");
    for (existing_name, placed_name, connect) in [
        ("oak_fence", "spruce_fence", true),
        ("oak_fence", "crimson_fence", true),
        ("crimson_fence", "warped_fence", true),
        ("oak_fence", "nether_brick_fence", false),
        ("nether_brick_fence", "oak_fence", false),
        ("nether_brick_fence", "crimson_fence", false),
        ("nether_brick_fence", "nether_brick_fence", true),
    ] {
        for side in SIDES {
            let mut s = shard_with_player(p).await;
            let existing = BlockPos::new(8, 65, 8);
            let placed = existing.offset(side);
            let (old_state, expected_state) = stale_wet_fence(existing_name, side, connect);
            set_world_block(&mut s, existing, old_state);
            let outputs = place_block(
                &mut s,
                p,
                placed,
                block_default_state(placed_name).expect("pinned placed fence"),
                Direction::Up,
                0.5,
                0.0,
                1,
            );
            assert_fence_arm(&s, placed, placed_name, side.opposite(), connect);
            assert_eq!(
                block_at(&s, existing),
                Some(BlockStateId::new(expected_state)),
                "refresh {existing_name} -> {placed_name} {side:?}"
            );
            assert_secondary_edit(&mut s, &outputs, existing, expected_state);
        }
    }
}

fn gate_connects(facing: Direction, side: Direction) -> bool {
    match facing {
        Direction::North | Direction::South => matches!(side, Direction::East | Direction::West),
        Direction::East | Direction::West => matches!(side, Direction::North | Direction::South),
        _ => panic!("horizontal gate facing"),
    }
}

#[tokio::test]
async fn consumer_gate_axis_and_open_state_apply_to_fresh_fence_placement() {
    let p = player("fence-gate-fresh-contract");
    for source in ["oak_fence", "nether_brick_fence"] {
        for gate in ["oak_fence_gate", "warped_fence_gate"] {
            for facing in SIDES {
                for side in SIDES {
                    for open in ["true", "false"] {
                        let mut s = shard_with_player(p).await;
                        let placed = BlockPos::new(8, 65, 8);
                        let neighbor = placed.offset(side);
                        let mut properties = BTreeMap::new();
                        properties.insert("facing", side_name(facing));
                        properties.insert("open", open);
                        properties.insert("in_wall", "true");
                        properties.insert("powered", "true");
                        let gate_state = reference_state(gate, &properties);
                        set_world_block(&mut s, neighbor, gate_state);
                        let outputs = place_block(
                            &mut s,
                            p,
                            placed,
                            block_default_state(source).expect("pinned fence"),
                            Direction::Up,
                            0.5,
                            0.0,
                            1,
                        );
                        assert_fence_arm(&s, placed, source, side, gate_connects(facing, side));
                        assert_eq!(block_at(&s, neighbor), Some(BlockStateId::new(gate_state)));
                        assert_eq!(outputs.len(), 1, "gate is not rewritten as a fence");
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn consumer_gate_axis_applies_to_secondary_fence_refresh() {
    let p = player("fence-gate-refresh-contract");
    for source in ["oak_fence", "nether_brick_fence"] {
        for gate in ["oak_fence_gate", "warped_fence_gate"] {
            for (facing, yaw) in [
                (Direction::North, 180.0),
                (Direction::South, 0.0),
                (Direction::East, 270.0),
                (Direction::West, 90.0),
            ] {
                for side in SIDES {
                    let mut s = shard_with_player(p).await;
                    let fence = BlockPos::new(8, 65, 8);
                    let gate_pos = fence.offset(side);
                    let (old, expected) =
                        stale_wet_fence(source, side, gate_connects(facing, side));
                    set_world_block(&mut s, fence, old);
                    let outputs = place_block(
                        &mut s,
                        p,
                        gate_pos,
                        block_default_state(gate).expect("pinned gate"),
                        Direction::Up,
                        0.5,
                        yaw,
                        1,
                    );
                    assert_eq!(
                        block_at(&s, fence),
                        Some(BlockStateId::new(expected)),
                        "{source} refresh against {gate} facing {facing:?} at {side:?}"
                    );
                    let mut gate_properties = BTreeMap::new();
                    gate_properties.insert("facing", side_name(facing));
                    assert_eq!(
                        block_at(&s, gate_pos),
                        Some(BlockStateId::new(reference_state(gate, &gate_properties)))
                    );
                    assert_secondary_edit(&mut s, &outputs, fence, expected);
                }
            }
        }
    }
}

// Independent names from the reviewed built-in leaf class registrations,
// SHULKER_BOXES tag and Block.isExceptionForConnection's explicit blocks.
const EXCEPTIONS: [&str; 33] = [
    "jungle_leaves",
    "oak_leaves",
    "spruce_leaves",
    "pale_oak_leaves",
    "dark_oak_leaves",
    "acacia_leaves",
    "birch_leaves",
    "azalea_leaves",
    "flowering_azalea_leaves",
    "mangrove_leaves",
    "cherry_leaves",
    "shulker_box",
    "black_shulker_box",
    "blue_shulker_box",
    "brown_shulker_box",
    "cyan_shulker_box",
    "gray_shulker_box",
    "green_shulker_box",
    "light_blue_shulker_box",
    "light_gray_shulker_box",
    "lime_shulker_box",
    "magenta_shulker_box",
    "orange_shulker_box",
    "pink_shulker_box",
    "purple_shulker_box",
    "red_shulker_box",
    "white_shulker_box",
    "yellow_shulker_box",
    "barrier",
    "carved_pumpkin",
    "jack_o_lantern",
    "melon",
    "pumpkin",
];

#[tokio::test]
async fn consumer_all_33_exceptions_remain_disconnected_on_fresh_placement() {
    let p = player("fence-exception-fresh-contract");
    for exception in EXCEPTIONS {
        for side in SIDES {
            let mut s = shard_with_player(p).await;
            let placed = BlockPos::new(8, 65, 8);
            set_world_block(
                &mut s,
                placed.offset(side),
                block_default_state(exception).expect("registered exception"),
            );
            let outputs = place_block(&mut s, p, placed, OAK_FENCE, Direction::Up, 0.5, 0.0, 1);
            assert_eq!(
                block_at(&s, placed),
                Some(BlockStateId::new(OAK_FENCE)),
                "fresh fence must reject {exception} at {side:?}"
            );
            assert_eq!(outputs.len(), 1);
        }
    }
}

#[tokio::test]
async fn consumer_all_33_exceptions_disconnect_only_refreshed_fence_arm() {
    let p = player("fence-exception-refresh-contract");
    for exception in EXCEPTIONS {
        for side in SIDES {
            let mut s = shard_with_player(p).await;
            let fence = BlockPos::new(8, 65, 8);
            let (old, expected) = stale_wet_fence("oak_fence", side, false);
            set_world_block(&mut s, fence, old);
            let outputs = place_block(
                &mut s,
                p,
                fence.offset(side),
                block_default_state(exception).expect("registered exception"),
                Direction::Up,
                0.5,
                0.0,
                1,
            );
            assert_eq!(
                block_at(&s, fence),
                Some(BlockStateId::new(expected)),
                "refresh must reject {exception} at {side:?}"
            );
            assert_secondary_edit(&mut s, &outputs, fence, expected);
        }
    }
}

#[tokio::test]
async fn consumer_face_query_refuses_unknown_sources_states_faces_and_unloaded_neighbors() {
    let p = player("fence-query-refusal-contract");
    let mut s = shard_with_player(p).await;
    let neighbor = BlockPos::new(8, 65, 8);
    set_world_block(&mut s, neighbor, 1);
    let query = ShardNeighborQuery {
        chunks: s.loaded_chunks(),
    };
    for source in [
        "stone",
        "oak_fence_gate",
        "fake_fence",
        "example:oak_fence",
        "",
    ] {
        assert!(!query.is_fence_connectable_on_face(neighbor, source, Direction::North));
    }
    for face in [Direction::Up, Direction::Down] {
        assert!(!query.is_fence_connectable_on_face(neighbor, "oak_fence", face));
    }
    assert!(!query.is_fence_connectable_on_face(
        BlockPos::new(16, 65, 8),
        "oak_fence",
        Direction::West
    ));
    assert!(!s.loaded_chunks().is_loaded(ChunkPos::new(1, 0)));
    set_world_block(&mut s, neighbor, u32::MAX);
    let query = ShardNeighborQuery {
        chunks: s.loaded_chunks(),
    };
    assert!(!query.is_fence_connectable_on_face(neighbor, "oak_fence", Direction::North));
}
