//! Bounded legacy ordinary-furnace scheduling; World owns slots and timers.

use ferrumc_core::{DimensionId, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use std::collections::BTreeSet;

/// Refusal before furnace ownership or a complete tick postimage is committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FurnaceError {
    /// Capacity must be one through 1024 registered loaded furnaces.
    #[error("invalid furnace capacity")]
    InvalidCapacity,
    /// Native durable ownership and legacy furnace ticking are exclusive.
    #[error("furnace tick owner unavailable")]
    OwnershipRequired,
    /// Only one live runtime may be selected for this shard.
    #[error("furnaces already initialized")]
    AlreadyInitialized,
    /// A newly discovered loaded furnace cannot obtain a registered tick slot.
    #[error("furnace registry full")]
    Capacity,
    /// The full original slots/timers failed native or planner admission.
    #[error("invalid furnace data")]
    InvalidData,
    /// The lit-state transition failed admission; slots and fuel stay unchanged.
    #[error("furnace block transition refused")]
    BlockRefused,
}

#[derive(Debug)]
pub(crate) struct FurnaceRuntime {
    pub(crate) live: bool,
    pub(crate) world: WorldId,
    pub(crate) dimension: DimensionId,
    pub(crate) capacity: usize,
    pub(crate) registered: BTreeSet<BlockPos>,
    pub(crate) scan_chunk: Option<ChunkPos>,
    pub(crate) scan_block: Option<BlockPos>,
}
impl Clone for FurnaceRuntime {
    fn clone(&self) -> Self {
        Self {
            live: false,
            world: self.world,
            dimension: self.dimension,
            capacity: self.capacity,
            registered: self.registered.clone(),
            scan_chunk: self.scan_chunk,
            scan_block: self.scan_block,
        }
    }
}
