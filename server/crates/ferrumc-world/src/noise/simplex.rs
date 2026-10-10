use ferrumc_math::Vec3;

use super::{finite, java_floor, validate_input, ImprovedNoise, NoiseError, GRADIENTS};
use crate::random::WorldgenRandomSource;

// Exact Math.sqrt(3.0) binary64 value from the pinned original runtime.
const SQRT_3: f64 = 1.732_050_807_568_877_2;
const F2: f64 = 0.5 * (SQRT_3 - 1.0);
const G2: f64 = (3.0 - SQRT_3) / 6.0;

/// Original 2D/3D simplex noise, owning its original seeded permutation.
/// The original samplers do not add the seeded offsets to their coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplexNoise {
    permutation: [u8; 256],
    offsets: Vec3,
}

impl SimplexNoise {
    /// Consumes three original doubles and 256 original bounded shuffle draws.
    pub fn new(random: &mut WorldgenRandomSource) -> Self {
        // Both original constructors use precisely the same shuffle. Simplex's
        // unused second 256 entries are never addressed by its masked lookup.
        let initialized = ImprovedNoise::new(random);
        Self {
            permutation: initialized.permutation,
            offsets: initialized.offsets,
        }
    }

    /// Returns all three seeded offsets, even though sampling does not add them.
    pub const fn offsets(&self) -> Vec3 {
        self.offsets
    }
    /// Borrows the complete permutation actually used by the original sampler.
    pub const fn permutation(&self) -> &[u8; 256] {
        &self.permutation
    }

    /// Samples the original two-dimensional equality and gradient branches.
    /// Inputs must be finite and within [`super::MAX_NOISE_COORDINATE`].
    pub fn sample_2d(&self, x: f64, y: f64) -> Result<f64, NoiseError> {
        validate_input(Vec3::new(x, y, 0.0), 0.0, 0.0)?;
        let skew = (x + y) * F2;
        let floor_x = java_floor(x + skew);
        let floor_y = java_floor(y + skew);
        let unskew = f64::from(floor_x.wrapping_add(floor_y)) * G2;
        let dx = x - (f64::from(floor_x) - unskew);
        let dy = y - (f64::from(floor_y) - unskew);
        let (ix, iy) = if dx > dy { (1, 0) } else { (0, 1) };
        let dx1 = dx - f64::from(ix) + G2;
        let dy1 = dy - f64::from(iy) + G2;
        let dx2 = dx - 1.0 + 2.0 * G2;
        let dy2 = dy - 1.0 + 2.0 * G2;
        let px = floor_x & 255;
        let py = floor_y & 255;
        let g0 = self.p(px + self.p(py)) % 12;
        let g1 = self.p(px + ix + self.p(py + iy)) % 12;
        let g2 = self.p(px + 1 + self.p(py + 1)) % 12;
        let c0 = corner(g0, Vec3::new(dx, dy, 0.0), 0.5);
        let c1 = corner(g1, Vec3::new(dx1, dy1, 0.0), 0.5);
        let c2 = corner(g2, Vec3::new(dx2, dy2, 0.0), 0.5);
        finite(70.0 * (c0 + c1 + c2))
    }

    /// Samples original three-dimensional ordering, including equal coordinates.
    /// Inputs must be finite and within [`super::MAX_NOISE_COORDINATE`].
    pub fn sample(&self, position: Vec3) -> Result<f64, NoiseError> {
        validate_input(position, 0.0, 0.0)?;
        let skew = (position.x + position.y + position.z) * 0.333_333_333_333_333_3;
        let floor_x = java_floor(position.x + skew);
        let floor_y = java_floor(position.y + skew);
        let floor_z = java_floor(position.z + skew);
        let unskew = f64::from(floor_x.wrapping_add(floor_y).wrapping_add(floor_z))
            * 0.166_666_666_666_666_66;
        let dx = position.x - (f64::from(floor_x) - unskew);
        let dy = position.y - (f64::from(floor_y) - unskew);
        let dz = position.z - (f64::from(floor_z) - unskew);
        let (first, second) = if dx >= dy {
            if dy >= dz {
                ([1, 0, 0], [1, 1, 0])
            } else if dx >= dz {
                ([1, 0, 0], [1, 0, 1])
            } else {
                ([0, 0, 1], [1, 0, 1])
            }
        } else if dy < dz {
            ([0, 0, 1], [0, 1, 1])
        } else if dx < dz {
            ([0, 1, 0], [0, 1, 1])
        } else {
            ([0, 1, 0], [1, 1, 0])
        };
        let d1 = Vec3::new(
            dx - f64::from(first[0]) + 0.166_666_666_666_666_66,
            dy - f64::from(first[1]) + 0.166_666_666_666_666_66,
            dz - f64::from(first[2]) + 0.166_666_666_666_666_66,
        );
        let d2 = Vec3::new(
            dx - f64::from(second[0]) + 0.333_333_333_333_333_3,
            dy - f64::from(second[1]) + 0.333_333_333_333_333_3,
            dz - f64::from(second[2]) + 0.333_333_333_333_333_3,
        );
        let d3 = Vec3::new(dx - 1.0 + 0.5, dy - 1.0 + 0.5, dz - 1.0 + 0.5);
        let px = floor_x & 255;
        let py = floor_y & 255;
        let pz = floor_z & 255;
        let g0 = self.p(px + self.p(py + self.p(pz))) % 12;
        let g1 = self.p(px + first[0] + self.p(py + first[1] + self.p(pz + first[2]))) % 12;
        let g2 = self.p(px + second[0] + self.p(py + second[1] + self.p(pz + second[2]))) % 12;
        let g3 = self.p(px + 1 + self.p(py + 1 + self.p(pz + 1))) % 12;
        let c0 = corner(g0, Vec3::new(dx, dy, dz), 0.6);
        let c1 = corner(g1, d1, 0.6);
        let c2 = corner(g2, d2, 0.6);
        let c3 = corner(g3, d3, 0.6);
        finite(32.0 * (c0 + c1 + c2 + c3))
    }

    fn p(&self, index: i32) -> i32 {
        i32::from(self.permutation[(index & 255) as usize])
    }
}

fn corner(gradient: i32, delta: Vec3, offset: f64) -> f64 {
    let mut weight = offset - delta.x * delta.x - delta.y * delta.y - delta.z * delta.z;
    if weight < 0.0 {
        0.0
    } else {
        weight *= weight;
        let g = GRADIENTS[gradient as usize];
        weight * weight * (g[0] * delta.x + g[1] * delta.y + g[2] * delta.z)
    }
}
