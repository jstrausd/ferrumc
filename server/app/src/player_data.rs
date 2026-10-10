//! The persisted per-player snapshot and its codec.
//!
//! [`PlayerData`] is the connection-owned view of everything that must survive a
//! player leaving and rejoining — or the server restarting: absolute
//! position, look (yaw + pitch), the selected hotbar index, and the 46-slot
//! inventory layout and player-owned carried item. It is serialized into the opaque [`PlayerRecord`] payload
//! (`PlayerRecord.data`) while the player's [`GameMode`] rides the record's own
//! typed field; storage never interprets either, so the storage layer stays
//! byte-oriented and free of any inventory/connection types.
//!
//! # Versioning and robustness
//!
//! New records use strict schema 3, with a bounded binary player envelope and
//! independent [`ferrumc_items`] storage records for all 46 slots and the cursor.
//! Counts, patches, typed/opaque representations and NBT are preserved without
//! creative-input normalization. The aggregate encoded payload is at most 1 MiB;
//! encoding retains at most one additional bounded item buffer at a time.
//!
//! Schema 1 and 2 JSON records explicitly migrate in memory with their original
//! slot-normalization policy. Schema 2 still requires the exact layout, a valid
//! cursor and known fields. Loading never rewrites a record: only a later
//! successful leave-save writes schema 3. Unsupported versions and malformed
//! payloads are classified; only a confirmed storage miss permits fresh defaults.

mod codec;

use std::error::Error;
use std::fmt;
use std::num::NonZeroU8;

use serde::{Deserialize, Serialize};

use ferrumc_core::{GameMode, PlayerId, ServerError};
use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
use ferrumc_math::Vec3;
use ferrumc_storage::{PlayerRecord, PlayerStore, SchemaVersion, StorageError};

use crate::inventory::{PlayerInventory, SLOT_COUNT};
use codec::PlayerPayloadError;

/// The schema version stamped on every [`PlayerRecord`] this module writes.
///
/// Schemas 1 and 2 migrate explicitly; other versions are incompatible.
pub(crate) const PLAYER_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(3);
const LEGACY_PLAYER_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1);
const JSON_PLAYER_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(2);

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
    /// An accepted schema's payload is malformed.
    Corrupt {
        /// The classified bounded payload decoder failure.
        source: PlayerPayloadError,
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
    #[cfg(test)]
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
/// New schema-3 saves use the independent item codec instead.
#[derive(Debug, Clone, PartialEq)]
struct PersistedCarried(ItemStack);

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
        // Schema 1 pads/truncates and normalizes; new records do neither.
        let slots = std::array::from_fn(|index| {
            data.slots
                .get(index)
                .copied()
                .map_or_else(ItemStack::empty, PersistedSlot::restore)
        });
        Self {
            x: data.x,
            y: data.y,
            z: data.z,
            yaw: data.yaw,
            pitch: data.pitch,
            selected_slot: normalize_legacy_selection(data.selected_slot),
            slots: Box::new(slots),
            carried: ItemStack::empty(),
        }
    }
}

/// The previous strict JSON envelope, accepted only as schema 2.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonPlayerData {
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    selected_slot: u8,
    #[serde(deserialize_with = "deserialize_inventory_slots")]
    slots: Vec<PersistedSlot>,
    carried: PersistedCarried,
}

impl From<JsonPlayerData> for PlayerData {
    fn from(data: JsonPlayerData) -> Self {
        Self {
            x: data.x,
            y: data.y,
            z: data.z,
            yaw: data.yaw,
            pitch: data.pitch,
            selected_slot: normalize_legacy_selection(data.selected_slot),
            // Keep schema 2's historical slot policy while its cursor is strict.
            slots: Box::new(std::array::from_fn(|index| data.slots[index].restore())),
            carried: data.carried.0,
        }
    }
}

fn normalize_legacy_selection(selected: u8) -> u8 {
    if selected < 9 {
        selected
    } else {
        0
    }
}

/// A fixed-layout snapshot of one player's persisted state.
///
/// The game mode lives in [`PlayerRecord::game_mode`]. Slots and carried item
/// retain their complete supported authoritative representation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerData {
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    selected_slot: u8,
    // Keep the fixed shape without copying its roughly 3 KiB through each
    // admission/save future. Component trees remain owned by the snapshot.
    slots: Box<[ItemStack; SLOT_COUNT]>,
    carried: ItemStack,
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
        let slots = std::array::from_fn(|index| {
            inventory
                .slot(index)
                .cloned()
                .unwrap_or_else(ItemStack::empty)
        });
        Self {
            x: position.x,
            y: position.y,
            z: position.z,
            yaw,
            pitch,
            selected_slot: inventory.selected(),
            slots: Box::new(slots),
            carried: inventory.carried().clone(),
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
    /// Legacy normalization has already happened during migration. Schema 3
    /// restores every stack exactly, including the carried component patch.
    pub(crate) fn restore_inventory(&self, game_mode: GameMode) -> PlayerInventory {
        PlayerInventory::from_persisted(
            game_mode,
            self.selected_slot,
            *self.slots.clone(),
            self.carried.clone(),
        )
    }

    /// Encodes the complete snapshot into one bounded schema-3 record.
    ///
    /// # Errors
    /// Rejects an unsafe state, invalid item/component, or aggregate encoded
    /// payload above 1 MiB before growing the parent buffer past that bound.
    /// Nothing is stripped or clamped and no store mutation happens here.
    pub(crate) fn to_record(&self, game_mode: GameMode) -> Result<PlayerRecord, StorageError> {
        let data = codec::encode(self).map_err(|error| match error {
            PlayerPayloadError::RecordTooLarge { len, max } => {
                StorageError::RecordTooLarge { len, max }
            }
            other => StorageError::backend(format!("encoding player snapshot: {other}")),
        })?;
        PlayerRecord::new(PLAYER_SCHEMA_VERSION, game_mode, data)
    }

    /// Decodes a [`PlayerData`] from a loaded [`PlayerRecord`].
    ///
    /// A stale schema and a malformed current-schema payload remain distinct,
    /// classified errors. Neither is absence and neither authorizes fresh-player
    /// defaults.
    pub(crate) fn from_record(record: &PlayerRecord) -> Result<Self, PlayerLoadError> {
        let result = if record.schema_version() == LEGACY_PLAYER_SCHEMA_VERSION {
            serde_json::from_slice::<LegacyPlayerData>(record.data())
                .map(Self::from)
                .map_err(PlayerPayloadError::Json)
        } else if record.schema_version() == JSON_PLAYER_SCHEMA_VERSION {
            serde_json::from_slice::<JsonPlayerData>(record.data())
                .map(Self::from)
                .map_err(PlayerPayloadError::Json)
        } else if record.schema_version() == PLAYER_SCHEMA_VERSION {
            codec::decode(record.data())
        } else {
            return Err(PlayerLoadError::Incompatible {
                found: record.schema_version(),
                expected: PLAYER_SCHEMA_VERSION,
            });
        };
        result.map_err(|source| PlayerLoadError::Corrupt { source })
    }
}

#[cfg(test)]
mod tests;
