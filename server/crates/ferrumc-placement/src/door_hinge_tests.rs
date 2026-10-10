//! Independent golden observations from the pinned complete DoorBlock.getHinge.
//! These tests exercise the public placement entry point and both output halves;
//! they do not establish full collision-shape or live Paper parity.

use super::*;

const OAK_DOOR: u32 = 4697;
const STONE: u32 = 1;

// Direction, yaw and exact lower-half states from the pinned block snapshot.
// Upper halves are checked independently below through their named properties.
const FACINGS: [(Direction, f32, &str, u32, u32); 4] = [
    (Direction::South, 0.0, "south", 4713, 4717),
    (Direction::West, 90.0, "west", 4729, 4733),
    (Direction::North, 180.0, "north", 4697, 4701),
    (Direction::East, 270.0, "east", 4745, 4749),
];

#[derive(Default)]
struct ObservedNeighbors {
    states: Vec<(BlockPos, u32)>,
}

impl NeighborQuery for ObservedNeighbors {
    fn is_fence_connectable(&self, _position: BlockPos, _fence_block_name: &str) -> bool {
        false
    }

    fn block_state_at(&self, position: BlockPos) -> Option<u32> {
        self.states
            .iter()
            .find_map(|(pos, state)| (*pos == position).then_some(*state))
    }
}

fn context(yaw: f32, position: BlockPos, face: Direction, cursor: Vec3) -> PlacementContext {
    PlacementContext {
        item_block_state: OAK_DOOR,
        clicked_face: face,
        cursor_position: cursor,
        player_yaw: yaw,
        position,
    }
}

fn assert_door(ctx: &PlacementContext, neighbors: &dyn NeighborQuery, hinge: &str, facing: &str) {
    let placed = compute_placement(ctx, neighbors).expect("recognized door item");
    assert_eq!(placed.rule, PlacementRule::Door);
    assert_eq!(placed.requested_state, OAK_DOOR);
    assert_eq!(placed.place_at, None);
    assert_eq!(placed.extra_blocks.len(), 1);
    let (upper_pos, upper) = placed.extra_blocks[0];
    assert_eq!(upper_pos, ctx.position.offset(Direction::Up));
    for (state, half) in [(placed.state_id, "lower"), (upper, "upper")] {
        assert_eq!(state_id_to_block_name(state), Some("oak_door"));
        assert_eq!(state_property(state, "half"), Some(half));
        assert_eq!(state_property(state, "hinge"), Some(hinge), "{ctx:?}");
        assert_eq!(state_property(state, "facing"), Some(facing));
        assert_eq!(state_property(state, "open"), Some("false"));
        assert_eq!(state_property(state, "powered"), Some("false"));
    }
}

#[test]
fn floor_click_golden_matrix_covers_all_yaws_and_exact_center() {
    // Rows follow FACINGS; columns are the local x/z samples .25, .5, .75.
    // At exact .5 the source's strict inequalities always choose LEFT.
    const EXPECTED: [[&str; 3]; 4] = [
        ["right", "left", "left"],
        ["right", "left", "left"],
        ["left", "left", "right"],
        ["left", "left", "right"],
    ];
    for position in [BlockPos::new(7, 64, 9), BlockPos::new(-17, 64, -33)] {
        for (row, (_, yaw, facing, left_id, right_id)) in FACINGS.iter().enumerate() {
            for (column, sample) in [0.25, 0.5, 0.75].into_iter().enumerate() {
                let ctx = context(
                    *yaw,
                    position,
                    Direction::Up,
                    Vec3::new(sample, 1.0, sample),
                );
                let hinge = EXPECTED[row][column];
                assert_door(&ctx, &NoNeighbors, hinge, facing);
                assert_eq!(
                    compute_placement(&ctx, &NoNeighbors).unwrap().state_id,
                    if hinge == "left" { *left_id } else { *right_id }
                );
            }
        }
    }
}

#[test]
fn horizontal_hit_point_is_relative_to_the_clicked_cell_before_face_step() {
    // These are actual points on each clicked face. The placement cell has
    // already moved one block along that face, as use_item_on_target does.
    // The expected rows use Paper's world hit point minus the PLACEMENT cell.
    const CASES: [(Direction, Vec3, [&str; 4]); 4] = [
        (
            Direction::North,
            Vec3::new(0.5, 0.5, 0.0),
            ["left", "left", "left", "right"],
        ),
        (
            Direction::South,
            Vec3::new(0.5, 0.5, 1.0),
            ["left", "right", "left", "left"],
        ),
        (
            Direction::East,
            Vec3::new(1.0, 0.5, 0.5),
            ["right", "left", "left", "left"],
        ),
        (
            Direction::West,
            Vec3::new(0.0, 0.5, 0.5),
            ["left", "left", "right", "left"],
        ),
    ];
    for clicked in [BlockPos::new(11, 64, 23), BlockPos::new(-17, 64, -33)] {
        for (face, cursor, expected) in CASES {
            for (column, (_, yaw, facing, _, _)) in FACINGS.iter().enumerate() {
                let ctx = context(*yaw, clicked.offset(face), face, cursor);
                assert_door(&ctx, &NoNeighbors, expected[column], facing);
            }
        }
    }
}

fn all_door_defaults() -> Vec<(String, u32)> {
    let blocks: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../ferrumc-registry/data/blocks.json")).unwrap();
    blocks
        .into_iter()
        .filter_map(|block| {
            let name = block["name"].as_str()?;
            name.ends_with("_door").then(|| {
                (
                    name.to_owned(),
                    u32::try_from(block["defaultState"].as_u64().unwrap()).unwrap(),
                )
            })
        })
        .collect()
}

#[test]
fn any_registered_door_material_lower_half_drives_pairing() {
    let defaults = all_door_defaults();
    assert_eq!(
        defaults.len(),
        21,
        "pinned DoorBlock registration inventory"
    );
    for (name, state) in defaults {
        assert_eq!(state_property(state, "half"), Some("lower"), "{name}");
        for (direction, yaw, facing, _, _) in FACINGS {
            let ctx = context(
                yaw,
                BlockPos::new(0, 64, 0),
                Direction::Up,
                Vec3::new(0.5, 1.0, 0.5),
            );
            // A lone left neighbor forces RIGHT, a lone right neighbor LEFT.
            for (neighbor, expected) in [
                (rotate_counterclockwise(direction), "right"),
                (rotate_clockwise(direction), "left"),
            ] {
                let observed = ObservedNeighbors {
                    states: vec![(ctx.position.offset(neighbor), state)],
                };
                assert_door(&ctx, &observed, expected, facing);
            }
        }
    }
}

#[test]
fn upper_door_halves_are_not_lower_pairing_flags() {
    for (name, state) in all_door_defaults() {
        // All pinned doors order upper before lower; verify the fixture shape
        // explicitly so a future re-pin cannot silently alter this observation.
        let upper = state - 8;
        assert_eq!(state_id_to_block_name(upper), Some(name.as_str()));
        assert_eq!(state_property(upper, "half"), Some("upper"));
        for (direction, yaw, facing, _, _) in FACINGS {
            for side in [
                rotate_counterclockwise(direction),
                rotate_clockwise(direction),
            ] {
                let ctx = context(
                    yaw,
                    BlockPos::new(0, 64, 0),
                    Direction::Up,
                    Vec3::new(0.5, 1.0, 0.5),
                );
                let observed = ObservedNeighbors {
                    states: vec![(ctx.position.offset(side), upper)],
                };
                assert_door(&ctx, &observed, "left", facing);
            }
        }
    }
}

#[test]
fn full_stone_obstacle_weight_golden_cases_cover_minus_two_through_two() {
    // Four cells in order: left lower, left upper, right lower, right upper.
    const CASES: [([bool; 4], &str); 5] = [
        ([true, true, false, false], "left"),   // weight -2
        ([false, true, false, false], "left"),  // weight -1
        ([true, true, true, true], "left"),     // weight 0, cursor tie
        ([false, false, false, true], "right"), // weight +1
        ([false, false, true, true], "right"),  // weight +2
    ];
    for (direction, yaw, facing, _, _) in FACINGS {
        let ctx = context(
            yaw,
            BlockPos::new(0, 64, 0),
            Direction::Up,
            Vec3::new(0.5, 1.0, 0.5),
        );
        let left = ctx.position.offset(rotate_counterclockwise(direction));
        let right = ctx.position.offset(rotate_clockwise(direction));
        let cells = [
            left,
            left.offset(Direction::Up),
            right,
            right.offset(Direction::Up),
        ];
        for (occupied, expected) in CASES {
            let observed = ObservedNeighbors {
                states: cells
                    .iter()
                    .zip(occupied)
                    .filter_map(|(pos, full)| full.then_some((*pos, STONE)))
                    .collect(),
            };
            assert_door(&ctx, &observed, expected, facing);
        }
    }
}

#[test]
fn lower_door_flags_and_upper_obstacle_weights_use_source_branch_order() {
    // Flag order: neither, left only, right only, both. Upper obstacle order:
    // neither, left only, right only, both. These physically realizable cases
    // cover the lower-door preference versus negative/positive/tied weights.
    const EXPECTED: [[&str; 4]; 4] = [
        ["left", "left", "right", "left"],
        ["right", "right", "right", "right"],
        ["left", "left", "right", "left"],
        ["left", "left", "right", "left"],
    ];
    const FLAGS: [(bool, bool); 4] = [(false, false), (true, false), (false, true), (true, true)];
    const BIRCH_LOWER: u32 = 12856;
    const IRON_LOWER: u32 = 5839;
    for (direction, yaw, facing, _, _) in FACINGS {
        let ctx = context(
            yaw,
            BlockPos::new(0, 64, 0),
            Direction::Up,
            Vec3::new(0.5, 1.0, 0.5),
        );
        let left = ctx.position.offset(rotate_counterclockwise(direction));
        let right = ctx.position.offset(rotate_clockwise(direction));
        for (row, (left_door, right_door)) in FLAGS.into_iter().enumerate() {
            for (column, (left_full, right_full)) in FLAGS.into_iter().enumerate() {
                let mut observed = ObservedNeighbors::default();
                for (present, position, state) in [
                    (left_door, left, BIRCH_LOWER),
                    (right_door, right, IRON_LOWER),
                    (left_full, left.offset(Direction::Up), STONE),
                    (right_full, right.offset(Direction::Up), STONE),
                ] {
                    if present {
                        observed.states.push((position, state));
                    }
                }
                assert_door(&ctx, &observed, EXPECTED[row][column], facing);
            }
        }
    }
}
