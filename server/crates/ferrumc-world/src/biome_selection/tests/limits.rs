use super::arrays::{noise, noise_budget, work_budget};
use super::*;
use crate::density::{ClimateQuartPos, NoiseChunkErrorKind, NoiseChunkWorkBudget};
use ferrumc_math::{BlockPos, ChunkPos};

fn source(kind: usize, allowance: BiomeWorkBudget) -> BiomeSourceSession {
    match kind {
        0 => BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
            tree("minecraft:nether"),
            allowance,
        )),
        1 => BiomeSourceSession::fixed(
            BiomeResourceId::from_name("minecraft:plains").unwrap(),
            allowance,
        ),
        2 => BiomeSourceSession::the_end(allowance).unwrap(),
        _ => panic!(),
    }
}
fn settings(kind: usize) -> &'static str {
    if kind == 2 {
        "minecraft:end"
    } else {
        "minecraft:nether"
    }
}
fn position(kind: usize) -> ChunkPos {
    if kind == 2 {
        ChunkPos::new(100, 100)
    } else {
        ChunkPos::ORIGIN
    }
}

#[test]
fn actual_tree_build_and_selection_exact_n_n_minus_one_are_atomic_and_resettable() {
    for name in ["minecraft:nether", "minecraft:overworld"] {
        let original = tree(name);
        let used = u32::try_from(original.build_work_used()).unwrap();
        let exact =
            BiomeTree::from_builtin(original.preset(), BiomeBuildBudget::new(used).unwrap())
                .unwrap();
        assert_eq!(exact.build_work_used(), usize::try_from(used).unwrap());
        assert_eq!(exact.node_count(), original.node_count());
        let refusal =
            BiomeTree::from_builtin(original.preset(), BiomeBuildBudget::new(used - 1).unwrap());
        assert!(
            matches!(refusal,Err(BiomeError{kind:BiomeErrorKind::WorkExceeded{limit,used:u}}) if limit==usize::try_from(used-1).unwrap() && limit==u)
        );
        let target = BiomeClimateTarget::from_quantized([0; 6]);
        let mut full = BiomeSelectionSession::new(Arc::clone(&original), work_budget());
        let expected = full.select(target).unwrap();
        let n = u32::try_from(full.work_used()).unwrap();
        let mut exact =
            BiomeSelectionSession::new(Arc::clone(&original), BiomeWorkBudget::new(n).unwrap());
        assert_eq!(exact.select(target).unwrap(), expected);
        let prior = exact.last_selection();
        assert!(matches!(
            exact.select(target).unwrap_err().kind(),
            BiomeErrorKind::WorkExceeded { .. }
        ));
        assert_eq!(exact.last_selection(), prior);
        assert!(exact.is_poisoned());
        assert_eq!(
            exact.select(target).unwrap_err().kind(),
            &BiomeErrorKind::Poisoned
        );
        assert_eq!(exact.work_used(), usize::try_from(n).unwrap());
        exact.reset(work_budget());
        assert_eq!(exact.last_selection(), None);
        assert!(!exact.is_poisoned());
        assert_eq!(exact.work_used(), 0);
        assert_eq!(exact.select(target).unwrap(), expected);
        let mut short = BiomeSelectionSession::new(original, BiomeWorkBudget::new(n - 1).unwrap());
        assert!(matches!(
            short.select(target).unwrap_err().kind(),
            BiomeErrorKind::WorkExceeded { .. }
        ));
        assert_eq!(short.last_selection(), None);
        assert!(short.is_poisoned());
    }
}

#[test]
fn all_source_whole_arrays_exact_work_no_partial_output_poison_and_independent_reset() {
    let geometry = BiomeQuartGeometry::new(0, 256).unwrap();
    for kind in 0..3 {
        let mut original = source(kind, work_budget());
        let mut chunk = noise(settings(kind), 0, position(kind));
        let result = original.sample_chunk_biomes(&mut chunk, geometry).unwrap();
        let n = u32::try_from(original.work_used()).unwrap();
        let mut exact = source(kind, BiomeWorkBudget::new(n).unwrap());
        let mut chunk = noise(settings(kind), 0, position(kind));
        assert_eq!(
            exact
                .sample_chunk_biomes(&mut chunk, geometry)
                .unwrap()
                .values(),
            result.values()
        );
        assert_eq!(exact.work_used(), usize::try_from(n).unwrap());
        let work = exact.work_used();
        let leaf = exact.last_selection();
        assert!(
            matches!(exact.sample_chunk_biomes(&mut chunk,geometry).unwrap_err().kind(),BiomeErrorKind::Noise(e) if matches!(e.kind(),NoiseChunkErrorKind::Lifecycle{..}))
        );
        assert_eq!(exact.work_used(), work);
        assert_eq!(exact.last_selection(), leaf);
        assert!(!exact.is_poisoned());
        let mut short = source(kind, BiomeWorkBudget::new(n - 1).unwrap());
        let mut chunk = noise(settings(kind), 0, position(kind));
        assert!(matches!(
            short
                .sample_chunk_biomes(&mut chunk, geometry)
                .unwrap_err()
                .kind(),
            BiomeErrorKind::WorkExceeded { .. }
        ));
        assert_eq!(short.work_used(), usize::try_from(n - 1).unwrap());
        assert!(short.is_poisoned());
        assert_eq!(short.last_selection(), None);
        assert!(chunk
            .sample_climate(ClimateQuartPos::new(0, 0, 0).unwrap())
            .is_err());
        chunk.reset(noise_budget()).unwrap();
        assert_eq!(
            short
                .sample_chunk_biomes(&mut chunk, geometry)
                .unwrap_err()
                .kind(),
            &BiomeErrorKind::Poisoned
        );
        short.reset(work_budget());
        assert_eq!(short.work_used(), 0);
        assert!(!short.is_poisoned());
        assert_eq!(
            short
                .sample_chunk_biomes(&mut chunk, geometry)
                .unwrap()
                .values(),
            result.values()
        );
        short.reset(work_budget());
        assert!(short.sample_chunk_biomes(&mut chunk, geometry).is_err());
        assert_eq!(short.work_used(), 0);
        assert!(!short.is_poisoned());
        chunk.reset(noise_budget()).unwrap();
        assert_eq!(
            short
                .sample_chunk_biomes(&mut chunk, geometry)
                .unwrap()
                .values(),
            result.values()
        );
    }
}

#[test]
fn whole_array_failure_retains_pre_array_leaf_and_original_shared_meter() {
    let tree = tree("minecraft:nether");
    let target = BiomeClimateTarget::from_quantized([0; 6]);
    let geometry = BiomeQuartGeometry::new(0, 256).unwrap();
    let mut scalar = BiomeSelectionSession::new(Arc::clone(&tree), work_budget());
    let first = scalar.select(target).unwrap();
    let before = scalar.work_used();
    let mut full = BiomeSourceSession::from_multinoise(scalar);
    assert_eq!(full.work_used(), before);
    assert_eq!(full.last_selection(), Some(first));
    let mut chunk = noise("minecraft:nether", 0, ChunkPos::ORIGIN);
    full.sample_chunk_biomes(&mut chunk, geometry).unwrap();
    let n = u32::try_from(full.work_used()).unwrap();
    let mut scalar = BiomeSelectionSession::new(tree, BiomeWorkBudget::new(n - 1).unwrap());
    assert_eq!(scalar.select(target).unwrap(), first);
    let mut short = BiomeSourceSession::from_multinoise(scalar);
    let mut chunk = noise("minecraft:nether", 0, ChunkPos::ORIGIN);
    assert!(short.sample_chunk_biomes(&mut chunk, geometry).is_err());
    assert_eq!(short.last_selection(), Some(first));
    assert!(short.is_poisoned());
    assert_eq!(short.work_used(), usize::try_from(n - 1).unwrap());
}

#[test]
fn noise_execution_failure_poisons_source_and_retains_leaf_but_fixed_never_samples_climate() {
    let geometry = BiomeQuartGeometry::new(0, 256).unwrap();
    for kind in [0, 2] {
        let original = noise(settings(kind), 0, position(kind));
        let construction = u32::try_from(original.work_used()).unwrap();
        let mut chunk = original;
        chunk
            .reset(NoiseChunkWorkBudget::new(construction).unwrap())
            .unwrap();
        let mut source = source(kind, work_budget());
        assert!(matches!(
            source
                .sample_chunk_biomes(&mut chunk, geometry)
                .unwrap_err()
                .kind(),
            BiomeErrorKind::Noise(_)
        ));
        assert!(source.is_poisoned());
        assert_eq!(source.last_selection(), None);
        assert!(chunk
            .sample_climate(ClimateQuartPos::new(0, 0, 0).unwrap())
            .is_err());
    }
    let mut chunk = noise("minecraft:nether", 0, ChunkPos::ORIGIN);
    let construction = u32::try_from(chunk.work_used()).unwrap();
    chunk
        .reset(NoiseChunkWorkBudget::new(construction).unwrap())
        .unwrap();
    let mut fixed = source(1, work_budget());
    let values = fixed.sample_chunk_biomes(&mut chunk, geometry).unwrap();
    assert_eq!(chunk.work_used(), usize::try_from(construction).unwrap());
    assert!(values
        .values()
        .iter()
        .all(|v| v.resource_location() == "minecraft:plains"));
}

#[test]
fn height_accessor_is_explicit_full_bounded_and_rejected_clamping_is_mutation_free() {
    for (min, height) in [
        (1, 256),
        (-63, 384),
        (0, 0),
        (0, 15),
        (0, 17),
        (0, 528),
        (i32::MAX - 15, 32),
    ] {
        assert_eq!(
            BiomeQuartGeometry::new(min, height).unwrap_err().kind(),
            &BiomeErrorKind::InvalidGeometry
        );
    }
    assert_eq!(
        BiomeQuartGeometry::new(i32::MAX - 15, 16)
            .unwrap()
            .sample_count(),
        64
    );
    let mut chunk = noise("minecraft:nether", 0, ChunkPos::ORIGIN);
    let before = chunk.work_used();
    let mut fixed = source(1, work_budget());
    assert_eq!(
        fixed
            .sample_chunk_biomes(&mut chunk, BiomeQuartGeometry::new(16, 128).unwrap())
            .unwrap_err()
            .kind(),
        &BiomeErrorKind::UnsupportedHeightClamping
    );
    assert_eq!(fixed.work_used(), 0);
    assert_eq!(chunk.work_used(), before);
    assert!(!fixed.is_poisoned());
    let values = fixed
        .sample_chunk_biomes(&mut chunk, BiomeQuartGeometry::new(0, 512).unwrap())
        .unwrap();
    assert_eq!(values.values().len(), MAX_BIOME_QUART_SAMPLES);
    assert_eq!(
        values.biome_at(BlockPos::new(0, 511, 0)),
        Some(BiomeResourceId::from_name("minecraft:plains").unwrap())
    );
    assert_eq!(values.biome_at(BlockPos::new(0, 512, 0)), None);
    chunk.sample_chunk_densities().unwrap();
    assert!(fixed
        .sample_chunk_biomes(&mut chunk, BiomeQuartGeometry::new(0, 256).unwrap())
        .is_err());
    for units in [0, MAX_BIOME_BUILD_WORK + 1, u32::MAX] {
        assert!(matches!(
            BiomeBuildBudget::new(units).unwrap_err().kind(),
            BiomeErrorKind::InvalidBudget { .. }
        ));
    }
    for units in [0, MAX_BIOME_SELECTION_WORK + 1, u32::MAX] {
        assert!(matches!(
            BiomeWorkBudget::new(units).unwrap_err().kind(),
            BiomeErrorKind::InvalidBudget { .. }
        ));
    }
}

#[test]
fn end_transform_checked_domain_negative_floors_and_exact_radius() {
    for (quart, expected) in [
        (-7_500_000, -29_999_992),
        (7_499_999, 29_999_992),
        (-261, -1048),
        (-260, -1032),
        (-259, -1032),
        (260, 1048),
    ] {
        let mut work = SampleWork::new(100);
        let pos = ClimateQuartPos::new(quart, 0, 0).unwrap();
        assert_eq!(end::center(pos, &mut work).unwrap().unwrap().x(), expected);
    }
    let mut work = SampleWork::new(100);
    assert!(matches!(
        end::center(ClimateQuartPos::new(7_500_000, 0, 0).unwrap(), &mut work)
            .unwrap_err()
            .kind(),
        BiomeErrorKind::PositionOutOfRange {
            axis: "x",
            coordinate: 30_000_008
        }
    ));
    for x in [-256, -255, 255, 256, 259] {
        let mut work = SampleWork::new(100);
        assert_eq!(
            end::center(ClimateQuartPos::new(x, 0, 0).unwrap(), &mut work).unwrap(),
            None
        );
    }
    assert!(ClimateQuartPos::new(0, i32::MAX / 4 + 1, 0).is_err());
    assert!(ClimateQuartPos::new(0, i32::MIN / 4 - 1, 0).is_err());
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(end::branch(value, &mut SampleWork::new(100)).is_err());
    }
}

#[test]
fn all_eight_actual_declared_tie_controls_and_independent_session_histories_match() {
    let preset = BiomeParametersResourceId::from_name("minecraft:nether").unwrap();
    let mut left = [[0; 2]; 7];
    left[0] = [-10000, -10000];
    let mut right = [[0; 2]; 7];
    right[0] = [10000, 10000];
    let mut parent = [[0; 2]; 7];
    parent[0] = [-10000, 10000];
    let tree = Arc::new(BiomeTree {
        preset,
        nodes: vec![
            Node {
                axes: left,
                kind: NodeKind::Leaf(0),
            },
            Node {
                axes: right,
                kind: NodeKind::Leaf(1),
            },
            Node {
                axes: parent,
                kind: NodeKind::Branch {
                    children: [0, 1, 0, 0, 0, 0],
                    len: 2,
                },
            },
        ],
        root: 2,
        leaf_nodes: vec![0, 1],
        edges: 2,
        depth: 2,
        retained_bytes: 0,
        build_work: 0,
        build_scratch_peak: 0,
    });
    let mut forward = BiomeSelectionSession::new(Arc::clone(&tree), work_budget());
    let mut reverse = BiomeSelectionSession::new(tree, work_budget());
    let mut count = 0;
    for line in QUERIES.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        if row["preset"] != "declared-two-point-tie" {
            continue;
        }
        let session = if row["group"] == "tie-forward" {
            &mut forward
        } else {
            &mut reverse
        };
        let target = BiomeClimateTarget::from_quantized(std::array::from_fn(|i| {
            row["target"][i].as_i64().unwrap()
        }));
        assert_eq!(
            session.select(target).unwrap().parameter_ordinal(),
            usize::try_from(row["indexed"].as_u64().unwrap()).unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 8);
    assert_eq!(forward.last_selection().unwrap().parameter_ordinal(), 1);
    assert_eq!(reverse.last_selection().unwrap().parameter_ordinal(), 0);
    forward.reset(work_budget());
    assert_eq!(reverse.last_selection().unwrap().parameter_ordinal(), 0);
    assert_eq!(forward.last_selection(), None);
}
