//! Closed producer/context admission order with actual publication before refusal.
use super::*;
use crate::surface_rules::SurfaceRuleErrorKind;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The complete ordered refusal matrix shares one publication harness."
)]
fn producer_context_and_allowance_suffixes_refuse_without_mutating_inputs() {
    let s = setup();
    let original = s.surface.carver_contents().unwrap();
    let other = NoiseSettingsResourceId::from_name("minecraft:amplified").unwrap();
    let foreign = RouterTemplate::compile_builtin(other)
        .unwrap()
        .bind_seed(WorldSeed::new(0))
        .unwrap();
    let mut output = CarvedChunkBuffer::new().unwrap();
    for combined in [false, true] {
        for first in 0..9 {
            let mut publisher = run(&s);
            s.binding
                .carve_chunk(
                    CarverInput::new(&s.surface, &mut publisher),
                    CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                    &mut output,
                )
                .unwrap();
            assert!(output.published().is_some());
            let mut invocation = run(&s);
            let defect = |n| first == n || combined && n > first;
            if defect(0) {
                invocation.zoom = BiomeZoom::new(
                    BiomeZoomSeed::from_world_seed(WorldSeed::new(7)),
                    BiomeZoomSemantics::Paper1_21_8Build60,
                );
            }
            if defect(1) {
                invocation.noise = foreign
                    .fresh_carver_noise_chunk(
                        invocation.chunk,
                        CarverCumulativeMeter::new(NOISE_TOTAL),
                    )
                    .unwrap();
            }
            if defect(2) {
                invocation.source = BiomeSourceIdentity::End;
            }
            if defect(4) {
                invocation.zoom =
                    BiomeZoom::new(invocation.zoom.seed(), BiomeZoomSemantics::Vanilla1_21_8);
            }
            if defect(5) {
                invocation.noise = (if defect(1) { &foreign } else { &s.router })
                    .fresh_carver_noise_chunk(
                        ChunkPos::new(1, 0),
                        CarverCumulativeMeter::new(NOISE_TOTAL),
                    )
                    .unwrap();
            }
            if defect(3) {
                invocation.noise.carver_test_geometry();
            }
            if defect(6) {
                invocation.lifecycle = PreparedCarverState::Complete;
            }
            if defect(7) {
                invocation
                    .context
                    .carver_test_context(ChunkPos::new(1, 0), defect(8));
            } else if defect(8) {
                invocation
                    .context
                    .carver_test_context(invocation.chunk, true);
            }
            // All later allowance defects are simultaneously present.
            if combined {
                let used = invocation.noise.carver_meter().used();
                invocation
                    .noise
                    .carver_meter_mut()
                    .test_position(used, used, 0);
                invocation.selector.carver_meter_mut().test_position(
                    Unsigned192::ZERO,
                    Unsigned192::ZERO,
                    0,
                );
                invocation.context.set_work_used(1);
            }
            let before = (
                invocation.state(),
                invocation.noise.carver_meter().used(),
                invocation.selector.carver_meter().used(),
                invocation.context.work_used(),
                invocation.noise.carver_history(),
                invocation.aquifer.carver_history(),
                invocation.selector.carver_history(),
            );
            let error = s
                .binding
                .carve_chunk(
                    CarverInput::new(&s.surface, &mut invocation),
                    CarverWorkBudget::new(CARVER_CHUNK_WORK - 1).unwrap(),
                    &mut output,
                )
                .unwrap_err();
            let expected = match first {
                0 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Seed),
                1 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Settings),
                2 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Source),
                3 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Geometry),
                4 => CarverErrorKind::InconsistentProducer(ProducerMismatch::ZoomSemantics),
                5 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Chunk),
                6 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Lifecycle),
                7 => CarverErrorKind::Rules(SurfaceRuleErrorKind::ColumnOutsideCentre),
                8 => CarverErrorKind::Rules(SurfaceRuleErrorKind::Poisoned),
                _ => unreachable!(),
            };
            assert_eq!(error.kind(), &expected, "first={first} combined={combined}");
            assert!(output.published().is_none());
            assert_eq!(s.surface.carver_contents().unwrap(), original);
            assert_eq!(
                before,
                (
                    invocation.state(),
                    invocation.noise.carver_meter().used(),
                    invocation.selector.carver_meter().used(),
                    invocation.context.work_used(),
                    invocation.noise.carver_history(),
                    invocation.aquifer.carver_history(),
                    invocation.selector.carver_history()
                )
            );
        }
    }
}

#[test]
fn remaining_noise_n_minus_n_plus_is_checked_by_the_actual_published_invocation() {
    let s = setup();
    let mut output = CarvedChunkBuffer::new().unwrap();
    for delta in [-1_i32, 0, 1] {
        let mut publisher = run(&s);
        s.binding
            .carve_chunk(
                CarverInput::new(&s.surface, &mut publisher),
                CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                &mut output,
            )
            .unwrap();
        assert!(output.published().is_some());
        let mut invocation = run(&s);
        let used = invocation.noise.carver_meter().used();
        let remaining = if delta < 0 {
            NOISE_REMAINING
                .checked_sub(Unsigned192::from_u64(1))
                .unwrap()
        } else {
            NOISE_REMAINING
                .checked_add(Unsigned192::from_u64(delta as u64))
                .unwrap()
        };
        let limit = used.checked_add(remaining).unwrap();
        invocation
            .noise
            .carver_meter_mut()
            .test_position(limit, used, 0);
        let result = s.binding.carve_chunk(
            CarverInput::new(&s.surface, &mut invocation),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut output,
        );
        if delta < 0 {
            assert_eq!(
                result.unwrap_err().kind(),
                &CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Noise,
                    required: NOISE_REMAINING.limbs(),
                    limit: limit.limbs(),
                    used: used.limbs()
                }
            );
            assert_eq!(invocation.noise.carver_meter().used(), used);
            assert_eq!(invocation.state(), PreparedCarverState::Ready);
            assert!(output.published().is_none());
        } else {
            assert!(result.is_ok());
            assert_eq!(invocation.state(), PreparedCarverState::Complete);
            assert!(output.published().is_some());
        }
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One matrix asserts every constructor admission suffix before any construction."
)]
fn prepare_individual_and_combined_refusals_are_exact() {
    let s = setup();
    let original = s.surface.carver_contents().unwrap();
    let other = NoiseSettingsResourceId::from_name("minecraft:amplified").unwrap();
    for combined in [false, true] {
        for first in 0..10 {
            let defect = |n| first == n || combined && n > first;
            let mut binding =
                SeededCarvers::bind(&s.columns, CarverBindBudget::new(90).unwrap()).unwrap();
            if defect(0) {
                binding.seed = WorldSeed::new(7);
            }
            if defect(1) {
                binding.resource = other;
            }
            let mut router = RouterTemplate::compile_builtin(if defect(3) {
                other
            } else {
                s.binding.resource()
            })
            .unwrap()
            .bind_seed(WorldSeed::new(if defect(2) { 9 } else { 0 }))
            .unwrap();
            if defect(5) {
                router.carver_test_router_geometry();
            }
            let noise_limit = if defect(7) {
                NOISE_TOTAL.checked_sub(Unsigned192::from_u64(1)).unwrap()
            } else {
                NOISE_TOTAL
            };
            let selector_limit = Unsigned192::from_u64(if defect(8) {
                SELECTOR_WORK - 1
            } else {
                SELECTOR_WORK
            });
            let rule_limit = if defect(9) { RULE_WORK - 1 } else { RULE_WORK };
            let cap = if defect(6) { 0 } else { RUN_VALUE_CAP };
            // Halo has its own matrix below; these suffixes retain a valid centre.
            let error = binding
                .prepare_with(
                    &router,
                    &s.rules,
                    &s.temperatures,
                    if defect(4) {
                        BiomeSourceIdentity::End
                    } else {
                        source()
                    },
                    ChunkPos::new(0, 0),
                    s.corners,
                    noise_limit,
                    selector_limit,
                    rule_limit,
                    cap,
                )
                .err()
                .unwrap();
            let expected = match first {
                0 => CarverErrorKind::InconsistentBinding(BindingMismatch::Seed),
                1 => CarverErrorKind::InconsistentBinding(BindingMismatch::Settings),
                2 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Seed),
                3 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Settings),
                4 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Source),
                5 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Geometry),
                6 => CarverErrorKind::NativeLimit {
                    category: CarverLimit::Run,
                    required: size_of::<PreparedCarverRun<'_>>(),
                    available: cap,
                },
                7 => CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Noise,
                    required: NOISE_TOTAL.limbs(),
                    limit: noise_limit.limbs(),
                    used: [0; 3],
                },
                8 => CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Selector,
                    required: Unsigned192::from_u64(SELECTOR_WORK).limbs(),
                    limit: selector_limit.limbs(),
                    used: [0; 3],
                },
                9 => CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Surface,
                    required: Unsigned192::from_u64(u64::from(RULE_WORK)).limbs(),
                    limit: Unsigned192::from_u64(u64::from(rule_limit)).limbs(),
                    used: [0; 3],
                },
                _ => unreachable!(),
            };
            assert_eq!(error.kind(), &expected, "first={first} combined={combined}");
            assert_eq!(s.surface.carver_contents().unwrap(), original);
        }
    }
    // The centre/halo refusal wins over native and all producer allowances.
    for combined in [false, true] {
        let error = s
            .binding
            .prepare_with(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                ChunkPos::new(1_875_000, 0),
                s.corners,
                if combined {
                    Unsigned192::ZERO
                } else {
                    NOISE_TOTAL
                },
                if combined {
                    Unsigned192::ZERO
                } else {
                    Unsigned192::from_u64(SELECTOR_WORK)
                },
                if combined { 1 } else { RULE_WORK },
                if combined { 0 } else { RUN_VALUE_CAP },
            )
            .err()
            .unwrap();
        assert_eq!(
            error.kind(),
            &CarverErrorKind::InconsistentProducer(ProducerMismatch::Chunk)
        );
        assert_eq!(s.surface.carver_contents().unwrap(), original);
    }
}
