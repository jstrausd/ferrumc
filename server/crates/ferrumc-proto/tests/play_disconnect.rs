//! The 1.21.8 Play Disconnect contract, with an independently encoded fixture.
use bytes::BytesMut;
use ferrumc_codec::{BoundedReader, CodecError};
use ferrumc_nbt::{NbtCompound, NbtError, NbtTag};
use ferrumc_proto::generated::play::{ClientboundPlayPacket, PlayDisconnect};
use ferrumc_proto::ProtoError;

// Produced by minecraft-protocol 1.66.2 / minecraft-data 3.111.0's 1.21.8
// server serializer for kick_disconnect {reason: {text: "UUID busy"}}.
// Includes the packet id; no frame length or NBT root name is present.
const PRISMARINE_PACKET: &[u8] = b"\x1c\x0a\x08\x00\x04text\x00\x09UUID busy\x00";

fn reason() -> NbtTag {
    let mut compound = NbtCompound::new();
    compound.push("text", NbtTag::String("UUID busy".to_owned()));
    NbtTag::Compound(compound)
}

#[test]
fn play_disconnect_matches_independent_1_21_8_packet_bytes() {
    assert_eq!(PlayDisconnect::PACKET_ID, 0x1c);
    let packet = PlayDisconnect::new(reason());
    let mut encoded = BytesMut::new();
    ClientboundPlayPacket::PlayDisconnect(packet.clone())
        .encode(&mut encoded)
        .expect("encode play disconnect");
    assert_eq!(encoded.as_ref(), PRISMARINE_PACKET);

    let mut reader = BoundedReader::new(PRISMARINE_PACKET);
    let id = reader.read_var_int().expect("packet id");
    let decoded = ClientboundPlayPacket::decode(id, &mut reader).expect("independent NBT");
    assert_eq!(decoded, ClientboundPlayPacket::PlayDisconnect(packet));
    reader.finish().expect("no trailing bytes");
}

#[test]
fn play_disconnect_rejects_every_truncated_nbt_prefix() {
    let body = &PRISMARINE_PACKET[1..];
    for len in 0..body.len() {
        let error = PlayDisconnect::decode(&mut BoundedReader::new(&body[..len]))
            .expect_err("truncated reason must fail");
        assert!(
            matches!(error, ProtoError::Nbt(NbtError::Codec(_))),
            "prefix {len}: {error:?}"
        );
    }
}

#[test]
fn play_disconnect_rejects_wrong_root_and_unknown_nested_tag() {
    for id in [0, 1, 8] {
        let error = PlayDisconnect::decode(&mut BoundedReader::new(&[id]))
            .expect_err("non-compound root must fail");
        assert!(matches!(
            error,
            ProtoError::Nbt(NbtError::UnexpectedRootTag { id: found }) if found == id
        ));
    }
    for body in [&[99][..], &[10, 99][..]] {
        let error = PlayDisconnect::decode(&mut BoundedReader::new(body))
            .expect_err("unknown root or child tag must fail");
        assert!(matches!(
            error,
            ProtoError::Nbt(NbtError::UnknownTagType { id: 99 })
        ));
    }
}

#[test]
fn play_disconnect_rejects_negative_and_unbounded_array_lengths() {
    // Anonymous compound, byte-array child "x", big-endian signed length.
    for (length, negative) in [(-1_i32, true), (i32::MAX, false)] {
        let mut body = b"\x0a\x07\x00\x01x".to_vec();
        body.extend_from_slice(&length.to_be_bytes());
        let error = PlayDisconnect::decode(&mut BoundedReader::new(&body))
            .expect_err("hostile array length must fail before allocation");
        if negative {
            assert!(matches!(
                error,
                ProtoError::Nbt(NbtError::NegativeLength { len: -1 })
            ));
        } else {
            assert!(matches!(
                error,
                ProtoError::Nbt(NbtError::ListTooLong { .. })
            ));
        }
    }
}

#[test]
fn play_disconnect_preserves_trailing_bytes_for_frame_rejection() {
    let mut body = PRISMARINE_PACKET[1..].to_vec();
    body.push(0xee);
    let mut reader = BoundedReader::new(&body);
    PlayDisconnect::decode(&mut reader).expect("one valid NBT root");
    assert_eq!(
        reader.finish(),
        Err(CodecError::TrailingBytes { remaining: 1 })
    );
}

#[test]
fn play_disconnect_rejects_malformed_modified_utf8() {
    // text field has one invalid Modified UTF-8 lead byte.
    let body = b"\x0a\x08\x00\x04text\x00\x01\xff\x00";
    let error = PlayDisconnect::decode(&mut BoundedReader::new(body))
        .expect_err("malformed text must fail");
    assert!(matches!(error, ProtoError::Nbt(NbtError::InvalidUtf8)));
}
