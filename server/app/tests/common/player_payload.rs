//! Independent test reader for the schema-5 envelope written after real sessions.
use ferrumc_codec::BoundedReader;
use ferrumc_items::{decode_persisted_stack, ItemStack, MAX_PERSISTED_STACK_BYTES};
use ferrumc_math::Vec3;
use ferrumc_storage::{PlayerRecord, SchemaVersion};

/// State observed in a completed schema-5 player save.
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
    /// Nine lossless crafting-table input frames.
    pub table_inputs: Vec<ItemStack>,
    /// Validated durable health, hunger and damage timers.
    pub vitals: ferrumc_sim::VitalsSnapshot,
}

fn item(reader: &mut BoundedReader<'_>) -> anyhow::Result<ItemStack> {
    let len = usize::try_from(reader.read_u32()?)?;
    anyhow::ensure!(len <= MAX_PERSISTED_STACK_BYTES, "oversized item frame");
    Ok(decode_persisted_stack(reader.read_bytes(len)?)?)
}

/// Reads the binary envelope without calling the application's private decoder.
pub fn decode(record: &PlayerRecord) -> anyhow::Result<StoredPlayer> {
    anyhow::ensure!(
        record.schema_version() == SchemaVersion::new(5),
        "expected schema five"
    );
    let mut reader = BoundedReader::new(record.data());
    anyhow::ensure!(reader.read_bytes(4)? == b"FCPS", "invalid signature");
    anyhow::ensure!(reader.read_u16()? == 5, "invalid payload version");
    let position = Vec3::new(reader.read_f64()?, reader.read_f64()?, reader.read_f64()?);
    let yaw = reader.read_f32()?;
    let pitch = reader.read_f32()?;
    let selected = reader.read_u8()?;
    anyhow::ensure!(reader.read_u16()? == 46, "wrong inventory shape");
    let slots = (0..46)
        .map(|_| item(&mut reader))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let carried = item(&mut reader)?;
    let table_inputs = (0..9)
        .map(|_| item(&mut reader))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let health = reader.read_f32()?;
    let food = reader.read_u8()?;
    let saturation = reader.read_f32()?;
    let exhaustion = reader.read_f32()?;
    let food_timer = reader.read_u8()?;
    let invulnerable = reader.read_u8()?;
    let hurt = reader.read_u8()?;
    let last_hurt = reader.read_f32()?;
    let phase = reader.read_u8()?;
    let vitals = ferrumc_sim::VitalsSnapshot::new(health, food, saturation, exhaustion)?
        .with_timers(food_timer, invulnerable, hurt, last_hurt, phase)?;
    reader.finish()?;
    Ok(StoredPlayer {
        position,
        yaw,
        pitch,
        selected,
        slots,
        carried,
        table_inputs,
        vitals,
    })
}
