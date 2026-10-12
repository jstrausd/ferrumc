//! External allocator measurement (C6 / contract §10): `SeededSurfaceColumns::bind`
//! and `SurfaceChunkBuffer::new` are counted; `build_chunk` — every admitted
//! fixture chunk and refused calls — must allocate nothing.
//!
//! Run only through `tests/reference/surface-column/measure_allocations.py`,
//! which preloads the accepted GEN-05.7 test-only probe in a fresh process.
//! Fixture parsing, producers, rules bindings and resolvers are prepared
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
fn measured_bind_buffer_and_every_fixture_chunk() {
    let fixture = fixture();
    let produced = produced();
    let resolvers: Vec<SurfaceBiomeResolver<'_>> =
        produced.iter().map(Produced::resolver).collect();
    let pairs: Vec<&(SeededSurfaceRules, SeededSurfaceColumns)> = produced
        .iter()
        .map(|p| {
            let group = &fixture.groups[p.gid];
            bindings(group.settings, group.seed)
        })
        .collect();
    let temperatures = temperatures();
    let (rules, _) = bindings(overworld_settings(), 0);
    let mut built = Vec::with_capacity(2);
    let mut buffers = Vec::with_capacity(2);

    boundary(b"GEN057_BEGIN_CONTROL\n");
    let mut control = Vec::<u8>::with_capacity(4096);
    control.push(1);
    black_box(&control);
    drop(control);
    boundary(b"GEN057_END_CONTROL\n");

    boundary(b"GEN057_BEGIN_BUILD\n");
    // Bind and the buffer allocate; they are counted, not claimed free.
    built.push(
        SeededSurfaceColumns::bind(rules, bind_budget(MAX_SURFACE_COLUMN_BIND_WORK)).unwrap(),
    );
    buffers.push(SurfaceChunkBuffer::new().unwrap());
    boundary(b"GEN057_END_BUILD\n");

    let mut buffer = buffers.pop().unwrap();
    let (mut published, mut refused, mut work) = (0_usize, 0_usize, 0_u64);
    boundary(b"GEN057_BEGIN_DRIVE\n");
    for ((produced, resolver), (rules, columns)) in produced.iter().zip(&resolvers).zip(&pairs) {
        let input = SurfaceChunkInput::new(
            rules,
            resolver,
            temperatures,
            produced.corners,
            &produced.material,
        );
        let report = columns
            .build_chunk(
                black_box(input),
                work_budget(SURFACE_CHUNK_WORK),
                &mut buffer,
            )
            .unwrap();
        work += u64::from(report.work_used());
        published += 1;
        // A refused call (driver allowance) after a published one.
        match columns.build_chunk(input, work_budget(SURFACE_CHUNK_WORK - 1), &mut buffer) {
            Ok(_) => {}
            Err(error) => {
                black_box(error);
                refused += 1;
            }
        }
    }
    boundary(b"GEN057_END_DRIVE\n");
    black_box((&built, &buffer, work));
    assert_eq!((published, refused), (70, 70));
    println!("measured 1 bind + 1 buffer, {published} published chunks, {refused} refused chunks");
}
