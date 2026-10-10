//! Public identity contracts and independent original-data integrity guards.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ferrumc_registry::entity_type::{
    entity_types, lookup_entity_type, lookup_entity_type_name, EntityTypeId, ENTITY_TYPE_COUNT,
};
use sha2::{Digest as _, Sha256};

fn original(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/fixtures/minecraft-1.21.8-original")
        .join(name)
}

#[test]
fn all_original_names_and_wire_ids_match_the_exported_oracle() {
    let oracle = include_str!("fixtures/entity_types_1_21_8.tsv");
    let mut lines = oracle.lines();
    assert_eq!(
        lines.next(),
        Some("protocol_id\tresource_location\tis_player")
    );
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut player_count = 0;
    for (expected_index, line) in lines.enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 3);
        let wire_id: i32 = fields[0].parse().unwrap();
        assert_eq!(wire_id, i32::try_from(expected_index).unwrap());
        let canonical = fields[1];
        let bare = canonical.strip_prefix("minecraft:").unwrap();
        let by_id = EntityTypeId::try_from_protocol_id(wire_id).unwrap();
        assert_eq!(by_id.protocol_id(), wire_id);
        assert_eq!(by_id.resource_location(), canonical);
        assert_eq!(lookup_entity_type_name(wire_id), Some(canonical));
        assert_eq!(lookup_entity_type(canonical), Some(by_id));
        assert_eq!(EntityTypeId::from_name(bare), Some(by_id));
        assert_eq!(by_id.is_player().to_string(), fields[2]);
        if by_id.is_player() {
            assert_eq!(canonical, "minecraft:player");
            player_count += 1;
        }
        assert!(names.insert(canonical));
        assert!(ids.insert(by_id));
    }
    assert_eq!(names.len(), 151);
    assert_eq!(ids.len(), ENTITY_TYPE_COUNT);
    assert_eq!(player_count, 1);
    assert_eq!(entity_types().collect::<BTreeSet<_>>(), ids);
}

#[test]
fn known_different_entity_families_and_player_are_classified() {
    for (name, wire_id) in [
        ("acacia_boat", 0),
        ("arrow", 6),
        ("cow", 28),
        ("item", 69),
        ("lightning_bolt", 75),
        ("tnt", 127),
        ("zombie", 145),
        ("player", 149),
        ("fishing_bobber", 150),
    ] {
        let registered = lookup_entity_type(name).unwrap();
        assert_eq!(registered.protocol_id(), wire_id);
        assert_eq!(registered.is_player(), name == "player");
    }
}

#[test]
fn unknown_and_malformed_ids_and_names_never_fall_back_to_pig() {
    for id in [i32::MIN, -151, -1, 151, 152, 999, i32::MAX] {
        assert_eq!(EntityTypeId::try_from_protocol_id(id), None);
        assert_eq!(lookup_entity_type_name(id), None);
    }
    for name in [
        "",
        "minecraft:",
        ":cow",
        "other:cow",
        "Minecraft:cow",
        "minecraft:COW",
        "minecraft:minecraft:cow",
        "minecraft:cow:extra",
        "minecraft:../cow",
        "minecraft:cow/../cow",
        "minecraft://cow",
        " cow",
        "cow ",
        "cow\n",
        "cow\0",
        "cöw",
        "minecraft:entity_type",
        "definitely_not_an_entity",
    ] {
        assert_eq!(lookup_entity_type(name), None, "{name:?}");
        assert_eq!(EntityTypeId::from_name(name), None, "{name:?}");
    }
    assert!(lookup_entity_type("pig").is_some());
}

#[test]
fn static_iteration_and_bounded_pagination_preserve_protocol_order() {
    let mut all = entity_types();
    assert_eq!(all.len(), 151);
    assert_eq!(all.next().unwrap().protocol_id(), 0);
    assert_eq!(all.next_back().unwrap().protocol_id(), 150);
    assert_eq!(all.len(), 149);
    assert_eq!(
        entity_types()
            .skip(69)
            .take(1)
            .next()
            .unwrap()
            .protocol_id(),
        69
    );
    assert_eq!(entity_types().skip(149).take(8).count(), 2);
    assert_eq!(entity_types().take(0).count(), 0);
    assert_eq!(entity_types().skip(151).count(), 0);
    all.for_each(|_| {});
    let mut exhausted = entity_types().skip(151);
    assert_eq!(exhausted.next(), None);
    assert_eq!(exhausted.next(), None);
}

#[test]
fn oracle_manifest_and_build_source_match_independent_pins() {
    for (bytes, length, hash) in [
        (
            include_bytes!("fixtures/entity_types_1_21_8.tsv").as_slice(),
            4566,
            "e56e7322b59e20ef74447fc0a7237f7784fe3455b308a2c4ebe7a1277bafa15d",
        ),
        (
            include_bytes!("fixtures/entity_types_1_21_8.manifest.json").as_slice(),
            2332,
            "08540db5110ea7cd7ad13d2a9f5d53bbf1bfe87747bef09563194aa58a4d6dab",
        ),
    ] {
        assert_eq!(bytes.len(), length);
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), hash);
    }
    let fixture = std::fs::read(original("entity-types.json")).unwrap();
    assert_eq!(fixture.len(), 9319);
    assert_eq!(
        format!("{:x}", Sha256::digest(&fixture)),
        "b65a61ac9f4346ad74ff159cb3f8be78ec96229ccfc3abe29a14a7e0063d4d6b"
    );
    let parsed: serde_json::Value = serde_json::from_slice(&fixture).unwrap();
    let entries = parsed["entries"].as_object().unwrap();
    assert_eq!(entries.len(), ENTITY_TYPE_COUNT);
    for (name, entry) in entries {
        assert_eq!(
            lookup_entity_type(name).unwrap().protocol_id(),
            i32::try_from(entry["protocol_id"].as_i64().unwrap()).unwrap()
        );
    }
}
