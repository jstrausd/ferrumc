use std::mem::size_of;

use sha2::{Digest, Sha256};

use super::*;

const TEMPERATURES: &str =
    include_str!("../../tests/reference/biome-temperature/fixtures/temperatures.tsv");
const METADATA: &str =
    include_str!("../../tests/reference/biome-temperature/fixtures/metadata.tsv");
const NOISE: &str = include_str!("../../tests/reference/biome-temperature/fixtures/noise.tsv");
const VANILLA_CONSTRUCTORS: &str =
    include_str!("../../tests/reference/biome-temperature/fixtures/vanilla-constructors.tsv");
const PAPER_CONSTRUCTORS: &str =
    include_str!("../../tests/reference/biome-temperature/fixtures/paper-constructors.tsv");

fn table() -> BuiltinBiomeTemperatures {
    BuiltinBiomeTemperatures::compile(BiomeTemperatureBuildBudget::new(CONSTRUCTION_WORK).unwrap())
        .unwrap()
}
fn bits(token: &str) -> u32 {
    u32::from_str_radix(token, 16).unwrap()
}
fn double(token: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(token, 16).unwrap())
}
fn position(row: &[&str]) -> BlockPos {
    BlockPos::new(
        row[3].parse().unwrap(),
        row[4].parse().unwrap(),
        row[5].parse().unwrap(),
    )
}

#[test]
fn complete_original_temperature_labels_mutable_calls_cache_diagnostics_and_frozen_branches() {
    let table = table();
    let mut groups = [0; 4];
    let mut branches = [0; 3];
    let mut cache_differences = 0;
    let mut scalar_groups = [0; 4];
    for line in TEMPERATURES.lines() {
        let row: Vec<_> = line.split('\t').collect();
        let biome = BiomeResourceId::from_name(row[2]).unwrap();
        let pos = position(&row);
        let sea = row[6].parse().unwrap();
        let sample = table.sample(biome, pos, sea).unwrap();
        let actual = sample.temperature().to_bits();
        let record = &table.records[table
            .records
            .binary_search_by_key(&biome, |r| r.biome)
            .unwrap()];
        match row[0] {
            "S" | "F" => {
                let extension = row[0] == "F";
                let paper = if extension { 17 } else { 14 };
                assert_eq!(actual, bits(row[7]), "Vanilla private {}", row[1]);
                assert_eq!(actual, bits(row[paper]), "Paper private {}", row[1]);
                assert_eq!(actual, bits(row[paper + 2]), "Paper public {}", row[1]);
                let modified = table.modified(record.climate, pos).unwrap().to_bits();
                assert_eq!(modified, bits(row[8]), "Vanilla modifier {}", row[1]);
                assert_eq!(&row[9..14], &["-", "-", "-", "-", "-"]);
                assert_eq!(modified, bits(row[paper + 1]), "Paper modifier {}", row[1]);
                assert_eq!(
                    sample.warm_enough_to_rain(),
                    row[paper + 3].parse::<bool>().unwrap(),
                    "rain {}",
                    row[1]
                );
                assert_eq!(
                    sample.cold_enough_to_snow(),
                    row[paper + 4].parse::<bool>().unwrap(),
                    "snow {}",
                    row[1]
                );
                assert_eq!(
                    sample.should_melt_iceberg_slightly(),
                    row[paper + 5].parse::<bool>().unwrap(),
                    "melt {}",
                    row[1]
                );
                let precipitation = match row[paper + 6] {
                    "NONE" => BiomePrecipitation::None,
                    "RAIN" => BiomePrecipitation::Rain,
                    "SNOW" => BiomePrecipitation::Snow,
                    unknown => panic!("unknown original precipitation {unknown}"),
                };
                assert_eq!(
                    sample.precipitation(),
                    precipitation,
                    "precipitation {}",
                    row[1]
                );
                if extension {
                    groups[3] += 1;
                    branches[verify_extension_noises(&table, pos, &row)] += 1;
                } else {
                    groups[0] += 1;
                    scalar_groups[scalar_group(row[1])] += 1;
                }
            }
            "M" => {
                groups[1] += 1;
                for index in [7, 8, 11, 12, 13, 14] {
                    assert_eq!(
                        actual,
                        bits(row[index]),
                        "actual MutableBlockPos {} column {index}",
                        row[1]
                    );
                }
                assert_eq!(&row[9..11], &["-", "-"]);
            }
            "C" => {
                groups[2] += 1;
                // Actual packed key and retained Vanilla public bits stay
                // diagnostic. Rust compares genuine private/cold and Paper.
                assert_eq!(row[7], row[11], "packed key {}", row[1]);
                for index in [8, 10, 12, 13, 14] {
                    assert_eq!(
                        actual,
                        bits(row[index]),
                        "cache stateless {} column {index}",
                        row[1]
                    );
                }
                cache_differences += usize::from(bits(row[9]) != actual);
            }
            kind => panic!("unexpected kind {kind}"),
        }
    }
    assert_eq!(groups, [93_584, 80, 3156, 32_768]);
    assert_eq!(scalar_groups, [87_360, 3120, 1024, 2080]);
    assert_eq!(branches, [19_296, 13_024, 448]);
    assert_eq!(cache_differences, 57);
}

fn scalar_group(label: &str) -> usize {
    match label.split('/').next().unwrap() {
        "main" => 0,
        "f32-precision" => 1,
        "frozen-grid" => 2,
        "sea-overflow" => 3,
        other => panic!("unknown original input group {other}"),
    }
}

fn verify_extension_noises(table: &BuiltinBiomeTemperatures, pos: BlockPos, row: &[&str]) -> usize {
    let x = f64::from(pos.x());
    let z = f64::from(pos.z());
    let observations = [
        table.frozen.sample(x * 0.05, z * 0.05).unwrap(),
        table.info.sample(x * 0.2, z * 0.2).unwrap(),
        table.info.sample(x * 0.09, z * 0.09).unwrap(),
    ];
    for (index, actual) in observations.iter().enumerate() {
        assert_eq!(
            actual.to_bits(),
            double(row[14 + index]).to_bits(),
            "Vanilla noise {} {index}",
            row[1]
        );
        assert_eq!(
            actual.to_bits(),
            double(row[24 + index]).to_bits(),
            "Paper noise {} {index}",
            row[1]
        );
    }
    if observations[0] * 7.0 + observations[1] >= 0.3 {
        1
    } else if observations[2] >= 0.8 {
        2
    } else {
        0
    }
}

#[test]
fn all_original_metadata_and_complete_json_source_census_match() {
    let table = table();
    assert_eq!(METADATA.lines().count(), RECORDS);
    let census: serde_json::Value = serde_json::from_str(include_str!("../../tests/reference/biome-temperature/fixtures/authority/.codex-tmp/gen05_7/census/metadata.json")).unwrap();
    let mut totals = [0; 3];
    for (record, line) in table.records.iter().zip(METADATA.lines()) {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(record.biome.resource_location(), row[0]);
        assert_eq!(record.climate.temperature.to_bits(), bits(row[1]));
        assert_eq!(record.climate.temperature.to_bits(), bits(row[4]));
        assert_eq!(
            record.climate.has_precipitation,
            row[2].parse::<bool>().unwrap()
        );
        assert_eq!(record.climate.frozen, row[3] == "FROZEN");
        let source = record.biome.definition().json();
        let (decoded, stats) = json::parse(source).unwrap();
        assert_eq!(decoded, record.climate);
        let pinned = &census["full_biome_roster"][row[0]];
        assert_eq!(
            format!("{:x}", Sha256::digest(source.as_bytes())),
            pinned["raw_slice_sha256"].as_str().unwrap()
        );
        assert_eq!(source.len(), pinned["bytes"].as_u64().unwrap() as usize);
        assert_eq!(stats.nodes, pinned["nodes"].as_u64().unwrap() as usize);
        assert_eq!(
            stats.depth,
            pinned["maximum_depth"].as_u64().unwrap() as usize
        );
        totals[0] += source.len();
        totals[1] += stats.nodes;
        totals[2] += 1;
    }
    assert_eq!(totals, [338_695, 14_466, 65]);
    assert!(BiomeResourceId::from_name("nether").is_none());
    assert!(BiomeResourceId::from_name("overworld").is_none());
    for preset in ferrumc_registry::biome::biome_parameters_resources() {
        assert!(json::parse(preset.definition().json()).is_err());
    }
}

#[test]
fn actual_fixed_permutations_factors_draw_chronology_final_state_and_separate_offsets() {
    let table = table();
    let constructors: Vec<_> = VANILLA_CONSTRUCTORS
        .lines()
        .zip(PAPER_CONSTRUCTORS.lines())
        .collect();
    assert_eq!(constructors.len(), 8);
    let mut levels = 0;
    let mut draw_total = 0;
    let mut paper_offset_differences = 0;
    for (vanilla, paper) in constructors {
        let v: Vec<_> = vanilla.split('\t').collect();
        let p: Vec<_> = paper.split('\t').collect();
        assert_eq!(v[..3], p[..3]);
        let (permutations, input, value): (&[SimplexNoise], _, _) = match v[1] {
            "temperature" => (
                &table.temperature.levels,
                table.temperature.input_factor,
                table.temperature.value_factor,
            ),
            "frozen" => (
                &table.frozen.levels,
                table.frozen.input_factor,
                table.frozen.value_factor,
            ),
            "info" => (
                &table.info.levels,
                table.info.input_factor,
                table.info.value_factor,
            ),
            other => panic!("{other}"),
        };
        if v[0] == "N" {
            assert_eq!(v, p);
            let seed = v[2].parse().unwrap();
            let draws: u32 = v[3].parse().unwrap();
            assert_eq!(draws, if v[1] == "frozen" { 786 } else { 262 });
            assert_eq!(input.to_bits(), double(v[5]).to_bits());
            assert_eq!(value.to_bits(), double(v[6]).to_bits());
            assert_eq!(permutations.len(), v[7].parse::<usize>().unwrap());
            let after = if v[1] == "frozen" {
                FixedNoise::frozen_with_state().1
            } else {
                FixedNoise::single_with_state(seed).1
            };
            let mut consumed = WorldgenRandomSource::legacy(WorldSeed::new(seed));
            consumed.consume_count(draws).unwrap();
            assert_eq!(after, consumed, "actual original draws {}", v[1]);
            // Invert only the existing Legacy constructor seed scramble to
            // admit the actual exported state. No expected RNG draws are made.
            let final_state = v[4].parse::<i64>().unwrap();
            assert_eq!(
                after,
                WorldgenRandomSource::legacy(WorldSeed::new(final_state ^ 0x0005_deec_e66d))
            );
            draw_total += draws;
        } else {
            assert_eq!(v[0], "P");
            let level = &permutations[v[2].parse::<usize>().unwrap()];
            let offsets = level.offsets();
            for (index, actual) in [offsets.x, offsets.y, offsets.z].iter().enumerate() {
                assert_eq!(
                    actual.to_bits(),
                    double(v[3 + index]).to_bits(),
                    "Vanilla offset {} {} axis {index}",
                    v[1],
                    v[2]
                );
                assert_ne!(
                    double(v[3 + index]).to_bits(),
                    double(p[3 + index]).to_bits()
                );
                paper_offset_differences += 1;
            }
            assert_eq!(v[6], p[6]);
            let original: Vec<u16> = v[6].split(',').map(|n| n.parse().unwrap()).collect();
            assert_eq!(original.len(), 512);
            for (actual, expected) in level.permutation().iter().zip(&original[..256]) {
                assert_eq!(u16::from(*actual), *expected);
            }
            assert!(original[256..].iter().all(|p| *p == 0));
            levels += 1;
        }
    }
    assert_eq!(
        (levels, draw_total, paper_offset_differences),
        (5, 1310, 15)
    );
}

#[test]
fn every_direct_false_offset_original_noise_probe_matches() {
    let table = table();
    let mut count = 0;
    for line in NOISE.lines() {
        let row: Vec<_> = line.split('\t').collect();
        let variant: usize = row[0].rsplit('/').next().unwrap().parse().unwrap();
        let x: i32 = row[1].parse().unwrap();
        let z: i32 = row[2].parse().unwrap();
        let (scaled_x, scaled_z) = match variant {
            0 => (f64::from(x as f32 / 8.0_f32), f64::from(z as f32 / 8.0_f32)),
            1 => (f64::from(x) * 0.05, f64::from(z) * 0.05),
            2 => (f64::from(x) * 0.2, f64::from(z) * 0.2),
            3 => (f64::from(x) * 0.09, f64::from(z) * 0.09),
            _ => panic!(),
        };
        assert_eq!(scaled_x.to_bits(), double(row[3]).to_bits());
        assert_eq!(scaled_z.to_bits(), double(row[4]).to_bits());
        let actual = match variant {
            0 => table.temperature.sample(scaled_x, scaled_z),
            1 => table.frozen.sample(scaled_x, scaled_z),
            _ => table.info.sample(scaled_x, scaled_z),
        }
        .unwrap();
        assert_eq!(actual.to_bits(), double(row[5]).to_bits(), "{}", row[0]);
        count += 1;
    }
    assert_eq!(count, 352);
}

#[test]
fn clone_reverse_request_order_and_source_lifetimes_never_introduce_history() {
    let clone = {
        let original = table();
        let clone = original.clone();
        assert_eq!(clone, original);
        clone
    };
    for line in TEMPERATURES.lines().rev() {
        let row: Vec<_> = line.split('\t').collect();
        let sample = clone
            .sample(
                BiomeResourceId::from_name(row[2]).unwrap(),
                position(&row),
                row[6].parse().unwrap(),
            )
            .unwrap();
        assert_eq!(
            sample.temperature().to_bits(),
            bits(row[if row[0] == "C" { 8 } else { 7 }]),
            "reverse {}",
            row[1]
        );
    }
    assert_eq!(clone.construction_work(), CONSTRUCTION_WORK);
}

#[test]
fn exact_construction_admission_and_native_capacity() {
    for allowance in [0, 1_048_577, u32::MAX] {
        assert!(
            matches!(BiomeTemperatureBuildBudget::new(allowance).unwrap_err().kind(), Kind::InvalidAllowance(n) if *n == allowance)
        );
    }
    for allowance in [1, CONSTRUCTION_WORK - 1] {
        let error =
            BuiltinBiomeTemperatures::compile(BiomeTemperatureBuildBudget::new(allowance).unwrap())
                .unwrap_err();
        assert_eq!(
            error.kind(),
            &Kind::WorkExhausted {
                required: CONSTRUCTION_WORK,
                allowance
            }
        );
        assert_eq!(error.resource(), None);
        assert_eq!(error.offset(), None);
    }
    for allowance in [CONSTRUCTION_WORK, CONSTRUCTION_WORK + 1, 1_048_576] {
        assert_eq!(
            BuiltinBiomeTemperatures::compile(BiomeTemperatureBuildBudget::new(allowance).unwrap())
                .unwrap()
                .construction_work(),
            CONSTRUCTION_WORK
        );
    }
    assert_eq!(size_of::<SimplexNoise>(), 280);
    assert_eq!(size_of::<Record>(), 12);
    assert_eq!(size_of::<BuiltinBiomeTemperatures>(), 2232);
    const {
        assert!(json::SCRATCH_BYTES < MAX_SCRATCH_BYTES);
    }
    println!(
        "temperature native retained={} declared simultaneous scratch={}",
        size_of::<BuiltinBiomeTemperatures>(),
        json::SCRATCH_BYTES
    );
    for (maximum, category) in [
        (
            MAX_SOURCE_BYTES,
            BiomeTemperatureLimit::AggregateSourceBytes,
        ),
        (RECORDS, BiomeTemperatureLimit::Records),
        (MAX_RETAINED_BYTES, BiomeTemperatureLimit::RetainedBytes),
        (MAX_SCRATCH_BYTES, BiomeTemperatureLimit::ScratchBytes),
    ] {
        assert!(check_limit(maximum - 1, maximum, category).is_ok());
        assert!(check_limit(maximum, maximum, category).is_ok());
        assert_eq!(
            check_limit(maximum + 1, maximum, category)
                .unwrap_err()
                .kind(),
            &Kind::Limit {
                limit: category,
                count: maximum + 1,
                maximum
            }
        );
    }
    assert!(check_census(1, 2).is_err());
}

#[test]
fn public_error_context_preserves_checked_identity_and_all_query_bits() {
    let biome = BiomeResourceId::from_name("frozen_ocean").unwrap();
    let pos = BlockPos::new(i32::MIN, i32::MAX, -1);
    let error = BiomeTemperatureError::new(Kind::Noise(NoiseError::InvalidInput)).at_query(
        biome,
        pos,
        i32::MAX,
    );
    assert_eq!(error.resource(), Some(biome));
    assert_eq!(error.position(), Some(pos));
    assert_eq!(error.sea_level(), Some(i32::MAX));
    assert_eq!(error.field(), None);
}

#[test]
fn complete_build_and_clone_fit_the_one_mebibyte_stack_envelope() {
    let handle = std::thread::Builder::new()
        .stack_size(MAX_SCRATCH_BYTES)
        .spawn(|| {
            let table = table();
            let clone = table.clone();
            for biome in biome_resources() {
                for x in [i32::MIN, i32::MAX] {
                    for y in [i32::MIN, i32::MAX] {
                        for z in [i32::MIN, i32::MAX] {
                            for sea in [i32::MIN, i32::MAX] {
                                assert_eq!(
                                    table.sample(biome, BlockPos::new(x, y, z), sea).unwrap(),
                                    clone.sample(biome, BlockPos::new(x, y, z), sea).unwrap()
                                );
                            }
                        }
                    }
                }
            }
        })
        .unwrap();
    handle.join().unwrap();
}

#[test]
fn imported_fixtures_are_exactly_pinned_and_original_diagnostics_stay_explicit() {
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/reference/biome-temperature/fixtures/provenance.json"
    ))
    .unwrap();
    for (name, data) in [
        ("temperatures.tsv", TEMPERATURES),
        ("metadata.tsv", METADATA),
        ("noise.tsv", NOISE),
        ("vanilla-constructors.tsv", VANILLA_CONSTRUCTORS),
        ("paper-constructors.tsv", PAPER_CONSTRUCTORS),
    ] {
        let pin = &provenance["imported_outputs"][name];
        assert_eq!(data.len(), pin["bytes"].as_u64().unwrap() as usize);
        assert_eq!(
            format!("{:x}", Sha256::digest(data.as_bytes())),
            pin["sha256"].as_str().unwrap()
        );
    }
    let diagnostics: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/reference/biome-temperature/fixtures/diagnostics.json"
    ))
    .unwrap();
    assert_eq!(
        diagnostics["offset_differences"].as_array().unwrap().len(),
        15
    );
    assert_eq!(
        diagnostics["modes"]["vanilla"]["retained_public_different_from_private"],
        57
    );
    assert_eq!(
        diagnostics["modes"]["paper"]["retained_public_different_from_private"],
        0
    );
    assert_eq!(diagnostics["stateless_difference_count"], 0);
    for source in [
        include_str!("../../tests/reference/biome-temperature/fixtures/vanilla-origins.tsv"),
        include_str!("../../tests/reference/biome-temperature/fixtures/paper-origins.tsv"),
    ] {
        assert_eq!(source.lines().count(), 47);
    }
}
