//! Independent all-or-nothing target publication, never a Surface mutation.

use super::{CarverError, CarverErrorKind, CarverLimit, MAX_CARVED_BUFFER_BYTES};
use crate::surface_rules::WorldSurfaceHeights;
use crate::{BlockStateId, WorldSeed};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;
use std::mem::{size_of, size_of_val};

pub(super) const STATES: usize = 16 * 16 * 384;
pub(super) const WORDS: usize = STATES / 64;

/// Owned complete target, with states, fluid set, live heights and carving mask.
/// Every invocation first makes this buffer Empty; accessors then expose no data.
#[derive(Debug)]
pub struct CarvedChunkBuffer {
    pub(super) states: Box<[BlockStateId]>,
    pub(super) fluid: Box<[u64]>,
    pub(super) mask: Box<[u64]>,
    pub(super) heights: WorldSurfaceHeights,
    pub(super) identity: Option<(ChunkPos, WorldSeed, NoiseSettingsResourceId)>,
}
impl CarvedChunkBuffer {
    /// Allocates the three bounded boxes after native admission.
    pub fn new() -> Result<Self, CarverError> {
        Self::new_with(MAX_CARVED_BUFFER_BYTES)
    }
    pub(super) fn new_with(cap: usize) -> Result<Self, CarverError> {
        let required = size_of::<Self>() + STATES * size_of::<BlockStateId>() + 2 * WORDS * 8;
        if required > cap {
            return Err(CarverError::new(CarverErrorKind::NativeLimit {
                category: CarverLimit::Buffer,
                required,
                available: cap,
            }));
        }
        Ok(Self {
            states: vec![BlockStateId::AIR; STATES].into_boxed_slice(),
            fluid: vec![0; WORDS].into_boxed_slice(),
            mask: vec![0; WORDS].into_boxed_slice(),
            heights: WorldSurfaceHeights::new([[-65; 16]; 16]),
            identity: None,
        })
    }
    /// Published chunk, seed and settings, or Empty.
    pub const fn published(&self) -> Option<(ChunkPos, WorldSeed, NoiseSettingsResourceId)> {
        self.identity
    }
    /// Exact retained value and box payload bytes, excluding allocator metadata.
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + size_of_val(&*self.states)
            + size_of_val(&*self.fluid)
            + size_of_val(&*self.mask)
    }
    /// State only inside the fully published target's build height.
    pub fn state(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.states.get(self.index(pos)?).copied()
    }
    /// Fluid postprocessing set membership only inside the published target.
    pub fn fluid_flag(&self, pos: BlockPos) -> Option<bool> {
        let i = self.index(pos)?;
        Some(self.fluid[i / 64] & (1 << (i % 64)) != 0)
    }
    /// Carving-mask membership only inside the published target.
    pub fn carving_mask(&self, pos: BlockPos) -> Option<bool> {
        self.index(pos)?;
        let i = mask_index(pos);
        Some(self.mask[i / 64] & (1 << (i % 64)) != 0)
    }
    /// Complete live `WORLD_SURFACE_WG` after all successful writes.
    pub const fn world_surface_heights(&self) -> Option<&WorldSurfaceHeights> {
        match self.identity {
            Some(_) => Some(&self.heights),
            None => None,
        }
    }
    pub(super) fn index(&self, pos: BlockPos) -> Option<usize> {
        if self.identity?.0 != pos.to_chunk_pos() {
            return None;
        }
        state_index(pos)
    }
}
pub(super) fn state_index(pos: BlockPos) -> Option<usize> {
    let y = i64::from(pos.y()) + 64;
    if !(0..384).contains(&y) {
        return None;
    }
    Some(((y as usize * 16) + (pos.x() & 15) as usize) * 16 + (pos.z() & 15) as usize)
}
pub(super) fn mask_index(pos: BlockPos) -> usize {
    (pos.x() & 15) as usize | ((pos.z() & 15) as usize) << 4 | ((pos.y() + 64) as usize) << 8
}
