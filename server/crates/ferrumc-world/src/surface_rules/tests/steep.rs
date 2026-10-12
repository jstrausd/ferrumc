//! A8 / H6: the heights view counts only at a column's first admitted `steep`
//! test (source-based requirement; no differential case exercises it).

use super::*;

struct Case {
    row: &'static Row,
    steep: WorldSurfaceHeights,
    flat: WorldSurfaceHeights,
}

/// A view whose original asymmetric Z comparison is steep at this column.
fn steep_view(pos: SurfaceColumnPos) -> WorldSurfaceHeights {
    let mut heights = [[100; 16]; 16];
    let (lx, lz) = (usize::from(pos.local_x()), usize::from(pos.local_z()));
    // Original: h(lx, min(lz+1, 15)) >= h(lx, max(lz-1, 0)) + 4.
    heights[lx][(lz + 1).min(15)] = 110;
    WorldSurfaceHeights::new(heights)
}

fn fresh(ctx: &Ctx, row: &Row, view: &WorldSurfaceHeights) -> SurfaceRuleOutcome {
    let mut session = ctx.session();
    session.begin_column(row.column_pos()).unwrap();
    session.evaluate(row.input(), view).unwrap()
}

struct Ctx {
    group: &'static Group,
    resolver: SurfaceBiomeResolver<'static>,
}

impl Ctx {
    fn session(&self) -> SurfaceRuleSession<'_> {
        SurfaceRuleSession::new(
            binding(self.group),
            &self.resolver,
            temperatures(),
            self.group.corners,
            work_budget(100_000),
        )
        .unwrap()
    }
}

/// The first genuine row (in fixture order) that tests `steep` and whose
/// outcome differs between a steep and a flat view: the test discriminates.
fn case() -> (Ctx, Case) {
    static REFS: OnceLock<Vec<Vec<&'static ChunkBiomeSamples>>> = OnceLock::new();
    let fixture = fixture();
    let refs = REFS.get_or_init(|| generated().iter().map(Generated::refs).collect());
    for entry in fixture.replay.iter().filter(|e| e.class != Class::Refused) {
        let row = &fixture.rows[fixture.by_ordinal[&entry.ordinal]];
        let (_, singles) = fixture.patterns[row.pattern].split_once('|').unwrap();
        if !singles.contains("steep:") {
            continue;
        }
        let index = generated().iter().position(|g| g.gid == row.gid).unwrap();
        let ctx = Ctx {
            group: &fixture.groups[row.gid],
            resolver: resolver(&generated()[index], &refs[index]),
        };
        let case = Case {
            row,
            steep: steep_view(row.column_pos()),
            flat: WorldSurfaceHeights::new([[100; 16]; 16]),
        };
        if fresh(&ctx, row, &case.steep) != fresh(&ctx, row, &case.flat) {
            return (ctx, case);
        }
    }
    panic!("no discriminating steep row");
}

/// The view changes after the column's first steep test; the result keeps the
/// first test's value.
#[test]
fn heights_changed_after_the_first_steep_test_do_not_change_the_result() {
    let (ctx, case) = case();
    let expected = fresh(&ctx, case.row, &case.steep);
    let mut session = ctx.session();
    session.begin_column(case.row.column_pos()).unwrap();
    assert_eq!(
        session.evaluate(case.row.input(), &case.steep).unwrap(),
        expected
    );
    let (steep, read) = session.column.unwrap().steep.unwrap();
    assert!(steep);
    assert_eq!(
        session.evaluate(case.row.input(), &case.flat).unwrap(),
        expected
    );
    assert_eq!(session.column.unwrap().steep, Some((steep, read)));
    assert_ne!(expected, fresh(&ctx, case.row, &case.flat));
    // Symmetric: a flat first test stays flat.
    let mut session = ctx.session();
    session.begin_column(case.row.column_pos()).unwrap();
    let flat = session.evaluate(case.row.input(), &case.flat).unwrap();
    assert_eq!(
        session.evaluate(case.row.input(), &case.steep).unwrap(),
        flat
    );
}

/// A refused call before the first steep test tests nothing and fills no cache;
/// the first admitted steep test fixes the view.
#[test]
fn a_refused_call_before_the_first_steep_test_fixes_nothing() {
    let (ctx, case) = case();
    let expected = fresh(&ctx, case.row, &case.steep);
    let mut session = ctx.session();
    session.begin_column(case.row.column_pos()).unwrap();
    let refused = SurfaceBlockInput::new(320, 1, 1, None);
    let error = session.evaluate(refused, &case.flat).unwrap_err();
    assert_eq!(
        *error.kind(),
        SurfaceRuleErrorKind::BlockOutsideGenerationHeight
    );
    assert_eq!(session.column.unwrap().steep, None);
    assert_eq!(
        session.evaluate(case.row.input(), &case.steep).unwrap(),
        expected
    );
    assert_eq!(
        session.evaluate(case.row.input(), &case.flat).unwrap(),
        expected
    );
}
