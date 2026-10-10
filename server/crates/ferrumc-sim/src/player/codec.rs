//! Exact FCPS3 grammar, using the independently pinned application envelope.

use ferrumc_items::{
    decode_persisted_stack, encode_persisted_stack, ItemStack, MAX_PERSISTED_STACK_BYTES,
};
use ferrumc_math::Vec3;
use ferrumc_storage::{PlayerRecord, MAX_PLAYER_DATA_LEN};

use super::{
    NativePlayerSnapshot, PlayerOwnershipError, NATIVE_PLAYER_HEADER_BYTES, NATIVE_PLAYER_SCHEMA,
    NATIVE_PLAYER_SLOT_COUNT,
};

const MAGIC: &[u8; 4] = b"FCPS";
const VERSION: u16 = 3;

fn validate(data: &NativePlayerSnapshot) -> Result<(), PlayerOwnershipError> {
    if [data.position.x, data.position.y, data.position.z]
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 30_000_000.0)
    {
        return Err(PlayerOwnershipError::Malformed("position"));
    }
    if data.look.iter().any(|v| !v.is_finite()) {
        return Err(PlayerOwnershipError::Malformed("look"));
    }
    if data.selected >= 9 {
        return Err(PlayerOwnershipError::Malformed("selection"));
    }
    Ok(())
}

pub(super) fn encode(data: &NativePlayerSnapshot) -> Result<PlayerRecord, PlayerOwnershipError> {
    validate(data)?;
    encode_parts(
        data.position,
        data.look,
        data.game_mode,
        data.selected,
        &data.slots,
        &data.carried,
    )
}

pub(crate) fn encode_parts(
    position: Vec3,
    look: [f32; 2],
    game_mode: ferrumc_core::GameMode,
    selected: u8,
    slots: &[ItemStack; NATIVE_PLAYER_SLOT_COUNT],
    carried: &ItemStack,
) -> Result<PlayerRecord, PlayerOwnershipError> {
    if [position.x, position.y, position.z]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 30_000_000.0)
        || look.iter().any(|v| !v.is_finite())
        || selected >= 9
    {
        return Err(PlayerOwnershipError::Malformed("native player header"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(NATIVE_PLAYER_HEADER_BYTES)
        .map_err(|_| PlayerOwnershipError::AllocationFailed)?;
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_be_bytes());
    for value in [position.x, position.y, position.z] {
        bytes.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    for value in look {
        bytes.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    bytes.push(selected);
    bytes.extend_from_slice(&46_u16.to_be_bytes());
    for stack in slots.iter().chain(std::iter::once(carried)) {
        let item = encode_persisted_stack(stack)?;
        let next = bytes
            .len()
            .checked_add(4)
            .and_then(|n| n.checked_add(item.len()))
            .ok_or(PlayerOwnershipError::Capacity("player bytes"))?;
        if next > MAX_PLAYER_DATA_LEN {
            return Err(PlayerOwnershipError::Capacity("player bytes"));
        }
        bytes
            .try_reserve_exact(4 + item.len())
            .map_err(|_| PlayerOwnershipError::AllocationFailed)?;
        let len =
            u32::try_from(item.len()).map_err(|_| PlayerOwnershipError::Capacity("item frame"))?;
        bytes.extend_from_slice(&len.to_be_bytes());
        bytes.extend_from_slice(&item);
    }
    PlayerRecord::new(NATIVE_PLAYER_SCHEMA, game_mode, bytes)
        .map_err(|e| PlayerOwnershipError::Storage(e.into()))
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], PlayerOwnershipError> {
        let end = self
            .offset
            .checked_add(size)
            .ok_or(PlayerOwnershipError::Malformed("length overflow"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(PlayerOwnershipError::Malformed("truncated"))?;
        self.offset = end;
        Ok(bytes)
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], PlayerOwnershipError> {
        self.take(N)?
            .try_into()
            .map_err(|_| PlayerOwnershipError::Malformed("truncated"))
    }
    fn stack(&mut self) -> Result<ItemStack, PlayerOwnershipError> {
        let raw = u32::from_be_bytes(self.fixed()?);
        let len =
            usize::try_from(raw).map_err(|_| PlayerOwnershipError::Malformed("frame length"))?;
        if len > MAX_PERSISTED_STACK_BYTES {
            return Err(PlayerOwnershipError::Capacity("item frame"));
        }
        Ok(decode_persisted_stack(self.take(len)?)?)
    }
}

pub(super) fn decode(record: &PlayerRecord) -> Result<NativePlayerSnapshot, PlayerOwnershipError> {
    if record.schema_version() != NATIVE_PLAYER_SCHEMA {
        return Err(PlayerOwnershipError::UnsupportedSchema(
            record.schema_version(),
        ));
    }
    if record.data().len() > MAX_PLAYER_DATA_LEN {
        return Err(PlayerOwnershipError::Capacity("player bytes"));
    }
    let mut r = Reader {
        bytes: record.data(),
        offset: 0,
    };
    if r.take(4)? != MAGIC {
        return Err(PlayerOwnershipError::Malformed("magic"));
    }
    if u16::from_be_bytes(r.fixed()?) != VERSION {
        return Err(PlayerOwnershipError::Malformed("version"));
    }
    let position = Vec3::new(
        f64::from_bits(u64::from_be_bytes(r.fixed()?)),
        f64::from_bits(u64::from_be_bytes(r.fixed()?)),
        f64::from_bits(u64::from_be_bytes(r.fixed()?)),
    );
    let look = [
        f32::from_bits(u32::from_be_bytes(r.fixed()?)),
        f32::from_bits(u32::from_be_bytes(r.fixed()?)),
    ];
    let selected = r.fixed::<1>()?[0];
    if u16::from_be_bytes(r.fixed()?) != 46 {
        return Err(PlayerOwnershipError::Malformed("slot count"));
    }
    // Header policy is checked before any untrusted component tree is allocated.
    if [position.x, position.y, position.z]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 30_000_000.0)
        || look.iter().any(|v| !v.is_finite())
        || selected >= 9
    {
        return Err(PlayerOwnershipError::Malformed("header admission"));
    }
    // Frame extents are checked for the entire fixed shape before the first decode.
    let mut scan = Reader {
        bytes: r.bytes,
        offset: r.offset,
    };
    for _ in 0..=NATIVE_PLAYER_SLOT_COUNT {
        let len = usize::try_from(u32::from_be_bytes(scan.fixed()?))
            .map_err(|_| PlayerOwnershipError::Malformed("frame length"))?;
        if len > MAX_PERSISTED_STACK_BYTES {
            return Err(PlayerOwnershipError::Capacity("item frame"));
        }
        scan.take(len)?;
    }
    if scan.offset != scan.bytes.len() {
        return Err(PlayerOwnershipError::Malformed("trailing bytes"));
    }
    let mut slots = Box::new(std::array::from_fn(|_| ItemStack::empty()));
    for stack in &mut *slots {
        *stack = r.stack()?;
    }
    let carried = r.stack()?;
    Ok(NativePlayerSnapshot::new(
        position,
        look,
        record.game_mode(),
        selected,
        slots,
        carried,
    ))
}
