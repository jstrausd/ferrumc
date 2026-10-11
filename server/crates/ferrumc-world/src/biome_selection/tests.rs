//! Complete actual original tree and scalar-history contracts.

use super::*;
use serde_json::{json, Value};

mod arrays;
mod limits;

const TREES: &str = include_str!("../../tests/fixtures/biome-selector-original-1.21.8/trees.json");
const QUERIES: &str =
    include_str!("../../tests/fixtures/biome-selector-original-1.21.8/queries.jsonl");

fn tree(preset: &str) -> Arc<BiomeTree> {
    Arc::new(
        BiomeTree::from_builtin(
            BiomeParametersResourceId::from_name(preset).unwrap(),
            BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
        )
        .unwrap(),
    )
}
fn snapshot(tree: &BiomeTree, node: u16, output: &mut Vec<Value>) -> usize {
    let index = output.len();
    output.push(Value::Null);
    let node = tree.nodes[usize::from(node)];
    let mut value = json!({"index":index,"axes":node.axes});
    match node.kind {
        NodeKind::Leaf(ordinal) => value["leaf"] = json!(ordinal),
        NodeKind::Branch { children, len } => {
            let children: Vec<_> = children[..usize::from(len)]
                .iter()
                .map(|&child| snapshot(tree, child, output))
                .collect();
            value["children"] = json!(children);
        }
    }
    output[index] = value;
    index
}

#[test]
fn complete_actual_original_tree_shapes_and_all_ordered_leaves_match() {
    let original: Value = serde_json::from_str(TREES).unwrap();
    for (name, nodes, leaves, depth) in [
        ("minecraft:overworld", 9112, 7593, 6),
        ("minecraft:nether", 6, 5, 2),
    ] {
        let tree = tree(name);
        let mut actual = Vec::new();
        snapshot(&tree, tree.root, &mut actual);
        assert_eq!(json!(actual), original[name]);
        assert_eq!(tree.node_count(), nodes);
        assert_eq!(tree.edge_count(), nodes - 1);
        assert_eq!(tree.leaf_count(), leaves);
        assert_eq!(tree.depth(), depth);
        assert!(tree.retained_bytes() <= MAX_BIOME_TREE_BYTES);
        eprintln!(
            "{name}: buildwork {} retainedbytes {} scratchpeak {} nodesize {} treesize {}",
            tree.build_work_used(),
            tree.retained_bytes(),
            tree.build_scratch_peak_bytes(),
            std::mem::size_of::<Node>(),
            std::mem::size_of::<BiomeTree>(),
        );
        assert!(tree.build_scratch_peak_bytes() <= MAX_BIOME_BUILD_SCRATCH_BYTES);
    }
}

#[test]
fn every_original_builtin_cold_forward_reverse_call_including_extreme_longs_matches() {
    let mut checked = 0;
    for name in ["minecraft:nether", "minecraft:overworld"] {
        let tree = tree(name);
        let mut session = BiomeSelectionSession::new(
            tree,
            BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
        );
        let mut prior_group = String::new();
        for line in QUERIES.lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            if row["preset"] != name {
                continue;
            }
            let group = row["group"].as_str().unwrap();
            if group == "cold" || group != prior_group {
                session.reset(BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap());
            }
            let values = row["target"].as_array().unwrap();
            let target = std::array::from_fn(|axis| values[axis].as_i64().unwrap());
            let selected = session
                .select(BiomeClimateTarget::from_quantized(target))
                .unwrap();
            assert_eq!(
                selected.parameter_ordinal(),
                row["indexed"].as_u64().unwrap() as usize,
                "{name}/{group}/{}",
                row["sequence"]
            );
            assert_eq!(
                selected.biome(),
                session.tree.preset.parameters()[selected.parameter_ordinal()].biome()
            );
            prior_group = group.to_owned();
            checked += 1;
        }
    }
    assert_eq!(checked, 3279);
}
