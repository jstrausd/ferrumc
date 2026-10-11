//! Original End section transform and f64 thresholds in checked coordinates.

use super::{charge, fail, BiomeError, BiomeErrorKind, BiomeResourceId, SampleWork};
use crate::density::{ClimateQuartPos, NoiseChunkSession};
use ferrumc_math::BlockPos;

pub(super) fn select(
    biomes: [BiomeResourceId; 5],
    pos: ClimateQuartPos,
    noise: &mut NoiseChunkSession,
    work: &mut SampleWork,
) -> Result<BiomeResourceId, BiomeError> {
    let position = center(pos, work)?;
    match position {
        None => Ok(biomes[0]),
        Some(position) => {
            charge(work)?;
            let erosion = noise.cached_biome_erosion(position)?;
            Ok(biomes[branch(erosion, work)?])
        }
    }
}

pub(super) fn center(
    pos: ClimateQuartPos,
    work: &mut SampleWork,
) -> Result<Option<BlockPos>, BiomeError> {
    let block = pos.block_origin();
    charge(work)?;
    let section_x = i64::from(block.x()) >> 4;
    charge(work)?;
    let section_z = i64::from(block.z()) >> 4;
    charge(work)?;
    let radius = section_x
        .checked_mul(section_x)
        .and_then(|x| {
            section_z
                .checked_mul(section_z)
                .and_then(|z| x.checked_add(z))
        })
        .ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))?;
    charge(work)?;
    if radius <= 4096 {
        return Ok(None);
    }
    let mut centers = [0_i32; 2];
    for ((axis, section), destination) in [("x", section_x), ("z", section_z)]
        .into_iter()
        .zip(&mut centers)
    {
        charge(work)?;
        let coordinate = section
            .checked_mul(2)
            .and_then(|v| v.checked_add(1))
            .and_then(|v| v.checked_mul(8))
            .ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))?;
        if !(-30_000_000..=30_000_000).contains(&coordinate) {
            return Err(fail(BiomeErrorKind::PositionOutOfRange {
                axis,
                coordinate,
            }));
        }
        *destination = i32::try_from(coordinate)
            .map_err(|_| fail(BiomeErrorKind::PositionOutOfRange { axis, coordinate }))?;
    }
    Ok(Some(BlockPos::new(centers[0], block.y(), centers[1])))
}

pub(super) fn branch(erosion: f64, work: &mut SampleWork) -> Result<usize, BiomeError> {
    // Existing noise evaluation already admits finite f64 before this helper.
    if !erosion.is_finite() {
        return Err(fail(BiomeErrorKind::InvalidTree));
    }
    charge(work)?;
    if erosion > 0.25 {
        return Ok(1);
    }
    charge(work)?;
    if erosion >= -0.0625 {
        return Ok(2);
    }
    charge(work)?;
    Ok(if erosion < -0.21875 { 3 } else { 4 })
}
