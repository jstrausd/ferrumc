//! Genuine original equality under the mechanical replay rule (§6, A1).
//!
//! `full` rows replay their column run with complete trace equality, `partial`
//! rows alone with the per-block subset, `refused` rows alone and expect only
//! the typed P4 refusal with an unchanged state projection. No row is added and
//! no class is relaxed; refused rows' original values are never compared.

use super::*;

/// All 720 actual GEN-05.5 stored arrays (cold history per group, nine chunks
/// row-major) equal the original arrays by digest.
#[test]
fn every_genuine_stored_biome_array_matches_its_original_digest() {
    let fixture = fixture();
    let mut count = 0;
    for generated in generated() {
        let expected: Vec<_> = fixture
            .arrays
            .iter()
            .filter(|a| a.0 == generated.gid)
            .collect();
        assert_eq!(expected.len(), 9);
        for (array, (_, chunk, digest)) in generated.arrays.iter().zip(expected) {
            assert_eq!(array.chunk_pos(), *chunk);
            assert_eq!(
                &array_digest(array.values()),
                digest,
                "group {}",
                generated.gid
            );
            count += 1;
        }
    }
    assert_eq!(count, 720);
}

fn state_name(state: BlockStateId) -> String {
    let name = ferrumc_registry::block_state::state_id_to_block_name(state.as_u32()).unwrap();
    if name.contains(':') {
        name.to_owned()
    } else {
        format!("minecraft:{name}")
    }
}

/// Clay bands, resolved anchors, sea level and generation context of every group.
#[test]
fn bindings_reproduce_original_bands_anchors_and_generation_context() {
    let fixture = fixture();
    for group in &fixture.groups {
        let rules = binding(group);
        let bands: Vec<String> = rules.bands.iter().map(|s| state_name(*s)).collect();
        assert_eq!(bands, group.bands, "{}", group.id);
        let template = rules.template();
        let mut anchors = Vec::new();
        for (id, node) in template.nodes.iter().enumerate() {
            match node.kind {
                NodeKind::VerticalGradient(g) | NodeKind::PaperFlatBedrock(g) => {
                    let g = template.gradients[usize::from(g)];
                    anchors.push(format!("{id}={}/{}", g.true_y, g.false_y));
                }
                NodeKind::YAbove(c) => {
                    anchors.push(format!("{id}={}", template.y_checks[usize::from(c)].y));
                }
                _ => {}
            }
        }
        assert_eq!(anchors.join(","), group.anchors, "{}", group.id);
        assert_eq!(template.sea_level(), group.sea_level);
        assert_eq!(template.min_generation_y(), group.min_gen_y);
        assert_eq!(template.generation_depth(), group.gen_depth);
    }
}

#[derive(Default)]
struct Census {
    counts: HashMap<(u8, Class), usize>,
    compared_noise: usize,
    compared_draws: usize,
    compared_steep: usize,
    compared_min: usize,
    compared_secondary: usize,
    compared_columns: usize,
}

struct Ctx<'a> {
    group: &'a Group,
    rules: &'a SeededSurfaceRules,
    resolver: &'a SurfaceBiomeResolver<'a>,
}

impl Ctx<'_> {
    fn session(&self) -> SurfaceRuleSession<'_> {
        SurfaceRuleSession::new(
            self.rules,
            self.resolver,
            temperatures(),
            self.group.corners,
            work_budget(MAX_SURFACE_RULE_SESSION_WORK),
        )
        .unwrap()
    }
    fn view<'f>(&'f self, fixture: &'f Fixture, steep_probe: bool) -> &'f WorldSurfaceHeights {
        if steep_probe {
            &fixture.steep[&self.group.gid]
        } else {
            &self.group.heights
        }
    }
    fn base_checks(
        &self,
        row: &Row,
        session: &SurfaceRuleSession<'_>,
        outcome: SurfaceRuleOutcome,
    ) {
        let fixture = fixture();
        let label = format!("row {}", row.ordinal);
        assert!(
            outcome_matches(outcome, &fixture.states[row.result]),
            "{label}: {outcome:?}"
        );
        assert_eq!(
            session.surface_depth().unwrap(),
            row.surface_depth,
            "{label}"
        );
        let expected = fixture.biomes[row.biome];
        let pos = BlockPos::new(row.x, row.y, row.z);
        assert_eq!(
            self.resolver.biome_at_block(pos).unwrap(),
            expected,
            "{label}"
        );
        assert!(session.block.biome.is_none_or(|b| b == expected), "{label}");
    }
}

/// One `begin_column` per run on the group's shared session: per-column state
/// never leaks across columns.
fn full_run(
    ctx: &Ctx<'_>,
    session: &mut SurfaceRuleSession<'_>,
    run: &[&Row],
    steep_probe: bool,
    census: &mut Census,
) {
    let fixture = fixture();
    session.begin_column(run[0].column_pos()).unwrap();
    for (i, row) in run.iter().enumerate() {
        assert_eq!(row.ordinal, run[0].ordinal + i, "run prefix is consecutive");
        let before = session.column;
        let outcome = session
            .evaluate(row.input(), ctx.view(fixture, steep_probe))
            .unwrap();
        ctx.base_checks(row, session, outcome);
        let t = trace(ctx.rules, before, session, ctx.group.corners, row.y);
        let (computed, singles) = fixture.patterns[row.pattern].split_once('|').unwrap();
        let label = format!("full row {}", row.ordinal);
        assert_eq!(t.computed, computed, "{label}");
        assert_eq!(t.singles, singles, "{label}");
        assert_eq!(t.noise_values, row.noise_values, "{label}");
        assert_eq!(t.gradient_draws, row.gradient_draws, "{label}");
        assert_eq!(t.steep_heights, row.steep_heights, "{label}");
        assert_eq!(t.min_surface, row.min_surface, "{label}");
        assert_eq!(t.secondary, row.secondary, "{label}");
        census.compared_noise += usize::from(row.noise_values != "-");
        census.compared_draws += usize::from(row.gradient_draws != "-");
        census.compared_steep += usize::from(row.steep_heights != "-");
        census.compared_min += usize::from(row.min_surface != "-");
        census.compared_secondary += usize::from(row.secondary != "-");
        if i == 0 {
            let column = session.column.unwrap();
            let cell = format!(
                "{};{}",
                double_text(column.surface_noise),
                double_text(column.draw)
            );
            assert_eq!(cell, fixture.columns[row.column.unwrap()], "{label}");
            census.compared_columns += 1;
        } else {
            assert_eq!(row.column, None, "{label}");
        }
    }
}

fn partial_row(ctx: &Ctx<'_>, row: &Row, steep_probe: bool) {
    let fixture = fixture();
    let mut session = ctx.session();
    session.begin_column(row.column_pos()).unwrap();
    let before = session.column;
    let outcome = session
        .evaluate(row.input(), ctx.view(fixture, steep_probe))
        .unwrap();
    ctx.base_checks(row, &session, outcome);
    let t = trace(ctx.rules, before, &session, ctx.group.corners, row.y);
    let nodes = &ctx.rules.template().nodes;
    let per_block_entries = |list: &str| -> String {
        list.split(',')
            .filter(|e| !e.is_empty())
            .filter(|e| {
                let id: usize = e.split(':').next().unwrap().parse().unwrap();
                per_block(nodes[id].kind)
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let (computed, singles) = fixture.patterns[row.pattern].split_once('|').unwrap();
    let label = format!("partial row {}", row.ordinal);
    assert_eq!(
        per_block_entries(&t.computed),
        per_block_entries(computed),
        "{label}"
    );
    assert_eq!(t.gradient_draws, row.gradient_draws, "{label}");
    let temperature = |list: &str| {
        list.split(',')
            .find(|s| s.starts_with("temperature:"))
            .map(str::to_owned)
    };
    assert_eq!(temperature(&t.singles), temperature(singles), "{label}");
}

fn refused_row(ctx: &Ctx<'_>, row: &Row, expected: &SurfaceRuleErrorKind, steep_probe: bool) {
    let fixture = fixture();
    let mut session = ctx.session();
    session.begin_column(row.column_pos()).unwrap();
    let before = (session.projection(), session.block);
    let error = session
        .evaluate(row.input(), ctx.view(fixture, steep_probe))
        .unwrap_err();
    assert_eq!(error.kind(), expected, "refused row {}", row.ordinal);
    assert_eq!(
        (session.projection(), session.block),
        before,
        "refused row {}",
        row.ordinal
    );
    // A refusal is no poison; the session stays usable.
    assert!(session.surface_depth().is_ok());
}

/// Every G/C fixture row under its declared class, with real resolvers.
#[test]
fn every_fixture_row_replays_under_its_declared_class() {
    let fixture = fixture();
    let mut census = Census::default();
    let mut by_gid: HashMap<usize, Vec<&Replay>> = HashMap::new();
    for entry in &fixture.replay {
        let row = &fixture.rows[fixture.by_ordinal[&entry.ordinal]];
        by_gid.entry(row.gid).or_default().push(entry);
    }
    for generated in generated() {
        let group = &fixture.groups[generated.gid];
        let refs = generated.refs();
        let resolver = resolver(generated, &refs);
        let ctx = Ctx {
            group,
            rules: binding(group),
            resolver: &resolver,
        };
        let entries = by_gid.remove(&group.gid).unwrap_or_default();
        let mut runs: Vec<(usize, bool, Vec<&Row>)> = Vec::new();
        for entry in entries {
            let row = &fixture.rows[fixture.by_ordinal[&entry.ordinal]];
            *census.counts.entry((row.tier, entry.class)).or_default() += 1;
            match entry.class {
                Class::Full => match runs.last_mut() {
                    Some((start, _, rows)) if *start == entry.run_start => rows.push(row),
                    _ => runs.push((entry.run_start, entry.steep_probe, vec![row])),
                },
                Class::Partial => partial_row(&ctx, row, entry.steep_probe),
                Class::Refused => refused_row(
                    &ctx,
                    row,
                    entry.refusal.as_ref().unwrap(),
                    entry.steep_probe,
                ),
            }
            assert_eq!(entry.refusal.is_some(), entry.class == Class::Refused);
        }
        let mut shared = ctx.session();
        for (start, steep_probe, rows) in &runs {
            assert_eq!(rows[0].ordinal, *start);
            full_run(&ctx, &mut shared, rows, *steep_probe, &mut census);
        }
    }
    assert!(by_gid.is_empty());
    let count = |tier: u8, class: Class| census.counts.get(&(tier, class)).copied().unwrap_or(0);
    assert_eq!(
        [
            count(b'G', Class::Full),
            count(b'G', Class::Partial),
            count(b'G', Class::Refused)
        ],
        [2490, 4230, 0]
    );
    assert_eq!(
        [
            count(b'C', Class::Full),
            count(b'C', Class::Partial),
            count(b'C', Class::Refused)
        ],
        [5478, 24_626, 2065]
    );
    println!(
        "GEN059_REPLAY full G/C {}/{} partial {}/{} refused {}/{}; full-trace noise rows {} draw rows {} steep {} min {} secondary {} column cells {}",
        count(b'G', Class::Full),
        count(b'C', Class::Full),
        count(b'G', Class::Partial),
        count(b'C', Class::Partial),
        count(b'G', Class::Refused),
        count(b'C', Class::Refused),
        census.compared_noise,
        census.compared_draws,
        census.compared_steep,
        census.compared_min,
        census.compared_secondary,
        census.compared_columns,
    );
}

/// The refused rows are exactly the P4 kinds per case family (manifest counts).
#[test]
fn refused_rows_carry_the_first_failing_p4_check() {
    let fixture = fixture();
    let mut height = 0;
    let mut stone = 0;
    for entry in &fixture.replay {
        let row = &fixture.rows[fixture.by_ordinal[&entry.ordinal]];
        let y_out = !(-64..=319).contains(&row.y);
        let stone_out =
            !(1..=4096).contains(&row.stone_above) || !(1..=4096).contains(&row.stone_below);
        match &entry.refusal {
            Some(SurfaceRuleErrorKind::BlockOutsideGenerationHeight) => {
                assert!(y_out);
                height += 1;
            }
            Some(SurfaceRuleErrorKind::InvalidStoneDepth) => {
                assert!(!y_out && stone_out);
                stone += 1;
            }
            None => assert!(!y_out && !stone_out && row.water != Some(i32::MIN)),
            other => panic!("unexpected refusal {other:?}"),
        }
    }
    assert_eq!((height, stone), (2059, 6));
}

/// The `temperature` condition composes the memoized biome with the GEN-05.7
/// table at the settings' sea level (63). The fixture rows never reach the
/// altitude adjustment (`y > sea + 17`), so this controlled scan of the frozen
/// ocean groups reaches it at `y = water - 1` and requires at least one case
/// where sea level 64 would give the other result (discriminating).
#[test]
fn temperature_condition_uses_the_settings_sea_level() {
    let fixture = fixture();
    let (mut checked, mut discriminating) = (0, 0);
    for generated in generated() {
        let group = &fixture.groups[generated.gid];
        if !group.id.contains("frozen_ocean") {
            continue;
        }
        let refs = generated.refs();
        let resolver = resolver(generated, &refs);
        let rules = binding(group);
        let sea = rules.template().sea_level();
        assert_eq!(sea, 63);
        let mut session = SurfaceRuleSession::new(
            rules,
            &resolver,
            temperatures(),
            group.corners,
            work_budget(MAX_SURFACE_RULE_SESSION_WORK),
        )
        .unwrap();
        for lx in 0..16 {
            for lz in 0..16 {
                let column = SurfaceColumnPos::new(group.center, lx, lz).unwrap();
                session.begin_column(column).unwrap();
                if session.surface_depth().unwrap() > 0 {
                    continue;
                }
                for y in 81..=200 {
                    let input = SurfaceBlockInput::new(y, 1, 1, Some(y + 1));
                    session.evaluate(input, &group.heights).unwrap();
                    let (Some(cold), Some(biome)) =
                        (session.block.temperature, session.block.biome)
                    else {
                        continue;
                    };
                    let pos = BlockPos::new(column.block_x(), y, column.block_z());
                    let at = |level| {
                        temperatures()
                            .sample(biome, pos, level)
                            .unwrap()
                            .cold_enough_to_snow()
                    };
                    assert_eq!(cold, at(sea), "{} {pos:?}", group.id);
                    checked += 1;
                    discriminating += usize::from(at(sea) != at(sea + 1));
                }
            }
        }
    }
    println!("GEN059_TEMPERATURE checked {checked} discriminating {discriminating}");
    assert!(
        discriminating > 0,
        "the scan must discriminate the sea level"
    );
}
