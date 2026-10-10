//! Independent raw fixtures and boundary probes; these are not codec self-roundtrips.

use ferrumc_core::{DimensionId, EntityUuid, GameMode, WorldId};
use ferrumc_world::WorldSeed;

use super::*;

pub(crate) fn config() -> OwnershipConfig {
    config_inflight(8)
}
pub(crate) fn config_inflight(inflight: u16) -> OwnershipConfig {
    OwnershipConfig::new(
        OwnershipCapacity::new(
            NonZeroUsize::new(MAX_OWNERSHIP_REQUEST_BYTES).unwrap(),
            NonZeroU64::new(1000).unwrap(),
            NonZeroU64::new(100).unwrap(),
        )
        .unwrap(),
        OwnershipOpenLimits::new(
            NonZeroU64::new(1_000_000).unwrap(),
            NonZeroU64::new(128 * 1024 * 1024).unwrap(),
            NonZeroU64::new(8 * 1024 * 1024).unwrap(),
            NonZeroU16::new(inflight).unwrap(),
        )
        .unwrap(),
    )
}
pub(crate) async fn wait_for_permit(runtime: &Runtime) {
    let _permit = runtime.permits.clone().acquire_owned().await.unwrap();
}
pub(crate) fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(4), DimensionId::new(2))
}
pub(crate) fn identity() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(37))
}
pub(crate) fn player() -> ScopedPlayerKey {
    ScopedPlayerKey::new(scope(), PlayerId::offline("OwnershipFixture"))
}
pub(crate) fn entity() -> PersistentEntityKey {
    PersistentEntityKey::new(
        scope().world(),
        scope().dimension(),
        EntityUuid::from_bytes([0x12; 16]),
    )
}
pub(crate) fn id(n: u8) -> OwnershipTransferId {
    OwnershipTransferId::from_bytes([n; 16])
}
pub(crate) fn p(body: u8) -> PlayerRecord {
    PlayerRecord::new(
        crate::SchemaVersion::new(19),
        GameMode::Survival,
        vec![body, 0xEF],
    )
    .unwrap()
}
pub(crate) fn e(x: i32, body: u8) -> PersistentEntityRecord {
    PersistentEntityRecord::new(
        ChunkPos::new(x, -2),
        crate::EntityRecord::new(crate::SchemaVersion::new(7), vec![body, 0xBA]).unwrap(),
    )
    .unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn fixed_identity() -> [u8; 27] {
    hex("464347490001000100150008000100000001000000000000002500")
        .try_into()
        .unwrap()
}
fn receipt_fixture() -> [u8; 256] {
    let mut b = [0; 256];
    b[..6].copy_from_slice(b"FCTR\0\x01");
    b[6] = 3;
    b[7..23].fill(0x34);
    b[23..55].copy_from_slice(&hex(
        "a02a8e2cf94ab227e060f14a3adc1e6b86671563589ac15213160127f6f35552",
    ));
    b[55..59].copy_from_slice(&4u32.to_be_bytes());
    b[59..63].copy_from_slice(&2u32.to_be_bytes());
    b[63..90].copy_from_slice(&fixed_identity());
    b[106..122].fill(0x12);
    b[149] = 1;
    b[157] = 1;
    b[158] = 1;
    b[159..163].copy_from_slice(&4u32.to_be_bytes());
    b[163..167].copy_from_slice(&2u32.to_be_bytes());
    b[167..171].copy_from_slice(&1i32.to_be_bytes());
    b[171..175].copy_from_slice(&(-2i32).to_be_bytes());
    b[190] = 1;
    b[232] = 1;
    b[233..237].copy_from_slice(&1i32.to_be_bytes());
    b[237..241].copy_from_slice(&(-2i32).to_be_bytes());
    b
}

#[test]
fn request_sha256_matches_independent_python_struct_fixture() {
    assert_eq!(identity().encode(), fixed_identity());
    let owner = Arc::new(());
    let expected = OwnedEntityExpectation {
        key: entity(),
        identity: identity(),
        stamp: OwnershipStamp::FRESH,
        owner: Arc::downgrade(&owner),
    };
    let request = EntityOwnershipCasRequest::new(
        id(0x34),
        expected,
        EntityOwnershipProposal::put(e(1, 0x11)),
    )
    .unwrap();
    assert_eq!(
        request.receipt_query().identity().to_bytes().as_slice(),
        hex("a02a8e2cf94ab227e060f14a3adc1e6b86671563589ac15213160127f6f35552")
    );
    assert_eq!(request.0.len().unwrap(), 173);
    let alternate_owner = Arc::new(());
    let alternate = EntityOwnershipCasRequest::new(
        id(0x34),
        OwnedEntityExpectation {
            owner: Arc::downgrade(&alternate_owner),
            ..request.0.entity.as_ref().unwrap().0.clone()
        },
        EntityOwnershipProposal::put(e(1, 0x11)),
    )
    .unwrap();
    assert_eq!(request.receipt_query(), alternate.receipt_query());
}
#[test]
fn independent_receipt_and_header_offsets_are_exact() {
    let b = receipt_fixture();
    let receipt = raw::receipt(&[0x34; 16], &b).unwrap();
    assert_eq!(receipt.kind, 3);
    assert_eq!(receipt.scope, scope());
    assert!(receipt.player().is_none());
    assert_eq!(receipt.entity().unwrap().0, entity().uuid());
    assert_eq!(receipt.entity().unwrap().2.revision().unwrap().get(), 1);
    assert_eq!(receipt.after_entity_chunk(), Some(ChunkPos::new(1, -2)));
    assert_eq!(receipt.epochs()[0].after(), 1);
    assert_eq!(raw::encode_receipt(&receipt), b);
    let mut h = [0; 39];
    h[..6].copy_from_slice(b"FCOP\0\x01");
    h[6] = 2;
    h[7..15].copy_from_slice(&[0, 0, 0, 4, 0, 0, 0, 2]);
    h[15..23].copy_from_slice(&0x0102_0304_0506_0708u64.to_be_bytes());
    h[23..].fill(0x34);
    let header = raw::player_header(&h).unwrap();
    assert!(header.stamp.is_tombstone());
    assert_eq!(header.scope, scope());
    assert_eq!(
        header.stamp.revision().unwrap().get(),
        0x0102_0304_0506_0708
    );
    assert_eq!(raw::encode_player_header(header), h);
    let mut e = [0; 35];
    e[..6].copy_from_slice(b"FCOR\0\x01");
    e[6] = 1;
    e[7..11].copy_from_slice(&2u32.to_be_bytes());
    e[18] = 1;
    e[19..].fill(0x34);
    let parsed = raw::entity_header(&raw::entity_key(entity()), &e).unwrap();
    assert_eq!(parsed.scope, scope());
    assert_eq!(raw::encode_entity_header(parsed), e);
}
#[test]
fn fixed_parsers_refuse_short_long_unknown_and_inactive_garbage() {
    let receipt = receipt_fixture();
    for len in 0..256 {
        assert!(
            raw::receipt(&[0x34; 16], &receipt[..len]).is_err(),
            "receipt length{len}"
        );
    }
    let mut long = receipt.to_vec();
    long.push(0);
    assert!(raw::receipt(&[0x34; 16], &long).is_err());
    for len in [0, 15, 17, 20] {
        assert!(raw::receipt(&vec![0x34; len], &receipt).is_err());
    }
    for (offset, value) in [
        (0, 0),
        (5, 2),
        (6, 0),
        (6, 4),
        (7, 0),
        (90, 1),
        (122, 3),
        (149, 0),
        (157, 0),
        (158, 3),
        (191, 1),
        (223, 1),
        (224, 1),
        (232, 2),
        (241, 1),
        (255, 1),
    ] {
        let mut corrupt = receipt;
        corrupt[offset] = value;
        assert!(
            raw::receipt(&[0x34; 16], &corrupt).is_err(),
            "receipt offset{offset} value{value}"
        );
    }
    let header = raw::Header {
        scope: scope(),
        stamp: OwnershipStamp::checked(1, 1).unwrap(),
        receipt: id(1),
    };
    let player = raw::encode_player_header(header);
    let entity = raw::encode_entity_header(header);
    for len in 0..39 {
        assert!(raw::player_header(&player[..len]).is_err());
    }
    for len in 0..35 {
        assert!(raw::entity_header(&raw::entity_key(self::entity()), &entity[..len]).is_err());
    }
    for (offset, value) in [(0, 0), (5, 2), (6, 0), (6, 3), (22, 0)] {
        let mut b = player;
        b[offset] = value;
        assert!(raw::player_header(&b).is_err());
    }
    for (offset, value) in [(0, 0), (5, 2), (6, 0), (6, 3), (18, 0)] {
        let mut b = entity;
        b[offset] = value;
        assert!(raw::entity_header(&raw::entity_key(self::entity()), &b).is_err());
    }
    for len in [0, 1, 8, 19, 21] {
        assert!(raw::entity_header(&vec![0; len], &entity).is_err());
    }
    for len in 0..9 {
        assert!(OwnershipStamp::decode(&vec![0; len]).is_err());
    }
    assert!(OwnershipStamp::decode(&[0; 10]).is_err());
    for (tag, revision) in [(0, 1), (1, 0), (2, 0), (3, 0), (255, u64::MAX)] {
        let mut b = [0; 9];
        b[0] = tag;
        b[1..].copy_from_slice(&revision.to_be_bytes());
        assert!(OwnershipStamp::decode(&b).is_err());
    }
}
#[test]
fn receipt_parser_checks_epoch_union_and_positive_nonwrapping_history() {
    let b = receipt_fixture();
    for mutate in 0..8 {
        let mut c = b;
        match mutate {
            0 => c[157] = 2,
            1 => c[158] = 0,
            2 => c[163] = 1,
            3 => c[183..191].fill(0),
            4 => c[175..183].fill(255),
            5 => {
                c[232] = 0;
                c[233..241].fill(0);
            }
            6 => c[233..237].copy_from_slice(&i32::MAX.to_be_bytes()),
            7 => {
                c[158] = 2;
                let entry: Vec<_> = c[159..191].to_vec();
                c[191..223].copy_from_slice(&entry);
            }
            _ => unreachable!(),
        }
        assert!(
            raw::receipt(&[0x34; 16], &c).is_err(),
            "epoch mutation{mutate}"
        );
    }
}
