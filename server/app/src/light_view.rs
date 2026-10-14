//! Allocation-free resident packet light reads and ordinary Overworld darkening.
//! Packet residency, freshness, weather/time authority and spawn acceptance
//! remain with the runtime owner. Missing light never becomes guessed darkness.

use ferrumc_codec::BoundedBytes;
use ferrumc_math::BlockPos;
use ferrumc_proto::generated::play::UpdateLight;

const LIGHT_SECTIONS: u32 = 26;
const LIGHT_BYTES: usize = 2048;
const MASK_BOUND: u32 = (1 << LIGHT_SECTIONS) - 1;
// Reuse milestone10's genuine original 65536-entry f32 sine table.
const SIN: &[u8; 65536 * 4] = include_bytes!("bucket_raycast/data/sin.bin");

/// Exact two-channel nibbles read from the owner's resident light snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResidentLight {
    sky: u8,
    block: u8,
}

impl ResidentLight {
    /// Original sky-light nibble, before the level's sky darkening.
    pub(crate) fn sky(self) -> u8 {
        self.sky
    }

    /// Original block-light nibble, independent of time and sky darkening.
    pub(crate) fn block(self) -> u8 {
        self.block
    }

    /// Brightness for ordinary mob-light facts: max(block, sky minus darkening).
    /// Saturation matches the original max with nonnegative block light; invalid
    /// darkening outside 0..15 refuses instead of silently clamping an input.
    pub(crate) fn raw_brightness(self, darken: u8) -> Option<u8> {
        (darken <= 15).then(|| self.block.max(self.sky.saturating_sub(darken)))
    }
}

/// Read exact light at an absolute position from a resident full light packet.
///
/// Require its chunk identity, section `y.div_euclid(16)+5` in 0..26, bounded
/// disjoint data/empty masks, exact popcount-matched array counts and 2048 bytes
/// per array in both channels. Each selected channel must explicitly provide
/// data or mark the section empty; absent/unchanged mask bits return None.
/// This validates packet structure without allocation or resident-world reads;
/// a valid packet alone does not establish that its chunk is current/resident.
pub(crate) fn read_light(position: BlockPos, packet: &UpdateLight) -> Option<ResidentLight> {
    let chunk = position.to_chunk_pos();
    if chunk.x() != packet.x() || chunk.z() != packet.z() {
        return None;
    }
    let section = u32::try_from(position.y().div_euclid(16) + 5).ok()?;
    if section >= LIGHT_SECTIONS {
        return None;
    }
    let local = position.to_local();
    let index =
        (usize::from(local.y()) << 8) | (usize::from(local.z()) << 4) | usize::from(local.x());
    Some(ResidentLight {
        sky: channel(
            packet.sky_light_mask(),
            packet.empty_sky_light_mask(),
            packet.sky_light(),
            section,
            index,
        )?,
        block: channel(
            packet.block_light_mask(),
            packet.empty_block_light_mask(),
            packet.block_light(),
            section,
            index,
        )?,
    })
}

fn mask(words: &[i64]) -> Option<u32> {
    if words.len() > 1 {
        return None;
    }
    let word = u32::try_from(words.first().copied().unwrap_or(0)).ok()?;
    (word & !MASK_BOUND == 0).then_some(word)
}

fn channel(
    data_mask: &[i64],
    empty_mask: &[i64],
    arrays: &[BoundedBytes<LIGHT_BYTES>],
    section: u32,
    index: usize,
) -> Option<u8> {
    let data_mask = mask(data_mask)?;
    let empty_mask = mask(empty_mask)?;
    if data_mask & empty_mask != 0
        || arrays.len() != data_mask.count_ones() as usize
        || arrays
            .iter()
            .any(|array| array.as_slice().len() != LIGHT_BYTES)
    {
        return None;
    }
    let bit = 1 << section;
    if empty_mask & bit != 0 {
        return Some(0);
    }
    if data_mask & bit == 0 {
        return None;
    }
    let ordinal = (data_mask & (bit - 1)).count_ones() as usize;
    let byte = arrays.get(ordinal)?.as_slice().get(index >> 1)?;
    Some((byte >> ((index & 1) * 4)) & 15)
}

/// Original ordinary unfixed-time Overworld sky darkening in clear weather.
/// Uses original long-to-double time arithmetic, f32 phase conversion/division
/// and exact shared `Mth.cos` lookup. All i64 day times are accepted unchanged;
/// no modulo normalization, saturating time counter or guessed daylight is used.
pub(crate) fn sky_darken(day_time: i64) -> u8 {
    darken_validated(day_time, 0.0, 0.0)
}

/// Original darkening using resolved `getRainLevel(1)`/`getThunderLevel(1)` values.
/// Both must be finite 0..1. Thunder is the effective original returned level,
/// already multiplied by rain, not raw thunder strength or an interpolation seed.
/// Fixed-time dimensions and weather interpolation remain runtime-owned.
pub(crate) fn sky_darken_weather(day_time: i64, rain: f32, thunder: f32) -> Option<u8> {
    if !rain.is_finite()
        || !thunder.is_finite()
        || !(0.0..=1.0).contains(&rain)
        || !(0.0..=1.0).contains(&thunder)
    {
        return None;
    }
    Some(darken_validated(day_time, rain, thunder))
}

fn darken_validated(day_time: i64, rain: f32, thunder: f32) -> u8 {
    // Original multiplication is f32 before division/widening to f64.
    let rain = 1.0 - f64::from(rain * 5.0) / 16.0;
    let thunder = 1.0 - f64::from(thunder * 5.0) / 16.0;
    let cosine = lut_cos(time_of_day(day_time) * std::f32::consts::TAU);
    let daylight = 0.5 + 2.0 * f64::from(cosine).clamp(-0.25, 0.25);
    ((1.0 - daylight * rain * thunder) * 11.0) as u8
}

fn time_of_day(day_time: i64) -> f32 {
    let unwrapped = day_time as f64 / 24000.0 - 0.25;
    // Mth.frac(double) subtracts lfloor; this finite i64-derived domain is far
    // within the long conversion bounds, so floor has the same exact result.
    let fraction = unwrapped - unwrapped.floor();
    let shaped = 0.5 - (fraction * std::f64::consts::PI).cos() / 2.0;
    (fraction * 2.0 + shaped) as f32 / 3.0
}

fn lut_cos(value: f32) -> f32 {
    let index = ((value * 10430.378 + 16384.0) as i32 & 65535) as usize * 4;
    f32::from_le_bytes([SIN[index], SIN[index + 1], SIN[index + 2], SIN[index + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    // Actual original Level.updateSkyBrightness for every normal day-time tick.
    const JVM_CLEAR_PERIOD: &[(i64, i64, u8)] = &[
        (0, 12041, 0),
        (12041, 12210, 1),
        (12210, 12377, 2),
        (12377, 12542, 3),
        (12542, 12705, 4),
        (12705, 12867, 5),
        (12867, 13027, 6),
        (13027, 13188, 7),
        (13188, 13348, 8),
        (13348, 13509, 9),
        (13509, 13670, 10),
        (13670, 22331, 11),
        (22331, 22492, 10),
        (22492, 22653, 9),
        (22653, 22813, 8),
        (22813, 22974, 7),
        (22974, 23135, 6),
        (23135, 23297, 5),
        (23297, 23460, 4),
        (23460, 23624, 3),
        (23624, 23791, 2),
        (23791, 23961, 1),
        (23961, 24000, 0),
    ];
    // Actual original phase/cos/darkening, including effective weather levels.
    const JVM_CASES: &[(i64, u32, u32, u32, u32, u8)] = &[
        (0, 0x0000_0000, 0x0000_0000, 0x3f48_d628, 0x3e5c_55b0, 0),
        (0, 0x3f80_0000, 0x0000_0000, 0x3f48_d628, 0x3e5c_55b0, 3),
        (0, 0x3f80_0000, 0x3f80_0000, 0x3f48_d628, 0x3e5c_55b0, 6),
        (0, 0x3f00_0000, 0x3e00_0000, 0x3f48_d628, 0x3e5c_55b0, 2),
        (0, 0x3dcc_cccd, 0x3db8_51eb, 0x3f48_d628, 0x3e5c_55b0, 1),
        (1, 0x0000_0000, 0x0000_0000, 0x3f48_d8fd, 0x3e5c_86c6, 0),
        (1, 0x3f80_0000, 0x0000_0000, 0x3f48_d8fd, 0x3e5c_86c6, 3),
        (1, 0x3f80_0000, 0x3f80_0000, 0x3f48_d8fd, 0x3e5c_86c6, 6),
        (1, 0x3f00_0000, 0x3e00_0000, 0x3f48_d8fd, 0x3e5c_86c6, 2),
        (1, 0x3dcc_cccd, 0x3db8_51eb, 0x3f48_d8fd, 0x3e5c_86c6, 1),
        (1000, 0x0000_0000, 0x0000_0000, 0x3f53_a0a5, 0x3eed_3063, 0),
        (1000, 0x3f80_0000, 0x0000_0000, 0x3f53_a0a5, 0x3eed_3063, 3),
        (1000, 0x3f80_0000, 0x3f80_0000, 0x3f53_a0a5, 0x3eed_3063, 5),
        (1000, 0x3f00_0000, 0x3e00_0000, 0x3f53_a0a5, 0x3eed_3063, 2),
        (1000, 0x3dcc_cccd, 0x3db8_51eb, 0x3f53_a0a5, 0x3eed_3063, 0),
        (6000, 0x0000_0000, 0x0000_0000, 0x0000_0000, 0x3f80_0000, 0),
        (6000, 0x3f80_0000, 0x0000_0000, 0x0000_0000, 0x3f80_0000, 3),
        (6000, 0x3f80_0000, 0x3f80_0000, 0x0000_0000, 0x3f80_0000, 5),
        (6000, 0x3f00_0000, 0x3e00_0000, 0x0000_0000, 0x3f80_0000, 2),
        (6000, 0x3dcc_cccd, 0x3db8_51eb, 0x0000_0000, 0x3f80_0000, 0),
        (12000, 0x0000_0000, 0x0000_0000, 0x3e5c_a75d, 0x3e5c_6e3b, 0),
        (12000, 0x3f80_0000, 0x0000_0000, 0x3e5c_a75d, 0x3e5c_6e3b, 3),
        (12000, 0x3f80_0000, 0x3f80_0000, 0x3e5c_a75d, 0x3e5c_6e3b, 6),
        (12000, 0x3f00_0000, 0x3e00_0000, 0x3e5c_a75d, 0x3e5c_6e3b, 2),
        (12000, 0x3dcc_cccd, 0x3db8_51eb, 0x3e5c_a75d, 0x3e5c_6e3b, 1),
        (12500, 0x0000_0000, 0x0000_0000, 0x3e73_06f7, 0x3da2_fe71, 3),
        (12500, 0x3f80_0000, 0x0000_0000, 0x3e73_06f7, 0x3da2_fe71, 6),
        (12500, 0x3f80_0000, 0x3f80_0000, 0x3e73_06f7, 0x3da2_fe71, 7),
        (12500, 0x3f00_0000, 0x3e00_0000, 0x3e73_06f7, 0x3da2_fe71, 5),
        (12500, 0x3dcc_cccd, 0x3db8_51eb, 0x3e73_06f7, 0x3da2_fe71, 4),
        (13000, 0x0000_0000, 0x0000_0000, 0x3e84_f0f5, 0xbd78_08b3, 6),
        (13000, 0x3f80_0000, 0x0000_0000, 0x3e84_f0f5, 0xbd78_08b3, 8),
        (13000, 0x3f80_0000, 0x3f80_0000, 0x3e84_f0f5, 0xbd78_08b3, 9),
        (13000, 0x3f00_0000, 0x3e00_0000, 0x3e84_f0f5, 0xbd78_08b3, 7),
        (13000, 0x3dcc_cccd, 0x3db8_51eb, 0x3e84_f0f5, 0xbd78_08b3, 7),
        (
            18000,
            0x0000_0000,
            0x0000_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            18000,
            0x3f80_0000,
            0x0000_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            18000,
            0x3f80_0000,
            0x3f80_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            18000,
            0x3f00_0000,
            0x3e00_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            18000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (23000, 0x0000_0000, 0x0000_0000, 0x3f3d_8785, 0xbd78_6d0c, 6),
        (23000, 0x3f80_0000, 0x0000_0000, 0x3f3d_8785, 0xbd78_6d0c, 8),
        (23000, 0x3f80_0000, 0x3f80_0000, 0x3f3d_8785, 0xbd78_6d0c, 9),
        (23000, 0x3f00_0000, 0x3e00_0000, 0x3f3d_8785, 0xbd78_6d0c, 7),
        (23000, 0x3dcc_cccd, 0x3db8_51eb, 0x3f3d_8785, 0xbd78_6d0c, 7),
        (23999, 0x0000_0000, 0x0000_0000, 0x3f48_d354, 0x3e5c_0c0d, 0),
        (23999, 0x3f80_0000, 0x0000_0000, 0x3f48_d354, 0x3e5c_0c0d, 3),
        (23999, 0x3f80_0000, 0x3f80_0000, 0x3f48_d354, 0x3e5c_0c0d, 6),
        (23999, 0x3f00_0000, 0x3e00_0000, 0x3f48_d354, 0x3e5c_0c0d, 2),
        (23999, 0x3dcc_cccd, 0x3db8_51eb, 0x3f48_d354, 0x3e5c_0c0d, 1),
        (24000, 0x0000_0000, 0x0000_0000, 0x3f48_d628, 0x3e5c_55b0, 0),
        (24000, 0x3f80_0000, 0x0000_0000, 0x3f48_d628, 0x3e5c_55b0, 3),
        (24000, 0x3f80_0000, 0x3f80_0000, 0x3f48_d628, 0x3e5c_55b0, 6),
        (24000, 0x3f00_0000, 0x3e00_0000, 0x3f48_d628, 0x3e5c_55b0, 2),
        (24000, 0x3dcc_cccd, 0x3db8_51eb, 0x3f48_d628, 0x3e5c_55b0, 1),
        (24001, 0x0000_0000, 0x0000_0000, 0x3f48_d8fd, 0x3e5c_86c6, 0),
        (24001, 0x3f80_0000, 0x0000_0000, 0x3f48_d8fd, 0x3e5c_86c6, 3),
        (24001, 0x3f80_0000, 0x3f80_0000, 0x3f48_d8fd, 0x3e5c_86c6, 6),
        (24001, 0x3f00_0000, 0x3e00_0000, 0x3f48_d8fd, 0x3e5c_86c6, 2),
        (24001, 0x3dcc_cccd, 0x3db8_51eb, 0x3f48_d8fd, 0x3e5c_86c6, 1),
        (-1, 0x0000_0000, 0x0000_0000, 0x3f48_d354, 0x3e5c_0c0d, 0),
        (-1, 0x3f80_0000, 0x0000_0000, 0x3f48_d354, 0x3e5c_0c0d, 3),
        (-1, 0x3f80_0000, 0x3f80_0000, 0x3f48_d354, 0x3e5c_0c0d, 6),
        (-1, 0x3f00_0000, 0x3e00_0000, 0x3f48_d354, 0x3e5c_0c0d, 2),
        (-1, 0x3dcc_cccd, 0x3db8_51eb, 0x3f48_d354, 0x3e5c_0c0d, 1),
        (-1000, 0x0000_0000, 0x0000_0000, 0x3f3d_8785, 0xbd78_6d0c, 6),
        (-1000, 0x3f80_0000, 0x0000_0000, 0x3f3d_8785, 0xbd78_6d0c, 8),
        (-1000, 0x3f80_0000, 0x3f80_0000, 0x3f3d_8785, 0xbd78_6d0c, 9),
        (-1000, 0x3f00_0000, 0x3e00_0000, 0x3f3d_8785, 0xbd78_6d0c, 7),
        (-1000, 0x3dcc_cccd, 0x3db8_51eb, 0x3f3d_8785, 0xbd78_6d0c, 7),
        (
            -6000,
            0x0000_0000,
            0x0000_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            -6000,
            0x3f80_0000,
            0x0000_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            -6000,
            0x3f80_0000,
            0x3f80_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            -6000,
            0x3f00_0000,
            0x3e00_0000,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            -6000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3f00_0000,
            0xbf80_0000,
            11,
        ),
        (
            -12000,
            0x0000_0000,
            0x0000_0000,
            0x3e5c_a75d,
            0x3e5c_6e3b,
            0,
        ),
        (
            -12000,
            0x3f80_0000,
            0x0000_0000,
            0x3e5c_a75d,
            0x3e5c_6e3b,
            3,
        ),
        (
            -12000,
            0x3f80_0000,
            0x3f80_0000,
            0x3e5c_a75d,
            0x3e5c_6e3b,
            6,
        ),
        (
            -12000,
            0x3f00_0000,
            0x3e00_0000,
            0x3e5c_a75d,
            0x3e5c_6e3b,
            2,
        ),
        (
            -12000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3e5c_a75d,
            0x3e5c_6e3b,
            1,
        ),
        (
            -18000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x3f80_0000,
            0,
        ),
        (
            -18000,
            0x3f80_0000,
            0x0000_0000,
            0x0000_0000,
            0x3f80_0000,
            3,
        ),
        (
            -18000,
            0x3f80_0000,
            0x3f80_0000,
            0x0000_0000,
            0x3f80_0000,
            5,
        ),
        (
            -18000,
            0x3f00_0000,
            0x3e00_0000,
            0x0000_0000,
            0x3f80_0000,
            2,
        ),
        (
            -18000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x0000_0000,
            0x3f80_0000,
            0,
        ),
        (
            -24000,
            0x0000_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            0,
        ),
        (
            -24000,
            0x3f80_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            3,
        ),
        (
            -24000,
            0x3f80_0000,
            0x3f80_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            6,
        ),
        (
            -24000,
            0x3f00_0000,
            0x3e00_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            2,
        ),
        (
            -24000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3f48_d628,
            0x3e5c_55b0,
            1,
        ),
        (
            -24001,
            0x0000_0000,
            0x0000_0000,
            0x3f48_d354,
            0x3e5c_0c0d,
            0,
        ),
        (
            -24001,
            0x3f80_0000,
            0x0000_0000,
            0x3f48_d354,
            0x3e5c_0c0d,
            3,
        ),
        (
            -24001,
            0x3f80_0000,
            0x3f80_0000,
            0x3f48_d354,
            0x3e5c_0c0d,
            6,
        ),
        (
            -24001,
            0x3f00_0000,
            0x3e00_0000,
            0x3f48_d354,
            0x3e5c_0c0d,
            2,
        ),
        (
            -24001,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3f48_d354,
            0x3e5c_0c0d,
            1,
        ),
        (
            -48000,
            0x0000_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            0,
        ),
        (
            -48000,
            0x3f80_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            3,
        ),
        (
            -48000,
            0x3f80_0000,
            0x3f80_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            6,
        ),
        (
            -48000,
            0x3f00_0000,
            0x3e00_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            2,
        ),
        (
            -48000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3f48_d628,
            0x3e5c_55b0,
            1,
        ),
        (
            -9_223_372_036_854_775_808,
            0x0000_0000,
            0x0000_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_808,
            0x3f80_0000,
            0x0000_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_808,
            0x3f80_0000,
            0x3f80_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_808,
            0x3f00_0000,
            0x3e00_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_808,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_807,
            0x0000_0000,
            0x0000_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_807,
            0x3f80_0000,
            0x0000_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_807,
            0x3f80_0000,
            0x3f80_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_807,
            0x3f00_0000,
            0x3e00_0000,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            -9_223_372_036_854_775_807,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3eda_04db,
            0xbf64_afff,
            11,
        ),
        (
            9_223_372_036_854_775_806,
            0x0000_0000,
            0x0000_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            0,
        ),
        (
            9_223_372_036_854_775_806,
            0x3f80_0000,
            0x0000_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            3,
        ),
        (
            9_223_372_036_854_775_806,
            0x3f80_0000,
            0x3f80_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            5,
        ),
        (
            9_223_372_036_854_775_806,
            0x3f00_0000,
            0x3e00_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            2,
        ),
        (
            9_223_372_036_854_775_806,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3d37_c8af,
            0x3f75_e5c5,
            0,
        ),
        (
            9_223_372_036_854_775_807,
            0x0000_0000,
            0x0000_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            0,
        ),
        (
            9_223_372_036_854_775_807,
            0x3f80_0000,
            0x0000_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            3,
        ),
        (
            9_223_372_036_854_775_807,
            0x3f80_0000,
            0x3f80_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            5,
        ),
        (
            9_223_372_036_854_775_807,
            0x3f00_0000,
            0x3e00_0000,
            0x3d37_c8af,
            0x3f75_e5c5,
            2,
        ),
        (
            9_223_372_036_854_775_807,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3d37_c8af,
            0x3f75_e5c5,
            0,
        ),
        (
            9_007_199_254_740_991,
            0x0000_0000,
            0x0000_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            6,
        ),
        (
            9_007_199_254_740_991,
            0x3f80_0000,
            0x0000_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            8,
        ),
        (
            9_007_199_254_740_991,
            0x3f80_0000,
            0x3f80_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            9,
        ),
        (
            9_007_199_254_740_991,
            0x3f00_0000,
            0x3e00_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            7,
        ),
        (
            9_007_199_254_740_991,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3e84_bfea,
            0xbd6e_3bdc,
            7,
        ),
        (
            9_007_199_254_740_992,
            0x0000_0000,
            0x0000_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            6,
        ),
        (
            9_007_199_254_740_992,
            0x3f80_0000,
            0x0000_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            8,
        ),
        (
            9_007_199_254_740_992,
            0x3f80_0000,
            0x3f80_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            9,
        ),
        (
            9_007_199_254_740_992,
            0x3f00_0000,
            0x3e00_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            7,
        ),
        (
            9_007_199_254_740_992,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3e84_bfea,
            0xbd6e_3bdc,
            7,
        ),
        (
            9_007_199_254_740_993,
            0x0000_0000,
            0x0000_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            6,
        ),
        (
            9_007_199_254_740_993,
            0x3f80_0000,
            0x0000_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            8,
        ),
        (
            9_007_199_254_740_993,
            0x3f80_0000,
            0x3f80_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            9,
        ),
        (
            9_007_199_254_740_993,
            0x3f00_0000,
            0x3e00_0000,
            0x3e84_bfea,
            0xbd6e_3bdc,
            7,
        ),
        (
            9_007_199_254_740_993,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3e84_bfea,
            0xbd6e_3bdc,
            7,
        ),
        (
            -9_007_199_254_740_991,
            0x0000_0000,
            0x0000_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            0,
        ),
        (
            -9_007_199_254_740_991,
            0x3f80_0000,
            0x0000_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            3,
        ),
        (
            -9_007_199_254_740_991,
            0x3f80_0000,
            0x3f80_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            5,
        ),
        (
            -9_007_199_254_740_991,
            0x3f00_0000,
            0x3e00_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            2,
        ),
        (
            -9_007_199_254_740_991,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3e31_d6c8,
            0x3eec_4665,
            0,
        ),
        (
            -9_007_199_254_740_992,
            0x0000_0000,
            0x0000_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            0,
        ),
        (
            -9_007_199_254_740_992,
            0x3f80_0000,
            0x0000_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            3,
        ),
        (
            -9_007_199_254_740_992,
            0x3f80_0000,
            0x3f80_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            5,
        ),
        (
            -9_007_199_254_740_992,
            0x3f00_0000,
            0x3e00_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            2,
        ),
        (
            -9_007_199_254_740_992,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3e31_d6c8,
            0x3eec_4665,
            0,
        ),
        (
            -9_007_199_254_740_993,
            0x0000_0000,
            0x0000_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            0,
        ),
        (
            -9_007_199_254_740_993,
            0x3f80_0000,
            0x0000_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            3,
        ),
        (
            -9_007_199_254_740_993,
            0x3f80_0000,
            0x3f80_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            5,
        ),
        (
            -9_007_199_254_740_993,
            0x3f00_0000,
            0x3e00_0000,
            0x3e31_d6c8,
            0x3eec_4665,
            2,
        ),
        (
            -9_007_199_254_740_993,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3e31_d6c8,
            0x3eec_4665,
            0,
        ),
        (
            24_000_000_000_000_000,
            0x0000_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            0,
        ),
        (
            24_000_000_000_000_000,
            0x3f80_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            3,
        ),
        (
            24_000_000_000_000_000,
            0x3f80_0000,
            0x3f80_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            6,
        ),
        (
            24_000_000_000_000_000,
            0x3f00_0000,
            0x3e00_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            2,
        ),
        (
            24_000_000_000_000_000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3f48_d628,
            0x3e5c_55b0,
            1,
        ),
        (
            -24_000_000_000_000_000,
            0x0000_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            0,
        ),
        (
            -24_000_000_000_000_000,
            0x3f80_0000,
            0x0000_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            3,
        ),
        (
            -24_000_000_000_000_000,
            0x3f80_0000,
            0x3f80_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            6,
        ),
        (
            -24_000_000_000_000_000,
            0x3f00_0000,
            0x3e00_0000,
            0x3f48_d628,
            0x3e5c_55b0,
            2,
        ),
        (
            -24_000_000_000_000_000,
            0x3dcc_cccd,
            0x3db8_51eb,
            0x3f48_d628,
            0x3e5c_55b0,
            1,
        ),
    ];
    const JVM_NIBBLES: &[(i32, i32, i32, u8)] = &[
        (0, 0, 0, 13),
        (1, 0, 0, 0),
        (15, 0, 0, 1),
        (0, 0, 1, 5),
        (0, 1, 0, 13),
        (15, 15, 15, 14),
        (7, 9, 11, 11),
    ];

    fn bytes(value: u8) -> BoundedBytes<2048> {
        BoundedBytes::new(vec![value; 2048]).expect("bounded light bytes")
    }

    fn packet(
        x: i32,
        z: i32,
        sky: (Vec<i64>, Vec<i64>, Vec<BoundedBytes<2048>>),
        block: (Vec<i64>, Vec<i64>, Vec<BoundedBytes<2048>>),
    ) -> UpdateLight {
        UpdateLight::new(x, z, sky.0, block.0, sky.1, block.1, sky.2, block.2)
    }

    fn empty() -> (Vec<i64>, Vec<i64>, Vec<BoundedBytes<2048>>) {
        (vec![], vec![i64::from(MASK_BOUND)], vec![])
    }

    #[test]
    fn clear_period_matches_every_original_tick_and_clear_weather_api() {
        let mut covered = 0;
        for &(start, end, expected) in JVM_CLEAR_PERIOD {
            assert_eq!(start, covered);
            for time in start..end {
                assert_eq!(sky_darken(time), expected, "tick {time}");
                assert_eq!(sky_darken_weather(time, 0.0, 0.0), Some(expected));
            }
            covered = end;
        }
        assert_eq!(covered, 24000);
    }

    #[test]
    fn original_phase_cos_and_weather_goldens_preserve_negative_and_large_times() {
        for &(time, rain_bits, thunder_bits, phase_bits, cos_bits, expected) in JVM_CASES {
            let phase = time_of_day(time);
            assert_eq!(phase.to_bits(), phase_bits, "phase at {time}");
            assert_eq!(
                lut_cos(phase * std::f32::consts::TAU).to_bits(),
                cos_bits,
                "cos at {time}"
            );
            assert_eq!(
                sky_darken_weather(
                    time,
                    f32::from_bits(rain_bits),
                    f32::from_bits(thunder_bits)
                ),
                Some(expected),
                "weather at {time}"
            );
        }
    }

    #[test]
    fn original_data_layer_nibbles_match_packet_xy_z_layout() {
        let data: Vec<u8> = (0..2048).map(|index| (index * 37 + 13) as u8).collect();
        let packet = packet(
            0,
            0,
            (
                vec![1 << 5],
                vec![],
                vec![BoundedBytes::new(data).expect("array")],
            ),
            empty(),
        );
        for &(x, y, z, expected) in JVM_NIBBLES {
            let light = read_light(BlockPos::new(x, y, z), &packet).expect("resident data");
            assert_eq!(light.sky(), expected, "{x},{y},{z}");
            assert_eq!(light.block(), 0);
        }
    }

    #[test]
    fn sparse_masks_use_original_ascending_section_array_ordinal() {
        let packet = packet(
            -1,
            -2,
            (
                vec![(1 << 0) | (1 << 5) | (1 << 25)],
                vec![],
                vec![bytes(0x21), bytes(0x43), bytes(0x65)],
            ),
            (vec![], vec![(1 << 0) | (1 << 5) | (1 << 25)], vec![]),
        );
        for (y, low, high) in [(-80, 1, 2), (0, 3, 4), (320, 5, 6)] {
            assert_eq!(
                read_light(BlockPos::new(-16, y, -32), &packet)
                    .expect("low nibble")
                    .sky(),
                low
            );
            assert_eq!(
                read_light(BlockPos::new(-15, y, -32), &packet)
                    .expect("high nibble")
                    .sky(),
                high
            );
        }
        assert!(read_light(BlockPos::new(-16, 16, -32), &packet).is_none());
    }

    #[test]
    fn negative_coordinates_and_exact_light_padding_bounds_are_floored() {
        let packet = packet(
            -1,
            -1,
            (
                vec![i64::from(MASK_BOUND)],
                vec![],
                (0..26).map(|section| bytes(section as u8)).collect(),
            ),
            empty(),
        );
        for (y, section) in [
            (-80, 0),
            (-65, 0),
            (-64, 1),
            (-17, 3),
            (-16, 4),
            (-1, 4),
            (0, 5),
            (15, 5),
            (16, 6),
            (319, 24),
            (320, 25),
            (335, 25),
        ] {
            assert_eq!(
                read_light(BlockPos::new(-2, y, -1), &packet)
                    .expect("bounded section")
                    .sky(),
                section & 15,
                "y {y}"
            );
        }
        for y in [i32::MIN, -81, 336, i32::MAX] {
            assert!(read_light(BlockPos::new(-1, y, -1), &packet).is_none());
        }
        assert!(read_light(BlockPos::new(0, 0, -1), &packet).is_none());
        assert!(read_light(BlockPos::new(-1, 0, 0), &packet).is_none());
    }

    #[test]
    fn explicitly_empty_is_known_zero_and_missing_channel_is_unavailable() {
        let known = packet(0, 0, empty(), empty());
        assert_eq!(
            read_light(BlockPos::ORIGIN, &known),
            Some(ResidentLight { sky: 0, block: 0 })
        );
        let missing = (vec![], vec![], vec![]);
        for candidate in [
            packet(0, 0, missing.clone(), empty()),
            packet(0, 0, empty(), missing),
        ] {
            assert!(read_light(BlockPos::ORIGIN, &candidate).is_none());
        }
    }

    #[test]
    fn malformed_masks_counts_and_short_arrays_refuse_even_off_target() {
        let bad = [
            (vec![1 << 26], vec![], vec![bytes(0)]),
            (vec![], vec![1 << 26], vec![]),
            (vec![-1], vec![], vec![]),
            (vec![0, 0], vec![], vec![]),
            (vec![], vec![0, 0], vec![]),
            (vec![1 << 5], vec![1 << 5], vec![bytes(0)]),
            (vec![1 << 5], vec![], vec![]),
            (vec![1 << 5], vec![], vec![bytes(0), bytes(0)]),
            (
                vec![(1 << 0) | (1 << 5)],
                vec![],
                vec![BoundedBytes::new(vec![0; 2047]).expect("short"), bytes(0)],
            ),
            (vec![], vec![i64::from(MASK_BOUND)], vec![bytes(0)]),
        ];
        for invalid in bad {
            for candidate in [
                packet(0, 0, invalid.clone(), empty()),
                packet(0, 0, empty(), invalid),
            ] {
                assert!(read_light(BlockPos::ORIGIN, &candidate).is_none());
            }
        }
    }

    #[test]
    fn raw_brightness_is_block_or_darkened_sky_without_underflow() {
        for sky in 0..16 {
            for block in 0..16 {
                let light = ResidentLight { sky, block };
                for darken in 0..16 {
                    let original = i16::from(block).max(i16::from(sky) - i16::from(darken)) as u8;
                    assert_eq!(light.raw_brightness(darken), Some(original));
                }
                assert_eq!(light.raw_brightness(16), None);
                assert_eq!(light.raw_brightness(u8::MAX), None);
            }
        }
    }

    #[test]
    fn malformed_weather_refuses_and_shared_original_lut_stays_finite() {
        for invalid in [-0.01, 1.01, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(sky_darken_weather(6000, invalid, 0.0), None);
            assert_eq!(sky_darken_weather(6000, 0.0, invalid), None);
        }
        for index in 0..65536 {
            let offset = index * 4;
            let value = f32::from_le_bytes([
                SIN[offset],
                SIN[offset + 1],
                SIN[offset + 2],
                SIN[offset + 3],
            ]);
            assert!(value.is_finite() && (-1.0..=1.0).contains(&value));
        }
    }
}
