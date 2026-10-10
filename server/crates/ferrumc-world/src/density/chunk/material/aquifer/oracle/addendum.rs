//! Immutable original enabled/cache cases, separate from nonfinite admission.
use super::*;

fn records() -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/material_addendum_1_21_8/addendum.jsonl");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn snapshot(prepared: &Prepared) -> Value {
    let aquifer = &prepared.aquifer;
    let locations = aquifer.locations.iter().enumerate().filter_map(|(index, &packed)| {
        if packed == i64::MAX { return None; }
        let pos = unpack(packed);
        Some(json!({"index":index,"packed_i64":packed.to_string(),"x":pos.x(),"y":pos.y(),"z":pos.z()}))
    }).collect::<Vec<_>>();
    let statuses = aquifer.statuses.iter().enumerate().filter_map(|(index, value)| {
        value.map(|value| json!({"index":index,"height":value.level,"fluid_raw_state_id":value.kind.as_u32()}))
    }).collect::<Vec<_>>();
    let surface = &prepared.session.surface;
    let mut columns = surface.keys[..surface.len]
        .iter()
        .zip(&surface.values[..surface.len])
        .map(|(&key, &value)| (key.cast_signed(), value))
        .collect::<Vec<_>>();
    columns.sort_by_key(|&(key, _)| key);
    let preliminary = columns
        .into_iter()
        .map(|(key, value)| {
            json!({"packed_i64":key.to_string(),"x":(key as u32).cast_signed(),
            "z":((key.cast_unsigned() >> 32) as u32).cast_signed(),"surface_level":value})
        })
        .collect::<Vec<_>>();
    json!({"location_capacity":aquifer.locations.len(),"status_capacity":aquifer.statuses.len(),
        "locations":locations,"statuses":statuses,"preliminary":preliminary})
}

fn event_count(prepared: &Prepared) -> usize {
    prepared
        .session
        .state
        .requests
        .as_ref()
        .unwrap()
        .trace
        .len()
}

fn execute(prepared: &mut Prepared, row: &Value) -> Result<Value, NoiseChunkError> {
    let before = snapshot(prepared);
    let inputs = &row["inputs"];
    let mut calls = Vec::new();
    let mut caches = Vec::new();
    for call in inputs["calls"].as_array().unwrap() {
        let start = event_count(prepared);
        let Prepared {
            session,
            blocks,
            aquifer,
            domain,
        } = prepared;
        let pos = session.state.pos();
        let mut output = match inputs["endpoint"].as_str().unwrap() {
            "enabled_substance" => {
                let substance = f64::from_bits(bits(&call["substance_f64_bits"]));
                let value = aquifer.compute(session, blocks, *domain, substance)?;
                json!({"raw_state_id":value.map_or(-1_i64, |value| i64::from(value.as_u32())),"schedule":aquifer.schedule})
            }
            "fluid_scan" | "fluid_cache" => {
                let value = if inputs["endpoint"] == "fluid_scan" {
                    aquifer.compute_fluid(session, blocks, *domain, pos)?
                } else {
                    aquifer.status(session, blocks, *domain, pack(pos))?
                };
                json!({"height":value.level,"fluid_raw_state_id":value.kind.as_u32()})
            }
            "pressure" => {
                let initial = bits(&call["sentinel_f64_bits"]);
                let value = f64::from_bits(initial);
                let mut sentinel = if value.is_nan() { None } else { Some(value) };
                let result = Aquifer::pressure(
                    session,
                    blocks,
                    &mut sentinel,
                    status(&inputs["first"]),
                    status(&inputs["second"]),
                )?;
                json!({"value_f64_bits":format!("{:016x}",result.to_bits()),
                    "sentinel_after_f64_bits":format!("{:016x}",sentinel.map_or(initial,f64::to_bits))})
            }
            other => panic!("undeclared addendum endpoint {other}"),
        };
        output["events_start"] = json!(start);
        output["events_end"] = json!(event_count(prepared));
        calls.push(output);
        caches.push(snapshot(prepared));
    }
    Ok(json!({"cache_before":before,"calls":calls,"cache_after_calls":caches}))
}

fn compare_projections(events: &[Value], row: &Value) {
    for (key, kinds) in [
        ("requests", &["density"][..]),
        ("rng_events", &["factory_at", "next_int"][..]),
        ("global_requests", &["global_fluid"][..]),
    ] {
        let values = events
            .iter()
            .filter(|event| kinds.contains(&event["kind"].as_str().unwrap()))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            *row[key].as_array().unwrap(),
            "{} {key}",
            row["case"]
        );
    }
}

#[test]
fn actual_enabled_ties_status_preliminary_caches_and_ordered_events() {
    let rows = records();
    assert_eq!(rows.len(), 57);
    let finite = rows
        .iter()
        .filter(|row| row["policy"] == "finite_parity")
        .collect::<Vec<_>>();
    assert_eq!(finite.len(), 42);
    for row in finite {
        let mut prepared = prepare(row, 1_000_000);
        let output = execute(&mut prepared, row).unwrap();
        assert_eq!(
            output, row["result"],
            "{} complete result/caches",
            row["case"]
        );
        let events = trace(&mut prepared, row);
        assert_eq!(
            events,
            *row["events"].as_array().unwrap(),
            "{} events",
            row["case"]
        );
        compare_projections(&events, row);
    }
}

#[test]
fn every_finite_addendum_endpoint_has_an_exact_shared_budget_boundary() {
    for row in records()
        .iter()
        .filter(|row| row["policy"] == "finite_parity")
    {
        let mut generous = prepare(row, 1_000_000);
        let expected = execute(&mut generous, row).unwrap();
        let units = generous.session.work.used();
        assert!(units > 0);
        let events = trace(&mut generous, row);
        let mut exact = prepare(row, units);
        assert_eq!(
            execute(&mut exact, row).unwrap(),
            expected,
            "{} exact result/caches",
            row["case"]
        );
        assert_eq!(exact.session.work.used(), units);
        assert_eq!(
            trace(&mut exact, row),
            events,
            "{} exact events",
            row["case"]
        );
        let mut short = prepare(row, units - 1);
        let error = execute(&mut short, row).unwrap_err();
        crate::density::chunk::material::tests::exhausted(&error, units - 1);
        assert_eq!(short.session.work.used(), units - 1);
        assert!(
            events.starts_with(&trace(&mut short, row)),
            "{} short event prefix",
            row["case"]
        );
    }
}

#[test]
fn actual_nonfinite_probes_are_typed_rust_admission_refusals() {
    let rows = records();
    let probes = rows
        .iter()
        .filter(|row| row["policy"] == "nonfinite_scope_probe")
        .collect::<Vec<_>>();
    assert_eq!(probes.len(), 15);
    for row in probes {
        let label = row["case"].as_str().unwrap().split('_').next().unwrap();
        let input_bits = match label {
            "substance" => bits(&row["inputs"]["calls"][0]["substance_f64_bits"]),
            "preliminary" => bits(&row["inputs"]["preliminary"]["default_f64_bits"]),
            _ => bits(&row["inputs"]["leaves"][label]),
        };
        assert!(!f64::from_bits(input_bits).is_finite());
        // The Java result is evidence that refusal is additional policy; it is
        // deliberately not the expected result of this finite-only Rust path.
        assert_eq!(
            row["result"]["calls"].as_array().unwrap().len(),
            row["inputs"]["calls"].as_array().unwrap().len()
        );
        let mut prepared = prepare(row, 1_000_000);
        let error = execute(&mut prepared, row).unwrap_err();
        match error.kind() {
            NoiseChunkErrorKind::Density(source) => {
                assert!(
                    matches!(source.kind(), crate::density::DensitySampleErrorKind::NonFiniteDensity { bits } if *bits == input_bits),
                    "{} typed raw nonfinite bits: {error:?}",
                    row["case"]
                );
                assert_eq!(
                    source.field(),
                    if label == "substance" {
                        "aquifer_substance"
                    } else {
                        "noise_chunk"
                    }
                );
            }
            other => panic!("{} unexpected refusal {other:?}", row["case"]),
        }
        let events = trace(&mut prepared, row);
        assert!(
            row["events"].as_array().unwrap().starts_with(&events),
            "{} requests before typed refusal",
            row["case"]
        );
        if label == "preliminary" {
            assert_eq!(
                prepared.session.surface.len, 0,
                "{} unpublished failed surface",
                row["case"]
            );
        }
        assert!(
            prepared.aquifer.statuses.iter().all(Option::is_none),
            "{} unpublished failed status",
            row["case"]
        );
    }
}
