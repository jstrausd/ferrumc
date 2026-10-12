//! Source-based synthetic branch controls, separate from the full-world oracle.
use super::super::tests::{run, setup};
use super::*;
use crate::carver::CARVER_CHUNK_WORK;
fn state(name: &str) -> BlockStateId {
    BlockStateId::new(ferrumc_registry::block_state::block_default_state(name).unwrap())
}
fn engine<'a, 'b>(
    binding: &'a SeededCarvers,
    run: &'a mut PreparedCarverRun<'b>,
    output: &'a mut CarvedChunkBuffer,
) -> Engine<'a, 'b> {
    Engine {
        binding,
        run,
        output,
        work: GeometryWork {
            limit: CARVER_CHUNK_WORK,
            used: 0,
        },
        report: CarverReport::default(),
    }
}
fn sphere(e: &mut Engine<'_, '_>, p: BlockPos) {
    e.ellipsoid(
        f64::from(p.x()) + 0.5,
        f64::from(p.y()) - 0.5,
        f64::from(p.z()) + 0.5,
        0.49,
        0.49,
        Skip::Cave(-1.0),
    )
    .unwrap();
}
fn set(out: &mut CarvedChunkBuffer, p: BlockPos, s: BlockStateId) {
    out.states[state_index(p).unwrap()] = s;
}
#[test]
fn mask_precedes_barrier_write_and_repeat_skips_the_aquifer() {
    let s = setup();
    let mut invocation = run(&s);
    let mut out = CarvedChunkBuffer::new().unwrap();
    out.states.fill(state("stone"));
    let line = include_str!("../../../tests/fixtures/carver-runtime-original-1.21.8/trace.tsv")
        .lines()
        .nth(1)
        .unwrap();
    let token = line
        .split('\t')
        .nth(1)
        .unwrap()
        .split(';')
        .find(|v| v.starts_with('A') && v.split(',').nth(3) == Some("-1"))
        .unwrap();
    let xyz: Vec<i32> = token[1..]
        .split(',')
        .take(3)
        .map(|v| v.parse().unwrap())
        .collect();
    let p = BlockPos::new(xyz[0], xyz[1], xyz[2]);
    let mut e = engine(&s.binding, &mut invocation, &mut out);
    sphere(&mut e, p);
    assert_eq!(
        (e.report.mask_bits, e.report.aquifer_calls, e.report.writes),
        (1, 1, 0)
    );
    assert_eq!(e.state(p).unwrap(), state("stone"));
    let before = (e.run.noise.carver_history(), e.run.aquifer.carver_history());
    sphere(&mut e, p);
    assert_eq!(
        (e.report.mask_bits, e.report.aquifer_calls, e.report.writes),
        (1, 1, 0)
    );
    assert_eq!(
        before,
        (e.run.noise.carver_history(), e.run.aquifer.carver_history())
    );
}
#[test]
fn mycelium_norule_fluid_fixup_and_stale_lava_schedule() {
    for control in [0, 3] {
        let mut s = setup();
        s.rules.carver_test_rule(control);
        let mut invocation = run(&s);
        let mut out = CarvedChunkBuffer::new().unwrap();
        out.states.fill(state("stone"));
        let p = BlockPos::new(0, -56, 0);
        let below = BlockPos::new(0, -57, 0);
        set(&mut out, p, state("mycelium"));
        set(&mut out, below, state("dirt"));
        let mut e = engine(&s.binding, &mut invocation, &mut out);
        let mut reached = false;
        e.block(p, &mut reached).unwrap();
        assert!(reached);
        assert_eq!(e.report.fixups, 1);
        assert_eq!(e.report.aquifer_calls, 0);
        assert_eq!(e.run.selector.carver_history(), (None, false));
        assert_eq!(
            e.state(below).unwrap(),
            state(if control == 0 { "dirt" } else { "water" })
        );
        assert_eq!(
            e.output.fluid[state_index(below).unwrap() / 64]
                & (1 << (state_index(below).unwrap() % 64))
                != 0,
            control == 3
        );
    }
    let s = setup();
    let mut invocation = run(&s);
    let mut out = CarvedChunkBuffer::new().unwrap();
    out.states.fill(state("stone"));
    // Establish a true schedule with a genuine Aquifer call; the Lava-level
    // branch must retain it and mark its fluid without another Aquifer call.
    let mut found = false;
    for y in (-55..=63).rev() {
        if invocation
            .aquifer
            .compute(&mut invocation.noise, BlockPos::new(0, y, 0))
            .unwrap()
            .is_some()
            && invocation.aquifer.schedule()
        {
            found = true;
            break;
        }
    }
    assert!(found);
    let p = BlockPos::new(0, -56, 0);
    let mut e = engine(&s.binding, &mut invocation, &mut out);
    let before = e.run.noise.carver_history();
    e.block(p, &mut false).unwrap();
    assert_eq!(e.report.aquifer_calls, 0);
    assert!(e.run.aquifer.schedule());
    assert_eq!(before, e.run.noise.carver_history());
    assert_eq!(e.state(p).unwrap(), state("lava"));
    assert_ne!(
        e.output.fluid[state_index(p).unwrap() / 64] & (1 << (state_index(p).unwrap() % 64)),
        0
    );
    e.block(BlockPos::new(0, -55, 0), &mut false).unwrap();
    assert_eq!(e.report.aquifer_calls, 1);
}
#[test]
fn original_vertical_edges_and_live_air_tip_rescan() {
    let s = setup();
    let mut invocation = run(&s);
    let mut out = CarvedChunkBuffer::new().unwrap();
    out.states.fill(state("stone"));
    let mut e = engine(&s.binding, &mut invocation, &mut out);
    for y in [-63, -62, 312, 313] {
        let p = BlockPos::new(2, y, 2);
        sphere(&mut e, p);
        let i = mask_index(p);
        assert_eq!(
            e.output.mask[i / 64] & (1 << (i % 64)) != 0,
            [-62, 312].contains(&y)
        );
    }
    e.output.states.fill(state("air"));
    set(e.output, BlockPos::new(0, 3, 0), state("stone"));
    set(e.output, BlockPos::new(0, 5, 0), state("stone"));
    e.output.heights.set(0, 0, 5);
    e.write(BlockPos::new(0, 5, 0), state("air")).unwrap();
    assert_eq!(e.output.heights.at(0, 0), 3);
    e.write(BlockPos::new(0, 3, 0), state("air")).unwrap();
    assert_eq!(e.output.heights.at(0, 0), -65);
}

#[test]
fn child_upper_one_executes_without_child_recursion() {
    let s = setup();
    for thickness in [1.0_f32, 2.0] {
        let mut invocation = run(&s);
        invocation.trace = Some(crate::carver::Trace::default());
        let mut out = CarvedChunkBuffer::new().unwrap();
        out.states.fill(state("stone"));
        let mut e = engine(&s.binding, &mut invocation, &mut out);
        e.tunnel(
            0,
            Tunnel {
                x: 8.0,
                y: 20.0,
                z: 8.0,
                hm: 1.0,
                vm: 1.0,
                thickness,
                yaw: 0.0,
                pitch: 0.0,
                start: 0,
                count: 16,
                ratio: 1.0,
                floor: -1.0,
            },
        )
        .unwrap();
        let calls = &e.run.trace.as_ref().unwrap().tunnels;
        assert_eq!(
            calls.len(),
            if thickness.to_bits() == 1.0_f32.to_bits() {
                1
            } else {
                3
            }
        );
        assert!(calls[1..].iter().all(|v| (0.5..=1.0).contains(v)));
        assert!(e.work.used > 1);
    }
}
