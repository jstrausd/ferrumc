//! Whole-generator admission and determinism tests for opaque bounded tails.

use std::path::{Path, PathBuf};

use ferrumc_proto_gen::{run, GenError, GenPaths, Mode};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crate has a repository ancestor")
}

fn manifest() -> PathBuf {
    repo_root().join("fixtures/protocol/1_21_8/manifest.toml")
}

fn synthetic_spec() -> String {
    let mut spec = std::fs::read_to_string(repo_root().join("docs/protocol/1_21_8/packets.toml"))
        .expect("read production spec");
    // These IDs belong only to this test schema, not to Minecraft's packet map.
    spec.push_str(
        r#"
[[packet]]
state = "play"
direction = "clientbound"
name = "BoundedTailProbe"
id = 0x7ffd
fields = [
    { name = "entity_id", type = "varint" },
    { name = "scalar", type = "i16" },
    { name = "payload", type = "bounded_remaining_bytes(4)" },
]

[[packet]]
state = "login"
direction = "clientbound"
name = "EmptyTailProbe"
id = 0x7ffe
fields = [
    { name = "tag", type = "u8" },
    { name = "payload", type = "bounded_remaining_bytes(0)" },
]

[[packet]]
state = "login"
direction = "clientbound"
name = "MaximumTailProbe"
id = 0x7fff
fields = [
    { name = "payload", type = "bounded_remaining_bytes(2097152)" },
]

[[packet]]
state = "login"
direction = "clientbound"
name = "SingleTailProbe"
id = 0x7ffc
fields = [
    { name = "payload", type = "bounded_remaining_bytes(1)" },
]
"#,
    );
    spec
}

#[test]
fn synthetic_full_output_is_byte_identical_and_checkable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_path = dir.path().join("packets.toml");
    std::fs::write(&spec_path, synthetic_spec()).expect("write synthetic spec");
    let first = GenPaths::new(manifest(), &spec_path, dir.path().join("first"));
    let second = GenPaths::new(manifest(), &spec_path, dir.path().join("second"));
    let first_report = run(&first, Mode::Write).expect("first generation");
    let second_report = run(&second, Mode::Write).expect("second generation");
    assert!(!first_report.is_up_to_date());
    assert_eq!(first_report.changed(), second_report.changed());
    for relative in first_report.changed() {
        assert_eq!(
            std::fs::read(first.generated_dir().join(relative)).expect("first output"),
            std::fs::read(second.generated_dir().join(relative)).expect("second output"),
            "{relative:?} must be byte-identical"
        );
    }
    assert!(run(&first, Mode::Write)
        .expect("repeat write")
        .is_up_to_date());
    assert!(run(&first, Mode::Check)
        .expect("first check")
        .is_up_to_date());
    assert!(run(&second, Mode::Check)
        .expect("second check")
        .is_up_to_date());
}

#[test]
fn invalid_bounded_tail_spec_never_creates_or_overwrites_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_path = dir.path().join("packets.toml");
    let existing_dir = dir.path().join("existing");
    std::fs::create_dir(&existing_dir).expect("existing output directory");
    let sentinel_path = existing_dir.join("mod.rs");
    let sentinel = b"// retained preexisting output\n";
    std::fs::write(&sentinel_path, sentinel).expect("write sentinel");
    let missing_dir = dir.path().join("missing");
    for fields in [
        "{ name = 'tail', type = 'bounded_remaining_bytes(2097153)' }",
        "{ name = 'tail', type = 'bounded_remaining_bytes(+1)' }",
        "{ name = 'tail', type = 'optional<bounded_remaining_bytes(4)>' }",
        "{ name = 'tail', type = 'bounded_remaining_bytes(4)' }, { name = 'later', type = 'u8' }",
        "{ name = 'legacy', type = 'remaining_bytes' }, { name = 'tail', type = 'bounded_remaining_bytes(4)' }",
        "{ name = 'legacy', type = 'optional<prefixed_array<remaining_bytes>>' }, { name = 'tail', type = 'bounded_remaining_bytes(4)' }",
    ] {
        // A valid packet before the invalid one proves admission is whole-spec.
        let raw = format!(
            "[[packet]]\nstate = 'status'\ndirection = 'clientbound'\nname = 'ValidFirst'\nid = 0\nfields = []\n\n\
             [[packet]]\nstate = 'play'\ndirection = 'clientbound'\nname = 'InvalidLast'\nid = 1\nfields = [{fields}]\n"
        );
        std::fs::write(&spec_path, raw).expect("invalid spec");
        for output in [&missing_dir, &existing_dir] {
            let paths = GenPaths::new(manifest(), &spec_path, output);
            for mode in [Mode::Write, Mode::Check] {
                assert!(matches!(
                    run(&paths, mode),
                    Err(GenError::PacketsInvalid(_))
                ));
            }
        }
        assert!(!missing_dir.exists());
        assert_eq!(std::fs::read(&sentinel_path).expect("sentinel"), sentinel);
        assert_eq!(
            std::fs::read_dir(&existing_dir)
                .expect("output entries")
                .count(),
            1
        );
    }
}
