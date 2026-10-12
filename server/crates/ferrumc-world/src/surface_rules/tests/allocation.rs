//! External allocator measurement (A4 / §8): compile and bind are counted,
//! session construction, `begin_column`, `evaluate` and `min_surface_level`
//! must allocate nothing.
//!
//! Run only through `tests/reference/surface-rules/measure_allocations.py`,
//! which preloads the accepted GEN-05.7 test-only probe in a fresh process.
//! Fixture parsing, GEN-05.5 production, resolvers and inputs are prepared
//! before the measured regions.

use std::hint::black_box;
use std::io::{self, Write};

use super::*;

fn boundary(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes).unwrap();
    stdout.flush().unwrap();
}

#[test]
#[ignore = "external Linux allocator probe; run measure_allocations.py in Debug and Release"]
fn measured_build_and_every_fixture_evaluation() {
    let fixture = fixture();
    let generated = generated();
    let refs: Vec<Vec<&ChunkBiomeSamples>> = generated.iter().map(Generated::refs).collect();
    let resolvers: Vec<SurfaceBiomeResolver<'_>> = generated
        .iter()
        .zip(&refs)
        .map(|(g, r)| resolver(g, r))
        .collect();
    let temperatures = temperatures();
    // Test bindings are built before the measured regions (lazily cached otherwise).
    let bindings: Vec<&SeededSurfaceRules> = generated
        .iter()
        .map(|g| binding(&fixture.groups[g.gid]))
        .collect();
    let mut rows_by_gid: Vec<Vec<(&Row, bool)>> =
        fixture.groups.iter().map(|_| Vec::new()).collect();
    for entry in &fixture.replay {
        let row = &fixture.rows[fixture.by_ordinal[&entry.ordinal]];
        rows_by_gid[row.gid].push((row, entry.steep_probe));
    }
    let resource = settings("minecraft:overworld");
    let mut built = Vec::with_capacity(4);

    boundary(b"GEN057_BEGIN_CONTROL\n");
    let mut control = Vec::<u8>::with_capacity(4096);
    control.push(1);
    black_box(&control);
    drop(control);
    boundary(b"GEN057_END_CONTROL\n");

    boundary(b"GEN057_BEGIN_BUILD\n");
    // Compile and bind may allocate; they are counted, not claimed free.
    let template = Arc::new(
        SurfaceRuleTemplate::compile_builtin(resource, build_budget(MAX_SURFACE_RULE_BUILD_WORK))
            .unwrap(),
    );
    built.push(bind(Arc::clone(&template), 0));
    boundary(b"GEN057_END_BUILD\n");

    let (mut sessions, mut columns, mut evaluations, mut minima, mut refused) =
        (0_usize, 0_usize, 0_usize, 0_usize, 0_usize);
    boundary(b"GEN057_BEGIN_EVALUATE\n");
    for ((generated, resolver), rules) in generated.iter().zip(&resolvers).zip(&bindings) {
        let group = &fixture.groups[generated.gid];
        for (row, steep_probe) in &rows_by_gid[generated.gid] {
            let view = if *steep_probe {
                &fixture.steep[&group.gid]
            } else {
                &group.heights
            };
            let mut session = SurfaceRuleSession::new(
                rules,
                resolver,
                temperatures,
                group.corners,
                work_budget(MAX_SURFACE_RULE_SESSION_WORK),
            )
            .unwrap();
            sessions += 1;
            session.begin_column(black_box(row.column_pos())).unwrap();
            columns += 1;
            match session.evaluate(black_box(row.input()), view) {
                Ok(outcome) => {
                    black_box(outcome);
                    evaluations += 1;
                }
                Err(error) => {
                    black_box(error);
                    refused += 1;
                }
            }
            black_box(session.min_surface_level().unwrap());
            minima += 1;
        }
    }
    boundary(b"GEN057_END_EVALUATE\n");
    black_box(&built);
    assert_eq!((sessions, columns, minima), (38_889, 38_889, 38_889));
    assert_eq!((evaluations, refused), (36_824, 2065));
    println!("measured 1 compile + 1 bind, {sessions} sessions, {columns} columns, {evaluations} evaluations, {refused} refused, {minima} minima");
}
