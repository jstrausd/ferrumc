//! Run explicitly with the external allocation probe described in the reference.
#![forbid(unsafe_code)]

use std::hint::black_box;
use std::io::{self, Write};

use ferrumc_math::BlockPos;
use ferrumc_registry::biome::BiomeResourceId;
use ferrumc_world::biome_temperature::{BiomeTemperatureBuildBudget, BuiltinBiomeTemperatures};

fn boundary(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes).unwrap();
    stdout.flush().unwrap();
}

#[test]
#[ignore = "external Linux allocator probe; run measure_allocations.py in Debug and Release"]
fn measured_compile_clone_and_every_original_query() {
    let requests: Vec<_> = include_str!("reference/biome-temperature/fixtures/temperatures.tsv")
        .lines()
        .map(|line| {
            let row: Vec<_> = line.split('\t').collect();
            (
                BiomeResourceId::from_name(row[2]).unwrap(),
                BlockPos::new(
                    row[3].parse().unwrap(),
                    row[4].parse().unwrap(),
                    row[5].parse().unwrap(),
                ),
                row[6].parse::<i32>().unwrap(),
                u32::from_str_radix(row[if row[0] == "C" { 8 } else { 7 }], 16).unwrap(),
            )
        })
        .collect();
    assert_eq!(requests.len(), 129_588);
    let budget = BiomeTemperatureBuildBudget::new(354_536).unwrap();
    let refused = BiomeTemperatureBuildBudget::new(354_535).unwrap();
    boundary(b"GEN057_BEGIN_CONTROL\n");
    let mut control = Vec::<u8>::with_capacity(4096);
    control.push(1);
    black_box(&control);
    drop(control);
    boundary(b"GEN057_END_CONTROL\n");

    boundary(b"GEN057_BEGIN_COMPILE\n");
    // Include this fresh process's first producer construction in measurement.
    let table = BuiltinBiomeTemperatures::compile(black_box(budget)).unwrap();
    for _ in 0..2 {
        black_box(BuiltinBiomeTemperatures::compile(black_box(budget)).unwrap());
    }
    for _ in 0..3 {
        black_box(BuiltinBiomeTemperatures::compile(black_box(refused)).unwrap_err());
    }
    boundary(b"GEN057_END_COMPILE\n");

    boundary(b"GEN057_BEGIN_CLONE\n");
    for _ in 0..100 {
        black_box(black_box(&table).clone());
    }
    boundary(b"GEN057_END_CLONE\n");

    boundary(b"GEN057_BEGIN_QUERY\n");
    for _ in 0..3 {
        for &(biome, pos, sea, expected) in &requests {
            let sample = black_box(&table)
                .sample(black_box(biome), black_box(pos), black_box(sea))
                .unwrap();
            assert_eq!(sample.temperature().to_bits(), expected);
            black_box((
                sample.warm_enough_to_rain(),
                sample.cold_enough_to_snow(),
                sample.should_melt_iceberg_slightly(),
                sample.precipitation(),
            ));
        }
    }
    boundary(b"GEN057_END_QUERY\n");
    println!("measured 3 successful + 3 refused compiles, 100 clones, 388764 original queries");
}
