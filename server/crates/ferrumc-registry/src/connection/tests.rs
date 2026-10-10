use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::{fence_family, is_connection_exception, FenceFamily};
use crate::block_state::{block_metadata, state_id_to_block_name};
use crate::{DATA_VERSION, MINECRAFT_VERSION};

const FIXTURES: [(&str, &[u8]); 4] = [
    (
        "fences.json",
        include_bytes!("../../data/connection-tags/fences.json"),
    ),
    (
        "wooden_fences.json",
        include_bytes!("../../data/connection-tags/wooden_fences.json"),
    ),
    (
        "leaves.json",
        include_bytes!("../../data/connection-tags/leaves.json"),
    ),
    (
        "shulker_boxes.json",
        include_bytes!("../../data/connection-tags/shulker_boxes.json"),
    ),
];

#[derive(serde::Deserialize)]
struct Tag {
    values: Vec<String>,
}

#[derive(serde::Deserialize)]
struct RawBlock {
    name: String,
    #[serde(rename = "minStateId")]
    min_state: u32,
    #[serde(rename = "maxStateId")]
    max_state: u32,
}

fn raw_blocks() -> Vec<RawBlock> {
    serde_json::from_str(include_str!("../../data/blocks.json"))
        .expect("independent vendored block snapshot parses")
}

fn tag(name: &str) -> Tag {
    let (_, bytes) = FIXTURES
        .iter()
        .find(|(fixture_name, _)| *fixture_name == name)
        .expect("tag fixture exists");
    serde_json::from_slice(bytes).expect("vendored Paper block tag parses")
}

fn direct_members(name: &str) -> BTreeSet<String> {
    let values = tag(name).values;
    let mut members = BTreeSet::new();
    for value in values {
        assert!(value.starts_with("minecraft:"), "direct member {value}");
        assert!(members.insert(value), "duplicate member in {name}");
    }
    members
}

fn fences() -> BTreeSet<String> {
    let mut members = BTreeSet::new();
    let mut wooden_references = 0;
    for value in tag("fences.json").values {
        if value == "#minecraft:wooden_fences" {
            wooden_references += 1;
            for member in direct_members("wooden_fences.json") {
                assert!(members.insert(member), "overlapping fence tag members");
            }
        } else {
            assert!(value.starts_with("minecraft:"), "unknown tag ref {value}");
            assert!(members.insert(value), "duplicate explicit fence member");
        }
    }
    assert_eq!(wooden_references, 1, "resolve the nested tag explicitly");
    members
}

fn exceptions() -> BTreeSet<String> {
    let mut members = direct_members("leaves.json");
    members.extend(direct_members("shulker_boxes.json"));
    // Reviewed Block.isExceptionForConnection's five explicit built-in blocks,
    // independent of the production constant lists.
    members.extend(
        [
            "minecraft:barrier",
            "minecraft:carved_pumpkin",
            "minecraft:jack_o_lantern",
            "minecraft:melon",
            "minecraft:pumpkin",
        ]
        .map(String::from),
    );
    members
}

#[test]
fn fixture_manifest_pins_source_and_authenticates_every_tag() {
    #[derive(serde::Deserialize)]
    struct Manifest {
        minecraft_version: String,
        data_version: i32,
        paper_build: u32,
        paper_commit: String,
        source_directory: String,
        files: Vec<FileEntry>,
    }
    #[derive(serde::Deserialize)]
    struct FileEntry {
        name: String,
        bytes: usize,
        sha256: String,
    }

    let manifest: Manifest =
        toml::from_str(include_str!("../../data/connection-tags/manifest.toml"))
            .expect("connection fact manifest parses");
    assert_eq!(manifest.minecraft_version, MINECRAFT_VERSION);
    assert_eq!(manifest.data_version, DATA_VERSION);
    assert_eq!(manifest.paper_build, 60);
    assert_eq!(
        manifest.paper_commit,
        "29c8822d90899c89d2689338e81a98f690bcba12"
    );
    assert_eq!(
        manifest.source_directory,
        "paper-server/src/minecraft/resources/data/minecraft/tags/block"
    );
    assert_eq!(manifest.files.len(), FIXTURES.len());

    let names: BTreeSet<_> = manifest
        .files
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(names.len(), manifest.files.len(), "no duplicate files");
    assert_eq!(
        names,
        FIXTURES
            .iter()
            .map(|(name, _)| *name)
            .collect::<BTreeSet<_>>()
    );
    for (name, bytes) in FIXTURES {
        let entry = manifest
            .files
            .iter()
            .find(|entry| entry.name == name)
            .expect("every vendored tag is covered by the manifest");
        assert_eq!(entry.bytes, bytes.len(), "byte-count drift for {name}");
        assert_eq!(
            entry.sha256,
            format!("{:x}", Sha256::digest(bytes)),
            "checksum drift for {name}"
        );
    }
}

#[test]
fn declared_facts_are_disjoint_complete_and_registered() {
    let wooden = direct_members("wooden_fences.json");
    let fence_members = fences();
    let leaves = direct_members("leaves.json");
    let shulker_boxes = direct_members("shulker_boxes.json");
    let exception_members = exceptions();
    assert_eq!(wooden.len(), 12);
    assert_eq!(fence_members.len(), 13);
    assert_eq!(leaves.len(), 11);
    assert_eq!(shulker_boxes.len(), 17);
    assert_eq!(exception_members.len(), 33);
    assert!(wooden.is_subset(&fence_members));
    assert_eq!(
        fence_members
            .difference(&wooden)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["minecraft:nether_brick_fence"]
    );
    assert!(leaves.is_disjoint(&shulker_boxes));
    assert!(fence_members.is_disjoint(&exception_members));

    let blocks = raw_blocks();
    let known_names: BTreeSet<_> = blocks.iter().map(|block| block.name.as_str()).collect();
    assert_eq!(blocks.len(), 1105);
    assert_eq!(known_names.len(), blocks.len());
    for member in fence_members.union(&exception_members) {
        let bare = member.strip_prefix("minecraft:").expect("canonical member");
        assert!(known_names.contains(bare), "unregistered fact {member}");
    }
}

#[test]
fn every_registered_block_matches_reference_fence_tag_membership() {
    let wooden = direct_members("wooden_fences.json");
    let fence_members = fences();
    for block in raw_blocks() {
        let namespaced = format!("minecraft:{}", block.name);
        let expected = if !fence_members.contains(&namespaced) {
            None
        } else if wooden.contains(&namespaced) {
            Some(FenceFamily::Wooden)
        } else {
            Some(FenceFamily::NetherBrick)
        };
        assert_eq!(fence_family(&block.name), expected, "bare {}", block.name);
        assert_eq!(fence_family(&namespaced), expected, "{namespaced}");
    }
}

#[test]
fn every_registered_block_matches_reference_exception_membership() {
    let expected_members = exceptions();
    for block in raw_blocks() {
        let namespaced = format!("minecraft:{}", block.name);
        let expected = Some(expected_members.contains(&namespaced));
        assert_eq!(
            is_connection_exception(&block.name),
            expected,
            "bare {}",
            block.name
        );
        assert_eq!(
            is_connection_exception(&namespaced),
            expected,
            "{namespaced}"
        );
    }
}

#[test]
fn nether_wood_is_wooden_while_nether_brick_and_gates_are_separate() {
    for name in ["crimson_fence", "warped_fence"] {
        assert_eq!(fence_family(name), Some(FenceFamily::Wooden));
    }
    assert_eq!(
        fence_family("nether_brick_fence"),
        Some(FenceFamily::NetherBrick)
    );
    for name in [
        "oak_fence_gate",
        "crimson_fence_gate",
        "warped_fence_gate",
        "nether_bricks",
        "iron_bars",
        "cobblestone_wall",
    ] {
        assert!(block_metadata(name).is_some(), "known control {name}");
        assert_eq!(fence_family(name), None, "{name}");
        assert_eq!(is_connection_exception(name), Some(false), "{name}");
    }
}

#[test]
fn known_nonexceptions_do_not_imply_any_face_support() {
    for name in [
        "stone",
        "glass",
        "water",
        "air",
        "oak_slab",
        "oak_stairs",
        "oak_trapdoor",
        "snow",
        "powder_snow",
    ] {
        assert_eq!(is_connection_exception(name), Some(false), "{name}");
        assert_eq!(fence_family(name), None, "{name}");
    }
}

#[test]
fn unknown_names_and_fake_suffixes_never_become_known_facts() {
    for name in [
        "",
        "minecraft:",
        "Minecraft:oak_fence",
        "minecraft:Oak_fence",
        "minecraft:minecraft:oak_fence",
        "example:oak_fence",
        "example:stone",
        " oak_fence",
        "oak_fence ",
        "oak_fence\n",
        "oak_fence\0",
        "oak:fence",
        "minecraft:oak/fence",
        "fake_fence",
        "fake_leaves",
        "fake_shulker_box",
        "nether_brick_fence_gate",
        "stone[axis=y]",
    ] {
        assert_eq!(fence_family(name), None, "{name:?}");
        assert_eq!(is_connection_exception(name), None, "{name:?}");
    }
}

#[test]
fn every_connection_fact_state_range_reverse_maps_to_its_registered_owner() {
    let fence_members = fences();
    let wooden = direct_members("wooden_fences.json");
    let exception_members = exceptions();
    for block in raw_blocks() {
        let namespaced = format!("minecraft:{}", block.name);
        if !fence_members.contains(&namespaced) && !exception_members.contains(&namespaced) {
            continue;
        }
        let expected_family = if wooden.contains(&namespaced) {
            Some(FenceFamily::Wooden)
        } else if fence_members.contains(&namespaced) {
            Some(FenceFamily::NetherBrick)
        } else {
            None
        };
        let expected_exception = Some(exception_members.contains(&namespaced));
        assert!(block.min_state <= block.max_state);
        for state in block.min_state..=block.max_state {
            let owner = state_id_to_block_name(state).expect("fact's state is registered");
            assert_eq!(owner, block.name, "owner of state {state}");
            assert_eq!(fence_family(owner), expected_family, "state {state}");
            assert_eq!(
                is_connection_exception(owner),
                expected_exception,
                "state {state}"
            );
        }
    }
}
