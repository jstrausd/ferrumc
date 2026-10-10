//! Independent test reader for the schema-3 envelope written after real sessions.
use ferrumc_codec::BoundedReader;
use ferrumc_items::{decode_persisted_stack, ItemStack, MAX_PERSISTED_STACK_BYTES};
use ferrumc_math::Vec3;
use ferrumc_storage::{PlayerRecord, SchemaVersion};

/// State observed in a completed schema-3 player save.
#[allow(dead_code)] // Different lifecycle regressions inspect different fields.
pub struct StoredPlayer {
    /// Saved absolute coordinates.
    pub position: Vec3,
    /// Saved look yaw.
    pub yaw: f32,
    /// Saved look pitch.
    pub pitch: f32,
    /// Saved hotbar selection.
    pub selected: u8,
    /// All forty-six saved inventory slots.
    pub slots: Vec<ItemStack>,
    /// Saved player-owned cursor.
    pub carried: ItemStack,
}

fn item(reader: &mut BoundedReader<'_>) -> anyhow::Result<ItemStack> {
    let len = usize::try_from(reader.read_u32()?)?;
    anyhow::ensure!(len <= MAX_PERSISTED_STACK_BYTES, "oversized item frame");
    Ok(decode_persisted_stack(reader.read_bytes(len)?)?)
}

/// Reads the binary envelope without calling the application's private decoder.
pub fn decode(record: &PlayerRecord) -> anyhow::Result<StoredPlayer> {
    anyhow::ensure!(
        record.schema_version() == SchemaVersion::new(3),
        "expected schema three"
    );
    let mut reader = BoundedReader::new(record.data());
    anyhow::ensure!(reader.read_bytes(4)? == b"FCPS", "invalid signature");
    anyhow::ensure!(reader.read_u16()? == 3, "invalid payload version");
    let position = Vec3::new(reader.read_f64()?, reader.read_f64()?, reader.read_f64()?);
    let yaw = reader.read_f32()?;
    let pitch = reader.read_f32()?;
    let selected = reader.read_u8()?;
    anyhow::ensure!(reader.read_u16()? == 46, "wrong inventory shape");
    let slots = (0..46)
        .map(|_| item(&mut reader))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let carried = item(&mut reader)?;
    reader.finish()?;
    Ok(StoredPlayer {
        position,
        yaw,
        pitch,
        selected,
        slots,
        carried,
    })
}
