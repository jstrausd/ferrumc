use std::collections::{BTreeMap, BTreeSet};

use super::{
    compute_fence_connection_state, fence_connects_to, recompute_fence_connection_state,
    state_property, NeighborQuery, CARDINALS,
};
use ferrumc_math::{BlockPos, Direction};
use ferrumc_registry::block_state::compute_state_id;

#[derive(serde::Deserialize)]
struct RawBlock {
    name: String,
    #[serde(rename = "defaultState")]
    default_state: u32,
    #[serde(rename = "minStateId")]
    min_state: u32,
    #[serde(rename = "maxStateId")]
    max_state: u32,
    states: Vec<RawProperty>,
}

#[derive(serde::Deserialize)]
struct RawProperty {
    name: String,
    #[serde(rename = "type")]
    typ: String,
    num_values: usize,
    #[serde(default)]
    values: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct RawTag {
    values: Vec<String>,
}

fn raw_blocks() -> Vec<RawBlock> {
    serde_json::from_str(include_str!("../../ferrumc-registry/data/blocks.json"))
        .expect("independent vendored registry snapshot parses")
}

fn block<'a>(blocks: &'a [RawBlock], name: &str) -> &'a RawBlock {
    blocks
        .iter()
        .find(|block| block.name == name)
        .expect("reference block is in the raw snapshot")
}

fn bare_tag_names(raw: &str) -> Vec<String> {
    let tag: RawTag = serde_json::from_str(raw).expect("pinned reference tag parses");
    tag.values
        .into_iter()
        .map(|name| {
            name.strip_prefix("minecraft:")
                .expect("direct canonical tag member")
                .to_owned()
        })
        .collect()
}

fn reference_fences() -> Vec<(String, bool)> {
    let mut fences: Vec<_> = bare_tag_names(include_str!(
        "../../ferrumc-registry/data/connection-tags/wooden_fences.json"
    ))
    .into_iter()
    .map(|name| (name, true))
    .collect();
    assert_eq!(fences.len(), 12);
    fences.push(("nether_brick_fence".to_owned(), false));
    fences
}

fn reference_exceptions() -> Vec<String> {
    let mut names = bare_tag_names(include_str!(
        "../../ferrumc-registry/data/connection-tags/leaves.json"
    ));
    names.extend(bare_tag_names(include_str!(
        "../../ferrumc-registry/data/connection-tags/shulker_boxes.json"
    )));
    names.extend(
        [
            "barrier",
            "carved_pumpkin",
            "jack_o_lantern",
            "melon",
            "pumpkin",
        ]
        .map(String::from),
    );
    assert_eq!(names.len(), 33);
    names
}

#[test]
fn every_fence_family_pair_uses_tag_membership_for_all_neighbor_states() {
    let blocks = raw_blocks();
    let fences = reference_fences();
    for (source, source_is_wooden) in &fences {
        for (neighbor, neighbor_is_wooden) in &fences {
            let raw = block(&blocks, neighbor);
            for state in raw.min_state..=raw.max_state {
                for face in CARDINALS {
                    assert_eq!(
                        fence_connects_to(source, state, face, false),
                        Some(source_is_wooden == neighbor_is_wooden),
                        "{source} -> {neighbor} state {state} {face:?}"
                    );
                    // Preserve the literal caller fact even for unlike families.
                    assert_eq!(fence_connects_to(source, state, face, true), Some(true));
                }
            }
        }
    }
    let warped = block(&blocks, "warped_fence").default_state;
    assert_eq!(
        fence_connects_to("minecraft:crimson_fence", warped, Direction::West, false),
        Some(true)
    );
}

#[test]
fn raw_gate_set_and_facing_schema_match_reviewed_class_registrations() {
    // Independent names reviewed in the pinned Blocks FenceGateBlock
    // registrations; the production set is deliberately not imported here.
    let expected: BTreeSet<_> = [
        "oak_fence_gate",
        "spruce_fence_gate",
        "birch_fence_gate",
        "jungle_fence_gate",
        "acacia_fence_gate",
        "cherry_fence_gate",
        "dark_oak_fence_gate",
        "pale_oak_fence_gate",
        "mangrove_fence_gate",
        "bamboo_fence_gate",
        "crimson_fence_gate",
        "warped_fence_gate",
    ]
    .into_iter()
    .collect();
    let blocks = raw_blocks();
    let gates: Vec<_> = blocks
        .iter()
        .filter(|block| block.name.ends_with("_fence_gate"))
        .collect();
    assert_eq!(gates.len(), 12);
    assert_eq!(
        gates
            .iter()
            .map(|gate| gate.name.as_str())
            .collect::<BTreeSet<_>>(),
        expected
    );
    for gate in gates {
        assert_eq!(gate.max_state - gate.min_state + 1, 32);
        assert_eq!(gate.states.len(), 4);
        let facing = &gate.states[0];
        assert_eq!(facing.name, "facing");
        assert_eq!(facing.typ, "enum");
        assert_eq!(facing.num_values, 4);
        assert_eq!(
            facing.values.as_deref().expect("gate has facing values"),
            ["north", "south", "west", "east"]
        );
        for (property, expected_name) in gate.states[1..].iter().zip(["in_wall", "open", "powered"])
        {
            assert_eq!(property.name, expected_name);
            assert_eq!(property.typ, "bool");
            assert_eq!(property.num_values, 2);
        }
    }
}

#[test]
fn all_gate_states_follow_perpendicular_faces_independent_of_open_wall_and_power() {
    let blocks = raw_blocks();
    let sources = reference_fences();
    for gate in blocks
        .iter()
        .filter(|block| block.name.ends_with("_fence_gate"))
    {
        let facings = gate.states[0]
            .values
            .as_ref()
            .expect("fixture facing values");
        for (index, facing) in facings.iter().enumerate() {
            // The raw schema above pins facing first and the three booleans
            // afterwards: these eight independent fixture states cover every
            // open/in_wall/powered combination for a facing.
            let first = gate.min_state + index as u32 * 8;
            for state in first..first + 8 {
                for face in CARDINALS {
                    let expected = match facing.as_str() {
                        "north" | "south" => matches!(face, Direction::East | Direction::West),
                        "east" | "west" => matches!(face, Direction::North | Direction::South),
                        other => panic!("unexpected raw gate facing {other}"),
                    };
                    for (source, _) in &sources {
                        assert_eq!(
                            fence_connects_to(source, state, face, false),
                            Some(expected),
                            "{source} -> {} {facing} state {state} {face:?}",
                            gate.name
                        );
                        // A parallel gate cannot discard the generic support
                        // branch when a caller explicitly supplies true.
                        assert_eq!(fence_connects_to(source, state, face, true), Some(true));
                    }
                }
            }
        }
    }
}

#[test]
fn every_pinned_connection_exception_blocks_the_full_support_branch() {
    let blocks = raw_blocks();
    for exception in reference_exceptions() {
        let raw = block(&blocks, &exception);
        for state in [raw.min_state, raw.default_state, raw.max_state] {
            for source in ["oak_fence", "nether_brick_fence"] {
                for face in CARDINALS {
                    for supplied_full_support in [false, true] {
                        assert_eq!(
                            fence_connects_to(source, state, face, supplied_full_support),
                            Some(false),
                            "{exception} state {state} {source} {face:?} {supplied_full_support}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn full_face_fact_is_applied_literally_to_known_nonexceptions() {
    let blocks = raw_blocks();
    for name in [
        "stone",
        "glass",
        "air",
        "water",
        "oak_slab",
        "oak_stairs",
        "iron_bars",
        "cobblestone_wall",
        "oak_trapdoor",
        "furnace",
    ] {
        let state = block(&blocks, name).default_state;
        for face in CARDINALS {
            for full in [false, true] {
                assert_eq!(
                    fence_connects_to("oak_fence", state, face, full),
                    Some(full)
                );
            }
        }
    }
    // A horizontal-facing property alone never confers FenceGateBlock identity.
    let furnace = block(&blocks, "furnace").default_state;
    assert_eq!(
        fence_connects_to("oak_fence", furnace, Direction::East, false),
        Some(false)
    );
}

#[test]
fn unknown_nonfence_sources_states_and_vertical_faces_are_rejected() {
    let blocks = raw_blocks();
    let oak = block(&blocks, "oak_fence").default_state;
    for full in [false, true] {
        for source in [
            "",
            "stone",
            "oak_fence_gate",
            "iron_bars",
            "fake_fence",
            "example:oak_fence",
            "Minecraft:oak_fence",
            "minecraft:minecraft:oak_fence",
            "oak_fence ",
        ] {
            assert_eq!(fence_connects_to(source, oak, Direction::North, full), None);
        }
        assert_eq!(
            fence_connects_to("oak_fence", u32::MAX, Direction::North, full),
            None
        );
        for state in [oak, block(&blocks, "oak_fence_gate").default_state] {
            for face in [Direction::Up, Direction::Down] {
                assert_eq!(fence_connects_to("oak_fence", state, face, full), None);
            }
        }
    }
}

#[test]
fn fresh_and_existing_fences_query_each_neighbors_face_towards_the_fence() {
    struct FaceQuery {
        fence_position: BlockPos,
    }
    impl NeighborQuery for FaceQuery {
        fn is_fence_connectable(&self, _: BlockPos, _: &str) -> bool {
            panic!("directional override must be used");
        }

        fn is_fence_connectable_on_face(
            &self,
            position: BlockPos,
            fence_name: &str,
            neighbor_face: Direction,
        ) -> bool {
            assert_eq!(fence_name, "oak_fence");
            assert_eq!(
                position,
                self.fence_position.offset(neighbor_face.opposite())
            );
            neighbor_face == Direction::West
        }
    }
    let position = BlockPos::new(15, 64, -16);
    let neighbors = FaceQuery {
        fence_position: position,
    };
    let fresh = compute_fence_connection_state("oak_fence", position, &neighbors)
        .expect("fresh directional fence");
    let mut properties = BTreeMap::new();
    properties.insert("waterlogged", "true");
    properties.insert("west", "true");
    let wet = compute_state_id("oak_fence", &properties).expect("existing wet fence");
    let updated = recompute_fence_connection_state(wet, position, &neighbors)
        .expect("existing directional fence");
    for state in [fresh, updated] {
        for direction in CARDINALS {
            let expected = if direction == Direction::East {
                "true"
            } else {
                "false"
            };
            assert_eq!(
                state_property(state, super::facing_string(direction)),
                Some(expected)
            );
        }
    }
    assert_eq!(state_property(fresh, "waterlogged"), Some("false"));
    assert_eq!(state_property(updated, "waterlogged"), Some("true"));
}

#[test]
fn legacy_query_implementations_keep_their_connection_behavior() {
    struct LegacyQuery {
        north_neighbor: BlockPos,
    }
    impl NeighborQuery for LegacyQuery {
        fn is_fence_connectable(&self, position: BlockPos, _: &str) -> bool {
            position == self.north_neighbor
        }
    }
    let position = BlockPos::new(-16, 64, 15);
    let legacy = LegacyQuery {
        north_neighbor: position.offset(Direction::North),
    };
    let state = compute_fence_connection_state("oak_fence", position, &legacy)
        .expect("legacy query still works");
    for direction in CARDINALS {
        let expected = if direction == Direction::North {
            "true"
        } else {
            "false"
        };
        assert_eq!(
            state_property(state, super::facing_string(direction)),
            Some(expected)
        );
    }
}
