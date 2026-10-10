//! Independent public-surface contracts and actual original-JAR float32 goldens.

use std::collections::BTreeMap;

use ferrumc_world::spline::{
    CubicSpline, SplineBuildError, SplineCoordinateId, SplineCoordinateRange, SplineFloatField,
    SplineNodeId, SplineNodeSpec, SplinePoint, SplineSampleError, MAX_SPLINE_COORDINATES,
    MAX_SPLINE_DEPTH, MAX_SPLINE_NODES, MAX_SPLINE_POINTS, MAX_SPLINE_POINTS_PER_NODE,
    MAX_SPLINE_SAMPLE_WORK,
};

fn node(index: usize) -> SplineNodeId {
    SplineNodeId::new(u32::try_from(index).expect("test node index"))
}

fn coordinate(index: usize) -> SplineCoordinateId {
    SplineCoordinateId::new(u32::try_from(index).expect("test coordinate index"))
}

fn point(location: f32, child: usize, derivative: f32) -> SplinePoint {
    SplinePoint::new(location, node(child), derivative).expect("finite test point")
}

fn constant(value: f32) -> SplineNodeSpec<'static> {
    SplineNodeSpec::constant(value).expect("finite test constant")
}

fn multi(coord: usize, points: &[SplinePoint]) -> SplineNodeSpec<'_> {
    SplineNodeSpec::multipoint(coordinate(coord), points).expect("ordered test points")
}

fn sample(spline: &CubicSpline, values: &[f32]) -> f32 {
    spline
        .sample_with(MAX_SPLINE_SAMPLE_WORK, &mut |id| {
            values
                .get(id.index() as usize)
                .copied()
                .ok_or("missing coordinate")
        })
        .expect("finite bounded sample")
}

enum InputNode {
    Constant(f32),
    Multi {
        coordinate: usize,
        points: Vec<SplinePoint>,
    },
}

struct GoldenSample {
    values: Vec<f32>,
    bits: u32,
    trace: Vec<u32>,
    finite: bool,
}

struct GoldenFailure {
    values: Vec<f32>,
    fail_at: usize,
    trace: Vec<u32>,
}

#[derive(Default)]
struct GoldenCase {
    coordinates: Vec<SplineCoordinateRange>,
    nodes: Vec<InputNode>,
    root: usize,
    min: u32,
    max: u32,
    samples: Vec<GoldenSample>,
    failures: Vec<GoldenFailure>,
}

fn bits(value: &str) -> u32 {
    u32::from_str_radix(value, 16).expect("original float bits")
}

fn float(value: &str) -> f32 {
    f32::from_bits(bits(value))
}

fn usize_value(value: &str) -> usize {
    value.parse().expect("original specification index")
}

fn values(value: &str) -> Vec<f32> {
    if value == "-" {
        Vec::new()
    } else {
        value.split(',').map(float).collect()
    }
}

fn requests(value: &str) -> Vec<u32> {
    if value == "-" {
        Vec::new()
    } else {
        value
            .split(',')
            .map(|id| id.parse().expect("original callback index"))
            .collect()
    }
}

fn goldens() -> BTreeMap<String, GoldenCase> {
    let mut cases = BTreeMap::<String, GoldenCase>::new();
    for line in include_str!("fixtures/cubic_spline_1_21_8.tsv").lines() {
        if line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        let case = cases.entry(fields[1].to_owned()).or_default();
        match fields[0] {
            "C" => {
                assert_eq!(usize_value(fields[2]), case.coordinates.len());
                let min = float(fields[3]);
                let max = float(fields[4]);
                case.coordinates
                    .push(if min == f32::NEG_INFINITY && max == f32::INFINITY {
                        SplineCoordinateRange::unlimited()
                    } else {
                        SplineCoordinateRange::bounded(min, max).expect("finite original range")
                    });
            }
            "N" => {
                assert_eq!(usize_value(fields[2]), case.nodes.len());
                let input = if fields[3] == "constant" {
                    InputNode::Constant(float(fields[4]))
                } else {
                    assert_eq!(fields[3], "multi");
                    InputNode::Multi {
                        coordinate: usize_value(fields[4]),
                        points: fields[5]
                            .split(';')
                            .map(|encoded| {
                                let p: Vec<_> = encoded.split(',').collect();
                                point(float(p[0]), usize_value(p[1]), float(p[2]))
                            })
                            .collect(),
                    }
                };
                case.nodes.push(input);
            }
            "D" => {
                case.root = usize_value(fields[2]);
                case.min = bits(fields[3]);
                case.max = bits(fields[4]);
            }
            "S" | "X" => {
                assert_eq!(usize_value(fields[2]), case.samples.len());
                case.samples.push(GoldenSample {
                    values: values(fields[3]),
                    bits: bits(fields[4]),
                    trace: requests(fields[5]),
                    finite: fields[0] == "S",
                });
            }
            "E" => {
                assert_eq!(usize_value(fields[2]), case.failures.len());
                case.failures.push(GoldenFailure {
                    values: values(fields[3]),
                    fail_at: usize_value(fields[4]),
                    trace: requests(fields[5]),
                });
            }
            other => panic!("unexpected original record {other}"),
        }
    }
    cases
}

#[test]
fn original_jar_bits_diagnostics_and_callback_sequences() {
    let cases = goldens();
    assert_eq!(cases.len(), 38);
    let mut successful = 0;
    let mut rejected = 0;
    let mut diagnostic_nan = false;
    for (name, case) in cases {
        let specs: Vec<_> = case
            .nodes
            .iter()
            .map(|input| match input {
                InputNode::Constant(value) => constant(*value),
                InputNode::Multi { coordinate, points } => multi(*coordinate, points),
            })
            .collect();
        let spline = CubicSpline::new(&case.coordinates, &specs, node(case.root))
            .expect("original finite-input graph within resource caps");
        assert_eq!(spline.min_value().to_bits(), case.min, "minimum {name}");
        assert_eq!(spline.max_value().to_bits(), case.max, "maximum {name}");
        diagnostic_nan |= spline.min_value().is_nan() || spline.max_value().is_nan();
        for (index, expected) in case.samples.iter().enumerate() {
            let mut trace = Vec::new();
            let result = spline.sample_with(MAX_SPLINE_SAMPLE_WORK, &mut |id| {
                trace.push(id.index());
                expected
                    .values
                    .get(id.index() as usize)
                    .copied()
                    .ok_or("missing original coordinate")
            });
            if expected.finite {
                successful += 1;
                assert_eq!(
                    result.expect("finite original sample").to_bits(),
                    expected.bits,
                    "sample {name}:{index}"
                );
            } else if name == "child_overflow" {
                rejected += 1;
                assert_eq!(
                    result.expect_err("additional Rust child rejection"),
                    SplineSampleError::NonFiniteResult {
                        node: node(1),
                        bits: f32::INFINITY.to_bits(),
                    }
                );
                assert!(f32::from_bits(expected.bits).is_nan());
                assert_eq!(expected.trace, [0, 1, 2], "original keeps evaluating");
                assert_eq!(trace, expected.trace[..2], "Rust stops at the child");
                continue;
            } else {
                rejected += 1;
                let SplineSampleError::NonFiniteResult { bits, .. } =
                    result.expect_err("explicit Rust nonfinite rejection")
                else {
                    panic!("wrong failure for nonfinite original sample {name}:{index}");
                };
                assert_eq!(
                    bits, expected.bits,
                    "original nonfinite arithmetic {name}:{index}"
                );
            }
            assert_eq!(trace, expected.trace, "requests {name}:{index}");
        }
    }
    assert!(successful > 7_000, "live oracle coverage");
    assert!(
        rejected > 200,
        "original nonfinite behavior classified separately"
    );
    assert!(
        diagnostic_nan,
        "Java diagnostics are retained without guarding finite samples"
    );
}

#[test]
fn original_source_exception_stops_at_the_same_request() {
    let mut failures = 0;
    for (name, case) in goldens() {
        let specs: Vec<_> = case
            .nodes
            .iter()
            .map(|input| match input {
                InputNode::Constant(value) => constant(*value),
                InputNode::Multi { coordinate, points } => multi(*coordinate, points),
            })
            .collect();
        let spline = CubicSpline::new(&case.coordinates, &specs, node(case.root))
            .expect("original exception probe graph");
        for expected in case.failures {
            failures += 1;
            let mut trace = Vec::new();
            let error = spline
                .sample_with(MAX_SPLINE_SAMPLE_WORK, &mut |id| {
                    trace.push(id.index());
                    if trace.len() == expected.fail_at {
                        Err("original probe source failure")
                    } else {
                        Ok(expected.values[id.index() as usize])
                    }
                })
                .expect_err("typed source error");
            assert_eq!(trace, expected.trace, "original error requests {name}");
            assert_eq!(
                error,
                SplineSampleError::CoordinateSource {
                    coordinate: SplineCoordinateId::new(*trace.last().expect("failing request")),
                    source: "original probe source failure",
                }
            );
        }
    }
    assert!(failures > 40, "live original exception probes");
}

#[test]
fn nonfinite_child_stops_before_right_callback_and_preserves_graph() {
    let left = [point(0.0, 0, f32::MAX)];
    let right = [point(0.0, 0, 0.0)];
    let root = [point(0.0, 1, 0.0), point(1.0, 2, 0.0)];
    let spline = CubicSpline::new(
        &[SplineCoordinateRange::unlimited(); 3],
        &[
            constant(1.0),
            multi(1, &left),
            multi(2, &right),
            multi(0, &root),
        ],
        node(3),
    )
    .expect("nested overflow graph");
    let before = (spline.min_value().to_bits(), spline.max_value().to_bits());
    let mut trace = Vec::new();
    let error = spline
        .sample_with(MAX_SPLINE_SAMPLE_WORK, &mut |id| {
            trace.push(id.index());
            match id.index() {
                0 => Ok(0.0),
                1 => Ok(f32::MAX),
                _ => Err("right callback must not execute"),
            }
        })
        .expect_err("checked child overflow");
    assert_eq!(
        error,
        SplineSampleError::NonFiniteResult {
            node: node(1),
            bits: f32::INFINITY.to_bits(),
        }
    );
    assert_eq!(trace, [0, 1]);
    assert_eq!(
        before,
        (spline.min_value().to_bits(), spline.max_value().to_bits())
    );
    assert_eq!(sample(&spline, &[0.0, 0.0, 0.0]), 1.0);
}

#[test]
fn malformed_fields_locations_and_indices_are_classified() {
    for value in [
        f32::NAN,
        f32::from_bits(0x7fa1_2345),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        assert_eq!(
            SplineNodeSpec::constant(value).expect_err("nonfinite constant"),
            SplineBuildError::NonFiniteInput {
                field: SplineFloatField::Value,
                bits: value.to_bits()
            }
        );
        assert_eq!(
            SplinePoint::new(value, node(0), 0.0).expect_err("nonfinite location"),
            SplineBuildError::NonFiniteInput {
                field: SplineFloatField::Location,
                bits: value.to_bits()
            }
        );
        assert_eq!(
            SplinePoint::new(0.0, node(0), value).expect_err("nonfinite derivative"),
            SplineBuildError::NonFiniteInput {
                field: SplineFloatField::Derivative,
                bits: value.to_bits()
            }
        );
        assert!(matches!(
            SplineCoordinateRange::bounded(value, 1.0),
            Err(SplineBuildError::InvalidCoordinateRange { .. })
        ));
        assert!(matches!(
            SplineCoordinateRange::bounded(-1.0, value),
            Err(SplineBuildError::InvalidCoordinateRange { .. })
        ));
    }
    assert!(SplineCoordinateRange::bounded(1.0, -1.0).is_err());
    for (a, b) in [(0.0, -0.0), (-0.0, 0.0), (1.0, 1.0), (2.0, 1.0)] {
        assert!(matches!(
            SplineNodeSpec::multipoint(coordinate(0), &[point(a, 0, 0.0), point(b, 0, 0.0)]),
            Err(SplineBuildError::InvalidLocationOrder { point: 1, .. })
        ));
    }
    assert_eq!(
        CubicSpline::new(&[], &[], node(0)).expect_err("empty graph"),
        SplineBuildError::EmptyGraph
    );
    assert_eq!(
        CubicSpline::new(&[], &[constant(1.0)], SplineNodeId::new(u32::MAX))
            .expect_err("malformed root"),
        SplineBuildError::UnknownNode {
            node: SplineNodeId::new(u32::MAX),
            parent: None
        }
    );
    let points =
        [SplinePoint::new(0.0, SplineNodeId::new(u32::MAX), 0.0).expect("typed child index")];
    assert_eq!(
        CubicSpline::new(
            &[SplineCoordinateRange::unlimited()],
            &[multi(0, &points)],
            node(0)
        )
        .expect_err("malformed child"),
        SplineBuildError::UnknownNode {
            node: SplineNodeId::new(u32::MAX),
            parent: Some(node(0))
        }
    );
    let points = [point(0.0, 0, 0.0)];
    assert_eq!(
        CubicSpline::new(&[], &[constant(1.0), multi(0, &points)], node(0))
            .expect_err("unknown coordinate in unused node"),
        SplineBuildError::UnknownCoordinate {
            node: node(1),
            coordinate: coordinate(0)
        }
    );
    let unknown = SplineCoordinateId::new(u32::MAX);
    let spec = SplineNodeSpec::multipoint(unknown, &points).expect("borrowed input");
    assert!(
        matches!(CubicSpline::new(&[SplineCoordinateRange::unlimited()], &[constant(1.0), spec], node(0)), Err(SplineBuildError::UnknownCoordinate { coordinate, .. }) if coordinate == unknown)
    );
}

#[test]
fn exact_node_coordinate_and_point_caps() {
    let specs = vec![constant(1.0); MAX_SPLINE_NODES];
    let ranges = vec![SplineCoordinateRange::unlimited(); MAX_SPLINE_COORDINATES];
    let spline = CubicSpline::new(&ranges, &specs, node(MAX_SPLINE_NODES - 1))
        .expect("exact node/coordinate caps");
    assert_eq!(spline.node_count(), MAX_SPLINE_NODES);
    assert_eq!(spline.coordinate_count(), MAX_SPLINE_COORDINATES);
    let endpoint = [point(0.0, 0, 0.0)];
    let last_coordinate = CubicSpline::new(
        &ranges,
        &[constant(3.0), multi(MAX_SPLINE_COORDINATES - 1, &endpoint)],
        node(1),
    )
    .expect("last admitted coordinate index");
    let mut requests = Vec::new();
    assert_eq!(
        last_coordinate
            .sample_with(4, &mut |id| {
                requests.push(id.index());
                Ok::<_, &'static str>(0.0)
            })
            .expect("admitted coordinate callback"),
        3.0
    );
    assert_eq!(
        requests,
        [u32::try_from(MAX_SPLINE_COORDINATES - 1).expect("cap fits")]
    );
    assert!(
        matches!(CubicSpline::new(&ranges, &vec![constant(1.0); MAX_SPLINE_NODES + 1], node(0)), Err(SplineBuildError::TooManyNodes { count }) if count == MAX_SPLINE_NODES + 1)
    );
    assert!(
        matches!(CubicSpline::new(&vec![SplineCoordinateRange::unlimited(); MAX_SPLINE_COORDINATES + 1], &[constant(1.0)], node(0)), Err(SplineBuildError::TooManyCoordinates { count }) if count == MAX_SPLINE_COORDINATES + 1)
    );
    assert_eq!(
        SplineNodeSpec::multipoint(coordinate(0), &[]).expect_err("empty points"),
        SplineBuildError::EmptyPoints
    );
    let points: Vec<_> = (0..=MAX_SPLINE_POINTS_PER_NODE)
        .map(|i| point(i as f32, 0, 0.0))
        .collect();
    assert_eq!(
        SplineNodeSpec::multipoint(coordinate(0), &points).expect_err("point cap"),
        SplineBuildError::TooManyPointsInNode {
            count: MAX_SPLINE_POINTS_PER_NODE + 1
        }
    );
    let points = &points[..MAX_SPLINE_POINTS_PER_NODE];
    let mut specs = vec![constant(1.0)];
    specs.extend(std::iter::repeat_n(
        multi(0, points),
        MAX_SPLINE_POINTS / MAX_SPLINE_POINTS_PER_NODE,
    ));
    let ranges = [SplineCoordinateRange::unlimited()];
    let valid = CubicSpline::new(&ranges, &specs, node(1)).expect("exact aggregate cap");
    assert_eq!(valid.point_count(), MAX_SPLINE_POINTS);
    specs.push(multi(0, points));
    assert!(
        matches!(CubicSpline::new(&ranges, &specs, node(1)), Err(SplineBuildError::TooManyPoints { count }) if count == MAX_SPLINE_POINTS + MAX_SPLINE_POINTS_PER_NODE)
    );
    assert_eq!(
        sample(&valid, &[0.5]),
        1.0,
        "prior graph is unchanged after rejection"
    );
}

#[test]
fn cycles_unused_subgraphs_and_exact_max_depth() {
    let ranges = [SplineCoordinateRange::unlimited()];
    let self_cycle = [point(0.0, 1, 0.0)];
    assert_eq!(
        CubicSpline::new(&ranges, &[constant(1.0), multi(0, &self_cycle)], node(0))
            .expect_err("unused cycle"),
        SplineBuildError::Cycle {
            node: node(1),
            child: node(1)
        }
    );
    let long_cycle: Vec<_> = (0..MAX_SPLINE_NODES)
        .map(|i| [point(0.0, (i + 1) % MAX_SPLINE_NODES, 0.0)])
        .collect();
    let specs: Vec<_> = long_cycle.iter().map(|points| multi(0, points)).collect();
    assert!(
        matches!(
            CubicSpline::new(&ranges, &specs, node(0)),
            Err(SplineBuildError::Cycle { .. })
        ),
        "cycles are found before recursive evaluation or depth admission"
    );
    let chain: Vec<_> = (0..MAX_SPLINE_DEPTH)
        .map(|i| [point(0.0, i, 0.0)])
        .collect();
    let mut specs = vec![constant(-0.0)];
    specs.extend(
        chain[..MAX_SPLINE_DEPTH - 1]
            .iter()
            .map(|points| multi(0, points)),
    );
    let valid =
        CubicSpline::new(&ranges, &specs, node(MAX_SPLINE_DEPTH - 1)).expect("exact depth 32");
    assert_eq!(valid.depth(), MAX_SPLINE_DEPTH);
    assert_eq!(sample(&valid, &[0.0]).to_bits(), (-0.0_f32).to_bits());
    specs.push(multi(0, &chain[MAX_SPLINE_DEPTH - 1]));
    assert_eq!(
        CubicSpline::new(&ranges, &specs, node(MAX_SPLINE_DEPTH)).expect_err("depth 33"),
        SplineBuildError::DepthExceeded {
            node: node(MAX_SPLINE_DEPTH),
            depth: MAX_SPLINE_DEPTH + 1
        }
    );
}

fn repeated_child() -> CubicSpline {
    let inner = [point(0.0, 0, 0.0)];
    let outer = [point(0.0, 1, 0.0), point(1.0, 1, 0.0)];
    CubicSpline::new(
        &[SplineCoordinateRange::unlimited(); 2],
        &[constant(5.0), multi(1, &inner), multi(0, &outer)],
        node(2),
    )
    .expect("shared graph")
}

#[test]
fn ordered_repeated_callbacks_budget_boundaries_and_source_failures() {
    let spline = repeated_child();
    let mut trace = Vec::new();
    let result = spline
        .sample_with(12, &mut |id| {
            trace.push(id.index());
            Ok::<_, &'static str>(0.0)
        })
        .expect("exact work budget");
    assert_eq!(result, 5.0);
    assert_eq!(
        trace,
        [0, 1, 1],
        "exact first endpoint still evaluates both children; aliases are repeated"
    );
    for budget in [0, MAX_SPLINE_SAMPLE_WORK + 1, usize::MAX] {
        let mut called = false;
        let error = spline
            .sample_with(budget, &mut |_| {
                called = true;
                Ok::<_, &'static str>(0.0)
            })
            .expect_err("invalid admission");
        assert_eq!(error, SplineSampleError::InvalidBudget { budget });
        assert!(!called);
    }
    let mut trace = Vec::new();
    assert_eq!(
        spline
            .sample_with(11, &mut |id| {
                trace.push(id.index());
                Ok::<_, &'static str>(0.0)
            })
            .expect_err("one missing work unit"),
        SplineSampleError::BudgetExceeded { limit: 11 }
    );
    assert_eq!(trace, [0, 1, 1]);
    let mut trace = Vec::new();
    assert!(matches!(
        spline.sample_with(1, &mut |id| {
            trace.push(id.index());
            Ok::<_, &'static str>(0.0)
        }),
        Err(SplineSampleError::BudgetExceeded { limit: 1 })
    ));
    assert!(trace.is_empty(), "callback is charged before execution");
    let mut trace = Vec::new();
    let error = spline
        .sample_with(12, &mut |id| {
            trace.push(id.index());
            if trace.len() == 3 {
                Err("right request failed")
            } else {
                Ok(0.0)
            }
        })
        .expect_err("typed callback rejection");
    assert_eq!(
        error,
        SplineSampleError::CoordinateSource {
            coordinate: coordinate(1),
            source: "right request failed"
        }
    );
    assert_eq!(
        trace,
        [0, 1, 1],
        "external callback effects are retained, no extra requests"
    );
    assert_eq!(
        sample(&spline, &[0.0, 0.0]),
        5.0,
        "graph remains valid after failures"
    );
}

#[test]
fn repeated_dag_has_a_bounded_work_failure_without_graph_mutation() {
    let levels: Vec<_> = (0..MAX_SPLINE_DEPTH - 1)
        .map(|i| [point(0.0, i, 0.0), point(1.0, i, 0.0)])
        .collect();
    let mut specs = vec![constant(0.125)];
    specs.extend(levels.iter().map(|p| multi(0, p)));
    let spline = CubicSpline::new(
        &[SplineCoordinateRange::unlimited()],
        &specs,
        node(MAX_SPLINE_DEPTH - 1),
    )
    .expect("deep shared DAG");
    let before = (
        spline.min_value().to_bits(),
        spline.max_value().to_bits(),
        spline.depth(),
        spline.node_count(),
        spline.point_count(),
    );
    let mut calls = 0;
    let error = spline
        .sample_with(MAX_SPLINE_SAMPLE_WORK, &mut |_| {
            calls += 1;
            Ok::<_, &'static str>(0.5)
        })
        .expect_err("no exponential unbounded evaluation");
    assert_eq!(
        error,
        SplineSampleError::BudgetExceeded {
            limit: MAX_SPLINE_SAMPLE_WORK
        }
    );
    assert!(calls > 100_000 && calls < MAX_SPLINE_SAMPLE_WORK);
    assert_eq!(
        before,
        (
            spline.min_value().to_bits(),
            spline.max_value().to_bits(),
            spline.depth(),
            spline.node_count(),
            spline.point_count()
        )
    );
    assert_eq!(
        sample(&spline, &[2.0]),
        0.125,
        "extension evaluates a single path after failed interior"
    );
}

#[test]
fn ownership_clone_and_sampling_request_order_are_independent() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CubicSpline>();
    let mut ranges = [SplineCoordinateRange::bounded(-1.0, 1.0).expect("finite range")];
    let mut points = [point(-1.0, 0, 1.0), point(1.0, 1, -1.0)];
    let mut specs = [constant(-2.0), constant(4.0), multi(0, &points)];
    let spline = CubicSpline::new(&ranges, &specs, node(2)).expect("owned graph");
    specs[0] = constant(99.0);
    let changed = CubicSpline::new(&ranges, &specs, node(0)).expect("changed independent input");
    assert_eq!(sample(&changed, &[]), 99.0);
    points[0] = point(-3.0, 1, 0.0);
    ranges[0] = SplineCoordinateRange::bounded(0.0, 0.0).expect("another range");
    assert_eq!(points[0].location(), -3.0);
    assert_eq!(ranges[0].min_value(), 0.0);
    let clone = spline.clone();
    let values = [-4.0, -1.0, -0.0, 0.0, 0.5, 1.0, 4.0];
    let expected: Vec<_> = values
        .iter()
        .map(|&x| sample(&spline, &[x]).to_bits())
        .collect();
    for index in (0..values.len()).rev() {
        assert_eq!(sample(&clone, &[values[index]]).to_bits(), expected[index]);
    }
    assert_eq!(
        spline
            .coordinate_range(coordinate(0))
            .expect("original declaration")
            .min_value(),
        -1.0
    );
    assert!(spline
        .coordinate_range(SplineCoordinateId::new(u32::MAX))
        .is_none());
    drop(spline);
    assert_eq!(
        sample(&clone, &[-1.0]),
        -2.0,
        "clone owns immutable shared graph, not input slices"
    );
}

#[test]
fn nonfinite_samples_are_explicit_errors_and_reported_ranges_are_not_guards() {
    let points = [point(0.0, 0, 2.0)];
    let spline = CubicSpline::new(
        &[SplineCoordinateRange::bounded(-1.0, 1.0).expect("range")],
        &[constant(1.0), multi(0, &points)],
        node(1),
    )
    .expect("finite graph");
    assert_eq!(
        sample(&spline, &[10.0]),
        21.0,
        "diagnostic range does not clamp coordinates"
    );
    for value in [
        f32::NAN,
        f32::from_bits(0x7fa1_2345),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let mut calls = 0;
        assert_eq!(
            spline
                .sample_with(16, &mut |_| {
                    calls += 1;
                    Ok::<_, &'static str>(value)
                })
                .expect_err("nonfinite callback"),
            SplineSampleError::NonFiniteCoordinate {
                coordinate: coordinate(0),
                bits: value.to_bits()
            }
        );
        assert_eq!(calls, 1);
    }
    let points = [point(0.0, 0, f32::MAX)];
    let overflow = CubicSpline::new(
        &[SplineCoordinateRange::unlimited()],
        &[constant(1.0), multi(0, &points)],
        node(1),
    )
    .expect("original infinite diagnostics retained");
    assert_eq!(overflow.min_value(), f32::NEG_INFINITY);
    assert_eq!(overflow.max_value(), f32::INFINITY);
    assert_eq!(
        overflow
            .sample_with(16, &mut |_| Ok::<_, &'static str>(f32::MAX))
            .expect_err("nonfinite arithmetic"),
        SplineSampleError::NonFiniteResult {
            node: node(1),
            bits: f32::INFINITY.to_bits()
        }
    );
    assert_eq!(sample(&overflow, &[0.0]), 1.0);
    assert_eq!(sample(&spline, &[1.0]), 3.0);
}
