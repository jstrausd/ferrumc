//! `FerrumC` admission, refusal-order, budget and capacity policy.
//!
//! These are policy and malformed-input controls, separate from original
//! equality. Real GEN-05.5 arrays are used wherever an actual producer can
//! express the defect; private controls reach only the checked kernel.

use super::*;

fn kind<T>(result: &Result<T, StoredBiomeError>) -> StoredBiomeErrorKind {
    match result {
        Ok(_) => panic!("unexpected admission"),
        Err(error) => error.kind(),
    }
}

fn geometry(min_y: i32, height: u32) -> BiomeQuartGeometry {
    BiomeQuartGeometry::new(min_y, height).unwrap()
}

#[test]
fn build_budget_admits_exactly_one_through_the_maximum() {
    for units in [0, MAX_STORED_BIOME_BUILD_WORK + 1, u32::MAX] {
        let error = StoredBiomeBuildBudget::new(units).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::InvalidBudget);
        assert_eq!(error.limit(), Some(StoredBiomeLimit::ConstructionWork));
        assert_eq!(error.requested(), units as usize);
        assert_eq!(error.maximum(), 1_048_576);
    }
    for units in [
        1,
        2,
        MAX_STORED_BIOME_BUILD_WORK - 1,
        MAX_STORED_BIOME_BUILD_WORK,
    ] {
        assert_eq!(budget(units).units(), units);
    }
}

#[test]
fn reused_admit_chunk_boundaries_on_both_axes() {
    // A1: the single accepted predicate, first/last admitted chunk and both neighbours.
    for value in [-1_875_000, 1_874_999] {
        for chunk in [ChunkPos::new(value, 0), ChunkPos::new(0, value)] {
            assert_eq!(admit_domain(chunk), Ok(()));
            assert!(admit_chunk(chunk).is_ok());
        }
    }
    for value in [-1_875_001, 1_875_000, i32::MIN, i32::MAX] {
        for chunk in [ChunkPos::new(value, 0), ChunkPos::new(0, value)] {
            let error = admit_domain(chunk).unwrap_err();
            assert_eq!(error.kind(), StoredBiomeErrorKind::CoordinateDomain);
            assert_eq!(error.chunk(), Some(chunk));
            assert!(admit_chunk(chunk).is_err());
        }
    }
}

#[test]
fn chunk_count_zero_one_eight_nine_ten() {
    let identity = control_identity(geometry(0, 16));
    let center = ChunkPos::new(3, -4);
    let mut controls = window(identity, center);
    controls.push(control(identity, center));
    let refs: Vec<_> = controls.iter().collect();
    for count in [0, 10] {
        let error = admit_controls(identity, center, &refs[..count], 1_000).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::InvalidChunkCount);
        assert_eq!(error.limit(), Some(StoredBiomeLimit::Chunks));
        assert_eq!((error.requested(), error.maximum()), (count, 9));
    }
    for count in [1, 8, 9] {
        let core = admit_controls(identity, center, &refs[..count], 1_000).unwrap();
        assert_eq!(usize::from(core.count), count);
        assert_eq!(core.borrowed_values as usize, count * 64);
        assert_eq!(core.construction_work as usize, 2 + 10 * count + 64 * count);
    }
}

#[test]
fn every_missing_halo_slot_is_sparse_for_lookup_and_refused_for_surface() {
    let identity = control_identity(geometry(-64, 384));
    let center = ChunkPos::new(-5, 7);
    let controls = window(identity, center);
    for missing in 0..9 {
        let refs: Vec<_> = controls
            .iter()
            .enumerate()
            .filter(|(slot, _)| *slot != missing)
            .map(|(_, c)| c)
            .collect();
        let core = admit_controls(identity, center, &refs, 100_000).unwrap();
        let key = slot_key(center, missing).unwrap();
        let error = complete_halo(&core).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::MissingChunk);
        assert_eq!(error.chunk(), Some(key));
        for slot in 0..9 {
            let target = slot_key(center, slot).unwrap();
            let quart = StoredBiomeQuartPos::new(target.x() * 4 + 1, 0, target.z() * 4 + 2);
            let result = core.resolve(quart);
            if slot == missing {
                let error = result.unwrap_err();
                assert_eq!(error.kind(), StoredBiomeErrorKind::MissingChunk);
                assert_eq!(error.chunk(), Some(key));
                assert_eq!(error.quart(), Some(quart));
            } else {
                assert_eq!(result.unwrap().0, target);
            }
        }
    }
}

#[test]
fn duplicates_first_last_and_multiple_refuse_deterministically() {
    let identity = control_identity(geometry(0, 16));
    let center = ChunkPos::ORIGIN;
    let controls = window(identity, center);
    let r: Vec<_> = controls.iter().collect();
    for (refs, at) in [
        (vec![r[0], r[0], r[1]], 1),
        (vec![r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[1]], 7),
        (vec![r[4], r[5], r[5], r[4], r[4]], 2),
    ] {
        let error = admit_controls(identity, center, &refs, 1_000).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::DuplicateChunk);
        assert_eq!(error.input_index(), Some(at));
        assert_eq!(error.chunk(), Some(refs[at].chunk));
    }
}

#[test]
fn each_inconsistent_identity_component_refuses_in_order_with_controls() {
    let identity = control_identity(geometry(0, 256));
    let center = ChunkPos::new(10, 10);
    let base = control(identity, center);
    let other_geometry = geometry(0, 128);
    let cases = [
        (
            Control {
                seed: WorldSeed::new(1),
                resource: settings("minecraft:nether"),
                ..base.clone()
            },
            StoredBiomeErrorKind::InconsistentSeed,
        ),
        (
            Control {
                resource: settings("minecraft:nether"),
                source: BiomeSourceIdentity::Fixed(biome("plains")),
                ..base.clone()
            },
            StoredBiomeErrorKind::InconsistentSettings,
        ),
        (
            Control {
                source: BiomeSourceIdentity::Fixed(biome("plains")),
                geometry: other_geometry,
                ..base.clone()
            },
            StoredBiomeErrorKind::InconsistentSource,
        ),
        (
            Control {
                geometry: other_geometry,
                values: vec![biome("plains"); 3],
                ..base.clone()
            },
            StoredBiomeErrorKind::InconsistentGeometry,
        ),
    ];
    let valid = control(identity, ChunkPos::new(11, 10));
    for (bad, expected) in &cases {
        let error = admit_controls(identity, center, &[&valid, bad], 100_000).unwrap_err();
        assert_eq!(error.kind(), *expected);
        assert_eq!(error.input_index(), Some(1));
        assert_eq!(error.chunk(), Some(center));
    }
}

#[test]
fn real_arrays_refuse_mixed_seed_settings_source_and_geometry() {
    let normal = generated_group("generated/normal/overworld/0/cold");
    let refs = normal.refs();
    // Each mixed pair comes from actual GEN-05.5 producers; values may coincide.
    for (other, expected) in [
        (
            "generated/normal/overworld/-1/cold",
            StoredBiomeErrorKind::InconsistentSeed,
        ),
        (
            "generated/large_biomes/overworld/0/cold",
            StoredBiomeErrorKind::InconsistentSettings,
        ),
        (
            "generated/single_biome_surface/overworld/0/cold",
            StoredBiomeErrorKind::InconsistentSource,
        ),
        (
            "generated/normal/overworld/0/accessor--80-416",
            StoredBiomeErrorKind::InconsistentGeometry,
        ),
    ] {
        let other = generated_group(other);
        let position = other
            .arrays
            .iter()
            .position(|a| a.chunk_pos() == ChunkPos::new(1, 1))
            .unwrap();
        let mut mixed: Vec<_> = refs
            .iter()
            .copied()
            .filter(|a| a.chunk_pos() != ChunkPos::new(1, 1))
            .collect();
        mixed.push(&other.arrays[position]);
        let error =
            StoredBiomeNeighborhood::new(normal.identity, normal.center, &mixed, max_budget())
                .unwrap_err();
        assert_eq!(
            error.kind(),
            expected,
            "{}",
            fixture().groups[other.group].id
        );
        assert_eq!(error.input_index(), Some(8));
        assert_eq!(error.chunk(), Some(ChunkPos::new(1, 1)));
        let error = SurfaceBiomeResolver::new(
            normal.identity,
            normal.center,
            &mixed,
            BiomeZoomSemantics::Paper1_21_8Build60,
            max_budget(),
        )
        .unwrap_err();
        assert_eq!(error.kind(), expected);
    }
}

#[test]
fn wrong_partial_extra_and_oversized_array_lengths() {
    let small = control_identity(geometry(0, 16));
    let large = control_identity(geometry(-64, 512));
    let center = ChunkPos::ORIGIN;
    let with_length = |identity: StoredBiomeIdentity, length: usize| Control {
        values: vec![biome("plains"); length],
        ..control(identity, center)
    };
    for (identity, length) in [(small, 0), (small, 63), (small, 65), (large, 2047)] {
        let bad = with_length(identity, length);
        let error = admit_controls(identity, center, &[&bad], 100_000).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::IncompleteArray);
        assert_eq!(
            (error.requested(), error.maximum()),
            (length, identity.geometry().sample_count())
        );
        assert_eq!(error.input_index(), Some(0));
    }
    let exact = with_length(large, 2048);
    assert!(admit_controls(large, center, &[&exact], 100_000).is_ok());
    let oversized = with_length(large, 2049);
    let error = admit_controls(large, center, &[&oversized], 100_000).unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::NativeLimit);
    assert_eq!(error.limit(), Some(StoredBiomeLimit::ValuesPerChunk));
    assert_eq!((error.requested(), error.maximum()), (2049, 2048));
}

fn multinoise(preset: &str) -> StoredBiomeIdentity {
    StoredBiomeIdentity::new(
        WorldSeed::new(0),
        settings("minecraft:overworld"),
        BiomeSourceIdentity::MultiNoise(BiomeParametersResourceId::from_name(preset).unwrap()),
        geometry(0, 16),
    )
}

fn point(preset: &str, ordinal: usize) -> HistoryPoint {
    let entry = BiomeParametersResourceId::from_name(preset)
        .unwrap()
        .parameters()[ordinal];
    HistoryPoint {
        ordinal,
        biome: entry.biome(),
    }
}

#[test]
fn invalid_histories_and_fixed_content_refuse() {
    let center = ChunkPos::ORIGIN;
    let identity = multinoise("minecraft:nether");
    let last = point("minecraft:nether", 4);
    let valid = Control {
        values: vec![last.biome; 64],
        last: Some(last),
        initial: Some(point("minecraft:nether", 0)),
        ..control(identity, center)
    };
    assert!(admit_controls(identity, center, &[&valid], 1_000).is_ok());
    let cold = Control {
        initial: None,
        ..valid.clone()
    };
    assert!(admit_controls(identity, center, &[&cold], 1_000).is_ok());
    let other = point("minecraft:nether", 0);
    let mismatched = HistoryPoint {
        ordinal: 4,
        biome: other.biome,
    };
    for bad in [
        Control {
            last: None,
            ..valid.clone()
        },
        Control {
            last: Some(HistoryPoint { ordinal: 5, ..last }),
            ..valid.clone()
        },
        Control {
            last: Some(mismatched),
            ..valid.clone()
        },
        Control {
            initial: Some(HistoryPoint { ordinal: 5, ..last }),
            ..valid.clone()
        },
        Control {
            initial: Some(mismatched),
            ..valid.clone()
        },
        // Final selection does not match the last execution/storage slot.
        Control {
            values: vec![other.biome; 64],
            ..valid.clone()
        },
    ] {
        let error = admit_controls(identity, center, &[&bad], 1_000).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::InvalidHistory);
        assert_eq!(error.chunk(), Some(center));
        assert_eq!(error.input_index(), Some(0));
    }
    let overworld = multinoise("minecraft:overworld");
    let high = point("minecraft:overworld", 7592);
    let edge = Control {
        values: vec![high.biome; 64],
        last: Some(high),
        ..control(overworld, center)
    };
    assert!(admit_controls(overworld, center, &[&edge], 1_000).is_ok());
    let beyond = Control {
        last: Some(HistoryPoint {
            ordinal: 7593,
            ..high
        }),
        ..edge
    };
    assert_eq!(
        admit_controls(overworld, center, &[&beyond], 1_000)
            .unwrap_err()
            .kind(),
        StoredBiomeErrorKind::InvalidHistory
    );
}

#[test]
fn fixed_and_end_histories_and_fixed_content_refuse() {
    let center = ChunkPos::ORIGIN;
    let last = point("minecraft:nether", 4);
    // Fixed and End never carry leaf history; fixed content is complete equality.
    for source in [
        BiomeSourceIdentity::Fixed(biome("plains")),
        BiomeSourceIdentity::End,
    ] {
        let identity = StoredBiomeIdentity::new(
            WorldSeed::new(0),
            settings("minecraft:overworld"),
            source,
            geometry(0, 16),
        );
        for (initial, last) in [(Some(last), None), (None, Some(last))] {
            let bad = Control {
                values: vec![biome("plains"); 64],
                initial,
                last,
                ..control(identity, center)
            };
            assert_eq!(
                admit_controls(identity, center, &[&bad], 1_000)
                    .unwrap_err()
                    .kind(),
                StoredBiomeErrorKind::InvalidHistory
            );
        }
    }
    let fixed = StoredBiomeIdentity::new(
        WorldSeed::new(0),
        settings("minecraft:overworld"),
        BiomeSourceIdentity::Fixed(biome("plains")),
        geometry(0, 16),
    );
    let mut values = vec![biome("plains"); 64];
    values[37] = biome("desert");
    let bad = Control {
        values,
        ..control(fixed, center)
    };
    let error = admit_controls(fixed, center, &[&bad], 1_000).unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::InvalidFixedValue);
    assert_eq!((error.requested(), error.maximum()), (37, 64));
    assert_eq!(error.chunk(), Some(center));
}

#[test]
fn one_thirty_two_and_thirty_three_sections() {
    assert!(BiomeQuartGeometry::new(0, 528).is_err());
    for (min_y, height) in [
        (0, 16),
        (-64, 512),
        (2_147_483_632, 16),
        (i32::MIN, 16),
        (i32::MIN, 512),
    ] {
        let identity = control_identity(geometry(min_y, height));
        let center = ChunkPos::new(-1_875_000, 1_874_999);
        let column = control(identity, center);
        let core = admit_controls(identity, center, &[&column], 100_000).unwrap();
        let (x, z) = (center.x() * 4 + 1, center.z() * 4 + 2);
        let bottom = min_y >> 2;
        let top = bottom + i32::try_from(height >> 2).unwrap() - 1;
        let at = |y| core.resolve(StoredBiomeQuartPos::new(x, y, z)).unwrap();
        // Only Y clamps, to the closed accessor interval at both checked ends.
        assert_eq!(at(i32::MIN), at(bottom));
        assert_eq!(at(i32::MAX), at(top));
        if bottom > i32::MIN {
            assert_eq!(at(bottom - 1), at(bottom));
        }
        if top < i32::MAX {
            assert_eq!(at(top + 1), at(top));
        }
        assert_ne!(at(bottom), at(top));
        assert_eq!(at(top).0, center);
    }
}

#[test]
fn sparse_and_surface_centre_domains_at_signed_chunk_extremes() {
    let identity = control_identity(geometry(0, 16));
    for (value, outside) in [(-1_875_000, -1_875_001), (1_874_999, 1_875_000)] {
        for (center, key) in [
            (ChunkPos::new(value, 0), ChunkPos::new(outside, -1)),
            (ChunkPos::new(0, value), ChunkPos::new(-1, outside)),
        ] {
            // Sparse admission of every in-domain window chunk succeeds.
            let controls: Vec<_> = (0..9)
                .filter_map(|slot| slot_key(center, slot).ok())
                .filter(|key| admit_chunk(*key).is_ok())
                .map(|key| control(identity, key))
                .collect();
            assert_eq!(controls.len(), 6);
            let refs: Vec<_> = controls.iter().collect();
            let core = admit_controls(identity, center, &refs, 1_000).unwrap();
            // Surface refuses the first out-of-domain halo key before any presence.
            let error = complete_halo(&core).unwrap_err();
            assert_eq!(error.kind(), StoredBiomeErrorKind::CoordinateDomain);
            assert_eq!(error.chunk(), Some(key));
        }
    }
    for value in [-1_874_999, 1_874_998] {
        for center in [ChunkPos::new(value, 0), ChunkPos::new(0, value)] {
            let controls = window(identity, center);
            let refs: Vec<_> = controls.iter().collect();
            let core = admit_controls(identity, center, &refs, 1_000).unwrap();
            assert_eq!(complete_halo(&core), Ok(()));
        }
    }
    for value in [-1_875_001, 1_875_000] {
        let center = ChunkPos::new(value, 0);
        let column = control(identity, ChunkPos::ORIGIN);
        let error = admit_controls(identity, center, &[&column], 1_000).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::CoordinateDomain);
        assert_eq!(error.chunk(), Some(center));
    }
}

#[test]
fn quart_refusal_order_outside_then_domain_then_missing() {
    let identity = control_identity(geometry(0, 16));
    let center = ChunkPos::new(-1_875_000, 0);
    let column = control(identity, center);
    let core = admit_controls(identity, center, &[&column], 1_000).unwrap();
    let base = center.x() * 4;
    // Selected chunk -1_875_002 lies outside the window: outside wins.
    let error = core
        .resolve(StoredBiomeQuartPos::new(base - 5, 0, 0))
        .unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::OutsideNeighborhood);
    assert_eq!(error.chunk(), Some(ChunkPos::new(-1_875_002, 0)));
    // Chunk -1_875_001 is in the window but outside the domain.
    let error = core
        .resolve(StoredBiomeQuartPos::new(base - 1, 0, 0))
        .unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::CoordinateDomain);
    assert_eq!(error.chunk(), Some(ChunkPos::new(-1_875_001, 0)));
    // Chunk -1_874_999 is admitted but absent: no fallback biome.
    let error = core
        .resolve(StoredBiomeQuartPos::new(base + 4, 0, 0))
        .unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::MissingChunk);
    assert_eq!(error.chunk(), Some(ChunkPos::new(-1_874_999, 0)));
    // Window boundaries in both signs and axes at i32 extremes of the quart domain.
    let center = ChunkPos::ORIGIN;
    let controls = window(identity, center);
    let refs: Vec<_> = controls.iter().collect();
    let core = admit_controls(identity, center, &refs, 1_000).unwrap();
    for (x, z, inside) in [
        (-4, 0, true),
        (7, 0, true),
        (-5, 0, false),
        (8, 0, false),
        (0, -4, true),
        (0, 7, true),
        (0, -5, false),
        (0, 8, false),
        (i32::MIN, i32::MAX, false),
    ] {
        let result = core.resolve(StoredBiomeQuartPos::new(x, 0, z));
        assert_eq!(result.is_ok(), inside, "{x}/{z}");
        if !inside {
            assert_eq!(
                result.unwrap_err().kind(),
                StoredBiomeErrorKind::OutsideNeighborhood
            );
        }
    }
}

#[test]
fn block_domain_policy_axis_order_and_surface_column() {
    let normal = generated_group("generated/normal/overworld/0/cold");
    let refs = normal.refs();
    let neighborhood =
        StoredBiomeNeighborhood::new(normal.identity, normal.center, &refs, max_budget()).unwrap();
    let surface = SurfaceBiomeResolver::new(
        normal.identity,
        normal.center,
        &refs,
        BiomeZoomSemantics::Vanilla1_21_8,
        max_budget(),
    )
    .unwrap();
    let domain =
        |axis, coordinate| StoredBiomeErrorKind::BlockCoordinateDomain { axis, coordinate };
    for (pos, expected) in [
        (
            BlockPos::new(30_000_001, 0, 0),
            domain(StoredBiomeAxis::X, 30_000_001),
        ),
        (
            BlockPos::new(-30_000_001, 0, 0),
            domain(StoredBiomeAxis::X, -30_000_001),
        ),
        (
            BlockPos::new(0, 0, 30_000_001),
            domain(StoredBiomeAxis::Z, 30_000_001),
        ),
        (
            BlockPos::new(0, 0, -30_000_001),
            domain(StoredBiomeAxis::Z, -30_000_001),
        ),
        (
            BlockPos::new(i32::MIN, 0, i32::MAX),
            domain(StoredBiomeAxis::X, i32::MIN),
        ),
        (
            BlockPos::new(i32::MAX, 5, i32::MIN),
            domain(StoredBiomeAxis::X, i32::MAX),
        ),
    ] {
        for error in [
            neighborhood.noise_biome_at_block(pos).unwrap_err(),
            neighborhood
                .biome_at_block(pos, BiomeZoomSemantics::Paper1_21_8Build60)
                .unwrap_err(),
            surface.biome_at_block(pos).unwrap_err(),
        ] {
            assert_eq!(error.kind(), expected);
            assert_eq!(error.block(), Some(pos));
            assert_eq!(error.chunk(), None);
        }
    }
    // ±30,000,000 is admitted by the policy; then the window decides.
    for pos in [
        BlockPos::new(30_000_000, 0, 0),
        BlockPos::new(-30_000_000, 0, 0),
        BlockPos::new(0, 0, 30_000_000),
        BlockPos::new(0, 0, -30_000_000),
    ] {
        let error = neighborhood.noise_biome_at_block(pos).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::OutsideNeighborhood);
        let error = surface.biome_at_block(pos).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::OutsideSurfaceColumn);
        assert_eq!(error.chunk(), Some(normal.center));
    }
    for pos in [
        BlockPos::new(-1, 64, 0),
        BlockPos::new(16, 64, 0),
        BlockPos::new(0, 64, -1),
        BlockPos::new(0, 64, 16),
    ] {
        let error = surface.biome_at_block(pos).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::OutsideSurfaceColumn);
        assert_eq!(error.block(), Some(pos));
        // The sparse neighborhood still resolves the same block.
        assert!(neighborhood
            .biome_at_block(pos, BiomeZoomSemantics::Vanilla1_21_8)
            .is_ok());
    }
    // Every signed32 Y is admitted for the centre column.
    for y in [
        i32::MIN,
        i32::MIN + 1,
        -65,
        319,
        320,
        i32::MAX - 1,
        i32::MAX,
    ] {
        for pos in [BlockPos::new(0, y, 0), BlockPos::new(15, y, 15)] {
            assert!(surface.biome_at_block(pos).is_ok(), "{y}");
            assert!(neighborhood.noise_biome_at_block(pos).is_ok(), "{y}");
        }
    }
}

#[test]
fn minimum_work_boundaries_seventy_five_seventy_six_seventy_seven() {
    let identity = control_identity(geometry(0, 16));
    let center = ChunkPos::ORIGIN;
    let column = control(identity, center);
    let error = admit_controls(identity, center, &[&column], 75).unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::WorkBudgetExceeded);
    assert_eq!(error.limit(), Some(StoredBiomeLimit::ConstructionWork));
    assert_eq!((error.requested(), error.maximum()), (76, 75));
    for units in [76, 77] {
        let core = admit_controls(identity, center, &[&column], units).unwrap();
        assert_eq!(core.construction_work, 76);
    }
    // The whole requirement is refused before the value scan.
    let fixed = StoredBiomeIdentity::new(
        WorldSeed::new(0),
        settings("minecraft:overworld"),
        BiomeSourceIdentity::Fixed(biome("plains")),
        geometry(0, 16),
    );
    let mut values = vec![biome("plains"); 64];
    values[0] = biome("desert");
    let bad = Control {
        values,
        ..control(fixed, center)
    };
    assert_eq!(
        admit_controls(fixed, center, &[&bad], 75)
            .unwrap_err()
            .kind(),
        StoredBiomeErrorKind::WorkBudgetExceeded
    );
    assert_eq!(
        admit_controls(fixed, center, &[&bad], 76)
            .unwrap_err()
            .kind(),
        StoredBiomeErrorKind::InvalidFixedValue
    );
}

#[test]
fn maximum_work_boundaries_with_real_two_thousand_forty_eight_value_arrays() {
    let large = generated_group("generated/single_biome_surface/overworld/0/accessor--64-512");
    let refs = large.refs();
    let values: usize = refs.iter().map(|a| a.values().len()).sum();
    assert_eq!(values, MAX_STORED_BIOME_BORROWED_VALUES);
    let error = StoredBiomeNeighborhood::new(large.identity, large.center, &refs, budget(18_523))
        .unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::WorkBudgetExceeded);
    assert_eq!((error.requested(), error.maximum()), (18_524, 18_523));
    for units in [18_524, 18_525] {
        let neighborhood =
            StoredBiomeNeighborhood::new(large.identity, large.center, &refs, budget(units))
                .unwrap();
        assert_eq!(neighborhood.construction_work(), 18_524);
        assert_eq!(neighborhood.borrowed_value_count(), 18_432);
        assert_eq!(neighborhood.chunk_count(), 9);
    }
    for semantics in SEMANTICS {
        let error = SurfaceBiomeResolver::new(
            large.identity,
            large.center,
            &refs,
            semantics,
            budget(18_532),
        )
        .unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::WorkBudgetExceeded);
        assert_eq!((error.requested(), error.maximum()), (18_533, 18_532));
        for units in [18_533, 18_534] {
            let surface = SurfaceBiomeResolver::new(
                large.identity,
                large.center,
                &refs,
                semantics,
                budget(units),
            )
            .unwrap();
            assert_eq!(surface.construction_work(), 18_533);
            assert_eq!(surface.neighborhood().construction_work(), 18_524);
        }
    }
    // An incomplete halo costs the full requirement first (scan, then halo check).
    let eight = &refs[..8];
    let error = SurfaceBiomeResolver::new(
        large.identity,
        large.center,
        eight,
        BiomeZoomSemantics::Vanilla1_21_8,
        budget(2 + 80 + 8 * 2048 + 8),
    )
    .unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::WorkBudgetExceeded);
    let error = SurfaceBiomeResolver::new(
        large.identity,
        large.center,
        eight,
        BiomeZoomSemantics::Vanilla1_21_8,
        budget(2 + 80 + 8 * 2048 + 9),
    )
    .unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::MissingChunk);
    assert_eq!(error.chunk(), Some(refs[8].chunk_pos()));
}

#[test]
fn native_ceilings_at_n_minus_one_n_and_n_plus_one() {
    for (limit, maximum) in [
        (
            StoredBiomeLimit::RetainedBytes,
            MAX_STORED_BIOME_RETAINED_BYTES,
        ),
        (
            StoredBiomeLimit::ScratchBytes,
            MAX_STORED_BIOME_SCRATCH_BYTES,
        ),
        (
            StoredBiomeLimit::BorrowedValues,
            MAX_STORED_BIOME_BORROWED_VALUES,
        ),
        (
            StoredBiomeLimit::ValuesPerChunk,
            MAX_STORED_BIOME_VALUES_PER_CHUNK,
        ),
    ] {
        assert_eq!(native_limit(limit, maximum - 1, maximum), Ok(()));
        assert_eq!(native_limit(limit, maximum, maximum), Ok(()));
        let error = native_limit(limit, maximum + 1, maximum).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::NativeLimit);
        assert_eq!(error.limit(), Some(limit));
        assert_eq!((error.requested(), error.maximum()), (maximum + 1, maximum));
    }
    // Retained-ceiling fault injection through the real admission path.
    let identity = control_identity(geometry(0, 16));
    let column = control(identity, ChunkPos::ORIGIN);
    for (retained, admitted) in [(511, true), (512, true), (513, false)] {
        let result = Core::admit(
            identity,
            ChunkPos::ORIGIN,
            &[&column],
            max_budget(),
            retained,
            0,
        );
        assert_eq!(result.is_ok(), admitted);
        if !admitted {
            let error = result.unwrap_err();
            assert_eq!(error.limit(), Some(StoredBiomeLimit::RetainedBytes));
        }
    }
}

#[test]
fn actual_native_layouts_and_scratch_census_fit_their_ceilings() {
    let sizes = [
        ("identity", size_of::<StoredBiomeIdentity>()),
        ("budget", size_of::<StoredBiomeBuildBudget>()),
        ("quart", size_of::<StoredBiomeQuartPos>()),
        ("neighborhood", size_of::<StoredBiomeNeighborhood<'_>>()),
        ("surface", size_of::<SurfaceBiomeResolver<'_>>()),
        ("error", size_of::<StoredBiomeError>()),
        ("error_kind", size_of::<StoredBiomeErrorKind>()),
        (
            "borrowed_reference",
            size_of::<Option<&ChunkBiomeSamples>>(),
        ),
        ("chunk_biome_samples_object", size_of::<ChunkBiomeSamples>()),
        ("construction_scratch", construction_scratch_bytes()),
    ];
    for (name, bytes) in sizes {
        eprintln!("GEN058_LAYOUT\t{name}\t{bytes}");
    }
    assert_eq!(size_of::<StoredBiomeIdentity>(), 24);
    assert!(size_of::<StoredBiomeNeighborhood<'_>>() <= MAX_STORED_BIOME_RETAINED_BYTES);
    assert!(size_of::<SurfaceBiomeResolver<'_>>() <= MAX_STORED_BIOME_RETAINED_BYTES);
    assert!(construction_scratch_bytes() <= MAX_STORED_BIOME_SCRATCH_BYTES);
    // Borrowed arrays are counted separately from retained resolver bytes.
    let borrowed_objects = MAX_STORED_BIOME_CHUNKS * size_of::<ChunkBiomeSamples>();
    let borrowed_values = MAX_STORED_BIOME_BORROWED_VALUES * size_of::<BiomeResourceId>();
    eprintln!("GEN058_LAYOUT\tborrowed_max_objects\t{borrowed_objects}");
    eprintln!("GEN058_LAYOUT\tborrowed_max_value_capacity\t{borrowed_values}");
    assert_eq!(size_of::<BiomeResourceId>(), 1);
    let generated = generated_group("generated/normal/overworld/0/cold");
    let neighborhood = StoredBiomeNeighborhood::new(
        generated.identity,
        generated.center,
        &generated.refs(),
        max_budget(),
    )
    .unwrap();
    assert_eq!(
        neighborhood.native_retained_bytes(),
        size_of::<StoredBiomeNeighborhood<'_>>()
    );
    assert_eq!(neighborhood.borrowed_value_count(), 9 * 1536);
}

#[test]
fn failed_publication_mutates_nothing_and_keeps_prior_results() {
    let normal = generated_group("generated/normal/overworld/0/cold");
    let refs = normal.refs();
    let before: Vec<Vec<BiomeResourceId>> = refs.iter().map(|a| a.values().to_vec()).collect();
    let histories: Vec<_> = refs
        .iter()
        .map(|a| (a.initial_selection(), a.final_selection()))
        .collect();
    let published =
        StoredBiomeNeighborhood::new(normal.identity, normal.center, &refs, max_budget()).unwrap();
    let probe = BlockPos::new(7, 70, 9);
    let answer = published
        .biome_at_block(probe, BiomeZoomSemantics::Vanilla1_21_8)
        .unwrap();
    let allowance = budget(100);
    let mut duplicated = refs.clone();
    duplicated[3] = refs[2];
    let wrong = StoredBiomeIdentity::new(
        WorldSeed::new(1),
        normal.identity.resource(),
        normal.identity.source(),
        normal.identity.geometry(),
    );
    for result in [
        StoredBiomeNeighborhood::new(normal.identity, normal.center, &refs, allowance),
        StoredBiomeNeighborhood::new(normal.identity, normal.center, &duplicated, max_budget()),
        StoredBiomeNeighborhood::new(wrong, normal.center, &refs, max_budget()),
        StoredBiomeNeighborhood::new(normal.identity, ChunkPos::new(5, 5), &refs, max_budget()),
    ] {
        assert!(result.is_err());
    }
    assert_eq!(allowance.units(), 100);
    for ((array, values), history) in refs.iter().zip(&before).zip(&histories) {
        assert_eq!(array.values(), values.as_slice());
        assert_eq!(
            (array.initial_selection(), array.final_selection()),
            *history
        );
    }
    assert_eq!(
        published.biome_at_block(probe, BiomeZoomSemantics::Vanilla1_21_8),
        Ok(answer)
    );
}

#[test]
fn construction_refusal_order_is_deterministic() {
    let identity = control_identity(geometry(0, 16));
    let center = ChunkPos::ORIGIN;
    let outside = control(identity, ChunkPos::new(2, 0));
    let foreign = Control {
        seed: WorldSeed::new(9),
        ..control(identity, ChunkPos::new(1, 0))
    };
    let valid = control(identity, center);
    let bad_center = ChunkPos::new(1_875_000, 0);
    let ten: Vec<_> = std::iter::repeat_n(&outside, 10).collect();
    // Count precedes centre domain; centre precedes entries.
    assert_eq!(
        kind(&admit_controls(identity, bad_center, &ten, 1)),
        StoredBiomeErrorKind::InvalidChunkCount
    );
    assert_eq!(
        kind(&admit_controls(identity, bad_center, &[&outside], 1)),
        StoredBiomeErrorKind::CoordinateDomain
    );
    // Caller order decides between independent defective entries.
    let error = admit_controls(identity, center, &[&valid, &outside, &foreign], 1).unwrap_err();
    assert_eq!(
        (error.kind(), error.input_index()),
        (StoredBiomeErrorKind::OutsideNeighborhood, Some(1))
    );
    let error = admit_controls(identity, center, &[&valid, &foreign, &outside], 1).unwrap_err();
    assert_eq!(
        (error.kind(), error.input_index()),
        (StoredBiomeErrorKind::InconsistentSeed, Some(1))
    );
    // Entry checks precede the whole-work refusal.
    let error = admit_controls(identity, center, &[&valid, &valid], 1).unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::DuplicateChunk);
    assert_eq!(
        kind(&admit_controls(identity, center, &[&valid], 1)),
        StoredBiomeErrorKind::WorkBudgetExceeded
    );
}

#[test]
fn corrupted_private_control_hits_the_defensive_stored_index_check() {
    let identity = control_identity(geometry(0, 16));
    let center = ChunkPos::ORIGIN;
    let column = control(identity, center);
    let mut core = admit_controls(identity, center, &[&column], 1_000).unwrap();
    let corrupted = Control {
        values: column.values[..10].to_vec(),
        ..column.clone()
    };
    core.chunks[4] = Some(&corrupted);
    let quart = StoredBiomeQuartPos::new(3, 3, 3);
    let error = core.resolve(quart).unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::InvalidStoredIndex);
    assert!(error.requested() >= error.maximum());
    assert_eq!(error.maximum(), 10);
    assert_eq!(error.quart(), Some(quart));
    assert_eq!(error.chunk(), Some(center));
}

#[test]
fn zoom_semantics_are_forwarded_exactly_to_the_selector() {
    // A3 substitute: the accepted GEN-05.6 corpus has zero Vanilla/Paper
    // differences in every record kind, so no output can discriminate the
    // modes. The exact selector input is observed instead, for every path.
    let projection: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/biome-zoom-original-1.21.8/projection.json"
    ))
    .unwrap();
    for kind in ["D", "F", "G", "H", "L"] {
        assert_eq!(projection["vanilla_paper_differences"][kind], 0);
    }
    let normal = generated_group("generated/normal/overworld/-1/cold");
    let refs = normal.refs();
    let neighborhood =
        StoredBiomeNeighborhood::new(normal.identity, normal.center, &refs, max_budget()).unwrap();
    let seed = BiomeZoomSeed::from_world_seed(WorldSeed::new(-1));
    let pos = BlockPos::new(3, 64, 12);
    for semantics in SEMANTICS {
        observed_zoom();
        neighborhood.biome_at_block(pos, semantics).unwrap();
        assert_eq!(observed_zoom(), Some(BiomeZoom::new(seed, semantics)));
        let surface = SurfaceBiomeResolver::new(
            normal.identity,
            normal.center,
            &refs,
            semantics,
            max_budget(),
        )
        .unwrap();
        surface.biome_at_block(pos).unwrap();
        assert_eq!(observed_zoom(), Some(BiomeZoom::new(seed, semantics)));
        // Unzoomed lookups never consult the selector.
        neighborhood.noise_biome_at_block(pos).unwrap();
        neighborhood
            .noise_biome_at_quart(StoredBiomeQuartPos::new(0, 16, 0))
            .unwrap();
        assert_eq!(observed_zoom(), None);
    }
}

/// Positive counterpart of the `compile_fail` detached-borrow example.
fn attached(
    samples: &ChunkBiomeSamples,
    identity: StoredBiomeIdentity,
) -> Result<StoredBiomeNeighborhood<'_>, StoredBiomeError> {
    StoredBiomeNeighborhood::new(identity, samples.chunk_pos(), &[samples], max_budget())
}

#[test]
fn borrowed_neighborhood_lives_exactly_as_long_as_its_arrays() {
    let normal = generated_group("generated/normal/overworld/0/cold");
    let origin = normal
        .arrays
        .iter()
        .find(|a| a.chunk_pos() == ChunkPos::ORIGIN)
        .unwrap();
    let neighborhood = attached(origin, normal.identity).unwrap();
    assert_eq!(neighborhood.center(), ChunkPos::ORIGIN);
    assert_eq!(neighborhood.chunk_count(), 1);
    assert_eq!(neighborhood.construction_work(), 2 + 10 + 1536);
    let quart = StoredBiomeQuartPos::from_zoom(
        BiomeZoom::new(
            BiomeZoomSeed::from_world_seed(WorldSeed::new(0)),
            BiomeZoomSemantics::Vanilla1_21_8,
        )
        .select_quart(BlockPos::new(8, 64, 8)),
    );
    assert_eq!(quart.x() >> 2, 0);
    assert!(neighborhood.noise_biome_at_quart(quart).is_ok());
}
