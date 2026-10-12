//! A3 / P5: budget domains, census allowances and the round-3 work matrix.

use super::policy::origin;
use super::*;

/// Every budget refuses outside its closed domain (both ends, N-1/N/N+1).
#[test]
fn budget_domains_at_both_ends() {
    let ok = |r: Result<u32, SurfaceRuleError>| r.is_ok();
    let refused = |r: Result<u32, SurfaceRuleError>| {
        *r.unwrap_err().kind() == SurfaceRuleErrorKind::InvalidBudget
    };
    let build = |u| SurfaceRuleBuildBudget::new(u).map(SurfaceRuleBuildBudget::units);
    let bind = |u| SurfaceRuleBindBudget::new(u).map(SurfaceRuleBindBudget::units);
    let work = |u| SurfaceRuleWorkBudget::new(u).map(SurfaceRuleWorkBudget::units);
    assert!(refused(build(0)) && ok(build(1)) && ok(build(262_144)) && refused(build(262_145)));
    assert!(refused(bind(0)) && ok(bind(1)) && ok(bind(65_536)) && refused(bind(65_537)));
    assert!(refused(work(0)) && ok(work(1)) && ok(work(67_108_864)) && refused(work(67_108_865)));
    assert_eq!(MAX_SURFACE_RULE_BUILD_WORK, 262_144);
    assert_eq!(MAX_SURFACE_RULE_BIND_WORK, 65_536);
    assert_eq!(MAX_SURFACE_RULE_SESSION_WORK, 67_108_864);
}

/// Compile census 133,778 and bind census 14,757 at N-1/N/N+1, refused before work.
#[test]
fn compile_and_bind_census_allowances() {
    let resource = settings("minecraft:overworld");
    let error = SurfaceRuleTemplate::compile_builtin(resource, build_budget(133_777)).unwrap_err();
    assert_eq!(*error.kind(), SurfaceRuleErrorKind::WorkBudgetExceeded);
    assert_eq!((error.requested(), error.maximum()), (133_778, 133_777));
    for units in [133_778, 133_779] {
        let template = SurfaceRuleTemplate::compile_builtin(resource, build_budget(units)).unwrap();
        assert_eq!(template.construction_work(), 133_778);
    }
    let template = overworld();
    let bind = |units| {
        SeededSurfaceRules::bind(
            Arc::clone(&template),
            WorldSeed::new(0),
            SurfaceNumericProfile::PaperBuild60,
            PaperFlatBedrock::Disabled,
            bind_budget(units),
        )
    };
    let error = bind(14_756).unwrap_err();
    assert_eq!(*error.kind(), SurfaceRuleErrorKind::WorkBudgetExceeded);
    assert_eq!((error.requested(), error.maximum()), (14_757, 14_756));
    for units in [14_757, 14_758] {
        assert_eq!(bind(units).unwrap().construction_work(), 14_757);
    }
    // The census terms: 58 octave slots, 56 Paper-basis ImprovedNoise copies of
    // 259 charges, 192 band slots and 3 factories (root + two gradient names).
    let octaves: usize = template
        .noises
        .iter()
        .map(|n| 2 * n.parameters().amplitudes().len())
        .sum();
    let improved: usize = template
        .noises
        .iter()
        .map(|n| {
            2 * n
                .parameters()
                .amplitudes()
                .iter()
                .filter(|a| **a != 0.0)
                .count()
        })
        .sum();
    assert_eq!((octaves, improved), (58, 56));
    assert_eq!(58 + 56 * 259 + 192 + 3, 14_757);
    // The existing SampleWork charge of the nine Paper-basis copies is exactly 58 + 56·259.
    let mut work = crate::sample_work::SampleWork::new(usize::MAX);
    for noise in &*template.noises {
        let id = noise.resource_location();
        vanilla_noise(0, id).paper_build60_basis(&mut work).unwrap();
    }
    assert_eq!(work.used(), 58 + 56 * 259);
}

struct Origin {
    group: &'static Group,
    resolver: SurfaceBiomeResolver<'static>,
}

fn origin_ctx() -> Origin {
    static REFS: OnceLock<Vec<&'static ChunkBiomeSamples>> = OnceLock::new();
    let (group, generated) = origin();
    let refs = REFS.get_or_init(|| generated.refs());
    Origin {
        group,
        resolver: resolver(generated, refs),
    }
}

impl Origin {
    fn session(&self, units: u32) -> SurfaceRuleSession<'_> {
        SurfaceRuleSession::new(
            binding(self.group),
            &self.resolver,
            temperatures(),
            self.group.corners,
            work_budget(units),
        )
        .unwrap()
    }
    fn column(&self) -> SurfaceColumnPos {
        SurfaceColumnPos::new(self.group.center, 3, 11).unwrap()
    }
}

fn kind<T: std::fmt::Debug>(result: Result<T, SurfaceRuleError>) -> SurfaceRuleErrorKind {
    result.unwrap_err().kind().clone()
}

const WBE: SurfaceRuleErrorKind = SurfaceRuleErrorKind::WorkBudgetExceeded;

/// `begin_column` at remaining 1/2/3; the column domain precedes work; a
/// refusal leaves an earlier column, its caches and `work_used` unchanged.
#[test]
fn begin_column_work_boundaries() {
    let ctx = origin_ctx();
    let mut session = ctx.session(1);
    assert_eq!(kind(session.begin_column(ctx.column())), WBE);
    assert_eq!(session.projection(), (None, 0, false));
    let outside = SurfaceColumnPos::new(ChunkPos::new(1, 0), 0, 0).unwrap();
    assert_eq!(
        kind(session.begin_column(outside)),
        SurfaceRuleErrorKind::ColumnOutsideCentre
    );
    let mut session = ctx.session(2);
    session.begin_column(ctx.column()).unwrap();
    assert_eq!(session.work_used(), 2);
    let mut session = ctx.session(3);
    session.begin_column(ctx.column()).unwrap();
    assert_eq!(session.work_used(), 2);
    // Remaining 1 with an active column: refused without effect on it.
    session.min_surface_level().unwrap();
    let before = session.projection();
    assert_eq!(before.1, 3);
    let other = SurfaceColumnPos::new(ctx.group.center, 0, 0).unwrap();
    let mut session2 = ctx.session(4);
    session2.begin_column(ctx.column()).unwrap();
    session2.min_surface_level().unwrap();
    let kept = session2.projection();
    assert_eq!(kind(session2.begin_column(other)), WBE);
    assert_eq!(session2.projection(), kept);
    assert_eq!(
        kind(session2.begin_column(outside)),
        SurfaceRuleErrorKind::ColumnOutsideCentre
    );
}

/// `min_surface_level`: no column, uncached 0/1/2, cached 0; `surface_depth` charges 0.
#[test]
fn min_surface_level_and_surface_depth_work_boundaries() {
    let ctx = origin_ctx();
    let mut session = ctx.session(1);
    session.set_work_used(1);
    assert_eq!(
        kind(session.min_surface_level()),
        SurfaceRuleErrorKind::NoActiveColumn
    );
    assert_eq!(
        kind(session.surface_depth()),
        SurfaceRuleErrorKind::NoActiveColumn
    );
    // Uncached at remaining 0: refused, minimum stays uncached.
    let mut session = ctx.session(2);
    session.begin_column(ctx.column()).unwrap();
    assert_eq!(kind(session.min_surface_level()), WBE);
    assert_eq!(session.projection().0.unwrap().min_surface, None);
    // ... so a later evaluate still needs 467 + 1.
    let input = SurfaceBlockInput::new(64, 1, 1, None);
    let mut later = ctx.session(2 + 467);
    later.begin_column(ctx.column()).unwrap();
    assert_eq!(kind(later.evaluate(input, &ctx.group.heights)), WBE);
    for remaining in [1, 2] {
        let mut session = ctx.session(2 + remaining);
        session.begin_column(ctx.column()).unwrap();
        let depth = session.surface_depth().unwrap();
        assert_eq!(session.work_used(), 2, "surface_depth charges 0");
        let min = session.min_surface_level().unwrap();
        assert_eq!(session.work_used(), 3);
        assert_eq!(session.projection().0.unwrap().min_surface, Some(min));
        assert_eq!(session.surface_depth().unwrap(), depth);
        // Cached at remaining 0 (or 1): returned for 0 units.
        assert_eq!(session.min_surface_level().unwrap(), min);
        assert_eq!(session.work_used(), 3);
    }
}

/// `evaluate`: cached minimum 466/467/468, uncached 467/468/469, P4 before work at 0.
#[test]
fn evaluate_work_boundaries() {
    let ctx = origin_ctx();
    let input = SurfaceBlockInput::new(64, 1, 1, None);
    for (remaining, admitted) in [(466, false), (467, true), (468, true)] {
        let mut session = ctx.session(2 + 1 + remaining);
        session.begin_column(ctx.column()).unwrap();
        session.min_surface_level().unwrap();
        let before = session.projection();
        let result = session.evaluate(input, &ctx.group.heights);
        assert_eq!(result.is_ok(), admitted, "cached {remaining}");
        if !admitted {
            assert_eq!(kind(result), WBE);
            assert_eq!(session.projection(), before);
        }
    }
    for (remaining, admitted) in [(467, false), (468, true), (469, true)] {
        let mut session = ctx.session(2 + remaining);
        session.begin_column(ctx.column()).unwrap();
        let before = session.projection();
        let result = session.evaluate(input, &ctx.group.heights);
        assert_eq!(result.is_ok(), admitted, "uncached {remaining}");
        if !admitted {
            assert_eq!(kind(result), WBE);
            assert_eq!(session.projection(), before);
        }
    }
    let mut session = ctx.session(2);
    session.begin_column(ctx.column()).unwrap();
    let before = session.projection();
    let refused = session.evaluate(SurfaceBlockInput::new(-65, 1, 1, None), &ctx.group.heights);
    assert_eq!(
        kind(refused),
        SurfaceRuleErrorKind::BlockOutsideGenerationHeight
    );
    assert_eq!(session.projection(), before);
}

/// A defensive failure (private corrupted control) poisons; `Poisoned` then
/// precedes every other check of every fallible method; `work_used` stays readable.
#[test]
fn poisoned_is_checked_first_and_is_terminal() {
    let ctx = origin_ctx();
    let mut corrupt = SurfaceRuleTemplate::compile_builtin(
        settings("minecraft:overworld"),
        build_budget(MAX_SURFACE_RULE_BUILD_WORK),
    )
    .unwrap();
    // A condition kind where the root rule must be: unreachable for compiled templates.
    corrupt.nodes[0].kind = NodeKind::Steep;
    let rules = bind(Arc::new(corrupt), ctx.group.seed);
    let mut session = SurfaceRuleSession::new(
        &rules,
        &ctx.resolver,
        temperatures(),
        ctx.group.corners,
        work_budget(1_000),
    )
    .unwrap();
    session.begin_column(ctx.column()).unwrap();
    let error = session
        .evaluate(SurfaceBlockInput::new(64, 1, 1, None), &ctx.group.heights)
        .unwrap_err();
    assert_eq!(*error.kind(), SurfaceRuleErrorKind::MalformedDefinition);
    assert_eq!(error.node(), Some(0));
    let used = session.work_used();
    assert!(used > 2);
    let poisoned = SurfaceRuleErrorKind::Poisoned;
    let outside = SurfaceColumnPos::new(ChunkPos::new(5, 5), 0, 0).unwrap();
    assert_eq!(kind(session.begin_column(outside)), poisoned);
    assert_eq!(kind(session.begin_column(ctx.column())), poisoned);
    assert_eq!(
        kind(session.evaluate(
            SurfaceBlockInput::new(-65, 0, 0, Some(i32::MIN)),
            &ctx.group.heights
        )),
        poisoned
    );
    assert_eq!(
        kind(session.evaluate(SurfaceBlockInput::new(64, 1, 1, None), &ctx.group.heights)),
        poisoned
    );
    assert_eq!(kind(session.min_surface_level()), poisoned);
    assert_eq!(kind(session.surface_depth()), poisoned);
    assert_eq!(session.work_used(), used);
    assert_eq!(poisoned.category(), SurfaceRuleRefusalCategory::Policy);
}

type Step = (SurfaceRuleOutcome, Trace);

fn evaluate_run(
    session: &mut SurfaceRuleSession<'_>,
    rows: &[&Row],
    ctx: &Origin,
    stop: usize,
) -> Vec<Step> {
    let rules = binding(ctx.group);
    session.begin_column(rows[0].column_pos()).unwrap();
    let mut steps = Vec::new();
    for row in &rows[..stop] {
        let before = session.column;
        let outcome = session.evaluate(row.input(), &ctx.group.heights).unwrap();
        steps.push((
            outcome,
            trace(rules, before, session, ctx.group.corners, row.y),
        ));
    }
    steps
}

/// R2-M1 continuation: `WorkBudgetExceeded` in the middle of a column does not
/// poison; re-evaluating the column from its first block in a new session equals
/// an uninterrupted evaluation (outcomes, trace and the column's `work_used`).
#[test]
fn restart_in_a_new_session_equals_an_uninterrupted_column() {
    let ctx = origin_ctx();
    let fixture = fixture();
    // The longest complete genuine `full` run of the origin group.
    let mut runs: HashMap<usize, Vec<&Row>> = HashMap::new();
    for entry in fixture
        .replay
        .iter()
        .filter(|e| e.class == Class::Full && !e.steep_probe)
    {
        let row = &fixture.rows[fixture.by_ordinal[&entry.ordinal]];
        if row.gid == ctx.group.gid && row.tier == b'G' {
            runs.entry(entry.run_start).or_default().push(row);
        }
    }
    let rows = runs.into_values().max_by_key(Vec::len).unwrap();
    assert!(rows.len() > 8);
    let mut whole = ctx.session(MAX_SURFACE_RULE_SESSION_WORK);
    let reference = evaluate_run(&mut whole, &rows, &ctx, rows.len());
    let total = whole.work_used();
    // Interrupt after three blocks: the fourth pre-check fails.
    let mut probe = ctx.session(MAX_SURFACE_RULE_SESSION_WORK);
    evaluate_run(&mut probe, &rows, &ctx, 3);
    let mut short = ctx.session(probe.work_used() + 466);
    let prefix = evaluate_run(&mut short, &rows, &ctx, 3);
    assert_eq!(prefix, reference[..3]);
    let state = short.projection();
    assert_eq!(
        kind(short.evaluate(rows[3].input(), &ctx.group.heights)),
        WBE
    );
    assert_eq!(short.projection(), state);
    assert!(
        short.surface_depth().is_ok(),
        "WorkBudgetExceeded does not poison"
    );
    let mut restart = ctx.session(MAX_SURFACE_RULE_SESSION_WORK);
    let replay = evaluate_run(&mut restart, &rows, &ctx, rows.len());
    assert_eq!(replay, reference);
    assert_eq!(restart.work_used(), total);
}

/// Exactly `256 × (2 + 1 + 384 × 467)` suffices for every block of one chunk.
#[test]
fn one_overworld_chunk_fits_the_declared_allowance() {
    assert_eq!(SURFACE_RULE_OVERWORLD_CHUNK_WORK, 256 * (2 + 1 + 384 * 467));
    let ctx = origin_ctx();
    let mut session = ctx.session(SURFACE_RULE_OVERWORLD_CHUNK_WORK);
    let mut evaluated = 0;
    for lx in 0..16 {
        for lz in 0..16 {
            session
                .begin_column(SurfaceColumnPos::new(ctx.group.center, lx, lz).unwrap())
                .unwrap();
            for y in -64..=319 {
                session
                    .evaluate(SurfaceBlockInput::new(y, 1, 1, None), &ctx.group.heights)
                    .unwrap();
                evaluated += 1;
            }
            session.min_surface_level().unwrap();
        }
    }
    assert_eq!(evaluated, 256 * 384);
    assert!(session.work_used() <= SURFACE_RULE_OVERWORLD_CHUNK_WORK);
    println!(
        "GEN059_CHUNK evaluated {evaluated} work_used {}",
        session.work_used()
    );
}
