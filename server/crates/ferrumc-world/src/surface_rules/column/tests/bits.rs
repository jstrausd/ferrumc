//! C4 — direct bits: the six bound extension noises, the positional extension
//! draws and the melt predicate against the genuine Paper cells of
//! `noises.tsv`, with the Vanilla basis and the unrounded draws as the
//! discriminating control (outcomes cannot discriminate, contract §2.2).

use ferrumc_math::Vec3;

use super::super::bind::EXTENSION_NOISES;
use super::super::driver::paper_double;
use super::*;
use crate::surface_rules::tests::vanilla_noise;

/// Original argument factors of the six noises (`getValue(x·f, 0, z·f)`).
const FACTORS: [f64; 6] = [1.0, 0.2, 0.75, 1.0, 1.28, 1.17];

#[test]
fn extension_noises_draws_and_melt_match_paper_bits_with_vanilla_control() {
    let fixture = fixture();
    let mut vanilla: HashMap<(i64, usize), crate::noise::NormalNoise> = HashMap::new();
    let (mut rows, mut paper_equal, mut vanilla_equal, mut control_equal) = (0, 0, 0, 0);
    let (mut draws_equal, mut unrounded_equal, mut melt_equal, mut melt_true) = (0, 0, 0, 0);
    for row in &fixture.noises {
        let group = &fixture.groups[row.gid];
        if group.class != Class::Full {
            continue;
        }
        rows += 1;
        let (rules, columns) = bindings(group.settings, group.seed);
        let pos = column_pos(group.center, row.lx, row.lz);
        let (x, z) = (pos.block_x(), pos.block_z());
        for (slot, factor) in FACTORS.iter().enumerate() {
            let at = Vec3::new(f64::from(x) * factor, 0.0, f64::from(z) * factor);
            let paper = columns.noises[slot].sample(at).unwrap().to_bits();
            let control = vanilla
                .entry((group.seed, slot))
                .or_insert_with(|| vanilla_noise(group.seed, EXTENSION_NOISES[slot]))
                .sample(at)
                .unwrap()
                .to_bits();
            paper_equal += usize::from(paper == row.paper_bits[slot]);
            vanilla_equal += usize::from(control == row.vanilla_bits[slot]);
            control_equal += usize::from(paper == row.vanilla_bits[slot]);
        }
        let mut random = rules.root.at(BlockPos::new(x, 0, z));
        let ints = format!(
            "{},{}",
            random.next_int_bounded(4).unwrap(),
            random.next_int_bounded(10).unwrap()
        );
        let doubles: Vec<String> = (0..4)
            .map(|_| format!("{:x}", paper_double(&mut random).to_bits()))
            .collect();
        draws_equal += usize::from(ints == row.ints && doubles.join(",") == row.paper_doubles);
        let mut random = rules.root.at(BlockPos::new(x, 0, z));
        random.next_int_bounded(4).unwrap();
        random.next_int_bounded(10).unwrap();
        let unrounded: Vec<String> = (0..4)
            .map(|_| format!("{:x}", random.next_double().to_bits()))
            .collect();
        unrounded_equal += usize::from(unrounded.join(",") == row.vanilla_doubles);
        let biome = fixture.biomes
            [fixture.columns[row.gid][usize::from(row.lx) * 16 + usize::from(row.lz)].biome];
        let melt = temperatures()
            .sample(biome, BlockPos::new(x, group.sea_level, z), group.sea_level)
            .unwrap()
            .should_melt_iceberg_slightly();
        melt_equal += usize::from(melt == row.melt);
        melt_true += usize::from(row.melt);
    }
    println!(
        "GEN0510_BITS rows {rows} paper {paper_equal} vanilla {vanilla_equal} control {control_equal} draws {draws_equal} unrounded {unrounded_equal} melt {melt_equal} melt_true {melt_true}"
    );
    assert!(rows > 2000);
    assert_eq!(paper_equal, rows * 6);
    assert_eq!(vanilla_equal, rows * 6);
    // Discriminating control: the Paper basis never reproduces a Vanilla bit.
    assert_eq!(control_equal, 0);
    assert_eq!(draws_equal, rows);
    assert_eq!(unrounded_equal, rows);
    assert_eq!(melt_equal, rows);
    assert!(melt_true > 0 && melt_true < rows);
}
