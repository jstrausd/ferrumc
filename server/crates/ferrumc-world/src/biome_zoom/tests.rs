//! Complete actual-original values and seed/domain/type boundary contracts.

use super::*;

const REQUESTS: &str = include_str!("../../tests/fixtures/biome-zoom-original-1.21.8/requests.tsv");
const ORIGINAL: &str = include_str!("../../tests/fixtures/biome-zoom-original-1.21.8/original.tsv");

fn integer(value: &str) -> i64 {
    value.parse().unwrap()
}

fn double(value: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(value, 16).unwrap())
}

fn hash_case(request: &[&str], original: &[&str]) {
    let world = WorldSeed::new(integer(request[2]));
    let hashed = BiomeZoomSeed::from_world_seed(world);
    assert_eq!(hashed.bits(), integer(original[2]));
    assert_eq!(BiomeZoomSeed::from_obfuscated_bits(hashed.bits()), hashed);
}

fn select_case(request: &[&str], original: &[&str], semantics: BiomeZoomSemantics) {
    let bits = integer(request[3]);
    let seed = match request[2] {
        "world" => BiomeZoomSeed::from_world_seed(WorldSeed::new(bits)),
        "zoom" => BiomeZoomSeed::from_obfuscated_bits(bits),
        kind => panic!("unsupported fixture seed kind {kind}"),
    };
    let pos = BlockPos::new(
        request[4].parse().unwrap(),
        request[5].parse().unwrap(),
        request[6].parse().unwrap(),
    );
    let producer = BiomeZoom::new(seed, semantics);
    let selected = producer.select_quart(pos);
    assert_eq!(seed.bits(), integer(original[2]));
    assert_eq!(producer.seed(), seed);
    assert_eq!(producer.semantics(), semantics);
    assert_eq!(
        [selected.x(), selected.y(), selected.z()],
        [
            original[3].parse::<i32>().unwrap(),
            original[4].parse::<i32>().unwrap(),
            original[5].parse::<i32>().unwrap(),
        ],
        "{} {semantics:?}",
        request[1],
    );
    assert_eq!(producer.select_quart(pos), selected);
    assert_eq!(BiomeZoom::new(seed, semantics).select_quart(pos), selected);
}

fn distance_case(request: &[&str], original: &[&str], semantics: BiomeZoomSemantics) {
    let quart = [
        request[3].parse().unwrap(),
        request[4].parse().unwrap(),
        request[5].parse().unwrap(),
    ];
    let offset = [double(request[6]), double(request[7]), double(request[8])];
    let actual = distance(integer(request[2]), quart, offset, semantics);
    assert_eq!(
        actual.to_bits(),
        double(original[2]).to_bits(),
        "{} {semantics:?}",
        request[1],
    );
}

#[test]
fn every_actual_original_hash_bucket_mixer_distance_and_selection_matches() {
    for semantics in [
        BiomeZoomSemantics::Vanilla1_21_8,
        BiomeZoomSemantics::Paper1_21_8Build60,
    ] {
        let mut counts = [0usize; 5];
        assert_eq!(REQUESTS.lines().count(), ORIGINAL.lines().count());
        for (request, original) in REQUESTS.lines().zip(ORIGINAL.lines()) {
            let request: Vec<_> = request.split('\t').collect();
            let original: Vec<_> = original.split('\t').collect();
            assert_eq!(request[..2], original[..2]);
            match request[0] {
                "H" => {
                    hash_case(&request, &original);
                    counts[0] += 1;
                }
                "F" => {
                    assert_eq!(
                        fiddle(integer(request[2]), semantics).to_bits(),
                        double(original[2]).to_bits(),
                        "{} {semantics:?}",
                        request[1],
                    );
                    counts[1] += 1;
                }
                "L" => {
                    assert_eq!(
                        next(integer(request[2]), integer(request[3])),
                        integer(original[2]),
                        "{}",
                        request[1],
                    );
                    counts[2] += 1;
                }
                "G" => {
                    select_case(&request, &original, semantics);
                    counts[3] += 1;
                }
                "D" => {
                    distance_case(&request, &original, semantics);
                    counts[4] += 1;
                }
                kind => panic!("unsupported genuine fixture kind {kind}"),
            }
        }
        assert_eq!(counts, [11, 2048, 49, 6815, 54_520]);
    }
}

#[test]
fn actual_original_equal_minimum_retains_corner_two_before_corner_four() {
    // Actual getBiome and separately reflected genuine distance outputs verify
    // this full-domain case in both original artifacts, not an injected source.
    for semantics in [
        BiomeZoomSemantics::Vanilla1_21_8,
        BiomeZoomSemantics::Paper1_21_8Build60,
    ] {
        let pos = BlockPos::new(134_217_733, -134_217_723, 134_217_733);
        for bits in [
            268_435_456,
            268_435_456 + (1 << 34),
            268_435_456 - (1 << 34),
            i64::MIN + 268_435_456,
        ] {
            let zoom = BiomeZoom::new(BiomeZoomSeed::from_obfuscated_bits(bits), semantics);
            let selected = zoom.select_quart(pos);
            assert_eq!(
                [selected.x(), selected.y(), selected.z()],
                [33_554_432, -33_554_431, 33_554_432],
            );
            let two = distance(
                bits,
                [33_554_432, -33_554_431, 33_554_432],
                [0.75, -0.25, 0.75],
                semantics,
            );
            let four = distance(
                bits,
                [33_554_433, -33_554_432, 33_554_432],
                [-0.25, 0.75, 0.75],
                semantics,
            );
            assert_eq!(two.to_bits(), four.to_bits());
        }
    }
}

#[test]
fn local_equal_minima_and_adjacent_finite_values_keep_original_strict_comparison() {
    // This is a local comparison control, distinct from the genuine tie above.
    let equal = 1.0f64;
    assert!(!prefer_candidate(equal, equal));
    assert!(!prefer_candidate(0.0, -0.0));
    assert!(prefer_candidate(equal, f64::from_bits(equal.to_bits() - 1)));
    assert!(!prefer_candidate(
        equal,
        f64::from_bits(equal.to_bits() + 1)
    ));
}

#[test]
fn seed_types_preserve_obfuscated_bits_and_never_implicitly_rehash() {
    for bits in [0, 1, -1, i64::MIN, i64::MAX] {
        let original = BiomeZoomSeed::from_obfuscated_bits(bits);
        let hashed = BiomeZoomSeed::from_world_seed(WorldSeed::new(bits));
        assert_eq!(original.bits(), bits);
        assert_ne!(original, hashed);
        assert_eq!(BiomeZoomSeed::from_obfuscated_bits(hashed.bits()), hashed);
    }
}

#[test]
fn all_signed_block_axis_boundaries_remain_independent_of_density_admission() {
    let producer = BiomeZoom::new(
        BiomeZoomSeed::from_obfuscated_bits(i64::MIN),
        BiomeZoomSemantics::Paper1_21_8Build60,
    );
    for x in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MAX - 1, i32::MAX] {
        for y in [i32::MIN, i32::MAX] {
            for z in [i32::MIN, i32::MAX] {
                let selected = producer.select_quart(BlockPos::new(x, y, z));
                for axis in [selected.x(), selected.y(), selected.z()] {
                    assert!((-536_870_912..=536_870_912).contains(&axis));
                }
                assert!(crate::density::ClimateQuartPos::new(
                    selected.x(),
                    selected.y(),
                    selected.z()
                )
                .is_err());
            }
        }
    }
}

#[test]
fn fixed_native_values_have_no_owned_heap_capacity() {
    assert_eq!(std::mem::size_of::<BiomeZoomSeed>(), 8);
    assert_eq!(std::mem::size_of::<BiomeQuartPos>(), 12);
    assert_eq!(std::mem::size_of::<BiomeZoomSemantics>(), 1);
    assert_eq!(std::mem::size_of::<BiomeZoom>(), 16);
}
