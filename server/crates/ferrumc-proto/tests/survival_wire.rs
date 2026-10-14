//! Independent protocol-772 bytes for the Survival wire surface.
use bytes::BytesMut;
use ferrumc_codec::{BoundedReader, CodecError};
use ferrumc_proto::generated::play::{ClientboundPlayPacket, ServerboundPlayPacket};
use ferrumc_proto::ProtoError;

const FIXTURE: &str = include_str!("fixtures/survival_1_21_8.tsv");

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                .expect("valid fixture hex")
        })
        .collect()
}

#[test]
fn independent_prismarine_bytes_dispatch_and_scalar_semantics() {
    let mut count = 0;
    for row in FIXTURE.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        let bytes = unhex(fields[2]);
        let mut reader = BoundedReader::new(&bytes);
        let id = reader.read_var_int().expect("packet ID");
        let mut encoded = BytesMut::new();
        if fields[0] == "client" {
            let packet = ClientboundPlayPacket::decode(id, &mut reader).expect(fields[1]);
            match (&packet, fields[1]) {
                (ClientboundPlayPacket::UpdateHealth(p), "update_health") => {
                    assert!((0.0..=20.0).contains(&p.health()));
                    assert!((0..=20).contains(&p.food()));
                    assert!((0.0..=5.0).contains(&p.saturation()));
                }
                (ClientboundPlayPacket::BlockDestruction(p), "block_break_animation") => {
                    assert_eq!(p.entity_id(), 129);
                    assert_eq!(p.position().x(), -17);
                    assert_eq!(p.position().y(), -64);
                    assert_eq!(p.position().z(), 16);
                }
                (ClientboundPlayPacket::UpdateLight(p), "update_light") => {
                    assert_eq!(p.x(), -2);
                    assert_eq!(p.z(), 128);
                    assert!(p.sky_light().iter().all(|a| a.as_slice().len() == 2048));
                }
                _ => {}
            }
            packet.encode(&mut encoded).expect("client encode");
        } else {
            let packet = ServerboundPlayPacket::decode(id, &mut reader).expect(fields[1]);
            if let ServerboundPlayPacket::UseItem(p) = &packet {
                assert_eq!(p.sequence(), 128);
                assert_eq!(p.yaw(), 180.0);
                assert_eq!(p.pitch(), -45.0);
            }
            packet.encode(&mut encoded).expect("server encode");
        }
        reader.finish().expect("exact fixture body");
        assert_eq!(encoded.as_ref(), bytes, "{}", fields[1]);
        count += 1;
    }
    assert_eq!(count, 33);
}

#[test]
fn scalar_and_light_fixture_truncations_refuse() {
    for row in FIXTURE.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        // InteractEntity intentionally carries an opaque bounded tail; its
        // action-specific shape is validated by the session consumer.
        if fields[1] == "use_entity" {
            continue;
        }
        let bytes = unhex(fields[2]);
        let body = &bytes[1..];
        for len in 0..body.len() {
            let mut reader = BoundedReader::new(&body[..len]);
            let refused = if fields[0] == "client" {
                ClientboundPlayPacket::decode(i32::from(bytes[0]), &mut reader).is_err()
            } else {
                ServerboundPlayPacket::decode(i32::from(bytes[0]), &mut reader).is_err()
            };
            assert!(refused, "{} truncated at {len}", fields[1]);
        }
    }
}

#[test]
fn interaction_tail_is_capped_before_copy() {
    let bytes = [0_u8; 21]; // target, action, nineteen tail bytes
    let error = ServerboundPlayPacket::decode(0x19, &mut BoundedReader::new(&bytes))
        .expect_err("tail exceeds eighteen-byte local admission cap");
    assert!(matches!(
        error,
        ProtoError::Codec(CodecError::BytesTooLong { .. })
    ));
}
