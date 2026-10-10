//! Tests the actual build-time input admission, including malformed fixtures.

use std::path::PathBuf;

#[path = "../build/entity_type_codegen.rs"]
mod entity_type_codegen;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/fixtures/minecraft-1.21.8-original/entity-types.json")
}

fn fixture() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(fixture_path()).unwrap()).unwrap()
}

#[test]
fn admitted_original_fixture_emits_deterministically_in_wire_order() {
    let names = entity_type_codegen::load(&fixture_path()).unwrap();
    assert_eq!(names.len(), 151);
    assert_eq!(names[69], "minecraft:item");
    assert_eq!(names[149], "minecraft:player");
    let first = entity_type_codegen::emit(&names).unwrap();
    let bytes = serde_json::to_vec(&fixture()).unwrap();
    let reencoded = entity_type_codegen::parse(&bytes).unwrap();
    assert_eq!(entity_type_codegen::emit(&reencoded).unwrap(), first);
}

#[test]
fn parser_refuses_truncated_duplicate_unknown_and_over_limit_input() {
    for bytes in [b"".as_slice(), b"{", b"null", b"{} {}", b"NaN"] {
        assert!(entity_type_codegen::parse(bytes).is_err());
    }
    let original = std::fs::read_to_string(fixture_path()).unwrap();
    let duplicate = original.replacen(
        "\"entries\": {",
        "\"entries\": {\"minecraft:allay\":{\"protocol_id\":2},",
        1,
    );
    assert!(entity_type_codegen::parse(duplicate.as_bytes()).is_err());
    let mut extra = fixture();
    extra["entries"]["minecraft:cow"]["extra"] = serde_json::json!(1);
    assert!(entity_type_codegen::parse(&serde_json::to_vec(&extra).unwrap()).is_err());
    assert!(matches!(
        entity_type_codegen::parse(&vec![b' '; 64 * 1024 + 1]),
        Err(entity_type_codegen::BuildError::Oversized(_))
    ));
}

#[test]
fn parser_refuses_invalid_scope_default_names_ids_and_counts() {
    for change in [
        "registry",
        "registry_id",
        "default",
        "negative",
        "unknown",
        "duplicate",
        "bool",
        "float",
        "too_big",
        "missing",
        "player",
        "resource",
        "namespace",
        "extra",
    ] {
        let mut invalid = fixture();
        match change {
            "registry" => invalid["registry"] = serde_json::json!("minecraft:item"),
            "registry_id" => invalid["protocol_id"] = serde_json::json!(4),
            "default" => invalid["default"] = serde_json::json!("minecraft:cow"),
            "missing" => {
                invalid["entries"]
                    .as_object_mut()
                    .unwrap()
                    .remove("minecraft:cow");
            }
            "player" | "resource" | "namespace" => {
                let old = if change == "player" {
                    "minecraft:player"
                } else {
                    "minecraft:cow"
                };
                let value = invalid["entries"]
                    .as_object_mut()
                    .unwrap()
                    .remove(old)
                    .unwrap();
                let new = match change {
                    "player" => "minecraft:replacement",
                    "resource" => "minecraft:a/../b",
                    _ => "other:cow",
                };
                invalid["entries"][new] = value;
            }
            "extra" => invalid["extra"] = serde_json::json!(0),
            _ => {
                invalid["entries"]["minecraft:cow"]["protocol_id"] = match change {
                    "negative" => serde_json::json!(-1),
                    "unknown" => serde_json::json!(151),
                    "duplicate" => serde_json::json!(0),
                    "bool" => serde_json::json!(true),
                    "float" => serde_json::json!(28.0),
                    _ => serde_json::json!(i64::MAX),
                };
            }
        }
        assert!(
            entity_type_codegen::parse(&serde_json::to_vec(&invalid).unwrap()).is_err(),
            "{change}"
        );
    }
    assert!(entity_type_codegen::emit(&[]).is_err());
    assert!(entity_type_codegen::emit(&vec![String::from("minecraft:pig"); 151]).is_err());
}
