use super::*;

#[test]
fn empty_native_player_has_exact_original_shape() {
    let native = NativePlayerSnapshot::empty_survival(Vec3::new(-0.0, 64.25, -3.5), [-0.0, 0.0]);
    let record = native.try_to_record().unwrap();
    assert_eq!(record.data().len(), 41 + 47 * 11);
    let restored = NativePlayerSnapshot::try_from_record(&record).unwrap();
    assert_eq!(restored.position().x.to_bits(), (-0.0_f64).to_bits());
    assert_eq!(restored.look()[0].to_bits(), (-0.0_f32).to_bits());
}

fn fixture() -> PlayerRecord {
    PlayerRecord::new(
        NATIVE_PLAYER_SCHEMA,
        GameMode::Survival,
        include_bytes!("../../tests/fixtures/native_player_fcps3/rich_app_layout.bin").to_vec(),
    )
    .unwrap()
}

#[test]
fn independent_app_layout_preserves_all_47_frames_and_component_bits() {
    let record = fixture();
    assert_eq!(&record.data()[..6], b"FCPS\0\x03");
    assert_eq!(&record.data()[6..14], &(-0.0_f64).to_bits().to_be_bytes());
    assert_eq!(&record.data()[30..34], &(-0.0_f32).to_bits().to_be_bytes());
    assert_eq!(&record.data()[38..41], &[8, 0, 46]);
    let snapshot = NativePlayerSnapshot::try_from_record(&record).unwrap();
    for (index, stack) in snapshot.slots().iter().enumerate() {
        assert_eq!(stack.count(), (index + 1) as u8);
        assert_eq!(stack.components().added().len(), 7);
        assert_eq!(stack.components().removed().len(), 2);
    }
    assert_eq!(snapshot.carried().count(), 47);
    assert_eq!(snapshot.try_to_record().unwrap().data(), record.data());
}

#[test]
fn native_parser_refuses_every_truncation_and_unknown_envelope() {
    let record = fixture();
    for at in 0..record.data().len() {
        let partial = PlayerRecord::new(
            NATIVE_PLAYER_SCHEMA,
            GameMode::Survival,
            record.data()[..at].to_vec(),
        )
        .unwrap();
        assert!(
            NativePlayerSnapshot::try_from_record(&partial).is_err(),
            "truncation {at}"
        );
    }
    for (offset, value) in [(0, b'X'), (5, 2), (38, 9), (40, 45), (41, 255)] {
        let mut bytes = record.data().to_vec();
        bytes[offset] = value;
        assert!(NativePlayerSnapshot::try_from_record(
            &PlayerRecord::new(NATIVE_PLAYER_SCHEMA, GameMode::Survival, bytes).unwrap()
        )
        .is_err());
    }
    let mut bytes = record.data().to_vec();
    bytes.push(0);
    assert!(NativePlayerSnapshot::try_from_record(
        &PlayerRecord::new(NATIVE_PLAYER_SCHEMA, GameMode::Survival, bytes).unwrap()
    )
    .is_err());
    for schema in [1, 2, 4, u32::MAX] {
        assert!(matches!(
            NativePlayerSnapshot::try_from_record(
                &PlayerRecord::new(
                    SchemaVersion::new(schema),
                    GameMode::Survival,
                    record.data().to_vec()
                )
                .unwrap()
            ),
            Err(PlayerOwnershipError::UnsupportedSchema(_))
        ));
    }
}

#[test]
fn native_parser_refuses_nonfinite_range_and_bad_nested_item() {
    for (offset, raw) in [
        (6, f64::NAN.to_bits().to_be_bytes().to_vec()),
        (14, f64::INFINITY.to_bits().to_be_bytes().to_vec()),
        (22, 30_000_000.1_f64.to_bits().to_be_bytes().to_vec()),
        (30, f32::NAN.to_bits().to_be_bytes().to_vec()),
        (34, f32::INFINITY.to_bits().to_be_bytes().to_vec()),
    ] {
        let mut bytes = fixture().data().to_vec();
        bytes[offset..offset + raw.len()].copy_from_slice(&raw);
        assert!(NativePlayerSnapshot::try_from_record(
            &PlayerRecord::new(NATIVE_PLAYER_SCHEMA, GameMode::Survival, bytes).unwrap()
        )
        .is_err());
    }
    let mut bytes = fixture().data().to_vec();
    bytes[45] = b'X';
    assert!(matches!(
        NativePlayerSnapshot::try_from_record(
            &PlayerRecord::new(NATIVE_PLAYER_SCHEMA, GameMode::Survival, bytes).unwrap()
        ),
        Err(PlayerOwnershipError::Item(_))
    ));
}
