use super::work::two_point_spline;
use super::*;

#[test]
fn budgets_and_batch_admission_are_checked_before_sampling_and_publication() {
    for units in [0, MAX_DENSITY_SAMPLE_WORK + 1, usize::MAX] {
        assert!(matches!(
            DensityWorkBudget::new(units).unwrap_err().kind(),
            DensitySampleErrorKind::InvalidBudget { .. }
        ));
    }
    for units in [0, MAX_DENSITY_BATCH_WORK + 1, usize::MAX] {
        assert!(matches!(
            DensityBatchBudget::new(units).unwrap_err().kind(),
            DensitySampleErrorKind::InvalidBudget { .. }
        ));
    }
    assert_eq!(
        DensityWorkBudget::new(MAX_DENSITY_SAMPLE_WORK)
            .unwrap()
            .units(),
        MAX_DENSITY_SAMPLE_WORK
    );
    assert_eq!(
        DensityBatchBudget::new(MAX_DENSITY_BATCH_WORK)
            .unwrap()
            .units(),
        MAX_DENSITY_BATCH_WORK
    );
    let router = probe_router(vec![(original(NodeKind::Constant(3.0)), "value")], 0);
    let work = DensityBatchBudget::new(MAX_DENSITY_BATCH_WORK).unwrap();
    assert!(matches!(
        router.sample_batch(&[], work).unwrap_err().kind(),
        DensitySampleErrorKind::InvalidBatchSize { count: 0, .. }
    ));
    let request = DensityRequest::field(RouterField::FinalDensity, BlockPos::new(0, 0, 0));
    let requests = vec![request; MAX_DENSITY_BATCH_REQUESTS];
    let result = router
        .sample_batch(
            &requests,
            DensityBatchBudget::new(MAX_DENSITY_BATCH_REQUESTS).unwrap(),
        )
        .unwrap();
    assert_eq!(result.len(), MAX_DENSITY_BATCH_REQUESTS);
    assert!(result
        .iter()
        .all(|&value| value.to_bits() == 3.0_f64.to_bits()));
    assert!(matches!(
        router
            .sample_batch(&vec![request; MAX_DENSITY_BATCH_REQUESTS + 1], work)
            .unwrap_err()
            .kind(),
        DensitySampleErrorKind::InvalidBatchSize { count: 4097, .. }
    ));
    let invalid = DensityRequest::field(RouterField::FinalDensity, BlockPos::new(30_000_001, 0, 0));
    let error = router
        .sample_batch(
            &[request, request, invalid],
            DensityBatchBudget::new(1).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.batch_index(), Some(2));
    assert!(matches!(
        error.kind(),
        DensitySampleErrorKind::PositionOutOfRange {
            axis: "x",
            value: 30_000_001
        }
    ));
    assert_eq!(
        router
            .sample_field(
                RouterField::FinalDensity,
                BlockPos::new(0, 0, 0),
                DensityWorkBudget::new(1).unwrap()
            )
            .unwrap(),
        3.0
    );
    let invalid_result = probe_router(
        vec![(original(NodeKind::Constant(f64::INFINITY)), "first_failure")],
        0,
    );
    let error = invalid_result
        .sample_batch(&[request, request], work)
        .unwrap_err();
    assert_eq!(error.batch_index(), Some(0));
    assert_eq!(error.resource(), "first_failure");
}
#[test]
fn quart_positions_floor_negatives_and_refuse_shift_wrap_without_clamping_y() {
    for (block, origin) in [
        (-1, -4),
        (-4, -4),
        (-5, -8),
        (0, 0),
        (1, 0),
        (4, 4),
        (i32::MIN, i32::MIN),
        (i32::MAX, 2_147_483_644),
    ] {
        let quart = ClimateQuartPos::from_block(BlockPos::new(-1, block, -5)).unwrap();
        assert_eq!(quart.block_origin(), BlockPos::new(-4, origin, -8));
    }
    assert_eq!(
        ClimateQuartPos::new(-7_500_000, -536_870_912, 7_500_000)
            .unwrap()
            .block_origin(),
        BlockPos::new(-30_000_000, i32::MIN, 30_000_000)
    );
    assert_eq!(
        ClimateQuartPos::new(7_500_000, 536_870_911, -7_500_000)
            .unwrap()
            .block_origin()
            .y(),
        2_147_483_644
    );
    for pos in [
        (7_500_001, 0, 0),
        (-7_500_001, 0, 0),
        (0, 536_870_912, 0),
        (0, -536_870_913, 0),
        (0, 0, i32::MAX),
    ] {
        assert!(matches!(
            ClimateQuartPos::new(pos.0, pos.1, pos.2)
                .unwrap_err()
                .kind(),
            DensitySampleErrorKind::PositionOutOfRange { .. }
        ));
    }
    assert!(ClimateQuartPos::from_block(BlockPos::new(30_000_001, 0, 0)).is_err());
    let router = probe_router(
        vec![(
            original(NodeKind::Gradient {
                from_y: i32::MIN,
                to_y: i32::MAX,
                from_value: -1.0,
                to_value: 1.0,
            }),
            "y",
        )],
        0,
    );
    let work = DensityWorkBudget::new(1).unwrap();
    for (y, value) in [(i32::MIN, -1.0), (i32::MAX, 1.0)] {
        assert_eq!(
            router
                .sample_field(
                    RouterField::FinalDensity,
                    BlockPos::new(30_000_000, y, -30_000_000),
                    work
                )
                .unwrap(),
            value
        );
    }
}
#[test]
fn finite_policy_preserves_metadata_and_stops_f32_overflow_without_partials() {
    let mut router = probe_router(
        vec![
            (original(NodeKind::Constant(f64::MAX)), "coordinate"),
            (original(NodeKind::Spline(0)), "spline"),
        ],
        1,
    );
    let data = Arc::get_mut(&mut router.data).unwrap();
    data.router.nodes[1].spline = Some(crate::density::bind::BoundSpline {
        spline: two_point_spline(),
        coordinates: vec![0].into_boxed_slice(),
    });
    data.router.nodes[1].bounds = crate::density::bind::pair(f64::NAN, f64::NEG_INFINITY);
    assert!(router
        .field_bounds(RouterField::FinalDensity)
        .min_value()
        .is_nan());
    assert_eq!(
        router.field_bounds(RouterField::FinalDensity).max_value(),
        f64::NEG_INFINITY
    );
    let (sample, work) = evaluate_probe(&router, 1, 100);
    assert!(
        matches!(sample.unwrap_err().kind(),DensitySampleErrorKind::NonFiniteCoordinate {bits} if *bits==f32::INFINITY.to_bits())
    );
    assert_eq!(work.used(), 4);
    assert_eq!(work.density_trace(), ["spline", "coordinate"]);
    let mut climate_router =
        probe_router(vec![(original(NodeKind::Constant(f64::MAX)), "value")], 0);
    let data = Arc::get_mut(&mut climate_router.data).unwrap();
    let origin = data.router.nodes[0].origin;
    data.climate = BoundView {
        pass: crate::density::bind::VisitorPass::Climate,
        nodes: vec![probe_node(
            original(NodeKind::Constant(f64::MAX)),
            "value",
            origin,
        )],
    };
    let pos = BlockPos::new(-1, 0, 1);
    assert_eq!(
        climate_router
            .sample_field(
                RouterField::FinalDensity,
                pos,
                DensityWorkBudget::new(1).unwrap()
            )
            .unwrap()
            .to_bits(),
        f64::MAX.to_bits()
    );
    let error = climate_router
        .sample_climate_at_block(pos, DensityWorkBudget::new(6).unwrap())
        .unwrap_err();
    assert_eq!(error.field(), "temperature");
    assert_eq!(error.position(), Some(pos));
    assert!(
        matches!(error.kind(),DensitySampleErrorKind::NonFiniteCoordinate {bits} if *bits==f32::INFINITY.to_bits())
    );
    let router = probe_router(
        vec![
            (original(NodeKind::Constant(4.0)), "value"),
            (BoundKind::Alias(0), "removed"),
        ],
        1,
    );
    let (sample, work) = evaluate_probe(&router, 1, 1);
    assert_eq!(sample.unwrap(), 4.0);
    assert_eq!(work.density_trace(), ["value"]);
    let router = probe_router(
        vec![(
            original(NodeKind::Gradient {
                from_y: 1,
                to_y: 1,
                from_value: 0.0,
                to_value: 1.0,
            }),
            "gradient",
        )],
        0,
    );
    assert!(matches!(
        evaluate_probe(&router, 0, 1).0.unwrap_err().kind(),
        DensitySampleErrorKind::NonFiniteDensity { .. }
    ));
}
