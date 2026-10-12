//! Reusable all-or-nothing output of one driven chunk (policy D3).

use std::mem::{size_of, size_of_val};

use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;

use super::super::{WorldSurfaceHeights, SURFACE_RULE_LEVEL_MIN_Y};
use super::error::limit;
use super::{
    SurfaceColumnError, SurfaceColumnLimit, MAX_SURFACE_CHUNK_BUFFER_BYTES,
    SURFACE_CHUNK_FLUID_WORDS, SURFACE_CHUNK_STATES,
};
use crate::{BlockStateId, WorldSeed};

/// Identity of a published chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Published {
    pub chunk: ChunkPos,
    pub seed: WorldSeed,
    pub resource: NoiseSettingsResourceId,
}

/// Caller-owned output of [`SeededSurfaceColumns::build_chunk`](super::SeededSurfaceColumns::build_chunk).
///
/// Holds exactly two boxes (the 98,304 states in GEN-05.4 material order and
/// the 1,536-word fluid flag bitmap) plus the live `WORLD_SURFACE_WG` heights.
/// It is either `Empty` or `Published` with its chunk, seed and settings; every
/// accessor returns data only when `Published`. Every `build_chunk` call first
/// marks the buffer `Empty`, and a refusal leaves it `Empty`.
#[derive(Debug)]
pub struct SurfaceChunkBuffer {
    pub(super) states: Box<[BlockStateId]>,
    pub(super) fluid: Box<[u64]>,
    pub(super) heights: WorldSurfaceHeights,
    pub(super) published: Option<Published>,
}

impl SurfaceChunkBuffer {
    pub(crate) fn carver_contents(
        &self,
    ) -> Option<(&[BlockStateId], &[u64], &WorldSurfaceHeights)> {
        self.published?;
        Some((&self.states, &self.fluid, &self.heights))
    }
    /// Allocates an `Empty` buffer after checking its retained-byte cap.
    pub fn new() -> Result<Self, SurfaceColumnError> {
        Self::new_with(MAX_SURFACE_CHUNK_BUFFER_BYTES)
    }

    pub(super) fn new_with(cap: usize) -> Result<Self, SurfaceColumnError> {
        limit(SurfaceColumnLimit::BufferBytes, Self::RETAINED_BYTES, cap)?;
        Ok(Self {
            states: vec![BlockStateId::new(0); SURFACE_CHUNK_STATES].into_boxed_slice(),
            fluid: vec![0; SURFACE_CHUNK_FLUID_WORDS].into_boxed_slice(),
            heights: WorldSurfaceHeights::new([[SURFACE_RULE_LEVEL_MIN_Y - 1; 16]; 16]),
            published: None,
        })
    }

    /// Retained bytes of every buffer: the value plus both boxes.
    const RETAINED_BYTES: usize = size_of::<Self>()
        + SURFACE_CHUNK_STATES * size_of::<BlockStateId>()
        + SURFACE_CHUNK_FLUID_WORDS * size_of::<u64>();

    /// Measured retained native bytes (value plus both boxes).
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>() + size_of_val(&*self.states) + size_of_val(&*self.fluid)
    }

    /// Chunk, seed and settings of the published result, or `None` when `Empty`.
    pub const fn published(&self) -> Option<(ChunkPos, WorldSeed, NoiseSettingsResourceId)> {
        match self.published {
            Some(published) => Some((published.chunk, published.seed, published.resource)),
            None => None,
        }
    }

    /// Final state at a block of the published chunk; `None` when `Empty`,
    /// outside the chunk or outside `[-64, 319]`.
    pub fn state(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.states.get(self.index(pos)?).copied()
    }

    /// Final `WORLD_SURFACE_WG` heights (top non-air Y or −65) when `Published`.
    pub const fn world_surface_heights(&self) -> Option<&WorldSurfaceHeights> {
        match self.published {
            Some(_) => Some(&self.heights),
            None => None,
        }
    }

    /// Fluid post-processing flag of a block (the GEN-05.4 material flags plus
    /// every surface write of a fluid state, policy D5); `None` as [`Self::state`].
    pub fn fluid_flag(&self, pos: BlockPos) -> Option<bool> {
        let index = self.index(pos)?;
        let word = self.fluid.get(index / 64)?;
        Some(word & (1_u64 << (index % 64)) != 0)
    }

    fn index(&self, pos: BlockPos) -> Option<usize> {
        let published = self.published?;
        if pos.to_chunk_pos() != published.chunk {
            return None;
        }
        let level = usize::try_from(pos.y().checked_sub(SURFACE_RULE_LEVEL_MIN_Y)?).ok()?;
        if level >= SURFACE_CHUNK_STATES / 256 {
            return None;
        }
        let (lx, lz) = ((pos.x() & 15) as usize, (pos.z() & 15) as usize);
        Some((level * 16 + lx) * 16 + lz)
    }
}

#[cfg(test)]
impl SurfaceChunkBuffer {
    pub(crate) fn carver_test_identity(
        &mut self,
        identity: Option<(ChunkPos, WorldSeed, NoiseSettingsResourceId)>,
    ) {
        self.published = identity.map(|(chunk, seed, resource)| Published {
            chunk,
            seed,
            resource,
        });
    }
}
