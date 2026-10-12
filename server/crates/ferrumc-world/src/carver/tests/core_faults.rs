//! Exact faults at the primitive shared cores and actual constructor boundaries.
use super::*;
use crate::noise::{
    BlendedNoise, BlendedNoiseParameters, ConsumerNoiseCoreError, ImprovedNoise, NormalNoise,
    PerlinNoise,
};
use crate::random::{RandomAlgorithm, WorldgenRandomSource};
use crate::sample_work::SampleWork;
use crate::spline::{CubicSpline, SplineCoreError, SplineNodeId, SplineNodeSpec};
use ferrumc_math::Vec3;
fn exhausted(limbs: [u64; 3]) -> SampleWork {
    let n = Unsigned192::from_limbs(limbs);
    let mut m = CarverCumulativeMeter::new(n);
    m.test_position(n, n, 0);
    SampleWork::with_carver_meter(m)
}
#[test]
fn exact_faults_cross_rng_basis_perlin_normal_blended_and_shared_spline() {
    let mut random =
        WorldgenRandomSource::new(RandomAlgorithm::Xoroshiro128PlusPlus, WorldSeed::new(0));
    let improved = ImprovedNoise::new(&mut random);
    let perlin = PerlinNoise::new(&mut random, -1, &[1.0, 1.0]).unwrap();
    let normal = NormalNoise::new(&mut random, -1, &[1.0, 1.0]).unwrap();
    let blended = BlendedNoise::new(
        &mut random,
        BlendedNoiseParameters::new(0.25, 0.125, 80.0, 160.0, 8.0).unwrap(),
    )
    .unwrap();
    let spline = CubicSpline::new(
        &[],
        &[SplineNodeSpec::constant(1.0).unwrap()],
        SplineNodeId::new(0),
    )
    .unwrap();
    for limbs in [[u64::MAX, 0, 0], [u64::MAX, u64::MAX, 0], [u64::MAX; 3]] {
        let expected = ChargeFault::Carver {
            limit: limbs,
            used: limbs,
        };
        let mut work = exhausted(limbs);
        let before = random;
        assert_eq!(
            random.bounded_positive_metered_core(10, &mut work),
            Err(expected)
        );
        assert_eq!(before, random);
        let meter = *work.carver_meter();
        assert_eq!(
            improved.paper_build60_basis_core(&mut work).unwrap_err(),
            expected
        );
        assert_eq!(
            perlin.paper_build60_basis_core(&mut work).unwrap_err(),
            expected
        );
        assert_eq!(
            normal.paper_build60_basis_core(&mut work).unwrap_err(),
            expected
        );
        assert_eq!(
            blended.paper_build60_basis_core(&mut work).unwrap_err(),
            expected
        );
        for error in [
            perlin
                .sample_consumer_core(Vec3::new(1.0, 2.0, 3.0), &mut work)
                .unwrap_err(),
            normal
                .sample_consumer_core(Vec3::new(1.0, 2.0, 3.0), &mut work)
                .unwrap_err(),
            blended
                .sample_consumer_core(BlockPos::new(1, 2, 3), &mut work)
                .unwrap_err(),
        ] {
            match error {
                ConsumerNoiseCoreError::Work(fault) => assert_eq!(fault, expected),
                ConsumerNoiseCoreError::Noise(_) => panic!("narrow or numeric fault"),
            }
        }
        let error = spline
            .sample_shared_core(&mut work, &mut |_, _| Ok::<_, ()>(0.0))
            .unwrap_err();
        match error {
            SplineCoreError::Work(fault) => assert_eq!(fault, expected),
            _ => panic!("narrow spline fault"),
        }
        assert_eq!(*work.carver_meter(), meter);
    }
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The exact constructor and nested fault payloads share one authority."
)]
fn actual_constructor_n_minus_n_plus_and_deep_queries_keep_exact_payloads() {
    let s = setup();
    let cold = s
        .router
        .fresh_carver_noise_chunk(ChunkPos::new(0, 0), CarverCumulativeMeter::new(NOISE_TOTAL))
        .unwrap();
    let actual = cold.carver_meter().used();
    assert!(actual < Unsigned192::from_u64(PREPARE_RESERVED));
    for delta in [-1_i32, 0, 1] {
        let allowance = if delta < 0 {
            actual.checked_sub(Unsigned192::from_u64(1)).unwrap()
        } else {
            actual
                .checked_add(Unsigned192::from_u64(delta as u64))
                .unwrap()
        };
        let result = s
            .router
            .fresh_carver_noise_chunk(ChunkPos::new(0, 0), CarverCumulativeMeter::new(allowance));
        assert_eq!(result.is_ok(), delta >= 0);
        if delta < 0 {
            assert_eq!(
                noise_error(result.err().unwrap()).kind(),
                &CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Noise,
                    required: [1, 0, 0],
                    limit: allowance.limbs(),
                    used: allowance.limbs()
                }
            );
        }
    }
    // The final constructor charge occurs inside the real validation closure,
    // with every Cold/Keys/Spline/basis charge already debited from this meter.
    let offset = Unsigned192::from_limbs([u64::MAX - 10, u64::MAX, 3]);
    let limit = offset
        .checked_add(actual)
        .unwrap()
        .checked_sub(Unsigned192::from_u64(1))
        .unwrap();
    let mut meter = CarverCumulativeMeter::new(limit);
    meter.test_position(limit, offset, 0);
    meter.test_short_segments();
    let error = s
        .router
        .fresh_carver_noise_chunk(ChunkPos::new(0, 0), meter)
        .err()
        .unwrap();
    assert_eq!(
        noise_error(error).kind(),
        &CarverErrorKind::ProducerWorkExceeded {
            producer: CarverProducer::Noise,
            required: [1, 0, 0],
            limit: limit.limbs(),
            used: limit.limbs()
        }
    );
    for allowance in [1, 5, 20, 50, 100, 300] {
        let mut invocation = run(&s);
        let used = offset
            .checked_add(invocation.noise.carver_meter().used())
            .unwrap();
        let limit = used.checked_add(Unsigned192::from_u64(allowance)).unwrap();
        invocation
            .noise
            .carver_meter_mut()
            .test_position(limit, used, 0);
        invocation.noise.carver_meter_mut().test_short_segments();
        let error = invocation
            .noise
            .sample_carver_climate(CarverClimateQuart::source(invocation.chunk, -8, -8).unwrap())
            .unwrap_err();
        assert_eq!(
            noise_error(error).kind(),
            &CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Noise,
                required: [1, 0, 0],
                limit: limit.limbs(),
                used: limit.limbs()
            }
        );
        let mut invocation = run(&s);
        let used = offset
            .checked_add(invocation.noise.carver_meter().used())
            .unwrap();
        let limit = used.checked_add(Unsigned192::from_u64(allowance)).unwrap();
        invocation
            .noise
            .carver_meter_mut()
            .test_position(limit, used, 0);
        invocation.noise.carver_meter_mut().test_short_segments();
        let error = invocation
            .aquifer
            .compute(&mut invocation.noise, BlockPos::new(0, 20, 0))
            .unwrap_err();
        assert_eq!(
            noise_error(error).kind(),
            &CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Noise,
                required: [1, 0, 0],
                limit: limit.limbs(),
                used: limit.limbs()
            }
        );
    }
}

#[test]
fn lazy_rule_provider_preserves_exact_noise_and_selector_faults() {
    use crate::surface_rules::{SurfaceBlockInput, SurfaceColumnPos, WorldSurfaceHeights};
    for producer in [CarverProducer::Noise, CarverProducer::Selector] {
        let mut s = setup();
        s.rules.carver_test_rule(2);
        let mut invocation = run(&s);
        let n = Unsigned192::from_limbs([u64::MAX - 1, u64::MAX, 9]);
        match producer {
            CarverProducer::Noise => invocation.noise.carver_meter_mut().test_position(n, n, 0),
            CarverProducer::Selector => invocation
                .selector
                .carver_meter_mut()
                .test_position(n, n, 0),
            CarverProducer::Surface => unreachable!(),
        }
        invocation
            .context
            .begin_column(SurfaceColumnPos::from_block(BlockPos::new(1, -63, 12)))
            .unwrap();
        let mut work = GeometryWork {
            limit: CARVER_CHUNK_WORK,
            used: 0,
        };
        let mut direct = DirectCarverBiomes {
            center: invocation.chunk,
            noise: &mut invocation.noise,
            selector: &mut invocation.selector,
            zoom: invocation.zoom,
            work: &mut work,
            queries: None,
        };
        let error = invocation
            .context
            .evaluate_carver(
                SurfaceBlockInput::new(-63, 1, 1, None),
                &WorldSurfaceHeights::new([[0; 16]; 16]),
                &mut direct,
            )
            .err()
            .unwrap();
        match error {
            crate::surface_rules::RuleCoreError::Direct(error) => assert_eq!(
                error.kind(),
                &CarverErrorKind::ProducerWorkExceeded {
                    producer,
                    required: [1, 0, 0],
                    limit: n.limbs(),
                    used: n.limbs()
                }
            ),
            crate::surface_rules::RuleCoreError::Rule(_) => {
                panic!("wide fault reached rule adapter")
            }
        }
        assert_eq!(work.used, 74);
        assert!(matches!(
            invocation
                .context
                .admit_carver(
                    crate::stored_biome::StoredBiomeIdentity::new(
                        s.router.seed(),
                        s.router.resource(),
                        source(),
                        crate::biome_selection::BiomeQuartGeometry::new(-64, 384).unwrap()
                    ),
                    invocation.zoom.semantics(),
                    invocation.chunk
                )
                .unwrap_err()
                .kind(),
            crate::surface_rules::SurfaceRuleErrorKind::Poisoned
        ));
    }
}
