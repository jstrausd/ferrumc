//! New GEN-05.11 full producer acceptance. Historical expectations are read only.
use super::*;
use crate::carver::{
    CarvedChunkBuffer, CarverBindBudget, CarverInput, CarverWorkBudget, SeededCarvers, Trace,
    CARVER_CHUNK_WORK,
};

const CARVER_DIR: &str = "tests/fixtures/carver-runtime-original-1.21.8";
fn rows(name: &str) -> Vec<Vec<String>> {
    std::fs::read_to_string(format!(
        "{}/{CARVER_DIR}/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
    .lines()
    .skip(1)
    .map(|line| line.split('\t').map(str::to_owned).collect())
    .collect()
}
fn all_states(buffer: &CarvedChunkBuffer, center: ChunkPos) -> Vec<BlockStateId> {
    let mut states = Vec::with_capacity(98304);
    for y in -64..=319 {
        for x in 0..16 {
            for z in 0..16 {
                states.push(
                    buffer
                        .state(BlockPos::new(center.x() * 16 + x, y, center.z() * 16 + z))
                        .unwrap(),
                );
            }
        }
    }
    states
}
fn digest(states: &[BlockStateId], names: &HashMap<u32, String>) -> String {
    let mut h = Sha256::new();
    for state in states {
        h.update(names[&state.as_u32()].as_bytes());
        h.update(b"\n");
    }
    format!("{:x}", h.finalize())
}
fn compare_states(states: &[BlockStateId], row: &[String], raw: &[u32], label: &str) {
    for (column, token) in row[7].split(';').enumerate() {
        let expected = unrle(token);
        assert_eq!(expected.len(), 384);
        for (level, state) in expected.into_iter().enumerate() {
            assert_eq!(
                states[(level * 16 + column / 16) * 16 + column % 16].as_u32(),
                raw[state],
                "{label} column{column} level{level}"
            );
        }
    }
}
fn post_set(token: &str) -> HashSet<usize> {
    // Original postprocessing preserves ordered duplicates; output policy is a set.
    if token == "-" {
        return HashSet::new();
    }
    token
        .split(';')
        .flat_map(|section| {
            let (s, values) = section.split_once(':').unwrap();
            let s: usize = s.parse().unwrap();
            values.split(',').filter(|v| !v.is_empty()).map(move |v| {
                let p: usize = v.parse().unwrap();
                ((s * 16 + (p >> 4 & 15)) * 16 + (p & 15)) * 16 + (p >> 8 & 15)
            })
        })
        .collect()
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One differential gate compares every full corpus projection."
)]
fn carver_runtime_all_full_producer_replays() {
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(format!(
            "{}/{CARVER_DIR}/manifest.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    for (name, hash) in manifest["files"].as_object().unwrap() {
        assert_eq!(
            sha(&std::fs::read(format!(
                "{}/{CARVER_DIR}/{name}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap()),
            hash.as_str().unwrap()
        );
    }
    let states = rows("states.tsv");
    let raw: Vec<u32> = states.iter().map(|r| r[2].parse().unwrap()).collect();
    let names: HashMap<u32, String> = states
        .iter()
        .map(|r| (r[2].parse().unwrap(), r[1].clone()))
        .collect();
    let chunks = rows("chunks.tsv");
    let traces = rows("trace.tsv");
    let groups = rows("groups.tsv");
    let query_rows = rows("biome-queries.tsv");
    let carves = rows("carves.tsv");
    let vanilla = rows("vanilla.tsv");
    let mut vanilla_differences = 0;
    let mut totals = [0_u64; 4];
    for g in &groups {
        if g[5] != "full" {
            continue;
        }
        let settings = settings(match g[1].as_str() {
            "normal" => "minecraft:overworld",
            "large_biomes" => "minecraft:large_biomes",
            "amplified" => "minecraft:amplified",
            _ => panic!("unexpected preset"),
        });
        let seed = g[2].parse().unwrap();
        let center = ChunkPos::new(g[3].parse().unwrap(), g[4].parse().unwrap());
        let produced = produce(settings, seed, center, 0);
        let (rules, columns) = bindings(settings, seed);
        let resolver = produced.resolver();
        let mut surface = SurfaceChunkBuffer::new().unwrap();
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
                &mut surface,
            )
            .unwrap();
        let s1 = chunks
            .iter()
            .find(|r| r[0] == g[0] && r[1] == "S1")
            .unwrap();
        compare_states(&surface.states, s1, &raw, &format!("{} S1", g[0]));
        assert_eq!(digest(&surface.states, &names), g[10], "{} S1 digest", g[0]);
        let saved = surface.states.clone();
        let saved_fluid = surface.fluid.clone();
        let saved_heights = surface.heights;
        let binding = SeededCarvers::bind(columns, CarverBindBudget::new(90).unwrap()).unwrap();
        let mut run = binding
            .prepare_run(
                router(settings, seed),
                rules,
                temperatures(),
                overworld_source(),
                center,
                produced.corners,
            )
            .unwrap();
        run.trace = Some(Trace::default());
        let mut output = CarvedChunkBuffer::new().unwrap();
        let report = binding
            .carve_chunk(
                CarverInput::new(&surface, &mut run),
                CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
                &mut output,
            )
            .unwrap();
        let final_row = chunks
            .iter()
            .find(|r| r[0] == g[0] && r[1] == "FINAL")
            .unwrap();
        let actual = all_states(&output, center);
        compare_states(&actual, final_row, &raw, &format!("{} FINAL", g[0]));
        let final_digest = digest(&actual, &names);
        assert_eq!(final_digest, g[11], "{} FINAL digest", g[0]);
        let control = vanilla.iter().find(|r| r[0] == g[0]).unwrap();
        assert_eq!(
            final_digest == control[1],
            control[5] == "1",
            "{} Vanilla discrimination",
            g[0]
        );
        if control[5] == "1" {
            assert_eq!(final_row[2], control[3]);
            assert_eq!(post_set(&final_row[4]), post_set(&control[4]));
            let mask = |text: &str| {
                text.split(',')
                    .filter(|v| !["empty", "absent"].contains(v))
                    .map(|v| u64::from_str_radix(v, 16).unwrap())
                    .collect::<Vec<_>>()
            };
            assert_eq!(mask(&final_row[6]), mask(&control[2]));
        } else {
            vanilla_differences += 1;
        }
        assert_eq!(
            heights_text(output.world_surface_heights().unwrap()),
            final_row[2],
            "{} heights",
            g[0]
        );
        let expected_mask: Vec<u64> = final_row[6]
            .split(',')
            .filter(|v| !["empty", "absent"].contains(v))
            .map(|v| u64::from_str_radix(v, 16).unwrap())
            .collect();
        let mut fluid = HashSet::new();
        for y in -64..=319 {
            for x in 0..16 {
                for z in 0..16 {
                    let p = BlockPos::new(center.x() * 16 + x, y, center.z() * 16 + z);
                    let mi = (x as usize) | ((z as usize) << 4) | (((y + 64) as usize) << 8);
                    assert_eq!(
                        output.carving_mask(p).unwrap(),
                        expected_mask
                            .get(mi / 64)
                            .is_some_and(|w| w & (1 << (mi % 64)) != 0),
                        "{} mask {p:?}",
                        g[0]
                    );
                    if output.fluid_flag(p).unwrap() {
                        fluid.insert((((y + 64) as usize * 16) + x as usize) * 16 + z as usize);
                    }
                }
            }
        }
        assert_eq!(fluid, post_set(&final_row[4]), "{} fluids", g[0]);
        let trace = run.trace.as_ref().unwrap();
        assert_eq!(trace.starts, g[9], "{} starts", g[0]);
        let expected_trace = &traces.iter().find(|r| r[0] == g[0]).unwrap()[1];
        let normalized = trace
            .events
            .iter()
            .map(|event| {
                let mut values: Vec<String> = event[1..].split(',').map(str::to_owned).collect();
                if event.starts_with('T') && values[3] == i32::MIN.to_string() {
                    values[3] = "-".to_owned();
                }
                if event.starts_with('A') || event.starts_with('T') {
                    let i = if event.starts_with('A') { 3 } else { 5 };
                    let state: i64 = values[i].parse().unwrap();
                    values[i] = if state < 0 {
                        "-1".to_owned()
                    } else {
                        raw.iter()
                            .position(|s| i64::from(*s) == state)
                            .unwrap()
                            .to_string()
                    };
                }
                format!("{}{}", &event[..1], values.join(","))
            })
            .collect::<Vec<_>>()
            .join(";");
        let actual_events: Vec<_> = (if normalized.is_empty() {
            "-"
        } else {
            &normalized
        })
        .split(';')
        .collect();
        let expected_events: Vec<_> = expected_trace.split(';').collect();
        assert_eq!(
            actual_events.len(),
            expected_events.len(),
            "{} trace length",
            g[0]
        );
        for (i, (a, e)) in actual_events.iter().zip(expected_events).enumerate() {
            assert_eq!(*a, e, "{} trace event{i}", g[0]);
        }
        let expected_carves: Vec<_> = carves.iter().filter(|r| r[0] == g[0]).collect();
        assert_eq!(trace.carves.len(), expected_carves.len());
        for ((source, kind, counts), row) in trace.carves.iter().zip(expected_carves) {
            assert_eq!(
                (source.x(), source.z(), *kind),
                (
                    row[1].parse().unwrap(),
                    row[2].parse().unwrap(),
                    row[3].parse().unwrap()
                )
            );
            let expected: Vec<u64> = row[5..].iter().map(|v| v.parse().unwrap()).collect();
            assert_eq!(
                &counts[..],
                &expected[..],
                "{} source{source:?} carver{kind} original post-call projection",
                g[0]
            );
        }
        let queries: Vec<_> = query_rows
            .iter()
            .filter(|r| r[0] == g[0] && r[1] == "R")
            .collect();
        assert_eq!(trace.queries.len(), queries.len());
        for (actual, row) in trace.queries.iter().zip(queries) {
            assert_eq!(actual.0, row[3] == "FIXUP");
            assert_eq!(
                (actual.1.x(), actual.1.y(), actual.1.z()),
                (
                    row[4].parse().unwrap(),
                    row[5].parse().unwrap(),
                    row[6].parse().unwrap()
                ),
                "{} query block",
                g[0]
            );
            assert_eq!(
                (actual.2.x(), actual.2.y(), actual.2.z()),
                (
                    row[7].parse().unwrap(),
                    row[8].parse().unwrap(),
                    row[9].parse().unwrap()
                ),
                "{} quart",
                g[0]
            );
            assert_eq!(
                actual.3.resource_location(),
                row[10],
                "{} query biome",
                g[0]
            );
        }
        assert_eq!(&*surface.states, &*saved);
        assert_eq!(&*surface.fluid, &*saved_fluid);
        assert_eq!(surface.heights, saved_heights);
        totals[0] += 1;
        totals[1] += report.aquifer_calls();
        totals[2] += report.fixups();
        totals[3] = totals[3].max(report.work_used());
        println!("GEN0511_GROUP {} {:?}", g[0], report);
    }
    assert_eq!(totals[0], 71);
    assert_eq!(totals[1], 42117);
    assert_eq!(totals[2], 355);
    assert_eq!(vanilla_differences, 25);
    println!("GEN0511_REPLAY {totals:?} Vanilla-different={vanilla_differences}");
}

#[test]
fn rr10_h1_and_h5_production_top_319_and_published_refusal() {
    let (group, produced) = produced_group("origin/normal/0");
    let (rules, columns) = bindings(group.settings, group.seed);
    let resolver = produced.resolver();
    let mut states =
        vec![
            BlockStateId::new(ferrumc_registry::block_state::block_default_state("air").unwrap());
            98304
        ];
    states[index(0, 319, 0)] =
        BlockStateId::new(ferrumc_registry::block_state::block_default_state("stone").unwrap());
    let material = produced.material.with_states(states.into_boxed_slice());
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    let (_, recorder) = drive_recorded(
        columns,
        rules,
        &resolver,
        produced.corners,
        &material,
        &mut buffer,
    );
    let first = &recorder.columns[0];
    assert_eq!(first.height_plus_1, 320);
    assert_eq!(first.first_call_height_plus_1, 320);
    assert_eq!(first.calls[0].0.y(), 319);
    assert_eq!(
        buffer.world_surface_heights().unwrap().height(0, 0),
        Some(319)
    );
    assert!(buffer.published().is_some());
    let error = columns
        .build_chunk(
            SurfaceChunkInput::new(
                rules,
                &resolver,
                temperatures(),
                produced.corners,
                &material,
            ),
            work_budget(SURFACE_CHUNK_WORK - 1),
            &mut buffer,
        )
        .unwrap_err();
    assert_eq!(error.kind(), &SurfaceColumnErrorKind::WorkBudgetExceeded);
    assert!(buffer.published().is_none());
    assert!(buffer.state(BlockPos::new(0, 319, 0)).is_none());
}

#[test]
fn carver_refused_corpus_classes_remain_typed_empty() {
    let classes: Vec<_> = rows("groups.tsv")
        .into_iter()
        .filter(|g| g[5] != "full")
        .map(|g| (g[0].clone(), g[5].clone()))
        .collect();
    assert_eq!(classes.len(), 2);
    // Reuse the original accepted producers' exact class controls; they build
    // genuine fixed arrays and the actual out-of-domain halo, respectively.
    super::refusals::refused_source_group_yields_only_the_typed_refusal();
    super::refusals::refused_centre_outside_domain_is_refused_by_the_producers();
    assert!(classes.contains(&(
        "policy/single_biome_surface/0".to_owned(),
        "refused:InconsistentResolver(Source)".to_owned()
    )));
    assert!(classes.contains(&(
        "outside/normal/0/+x".to_owned(),
        "refused:CentreOutsideDomain".to_owned()
    )));
    let (group, produced) = produced_group("origin/normal/0");
    let (rules, columns) = bindings(group.settings, group.seed);
    let binding = SeededCarvers::bind(columns, CarverBindBudget::new(90).unwrap()).unwrap();
    let resolver = produced.resolver();
    let mut surface = SurfaceChunkBuffer::new().unwrap();
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
            &mut surface,
        )
        .unwrap();
    let mut output = CarvedChunkBuffer::new().unwrap();
    let mut publisher = binding
        .prepare_run(
            router(group.settings, group.seed),
            rules,
            temperatures(),
            overworld_source(),
            group.center,
            produced.corners,
        )
        .unwrap();
    binding
        .carve_chunk(
            CarverInput::new(&surface, &mut publisher),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut output,
        )
        .unwrap();
    assert!(output.published().is_some());
    // Upstream refusals have no Published Surface to give the Carver. No
    // original values from either refused row enter this invocation.
    let empty = SurfaceChunkBuffer::new().unwrap();
    let mut invocation = binding
        .prepare_run(
            router(group.settings, group.seed),
            rules,
            temperatures(),
            overworld_source(),
            group.center,
            produced.corners,
        )
        .unwrap();
    let error = binding
        .carve_chunk(
            CarverInput::new(&empty, &mut invocation),
            CarverWorkBudget::new(CARVER_CHUNK_WORK).unwrap(),
            &mut output,
        )
        .unwrap_err();
    assert_eq!(
        error.kind(),
        &crate::carver::CarverErrorKind::InconsistentSurface(
            crate::carver::SurfaceMismatch::NotPublished
        )
    );
    assert!(output.published().is_none());
}
