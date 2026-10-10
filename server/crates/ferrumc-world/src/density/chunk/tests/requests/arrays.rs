use super::*;

fn matrix(row: &Value, inputs: &[f64]) {
    let case = row["case"].as_str().unwrap();
    let mut s = fresh(false);
    let parts = case.split('|').collect::<Vec<_>>();
    let mut initial = vec![-99.0; inputs.len()];
    let root = match parts[0] {
        "add" | "mul" | "min" | "max" => {
            let left = leaf(&mut s, "left", inputs, (-4.0, 6.0));
            let right = leaf(&mut s, "right", &[11.0], (2.0, 2.0));
            controls(&mut s, left, None, parts[1] == "left");
            controls(&mut s, right, (parts[1] == "right").then_some(1), false);
            let operation = match parts[0] {
                "add" => Binary::Add,
                "mul" => Binary::Mul,
                "min" => Binary::Min,
                _ => Binary::Max,
            };
            node(
                &mut s,
                original(NodeKind::Binary {
                    operation,
                    left,
                    right,
                }),
            )
        }
        "mulOrAdd-zero" => {
            initial.fill(0.0);
            let input = leaf(&mut s, "input", inputs, (-5.0, 5.0));
            controls(&mut s, input, None, parts[2] == "true");
            node(
                &mut s,
                BoundKind::MulOrAdd {
                    operation: Binary::Mul,
                    input,
                    argument: f64::from_bits(u64::from_str_radix(parts[1], 16).unwrap()),
                },
            )
        }
        "rangeChoice" => {
            initial.fill(0.0);
            let input = leaf(&mut s, "input", inputs, (-3.0, 5.0));
            let inside = leaf(&mut s, "inside", &[17.0], (17.0, 17.0));
            let outside = leaf(&mut s, "outside", &[-19.0], (-19.0, -19.0));
            controls(&mut s, input, None, parts[1] == "input");
            controls(&mut s, inside, (parts[1] == "inside").then_some(0), false);
            controls(&mut s, outside, (parts[1] == "outside").then_some(0), false);
            node(
                &mut s,
                original(NodeKind::RangeChoice {
                    input,
                    min: 0.0,
                    max: 2.0,
                    inside,
                    outside,
                }),
            )
        }
        "blendDensity" | "abs" | "clamp" => {
            initial.fill(0.0);
            let input = leaf(&mut s, "input", inputs, (-5.0, 5.0));
            let kind = match parts[0] {
                "blendDensity" => NodeKind::BlendDensity(input),
                "abs" => NodeKind::Unary {
                    operation: Unary::Abs,
                    input,
                },
                _ => NodeKind::Clamp {
                    input,
                    min: -1.0,
                    max: 1.0,
                },
            };
            node(&mut s, original(kind))
        }
        _ => panic!("unknown matrix {case}"),
    };
    array(&mut s, root, row, initial, Provider::Foreign(inputs.len()));
}

#[test]
fn original_array_finite_failures_and_shared_once() {
    let rows = rows();
    let inputs = [
        0.0,
        -0.0,
        -3.0,
        f64::from_bits(2.0_f64.to_bits() - 1),
        2.0,
        f64::from_bits(2.0_f64.to_bits() + 1),
        5.0,
        f64::NAN,
    ];
    let mut count = 0;
    for row in rows.iter().filter(|r| r["kind"] == "array") {
        let case = row["case"].as_str().unwrap();
        if [
            "add|left",
            "mul|left",
            "min|left",
            "max|left",
            "mulOrAdd-zero|0000000000000000|true",
            "mulOrAdd-zero|8000000000000000|true",
            "rangeChoice|input",
        ]
        .contains(&case)
        {
            matrix(row, &inputs);
            count += 1;
        }
    }
    assert_eq!(count, 7);
    let mut s = fresh(true);
    let child = leaf(&mut s, "shared-child", &[2.0], (-10.0, 10.0));
    let once = marker(&mut s, Marker::CacheOnce, child);
    let root = node(
        &mut s,
        original(NodeKind::Binary {
            operation: Binary::Add,
            left: once,
            right: once,
        }),
    );
    array(
        &mut s,
        root,
        find(&rows, "array", "shared-once-add-array"),
        vec![0.0; 4],
        Provider::Foreign(4),
    );
}

#[test]
fn original_unit_gradient_weird_transform_request_contracts() {
    let rows = rows();
    for operation in [
        "constant",
        "blendAlpha",
        "blendOffset",
        "beard",
        "gradient",
        "weirdScaledSampler",
    ] {
        let mut s = fresh(false);
        let kind = match operation {
            "constant" => NodeKind::Constant(-0.0),
            "blendAlpha" => NodeKind::Unit(Unit::BlendAlpha),
            "blendOffset" => NodeKind::Unit(Unit::BlendOffset),
            "beard" => NodeKind::Unit(Unit::Beardifier),
            "gradient" => NodeKind::Gradient {
                from_y: 100,
                to_y: 107,
                from_value: -1.0,
                to_value: 1.0,
            },
            _ => {
                let input = leaf(&mut s, "weird-input", &[-1.0, 0.0, 1.0, 2.0], (-3.0, 3.0));
                NodeKind::Weird {
                    input,
                    resource: ferrumc_registry::worldgen::NoiseResourceId::from_name(
                        "minecraft:temperature",
                    )
                    .unwrap(),
                    rarity: Rarity::Type1,
                }
            }
        };
        let root = node(&mut s, original(kind));
        if operation == "weirdScaledSampler" {
            let mut random = crate::random::WorldgenRandomSource::xoroshiro(WorldSeed::new(0));
            s.state.normal_noises[0] =
                crate::noise::NormalNoise::new(&mut random, 0, &[0.0]).unwrap();
            s.view.nodes[root].noise = Some(0);
        }
        array(
            &mut s,
            root,
            find(&rows, "array", &format!("{operation}|actual-fill-contract")),
            vec![0.0; 8],
            Provider::Foreign(8),
        );
    }
}

#[test]
fn original_once_cell_direct_array_index_requests_and_prefix() {
    let rows = rows();
    let mut s = fresh(true);
    let child = leaf(
        &mut s,
        "nested-once-child",
        &[7.0, 11.0, 13.0, 17.0],
        (-20.0, 20.0),
    );
    let once = marker(&mut s, Marker::CacheOnce, child);
    s.state.lifecycle = Lifecycle::Running;
    s.initialize_driver().unwrap();
    s.fill_slice(true, 0).unwrap();
    s.fill_slice(false, 1).unwrap();
    s.state.array_counter = 41;
    array(
        &mut s,
        once,
        find(&rows, "array", "cacheonce-own-array-before-direct-provider"),
        vec![0.0; 128],
        Provider::Cell,
    );
    let row = find(
        &rows,
        "array",
        "cacheonce-own-cell-direct-arrayIndex-contract",
    );
    let mut values = vec![0.0; 128];
    assert!(s
        .state
        .fill_direct(
            &s.data,
            &s.view,
            once,
            &mut values,
            Provider::Cell,
            &mut s.work
        )
        .is_err());
    let expected = &row["trace"][0];
    compare_bits(&values, &expected["bits"], "once-direct-prefix");
    assert_eq!(take_trace(&mut s), [] as [Value; 0]);
    check_state(&s, row);
}

#[test]
fn actual_finite_array_addendum_all_16_complete_bits_prefixes_and_traces() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/noise_chunk_1_21_8/paper-finite-arrays.jsonl");
    let records = std::fs::read_to_string(path).unwrap();
    let inputs = [
        0.0,
        -0.0,
        -3.0,
        f64::from_bits(2.0_f64.to_bits() - 1),
        2.0,
        f64::from_bits(2.0_f64.to_bits() + 1),
        5.0,
        6.0,
    ];
    let mut count = 0;
    for line in records.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        matrix(&row, &inputs);
        count += 1;
    }
    assert_eq!(count, 16);
}

#[test]
fn declared_nonfinite_array_child_stops_before_parent_and_right_requests() {
    let mut s = fresh(false);
    let input = [0.0, -0.0, -3.0, 2.0, 2.0, 2.0, 5.0, f64::NAN];
    let left = leaf(&mut s, "left", &input, (-4.0, 6.0));
    let right = leaf(&mut s, "right", &[11.0], (2.0, 2.0));
    let root = node(
        &mut s,
        original(NodeKind::Binary {
            operation: Binary::Add,
            left,
            right,
        }),
    );
    let mut values = vec![-99.0; 8];
    let error = s
        .state
        .fill(
            &s.data,
            &s.view,
            root,
            &mut values,
            Provider::Foreign(8),
            &mut s.work,
        )
        .unwrap_err();
    assert!(
        matches!(error.kind(),NoiseChunkErrorKind::Density(e) if matches!(e.kind(),DensitySampleErrorKind::NonFiniteDensity{..}))
    );
    assert_eq!(&values[..7], &input[..7]);
    assert_eq!(values[7], -99.0);
    assert_eq!(
        take_trace(&mut s),
        vec![json!({"kind":"request","case":"left","operation":"fillArray","length":8})]
    );
}
