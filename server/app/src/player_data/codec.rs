//! Bounded schema-5 player envelope, independent of Minecraft network framing.
//! Existing item frames remain byte-identical; a fixed twenty-one-byte vitals
//! suffix records all nine fields. Explicit schemas three/four omit that suffix.
use std::error::Error;
use std::fmt;

use ferrumc_codec::{BoundedReader, CodecError};
use ferrumc_items::{
    decode_persisted_stack, encode_persisted_stack, ItemPersistenceError, ItemStack,
    MAX_PERSISTED_STACK_BYTES,
};
use ferrumc_sim::{PlayerVitals, VitalsError, VitalsSnapshot};
use ferrumc_storage::MAX_PLAYER_DATA_LEN;

use super::{is_valid_player_position, PlayerData};
use crate::inventory::SLOT_COUNT;

const MAGIC: &[u8; 4] = b"FCPS";
const VERSION: u16 = 5;
const HEADER_BYTES: usize = 4 + 2 + 3 * 8 + 2 * 4 + 1 + 2;
const FRAME_BYTES: usize = 4;
/// Four f32 scalars and five u8 hunger/timer/phase scalars, with no lengths.
pub(super) const VITALS_BYTES: usize = 4 * 4 + 5;

/// A specific malformed player envelope or one nested item-storage failure.
#[derive(Debug)]
pub(crate) enum PlayerPayloadError {
    /// Legacy JSON failed its version-specific decoder.
    Json(serde_json::Error),
    /// A bounded field read failed, including truncation and trailing bytes.
    Codec(CodecError),
    /// One nested authoritative stack failed strict storage validation.
    Item(ItemPersistenceError),
    /// Vitals scalars or timers failed authoritative Sim snapshot admission.
    InvalidVitals(VitalsError),
    /// The envelope signature did not identify player storage.
    InvalidMagic,
    /// The inner envelope version disagreed with its explicit outer schema.
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
            Self::InvalidVitals(error) => write!(f, "player vitals: {error}"),
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
            Self::InvalidVitals(source) => Some(source),
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
    PlayerVitals::from_snapshot(data.vitals).map_err(PlayerPayloadError::InvalidVitals)?;
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
    for stack in data
        .slots
        .iter()
        .chain(std::iter::once(&data.carried))
        .chain(data.table_inputs.iter())
    {
        let encoded = encode_persisted_stack(stack)?;
        let next = out.len() + FRAME_BYTES + encoded.len() + VITALS_BYTES;
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
    out.reserve_exact(VITALS_BYTES);
    let vitals = data.vitals();
    out.extend_from_slice(&vitals.health().to_bits().to_be_bytes());
    out.push(vitals.food());
    out.extend_from_slice(&vitals.saturation().to_bits().to_be_bytes());
    out.extend_from_slice(&vitals.exhaustion().to_bits().to_be_bytes());
    out.extend_from_slice(&[
        vitals.food_tick_timer(),
        vitals.invulnerable_ticks(),
        vitals.hurt_ticks(),
    ]);
    out.extend_from_slice(&vitals.last_hurt().to_bits().to_be_bytes());
    out.push(vitals.peaceful_phase());
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

/// Decode schema five with 56 exact item frames and nine validated vitals fields.
/// Explicit schema four retains 56 frames and schema three retains 47; both
/// restore fresh vitals, with empty table inputs only in schema three.
/// Fixed inventory storage is initialized without untrusted list allocation;
/// every nested frame is checked before its item decoder can copy components.
pub(super) fn decode(
    bytes: &[u8],
    expected_version: u16,
) -> Result<PlayerData, PlayerPayloadError> {
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
    if !matches!(version, 3 | 4 | VERSION) || version != expected_version {
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
        table_inputs: Box::new(std::array::from_fn(|_| ItemStack::empty())),
        vitals: PlayerVitals::new().snapshot(),
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
    if version >= 4 {
        for stack in &mut *data.table_inputs {
            *stack = read_stack(&mut reader)?;
        }
    }
    if version == VERSION {
        let health = reader.read_f32()?;
        let food = reader.read_u8()?;
        let saturation = reader.read_f32()?;
        let exhaustion = reader.read_f32()?;
        let food_timer = reader.read_u8()?;
        let invulnerable = reader.read_u8()?;
        let hurt = reader.read_u8()?;
        let last_hurt = reader.read_f32()?;
        let phase = reader.read_u8()?;
        let snapshot = VitalsSnapshot::new(health, food, saturation, exhaustion)
            .and_then(|snapshot| {
                snapshot.with_timers(food_timer, invulnerable, hurt, last_hurt, phase)
            })
            .map_err(PlayerPayloadError::InvalidVitals)?;
        data.set_vitals(snapshot)
            .map_err(PlayerPayloadError::InvalidVitals)?;
    }
    reader.finish()?;
    Ok(data)
}
