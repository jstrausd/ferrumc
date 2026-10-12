//! Fixed grammars and whole-endpoint evidence; no parser issues write authority.
use super::*;
use crate::block_test_support::*;
use ferrumc_world::{decode_block_entity, try_encode_block_entity, BlockEntity};
const Q: &[u8] = include_bytes!("../../tests/fixtures/player_block_format7/joint.fcbq");
const RECEIPT: &[u8] = include_bytes!("../../tests/fixtures/player_block_format7/joint.fcbr");
#[test]
fn every_truncated_fcbq_fcbr_fcob_fcbl_prefix_refuses() {
    for n in 0..Q.len() {
        assert!(raw::q(&Q[..n]).is_err(), "Q prefix{n}");
    }
    for n in 0..RECEIPT.len() {
        assert!(
            raw::receipt(
                &id(3).to_bytes(),
                &RECEIPT[..n],
                &mut engine::Budget::hard()
            )
            .is_err(),
            "receipt prefix{n}"
        );
    }
    let header=b"FCOB\0\x01\x01\0\0\0\0\0\0\0\x02\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03";
    assert!(raw::header(header).is_ok());
    for n in 0..header.len() {
        assert!(raw::header(&header[..n]).is_err());
    }
    let body = raw::q(Q).unwrap();
    for (ordinal, e) in body.effects.iter().enumerate() {
        let log = raw::journal(scope(), id(3), ordinal as u16, ordinal as u64, e.bytes);
        assert!(raw::journal_ref(&log).is_ok());
        for n in 0..log.len() {
            assert!(raw::journal_ref(&log[..n]).is_err());
        }
    }
}
#[test]
fn receipt_magic_kind_lengths_inactive_reserved_digest_range_and_trailing_refuse() {
    let valid = raw::receipt(&id(3).to_bytes(), RECEIPT, &mut engine::Budget::hard()).unwrap();
    assert_eq!(valid.request_bytes(), Q);
    let mut cases = Vec::new();
    for (at, value) in [
        (0, 0),
        (4, 1),
        (5, 2),
        (16, 0),
        (16, 4),
        (RECEIPT.len() - 1, 1),
        (RECEIPT.len() - 40, 1),
    ] {
        let mut v = RECEIPT.to_vec();
        v[at] = value;
        cases.push(v);
    }
    let mut v = RECEIPT.to_vec();
    v[6..10].copy_from_slice(&u32::MAX.to_be_bytes());
    cases.push(v);
    let mut v = RECEIPT.to_vec();
    v.extend_from_slice(&[0]);
    cases.push(v);
    let tail = 10 + Q.len() + 9 + 1 + 18;
    for (offset, value) in [(0, 0), (0, 2), (9, 0xff), (10, 0xff)] {
        let mut v = RECEIPT.to_vec();
        v[tail + offset] = value;
        cases.push(v);
    }
    for (n, v) in cases.iter().enumerate() {
        assert!(
            raw::receipt(&id(3).to_bytes(), v, &mut engine::Budget::hard()).is_err(),
            "receipt malformed{n}"
        );
    }
    assert!(raw::receipt(&id(4).to_bytes(), RECEIPT, &mut engine::Budget::hard()).is_err());
    let image = include_bytes!("../../tests/fixtures/player_block_format7/seed.fcbq");
    for at in [58, 74, 75, 83, 88] {
        let mut v = image.to_vec();
        v[at] = 1;
        assert!(raw::q(&v).is_err(), "inactive{at}");
    }
    let mut q = Q.to_vec();
    q.push(0);
    assert!(raw::q(&q).is_err());
}
#[test]
fn fixed_complete_endpoint_and_each_effect_chain_and_current_entity_encoding() {
    let q = raw::q(Q).unwrap();
    engine::endpoints(&q, &mut engine::Budget::hard()).unwrap();
    let e = raw::effect_bytes(&effect(pos(15, 64), 0, 0, None, None));
    let mut q = raw::q(Q).unwrap();
    q.effects[0] = raw::effect(&e).unwrap();
    assert!(matches!(
        engine::endpoints(&q, &mut engine::Budget::hard()),
        Err(BlockDurabilityError::InvalidEffect { ordinal: 0, .. })
    ));
    let e = raw::effect_bytes(&effect(pos(15, 64), 9, 100, None, None));
    let mut q = raw::q(Q).unwrap();
    q.effects[0] = raw::effect(&e).unwrap();
    assert!(matches!(
        engine::endpoints(&q, &mut engine::Budget::hard()),
        Err(BlockDurabilityError::InvalidEffect { ordinal: 0, .. })
    ));
    for entity in [vec![], vec![0], vec![0; 16 * 1024 + 1], vec![0x7f, 0, 9, 1]] {
        assert!(raw::canonical_entity(&entity).is_err());
    }
    raw::canonical_entity(include_bytes!(
        "../../tests/fixtures/player_block_format7/chest8.entity"
    ))
    .unwrap();
}
#[test]
fn exact_sequence_allocator_max_boundary_and_capacity_configuration_do_not_wrap() {
    let c = config();
    assert!(BlockDurabilityCapacity::new(
        c.capacity.ownership,
        NonZeroUsize::new(143).unwrap(),
        nz(1),
        nz(1),
        nz(1),
        nz(1)
    )
    .is_err());
    assert!(BlockDurabilityCapacity::new(
        c.capacity.ownership,
        NonZeroUsize::new(MAX_BLOCK_REQUEST_BYTES + 1).unwrap(),
        nz(1),
        nz(1),
        nz(1),
        nz(1)
    )
    .is_err());
    assert!(BlockDurabilityCapacity::new(
        c.capacity.ownership,
        NonZeroUsize::new(144).unwrap(),
        nz(u64::MAX),
        nz(1),
        nz(1),
        nz(1)
    )
    .is_err());
    assert!(
        BlockDurabilityLimits::new(nz(512 * 1024 * 1024 + 1), nz(1), nz(1), nz(1), nz(1)).is_err()
    );
    assert!(BlockDurabilityLimits::new(nz(1), nz(16_777_216 + 1), nz(1), nz(1), nz(1)).is_err());
    assert_eq!(
        engine::allocate_sequences(u64::MAX - 1, Some(u64::MAX - 2), 1).unwrap(),
        (Some(u64::MAX - 1), u64::MAX)
    );
    assert!(matches!(
        engine::allocate_sequences(u64::MAX, Some(u64::MAX - 1), 1),
        Err(BlockDurabilityError::SequenceExhausted)
    ));
    assert_eq!(
        engine::allocate_sequences(u64::MAX, Some(u64::MAX - 1), 0).unwrap(),
        (None, u64::MAX)
    );
    assert!(matches!(
        engine::allocate_sequences(0, Some(u64::MAX), 1),
        Err(BlockDurabilityError::MalformedData { .. })
    ));
    let mut b = engine::Budget::runtime(c);
    b.bytes(c.limits.bytes).unwrap();
    assert!(b.bytes(1).is_err());
    let mut b = engine::Budget::runtime(c);
    b.cells(c.limits.cells).unwrap();
    assert!(b.cells(1).is_err());
}
/// Each distinct last receipt is read, digested and parsed once per call; the call-local
/// cache holds at most one entry per column and refuses a ninth distinct receipt.
#[test]
fn per_call_receipt_cache_loads_each_row_once_and_holds_at_most_eight() {
    let valid = raw::frame(&id(3).to_bytes(), RECEIPT, &mut engine::Budget::hard()).unwrap();
    let rows: Vec<_> = (0..=MAX_BLOCK_CHUNKS as u8)
        .map(|n| {
            let mut q = Q.to_vec();
            q[7..23].copy_from_slice(&[0x40 + n; 16]);
            raw::encode_receipt(&q, valid.player, &valid.chunks, valid.first, valid.count)
        })
        .collect();
    let slots = engine::rows();
    let mut lasts = engine::Lasts::new(&slots);
    let mut b = engine::Budget::hard();
    let mut reads = 0;
    for _ in 0..3 {
        let at = lasts
            .load(id(0x40), &mut b, || {
                reads += 1;
                Ok(Some(rows[0].as_slice()))
            })
            .unwrap();
        assert_eq!(at, 0);
    }
    assert_eq!(reads, 1);
    // The reader charges the row read; the cache charges the body digest exactly once.
    assert_eq!(b.used_bytes(), Q.len() as u64);
    for (n, row) in rows.iter().enumerate().take(MAX_BLOCK_CHUNKS).skip(1) {
        let at = lasts
            .load(id(0x40 + n as u8), &mut b, || Ok(Some(row.as_slice())))
            .unwrap();
        assert_eq!(at, n);
    }
    assert_eq!(b.used_bytes(), MAX_BLOCK_CHUNKS as u64 * Q.len() as u64);
    let ninth = MAX_BLOCK_CHUNKS as u8;
    assert!(matches!(
        lasts.load(id(0x40 + ninth), &mut b, || Ok(Some(
            rows[ninth as usize].as_slice()
        ))),
        Err(BlockDurabilityError::MalformedData { .. })
    ));
    // An absent or forged row refuses without being cached.
    let slots = engine::rows::<&[u8]>();
    let mut lasts = engine::Lasts::new(&slots);
    assert!(lasts.load(id(0x40), &mut b, || Ok(None)).is_err());
    let slots = engine::rows();
    let mut lasts = engine::Lasts::new(&slots);
    assert!(lasts
        .load(id(0x41), &mut b, || Ok(Some(rows[0].as_slice())))
        .is_err());
}
/// A sign through the World codec's own legacy grammar (tag 0), so color, glow and waxing can
/// be set without a crate-internal constructor.
fn legacy_sign(kind: u8, waxed: u8, color: &str, glow: u8, lines: [&str; 4]) -> BlockEntity {
    let mut b = vec![0, kind, waxed];
    for _ in 0..2 {
        b.extend_from_slice(&(color.len() as u16).to_be_bytes());
        b.extend_from_slice(color.as_bytes());
        b.push(glow);
        for l in lines {
            b.extend_from_slice(&(l.len() as u16).to_be_bytes());
            b.extend_from_slice(l.as_bytes());
        }
    }
    decode_block_entity(&b).unwrap()
}
/// The charged entity length equals the canonical World encoding's actual length, computed
/// without encoding; entities the encoder refuses are charged the 16 KiB form bound, which
/// bounds the encoder's work as well.
#[test]
fn entity_len_is_the_exact_canonical_encoding_length_or_the_form_bound() {
    use ferrumc_items::{
        ComponentPatch, ComponentValue, ItemId, ItemStack, OpaqueComponent, MAX_COMPONENT_BYTES,
    };
    use ferrumc_world::{
        BlockEntity as E, ChestInventory, Sign, SignKind, MAX_BLOCK_ENTITY_PAYLOAD_LEN as MAX,
    };
    let stone = |n: u8, patch: ComponentPatch| {
        ItemStack::try_new(ItemId::from_name("stone").unwrap(), n, patch).unwrap()
    };
    let chest = |slots: &[(usize, ItemStack)]| {
        let mut c = ChestInventory::new();
        for (i, s) in slots {
            *c.slot_mut(*i).unwrap() = s.clone();
        }
        E::Chest(c)
    };
    let opaque = |id: i32, len: usize| {
        ComponentValue::Opaque(OpaqueComponent::new(id, vec![0x5a; len]).unwrap())
    };
    let long = "\u{e9}".repeat(512);
    let encodable = [
        E::Chest(ChestInventory::new()),
        E::Sign(Sign::new(SignKind::Sign)),
        E::Sign(Sign::new(SignKind::Hanging)),
        legacy_sign(1, 1, "light_blue", 1, ["a", "\u{fc}ber", "", &long]),
        legacy_sign(0, 0, &"c".repeat(64), 0, [&long, &long, &long, &long]),
        decode_block_entity(&chest_bytes(5)).unwrap(),
        chest(&[
            (0, stone(1, ComponentPatch::default())),
            (26, stone(64, ComponentPatch::default())),
        ]),
        chest(&[(
            13,
            stone(
                13,
                ComponentPatch::new(vec![opaque(6, 300), opaque(7, 4000)], Vec::new()),
            ),
        )]),
        // The World codec's exact 16 KiB chest (its own golden layout: 339 overhead bytes).
        chest(&[(
            26,
            stone(
                13,
                ComponentPatch::new(
                    vec![
                        opaque(6, MAX_COMPONENT_BYTES),
                        opaque(7, MAX - 339 - MAX_COMPONENT_BYTES),
                    ],
                    Vec::new(),
                ),
            ),
        )]),
    ];
    let mut lens = Vec::new();
    for e in &encodable {
        let mut out = Vec::new();
        try_encode_block_entity(e, &mut out).unwrap();
        assert_eq!(engine::entity_len(e), out.len() as u64, "{e:?}");
        lens.push(out.len());
    }
    // Independent anchors: an empty chest is 4 + 27·(4 + 7); a blank sign, whose faces default
    // to the color "black", 6 + 2·(3 + 5 + 4·2).
    assert_eq!(&lens[..3], &[301, 38, 38]);
    assert_eq!(lens[8], MAX);
    // Refused encodings are charged their would-be length capped at the bound; the encoder
    // never writes more than that before refusing. One byte over the aggregate chest bound:
    let over = chest(&[(
        26,
        stone(
            13,
            ComponentPatch::new(
                vec![
                    opaque(6, MAX_COMPONENT_BYTES),
                    opaque(7, MAX - 338 - MAX_COMPONENT_BYTES),
                ],
                Vec::new(),
            ),
        ),
    )]);
    assert!(try_encode_block_entity(&over, &mut Vec::new()).is_err());
    assert_eq!(engine::entity_len(&over), MAX as u64);
    // A sign line over its 1024-byte field cap: refused, charged its would-be length
    // 6 + (3 + 5 + 4·1027) + (3 + 5 + 4·2) with both faces' default color "black".
    let mut wide = Sign::new(SignKind::Sign);
    wide.set_face_lines(true, std::array::from_fn(|_| "w".repeat(1025)));
    let wide = E::Sign(wide);
    assert!(try_encode_block_entity(&wide, &mut Vec::new()).is_err());
    assert_eq!(engine::entity_len(&wide), 4138);
    // The canonical check charges its re-encoding's own length before making it: a legacy
    // empty chest (tag 1, 27 empty slots: 28 bytes) re-encodes to 301 bytes and then refuses.
    let legacy = [&[1u8][..], &[0; 27]].concat();
    let mut charged = Vec::new();
    assert!(raw::canonical_entity_charged(&legacy, |n| {
        charged.push(n);
        Ok(())
    })
    .is_err());
    assert_eq!(charged, vec![301]);
    let canonical = chest_bytes(5);
    let mut charged = Vec::new();
    raw::canonical_entity_charged(&canonical, |n| {
        charged.push(n);
        Ok(())
    })
    .unwrap();
    assert_eq!(charged, vec![canonical.len() as u64]);
}
