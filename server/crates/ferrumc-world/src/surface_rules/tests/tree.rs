//! Compiled form against the mechanically committed original trees (A2) and the
//! pinned census (§3, §8).

use ferrumc_registry::block_state::{block_metadata, state_id_to_block_name};

use super::super::compile::{Anchor, ADMITTED_TREE_NODES, ADMITTED_TREE_REFERENCES};
use super::*;

fn anchor_text(anchor: Anchor) -> String {
    match anchor {
        Anchor::Absolute(y) => format!("absolute:{y}"),
        Anchor::AboveBottom(o) => format!("above_bottom:{o}"),
        Anchor::BelowTop(o) => format!("below_top:{o}"),
    }
}

/// Original `stateText`: name, all properties sorted, protocol id, default flag.
pub(super) fn state_text(state: BlockStateId) -> String {
    let id = state.as_u32();
    let bare = state_id_to_block_name(id).unwrap();
    let meta = block_metadata(bare).unwrap();
    let name = if bare.contains(':') {
        bare.to_owned()
    } else {
        format!("minecraft:{bare}")
    };
    let mut properties: Vec<(String, String)> = meta
        .properties
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let multiplier: u32 = meta.properties[i + 1..]
                .iter()
                .map(|q| q.cardinality as u32)
                .product();
            let index = ((id - meta.min_state) / multiplier) % p.cardinality as u32;
            (p.name.to_owned(), p.values[index as usize].to_owned())
        })
        .collect();
    properties.sort();
    let props = if properties.is_empty() {
        String::new()
    } else {
        format!(
            "[{}]",
            properties
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let default = if id == meta.default_state {
        "default"
    } else {
        "nondefault"
    };
    format!("{name}{props}|{id}|{default}")
}

/// Projects the compiled table to the oracle's canonical five columns.
/// `paper = false` renders the pre-substitution (Vanilla/registry) form.
pub(super) fn project(template: &SurfaceRuleTemplate, paper: bool) -> String {
    let mut out = String::from("id\tparent\tdepth\tkind\tattributes\n");
    for (id, node) in template.nodes.iter().enumerate() {
        let (kind, attributes) = match node.kind {
            NodeKind::Sequence { len, .. } => ("sequence", format!("n={len}")),
            NodeKind::Condition { .. } => ("condition", "-".to_owned()),
            NodeKind::Block(s) => ("block", state_text(template.states[usize::from(s)])),
            NodeKind::Bandlands => ("bandlands", "-".to_owned()),
            NodeKind::Biome { first, len } => (
                "biome",
                template.biomes[usize::from(first)..usize::from(first + len)]
                    .iter()
                    .map(|b| b.resource_location())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            NodeKind::NoiseThreshold(t) => {
                let t = template.thresholds[usize::from(t)];
                (
                    "noise_threshold",
                    format!(
                        "{};{};{}",
                        template.noises[usize::from(t.noise)].resource_location(),
                        double_text(t.min),
                        double_text(t.max)
                    ),
                )
            }
            NodeKind::VerticalGradient(g) | NodeKind::PaperFlatBedrock(g) => {
                let g = template.gradients[usize::from(g)];
                let base = format!(
                    "{};{};{}",
                    template.random_names[usize::from(g.random)],
                    anchor_text(g.true_at),
                    anchor_text(g.false_at)
                );
                match (paper, g.paper_roof, node.kind) {
                    (true, Some(roof), NodeKind::PaperFlatBedrock(_)) => (
                        "paper_optionally_flat_bedrock",
                        format!("{base};is_roof={roof}"),
                    ),
                    _ => ("vertical_gradient", base),
                }
            }
            NodeKind::YAbove(c) => {
                let c = template.y_checks[usize::from(c)];
                (
                    "y_above",
                    format!("{};{};{}", anchor_text(c.anchor), c.multiplier, c.add_stone),
                )
            }
            NodeKind::Water(c) => {
                let c = template.water_checks[usize::from(c)];
                (
                    "water",
                    format!("{};{};{}", c.offset, c.multiplier, c.add_stone),
                )
            }
            NodeKind::StoneDepth(c) => {
                let c = template.stone_checks[usize::from(c)];
                let surface = if c.ceiling { "ceiling" } else { "floor" };
                (
                    "stone_depth",
                    format!("{};{};{};{surface}", c.offset, c.add_surface, c.range),
                )
            }
            NodeKind::Steep => ("steep", "-".to_owned()),
            NodeKind::Hole => ("hole", "-".to_owned()),
            NodeKind::AbovePreliminarySurface => ("above_preliminary_surface", "-".to_owned()),
            NodeKind::Temperature => ("temperature", "-".to_owned()),
            NodeKind::Not(_) => ("not", "-".to_owned()),
        };
        let parent = if node.parent == u16::MAX {
            -1
        } else {
            i32::from(node.parent)
        };
        writeln!(out, "{id}\t{parent}\t{}\t{kind}\t{attributes}", node.depth).unwrap();
    }
    out
}

/// A2: before substitution the table equals `tree.tsv`; after it equals
/// `tree-paper.tsv`; they differ exactly by `paper-tree-substitution.json`.
#[test]
fn compiled_tables_equal_the_committed_original_trees_for_every_admitted_setting() {
    let substitution: Value = serde_json::from_str(SUBSTITUTION).unwrap();
    for name in [
        "minecraft:overworld",
        "minecraft:large_biomes",
        "minecraft:amplified",
    ] {
        let template = template(settings(name));
        let vanilla = project(&template, false);
        let paper = project(&template, true);
        assert_eq!(vanilla, TREE, "{name}");
        assert_eq!(paper, TREE_PAPER, "{name}");
        let changed: Vec<(&str, &str)> = vanilla
            .lines()
            .zip(paper.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(
            changed,
            [(
                substitution["vanilla_and_registry"].as_str().unwrap(),
                substitution["paper_jar"].as_str().unwrap()
            )]
        );
    }
}

/// The exact §3 census, maximum depth, widest sequence and per-call worst case.
#[test]
fn template_census_matches_the_contract() {
    let template = overworld();
    let projected = project(&template, true);
    let mut kinds: HashMap<&str, usize> = HashMap::new();
    for line in projected.lines().skip(1) {
        *kinds.entry(line.split('\t').nth(3).unwrap()).or_default() += 1;
    }
    let expected = [
        ("condition", 137),
        ("block", 100),
        ("sequence", 48),
        ("bandlands", 2),
        ("biome", 39),
        ("noise_threshold", 33),
        ("stone_depth", 23),
        ("water", 20),
        ("y_above", 10),
        ("steep", 5),
        ("not", 4),
        ("hole", 3),
        ("vertical_gradient", 1),
        ("paper_optionally_flat_bedrock", 1),
        ("above_preliminary_surface", 1),
        ("temperature", 1),
    ];
    assert_eq!(kinds.len(), expected.len());
    for (kind, count) in expected {
        assert_eq!(kinds[kind], count, "{kind}");
    }
    assert_eq!(template.node_count(), ADMITTED_TREE_NODES);
    assert_eq!(template.census.references, ADMITTED_TREE_REFERENCES);
    assert_eq!(template.census.depth, 13);
    assert_eq!(
        template
            .nodes
            .iter()
            .map(|n| match n.kind {
                NodeKind::Sequence { len, .. } => usize::from(len),
                _ => 0,
            })
            .max(),
        Some(12)
    );
    assert_eq!(template.biomes.len(), 53);
    assert_eq!(
        template
            .nodes
            .iter()
            .map(|n| match n.kind {
                NodeKind::Biome { len, .. } => usize::from(len),
                _ => 0,
            })
            .max(),
        Some(3)
    );
    assert_eq!(template.states.len(), 24);
    assert_eq!(template.noises.len(), 9);
    assert_eq!(template.random_names.len(), 2);
    assert_eq!(template.evaluate_work(), 467);
    // Compile census: 129737 bytes + 3411 JSON nodes + 428 nodes + 202 references.
    assert_eq!(template.census.definition_bytes, 129_737);
    assert_eq!(template.census.json_nodes, 3411);
    assert_eq!(template.construction_work(), 133_778);
    let stats = settings("minecraft:overworld").definition().stats();
    assert_eq!((stats.bytes(), stats.nodes()), (129_737, 3411));
}

/// The template keeps block, biome and noise identities as distinct checked types.
#[test]
fn template_identities_resolve_through_the_registry_only() {
    let template = overworld();
    for state in &*template.states {
        assert!(state_id_to_block_name(state.as_u32()).is_some());
    }
    let mut names: Vec<&str> = template
        .noises
        .iter()
        .map(|n| n.resource_location())
        .collect();
    // The three SurfaceSystem noises come first; the tree adds six distinct threshold noises.
    assert_eq!(
        names[..3],
        [
            "minecraft:surface",
            "minecraft:surface_secondary",
            "minecraft:clay_bands_offset"
        ]
    );
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "minecraft:calcite",
            "minecraft:clay_bands_offset",
            "minecraft:gravel",
            "minecraft:ice",
            "minecraft:packed_ice",
            "minecraft:powder_snow",
            "minecraft:surface",
            "minecraft:surface_secondary",
            "minecraft:surface_swamp",
        ]
    );
    let random: Vec<&str> = template.random_names.iter().map(|n| &**n).collect();
    assert_eq!(random, ["minecraft:bedrock_floor", "minecraft:deepslate"]);
}
