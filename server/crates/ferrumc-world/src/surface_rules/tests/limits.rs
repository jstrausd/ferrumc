//! A4: measured retained bytes pinned exactly; `NativeLimit` at N-1/N/N+1 of
//! every cap through the real admission. Synthetic size boundaries are private
//! `cfg(test)` compiles, separate from original-behaviour tests.

use std::mem::size_of;

use super::super::compile::{compile_synthetic, compile_with, Caps};
use super::super::session::SurfaceRuleSession as Session;
use super::*;

/// Measured on `x86_64` with Rust 1.99.0 (Debug = Release).
const TEMPLATE_BYTES: usize = 6622;
const BINDING_BYTES: usize = 19_216;
const SESSION_BYTES: usize = 400;

#[test]
fn retained_bytes_are_pinned_exactly_and_below_the_design_caps() {
    let template = overworld();
    println!(
        "GEN059_LAYOUT template {} binding {} session {} error {} outcome {} input {} heights {} corners {}",
        template.retained_bytes(),
        binding(&fixture().groups[0]).retained_bytes(),
        size_of::<Session<'static>>(),
        size_of::<SurfaceRuleError>(),
        size_of::<SurfaceRuleOutcome>(),
        size_of::<SurfaceBlockInput>(),
        size_of::<WorldSurfaceHeights>(),
        size_of::<PreliminarySurfaceCorners>(),
    );
    assert_eq!(template.retained_bytes(), TEMPLATE_BYTES);
    for name in ["minecraft:large_biomes", "minecraft:amplified"] {
        assert_eq!(
            super::template(settings(name)).retained_bytes(),
            TEMPLATE_BYTES
        );
    }
    for group in &fixture().groups {
        assert_eq!(
            binding(group).retained_bytes(),
            BINDING_BYTES,
            "{}",
            group.id
        );
    }
    assert_eq!(size_of::<Session<'static>>(), SESSION_BYTES);
    const { assert!(TEMPLATE_BYTES <= MAX_SURFACE_RULE_TEMPLATE_BYTES) };
    const { assert!(BINDING_BYTES <= MAX_SURFACE_RULE_BINDING_BYTES) };
    const { assert!(SESSION_BYTES <= MAX_SURFACE_RULE_SESSION_BYTES) };
}

fn native(
    result: Result<impl Sized, SurfaceRuleError>,
) -> (Option<SurfaceRuleLimit>, usize, usize) {
    let Err(error) = result else {
        return (None, 0, 0);
    };
    assert_eq!(*error.kind(), SurfaceRuleErrorKind::NativeLimit);
    (error.limit(), error.requested(), error.maximum())
}

#[test]
fn retained_caps_refuse_at_the_measured_value_minus_one() {
    let resource = settings("minecraft:overworld");
    let budget = build_budget(MAX_SURFACE_RULE_BUILD_WORK);
    let template = |cap| {
        compile_with(
            resource,
            budget,
            Caps {
                template_bytes: cap,
                ..Caps::PRODUCTION
            },
        )
    };
    assert_eq!(
        native(template(TEMPLATE_BYTES - 1)),
        (
            Some(SurfaceRuleLimit::TemplateBytes),
            TEMPLATE_BYTES,
            TEMPLATE_BYTES - 1
        )
    );
    assert!(template(TEMPLATE_BYTES).is_ok() && template(TEMPLATE_BYTES + 1).is_ok());
    let bind = |cap| {
        SeededSurfaceRules::bind_with(
            overworld(),
            WorldSeed::new(0),
            SurfaceNumericProfile::PaperBuild60,
            PaperFlatBedrock::Disabled,
            bind_budget(MAX_SURFACE_RULE_BIND_WORK),
            cap,
        )
    };
    assert_eq!(
        native(bind(BINDING_BYTES - 1)),
        (
            Some(SurfaceRuleLimit::BindingBytes),
            BINDING_BYTES,
            BINDING_BYTES - 1
        )
    );
    assert!(bind(BINDING_BYTES).is_ok() && bind(BINDING_BYTES + 1).is_ok());
    let (group, generated) = super::policy::origin();
    let refs = generated.refs();
    let resolver = resolver(generated, &refs);
    let rules = binding(group);
    let session = |cap| {
        Session::new_with(
            rules,
            &resolver,
            temperatures(),
            group.corners,
            work_budget(1),
            cap,
        )
    };
    assert_eq!(
        native(session(SESSION_BYTES - 1)),
        (
            Some(SurfaceRuleLimit::SessionBytes),
            SESSION_BYTES,
            SESSION_BYTES - 1
        )
    );
    assert!(session(SESSION_BYTES).is_ok() && session(SESSION_BYTES + 1).is_ok());
}

type CapCase = (fn(usize) -> Caps, usize, SurfaceRuleLimit);

/// Census caps with the real overworld data at the measured value N: N-1 refused.
#[test]
fn census_caps_refuse_below_the_real_census() {
    let resource = settings("minecraft:overworld");
    let budget = build_budget(MAX_SURFACE_RULE_BUILD_WORK);
    let p = Caps::PRODUCTION;
    let cases: [CapCase; 8] = [
        (
            |v| Caps {
                nodes: v,
                ..Caps::PRODUCTION
            },
            428,
            SurfaceRuleLimit::Nodes,
        ),
        (
            |v| Caps {
                depth: v,
                ..Caps::PRODUCTION
            },
            13,
            SurfaceRuleLimit::Depth,
        ),
        (
            |v| Caps {
                sequence_children: v,
                ..Caps::PRODUCTION
            },
            12,
            SurfaceRuleLimit::SequenceChildren,
        ),
        (
            |v| Caps {
                biome_references: v,
                ..Caps::PRODUCTION
            },
            53,
            SurfaceRuleLimit::BiomeReferences,
        ),
        (
            |v| Caps {
                biomes_per_condition: v,
                ..Caps::PRODUCTION
            },
            3,
            SurfaceRuleLimit::BiomesPerCondition,
        ),
        (
            |v| Caps {
                noises: v,
                ..Caps::PRODUCTION
            },
            9,
            SurfaceRuleLimit::Noises,
        ),
        (
            |v| Caps {
                random_names: v,
                ..Caps::PRODUCTION
            },
            2,
            SurfaceRuleLimit::RandomNames,
        ),
        (
            |v| Caps {
                states: v,
                ..Caps::PRODUCTION
            },
            24,
            SurfaceRuleLimit::States,
        ),
    ];
    for (caps, value, limit) in cases {
        assert_eq!(
            native(compile_with(resource, budget, caps(value - 1))),
            (Some(limit), value, value - 1)
        );
        assert!(
            compile_with(resource, budget, caps(value)).is_ok(),
            "{limit:?}"
        );
        assert!(
            compile_with(resource, budget, caps(value + 1)).is_ok(),
            "{limit:?}"
        );
    }
    assert_eq!(p, Caps::PRODUCTION);
    assert_eq!(
        [
            p.nodes,
            p.depth,
            p.sequence_children,
            p.biome_references,
            p.biomes_per_condition,
            p.noises,
            p.random_names,
            p.states
        ],
        [512, 32, 64, 256, 16, 16, 8, 128]
    );
}

pub(super) fn block(name: &str) -> String {
    format!(r#"{{"type":"minecraft:block","result_state":{{"Name":"{name}"}}}}"#)
}

pub(super) fn sequence(children: &[String]) -> String {
    format!(
        r#"{{"type":"minecraft:sequence","sequence":[{}]}}"#,
        children.join(",")
    )
}

pub(super) fn condition(test: &str, then: &str) -> String {
    format!(r#"{{"type":"minecraft:condition","if_true":{test},"then_run":{then}}}"#)
}

pub(super) fn biome(names: &[&str]) -> String {
    let names: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    format!(
        r#"{{"type":"minecraft:biome","biome_is":[{}]}}"#,
        names.join(",")
    )
}

pub(super) fn noise(name: &str) -> String {
    format!(
        r#"{{"type":"minecraft:noise_threshold","noise":"{name}","min_threshold":0.0,"max_threshold":1.0}}"#
    )
}

pub(super) fn gradient(name: &str) -> String {
    format!(
        r#"{{"type":"minecraft:vertical_gradient","random_name":"{name}","true_at_and_below":{{"absolute":0}},"false_at_and_above":{{"absolute":8}}}}"#
    )
}

fn stone() -> String {
    block("minecraft:stone")
}

/// A two-level sequence with exactly `nodes` compiled nodes (every sequence ≤ 63 children).
fn tree_of(nodes: usize) -> String {
    let subsequences = nodes.div_ceil(64);
    let blocks = nodes - 1 - subsequences;
    let mut groups = Vec::new();
    for i in 0..subsequences {
        let count = blocks / subsequences + usize::from(i < blocks % subsequences);
        groups.push(sequence(&vec![stone(); count]));
    }
    sequence(&groups)
}

fn synthetic(json: &str) -> Result<SurfaceRuleTemplate, SurfaceRuleError> {
    compile_synthetic(json, Caps::PRODUCTION)
}

fn limit_of(json: &str) -> Option<SurfaceRuleLimit> {
    native(synthetic(json)).0
}

#[test]
fn synthetic_size_boundaries_at_the_production_caps() {
    for (nodes, refused) in [(511, false), (512, false), (513, true)] {
        let json = tree_of(nodes);
        let compiled = synthetic(&json);
        if refused {
            assert_eq!(limit_of(&json), Some(SurfaceRuleLimit::Nodes));
        } else {
            assert_eq!(compiled.unwrap().node_count(), nodes);
        }
    }
    for (children, limit) in [
        (63, None),
        (64, None),
        (65, Some(SurfaceRuleLimit::SequenceChildren)),
    ] {
        assert_eq!(
            limit_of(&sequence(&vec![stone(); children])),
            limit,
            "{children}"
        );
    }
    let names: Vec<&str> = ferrumc_registry::biome::biome_resources()
        .map(BiomeResourceId::resource_location)
        .collect();
    for (count, limit) in [
        (15, None),
        (16, None),
        (17, Some(SurfaceRuleLimit::BiomesPerCondition)),
    ] {
        let json = condition(&biome(&names[..count]), &stone());
        assert_eq!(limit_of(&json), limit, "per condition {count}");
    }
    for (count, limit) in [
        (255_usize, None),
        (256, None),
        (257, Some(SurfaceRuleLimit::BiomeReferences)),
    ] {
        let conditions: Vec<String> = (0..count.div_ceil(16))
            .map(|i| condition(&biome(&names[..(count - i * 16).min(16)]), &stone()))
            .collect();
        assert_eq!(
            limit_of(&sequence(&conditions)),
            limit,
            "references {count}"
        );
    }
    let free: Vec<&str> = ferrumc_registry::worldgen::noise_resources()
        .map(ferrumc_registry::worldgen::NoiseResourceId::resource_location)
        .filter(|n| !super::super::compile::SYSTEM_NOISES.contains(n))
        .collect();
    for (tree_noises, limit) in [(12, None), (13, None), (14, Some(SurfaceRuleLimit::Noises))] {
        let conditions: Vec<String> = free[..tree_noises]
            .iter()
            .map(|n| condition(&noise(n), &stone()))
            .collect();
        assert_eq!(
            limit_of(&sequence(&conditions)),
            limit,
            "noises {}",
            tree_noises + 3
        );
    }
    for (count, limit) in [
        (7, None),
        (8, None),
        (9, Some(SurfaceRuleLimit::RandomNames)),
    ] {
        let conditions: Vec<String> = (0..count)
            .map(|i| condition(&gradient(&format!("minecraft:r{i}")), &stone()))
            .collect();
        assert_eq!(
            limit_of(&sequence(&conditions)),
            limit,
            "random names {count}"
        );
    }
    let mut blocks = Vec::new();
    let mut state = 0;
    while blocks.len() < 129 {
        let name = ferrumc_registry::block_state::state_id_to_block_name(state).unwrap();
        if !blocks.contains(&name) {
            blocks.push(name);
        }
        state += 1;
    }
    for (count, limit) in [
        (127, None),
        (128, None),
        (129, Some(SurfaceRuleLimit::States)),
    ] {
        let groups: Vec<String> = blocks[..count]
            .chunks(60)
            .map(|chunk| sequence(&chunk.iter().map(|n| block(n)).collect::<Vec<_>>()))
            .collect();
        assert_eq!(limit_of(&sequence(&groups)), limit, "states {count}");
    }
}

/// Compiled depth is bounded by the JSON depth: 30/31 compile, 32 needs JSON
/// depth 33 and is refused by the JSON admission; the compiled `Depth` cap
/// itself is exercised with the real tree above (12/13/14).
#[test]
fn synthetic_depth_boundaries() {
    let chain = |nots: usize| {
        let mut test = r#"{"type":"minecraft:steep"}"#.to_owned();
        for _ in 0..nots {
            test = format!(r#"{{"type":"minecraft:not","invert":{test}}}"#);
        }
        condition(&test, &stone())
    };
    assert_eq!(synthetic(&chain(29)).unwrap().census.depth, 30);
    assert_eq!(synthetic(&chain(30)).unwrap().census.depth, 31);
    assert_eq!(limit_of(&chain(31)), Some(SurfaceRuleLimit::JsonDepth));
}
