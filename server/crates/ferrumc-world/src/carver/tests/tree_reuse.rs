//! Immutable binding reuse against genuine Surface/Carver output and refusals.
use super::*;
use crate::biome_selection::{BiomeClimateTarget, BiomeWorkBudget, MAX_BIOME_SELECTION_WORK};
use crate::density::RouterTemplate;

fn tree(name: &str) -> Arc<BiomeTree> {
    Arc::new(
        BiomeTree::from_builtin(
            BiomeParametersResourceId::from_name(name).unwrap(),
            BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
        )
        .unwrap(),
    )
}
fn source() -> BiomeSourceIdentity {
    BiomeSourceIdentity::MultiNoise(
        BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
    )
}
fn history(run: &PreparedCarverRun<'_>) -> (String, String, (Option<u16>, bool), [u64; 3], u32) {
    (
        run.noise.carver_history(),
        run.aquifer.carver_history(),
        run.selector.carver_history(),
        run.selector.carver_meter().used().limbs(),
        run.context.work_used(),
    )
}

#[test]
fn reused_tree_keeps_runs_cold_and_matches_actual_carver_output_report_and_history() {
    let s = super::tests::setup();
    let shared = tree("minecraft:overworld");
    // A different source session may have a warm history on this very tree.
    let mut warm = BiomeSelectionSession::new(
        Arc::clone(&shared),
        BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
    );
    warm.select(BiomeClimateTarget::from_quantized([0; 6]))
        .unwrap();
    assert!(warm.last_selection().is_some());
    let warm_history = warm.last_selection();
    let warm_work = warm.work_used();

    let mut legacy = super::tests::run(&s);
    let cold = history(&legacy);
    let cold_noise = legacy.noise.carver_meter().used();
    assert_eq!(legacy.selector.carver_native_tree(), (1_181_618, 1));
    legacy.trace = Some(Trace::default());
    let mut expected = CarvedChunkBuffer::new().unwrap();
    let report = s
        .binding
        .carve_chunk(
            CarverInput::new(&s.surface, &mut legacy),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut expected,
        )
        .unwrap();
    let surface = s.surface.carver_contents().unwrap();
    for _ in 0..2 {
        let mut reused = s
            .binding
            .prepare_run_with_tree(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                ChunkPos::ORIGIN,
                s.corners,
                &shared,
            )
            .unwrap();
        assert_eq!(reused.state(), PreparedCarverState::Ready);
        assert_eq!(history(&reused), cold);
        assert_eq!(reused.selector.carver_native_tree(), (1_181_618, 3));
        assert_eq!(reused.noise.carver_meter().used(), cold_noise);
        reused.trace = Some(Trace::default());
        let mut actual = CarvedChunkBuffer::new().unwrap();
        let actual_report = s
            .binding
            .carve_chunk(
                CarverInput::new(&s.surface, &mut reused),
                CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                &mut actual,
            )
            .unwrap();
        assert_eq!(actual_report, report);
        assert_eq!(actual.published(), expected.published());
        assert_eq!(actual.states, expected.states);
        assert_eq!(actual.fluid, expected.fluid);
        assert_eq!(actual.mask, expected.mask);
        assert_eq!(actual.heights, expected.heights);
        assert_eq!(history(&reused), history(&legacy));
        let a = reused.trace.as_ref().unwrap();
        let e = legacy.trace.as_ref().unwrap();
        assert_eq!(a.events, e.events);
        assert_eq!(a.queries, e.queries);
        assert_eq!(a.starts, e.starts);
        assert_eq!(a.carves, e.carves);
        assert_eq!(a.writes, e.writes);
        assert_eq!(a.shadow, e.shadow);
        assert_eq!(
            a.tunnels.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            e.tunnels.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(reused.state(), PreparedCarverState::Complete);
        let complete = history(&reused);
        assert_eq!(
            s.binding
                .carve_chunk(
                    CarverInput::new(&s.surface, &mut reused),
                    CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                    &mut actual,
                )
                .unwrap_err()
                .kind(),
            &CarverErrorKind::InconsistentProducer(ProducerMismatch::Lifecycle),
        );
        assert!(actual.published().is_none());
        assert_eq!(history(&reused), complete);
    }
    assert_eq!(Arc::strong_count(&shared), 2);
    assert_eq!(warm.last_selection(), warm_history);
    assert_eq!(warm.work_used(), warm_work);
    assert_eq!(s.surface.carver_contents().unwrap(), surface);
}

#[test]
fn wrong_tree_refuses_only_after_existing_preparation_admissions_without_retaining_it() {
    let s = super::tests::setup();
    let wrong = tree("minecraft:nether");
    let other = NoiseSettingsResourceId::from_name("minecraft:amplified").unwrap();
    for first in 0..12 {
        let mut binding =
            SeededCarvers::bind(&s.columns, CarverBindBudget::new(90).unwrap()).unwrap();
        if first == 0 {
            binding.seed = WorldSeed::new(7);
        }
        if first == 1 {
            binding.resource = other;
        }
        let mut router = RouterTemplate::compile_builtin(if first == 3 {
            other
        } else {
            s.binding.resource()
        })
        .unwrap()
        .bind_seed(WorldSeed::new(if first == 2 { 7 } else { 0 }))
        .unwrap();
        if first == 5 {
            router.carver_test_router_geometry();
        }
        let source = if first == 4 {
            BiomeSourceIdentity::End
        } else {
            source()
        };
        let chunk = if first == 6 {
            ChunkPos::new(1_875_000, 0)
        } else {
            ChunkPos::ORIGIN
        };
        let cap = if first == 7 { 0 } else { RUN_VALUE_CAP };
        let noise = if first == 8 {
            Unsigned192::ZERO
        } else {
            NOISE_TOTAL
        };
        let selector = Unsigned192::from_u64(if first == 9 { 0 } else { SELECTOR_WORK });
        let rules = if first == 10 { 1 } else { RULE_WORK };
        let reused = binding
            .prepare_with_tree(
                &router,
                &s.rules,
                &s.temperatures,
                source,
                chunk,
                s.corners,
                noise,
                selector,
                rules,
                cap,
                Some(&wrong),
            )
            .err()
            .unwrap();
        if first < 11 {
            let legacy = binding
                .prepare_with(
                    &router,
                    &s.rules,
                    &s.temperatures,
                    source,
                    chunk,
                    s.corners,
                    noise,
                    selector,
                    rules,
                    cap,
                )
                .err()
                .unwrap();
            assert_eq!(reused.kind(), legacy.kind(), "first={first}");
        } else {
            assert_eq!(
                reused.kind(),
                &CarverErrorKind::InconsistentProducer(ProducerMismatch::Source)
            );
        }
        assert_eq!(Arc::strong_count(&wrong), 1);
    }
    assert_eq!(
        s.binding
            .prepare_run_with_tree(
                &s.router,
                &s.rules,
                &s.temperatures,
                source(),
                ChunkPos::ORIGIN,
                s.corners,
                &wrong,
            )
            .err()
            .unwrap()
            .kind(),
        &CarverErrorKind::InconsistentProducer(ProducerMismatch::Source)
    );
}
