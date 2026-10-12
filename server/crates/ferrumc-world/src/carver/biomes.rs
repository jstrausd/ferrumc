//! Closed source and lazy fix-up requests; no public raw climate address.

use super::{
    biome_error, noise_error, CarverError, CarverErrorKind, GeometryWork, ProducerMismatch,
};
use crate::biome_selection::{BiomeClimateTarget, BiomeSelectionSession};
use crate::biome_zoom::{BiomeQuartPos, BiomeZoom};
use crate::density::NoiseChunkSession;
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::biome::BiomeResourceId;

/// Only the closed source/actual-zoom constructors can admit this address.
#[derive(Clone, Copy)]
pub(crate) struct CarverClimateQuart {
    center: ChunkPos,
    origin: BlockPos,
}

/// Temporary disjoint borrows; no self-reference or trait-object callback.
pub(crate) struct DirectCarverBiomes<'a> {
    pub(super) center: ChunkPos,
    pub(super) noise: &'a mut NoiseChunkSession,
    pub(super) selector: &'a mut BiomeSelectionSession,
    pub(super) zoom: BiomeZoom,
    pub(super) work: &'a mut GeometryWork,
    #[cfg(test)]
    pub(super) queries: Option<&'a mut Vec<(bool, BlockPos, BlockPos, BiomeResourceId)>>,
}
impl DirectCarverBiomes<'_> {
    pub(crate) fn biome(&mut self, pos: BlockPos) -> Result<BiomeResourceId, CarverError> {
        // 64 LCG steps, 8 corner candidates, one memo access and one provider entry.
        for _ in 0..74 {
            self.work.charge()?;
        }
        let quart = self.zoom.select_quart(pos);
        let request = CarverClimateQuart::fixup(self.center, pos, quart)?;
        let climate = self
            .noise
            .sample_carver_climate(request)
            .map_err(noise_error)?;
        let selected = self
            .selector
            .select_core(BiomeClimateTarget::from_climate(climate))
            .map_err(biome_error)?;
        #[cfg(test)]
        if let Some(queries) = &mut self.queries {
            queries.push((
                true,
                pos,
                BlockPos::new(quart.x(), quart.y(), quart.z()),
                selected.biome(),
            ));
        }
        Ok(selected.biome())
    }
}
impl CarverClimateQuart {
    fn invalid() -> CarverError {
        CarverError::new(CarverErrorKind::InconsistentProducer(
            ProducerMismatch::Chunk,
        ))
    }
    pub(super) fn source(center: ChunkPos, dx: i32, dz: i32) -> Result<Self, CarverError> {
        if !(-8..=8).contains(&dx) || !(-8..=8).contains(&dz) {
            return Err(Self::invalid());
        }
        crate::density::admit_chunk(center).map_err(|_| Self::invalid())?;
        let qx = (i64::from(center.x()) + i64::from(dx)) * 4;
        let qz = (i64::from(center.z()) + i64::from(dz)) * 4;
        Self::admit(center, qx, 0, qz)
    }
    pub(super) fn fixup(
        center: ChunkPos,
        block: BlockPos,
        selected: BiomeQuartPos,
    ) -> Result<Self, CarverError> {
        if block.to_chunk_pos() != center || !(-63..=311).contains(&block.y()) {
            return Err(Self::invalid());
        }
        let qx = i64::from(selected.x());
        let qz = i64::from(selected.z());
        for (q, c) in [(qx, center.x()), (qz, center.z())] {
            if !(4 * i64::from(c) - 1..=4 * i64::from(c) + 4).contains(&q) {
                return Err(Self::invalid());
            }
        }
        if !(-17..=78).contains(&selected.y()) {
            return Err(Self::invalid());
        }
        Self::admit(center, qx, i64::from(selected.y()), qz)
    }
    fn admit(center: ChunkPos, qx: i64, qy: i64, qz: i64) -> Result<Self, CarverError> {
        // X precedes Z and Y, before physical products or any producer charge.
        if !(-7_500_028..=7_500_024).contains(&qx)
            || !(-7_500_028..=7_500_024).contains(&qz)
            || !(-17..=78).contains(&qy)
        {
            return Err(Self::invalid());
        }
        let x = i32::try_from(qx * 4).map_err(|_| Self::invalid())?;
        let z = i32::try_from(qz * 4).map_err(|_| Self::invalid())?;
        let y = i32::try_from(qy * 4).map_err(|_| Self::invalid())?;
        Ok(Self {
            center,
            origin: BlockPos::new(x, y, z),
        })
    }
    pub(crate) const fn center(&self) -> ChunkPos {
        self.center
    }
    pub(crate) const fn origin(&self) -> BlockPos {
        self.origin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_halo_boundaries_do_not_widen_public_domains() {
        let center = ChunkPos::new(0, 0);
        for (low, high, axis) in [(-7_500_028_i64, 7_500_024_i64, 0), (-17, 78, 1)] {
            for q in [low - 1, low, low + 1, high - 1, high, high + 1] {
                let result = if axis == 0 {
                    CarverClimateQuart::admit(center, q, 0, 0)
                } else {
                    CarverClimateQuart::admit(center, 0, q, 0)
                };
                assert_eq!(result.is_ok(), (low..=high).contains(&q));
                if let Ok(request) = result {
                    assert_eq!(
                        i64::from(if axis == 0 {
                            request.origin().x()
                        } else {
                            request.origin().y()
                        }),
                        q * 4
                    );
                }
                if axis == 0 {
                    assert_eq!(
                        CarverClimateQuart::admit(center, 0, 0, q).is_ok(),
                        (low..=high).contains(&q)
                    );
                }
            }
        }
        for c in [
            -1_875_000, -1_874_999, -1_874_998, 1_874_997, 1_874_998, 1_874_999,
        ] {
            let admitted = (-1_874_999..=1_874_998).contains(&c);
            assert_eq!(
                [(-8, -8), (8, 8)]
                    .into_iter()
                    .all(
                        |(dx, dz)| CarverClimateQuart::source(ChunkPos::new(c, c), dx, dz).is_ok()
                    ),
                admitted
            );
        }
        for d in [-9, -8, -7, 7, 8, 9] {
            assert_eq!(
                CarverClimateQuart::source(center, d, 0).is_ok(),
                (-8..=8).contains(&d)
            );
            assert_eq!(
                CarverClimateQuart::source(center, 0, d).is_ok(),
                (-8..=8).contains(&d)
            );
        }
        // Public physical and quart domains retain their accepted caps.
        assert!(crate::density::ClimateQuartPos::new(-7_500_028, -17, 0).is_err());
        assert!(crate::density::admit_chunk(ChunkPos::new(-1_875_001, 0)).is_err());
        assert!(crate::density::admit_chunk(ChunkPos::new(1_875_000, 0)).is_err());
    }
}
