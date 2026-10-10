//! The persisted per-player snapshot and its codec.
//!
//! [`PlayerData`] is the connection-owned view of everything that must survive a
//! player leaving and rejoining — or the server restarting: spawn-relative
//! position, look (yaw + pitch), the selected hotbar index, and the 46-slot
//! inventory layout and player-owned carried item. It is serialized into the opaque [`PlayerRecord`] payload
//! (`PlayerRecord.data`) while the player's [`GameMode`] rides the record's own
//! typed field; storage never interprets either, so the storage layer stays
//! byte-oriented and free of any inventory/connection types.
//!
//! # Versioning and robustness
//!
//! New records use schema 2 with exactly 46 slots and a required, validated
//! carried item. Schema 1 explicitly migrates its layout with an empty cursor;
//! its old slot normalization policy remains intact. Migration happens in memory
//! and is written only by a later successful leave-save. Other versions and
//! malformed payloads are rejected with a classified [`PlayerLoadError`].
//! Only a confirmed missing storage record may
//! use fresh-player defaults, so an unreadable record can never be overwritten by
//! a later default-state leave save.
//!
//! # Inventory fidelity
//!
//! Each slot and the carried item persist their registry item id and stack count, which restores the
//! full inventory *layout* (what is where, and how many). Data components
//! (custom name, damage, NBT, …) are intentionally not persisted in this version:
//! the trusted slot encoder and the untrusted slot decoder use different on-wire
//! framing, so they do not round-trip, and a lossy component codec would be worse
//! than none. Live exchanges retain components, but a restart does not. Stack
//! counts are validated against the registry maximum, so a component-supplied
//! custom stack limit is not supported by this storage contract. Component persistence
//! remains a follow-up that would bump
//! [`PLAYER_SCHEMA_VERSION`].

use std::error::Error;
use std::fmt;
use std::num::NonZeroU8;

use serde::{Deserialize, Serialize};

use ferrumc_core::{GameMode, PlayerId, ServerError};
use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
use ferrumc_math::Vec3;
use ferrumc_storage::{PlayerRecord, PlayerStore, SchemaVersion, StorageError};

use crate::inventory::{PlayerInventory, SLOT_COUNT};

/// The schema version stamped on every [`PlayerRecord`] this module writes.
///
/// Schema 1 migrates explicitly; other versions are rejected as incompatible.
pub(crate) const PLAYER_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(2);
const LEGACY_PLAYER_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1);

/// Largest magnitude accepted for a player coordinate.
///
/// This mirrors the simulation's move sanity boundary. Both restoration and live
/// serverbound movement apply it before shard selection, connection-side state,
/// or float-to-integer chunk conversion.
const MAX_PLAYER_POSITION_MAGNITUDE: f64 = 3.0e7;

/// Returns whether a player position is safe to route and admit.
pub(crate) fn is_valid_player_position(position: Vec3) -> bool {
    let valid_axis = |coordinate: f64| {
        coordinate.is_finite() && coordinate.abs() <= MAX_PLAYER_POSITION_MAGNITUDE
    };
    valid_axis(position.x) && valid_axis(position.y) && valid_axis(position.z)
}

/// A classified failure to load a player's persisted state for admission.
#[derive(Debug)]
pub(crate) enum PlayerLoadError {
    /// The storage operation itself failed.
    BackendError {
        /// The classified storage-layer error.
        source: ServerError,
    },
    /// The record uses a schema this binary cannot interpret.
    Incompatible {
        /// Schema version carried by the record.
        found: SchemaVersion,
        /// Schema version this binary accepts.
        expected: SchemaVersion,
    },
    /// The current-schema payload is malformed.
    Corrupt {
        /// The bounded JSON decoder failure.
        source: serde_json::Error,
    },
}

impl fmt::Display for PlayerLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendError { source } => {
                write!(formatter, "player storage load failed: {source}")
            }
            Self::Incompatible { found, expected } => write!(
                formatter,
                "incompatible player schema {found}; expected {expected}"
            ),
            Self::Corrupt { source } => {
                write!(formatter, "corrupt player record payload: {source}")
            }
        }
    }
}

impl Error for PlayerLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BackendError { source } => Some(source),
            Self::Corrupt { source } => Some(source),
            Self::Incompatible { .. } => None,
        }
    }
}

/// The only two successful player-load outcomes.
///
/// `NotFound` is a confirmed store miss and may start fresh. `Restored` carries
/// a record whose schema and payload were both validated.
#[derive(Debug)]
pub(crate) enum PlayerLoad {
    /// No record exists for this player.
    NotFound,
    /// A current-schema record decoded successfully.
    Restored {
        /// The decoded app-owned state.
        data: PlayerData,
        /// The typed game mode stored beside the payload.
        game_mode: GameMode,
    },
}

/// Loads and validates the one player record used for Play admission.
///
/// Only the storage trait's `Ok(None)` result becomes [`PlayerLoad::NotFound`].
/// Every error, including a backend that incorrectly reports a not-found error
/// instead of `Ok(None)`, remains a [`PlayerLoadError::BackendError`].
pub(crate) async fn load_player_for_join(
    store: &dyn PlayerStore,
    player: PlayerId,
) -> Result<PlayerLoad, PlayerLoadError> {
    match store.load_player(player).await {
        Ok(Some(record)) => {
            let game_mode = record.game_mode();
            let data = PlayerData::from_record(&record)?;
            Ok(PlayerLoad::Restored { data, game_mode })
        }
        Ok(None) => Ok(PlayerLoad::NotFound),
        Err(source) => Err(PlayerLoadError::BackendError { source }),
    }
}

/// One persisted inventory slot: a registry item id and a stack count.
///
/// An empty slot is `item_id == 0` (no item) with `count == 0`. On restore an
/// unknown id, a zero count, or a count above the item's maximum is normalized
/// to a clean stack (or an empty slot), so a corrupt entry never produces an
/// invalid [`ItemStack`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedSlot {
    /// The registry item id, or `0` for an empty slot.
    item_id: i32,
    /// The stack count (`0` for an empty slot).
    count: u8,
}

impl PersistedSlot {
    /// Captures one [`ItemStack`] as its id + count, collapsing an empty stack to
    /// the empty-slot encoding.
    fn capture(stack: &ItemStack) -> Self {
        match stack.item() {
            Some(item) => Self {
                item_id: item.id(),
                count: stack.count(),
            },
            None => Self {
                item_id: 0,
                count: 0,
            },
        }
    }

    /// Rebuilds an [`ItemStack`], normalizing any out-of-range data to a valid
    /// stack or the empty slot — never panicking on a hostile persisted value.
    fn restore(self) -> ItemStack {
        let Some(item) = ItemId::new(self.item_id) else {
            return ItemStack::empty();
        };
        // Clamp the count into `1..=max_stack`; a zero or over-cap count from a
        // corrupt blob must not produce an inconsistent stack.
        let clamped = self.count.min(item.max_stack());
        let Some(count) = NonZeroU8::new(clamped) else {
            return ItemStack::empty();
        };
        ItemStack::new(item, count, ComponentPatch::empty())
    }
}

/// Schema-2 carried item, strictly validated rather than silently normalized.
/// Its storage contract retains item identity/count, matching inventory slots;
/// component fidelity across restart remains a separate persistence milestone.
#[derive(Debug, Clone, PartialEq)]
struct PersistedCarried(ItemStack);

impl Serialize for PersistedCarried {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if !self.0.is_valid() || self.0.item().is_some_and(ItemId::is_air) {
            return Err(serde::ser::Error::custom("invalid carried stack"));
        }
        PersistedSlot::capture(&self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PersistedCarried {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let slot = PersistedSlot::deserialize(deserializer)?;
        if slot.item_id == 0 && slot.count == 0 {
            return Ok(Self(ItemStack::empty()));
        }
        let item = ItemId::new(slot.item_id)
            .filter(|item| !item.is_air())
            .ok_or_else(|| serde::de::Error::custom("unknown or air carried item ID"))?;
        let stack = ItemStack::try_new(item, slot.count, ComponentPatch::empty())
            .map_err(serde::de::Error::custom)?;
        Ok(Self(stack))
    }
}

/// The original JSON layout, read only for explicit schema-1 migration.
#[derive(Deserialize)]
struct LegacyPlayerData {
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    selected_slot: u8,
    slots: Vec<PersistedSlot>,
}

impl From<LegacyPlayerData> for PlayerData {
    fn from(data: LegacyPlayerData) -> Self {
        // Retain the schema-1 policy: pad short layouts, ignore slots beyond the
        // known window, and normalize malformed entries when restoring slots.
        let slots = (0..SLOT_COUNT)
            .map(|index| {
                data.slots.get(index).copied().unwrap_or(PersistedSlot {
                    item_id: 0,
                    count: 0,
                })
            })
            .collect();
        Self {
            x: data.x,
            y: data.y,
            z: data.z,
            yaw: data.yaw,
            pitch: data.pitch,
            selected_slot: data.selected_slot,
            slots,
            carried: PersistedCarried(ItemStack::empty()),
        }
    }
}

/// A versioned, serializable snapshot of one player's persisted state.
///
/// The [`GameMode`] is *not* held here: it lives in the typed
/// [`PlayerRecord::game_mode`] field. Construct one from live connection state
/// with [`PlayerData::capture`] and turn it back into a seeded
/// [`PlayerInventory`] with [`PlayerData::restore_inventory`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlayerData {
    /// Player position, x component (world space).
    x: f64,
    /// Player position, y component (world space).
    y: f64,
    /// Player position, z component (world space).
    z: f64,
    /// Look yaw, in degrees.
    yaw: f32,
    /// Look pitch, in degrees.
    pitch: f32,
    /// The selected hotbar index (`0..=8`).
    selected_slot: u8,
    /// The 46 inventory slots, in window-0 order.
    #[serde(deserialize_with = "deserialize_inventory_slots")]
    slots: Vec<PersistedSlot>,
    /// Required schema-2 carried item; missing data is corruption, not empty.
    carried: PersistedCarried,
}

/// Decodes only the schema-2 window layout, rejecting excess entries before
/// they can grow an allocation or be silently discarded during restoration.
fn deserialize_inventory_slots<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<PersistedSlot>, D::Error> {
    struct SlotsVisitor;
    impl<'de> serde::de::Visitor<'de> for SlotsVisitor {
        type Value = Vec<PersistedSlot>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "exactly {SLOT_COUNT} inventory slots")
        }

        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            if sequence.size_hint().is_some_and(|count| count > SLOT_COUNT) {
                return Err(serde::de::Error::custom("too many inventory slots"));
            }
            let mut slots = Vec::with_capacity(SLOT_COUNT);
            while let Some(slot) = sequence.next_element()? {
                if slots.len() == SLOT_COUNT {
                    return Err(serde::de::Error::custom("too many inventory slots"));
                }
                slots.push(slot);
            }
            if slots.len() != SLOT_COUNT {
                return Err(serde::de::Error::custom(
                    "inventory must contain exactly 46 slots",
                ));
            }
            Ok(slots)
        }
    }
    deserializer.deserialize_seq(SlotsVisitor)
}

impl PlayerData {
    /// Captures the live connection state into a persistable snapshot.
    ///
    /// `position`, `yaw`, and `pitch` are the connection-tracked look/position;
    /// the selected slot and every inventory slot are read from `inventory`.
    pub(crate) fn capture(
        position: Vec3,
        yaw: f32,
        pitch: f32,
        inventory: &PlayerInventory,
    ) -> Self {
        let slots = (0..SLOT_COUNT)
            .map(|index| {
                inventory.slot(index).map_or(
                    PersistedSlot {
                        item_id: 0,
                        count: 0,
                    },
                    PersistedSlot::capture,
                )
            })
            .collect();
        Self {
            x: position.x,
            y: position.y,
            z: position.z,
            yaw,
            pitch,
            selected_slot: inventory.selected(),
            slots,
            carried: PersistedCarried(inventory.carried().clone()),
        }
    }

    /// The restored player position as a typed [`Vec3`].
    pub(crate) fn position(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// The restored look yaw, in degrees.
    pub(crate) fn yaw(&self) -> f32 {
        self.yaw
    }

    /// The restored look pitch, in degrees.
    pub(crate) fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Rebuilds the authoritative [`PlayerInventory`] for `game_mode` from the
    /// persisted slots and selected index.
    ///
    /// Missing slots (a short or empty list from an older or corrupt blob) and
    /// out-of-range entries restore as empty, so the result is always a valid
    /// 46-slot window.
    pub(crate) fn restore_inventory(&self, game_mode: GameMode) -> PlayerInventory {
        let mut slots: [ItemStack; SLOT_COUNT] = std::array::from_fn(|_| ItemStack::empty());
        for (index, slot) in self.slots.iter().take(SLOT_COUNT).enumerate() {
            slots[index] = slot.restore();
        }
        PlayerInventory::from_persisted(
            game_mode,
            self.selected_slot,
            slots,
            self.carried.0.clone(),
        )
    }

    /// Encodes this snapshot, plus the typed `game_mode`, into a [`PlayerRecord`].
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the payload exceeds [`MAX_PLAYER_DATA_LEN`]
    /// (unreachable for a 46-slot inventory) or — never in practice — fails to
    /// serialize.
    ///
    /// [`MAX_PLAYER_DATA_LEN`]: ferrumc_storage::MAX_PLAYER_DATA_LEN
    pub(crate) fn to_record(&self, game_mode: GameMode) -> Result<PlayerRecord, StorageError> {
        let data = serde_json::to_vec(self)
            .map_err(|err| StorageError::backend(format!("serializing player data: {err}")))?;
        PlayerRecord::new(PLAYER_SCHEMA_VERSION, game_mode, data)
    }

    /// Decodes a [`PlayerData`] from a loaded [`PlayerRecord`].
    ///
    /// A stale schema and a malformed current-schema payload remain distinct,
    /// classified errors. Neither is absence and neither authorizes fresh-player
    /// defaults.
    pub(crate) fn from_record(record: &PlayerRecord) -> Result<Self, PlayerLoadError> {
        if record.schema_version() == LEGACY_PLAYER_SCHEMA_VERSION {
            return serde_json::from_slice::<LegacyPlayerData>(record.data())
                .map(Self::from)
                .map_err(|source| PlayerLoadError::Corrupt { source });
        }
        if record.schema_version() != PLAYER_SCHEMA_VERSION {
            return Err(PlayerLoadError::Incompatible {
                found: record.schema_version(),
                expected: PLAYER_SCHEMA_VERSION,
            });
        }
        serde_json::from_slice(record.data()).map_err(|source| PlayerLoadError::Corrupt { source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_two_payload() -> serde_json::Value {
        serde_json::to_value(PlayerData::capture(
            Vec3::new(4.0, 65.0, -9.0),
            23.0,
            -7.0,
            &sample_inventory(),
        ))
        .expect("valid current payload")
    }

    fn assert_corrupt(payload: Vec<u8>) {
        let record = PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload)
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
        let current: serde_json::Value = serde_json::from_slice(rewritten.data()).expect("JSON");
        assert_eq!(
            current["carried"],
            serde_json::json!({"item_id":0,"count":0})
        );
        assert_eq!(
            current["slots"].as_array().expect("slots").len(),
            SLOT_COUNT
        );
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
    fn component_persistence_is_explicitly_limited_to_id_and_count() {
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
        assert!(
            restored.carried().components().is_empty(),
            "S02 component codec is not implemented"
        );
    }

    #[test]
    fn schema_two_rejects_missing_invalid_unknown_or_duplicate_carried_data() {
        let valid = schema_two_payload();
        assert!(PlayerData::from_record(
            &PlayerRecord::new(
                PLAYER_SCHEMA_VERSION,
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
            br#"{"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
                .to_vec(),
            br#"{"x":null,"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
                .to_vec(),
            br#"{"x":"NaN","y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
                .to_vec(),
            br#"{"x":[],"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
                .to_vec(),
            br#"{"x":{},"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
                .to_vec(),
            br#"{"x":1e400,"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}"#
                .to_vec(),
            br#"{"x":0.0,"y":64.0,"z":0.0,"yaw":0.0,"pitch":0.0,"selected_slot":0,"slots":[]}trailing"#
                .to_vec(),
        ];
        for payload in malformed {
            let record =
                PlayerRecord::new(PLAYER_SCHEMA_VERSION, GameMode::Survival, payload.clone())
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
        let bytes = serde_json::to_vec(&data).expect("encodes");
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
        let data = PlayerData {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            selected_slot: 200, // out of range
            carried: PersistedCarried(ItemStack::empty()),
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
        };
        let inv = data.restore_inventory(GameMode::Creative);
        assert!(inv.slot(0).expect("slot 0").item().is_none());
        assert!(inv.slot(1).expect("slot 1").item().is_none());
        // Out-of-range selected index clamps to a valid hotbar slot.
        assert!(inv.selected() <= 8);
    }
}
