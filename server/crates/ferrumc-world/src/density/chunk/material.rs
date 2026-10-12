//! Run-owned aquifer and vein material selection, with no general callback API.

use ferrumc_math::BlockPos;

use super::{
    bytes, core_charge as charge, core_fail as fail, ChunkMaterialSamples,
    NoiseChunkCoreError as NoiseChunkError, NoiseChunkErrorKind, NoiseChunkLimit,
    NoiseChunkSession,
};
use crate::sample_work::SampleWork;
use crate::BlockStateId;

mod aquifer;
mod blocks;
mod ore;

use aquifer::Aquifer;
use blocks::MaterialBlocks;

/// One fresh, persistent Aquifer; the same admitted Noise meter owns all work.
pub(crate) struct CarverAquifer {
    aquifer: Aquifer,
    blocks: MaterialBlocks,
    domain: MaterialDomain,
}
impl CarverAquifer {
    pub(crate) fn new(session: &mut NoiseChunkSession) -> Result<Self, NoiseChunkError> {
        let domain = MaterialDomain::new(session)?;
        let blocks = MaterialBlocks::new(
            session.data.metadata.default_block,
            session.data.metadata.default_fluid,
            &mut session.work,
        )?;
        let aquifer = Aquifer::new(session)?;
        Ok(Self {
            aquifer,
            blocks,
            domain,
        })
    }
    pub(crate) fn compute(
        &mut self,
        session: &mut NoiseChunkSession,
        pos: BlockPos,
    ) -> Result<Option<BlockStateId>, NoiseChunkError> {
        self.domain.center(pos)?;
        session.state.start = pos;
        session.state.in_x = 0;
        session.state.in_y = 0;
        session.state.in_z = 0;
        self.aquifer
            .compute(session, &self.blocks, self.domain, 0.0)
    }
    pub(crate) const fn schedule(&self) -> bool {
        self.aquifer.schedule
    }
    pub(crate) const fn lava(&self) -> BlockStateId {
        self.blocks.lava
    }
}

const HALO_LIMIT: i32 = 30_000_128;
/// States of the closed GEN-05.4 material alphabet ([`material_alphabet`]).
pub(crate) const MATERIAL_ALPHABET_STATES: usize = 10;
/// Separate allowance of one alphabet resolution: two bounded definition
/// resolutions (each at most 4096 bytes plus its JSON nodes) and ten lookups.
const MATERIAL_ALPHABET_WORK: usize = 16_384;

/// The closed set of states a GEN-05.4 material run can write, resolved through
/// the same `MaterialBlocks` as [`MaterialRun::new`] from a settings
/// `default_block` and `default_fluid` definition: `[default_block,
/// default_fluid, air, lava, copper_ore, raw_copper_block, granite,
/// deepslate_iron_ore, raw_iron_block, tuff]`. Read-only consumer path for the
/// GEN-05.10 column driver; material sampling is unchanged.
pub(crate) fn material_alphabet(
    default_block: &str,
    default_fluid: &str,
) -> Result<[BlockStateId; MATERIAL_ALPHABET_STATES], NoiseChunkError> {
    let mut work = SampleWork::new(MATERIAL_ALPHABET_WORK);
    let blocks = MaterialBlocks::new(default_block, default_fluid, &mut work)?;
    Ok([
        blocks.default_block,
        blocks.default_fluid,
        blocks.air,
        blocks.lava,
        blocks.copper,
        blocks.raw_copper,
        blocks.granite,
        blocks.iron,
        blocks.raw_iron,
        blocks.tuff,
    ])
}

pub(super) struct MaterialRun {
    pub states: Vec<BlockStateId>,
    pub fluid_updates: Vec<u64>,
    blocks: MaterialBlocks,
    aquifer: Aquifer,
    domain: MaterialDomain,
    ore_veins: bool,
}
impl MaterialRun {
    pub fn new(session: &mut NoiseChunkSession, count: usize) -> Result<Self, NoiseChunkError> {
        let domain = MaterialDomain::new(session)?;
        bytes(
            NoiseChunkLimit::MaterialOutput,
            0,
            count,
            std::mem::size_of::<BlockStateId>(),
            512 * 1024,
        )?;
        bytes(
            NoiseChunkLimit::FluidBitmap,
            0,
            count.div_ceil(64),
            8,
            16 * 1024,
        )?;
        charge(&mut session.work)?;
        let blocks = MaterialBlocks::new(
            session.data.metadata.default_block,
            session.data.metadata.default_fluid,
            &mut session.work,
        )?;
        let aquifer = Aquifer::new(session)?;
        // Every element is charged before its fixed, admitted allocation.
        for _ in 0..count {
            charge(&mut session.work)?;
        }
        for _ in 0..count.div_ceil(64) {
            charge(&mut session.work)?;
        }
        Ok(Self {
            states: vec![blocks.air; count],
            fluid_updates: vec![0; count.div_ceil(64)],
            blocks,
            aquifer,
            domain,
            ore_veins: session.data.metadata.ore_veins,
        })
    }
    pub fn write(
        &mut self,
        session: &mut NoiseChunkSession,
        index: usize,
    ) -> Result<(), NoiseChunkError> {
        let substance = session.sample_material()?;
        let state = if let Some(state) =
            self.aquifer
                .compute(session, &self.blocks, self.domain, substance)?
        {
            state
        } else if self.ore_veins {
            ore::compute(session, &self.blocks)?.unwrap_or(self.blocks.default_block)
        } else {
            self.blocks.default_block
        };
        let flag =
            self.aquifer.schedule && state != self.blocks.air && self.blocks.has_fluid(state);
        charge(&mut session.work)?;
        let slot = self
            .states
            .get_mut(index)
            .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
        *slot = state;
        charge(&mut session.work)?;
        if flag {
            let word = self
                .fluid_updates
                .get_mut(index / 64)
                .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
            *word |= 1_u64 << (index % 64);
        }
        Ok(())
    }
    pub fn finish(self, session: &NoiseChunkSession) -> ChunkMaterialSamples {
        ChunkMaterialSamples {
            seed: session.seed(),
            resource: session.resource(),
            chunk: session.chunk,
            geometry: session.geometry(),
            states: self.states.into_boxed_slice(),
            fluid_updates: self.fluid_updates.into_boxed_slice(),
        }
    }
}

#[derive(Clone, Copy)]
struct MaterialDomain {
    min_center_y: i32,
    max_center_y: i32,
}
impl MaterialDomain {
    fn new(session: &NoiseChunkSession) -> Result<Self, NoiseChunkError> {
        // Public chunk admission already guarantees origin/interpolation corner
        // within ±30M. Check the separately derived physical surface extents
        // before any material allocation, without changing that public domain.
        for (axis, origin, low, high) in [
            ("x", session.state.origin.x(), -71_i64, 36_i64),
            ("z", session.state.origin.z(), -39_i64, 36_i64),
        ] {
            for value in [i64::from(origin) + low, i64::from(origin) + 15 + high] {
                if value < -i64::from(HALO_LIMIT) || value > i64::from(HALO_LIMIT) {
                    return Err(fail(NoiseChunkErrorKind::InvalidGraph));
                }
            }
            position_component(axis, origin, -30_000_000, 30_000_000)?;
        }
        let min = i64::from(session.geometry().min_y()) - 22;
        let max =
            i64::from(session.geometry().min_y()) + i64::from(session.geometry().height()) + 20;
        if min < -2048 || max > 2047 {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        Ok(Self {
            min_center_y: min as i32,
            max_center_y: max as i32,
        })
    }
    fn center(self, pos: BlockPos) -> Result<(), NoiseChunkError> {
        Self::horizontal(pos)?;
        position_component("y", pos.y(), self.min_center_y, self.max_center_y)
    }
    fn horizontal(pos: BlockPos) -> Result<(), NoiseChunkError> {
        position_component("x", pos.x(), -HALO_LIMIT, HALO_LIMIT)?;
        position_component("z", pos.z(), -HALO_LIMIT, HALO_LIMIT)
    }
    fn scaled(self, pos: BlockPos, scale: i32) -> Result<(), NoiseChunkError> {
        position_component(
            "x",
            pos.x(),
            (-HALO_LIMIT).div_euclid(scale),
            HALO_LIMIT.div_euclid(scale),
        )?;
        position_component(
            "z",
            pos.z(),
            (-HALO_LIMIT).div_euclid(scale),
            HALO_LIMIT.div_euclid(scale),
        )?;
        position_component(
            "y",
            pos.y(),
            self.min_center_y.div_euclid(40),
            self.max_center_y.div_euclid(40),
        )
    }
}
fn position_component(
    axis: &'static str,
    coordinate: i32,
    minimum: i32,
    maximum: i32,
) -> Result<(), NoiseChunkError> {
    if (minimum..=maximum).contains(&coordinate) {
        Ok(())
    } else {
        Err(fail(NoiseChunkErrorKind::MaterialPositionOutOfRange {
            axis,
            coordinate,
            minimum,
            maximum,
        }))
    }
}
fn sample(
    session: &mut NoiseChunkSession,
    field: usize,
    pos: BlockPos,
    own: bool,
) -> Result<f64, NoiseChunkError> {
    #[cfg(test)]
    let own = session.state.material_context_override.unwrap_or(own);
    session.state.sample(
        &session.data,
        &session.view,
        session.fields[field],
        pos,
        own,
        &mut session.work,
    )
}
fn finite(
    session: &NoiseChunkSession,
    operation: &'static str,
    value: f64,
) -> Result<f64, NoiseChunkError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(crate::density::DensitySampleError {
            resource: session.resource().resource_location(),
            field: operation,
            position: Some(session.state.pos()),
            batch_index: None,
            kind: crate::density::DensitySampleErrorKind::NonFiniteDensity {
                bits: value.to_bits(),
            },
        }
        .into())
    }
}
fn clamped_map(value: f64, min: f64, max: f64, low: f64, high: f64) -> f64 {
    let fraction = (value - min) / (max - min);
    if fraction < 0.0 {
        low
    } else if fraction > 1.0 {
        high
    } else {
        low + fraction * (high - low)
    }
}

fn random_at(
    session: &mut NoiseChunkSession,
    pos: BlockPos,
    factory: &'static str,
) -> Result<(), NoiseChunkError> {
    charge(&mut session.work)?;
    #[cfg(test)]
    if let Some(requests) = session.state.requests.as_mut() {
        requests.record(serde_json::json!({"kind":"random_at","factory":factory,"x":pos.x(),"y":pos.y(),"z":pos.z()}));
    }
    #[cfg(not(test))]
    let _ = (pos, factory);
    Ok(())
}
fn random_float(
    session: &mut NoiseChunkSession,
    random: &mut crate::random::WorldgenRandomSource,
) -> Result<f32, NoiseChunkError> {
    charge(&mut session.work)?;
    #[cfg(test)]
    let value = if let Some(floats) = session.state.material_floats.as_mut() {
        floats.next()?
    } else {
        random.next_float()
    };
    #[cfg(not(test))]
    let value = random.next_float();
    #[cfg(test)]
    if let Some(requests) = session.state.requests.as_mut() {
        requests.record(
            serde_json::json!({"kind":"next_float","bits":format!("{:08x}",value.to_bits())}),
        );
    }
    Ok(value)
}

fn random_int(
    session: &mut NoiseChunkSession,
    random: &mut crate::random::WorldgenRandomSource,
    bound: u32,
) -> Result<i32, NoiseChunkError> {
    let value = random
        .bounded_positive_metered_core(bound, &mut session.work)?
        .cast_signed();
    #[cfg(test)]
    if let Some(requests) = session.state.requests.as_mut() {
        requests.record(serde_json::json!({"kind":"next_int","bound":bound,"value":value}));
    }
    Ok(value)
}

#[cfg(test)]
pub(super) mod tests;

#[cfg(test)]
impl CarverAquifer {
    pub(crate) fn carver_history(&self) -> String {
        format!("{:?}", self.aquifer)
    }
}
