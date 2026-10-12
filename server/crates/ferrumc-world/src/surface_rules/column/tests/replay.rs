//! C1/C2 — original replay: every `full` group is driven from the genuine
//! GEN-05.3/.4/.5/.8/.9 producers and compared exactly with the projected
//! original trace, the final digest, `WORLD_SURFACE_WG` and the fluid marks.
//! Refused groups never contribute an original value here.

use super::*;
use crate::surface_rules::tests::array_digest;

/// C2: the producers deliver the genuine driver inputs of a group.
fn check_producers(group: &Group, produced: &Produced) {
    let fixture = fixture();
    let chunk = &fixture.chunks[group.gid];
    let states = produced.material.block_states();
    for (k, column) in chunk.s0_rle.split(';').enumerate() {
        let (lx, lz) = (k / 16, k % 16);
        let expected = unrle(column);
        assert_eq!(expected.len(), 384);
        for (i, s) in expected.iter().enumerate() {
            let y = MIN_Y + i32::try_from(i).unwrap();
            assert_eq!(
                states[index(lx, y, lz)].as_u32(),
                fixture.raw[*s],
                "{} S0 ({lx},{y},{lz})",
                group.id
            );
        }
    }
    assert_eq!(
        bitmap_positions(produced.material.fluid_update_bitmap()),
        post_positions(&chunk.s0_post),
        "{} NOISE post-processing",
        group.id
    );
    // A2: the caller-trusted corners come from the GEN-05.3 producer.
    assert_eq!(produced.corners, group.corners, "{} corners", group.id);
    let digests: Vec<String> = produced
        .arrays
        .iter()
        .map(|a| array_digest(a.values()))
        .collect();
    assert_eq!(digests, group.digests, "{} arrays", group.id);
    let model = LiveModel::new(states.iter().map(|s| s.as_u32()).collect());
    let mut heights = Vec::with_capacity(256);
    for lx in 0..16 {
        for lz in 0..16 {
            heights.push(model.heights[lx][lz].to_string());
        }
    }
    assert_eq!(
        heights.join(","),
        chunk.s0_heights,
        "{} S0 heights",
        group.id
    );
}

#[derive(Default, Debug)]
struct Totals {
    groups: usize,
    columns: usize,
    calls: usize,
    eroded: u64,
    rules: u64,
    frozen: u64,
    steep: usize,
    steep_changed: usize,
    max_column_work: u32,
    max_chunk_work: u32,
    max_session_work: u32,
    biome_called: usize,
    biome_not_called: usize,
    max_call_work: u32,
}

/// The nine trace tokens of every column of one group, exactly.
fn compare_columns(group: &Group, recorder: &Recorder<'_>, totals: &mut Totals) -> (u32, u32, u32) {
    let fixture = fixture();
    let rows = &fixture.columns[group.gid];
    let (mut eroded, mut rule, mut frozen) = (0_u32, 0_u32, 0_u32);
    for (trace, row) in recorder.columns.iter().zip(rows) {
        assert_eq!((trace.lx, trace.lz), (row.lx, row.lz));
        assert_eq!(
            trace.biome,
            Some(fixture.biomes[row.biome]),
            "{} ({},{}) biome",
            group.id,
            row.lx,
            row.lz
        );
        let tokens = project(trace);
        for (i, name) in TRACE_CELLS.iter().enumerate() {
            assert_eq!(
                tokens[i], row.cells[i],
                "{} ({},{}) {name}",
                group.id, row.lx, row.lz
            );
        }
        assert!(
            trace.work <= SURFACE_COLUMN_WORK,
            "{} column work",
            group.id
        );
        totals.max_column_work = totals.max_column_work.max(trace.work);
        totals.calls += trace.calls.len();
        totals.steep += usize::from(trace.steep.is_some());
        eroded += trace.eroded.len() as u32;
        frozen += trace.frozen.len() as u32;
        rule += trace
            .calls
            .iter()
            .filter(|(_, o)| matches!(o, SurfaceRuleOutcome::State(_)))
            .count() as u32;
    }
    (eroded, rule, frozen)
}

/// Final digest, `WORLD_SURFACE_WG`, fluid marks and the live steep views.
fn compare_final(
    group: &Group,
    produced: &Produced,
    buffer: &SurfaceChunkBuffer,
    recorder: &Recorder<'_>,
    totals: &mut Totals,
) {
    let fixture = fixture();
    let chunk = &fixture.chunks[group.gid];
    assert_eq!(
        states_digest(&buffer.states),
        chunk.final_sha,
        "{} digest",
        group.id
    );
    assert_eq!(
        heights_text(buffer.world_surface_heights().unwrap()),
        chunk.final_heights,
        "{} WORLD_SURFACE_WG",
        group.id
    );
    assert_eq!(
        bitmap_positions(&buffer.fluid),
        post_positions(&chunk.final_post),
        "{} fluid marks",
        group.id
    );
    // The independent live model reproduces every first steep read.
    let mut model = LiveModel::new(
        produced
            .material
            .block_states()
            .iter()
            .map(|s| s.as_u32())
            .collect(),
    );
    for SteepRead {
        column,
        read,
        live,
        initial,
    } in model.replay(&recorder.events)
    {
        assert_eq!(read, live, "{} steep view {column:?}", group.id);
        totals.steep_changed += usize::from(read != initial);
    }
}
/// C1: 70 `full` groups, every column trace, final chunk, heights and marks.
#[test]
fn every_full_group_replays_the_original_trace_exactly() {
    let fixture = fixture();
    let classes = fixture.manifest["counts"]["classes"].as_object().unwrap();
    assert_eq!(classes["full"], 70);
    assert_eq!(classes["refused:CentreOutsideDomain"], 1);
    assert_eq!(classes["refused:InconsistentResolver(Source)"], 1);
    assert_eq!(produced().len(), 70);
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    let mut totals = Totals::default();
    for produced in produced() {
        let group = &fixture.groups[produced.gid];
        assert_eq!(group.class, Class::Full);
        assert_eq!(group.sea_level, 63);
        check_producers(group, produced);
        let (rules, columns) = bindings(group.settings, group.seed);
        let resolver = produced.resolver();
        let (report, recorder) = drive_recorded(
            columns,
            rules,
            &resolver,
            produced.corners,
            &produced.material,
            &mut buffer,
        );
        assert_eq!(
            buffer.published(),
            Some((group.center, WorldSeed::new(group.seed), group.settings))
        );
        assert_eq!(recorder.columns.len(), 256);
        let (eroded, rule, frozen) = compare_columns(group, &recorder, &mut totals);
        compare_final(group, produced, &buffer, &recorder, &mut totals);
        assert_eq!(
            (
                report.eroded_writes(),
                report.rule_writes(),
                report.frozen_writes()
            ),
            (eroded, rule, frozen)
        );
        assert!(report.work_used() <= SURFACE_CHUNK_WORK);
        assert_eq!(
            report.max_column_work(),
            recorder.columns.iter().map(|c| c.work).max().unwrap()
        );
        totals.groups += 1;
        totals.columns += 256;
        totals.eroded += u64::from(eroded);
        totals.rules += u64::from(rule);
        totals.frozen += u64::from(frozen);
        totals.max_chunk_work = totals.max_chunk_work.max(report.work_used());
        totals.max_session_work = totals.max_session_work.max(report.session_work_used());
        totals.biome_called += recorder.biome_called;
        totals.biome_not_called += recorder.biome_not_called;
        totals.max_call_work = totals.max_call_work.max(recorder.max_call_work);
    }
    assert_eq!((totals.groups, totals.columns), (70, 17_920));
    assert_eq!(totals.calls, 2_261_637);
    // RR-H2: both sides of the memo laziness occur.
    assert!(totals.biome_called > 0 && totals.biome_not_called > 0);
    println!("GEN0510_REPLAY {totals:?}");
}

/// The public accessors return exactly the internal published data, and
/// nothing outside the chunk or the build height.
#[test]
fn public_accessors_expose_the_published_chunk_only() {
    let (group, produced) = produced_group("origin/normal/0");
    let (rules, columns) = bindings(group.settings, group.seed);
    let resolver = produced.resolver();
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    assert_eq!(buffer.published(), None);
    assert!(buffer.world_surface_heights().is_none());
    assert_eq!(buffer.state(BlockPos::new(0, 0, 0)), None);
    columns
        .build_chunk(
            SurfaceChunkInput::new(
                rules,
                &resolver,
                temperatures(),
                produced.corners,
                &produced.material,
            ),
            work_budget(SURFACE_CHUNK_WORK),
            &mut buffer,
        )
        .unwrap();
    for lx in 0..16 {
        for lz in 0..16 {
            for y in MIN_Y..=TOP_Y {
                let pos = BlockPos::new(lx, y, lz);
                let at = index(lx as usize, y, lz as usize);
                assert_eq!(buffer.state(pos), Some(buffer.states[at]));
                assert_eq!(
                    buffer.fluid_flag(pos),
                    Some(buffer.fluid[at / 64] & (1 << (at % 64)) != 0)
                );
            }
        }
    }
    for pos in [
        BlockPos::new(0, MIN_Y - 1, 0),
        BlockPos::new(0, TOP_Y + 1, 0),
        BlockPos::new(16, 0, 0),
        BlockPos::new(-1, 0, 0),
        BlockPos::new(0, i32::MIN, 0),
        BlockPos::new(0, i32::MAX, 0),
    ] {
        assert_eq!(buffer.state(pos), None);
        assert_eq!(buffer.fluid_flag(pos), None);
    }
}
