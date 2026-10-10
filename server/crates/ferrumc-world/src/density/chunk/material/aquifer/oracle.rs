//! Consumer of independent actual-JAR endpoint results and ordered requests.

use super::*;
use crate::density::chunk::material::ore;
use crate::density::chunk::material::tests::Floats;
use crate::density::chunk::tests::requests;
use crate::density::chunk::tests::router;
use crate::sample_work::SampleWork;
use serde_json::{json, Value};

mod addendum;

fn bits(value: &Value) -> u64 {
    u64::from_str_radix(value.as_str().unwrap(), 16).unwrap()
}
fn number(value: &Value) -> i32 {
    value.as_i64().unwrap().try_into().unwrap()
}
fn fluid(name: &Value) -> BlockStateId {
    match name.as_str().unwrap() {
        "air" => BlockStateId::AIR,
        "water" => BlockStateId::new(86),
        "lava" => BlockStateId::new(102),
        other => panic!("undeclared fluid {other}"),
    }
}
fn status(value: &Value) -> FluidStatus {
    FluidStatus {
        level: number(&value["height"]),
        kind: fluid(&value["fluid"]),
    }
}
struct Prepared {
    session: NoiseChunkSession,
    blocks: MaterialBlocks,
    aquifer: Aquifer,
    domain: MaterialDomain,
}
fn prepare(row: &Value, budget: usize) -> Prepared {
    let inputs = &row["inputs"];
    let mut session = requests::fresh(false);
    session.state.requests.as_mut().unwrap().material_values = true;
    if let Some(seed) = inputs.get("actual_seed") {
        session.data = router(
            "minecraft:overworld",
            seed.as_str().unwrap().parse().unwrap(),
        )
        .data;
    }
    session.state.start = BlockPos::new(
        number(&inputs["x"]),
        number(&inputs["y"]),
        number(&inputs["z"]),
    );
    if matches!(row["kind"].as_str().unwrap(), "pressure" | "ore")
        || inputs["endpoint"] == "pressure"
        || (inputs["endpoint"] == "enabled_substance" && inputs["context"] == "SinglePoint")
    {
        session.state.material_context_override = Some(inputs["context"] == "NoiseChunk");
    }
    for (field, label) in [
        (0, "barrier"),
        (1, "floodedness"),
        (2, "spread"),
        (3, "lava"),
        (7, "erosion"),
        (8, "depth"),
        (12, "toggle"),
        (13, "ridged"),
        (14, "gap"),
    ] {
        let value = f64::from_bits(bits(&inputs["leaves"][label]));
        session.fields[field] = requests::leaf(&mut session, label, &[value], (value, value));
    }
    declared_inputs(&mut session, inputs);
    let blocks = MaterialBlocks::new(
        session.data.metadata.default_block,
        session.data.metadata.default_fluid,
        &mut session.work,
    )
    .unwrap();
    let mut aquifer = Aquifer::new(&mut session).unwrap();
    let domain = MaterialDomain::new(&session).unwrap();
    if let Some(centers) = inputs.get("centers") {
        let centers = centers.as_array().unwrap();
        assert_eq!(centers.len(), 12);
        for center in centers {
            let grid = &center["grid"];
            let point = &center["point"];
            let grid = BlockPos::new(number(&grid[0]), number(&grid[1]), number(&grid[2]));
            let point = BlockPos::new(number(&point[0]), number(&point[1]), number(&point[2]));
            let index = aquifer.index(grid).unwrap();
            assert_eq!(
                aquifer.locations[index],
                i64::MAX,
                "duplicate declared slot"
            );
            domain.center(point).unwrap();
            aquifer.locations[index] = pack(point);
        }
    }
    session.work = SampleWork::new(budget);
    Prepared {
        session,
        blocks,
        aquifer,
        domain,
    }
}

fn declared_inputs(session: &mut NoiseChunkSession, inputs: &Value) {
    if let Some(preliminary) = inputs.get("preliminary") {
        let value = f64::from_bits(bits(&preliminary["default_f64_bits"]));
        let root = requests::leaf(session, "preliminary", &[value], (value, value));
        session.fields[10] = root;
        let overrides = preliminary["overrides"]
            .as_array()
            .unwrap()
            .iter()
            .map(|point| {
                (
                    BlockPos::new(
                        number(&point["x"]),
                        number(&point["y"]),
                        number(&point["z"]),
                    ),
                    f64::from_bits(bits(&point["value_f64_bits"])),
                )
            })
            .collect::<Vec<_>>();
        requests::point_overrides(session, root, &overrides);
    }
    if let Some(picker) = inputs.get("picker") {
        let default = status(&picker["default"]);
        let overrides = picker["overrides"]
            .as_array()
            .unwrap()
            .iter()
            .map(|point| {
                let value = status(&point["status"]);
                (
                    BlockPos::new(
                        number(&point["x"]),
                        number(&point["y"]),
                        number(&point["z"]),
                    ),
                    value.level,
                    value.kind,
                )
            })
            .collect::<Vec<_>>();
        session.state.material_picker = Some(crate::density::chunk::material::tests::Picker::new(
            (default.level, default.kind),
            &overrides,
        ));
    }
    if let Some(values) = inputs.get("float_bits") {
        let values = values
            .as_array()
            .unwrap()
            .iter()
            .map(|value| f32::from_bits(u32::from_str_radix(value.as_str().unwrap(), 16).unwrap()))
            .collect::<Vec<_>>();
        if !values.is_empty() {
            session.state.material_floats = Some(Floats::new(&values));
        }
    }
}

fn execute(prepared: &mut Prepared, row: &Value) -> Result<Value, NoiseChunkError> {
    let Prepared {
        session,
        blocks,
        aquifer,
        domain,
    } = prepared;
    let inputs = &row["inputs"];
    let pos = session.state.pos();
    match row["kind"].as_str().unwrap() {
        "pressure" => {
            let initial = bits(&inputs["sentinel_f64_bits"]);
            let value = f64::from_bits(initial);
            let mut sentinel = if value.is_nan() { None } else { Some(value) };
            let mut values = Vec::new();
            let mut sentinels = Vec::new();
            for _ in 0..number(&inputs["calls"]) {
                let value = Aquifer::pressure(
                    session,
                    blocks,
                    &mut sentinel,
                    status(&inputs["first"]),
                    status(&inputs["second"]),
                )?;
                values.push(format!("{:016x}", value.to_bits()));
                sentinels.push(format!("{:016x}", sentinel.map_or(initial, f64::to_bits)));
            }
            Ok(json!({"value_f64_bits":values,"sentinel_after_calls_f64_bits":sentinels}))
        }
        "surface_level" => Ok(
            json!({"surface_level": Aquifer::surface_level(session, *domain, pos,
            status(&inputs["fluid"]), number(&inputs["max_surface"]), inputs["fluid_present"].as_bool().unwrap())?}),
        ),
        "random_surface" => Ok(
            json!({"surface_level": Aquifer::random_surface(session, *domain, pos,
            number(&inputs["max_surface"]))?}),
        ),
        "fluid_type" => Ok(
            json!({"raw_state_id":Aquifer::fluid_type(session, blocks, *domain, pos,
            status(&inputs["fluid"]), number(&inputs["surface_level"]))?.as_u32()}),
        ),
        "ore" => Ok(
            json!({"raw_state_id":ore::compute(session, blocks)?.map_or(-1_i64, |state|i64::from(state.as_u32()))}),
        ),
        "disabled_substance" => {
            let global = status(&inputs["fluid"]);
            aquifer.enabled = false;
            aquifer.sea_level = global.level;
            blocks.default_fluid = global.kind;
            Ok(
                json!({"raw_state_id":aquifer.compute(session, blocks, *domain,
                f64::from_bits(bits(&inputs["substance_f64_bits"])))?.map_or(-1_i64, |state|i64::from(state.as_u32())),
                "schedule":aquifer.schedule}),
            )
        }
        other => panic!("undeclared actual-JAR endpoint {other}"),
    }
}

fn trace(prepared: &mut Prepared, row: &Value) -> Vec<Value> {
    let declared = row["inputs"].get("float_bits").is_some();
    requests::take_trace(&mut prepared.session).into_iter().map(|event| {
        match event["kind"].as_str().unwrap() {
            "request" => {
                let label = event["case"].as_str().unwrap();
                let context = &event["context"];
                json!({"kind":"density","label":label,"x":context["x"],"y":context["y"],"z":context["z"],
                    "own_context":if context["own"].as_bool().unwrap() {"NoiseChunk"} else {"SinglePoint"},
                    "value_f64_bits":event["value_f64_bits"]})
            }
            "random_at" => json!({"kind":"factory_at","x":event["x"],"y":event["y"],"z":event["z"],
                "mode":if declared {"declared_floats"} else {"actual"}}),
            "next_float" => json!({"kind":"next_float","f32_bits":event["bits"]}),
            "next_int" | "global_fluid" => event,
            other => panic!("undeclared material trace event {other}"),
        }
    }).collect()
}

fn records() -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/material_branch_1_21_8/branches.jsonl");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn actual_paper_material_endpoint_bits_contexts_and_ordered_random_trace() {
    let rows = records();
    assert_eq!(rows.len(), 499);
    for row in &rows {
        let mut prepared = prepare(row, 1_000_000);
        let output = execute(&mut prepared, row).unwrap();
        assert_eq!(output, row["result"], "{} result", row["case"]);
        let events = trace(&mut prepared, row);
        assert_eq!(
            events,
            *row["events"].as_array().unwrap(),
            "{} events",
            row["case"]
        );
        let requests = events
            .iter()
            .filter(|event| event["kind"] == "density")
            .cloned()
            .collect::<Vec<_>>();
        let random = events
            .iter()
            .filter(|event| event["kind"] != "density")
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            requests,
            *row["requests"].as_array().unwrap(),
            "{} requests",
            row["case"]
        );
        assert_eq!(
            random,
            *row["rng_events"].as_array().unwrap(),
            "{} random",
            row["case"]
        );
    }
}

#[test]
fn every_actual_material_endpoint_has_an_exact_shared_budget_boundary() {
    for row in &records() {
        let mut generous = prepare(row, 1_000_000);
        let expected = execute(&mut generous, row).unwrap();
        let units = generous.session.work.used();
        assert!(units > 0);
        let events = trace(&mut generous, row);
        let mut exact = prepare(row, units);
        assert_eq!(
            execute(&mut exact, row).unwrap(),
            expected,
            "{} exact",
            row["case"]
        );
        assert_eq!(exact.session.work.used(), units);
        assert_eq!(
            trace(&mut exact, row),
            events,
            "{} exact trace",
            row["case"]
        );
        let mut short = prepare(row, units - 1);
        let error = execute(&mut short, row).unwrap_err();
        crate::density::chunk::material::tests::exhausted(&error, units - 1);
        assert_eq!(short.session.work.used(), units - 1);
        assert!(
            events.starts_with(&trace(&mut short, row)),
            "{} short trace prefix",
            row["case"]
        );
    }
}
