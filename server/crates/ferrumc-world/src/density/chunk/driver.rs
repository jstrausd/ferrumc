//! Complete original X/Z/descending-Y cell driver with one material sink.

#![expect(
    clippy::cast_possible_wrap,
    reason = "Private geometry and provider indices are admitted at no more than 512 or 131072 before signed coordinate conversion."
)]

use ferrumc_math::BlockPos;

use super::fill::{write, Provider};
use super::state::{lerp, Cache, DriverPhase, Lifecycle};
use super::{
    bytes, charge, fail, output_index, ChunkDensitySamples, NoiseChunkError, NoiseChunkErrorKind,
    NoiseChunkLimit, NoiseChunkSession,
};

impl NoiseChunkSession {
    fn require_driver(
        &self,
        operation: &'static str,
        phases: &[DriverPhase],
    ) -> Result<(), NoiseChunkError> {
        self.state.require(Lifecycle::Running, operation)?;
        if self.state.interpolating
            && !self.state.filling_cell
            && phases.contains(&self.state.driver_phase)
        {
            Ok(())
        } else {
            Err(fail(NoiseChunkErrorKind::Lifecycle {
                operation,
                phase: self.state.driver_phase.name(),
            }))
        }
    }
    pub(super) fn initialize_driver(&mut self) -> Result<(), NoiseChunkError> {
        self.state.require(Lifecycle::Running, "initialize")?;
        if self.state.interpolating || self.state.driver_phase != DriverPhase::Dormant {
            return Err(fail(NoiseChunkErrorKind::Lifecycle {
                operation: "initialize",
                phase: self.state.driver_phase.name(),
            }));
        }
        charge(&mut self.work)?;
        self.state.interpolating = true;
        self.state.scalar_counter = 0;
        self.state.driver_phase = DriverPhase::FirstSlice;
        Ok(())
    }
    pub(super) fn sample_material(&mut self) -> Result<f64, NoiseChunkError> {
        self.require_driver("sample_material", &[DriverPhase::ZUpdated])?;
        self.state.sample(
            &self.data,
            &self.view,
            self.material,
            self.state.pos(),
            true,
            &mut self.work,
        )
    }
    pub(super) fn drive(&mut self) -> Result<ChunkDensitySamples, NoiseChunkError> {
        self.state.require(Lifecycle::Running, "drive")?;
        let count = bytes(
            NoiseChunkLimit::Output,
            0,
            self.geometry().height() as usize,
            256,
            131_072,
        )?;
        bytes(NoiseChunkLimit::Output, 0, count, 8, 1024 * 1024)?;
        // Its size is independently derived from the admitted output count:
        // at most 2048 u64s (16KiB), without additional per-block allocation.
        let bitmap_len = count.div_ceil(64);
        bytes(NoiseChunkLimit::Output, 0, bitmap_len, 8, 16 * 1024)?;
        let mut values = vec![0.0; count];
        let mut written = vec![0_u64; bitmap_len];
        let mut writes = 0;
        self.initialize_driver()?;
        self.fill_slice(true, self.state.origin.x() / self.state.width as i32)?;
        for cell_x in 0..self.state.count_xz {
            let first_x = self.state.origin.x() / self.state.width as i32;
            self.fill_slice(false, first_x + cell_x as i32 + 1)?;
            self.state.start = BlockPos::new(
                (first_x + cell_x as i32) * self.state.width as i32,
                self.state.start.y(),
                self.state.start.z(),
            );
            for cell_z in 0..self.state.count_xz {
                for cell_y in (0..self.state.count_y).rev() {
                    self.select_cell(cell_y, cell_z)?;
                    for in_y in (0..self.state.height).rev() {
                        self.state.in_y = in_y as i32;
                        self.update_y(in_y as f64 / self.state.height as f64)?;
                        for in_x in 0..self.state.width {
                            self.state.in_x = in_x as i32;
                            self.update_x(in_x as f64 / self.state.width as f64)?;
                            for in_z in 0..self.state.width {
                                self.state.in_z = in_z as i32;
                                charge(&mut self.work)?;
                                self.state.scalar_counter =
                                    self.state.scalar_counter.wrapping_add(1);
                                self.update_z(in_z as f64 / self.state.width as f64)?;
                                let pos = self.state.pos();
                                let index = output_index(self.chunk, self.geometry(), pos)
                                    .filter(|&i| i < count)
                                    .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
                                if written[index / 64] & (1_u64 << (index % 64)) != 0 {
                                    return Err(fail(NoiseChunkErrorKind::InvalidGraph));
                                }
                                let value = self.sample_material()?;
                                write(&mut values[index], value, pos, &mut self.work)?;
                                charge(&mut self.work)?;
                                written[index / 64] |= 1_u64 << (index % 64);
                                writes += 1;
                            }
                        }
                    }
                }
            }
            self.swap_slices()?;
        }
        if !self.state.interpolating || writes != count {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        self.stop_driver()?;
        Ok(ChunkDensitySamples {
            seed: self.seed(),
            resource: self.resource(),
            chunk: self.chunk,
            geometry: self.geometry(),
            values: values.into_boxed_slice(),
        })
    }
    pub(super) fn fill_slice(&mut self, first: bool, x: i32) -> Result<(), NoiseChunkError> {
        self.require_driver(
            "fill_slice",
            &[if first {
                DriverPhase::FirstSlice
            } else {
                DriverPhase::NeedAdvance
            }],
        )?;
        let expected = self.state.origin.x() / self.state.width as i32
            + if first {
                0
            } else {
                self.state.driver_x as i32 + 1
            };
        if x != expected || self.state.driver_x >= self.state.count_xz {
            return Err(fail(NoiseChunkErrorKind::Lifecycle {
                operation: "fill_slice",
                phase: "wrong_slice_column",
            }));
        }
        charge(&mut self.work)?;
        self.state.start = BlockPos::new(
            x * self.state.width as i32,
            self.state.start.y(),
            self.state.start.z(),
        );
        self.state.in_x = 0;
        for z in 0..=self.state.count_xz {
            self.state.start = BlockPos::new(
                self.state.start.x(),
                self.state.start.y(),
                self.state.origin.z() + z as i32 * self.state.width as i32,
            );
            self.state.in_z = 0;
            charge(&mut self.work)?;
            self.state.array_counter = self.state.array_counter.wrapping_add(1);
            for i in 0..self.state.interpolators.len() {
                let id = self.state.interpolators[i];
                let root = self.state.wrappers[id].node;
                let mut array = match &mut self.state.wrappers[id].cache {
                    Cache::Interpolated(interp) => std::mem::take(if first {
                        &mut interp.slice0
                    } else {
                        &mut interp.slice1
                    }),
                    _ => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
                };
                let start = z * (self.state.count_y + 1);
                let result = self.state.fill(
                    &self.data,
                    &self.view,
                    root,
                    &mut array[start..=start + self.state.count_y],
                    Provider::Slice,
                    &mut self.work,
                );
                if let Cache::Interpolated(interp) = &mut self.state.wrappers[id].cache {
                    if first {
                        interp.slice0 = array;
                    } else {
                        interp.slice1 = array;
                    }
                }
                result?;
            }
        }
        charge(&mut self.work)?;
        self.state.array_counter = self.state.array_counter.wrapping_add(1);
        self.state.driver_phase = if first {
            DriverPhase::NeedAdvance
        } else {
            DriverPhase::SlicesReady
        };
        Ok(())
    }
    pub(super) fn select_cell(&mut self, y: usize, z: usize) -> Result<(), NoiseChunkError> {
        self.require_driver(
            "select_cell",
            &[
                DriverPhase::SlicesReady,
                DriverPhase::CellSelected,
                DriverPhase::YUpdated,
                DriverPhase::XUpdated,
                DriverPhase::ZUpdated,
            ],
        )?;
        if y >= self.state.count_y || z >= self.state.count_xz {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        for i in 0..self.state.interpolators.len() {
            let id = self.state.interpolators[i];
            let stride = self.state.count_y + 1;
            if let Cache::Interpolated(interp) = &mut self.state.wrappers[id].cache {
                let corners = [
                    interp.slice0[z * stride + y],
                    interp.slice1[z * stride + y],
                    interp.slice0[z * stride + y + 1],
                    interp.slice1[z * stride + y + 1],
                    interp.slice0[(z + 1) * stride + y],
                    interp.slice1[(z + 1) * stride + y],
                    interp.slice0[(z + 1) * stride + y + 1],
                    interp.slice1[(z + 1) * stride + y + 1],
                ];
                for (slot, value) in interp.corners.iter_mut().zip(corners) {
                    charge(&mut self.work)?;
                    *slot = value;
                }
            } else {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        charge(&mut self.work)?;
        self.state.filling_cell = true;
        self.state.start = BlockPos::new(
            self.state.start.x(),
            (y as i32 + self.state.min_cell_y) * self.state.height as i32,
            self.state.origin.z() + z as i32 * self.state.width as i32,
        );
        self.state.array_counter = self.state.array_counter.wrapping_add(1);
        for i in 0..self.state.cells.len() {
            let id = self.state.cells[i];
            let root = self.state.wrappers[id].input;
            let mut values = match &mut self.state.wrappers[id].cache {
                Cache::Cell(values) => std::mem::take(values),
                _ => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
            };
            let result = self.state.fill(
                &self.data,
                &self.view,
                root,
                &mut values,
                Provider::Cell,
                &mut self.work,
            );
            if let Cache::Cell(slot) = &mut self.state.wrappers[id].cache {
                *slot = values;
            }
            result?;
        }
        charge(&mut self.work)?;
        self.state.array_counter = self.state.array_counter.wrapping_add(1);
        self.state.filling_cell = false;
        self.state.driver_phase = DriverPhase::CellSelected;
        Ok(())
    }
    pub(super) fn update_y(&mut self, delta: f64) -> Result<(), NoiseChunkError> {
        self.require_driver(
            "update_y",
            &[
                DriverPhase::CellSelected,
                DriverPhase::YUpdated,
                DriverPhase::XUpdated,
                DriverPhase::ZUpdated,
            ],
        )?;
        if !delta.is_finite() {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        charge(&mut self.work)?;
        for i in 0..self.state.interpolators.len() {
            let id = self.state.interpolators[i];
            if let Cache::Interpolated(interp) = &mut self.state.wrappers[id].cache {
                for (j, (a, b)) in [(0, 2), (1, 3), (4, 6), (5, 7)].into_iter().enumerate() {
                    charge(&mut self.work)?;
                    interp.y_values[j] = lerp(delta, interp.corners[a], interp.corners[b]);
                }
            } else {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        self.state.driver_phase = DriverPhase::YUpdated;
        Ok(())
    }
    pub(super) fn update_x(&mut self, delta: f64) -> Result<(), NoiseChunkError> {
        self.require_driver(
            "update_x",
            &[
                DriverPhase::YUpdated,
                DriverPhase::XUpdated,
                DriverPhase::ZUpdated,
            ],
        )?;
        if !delta.is_finite() {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        charge(&mut self.work)?;
        for i in 0..self.state.interpolators.len() {
            let id = self.state.interpolators[i];
            if let Cache::Interpolated(interp) = &mut self.state.wrappers[id].cache {
                for (j, (a, b)) in [(0, 1), (2, 3)].into_iter().enumerate() {
                    charge(&mut self.work)?;
                    interp.x_values[j] = lerp(delta, interp.y_values[a], interp.y_values[b]);
                }
            } else {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        self.state.driver_phase = DriverPhase::XUpdated;
        Ok(())
    }
    pub(super) fn update_z(&mut self, delta: f64) -> Result<(), NoiseChunkError> {
        self.require_driver("update_z", &[DriverPhase::XUpdated, DriverPhase::ZUpdated])?;
        if !delta.is_finite() {
            return Err(fail(NoiseChunkErrorKind::InvalidGraph));
        }
        for i in 0..self.state.interpolators.len() {
            charge(&mut self.work)?;
            let id = self.state.interpolators[i];
            if let Cache::Interpolated(interp) = &mut self.state.wrappers[id].cache {
                interp.value = lerp(delta, interp.x_values[0], interp.x_values[1]);
            } else {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        self.state.driver_phase = DriverPhase::ZUpdated;
        Ok(())
    }
    pub(super) fn swap_slices(&mut self) -> Result<(), NoiseChunkError> {
        self.require_driver(
            "swap_slices",
            &[
                DriverPhase::CellSelected,
                DriverPhase::YUpdated,
                DriverPhase::XUpdated,
                DriverPhase::ZUpdated,
            ],
        )?;
        for i in 0..self.state.interpolators.len() {
            charge(&mut self.work)?;
            let id = self.state.interpolators[i];
            if let Cache::Interpolated(interp) = &mut self.state.wrappers[id].cache {
                std::mem::swap(&mut interp.slice0, &mut interp.slice1);
            } else {
                return Err(fail(NoiseChunkErrorKind::InvalidGraph));
            }
        }
        self.state.driver_x += 1;
        self.state.driver_phase = DriverPhase::NeedAdvance;
        Ok(())
    }
    pub(super) fn stop_driver(&mut self) -> Result<(), NoiseChunkError> {
        self.require_driver("stop", &[DriverPhase::NeedAdvance])?;
        if self.state.driver_x != self.state.count_xz {
            return Err(fail(NoiseChunkErrorKind::Lifecycle {
                operation: "stop",
                phase: "columns_incomplete",
            }));
        }
        charge(&mut self.work)?;
        self.state.interpolating = false;
        self.state.driver_phase = DriverPhase::Complete;
        Ok(())
    }
}
