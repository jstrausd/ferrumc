//! Fresh-process allocator checks. No Carver call occurs before DRIVE.
use super::*;
use std::hint::black_box;
use std::io::{self, Write};
fn boundary(bytes: &[u8]) {
    let mut out = io::stdout().lock();
    out.write_all(bytes).unwrap();
    out.flush().unwrap();
}
#[test]
#[ignore = "external cold allocator probe; Debug and Release"]
#[expect(
    clippy::too_many_lines,
    reason = "All prepared refusal families share the first measured cold interval."
)]
fn cold_success_and_every_refusal() {
    let mut s = setup();
    let identity = s.surface.published().unwrap();
    let other_resource = NoiseSettingsResourceId::from_name("minecraft:amplified").unwrap();
    let foreign_resource_router = RouterTemplate::compile_builtin(other_resource)
        .unwrap()
        .bind_seed(WorldSeed::new(0))
        .unwrap();
    let foreign_seed_router = RouterTemplate::compile_builtin(s.binding.resource)
        .unwrap()
        .bind_seed(WorldSeed::new(7))
        .unwrap();
    let mut publishers = Vec::with_capacity(21);
    let mut invocations = Vec::with_capacity(21);
    let mut bindings = Vec::with_capacity(21);
    for defect in 0..21 {
        publishers.push(
            s.binding
                .prepare_run(
                    &s.router,
                    &s.rules,
                    &s.temperatures,
                    source(),
                    identity.0,
                    s.corners,
                )
                .unwrap(),
        );
        let mut invocation = s
            .binding
            .prepare_run(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                identity.0,
                s.corners,
            )
            .unwrap();
        let mut binding =
            SeededCarvers::bind(&s.columns, CarverBindBudget::new(90).unwrap()).unwrap();
        match defect {
            0 => binding.seed = WorldSeed::new(7),
            1 => binding.resource = other_resource,
            6 => {
                invocation.zoom = BiomeZoom::new(
                    BiomeZoomSeed::from_world_seed(WorldSeed::new(7)),
                    BiomeZoomSemantics::Paper1_21_8Build60,
                );
            }
            7 => invocation.source = BiomeSourceIdentity::End,
            8 => {
                invocation.zoom =
                    BiomeZoom::new(invocation.zoom.seed(), BiomeZoomSemantics::Vanilla1_21_8);
            }
            9 => invocation.chunk = ChunkPos::new(1, 0),
            10 => invocation.lifecycle = PreparedCarverState::Complete,
            11 => invocation
                .context
                .carver_test_context(ChunkPos::new(1, 0), false),
            12 => invocation.context.carver_test_context(identity.0, true),
            13 => {
                let used = invocation.noise.carver_meter().used();
                invocation.noise.carver_meter_mut().test_position(
                    NOISE_REMAINING
                        .checked_add(used)
                        .unwrap()
                        .checked_sub(Unsigned192::from_u64(1))
                        .unwrap(),
                    used,
                    0,
                );
            }
            14 => invocation.selector.carver_meter_mut().test_position(
                Unsigned192::from_u64(SELECTOR_WORK - 1),
                Unsigned192::ZERO,
                0,
            ),
            15 => invocation.context.set_work_used(1),
            17 => binding.len = 0,
            18 => {
                invocation.noise = foreign_resource_router
                    .fresh_carver_noise_chunk(identity.0, CarverCumulativeMeter::new(NOISE_TOTAL))
                    .unwrap();
            }
            19 => invocation.noise.carver_test_geometry(),
            20 => {
                invocation.noise = foreign_seed_router
                    .fresh_carver_noise_chunk(identity.0, CarverCumulativeMeter::new(NOISE_TOTAL))
                    .unwrap();
            }
            _ => {}
        }
        invocations.push(invocation);
        bindings.push(binding);
    }
    let mut constructors = Vec::with_capacity(1);
    let mut outputs = Vec::with_capacity(1);
    boundary(b"GEN057_BEGIN_CONTROL\n");
    let mut control = Vec::<u8>::with_capacity(4096);
    control.push(1);
    black_box(&control);
    drop(control);
    boundary(b"GEN057_END_CONTROL\n");
    boundary(b"GEN057_BEGIN_BUILD\n");
    constructors.push(
        s.binding
            .prepare_run(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                identity.0,
                s.corners,
            )
            .unwrap(),
    );
    outputs.push(CarvedChunkBuffer::new().unwrap());
    boundary(b"GEN057_END_BUILD\n");
    let mut output = outputs.pop().unwrap();
    let mut successes = 0;
    let mut refusals = 0;
    boundary(b"GEN057_BEGIN_DRIVE\n");
    for (defect, ((publisher, invocation), binding)) in publishers
        .iter_mut()
        .zip(&mut invocations)
        .zip(&bindings)
        .enumerate()
    {
        s.surface.carver_test_identity(Some(identity));
        s.binding
            .carve_chunk(
                CarverInput::new(&s.surface, publisher),
                CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                &mut output,
            )
            .unwrap();
        successes += 1;
        assert!(output.published().is_some());
        match defect {
            2 => s.surface.carver_test_identity(None),
            3 => s
                .surface
                .carver_test_identity(Some((identity.0, WorldSeed::new(7), identity.2))),
            4 => s
                .surface
                .carver_test_identity(Some((identity.0, identity.1, other_resource))),
            5 | 9 => {
                s.surface
                    .carver_test_identity(Some((ChunkPos::new(1, 0), identity.1, identity.2)));
            }
            _ => {}
        }
        let error = binding
            .carve_chunk(
                CarverInput::new(&s.surface, invocation),
                CarverWorkBudget::new(if defect == 16 {
                    CARVER_CHUNK_WORK - 1
                } else {
                    CARVER_CHUNK_WORK
                })
                .unwrap(),
                &mut output,
            )
            .unwrap_err();
        let expected = match defect {
            0 => CarverErrorKind::InconsistentBinding(BindingMismatch::Seed),
            1 => CarverErrorKind::InconsistentBinding(BindingMismatch::Settings),
            2 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::NotPublished),
            3 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::Seed),
            4 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::Settings),
            5 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::Chunk),
            6 | 20 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Seed),
            7 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Source),
            8 => CarverErrorKind::InconsistentProducer(ProducerMismatch::ZoomSemantics),
            9 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Chunk),
            10 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Lifecycle),
            11 => CarverErrorKind::Rules(
                crate::surface_rules::SurfaceRuleErrorKind::ColumnOutsideCentre,
            ),
            12 => CarverErrorKind::Rules(crate::surface_rules::SurfaceRuleErrorKind::Poisoned),
            13 => CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Noise,
                required: NOISE_REMAINING.limbs(),
                limit: invocation.noise.carver_meter().limit().limbs(),
                used: invocation.noise.carver_meter().used().limbs(),
            },
            14 => CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Selector,
                required: Unsigned192::from_u64(SELECTOR_WORK).limbs(),
                limit: Unsigned192::from_u64(SELECTOR_WORK - 1).limbs(),
                used: [0; 3],
            },
            15 => CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Surface,
                required: Unsigned192::from_u64(u64::from(RULE_WORK)).limbs(),
                limit: Unsigned192::from_u64(u64::from(RULE_WORK - 1)).limbs(),
                used: [0; 3],
            },
            16 => CarverErrorKind::WorkBudgetExceeded {
                required: CARVER_CHUNK_WORK,
                available: CARVER_CHUNK_WORK - 1,
            },
            17 => CarverErrorKind::UnclassifiedState,
            18 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Settings),
            19 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Geometry),
            _ => unreachable!(),
        };
        assert_eq!(black_box(error).kind(), &expected);
        assert_eq!(
            invocation.state(),
            if defect == 17 {
                PreparedCarverState::Poisoned
            } else if defect == 10 {
                PreparedCarverState::Complete
            } else {
                PreparedCarverState::Ready
            }
        );
        assert!(output.published().is_none());
        refusals += 1;
    }
    // All pre-construction work/native/source refusals are cold and typed.
    for units in [0, MAX_CARVER_WORK + 1] {
        assert_eq!(
            CarverWorkBudget::new(units).unwrap_err().kind(),
            &CarverErrorKind::InvalidBudget {
                units,
                maximum: MAX_CARVER_WORK
            }
        );
    }
    for units in [0, MAX_CARVER_BIND_WORK + 1] {
        assert_eq!(
            CarverBindBudget::new(units).unwrap_err().kind(),
            &CarverErrorKind::InvalidBudget {
                units: u64::from(units),
                maximum: u64::from(MAX_CARVER_BIND_WORK)
            }
        );
    }
    assert_eq!(
        SeededCarvers::bind(&s.columns, CarverBindBudget::new(89).unwrap())
            .unwrap_err()
            .kind(),
        &CarverErrorKind::WorkBudgetExceeded {
            required: 90,
            available: 89
        }
    );
    assert_eq!(
        SeededCarvers::bind_with(&s.columns, CarverBindBudget::new(90).unwrap(), 0)
            .unwrap_err()
            .kind(),
        &CarverErrorKind::NativeLimit {
            category: CarverLimit::Binding,
            required: size_of::<SeededCarvers>(),
            available: 0
        }
    );
    assert_eq!(
        CarvedChunkBuffer::new_with(0).unwrap_err().kind(),
        &CarverErrorKind::NativeLimit {
            category: CarverLimit::Buffer,
            required: output.retained_bytes(),
            available: 0
        }
    );
    assert_eq!(
        s.binding
            .prepare_with(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                identity.0,
                s.corners,
                NOISE_TOTAL,
                Unsigned192::from_u64(SELECTOR_WORK),
                RULE_WORK,
                0
            )
            .err()
            .unwrap()
            .kind(),
        &CarverErrorKind::NativeLimit {
            category: CarverLimit::Run,
            required: size_of::<PreparedCarverRun<'_>>(),
            available: 0
        }
    );
    assert_eq!(
        s.binding
            .prepare_run(
                &s.router,
                &s.rules,
                &s.temperatures,
                BiomeSourceIdentity::End,
                identity.0,
                s.corners
            )
            .err()
            .unwrap()
            .kind(),
        &CarverErrorKind::InconsistentProducer(ProducerMismatch::Source)
    );
    assert_eq!(
        s.binding
            .prepare_run(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                ChunkPos::new(1_875_000, 0),
                s.corners
            )
            .err()
            .unwrap()
            .kind(),
        &CarverErrorKind::InconsistentProducer(ProducerMismatch::Chunk)
    );
    for producer in 0..3 {
        let noise_limit = if producer == 0 {
            NOISE_TOTAL.checked_sub(Unsigned192::from_u64(1)).unwrap()
        } else {
            NOISE_TOTAL
        };
        let selector_limit = Unsigned192::from_u64(if producer == 1 {
            SELECTOR_WORK - 1
        } else {
            SELECTOR_WORK
        });
        let rule_limit = if producer == 2 {
            RULE_WORK - 1
        } else {
            RULE_WORK
        };
        let error = s
            .binding
            .prepare_with(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                identity.0,
                s.corners,
                noise_limit,
                selector_limit,
                rule_limit,
                RUN_VALUE_CAP,
            )
            .err()
            .unwrap();
        let (kind, required, limit) = match producer {
            0 => (CarverProducer::Noise, NOISE_TOTAL, noise_limit),
            1 => (
                CarverProducer::Selector,
                Unsigned192::from_u64(SELECTOR_WORK),
                selector_limit,
            ),
            2 => (
                CarverProducer::Surface,
                Unsigned192::from_u64(u64::from(RULE_WORK)),
                Unsigned192::from_u64(u64::from(rule_limit)),
            ),
            _ => unreachable!(),
        };
        assert_eq!(
            error.kind(),
            &CarverErrorKind::ProducerWorkExceeded {
                producer: kind,
                required: required.limbs(),
                limit: limit.limbs(),
                used: [0; 3]
            }
        );
    }
    boundary(b"GEN057_END_DRIVE\n");
    black_box((&constructors, &output));
    assert_eq!((successes, refusals), (21, 21));
    println!(
        "GEN0511_ALLOC_CENSUS first-cold=1 success=21 published-refusal=21 constructor-refusal=13"
    );
}

#[test]
#[ignore = "external full ownership census; Debug and Release"]
fn cold_full_native_census() {
    boundary(b"GEN057_BEGIN_CONTROL\n");
    let mut control = Vec::<u8>::with_capacity(4096);
    control.push(1);
    black_box(&control);
    drop(control);
    boundary(b"GEN057_END_CONTROL\n");
    boundary(b"GEN057_BEGIN_BUILD\n");
    let s = setup();
    let mut invocation = run(&s);
    let mut output = CarvedChunkBuffer::new().unwrap();
    boundary(b"GEN057_END_BUILD\n");
    boundary(b"GEN057_BEGIN_DRIVE\n");
    s.binding
        .carve_chunk(
            CarverInput::new(&s.surface, &mut invocation),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut output,
        )
        .unwrap();
    boundary(b"GEN057_END_DRIVE\n");
    black_box((&s, &invocation, &output));
    let (tree_bytes, tree_refs) = invocation.selector.carver_native_tree();
    assert_eq!(tree_bytes, 1_181_618);
    assert_eq!(tree_refs, 1);
    assert_eq!(s.columns.retained_bytes(), 10_816);
    assert_eq!(s.surface.retained_bytes(), 406_592);
    assert_eq!(output.retained_bytes(), 418_896);
    // The complete external BUILD live census includes all heap categories,
    // even Arc headers and BTree allocations not included in declared caps.
    // These subcategories overlap that census and must never be added twice.
    println!("GEN0511_NATIVE_CATEGORIES output-owned={} surface-borrowed={} columns-borrowed={} binding-value={} rules-binding-borrowed={} rules-template-borrowed={} temperature-inline={} selector-tree={} tree-arc-header=16 tree-refs={} noise-graph-admitted/cache-admitted/scratch-requested/preliminary-requested/router-arc-refs/definition-map-entries={:?} canyon-stack=1536 tunnel-depth=2", output.retained_bytes(), s.surface.retained_bytes(), s.columns.retained_bytes(), size_of::<SeededCarvers>(), s.rules.retained_bytes(), s.rules.template().retained_bytes(), size_of::<BuiltinBiomeTemperatures>(), tree_bytes, tree_refs, invocation.noise.carver_native_categories());
    println!(
        "GEN0511_NATIVE_CENSUS complete-simultaneous-inputs=1 prepared=1 output=1 staticSIN=262144"
    );
}
