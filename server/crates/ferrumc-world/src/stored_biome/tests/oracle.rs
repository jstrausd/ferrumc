//! Genuine original equality: every projected query, stored array and history.
//!
//! Original rows are compared mechanically. Controlled-hook rows are `FerrumC`
//! policy (`OutsideNeighborhood`, plus the block-domain policy on block
//! methods) and are counted separately, never as original equality.

use super::*;

/// Policy refusal the public block methods apply before any selection.
fn block_policy(pos: BlockPos) -> Option<StoredBiomeErrorKind> {
    let bound = -MAX_STORED_BIOME_BLOCK_COORDINATE..=MAX_STORED_BIOME_BLOCK_COORDINATE;
    if !bound.contains(&pos.x()) {
        Some(StoredBiomeErrorKind::BlockCoordinateDomain {
            axis: StoredBiomeAxis::X,
            coordinate: pos.x(),
        })
    } else if !bound.contains(&pos.z()) {
        Some(StoredBiomeErrorKind::BlockCoordinateDomain {
            axis: StoredBiomeAxis::Z,
            coordinate: pos.z(),
        })
    } else {
        None
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Census {
    original_values: usize,
    policy_outside: usize,
    policy_block_domain: usize,
}

/// Checks one row through the shared kernel composition.
fn check_row<C: StoredColumn>(
    core: &Core<'_, C>,
    query: &Query,
    seed: &Seed,
    semantics: BiomeZoomSemantics,
    census: &mut Census,
) {
    let fixture_label = || {
        format!(
            "{:?} {:?} {semantics:?}",
            fixture().groups[query.group].id,
            query.input
        )
    };
    let pos = query.block();
    let delegated = match query.kind {
        b'G' => {
            let zoom = BiomeZoom::new(seed.zoom_seed(), semantics);
            assert_eq!(zoom.seed().bits(), seed.zoom_bits, "{}", fixture_label());
            StoredBiomeQuartPos::from_zoom(zoom.select_quart(pos))
        }
        b'B' => block_quart(pos),
        b'Q' => StoredBiomeQuartPos::new(query.input[0], query.input[1], query.input[2]),
        other => panic!("unsupported kind {other}"),
    };
    // Genuine original delegated quart, for value and hook rows alike.
    assert_eq!(delegated, query.quart, "{}", fixture_label());
    let selected = core.resolve(delegated);
    let public = match query.kind {
        b'G' => admit_block(pos)
            .and_then(|()| core.zoomed(BiomeZoom::new(seed.zoom_seed(), semantics), pos)),
        b'B' => core.noise_biome_at_block(pos),
        _ => core.noise_biome(delegated),
    };
    if let Some(expected) = query.value {
        assert_eq!(selected, Ok((query.chunk, expected)), "{}", fixture_label());
        assert_eq!(public, Ok(expected), "{}", fixture_label());
        census.original_values += 1;
    } else {
        // Policy row: the original observed its empty hook outside the window.
        let error = selected.unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::OutsideNeighborhood);
        assert_eq!(error.chunk(), Some(query.chunk), "{}", fixture_label());
        assert_eq!(error.quart(), Some(delegated));
        let public = public.unwrap_err();
        let policy = (query.kind != b'Q').then(|| block_policy(pos)).flatten();
        if let Some(kind) = policy {
            assert_eq!(public.kind(), kind, "{}", fixture_label());
            assert_eq!(public.block(), Some(pos));
            assert_eq!(public.chunk(), None);
            census.policy_block_domain += 1;
        } else {
            assert_eq!(public.kind(), StoredBiomeErrorKind::OutsideNeighborhood);
            assert_eq!(public.chunk(), Some(query.chunk));
            census.policy_outside += 1;
        }
    }
}

/// Synthetic axis controls in the original listing order, which the generator
/// proved is the lexicographic (quart Y, X, Z) order of the original's own quarts.
fn axis_controls(group: usize) -> (StoredBiomeIdentity, Vec<Control>) {
    let fixture = fixture();
    let arrays: Vec<_> = fixture.arrays.iter().filter(|a| a.group == group).collect();
    assert_eq!(arrays.len(), 9);
    let identity = control_identity(arrays[0].geometry);
    let controls = arrays
        .iter()
        .map(|a| Control {
            values: a.values.clone(),
            ..control(identity, a.chunk)
        })
        .collect();
    (identity, controls)
}

#[test]
fn every_axis_original_getter_query_and_clamp_matches_through_the_shared_kernel() {
    let fixture = fixture();
    let mut census = Census::default();
    let mut groups = 0;
    for (index, group) in fixture.groups.iter().enumerate() {
        if !group.axis {
            continue;
        }
        groups += 1;
        let (identity, controls) = axis_controls(index);
        let refs: Vec<_> = controls.iter().collect();
        let core = Core::admit(identity, group.center, &refs, max_budget(), 0, 0).unwrap();
        for query in fixture.queries.iter().filter(|q| q.group == index) {
            for semantics in SEMANTICS {
                check_row(
                    &core,
                    query,
                    &fixture.seeds[query.seed],
                    semantics,
                    &mut census,
                );
            }
        }
    }
    assert_eq!(groups, 7);
    // Both semantics evaluate every row: 131392 original values and 5568 policy rows.
    assert_eq!(
        census,
        Census {
            original_values: 2 * 131_392,
            policy_outside: 2 * 4560,
            policy_block_domain: 2 * 1008,
        }
    );
}

#[test]
fn every_actual_create_biomes_array_and_retained_history_matches_original() {
    let fixture = fixture();
    let mut arrays = 0;
    let mut values = 0;
    for generated in generated() {
        let rows: Vec<_> = fixture
            .histories
            .iter()
            .filter(|h| h.group == generated.group)
            .collect();
        for (array, row) in generated.arrays.iter().zip(&rows) {
            let original = fixture
                .arrays
                .iter()
                .find(|a| a.group == generated.group && a.chunk == row.chunk)
                .unwrap();
            assert_eq!(array.chunk_pos(), row.chunk);
            assert_eq!(array.geometry(), original.geometry);
            assert_eq!(array.seed(), generated.identity.seed());
            assert_eq!(array.resource(), row.settings);
            assert_eq!(array.source(), generated.identity.source());
            // Whole actual GEN-05.5 storage equals the original listing.
            assert_eq!(array.values(), original.values.as_slice());
            let point = |selection: Option<BiomeSelection>| {
                selection.map(|s| (s.parameter_ordinal(), s.biome()))
            };
            assert_eq!(point(array.initial_selection()), row.initial);
            assert_eq!(point(array.final_selection()), row.last);
            arrays += 1;
            values += array.values().len();
        }
    }
    assert_eq!(arrays, 180);
    assert_eq!(values, 254_592);
    let axis: usize = fixture
        .arrays
        .iter()
        .filter(|a| fixture.groups[a.group].axis)
        .map(|a| a.values.len())
        .sum();
    assert_eq!(values + axis, 300_096);
}

fn check_generated(
    neighborhood: &StoredBiomeNeighborhood<'_>,
    surfaces: &[SurfaceBiomeResolver<'_>; 2],
    generated: &Generated,
    census: &mut usize,
) {
    let fixture = fixture();
    for query in fixture
        .queries
        .iter()
        .filter(|q| q.group == generated.group)
    {
        let seed = &fixture.seeds[query.seed];
        assert!(seed.world && seed.value == generated.identity.seed().value());
        let expected = query.value.unwrap();
        match query.kind {
            b'G' => {
                let pos = query.block();
                for (semantics, surface) in SEMANTICS.into_iter().zip(surfaces) {
                    let zoom = BiomeZoom::new(seed.zoom_seed(), semantics);
                    assert_eq!(zoom.seed().bits(), seed.zoom_bits);
                    let delegated = StoredBiomeQuartPos::from_zoom(zoom.select_quart(pos));
                    assert_eq!(delegated, query.quart);
                    assert_eq!(
                        neighborhood.core.resolve(delegated),
                        Ok((query.chunk, expected))
                    );
                    assert_eq!(neighborhood.biome_at_block(pos, semantics), Ok(expected));
                    assert_eq!(observed_zoom(), Some(zoom));
                    assert_eq!(surface.biome_at_block(pos), Ok(expected));
                    assert_eq!(observed_zoom(), Some(zoom));
                    *census += 1;
                }
            }
            b'Q' => {
                assert_eq!(neighborhood.noise_biome_at_quart(query.quart), Ok(expected));
                assert_eq!(
                    neighborhood.core.resolve(query.quart),
                    Ok((query.chunk, expected))
                );
                for surface in surfaces {
                    assert_eq!(
                        surface.neighborhood().noise_biome_at_quart(query.quart),
                        Ok(expected)
                    );
                }
                *census += 1;
            }
            other => panic!("unexpected generated kind {other}"),
        }
    }
}

#[test]
fn every_genuine_neighborhood_query_matches_with_reordered_borrowing_and_both_modes() {
    let mut census = 0;
    for generated in generated() {
        let mut orders = vec![generated.refs()];
        let mut reversed = generated.refs();
        reversed.reverse();
        orders.push(reversed);
        let mut rotated = generated.refs();
        rotated.rotate_left(4);
        orders.push(rotated);
        for refs in &orders {
            let neighborhood = StoredBiomeNeighborhood::new(
                generated.identity,
                generated.center,
                refs,
                max_budget(),
            )
            .unwrap();
            let surfaces = SEMANTICS.map(|semantics| {
                SurfaceBiomeResolver::new(
                    generated.identity,
                    generated.center,
                    refs,
                    semantics,
                    max_budget(),
                )
                .unwrap()
            });
            for (semantics, surface) in SEMANTICS.into_iter().zip(&surfaces) {
                assert_eq!(surface.semantics(), semantics);
                assert_eq!(surface.center(), generated.center);
                assert_eq!(surface.identity(), generated.identity);
            }
            assert_eq!(neighborhood.chunk_count(), 9);
            assert_eq!(neighborhood.identity(), generated.identity);
            let copy = neighborhood;
            check_generated(&copy, &surfaces, generated, &mut census);
        }
    }
    // 20 groups × 3 borrow orders × (3840 G × 2 modes + 360 Q).
    assert_eq!(census, 20 * 3 * (3840 * 2 + 360));
}

#[test]
fn shared_histories_continue_and_cold_histories_start_cold() {
    let fixture = fixture();
    let mut shared = 0;
    for generated in generated() {
        let rows: Vec<_> = fixture
            .histories
            .iter()
            .filter(|h| h.group == generated.group)
            .collect();
        for (i, array) in generated.arrays.iter().enumerate() {
            match (rows[i].history.as_str(), generated.identity.source()) {
                (_, BiomeSourceIdentity::Fixed(_) | BiomeSourceIdentity::End) => {
                    assert!(
                        array.initial_selection().is_none() && array.final_selection().is_none()
                    );
                }
                ("cold", _) => assert!(array.initial_selection().is_none()),
                (_, _) if i > 0 => {
                    assert_eq!(
                        array.initial_selection(),
                        generated.arrays[i - 1].final_selection()
                    );
                    shared += 1;
                }
                _ => assert!(array.initial_selection().is_none()),
            }
        }
    }
    // Four forward/reverse groups with eight continuing arrays each.
    assert_eq!(shared, 32);
}

#[test]
fn excluded_original_exception_geometries_are_not_admitted_accessors() {
    let fixture = fixture();
    assert_eq!(fixture.evidence.len(), 32);
    let mut accessors = 0;
    for id in &fixture.evidence {
        let Some(rest) = id.strip_prefix("accessor/") else {
            assert!(id == "before_biomes" || id == "actual_null_section");
            continue;
        };
        let parts: Vec<i64> = rest.split('/').map(|p| p.parse().unwrap()).collect();
        let geometry = BiomeQuartGeometry::new(
            i32::try_from(parts[0]).unwrap(),
            u32::try_from(parts[1]).unwrap(),
        );
        assert!(geometry.is_err(), "{id}");
        accessors += 1;
    }
    assert_eq!(accessors, 30);
}

#[test]
fn fixture_manifest_records_provenance_and_mode_equivalence() {
    let manifest = &fixture().manifest;
    assert!(manifest["provenance"]
        .as_str()
        .unwrap()
        .contains("-Dmax.bg.threads=1"));
    assert_eq!(
        manifest["freeze_manifest_sha256"],
        "d838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee"
    );
    let census = &manifest["counts"]["query_census"];
    let hooks: u64 = [
        "axis/B/controlled_null_hook",
        "axis/G/controlled_null_hook",
        "axis/Q/controlled_null_hook",
    ]
    .iter()
    .map(|k| census[*k].as_u64().unwrap())
    .sum();
    assert_eq!(hooks, 5568);
}
