use super::*;

fn schema_two_payload() -> serde_json::Value {
    let inventory = sample_inventory();
    serde_json::json!({
        "x":4.0,"y":65.0,"z":-9.0,"yaw":23.0,"pitch":-7.0,"selected_slot":5,
        "slots":(0..SLOT_COUNT).map(|index| {
            PersistedSlot::capture(inventory.slot(index).expect("slot"))
        }).collect::<Vec<_>>(),
        "carried":{"item_id":0,"count":0},
    })
}

fn assert_corrupt(payload: Vec<u8>) {
    let record = PlayerRecord::new(JSON_PLAYER_SCHEMA_VERSION, GameMode::Survival, payload)
        .expect("bounded malformed record");
    assert!(matches!(
        PlayerData::from_record(&record),
        Err(PlayerLoadError::Corrupt { .. })
    ));
}

#[test]
fn schema_one_explicitly_migrates_without_rewriting_the_loaded_record() {
    let mut legacy = schema_two_payload();
    legacy.as_object_mut().expect("object").remove("carried");
    legacy["slots"].as_array_mut().expect("slots").truncate(39);
    let original = serde_json::to_vec(&legacy).expect("legacy JSON");
    let record = PlayerRecord::new(
        LEGACY_PLAYER_SCHEMA_VERSION,
        GameMode::Survival,
        original.clone(),
    )
    .expect("record");
    let decoded = PlayerData::from_record(&record).expect("explicit migration");
    assert_eq!(
        record.data(),
        original,
        "loading never rewrites storage bytes"
    );
    assert_eq!(record.schema_version(), LEGACY_PLAYER_SCHEMA_VERSION);
    assert_eq!(decoded.position(), Vec3::new(4.0, 65.0, -9.0));
    assert_eq!((decoded.yaw(), decoded.pitch()), (23.0, -7.0));
    let inventory = decoded.restore_inventory(GameMode::Survival);
    assert_eq!(inventory.selected(), 5);
    assert!(inventory.carried().item().is_none());
    for index in 0..39 {
        let slot: PersistedSlot =
            serde_json::from_value(legacy["slots"][index].clone()).expect("legacy slot");
        assert_eq!(inventory.slot(index), Some(&slot.restore()));
    }
    assert!((39..SLOT_COUNT).all(|index| inventory
        .slot(index)
        .expect("padded slot")
        .item()
        .is_none()));
    let rewritten = decoded
        .to_record(GameMode::Survival)
        .expect("next leave encoding");
    assert_eq!(rewritten.schema_version(), PLAYER_SCHEMA_VERSION);
    assert!(rewritten.data().starts_with(b"FCPS"));
    let current = PlayerData::from_record(&rewritten).expect("schema-three rewrite");
    assert!(current.carried.item().is_none());
    assert_eq!(current.slots.len(), SLOT_COUNT);
    assert!(PlayerData::from_record(&rewritten).is_ok());
}

#[test]
fn full_inventory_overflow_round_trips_without_components() {
    let mut inventory = sample_inventory();
    let stone = ItemId::from_name("stone").expect("stone");
    for index in crate::inventory::MAIN_START..crate::inventory::PLAYER_WINDOW_END {
        inventory.set_creative_slot(
            index,
            ItemStack::try_new(stone, 64, ComponentPatch::empty()).expect("full stack"),
        );
    }
    let carried = ItemStack::try_new(stone, 7, ComponentPatch::empty()).expect("carried");
    inventory.set_carried(carried.clone());
    inventory.return_carried();
    let captured = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &inventory);
    let record = captured.to_record(GameMode::Creative).expect("save");
    let restored = PlayerData::from_record(&record)
        .expect("load")
        .restore_inventory(GameMode::Creative);
    assert_eq!(restored.carried(), &carried);
    assert_eq!(
        restored.container_content_payload().expect("payload"),
        inventory.container_content_payload().expect("payload")
    );
}

#[test]
fn component_persistence_retains_the_complete_carried_patch() {
    let mut inventory = sample_inventory();
    let stack = ItemStack::try_new(
        ItemId::from_name("stone").expect("stone"),
        7,
        ComponentPatch::new(vec![ferrumc_items::ComponentValue::Damage(3)], Vec::new()),
    )
    .expect("component stack");
    inventory.set_carried(stack.clone());
    let record = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &inventory)
        .to_record(GameMode::Creative)
        .expect("encode");
    let restored = PlayerData::from_record(&record)
        .expect("decode")
        .restore_inventory(GameMode::Creative);
    assert_eq!(restored.carried().item(), stack.item());
    assert_eq!(restored.carried().count(), stack.count());
    assert_eq!(restored.carried(), &stack);
}

#[test]
fn schema_two_rejects_missing_invalid_unknown_or_duplicate_carried_data() {
    let valid = schema_two_payload();
    assert!(PlayerData::from_record(
        &PlayerRecord::new(
            JSON_PLAYER_SCHEMA_VERSION,
            GameMode::Survival,
            serde_json::to_vec(&valid).expect("valid")
        )
        .expect("record")
    )
    .is_ok());
    let mut missing = valid.clone();
    missing.as_object_mut().expect("object").remove("carried");
    assert_corrupt(serde_json::to_vec(&missing).expect("JSON"));
    let stone = ItemId::from_name("stone").expect("stone").id();
    let sword = ItemId::from_name("diamond_sword").expect("sword").id();
    for carried in [
        serde_json::Value::Null,
        serde_json::json!([]),
        serde_json::json!("empty"),
        serde_json::json!({}),
        serde_json::json!({"item_id":stone}),
        serde_json::json!({"item_id":stone,"count":0}),
        serde_json::json!({"item_id":stone,"count":65}),
        serde_json::json!({"item_id":sword,"count":2}),
        serde_json::json!({"item_id":0,"count":1}),
        serde_json::json!({"item_id":-1,"count":7}),
        serde_json::json!({"item_id":i32::MAX,"count":7}),
        serde_json::json!({"item_id":stone,"count":-1}),
        serde_json::json!({"item_id":stone,"count":256}),
        serde_json::json!({"item_id":stone,"count":1,"components":[]}),
        serde_json::json!({"item_id":stone,"count":1,"ignored":true}),
    ] {
        let mut payload = valid.clone();
        payload["carried"] = carried;
        assert_corrupt(serde_json::to_vec(&payload).expect("JSON"));
    }
    let bytes = serde_json::to_vec(&valid).expect("JSON");
    let json = String::from_utf8(bytes.clone()).expect("UTF8");
    let duplicate = json.replacen(
        "\"carried\":",
        "\"carried\":{\"item_id\":0,\"count\":0},\"carried\":",
        1,
    );
    assert_corrupt(duplicate.into_bytes());
    let duplicate_field = json.replacen("\"carried\":{", "\"carried\":{\"count\":0,", 1);
    assert_corrupt(duplicate_field.into_bytes());
    assert_corrupt(bytes[..bytes.len() - 1].to_vec());
    let mut trailing = bytes;
    trailing.extend_from_slice(b"trailing");
    assert_corrupt(trailing);
}

#[test]
fn schema_two_rejects_wrong_layout_lengths_and_unknown_fields() {
    let valid = schema_two_payload();
    for count in [0, SLOT_COUNT - 1, SLOT_COUNT + 1, 4096] {
        let mut payload = valid.clone();
        payload["slots"] =
            serde_json::json!(vec![serde_json::json!({"item_id":0,"count":0}); count]);
        assert_corrupt(serde_json::to_vec(&payload).expect("JSON"));
    }
    let mut unknown = valid.clone();
    unknown["future_items"] = serde_json::json!([1]);
    assert_corrupt(serde_json::to_vec(&unknown).expect("JSON"));
    let mut unknown_slot = valid;
    unknown_slot["slots"][0]["future_items"] = serde_json::json!([1]);
    assert_corrupt(serde_json::to_vec(&unknown_slot).expect("JSON"));
}

fn sample_inventory() -> PlayerInventory {
    let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
    // Move the selector off the default and edit a non-hotbar slot so the
    // round-trip has something distinctive to preserve.
    assert!(inv.set_selected(5));
    inv
}

#[test]
fn round_trips_through_a_record() {
    let inv = sample_inventory();
    let data = PlayerData::capture(Vec3::new(1.5, 64.0, -3.25), 90.0, -12.5, &inv);

    let record = data.to_record(GameMode::Creative).expect("encodes");
    assert_eq!(record.schema_version(), PLAYER_SCHEMA_VERSION);
    assert_eq!(record.game_mode(), GameMode::Creative);

    let decoded = PlayerData::from_record(&record).expect("decodes");
    // Full-struct equality covers position, yaw, pitch, selected slot, and every
    // inventory slot in one shot (avoiding bare float comparisons); the typed
    // position accessor is spot-checked separately.
    assert_eq!(decoded, data);
    assert_eq!(decoded.position(), Vec3::new(1.5, 64.0, -3.25));

    // The restored inventory matches the captured one slot-for-slot (item id +
    // count) and keeps the selected index.
    let restored = decoded.restore_inventory(GameMode::Creative);
    for index in 0..SLOT_COUNT {
        let before = inv.slot(index).and_then(ItemStack::item).map(ItemId::id);
        let after = restored
            .slot(index)
            .and_then(ItemStack::item)
            .map(ItemId::id);
        assert_eq!(before, after, "slot {index} item id");
        assert_eq!(
            inv.slot(index).map(ItemStack::count),
            restored.slot(index).map(ItemStack::count),
            "slot {index} count"
        );
    }
    assert_eq!(restored.selected(), 5);
    assert_eq!(restored.game_mode(), GameMode::Creative);
}

#[tokio::test]
async fn survives_a_store_reload() {
    use ferrumc_core::PlayerId;
    use ferrumc_storage::{InMemoryStore, PlayerStore};

    // Save into the fake store, then load it back — the fast stand-in for a
    // server restart: the bytes that come back must decode to the same state.
    let store = InMemoryStore::new();
    let player = PlayerId::offline("Saad");
    let inv = sample_inventory();
    let data = PlayerData::capture(Vec3::new(10.0, 65.0, -7.0), 12.0, 34.0, &inv);
    store
        .save_player(player, data.to_record(GameMode::Creative).expect("encode"))
        .await
        .expect("save");

    let loaded = store
        .load_player(player)
        .await
        .expect("load")
        .expect("record present");
    assert_eq!(loaded.game_mode(), GameMode::Creative);
    assert_eq!(PlayerData::from_record(&loaded).expect("decodes"), data);
}

#[test]
fn malformed_payload_is_classified_as_corrupt_without_panicking() {
    let malformed = [
        Vec::new(),
        vec![0xff, 0x00, 0x42],
        b"{".to_vec(),
        b"[]".to_vec(),
        br#"{"x":"wrong"}"#.to_vec(),
        br#"{"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#.to_vec(),
        br#"{"x":null,"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
            .to_vec(),
        br#"{"x":"NaN","y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
            .to_vec(),
        br#"{"x":[],"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#.to_vec(),
        br#"{"x":{},"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#.to_vec(),
        br#"{"x":1e400,"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
            .to_vec(),
        br#"{"x":0.0,"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}trailing"#
            .to_vec(),
    ];
    for payload in malformed {
        let record = PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload.clone())
            .expect("malformed payload remains within the storage bound");
        assert!(
            matches!(
                PlayerData::from_record(&record),
                Err(PlayerLoadError::Corrupt { .. })
            ),
            "payload {payload:?} was not classified as corrupt",
        );
    }
}

#[test]
fn player_position_validation_is_finite_and_inclusive_on_every_axis() {
    let edge = MAX_PLAYER_POSITION_MAGNITUDE;
    for position in [
        Vec3::ZERO,
        Vec3::new(edge, 0.0, 0.0),
        Vec3::new(-edge, 0.0, 0.0),
        Vec3::new(0.0, edge, 0.0),
        Vec3::new(0.0, -edge, 0.0),
        Vec3::new(0.0, 0.0, edge),
        Vec3::new(0.0, 0.0, -edge),
    ] {
        assert!(
            is_valid_player_position(position),
            "boundary position {position:?} should be accepted",
        );
    }

    let outside = edge + 1.0;
    for position in [
        Vec3::new(outside, 0.0, 0.0),
        Vec3::new(-outside, 0.0, 0.0),
        Vec3::new(0.0, outside, 0.0),
        Vec3::new(0.0, -outside, 0.0),
        Vec3::new(0.0, 0.0, outside),
        Vec3::new(0.0, 0.0, -outside),
        Vec3::new(f64::NAN, 0.0, 0.0),
        Vec3::new(0.0, f64::INFINITY, 0.0),
        Vec3::new(0.0, 0.0, f64::NEG_INFINITY),
    ] {
        assert!(
            !is_valid_player_position(position),
            "unsafe position {position:?} should be rejected",
        );
    }
}

#[test]
fn incompatible_schema_version_is_classified() {
    let inv = sample_inventory();
    let data = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &inv);
    let bytes = data
        .to_record(GameMode::Creative)
        .expect("encodes")
        .data()
        .to_vec();
    // A valid payload under a *different* schema version must be treated as
    // incompatible rather than misread.
    for found in [SchemaVersion::new(0), SchemaVersion::new(9999)] {
        let record =
            PlayerRecord::new(found, GameMode::Creative, bytes.clone()).expect("within bound");
        assert!(matches!(
            PlayerData::from_record(&record),
            Err(PlayerLoadError::Incompatible {
                found: actual,
                expected: PLAYER_SCHEMA_VERSION,
            }) if actual == found
        ));
    }
}

#[test]
fn restore_normalizes_corrupt_slots() {
    // A short slot list, an unknown item id, and an absurd count must all
    // restore to a valid window without panicking.
    let data = PlayerData::from(LegacyPlayerData {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        yaw: 0.0,
        pitch: 0.0,
        selected_slot: 200, // out of range
        slots: vec![
            PersistedSlot {
                item_id: -1,
                count: 200,
            }, // unknown id
            PersistedSlot {
                item_id: 1,
                count: 0,
            }, // zero count -> empty
        ],
    });
    let inv = data.restore_inventory(GameMode::Creative);
    assert!(inv.slot(0).expect("slot 0").item().is_none());
    assert!(inv.slot(1).expect("slot 1").item().is_none());
    // Out-of-range selected index clamps to a valid hotbar slot.
    assert!(inv.selected() <= 8);
}

fn assert_schema_three_corrupt(payload: Vec<u8>) {
    let record = PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload)
        .expect("malformed record is within storage cap");
    assert!(matches!(
        PlayerData::from_record(&record),
        Err(PlayerLoadError::Corrupt { .. })
    ));
}

#[test]
fn schema_two_explicitly_migrates_and_retains_its_slot_normalization_policy() {
    let mut payload = schema_two_payload();
    payload["slots"][9] = serde_json::json!({"item_id":1,"count":200});
    payload["slots"][10] = serde_json::json!({"item_id":-1,"count":7});
    payload["selected_slot"] = serde_json::json!(200);
    payload["carried"] = serde_json::json!({"item_id":1,"count":7});
    let bytes = serde_json::to_vec(&payload).expect("JSON");
    let record = PlayerRecord::new(
        JSON_PLAYER_SCHEMA_VERSION,
        GameMode::Survival,
        bytes.clone(),
    )
    .expect("record");
    let data = PlayerData::from_record(&record).expect("schema-two migration");
    let inventory = data.restore_inventory(GameMode::Survival);
    assert_eq!(inventory.slot(9).expect("slot").count(), 64);
    assert!(inventory.slot(10).expect("slot").item().is_none());
    assert_eq!(inventory.selected(), 0);
    assert_eq!(inventory.carried().count(), 7);
    assert_eq!(
        record.data(),
        bytes,
        "migration never changes storage bytes"
    );
    assert_eq!(record.schema_version(), JSON_PLAYER_SCHEMA_VERSION);
    let updated = data
        .to_record(GameMode::Survival)
        .expect("next successful save");
    assert_eq!(updated.schema_version(), PLAYER_SCHEMA_VERSION);
    assert_eq!(PlayerData::from_record(&updated).expect("new schema"), data);
}

fn rich_patch() -> ComponentPatch {
    use ferrumc_items::{ComponentTypeId, ComponentValue, OpaqueComponent};
    use ferrumc_nbt::{NbtCompound, NbtTag};
    let mut data = NbtCompound::new();
    data.push("duplicate", NbtTag::Int(3));
    data.push("duplicate", NbtTag::String("Schlüssel\0🙂".to_string()));
    data.push("float", NbtTag::Float(f32::from_bits(0x7FC0_0011)));
    data.push("double", NbtTag::Double(-0.0));
    let mut name = NbtCompound::new();
    name.push("text", NbtTag::String("Saved 🙂 name".to_string()));
    ComponentPatch::new(
        vec![
            ComponentValue::CustomData(NbtTag::Compound(data)),
            ComponentValue::MaxStackSize(64),
            ComponentValue::MaxDamage(12),
            ComponentValue::Damage(-3),
            ComponentValue::Unbreakable,
            ComponentValue::CustomName(NbtTag::Compound(name)),
            ComponentValue::Opaque(OpaqueComponent::new(20, vec![1, 2, 3]).expect("opaque")),
        ],
        vec![ComponentTypeId::new(51), ComponentTypeId::new(95)],
    )
}

#[test]
fn schema_three_preserves_all_slots_cursor_component_variants_and_float_bits() {
    let stone = ItemId::from_name("stone").expect("stone");
    let slots = std::array::from_fn(|index| {
        ItemStack::try_new(
            stone,
            u8::try_from(index + 1).expect("bounded count"),
            rich_patch(),
        )
        .expect("stack")
    });
    let carried = ItemStack::try_new(stone, 47, rich_patch()).expect("cursor");
    let inventory = PlayerInventory::from_persisted(GameMode::Survival, 8, slots, carried);
    let captured = PlayerData::capture(Vec3::new(-0.0, 64.25, -3.5), -0.0, 0.0, &inventory);
    let record = captured
        .to_record(GameMode::Survival)
        .expect("encode complete snapshot");
    let restored = PlayerData::from_record(&record).expect("strict decode");
    assert_eq!(restored.x.to_bits(), captured.x.to_bits());
    assert_eq!(restored.yaw.to_bits(), captured.yaw.to_bits());
    assert_eq!(restored.pitch.to_bits(), captured.pitch.to_bits());
    assert_eq!(restored.selected_slot, 8);
    // NaN-containing components must compare by their exact encoded bits.
    assert_eq!(
        restored
            .to_record(GameMode::Survival)
            .expect("reencode")
            .data(),
        record.data()
    );
    let inventory_after = restored.restore_inventory(GameMode::Survival);
    for index in 0..SLOT_COUNT {
        assert_eq!(
            ferrumc_items::encode_persisted_stack(inventory_after.slot(index).expect("slot"))
                .expect("stored item"),
            ferrumc_items::encode_persisted_stack(inventory.slot(index).expect("slot"))
                .expect("original item"),
            "slot {index} retains every component and its representation",
        );
    }
    assert_eq!(
        ferrumc_items::encode_persisted_stack(inventory_after.carried()).expect("stored cursor"),
        ferrumc_items::encode_persisted_stack(inventory.carried()).expect("original cursor"),
    );
}

#[test]
fn schema_three_rejects_every_truncation_and_invalid_envelope_before_restoring() {
    let empty = PlayerInventory::from_persisted(
        GameMode::Survival,
        0,
        std::array::from_fn(|_| ItemStack::empty()),
        ItemStack::empty(),
    );
    let data = PlayerData::capture(Vec3::new(1.0, 64.0, 2.0), 3.0, 4.0, &empty);
    let bytes = data
        .to_record(GameMode::Survival)
        .expect("valid empty snapshot")
        .data()
        .to_vec();
    for cut in 0..bytes.len() {
        assert_schema_three_corrupt(bytes[..cut].to_vec());
    }
    for (offset, replacement) in [
        (0, b"NOPE".as_slice()),
        (4, 6_u16.to_be_bytes().as_slice()),
        (38, [9].as_slice()),
        (39, 45_u16.to_be_bytes().as_slice()),
        (39, 47_u16.to_be_bytes().as_slice()),
        (41, 0_u32.to_be_bytes().as_slice()),
        (41, u32::MAX.to_be_bytes().as_slice()),
        (45, b"NOPE".as_slice()),
    ] {
        let mut corrupted = bytes.clone();
        corrupted[offset..offset + replacement.len()].copy_from_slice(replacement);
        assert_schema_three_corrupt(corrupted);
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_schema_three_corrupt(trailing);
}

#[test]
fn schema_three_rejects_unsafe_state_on_both_encode_and_decode() {
    let base = PlayerData::capture(Vec3::new(1.0, 64.0, 2.0), 3.0, 4.0, &sample_inventory());
    let bytes = base
        .to_record(GameMode::Creative)
        .expect("valid")
        .data()
        .to_vec();
    for position in [
        Vec3::new(f64::NAN, 64.0, 0.0),
        Vec3::new(0.0, f64::INFINITY, 0.0),
        Vec3::new(0.0, 64.0, 30_000_001.0),
    ] {
        let mut data = base.clone();
        data.x = position.x;
        data.y = position.y;
        data.z = position.z;
        assert!(data.to_record(GameMode::Creative).is_err());
        let mut malformed = bytes.clone();
        for (offset, value) in [(6, data.x), (14, data.y), (22, data.z)] {
            malformed[offset..offset + 8].copy_from_slice(&value.to_bits().to_be_bytes());
        }
        assert_schema_three_corrupt(malformed);
    }
    for (yaw, pitch) in [(f32::NAN, 0.0), (0.0, f32::NEG_INFINITY)] {
        let mut data = base.clone();
        data.yaw = yaw;
        data.pitch = pitch;
        assert!(data.to_record(GameMode::Creative).is_err());
        let mut malformed = bytes.clone();
        malformed[30..34].copy_from_slice(&yaw.to_bits().to_be_bytes());
        malformed[34..38].copy_from_slice(&pitch.to_bits().to_be_bytes());
        assert_schema_three_corrupt(malformed);
    }
    let mut data = base;
    data.selected_slot = 255;
    assert!(data.to_record(GameMode::Creative).is_err());
}

#[test]
fn schema_three_never_clamps_or_strips_a_corrupt_nested_stack() {
    let mut data = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &sample_inventory());
    let stack = ItemStack::try_new(
        ItemId::from_name("stone").expect("stone"),
        7,
        ComponentPatch::empty(),
    )
    .expect("stack");
    data.slots[0] = stack;
    let bytes = data
        .to_record(GameMode::Creative)
        .expect("valid")
        .data()
        .to_vec();
    // FCPS header41 + frame4 + FCIS signature/version/kind/name-size9 + name.
    let count_offset = 41 + 4 + 9 + "minecraft:stone".len();
    let mut corrupted = bytes;
    corrupted[count_offset] = 65;
    assert_schema_three_corrupt(corrupted);
    data.slots[0] = ItemStack::new(
        ItemId::from_name("stone").expect("stone"),
        NonZeroU8::new(65).expect("nonzero"),
        ComponentPatch::empty(),
    );
    assert!(data.to_record(GameMode::Creative).is_err());
    data.slots[0] = ItemStack::try_new(
        ItemId::from_name("stone").expect("stone"),
        7,
        ComponentPatch::new(
            vec![
                ferrumc_items::ComponentValue::Damage(1),
                ferrumc_items::ComponentValue::Damage(2),
            ],
            Vec::new(),
        ),
    )
    .expect("existing model permits constructing duplicate patches");
    assert!(
        data.to_record(GameMode::Creative).is_err(),
        "unsupported duplicate patches must fail rather than be stripped"
    );
}

fn large_stack(last_size: usize) -> ItemStack {
    use ferrumc_items::{ComponentValue, OpaqueComponent};
    let components = (6..38)
        .map(|id| {
            let len = if id == 37 { last_size } else { 8192 };
            ComponentValue::Opaque(OpaqueComponent::new(id, vec![7; len]).expect("bounded opaque"))
        })
        .collect();
    ItemStack::try_new(
        ItemId::from_name("stone").expect("stone"),
        1,
        ComponentPatch::new(components, Vec::new()),
    )
    .expect("large valid stack")
}

#[tokio::test]
async fn aggregate_limit_accepts_exact_boundary_and_failed_save_preserves_prior_record() {
    use ferrumc_storage::{InMemoryStore, MAX_PLAYER_DATA_LEN};
    let store = InMemoryStore::new();
    let player = PlayerId::offline("OverBudget");
    let original_data =
        PlayerData::capture(Vec3::new(8.0, 64.0, 8.0), 0.0, 0.0, &sample_inventory());
    let original = original_data
        .to_record(GameMode::Creative)
        .expect("prior good record");
    store
        .save_player(player, original.clone())
        .await
        .expect("prior save");
    let mut data = PlayerData::capture(
        Vec3::ZERO,
        0.0,
        0.0,
        &PlayerInventory::from_persisted(
            GameMode::Creative,
            0,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        ),
    );
    let huge = large_stack(8192);
    for index in 0..4 {
        data.slots[index] = huge.clone();
    }
    let total = 41
        + codec::VITALS_BYTES
        + 56 * 4
        + data
            .slots
            .iter()
            .chain(std::iter::once(&data.carried))
            .chain(data.table_inputs.iter())
            .map(|stack| {
                ferrumc_items::encode_persisted_stack(stack)
                    .expect("individually valid")
                    .len()
            })
            .sum::<usize>();
    assert!(total > MAX_PLAYER_DATA_LEN);
    let excess = total - MAX_PLAYER_DATA_LEN;
    assert!(excess < 8192);
    data.slots[3] = large_stack(8192 - excess);
    let exact = data
        .to_record(GameMode::Creative)
        .expect("exact aggregate boundary");
    assert_eq!(exact.data().len(), MAX_PLAYER_DATA_LEN);
    assert!(PlayerData::from_record(&exact).is_ok());
    data.slots[3] = large_stack(8192 - excess + 1);
    let error = data
        .to_record(GameMode::Creative)
        .expect_err("one extra byte refuses the complete snapshot");
    assert!(matches!(
        error,
        StorageError::RecordTooLarge {
            max: MAX_PLAYER_DATA_LEN,
            ..
        }
    ));
    assert_eq!(
        store
            .load_player(player)
            .await
            .expect("load prior")
            .expect("still present"),
        original,
        "failed encoding never submits a partial or lossy record"
    );
}

#[test]
fn crafting_table_inputs_persist_losslessly_and_schema_three_migrates() {
    let mut inventory = PlayerInventory::from_persisted(
        GameMode::Survival,
        0,
        std::array::from_fn(|_| ItemStack::empty()),
        ItemStack::empty(),
    );
    let patch = rich_patch();
    let inputs = std::array::from_fn(|index| {
        ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            (index + 1) as u8,
            patch.clone(),
        )
        .unwrap()
    });
    inventory.restore_table_inputs(inputs.clone());
    let captured = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &inventory);
    let record = captured.to_record(GameMode::Survival).unwrap();
    let restored = PlayerData::from_record(&record)
        .unwrap()
        .restore_inventory(GameMode::Survival);
    for (actual, expected) in restored.table_inputs().iter().zip(&inputs) {
        assert_eq!(
            ferrumc_items::encode_persisted_stack(actual).unwrap(),
            ferrumc_items::encode_persisted_stack(expected).unwrap()
        );
    }
    let empty = PlayerInventory::from_persisted(
        GameMode::Survival,
        0,
        std::array::from_fn(|_| ItemStack::empty()),
        ItemStack::empty(),
    );
    let record = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &empty)
        .to_record(GameMode::Survival)
        .unwrap();
    let mut old = record.data().to_vec();
    old.truncate(old.len() - 9 * 11 - codec::VITALS_BYTES);
    old[4..6].copy_from_slice(&3u16.to_be_bytes());
    let legacy = PlayerRecord::new(
        LOSSLESS_PLAYER_SCHEMA_VERSION,
        GameMode::Survival,
        old.clone(),
    )
    .unwrap();
    let migrated = PlayerData::from_record(&legacy).unwrap();
    assert!(migrated
        .table_inputs
        .iter()
        .all(|stack| stack.item().is_none()));
    assert_eq!(legacy.data(), old);
    assert_eq!(
        migrated
            .to_record(GameMode::Survival)
            .unwrap()
            .schema_version(),
        PLAYER_SCHEMA_VERSION
    );
    let mismatched = PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, old).unwrap();
    assert!(PlayerData::from_record(&mismatched).is_err());
}

fn snapshot_fixture() -> VitalsSnapshot {
    VitalsSnapshot::new(13.25, 17, 4.75, 3.5)
        .unwrap()
        .with_timers(71, 19, 9, 6.25, 13)
        .unwrap()
}

#[test]
fn schema_five_vitals_suffix_matches_independent_twenty_one_byte_grammar() {
    let mut data = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &sample_inventory());
    let before = data.to_record(GameMode::Survival).unwrap();
    data.set_vitals(snapshot_fixture()).unwrap();
    let record = data.to_record(GameMode::Survival).unwrap();
    assert_eq!(record.schema_version(), SchemaVersion::new(5));
    assert_eq!(&record.data()[4..6], &5u16.to_be_bytes());
    let suffix = &record.data()[record.data().len() - codec::VITALS_BYTES..];
    let golden: [u8; 21] = [
        0x41, 0x54, 0, 0, 17, 0x40, 0x98, 0, 0, 0x40, 0x60, 0, 0, 71, 19, 9, 0x40, 0xc8, 0, 0, 13,
    ];
    assert_eq!(suffix, golden);
    assert_eq!(
        &before.data()[..before.data().len() - 21],
        &record.data()[..record.data().len() - 21]
    );
    let restored = PlayerData::from_record(&record).unwrap();
    assert_eq!(restored.vitals(), snapshot_fixture());
    assert_eq!(
        restored.to_record(GameMode::Survival).unwrap().data(),
        record.data()
    );
}

#[test]
fn schemas_one_through_four_migrate_to_fresh_vitals_without_changing_item_bytes() {
    let fresh = PlayerVitals::new().snapshot();
    let mut legacy_json = schema_two_payload();
    legacy_json.as_object_mut().unwrap().remove("carried");
    for (schema, json) in [
        (LEGACY_PLAYER_SCHEMA_VERSION, legacy_json),
        (JSON_PLAYER_SCHEMA_VERSION, schema_two_payload()),
    ] {
        let bytes = serde_json::to_vec(&json).unwrap();
        let record = PlayerRecord::new(schema, GameMode::Survival, bytes.clone()).unwrap();
        assert_eq!(PlayerData::from_record(&record).unwrap().vitals(), fresh);
        assert_eq!(record.data(), bytes);
    }
    let mut data = PlayerData::capture(Vec3::new(2.0, 64.0, -8.0), 3.0, 4.0, &sample_inventory());
    data.slots[0] =
        ItemStack::try_new(ItemId::from_name("stone").unwrap(), 13, rich_patch()).unwrap();
    data.carried =
        ItemStack::try_new(ItemId::from_name("stone").unwrap(), 7, rich_patch()).unwrap();
    data.table_inputs[0] =
        ItemStack::try_new(ItemId::from_name("stone").unwrap(), 3, rich_patch()).unwrap();
    data.set_vitals(snapshot_fixture()).unwrap();
    let current = data.to_record(GameMode::Survival).unwrap();
    let mut four = current.data()[..current.data().len() - 21].to_vec();
    four[4..6].copy_from_slice(&4u16.to_be_bytes());
    let record_four = PlayerRecord::new(
        TABLE_PLAYER_SCHEMA_VERSION,
        GameMode::Survival,
        four.clone(),
    )
    .unwrap();
    let restored_four = PlayerData::from_record(&record_four).unwrap();
    assert_eq!(restored_four.vitals(), fresh);
    assert_eq!(record_four.data(), four);
    let rewritten = restored_four.to_record(GameMode::Survival).unwrap();
    assert_eq!(
        &rewritten.data()[6..rewritten.data().len() - 21],
        &four[6..]
    );
    // Independently find the first 47 frames rather than assuming table item size.
    let mut offset = 41;
    for _ in 0..47 {
        let length = u32::from_be_bytes(four[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4 + length;
    }
    let mut three = four[..offset].to_vec();
    three[4..6].copy_from_slice(&3u16.to_be_bytes());
    let record_three = PlayerRecord::new(
        LOSSLESS_PLAYER_SCHEMA_VERSION,
        GameMode::Survival,
        three.clone(),
    )
    .unwrap();
    let restored_three = PlayerData::from_record(&record_three).unwrap();
    assert_eq!(restored_three.vitals(), fresh);
    assert!(restored_three
        .table_inputs
        .iter()
        .all(|item| item.item().is_none()));
    assert_eq!(record_three.data(), three);
    let rewritten = restored_three.to_record(GameMode::Survival).unwrap();
    assert_eq!(&rewritten.data()[6..offset], &three[6..]);
}

#[test]
fn every_binary_outer_inner_schema_pair_requires_exact_agreement() {
    let data = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &sample_inventory());
    let five = data.to_record(GameMode::Survival).unwrap().data().to_vec();
    for outer in 3..=5 {
        for inner in 3..=5 {
            let mut payload = five.clone();
            if inner < 5 {
                payload.truncate(payload.len() - 21);
            }
            if inner == 3 {
                payload.truncate(payload.len() - 9 * 11);
            }
            payload[4..6].copy_from_slice(&(inner as u16).to_be_bytes());
            let record =
                PlayerRecord::new(SchemaVersion::new(outer), GameMode::Survival, payload).unwrap();
            assert_eq!(
                PlayerData::from_record(&record).is_ok(),
                outer == inner,
                "outer{outer} inner{inner}"
            );
        }
    }
}

#[test]
fn malformed_vitals_scalars_and_each_timer_are_classified_without_default_fallback() {
    let mut data = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &sample_inventory());
    data.set_vitals(snapshot_fixture()).unwrap();
    let valid = data.to_record(GameMode::Survival).unwrap().data().to_vec();
    let start = valid.len() - 21;
    for (offset, error) in [
        (0, VitalsError::InvalidHealth),
        (5, VitalsError::InvalidSaturation),
        (9, VitalsError::InvalidExhaustion),
        (16, VitalsError::InvalidTimers),
    ] {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
            let mut payload = valid.clone();
            payload[start + offset..start + offset + 4]
                .copy_from_slice(&invalid.to_bits().to_be_bytes());
            let record =
                PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload).unwrap();
            assert!(
                matches!(PlayerData::from_record(&record),Err(PlayerLoadError::Corrupt{
                source:PlayerPayloadError::InvalidVitals(actual)}) if actual==error)
            );
        }
    }
    for (offset, value, error) in [
        (4, 21, VitalsError::InvalidFood),
        (13, 80, VitalsError::InvalidTimers),
        (14, 21, VitalsError::InvalidTimers),
        (15, 11, VitalsError::InvalidTimers),
        (20, 20, VitalsError::InvalidTimers),
    ] {
        let mut payload = valid.clone();
        payload[start + offset] = value;
        let record = PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload).unwrap();
        assert!(
            matches!(PlayerData::from_record(&record),Err(PlayerLoadError::Corrupt{
            source:PlayerPayloadError::InvalidVitals(actual)}) if actual==error)
        );
    }
    for (offset, value, error) in [
        (0, 20.5, VitalsError::InvalidHealth),
        (5, 17.5, VitalsError::InvalidSaturation),
        (9, 40.5, VitalsError::InvalidExhaustion),
    ] {
        let mut payload = valid.clone();
        payload[start + offset..start + offset + 4]
            .copy_from_slice(&f32::to_bits(value).to_be_bytes());
        let record = PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload).unwrap();
        assert!(
            matches!(PlayerData::from_record(&record),Err(PlayerLoadError::Corrupt{
            source:PlayerPayloadError::InvalidVitals(actual)}) if actual==error)
        );
    }
}

#[test]
fn schema_five_accepts_boundary_vitals_and_preserves_all_signed_zero_bits() {
    let mut data = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &sample_inventory());
    for snapshot in [
        VitalsSnapshot::new(20.0, 20, 20.0, 40.0)
            .unwrap()
            .with_timers(79, 20, 10, f32::MAX, 19)
            .unwrap(),
        VitalsSnapshot::new(-0.0, 0, -0.0, -0.0)
            .unwrap()
            .with_timers(0, 0, 0, -0.0, 0)
            .unwrap(),
    ] {
        data.set_vitals(snapshot).unwrap();
        let record = data.to_record(GameMode::Survival).unwrap();
        let restored = PlayerData::from_record(&record).unwrap();
        let actual = restored.vitals();
        for (a, b) in [
            (actual.health(), snapshot.health()),
            (actual.saturation(), snapshot.saturation()),
            (actual.exhaustion(), snapshot.exhaustion()),
            (actual.last_hurt(), snapshot.last_hurt()),
        ] {
            assert_eq!(a.to_bits(), b.to_bits());
        }
        assert_eq!(
            restored.to_record(GameMode::Survival).unwrap().data(),
            record.data()
        );
    }
}

#[tokio::test]
async fn saved_vitals_restore_midcycle_and_repeat_exact_simulation_after_reload() {
    use ferrumc_sim::{DamageRequest, Difficulty, VitalsTickPolicy};
    use ferrumc_storage::{InMemoryStore, PlayerStore};
    let store = InMemoryStore::new();
    let id = PlayerId::offline("VitalsRestart");
    let policy = VitalsTickPolicy::new(Difficulty::Normal, true);
    let mut live = PlayerVitals::from_snapshot(snapshot_fixture()).unwrap();
    for _ in 0..7 {
        live.tick(policy);
    }
    let mut data = PlayerData::capture(Vec3::new(8.0, 64.0, -8.0), 1.0, 2.0, &sample_inventory());
    data.set_vitals(live.snapshot()).unwrap();
    store
        .save_player(id, data.to_record(GameMode::Survival).unwrap())
        .await
        .unwrap();
    let PlayerLoad::Restored { data, .. } = load_player_for_join(&store, id).await.unwrap() else {
        panic!("savedplayer missing");
    };
    let mut restored = PlayerVitals::from_snapshot(data.vitals()).unwrap();
    for tick in 0..240 {
        if tick % 37 == 0 {
            let request = DamageRequest::new(1.0, 0.0, 0.0).unwrap();
            assert_eq!(live.hurt(request).unwrap(), restored.hurt(request).unwrap());
        }
        assert_eq!(live.tick(policy), restored.tick(policy));
        assert_eq!(live.snapshot(), restored.snapshot());
    }
}

#[tokio::test]
async fn corrupt_vitals_rejoin_refuses_and_keeps_the_exact_stored_record() {
    use ferrumc_storage::{InMemoryStore, PlayerStore};
    let store = InMemoryStore::new();
    let id = PlayerId::offline("VitalsCorrupt");
    let data = PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &sample_inventory());
    let mut payload = data.to_record(GameMode::Survival).unwrap().data().to_vec();
    let start = payload.len() - 21;
    payload[start..start + 4].copy_from_slice(&f32::NAN.to_bits().to_be_bytes());
    let record = PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload).unwrap();
    store.save_player(id, record.clone()).await.unwrap();
    assert!(matches!(
        load_player_for_join(&store, id).await,
        Err(PlayerLoadError::Corrupt {
            source: PlayerPayloadError::InvalidVitals(VitalsError::InvalidHealth)
        })
    ));
    assert_eq!(store.load_player(id).await.unwrap().unwrap(), record);
}
