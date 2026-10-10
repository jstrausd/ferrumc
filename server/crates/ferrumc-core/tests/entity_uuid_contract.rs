//! Public durable identity and parser contracts, independent of entity owners.

use std::collections::{BTreeSet, HashSet};

use ferrumc_core::EntityUuid;
use uuid::Uuid;

const ASYMMETRIC_BYTES: [u8; 16] = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10,
];
const ASYMMETRIC_TEXT: &str = "01234567-89ab-cdef-fedc-ba9876543210";
const ASYMMETRIC: EntityUuid = EntityUuid::from_bytes(ASYMMETRIC_BYTES);
const ASYMMETRIC_UUID: Uuid = ASYMMETRIC.as_uuid();
const REWRAPPED: EntityUuid = EntityUuid::from_uuid(ASYMMETRIC_UUID);
const EXTRACTED: [u8; 16] = REWRAPPED.to_bytes();

#[test]
fn canonical_bytes_text_and_explicit_uuid_conversions_preserve_all_bits() {
    let goldens = [
        (ASYMMETRIC_BYTES, ASYMMETRIC_TEXT),
        ([0; 16], "00000000-0000-0000-0000-000000000000"),
        ([255; 16], "ffffffff-ffff-ffff-ffff-ffffffffffff"),
        (
            [0x80, 0, 0, 0, 0, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0, 1],
            "80000000-0000-0000-8000-000000000001",
        ),
        (
            [
                0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 1,
            ],
            "7fffffff-ffff-ffff-0000-000000000001",
        ),
    ];
    assert_eq!(EXTRACTED, ASYMMETRIC_BYTES);
    assert_eq!(REWRAPPED, ASYMMETRIC);
    for (bytes, text) in goldens {
        let identity = EntityUuid::from_bytes(bytes);
        assert_eq!(identity.to_bytes(), bytes);
        assert_eq!(identity.to_string(), text);
        assert_eq!(text.parse::<EntityUuid>().unwrap(), identity);
        let raw: Uuid = identity.into();
        assert_eq!(raw.into_bytes(), bytes);
        assert_eq!(EntityUuid::from_uuid(raw), identity);
        assert_eq!(EntityUuid::from(raw), identity);
        assert_eq!(identity.as_uuid(), raw);
    }
}

#[test]
fn parsing_accepts_the_existing_uuid_spellings_and_canonicalizes_only_text() {
    for text in [
        ASYMMETRIC_TEXT,
        "0123456789abcdeffedcba9876543210",
        "01234567-89AB-CDEF-FEDC-BA9876543210",
        "{01234567-89ab-cdef-fedc-ba9876543210}",
        "urn:uuid:01234567-89ab-cdef-fedc-ba9876543210",
    ] {
        let identity: EntityUuid = text.parse().unwrap();
        assert_eq!(identity, ASYMMETRIC);
        assert_eq!(identity.to_string(), ASYMMETRIC_TEXT);
        assert_eq!(identity.as_uuid(), Uuid::parse_str(text).unwrap());
    }
}

#[test]
fn malformed_parse_reports_the_underlying_uuid_error_without_repair() {
    for text in [
        "",
        "not-a-uuid",
        "01234567-89ab-cdef-fedc-ba987654321",
        "01234567-89ab-cdef-fedc-ba98765432100",
        "01234567_89ab-cdef-fedc-ba9876543210",
        "01234567-89ab-cdef-fedc-ba987654321g",
        "01234567-89ab-cdef-fedc-ba987654321é",
        "01234567-89ab-cdef-fedc-ba9876543210\0",
        " 01234567-89ab-cdef-fedc-ba9876543210",
        "01234567-89ab-cdef-fedc-ba9876543210 ",
        "01234567-89ab-cdef-fedc-ba9876543210\n",
        "{01234567-89ab-cdef-fedc-ba9876543210",
        "URN:UUID:01234567-89ab-cdef-fedc-ba9876543210",
        "1-1-1-1-1",
    ] {
        let error = text.parse::<EntityUuid>().unwrap_err();
        let expected = Uuid::parse_str(text).unwrap_err();
        assert_eq!(error.to_string(), expected.to_string(), "{text:?}");
    }
    let oversized = "0".repeat(65_536);
    assert!(oversized.parse::<EntityUuid>().is_err());
}

#[test]
fn equality_hash_and_unsigned_canonical_order_keep_distinct_identities() {
    let nil = EntityUuid::from_bytes([0; 16]);
    let high_bit: EntityUuid = "80000000-0000-0000-8000-000000000001".parse().unwrap();
    let low_bit: EntityUuid = "7fffffff-ffff-ffff-0000-000000000001".parse().unwrap();
    let max = EntityUuid::from_bytes([255; 16]);
    let ordered: BTreeSet<_> = [max, high_bit, ASYMMETRIC, nil, low_bit]
        .into_iter()
        .collect();
    assert_eq!(
        ordered.into_iter().collect::<Vec<_>>(),
        [nil, ASYMMETRIC, low_bit, high_bit, max]
    );
    let mut identities = HashSet::from([ASYMMETRIC, nil, high_bit]);
    let duplicate = ASYMMETRIC;
    assert!(!identities.insert(duplicate));
    assert_eq!(identities.len(), 3);
    let mut changed = ASYMMETRIC_BYTES;
    changed[15] ^= 1;
    assert!(identities.insert(EntityUuid::from_bytes(changed)));
    assert_eq!(identities.len(), 4);
    assert_eq!(ASYMMETRIC.to_bytes(), ASYMMETRIC_BYTES);
}

#[cfg(feature = "serde")]
#[test]
fn serde_json_transparently_preserves_supplied_uuid_bits() {
    for identity in [
        ASYMMETRIC,
        EntityUuid::from_bytes([0; 16]),
        EntityUuid::from_bytes([255; 16]),
    ] {
        let json = serde_json::to_string(&identity).unwrap();
        assert_eq!(json, format!("\"{identity}\""));
        assert_eq!(json, serde_json::to_string(&identity.as_uuid()).unwrap());
        assert_eq!(serde_json::from_str::<EntityUuid>(&json).unwrap(), identity);
    }
    let json = "\"{01234567-89AB-CDEF-FEDC-BA9876543210}\"";
    assert_eq!(
        serde_json::from_str::<EntityUuid>(json).unwrap(),
        ASYMMETRIC
    );
}

#[cfg(feature = "serde")]
#[test]
fn malformed_serde_json_is_refused_without_defaulting_to_nil() {
    for json in [
        "null",
        "false",
        "42",
        "[]",
        "{}",
        "\"\"",
        "\"not-a-uuid\"",
        "\"01234567-89ab-cdef-fedc-ba987654321\"",
        "\"01234567-89ab-cdef-fedc-ba98765432100\"",
        "\"01234567-89ab-cdef-fedc-ba987654321g\"",
        "\"01234567-89ab-cdef-fedc-ba9876543210\" true",
        "\"01234567-89ab-cdef-fedc-ba9876543210",
    ] {
        assert!(serde_json::from_str::<EntityUuid>(json).is_err(), "{json}");
    }
}
