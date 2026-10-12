#![expect(
    clippy::cast_possible_wrap,
    reason = "Synthetic positions are bounded by the 2048-column cache cap."
)]
use super::wrappers::{original, probe};
use super::*;
use crate::density::chunk::fill::Provider;
use crate::density::graph::NodeKind;
use crate::density::DensitySampleErrorKind;

fn exhausted(error: &NoiseChunkError, units: usize) {
    match error.kind() {
        NoiseChunkErrorKind::WorkExceeded { limit, used } => {
            assert_eq!((*limit, *used), (units, units));
        }
        NoiseChunkErrorKind::Density(e) => assert!(
            matches!(e.kind(),DensitySampleErrorKind::WorkExceeded {limit,used} if (*limit,*used)==(units,units))
        ),
        other => panic!("expected metered refusal, got {other:?}"),
    }
}
fn fingerprint(s: &NoiseChunkSession) -> Vec<u64> {
    let mut out = vec![
        s.work_used() as u64,
        s.state.scalar_counter as u64,
        s.state.array_counter as u64,
        s.state.array_index as u64,
        s.state.lifecycle as u64,
        s.state.driver_phase as u64,
        s.state.driver_x as u64,
        u64::from(s.state.interpolating),
        u64::from(s.state.filling_cell),
        s.state.start.x() as u64,
        s.state.start.y() as u64,
        s.state.start.z() as u64,
        s.state.in_x as u64,
        s.state.in_y as u64,
        s.state.in_z as u64,
        s.surface.len as u64,
    ];
    out.extend(s.surface.keys[..s.surface.len].iter().copied());
    out.extend(s.surface.values[..s.surface.len].iter().map(|&v| v as u64));
    for wrapper in &s.state.wrappers {
        match &wrapper.cache {
            Cache::TwoD { key, value } => out.extend([*key, value.to_bits()]),
            Cache::Once {
                counter,
                array_counter,
                value,
                array,
                valid_len,
            } => {
                out.extend([
                    *counter as u64,
                    *array_counter as u64,
                    value.to_bits(),
                    valid_len.map_or(u64::MAX, |n| n as u64),
                ]);
                out.extend(array.iter().map(|v| v.to_bits()));
            }
            Cache::Interpolated(v) => {
                for a in [
                    &v.slice0[..],
                    &v.slice1[..],
                    &v.corners[..],
                    &v.y_values[..],
                    &v.x_values[..],
                ] {
                    out.extend(a.iter().map(|v| v.to_bits()));
                }
                out.push(v.value.to_bits());
            }
            Cache::Flat(v) | Cache::Cell(v) => out.extend(v.iter().map(|v| v.to_bits())),
        }
    }
    out
}

#[test]
fn budgets_and_chunk_admission_are_closed_and_mutation_free() {
    for units in [0, MAX_NOISE_CHUNK_WORK + 1, u32::MAX] {
        assert!(matches!(
            NoiseChunkWorkBudget::new(units).unwrap_err().kind(),
            NoiseChunkErrorKind::InvalidBudget { .. }
        ));
    }
    for units in [1, MAX_NOISE_CHUNK_WORK] {
        assert_eq!(NoiseChunkWorkBudget::new(units).unwrap().units(), units);
    }
    for chunk in [
        ChunkPos::new(-1_875_000, -1_875_000),
        ChunkPos::new(1_874_999, 1_874_999),
    ] {
        admit_chunk(chunk).unwrap();
    }
    for chunk in [
        ChunkPos::new(-1_875_001, 0),
        ChunkPos::new(1_875_000, 0),
        ChunkPos::new(0, i32::MIN),
        ChunkPos::new(0, i32::MAX),
    ] {
        assert!(matches!(
            admit_chunk(chunk).unwrap_err().kind(),
            NoiseChunkErrorKind::ChunkOutOfRange { .. }
        ));
    }
    let mut s = router("minecraft:nether", 7)
        .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
        .unwrap();
    let before = fingerprint(&s);
    assert!(s
        .preliminary_surface_level(BlockPos::new(30_000_001, 0, 0))
        .is_err());
    assert_eq!(fingerprint(&s), before);
    assert!(s
        .sample_climate(ClimateQuartPos::new(0, 0, 0).unwrap())
        .is_ok());
}

#[test]
fn construction_exact_budget_and_failed_reset_preserve_old_candidate() {
    let r = router("minecraft:nether", 0);
    let n = r
        .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
        .unwrap()
        .work_used();
    let mut s = r
        .fresh_paper_noise_chunk(
            ChunkPos::ORIGIN,
            NoiseChunkWorkBudget::new(n as u32).unwrap(),
        )
        .unwrap();
    assert_eq!(s.work_used(), n);
    let error = r
        .fresh_paper_noise_chunk(
            ChunkPos::ORIGIN,
            NoiseChunkWorkBudget::new(n as u32 - 1).unwrap(),
        )
        .err()
        .unwrap();
    exhausted(&error, n - 1);
    let before = fingerprint(&s);
    exhausted(
        &s.reset(NoiseChunkWorkBudget::new(n as u32 - 1).unwrap())
            .unwrap_err(),
        n - 1,
    );
    assert_eq!(fingerprint(&s), before);
    s.reset(budget()).unwrap();
    s.preliminary_surface_level(BlockPos::new(18, 123, -18))
        .unwrap();
    s.sample_climate(ClimateQuartPos::new(-3, 12, 4).unwrap())
        .unwrap();
    let values = s.sample_chunk_densities().unwrap();
    let before = fingerprint(&s);
    assert!(s.sample_chunk_densities().is_err());
    assert_eq!(fingerprint(&s), before);
    exhausted(
        &s.reset(NoiseChunkWorkBudget::new(n as u32 - 1).unwrap())
            .unwrap_err(),
        n - 1,
    );
    assert_eq!(fingerprint(&s), before);
    s.reset(budget()).unwrap();
    let after = s.sample_chunk_densities().unwrap();
    assert!(values
        .values()
        .iter()
        .zip(after.values())
        .all(|(a, b)| a.to_bits() == b.to_bits()));
}

#[derive(Clone, Copy)]
enum Operation {
    Climate,
    Surface,
    Slice,
    Cell,
    Driver,
}
fn run(s: &mut NoiseChunkSession, op: Operation) -> Result<(), NoiseChunkError> {
    match op {
        Operation::Climate => {
            s.sample_climate(ClimateQuartPos::new(-31, 1, 47).unwrap())?;
        }
        Operation::Surface => {
            s.preliminary_surface_level(BlockPos::new(-101, 8, 155))?;
        }
        Operation::Slice => {
            s.state.lifecycle = Lifecycle::Running;
            s.initialize_driver()?;
            s.fill_slice(true, 0)?;
        }
        Operation::Cell => {
            s.state.interpolating = true;
            s.state.filling_cell = true;
            let root = s.state.wrappers[s.state.cells[0]].input;
            let mut values = vec![0.0; s.state.cell_volume()];
            s.state.fill(
                &s.data,
                &s.view,
                root,
                &mut values,
                Provider::Cell,
                &mut s.work,
            )?;
        }
        Operation::Driver => {
            s.sample_chunk_densities()?;
        }
    }
    Ok(())
}

#[test]
fn one_shared_meter_refuses_exact_next_operation_without_partial_output() {
    let r = router("minecraft:overworld", 0);
    for op in [
        Operation::Climate,
        Operation::Surface,
        Operation::Slice,
        Operation::Cell,
        Operation::Driver,
    ] {
        let mut measured = r
            .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
            .unwrap();
        run(&mut measured, op).unwrap();
        let n = measured.work_used();
        let mut exact = r
            .fresh_paper_noise_chunk(
                ChunkPos::ORIGIN,
                NoiseChunkWorkBudget::new(n as u32).unwrap(),
            )
            .unwrap();
        run(&mut exact, op).unwrap();
        assert_eq!(exact.work_used(), n);
        let mut short = r
            .fresh_paper_noise_chunk(
                ChunkPos::ORIGIN,
                NoiseChunkWorkBudget::new(n as u32 - 1).unwrap(),
            )
            .unwrap();
        exhausted(&run(&mut short, op).unwrap_err(), n - 1);
        if matches!(
            op,
            Operation::Climate | Operation::Surface | Operation::Driver
        ) {
            assert_eq!(short.state.lifecycle, Lifecycle::Poisoned);
            let old = fingerprint(&short);
            assert!(short
                .sample_climate(ClimateQuartPos::new(0, 0, 0).unwrap())
                .is_err());
            assert!(short
                .preliminary_surface_level(BlockPos::new(0, 0, 0))
                .is_err());
            assert!(short.sample_chunk_densities().is_err());
            assert_eq!(fingerprint(&short), old);
            short.reset(budget()).unwrap();
            assert_eq!(short.state.lifecycle, Lifecycle::Fresh);
        }
    }
}

#[test]
fn surface_threshold_sentinel_capacity_hits_and_failure_publication() {
    let threshold = 0.390_625_f64;
    for value in [
        f64::from_bits(threshold.to_bits() - 1),
        threshold,
        f64::from_bits(threshold.to_bits() + 1),
    ] {
        let mut s = probe(vec![(original(NodeKind::Constant(value)), "initial")], 0);
        let level = s
            .preliminary_surface_level(BlockPos::new(-1, i32::MAX, -1))
            .unwrap();
        assert_eq!(level, if value > threshold { 320 } else { i32::MAX });
        assert_eq!(s.surface.len, 1);
        let before = s.work_used();
        assert_eq!(
            s.preliminary_surface_level(BlockPos::new(-4, i32::MIN, -4))
                .unwrap(),
            level
        );
        assert_eq!(s.work_used(), before + 1);
    }
    let mut s = probe(vec![(original(NodeKind::Constant(1.0)), "initial")], 0);
    for i in 0..MAX_SURFACE_COLUMNS {
        s.preliminary_surface_level(BlockPos::new(i as i32 * 4, 0, 0))
            .unwrap();
    }
    assert_eq!(s.surface.len, MAX_SURFACE_COLUMNS);
    assert_eq!(
        s.preliminary_surface_level(BlockPos::new(0, 0, 0)).unwrap(),
        320
    );
    assert!(matches!(
        s.preliminary_surface_level(BlockPos::new(MAX_SURFACE_COLUMNS as i32 * 4, 0, 0))
            .unwrap_err()
            .kind(),
        NoiseChunkErrorKind::Limit {
            limit: NoiseChunkLimit::SurfaceColumns,
            ..
        }
    ));
    assert_eq!(s.surface.len, MAX_SURFACE_COLUMNS);
    assert_eq!(s.state.lifecycle, Lifecycle::Poisoned);
    let mut s = probe(vec![(original(NodeKind::Constant(f64::NAN)), "initial")], 0);
    assert!(s.preliminary_surface_level(BlockPos::new(0, 0, 0)).is_err());
    assert_eq!(s.surface.len, 0);
    assert_eq!(s.state.lifecycle, Lifecycle::Poisoned);
}

#[test]
fn climate_float_overflow_reports_original_axis_and_poison() {
    let mut s = probe(
        vec![(original(NodeKind::Constant(f64::MAX)), "finite-double")],
        0,
    );
    let error = s
        .sample_climate(ClimateQuartPos::new(1, 2, 3).unwrap())
        .unwrap_err();
    let NoiseChunkErrorKind::Density(error) = error.kind() else {
        panic!("{error:?}")
    };
    assert_eq!(error.field(), "temperature");
    assert_eq!(error.position(), Some(BlockPos::new(4, 8, 12)));
    assert!(
        matches!(error.kind(),DensitySampleErrorKind::NonFiniteCoordinate {bits} if *bits==f32::INFINITY.to_bits())
    );
    assert_eq!(s.state.lifecycle, Lifecycle::Poisoned);
}

#[test]
fn aggregate_byte_admission_checks_exact_boundary_and_overflow() {
    for (kind, max) in [
        (NoiseChunkLimit::CacheBytes, 8 * 1024 * 1024),
        (NoiseChunkLimit::ScratchBytes, 4 * 1024 * 1024),
        (NoiseChunkLimit::GraphBytes, 2 * 1024 * 1024),
        (NoiseChunkLimit::KeyBytes, 2 * 1024 * 1024),
        (NoiseChunkLimit::ColdBytes, 4 * 1024 * 1024),
        (NoiseChunkLimit::Output, 1024 * 1024),
    ] {
        assert_eq!(bytes(kind, 0, max / 8, 8, max).unwrap(), max);
        assert!(
            matches!(bytes(kind,1,max/8,8,max).unwrap_err().kind(),NoiseChunkErrorKind::Limit {limit,count,maximum} if *limit==kind&&*count==max+1&&*maximum==max)
        );
        assert!(
            matches!(bytes(kind,0,usize::MAX,8,max).unwrap_err().kind(),NoiseChunkErrorKind::Limit {count,..} if *count==usize::MAX)
        );
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The successive driver phases verify rejection atomicity before and after valid transitions."
)]
fn private_driver_phase_admission_precedes_every_mutation() {
    use crate::density::chunk::state::DriverPhase;
    type Request = fn(&mut NoiseChunkSession) -> Result<(), NoiseChunkError>;
    let requests: [Request; 8] = [
        |s| s.initialize_driver().map_err(Into::into),
        |s| s.fill_slice(true, 0).map_err(Into::into),
        |s| s.fill_slice(false, 1).map_err(Into::into),
        |s| s.select_cell(0, 0).map_err(Into::into),
        |s| s.update_y(0.5).map_err(Into::into),
        |s| s.update_x(0.5).map_err(Into::into),
        |s| s.update_z(0.5).map_err(Into::into),
        |s| s.swap_slices().map_err(Into::into),
    ];
    let mut s = router("minecraft:nether", 0)
        .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
        .unwrap();
    for lifecycle in [Lifecycle::Fresh, Lifecycle::Complete, Lifecycle::Poisoned] {
        s.state.lifecycle = lifecycle;
        for request in requests {
            let before = fingerprint(&s);
            assert!(matches!(
                request(&mut s).unwrap_err().kind(),
                NoiseChunkErrorKind::Lifecycle { .. }
            ));
            assert_eq!(before, fingerprint(&s));
        }
    }
    s.state.lifecycle = Lifecycle::Running;
    s.initialize_driver().unwrap();
    for request in [
        requests[0],
        requests[2],
        requests[3],
        requests[4],
        requests[5],
        requests[6],
        requests[7],
    ] {
        let before = fingerprint(&s);
        assert!(matches!(
            request(&mut s).unwrap_err().kind(),
            NoiseChunkErrorKind::Lifecycle { .. }
        ));
        assert_eq!(before, fingerprint(&s));
    }
    for x in [i32::MIN, i32::MAX, 1] {
        let before = fingerprint(&s);
        assert!(s.fill_slice(true, x).is_err());
        assert_eq!(before, fingerprint(&s));
    }
    s.fill_slice(true, 0).unwrap();
    let before = fingerprint(&s);
    assert!(s.select_cell(0, 0).is_err());
    assert_eq!(before, fingerprint(&s));
    s.fill_slice(false, 1).unwrap();
    for request in [requests[4], requests[5], requests[6], requests[7]] {
        let before = fingerprint(&s);
        assert!(request(&mut s).is_err());
        assert_eq!(before, fingerprint(&s));
    }
    for (y, z) in [(s.state.count_y, 0), (0, s.state.count_xz)] {
        let before = fingerprint(&s);
        assert!(s.select_cell(y, z).is_err());
        assert_eq!(before, fingerprint(&s));
    }
    s.state.start = BlockPos::new(0, s.state.start.y(), s.state.start.z());
    s.select_cell(0, 0).unwrap();
    let before = fingerprint(&s);
    assert!(s.update_x(0.5).is_err());
    assert!(s.update_z(0.5).is_err());
    assert_eq!(before, fingerprint(&s));
    for delta in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let before = fingerprint(&s);
        assert!(s.update_y(delta).is_err());
        assert_eq!(before, fingerprint(&s));
    }
    s.update_y(0.5).unwrap();
    s.update_x(0.5).unwrap();
    s.update_z(0.5).unwrap();
    s.swap_slices().unwrap();
    assert_eq!(s.state.driver_phase, DriverPhase::NeedAdvance);
    let before = fingerprint(&s);
    assert!(matches!(
        s.sample_material().unwrap_err().kind(),
        NoiseChunkErrorKind::Lifecycle { .. }
    ));
    assert_eq!(before, fingerprint(&s));
    for request in [
        requests[3],
        requests[4],
        requests[5],
        requests[6],
        requests[7],
    ] {
        let before = fingerprint(&s);
        assert!(request(&mut s).is_err());
        assert_eq!(before, fingerprint(&s));
    }
    let before = fingerprint(&s);
    assert!(s.stop_driver().is_err());
    assert_eq!(before, fingerprint(&s));
    for provider in [Provider::Slice, Provider::Cell] {
        let before = fingerprint(&s);
        assert!(s
            .state
            .for_index(provider, usize::MAX, &mut s.work)
            .is_err());
        assert_eq!(before, fingerprint(&s));
    }
}

#[test]
fn private_direct_shape_root_and_foreign_caps_reject_before_index_reset() {
    let mut s = probe(vec![(original(NodeKind::Constant(7.0)), "constant")], 0);
    s.state.array_index = 17;
    for provider in [Provider::Cell, Provider::Slice, Provider::Foreign(129)] {
        let before = fingerprint(&s);
        let mut values = [-99.0];
        assert!(s
            .state
            .fill_direct(&s.data, &s.view, 0, &mut values, provider, &mut s.work)
            .is_err());
        assert_eq!(fingerprint(&s), before);
        assert_eq!(values, [-99.0]);
    }
    let before = fingerprint(&s);
    let mut values = [-99.0];
    assert!(s
        .state
        .fill_direct(
            &s.data,
            &s.view,
            usize::MAX,
            &mut values,
            Provider::Foreign(1),
            &mut s.work
        )
        .is_err());
    assert_eq!(fingerprint(&s), before);
    assert!(s
        .state
        .for_index(Provider::Foreign(129), 0, &mut s.work)
        .is_err());
    assert_eq!(fingerprint(&s), before);
}
