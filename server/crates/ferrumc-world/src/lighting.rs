//! Bounded cold relighting of a chunk and its loaded one-chunk halo.
//!
//! Every call starts from actual blocks, so removals cannot retain stale light.
//! Missing neighbours are closed boundaries. Callers relight affected resident
//! centres after block edits and after neighbour load/unload, then publish the
//! complete resulting snapshot. No world mutation, scheduling or I/O occurs here.
use std::collections::VecDeque;

use ferrumc_math::{ChunkPos, LocalBlockPos};

use crate::{BlockStateId, Chunk, ChunkLightData, SECTION_COUNT};

const WIDTH: usize = 48;
const HEIGHT: usize = SECTION_COUNT * 16;
const PLANE: usize = WIDTH * WIDTH;
const CELLS: usize = PLANE * HEIGHT;
const PROPERTIES: &[u8] = include_bytes!("light_data/properties.bin");
const FACES: &[u8] = include_bytes!("light_data/faces.bin");
// The original Direction enum order: DOWN, UP, NORTH, SOUTH, WEST, EAST.
const OPPOSITE: [usize; 6] = [1, 0, 3, 2, 5, 4];

/// A malformed lighting input, refused before a snapshot is published.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LightError {
    /// A supplied chunk is outside the centre's 3×3 window.
    #[error("chunk {0:?} is outside the lighting neighborhood")]
    OutsideNeighborhood(ChunkPos),
    /// A supplied chunk key occurs more than once.
    #[error("duplicate lighting chunk {0:?}")]
    DuplicateChunk(ChunkPos),
    /// The centre itself was not supplied.
    #[error("lighting neighborhood has no centre chunk")]
    MissingCenter,
    /// More than nine chunks were supplied.
    #[error("lighting neighborhood contains more than nine chunks")]
    TooManyChunks,
    /// A palette contains a state absent from the pinned Paper state table.
    #[error("unknown lighting block state {0}")]
    UnknownState(u32),
}

/// Borrowed immutable blocks for one centre and at most eight loaded neighbours.
///
/// A full one-chunk halo covers every non-source light path reaching the centre:
/// propagation loses at least one level per edge and no level exceeds 15. Missing
/// neighbours are opaque, never substituted with air or freshly generated blocks.
/// Compute this synchronous work on a bounded producer lane. Relight the centre
/// when its blocks or any loaded neighbour change, arrive, or depart.
pub struct LightingNeighborhood<'a> {
    center: ChunkPos,
    chunks: [Option<&'a Chunk>; 9],
}
impl<'a> LightingNeighborhood<'a> {
    /// Checks keys and captures at most nine immutable chunk references.
    pub fn new(center: ChunkPos, chunks: &[&'a Chunk]) -> Result<Self, LightError> {
        if chunks.len() > 9 {
            return Err(LightError::TooManyChunks);
        }
        let mut result = Self {
            center,
            chunks: [None; 9],
        };
        for chunk in chunks {
            let dx = i64::from(chunk.pos().x()) - i64::from(center.x());
            let dz = i64::from(chunk.pos().z()) - i64::from(center.z());
            if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) {
                return Err(LightError::OutsideNeighborhood(chunk.pos()));
            }
            let index = ((dz + 1) * 3 + dx + 1) as usize;
            if result.chunks[index].replace(chunk).is_some() {
                return Err(LightError::DuplicateChunk(chunk.pos()));
            }
        }
        if result.chunks[4].is_none() {
            return Err(LightError::MissingCenter);
        }
        Ok(result)
    }

    /// Returns the centre whose complete light snapshot this input produces.
    #[must_use]
    pub const fn center(&self) -> ChunkPos {
        self.center
    }

    /// Rebuilds skylight and emitted block light, including lateral halo paths.
    ///
    /// Uses actual pinned opacity, emission and face occlusion for every state.
    /// At most 884,736 cells, that many queued indices, and 15 improvements per
    /// cell/layer exist. Storage and work are therefore bounded independently of
    /// the number of edits. Runtime piston/world-dependent occlusion is outside
    /// this static block-state engine. The two sentinel sections are sky-open
    /// above the ceiling and vertically extended floor skylight below the floor;
    /// sentinel block light is zero.
    pub fn compute(&self) -> Result<ChunkLightData, LightError> {
        let bedrock = ferrumc_registry::block_state::BEDROCK;
        let mut states = vec![bedrock; CELLS];
        for (slot, chunk) in self.chunks.iter().enumerate() {
            let Some(chunk) = chunk else { continue };
            let offset_x = (slot % 3) * 16;
            let offset_z = (slot / 3) * 16;
            for (section, blocks) in chunk.sections().iter().enumerate() {
                for y in 0..16_u8 {
                    for z in 0..16_u8 {
                        for x in 0..16_u8 {
                            let Some(local) = LocalBlockPos::new(x, y, z) else {
                                continue;
                            };
                            let raw = blocks.get(local).as_u32();
                            if raw as usize >= PROPERTIES.len() / 13 {
                                return Err(LightError::UnknownState(raw));
                            }
                            states[(section * 16 + usize::from(y)) * PLANE
                                + (offset_z + usize::from(z)) * WIDTH
                                + offset_x
                                + usize::from(x)] = raw;
                        }
                    }
                }
            }
        }
        let mut sky = vec![0_u8; CELLS];
        let mut block = vec![0_u8; CELLS];
        for z in 0..WIDTH {
            for x in 0..WIDTH {
                let mut open = true;
                let mut above = 0;
                for y in (0..HEIGHT).rev() {
                    let index = y * PLANE + z * WIDTH + x;
                    let raw = states[index];
                    open &= opacity(raw) == 0 && !occludes(above, raw, 0);
                    if open {
                        sky[index] = 15;
                    }
                    block[index] = emission(raw);
                    above = raw;
                }
            }
        }
        propagate(&states, &mut sky);
        propagate(&states, &mut block);
        let mut sky_sections = vec![vec![0_u8; 2048]; SECTION_COUNT + 2];
        let mut block_sections = vec![vec![0_u8; 2048]; SECTION_COUNT + 2];
        sky_sections[SECTION_COUNT + 1].fill(0xff);
        for y in 0..HEIGHT {
            for z in 0..16 {
                for x in 0..16 {
                    let index = y * PLANE + (z + 16) * WIDTH + x + 16;
                    let local = (y % 16) * 256 + z * 16 + x;
                    sky_sections[y / 16 + 1][local / 2] |= sky[index] << ((local % 2) * 4);
                    block_sections[y / 16 + 1][local / 2] |= block[index] << ((local % 2) * 4);
                    if y == 0 {
                        for below_y in 0..16 {
                            let below = below_y * 256 + z * 16 + x;
                            sky_sections[0][below / 2] |= sky[index] << ((below % 2) * 4);
                        }
                    }
                }
            }
        }
        Ok(ChunkLightData::from_sections(sky_sections, block_sections))
    }
}

fn opacity(raw: u32) -> u8 {
    PROPERTIES[raw as usize * 13] & 15
}
fn emission(raw: u32) -> u8 {
    PROPERTIES[raw as usize * 13] >> 4
}
fn face(raw: u32, direction: usize) -> usize {
    let offset = raw as usize * 13 + 1 + direction * 2;
    usize::from(u16::from_be_bytes([
        PROPERTIES[offset],
        PROPERTIES[offset + 1],
    ])) * 32
}
fn occludes(from: u32, to: u32, direction: usize) -> bool {
    let a = face(from, direction);
    let b = face(to, OPPOSITE[direction]);
    (0..32).all(|byte| FACES[a + byte] | FACES[b + byte] == 255)
}
fn neighbors(index: usize) -> [Option<usize>; 6] {
    let y = index / PLANE;
    let z = (index % PLANE) / WIDTH;
    let x = index % WIDTH;
    [
        (y > 0).then(|| index - PLANE),
        (y + 1 < HEIGHT).then_some(index + PLANE),
        (z > 0).then(|| index - WIDTH),
        (z + 1 < WIDTH).then_some(index + WIDTH),
        (x > 0).then(|| index - 1),
        (x + 1 < WIDTH).then_some(index + 1),
    ]
}
fn propagate(states: &[u32], values: &mut [u8]) {
    // One admission bit per cell bounds the live queue to CELLS even when a
    // different source improves a cell before its earlier entry is processed.
    let mut queued = vec![false; CELLS];
    let mut queue = VecDeque::with_capacity(CELLS);
    for index in 0..CELLS {
        if values[index] > 1
            && neighbors(index)
                .iter()
                .flatten()
                .any(|&next| values[next] + 1 < values[index])
        {
            queued[index] = true;
            queue.push_back(index);
        }
    }
    while let Some(index) = queue.pop_front() {
        queued[index] = false;
        let level = values[index];
        for (direction, next) in neighbors(index).into_iter().enumerate() {
            let Some(next) = next else { continue };
            let candidate = level.saturating_sub(opacity(states[next]).max(1));
            if candidate > values[next] && !occludes(states[index], states[next], direction) {
                values[next] = candidate;
                if candidate > 1 && !queued[next] {
                    queued[next] = true;
                    queue.push_back(next);
                }
            }
        }
    }
}

/// Returns the pinned state emission, or refuses an unknown state.
pub fn block_light_emission(state: BlockStateId) -> Result<u8, LightError> {
    if state.as_u32() as usize >= PROPERTIES.len() / 13 {
        return Err(LightError::UnknownState(state.as_u32()));
    }
    Ok(emission(state.as_u32()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_math::BlockPos;
    fn state(name: &str) -> BlockStateId {
        BlockStateId::new(ferrumc_registry::block_state::block_default_state(name).unwrap())
    }
    fn level(data: &ChunkLightData, sky: bool, pos: BlockPos) -> u8 {
        let (mask, arrays) = if sky {
            (data.sky_light_mask(), data.sky_light())
        } else {
            (data.block_light_mask(), data.block_light())
        };
        let section = ((pos.y() + 64) / 16 + 1) as usize;
        let word = mask.first().copied().unwrap_or(0) as u64;
        if word & (1 << section) == 0 {
            return 0;
        }
        let ordinal = (word & ((1 << section) - 1)).count_ones() as usize;
        let index =
            (pos.y() & 15) as usize * 256 + (pos.z() & 15) as usize * 16 + (pos.x() & 15) as usize;
        (arrays[ordinal][index / 2] >> ((index % 2) * 4)) & 15
    }
    #[test]
    fn pinned_light_properties_and_faces() {
        assert_eq!(PROPERTIES.len(), 27946 * 13);
        assert_eq!(FACES.len(), 38 * 32);
        assert_eq!(opacity(state("glass").as_u32()), 0);
        assert_eq!(opacity(state("water").as_u32()), 1);
        assert_eq!(opacity(state("oak_leaves").as_u32()), 1);
        assert_eq!(block_light_emission(state("torch")), Ok(14));
        assert_eq!(block_light_emission(state("lava")), Ok(15));
        let slab = ferrumc_registry::block_state::compute_state_id(
            "stone_slab",
            &std::collections::BTreeMap::from([("type", "bottom"), ("waterlogged", "false")]),
        )
        .unwrap();
        assert!(occludes(0, slab, 1)); // underside of a bottom slab
        assert!(!occludes(0, slab, 0)); // its upper half is open
    }
    #[test]
    fn skylight_lateral_torch_removal_and_roof() {
        let mut chunk = Chunk::new(ChunkPos::ORIGIN);
        for x in 0..16 {
            for z in 0..16 {
                chunk
                    .set_block(BlockPos::new(x, 10, z), state("stone"))
                    .unwrap();
            }
        }
        chunk
            .set_block(BlockPos::new(8, 10, 8), state("glass"))
            .unwrap();
        chunk
            .set_block(BlockPos::new(3, 3, 3), state("torch"))
            .unwrap();
        let lit = LightingNeighborhood::new(chunk.pos(), &[&chunk])
            .unwrap()
            .compute()
            .unwrap();
        assert_eq!(level(&lit, true, BlockPos::new(8, 9, 8)), 15);
        assert_eq!(level(&lit, true, BlockPos::new(9, 9, 8)), 14);
        assert_eq!(level(&lit, false, BlockPos::new(3, 3, 3)), 14);
        assert_eq!(level(&lit, false, BlockPos::new(4, 3, 3)), 13);
        assert_eq!(level(&lit, true, BlockPos::new(0, 3, 0)), 0);
        chunk
            .set_block(BlockPos::new(3, 3, 3), BlockStateId::AIR)
            .unwrap();
        chunk
            .set_block(BlockPos::new(8, 10, 8), state("stone"))
            .unwrap();
        let dark = LightingNeighborhood::new(chunk.pos(), &[&chunk])
            .unwrap()
            .compute()
            .unwrap();
        assert_eq!(dark.block_light().len(), 0);
        assert_eq!(level(&dark, true, BlockPos::new(8, 9, 8)), 0);
    }
    #[test]
    fn border_sources_and_neighbor_arrival_are_order_independent() {
        let center = Chunk::new(ChunkPos::new(-1, 0));
        let mut east = Chunk::new(ChunkPos::ORIGIN);
        east.set_block(BlockPos::new(0, 0, 5), state("torch"))
            .unwrap();
        let a = LightingNeighborhood::new(center.pos(), &[&center, &east])
            .unwrap()
            .compute()
            .unwrap();
        let b = LightingNeighborhood::new(center.pos(), &[&east, &center])
            .unwrap()
            .compute()
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(level(&a, false, BlockPos::new(-1, 0, 5)), 13);
        assert_eq!(level(&a, false, BlockPos::new(-14, 0, 5)), 0);
        let unloaded = LightingNeighborhood::new(center.pos(), &[&center])
            .unwrap()
            .compute()
            .unwrap();
        assert_eq!(unloaded.block_light().len(), 0);
    }
    #[test]
    fn malformed_input_is_refused() {
        let chunk = Chunk::new(ChunkPos::ORIGIN);
        assert!(matches!(
            LightingNeighborhood::new(chunk.pos(), &[]),
            Err(LightError::MissingCenter)
        ));
        assert!(matches!(
            LightingNeighborhood::new(chunk.pos(), &[&chunk, &chunk]),
            Err(LightError::DuplicateChunk(_))
        ));
        assert!(matches!(
            LightingNeighborhood::new(ChunkPos::new(i32::MIN, 0), &[&chunk]),
            Err(LightError::OutsideNeighborhood(_))
        ));
        let mut bad = chunk.clone();
        bad.set_block(BlockPos::new(1, 0, 1), BlockStateId::new(u32::MAX))
            .unwrap();
        assert!(matches!(
            LightingNeighborhood::new(bad.pos(), &[&bad])
                .unwrap()
                .compute(),
            Err(LightError::UnknownState(u32::MAX))
        ));
    }
}
