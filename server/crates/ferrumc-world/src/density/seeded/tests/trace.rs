use super::*;

fn original_bounds(value: &Value) -> DensityBounds {
    crate::density::bind::pair(
        f64::from_bits(bits64(&value[0])),
        f64::from_bits(bits64(&value[1])),
    )
}
#[expect(
    clippy::too_many_lines,
    reason = "Keeps the complete original reference or shared-meter acceptance matrix together."
)]
pub(super) fn compare_original_trace(record: &Value) {
    use crate::density::bind::{
        binary_bounds, java_max, java_min, mapped_bounds, mul_or_add_bounds, pair,
    };
    let context = record["case"].as_str().unwrap();
    let operation = match record["operation"].as_str().unwrap() {
        "mul" => Binary::Mul,
        "min" => Binary::Min,
        "max" => Binary::Max,
        _ => Binary::Add,
    };
    let throwing = record.get("throw_child").and_then(Value::as_str);
    let child = |key: &str, label: &'static str| {
        // The public closed builtin consumer has no arbitrary Java callback.
        // Infinity is a private probe sentinel for the same selected-child
        // failure trace; error classes remain FerrumC finite-policy evidence.
        original(NodeKind::Constant(if throwing == Some(label) {
            f64::INFINITY
        } else {
            f64::from_bits(bits64(&record[key]))
        }))
    };
    let (kinds, root, bounds, children) = match record["factory"].as_str().unwrap() {
        "binary" => {
            let a = original_bounds(&record["left_bounds"]);
            let b = original_bounds(&record["right_bounds"]);
            (
                vec![
                    (child("left_bits", "left"), "left"),
                    (child("right_bits", "right"), "right"),
                    (
                        original(NodeKind::Binary {
                            operation,
                            left: 0,
                            right: 1,
                        }),
                        "factory",
                    ),
                ],
                2,
                binary_bounds(operation, a, b),
                vec![(0, a), (1, b)],
            )
        }
        "mul_or_add" => {
            let input = original_bounds(&record["input_bounds"]);
            let argument = f64::from_bits(bits64(&record["argument_bits"]));
            let bounds = if record["view"] == "factory" {
                binary_bounds(operation, pair(argument, argument), input)
            } else {
                mul_or_add_bounds(operation, input, argument)
            };
            (
                vec![
                    (child("input_bits", "input"), "input"),
                    (
                        BoundKind::MulOrAdd {
                            operation,
                            input: 0,
                            argument,
                        },
                        "factory",
                    ),
                ],
                1,
                bounds,
                vec![(0, input)],
            )
        }
        "range_choice" => {
            let a = original_bounds(&record["inside_bounds"]);
            let b = original_bounds(&record["outside_bounds"]);
            (
                vec![
                    (child("input_bits", "input"), "input"),
                    (child("inside_bits", "inside"), "inside"),
                    (child("outside_bits", "outside"), "outside"),
                    (
                        original(NodeKind::RangeChoice {
                            input: 0,
                            min: f64::from_bits(bits64(&record["min_bits"])),
                            max: f64::from_bits(bits64(&record["max_bits"])),
                            inside: 1,
                            outside: 2,
                        }),
                        "factory",
                    ),
                ],
                3,
                pair(java_min(a.min, b.min), java_max(a.max, b.max)),
                vec![
                    (0, original_bounds(&record["input_bounds"])),
                    (1, a),
                    (2, b),
                ],
            )
        }
        "mapped" => {
            let operation = match record["mapped_operation"].as_str().unwrap() {
                "abs" => Unary::Abs,
                "square" => Unary::Square,
                "cube" => Unary::Cube,
                "half_negative" => Unary::HalfNegative,
                "quarter_negative" => Unary::QuarterNegative,
                "squeeze" => Unary::Squeeze,
                _ => panic!("unknown original mapped"),
            };
            let input = original_bounds(&record["input_bounds"]);
            (
                vec![
                    (child("input_bits", "input"), "input"),
                    (
                        original(NodeKind::Unary {
                            operation,
                            input: 0,
                        }),
                        "factory",
                    ),
                ],
                1,
                mapped_bounds(operation, input),
                vec![(0, input)],
            )
        }
        _ => panic!("unknown original factory"),
    };
    assert_eq!(
        bounds.min.to_bits(),
        bits64(&record["min_value_bits"]),
        "{context}/min"
    );
    assert_eq!(
        bounds.max.to_bits(),
        bits64(&record["max_value_bits"]),
        "{context}/max"
    );
    let mut router = probe_router(kinds, root);
    for (index, bounds) in children {
        Arc::get_mut(&mut router.data).unwrap().router.nodes[index].bounds = bounds;
    }
    let (sample, work) = evaluate_probe(&router, root, 100);
    let actual_trace = work
        .density_trace()
        .iter()
        .copied()
        .filter(|&label| label != "factory")
        .collect::<Vec<_>>();
    let expected_trace = record["trace"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    let first_policy_failure = expected_trace.iter().position(|&label| {
        let key = match label {
            "left" => "left_bits",
            "right" => "right_bits",
            "input" => "input_bits",
            "inside" => "inside_bits",
            "outside" => "outside_bits",
            _ => panic!("unknown actual probe label"),
        };
        throwing == Some(label) || !f64::from_bits(bits64(&record[key])).is_finite()
    });
    let requested_trace = if let Some(index) = first_policy_failure {
        &expected_trace[..=index]
    } else {
        &expected_trace[..]
    };
    // Java may select a finite outside child after a NaN input. FerrumC
    // intentionally rejects that completed input first; this is not Java
    // branch-order parity for a successful request.
    assert_eq!(
        actual_trace, requested_trace,
        "{context}/selected children or explicit finite-policy prefix"
    );
    if first_policy_failure.is_some()
        || record.get("error").is_some()
        || !f64::from_bits(bits64(&record["result_bits"])).is_finite()
    {
        assert!(
            matches!(
                sample.unwrap_err().kind(),
                DensitySampleErrorKind::NonFiniteDensity { .. }
            ),
            "{context}/finite policy"
        );
    } else {
        let actual = sample.unwrap_or_else(|error| panic!("{context}: {error:?}"));
        assert_eq!(
            actual.to_bits(),
            bits64(&record["result_bits"]),
            "{context}/result"
        );
        let (exact, exact_work) = evaluate_probe(&router, root, work.used());
        assert_eq!(exact.unwrap().to_bits(), actual.to_bits());
        assert_eq!(exact_work.density_trace(), work.density_trace());
        let (short, short_work) = evaluate_probe(&router, root, work.used() - 1);
        assert!(
            matches!(short.unwrap_err().kind(),DensitySampleErrorKind::WorkExceeded {limit,used} if *limit==work.used()-1 && *used==work.used()-1)
        );
        assert_eq!(
            short_work.density_trace(),
            &work.density_trace()[..work.density_trace().len() - 1]
        );
    }
}
