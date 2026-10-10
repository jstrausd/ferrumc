//! Goldens call the official signed original JAR directly through reflection;
//! tests compare integer outputs and raw float bits without epsilon allowances.

use std::collections::BTreeMap;

use ferrumc_math::{BlockPos, Vec3};
use ferrumc_world::noise::{
    wrap_noise_coordinate, ImprovedNoise, NoiseError, PerlinNoise, MAX_NOISE_COORDINATE,
    MAX_NOISE_OCTAVES,
};
use ferrumc_world::random::{
    RandomAlgorithm, RandomError, Seed128, WorldgenRandomSource, MAX_RANDOM_CONSUME_COUNT,
};
use ferrumc_world::WorldSeed;

const GOLDENS: &str = include_str!("fixtures/random_noise_1_21_8.tsv");

fn algorithm(kind: &str) -> RandomAlgorithm {
    match kind {
        "L" => RandomAlgorithm::Legacy,
        "X" => RandomAlgorithm::Xoroshiro128PlusPlus,
        _ => panic!("unexpected algorithm"),
    }
}

fn random(kind: &str, seed: &str) -> WorldgenRandomSource {
    WorldgenRandomSource::new(algorithm(kind), WorldSeed::new(seed.parse().unwrap()))
}

fn float(bits: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(bits, 16).unwrap())
}

fn position(fields: &[&str]) -> Vec3 {
    Vec3::new(float(fields[0]), float(fields[1]), float(fields[2]))
}

fn hex_bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_bits(value: f64, expected: &str, line: usize) {
    assert_eq!(
        value.to_bits(),
        u64::from_str_radix(expected, 16).unwrap(),
        "original Java fixture line {line}"
    );
}

fn assert_configuration(noise: &ImprovedNoise, offsets: &str, permutation: &str, line: usize) {
    let values: Vec<_> = offsets.split(',').collect();
    let actual = noise.offsets();
    for (value, expected) in [actual.x, actual.y, actual.z].into_iter().zip(values) {
        assert_bits(value, expected, line);
    }
    assert_eq!(
        noise.permutation().as_slice(),
        hex_bytes(permutation),
        "permutation line {line}"
    );
}

#[test]
fn original_java_random_streams_seed_upgrades_and_positional_factories() {
    let mut sources = BTreeMap::new();
    let mut raw_sources = BTreeMap::new();
    let mut records = 0;
    for (index, line) in GOLDENS.lines().enumerate() {
        let f: Vec<_> = line.split('|').collect();
        match f[0] {
            "U" => {
                let upgraded = Seed128::upgrade(WorldSeed::new(f[1].parse().unwrap()));
                assert_eq!(upgraded.low(), f[2].parse::<i64>().unwrap());
                assert_eq!(upgraded.high(), f[3].parse::<i64>().unwrap());
            }
            "E" => {
                let mut rejected = random(f[1], f[2]);
                let bound = f[3].parse().unwrap();
                assert_eq!(
                    rejected.next_int_bounded(bound),
                    Err(RandomError::InvalidBound(bound))
                );
                assert_eq!(
                    rejected.next_long(),
                    f[4].parse::<i64>().unwrap(),
                    "invalid Java bound consumption line {index}"
                );
            }
            "Z" => {
                let source = raw_sources.entry((f[1], f[2])).or_insert_with(|| {
                    WorldgenRandomSource::xoroshiro_128(Seed128::new(
                        f[1].parse().unwrap(),
                        f[2].parse().unwrap(),
                    ))
                });
                assert_eq!(
                    source.next_long(),
                    f[3].parse::<i64>().unwrap(),
                    "raw source line {index}"
                );
            }
            "R" => {
                let source = sources
                    .entry((f[1], f[2]))
                    .or_insert_with(|| random(f[1], f[2]));
                let actual = match f[3] {
                    "int" => source.next_int().to_string(),
                    "long" => source.next_long().to_string(),
                    "bool" => source.next_boolean().to_string(),
                    "float" => format!("{:08x}", source.next_float().to_bits()),
                    "double" => format!("{:016x}", source.next_double().to_bits()),
                    "bound" => source
                        .next_int_bounded(f[4].parse().unwrap())
                        .unwrap()
                        .to_string(),
                    "consume" => {
                        source.consume_count(f[4].parse().unwrap()).unwrap();
                        source.next_long().to_string()
                    }
                    "fork" => source.fork().next_long().to_string(),
                    "forkpos" => source
                        .fork_positional()
                        .at(BlockPos::ORIGIN)
                        .next_long()
                        .to_string(),
                    "reseed" => {
                        source.set_seed(WorldSeed::new(f[4].parse().unwrap()));
                        source.next_long().to_string()
                    }
                    _ => panic!("unknown random operation"),
                };
                assert_eq!(actual, f[5], "original Java random fixture line {index}");
            }
            "P" => {
                let factory = random(f[1], f[2]).fork_positional();
                let mut derived = match f[3] {
                    "at" => {
                        let p: Vec<i32> =
                            f[4].split(',').map(|part| part.parse().unwrap()).collect();
                        factory.at(BlockPos::new(p[0], p[1], p[2]))
                    }
                    "hash" => factory.from_hash_of(std::str::from_utf8(&hex_bytes(f[4])).unwrap()),
                    "seed" => factory.from_seed(WorldSeed::new(f[4].parse().unwrap())),
                    _ => panic!("unknown positional operation"),
                };
                assert_eq!(
                    derived.next_long(),
                    f[5].parse::<i64>().unwrap(),
                    "positional line {index}"
                );
                assert_bits(derived.next_double(), f[6], index);
            }
            _ => continue,
        }
        records += 1;
    }
    assert_eq!(records, 6488);
}

#[test]
fn original_java_improved_noise_permutations_samples_and_analytic_derivatives() {
    let mut improved = BTreeMap::new();
    let mut records = 0;
    for (index, line) in GOLDENS.lines().enumerate() {
        let f: Vec<_> = line.split('|').collect();
        match f[0] {
            "I" => {
                let mut source = random(f[2], f[3]);
                let noise = ImprovedNoise::new(&mut source);
                assert_configuration(&noise, f[4], f[5], index);
                assert_eq!(
                    source.next_long(),
                    f[6].parse::<i64>().unwrap(),
                    "improved constructor consumption line {index}"
                );
                improved.insert(f[1], noise);
            }
            "N" => assert_bits(
                improved[f[1]]
                    .sample_with_y(position(&f[2..5]), float(f[5]), float(f[6]))
                    .unwrap(),
                f[7],
                index,
            ),
            "D" => {
                let sample = improved[f[1]]
                    .sample_with_derivative(position(&f[2..5]))
                    .unwrap();
                assert_bits(sample.value(), f[5], index);
                let derivative = sample.derivative();
                for (value, expected) in [derivative.x, derivative.y, derivative.z]
                    .into_iter()
                    .zip(&f[6..9])
                {
                    assert_bits(value, expected, index);
                }
            }
            _ => continue,
        }
        records += 1;
    }
    assert_eq!(records, 976);
}

#[test]
fn original_java_perlin_modern_legacy_gaps_wrap_and_fixed_y() {
    let mut perlin = BTreeMap::new();
    let mut records = 0;
    for (index, line) in GOLDENS.lines().enumerate() {
        let f: Vec<_> = line.split('|').collect();
        match f[0] {
            "C" => {
                let mut source = random(f[2], f[3]);
                let amplitudes: Vec<_> = f[6].split(',').map(float).collect();
                let first = f[5].parse().unwrap();
                let noise = if f[4] == "true" {
                    PerlinNoise::legacy(&mut source, first, &amplitudes)
                } else {
                    PerlinNoise::new(&mut source, first, &amplitudes)
                }
                .unwrap();
                assert_eq!(noise.first_octave(), first);
                assert_eq!(
                    noise.amplitudes().map(f64::to_bits).collect::<Vec<_>>(),
                    amplitudes.into_iter().map(f64::to_bits).collect::<Vec<_>>()
                );
                assert_bits(noise.max_value(), f[7], index);
                assert_eq!(
                    source.next_long(),
                    f[8].parse::<i64>().unwrap(),
                    "Perlin constructor consumption line {index}"
                );
                assert!(noise.octave_noise(usize::MAX).is_none());
                perlin.insert(f[1], noise);
            }
            "O" => {
                let octave = perlin[f[1]].octave_noise(f[2].parse().unwrap());
                if f[3] == "null" {
                    assert!(octave.is_none(), "octave gap line {index}");
                } else {
                    assert_configuration(octave.unwrap(), f[3], f[4], index);
                }
            }
            "B" => assert_bits(
                perlin[f[1]].max_broken_value(float(f[2])).unwrap(),
                f[3],
                index,
            ),
            "V" => assert_bits(
                perlin[f[1]]
                    .sample_with_y(position(&f[2..5]), float(f[5]), float(f[6]), f[7] == "true")
                    .unwrap(),
                f[8],
                index,
            ),
            "W" => assert_bits(wrap_noise_coordinate(float(f[1])).unwrap(), f[2], index),
            _ => continue,
        }
        records += 1;
    }
    assert_eq!(records, 9092);
}

#[test]
fn invalid_random_requests_leave_the_stream_unchanged() {
    for algorithm in [
        RandomAlgorithm::Legacy,
        RandomAlgorithm::Xoroshiro128PlusPlus,
    ] {
        let mut source = WorldgenRandomSource::new(algorithm, WorldSeed::new(i64::MIN));
        for bound in [0, -1, i32::MIN] {
            let before = source;
            assert_eq!(
                source.next_int_bounded(bound),
                Err(RandomError::InvalidBound(bound))
            );
            assert_eq!(source, before);
        }
        let before = source;
        assert_eq!(
            source.consume_count(MAX_RANDOM_CONSUME_COUNT + 1),
            Err(RandomError::ConsumeCountTooLarge(
                MAX_RANDOM_CONSUME_COUNT + 1
            ))
        );
        assert_eq!(source, before);
        source.consume_count(0).unwrap();
        assert_eq!(source, before);
        // This bound probes the exact accepted CPU cap using direct core draws.
        let mut reference = before;
        for _ in 0..MAX_RANDOM_CONSUME_COUNT {
            let _ = reference.next_int();
        }
        source.consume_count(MAX_RANDOM_CONSUME_COUNT).unwrap();
        assert_eq!(source, reference);
    }
}

#[test]
fn invalid_noise_configuration_fails_before_random_consumption() {
    let mut source = WorldgenRandomSource::xoroshiro(WorldSeed::new(-1));
    for (first, amplitudes, expected) in [
        (0, vec![], NoiseError::OctaveCount(0)),
        (
            0,
            vec![0.0; MAX_NOISE_OCTAVES + 1],
            NoiseError::OctaveCount(MAX_NOISE_OCTAVES + 1),
        ),
        (i32::MIN, vec![1.0], NoiseError::OctaveOutOfRange(i32::MIN)),
        (i32::MAX, vec![1.0], NoiseError::OctaveOutOfRange(i32::MAX)),
        (-65, vec![1.0], NoiseError::OctaveOutOfRange(-65)),
        (32, vec![1.0, 1.0], NoiseError::OctaveOutOfRange(33)),
        (0, vec![f64::NAN], NoiseError::NonFiniteAmplitude(0)),
        (0, vec![f64::INFINITY], NoiseError::NonFiniteAmplitude(0)),
        (
            -1,
            vec![0.0, f64::NEG_INFINITY],
            NoiseError::NonFiniteAmplitude(1),
        ),
        (0, vec![f64::MAX], NoiseError::NonFiniteResult),
    ] {
        let before = source;
        assert_eq!(
            PerlinNoise::new(&mut source, first, &amplitudes).unwrap_err(),
            expected
        );
        assert_eq!(source, before);
    }
    for (first, amplitudes) in [(1, vec![0.0]), (-1, vec![0.0; 3])] {
        let before = source;
        assert_eq!(
            PerlinNoise::legacy(&mut source, first, &amplitudes).unwrap_err(),
            NoiseError::PositiveLegacyOctave
        );
        assert_eq!(source, before);
    }
}

#[test]
fn finite_input_boundaries_and_pure_sample_order() {
    let mut source = WorldgenRandomSource::legacy(WorldSeed::new(42));
    let improved = ImprovedNoise::new(&mut source);
    let perlin = PerlinNoise::new(&mut source, -3, &[1.0, 0.0, 0.5, 1.0]).unwrap();
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        MAX_NOISE_COORDINATE + 1.0,
        -MAX_NOISE_COORDINATE - 1.0,
    ] {
        for position in [
            Vec3::new(bad, 0.0, 0.0),
            Vec3::new(0.0, bad, 0.0),
            Vec3::new(0.0, 0.0, bad),
        ] {
            assert_eq!(improved.sample(position), Err(NoiseError::InvalidInput));
            assert_eq!(
                improved.sample_with_derivative(position),
                Err(NoiseError::InvalidInput)
            );
            assert_eq!(perlin.sample(position), Err(NoiseError::InvalidInput));
        }
        assert_eq!(
            improved.sample_with_y(Vec3::ZERO, bad, 0.0),
            Err(NoiseError::InvalidInput)
        );
        assert_eq!(
            improved.sample_with_y(Vec3::ZERO, 0.0, bad),
            Err(NoiseError::InvalidInput)
        );
        assert_eq!(
            perlin.sample_with_y(Vec3::ZERO, bad, 0.0, false),
            Err(NoiseError::InvalidInput)
        );
        assert_eq!(wrap_noise_coordinate(bad), Err(NoiseError::InvalidInput));
    }
    let positions = [
        Vec3::ZERO,
        Vec3::new(-1.0, -64.0, -16.0),
        Vec3::new(MAX_NOISE_COORDINATE, -MAX_NOISE_COORDINATE, 0.5),
    ];
    let values = positions.map(|position| {
        (
            improved.sample(position).unwrap().to_bits(),
            perlin.sample(position).unwrap().to_bits(),
        )
    });
    for index in [2, 0, 1, 0, 2, 1] {
        assert_eq!(
            (
                improved.sample(positions[index]).unwrap().to_bits(),
                perlin.sample(positions[index]).unwrap().to_bits()
            ),
            values[index]
        );
    }
}

#[test]
fn immutable_factories_do_not_depend_on_coordinate_or_name_request_order() {
    for algorithm in [
        RandomAlgorithm::Legacy,
        RandomAlgorithm::Xoroshiro128PlusPlus,
    ] {
        let mut source = WorldgenRandomSource::new(algorithm, WorldSeed::new(i64::MAX));
        let factory = source.fork_positional();
        let positions = [
            BlockPos::new(i32::MIN, i32::MAX, -1),
            BlockPos::ORIGIN,
            BlockPos::new(30_000_000, -64, -30_000_000),
        ];
        let expected = positions.map(|position| factory.at(position).next_long());
        let names = ["octave_-64", "minecraft:temperature", "💎\0"];
        let expected_names = names.map(|name| factory.from_hash_of(name).next_long());
        for index in [2, 0, 1, 2, 1, 0] {
            assert_eq!(factory.at(positions[index]).next_long(), expected[index]);
            assert_eq!(
                factory.from_hash_of(names[index]).next_long(),
                expected_names[index]
            );
        }
    }
}
