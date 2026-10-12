//! Closed material states resolved once through the existing pinned registry.

use std::collections::BTreeMap;

use ferrumc_registry::block_state::{
    block_default_state, block_metadata, compute_state_id, state_id_to_block_name,
};

use super::super::{
    core_charge as charge, core_fail as fail, limit, NoiseChunkCoreError as NoiseChunkError,
    NoiseChunkErrorKind, NoiseChunkLimit,
};
use crate::density::json::{self, ValueKind};
use crate::sample_work::SampleWork;
use crate::BlockStateId;

pub(super) struct MaterialBlocks {
    pub default_block: BlockStateId,
    pub default_fluid: BlockStateId,
    pub air: BlockStateId,
    pub lava: BlockStateId,
    pub copper: BlockStateId,
    pub raw_copper: BlockStateId,
    pub granite: BlockStateId,
    pub iron: BlockStateId,
    pub raw_iron: BlockStateId,
    pub tuff: BlockStateId,
    water_range: (u32, u32),
    lava_range: (u32, u32),
}

impl MaterialBlocks {
    pub fn new(
        default_block: &str,
        default_fluid: &str,
        work: &mut SampleWork,
    ) -> Result<Self, NoiseChunkError> {
        let default_block = resolve(default_block, "default_block", work)?;
        let default_fluid = resolve(default_fluid, "default_fluid", work)?;
        // The public input is the seven immutable builtin settings. Supporting
        // arbitrary fluid-carrying blocks requires an actual fluid-state table,
        // rather than an inference from solidity or a name fragment.
        charge(work)?;
        if !matches!(
            state_id_to_block_name(default_block.as_u32()),
            Some("stone" | "netherrack" | "end_stone")
        ) {
            return Err(invalid("default_block"));
        }
        charge(work)?;
        if !matches!(
            state_id_to_block_name(default_fluid.as_u32()),
            Some("air" | "water" | "lava")
        ) {
            return Err(invalid("default_fluid"));
        }
        let water_range = range("water", work)?;
        let lava_range = range("lava", work)?;
        Ok(Self {
            default_block,
            default_fluid,
            air: fixed("air", work)?,
            lava: fixed("lava", work)?,
            copper: fixed("copper_ore", work)?,
            raw_copper: fixed("raw_copper_block", work)?,
            granite: fixed("granite", work)?,
            iron: fixed("deepslate_iron_ore", work)?,
            raw_iron: fixed("raw_iron_block", work)?,
            tuff: fixed("tuff", work)?,
            water_range,
            lava_range,
        })
    }
    pub fn is_water(&self, state: BlockStateId) -> bool {
        (self.water_range.0..=self.water_range.1).contains(&state.as_u32())
    }
    pub fn is_lava(&self, state: BlockStateId) -> bool {
        (self.lava_range.0..=self.lava_range.1).contains(&state.as_u32())
    }
    pub fn has_fluid(&self, state: BlockStateId) -> bool {
        self.is_water(state) || self.is_lava(state)
    }
}

fn range(name: &'static str, work: &mut SampleWork) -> Result<(u32, u32), NoiseChunkError> {
    charge(work)?;
    let meta = block_metadata(name).ok_or_else(|| invalid(name))?;
    Ok((meta.min_state, meta.max_state))
}
fn fixed(name: &'static str, work: &mut SampleWork) -> Result<BlockStateId, NoiseChunkError> {
    charge(work)?;
    block_default_state(name)
        .map(BlockStateId::new)
        .ok_or_else(|| invalid(name))
}
pub(super) fn invalid(field: &'static str) -> NoiseChunkError {
    fail(NoiseChunkErrorKind::InvalidMaterialState { field })
}
pub(super) fn resolve(
    source: &str,
    field: &'static str,
    work: &mut SampleWork,
) -> Result<BlockStateId, NoiseChunkError> {
    limit(NoiseChunkLimit::MaterialResolver, source.len(), 4096)?;
    // Meter the bounded syntax scan before its temporary tree can allocate.
    for _ in source.bytes() {
        charge(work)?;
    }
    let (value, stats) = json::parse(source, "noise_chunk_material").map_err(|_| invalid(field))?;
    for _ in 0..stats.nodes {
        charge(work)?;
    }
    let ValueKind::Object(object) = &value.kind else {
        return Err(invalid(field));
    };
    if object
        .keys()
        .any(|name| name != "Name" && name != "Properties")
    {
        return Err(invalid(field));
    }
    let Some(ValueKind::String(name)) = object.get("Name").map(|v| &v.kind) else {
        return Err(invalid(field));
    };
    let mut properties = BTreeMap::new();
    if let Some(value) = object.get("Properties") {
        let ValueKind::Object(values) = &value.kind else {
            return Err(invalid(field));
        };
        if values.len() > 32 {
            return Err(invalid(field));
        }
        for (key, value) in values {
            charge(work)?;
            let ValueKind::String(value) = &value.kind else {
                return Err(invalid(field));
            };
            properties.insert(key.as_str(), value.as_str());
        }
    }
    charge(work)?;
    compute_state_id(name, &properties)
        .map(BlockStateId::new)
        .ok_or_else(|| invalid(field))
}
