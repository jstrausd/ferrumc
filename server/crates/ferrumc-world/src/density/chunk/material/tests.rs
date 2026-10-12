use super::super::state::Lifecycle;
use super::super::tests::{budget, router};
use super::super::{NoiseChunkWorkBudget, MAX_NOISE_CHUNK_WORK};
use super::*;
use crate::sample_work::SampleWork;

// Exact input points supplied by the actual-JAR helper; no surface/fluid rule.
pub(in crate::density::chunk) struct Picker {
    default: (i32, BlockStateId),
    overrides: [Option<(BlockPos, i32, BlockStateId)>; 16],
}
impl Picker {
    pub(in crate::density::chunk) fn new(
        default: (i32, BlockStateId),
        overrides: &[(BlockPos, i32, BlockStateId)],
    ) -> Self {
        assert!(overrides.len() <= 16);
        let mut own = [None; 16];
        for (slot, value) in own.iter_mut().zip(overrides) {
            *slot = Some(*value);
        }
        Self {
            default,
            overrides: own,
        }
    }
    pub(in crate::density::chunk) fn at(&self, pos: BlockPos) -> (i32, BlockStateId) {
        self.overrides
            .iter()
            .flatten()
            .find(|(point, _, _)| *point == pos)
            .map_or(self.default, |(_, height, kind)| (*height, *kind))
    }
}

// Only actual-JAR declared-float endpoint controls use this fixed test seam.
// Production keeps its concrete positional stream without a callback/allocator.
pub(in crate::density::chunk) struct Floats {
    values: [f32; 3],
    len: usize,
    index: usize,
}
impl Floats {
    pub(in crate::density::chunk) fn new(values: &[f32]) -> Self {
        assert!(!values.is_empty() && values.len() <= 3);
        let mut own = [0.0; 3];
        own[..values.len()].copy_from_slice(values);
        Self {
            values: own,
            len: values.len(),
            index: 0,
        }
    }
    pub(in crate::density::chunk) fn next(&mut self) -> Result<f32, NoiseChunkError> {
        if self.index >= self.len {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        let value = self.values[self.index];
        self.index += 1;
        Ok(value)
    }
}

#[test]
fn declared_ore_float_seam_is_bounded_and_metered() {
    let mut s = super::super::tests::requests::fresh(false);
    let toggle = super::super::tests::requests::leaf(&mut s, "toggle", &[1.0], (1.0, 1.0));
    let ridged = super::super::tests::requests::leaf(&mut s, "ridged", &[-1.0], (-1.0, -1.0));
    let gap = super::super::tests::requests::leaf(&mut s, "gap", &[0.0], (0.0, 0.0));
    s.fields[12] = toggle;
    s.fields[13] = ridged;
    s.fields[14] = gap;
    s.state.start = BlockPos::new(0, 25, 0);
    s.state.material_floats = Some(Floats::new(&[0.0, 0.0, 0.0]));
    let b = MaterialBlocks::new(
        s.data.metadata.default_block,
        s.data.metadata.default_fluid,
        &mut s.work,
    )
    .unwrap();
    assert_eq!(ore::compute(&mut s, &b).unwrap(), Some(b.raw_copper));
    assert_eq!(s.state.material_floats.as_ref().unwrap().index, 3);
    assert!(s.state.material_floats.as_mut().unwrap().next().is_err());
}

#[test]
fn final_state_and_fluid_write_have_separate_budget_boundaries() {
    fn prepared() -> (NoiseChunkSession, MaterialRun) {
        let mut s = super::super::tests::requests::fresh(false);
        s.material = super::super::tests::requests::leaf(&mut s, "substance", &[1.0], (1.0, 1.0));
        s.state.lifecycle = Lifecycle::Running;
        s.state.interpolating = true;
        s.state.driver_phase = super::super::state::DriverPhase::ZUpdated;
        let mut run = MaterialRun::new(&mut s, 1).unwrap();
        run.ore_veins = false;
        s.work = SampleWork::new(1_000_000);
        (s, run)
    }
    let (mut s, mut run) = prepared();
    run.write(&mut s, 0).unwrap();
    let units = s.work.used();
    assert_eq!(run.states, [run.blocks.default_block]);
    assert_eq!(run.fluid_updates, [0]);
    let (mut exact, mut run) = prepared();
    exact.work = SampleWork::new(units);
    run.write(&mut exact, 0).unwrap();
    assert_eq!(exact.work.used(), units);
    assert_eq!(run.states, [run.blocks.default_block]);
    let (mut short, mut run) = prepared();
    short.work = SampleWork::new(units - 2);
    exhausted(&run.write(&mut short, 0).unwrap_err(), units - 2);
    assert_eq!(run.states, [run.blocks.air]);
    assert_eq!(run.fluid_updates, [0]);
    let (mut short, mut run) = prepared();
    short.work = SampleWork::new(units - 1);
    exhausted(&run.write(&mut short, 0).unwrap_err(), units - 1);
    // Private partial writes are discarded by the public poisoned driver;
    // admitting the state write must not admit the separate bitmap charge.
    assert_eq!(run.states, [run.blocks.default_block]);
    assert_eq!(run.fluid_updates, [0]);
}

#[test]
fn material_output_lifecycle_and_reset() {
    let r = router("minecraft:overworld", 0);
    let mut s = r
        .fresh_paper_noise_chunk(ferrumc_math::ChunkPos::ORIGIN, budget())
        .unwrap();
    let out = s.sample_chunk_materials().unwrap();
    assert_eq!(out.block_states().len(), 98_304);
    assert_eq!(out.fluid_update_bitmap().len(), 1536);
    assert_eq!(
        out.block_state_at(BlockPos::new(0, -64, 0)),
        Some(out.block_states()[0])
    );
    assert_eq!(out.block_state_at(BlockPos::new(16, 0, 0)), None);
    assert_eq!(
        out.should_schedule_fluid_update(BlockPos::new(16, 0, 0)),
        None
    );
    assert_eq!(s.state.lifecycle, Lifecycle::Complete);
    let used = s.work_used();
    assert!(s.sample_chunk_densities().is_err());
    assert!(s.sample_chunk_materials().is_err());
    assert_eq!(s.work_used(), used);
    s.reset(budget()).unwrap();
    assert_eq!(
        s.sample_chunk_materials().unwrap().block_states(),
        out.block_states()
    );
}

#[test]
fn actual_paper_126_complete_material_chunks() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/material_chunk_1_21_8");
    let records = std::fs::read_to_string(directory.join("chunks.jsonl")).unwrap();
    let bytes = std::fs::read(directory.join("material_states.bits")).unwrap();
    assert_eq!(bytes.len(), 35_389_440);
    let mut samples = 0;
    let mut offset = 0;
    for line in records.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let name = row["settings"].as_str().unwrap();
        let seed = row["seed"].as_str().unwrap().parse().unwrap();
        let chunk = ferrumc_math::ChunkPos::new(
            row["chunk_x"].as_i64().unwrap().try_into().unwrap(),
            row["chunk_z"].as_i64().unwrap().try_into().unwrap(),
        );
        let r = router(name, seed);
        let mut s = r.fresh_paper_noise_chunk(chunk, budget()).unwrap();
        let out = s.sample_chunk_materials().unwrap();
        assert_eq!(
            out.geometry().min_y(),
            row["min_y"].as_i64().unwrap() as i32
        );
        assert_eq!(
            out.geometry().height(),
            row["height"].as_u64().unwrap() as u32
        );
        assert_eq!(row["offset"].as_u64().unwrap() as usize, offset);
        assert_eq!(
            row["bytes"].as_u64().unwrap() as usize,
            out.block_states().len() * 5
        );
        for (index, state) in out.block_states().iter().enumerate() {
            let at = offset + index * 5;
            let expected = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
            assert_eq!(
                state.as_u32(),
                expected,
                "{name} seed={seed} chunk={chunk:?} index={index}"
            );
            let flag = out.fluid_update_bitmap()[index / 64] >> (index % 64) & 1;
            assert_eq!(
                flag,
                u64::from(bytes[at + 4]),
                "{name} seed={seed} chunk={chunk:?} fluid index={index}"
            );
        }
        samples += out.block_states().len();
        offset += out.block_states().len() * 5;
    }
    assert_eq!(records.lines().count(), 126);
    assert_eq!(samples, 7_077_888);
    assert_eq!(offset, bytes.len());
}

#[test]
fn material_arrays_survive_reset_router_drop_and_reversed_chunk_requests() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/material_chunk_1_21_8");
    let records = std::fs::read_to_string(directory.join("chunks.jsonl")).unwrap();
    let rows = records
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|row| {
            row["seed"] == "81985529216486895"
                && ((row["chunk_x"] == -1 && row["chunk_z"] == 0)
                    || (row["chunk_x"] == 0 && row["chunk_z"] == 1))
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 14);
    let mut forward = Vec::new();
    for pair in rows.as_chunks::<2>().0 {
        assert_eq!(pair[0]["settings"], pair[1]["settings"]);
        let r = router(
            pair[0]["settings"].as_str().unwrap(),
            81_985_529_216_486_895,
        );
        let position = |row: &serde_json::Value| {
            ferrumc_math::ChunkPos::new(
                row["chunk_x"].as_i64().unwrap().try_into().unwrap(),
                row["chunk_z"].as_i64().unwrap().try_into().unwrap(),
            )
        };
        let mut session = r
            .fresh_paper_noise_chunk(position(&pair[0]), budget())
            .unwrap();
        let mut neighbor = r
            .fresh_paper_noise_chunk(position(&pair[1]), budget())
            .unwrap();
        let first = session.sample_chunk_materials().unwrap();
        let second = neighbor.sample_chunk_materials().unwrap();
        session.reset(budget()).unwrap();
        let reset = session.sample_chunk_materials().unwrap();
        assert_eq!(reset.block_states(), first.block_states());
        assert_eq!(reset.fluid_update_bitmap(), first.fluid_update_bitmap());
        forward.push(first);
        forward.push(second);
    }
    for (pair, expected) in rows
        .as_chunks::<2>()
        .0
        .iter()
        .zip(forward.as_chunks::<2>().0)
        .rev()
    {
        let r = router(
            pair[0]["settings"].as_str().unwrap(),
            81_985_529_216_486_895,
        );
        for first in expected.iter().rev() {
            let mut session = r
                .fresh_paper_noise_chunk(first.chunk_pos(), budget())
                .unwrap();
            let reverse = session.sample_chunk_materials().unwrap();
            drop(session);
            assert_eq!(reverse.block_states(), first.block_states());
            assert_eq!(reverse.fluid_update_bitmap(), first.fluid_update_bitmap());
            assert_eq!(reverse.resource(), first.resource());
            assert_eq!(reverse.seed(), first.seed());
            assert_eq!(reverse.geometry(), first.geometry());
        }
        drop(r);
    }
}

pub(in crate::density::chunk) fn exhausted(error: &NoiseChunkError, n: usize) {
    match error.kind() {
        NoiseChunkErrorKind::WorkExceeded { limit, used } => assert_eq!((*limit, *used), (n, n)),
        NoiseChunkErrorKind::Density(e) => assert!(
            matches!(e.kind(), crate::density::DensitySampleErrorKind::WorkExceeded { limit, used } if (*limit,*used)==(n,n))
        ),
        other => panic!("expected metered refusal, got {other:?}"),
    }
}

#[test]
fn full_material_exact_budget_poison_and_atomic_reset() {
    let r = router("minecraft:overworld", 0);
    let chunk = ferrumc_math::ChunkPos::new(-1, 0);
    let mut first = r.fresh_paper_noise_chunk(chunk, budget()).unwrap();
    let expected = first.sample_chunk_materials().unwrap();
    let n = first.work_used();
    assert!(n < MAX_NOISE_CHUNK_WORK as usize);
    let mut exact = r
        .fresh_paper_noise_chunk(
            chunk,
            NoiseChunkWorkBudget::new(n.try_into().unwrap()).unwrap(),
        )
        .unwrap();
    let output = exact.sample_chunk_materials().unwrap();
    assert_eq!(exact.work_used(), n);
    assert_eq!(output.block_states(), expected.block_states());
    assert_eq!(output.fluid_update_bitmap(), expected.fluid_update_bitmap());
    let mut short = r
        .fresh_paper_noise_chunk(
            chunk,
            NoiseChunkWorkBudget::new((n - 1).try_into().unwrap()).unwrap(),
        )
        .unwrap();
    exhausted(&short.sample_chunk_materials().unwrap_err().into(), n - 1);
    assert_eq!(short.state.lifecycle, Lifecycle::Poisoned);
    let before = (
        short.work_used(),
        short.surface.len,
        short.state.scalar_counter,
        short.state.array_counter,
        short.state.driver_phase,
    );
    assert!(short.reset(NoiseChunkWorkBudget::new(1).unwrap()).is_err());
    assert_eq!(short.state.lifecycle, Lifecycle::Poisoned);
    assert_eq!(
        (
            short.work_used(),
            short.surface.len,
            short.state.scalar_counter,
            short.state.array_counter,
            short.state.driver_phase
        ),
        before
    );
    assert!(short.sample_chunk_materials().is_err());
    assert!(short.sample_chunk_densities().is_err());
    short.reset(budget()).unwrap();
    let reset = short.sample_chunk_materials().unwrap();
    assert_eq!(reset.block_states(), expected.block_states());
    assert_eq!(reset.fluid_update_bitmap(), expected.fluid_update_bitmap());
}

#[test]
fn material_preparation_exact_budget_failure_poison_and_output_limits() {
    let r = router("minecraft:overworld", 0);
    let mut first = r
        .fresh_paper_noise_chunk(ferrumc_math::ChunkPos::ORIGIN, budget())
        .unwrap();
    let constructor = first.work_used();
    first.state.lifecycle = Lifecycle::Running;
    let run = MaterialRun::new(&mut first, 98_304).unwrap();
    let n = first.work_used();
    assert_eq!(run.aquifer.locations_len(), 315);
    assert_eq!(run.aquifer.statuses_len(), 315);
    assert!(n > constructor);
    let mut exact = r
        .fresh_paper_noise_chunk(
            ferrumc_math::ChunkPos::ORIGIN,
            NoiseChunkWorkBudget::new(n.try_into().unwrap()).unwrap(),
        )
        .unwrap();
    exact.state.lifecycle = Lifecycle::Running;
    assert!(MaterialRun::new(&mut exact, 98_304).is_ok());
    assert_eq!(exact.work_used(), n);
    let mut short = r
        .fresh_paper_noise_chunk(
            ferrumc_math::ChunkPos::ORIGIN,
            NoiseChunkWorkBudget::new((n - 1).try_into().unwrap()).unwrap(),
        )
        .unwrap();
    exhausted(&short.sample_chunk_materials().unwrap_err().into(), n - 1);
    assert_eq!(short.state.lifecycle, Lifecycle::Poisoned);
    assert!(!short.state.interpolating);
    assert_eq!(short.state.scalar_counter, 0);
    for (kind, size, maximum) in [
        (NoiseChunkLimit::MaterialOutput, 4, 512 * 1024),
        (NoiseChunkLimit::FluidBitmap, 8, 16 * 1024),
    ] {
        let count = maximum / size;
        assert_eq!(bytes(kind, 0, count, size, maximum).unwrap(), maximum);
        assert!(
            matches!(bytes(kind,0,count+1,size,maximum).unwrap_err().kind(),NoiseChunkErrorKind::Limit {limit,..} if *limit==kind)
        );
        assert!(bytes(kind, 0, usize::MAX, size, maximum).is_err());
    }
}

#[test]
fn resolver_checked_names_properties_json_and_exact_work() {
    for (source, raw) in [
        (r#"{"Name":"minecraft:stone"}"#, 1),
        (
            r#"{"Name":"minecraft:water","Properties":{"level":"0"}}"#,
            86,
        ),
        (
            r#"{"Name":"minecraft:water","Properties":{"level":"15"}}"#,
            101,
        ),
        (
            r#"{"Name":"minecraft:oak_log","Properties":{"axis":"z"}}"#,
            138,
        ),
    ] {
        let mut work = SampleWork::new(1_000_000);
        assert_eq!(
            blocks::resolve(source, "test", &mut work).unwrap().as_u32(),
            raw
        );
        let n = work.used();
        let mut exact = SampleWork::new(n);
        assert_eq!(
            blocks::resolve(source, "test", &mut exact)
                .unwrap()
                .as_u32(),
            raw
        );
        assert_eq!(exact.used(), n);
        let mut short = SampleWork::new(n - 1);
        exhausted(
            &blocks::resolve(source, "test", &mut short).unwrap_err(),
            n - 1,
        );
    }
    for source in [
        "",
        "{}",
        "[]",
        r#"{"Name":1}"#,
        r#"{"Name":"minecraft:unknown"}"#,
        r#"{"Name":"minecraft:water","extra":1}"#,
        r#"{"Name":"minecraft:water","Properties":null}"#,
        r#"{"Name":"minecraft:water","Properties":{"level":1}}"#,
        r#"{"Name":"minecraft:water","Properties":{"level":"16"}}"#,
        r#"{"Name":"minecraft:stone","Properties":{"level":"0"}}"#,
        r#"{"Name":"minecraft:stone","Name":"minecraft:water"}"#,
        r#"{"Name":"minecraft:water","Properties":{"level":"0","\u006cevel":"1"}}"#,
    ] {
        let error = blocks::resolve(source, "test", &mut SampleWork::new(1_000_000)).unwrap_err();
        assert!(matches!(
            error.kind(),
            NoiseChunkErrorKind::InvalidMaterialState { field: "test" }
        ));
    }
    let mut work = SampleWork::new(1_000_000);
    let maximum = format!(
        "{}{}",
        " ".repeat(4096 - 26),
        r#"{"Name":"minecraft:stone"}"#
    );
    assert_eq!(maximum.len(), 4096);
    assert!(blocks::resolve(&maximum, "test", &mut work).is_ok());
    let used = work.used();
    assert!(matches!(
        blocks::resolve(&(maximum + " "), "test", &mut work)
            .unwrap_err()
            .kind(),
        NoiseChunkErrorKind::Limit {
            limit: NoiseChunkLimit::MaterialResolver,
            ..
        }
    ));
    assert_eq!(used, work.used());
    for fluid in [
        r#"{"Name":"minecraft:oak_log"}"#,
        r#"{"Name":"minecraft:oak_slab","Properties":{"waterlogged":"true"}}"#,
    ] {
        assert!(MaterialBlocks::new(
            r#"{"Name":"minecraft:stone"}"#,
            fluid,
            &mut SampleWork::new(1_000_000)
        )
        .is_err());
    }
}

#[test]
fn malformed_settings_poison_before_driver_and_reset_restores_fresh_run() {
    let r = router("minecraft:overworld", 0);
    let mut s = r
        .fresh_paper_noise_chunk(ferrumc_math::ChunkPos::ORIGIN, budget())
        .unwrap();
    drop(r);
    let metadata = &mut std::sync::Arc::get_mut(&mut s.data).unwrap().metadata;
    let original = metadata.default_fluid;
    metadata.default_fluid = r#"{"Name":"minecraft:water","Properties":{"level":"bad"}}"#;
    assert!(matches!(
        s.sample_chunk_materials().unwrap_err().kind(),
        NoiseChunkErrorKind::InvalidMaterialState {
            field: "default_fluid"
        }
    ));
    assert_eq!(s.state.lifecycle, Lifecycle::Poisoned);
    assert!(!s.state.interpolating);
    assert_eq!(s.state.scalar_counter, 0);
    std::sync::Arc::get_mut(&mut s.data)
        .unwrap()
        .metadata
        .default_fluid = original;
    s.reset(budget()).unwrap();
    assert_eq!(s.state.lifecycle, Lifecycle::Fresh);
    assert!(s.sample_chunk_materials().is_ok());
}

#[test]
fn physical_scaled_halo_and_packed_y_admission_are_distinct() {
    for chunk in [
        ferrumc_math::ChunkPos::new(-1_875_000, 0),
        ferrumc_math::ChunkPos::new(1_874_999, 0),
        ferrumc_math::ChunkPos::new(0, -1_875_000),
        ferrumc_math::ChunkPos::new(0, 1_874_999),
    ] {
        let r = router("minecraft:overworld", 0);
        let mut s = r.fresh_paper_noise_chunk(chunk, budget()).unwrap();
        let domain = MaterialDomain::new(&s).unwrap();
        for x in [-HALO_LIMIT, HALO_LIMIT] {
            let physical = BlockPos::new(x, -64, x);
            domain.center(physical).unwrap();
            domain
                .scaled(
                    BlockPos::new(x.div_euclid(16), (-64_i32).div_euclid(40), x.div_euclid(16)),
                    16,
                )
                .unwrap();
            domain
                .scaled(
                    BlockPos::new(x.div_euclid(64), (-64_i32).div_euclid(40), x.div_euclid(64)),
                    64,
                )
                .unwrap();
        }
        assert!(MaterialDomain::horizontal(BlockPos::new(HALO_LIMIT + 1, 0, 0)).is_err());
        assert!(domain
            .scaled(BlockPos::new(HALO_LIMIT.div_euclid(16) + 1, 0, 0), 16)
            .is_err());
        assert!(s
            .preliminary_surface_level(BlockPos::new(30_000_001, 0, 0))
            .is_err());
        let output = s.sample_chunk_materials().unwrap();
        assert_eq!(output.block_states().len(), 98_304);
        assert!(s
            .preliminary_surface_level(BlockPos::new(-30_000_001, 0, 0))
            .is_err());
    }
    let r = router("minecraft:overworld", 0);
    let mut s = r
        .fresh_paper_noise_chunk(ferrumc_math::ChunkPos::ORIGIN, budget())
        .unwrap();
    s.state.geometry.min_y = -2032;
    assert!(MaterialDomain::new(&s).is_err());
}
