use super::*;
use crate::density::chunk::state::Cache;
use crate::density::graph::{Shift, Unit};
use std::collections::BTreeMap;

pub(in crate::density::chunk::cold) fn class(
    c: &Cold<'_>,
    kind: &BoundKind,
    index: usize,
) -> &'static str {
    if let Some(wrapper) = c.state.wrapper_nodes.get(index).copied().flatten() {
        return match c.state.wrappers[wrapper].cache {
            Cache::Once { .. } => "NoiseChunk$CacheOnce",
            Cache::TwoD { .. } => "NoiseChunk$Cache2D",
            Cache::Flat(_) => "NoiseChunk$FlatCache",
            Cache::Interpolated(_) => "NoiseChunk$NoiseInterpolator",
            Cache::Cell(_) => "NoiseChunk$CacheAllInCell",
        };
    }
    match kind {
        BoundKind::Alias(_) => "$invalid",
        BoundKind::MulOrAdd { .. } => "DensityFunctions$MulOrAdd",
        BoundKind::Original(kind) => match kind {
            NodeKind::Constant(_) => "DensityFunctions$Constant",
            NodeKind::Holder { .. } => "DensityFunctions$HolderHolder",
            NodeKind::Coordinate { .. } => "DensityFunctions$Spline$Coordinate",
            NodeKind::Binary { .. } => "DensityFunctions$Ap2",
            NodeKind::Unary { .. } => "DensityFunctions$Mapped",
            NodeKind::Marker { .. } => "DensityFunctions$Marker",
            NodeKind::BlendDensity(_) => "DensityFunctions$BlendDensity",
            NodeKind::Clamp { .. } => "DensityFunctions$Clamp",
            NodeKind::RangeChoice { .. } => "DensityFunctions$RangeChoice",
            NodeKind::Gradient { .. } => "DensityFunctions$YClampedGradient",
            NodeKind::Noise { .. } => "DensityFunctions$Noise",
            NodeKind::Shift { shift, .. } => match shift {
                Shift::All => "DensityFunctions$Shift",
                Shift::A => "DensityFunctions$ShiftA",
                Shift::B => "DensityFunctions$ShiftB",
            },
            NodeKind::ShiftedNoise { .. } => "DensityFunctions$ShiftedNoise",
            NodeKind::Weird { .. } => "DensityFunctions$WeirdScaledSampler",
            NodeKind::Blended { .. } => "synth.BlendedNoise",
            NodeKind::Unit(unit) => match unit {
                Unit::EndIslands => "DensityFunctions$EndIslandDensityFunction",
                Unit::BlendAlpha => "DensityFunctions$BlendAlpha",
                Unit::BlendOffset => "DensityFunctions$BlendOffset",
                Unit::Beardifier => "DensityFunctions$BeardifierMarker",
            },
            NodeKind::Spline(_) => "DensityFunctions$Spline",
            NodeKind::SplineConstant(_) => "CubicSpline$Constant",
            NodeKind::SplineMultipoint { .. } => "CubicSpline$Multipoint",
        },
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One original constructor trace verifies visitation, equality partition, mutable registration and shared climate roots together."
)]
fn original_equals_partition_registration_all_15_and_climate_roots() {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/noise_chunk_1_21_8");
    let classes: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(p.join("classes.json")).unwrap()).unwrap();
    let rows = std::fs::read_to_string(p.join("paper-constructors.jsonl")).unwrap();
    let mut decoded = 0;
    for line in rows.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let case = row["case"].as_str().unwrap();
        let parts = case.split('|').collect::<Vec<_>>();
        if parts[1] != "decoded" {
            continue;
        }
        let r = router(parts[0], parts[2].parse().unwrap());
        let mut work = SampleWork::new(budget().units() as usize);
        let state = ChunkState::new(
            &r.data,
            ChunkPos::ORIGIN,
            r.data.metadata.geometry,
            &mut work,
        )
        .unwrap();
        let mut c = Cold::new(&r.data, state, &mut work).unwrap();
        let (fields, material, climate, seeds) = c.roots().unwrap();
        let mut ids = BTreeMap::new();
        for (expected, &actual) in row["constructor_registration"]
            .as_array()
            .unwrap()
            .iter()
            .zip(c.state.interpolators.iter().chain(&c.state.cells))
        {
            ids.insert(
                expected["id"].as_u64().unwrap(),
                c.state.wrappers[actual].node,
            );
            assert_eq!(
                expected["class"].as_str().unwrap(),
                class(
                    &c,
                    &c.view.nodes[c.state.wrappers[actual].node].kind,
                    c.state.wrappers[actual].node
                )
            );
        }
        assert_eq!(row["material_id"].as_u64().map(|i| ids[&i]), Some(material));
        assert_eq!(
            row["after_climate"]["map_entries"].as_u64().unwrap() as usize,
            c.wrap_entries
        );
        c.trace = Some(Vec::with_capacity(131_072));
        let mut roots = [0; 15];
        for (slot, seed) in roots.iter_mut().zip(seeds) {
            *slot = c.wrap(seed, 1).unwrap();
        }
        let trace = c.trace.take().unwrap();
        let expected = row["visits"].as_array().unwrap();
        assert_eq!(trace.len(), expected.len(), "{case}");
        let mut partitions = BTreeMap::new();
        for (i, ((key, kind, value, valuekind), expected)) in trace.iter().zip(expected).enumerate()
        {
            let next = partitions.len();
            let partition = *partitions.entry(*key).or_insert(next);
            assert_eq!(
                *kind,
                classes[expected[0].as_u64().unwrap() as usize],
                "{case} visit{i}"
            );
            assert_eq!(
                partition,
                expected[1].as_u64().unwrap() as usize,
                "{case} equals visit{i}"
            );
            assert_eq!(
                *valuekind,
                classes[expected[4].as_u64().unwrap() as usize],
                "{case} resultclass visit{i}"
            );
            if valuekind.starts_with("NoiseChunk$") {
                let expectedid = expected[3].as_u64().unwrap();
                let prior = ids.entry(expectedid).or_insert(*value);
                assert_eq!(*prior, *value, "{case} shared wrapper visit{i}");
            }
        }
        for ((expected, &actual), &first) in row["roots"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&roots)
            .zip(&fields)
        {
            assert_eq!(
                expected["class"].as_str().unwrap(),
                class(&c, &c.view.nodes[actual].kind, actual),
                "{case} root"
            );
            if class(&c, &c.view.nodes[actual].kind, actual).starts_with("NoiseChunk$") {
                assert_eq!(ids[&expected["id"].as_u64().unwrap()], actual);
                assert_eq!(actual, first);
            }
        }
        for (&actual, expected) in climate.iter().zip(row["climate_roots"].as_array().unwrap()) {
            if class(&c, &c.view.nodes[actual].kind, actual).starts_with("NoiseChunk$") {
                assert_eq!(
                    ids[&expected.as_u64().unwrap()],
                    actual,
                    "{case} climate wrapper"
                );
            }
        }
        decoded += 1;
    }
    assert_eq!(decoded, 49);
}
