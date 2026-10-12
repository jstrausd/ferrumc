//! Admission and literal domain controls, separate from natural-world expectations.
use super::*;
use crate::biome_selection::{
    BiomeQuartGeometry, BiomeSourceSession, BiomeWorkBudget, MAX_BIOME_SELECTION_WORK,
};
use crate::biome_temperature::BiomeTemperatureBuildBudget;
use crate::biome_zoom::BiomeZoomSemantics;
use crate::density::{NoiseChunkWorkBudget, RouterTemplate, MAX_NOISE_CHUNK_WORK};
use crate::stored_biome::{
    StoredBiomeBuildBudget, StoredBiomeIdentity, SurfaceBiomeResolver, MAX_STORED_BIOME_BUILD_WORK,
};
use crate::surface_rules::{
    PaperFlatBedrock, SurfaceChunkInput, SurfaceColumnBindBudget, SurfaceColumnWorkBudget,
    SurfaceNumericProfile, SurfaceRuleBindBudget, SurfaceRuleBuildBudget, SurfaceRuleTemplate,
    MAX_SURFACE_COLUMN_BIND_WORK, MAX_SURFACE_RULE_BIND_WORK, MAX_SURFACE_RULE_BUILD_WORK,
    SURFACE_CHUNK_WORK,
};
use ferrumc_math::BlockPos;

pub(super) struct Setup {
    pub(super) router: SeededRouter,
    pub(super) rules: SeededSurfaceRules,
    pub(super) temperatures: BuiltinBiomeTemperatures,
    pub(super) columns: SeededSurfaceColumns,
    pub(super) binding: SeededCarvers,
    pub(super) corners: PreliminarySurfaceCorners,
    pub(super) surface: SurfaceChunkBuffer,
}
pub(super) fn setup() -> Setup {
    setup_at(0, ChunkPos::new(0, 0))
}
#[expect(
    clippy::too_many_lines,
    reason = "The fixture constructs all actual accepted producers."
)]
fn setup_at(seed: i64, chunk: ChunkPos) -> Setup {
    let settings = NoiseSettingsResourceId::from_name("minecraft:overworld").unwrap();
    let seed = WorldSeed::new(seed);
    let router = RouterTemplate::compile_builtin(settings)
        .unwrap()
        .bind_seed(seed)
        .unwrap();
    let rules = SeededSurfaceRules::bind(
        Arc::new(
            SurfaceRuleTemplate::compile_builtin(
                settings,
                SurfaceRuleBuildBudget::new(MAX_SURFACE_RULE_BUILD_WORK).unwrap(),
            )
            .unwrap(),
        ),
        seed,
        SurfaceNumericProfile::PaperBuild60,
        PaperFlatBedrock::Disabled,
        SurfaceRuleBindBudget::new(MAX_SURFACE_RULE_BIND_WORK).unwrap(),
    )
    .unwrap();
    let temperatures =
        BuiltinBiomeTemperatures::compile(BiomeTemperatureBuildBudget::new(1_048_576).unwrap())
            .unwrap();
    let columns = SeededSurfaceColumns::bind(
        &rules,
        SurfaceColumnBindBudget::new(MAX_SURFACE_COLUMN_BIND_WORK).unwrap(),
    )
    .unwrap();
    let binding = SeededCarvers::bind(&columns, CarverBindBudget::new(90).unwrap()).unwrap();
    let mut noise = router
        .fresh_paper_noise_chunk(
            chunk,
            NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
        )
        .unwrap();
    let corners = PreliminarySurfaceCorners::new(
        noise
            .preliminary_surface_level(BlockPos::new(chunk.x() * 16, 0, chunk.z() * 16))
            .unwrap(),
        noise
            .preliminary_surface_level(BlockPos::new((chunk.x() + 1) * 16, 0, chunk.z() * 16))
            .unwrap(),
        noise
            .preliminary_surface_level(BlockPos::new(chunk.x() * 16, 0, (chunk.z() + 1) * 16))
            .unwrap(),
        noise
            .preliminary_surface_level(BlockPos::new((chunk.x() + 1) * 16, 0, (chunk.z() + 1) * 16))
            .unwrap(),
    );
    let material = router
        .fresh_paper_noise_chunk(
            chunk,
            NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
        )
        .unwrap()
        .sample_chunk_materials()
        .unwrap();
    let source = source();
    let geometry = BiomeQuartGeometry::new(-64, 384).unwrap();
    let tree = Arc::new(
        BiomeTree::from_builtin(
            BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
            BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
        )
        .unwrap(),
    );
    let mut biomes = BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
        tree,
        BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
    ));
    let mut arrays = Vec::new();
    for x in -1..=1 {
        for z in -1..=1 {
            let mut noise = router
                .fresh_paper_noise_chunk(
                    ChunkPos::new(chunk.x() + x, chunk.z() + z),
                    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
                )
                .unwrap();
            arrays.push(biomes.sample_chunk_biomes(&mut noise, geometry).unwrap());
        }
    }
    let refs: Vec<_> = arrays.iter().collect();
    let resolver = SurfaceBiomeResolver::new(
        StoredBiomeIdentity::new(seed, settings, source, geometry),
        chunk,
        &refs,
        BiomeZoomSemantics::Paper1_21_8Build60,
        StoredBiomeBuildBudget::new(MAX_STORED_BIOME_BUILD_WORK).unwrap(),
    )
    .unwrap();
    let mut surface = SurfaceChunkBuffer::new().unwrap();
    columns
        .build_chunk(
            SurfaceChunkInput::new(&rules, &resolver, &temperatures, corners, &material),
            SurfaceColumnWorkBudget::new(SURFACE_CHUNK_WORK).unwrap(),
            &mut surface,
        )
        .unwrap();
    Setup {
        router,
        rules,
        temperatures,
        columns,
        binding,
        corners,
        surface,
    }
}
fn source() -> BiomeSourceIdentity {
    BiomeSourceIdentity::MultiNoise(
        BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
    )
}
pub(super) fn run(s: &Setup) -> PreparedCarverRun<'_> {
    s.binding
        .prepare_run(
            &s.router,
            &s.rules,
            &s.temperatures,
            source(),
            s.surface.published().unwrap().0,
            s.corners,
        )
        .unwrap()
}
fn prepare(
    s: &Setup,
    n: Unsigned192,
    selector: Unsigned192,
    r: u32,
    cap: usize,
) -> Result<PreparedCarverRun<'_>, CarverError> {
    s.binding.prepare_with(
        &s.router,
        &s.rules,
        &s.temperatures,
        source(),
        ChunkPos::new(0, 0),
        s.corners,
        n,
        selector,
        r,
        cap,
    )
}
#[test]
fn public_caps_bind_census_and_literal_native_values() {
    for n in [0, 1, MAX_CARVER_WORK, MAX_CARVER_WORK + 1] {
        assert_eq!(
            CarverWorkBudget::new(n).is_ok(),
            (1..=MAX_CARVER_WORK).contains(&n)
        );
    }
    for n in [0, 1, MAX_CARVER_BIND_WORK, MAX_CARVER_BIND_WORK + 1] {
        assert_eq!(
            CarverBindBudget::new(n).is_ok(),
            (1..=MAX_CARVER_BIND_WORK).contains(&n)
        );
    }
    let s = setup();
    for n in [89, 90, 91] {
        assert_eq!(
            SeededCarvers::bind(&s.columns, CarverBindBudget::new(n).unwrap()).is_ok(),
            n >= 90
        );
    }
    let binding_size = size_of::<SeededCarvers>();
    let buffer_size = CarvedChunkBuffer::new().unwrap().retained_bytes();
    for delta in [-1_isize, 0, 1] {
        assert_eq!(
            SeededCarvers::bind_with(
                &s.columns,
                CarverBindBudget::new(90).unwrap(),
                binding_size.wrapping_add_signed(delta)
            )
            .is_ok(),
            delta >= 0
        );
        assert_eq!(
            CarvedChunkBuffer::new_with(buffer_size.wrapping_add_signed(delta)).is_ok(),
            delta >= 0
        );
    }
    assert_eq!(s.columns.retained_bytes(), 10816);
    assert_eq!(s.surface.retained_bytes(), 406_592);
    println!("GEN0511_NATIVE binding={} buffer_value={} buffer_retained={} prepared={} noise={} aquifer={} selector={} rules={} meter={} work={} provider={} staticSIN={}",binding_size,size_of::<CarvedChunkBuffer>(),buffer_size,size_of::<PreparedCarverRun<'_>>(),size_of::<NoiseChunkSession>(),size_of::<CarverAquifer>(),size_of::<BiomeSelectionSession>(),size_of::<SurfaceRuleSession<'_>>(),size_of::<CarverCumulativeMeter>(),size_of::<crate::sample_work::SampleWork>(),size_of::<DirectCarverBiomes<'_>>(),std::mem::size_of_val(&sin::ORIGINAL_SIN_BITS));
}
#[test]
fn all_total_admissions_and_reserved_remaining_are_exact() {
    let s = setup();
    let one = Unsigned192::from_u64(1);
    for n in [
        NOISE_TOTAL.checked_sub(one).unwrap(),
        NOISE_TOTAL,
        NOISE_TOTAL.checked_add(one).unwrap(),
    ] {
        let result = prepare(
            &s,
            n,
            Unsigned192::from_u64(SELECTOR_WORK),
            RULE_WORK,
            RUN_VALUE_CAP,
        );
        assert_eq!(result.is_ok(), n >= NOISE_TOTAL);
        if let Err(error) = result {
            assert_eq!(
                error.kind(),
                &CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Noise,
                    required: NOISE_TOTAL.limbs(),
                    limit: n.limbs(),
                    used: [0; 3]
                }
            );
        }
    }
    for n in [SELECTOR_WORK - 1, SELECTOR_WORK, SELECTOR_WORK + 1] {
        assert_eq!(
            prepare(
                &s,
                NOISE_TOTAL,
                Unsigned192::from_u64(n),
                RULE_WORK,
                RUN_VALUE_CAP
            )
            .is_ok(),
            n >= SELECTOR_WORK
        );
    }
    for n in [RULE_WORK - 1, RULE_WORK, RULE_WORK + 1] {
        assert_eq!(
            prepare(
                &s,
                NOISE_TOTAL,
                Unsigned192::from_u64(SELECTOR_WORK),
                n,
                RUN_VALUE_CAP
            )
            .is_ok(),
            n >= RULE_WORK
        );
    }
    let prepared = run(&s);
    assert!(prepared.noise.carver_meter().used() <= Unsigned192::from_u64(PREPARE_RESERVED));
    assert_eq!(
        NOISE_TOTAL.checked_sub(Unsigned192::from_u64(PREPARE_RESERVED)),
        Some(NOISE_REMAINING)
    );
    let used = prepared.noise.carver_meter().used();
    for n in [
        NOISE_REMAINING.checked_sub(one).unwrap(),
        NOISE_REMAINING,
        NOISE_REMAINING.checked_add(one).unwrap(),
    ] {
        let limit = n.checked_add(used).unwrap();
        assert_eq!(
            admit_allowance(CarverProducer::Noise, NOISE_REMAINING, limit, used).is_ok(),
            n >= NOISE_REMAINING
        );
    }
    for delta in [-1_isize, 0, 1] {
        assert_eq!(
            prepare(
                &s,
                NOISE_TOTAL,
                Unsigned192::from_u64(SELECTOR_WORK),
                RULE_WORK,
                size_of::<PreparedCarverRun<'_>>().wrapping_add_signed(delta)
            )
            .is_ok(),
            delta >= 0
        );
    }
}
#[test]
fn published_output_becomes_empty_and_refusal_preserves_ready_run_and_surface() {
    let s = setup();
    let original = s.surface.carver_contents().unwrap();
    let mut output = CarvedChunkBuffer::new().unwrap();
    for budget in [
        CARVER_CHUNK_WORK - 1,
        CARVER_CHUNK_WORK,
        CARVER_CHUNK_WORK + 1,
    ] {
        let mut invocation = run(&s);
        let mut publisher = run(&s);
        s.binding
            .carve_chunk(
                CarverInput::new(&s.surface, &mut publisher),
                CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                &mut output,
            )
            .unwrap();
        assert!(output.published().is_some());
        let counters = (
            invocation.noise.carver_meter().used(),
            invocation.selector.carver_meter().used(),
            invocation.context.work_used(),
        );
        let result = s.binding.carve_chunk(
            CarverInput::new(&s.surface, &mut invocation),
            CarverWorkBudget::new(budget).unwrap(),
            &mut output,
        );
        assert_eq!(result.is_ok(), budget >= CARVER_CHUNK_WORK);
        if budget < CARVER_CHUNK_WORK {
            assert!(output.published().is_none());
            assert_eq!(invocation.state(), PreparedCarverState::Ready);
            assert_eq!(
                counters,
                (
                    invocation.noise.carver_meter().used(),
                    invocation.selector.carver_meter().used(),
                    invocation.context.work_used()
                )
            );
        } else {
            assert_eq!(invocation.state(), PreparedCarverState::Complete);
            assert!(s
                .binding
                .carve_chunk(
                    CarverInput::new(&s.surface, &mut invocation),
                    CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                    &mut output
                )
                .is_err());
            assert!(output.published().is_none());
        }
        assert_eq!(s.surface.carver_contents().unwrap(), original);
    }
}
#[test]
fn genuine_full_sin_membership_and_all_63291_start_tests() {
    use sha2::{Digest, Sha256};
    let sin_text = include_str!("../../tests/fixtures/carver-runtime-original-1.21.8/sin.tsv");
    let values: Vec<u32> = sin_text
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .map(|v| u32::from_str_radix(v, 16).unwrap())
        .collect();
    assert_eq!(values.len(), 65536);
    assert_eq!(&values[..], &sin::ORIGINAL_SIN_BITS);
    let mut h = Sha256::new();
    for value in values {
        h.update(value.to_le_bytes());
    }
    assert_eq!(
        format!("{:x}", h.finalize()),
        "cdfaec6870788e193dff3f1ddfa3ca7d3f1a897042d78365a1fe3a433be3627d"
    );
    let mut tests = 0;
    for row in include_str!("../../tests/fixtures/carver-runtime-original-1.21.8/groups.tsv")
        .lines()
        .skip(1)
    {
        let r: Vec<_> = row.split('\t').collect();
        let seed = WorldSeed::new(r[2].parse().unwrap());
        let cx: i32 = r[3].parse().unwrap();
        let cz: i32 = r[4].parse().unwrap();
        let mut starts = String::new();
        for dx in -8..=8 {
            for dz in -8..=8 {
                for k in 0..3 {
                    let mut random =
                        geometry::large_feature(seed, k, ChunkPos::new(cx + dx, cz + dz));
                    starts.push(
                        if random.next_float() <= [0.15_f32, 0.07_f32, 0.01_f32][k] {
                            '1'
                        } else {
                            '0'
                        },
                    );
                    tests += 1;
                }
            }
        }
        assert_eq!(starts, r[9], "{}", r[0]);
    }
    assert_eq!(tests, 63291);
}

#[test]
fn segments_and_wide_carries_preserve_the_entire_execution_chronology() {
    let s = setup_at(i64::MAX, ChunkPos::new(0, 0));
    let mut ordinary = run(&s);
    ordinary.trace = Some(Trace::default());
    let mut output = CarvedChunkBuffer::new().unwrap();
    let report = s
        .binding
        .carve_chunk(
            CarverInput::new(&s.surface, &mut ordinary),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut output,
        )
        .unwrap();
    assert!(report.fixups() > 0);
    let expected_states = output.states.clone();
    let expected_fluid = output.fluid.clone();
    let expected_mask = output.mask.clone();
    let heights = output.heights;
    for offset in [
        Unsigned192::from_limbs([u64::MAX - 10, 0, 0]),
        Unsigned192::from_limbs([u64::MAX - 10, u64::MAX, 0]),
    ] {
        let mut shifted = run(&s);
        shifted.trace = Some(Trace::default());
        let before = shifted.noise.carver_meter().used();
        shifted.noise.carver_meter_mut().test_position(
            NOISE_TOTAL.checked_add(offset).unwrap(),
            before.checked_add(offset).unwrap(),
            0,
        );
        shifted.selector.carver_meter_mut().test_position(
            Unsigned192::from_u64(SELECTOR_WORK)
                .checked_add(offset)
                .unwrap(),
            offset,
            0,
        );
        // Every actual charge now crosses a segment, including nested Aquifer,
        // spline/marker and source calls. No producer is replaced or reset.
        shifted.noise.carver_meter_mut().test_short_segments();
        shifted.selector.carver_meter_mut().test_short_segments();
        let actual = s
            .binding
            .carve_chunk(
                CarverInput::new(&s.surface, &mut shifted),
                CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                &mut output,
            )
            .unwrap();
        assert_eq!(&*output.states, &*expected_states);
        assert_eq!(&*output.fluid, &*expected_fluid);
        assert_eq!(&*output.mask, &*expected_mask);
        assert_eq!(output.heights, heights);
        let a = shifted.trace.as_ref().unwrap();
        let e = ordinary.trace.as_ref().unwrap();
        assert_eq!(a.events, e.events);
        assert_eq!(a.queries, e.queries);
        assert_eq!(a.starts, e.starts);
        assert_eq!(a.carves, e.carves);
        assert_eq!(a.writes, e.writes);
        assert_eq!(a.tunnels, e.tunnels);
        assert_eq!(
            Unsigned192::from_limbs(actual.noise_work_used())
                .checked_sub(offset)
                .unwrap()
                .limbs(),
            report.noise_work_used()
        );
        assert_eq!(
            Unsigned192::from_limbs(actual.selector_work_used())
                .checked_sub(offset)
                .unwrap()
                .limbs(),
            report.selector_work_used()
        );
        assert_eq!(
            shifted.aquifer.carver_history(),
            ordinary.aquifer.carver_history()
        );
        assert_eq!(
            shifted.noise.carver_history(),
            ordinary.noise.carver_history()
        );
        assert_eq!(
            shifted.selector.carver_history(),
            ordinary.selector.carver_history()
        );
    }
}
#[test]
fn nested_producer_faults_preserve_all_192_bits() {
    let s = setup();
    for limbs in [[u64::MAX, 0, 0], [u64::MAX, u64::MAX, 0], [u64::MAX; 3]] {
        let n = Unsigned192::from_limbs(limbs);
        let mut session = run(&s);
        session.noise.carver_meter_mut().test_position(n, n, 0);
        let error = session
            .noise
            .sample_carver_climate(CarverClimateQuart::source(session.chunk, 0, 0).unwrap())
            .unwrap_err();
        assert_eq!(
            noise_error(error).kind(),
            &CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Noise,
                required: [1, 0, 0],
                limit: limbs,
                used: limbs
            }
        );
        let mut session = run(&s);
        session.noise.carver_meter_mut().test_position(n, n, 0);
        let error = session
            .aquifer
            .compute(&mut session.noise, BlockPos::new(0, 0, 0))
            .unwrap_err();
        assert_eq!(
            noise_error(error).kind(),
            &CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Noise,
                required: [1, 0, 0],
                limit: limbs,
                used: limbs
            }
        );
        let mut session = run(&s);
        session.selector.carver_meter_mut().test_position(n, n, 0);
        let request = CarverClimateQuart::source(session.chunk, 0, 0).unwrap();
        let climate = session.noise.sample_carver_climate(request).unwrap();
        let error = session
            .selector
            .select_core(crate::biome_selection::BiomeClimateTarget::from_climate(
                climate,
            ))
            .unwrap_err();
        assert_eq!(
            biome_error(error).kind(),
            &CarverErrorKind::ProducerWorkExceeded {
                producer: CarverProducer::Selector,
                required: [1, 0, 0],
                limit: limbs,
                used: limbs
            }
        );
        let mut meter = CarverCumulativeMeter::new(n);
        meter.test_position(n, n, 0);
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
                limit: limbs,
                used: limbs
            }
        );
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One gate preserves exact combined refusal precedence."
)]
fn every_carve_refusal_precedence_starts_published_ends_empty() {
    let mut s = setup();
    let identity = s.surface.published().unwrap();
    let mut output = CarvedChunkBuffer::new().unwrap();
    let settings_other = NoiseSettingsResourceId::from_name("minecraft:amplified").unwrap();
    // Test every individual refusal, then every suffix combination: the first
    // defect wins in the binding/Surface/producer/allowance/Carver order.
    for combined in [false, true] {
        for first in 0..10 {
            s.surface.carver_test_identity(Some(identity));
            let mut publisher = run(&s);
            s.binding
                .carve_chunk(
                    CarverInput::new(&s.surface, &mut publisher),
                    CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                    &mut output,
                )
                .unwrap();
            assert!(output.published().is_some());
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
            let defect = |n| n == first || combined && n > first;
            if defect(0) {
                binding.seed = WorldSeed::new(7);
            }
            if defect(1) {
                binding.resource = settings_other;
            }
            if defect(2) {
                s.surface.carver_test_identity(None);
            } else if defect(3) {
                s.surface
                    .carver_test_identity(Some((identity.0, WorldSeed::new(7), identity.2)));
            } else if defect(4) {
                s.surface
                    .carver_test_identity(Some((identity.0, identity.1, settings_other)));
            } else if defect(5) {
                s.surface
                    .carver_test_identity(Some((ChunkPos::new(1, 0), identity.1, identity.2)));
            }
            if defect(6) {
                invocation.lifecycle = PreparedCarverState::Complete;
            }
            if defect(7) {
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
            if defect(8) {
                invocation.selector.carver_meter_mut().test_position(
                    Unsigned192::from_u64(SELECTOR_WORK - 1),
                    Unsigned192::ZERO,
                    0,
                );
            }
            if defect(9) {
                invocation.context.set_work_used(1);
            }
            let budget = CarverWorkBudget::new(if combined {
                CARVER_CHUNK_WORK - 1
            } else {
                CARVER_CHUNK_WORK
            })
            .unwrap();
            let error = binding
                .carve_chunk(
                    CarverInput::new(&s.surface, &mut invocation),
                    budget,
                    &mut output,
                )
                .unwrap_err();
            let expected = match first {
                0 => CarverErrorKind::InconsistentBinding(BindingMismatch::Seed),
                1 => CarverErrorKind::InconsistentBinding(BindingMismatch::Settings),
                2 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::NotPublished),
                3 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::Seed),
                4 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::Settings),
                5 => CarverErrorKind::InconsistentSurface(SurfaceMismatch::Chunk),
                6 => CarverErrorKind::InconsistentProducer(ProducerMismatch::Lifecycle),
                7 => CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Noise,
                    required: NOISE_REMAINING.limbs(),
                    limit: invocation.noise.carver_meter().limit().limbs(),
                    used: invocation.noise.carver_meter().used().limbs(),
                },
                8 => CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Selector,
                    required: Unsigned192::from_u64(SELECTOR_WORK).limbs(),
                    limit: Unsigned192::from_u64(SELECTOR_WORK - 1).limbs(),
                    used: [0; 3],
                },
                9 => CarverErrorKind::ProducerWorkExceeded {
                    producer: CarverProducer::Surface,
                    required: Unsigned192::from_u64(u64::from(RULE_WORK)).limbs(),
                    limit: Unsigned192::from_u64(u64::from(RULE_WORK - 1)).limbs(),
                    used: [0; 3],
                },
                _ => unreachable!(),
            };
            assert_eq!(error.kind(), &expected, "combined={combined} first={first}");
            assert!(output.published().is_none());
        }
    }
    s.surface.carver_test_identity(Some(identity));
    let mut publisher = run(&s);
    s.binding
        .carve_chunk(
            CarverInput::new(&s.surface, &mut publisher),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut output,
        )
        .unwrap();
    let mut invocation = run(&s);
    let mut corrupted =
        SeededCarvers::bind(&s.columns, CarverBindBudget::new(90).unwrap()).unwrap();
    corrupted.len = 0;
    assert!(output.published().is_some());
    let error = corrupted
        .carve_chunk(
            CarverInput::new(&s.surface, &mut invocation),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut output,
        )
        .unwrap_err();
    assert_eq!(error.kind(), &CarverErrorKind::UnclassifiedState);
    assert_eq!(invocation.state(), PreparedCarverState::Poisoned);
    assert!(output.published().is_none());
}

#[test]
fn every_original_scripted_float_boundary_uses_the_production_arithmetic() {
    let max = f32::from_bits(0x3f7f_ffff);
    let mut rows = 0;
    for line in include_str!("../../tests/fixtures/carver-runtime-original-1.21.8/boundary.tsv")
        .lines()
        .skip(1)
    {
        let r: Vec<_> = line.split('\t').collect();
        let draw = if r[1].contains("max") || r[1].contains("3f7fffff") {
            max
        } else {
            0.0
        };
        let expected = u32::from_str_radix(r[2].split('/').nth(1).unwrap(), 16).unwrap();
        let value = if r[0].starts_with("UniformFloat") {
            let spec = r[0]
                .strip_prefix("UniformFloat.of(")
                .unwrap()
                .split(')')
                .next()
                .unwrap();
            let bounds: Vec<_> = spec
                .split(',')
                .map(|v| {
                    f32::from_bits(u32::from_str_radix(v.split('/').nth(1).unwrap(), 16).unwrap())
                })
                .collect();
            geometry::uniform_draw(draw, bounds[0], bounds[1])
        } else if r[0].starts_with("TrapezoidFloat") {
            geometry::trapezoid_draw(draw, draw)
        } else if r[0].starts_with("CaveWorldCarver") {
            let base = geometry::thickness_base(draw, draw);
            if r[1].contains("nextInt(10)=0") {
                geometry::thickness_expand(base, draw, draw)
            } else {
                base
            }
        } else if r[0].starts_with("Mth.sin") {
            sin::sin(std::f32::consts::FRAC_PI_2)
        } else {
            geometry::uniform_draw(draw, 0.75, 1.0)
        };
        assert_eq!(value.to_bits(), expected, "{} {}", r[0], r[1]);
        rows += 1;
    }
    assert_eq!(rows, 20);
    assert_eq!(
        (112.0_f32 * geometry::uniform_draw(max, 0.75, 1.0)) as i32,
        112
    );
    assert_eq!(max * 0.5_f32 + 0.5_f32, 1.0); // labelled child upper-end control
}

#[test]
fn original_lower_quart_and_history_tie_controls_remain_distinct() {
    use crate::biome_selection::BiomeClimateTarget;
    let s = setup();
    let mut forward = run(&s);
    let mut lower = 0;
    let mut histories = 0;
    for line in
        include_str!("../../tests/fixtures/carver-runtime-original-1.21.8/controlled-edges.tsv")
            .lines()
            .skip(1)
    {
        let r: Vec<_> = line.split('\t').collect();
        if r[0] == "original_zoom_below_stored_y" {
            let block: Vec<i32> = r[1]
                .split("block=")
                .nth(1)
                .unwrap()
                .split(',')
                .map(|v| v.parse().unwrap())
                .collect();
            let pos = BlockPos::new(block[0], block[1], block[2]);
            let quart = forward.zoom.select_quart(pos);
            assert_eq!(format!("{},{},{}", quart.x(), quart.y(), quart.z()), r[2]);
            // Request uses the actual selected quart, without stored clamping.
            let request =
                super::biomes::CarverClimateQuart::fixup(ChunkPos::new(0, 0), pos, quart).unwrap();
            assert_eq!(request.origin().y(), -68);
            forward.noise.sample_carver_climate(request).unwrap();
            lower += 1;
        } else {
            let t: Vec<i64> = r[1]
                .split("target=")
                .nth(1)
                .unwrap()
                .split(',')
                .map(|v| v.parse().unwrap())
                .collect();
            let target = BiomeClimateTarget::from_quantized(t.try_into().unwrap());
            let selection = if r[1].starts_with("cold;") {
                run(&s).selector.select_core(target).unwrap()
            } else {
                forward.selector.select_core(target).unwrap()
            };
            assert_eq!(selection.biome().resource_location(), r[2]);
            histories += 1;
        }
    }
    assert_eq!((lower, histories), (60, 24));
}

#[test]
fn direct_provider_is_lazy_and_temperature_shares_the_biome_memo() {
    use crate::surface_rules::{
        SurfaceBlockInput, SurfaceColumnPos, SurfaceRuleOutcome, WorldSurfaceHeights,
    };
    for control in 0..=2 {
        let mut s = setup();
        s.rules.carver_test_rule(control);
        let mut invocation = run(&s);
        let mut queries = Vec::new();
        let mut work = GeometryWork {
            limit: CARVER_CHUNK_WORK,
            used: 0,
        };
        let before = (
            invocation.noise.carver_meter().used(),
            invocation.selector.carver_meter().used(),
        );
        invocation
            .context
            .begin_column(SurfaceColumnPos::from_block(BlockPos::new(0, 0, 0)))
            .unwrap();
        let mut provider = DirectCarverBiomes {
            center: invocation.chunk,
            noise: &mut invocation.noise,
            selector: &mut invocation.selector,
            zoom: invocation.zoom,
            work: &mut work,
            queries: Some(&mut queries),
        };
        let outcome = invocation
            .context
            .evaluate_carver(
                SurfaceBlockInput::new(0, 1, 1, None),
                &WorldSurfaceHeights::new([[1; 16]; 16]),
                &mut provider,
            )
            .unwrap_or_else(|_| panic!("controlled rule refused"));
        if control == 1 {
            assert!(matches!(outcome, SurfaceRuleOutcome::State(_)));
        } else {
            assert_eq!(outcome, SurfaceRuleOutcome::NoRule);
        }
        assert_eq!(queries.len(), usize::from(control == 2));
        assert_eq!(work.used, if control == 2 { 74 } else { 0 });
        if control < 2 {
            assert_eq!(
                before,
                (
                    invocation.noise.carver_meter().used(),
                    invocation.selector.carver_meter().used()
                )
            );
        } else {
            assert!(invocation.noise.carver_meter().used() > before.0);
            assert!(invocation.selector.carver_meter().used() > before.1);
        }
    }
}

mod allocation;
mod core_faults;
mod precedence;

mod precharge;
