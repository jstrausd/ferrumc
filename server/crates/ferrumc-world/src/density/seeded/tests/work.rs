use super::*;

#[test]
fn ap2_zero_skips_but_actual_mul_or_add_zero_evaluates_and_errors_stop() {
    for zero in [0.0, -0.0] {
        let router = probe_router(
            vec![
                (original(NodeKind::Constant(zero)), "left"),
                (original(NodeKind::Constant(f64::INFINITY)), "right"),
                (
                    original(NodeKind::Binary {
                        operation: Binary::Mul,
                        left: 0,
                        right: 1,
                    }),
                    "factory",
                ),
            ],
            2,
        );
        let (sample, work) = evaluate_probe(&router, 2, 2);
        assert_eq!(sample.unwrap().to_bits(), 0.0_f64.to_bits());
        assert_eq!(work.density_trace(), ["factory", "left"]);
        let (sample, work) = evaluate_probe(&router, 2, 1);
        assert!(matches!(
            sample.unwrap_err().kind(),
            DensitySampleErrorKind::WorkExceeded { limit: 1, used: 1 }
        ));
        assert_eq!(work.density_trace(), ["factory"]);
        let router = probe_router(
            vec![
                (original(NodeKind::Constant(f64::INFINITY)), "input"),
                (
                    BoundKind::MulOrAdd {
                        operation: Binary::Mul,
                        input: 0,
                        argument: zero,
                    },
                    "factory",
                ),
            ],
            1,
        );
        let (sample, work) = evaluate_probe(&router, 1, 2);
        assert!(
            matches!(sample.unwrap_err().kind(),DensitySampleErrorKind::NonFiniteDensity {bits} if *bits==f64::INFINITY.to_bits())
        );
        assert_eq!(work.density_trace(), ["factory", "input"]);
    }
    let router = probe_router(
        vec![
            (original(NodeKind::Constant(f64::NAN)), "left"),
            (original(NodeKind::Constant(1.0)), "right"),
            (
                original(NodeKind::Binary {
                    operation: Binary::Add,
                    left: 0,
                    right: 1,
                }),
                "factory",
            ),
        ],
        2,
    );
    let (sample, work) = evaluate_probe(&router, 2, 3);
    let error = sample.unwrap_err();
    assert_eq!(error.resource(), "left");
    assert_eq!(work.density_trace(), ["factory", "left"]);
}
#[test]
fn ap2_min_max_only_skip_strictly_beyond_actual_right_diagnostics() {
    for (operation, threshold, before, equal, after) in [
        (
            Binary::Min,
            -1.0,
            -1.000_000_000_000_000_2,
            -1.0,
            -0.999_999_999_999_999_9,
        ),
        (
            Binary::Max,
            1.0,
            0.999_999_999_999_999_9,
            1.0,
            1.000_000_000_000_000_2,
        ),
    ] {
        for value in [before, equal, after] {
            let mut router = probe_router(
                vec![
                    (original(NodeKind::Constant(value)), "left"),
                    (original(NodeKind::Constant(0.0)), "right"),
                    (
                        original(NodeKind::Binary {
                            operation,
                            left: 0,
                            right: 1,
                        }),
                        "factory",
                    ),
                ],
                2,
            );
            Arc::get_mut(&mut router.data).unwrap().router.nodes[1].bounds =
                if operation == Binary::Min {
                    crate::density::bind::pair(threshold, 10.0)
                } else {
                    crate::density::bind::pair(-10.0, threshold)
                };
            let (sample, work) = evaluate_probe(&router, 2, 3);
            sample.unwrap();
            let skipped = if operation == Binary::Min {
                value < threshold
            } else {
                value > threshold
            };
            assert_eq!(
                work.density_trace(),
                if skipped {
                    &["factory", "left"][..]
                } else {
                    &["factory", "left", "right"][..]
                }
            );
        }
    }
}
#[test]
fn range_choice_queries_only_the_selected_child_at_both_boundaries() {
    for (input, inside) in [
        (f64::from_bits(1.0_f64.to_bits() - 1), false),
        (1.0, true),
        (f64::from_bits(2.0_f64.to_bits() - 1), true),
        (2.0, false),
    ] {
        let router = probe_router(
            vec![
                (original(NodeKind::Constant(input)), "input"),
                (original(NodeKind::Constant(7.0)), "inside"),
                (original(NodeKind::Constant(9.0)), "outside"),
                (
                    original(NodeKind::RangeChoice {
                        input: 0,
                        min: 1.0,
                        max: 2.0,
                        inside: 1,
                        outside: 2,
                    }),
                    "factory",
                ),
            ],
            3,
        );
        let (sample, work) = evaluate_probe(&router, 3, 3);
        assert_eq!(sample.unwrap(), if inside { 7.0 } else { 9.0 });
        assert_eq!(
            work.density_trace(),
            if inside {
                &["factory", "input", "inside"][..]
            } else {
                &["factory", "input", "outside"][..]
            }
        );
    }
}
pub(super) fn two_point_spline() -> crate::spline::CubicSpline {
    use crate::spline::*;
    let points = [
        SplinePoint::new(-1.0, SplineNodeId::new(1), 0.0).unwrap(),
        SplinePoint::new(1.0, SplineNodeId::new(2), 0.0).unwrap(),
    ];
    CubicSpline::new(
        &[SplineCoordinateRange::bounded(-2.0, 2.0).unwrap()],
        &[
            SplineNodeSpec::multipoint(SplineCoordinateId::new(0), &points).unwrap(),
            SplineNodeSpec::constant(0.0).unwrap(),
            SplineNodeSpec::constant(1.0).unwrap(),
        ],
        SplineNodeId::new(0),
    )
    .unwrap()
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Keeps the complete original reference or shared-meter acceptance matrix together."
)]
fn one_meter_crosses_density_spline_density_multiple_times_and_climate_and_batch() {
    let mut router = probe_router(
        vec![
            (
                original(NodeKind::Gradient {
                    from_y: 0,
                    to_y: 4,
                    from_value: 0.0,
                    to_value: 1.0,
                }),
                "coordinate",
            ),
            (original(NodeKind::Spline(0)), "inner"),
            (original(NodeKind::Spline(0)), "outer"),
        ],
        2,
    );
    let data = Arc::get_mut(&mut router.data).unwrap();
    data.router.nodes[1].spline = Some(crate::density::bind::BoundSpline {
        spline: two_point_spline(),
        coordinates: vec![0].into_boxed_slice(),
    });
    data.router.nodes[2].spline = Some(crate::density::bind::BoundSpline {
        spline: two_point_spline(),
        coordinates: vec![1].into_boxed_slice(),
    });
    let origin = data.router.nodes[0].origin;
    data.climate = BoundView {
        pass: crate::density::bind::VisitorPass::Climate,
        nodes: vec![
            probe_node(
                original(NodeKind::Gradient {
                    from_y: 0,
                    to_y: 4,
                    from_value: 0.0,
                    to_value: 1.0,
                }),
                "coordinate",
                origin,
            ),
            probe_node(original(NodeKind::Spline(0)), "inner", origin),
            probe_node(original(NodeKind::Spline(0)), "outer", origin),
        ],
    };
    data.climate.nodes[1].spline = Some(crate::density::bind::BoundSpline {
        spline: two_point_spline(),
        coordinates: vec![0].into_boxed_slice(),
    });
    data.climate.nodes[2].spline = Some(crate::density::bind::BoundSpline {
        spline: two_point_spline(),
        coordinates: vec![1].into_boxed_slice(),
    });
    let (sample, work) = evaluate_probe(&router, 2, 100);
    let expected = sample.unwrap();
    assert_eq!(work.used(), 15);
    assert_eq!(work.density_trace(), ["outer", "inner", "coordinate"]);
    let pos = BlockPos::new(0, 1, 0);
    let clone = router.clone();
    assert_eq!(
        router
            .sample_field(
                RouterField::FinalDensity,
                pos,
                DensityWorkBudget::new(15).unwrap()
            )
            .unwrap()
            .to_bits(),
        expected.to_bits()
    );
    let error = router
        .sample_field(
            RouterField::FinalDensity,
            pos,
            DensityWorkBudget::new(14).unwrap(),
        )
        .unwrap_err();
    assert!(matches!(
        error.kind(),
        DensitySampleErrorKind::WorkExceeded {
            limit: 14,
            used: 14
        }
    ));
    assert_eq!(error.position(), Some(pos));
    assert_eq!(error.field(), "final_density");
    let climate = router
        .sample_climate_at_block(pos, DensityWorkBudget::new(90).unwrap())
        .unwrap();
    assert_eq!(climate.values(), [expected as f32; 6]);
    let error = router
        .sample_climate_at_block(pos, DensityWorkBudget::new(89).unwrap())
        .unwrap_err();
    assert_eq!(error.field(), "weirdness");
    assert!(matches!(
        error.kind(),
        DensitySampleErrorKind::WorkExceeded {
            limit: 89,
            used: 89
        }
    ));
    let requests = [DensityRequest::field(RouterField::FinalDensity, pos); 2];
    assert_eq!(
        &*router
            .sample_batch(&requests, DensityBatchBudget::new(30).unwrap())
            .unwrap(),
        [expected; 2]
    );
    let error = router
        .sample_batch(&requests, DensityBatchBudget::new(29).unwrap())
        .unwrap_err();
    assert_eq!(error.batch_index(), Some(1));
    assert!(matches!(
        error.kind(),
        DensitySampleErrorKind::WorkExceeded {
            limit: 29,
            used: 29
        }
    ));
    assert_eq!(
        clone
            .sample_field(
                RouterField::FinalDensity,
                pos,
                DensityWorkBudget::new(15).unwrap()
            )
            .unwrap()
            .to_bits(),
        expected.to_bits()
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Keeps the complete original reference or shared-meter acceptance matrix together."
)]
fn noise_octave_slots_blended_iterations_and_all_625_end_candidates_share_the_meter() {
    use ferrumc_math::Vec3;
    let router =
        RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name("nether").unwrap())
            .unwrap()
            .bind_seed(WorldSeed::new(0))
            .unwrap();
    let offset = router
        .data
        .noises
        .iter()
        .find(|(id, _)| id.resource_location() == "minecraft:offset")
        .unwrap();
    let mut work = SampleWork::new(2);
    assert_eq!(
        offset
            .1
            .sample_consumer(
                Vec3::new(45_000_000_000.0, 0.0, -45_000_000_000.0),
                &mut work
            )
            .unwrap()
            .to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(work.used(), 2);
    let error = offset
        .1
        .sample_consumer(Vec3::new(0.0, 0.0, 0.0), &mut SampleWork::new(1))
        .unwrap_err();
    assert!(matches!(
        error,
        crate::noise::ConsumerNoiseError::Work(crate::sample_work::SampleWorkExceeded {
            limit: 1,
            used: 1
        })
    ));
    let jagged = router
        .data
        .noises
        .iter()
        .find(|(id, _)| id.resource_location() == "minecraft:jagged")
        .unwrap();
    let input = Vec3::new(45_000_000_000.0, f64::from(i32::MAX), -45_000_000_000.0);
    assert_eq!(
        jagged.1.sample(input).unwrap_err(),
        crate::noise::NoiseError::InvalidInput
    );
    let count = jagged.1.first().amplitudes().len() + jagged.1.second().amplitudes().len();
    let mut work = SampleWork::new(count);
    assert!(jagged
        .1
        .sample_consumer(input, &mut work)
        .unwrap()
        .is_finite());
    assert_eq!(work.used(), count);
    assert!(matches!(
        jagged
            .1
            .sample_consumer(input, &mut SampleWork::new(count - 1))
            .unwrap_err(),
        crate::noise::ConsumerNoiseError::Work(_)
    ));
    let id = DensityFunctionResourceId::from_name("nether/base_3d_noise").unwrap();
    let pos = BlockPos::new(30_000_000, i32::MAX, -30_000_000);
    assert!(router
        .sample_definition(id, pos, DensityWorkBudget::new(26).unwrap())
        .unwrap()
        .is_finite());
    assert!(matches!(
        router
            .sample_definition(id, pos, DensityWorkBudget::new(25).unwrap())
            .unwrap_err()
            .kind(),
        DensitySampleErrorKind::WorkExceeded {
            limit: 25,
            used: 25
        }
    ));
    let router = probe_router(vec![(original(NodeKind::Unit(Unit::EndIslands)), "end")], 0);
    for (pos, count) in [
        (BlockPos::new(0, 0, 0), 626),
        (BlockPos::new(64000, 0, 0), 1251),
    ] {
        assert!(router
            .sample_field(
                RouterField::FinalDensity,
                pos,
                DensityWorkBudget::new(count).unwrap()
            )
            .unwrap()
            .is_finite());
        assert!(
            matches!(router.sample_field(RouterField::FinalDensity,pos,DensityWorkBudget::new(count-1).unwrap()).unwrap_err().kind(),DensitySampleErrorKind::WorkExceeded {limit,used} if *limit==count-1 && *used==count-1)
        );
    }
    let overflow = BlockPos::new(370_728, 0, 0);
    assert!(matches!(
        router
            .sample_field(
                RouterField::FinalDensity,
                overflow,
                DensityWorkBudget::new(1251).unwrap()
            )
            .unwrap_err()
            .kind(),
        DensitySampleErrorKind::NonFiniteDensity { .. }
    ));
    assert!(matches!(
        router
            .sample_field(
                RouterField::FinalDensity,
                overflow,
                DensityWorkBudget::new(1250).unwrap()
            )
            .unwrap_err()
            .kind(),
        DensitySampleErrorKind::WorkExceeded { .. }
    ));
}
