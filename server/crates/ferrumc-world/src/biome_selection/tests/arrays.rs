use std::fs::File;
use std::io::{BufRead, BufReader};

use super::*;
use crate::density::{
    ClimateQuartPos, NoiseChunkSession, NoiseChunkWorkBudget, RouterTemplate, MAX_NOISE_CHUNK_WORK,
};
use crate::WorldSeed;
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;

pub(super) fn noise(name: &str, seed: i64, chunk: ChunkPos) -> NoiseChunkSession {
    RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name(name).unwrap())
        .unwrap()
        .bind_seed(WorldSeed::new(seed))
        .unwrap()
        .fresh_paper_noise_chunk(chunk, noise_budget())
        .unwrap()
}
pub(super) fn noise_budget() -> NoiseChunkWorkBudget {
    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap()
}
pub(super) fn work_budget() -> BiomeWorkBudget {
    BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap()
}
fn original_ordinal(value: &Value) -> Option<usize> {
    value.as_u64().map(|v| usize::try_from(v).unwrap())
}
fn selection_ordinal(value: Option<BiomeSelection>) -> Option<usize> {
    value.map(BiomeSelection::parameter_ordinal)
}

type Trace = Vec<(ClimateQuartPos, BiomeResourceId, Option<BiomeSelection>)>;

fn check_trace(
    source: &mut BiomeSourceSession,
    row: &Value,
    result: &ChunkBiomeSamples,
    label: &str,
) -> Trace {
    let trace = source.take_trace();
    assert_eq!(trace.len(), row["trace_quarts"].as_array().unwrap().len());
    for (i, (pos, biome, selection)) in trace.iter().enumerate() {
        let block = pos.block_origin();
        let expected = &row["trace_quarts"][i];
        assert_eq!(
            [
                i64::from(block.x() >> 2),
                i64::from(block.y() >> 2),
                i64::from(block.z() >> 2)
            ],
            [
                expected[0].as_i64().unwrap(),
                expected[1].as_i64().unwrap(),
                expected[2].as_i64().unwrap()
            ],
            "{label} actual query{i}"
        );
        assert_eq!(result.biome_at(block), Some(*biome));
        if row.get("ordinals").is_some() {
            assert_eq!(
                selection_ordinal(*selection),
                original_ordinal(&row["ordinals"][i]),
                "{label} original query ordinal{i}"
            );
        } else {
            assert_eq!(*selection, None);
        }
    }
    assert_eq!(
        result.biome_at(BlockPos::new(
            result.chunk_pos().x() * 16,
            result.geometry().min_y() - 1,
            result.chunk_pos().z() * 16
        )),
        None
    );
    assert_eq!(
        result.biome_at(BlockPos::new(
            result.chunk_pos().x() * 16 + 16,
            result.geometry().min_y(),
            result.chunk_pos().z() * 16
        )),
        None
    );
    trace
}

fn check_end_calls(
    row: &Value,
    trace: &[(ClimateQuartPos, BiomeResourceId, Option<BiomeSelection>)],
    result: &ChunkBiomeSamples,
    label: &str,
) {
    if row.get("end_calls").is_some() {
        let mut sampled = noise(
            result.resource().resource_location(),
            result.seed().value(),
            result.chunk_pos(),
        );
        for (i, calls) in row["end_calls"].as_array().unwrap().iter().enumerate() {
            let origin = trace[i].0;
            let mut work = SampleWork::new(MAX_BIOME_SELECTION_WORK as usize);
            let center = end::center(origin, &mut work).unwrap();
            if calls.as_array().unwrap().is_empty() {
                assert_eq!(center, None, "{label} central End query{i}");
            } else {
                let call = &calls[0];
                let expected = BlockPos::new(
                    i32::try_from(call[1].as_i64().unwrap()).unwrap(),
                    i32::try_from(call[2].as_i64().unwrap()).unwrap(),
                    i32::try_from(call[3].as_i64().unwrap()).unwrap(),
                );
                assert_eq!(center, Some(expected));
                assert_eq!(
                    sampled.cached_biome_erosion(expected).unwrap().to_bits(),
                    u64::from_str_radix(call[4].as_str().unwrap(), 16).unwrap(),
                    "{label} cached f64 End query{i}"
                );
            }
        }
    }
}

fn oracle_identity(row: &Value, counts: &mut [usize; 3]) -> BiomeSourceIdentity {
    let name = row["noise_settings"].as_str().unwrap();
    match row["source_class"]
        .as_str()
        .unwrap()
        .rsplit('.')
        .next()
        .unwrap()
    {
        "MultiNoiseBiomeSource" => {
            counts[0] += 1;
            BiomeSourceIdentity::MultiNoise(
                BiomeParametersResourceId::from_name(if name == "minecraft:nether" {
                    "minecraft:nether"
                } else {
                    "minecraft:overworld"
                })
                .unwrap(),
            )
        }
        "TheEndBiomeSource" => {
            counts[1] += 1;
            BiomeSourceIdentity::End
        }
        "FixedBiomeSource" => {
            counts[2] += 1;
            BiomeSourceIdentity::Fixed(BiomeResourceId::from_name("minecraft:plains").unwrap())
        }
        other => panic!("unsupported oracle source {other}"),
    }
}

#[test]
fn every_complete_actual_create_biomes_array_traversal_and_retained_leaf_matches() {
    let overworld = tree("minecraft:overworld");
    let nether = tree("minecraft:nether");
    let mut source = BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
        Arc::clone(&overworld),
        work_budget(),
    ));
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/biome-arrays-original-1.21.8/arrays-compact.jsonl");
    let mut counts = [0; 3];
    for line in BufReader::new(File::open(path).unwrap()).lines() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let name = row["noise_settings"].as_str().unwrap();
        let seed = row["seed"].as_i64().unwrap();
        let position = ChunkPos::new(
            i32::try_from(row["chunk"][0].as_i64().unwrap()).unwrap(),
            i32::try_from(row["chunk"][1].as_i64().unwrap()).unwrap(),
        );
        let identity = oracle_identity(&row, &mut counts);
        if source.identity() != identity {
            source = match identity {
                BiomeSourceIdentity::MultiNoise(preset) => {
                    BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
                        Arc::clone(if preset.resource_location() == "minecraft:nether" {
                            &nether
                        } else {
                            &overworld
                        }),
                        work_budget(),
                    ))
                }
                BiomeSourceIdentity::Fixed(biome) => {
                    BiomeSourceSession::fixed(biome, work_budget())
                }
                BiomeSourceIdentity::End => BiomeSourceSession::the_end(work_budget()).unwrap(),
            };
        }
        if row["cold"].as_bool().unwrap() {
            source.reset(work_budget());
        }
        let label = format!(
            "{} {} {seed} {position:?}/{}",
            row["world_preset"], row["dimension"], row["group"]
        );
        assert_eq!(
            selection_ordinal(source.last_selection()),
            original_ordinal(&row["initial_ordinal"]),
            "{label} initial"
        );
        let mut noise = noise(name, seed, position);
        let geometry = BiomeQuartGeometry::new(
            i32::try_from(row["min_y"].as_i64().unwrap()).unwrap(),
            u32::try_from(row["height"].as_u64().unwrap()).unwrap(),
        )
        .unwrap();
        source.enable_trace();
        let result = source.sample_chunk_biomes(&mut noise, geometry).unwrap();
        assert_eq!(result.seed(), WorldSeed::new(seed));
        assert_eq!(result.chunk_pos(), position);
        assert_eq!(result.geometry(), geometry);
        assert_eq!(result.source(), identity);
        assert_eq!(result.resource().resource_location(), name);
        assert_eq!(
            selection_ordinal(result.initial_selection()),
            original_ordinal(&row["initial_ordinal"]),
            "{label} output initial"
        );
        assert_eq!(
            selection_ordinal(result.final_selection()),
            original_ordinal(&row["final_ordinal"]),
            "{label} output final"
        );
        assert_eq!(
            selection_ordinal(source.last_selection()),
            original_ordinal(&row["final_ordinal"]),
            "{label} committed final"
        );
        let expected = row["biomes"].as_array().unwrap();
        assert_eq!(result.values().len(), expected.len());
        for (i, (actual, expected)) in result.values().iter().zip(expected).enumerate() {
            assert_eq!(
                actual.resource_location(),
                expected.as_str().unwrap(),
                "{label} stored quart{i}"
            );
        }
        let trace = check_trace(&mut source, &row, &result, &label);
        check_end_calls(&row, &trace, &result, &label);
    }
    assert_eq!(counts, [88, 64, 4]);
}

#[test]
fn original_end_exact_f64_threshold_and_radius_probes_match() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/biome-arrays-original-1.21.8/end-probes.jsonl");
    let names = [
        "minecraft:the_end",
        "minecraft:end_highlands",
        "minecraft:end_midlands",
        "minecraft:small_end_islands",
        "minecraft:end_barrens",
    ];
    let mut count = 0;
    for line in BufReader::new(File::open(path).unwrap()).lines() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let q = &row["quart"];
        let pos = ClimateQuartPos::new(
            i32::try_from(q[0].as_i64().unwrap()).unwrap(),
            i32::try_from(q[1].as_i64().unwrap()).unwrap(),
            i32::try_from(q[2].as_i64().unwrap()).unwrap(),
        )
        .unwrap();
        let mut work = SampleWork::new(100);
        let center = end::center(pos, &mut work).unwrap();
        let expected = &row["density_calls"];
        let branch = if expected.as_array().unwrap().is_empty() {
            assert_eq!(center, None);
            0
        } else {
            let call = &expected[0];
            assert_eq!(
                center,
                Some(BlockPos::new(
                    i32::try_from(call[1].as_i64().unwrap()).unwrap(),
                    i32::try_from(call[2].as_i64().unwrap()).unwrap(),
                    i32::try_from(call[3].as_i64().unwrap()).unwrap()
                ))
            );
            end::branch(
                f64::from_bits(
                    u64::from_str_radix(row["input_bits"].as_str().unwrap(), 16).unwrap(),
                ),
                &mut work,
            )
            .unwrap()
        };
        assert_eq!(names[branch], row["biome"].as_str().unwrap());
        count += 1;
    }
    assert_eq!(count, 27);
}
