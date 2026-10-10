use super::projection::compare_projection;
use super::trace::compare_original_trace;
use super::*;
const ORIGINAL: &str = include_str!("../../../../tests/fixtures/density_router_1_21_8.jsonl");

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Keeps the complete original reference or shared-meter acceptance matrix together."
)]
fn actual_original_randomstate_bounds_scalar_and_climate_all_settings_seeds() {
    let work = DensityWorkBudget::new(MAX_DENSITY_SAMPLE_WORK).unwrap();
    let mut routers = BTreeMap::new();
    let mut counts = BTreeMap::new();
    let mut compared = 0;
    let mut alternate_noises = BTreeMap::new();
    let mut unused_original_noises = BTreeMap::new();
    let mut original_noisemax_pins = 0;
    let mut original_factory_draws = 0;
    for line in ORIGINAL.lines() {
        let record: Value = serde_json::from_str(line).unwrap();
        let kind = record["kind"].as_str().unwrap();
        *counts.entry(kind.to_owned()).or_insert(0) += 1;
        if kind == "quantize" {
            assert_eq!(
                quantize(f32::from_bits(bits32(&record["input_bits"]))),
                record["quantized"].as_i64().unwrap(),
                "{}",
                record["case"]
            );
            continue;
        }
        if kind == "trace" {
            compare_original_trace(&record);
            continue;
        }
        let name = record["settings"].as_str().unwrap();
        let seed = record["seed"].as_i64().unwrap();
        let key = (name.to_owned(), seed);
        let router = routers.entry(key).or_insert_with(|| {
            RouterTemplate::compile_builtin(NoiseSettingsResourceId::from_name(name).unwrap())
                .unwrap()
                .bind_seed(WorldSeed::new(seed))
                .unwrap()
        });
        let context = format!("{name}/{seed}");
        match kind {
            "binding" => {
                assert_eq!(record["noise_max_values"].as_object().unwrap().len(), 60);
                original_noisemax_pins += 60;
                for (id, noise) in &router.data.noises {
                    assert_eq!(
                        noise.max_value().to_bits(),
                        bits64(&record["noise_max_values"][id.resource_location()]),
                        "{context}/{}",
                        id.resource_location()
                    );
                }
                for (factory, key) in [
                    (router.data.aquifer_random, "aquifer_draws"),
                    (router.data.ore_random, "ore_draws"),
                ] {
                    let draws = record[key].as_array().unwrap();
                    assert_eq!(draws.len(), 7);
                    for draw in draws.iter().rev() {
                        let mut random = factory.at(position(draw));
                        for expected in draw["next_longs"].as_array().unwrap() {
                            assert_eq!(
                                random.next_long(),
                                expected.as_i64().unwrap(),
                                "{context}/{key}"
                            );
                            original_factory_draws += 1;
                        }
                    }
                }
                for field in RouterField::ALL {
                    compare_bounds(
                        router.field_bounds(field),
                        &record["field_bounds"][field.name()],
                        &format!("{context}/{}", field.name()),
                    );
                }
                for (name, expected) in record["named_bounds"].as_object().unwrap() {
                    compare_bounds(
                        router
                            .definition_bounds(DensityFunctionResourceId::from_name(name).unwrap()),
                        expected,
                        &format!("{context}/{name}"),
                    );
                }
                for axis in ClimateAxis::ALL {
                    compare_bounds(
                        router.climate_bounds(axis),
                        &record["climate_bounds"][axis as usize],
                        &format!("{context}/{}", axis.name()),
                    );
                }
            }
            "point" => {
                let pos = position(&record);
                for field in RouterField::ALL {
                    compare_value(
                        router.sample_field(field, pos, work),
                        &record["fields"][field.name()],
                        &format!("{context}/{}/ {pos:?}", field.name()),
                    );
                    compared += 1;
                }
                for (name, expected) in record["named"].as_object().unwrap() {
                    compare_value(
                        router.sample_definition(
                            DensityFunctionResourceId::from_name(name).unwrap(),
                            pos,
                            work,
                        ),
                        expected,
                        &format!("{context}/{name}/{pos:?}"),
                    );
                    compared += 1;
                }
                compare_climate(router.sample_climate_at_block(pos, work), &record, &context);
            }
            "quart" => {
                let pos = position(&record);
                let quart = ClimateQuartPos::new(pos.x(), pos.y(), pos.z()).unwrap();
                compare_climate(router.sample_climate(quart, work), &record, &context);
            }
            "wrapped_noise" => {
                let id = NoiseResourceId::from_name(record["noise"].as_str().unwrap()).unwrap();
                let mode = record["name_mode"].as_str().unwrap();
                let noise = if mode == "canonical" {
                    if let Some((_, noise)) = router
                        .data
                        .noises
                        .iter()
                        .find(|(resource, _)| *resource == id)
                    {
                        noise
                    } else {
                        unused_original_noises
                            .entry((name.to_owned(), seed, id))
                            .or_insert_with(|| {
                                // Actual oracle explicitly constructs unused registry
                                // noises; these tests do not add them to a Rust router.
                                let template =
                                    RouterTemplate::compile_builtin(router.resource()).unwrap();
                                let mut initial = crate::random::WorldgenRandomSource::new(
                                    template.metadata.algorithm,
                                    WorldSeed::new(seed),
                                );
                                crate::density::bind::bind_noise(
                                    template.uses_legacy_random_source(),
                                    WorldSeed::new(seed),
                                    initial.fork_positional(),
                                    id,
                                )
                                .unwrap()
                            })
                    }
                } else {
                    alternate_noises
                        .entry((name.to_owned(), seed, id, mode.to_owned()))
                        .or_insert_with(|| {
                            let template =
                                RouterTemplate::compile_builtin(router.resource()).unwrap();
                            let mut initial = crate::random::WorldgenRandomSource::new(
                                template.metadata.algorithm,
                                WorldSeed::new(seed),
                            );
                            let factory = initial.fork_positional();
                            let slot = [
                                "minecraft:jagged",
                                "minecraft:temperature",
                                "minecraft:vegetation",
                                "minecraft:offset",
                                "minecraft:continentalness",
                            ]
                            .iter()
                            .position(|&name| name == id.resource_location())
                            .unwrap();
                            let random_name = if mode == "shortname" {
                                id.resource_location()
                                    .strip_prefix("minecraft:")
                                    .unwrap()
                                    .to_owned()
                            } else {
                                format!("slot_{slot}")
                            };
                            let parameters = id.parameters();
                            crate::noise::NormalNoise::new(
                                &mut factory.from_hash_of(&random_name),
                                parameters.first_octave(),
                                parameters.amplitudes(),
                            )
                            .unwrap()
                        })
                };
                let input = &record["input_bits"];
                let position = ferrumc_math::Vec3::new(
                    f64::from_bits(bits64(&input[0])),
                    f64::from_bits(bits64(&input[1])),
                    f64::from_bits(bits64(&input[2])),
                );
                let mut meter = SampleWork::new(MAX_DENSITY_SAMPLE_WORK);
                let (actual, maximum) = match record["stack"].as_str().unwrap() {
                    "normal" => (
                        noise.sample_consumer(position, &mut meter),
                        noise.max_value(),
                    ),
                    "first" => (
                        noise.first().sample_consumer(position, &mut meter),
                        noise.first().max_value(),
                    ),
                    "second" => (
                        noise.second().sample_consumer(position, &mut meter),
                        noise.second().max_value(),
                    ),
                    _ => panic!("unknown actual stack"),
                };
                let context = format!(
                    "{context}/{}/{mode}/{}/{}",
                    id.resource_location(),
                    record["stack"],
                    record["case"]
                );
                assert_eq!(
                    actual
                        .unwrap_or_else(|error| panic!("{context}: {error:?}"))
                        .to_bits(),
                    bits64(&record["result_bits"]),
                    "{context}"
                );
                assert_eq!(
                    maximum.to_bits(),
                    bits64(&record["max_value_bits"]),
                    "{context}"
                );
            }
            "projection" => {
                for field in RouterField::ALL {
                    compare_projection(
                        &router.data.router,
                        router.data.fields[field as usize],
                        &record["fields"][field.name()],
                        &format!("{context}/{}", field.name()),
                    );
                }
                for (name, expected) in record["named"].as_object().unwrap() {
                    let id = DensityFunctionResourceId::from_name(name).unwrap();
                    compare_projection(
                        &router.data.router,
                        router.data.definitions[&id],
                        expected,
                        &format!("{context}/{name}"),
                    );
                }
                for (name, expected) in record["registered_named"].as_object().unwrap() {
                    let id = DensityFunctionResourceId::from_name(name).unwrap();
                    compare_projection(
                        &router.data.router,
                        router.data.registered_definitions[&id],
                        expected,
                        &format!("{context}/registered/{name}"),
                    );
                }
                for axis in ClimateAxis::ALL {
                    compare_projection(
                        &router.data.climate,
                        router.data.climate_fields[axis as usize],
                        &record["climate"][axis as usize],
                        &format!("{context}/{}", axis.name()),
                    );
                }
            }
            _ => panic!("unknown original record: {kind}"),
        }
    }
    assert_eq!(routers.len(), 49);
    assert_eq!(
        counts,
        BTreeMap::from([
            ("binding".to_owned(), 49),
            ("point".to_owned(), 1813),
            ("projection".to_owned(), 7),
            ("quart".to_owned(), 294),
            ("quantize".to_owned(), 42),
            ("trace".to_owned(), 308),
            ("wrapped_noise".to_owned(), 14700)
        ])
    );
    assert_eq!(compared, 90_650);
    assert_eq!(original_noisemax_pins, 2940);
    assert_eq!(original_factory_draws, 2058);
    assert_eq!(alternate_noises.len(), 490);
}

fn compare_climate(
    actual: Result<ClimateSample, DensitySampleError>,
    record: &Value,
    context: &str,
) {
    let expected = record["climate_f32"].as_array().unwrap();
    if expected
        .iter()
        .any(|value| !f32::from_bits(bits32(value)).is_finite())
    {
        assert!(
            matches!(
                actual.unwrap_err().kind(),
                DensitySampleErrorKind::NonFiniteDensity { .. }
                    | DensitySampleErrorKind::NonFiniteCoordinate { .. }
                    | DensitySampleErrorKind::NonFiniteSpline { .. }
            ),
            "{context}"
        );
    } else {
        let actual = actual.unwrap_or_else(|error| panic!("{context}: {error:?}"));
        for axis in ClimateAxis::ALL {
            assert_eq!(
                actual.value(axis).to_bits(),
                bits32(&expected[axis as usize]),
                "{context}/{}",
                axis.name()
            );
            assert_eq!(
                actual.quantized_value(axis),
                record["climate_quantized"][axis as usize].as_i64().unwrap(),
                "{context}/{}",
                axis.name()
            );
        }
    }
}
