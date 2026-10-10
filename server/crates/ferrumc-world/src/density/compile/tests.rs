use super::*;

#[test]
fn all_builtin_entries_are_checked_and_templates_remain_owned() {
    for id in density_function_resources() {
        let template = DensityTemplate::compile_definition(id).unwrap();
        assert_eq!(template.resource(), id);
        assert!(template.owned_stats().vertices() > template.root_stats().vertices());
        assert!(template.owned_stats().depth() <= super::super::MAX_DENSITY_DEPTH);
        assert!(template.source_bytes() <= MAX_DENSITY_SOURCE_BYTES);
        let clone = template.clone();
        drop(template);
        assert_eq!(clone.resource(), id);
    }
    for id in ferrumc_registry::worldgen::noise_settings_resources() {
        let template = RouterTemplate::compile_builtin(id).unwrap();
        assert_eq!(template.resource(), id);
        assert!((4..=16).contains(&template.geometry().cell_width()));
        assert!((4..=16).contains(&template.geometry().cell_height()));
        for field in RouterField::ALL {
            assert!(template.field_stats(field).vertices() > 0);
        }
    }
}

fn synthetic(source: &str) -> Result<Compiler, DensityCompileError> {
    let (value, _) = json::parse(source, "test")?;
    let mut compiler = Compiler::new();
    compiler.resource = "test";
    compiler.direct(&value)?;
    compiler.resolve()?;
    graph::validate(&mut compiler.nodes)?;
    Ok(compiler)
}

#[test]
fn checked_schema_ref_roles_numeric_boundaries_and_spline_point_order() {
    for source in [
        r#"{"type":"missing"}"#,
        r#"{"type":"abs"}"#,
        r#"{"type":"abs","argument":0,"extra":1}"#,
        r#"{"type":"abs","argument":"minecraft:unknown"}"#,
        r#"{"type":"clamp","input":"minecraft:zero","min":-1,"max":1}"#,
        r#"{"type":"shift","argument":"minecraft:zero"}"#,
        r#"{"type":"noise","noise":"minecraft:offset","xz_scale":1}"#,
        r#"{"type":"constant","argument":1000000.1}"#,
        r#"{"type":"old_blended_noise","xz_scale":0.0009,"y_scale":1,"xz_factor":1,"y_factor":1,"smear_scale_multiplier":1}"#,
        r#"{"type":"spline","spline":{"coordinate":0,"points":[{"location":0,"value":0}]}}"#,
        r#"{"type":"spline","spline":{"coordinate":0,"points":[{"location":-0.0,"value":0,"derivative":0},{"location":0.0,"value":1,"derivative":0}]}}"#,
        r#"{"type":"spline","spline":{"coordinate":0,"points":[{"location":1e39,"value":0,"derivative":0}]}}"#,
    ] {
        assert!(synthetic(source).is_err(), "{source}");
    }
    for source in [
        "-0.0",
        "-1000000",
        "1000000",
        r#"{"type":"constant","argument":-0.0}"#,
        r#"{"type":"clamp","input":0,"min":1,"max":-1}"#,
        r#"{"type":"range_choice","input":0,"min_inclusive":1,"max_exclusive":-1,"when_in_range":0,"when_out_of_range":0}"#,
        r#"{"type":"y_clamped_gradient","from_y":1.5,"to_y":2.9,"from_value":0,"to_value":1}"#,
        r#"{"type":"cache_all_in_cell","argument":0}"#,
        r#"{"type":"beardifier"}"#,
    ] {
        assert!(synthetic(source).is_ok(), "{source}");
    }
}

#[test]
fn original_integer_coercion_truncates_and_wraps_before_range_check() {
    for (source, expected) in [
        ("1.5", 1),
        ("-1.9", -1),
        ("4294967297", 1),
        ("9223372036854775809", 1),
        ("-4294967297", -1),
        ("1.28e2", 128),
        ("1e-3", 0),
        ("1e308", 0),
        ("1e1000", 0),
        ("1e-9999", 0),
        ("1.234567890123456789e18", 2_112_454_933),
        ("1234567890123456789012345678901234567890", -834_729_262),
    ] {
        let (value, _) = json::parse(source, "test").unwrap();
        assert_eq!(
            integer(&value, "test", "$", i32::MIN, i32::MAX).unwrap(),
            expected,
            "{source}"
        );
    }
    for source in ["1e-2147483648", "1e-2147483649", "1e-10000", "1e10000"] {
        let (value, _) = json::parse(source, "test").unwrap();
        assert!(integer(&value, "test", "$", i32::MIN, i32::MAX).is_err());
    }
    for (digits, good) in [(10_000, true), (10_001, false)] {
        let source = "9".repeat(digits);
        let (value, _) = json::parse(&source, "test").unwrap();
        assert_eq!(
            integer(&value, "test", "$", i32::MIN, i32::MAX).is_ok(),
            good
        );
    }
}

#[test]
fn holder_helper_preserves_inline_constant_factory_identity() {
    let compiler =
        synthetic(r#"{"type":"mul","argument1":-0.0,"argument2":{"type":"blend_alpha"}}"#).unwrap();
    let NodeKind::Binary {
        operation: Binary::Mul,
        left,
        right,
    } = compiler.nodes.last().unwrap().kind
    else {
        panic!()
    };
    assert!(matches!(
        compiler.nodes[left].kind,
        NodeKind::Holder {
            origin: HolderOrigin::Direct,
            ..
        }
    ));
    assert!(matches!(
        compiler.nodes[right].kind,
        NodeKind::Holder {
            origin: HolderOrigin::Direct,
            ..
        }
    ));
    let NodeKind::Constant(value) = compiler.nodes[0].kind else {
        panic!()
    };
    assert_eq!(value.to_bits(), (-0.0f64).to_bits());
}

#[test]
fn exact_geometry_boundaries_and_original_zero_height() {
    for (source, good) in [
        (
            r#"{"min_y":-2032,"height":4064,"size_horizontal":4,"size_vertical":4}"#,
            true,
        ),
        (
            r#"{"min_y":2032,"height":0,"size_horizontal":1,"size_vertical":1}"#,
            false,
        ),
        (
            r#"{"min_y":0,"height":0,"size_horizontal":1,"size_vertical":1}"#,
            true,
        ),
        (
            r#"{"min_y":0,"height":128,"size_horizontal":0,"size_vertical":1}"#,
            false,
        ),
        (
            r#"{"min_y":1,"height":128,"size_horizontal":1,"size_vertical":1}"#,
            false,
        ),
        (
            r#"{"min_y":0,"height":127,"size_horizontal":1,"size_vertical":1}"#,
            false,
        ),
        (
            r#"{"min_y":-64.5,"height":128.9,"size_horizontal":1.9,"size_vertical":2.9}"#,
            true,
        ),
    ] {
        let (value, _) = json::parse(source, "test").unwrap();
        assert_eq!(geometry(&value, "test").is_ok(), good, "{source}");
    }
}

#[test]
fn private_builder_stops_before_vertex_edge_and_source_overallocations() {
    let mut compiler = Compiler::new();
    for _ in 0..MAX_DENSITY_VERTICES {
        compiler.add(NodeKind::Constant(0.0)).unwrap();
    }
    assert!(matches!(
        compiler.add(NodeKind::Constant(0.0)).unwrap_err().kind(),
        Kind::Limit {
            limit: DensityLimit::Vertices,
            ..
        }
    ));
    assert_eq!(compiler.nodes.len(), MAX_DENSITY_VERTICES);
    compiler.nodes.clear();
    compiler.edges = MAX_DENSITY_EDGES;
    assert!(matches!(
        compiler.add(NodeKind::Spline(0)).unwrap_err().kind(),
        Kind::Limit {
            limit: DensityLimit::Edges,
            ..
        }
    ));
    assert!(compiler.nodes.is_empty());
    let id = density_function_resources().next().unwrap();
    let mut bytes = MAX_DENSITY_SOURCE_BYTES - id.definition().json().len();
    source_admit(&mut bytes, id.resource_location(), id.definition()).unwrap();
    assert_eq!(bytes, MAX_DENSITY_SOURCE_BYTES);
    assert!(matches!(
        source_admit(&mut bytes, id.resource_location(), id.definition())
            .unwrap_err()
            .kind(),
        Kind::Limit {
            limit: DensityLimit::SourceBytes,
            ..
        }
    ));
    assert_eq!(bytes, MAX_DENSITY_SOURCE_BYTES);
}

#[test]
fn spline_local_caps_are_checked_before_corresponding_growth() {
    let (constant, _) = json::parse("0", "test").unwrap();
    let mut compiler = Compiler::new();
    let mut budget = SplineBudget {
        nodes: crate::spline::MAX_SPLINE_NODES - 1,
        ..SplineBudget::default()
    };
    compiler.spline(&constant, &mut budget, 32).unwrap();
    let count = compiler.nodes.len();
    assert!(matches!(
        compiler
            .spline(&constant, &mut budget, 32)
            .unwrap_err()
            .kind(),
        Kind::Limit {
            limit: DensityLimit::SplineVertices,
            ..
        }
    ));
    assert_eq!(compiler.nodes.len(), count);
    assert!(matches!(
        compiler
            .spline(&constant, &mut SplineBudget::default(), 33)
            .unwrap_err()
            .kind(),
        Kind::Limit {
            limit: DensityLimit::SplineDepth,
            ..
        }
    ));
    for (count, good) in [(64, true), (65, false)] {
        let source = serde_json::json!({"coordinate":0,"points":(0..count).map(|n|serde_json::json!({"location":n,"derivative":0,"value":0})).collect::<Vec<_>>()}).to_string();
        let (value, _) = json::parse(&source, "test").unwrap();
        assert_eq!(
            Compiler::new()
                .spline(&value, &mut SplineBudget::default(), 1)
                .is_ok(),
            good
        );
    }
    let (value, _) = json::parse(
        r#"{"coordinate":"zero","points":[{"location":0,"value":0,"derivative":0}]}"#,
        "test",
    )
    .unwrap();
    let mut budget = SplineBudget {
        points: crate::spline::MAX_SPLINE_POINTS - 1,
        ..SplineBudget::default()
    };
    Compiler::new().spline(&value, &mut budget, 1).unwrap();
    assert!(matches!(
        Compiler::new()
            .spline(&value, &mut budget, 1)
            .unwrap_err()
            .kind(),
        Kind::Limit {
            limit: DensityLimit::SplinePoints,
            ..
        }
    ));
    let mut budget = SplineBudget::default();
    for n in 0..crate::spline::MAX_SPLINE_COORDINATES {
        budget.coordinates.insert(CoordinateIdentity::Inline(n));
    }
    assert!(matches!(
        Compiler::new()
            .spline(&value, &mut budget, 1)
            .unwrap_err()
            .kind(),
        Kind::Limit {
            limit: DensityLimit::SplineCoordinates,
            ..
        }
    ));
}

#[test]
fn fully_valid_density_spline_coordinate_resource_cycle_is_rejected() {
    let resource = DensityFunctionResourceId::from_name("zero").unwrap();
    let (value, _) = json::parse(r#"{"type":"spline","spline":{"coordinate":"zero","points":[{"location":0,"value":0,"derivative":0}]}}"#, "test").unwrap();
    let mut compiler = Compiler::new();
    let root = compiler.direct(&value).unwrap();
    compiler.roots.insert(resource, root);
    compiler.resolve().unwrap();
    assert!(matches!(
        graph::validate(&mut compiler.nodes).unwrap_err().kind(),
        Kind::Cycle { .. }
    ));
}

fn project(nodes: &[Node], index: usize) -> serde_json::Value {
    use serde_json::json;
    let double = |value: f64| format!("{:016x}", value.to_bits());
    let float = |value: f32| format!("{:08x}", value.to_bits());
    let child = |index| project(nodes, index);
    let noise = |id: NoiseResourceId| json!({"resource":id.resource_location()});
    match &nodes[index].kind {
        NodeKind::Constant(value) => json!({"kind":"constant","value_bits":double(*value)}),
        NodeKind::Holder { origin, target } | NodeKind::Coordinate { origin, target } => {
            let mut value = match origin {
                HolderOrigin::Reference(id) => json!({"resource":id.resource_location()}),
                HolderOrigin::Direct => json!({"inline":child(*target)}),
            };
            if matches!(nodes[index].kind, NodeKind::Holder { .. }) {
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
        NodeKind::BlendDensity(input) => json!({"kind":"blend_density","argument":child(*input)}),
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
        NodeKind::SplineConstant(value) => json!({"kind":"constant","value_bits":float(*value)}),
        NodeKind::SplineMultipoint { coordinate, points } => {
            json!({"kind":"multipoint","coordinate":child(*coordinate),"points":points.iter().map(|point|json!({"location_bits":float(point.location),"derivative_bits":float(point.derivative),"value":child(point.value)})).collect::<Vec<_>>()})
        }
    }
}

fn project_router(graph: &Graph, metadata: RouterMetadata) -> serde_json::Value {
    use serde_json::json;
    let geometry = metadata.geometry;
    let mut fields = serde_json::Map::new();
    for field in RouterField::ALL {
        fields.insert(
            field.name().to_owned(),
            project(&graph.nodes, graph.router_roots.unwrap()[field as usize]),
        );
    }
    json!({"noise_router":fields,"noise":{"min_y":geometry.min_y(),"height":geometry.height(),"size_horizontal":geometry.cell_width()/4,"size_vertical":geometry.cell_height()/4,"cell_width":geometry.cell_width(),"cell_height":geometry.cell_height()},"sea_level":metadata.sea_level,"aquifers_enabled":metadata.aquifers,"ore_veins_enabled":metadata.ore_veins,"disable_mob_generation":metadata.disable_mobs,"legacy_random_source":matches!(metadata.algorithm, crate::random::RandomAlgorithm::Legacy)})
}

#[test]
fn actual_original_jar_codec_projections_all_35_definitions_and_7_routers() {
    let corpus = include_str!("../../../tests/fixtures/density_compiler_1_21_8.jsonl");
    let mut definitions = 0;
    let mut routers = 0;
    let mut probes = 0;
    let mut settings_probes = 0;
    for line in corpus.lines() {
        let record: serde_json::Value = serde_json::from_str(line).unwrap();
        let id = record["id"].as_str().unwrap();
        match record["kind"].as_str().unwrap() {
            "density" => {
                let id = DensityFunctionResourceId::from_name(id).unwrap();
                let template = DensityTemplate::compile_definition(id).unwrap();
                assert_eq!(
                    project(&template.graph.nodes, template.graph.roots[&id]),
                    record["projection"],
                    "{id:?}"
                );
                definitions += 1;
            }
            "router" => {
                let id = NoiseSettingsResourceId::from_name(id).unwrap();
                let template = RouterTemplate::compile_builtin(id).unwrap();
                assert_eq!(
                    project_router(&template.graph, template.metadata),
                    record["projection"],
                    "{id:?}"
                );
                let stats = graph::measure(
                    &template.graph.nodes,
                    Some(&template.graph.router_roots.unwrap()),
                );
                if id.resource_location().ends_with("overworld")
                    || id.resource_location().ends_with("amplified")
                    || id.resource_location().ends_with("large_biomes")
                {
                    assert_eq!(
                        (stats.vertices(), stats.edges(), stats.depth()),
                        (1091, 1217, 87)
                    );
                } else if id.resource_location().ends_with(":end") {
                    assert_eq!(
                        (stats.vertices(), stats.edges(), stats.depth()),
                        (100, 85, 24)
                    );
                } else {
                    assert_eq!(
                        (stats.vertices(), stats.edges(), stats.depth()),
                        (82, 69, 22)
                    );
                }
                routers += 1;
            }
            "probe" => {
                let source = record["input_json"].as_str().unwrap();
                let result = json::parse(source, "test").and_then(|(value, _)| {
                    let mut compiler = Compiler::new();
                    let root = if id.starts_with("holder_") {
                        compiler.density_holder(&value, false)?
                    } else {
                        compiler.direct(&value)?
                    };
                    Ok(project(&compiler.nodes, root))
                });
                let policy_reject = matches!(
                    id,
                    "spline_infinite_float"
                        | "spline_duplicate_signed_zero"
                        | "spline_reversed_locations"
                        | "unknown_field_ignored"
                        | "noise_infinite_scale"
                        | "noise_inline_parameters"
                );
                if record["status"] == "error" || policy_reject {
                    assert!(result.is_err(), "{id}");
                } else {
                    assert_eq!(result.unwrap(), record["projection"], "{id}");
                }
                probes += 1;
            }
            "settings_probe" => {
                check_settings_probe(&record);
                settings_probes += 1;
            }
            // Real direct factories establish why seed/visitor lowering must be
            // preserved in the next task; this compiler has no public factory.
            "factory" => {}
            kind => panic!("unknown oracle record {kind}"),
        }
    }
    assert_eq!(
        (definitions, routers, probes, settings_probes),
        (35, 7, 72, 8)
    );
}

fn check_settings_probe(record: &serde_json::Value) {
    let source: &'static str = Box::leak(
        record["input_json"]
            .as_str()
            .unwrap()
            .to_owned()
            .into_boxed_str(),
    );
    let result = json::parse(source, "test").and_then(|(value, _)| {
        let mut compiler = Compiler::new();
        let (roots, metadata) = compiler.settings(&value, source)?;
        let stats = graph::validate(&mut compiler.nodes)?;
        let graph = Graph {
            nodes: compiler.nodes.into_boxed_slice(),
            roots: compiler.roots,
            router_roots: Some(roots),
            stats,
            source_bytes: source.len(),
            noises: Box::new([]),
        };
        Ok(project_router(&graph, metadata))
    });
    if record["status"] == "error" {
        assert!(result.is_err(), "{}", record["id"]);
    } else {
        assert_eq!(result.unwrap(), record["projection"], "{}", record["id"]);
    }
}
