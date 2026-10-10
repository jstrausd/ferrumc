//! Independent original-Paper/Prismarine packet bytes and the opaque-tail policy.
use bytes::BytesMut;
use ferrumc_codec::{BoundedBytes, BoundedReader, CodecError};
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, SetEntityData, SetEntityMotion, TakeItemEntity,
};
use ferrumc_proto::ProtoError;

const CAP: usize = 263_168;
const PAPER: &str = include_str!("fixtures/item_entity_1_21_8/paper.tsv");
const PRISMARINE: &str = include_str!("fixtures/item_entity_1_21_8/prismarine.tsv");

fn unhex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "whole fixture bytes");
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII fixture"), 16)
                .expect("fixture hex")
        })
        .collect()
}

fn fixture_packet(fields: &[&str]) -> ClientboundPlayPacket {
    let entity = fields[2].parse().expect("entity id");
    match fields[1] {
        "metadata" => ClientboundPlayPacket::SetEntityData(SetEntityData::new(
            entity,
            BoundedBytes::new(unhex(fields[6])).expect("bounded metadata fixture"),
        )),
        "motion" => ClientboundPlayPacket::SetEntityMotion(SetEntityMotion::new(
            entity,
            fields[3].parse().expect("x i16"),
            fields[4].parse().expect("y i16"),
            fields[5].parse().expect("z i16"),
        )),
        "take" => ClientboundPlayPacket::TakeItemEntity(TakeItemEntity::new(
            entity,
            fields[3].parse().expect("collector id"),
            fields[4].parse().expect("original pickup count"),
        )),
        kind => panic!("unexpected fixture kind: {kind}"),
    }
}

fn assert_direct_api(packet: &ClientboundPlayPacket, fields: &[&str], bytes: &[u8]) {
    let mut reader = BoundedReader::new(&bytes[1..]);
    let mut encoded = BytesMut::from(&b"prefix"[..]);
    match packet {
        ClientboundPlayPacket::SetEntityData(value) => {
            assert_eq!(value.entity_id().to_string(), fields[2]);
            assert_eq!(value.metadata_raw().as_slice(), unhex(fields[6]));
            assert_eq!(
                SetEntityData::decode(&mut reader).expect("data body"),
                *value
            );
            value.encode(&mut encoded).expect("data encode");
        }
        ClientboundPlayPacket::SetEntityMotion(value) => {
            assert_eq!(value.entity_id().to_string(), fields[2]);
            assert_eq!(value.velocity_x().to_string(), fields[3]);
            assert_eq!(value.velocity_y().to_string(), fields[4]);
            assert_eq!(value.velocity_z().to_string(), fields[5]);
            assert_eq!(
                SetEntityMotion::decode(&mut reader).expect("motion body"),
                *value
            );
            value.encode(&mut encoded).expect("motion encode");
        }
        ClientboundPlayPacket::TakeItemEntity(value) => {
            assert_eq!(value.item_entity_id().to_string(), fields[2]);
            assert_eq!(value.collector_entity_id().to_string(), fields[3]);
            assert_eq!(value.pickup_count().to_string(), fields[4]);
            assert_eq!(
                TakeItemEntity::decode(&mut reader).expect("take body"),
                *value
            );
            value.encode(&mut encoded).expect("take encode");
        }
        value => panic!("unexpected fixture packet: {value:?}"),
    }
    reader.finish().expect("complete direct body");
    assert_eq!(
        &encoded[..6],
        b"prefix",
        "encoding appends to existing bytes"
    );
    assert_eq!(&encoded[6..], bytes, "exact direct original bytes");
}

#[test]
fn independent_original_and_prismarine_bytes_match_apis_and_dispatch() {
    assert_eq!(PAPER, PRISMARINE, "independent declared semantic inputs");
    assert_eq!(SetEntityData::PACKET_ID, 0x5c);
    assert_eq!(SetEntityMotion::PACKET_ID, 0x5e);
    assert_eq!(TakeItemEntity::PACKET_ID, 0x75);
    let mut counts = [0; 3];
    for row in PAPER.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 8);
        let packet = fixture_packet(&fields);
        let bytes = unhex(fields[7]);
        assert_direct_api(&packet, &fields, &bytes);
        let mut encoded = BytesMut::new();
        packet.encode(&mut encoded).expect("dispatch encode");
        assert_eq!(&encoded[..], bytes, "{}", fields[0]);
        let mut reader = BoundedReader::new(&bytes);
        let id = reader.read_var_int().expect("original dispatch id");
        assert_eq!(packet.packet_id(), id);
        let decoded = ClientboundPlayPacket::decode(id, &mut reader).expect("original body");
        assert_eq!(decoded, packet, "{}", fields[0]);
        reader.finish().expect("complete independent fixture");
        let index = match fields[1] {
            "metadata" => 0,
            "motion" => 1,
            "take" => 2,
            kind => panic!("unknown fixture kind: {kind}"),
        };
        counts[index] += 1;
    }
    assert_eq!(counts, [14, 8, 8]);
}

#[test]
fn every_truncated_scalar_fixture_prefix_is_classified_as_eof() {
    for row in PAPER.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        if fields[1] == "metadata" {
            continue;
        }
        let bytes = unhex(fields[7]);
        let body = &bytes[1..];
        for len in 0..body.len() {
            let error = ClientboundPlayPacket::decode(
                i32::from(bytes[0]),
                &mut BoundedReader::new(&body[..len]),
            )
            .expect_err("truncated scalar field");
            assert!(
                matches!(error, ProtoError::Codec(CodecError::UnexpectedEof { .. })),
                "{} prefix {len}: {error:?}",
                fields[0]
            );
        }
    }
}

#[test]
fn opaque_tail_preserves_original_negative_controls_and_every_shortened_tail() {
    // The actual semantic original codec rejects the first four bodies. Our
    // opaque envelope deliberately admits all five, retaining complete tails.
    for hex in ["00", "00000001", "000063ff", "00080701", "00ffee"] {
        let bytes = unhex(hex);
        let mut reader = BoundedReader::new(&bytes);
        let value = SetEntityData::decode(&mut reader).expect("opaque original control");
        assert_eq!(value.entity_id(), 0);
        assert_eq!(value.metadata_raw().as_slice(), &bytes[1..]);
        reader
            .finish()
            .expect("opaque decoder consumes even after EOF");
        let mut encoded = BytesMut::new();
        value.encode(&mut encoded).expect("opaque encode");
        assert_eq!(&encoded[1..], bytes);
    }
    let fields: Vec<_> = PAPER
        .lines()
        .find(|row| row.starts_with("metadata_item_components\t"))
        .expect("real components with internal FF")
        .split('\t')
        .collect();
    let tail = unhex(fields[6]);
    assert!(tail[..tail.len() - 1].contains(&0xff));
    for len in 0..=tail.len() {
        let mut body = vec![0];
        body.extend_from_slice(&tail[..len]);
        let value = SetEntityData::decode(&mut BoundedReader::new(&body))
            .expect("semantic truncation is not envelope truncation");
        assert_eq!(value.metadata_raw().as_slice(), &tail[..len]);
    }
}

#[test]
fn raw_cap_is_unprefixed_and_refuses_n_plus_one_at_tail_entry() {
    let tail: Vec<_> = (0..CAP).map(|i| i.to_le_bytes()[0]).collect();
    let packet = SetEntityData::new(128, BoundedBytes::new(tail.clone()).expect("N admitted"));
    let mut encoded = BytesMut::new();
    packet.encode(&mut encoded).expect("N encode");
    assert_eq!(&encoded[..3], &[0x5c, 0x80, 0x01]);
    assert_eq!(&encoded[3..], tail, "no byte-length prefix");
    let mut reader = BoundedReader::new(&encoded[1..]);
    assert_eq!(
        SetEntityData::decode(&mut reader).expect("N decode"),
        packet
    );
    reader.finish().expect("N consumed");
    encoded.extend_from_slice(&[0xee]);
    let mut reader = BoundedReader::new(&encoded[1..]);
    let error = SetEntityData::decode(&mut reader).expect_err("N+1 refused");
    assert!(matches!(
        error,
        ProtoError::Codec(CodecError::BytesTooLong { length, max }) if length == CAP + 1 && max == CAP
    ));
    assert_eq!(reader.position(), 2, "entity id alone consumed");
    assert_eq!(
        reader.remaining(),
        CAP + 1,
        "oversized tail not read/copied"
    );
    assert_eq!(reader.read_u8().expect("first unconsumed byte"), tail[0]);
    assert_eq!(
        BoundedBytes::<CAP>::new(vec![0; CAP + 1]),
        Err(CodecError::BytesTooLong {
            length: CAP + 1,
            max: CAP
        })
    );
}

#[test]
fn malformed_varints_keep_existing_codec_policy_and_field_positions() {
    for id in [0x5c, 0x5e, 0x75] {
        for bad in [&[0x80][..], &[0x80; 4][..], &[0x80; 5][..], &[0x80; 6][..]] {
            let mut reader = BoundedReader::new(bad);
            let error = ClientboundPlayPacket::decode(id, &mut reader).expect_err("bad id field");
            if bad.len() < 5 {
                assert!(matches!(
                    error,
                    ProtoError::Codec(CodecError::UnexpectedEof { .. })
                ));
                assert_eq!(reader.position(), bad.len());
            } else {
                assert!(matches!(
                    error,
                    ProtoError::Codec(CodecError::VarIntTooLong)
                ));
                assert_eq!(reader.position(), 5);
            }
        }
    }
    // Every Take field uses the same signed VarInt policy; earlier fields may
    // be consumed. No packet-wide rollback is promised.
    for field in 0..3 {
        let mut body = vec![0; field];
        body.extend_from_slice(&[0x80; 6]);
        let mut reader = BoundedReader::new(&body);
        let error = TakeItemEntity::decode(&mut reader).expect_err("bad take field");
        assert!(matches!(
            error,
            ProtoError::Codec(CodecError::VarIntTooLong)
        ));
        assert_eq!(reader.position(), field + 5);
        assert_eq!(reader.remaining(), 1);
    }
    for (bytes, expected) in [
        (&[0x80, 0][..], 0),
        (&[0xff, 0xff, 0xff, 0xff, 0x7f][..], -1),
    ] {
        let value =
            SetEntityData::decode(&mut BoundedReader::new(bytes)).expect("legacy overflow bits");
        assert_eq!(value.entity_id(), expected);
        assert_eq!(value.metadata_raw().as_slice(), []);
    }
}

#[test]
fn fixed_scalar_packets_leave_trailing_bytes_for_frame_rejection() {
    for row in PAPER.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        if fields[1] == "metadata" {
            continue;
        }
        let mut bytes = unhex(fields[7]);
        bytes.push(0xee);
        let mut reader = BoundedReader::new(&bytes[1..]);
        ClientboundPlayPacket::decode(i32::from(bytes[0]), &mut reader)
            .expect("complete scalar body");
        assert_eq!(reader.read_u8().expect("trailing byte unchanged"), 0xee);
        let mut reader = BoundedReader::new(&bytes[1..]);
        ClientboundPlayPacket::decode(i32::from(bytes[0]), &mut reader)
            .expect("complete scalar body");
        assert_eq!(
            reader.finish(),
            Err(CodecError::TrailingBytes { remaining: 1 })
        );
    }
}
