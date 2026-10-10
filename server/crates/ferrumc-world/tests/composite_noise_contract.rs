//! Bitwise comparisons against direct reflection calls into the original JAR.

use std::collections::BTreeMap;

use ferrumc_math::{BlockPos, Vec3};
use ferrumc_world::noise::{
    BlendedNoise, BlendedNoiseParameters, ImprovedNoise, NoiseError, NormalNoise, PerlinNoise,
    SimplexNoise, MAX_NOISE_COORDINATE, MAX_NOISE_OCTAVES, MAX_NORMAL_NOISE_COORDINATE,
};
use ferrumc_world::random::{RandomAlgorithm, WorldgenRandomSource};
use ferrumc_world::WorldSeed;

const GOLDENS: &str = include_str!("fixtures/composite_noise_1_21_8.tsv");

fn random(kind: &str, seed: &str) -> WorldgenRandomSource {
    let algorithm = match kind {
        "L" => RandomAlgorithm::Legacy,
        "X" => RandomAlgorithm::Xoroshiro128PlusPlus,
        _ => panic!("unknown original random algorithm"),
    };
    WorldgenRandomSource::new(algorithm, WorldSeed::new(seed.parse().unwrap()))
}

fn float(value: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(value, 16).unwrap())
}
fn bits(actual: f64, expected: &str, line: usize) {
    assert_eq!(
        actual.to_bits(),
        float(expected).to_bits(),
        "original oracle line {line}"
    );
}
fn position(values: &[&str]) -> Vec3 {
    Vec3::new(float(values[0]), float(values[1]), float(values[2]))
}
fn block_position(values: &[&str]) -> BlockPos {
    BlockPos::new(
        values[0].parse().unwrap(),
        values[1].parse().unwrap(),
        values[2].parse().unwrap(),
    )
}
fn permutation(actual: &[u8; 256], expected: &str, line: usize) {
    assert_eq!(expected.len(), 512);
    for (byte, encoded) in actual.iter().zip(expected.as_bytes().as_chunks::<2>().0) {
        assert_eq!(
            *byte,
            u8::from_str_radix(std::str::from_utf8(encoded).unwrap(), 16).unwrap(),
            "permutation line {line}"
        );
    }
}
fn offsets(actual: Vec3, expected: &str, line: usize) {
    for (component, encoded) in [actual.x, actual.y, actual.z]
        .into_iter()
        .zip(expected.split(','))
    {
        bits(component, encoded, line);
    }
}
fn octave(actual: Option<&ImprovedNoise>, fields: &[&str], line: usize) {
    if fields[0] == "null" {
        assert!(actual.is_none(), "gap line {line}");
    } else {
        let noise = actual.unwrap();
        offsets(noise.offsets(), fields[0], line);
        permutation(noise.permutation(), fields[1], line);
    }
}
fn parameters(encoded: &str) -> BlendedNoiseParameters {
    let values: Vec<_> = encoded.split(',').map(float).collect();
    let config =
        BlendedNoiseParameters::new(values[0], values[1], values[2], values[3], values[4]).unwrap();
    assert_eq!(
        [
            config.xz_scale(),
            config.y_scale(),
            config.xz_factor(),
            config.y_factor(),
            config.smear_scale_multiplier()
        ],
        values.as_slice()
    );
    config
}
fn stack<'a>(noise: &'a BlendedNoise, label: &str) -> &'a PerlinNoise {
    match label {
        "min" => noise.min_limit_noise(),
        "max" => noise.max_limit_noise(),
        "main" => noise.main_noise(),
        _ => panic!("unknown stack"),
    }
}

#[test]
fn original_simplex_offsets_parent_consumption_and_2d_3d_samples() {
    let mut samplers = BTreeMap::new();
    let mut records = 0;
    for (line, record) in GOLDENS.lines().enumerate() {
        let f: Vec<_> = record.split('|').collect();
        match f[0] {
            "S" => {
                let mut rng = random(f[2], f[3]);
                rng.consume_count(f[4].parse().unwrap()).unwrap();
                let noise = SimplexNoise::new(&mut rng);
                offsets(noise.offsets(), f[5], line);
                permutation(noise.permutation(), f[6], line);
                assert_eq!(
                    rng.next_long(),
                    f[7].parse::<i64>().unwrap(),
                    "Simplex parent consumption line {line}"
                );
                samplers.insert(f[1], noise);
            }
            "S2" => bits(
                samplers[f[1]].sample_2d(float(f[2]), float(f[3])).unwrap(),
                f[4],
                line,
            ),
            "S3" => bits(
                samplers[f[1]].sample(position(&f[2..5])).unwrap(),
                f[5],
                line,
            ),
            _ => continue,
        }
        records += 1;
    }
    assert_eq!(samplers.len(), 32);
    assert_eq!(records, 1504);
}

#[test]
fn original_normal_modern_legacy_stacks_gaps_bounds_and_scaled_coordinates() {
    let mut samplers = BTreeMap::new();
    let mut records = 0;
    for (line, record) in GOLDENS.lines().enumerate() {
        let f: Vec<_> = record.split('|').collect();
        match f[0] {
            "N" => {
                let mut rng = random(f[2], f[3]);
                let first = f[5].parse().unwrap();
                let amplitudes: Vec<_> = f[6].split(',').map(float).collect();
                let noise = if f[4] == "true" {
                    NormalNoise::legacy(&mut rng, first, &amplitudes)
                } else {
                    NormalNoise::new(&mut rng, first, &amplitudes)
                }
                .unwrap();
                for perlin in [noise.first(), noise.second()] {
                    assert_eq!(perlin.first_octave(), first);
                    assert_eq!(
                        perlin.amplitudes().map(f64::to_bits).collect::<Vec<_>>(),
                        amplitudes.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                    );
                }
                bits(noise.max_value(), f[7], line);
                bits(noise.value_factor(), f[8], line);
                assert_eq!(
                    rng.next_long(),
                    f[9].parse::<i64>().unwrap(),
                    "Normal parent consumption line {line}"
                );
                samplers.insert(f[1], noise);
            }
            "NO" => {
                let noise = &samplers[f[1]];
                let selected = if f[2] == "first" {
                    noise.first()
                } else {
                    assert_eq!(f[2], "second");
                    noise.second()
                };
                octave(selected.octave_noise(f[3].parse().unwrap()), &f[4..], line);
            }
            "NV" => bits(
                samplers[f[1]].sample(position(&f[2..5])).unwrap(),
                f[5],
                line,
            ),
            _ => continue,
        }
        records += 1;
    }
    assert_eq!(samplers.len(), 224);
    assert_eq!(records, 8128);
}

#[test]
fn original_blended_stacks_reseed_bounds_and_full_i32_block_coordinates() {
    let mut samplers = BTreeMap::new();
    let mut previous = BTreeMap::new();
    let mut records = 0;
    for (line, record) in GOLDENS.lines().enumerate() {
        let f: Vec<_> = record.split('|').collect();
        match f[0] {
            "B" => {
                let mut rng = random(f[2], f[3]);
                let config = parameters(f[4]);
                let noise = if f[8] == "-" {
                    BlendedNoise::new(&mut rng, config).unwrap()
                } else {
                    let mut origin = random(f[8], f[9]);
                    let old = BlendedNoise::new(&mut origin, config).unwrap();
                    let new = old.with_new_random(&mut rng).unwrap();
                    assert_eq!(
                        origin.next_long(),
                        f[10].parse::<i64>().unwrap(),
                        "withNewRandom original parent line {line}"
                    );
                    previous.insert(f[1], old);
                    new
                };
                assert_eq!(noise.parameters(), config);
                bits(noise.max_value(), f[5], line);
                bits(noise.min_value(), f[6], line);
                assert_eq!(
                    rng.next_long(),
                    f[7].parse::<i64>().unwrap(),
                    "Blended parent consumption line {line}"
                );
                samplers.insert(f[1], noise);
            }
            "BO" => octave(
                stack(&samplers[f[1]], f[2]).octave_noise(f[3].parse().unwrap()),
                &f[4..],
                line,
            ),
            "BV" => bits(
                samplers[f[1]].sample(block_position(&f[2..5])).unwrap(),
                f[5],
                line,
            ),
            "BA" => bits(
                previous[f[1]].sample(BlockPos::new(1, 1, 1)).unwrap(),
                f[2],
                line,
            ),
            _ => continue,
        }
        records += 1;
    }
    assert_eq!(samplers.len(), 128);
    assert_eq!(previous.len(), 16);
    assert_eq!(records, 7312);
}

#[test]
fn normal_configuration_errors_preserve_both_random_algorithms() {
    for kind in ["L", "X"] {
        let original = random(kind, "-982");
        for legacy in [false, true] {
            for (first, amplitudes, expected) in [
                (0, Vec::new(), NoiseError::OctaveCount(0)),
                (
                    -64,
                    vec![1.0; MAX_NOISE_OCTAVES + 1],
                    NoiseError::OctaveCount(MAX_NOISE_OCTAVES + 1),
                ),
                (i32::MIN, vec![1.0], NoiseError::OctaveOutOfRange(i32::MIN)),
                (i32::MAX, vec![1.0], NoiseError::OctaveOutOfRange(i32::MAX)),
                (-1, vec![1.0, f64::NAN], NoiseError::NonFiniteAmplitude(1)),
                (-1, vec![f64::INFINITY], NoiseError::NonFiniteAmplitude(0)),
                (0, vec![f64::MAX / 2.0], NoiseError::NonFiniteResult),
                (0, vec![-f64::MAX / 2.0], NoiseError::NonFiniteResult),
            ] {
                let mut rng = original;
                let result = if legacy {
                    NormalNoise::legacy(&mut rng, first, &amplitudes)
                } else {
                    NormalNoise::new(&mut rng, first, &amplitudes)
                };
                assert_eq!(result.unwrap_err(), expected);
                assert_eq!(
                    rng, original,
                    "configuration error mutated {kind}, legacy={legacy}"
                );
            }
        }
        let mut rng = original;
        assert_eq!(
            NormalNoise::legacy(&mut rng, 0, &[0.0, -0.0]).unwrap_err(),
            NoiseError::PositiveLegacyOctave
        );
        assert_eq!(rng, original);
        assert_eq!(
            NormalNoise::new(&mut rng, 32, &[1.0, 0.0]).unwrap_err(),
            NoiseError::OctaveOutOfRange(33)
        );
        assert_eq!(rng, original);
    }
}

#[test]
fn blended_parameters_validate_each_codec_boundary_and_nonfinite_input() {
    for index in 0..5 {
        let (minimum, maximum, label) = if index == 4 {
            (1.0_f64, 8.0_f64, "smear_scale_multiplier")
        } else {
            (
                0.001_f64,
                1000.0_f64,
                ["xz_scale", "y_scale", "xz_factor", "y_factor"][index],
            )
        };
        for rejected in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -0.0,
            minimum.next_down(),
            maximum.next_up(),
        ] {
            let mut values = [1.0; 5];
            values[index] = rejected;
            assert_eq!(
                BlendedNoiseParameters::new(values[0], values[1], values[2], values[3], values[4]),
                Err(NoiseError::InvalidBlendedParameter(label))
            );
        }
        for accepted in [minimum, maximum] {
            let mut values = [1.0; 5];
            values[index] = accepted;
            assert!(BlendedNoiseParameters::new(
                values[0], values[1], values[2], values[3], values[4]
            )
            .is_ok());
        }
    }
}

#[test]
fn simplex_and_normal_reject_unsupported_input_without_mutating_or_fallback() {
    let mut rng = random("X", "42");
    let simplex = SimplexNoise::new(&mut rng);
    let normal = NormalNoise::new(&mut rng, -3, &[1.0, 0.0, 0.5, 1.0]).unwrap();
    let parent = rng;
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        MAX_NOISE_COORDINATE.next_up(),
        -MAX_NOISE_COORDINATE.next_up(),
    ] {
        assert_eq!(
            simplex.sample_2d(invalid, 0.0),
            Err(NoiseError::InvalidInput)
        );
        assert_eq!(
            simplex.sample_2d(0.0, invalid),
            Err(NoiseError::InvalidInput)
        );
        for p in [
            Vec3::new(invalid, 0.0, 0.0),
            Vec3::new(0.0, invalid, 0.0),
            Vec3::new(0.0, 0.0, invalid),
        ] {
            assert_eq!(simplex.sample(p), Err(NoiseError::InvalidInput));
            assert_eq!(normal.sample(p), Err(NoiseError::InvalidInput));
        }
    }
    for value in [
        MAX_NORMAL_NOISE_COORDINATE.next_up(),
        -MAX_NORMAL_NOISE_COORDINATE.next_up(),
        MAX_NOISE_COORDINATE,
    ] {
        assert_eq!(
            normal.sample(Vec3::new(value, 0.0, 0.0)),
            Err(NoiseError::InvalidInput)
        );
    }
    assert!(normal
        .sample(Vec3::new(
            MAX_NORMAL_NOISE_COORDINATE,
            -MAX_NORMAL_NOISE_COORDINATE,
            0.0
        ))
        .is_ok());
    assert_eq!(rng, parent);
}

#[test]
fn pure_sampling_is_order_independent_and_clones_preserve_exact_results() {
    for kind in ["L", "X"] {
        let mut rng = random(kind, "-1");
        let simplex = SimplexNoise::new(&mut rng);
        let normal = NormalNoise::legacy(&mut rng, -3, &[1.0, 0.0, 0.5, 1.0]).unwrap();
        let blended = BlendedNoise::new(
            &mut rng,
            BlendedNoiseParameters::new(0.25, 0.125, 80.0, 160.0, 8.0).unwrap(),
        )
        .unwrap();
        let parent = rng;
        let p = Vec3::new(0.125, -64.0, 0.875);
        let block = BlockPos::new(-1, -64, 16);
        let expected = (
            simplex.sample(p).unwrap(),
            normal.sample(p).unwrap(),
            blended.sample(block).unwrap(),
        );
        for other in [Vec3::ZERO, Vec3::new(30_000_000.0, 255.5, -30_000_000.0)] {
            simplex.sample(other).unwrap();
            normal.sample(other).unwrap();
            blended
                .sample(BlockPos::new(i32::MIN, i32::MAX, 0))
                .unwrap();
        }
        assert_eq!(
            simplex.clone().sample(p).unwrap().to_bits(),
            expected.0.to_bits()
        );
        assert_eq!(
            normal.clone().sample(p).unwrap().to_bits(),
            expected.1.to_bits()
        );
        assert_eq!(
            blended.clone().sample(block).unwrap().to_bits(),
            expected.2.to_bits()
        );
        assert_eq!(rng, parent);
    }
}
