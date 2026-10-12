//! Malformed-definition inputs (private synthetic compiles), separate from
//! original behaviour and policy: JSON admission, codec shape, references and
//! their refusal order.

use super::super::compile::{compile_synthetic, Caps};
use super::limits::{biome, block, condition, gradient, noise, sequence};
use super::*;

fn refuse(json: &str) -> SurfaceRuleError {
    compile_synthetic(json, Caps::PRODUCTION).unwrap_err()
}

fn kind_of(json: &str) -> SurfaceRuleErrorKind {
    refuse(json).kind().clone()
}

fn stone() -> String {
    block("minecraft:stone")
}

#[test]
fn json_syntax_and_admission_limits() {
    use SurfaceRuleErrorKind::{MalformedDefinition as M, NativeLimit as N};
    for source in [
        "",
        "{",
        "[1,]",
        "{\"type\":\"minecraft:block\"",
        "nul",
        "1 2",
        "{\"a\":1,\"a\":2}",
    ] {
        assert_eq!(kind_of(source), M, "{source:?}");
    }
    let long = format!("\"{}\"", "x".repeat(257));
    assert_eq!(
        refuse(&format!(r#"{{"type":{long}}}"#)).limit(),
        Some(SurfaceRuleLimit::StringBytes)
    );
    let entries = vec!["\"minecraft:plains\""; 257].join(",");
    let wide = format!(
        r#"{{"type":"minecraft:condition","if_true":{{"type":"minecraft:biome","biome_is":[{entries}]}},"then_run":{}}}"#,
        stone()
    );
    assert_eq!(
        refuse(&wide).limit(),
        Some(SurfaceRuleLimit::ContainerEntries)
    );
    let many = sequence(&[
        sequence(&vec![stone(); 200]),
        sequence(&vec![stone(); 200]),
        sequence(&vec![stone(); 200]),
    ]);
    let error = refuse(&many);
    assert_eq!(*error.kind(), N);
    assert_eq!(error.limit(), Some(SurfaceRuleLimit::JsonNodes));
    let deep = format!("{}1{}", "[".repeat(33), "]".repeat(33));
    assert_eq!(refuse(&deep).limit(), Some(SurfaceRuleLimit::JsonDepth));
    let huge = format!(
        "[{}]",
        " ".repeat(ferrumc_registry::worldgen::MAX_DEFINITION_BYTES)
    );
    assert_eq!(
        refuse(&huge).limit(),
        Some(SurfaceRuleLimit::DefinitionBytes)
    );
}

#[test]
#[allow(clippy::too_many_lines)] // one declared table of shape cases
fn codec_shape_types_fields_and_ranges() {
    use SurfaceRuleErrorKind::{
        InvalidAnchor as A, MalformedDefinition as M, UnknownCodecType as U,
    };
    let steep = r#"{"type":"minecraft:steep"}"#;
    let cases: Vec<(String, SurfaceRuleErrorKind)> = vec![
        (r#"{"type":"minecraft:unknown"}"#.to_owned(), U),
        (
            r#"{"type":"other:block","result_state":{"Name":"minecraft:stone"}}"#.to_owned(),
            U,
        ),
        (
            condition(
                r#"{"type":"minecraft:block","result_state":{"Name":"minecraft:stone"}}"#,
                &stone(),
            ),
            U,
        ),
        (condition(r#"{"type":"minecraft:unknown"}"#, &stone()), U),
        (
            r#"{"type":"minecraft:sequence","sequence":{}}"#.to_owned(),
            M,
        ),
        (r#"{"type":"minecraft:sequence"}"#.to_owned(), M),
        (r#"{"type":"minecraft:bandlands","extra":1}"#.to_owned(), M),
        (r#"{"sequence":[]}"#.to_owned(), M),
        (r#"{"type":7}"#.to_owned(), M),
        ("[]".to_owned(), M),
        (
            condition(
                steep,
                r#"{"type":"minecraft:block","result_state":{"Name":"minecraft:stone"},"x":0}"#,
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:biome","biome_is":"minecraft:plains"}"#,
                &stone(),
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:noise_threshold","noise":"minecraft:surface","min_threshold":"0","max_threshold":1}"#,
                &stone(),
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:noise_threshold","noise":"minecraft:surface","min_threshold":0,"max_threshold":1e400}"#,
                &stone(),
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":{"absolute":0},"surface_depth_multiplier":21,"add_stone_depth":false}"#,
                &stone(),
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:water","offset":0,"surface_depth_multiplier":-21,"add_stone_depth":false}"#,
                &stone(),
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:water","offset":0,"surface_depth_multiplier":0,"add_stone_depth":1}"#,
                &stone(),
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:stone_depth","offset":0,"add_surface_depth":false,"secondary_depth_range":0,"surface_type":"wall"}"#,
                &stone(),
            ),
            M,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":{"absolute":2032},"surface_depth_multiplier":0,"add_stone_depth":false}"#,
                &stone(),
            ),
            A,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":{"below_top":-2033},"surface_depth_multiplier":0,"add_stone_depth":false}"#,
                &stone(),
            ),
            A,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":{"absolute":0,"below_top":0},"surface_depth_multiplier":0,"add_stone_depth":false}"#,
                &stone(),
            ),
            A,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":{},"surface_depth_multiplier":0,"add_stone_depth":false}"#,
                &stone(),
            ),
            A,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":{"top":0},"surface_depth_multiplier":0,"add_stone_depth":false}"#,
                &stone(),
            ),
            A,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":{"absolute":"0"},"surface_depth_multiplier":0,"add_stone_depth":false}"#,
                &stone(),
            ),
            A,
        ),
        (
            condition(
                r#"{"type":"minecraft:y_above","anchor":5,"surface_depth_multiplier":0,"add_stone_depth":false}"#,
                &stone(),
            ),
            A,
        ),
    ];
    for (json, expected) in cases {
        assert_eq!(kind_of(&json), expected, "{json}");
    }
    // Inclusive original ranges are admitted.
    for json in [
        condition(
            r#"{"type":"minecraft:y_above","anchor":{"absolute":2031},"surface_depth_multiplier":20,"add_stone_depth":false}"#,
            &stone(),
        ),
        condition(
            r#"{"type":"minecraft:y_above","anchor":{"above_bottom":-2032},"surface_depth_multiplier":-20,"add_stone_depth":true}"#,
            &stone(),
        ),
        condition(
            r#"{"type":"minecraft:stone_depth","offset":-5,"add_surface_depth":true,"secondary_depth_range":30,"surface_type":"ceiling"}"#,
            &stone(),
        ),
        sequence(&[]),
        r#"{"type":"bandlands"}"#.to_owned(),
    ] {
        assert!(compile_synthetic(&json, Caps::PRODUCTION).is_ok(), "{json}");
    }
}

#[test]
fn references_resolve_strictly_and_report_their_node() {
    use SurfaceRuleErrorKind::{
        InvalidBlockState as B, MalformedDefinition as M, UnknownBiome as Bi, UnknownNoise as N,
    };
    let props = |p: &str| {
        format!(
            r#"{{"type":"minecraft:block","result_state":{{"Name":"minecraft:grass_block","Properties":{p}}}}}"#
        )
    };
    let cases = [
        (block("minecraft:not_a_block"), B),
        (block("other:stone"), B),
        (props(r#"{"snowy":"maybe"}"#), B),
        (props(r#"{"facing":"north"}"#), B),
        (props(r#"{"snowy":true}"#), B),
        (
            r#"{"type":"minecraft:block","result_state":{"Name":"minecraft:stone","Extra":1}}"#
                .to_owned(),
            B,
        ),
        (condition(&biome(&["minecraft:not_a_biome"]), &stone()), Bi),
        (condition(&noise("minecraft:not_a_noise"), &stone()), N),
        (condition(&gradient("Minecraft:Bad"), &stone()), M),
        (condition(&gradient("minecraft:"), &stone()), M),
        (condition(&gradient("a b"), &stone()), M),
    ];
    for (json, expected) in cases {
        let error = refuse(&json);
        assert_eq!(*error.kind(), expected, "{json}");
        assert!(error.node().is_some(), "{json}");
    }
    // Explicit properties override the default; bare names take the minecraft namespace.
    let template = compile_synthetic(&props(r#"{"snowy":"true"}"#), Caps::PRODUCTION).unwrap();
    assert_eq!(template.states[0], BlockStateId::new(8));
    let template = compile_synthetic(
        &condition(&gradient("bedrock_floor"), &stone()),
        Caps::PRODUCTION,
    )
    .unwrap();
    assert_eq!(&*template.random_names[0], "minecraft:bedrock_floor");
    assert!(matches!(
        template.nodes[1].kind,
        NodeKind::PaperFlatBedrock(_)
    ));
}

/// Shape before references; references in the order blocks, biomes, noises,
/// random names; census limits last.
#[test]
fn refusal_order_inside_the_definition() {
    use SurfaceRuleErrorKind::{
        InvalidBlockState, MalformedDefinition, NativeLimit, UnknownBiome, UnknownCodecType,
        UnknownNoise,
    };
    let bad_block = block("minecraft:not_a_block");
    let bad_biome = condition(&biome(&["minecraft:not_a_biome"]), &stone());
    let bad_noise = condition(&noise("minecraft:not_a_noise"), &stone());
    let bad_random = condition(&gradient("Bad Name"), &stone());
    let unknown = r#"{"type":"minecraft:unknown"}"#.to_owned();
    // Later in preorder, yet earlier in the declared reference order.
    assert_eq!(
        kind_of(&sequence(&[
            bad_random.clone(),
            bad_noise.clone(),
            bad_biome.clone(),
            bad_block.clone()
        ])),
        InvalidBlockState
    );
    assert_eq!(
        kind_of(&sequence(&[
            bad_random.clone(),
            bad_noise.clone(),
            bad_biome.clone()
        ])),
        UnknownBiome
    );
    assert_eq!(
        kind_of(&sequence(&[bad_random.clone(), bad_noise])),
        UnknownNoise
    );
    assert_eq!(kind_of(&sequence(&[bad_random])), MalformedDefinition);
    assert_eq!(
        kind_of(&sequence(&[bad_block.clone(), unknown])),
        UnknownCodecType
    );
    let names: Vec<&str> = ferrumc_registry::biome::biome_resources()
        .take(16)
        .map(BiomeResourceId::resource_location)
        .collect();
    let mut seventeen = names.clone();
    seventeen.push("minecraft:not_a_biome");
    assert_eq!(
        kind_of(&condition(&biome(&seventeen), &stone())),
        UnknownBiome
    );
    let mut seventeen = names;
    seventeen.push("minecraft:plains");
    assert_eq!(
        kind_of(&condition(&biome(&seventeen), &stone())),
        NativeLimit
    );
    assert_eq!(kind_of(&sequence(&vec![stone(); 65])), NativeLimit);
    assert_eq!(
        kind_of(&sequence(&[sequence(&vec![stone(); 65]), bad_block])),
        InvalidBlockState
    );
}
