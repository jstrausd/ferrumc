use super::*;

#[test]
fn original_cache2d_requests_sentinel_y_hit_and_poison_timing() {
    let rows = rows();
    let mut s = fresh(true);
    let input = leaf(&mut s, "cache2d-child", &[13.0], (-20.0, 20.0));
    let root = marker(&mut s, Marker::Cache2D, input);
    for p in [
        [1_875_066, 0, 1_875_066],
        [1_875_066, 79, 1_875_066],
        [1, 0, 2],
        [1, 100, 2],
        [2, -3, 1],
    ] {
        scalar(
            &mut s,
            root,
            find(&rows, "wrapper", &format!("cache2d|{p:?}")),
            BlockPos::new(p[0], p[1], p[2]),
            false,
        );
    }
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "cache2d-fill-bypass"),
        vec![0.0; 3],
        Provider::Foreign(3),
    );
    let input = leaf(&mut s, "poison2d-child", &[17.0], (-20.0, 20.0));
    controls(&mut s, input, Some(0), false);
    let root = marker(&mut s, Marker::Cache2D, input);
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "cache2d-child-error"),
        BlockPos::new(73, 9, -29),
        false,
    );
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "cache2d-poisoned-key-hit"),
        BlockPos::new(73, 79, -29),
        false,
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One successive original CacheOnce stream preserves cross-call cache lengths, failure timing and wrapped scalar/array epochs."
)]
fn original_cacheonce_scalar_array_length_epochs_nested_and_failures() {
    let rows = rows();
    let mut s = fresh(true);
    let input = leaf(
        &mut s,
        "once-child",
        &[11.0, 22.0, 33.0, 44.0],
        (-100.0, 100.0),
    );
    let root = marker(&mut s, Marker::CacheOnce, input);
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "once-initial-counter-zero"),
        BlockPos::new(0, 0, 0),
        true,
    );
    s.state.scalar_counter = 1;
    for case in ["once-scalar-miss", "once-scalar-hit"] {
        scalar(
            &mut s,
            root,
            find(&rows, "wrapper", case),
            BlockPos::new(0, 0, 0),
            true,
        );
    }
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "once-array-miss"),
        vec![0.0; 4],
        Provider::Foreign(4),
    );
    s.state.array_index = 2;
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "once-array-before-scalar"),
        BlockPos::new(0, 0, 0),
        true,
    );
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "once-array-prefix"),
        vec![0.0; 2],
        Provider::Foreign(2),
    );
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "once-array-longer-same-epoch"),
        vec![0.0; 5],
        Provider::Foreign(5),
    );
    s.state.array_counter = 1;
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "once-changed-logical-length"),
        vec![0.0; 2],
        Provider::Foreign(2),
    );
    s.state.array_index = 1;
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "once-new-array-last-index"),
        BlockPos::new(0, 0, 0),
        true,
    );
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "once-foreign-context"),
        BlockPos::new(3, 7, 11),
        false,
    );
    s.state.array_counter = 2;
    controls(&mut s, input, None, true);
    let row = find(&rows, "wrapper", "once-array-failure");
    let mut values = vec![-99.0, -98.0, -97.0];
    assert!(s
        .state
        .fill(
            &s.data,
            &s.view,
            root,
            &mut values,
            Provider::Foreign(3),
            &mut s.work
        )
        .is_err());
    let id = s.state.wrapper_nodes[root].unwrap();
    let Cache::Once {
        array: cached_array,
        array_counter,
        valid_len,
        ..
    } = &s.state.wrappers[id].cache
    else {
        panic!()
    };
    let event = json!({"kind":"failure-prefix","case":"array","bits":values.iter().map(|v|format!("{:016x}",v.to_bits())).collect::<Vec<_>>(),"last_epoch":array_counter,"last_array":cached_array[..valid_len.unwrap()].iter().map(|v|format!("{:016x}",v.to_bits())).collect::<Vec<_>>()});
    s.state.requests.as_mut().unwrap().record(event);
    assert_eq!(take_trace(&mut s), *row["trace"].as_array().unwrap());
    check_state(&s, row);
    controls(&mut s, input, None, false);
    s.state.array_counter = i64::MAX;
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "once-near-wrap-array"),
        vec![0.0; 3],
        Provider::Foreign(3),
    );
    s.state.array_counter = i64::MIN;
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "once-wrapped-array-counter"),
        vec![0.0; 1],
        Provider::Foreign(1),
    );
    let two = marker(&mut s, Marker::Cache2D, input);
    let nested = marker(&mut s, Marker::CacheOnce, two);
    s.state.scalar_counter = i64::MAX;
    scalar(
        &mut s,
        nested,
        find(&rows, "wrapper", "once2d-nested-scalar-max"),
        BlockPos::new(0, 0, 0),
        true,
    );
    s.state.scalar_counter = i64::MIN;
    scalar(
        &mut s,
        nested,
        find(&rows, "wrapper", "once2d-nested-scalar-min"),
        BlockPos::new(0, 0, 0),
        true,
    );
}

#[test]
fn original_flat_cache_constructor_grid_foreign_fallback_and_nested_once() {
    let rows = rows();
    let mut s = fresh(true);
    let input = leaf(
        &mut s,
        "flat-child",
        &[7.0, 11.0, 13.0, 17.0, 19.0],
        (-100.0, 100.0),
    );
    let root = marker(&mut s, Marker::FlatCache, input);
    let row = find(&rows, "wrapper", "flat-constructor-inclusive-grid");
    assert_eq!(take_trace(&mut s), *row["trace"].as_array().unwrap());
    let id = s.state.wrapper_nodes[root].unwrap();
    let Cache::Flat(v) = &s.state.wrappers[id].cache else {
        panic!()
    };
    for (v, e) in v.chunks(5).zip(row["result"].as_array().unwrap()) {
        compare_bits(v, e, "flat-constructor");
    }
    check_state(&s, row);
    for p in [
        [0, 79, 0],
        [16, -63, 16],
        [19, 12, 19],
        [20, 12, 0],
        [-1, 12, 0],
        [0, -50, -1],
    ] {
        scalar(
            &mut s,
            root,
            find(&rows, "wrapper", &format!("flat|{p:?}")),
            BlockPos::new(p[0], p[1], p[2]),
            false,
        );
    }
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "flat-fill-provider-direct"),
        vec![0.0; 3],
        Provider::Foreign(3),
    );
    let once = marker(&mut s, Marker::CacheOnce, root);
    scalar(
        &mut s,
        once,
        find(&rows, "wrapper", "flat-once-foreign"),
        BlockPos::new(20, 13, -3),
        false,
    );
}

#[test]
fn original_cell_wrapper_owned_foreign_lifecycle_and_fill_requests() {
    let rows = rows();
    let mut s = fresh(true);
    let input = leaf(&mut s, "cell-child", &[37.0], (-5000.0, 5000.0));
    let root = marker(&mut s, Marker::CacheAllInCell, input);
    let id = s.state.wrapper_nodes[root].unwrap();
    if let Cache::Cell(values) = &mut s.state.wrappers[id].cache {
        for (i, v) in values.iter_mut().enumerate() {
            *v = 1000.0 + i as f64;
        }
    }
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "cell-before-interpolation"),
        BlockPos::new(0, 0, 0),
        true,
    );
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "cell-foreign"),
        BlockPos::new(1, 2, 3),
        false,
    );
    s.state.interpolating = true;
    for p in [
        [0, 0, 0],
        [3, 7, 3],
        [1, 3, 2],
        [-1, 0, 0],
        [4, 0, 0],
        [0, 8, 0],
    ] {
        s.state.in_x = p[0];
        s.state.in_y = p[1];
        s.state.in_z = p[2];
        let pos = s.state.pos();
        scalar(
            &mut s,
            root,
            find(&rows, "wrapper", &format!("cell|{p:?}")),
            pos,
            true,
        );
    }
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "cell-fill-foreign-provider"),
        vec![0.0; 3],
        Provider::Foreign(3),
    );
}

#[test]
fn original_interpolator_requests_filling_cell_foreign_and_normal_fill() {
    let rows = rows();
    let mut s = fresh(true);
    let input = leaf(&mut s, "interpolator-child", &[23.0], (-1e30, 1e30));
    let root = marker(&mut s, Marker::Interpolated, input);
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "interpolator-own-before-init"),
        BlockPos::new(0, 0, 0),
        true,
    );
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "interpolator-foreign-fallback"),
        BlockPos::new(2, 3, 1),
        false,
    );
    let id = s.state.wrapper_nodes[root].unwrap();
    if let Cache::Interpolated(v) = &mut s.state.wrappers[id].cache {
        v.corners = [
            1e16,
            1.0,
            -1e16,
            3.25,
            f64::from_bits(1.0_f64.to_bits() + 1),
            -1e-20,
            17.0,
            -37.0,
        ];
    }
    s.state.interpolating = true;
    s.state.in_x = 1;
    s.state.in_y = 3;
    s.state.in_z = 2;
    s.state.filling_cell = true;
    let pos = s.state.pos();
    scalar(
        &mut s,
        root,
        find(&rows, "wrapper", "interpolator-fillingCell-actual-lerp3"),
        pos,
        true,
    );
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "interpolator-fillingCell-direct-fill"),
        vec![0.0; 3],
        Provider::Foreign(3),
    );
    s.state.filling_cell = false;
    s.state.lifecycle = Lifecycle::Running;
    s.state.driver_phase = crate::density::chunk::state::DriverPhase::CellSelected;
    for fraction in [
        f64::from_bits(0x8000_0000_0000_0001),
        0.0,
        f64::from_bits(1),
        0.25,
        f64::from_bits(1.0_f64.to_bits() - 1),
        1.0,
        f64::from_bits(1.0_f64.to_bits() + 1),
    ] {
        s.update_y(0.375).unwrap();
        s.update_x(fraction).unwrap();
        s.update_z(0.5).unwrap();
        let pos = s.state.pos();
        scalar(
            &mut s,
            root,
            find(
                &rows,
                "wrapper",
                &format!("interpolator-normal-update|{:016x}", fraction.to_bits()),
            ),
            pos,
            true,
        );
    }
    array(
        &mut s,
        root,
        find(&rows, "wrapper", "interpolator-normal-fill-child"),
        vec![0.0; 3],
        Provider::Foreign(3),
    );
}

#[test]
fn original_cacheonce_scalar_failure_key_is_updated_before_child() {
    let rows = rows();
    let mut s = fresh(true);
    let input = leaf(&mut s, "once-poison", &[17.0], (-20.0, 20.0));
    controls(&mut s, input, Some(0), false);
    let root = marker(&mut s, Marker::CacheOnce, input);
    s.state.scalar_counter = 1;
    for case in ["once-scalar-failure", "once-poisoned-scalar-hit"] {
        scalar(
            &mut s,
            root,
            find(&rows, "wrapper", case),
            BlockPos::new(0, 0, 0),
            true,
        );
    }
}
