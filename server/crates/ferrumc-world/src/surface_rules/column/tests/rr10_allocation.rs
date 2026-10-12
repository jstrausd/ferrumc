//! RR10-H4: all four previously omitted refusal families, genuinely Published first.
use super::*;
use std::hint::black_box;
use std::io::{self, Write};
fn boundary(bytes: &[u8]) {
    let mut out = io::stdout().lock();
    out.write_all(bytes).unwrap();
    out.flush().unwrap();
}
#[test]
#[ignore = "external allocator probe; cold Debug and Release"]
fn binding_material_session_and_unclassified_refusals() {
    let (group, origin) = produced_group("origin/normal/0");
    let (_, minus) = produced_group("origin/normal/-1");
    let (rules, columns) = bindings(group.settings, group.seed);
    let (wrong_rules, _) = bindings(group.settings, -1);
    let resolver = origin.resolver();
    let wrong_resolver = minus.resolver();
    let table = temperatures();
    let input = SurfaceChunkInput::new(rules, &resolver, table, group.corners, &origin.material);
    let mut corrupted =
        SeededSurfaceColumns::bind(rules, bind_budget(MAX_SURFACE_COLUMN_BIND_WORK)).unwrap();
    corrupted.without_state(BlockStateId::new(
        ferrumc_registry::block_state::block_default_state("stone").unwrap(),
    ));
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    boundary(b"GEN057_BEGIN_CONTROL\n");
    let mut control = Vec::<u8>::with_capacity(4096);
    control.push(1);
    black_box(&control);
    drop(control);
    boundary(b"GEN057_END_CONTROL\n");
    boundary(b"GEN057_BEGIN_BUILD\n");
    black_box(
        SeededSurfaceColumns::bind(rules, bind_budget(MAX_SURFACE_COLUMN_BIND_WORK)).unwrap(),
    );
    boundary(b"GEN057_END_BUILD\n");
    boundary(b"GEN057_BEGIN_DRIVE\n");
    for defect in 0..4 {
        columns
            .build_chunk(input, work_budget(SURFACE_CHUNK_WORK), &mut buffer)
            .unwrap();
        assert!(buffer.published().is_some());
        let bad = match defect {
            0 => SurfaceChunkInput::new(
                wrong_rules,
                &resolver,
                table,
                group.corners,
                &origin.material,
            ),
            1 => SurfaceChunkInput::new(rules, &resolver, table, group.corners, &minus.material),
            2 => SurfaceChunkInput::new(
                rules,
                &wrong_resolver,
                table,
                group.corners,
                &origin.material,
            ),
            _ => input,
        };
        let error = if defect == 3 { &corrupted } else { columns }
            .build_chunk(bad, work_budget(SURFACE_CHUNK_WORK), &mut buffer)
            .unwrap_err();
        assert!(matches!(
            (defect, error.kind()),
            (0, SurfaceColumnErrorKind::InconsistentBinding(_))
                | (1, SurfaceColumnErrorKind::InconsistentMaterial(_))
                | (2, SurfaceColumnErrorKind::Rules(_))
                | (3, SurfaceColumnErrorKind::UnclassifiedState)
        ));
        black_box(error);
        assert!(buffer.published().is_none());
    }
    boundary(b"GEN057_END_DRIVE\n");
    println!("GEN0511_RR10_ALLOC success=4 published-refusal=4 families=Binding,Material,Session,UnclassifiedState");
}
