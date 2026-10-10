use super::*;
use crate::density::evaluate::{self, EndRadialReference, MarkerEvaluator};
use serde_json::Value;

pub(super) fn records() -> Vec<Value> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/noise_chunk_1_21_8/paper-contracts.jsonl");
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn session(row: &Value) -> NoiseChunkSession {
    let mut parts = row["case"].as_str().unwrap().split('|');
    let name = parts.next().unwrap();
    let seed = parts.next().unwrap().parse().unwrap();
    router(name, seed)
        .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
        .unwrap()
}
fn bits(value: &Value) -> u64 {
    u64::from_str_radix(value.as_str().unwrap(), 16).unwrap()
}
fn compare(values: &[f64], expected: &Value, label: &str) {
    let expected = expected.as_array().unwrap();
    assert_eq!(values.len(), expected.len(), "{label}");
    for (i, (v, e)) in values.iter().zip(expected).enumerate() {
        assert_eq!(v.to_bits(), bits(e), "{label} index{i}");
    }
}
fn slices(s: &NoiseChunkSession, row: &Value, first: bool, label: &str) {
    let arrays = row.as_array().unwrap();
    assert_eq!(arrays.len(), s.state.interpolators.len());
    for (i, (&id, array)) in s.state.interpolators.iter().zip(arrays).enumerate() {
        let Cache::Interpolated(v) = &s.state.wrappers[id].cache else {
            panic!()
        };
        let values = if first { &v.slice0 } else { &v.slice1 };
        let rows = array.as_array().unwrap();
        assert_eq!(rows.len(), s.state.count_xz + 1);
        for (j, (v, e)) in values.chunks(s.state.count_y + 1).zip(rows).enumerate() {
            compare(v, e, &format!("{label} interp{i} column{j}"));
        }
    }
}

#[test]
fn actual_preliminary_and_cached_climate_all_settings_seeds_boundaries() {
    let mut counts = [0; 2];
    for row in records() {
        match row["kind"].as_str().unwrap() {
            "preliminary" => {
                let mut s = session(&row);
                for column in row["columns"].as_array().unwrap() {
                    let pos = BlockPos::new(
                        column["x"].as_i64().unwrap().try_into().unwrap(),
                        i32::MIN,
                        column["z"].as_i64().unwrap().try_into().unwrap(),
                    );
                    assert_eq!(
                        s.preliminary_surface_level(pos).unwrap(),
                        column["surface"].as_i64().unwrap() as i32,
                        "{} {pos:?}",
                        row["case"]
                    );
                    assert_eq!(
                        s.surface.len,
                        column["cache_size"].as_u64().unwrap() as usize
                    );
                }
                counts[0] += 1;
            }
            "climate" => {
                let mut s = session(&row);
                for sample in row["samples"].as_array().unwrap() {
                    let p = sample["position"].as_array().unwrap();
                    let p = ClimateQuartPos::new(
                        p[0].as_i64().unwrap().try_into().unwrap(),
                        p[1].as_i64().unwrap().try_into().unwrap(),
                        p[2].as_i64().unwrap().try_into().unwrap(),
                    )
                    .unwrap();
                    let result = s.sample_climate(p).unwrap();
                    for (i, axis) in ClimateAxis::ALL.iter().enumerate() {
                        assert_eq!(
                            result.value(*axis).to_bits(),
                            bits(&sample["f32_bits"][i]) as u32,
                            "{} climate{axis:?} {p:?}",
                            row["case"]
                        );
                        assert_eq!(
                            result.quantized_value(*axis),
                            sample["quantized"][i].as_i64().unwrap()
                        );
                    }
                }
                counts[1] += 1;
            }
            _ => {}
        }
    }
    assert_eq!(counts, [49, 49]);
}

struct BarePoint<'a>(&'a [crate::noise::NormalNoise]);
impl MarkerEvaluator for BarePoint<'_> {
    fn end_reference(&self) -> EndRadialReference {
        EndRadialReference::Paper1_21_8Build60
    }
    fn normal_sampler<'a>(
        &'a self,
        _data: &'a BoundRouter,
        index: usize,
    ) -> Option<&'a crate::noise::NormalNoise> {
        self.0.get(index)
    }
    fn sample_marker(
        &mut self,
        data: &BoundRouter,
        view: &BoundView,
        _root: usize,
        input: usize,
        pos: BlockPos,
        work: &mut SampleWork,
        field: &'static str,
    ) -> Result<f64, DensitySampleError> {
        evaluate::visit_with(data, view, input, pos, work, field, self)
    }
}
fn foreign(row: &Value, label: &str, wrapped: bool) {
    let mut s = session(row);
    // The bare reference is the seeded router before NoiseChunk wrapping.
    // Its acceptance-only copy also needs the private Paper Blended basis.
    let mut bare = BoundView {
        pass: s.data.router.pass,
        nodes: s.data.router.nodes.clone(),
    };
    if !wrapped {
        for node in &mut bare.nodes {
            if let Some(noise) = &node.blended {
                node.blended = Some(std::sync::Arc::new(
                    noise.paper_build60_basis(&mut s.work).unwrap(),
                ));
            }
        }
    }
    for point in row[label].as_array().unwrap() {
        let p: Vec<i32> = serde_json::from_str(point["case"].as_str().unwrap()).unwrap();
        let p = BlockPos::new(p[0], p[1], p[2]);
        let value = if wrapped {
            s.state
                .sample(&s.data, &s.view, s.fields[10], p, false, &mut s.work)
                .unwrap()
        } else {
            evaluate::visit_with(
                &s.data,
                &bare,
                s.data.fields[11],
                p,
                &mut s.work,
                "bare_final",
                &mut BarePoint(&s.state.normal_noises),
            )
            .unwrap()
        };
        assert_eq!(
            value.to_bits(),
            bits(&point["bits"]),
            "{} {label} {p:?}",
            row["case"]
        );
    }
}
fn driver_probe(row: &Value) {
    let mut s = session(row);
    s.state.lifecycle = Lifecycle::Running;
    s.initialize_driver().unwrap();
    s.fill_slice(true, 0).unwrap();
    slices(&s, &row["initial_slice0"], true, "initial");
    s.fill_slice(false, 1).unwrap();
    s.state.start = BlockPos::new(0, s.state.start.y(), s.state.start.z());
    slices(&s, &row["advanced_slice1"], false, "advanced");
    s.select_cell(0, 0).unwrap();
    for ((&id, array), i) in s
        .state
        .cells
        .iter()
        .zip(row["first_cell_arrays"].as_array().unwrap())
        .zip(0..)
    {
        let Cache::Cell(v) = &s.state.wrappers[id].cache else {
            panic!()
        };
        compare(v, array, &format!("{} cell{i}", row["case"]));
    }
    for (&id, corners) in s
        .state
        .interpolators
        .iter()
        .zip(row["first_cell_corners"].as_array().unwrap())
    {
        let Cache::Interpolated(v) = &s.state.wrappers[id].cache else {
            panic!()
        };
        compare(&v.corners, corners, "corners");
    }
    for point in row["driver_samples"].as_array().unwrap() {
        let p: Vec<i32> = serde_json::from_str(point["case"].as_str().unwrap()).unwrap();
        s.state.in_x = p[0];
        s.state.in_y = p[1];
        s.state.in_z = p[2];
        s.update_y(f64::from(p[1]) / s.state.height as f64).unwrap();
        s.update_x(f64::from(p[0]) / s.state.width as f64).unwrap();
        s.state.scalar_counter = s.state.scalar_counter.wrapping_add(1);
        s.update_z(f64::from(p[2]) / s.state.width as f64).unwrap();
        let value = s.sample_material().unwrap();
        assert_eq!(
            value.to_bits(),
            bits(&point["bits"]),
            "{} driver-point",
            row["case"]
        );
        let context = &point["context"];
        assert_eq!(i64::from(s.state.pos().x()), context["x"].as_i64().unwrap());
        assert_eq!(i64::from(s.state.pos().y()), context["y"].as_i64().unwrap());
        assert_eq!(i64::from(s.state.pos().z()), context["z"].as_i64().unwrap());
        assert_eq!(
            s.state.scalar_counter,
            context["interpolationCounter"].as_i64().unwrap()
        );
        assert_eq!(
            s.state.array_counter,
            context["arrayInterpolationCounter"].as_i64().unwrap()
        );
        assert_eq!(
            s.state.array_index,
            context["arrayIndex"].as_u64().unwrap() as usize
        );
    }
    s.swap_slices().unwrap();
    slices(&s, &row["after_swap_slice0"], true, "swapped0");
    slices(&s, &row["after_swap_slice1"], false, "swapped1");
    assert_eq!(
        s.state.scalar_counter,
        row["scalar_counter"].as_i64().unwrap()
    );
    assert_eq!(
        s.state.array_counter,
        row["array_counter"].as_i64().unwrap()
    );
}
#[test]
fn actual_initial_bare_final_slices_cells_corners_driver_and_swap_counters() {
    let mut count = 0;
    for row in records().iter().filter(|r| r["kind"] == "diagnostic") {
        foreign(row, "initial_foreign", true);
        foreign(row, "bare_final_foreign", false);
        driver_probe(row);
        count += 1;
    }
    assert_eq!(count, 49);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Original RNG offset/permutation and bound noise stacks jointly verify the private Paper basis without changing public Vanilla streams."
)]
fn actual_paper_rng_offset_permutation_and_bound_normal_offsets() {
    let rows = records();
    let mut counts = [0; 2];
    for row in &rows {
        match row["kind"].as_str().unwrap() {
            "random" => {
                let mut p = row["case"].as_str().unwrap().split('|');
                let algorithm = if p.next().unwrap() == "XoroshiroRandomSource" {
                    crate::random::RandomAlgorithm::Xoroshiro128PlusPlus
                } else {
                    crate::random::RandomAlgorithm::Legacy
                };
                let seed = WorldSeed::new(p.next().unwrap().parse().unwrap());
                let mut r = crate::random::WorldgenRandomSource::new(algorithm, seed);
                for v in row["next_double_bits"].as_array().unwrap() {
                    assert_eq!(f64::from(r.next_double() as f32).to_bits(), bits(v));
                }
                let mut r = crate::random::WorldgenRandomSource::new(algorithm, seed);
                let vanilla = crate::noise::ImprovedNoise::new(&mut r);
                let paper = vanilla
                    .paper_build60_basis(&mut SampleWork::new(259))
                    .unwrap();
                let p = paper.offsets();
                for (v, e) in [p.x, p.y, p.z]
                    .iter()
                    .zip(row["offset_bits"].as_array().unwrap())
                {
                    assert_eq!(v.to_bits(), bits(e));
                }
                for (v, e) in paper
                    .permutation()
                    .iter()
                    .zip(row["permutation"].as_array().unwrap())
                {
                    assert_eq!(u64::from(*v), e.as_u64().unwrap());
                }
                counts[0] += 1;
            }
            "bound-noise-offsets" => {
                let s = router(row["case"].as_str().unwrap(), 0)
                    .fresh_paper_noise_chunk(ChunkPos::ORIGIN, budget())
                    .unwrap();
                for id in s
                    .view
                    .nodes
                    .iter()
                    .filter_map(|n| n.noise)
                    .collect::<std::collections::BTreeSet<_>>()
                {
                    let resource = s.data.noises[id].0;
                    let Some(noise) = row["noises"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|n| n["case"].as_str() == Some(resource.resource_location()))
                    else {
                        // RandomState retains these legacy special samplers outside its named-noise map.
                        assert_eq!(
                            s.data.metadata.algorithm,
                            crate::random::RandomAlgorithm::Legacy
                        );
                        assert!(matches!(
                            resource.resource_location(),
                            "minecraft:offset" | "minecraft:temperature" | "minecraft:vegetation"
                        ));
                        continue;
                    };
                    let paper = &s.state.normal_noises[id];
                    let vanilla = &s.data.noises[id].1;
                    for (field, stack, original) in [
                        ("first", paper.first(), vanilla.first()),
                        ("second", paper.second(), vanilla.second()),
                    ] {
                        let octaves = noise[field].as_array().unwrap();
                        assert_eq!(octaves.len(), stack.amplitudes().len());
                        for (i, octave) in octaves.iter().enumerate() {
                            let n = stack.octave_noise(octaves.len() - 1 - i);
                            if octave.is_null() {
                                assert!(n.is_none());
                                continue;
                            }
                            let n = n.unwrap();
                            let p = n.offsets();
                            for (v, e) in [p.x, p.y, p.z]
                                .iter()
                                .zip(octave["offset_bits"].as_array().unwrap())
                            {
                                assert_eq!(
                                    v.to_bits(),
                                    bits(e),
                                    "{} {} {field}/{i}",
                                    row["case"],
                                    noise["case"]
                                );
                            }
                            assert_eq!(
                                n.permutation(),
                                original
                                    .octave_noise(octaves.len() - 1 - i)
                                    .unwrap()
                                    .permutation()
                            );
                        }
                    }
                }
                counts[1] += 1;
            }
            _ => {}
        }
    }
    assert_eq!(counts, [14, 7]);
}

#[test]
fn actual_lerp3_and_normal_update_order_at_fraction_boundaries() {
    use super::wrappers::{marker, original, probe};
    use crate::density::graph::{Marker, NodeKind};
    let mut s = probe(vec![(original(NodeKind::Constant(23.0)), "child")], 0);
    let root = marker(&mut s, Marker::Interpolated, 0);
    let id = s.state.wrapper_nodes[root].unwrap();
    if let Cache::Interpolated(v) = &mut s.state.wrappers[id].cache {
        v.corners = [
            1e16,
            1.0,
            -1e16,
            3.25,
            f64::from_bits(1.0_f64.to_bits() + 1),
            -1e-20,
            17.0,
            -37.0,
        ];
    }
    s.state.in_x = 1;
    s.state.in_y = 3;
    s.state.in_z = 2;
    let mut count = 0;
    for row in records().iter().filter(|r| r["kind"] == "wrapper") {
        let case = row["case"].as_str().unwrap();
        if case == "interpolator-fillingCell-actual-lerp3" {
            s.state.interpolating = true;
            s.state.filling_cell = true;
        } else if let Some(fraction) = case.strip_prefix("interpolator-normal-update|") {
            s.state.lifecycle = Lifecycle::Running;
            s.state.driver_phase = super::super::state::DriverPhase::CellSelected;
            s.state.interpolating = true;
            s.state.filling_cell = false;
            let fraction = f64::from_bits(u64::from_str_radix(fraction, 16).unwrap());
            s.update_y(0.375).unwrap();
            s.update_x(fraction).unwrap();
            s.update_z(0.5).unwrap();
        } else {
            continue;
        }
        let value = s
            .state
            .sample(&s.data, &s.view, root, s.state.pos(), true, &mut s.work)
            .unwrap();
        assert_eq!(value.to_bits(), bits(&row["result_bits"]), "{case}");
        count += 1;
    }
    assert_eq!(count, 8);
}

#[test]
fn actual_paper_end_foreign_interpolator_all_658_reordered_requests() {
    use super::wrappers::{marker, original, probe};
    use crate::density::graph::{Marker, NodeKind, Unit};
    use std::collections::BTreeMap;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/noise_chunk_1_21_8/paper-seams.jsonl");
    let rows = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|r| r["kind"] == "end-foreign-fallback")
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 658);
    for reverse in [false, true] {
        let mut sessions = BTreeMap::new();
        for index in 0..rows.len() {
            let row = &rows[if reverse {
                rows.len() - 1 - index
            } else {
                index
            }];
            let request = &row["frozen_request"];
            let seed = request["seed"].as_str().unwrap().parse::<i64>().unwrap();
            let (s, root) = sessions.entry(seed).or_insert_with(|| {
                let mut s = probe(vec![(original(NodeKind::Unit(Unit::EndIslands)), "end")], 0);
                s.data = router("minecraft:end", seed).data;
                let root = marker(&mut s, Marker::Interpolated, 0);
                (s, root)
            });
            let pos = BlockPos::new(
                request["x"].as_i64().unwrap() as i32,
                request["y"].as_i64().unwrap() as i32,
                request["z"].as_i64().unwrap() as i32,
            );
            let value = s
                .state
                .sample(&s.data, &s.view, *root, pos, false, &mut s.work)
                .unwrap();
            assert_eq!(
                value.to_bits(),
                bits(&row["wrapper_bits"]),
                "{} reverse={reverse}",
                row["case"]
            );
            assert!(!s.state.interpolating);
        }
    }
}
