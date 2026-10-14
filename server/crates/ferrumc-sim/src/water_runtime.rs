//! Explicit legacy tick runtime; queue state grants no native durable authority.

use std::collections::BTreeMap;

use ferrumc_math::ChunkPos;

use crate::{WaterRules, WaterTickQueue};

// A cursor, never a copy of column data. Overflow uses a resumable resident sweep
// instead of dropping an arrival or accumulating another unbounded retry list.
pub(crate) const MAX_WATER_DISCOVERY_COLUMNS: usize = 4096;
pub(crate) const WATER_DISCOVERY_READS: usize = 8192;
pub(crate) const WATER_DISCOVERY_COLUMN_SLICE: usize = 256;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WaterDiscovery {
    pub(crate) cell: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WaterSweep {
    pub(crate) active: bool,
    pub(crate) restart: bool,
    pub(crate) after: Option<ChunkPos>,
}

#[derive(Debug)]
pub(crate) struct WaterRuntime {
    pub(crate) live: bool,
    pub(crate) world: ferrumc_core::WorldId,
    pub(crate) dimension: ferrumc_core::DimensionId,
    pub(crate) now: u64,
    pub(crate) conversion: bool,
    pub(crate) rules: WaterRules,
    pub(crate) queue: WaterTickQueue,
    pub(crate) discovery: BTreeMap<ChunkPos, WaterDiscovery>,
    pub(crate) discovery_cursor: Option<ChunkPos>,
    pub(crate) sweep: WaterSweep,
}
impl Clone for WaterRuntime {
    fn clone(&self) -> Self {
        Self {
            live: false,
            world: self.world,
            dimension: self.dimension,
            now: self.now,
            conversion: self.conversion,
            rules: self.rules.clone(),
            queue: self.queue.clone(),
            discovery: self.discovery.clone(),
            discovery_cursor: self.discovery_cursor,
            sweep: self.sweep,
        }
    }
}
