//! A5: Paper basis of all nine bound noises, bit for bit, with the Vanilla basis
//! as discriminating control. Refused rows are excluded (their original values
//! are never expectations).

use std::collections::{BTreeMap, HashSet};

use super::super::compile::{CLAY_NOISE, SECONDARY_NOISE, SURFACE_NOISE};
use super::super::session::java_round;
use super::*;

fn hex(text: &str) -> u64 {
    u64::from_str_radix(text, 16).unwrap()
}

fn cell_bits(cell: &str) -> u64 {
    hex(cell.split_once('/').unwrap().1)
}

#[derive(Default, Debug)]
struct Tally {
    compared: usize,
    paper_equal: usize,
    vanilla_equal: usize,
}

/// Node-wise and noise-wise threshold samples, the surface column cell, the
/// secondary noise and every gradient draw against the genuine Paper cells.
#[test]
fn paper_basis_matches_every_genuine_noise_cell_and_vanilla_matches_none() {
    let fixture = fixture();
    let refused: HashSet<usize> = fixture
        .replay
        .iter()
        .filter(|e| e.class == Class::Refused)
        .map(|e| e.ordinal)
        .collect();
    let mut nodes: BTreeMap<usize, Tally> = BTreeMap::new();
    let mut noises: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut items: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut vanilla: HashMap<(i64, &str), NormalNoise> = HashMap::new();
    let mut draws = 0;
    for row in fixture
        .rows
        .iter()
        .filter(|r| !refused.contains(&r.ordinal))
    {
        let group = &fixture.groups[row.gid];
        let rules = binding(group);
        let template = rules.template();
        let mut check = |slot: usize, expected: u64, tally: &mut Tally| {
            let name = template.noises[slot].resource_location();
            let control = vanilla
                .entry((group.seed, name))
                .or_insert_with(|| vanilla_noise(group.seed, name));
            tally.compared += 1;
            tally.paper_equal +=
                usize::from(sample_bits(&rules.noises[slot], row.x, row.z) == expected);
            tally.vanilla_equal += usize::from(sample_bits(control, row.x, row.z) == expected);
        };
        for entry in row.noise_values.split(',').filter(|e| *e != "-") {
            let (node, bits) = entry.split_once('=').unwrap();
            let node: usize = node.parse().unwrap();
            let NodeKind::NoiseThreshold(t) = template.nodes[node].kind else {
                panic!("noise cell on node {node}")
            };
            let slot = usize::from(template.thresholds[usize::from(t)].noise);
            let name = template.noises[slot].resource_location();
            check(slot, hex(bits), nodes.entry(node).or_default());
            check(slot, hex(bits), noises.entry(name).or_default());
        }
        if let Some(column) = row.column {
            let (noise, draw) = fixture.columns[column].split_once(';').unwrap();
            check(
                SURFACE_NOISE,
                cell_bits(noise),
                items.entry("surface_column").or_default(),
            );
            let raw = rules.root.at(BlockPos::new(row.x, 0, row.z)).next_double();
            let tally = items.entry("column_draw").or_default();
            tally.compared += 1;
            tally.paper_equal += usize::from(f64::from(raw as f32).to_bits() == cell_bits(draw));
            tally.vanilla_equal += usize::from(raw.to_bits() == cell_bits(draw));
        }
        if row.secondary != "-" {
            check(
                SECONDARY_NOISE,
                cell_bits(&row.secondary),
                items.entry("surface_secondary").or_default(),
            );
        }
        for entry in row.gradient_draws.split(',').filter(|e| *e != "-") {
            let (node, bits) = entry.split_once('=').unwrap();
            let (NodeKind::VerticalGradient(g) | NodeKind::PaperFlatBedrock(g)) =
                template.nodes[node.parse::<usize>().unwrap()].kind
            else {
                panic!("draw on non-gradient node {node}")
            };
            let factory = rules.factories[usize::from(template.gradients[usize::from(g)].random)];
            let draw = factory.at(BlockPos::new(row.x, row.y, row.z)).next_float();
            assert_eq!(
                u64::from(draw.to_bits()),
                hex(bits),
                "row {} node {node}",
                row.ordinal
            );
            draws += 1;
        }
    }
    for (label, tally) in nodes.iter().map(|(n, t)| (format!("node {n}"), t)).chain(
        noises
            .iter()
            .chain(items.iter())
            .map(|(n, t)| ((*n).to_owned(), t)),
    ) {
        assert!(tally.compared > 0, "{label}");
        assert_eq!(tally.paper_equal, tally.compared, "{label}: {tally:?}");
        assert_eq!(tally.vanilla_equal, 0, "{label}: {tally:?}");
        println!(
            "GEN059_BASIS {label} compared {} paper_equal {} vanilla_equal {}",
            tally.compared, tally.paper_equal, tally.vanilla_equal
        );
    }
    assert_eq!(nodes.len(), 33);
    assert_eq!(noises.len(), 7);
    assert_eq!(items.len(), 3);
    println!("GEN059_BASIS gradient_draws {draws}");
    assert!(draws > 0);
}

/// A5 for `clay_bands_offset` (R3-H2): the direct bit comparison against the
/// genuine values of the Root-authorized extended helper copy. Column set and
/// pass criterion are the fixture manifest's, mechanically.
#[test]
fn clay_bands_offset_paper_basis_equals_every_genuine_paper_value() {
    let fixture = fixture();
    let manifest: Value = serde_json::from_str(CLAY_MANIFEST).unwrap();
    assert!(manifest["column_set"]
        .as_str()
        .unwrap()
        .contains("biome_direct is one of minecraft:badlands"));
    assert!(manifest["pass_criterion"]
        .as_str()
        .unwrap()
        .contains("any deviation means A5 is not met"));
    let mut controls: HashMap<i64, NormalNoise> = HashMap::new();
    let (mut columns, mut paper_equal, mut vanilla_equal, mut genuine_differ) = (0, 0, 0, 0);
    let mut seen = HashSet::new();
    for line in CLAY.lines().skip(1) {
        let cells: Vec<&str> = line.split('\t').collect();
        let group = &fixture.groups[cells[0].parse::<usize>().unwrap()];
        let (x, z): (i32, i32) = (cells[1].parse().unwrap(), cells[2].parse().unwrap());
        assert!(seen.insert((group.gid, x, z)), "distinct columns");
        let (paper, vanilla) = (cell_bits(cells[3]), cell_bits(cells[4]));
        let rules = binding(group);
        assert_eq!(
            rules.template().noises[CLAY_NOISE].resource_location(),
            "minecraft:clay_bands_offset"
        );
        let control = controls
            .entry(group.seed)
            .or_insert_with(|| vanilla_noise(group.seed, "minecraft:clay_bands_offset"));
        columns += 1;
        paper_equal += usize::from(sample_bits(rules.clay_noise(), x, z) == paper);
        vanilla_equal += usize::from(sample_bits(control, x, z) == vanilla);
        genuine_differ += usize::from(paper != vanilla);
    }
    println!("GEN059_CLAY columns {columns} paper_equal {paper_equal} vanilla_equal {vanilla_equal} genuine_paper_vanilla_differ {genuine_differ}");
    assert_eq!(columns, 3393);
    assert_eq!(manifest["counts"]["columns"], 3393);
    assert_eq!(paper_equal, columns, "A5 not met");
    assert_eq!(vanilla_equal, columns, "Vanilla control not met");
    assert_eq!(
        genuine_differ, columns,
        "the comparison must discriminate the bases"
    );
}

/// H9: `getBand` keeps Java `%`; for admitted `y` the left operand is
/// nonnegative because `|round(4 * clay)| <= 14` (max value 10/3).
#[test]
fn band_index_invariant_holds_at_both_height_ends() {
    let rules = binding(&fixture().groups[0]);
    let max = rules.clay_noise().max_value();
    assert!((max - 10.0 / 3.0).abs() < 1e-12, "{max}");
    let bound = java_round(max * 4.0);
    assert!(bound <= 14);
    for y in [-64, 319] {
        for offset in [-14_i32, 14] {
            let index = (y + offset + 192) % 192;
            assert!((0..192).contains(&index), "{y} {offset}");
        }
    }
    // Java Math.round edge cases (exact floor(x + 1/2)).
    for (value, expected) in [
        (0.499_999_999_999_999_94, 0),
        (0.5, 1),
        (-0.5, 0),
        (-1.5, -1),
        (2.5, 3),
        (-2.5, -2),
        (13.333_333_333_333_334, 13),
        (f64::NAN, 0),
        (1e300, i64::MAX),
    ] {
        assert_eq!(java_round(value), expected, "{value}");
    }
}
