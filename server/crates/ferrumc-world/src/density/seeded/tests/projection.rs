use super::*;

// Build only one local shape at a time. Actual expected trees can contain many
// repeated DAG occurrences; comparing them does not expand the owned Rust DAG.
pub(super) fn compare_projection(
    view: &BoundView,
    mut index: usize,
    expected: &Value,
    context: &str,
) {
    for _ in 0..=crate::density::MAX_DENSITY_DEPTH {
        match view.nodes[index].kind {
            BoundKind::Alias(target) => index = target,
            _ => return compare_shape(view, &project_bound_shape(view, index), expected, context),
        }
    }
    panic!("alias cycle at {context}");
}
fn compare_shape(view: &BoundView, actual: &Value, expected: &Value, context: &str) {
    if let Some(index) = actual.get("__bound_node") {
        return compare_projection(view, index.as_u64().unwrap() as usize, expected, context);
    }
    match actual {
        Value::Object(actual) => {
            let expected = expected
                .as_object()
                .unwrap_or_else(|| panic!("expected object at {context}"));
            assert_eq!(
                actual.len(),
                expected.len(),
                "shape key count at {context}; expected keys {:?}; actual {:?}",
                expected.keys().collect::<Vec<_>>(),
                actual
            );
            for (key, value) in actual {
                compare_shape(
                    view,
                    value,
                    expected
                        .get(key)
                        .unwrap_or_else(|| panic!("missing key {key} at {context}")),
                    &format!("{context}/{key}"),
                );
            }
        }
        Value::Array(actual) => {
            let expected = expected.as_array().unwrap();
            assert_eq!(actual.len(), expected.len(), "array at {context}");
            for (index, (a, b)) in actual.iter().zip(expected).enumerate() {
                compare_shape(view, a, b, &format!("{context}/{index}"));
            }
        }
        _ => assert_eq!(actual, expected, "{context}"),
    }
}
#[expect(
    clippy::too_many_lines,
    reason = "Keeps the complete original reference or shared-meter acceptance matrix together."
)]
fn project_bound_shape(view: &crate::density::bind::BoundView, index: usize) -> serde_json::Value {
    use serde_json::json;
    let double = |value: f64| format!("{:016x}", value.to_bits());
    let float = |value: f32| format!("{:08x}", value.to_bits());
    let child = |index| json!({"__bound_node":index});
    let noise = |id: NoiseResourceId| json!({"resource":id.resource_location()});
    let node = &view.nodes[index];
    let mut value = match &node.kind {
        BoundKind::Alias(_) => unreachable!("aliases resolved before projection"),
        BoundKind::MulOrAdd {
            operation,
            input,
            argument,
        } => {
            json!({"kind":"mul_or_add","operation":match operation { Binary::Add=>"add",Binary::Mul=>"mul",_=>unreachable!() },"argument":child(*input),"mapped_argument_bits":double(*argument)})
        }
        BoundKind::Original(kind) => match kind {
            NodeKind::Constant(value) => json!({"kind":"constant","value_bits":double(*value)}),
            NodeKind::Holder { origin, target } | NodeKind::Coordinate { origin, target } => {
                let mut value = match origin {
                    HolderOrigin::Reference(id) => json!({"resource":id.resource_location()}),
                    HolderOrigin::Direct => json!({"inline":child(*target)}),
                };
                if matches!(node.kind, BoundKind::Original(NodeKind::Holder { .. })) {
                    value["kind"] = json!("holder");
                }
                value
            }
            NodeKind::Binary {
                operation,
                left,
                right,
            } => {
                json!({"kind":"binary","operation":match operation { Binary::Add=>"add",Binary::Mul=>"mul",Binary::Min=>"min",Binary::Max=>"max" },"argument1":child(*left),"argument2":child(*right)})
            }
            NodeKind::Unary { operation, input } => {
                json!({"kind":"mapped","operation":match operation { Unary::Abs=>"abs",Unary::Square=>"square",Unary::Cube=>"cube",Unary::HalfNegative=>"half_negative",Unary::QuarterNegative=>"quarter_negative",Unary::Squeeze=>"squeeze" },"argument":child(*input)})
            }
            NodeKind::Marker { marker, input } => {
                json!({"kind":"marker","operation":match marker { Marker::Interpolated=>"interpolated",Marker::FlatCache=>"flat_cache",Marker::Cache2D=>"cache_2d",Marker::CacheOnce=>"cache_once",Marker::CacheAllInCell=>"cache_all_in_cell" },"argument":child(*input)})
            }
            NodeKind::BlendDensity(input) => {
                json!({"kind":"blend_density","argument":child(*input)})
            }
            NodeKind::Clamp { input, min, max } => {
                json!({"kind":"clamp","input":child(*input),"min_bits":double(*min),"max_bits":double(*max)})
            }
            NodeKind::RangeChoice {
                input,
                min,
                max,
                inside,
                outside,
            } => {
                json!({"kind":"range_choice","input":child(*input),"min_inclusive_bits":double(*min),"max_exclusive_bits":double(*max),"when_in_range":child(*inside),"when_out_of_range":child(*outside)})
            }
            NodeKind::Gradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => {
                json!({"kind":"y_clamped_gradient","from_y":from_y,"to_y":to_y,"from_value_bits":double(*from_value),"to_value_bits":double(*to_value)})
            }
            NodeKind::Noise { resource, xz, y } => {
                json!({"kind":"noise","noise":noise(*resource),"xz_scale_bits":double(*xz),"y_scale_bits":double(*y)})
            }
            NodeKind::Shift { shift, resource } => {
                json!({"kind":match shift { Shift::All=>"shift",Shift::A=>"shift_a",Shift::B=>"shift_b" },"argument":noise(*resource)})
            }
            NodeKind::ShiftedNoise {
                shift_x,
                shift_y,
                shift_z,
                resource,
                xz,
                y,
            } => {
                json!({"kind":"shifted_noise","shift_x":child(*shift_x),"shift_y":child(*shift_y),"shift_z":child(*shift_z),"noise":noise(*resource),"xz_scale_bits":double(*xz),"y_scale_bits":double(*y)})
            }
            NodeKind::Weird {
                input,
                resource,
                rarity,
            } => {
                json!({"kind":"weird_scaled_sampler","input":child(*input),"noise":noise(*resource),"rarity_value_mapper":match rarity { Rarity::Type1=>"type_1",Rarity::Type2=>"type_2" }})
            }
            NodeKind::Blended {
                xz_scale,
                y_scale,
                xz_factor,
                y_factor,
                smear,
            } => {
                json!({"kind":"old_blended_noise","xz_scale_bits":double(*xz_scale),"y_scale_bits":double(*y_scale),"xz_factor_bits":double(*xz_factor),"y_factor_bits":double(*y_factor),"smear_scale_multiplier_bits":double(*smear)})
            }
            NodeKind::Unit(unit) => {
                json!({"kind":match unit { Unit::EndIslands=>"end_islands",Unit::BlendAlpha=>"blend_alpha",Unit::BlendOffset=>"blend_offset",Unit::Beardifier=>"beardifier" }})
            }
            NodeKind::Spline(root) => json!({"kind":"spline","spline":child(*root)}),
            NodeKind::SplineConstant(value) => {
                json!({"kind":"constant","value_bits":float(*value)})
            }
            NodeKind::SplineMultipoint { coordinate, points } => {
                json!({"kind":"multipoint","coordinate":child(*coordinate),"points":points.iter().map(|point|json!({"location_bits":float(point.location),"derivative_bits":float(point.derivative),"value":child(point.value)})).collect::<Vec<_>>()})
            }
        },
    };
    if matches!(
        node.kind,
        BoundKind::Original(NodeKind::SplineConstant(_) | NodeKind::SplineMultipoint { .. })
    ) {
        value["min_value_bits"] = json!(float(node.bounds.min as f32));
        value["max_value_bits"] = json!(float(node.bounds.max as f32));
    } else if !matches!(node.kind, BoundKind::Original(NodeKind::Coordinate { .. })) {
        value["min_value_bits"] = json!(double(node.bounds.min));
        value["max_value_bits"] = json!(double(node.bounds.max));
    }
    value
}
