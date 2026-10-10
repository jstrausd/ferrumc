//! Original signed-toggle vein selection and branch-specific random draws.

use super::super::charge;
use super::{
    clamped_map, random_at, random_float, sample, MaterialBlocks, NoiseChunkError,
    NoiseChunkSession,
};
use crate::BlockStateId;

pub(super) fn compute(
    session: &mut NoiseChunkSession,
    blocks: &MaterialBlocks,
) -> Result<Option<BlockStateId>, NoiseChunkError> {
    charge(&mut session.work)?;
    let pos = session.state.pos();
    let toggle = sample(session, 12, pos, true)?;
    let (min_y, max_y, ore, raw, filler) = if toggle > 0.0 {
        (0, 50, blocks.copper, blocks.raw_copper, blocks.granite)
    } else {
        (-60, -8, blocks.iron, blocks.raw_iron, blocks.tuff)
    };
    let upper = max_y - pos.y();
    let lower = pos.y() - min_y;
    if lower < 0 || upper < 0 {
        return Ok(None);
    }
    let magnitude = toggle.abs();
    let roundoff = clamped_map(f64::from(upper.min(lower)), 0.0, 20.0, -0.2, 0.0);
    if magnitude + roundoff < f64::from(0.4_f32) {
        return Ok(None);
    }
    random_at(session, pos, "ore")?;
    let mut random = session.data.ore_random.at(pos);
    if random_float(session, &mut random)? > 0.7_f32 || sample(session, 13, pos, true)? >= 0.0 {
        return Ok(None);
    }
    let richness = clamped_map(
        magnitude,
        f64::from(0.4_f32),
        f64::from(0.6_f32),
        f64::from(0.1_f32),
        f64::from(0.3_f32),
    );
    if f64::from(random_float(session, &mut random)?) < richness
        && sample(session, 14, pos, true)? > f64::from(-0.3_f32)
    {
        Ok(Some(if random_float(session, &mut random)? < 0.02_f32 {
            raw
        } else {
            ore
        }))
    } else {
        Ok(Some(filler))
    }
}
