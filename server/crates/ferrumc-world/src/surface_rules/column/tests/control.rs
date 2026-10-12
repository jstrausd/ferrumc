//! Contract control tests (labelled contract tests, never original
//! comparisons): C3 steep timing through the driver's real write path (A3)
//! and C5 source-based edges through the internal `cfg(test)` routines (A4).
//! Every expectation here is the contract text (§3, §5, §6) emulated directly
//! in the test, with the original's unclamped loops and `VOID_AIR` reads.

use std::sync::Arc;

use ferrumc_registry::block_state::{block_default_state, state_id_to_block_name};

use super::super::super::compile::{compile_synthetic, Caps};
use super::super::super::SurfaceRuleErrorKind;
use super::super::driver::controls::Synthetic;
use super::super::driver::{
    eroded_floor, frozen_shape, java_floor, java_min, paper_double, FrozenShape, Silent,
};
use super::*;
use crate::random::WorldgenRandomSource;
use crate::surface_rules::SurfaceRuleWorkBudget;

/// Every default-stone call tests `steep` (and rewrites stone onto stone).
const STEEP_STONE: &str = r#"{"type":"minecraft:condition","if_true":{"type":"minecraft:steep"},"then_run":{"type":"minecraft:block","result_state":{"Name":"minecraft:stone"}}}"#;
/// The top three stone blocks of each run become air; the fourth tests `steep`.
const AIR_THEN_STEEP: &str = r#"{"type":"minecraft:sequence","sequence":[{"type":"minecraft:condition","if_true":{"type":"minecraft:stone_depth","offset":2,"add_surface_depth":false,"secondary_depth_range":0,"surface_type":"floor"},"then_run":{"type":"minecraft:block","result_state":{"Name":"minecraft:air"}}},{"type":"minecraft:condition","if_true":{"type":"minecraft:steep"},"then_run":{"type":"minecraft:block","result_state":{"Name":"minecraft:stone"}}}]}"#;

fn synthetic(json: &str, seed: i64) -> (SeededSurfaceRules, SeededSurfaceColumns) {
    let template = Arc::new(compile_synthetic(json, Caps::PRODUCTION).unwrap());
    let rules = bind(template, seed);
    let columns =
        SeededSurfaceColumns::bind(&rules, bind_budget(MAX_SURFACE_COLUMN_BIND_WORK)).unwrap();
    (rules, columns)
}

/// `(steep events, reads of a changed earlier column, of the changed current column)`.
fn steep_changes(json: &str, group_id: &str) -> (usize, usize, usize) {
    let (group, produced) = produced_group(group_id);
    let (rules, columns) = synthetic(json, group.seed);
    let resolver = produced.resolver();
    let mut buffer = SurfaceChunkBuffer::new().unwrap();
    let (_, recorder) = drive_recorded(
        &columns,
        &rules,
        &resolver,
        produced.corners,
        &produced.material,
        &mut buffer,
    );
    let mut model = LiveModel::new(
        produced
            .material
            .block_states()
            .iter()
            .map(|s| s.as_u32())
            .collect(),
    );
    let (mut events, mut earlier, mut current) = (0, 0, 0);
    for SteepRead {
        column: (lx, lz),
        read,
        live,
        initial,
    } in model.replay(&recorder.events)
    {
        events += 1;
        // A3: the four reads equal the live view at that moment.
        assert_eq!(read, live, "{group_id} steep view at ({lx},{lz})");
        let (x, z) = (usize::from(lx), usize::from(lz));
        let neighbours = [
            (x, z.saturating_sub(1)),
            (x, (z + 1).min(15)),
            (x.saturating_sub(1), z),
            ((x + 1).min(15), z),
        ];
        for (i, (nx, nz)) in neighbours.into_iter().enumerate() {
            if read[i] == initial[i] {
                continue;
            }
            if (nx, nz) == (x, z) {
                current += 1;
            } else if nx * 16 + nz < x * 16 + z {
                earlier += 1;
            } else {
                panic!("{group_id}: a later column changed before its turn");
            }
        }
    }
    (events, earlier, current)
}

/// C3/A3 (a): eroded prepass raises (real writes of genuine eroded columns)
/// change an earlier column's height and, at the chunk edge, the current
/// column's own height before the first steep test; the reads follow the live
/// view and differ from post-NOISE. A driver passing post-NOISE heights fails.
#[test]
fn steep_reads_follow_eroded_raises_of_earlier_and_current_columns() {
    let mut totals = (0, 0, 0);
    for group in ["biome/eroded_badlands", "seam/eroded_pillar"] {
        let (events, earlier, current) = steep_changes(STEEP_STONE, group);
        assert!(
            events > 200 && earlier > 0,
            "{group}: {events} {earlier} {current}"
        );
        totals = (totals.0 + events, totals.1 + earlier, totals.2 + current);
    }
    assert!(totals.2 > 0, "{totals:?}");
    println!(
        "GEN0510_C3_ERODED events {} earlier {} current {}",
        totals.0, totals.1, totals.2
    );
}

/// C3/A3 (b): rule air writes at the column top rescan the height downward;
/// the fourth call of the run then reads the lowered earlier columns and, at
/// the chunk edge, the lowered current column.
#[test]
fn steep_reads_follow_air_write_rescans_of_earlier_and_current_columns() {
    let (events, earlier, current) = steep_changes(AIR_THEN_STEEP, "biome/plains");
    assert!(
        events > 200 && earlier > 0 && current > 0,
        "{events} {earlier} {current}"
    );
    println!("GEN0510_C3_AIR events {events} earlier {earlier} current {current}");
}

/// A named synthetic column: state by Y.
type Shape = (&'static str, fn(i32) -> u32);

fn raw(name: &str) -> u32 {
    block_default_state(name).unwrap()
}

fn is_water(raw: u32) -> bool {
    state_id_to_block_name(raw) == Some("water")
}

/// Contract test fixture: the origin group's genuine resolver and session over
/// a buffer whose column `(lx, lz)` holds `column(y)` and is air elsewhere.
struct Bench {
    rules: &'static SeededSurfaceRules,
    columns: &'static SeededSurfaceColumns,
    resolver: SurfaceBiomeResolver<'static>,
    produced: &'static Produced,
    buffer: SurfaceChunkBuffer,
}

impl Bench {
    fn new(lx: usize, lz: usize, column: impl Fn(i32) -> u32) -> Self {
        let (group, produced) = produced_group("origin/normal/0");
        let (rules, columns) = bindings(group.settings, group.seed);
        let mut buffer = SurfaceChunkBuffer::new().unwrap();
        buffer.states.fill(BlockStateId::new(raw("air")));
        for y in MIN_Y..=TOP_Y {
            buffer.states[index(lx, y, lz)] = BlockStateId::new(column(y));
        }
        Self {
            rules,
            columns,
            resolver: produced.resolver(),
            produced,
            buffer,
        }
    }
    fn run<O: Observer, R>(
        &mut self,
        observer: &mut O,
        body: impl FnOnce(&mut Synthetic<'_, '_, O>) -> R,
    ) -> R {
        let session = SurfaceRuleSession::new(
            self.rules,
            &self.resolver,
            temperatures(),
            self.produced.corners,
            SurfaceRuleWorkBudget::new(crate::surface_rules::SURFACE_RULE_OVERWORLD_CHUNK_WORK)
                .unwrap(),
        )
        .unwrap();
        let input = SurfaceChunkInput::new(
            self.rules,
            &self.resolver,
            temperatures(),
            self.produced.corners,
            &self.produced.material,
        );
        let mut synthetic =
            Synthetic::new(self.columns, input, session, &mut self.buffer, observer).unwrap();
        body(&mut synthetic)
    }
    fn column(&self, lx: usize, lz: usize) -> Vec<u32> {
        (MIN_Y..=TOP_Y)
            .map(|y| self.buffer.states[index(lx, y, lz)].as_u32())
            .collect()
    }
}

/// The original eroded scan and fill, unclamped, over `[-64, 319]` + `VOID_AIR`.
fn original_eroded(column: &mut [u32], i4: i32, floor: i32) -> usize {
    let read = |column: &[u32], y: i32| {
        if (MIN_Y..=TOP_Y).contains(&y) {
            column[(y - MIN_Y) as usize]
        } else {
            raw("void_air")
        }
    };
    if i4 > floor {
        return 0;
    }
    let mut y = floor;
    while y >= MIN_Y {
        let state = read(column, y);
        if state_id_to_block_name(state) == Some("stone") {
            break;
        }
        if is_water(state) {
            return 0;
        }
        y -= 1;
    }
    let mut writes = 0;
    let mut y = floor;
    while y >= MIN_Y && is_air(read(column, y)) {
        if (MIN_Y..=TOP_Y).contains(&y) {
            column[(y - MIN_Y) as usize] = raw("stone");
            writes += 1;
        }
        y -= 1;
    }
    writes
}

/// C5: an eroded floor above 319 (and every eroded branch) equals the
/// unclamped original; the loops start at `min(floor, 319)`.
#[test]
fn eroded_fill_from_min_floor_319_equals_the_unclamped_original() {
    let shapes: [Shape; 3] = [
        ("stone below 100", |y| {
            if y <= 100 {
                raw("stone")
            } else {
                raw("air")
            }
        }),
        ("water above stone", |y| {
            if y <= 80 {
                raw("stone")
            } else if y <= 95 {
                raw("water")
            } else {
                raw("air")
            }
        }),
        ("stone above water", |y| {
            if y <= 40 {
                raw("water")
            } else if y <= 100 {
                raw("stone")
            } else {
                raw("air")
            }
        }),
    ];
    let mut checked = 0;
    for (name, shape) in shapes {
        for floor in [100, 150, 318, 319, 320, 330, 338, 400] {
            for i4 in [96, 101, 150, 320] {
                let mut bench = Bench::new(3, 0, shape);
                let mut expected = bench.column(3, 0);
                let writes = original_eroded(&mut expected, i4, floor);
                let (work, height) = bench.run(&mut Silent, |s| {
                    s.reset_work();
                    s.eroded_fill(3, 0, i4, floor).unwrap();
                    (s.work(), s.height(3, 0))
                });
                assert_eq!(bench.column(3, 0), expected, "{name} floor {floor} i4 {i4}");
                let top = (MIN_Y..=TOP_Y)
                    .rev()
                    .find(|y| !is_air(expected[(y - MIN_Y) as usize]))
                    .unwrap_or(MIN_Y - 1);
                assert_eq!(height, top);
                assert!(work <= SURFACE_COLUMN_ERODED_WORK - 3);
                checked += usize::from(writes > 0);
            }
        }
    }
    assert!(checked > 0);
}

/// The original iceberg loop from the unclamped `start` down to the unclamped
/// minimum surface, `VOID_AIR` outside the build height, dropped writes.
#[allow(clippy::too_many_arguments)]
fn original_frozen(
    column: &mut [u32],
    start: i32,
    min_surface: i32,
    shape: FrozenShape,
    sea: i32,
    snow_limit: i32,
    snow_above: i32,
    random: &mut WorldgenRandomSource,
) -> usize {
    let mut draws = 0;
    let mut snow = 0;
    let mut y = start;
    while y >= min_surface {
        let inside = (MIN_Y..=TOP_Y).contains(&y);
        let state = if inside {
            column[(y - MIN_Y) as usize]
        } else {
            raw("void_air")
        };
        let hit =
            is_air(state) && y < shape.top as i32 && {
                draws += 1;
                f64::from(random.next_double() as f32) > 0.01
            } || is_water(state) && y > shape.bottom as i32 && y < sea && shape.bottom != 0.0 && {
                draws += 1;
                f64::from(random.next_double() as f32) > 0.15
            };
        if hit {
            let target = if snow <= snow_limit && y > snow_above {
                snow += 1;
                raw("snow_block")
            } else {
                raw("packed_ice")
            };
            if inside {
                column[(y - MIN_Y) as usize] = target;
            }
        }
        y -= 1;
    }
    draws
}

/// C5/A4: the frozen loop from `min(start, 320)` with 320 as `VOID_AIR`, and a
/// minimum surface below −64, equal the unclamped original; snow then packed
/// ice; the flat shape writes nothing; a start above 320 is refused.
#[test]
fn frozen_fill_from_min_start_320_equals_the_unclamped_original() {
    let shape_column = |y: i32| {
        if y <= -60 {
            raw("air")
        } else if y <= 40 {
            raw("stone")
        } else if y <= 62 {
            raw("water")
        } else {
            raw("air")
        }
    };
    let iceberg = FrozenShape {
        top: 120.75,
        bottom: 45.5,
    };
    let flat = FrozenShape {
        top: 0.0,
        bottom: 0.0,
    };
    let deep = FrozenShape {
        top: 318.5,
        bottom: -30.25,
    };
    let mut cases = 0;
    for shape in [iceberg, flat, deep] {
        for start in [63, 121, 200, 319, 320] {
            if start < shape.top as i32 + 1 {
                continue;
            }
            for min_surface in [-200, -65, -64, -10, 30, 70] {
                let mut bench = Bench::new(7, 15, shape_column);
                let mut expected = bench.column(7, 15);
                let seed_pos = BlockPos::new(7, 0, 15);
                let mut original_random = bench.rules.root.at(seed_pos);
                original_frozen(
                    &mut expected,
                    start,
                    min_surface,
                    shape,
                    63,
                    3,
                    90,
                    &mut original_random,
                );
                let mut random = bench.rules.root.at(seed_pos);
                let work = bench.run(&mut Silent, |s| {
                    s.reset_work();
                    s.frozen_fill(7, 15, start, min_surface, shape, 3, 90, &mut random)
                        .unwrap();
                    s.work()
                });
                assert_eq!(
                    bench.column(7, 15),
                    expected,
                    "start {start} min {min_surface} {shape:?}"
                );
                // Above the minimum Y both sources consumed the same draws;
                // below it only the original draws (and then discards the source).
                if min_surface >= MIN_Y {
                    assert_eq!(random.next_long(), original_random.next_long());
                }
                assert!(work <= 385 + 384 + 384);
                cases += 1;
            }
        }
    }
    assert!(cases > 40);
    // Snow first (counter ≤ limit and above the snow line), then packed ice.
    let mut bench = Bench::new(0, 0, shape_column);
    let mut random = bench.rules.root.at(BlockPos::new(0, 0, 0));
    bench.run(&mut Silent, |s| {
        s.frozen_fill(0, 0, 121, -64, iceberg, 3, 90, &mut random)
            .unwrap();
    });
    let column = bench.column(0, 0);
    let snow = column.iter().filter(|s| **s == raw("snow_block")).count();
    let ice = column.iter().filter(|s| **s == raw("packed_ice")).count();
    assert!(snow > 0 && snow <= 4 && ice > snow, "{snow} {ice}");
    // A start above 320 would draw above the build height: defensive refusal.
    let mut bench = Bench::new(0, 0, shape_column);
    let mut random = bench.rules.root.at(BlockPos::new(0, 0, 0));
    let error = bench.run(&mut Silent, |s| {
        s.frozen_fill(0, 0, 321, -64, iceberg, 3, 90, &mut random)
            .unwrap_err()
    });
    assert_eq!(*error.kind(), SurfaceColumnErrorKind::Noise);
    assert_eq!((error.requested(), error.maximum()), (321, 320));
    assert_eq!(
        error.kind().category(),
        SurfaceColumnRefusalCategory::Defensive
    );
}

/// C5: a column whose top is 319 (`i4 = 320`): the resolver admits y 320, the
/// loop reads `VOID_AIR` at 320 and the call inputs follow contract §3.
#[test]
fn column_loop_at_i4_320_reads_void_air_and_derives_contract_inputs() {
    let (_, produced) = produced_group("origin/normal/0");
    produced
        .resolver()
        .biome_at_block(BlockPos::new(2, 320, 9))
        .unwrap();
    // stone, a water pocket, lava, an ore vein block and air gaps.
    let column = |y: i32| match y {
        -64..=258 | 260..=267 | 280..=294 | 300..=319 => raw("stone"),
        270..=279 => raw("water"),
        268..=269 => raw("lava"),
        259 => raw("granite"),
        _ => raw("air"),
    };
    let mut bench = Bench::new(2, 9, column);
    let initial = bench.column(2, 9);
    let mut recorder = Recorder::new(bench.rules);
    bench.run(&mut recorder, |s| {
        assert_eq!(s.height(2, 9) + 1, 320);
        s.begin_column(2, 9);
        s.column_loop(2, 9, 320).unwrap();
    });
    // Contract §3 step 2.6, emulated on the pre-loop column.
    let read = |y: i32| {
        if (MIN_Y..=TOP_Y).contains(&y) {
            initial[(y - MIN_Y) as usize]
        } else {
            raw("void_air")
        }
    };
    let fluid = |s: u32| matches!(state_id_to_block_name(s), Some("water" | "lava"));
    let mut expected = Vec::new();
    let (mut above, mut water, mut bottom) = (0, None, i32::MAX);
    for y in (MIN_Y..=320).rev() {
        let s = read(y);
        if is_air(s) {
            above = 0;
            water = None;
        } else if fluid(s) {
            water = water.or(Some(y + 1));
        } else {
            if bottom >= y {
                let mut t = y - 1;
                while t >= MIN_Y && !is_air(read(t)) && !fluid(read(t)) {
                    t -= 1;
                }
                bottom = t + 1;
            }
            above += 1;
            if s == raw("stone") {
                expected.push((y, above, y - bottom + 1, water));
            }
        }
    }
    let calls: Vec<(i32, i32, i32, Option<i32>)> = recorder
        .current_calls()
        .iter()
        .map(|(i, _)| {
            (
                i.y(),
                i.stone_depth_above(),
                i.stone_depth_below(),
                i.water_height(),
            )
        })
        .collect();
    assert_eq!(calls, expected);
    assert_eq!(calls[0], (319, 1, 20, None));
}

/// C5: `WORLD_SURFACE_WG` follows the original `Heightmap.update` rule through
/// the write path: raise, unchanged below, rescan on an air write at the top
/// (down to −65), dropped writes outside `[-64, 319]`, and fluid flags.
#[test]
fn writes_update_the_live_height_with_the_original_rule() {
    let column = |y: i32| if y <= 100 { raw("stone") } else { raw("air") };
    let mut bench = Bench::new(4, 4, column);
    let air = BlockStateId::new(raw("air"));
    let stone = BlockStateId::new(raw("stone"));
    let water = BlockStateId::new(raw("water"));
    bench.run(&mut Silent, |s| {
        assert_eq!(s.height(4, 4), 100);
        s.reset_work();
        s.write(4, 100, 4, air).unwrap(); // at the top: rescan one step
        assert_eq!((s.height(4, 4), s.work()), (99, 2));
        s.write(4, 50, 4, air).unwrap(); // below the top: unchanged
        assert_eq!(s.height(4, 4), 99);
        s.write(4, 150, 4, stone).unwrap(); // above: raise
        assert_eq!(s.height(4, 4), 150);
        s.write(4, 120, 4, stone).unwrap(); // non-air below the top: unchanged
        assert_eq!(s.height(4, 4), 150);
        s.reset_work();
        s.write(4, 150, 4, air).unwrap(); // rescan 149..120
        assert_eq!((s.height(4, 4), s.work()), (120, 1 + 30));
        s.write(4, 30, 4, water).unwrap(); // fluid flag set by a fluid write
        assert!(s.fluid(4, 30, 4) && !s.fluid(4, 31, 4));
        s.reset_work();
        for y in [TOP_Y + 1, MIN_Y - 1, i32::MAX, i32::MIN] {
            s.write(4, y, 4, stone).unwrap(); // dropped: no work, no change
        }
        assert_eq!((s.height(4, 4), s.work(), s.writes()), (120, 0, [0, 6, 0]));
        assert_eq!(s.state(4, TOP_Y, 4), air);
    });
    // A column reduced to its lowest block: the rescan ends at minY - 1.
    let mut bench = Bench::new(9, 1, |y| if y == MIN_Y { raw("stone") } else { raw("air") });
    bench.run(&mut Silent, |s| {
        assert_eq!(s.height(9, 1), MIN_Y);
        s.write(9, MIN_Y, 1, air).unwrap();
        assert_eq!(s.height(9, 1), MIN_Y - 1);
        s.write(9, MIN_Y, 1, air).unwrap(); // not at the top any more
        assert_eq!(s.height(9, 1), MIN_Y - 1);
    });
}

/// C5: the extension arithmetic branches and the Java helpers.
#[test]
fn extension_arithmetic_follows_the_original_branches() {
    // Iceberg: m1 = min(min² · 1.2, ceil(|roof · 1.5| · 40) + 14).
    let shape = frozen_shape(3.0, 0.1, false, 63);
    assert_eq!(shape.top, 10.8 + 63.0);
    assert_eq!(shape.bottom, 63.0 - 10.8 - 7.0);
    // ceil(|0.1 · 1.5| · 40) is 7, not 6: 0.15000000000000002 · 40 > 6.
    let shape = frozen_shape(10.0, 0.1, false, 63);
    assert_eq!((shape.top, shape.bottom), (21.0 + 63.0, 63.0 - 21.0 - 7.0));
    let shape = frozen_shape(10.0, 0.0, false, 63);
    assert_eq!((shape.top, shape.bottom), (14.0 + 63.0, 63.0 - 14.0 - 7.0));
    // Melt subtracts 2 before the flat test: 3.24 → 1.24 is flat.
    let raw_min = (3.24_f64 / 1.2).sqrt();
    let unmelted = frozen_shape(raw_min, 0.1, false, 63);
    assert!(unmelted.top > 63.0 + 2.0);
    let melted = frozen_shape(raw_min, 0.1, true, 63);
    assert_eq!((melted.top, melted.bottom), (0.0, 0.0));
    // Eroded floor: Mth.floor(64 + min(min² · 2.5, ceil(|roof · 1.5| · 50) + 24)).
    assert_eq!(eroded_floor(2.0, 0.0), 74);
    assert_eq!(eroded_floor(100.0, 10.0 / 3.0), 64 + 250 + 24);
    assert_eq!(eroded_floor(0.5, 0.0), 64);
    // Java Math.min and Mth.floor.
    assert_eq!(java_min(0.0, -0.0).to_bits(), (-0.0_f64).to_bits());
    assert_eq!(java_min(-0.0, 0.0).to_bits(), (-0.0_f64).to_bits());
    assert!(java_min(f64::NAN, 1.0).is_nan() && java_min(1.0, f64::NAN).is_nan());
    assert_eq!(java_min(1.0, 2.0), 1.0);
    // Java `(int)` saturates and `i - 1` wraps: Mth.floor(-1e10) is i32::MAX.
    assert_eq!(
        (
            java_floor(-0.5),
            java_floor(2.0),
            java_floor(-2.0),
            java_floor(1e10),
            java_floor(-1e10)
        ),
        (-1, 2, -2, i32::MAX, i32::MAX)
    );
    // Paper nextDouble rounds through f32 (P1).
    let mut a = WorldgenRandomSource::xoroshiro(WorldSeed::new(1));
    let mut b = WorldgenRandomSource::xoroshiro(WorldSeed::new(1));
    assert_eq!(paper_double(&mut a), f64::from(b.next_double() as f32));
}

/// C7 defensive: a session refusal inside the loop is a typed `Rules` error
/// carrying its block (no active column here).
#[test]
fn session_refusals_inside_the_loop_are_typed_and_carry_their_block() {
    let mut bench = Bench::new(1, 1, |y| if y <= 10 { raw("stone") } else { raw("air") });
    let error = bench.run(&mut Silent, |s| s.column_loop(1, 1, 11).unwrap_err());
    assert_eq!(
        *error.kind(),
        SurfaceColumnErrorKind::Rules(SurfaceRuleErrorKind::NoActiveColumn)
    );
    assert_eq!(
        error.kind().category(),
        SurfaceColumnRefusalCategory::Consistency
    );
}

mod rr10;
