//! Bounded schema-3 player envelope, independent of Minecraft network framing.
use std::error::Error;
use std::fmt;

use ferrumc_codec::{BoundedReader, CodecError};
use ferrumc_items::{
    decode_persisted_stack, encode_persisted_stack, ItemPersistenceError, ItemStack,
    MAX_PERSISTED_STACK_BYTES,
};
use ferrumc_storage::MAX_PLAYER_DATA_LEN;

use super::{is_valid_player_position, PlayerData};
use crate::inventory::SLOT_COUNT;

const MAGIC: &[u8; 4] = b"FCPS";
const VERSION: u16 = 3;
const HEADER_BYTES: usize = 4 + 2 + 3 * 8 + 2 * 4 + 1 + 2;
const FRAME_BYTES: usize = 4;

/// A specific malformed player envelope or one nested item-storage failure.
#[derive(Debug)]
pub(crate) enum PlayerPayloadError {
    /// Legacy JSON failed its version-specific decoder.
    Json(serde_json::Error),
    /// A bounded field read failed, including truncation and trailing bytes.
    Codec(CodecError),
    /// One nested authoritative stack failed strict storage validation.
    Item(ItemPersistenceError),
    /// The envelope signature did not identify player storage.
    InvalidMagic,
    /// The inner envelope version disagreed with the schema-3 contract.
    UnsupportedVersion(u16),
    /// The envelope did not contain exactly the fixed player-window layout.
    WrongSlotCount(u16),
    /// Coordinates were nonfinite or outside the admission bounds.
    InvalidPosition,
    /// A yaw or pitch was not finite.
    InvalidLook,
    /// The selected hotbar slot was outside `0..=8`.
    InvalidSelected(u8),
    /// The aggregate encoded record exceeded its storage ceiling.
    RecordTooLarge {
        /// Declared or encoded number of bytes.
        len: usize,
        /// The encoded record ceiling.
        max: usize,
    },
    /// One framed nested item exceeded its independent storage ceiling.
    StackTooLarge(usize),
}

impl fmt::Display for PlayerPayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "legacy player JSON: {error}"),
            Self::Codec(error) => write!(f, "player envelope field: {error}"),
            Self::Item(error) => write!(f, "player inventory item: {error}"),
            Self::InvalidMagic => write!(f, "invalid player persistence signature"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported player payload version {version}")
            }
            Self::WrongSlotCount(count) => write!(
                f,
                "player layout contains {count} slots; expected {SLOT_COUNT}"
            ),
            Self::InvalidPosition => write!(f, "unsafe persisted player position"),
            Self::InvalidLook => write!(f, "nonfinite persisted player look"),
            Self::InvalidSelected(selected) => write!(f, "invalid selected hotbar slot {selected}"),
            Self::RecordTooLarge { len, max } => {
                write!(f, "player snapshot has {len} bytes; maximum is {max}")
            }
            Self::StackTooLarge(len) => write!(
                f,
                "player item frame has {len} bytes; maximum is {MAX_PERSISTED_STACK_BYTES}"
            ),
        }
    }
}

impl Error for PlayerPayloadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(source) => Some(source),
            Self::Codec(source) => Some(source),
            Self::Item(source) => Some(source),
            _ => None,
        }
    }
}

impl From<CodecError> for PlayerPayloadError {
    fn from(source: CodecError) -> Self {
        Self::Codec(source)
    }
}

impl From<ItemPersistenceError> for PlayerPayloadError {
    fn from(source: ItemPersistenceError) -> Self {
        Self::Item(source)
    }
}

fn validate_state(data: &PlayerData) -> Result<(), PlayerPayloadError> {
    if !is_valid_player_position(data.position()) {
        return Err(PlayerPayloadError::InvalidPosition);
    }
    if !data.yaw.is_finite() || !data.pitch.is_finite() {
        return Err(PlayerPayloadError::InvalidLook);
    }
    if data.selected_slot >= 9 {
        return Err(PlayerPayloadError::InvalidSelected(data.selected_slot));
    }
    Ok(())
}

/// Encodes at most 1 MiB, retaining one additional item buffer at a time.
/// Each append is approved before reserving parent capacity; exact reservation
/// avoids geometric growth beyond the encoded ceiling. Caps bound encoded bytes,
/// not the heap footprint of the already-owned authoritative NBT trees.
pub(super) fn encode(data: &PlayerData) -> Result<Vec<u8>, PlayerPayloadError> {
    validate_state(data)?;
    let mut out = Vec::with_capacity(HEADER_BYTES);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_be_bytes());
    for coordinate in [data.x, data.y, data.z] {
        out.extend_from_slice(&coordinate.to_bits().to_be_bytes());
    }
    for look in [data.yaw, data.pitch] {
        out.extend_from_slice(&look.to_bits().to_be_bytes());
    }
    out.push(data.selected_slot);
    // The fixed array is the authoritative shape; no caller-controlled count.
    out.extend_from_slice(&46_u16.to_be_bytes());
    for stack in data.slots.iter().chain(std::iter::once(&data.carried)) {
        let encoded = encode_persisted_stack(stack)?;
        let next = out.len() + FRAME_BYTES + encoded.len();
        if next > MAX_PLAYER_DATA_LEN {
            return Err(PlayerPayloadError::RecordTooLarge {
                len: next,
                max: MAX_PLAYER_DATA_LEN,
            });
        }
        out.reserve_exact(FRAME_BYTES + encoded.len());
        let size = u32::try_from(encoded.len())
            .map_err(|_| PlayerPayloadError::StackTooLarge(encoded.len()))?;
        out.extend_from_slice(&size.to_be_bytes());
        out.extend_from_slice(&encoded);
    }
    Ok(out)
}

fn read_stack(reader: &mut BoundedReader<'_>) -> Result<ItemStack, PlayerPayloadError> {
    let raw_size = reader.read_u32()?;
    let size =
        usize::try_from(raw_size).map_err(|_| PlayerPayloadError::StackTooLarge(usize::MAX))?;
    if size > MAX_PERSISTED_STACK_BYTES {
        return Err(PlayerPayloadError::StackTooLarge(size));
    }
    Ok(decode_persisted_stack(reader.read_bytes(size)?)?)
}

/// Decodes exactly the bounded schema-3 envelope and all 47 item frames.
/// Fixed inventory storage is initialized without untrusted list allocation;
/// every nested frame is checked before its item decoder can copy components.
pub(super) fn decode(bytes: &[u8]) -> Result<PlayerData, PlayerPayloadError> {
    if bytes.len() > MAX_PLAYER_DATA_LEN {
        return Err(PlayerPayloadError::RecordTooLarge {
            len: bytes.len(),
            max: MAX_PLAYER_DATA_LEN,
        });
    }
    let mut reader = BoundedReader::new(bytes);
    if reader.read_bytes(MAGIC.len())? != MAGIC {
        return Err(PlayerPayloadError::InvalidMagic);
    }
    let version = reader.read_u16()?;
    if version != VERSION {
        return Err(PlayerPayloadError::UnsupportedVersion(version));
    }
    let mut data = PlayerData {
        x: reader.read_f64()?,
        y: reader.read_f64()?,
        z: reader.read_f64()?,
        yaw: reader.read_f32()?,
        pitch: reader.read_f32()?,
        selected_slot: reader.read_u8()?,
        slots: Box::new(std::array::from_fn(|_| ItemStack::empty())),
        carried: ItemStack::empty(),
    };
    validate_state(&data)?;
    let count = reader.read_u16()?;
    if usize::from(count) != SLOT_COUNT {
        return Err(PlayerPayloadError::WrongSlotCount(count));
    }
    for slot in &mut *data.slots {
        *slot = read_stack(&mut reader)?;
    }
    data.carried = read_stack(&mut reader)?;
    reader.finish()?;
    Ok(data)
}
