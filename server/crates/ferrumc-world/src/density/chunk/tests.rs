#![expect(
    clippy::cast_possible_wrap,
    reason = "Reference fixture coordinates and array indices are explicitly bounded."
)]
use super::state::Cache;
use super::*;
use crate::density::RouterTemplate;

pub(super) fn router(name: &str, seed: i64) -> SeededRouter {
    RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name(name).unwrap())
        .unwrap()
        .bind_seed(WorldSeed::new(seed))
        .unwrap()
}
pub(super) fn budget() -> NoiseChunkWorkBudget {
    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap()
}

#[test]
fn actual_paper_constructor_census_seven_settings_seven_seeds() {
    for (name, counts, entries) in [
        ("minecraft:overworld", [8, 10, 6, 6, 1], 254),
        ("minecraft:large_biomes", [8, 10, 6, 6, 1], 254),
        ("minecraft:amplified", [8, 10, 6, 6, 1], 254),
        ("minecraft:caves", [1, 6, 4, 0, 1], 60),
        ("minecraft:floating_islands", [1, 6, 4, 0, 1], 60),
        ("minecraft:nether", [1, 6, 4, 0, 1], 60),
        ("minecraft:end", [1, 2, 1, 0, 1], 63),
    ] {
        for seed in [
            0,
            1,
            -1,
            i64::MIN,
            i64::MAX,
            0x0123_4567_89ab_cdef,
            -0x0123_4567_890a_bcde,
        ] {
            let session = router(name, seed)
                .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
                .unwrap();
            let mut actual = [0; 5];
            for wrapper in &session.state.wrappers {
                actual[match wrapper.cache {
                    Cache::Interpolated(_) => 0,
                    Cache::Flat(_) => 1,
                    Cache::TwoD { .. } => 2,
                    Cache::Once { .. } => 3,
                    Cache::Cell(_) => 4,
                }] += 1;
            }
            eprintln!("{name} seed={seed} wrappers={actual:?} entries={} vertices={} cachebytes={} graphbytes={} samplerbytes={} scratchbytes={} work={}",session.state.wrap_entries,session.view.nodes.len(),session.state.cache_bytes,session.state.graph_bytes,session.state.sampler_bytes,session.state.scratch_bytes,session.work_used());
            assert_eq!(actual, counts, "{name}/{seed}");
            assert_eq!(session.state.wrap_entries, entries, "{name}/{seed}");
        }
    }
}

mod failure;
mod oracle;
pub(super) mod requests;
mod wrappers;

#[test]
fn full_material_driver_lifecycle_output_and_reset() {
    let router = router("minecraft:overworld", 0);
    let mut session = router
        .fresh_paper_noise_chunk(ChunkPos::new(-1, -1), budget())
        .unwrap();
    let surface = session
        .preliminary_surface_level(BlockPos::new(-3, i32::MAX, -6))
        .unwrap();
    assert_eq!(
        surface,
        session
            .preliminary_surface_level(BlockPos::new(-4, i32::MIN, -8))
            .unwrap()
    );
    let climate = session
        .sample_climate(ClimateQuartPos::new(-1, 4, -2).unwrap())
        .unwrap();
    let first = session.sample_chunk_densities().unwrap();
    assert_eq!(first.values().len(), 98_304);
    assert!(first.values().iter().all(|v| v.is_finite()));
    assert_eq!(
        first.density_at(BlockPos::new(-16, -64, -16)),
        Some(first.values()[0])
    );
    assert_eq!(
        first.density_at(BlockPos::new(-1, 319, -1)),
        first.values().last().copied()
    );
    assert_eq!(first.density_at(BlockPos::new(0, 64, 0)), None);
    assert!(matches!(
        session.sample_chunk_densities().unwrap_err().kind(),
        NoiseChunkErrorKind::Lifecycle { .. }
    ));
    assert_eq!(
        climate,
        session
            .sample_climate(ClimateQuartPos::new(-1, 4, -2).unwrap())
            .unwrap()
    );
    session.reset(budget()).unwrap();
    let second = session.sample_chunk_densities().unwrap();
    assert!(first
        .values()
        .iter()
        .zip(second.values())
        .all(|(a, b)| a.to_bits() == b.to_bits()));
    eprintln!("full driver work={}", session.work_used());
}

#[test]
fn actual_paper_126_complete_density_chunks() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/noise_chunk_1_21_8");
    let records = std::fs::read_to_string(directory.join("chunks.jsonl")).unwrap();
    let bytes = std::fs::read(directory.join("material_density.bits")).unwrap();
    assert_eq!(bytes.len(), 56_623_104);
    let mut samples = 0;
    for row in records.lines() {
        let row: serde_json::Value = serde_json::from_str(row).unwrap();
        let name = row["settings"].as_str().unwrap();
        let seed = row["seed"].as_str().unwrap().parse::<i64>().unwrap();
        let chunk = ChunkPos::new(
            row["chunk_x"].as_i64().unwrap() as i32,
            row["chunk_z"].as_i64().unwrap() as i32,
        );
        let mut session = router(name, seed)
            .fresh_paper_noise_chunk(chunk, budget())
            .unwrap();
        let values = session.sample_chunk_densities().unwrap();
        let offset = row["offset"].as_u64().unwrap() as usize;
        let length = row["bytes"].as_u64().unwrap() as usize;
        assert_eq!(length, values.values().len() * 8);
        let expected = bytes
            .get(offset..offset.checked_add(length).unwrap())
            .unwrap();
        for (i, (&actual, expected)) in values
            .values()
            .iter()
            .zip(expected.as_chunks::<8>().0)
            .enumerate()
        {
            let bits = u64::from_be_bytes(*expected);
            assert_eq!(actual.to_bits(), bits, "{name}/{seed}/{chunk:?} index={i}");
        }
        samples += values.values().len();
        eprintln!(
            "{name}/{seed}/{chunk:?}: {} exact Paperbits, work={}",
            values.values().len(),
            session.work_used()
        );
    }
    assert_eq!(records.lines().count(), 126);
    assert_eq!(samples, 7_077_888);
}

pub(crate) mod precharge;
