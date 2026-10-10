use super::*;
#[test]
fn actual_provider_requests_direct_fill_registration_and_signed_wrap() {
    let rows = rows();
    let mut s = fresh(true);
    let input = leaf(&mut s, "provider-child", &[13.0], (-100.0, 100.0));
    let mut count = 0;
    for index in [0, 1, 48] {
        let row = find(&rows, "provider", &format!("slice-forIndex|{index}"));
        let pos = s
            .state
            .for_index(Provider::Slice, index, &mut s.work)
            .unwrap();
        assert_eq!(context(&s.state, pos, true), row["result"]);
        check_state(&s, row);
        assert_eq!(take_trace(&mut s), [] as [Value; 0]);
        count += 1;
    }
    for (provider, length, case) in [
        (Provider::Slice, 49, "slice-direct-fill"),
        (Provider::Cell, 128, "cell-direct-fill"),
    ] {
        if matches!(provider, Provider::Cell) {
            for index in [0, 1, 15, 16, 127] {
                let row = find(&rows, "provider", &format!("cell-forIndex|{index}"));
                let pos = s.state.for_index(provider, index, &mut s.work).unwrap();
                assert_eq!(context(&s.state, pos, true), row["result"]);
                check_state(&s, row);
                count += 1;
            }
        }
        let row = find(&rows, "provider", case);
        let mut values = vec![0.0; length];
        s.state
            .fill_direct(&s.data, &s.view, input, &mut values, provider, &mut s.work)
            .unwrap();
        compare_bits(&values, &row["array_bits"], case);
        assert_eq!(take_trace(&mut s), *row["trace"].as_array().unwrap());
        check_state(&s, row);
        count += 1;
    }
    let left = leaf(&mut s, "first-cell-registration", &[19.0], (-100.0, 100.0));
    let right = leaf(&mut s, "second-cell-registration", &[23.0], (-100.0, 100.0));
    marker(&mut s, Marker::CacheAllInCell, left);
    marker(&mut s, Marker::CacheAllInCell, right);
    s.state.lifecycle = Lifecycle::Running;
    s.initialize_driver().unwrap();
    s.fill_slice(true, 0).unwrap();
    s.fill_slice(false, 1).unwrap();
    s.state.start = BlockPos::new(0, s.state.start.y(), s.state.start.z());
    s.select_cell(0, 0).unwrap();
    let row = find(&rows, "provider", "actual-cellcache-registration-fill");
    assert_eq!(take_trace(&mut s), *row["trace"].as_array().unwrap());
    check_state(&s, row);
    count += 1;
    s.state.scalar_counter = i64::MAX;
    let pos = s.state.for_index(Provider::Slice, 0, &mut s.work).unwrap();
    let row = find(&rows, "provider", "slice-scalar-counter-wrap");
    assert_eq!(context(&s.state, pos, true), row["result"]);
    check_state(&s, row);
    count += 1;
    assert_eq!(count, 12);
}
