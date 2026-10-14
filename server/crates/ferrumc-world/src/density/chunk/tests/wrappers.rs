use super::*;
use crate::density::bind::{BoundKind, BoundNode, VisitorPass};
use crate::density::chunk::fill::Provider;
use crate::density::graph::{Binary, Marker, NodeKind};

pub(super) fn original(kind: NodeKind) -> BoundKind {
    BoundKind::Original(kind)
}
pub(super) fn probe(kinds: Vec<(BoundKind, &'static str)>, root: usize) -> NoiseChunkSession {
    let r = router("minecraft:overworld", 0);
    let mut work = SampleWork::new(MAX_NOISE_CHUNK_WORK as usize);
    let mut state = ChunkState::new(
        &r.data,
        ChunkPos::ORIGIN,
        r.data.metadata.geometry,
        &mut work,
    )
    .unwrap();
    let origin = r.data.router.nodes[0].origin;
    let nodes = kinds
        .into_iter()
        .map(|(kind, resource)| BoundNode {
            origin,
            resource,
            bounds: match kind {
                BoundKind::Original(NodeKind::Constant(v)) => crate::density::bind::pair(v, v),
                _ => crate::density::bind::pair(f64::NEG_INFINITY, f64::INFINITY),
            },
            kind,
            noise: None,
            blended: None,
            spline: None,
        })
        .collect::<Vec<_>>();
    state.wrapper_nodes = vec![None; nodes.len()];
    state.scratch = vec![vec![0.0; state.cell_volume().max(state.count_y + 1)]; 8];
    work.enable_density_trace(4096);
    NoiseChunkSession {
        data: r.data,
        chunk: ChunkPos::ORIGIN,
        view: BoundView {
            pass: VisitorPass::SeedRouter,
            nodes,
        },
        state,
        fields: [root; 15],
        material: root,
        climate: [root; 6],
        work,
        surface: SurfaceCache::new(),
        biome_phase: Lifecycle::Fresh,
    }
}
pub(super) fn marker(s: &mut NoiseChunkSession, kind: Marker, input: usize) -> usize {
    let id = s.state.add_wrapper(kind, input).unwrap();
    let root = s.view.nodes.len();
    let mut n = s.view.nodes[input].clone();
    n.kind = original(NodeKind::Marker {
        marker: kind,
        input,
    });
    n.resource = "wrapper";
    s.view.nodes.push(n);
    s.state.wrapper_nodes.push(Some(id));
    s.state.wrappers[id].node = root;
    if kind == Marker::FlatCache {
        for x in 0..5 {
            for z in 0..5 {
                let pos = BlockPos::new(x * 4, 0, z * 4);
                let v = s
                    .state
                    .sample(&s.data, &s.view, input, pos, false, &mut s.work)
                    .unwrap();
                charge(&mut s.work).unwrap();
                if let Cache::Flat(a) = &mut s.state.wrappers[id].cache {
                    a[(x * 5 + z) as usize] = v;
                }
            }
        }
    }
    root
}
fn sample(
    s: &mut NoiseChunkSession,
    root: usize,
    pos: BlockPos,
    own: bool,
) -> Result<f64, NoiseChunkError> {
    s.state
        .sample(&s.data, &s.view, root, pos, own, &mut s.work)
        .map_err(Into::into)
}
fn reset_trace(s: &mut NoiseChunkSession) {
    s.work.enable_density_trace(4096);
}
fn wrapper_id(s: &NoiseChunkSession, root: usize) -> usize {
    s.state.wrapper_nodes[root].unwrap()
}

#[test]
fn cache2d_actual_sentinel_y_hit_and_failure_key_timing() {
    let mut s = probe(
        vec![(
            original(NodeKind::Gradient {
                from_y: 0,
                to_y: 10,
                from_value: 0.0,
                to_value: 10.0,
            }),
            "child",
        )],
        0,
    );
    let root = marker(&mut s, Marker::Cache2D, 0);
    assert_eq!(
        sample(&mut s, root, BlockPos::new(1_875_066, 7, 1_875_066), false)
            .unwrap()
            .to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(s.work.density_trace(), ["wrapper"]);
    assert_eq!(
        sample(&mut s, root, BlockPos::new(-2, 7, 3), false).unwrap(),
        7.0
    );
    reset_trace(&mut s);
    assert_eq!(
        sample(&mut s, root, BlockPos::new(-2, 9, 3), true).unwrap(),
        7.0
    );
    assert_eq!(s.work.density_trace(), ["wrapper"]);
    s.view.nodes[0].kind = original(NodeKind::Constant(f64::NAN));
    assert!(sample(&mut s, root, BlockPos::new(99, 0, -99), false).is_err());
    let Cache::TwoD { key, value } = &s.state.wrappers[wrapper_id(&s, root)].cache else {
        panic!()
    };
    assert_eq!(*key, pack_column(99, -99));
    assert_eq!(*value, 7.0);
}

#[test]
fn once_scalar_zero_foreign_epochs_and_counter_wrap() {
    let mut s = probe(vec![(original(NodeKind::Constant(17.0)), "child")], 0);
    let root = marker(&mut s, Marker::CacheOnce, 0);
    let p = BlockPos::new(0, 0, 0);
    assert_eq!(sample(&mut s, root, p, true).unwrap(), 0.0);
    assert_eq!(s.work.density_trace(), ["wrapper"]);
    assert_eq!(sample(&mut s, root, p, false).unwrap(), 17.0);
    s.state.scalar_counter = 1;
    assert_eq!(sample(&mut s, root, p, true).unwrap(), 17.0);
    reset_trace(&mut s);
    assert_eq!(sample(&mut s, root, p, true).unwrap(), 17.0);
    assert_eq!(s.work.density_trace(), ["wrapper"]);
    for epoch in [i64::MAX, i64::MIN, 0] {
        s.state.scalar_counter = epoch;
        sample(&mut s, root, p, true).unwrap();
        let Cache::Once { counter, .. } = &s.state.wrappers[wrapper_id(&s, root)].cache else {
            panic!()
        };
        assert_eq!(*counter, epoch);
    }
    s.state.scalar_counter = 2;
    s.view.nodes[0].kind = original(NodeKind::Constant(f64::INFINITY));
    assert!(sample(&mut s, root, p, true).is_err());
    let Cache::Once { counter, value, .. } = &s.state.wrappers[wrapper_id(&s, root)].cache else {
        panic!()
    };
    assert_eq!(*counter, 2);
    assert_eq!(*value, 17.0);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Single wrapper instance must retain its successive array and failure epochs."
)]
fn once_array_prefix_logical_length_failure_and_own_index_precedence() {
    let mut s = probe(
        vec![(
            original(NodeKind::Gradient {
                from_y: -64,
                to_y: 320,
                from_value: -64.0,
                to_value: 320.0,
            }),
            "child",
        )],
        0,
    );
    let root = marker(&mut s, Marker::CacheOnce, 0);
    let mut cell = vec![0.0; s.state.cell_volume()];
    s.state.start = BlockPos::new(0, 0, 0);
    s.state
        .fill(
            &s.data,
            &s.view,
            root,
            &mut cell,
            Provider::Cell,
            &mut s.work,
        )
        .unwrap();
    let id = wrapper_id(&s, root);
    let Cache::Once {
        valid_len,
        array_counter,
        ..
    } = &s.state.wrappers[id].cache
    else {
        panic!()
    };
    assert_eq!(*valid_len, Some(128));
    assert_eq!(*array_counter, 0);
    s.state.array_index = 3;
    s.state.scalar_counter = 0;
    assert_eq!(
        sample(&mut s, root, BlockPos::new(0, 10, 0), true).unwrap(),
        cell[3]
    );
    assert_eq!(
        sample(&mut s, root, BlockPos::new(0, 10, 0), false).unwrap(),
        10.0
    );
    let mut slice = vec![-99.0; s.state.count_y + 1];
    reset_trace(&mut s);
    s.state
        .fill(
            &s.data,
            &s.view,
            root,
            &mut slice,
            Provider::Slice,
            &mut s.work,
        )
        .unwrap();
    assert_eq!(&slice[..], &cell[..slice.len()]);
    assert_eq!(s.work.density_trace(), &[] as &[&str]);
    s.state.array_counter = 1;
    s.state
        .fill(
            &s.data,
            &s.view,
            root,
            &mut slice,
            Provider::Slice,
            &mut s.work,
        )
        .unwrap();
    let Cache::Once { valid_len, .. } = &s.state.wrappers[id].cache else {
        panic!()
    };
    assert_eq!(*valid_len, Some(49));
    assert!(s
        .state
        .fill(
            &s.data,
            &s.view,
            root,
            &mut cell,
            Provider::Cell,
            &mut s.work
        )
        .is_err());
    s.state.array_counter = 2;
    s.view.nodes[0].kind = original(NodeKind::Constant(f64::NAN));
    assert!(s
        .state
        .fill(
            &s.data,
            &s.view,
            root,
            &mut slice,
            Provider::Slice,
            &mut s.work
        )
        .is_err());
    let Cache::Once {
        valid_len,
        array_counter,
        ..
    } = &s.state.wrappers[id].cache
    else {
        panic!()
    };
    assert_eq!(*valid_len, Some(49));
    assert_eq!(*array_counter, 1);
}

#[test]
fn flat_inclusive_quart_grid_negative_fallback_and_cell_foreign_rules() {
    let mut s = probe(
        vec![(
            original(NodeKind::Gradient {
                from_y: -64,
                to_y: 320,
                from_value: -64.0,
                to_value: 320.0,
            }),
            "child",
        )],
        0,
    );
    let flat = marker(&mut s, Marker::FlatCache, 0);
    reset_trace(&mut s);
    for p in [BlockPos::new(0, 88, 0), BlockPos::new(19, -88, 19)] {
        assert_eq!(sample(&mut s, flat, p, false).unwrap(), 0.0);
    }
    assert_eq!(s.work.density_trace(), ["wrapper", "wrapper"]);
    for p in [
        BlockPos::new(-1, 7, 0),
        BlockPos::new(20, 7, 0),
        BlockPos::new(0, 7, -1),
    ] {
        assert_eq!(sample(&mut s, flat, p, false).unwrap(), 7.0);
    }
    let cell = marker(&mut s, Marker::CacheAllInCell, 0);
    assert_eq!(
        sample(&mut s, cell, BlockPos::new(0, 4, 0), false).unwrap(),
        4.0
    );
    assert!(matches!(
        sample(&mut s, cell, BlockPos::new(0, 4, 0), true)
            .unwrap_err()
            .kind(),
        NoiseChunkErrorKind::Lifecycle { .. }
    ));
    s.state.interpolating = true;
    let id = wrapper_id(&s, cell);
    if let Cache::Cell(a) = &mut s.state.wrappers[id].cache {
        for (i, v) in a.iter_mut().enumerate() {
            *v = i as f64;
        }
    }
    for (x, y, z, index) in [(0, 7, 0, 0), (3, 7, 3, 15), (0, 0, 0, 112), (3, 0, 3, 127)] {
        s.state.in_x = x;
        s.state.in_y = y;
        s.state.in_z = z;
        assert_eq!(
            sample(&mut s, cell, BlockPos::new(x, y, z), true).unwrap(),
            f64::from(index)
        );
    }
    s.state.in_x = 4;
    s.state.in_y = 2;
    s.state.in_z = 0;
    assert_eq!(
        sample(&mut s, cell, BlockPos::new(4, 2, 0), true).unwrap(),
        2.0
    );
}

#[test]
fn cell_direct_array_index_is_incremented_before_compute_slice_is_not() {
    let mut s = probe(vec![(original(NodeKind::Constant(23.0)), "child")], 0);
    let once = marker(&mut s, Marker::CacheOnce, 0);
    let id = wrapper_id(&s, once);
    if let Cache::Once {
        array, valid_len, ..
    } = &mut s.state.wrappers[id].cache
    {
        *valid_len = Some(128);
        for (i, v) in array.iter_mut().enumerate() {
            *v = i as f64;
        }
    }
    // Flat outside its grid delegates to CacheOnce, whose array lookup exposes
    // the actual direct provider's index to the test without a synthetic hook.
    let flat = marker(&mut s, Marker::FlatCache, once);
    s.state.start = BlockPos::new(1000, 0, 1000);
    let mut cell = vec![-99.0; 128];
    assert!(s
        .state
        .fill(
            &s.data,
            &s.view,
            flat,
            &mut cell,
            Provider::Cell,
            &mut s.work
        )
        .is_err());
    assert_eq!(&cell[..127], &(1..128).map(f64::from).collect::<Vec<_>>());
    assert_eq!(cell[127], -99.0);
    assert_eq!(s.state.array_index, 128);
    let mut slice = vec![-99.0; 49];
    s.state
        .fill(
            &s.data,
            &s.view,
            flat,
            &mut slice,
            Provider::Slice,
            &mut s.work,
        )
        .unwrap();
    assert_eq!(slice, (0..49).map(f64::from).collect::<Vec<_>>());
    assert_eq!(s.state.array_index, 48);
}

#[test]
fn array_dispatch_zero_skip_strict_bounds_context_and_scratch_failure() {
    for zero in [0.0, -0.0] {
        let mut s = probe(
            vec![
                (original(NodeKind::Constant(zero)), "left"),
                (original(NodeKind::Constant(f64::INFINITY)), "right"),
                (
                    original(NodeKind::Binary {
                        operation: Binary::Mul,
                        left: 0,
                        right: 1,
                    }),
                    "mul",
                ),
            ],
            2,
        );
        let mut a = vec![1.0; 49];
        s.state
            .fill(&s.data, &s.view, 2, &mut a, Provider::Slice, &mut s.work)
            .unwrap();
        assert!(a.iter().all(|v| v.to_bits() == 0.0_f64.to_bits()));
        assert_eq!(s.work.density_trace(), &[] as &[&str]);
        s.view.nodes[2].kind = BoundKind::MulOrAdd {
            operation: Binary::Mul,
            input: 1,
            argument: zero,
        };
        assert!(s
            .state
            .fill(&s.data, &s.view, 2, &mut a, Provider::Slice, &mut s.work)
            .is_err());
    }
    for (operation, a, bound, skip) in [
        (Binary::Min, -1.1, -1.0, true),
        (Binary::Min, -1.0, -1.0, false),
        (Binary::Max, 1.1, 1.0, true),
        (Binary::Max, 1.0, 1.0, false),
    ] {
        let mut s = probe(
            vec![
                (original(NodeKind::Constant(a)), "left"),
                (original(NodeKind::Constant(7.0)), "right"),
                (
                    original(NodeKind::Binary {
                        operation,
                        left: 0,
                        right: 1,
                    }),
                    "op",
                ),
            ],
            2,
        );
        s.view.nodes[1].bounds = if operation == Binary::Min {
            crate::density::bind::pair(bound, 9.0)
        } else {
            crate::density::bind::pair(-9.0, bound)
        };
        let mut a = vec![0.0; 49];
        s.state
            .fill(&s.data, &s.view, 2, &mut a, Provider::Slice, &mut s.work)
            .unwrap();
        assert_eq!(s.work.density_trace().len(), if skip { 0 } else { 49 });
    }
    let mut s = probe(
        vec![
            (
                original(NodeKind::Gradient {
                    from_y: 0,
                    to_y: 8,
                    from_value: 0.0,
                    to_value: 8.0,
                }),
                "left",
            ),
            (original(NodeKind::Constant(f64::NAN)), "right"),
            (
                original(NodeKind::Binary {
                    operation: Binary::Add,
                    left: 0,
                    right: 1,
                }),
                "add",
            ),
        ],
        2,
    );
    let mut a = vec![-99.0; 49];
    let leases = s.state.scratch.len();
    assert!(s
        .state
        .fill(&s.data, &s.view, 2, &mut a, Provider::Slice, &mut s.work)
        .is_err());
    assert_eq!(s.state.scratch.len(), leases);
    assert_eq!(a[0], 0.0);
}

#[test]
fn provider_counter_wrap_and_shape_refusal_before_mutation() {
    let mut s = probe(vec![(original(NodeKind::Constant(1.0)), "child")], 0);
    s.state.scalar_counter = i64::MAX;
    s.state.for_index(Provider::Slice, 0, &mut s.work).unwrap();
    assert_eq!(s.state.scalar_counter, i64::MIN);
    s.state.for_index(Provider::Cell, 127, &mut s.work).unwrap();
    assert_eq!(s.state.scalar_counter, i64::MIN);
    assert_eq!(s.state.array_index, 127);
    assert_eq!(s.state.pos(), BlockPos::new(3, -64, 3));
    let work = s.work_used();
    let mut a = vec![0.0; 48];
    assert!(s
        .state
        .fill(&s.data, &s.view, 0, &mut a, Provider::Slice, &mut s.work)
        .is_err());
    assert_eq!(s.work_used(), work);
}

#[test]
fn interpolated_cell_group_retains_public_and_wide_budget_boundaries() {
    use crate::sample_work::{CarverCumulativeMeter, ChargeFault, Unsigned192};

    let mut s = probe(vec![(original(NodeKind::Constant(17.0)), "child")], 0);
    let root = marker(&mut s, Marker::Interpolated, 0);
    let id = wrapper_id(&s, root);
    let corners = [0.25, 1.25, 10.25, 11.25, 100.25, 101.25, 110.25, 111.25];
    if let Cache::Interpolated(interp) = &mut s.state.wrappers[id].cache {
        interp.corners = corners;
    } else {
        panic!("wrong probe wrapper");
    }
    s.state.interpolating = true;
    s.state.filling_cell = true;
    s.state.in_x = 2;
    s.state.in_y = 2;
    s.state.in_z = 3;
    let pos = s.state.pos();
    for prefix in [0, 3] {
        for remaining in 0..=10 {
            for wide in [false, true] {
                let limit = prefix + remaining;
                let make = || {
                    if wide {
                        let mut meter =
                            CarverCumulativeMeter::new(Unsigned192::from_u64(limit as u64));
                        meter.test_short_segments();
                        SampleWork::with_carver_meter(meter)
                    } else {
                        SampleWork::new(limit)
                    }
                };
                let mut actual = make();
                let mut original = make();
                for _ in 0..prefix {
                    actual.charge_core().unwrap();
                    original.charge_core().unwrap();
                }
                // One marker entry followed by the original seven consecutive
                // lerp charges. The independent affine corners yield 78.25.
                let expected = (0..8)
                    .try_for_each(|_| original.charge_core())
                    .map(|()| 78.25_f64.to_bits())
                    .map_err(|fault| match fault {
                        ChargeFault::Public { limit, used } => DensitySampleError {
                            resource: if remaining == 0 {
                                "wrapper"
                            } else {
                                "noise_chunk"
                            },
                            field: if remaining == 0 { "noise_chunk" } else { "$" },
                            position: Some(pos),
                            batch_index: None,
                            kind: DensitySampleErrorKind::WorkExceeded { limit, used },
                        }
                        .into(),
                        fault @ ChargeFault::Carver { .. } => NoiseChunkCoreError::Work(fault),
                    });
                let result = s
                    .state
                    .sample(&s.data, &s.view, root, pos, true, &mut actual)
                    .map(f64::to_bits);
                assert_eq!(
                    result, expected,
                    "prefix={prefix} remaining={remaining} wide={wide}"
                );
                if wide {
                    assert_eq!(actual.carver_meter(), original.carver_meter());
                } else {
                    assert_eq!(actual.used(), original.used());
                }
                let Cache::Interpolated(interp) = &s.state.wrappers[id].cache else {
                    panic!()
                };
                assert_eq!(interp.corners, corners);
            }
        }
    }
}
