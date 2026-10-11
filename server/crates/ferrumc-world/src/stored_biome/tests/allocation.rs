//! External allocator measurement of build, copy and every corpus query.
//!
//! Run only through `tests/reference/stored-biome/measure_allocations.py`,
//! which preloads the accepted GEN-05.7 test-only probe in a fresh process.
//! All fixture parsing, GEN-05.5 production and result storage happen before
//! the measured regions; the first resolver construction is measured.

use std::hint::black_box;
use std::io::{self, Write};

use super::*;

fn boundary(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes).unwrap();
    stdout.flush().unwrap();
}

fn same(actual: Result<BiomeResourceId, StoredBiomeError>, expected: Option<BiomeResourceId>) {
    match (actual, expected) {
        (Ok(actual), Some(expected)) if actual == expected => {}
        (Err(error), None)
            if matches!(
                error.kind(),
                StoredBiomeErrorKind::OutsideNeighborhood
                    | StoredBiomeErrorKind::BlockCoordinateDomain { .. }
            ) => {}
        _ => panic!("stored biome measurement mismatch"),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
#[ignore = "external Linux allocator probe; run measure_allocations.py in Debug and Release"]
fn measured_build_clone_and_every_original_query() {
    let fixture = fixture();
    let generated = generated();
    let refs: Vec<Vec<&ChunkBiomeSamples>> = generated.iter().map(Generated::refs).collect();
    let mut axis = Vec::new();
    for (index, group) in fixture.groups.iter().enumerate() {
        if group.axis {
            let arrays: Vec<_> = fixture.arrays.iter().filter(|a| a.group == index).collect();
            let identity = control_identity(arrays[0].geometry);
            let controls: Vec<_> = arrays
                .iter()
                .map(|a| Control {
                    values: a.values.clone(),
                    ..control(identity, a.chunk)
                })
                .collect();
            axis.push((index, identity, group.center, controls));
        }
    }
    let axis_refs: Vec<Vec<&Control>> = axis.iter().map(|(.., c)| c.iter().collect()).collect();
    let mut by_group: Vec<Vec<Query>> = fixture.groups.iter().map(|_| Vec::new()).collect();
    for query in &fixture.queries {
        by_group[query.group].push(*query);
    }
    let zooms: Vec<[BiomeZoom; 2]> = fixture
        .seeds
        .iter()
        .map(|seed| SEMANTICS.map(|semantics| BiomeZoom::new(seed.zoom_seed(), semantics)))
        .collect();
    let mut neighborhoods = Vec::with_capacity(generated.len());
    let mut surfaces = Vec::with_capacity(generated.len());
    let mut cores = Vec::with_capacity(axis.len());
    let (mut built, mut refused) = (0_usize, 0_usize);

    boundary(b"GEN057_BEGIN_CONTROL\n");
    let mut control_allocation = Vec::<u8>::with_capacity(4096);
    control_allocation.push(1);
    black_box(&control_allocation);
    drop(control_allocation);
    boundary(b"GEN057_END_CONTROL\n");

    boundary(b"GEN057_BEGIN_BUILD\n");
    // This fresh process's first resolver construction happens here.
    for (generated, refs) in generated.iter().zip(&refs) {
        let identity = black_box(generated.identity);
        neighborhoods.push(
            StoredBiomeNeighborhood::new(identity, generated.center, refs, max_budget()).unwrap(),
        );
        surfaces.push(SEMANTICS.map(|semantics| {
            SurfaceBiomeResolver::new(identity, generated.center, refs, semantics, max_budget())
                .unwrap()
        }));
        built += 3;
        black_box(
            StoredBiomeNeighborhood::new(identity, generated.center, refs, budget(100))
                .unwrap_err(),
        );
        black_box(
            SurfaceBiomeResolver::new(
                identity,
                generated.center,
                &refs[..8],
                BiomeZoomSemantics::Paper1_21_8Build60,
                max_budget(),
            )
            .unwrap_err(),
        );
        black_box(
            StoredBiomeNeighborhood::new(
                identity,
                generated.center,
                &[refs[0], refs[0]],
                max_budget(),
            )
            .unwrap_err(),
        );
        refused += 3;
    }
    for ((_, identity, center, _), refs) in axis.iter().zip(&axis_refs) {
        cores.push(Core::admit(*identity, *center, refs, max_budget(), 0, 0).unwrap());
        built += 1;
    }
    boundary(b"GEN057_END_BUILD\n");

    boundary(b"GEN057_BEGIN_CLONE\n");
    for _ in 0..100 {
        for (neighborhood, pair) in neighborhoods.iter().zip(&surfaces) {
            black_box(*black_box(neighborhood));
            black_box(*black_box(pair));
        }
        for core in &cores {
            black_box(*black_box(core));
        }
    }
    boundary(b"GEN057_END_CLONE\n");

    let mut queries = 0_usize;
    boundary(b"GEN057_BEGIN_QUERY\n");
    for ((generated, neighborhood), pair) in generated.iter().zip(&neighborhoods).zip(&surfaces) {
        for query in &by_group[generated.group] {
            let pos = query.block();
            if query.kind == b'G' {
                for (semantics, surface) in SEMANTICS.into_iter().zip(pair) {
                    same(
                        black_box(neighborhood).biome_at_block(pos, semantics),
                        query.value,
                    );
                    same(black_box(surface).biome_at_block(pos), query.value);
                    queries += 2;
                }
            } else {
                same(neighborhood.noise_biome_at_quart(query.quart), query.value);
                queries += 1;
            }
        }
    }
    for ((index, ..), core) in axis.iter().zip(&cores) {
        for query in &by_group[*index] {
            let pos = query.block();
            match query.kind {
                b'G' => {
                    for zoom in zooms[query.seed] {
                        same(
                            admit_block(pos).and_then(|()| black_box(core).zoomed(zoom, pos)),
                            query.value,
                        );
                        queries += 1;
                    }
                }
                b'B' => {
                    same(core.noise_biome_at_block(pos), query.value);
                    queries += 1;
                }
                _ => {
                    same(core.noise_biome(query.quart), query.value);
                    queries += 1;
                }
            }
        }
    }
    boundary(b"GEN057_END_QUERY\n");
    assert_eq!((built, refused), (67, 60));
    assert_eq!(
        queries,
        20 * (3840 * 4 + 360) + 2 * 81_216 + 3328 + 1872 + 48_192 + 2352
    );
    println!("measured {built} admitted + {refused} refused builds, 100 copy rounds, {queries} corpus queries");
}
