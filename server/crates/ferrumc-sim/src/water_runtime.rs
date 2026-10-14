//! Explicit legacy tick runtime; queue state grants no native durable authority.

use crate::{WaterRules, WaterTickQueue};

#[derive(Debug)]
pub(crate) struct WaterRuntime {
    pub(crate) live: bool,
    pub(crate) world: ferrumc_core::WorldId,
    pub(crate) dimension: ferrumc_core::DimensionId,
    pub(crate) now: u64,
    pub(crate) conversion: bool,
    pub(crate) rules: WaterRules,
    pub(crate) queue: WaterTickQueue,
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
        }
    }
}
