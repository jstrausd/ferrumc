use crate::{compute_placement, NoNeighbors, PlacementContext, PlacementRule};
use ferrumc_math::{BlockPos, Direction, Vec3};
use ferrumc_registry::block_state::{block_metadata, state_id_to_block_name};

fn context(item: u32, face: Direction, yaw: f32) -> PlacementContext {
    PlacementContext {
        item_block_state: item,
        clicked_face: face,
        cursor_position: Vec3::new(0.5, 1.0, 0.5),
        player_yaw: yaw,
        position: BlockPos::new(7, 64, 10),
    }
}

#[test]
fn standing_wall_all_families_both_reverse_aliases_and_yaw_boundaries() {
    let fixture = include_str!("../../tests/fixtures/standing-wall-placement-1.21.8.tsv");
    let mut rows = 0;
    for row in fixture.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 5);
        let item: u32 = fields[1].parse().unwrap();
        assert_eq!(state_id_to_block_name(item), Some(fields[0]));
        let face = match fields[2] {
            "up" => Direction::Up,
            "north" => Direction::North,
            "south" => Direction::South,
            "east" => Direction::East,
            "west" => Direction::West,
            invalid => panic!("invalid frozen face {invalid}"),
        };
        let ctx = context(item, face, fields[3].parse().unwrap());
        let actual = compute_placement(&ctx, &NoNeighbors).unwrap();
        let expected: u32 = fields[4].parse().unwrap();
        assert_eq!(
            actual.state_id, expected,
            "source-backed independent row {row}"
        );
        assert_eq!(actual.rule, PlacementRule::StandingAndWall);
        assert_eq!(actual.requested_state, item);
        assert_eq!(actual.place_at, None);
        assert_eq!(actual.extra_blocks.len(), 0);
        rows += 1;
    }
    assert_eq!(
        rows, 472,
        "all 23 paired families plus signed half-turn ties"
    );
}

#[test]
fn standing_wall_preserves_unrepresented_ceiling_and_nonfinite_fallback() {
    for name in [
        "white_banner",
        "white_wall_banner",
        "skeleton_skull",
        "skeleton_wall_skull",
    ] {
        let item = block_metadata(name).unwrap().default_state;
        let actual =
            compute_placement(&context(item, Direction::Down, 90.0), &NoNeighbors).unwrap();
        assert_eq!(actual.state_id, item);
        assert_eq!(actual.rule, PlacementRule::SimpleCube);
        for yaw in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let actual =
                compute_placement(&context(item, Direction::Up, yaw), &NoNeighbors).unwrap();
            assert_eq!(actual.state_id, item);
            assert_eq!(actual.rule, PlacementRule::SimpleCube);
        }
    }
}

#[test]
fn decorated_pot_faces_the_player_direction_and_resets_fresh_properties() {
    // Independently encoded1.21.8 fresh dry pot states N/E/S/W, not furnace-facing.
    let item = block_metadata("decorated_pot").unwrap().default_state;
    for (yaw, expected) in [(0.0, 27677), (90.0, 27679), (180.0, 27675), (-90.0, 27681)] {
        let actual = compute_placement(&context(item, Direction::Up, yaw), &NoNeighbors).unwrap();
        assert_eq!(actual.state_id, expected);
        assert_eq!(actual.rule, PlacementRule::HorizontalFacing);
        assert_eq!(actual.extra_blocks.len(), 0);
    }
    // A nondefault requested cracked/wet state is still a fresh dry/uncracked pot.
    let actual = compute_placement(&context(27666, Direction::East, 0.0), &NoNeighbors).unwrap();
    assert_eq!(actual.state_id, 27677);
}

#[test]
fn decorated_pot_keeps_common_source_waterlogging_and_unrelated_defaults() {
    struct Water;
    impl crate::NeighborQuery for Water {
        fn is_fence_connectable(&self, _position: BlockPos, _name: &str) -> bool {
            false
        }
        fn block_state_at(&self, _position: BlockPos) -> Option<u32> {
            Some(86)
        }
    }
    let item = block_metadata("decorated_pot").unwrap().default_state;
    let actual = compute_placement(&context(item, Direction::Up, 0.0), &Water).unwrap();
    assert_eq!(actual.state_id, 27676, "fresh south pot with source water");
    for name in ["stone", "oak_sign", "rail", "redstone_wire"] {
        let item = block_metadata(name).unwrap().default_state;
        let actual =
            compute_placement(&context(item, Direction::East, 90.0), &NoNeighbors).unwrap();
        assert_eq!(actual.state_id, item, "unrelated default {name}");
        assert_eq!(actual.rule, PlacementRule::SimpleCube);
    }
}
