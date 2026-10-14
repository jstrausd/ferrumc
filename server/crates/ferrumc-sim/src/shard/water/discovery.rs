//! Resident column wakeups, with scalar cursors instead of column copies.

use std::ops::Bound::{Excluded, Unbounded};

use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::dimension::{HEIGHT, MIN_Y};

use crate::water_runtime::{
    WaterDiscovery, WaterRuntime, MAX_WATER_DISCOVERY_COLUMNS, WATER_DISCOVERY_COLUMN_SLICE,
    WATER_DISCOVERY_READS,
};
use crate::{SimShard, WaterError, WATER_TICK_DELAY};

impl SimShard {
    /// Wakes a newly resident column and its resident cardinal border neighbors.
    ///
    /// Call once after the first successful acquisition, including restored
    /// columns. Actual discovery and flow run at tick boundaries. At most 4096
    /// scalar column cursors are retained; overflow requests a resumable sweep of
    /// resident columns rather than dropping the arrival or invalidating tickets.
    /// Scheduling pressure pauses at the exact cell, and never loads a neighbor.
    /// Disabled water is a no-op; copied/native owners cannot activate this work.
    pub fn wake_water_chunk(&mut self, chunk: ChunkPos) -> Result<(), WaterError> {
        let Some(water) = self.water.as_mut() else {
            return Ok(());
        };
        if !water.live {
            return Err(WaterError::SnapshotOnly);
        }
        if !self.chunks.is_loaded(chunk) {
            return Err(WaterError::ChunkNotLoaded);
        }
        let mut positions = [None; 5];
        positions[0] = Some(chunk);
        for (cell, (dx, dz)) in positions[1..]
            .iter_mut()
            .zip([(0, -1), (1, 0), (0, 1), (-1, 0)])
        {
            if let (Some(x), Some(z)) = (chunk.x().checked_add(dx), chunk.z().checked_add(dz)) {
                let neighbor = ChunkPos::new(x, z);
                if self.chunks.is_loaded(neighbor) {
                    *cell = Some(neighbor);
                }
            }
        }
        let added = positions
            .iter()
            .flatten()
            .filter(|pos| !water.discovery.contains_key(pos))
            .count();
        if water.discovery.len() + added > MAX_WATER_DISCOVERY_COLUMNS {
            if water.sweep.active {
                water.sweep.restart = true;
            } else {
                water.sweep.active = true;
                water.sweep.after = None;
            }
            return Ok(());
        }
        for pos in positions.into_iter().flatten() {
            // A neighbor may have scanned its border before this arrival. Rewind
            // that cursor so an earlier settled source sees the new passage.
            water.discovery.insert(pos, WaterDiscovery::default());
        }
        Ok(())
    }

    pub(super) fn discover_water(&self, water: &mut WaterRuntime) {
        // Initial seeding and arrival overflow visit ordered live keys with a
        // bounded range lookup. No whole-resident-set copy or scan is needed.
        for _ in 0..4 {
            if !water.sweep.active || water.discovery.len() == MAX_WATER_DISCOVERY_COLUMNS {
                break;
            }
            if let Some(pos) = self.chunks.next_loaded_position(water.sweep.after) {
                water.discovery.entry(pos).or_default();
                water.sweep.after = Some(pos);
            } else {
                water.sweep.active = water.sweep.restart;
                water.sweep.restart = false;
                water.sweep.after = None;
            }
        }
        let Some(due) = water.now.checked_add(WATER_TICK_DELAY) else {
            return;
        };
        let mut allowance = WATER_DISCOVERY_READS;
        while allowance > 0 {
            let next = water
                .discovery_cursor
                .and_then(|pos| water.discovery.range((Excluded(pos), Unbounded)).next())
                .or_else(|| water.discovery.first_key_value())
                .map(|(&pos, &scan)| (pos, scan));
            let Some((pos, mut scan)) = next else {
                break;
            };
            water.discovery_cursor = Some(pos);
            let Some(chunk) = self.chunks.get(pos) else {
                water.discovery.remove(&pos);
                allowance -= 1;
                continue;
            };
            let limit = allowance.min(WATER_DISCOVERY_COLUMN_SLICE);
            let mut consumed = 0;
            let mut blocked = false;
            while consumed < limit && scan.cell < HEIGHT as usize * 256 {
                // Top-down reaches natural surfaces before buried flow. Empty
                // sections consume one work unit, not 4096 air-state lookups.
                let relative_y = HEIGHT as usize - 1 - scan.cell / 256;
                if chunk
                    .section(relative_y / 16)
                    .is_some_and(ferrumc_world::ChunkSection::is_empty)
                {
                    scan.cell = (scan.cell / 4096 + 1) * 4096;
                    consumed += 1;
                    continue;
                }
                let (Ok(x), Ok(y), Ok(z)) = (
                    i32::try_from(scan.cell & 15),
                    i32::try_from(relative_y),
                    i32::try_from((scan.cell >> 4) & 15),
                ) else {
                    panic!("bounded discovery coordinate is not representable");
                };
                let position = BlockPos::new(pos.x() * 16 + x, MIN_Y + y, pos.z() * 16 + z);
                consumed += 1;
                let state = chunk.get_block(position);
                let active = state.is_some_and(|state| {
                    water
                        .rules
                        .needs_initial_tick(position, state, |p| {
                            self.chunks
                                .get(p.to_chunk_pos())
                                .and_then(|column| column.get_block(p))
                        })
                        // Actual tick publishes the typed unsupported-state
                        // refusal. Such a cell cannot suppress other discovery.
                        .unwrap_or(true)
                });
                if active {
                    if !water.queue.discovery_room(position) {
                        blocked = true;
                        break;
                    }
                    if let Err(error) = water.queue.schedule_at(due, position) {
                        panic!("preflighted discovery admission failed: {error}");
                    }
                }
                scan.cell += 1;
            }
            allowance -= consumed;
            if scan.cell >= HEIGHT as usize * 256 {
                water.discovery.remove(&pos);
            } else {
                water.discovery.insert(pos, scan);
            }
            // Queue fullness is global: preserve this exact cursor and let due
            // ticks release room, instead of burning the remaining scan budget.
            if blocked {
                break;
            }
        }
    }
}
