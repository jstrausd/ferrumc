//! Independent public-consumer checks using frozen actual Paper density bits.

use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;
use ferrumc_world::density::{
    ClimateQuartPos, NoiseChunkErrorKind, NoiseChunkWorkBudget, RouterTemplate, SeededRouter,
    MAX_NOISE_CHUNK_WORK,
};
use ferrumc_world::WorldSeed;

fn router(name: &str, seed: i64) -> SeededRouter {
    RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name(name).unwrap())
        .unwrap()
        .bind_seed(WorldSeed::new(seed))
        .unwrap()
}
fn budget() -> NoiseChunkWorkBudget {
    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap()
}

#[test]
fn actual_paper_bits_survive_source_drop_and_preserve_negative_coordinates() {
    // Values extracted independently from the frozen Root actual-JAR 126-chunk export.
    // SHA256 material_density.bits: 9dd6827609a0a6b65968b239a588fd60e21121097520501ff5828eb19655a9db.
    for (name, seed, x, z, height, points) in [
        (
            "minecraft:overworld",
            0,
            -1,
            -1,
            384usize,
            [
                (-64, 0, 0, 0x3fa3_30e5_6041_8937_u64),
                (-64, 15, 15, 0x3fa3_30e5_6041_8937),
                (128, 3, 7, 0xbfdd_5555_5555_5555),
                (129, 11, 5, 0xbfdd_5555_5555_5555),
                (319, 0, 0, 0xbf99_983c_131d_5acc),
                (319, 15, 15, 0xbf99_983c_131d_5acc),
            ],
        ),
        (
            "minecraft:amplified",
            0x0123_4567_89ab_cdef,
            1,
            0,
            384usize,
            [
                (-64, 0, 0, 0x3fc0_4b65_c311_7633),
                (-64, 15, 15, 0x3fc0_4b65_c311_7633),
                (128, 3, 7, 0x3fba_a2e2_118b_50bc),
                (129, 11, 5, 0x3fc7_e1a0_777c_cd50),
                (319, 0, 0, 0xbfa5_49c0_14be_0f37),
                (319, 15, 15, 0xbfa3_e67c_8432_96a1),
            ],
        ),
        (
            "minecraft:end",
            i64::MIN,
            -1_875_000,
            -1_875_000,
            128usize,
            [
                (0, 0, 0, 0xbfb3_29fb_e76c_8b43),
                (0, 15, 15, 0xbfb3_29fb_e76c_8b43),
                (64, 3, 7, 0xbfc7_4a40_62a4_696a),
                (65, 11, 5, 0xbfc8_d82d_fc23_c876),
                (127, 0, 0, 0xbfdd_5555_5555_5555),
                (127, 15, 15, 0xbfdd_5555_5555_5555),
            ],
        ),
    ] {
        let source = router(name, seed);
        let column = ChunkPos::new(x, z);
        let mut session = source.fresh_paper_noise_chunk(column, budget()).unwrap();
        drop(source);
        let output = session.sample_chunk_densities().unwrap();
        drop(session);
        assert_eq!(output.seed(), WorldSeed::new(seed));
        assert_eq!(output.resource().resource_location(), name);
        assert_eq!(output.chunk_pos(), column);
        assert_eq!(output.values().len(), height * 256);
        for (y, lx, lz, expected) in points {
            let pos = BlockPos::new(x * 16 + lx, y, z * 16 + lz);
            assert_eq!(
                output.density_at(pos).unwrap().to_bits(),
                expected,
                "{name}/{seed}/{pos:?}"
            );
        }
        let bottom = output.geometry().min_y();
        let height_i32 = i32::try_from(height).unwrap();
        assert_eq!(
            output.density_at(BlockPos::new(x * 16, bottom - 1, z * 16)),
            None
        );
        assert_eq!(
            output.density_at(BlockPos::new(x * 16, bottom + height_i32, z * 16)),
            None
        );
        assert_eq!(
            output.density_at(BlockPos::new(x * 16 + 16, bottom, z * 16)),
            None
        );
        assert_eq!(
            output.density_at(BlockPos::new(x * 16, bottom, z * 16 - 1)),
            None
        );
    }
}

#[test]
fn exhausted_session_retains_poison_across_failed_reset_then_rebuilds() {
    let source = router("minecraft:nether", -37);
    let full = source
        .fresh_paper_noise_chunk(ChunkPos::new(-1, 2), budget())
        .unwrap();
    let constructor_units = u32::try_from(full.work_used()).unwrap();
    drop(full);
    let mut session = source
        .fresh_paper_noise_chunk(
            ChunkPos::new(-1, 2),
            NoiseChunkWorkBudget::new(constructor_units).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        session.sample_chunk_densities().unwrap_err().kind(),
        NoiseChunkErrorKind::WorkExceeded { .. }
    ));
    let spent = session.work_used();
    assert!(matches!(
        session
            .reset(NoiseChunkWorkBudget::new(1).unwrap())
            .unwrap_err()
            .kind(),
        NoiseChunkErrorKind::WorkExceeded { .. }
    ));
    assert_eq!(session.work_used(), spent);
    assert!(matches!(
        session.sample_chunk_densities().unwrap_err().kind(),
        NoiseChunkErrorKind::Lifecycle {
            phase: "poisoned",
            ..
        }
    ));
    assert!(matches!(
        session
            .sample_climate(ClimateQuartPos::new(0, 0, 0).unwrap())
            .unwrap_err()
            .kind(),
        NoiseChunkErrorKind::Lifecycle {
            phase: "poisoned",
            ..
        }
    ));
    session.reset(budget()).unwrap();
    let output = session.sample_chunk_densities().unwrap();
    assert_eq!(output.values().len(), 128 * 256);
    assert!(matches!(
        session.sample_chunk_densities().unwrap_err().kind(),
        NoiseChunkErrorKind::Lifecycle {
            phase: "complete",
            ..
        }
    ));
}

#[test]
fn public_chunk_domain_includes_interpolation_corner_without_i32_wrap() {
    let source = router("minecraft:end", 0);
    for (x, z) in [
        (-1_875_000, 0),
        (0, -1_875_000),
        (1_874_999, 0),
        (0, 1_874_999),
    ] {
        assert!(source
            .fresh_paper_noise_chunk(ChunkPos::new(x, z), budget())
            .is_ok());
    }
    for (x, z) in [
        (-1_875_001, 0),
        (0, -1_875_001),
        (1_875_000, 0),
        (0, 1_875_000),
        (i32::MIN, 0),
        (0, i32::MAX),
    ] {
        assert!(matches!(
            source
                .fresh_paper_noise_chunk(ChunkPos::new(x, z), budget())
                .err()
                .unwrap()
                .kind(),
            NoiseChunkErrorKind::ChunkOutOfRange { .. }
        ));
    }
}
