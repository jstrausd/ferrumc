//! Nonrecursive finite equality keys; children use already-interned equals IDs.

use std::mem::size_of;

use super::super::{
    bytes, core_charge as charge, core_fail as fail, limit, NoiseChunkCoreError as NoiseChunkError,
    NoiseChunkErrorKind, NoiseChunkLimit,
};
use crate::density::bind::{BoundKind, BoundNode};
use crate::density::graph::{NodeKind, Unit};
use crate::sample_work::SampleWork;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Identity {
    ModernNoise(usize),
    FreshNoise,
    SeedEnd(usize),
    SeedBlended(usize, bool),
    SplineLocations(usize, bool),
    SplineDerivatives(usize, bool),
    Wrapper(usize),
}
struct Key {
    words: Box<[u64]>,
    next: Option<usize>,
    node: Option<usize>,
    identity: Option<u64>,
}
pub(super) struct Keys {
    entries: Vec<Key>,
    buckets: Box<[Option<usize>]>,
    identities: Vec<Identity>,
    bytes: usize,
}
impl Keys {
    pub fn reserved_bytes() -> usize {
        size_of::<Self>()
            + 4096 * (size_of::<Key>() + size_of::<Option<usize>>() + size_of::<Identity>())
    }
    pub fn new() -> Result<Self, NoiseChunkError> {
        let bytes = Self::reserved_bytes();
        limit(NoiseChunkLimit::KeyBytes, bytes, 2 * 1024 * 1024)?;
        Ok(Self {
            entries: Vec::with_capacity(4096),
            buckets: vec![None; 4096].into_boxed_slice(),
            identities: Vec::with_capacity(4096),
            bytes,
        })
    }
    pub fn identity(
        &mut self,
        identity: Identity,
        work: &mut SampleWork,
    ) -> Result<u64, NoiseChunkError> {
        if !matches!(identity, Identity::FreshNoise) {
            for (i, &prior) in self.identities.iter().enumerate() {
                charge(work)?;
                if identity == prior {
                    return Ok(i as u64 + 1);
                }
            }
        }
        limit(
            NoiseChunkLimit::IdentityTokens,
            self.identities.len() + 1,
            4096,
        )?;
        charge(work)?;
        self.identities.push(identity);
        Ok(self.identities.len() as u64)
    }
    pub fn intern(
        &mut self,
        words: Vec<u64>,
        identity: Option<u64>,
        work: &mut SampleWork,
    ) -> Result<usize, NoiseChunkError> {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for &word in &words {
            charge(work)?;
            hash = (hash ^ word).wrapping_mul(0x100_0000_01b3);
        }
        let bucket = (hash & 4095) as usize;
        let mut next = self.buckets[bucket];
        while let Some(index) = next {
            let prior = &self.entries[index];
            charge(work)?;
            let mut equal = prior.words.len() == words.len();
            if equal {
                for (&a, &b) in prior.words.iter().zip(&words) {
                    charge(work)?;
                    if a != b {
                        equal = false;
                        break;
                    }
                }
            }
            if equal {
                return Ok(index);
            }
            next = prior.next;
        }
        let index = self.entries.len();
        limit(NoiseChunkLimit::EqualityKeys, index + 1, 4096)?;
        let bytes = bytes(
            NoiseChunkLimit::KeyBytes,
            self.bytes,
            words.len(),
            size_of::<u64>(),
            2 * 1024 * 1024,
        )?;
        charge(work)?;
        self.bytes = bytes;
        self.entries.push(Key {
            words: words.into_boxed_slice(),
            next: self.buckets[bucket],
            node: None,
            identity,
        });
        self.buckets[bucket] = Some(index);
        Ok(index)
    }
    pub fn node(&self, key: usize) -> Option<usize> {
        self.entries[key].node
    }
    pub fn node_identity(&self, key: usize) -> Option<u64> {
        self.entries[key].identity
    }
    pub fn set_node(&mut self, key: usize, node: usize) {
        self.entries[key].node = Some(node);
    }
}
fn double(value: f64) -> u64 {
    if value.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        value.to_bits()
    }
}
fn float(value: f32) -> u64 {
    u64::from(if value.is_nan() {
        0x7fc0_0000
    } else {
        value.to_bits()
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "Exhaustive original class-specific record equality keeps component order auditable."
)]
pub(super) fn words(
    node: &BoundNode,
    node_keys: &[usize],
    noise: u64,
    arrays: Option<(u64, u64)>,
    identity: Option<u64>,
) -> Result<Vec<u64>, NoiseChunkError> {
    let child = |i: usize| {
        node_keys
            .get(i)
            .copied()
            .map(|i| i as u64)
            .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))
    };
    let bounds = || [double(node.bounds.min), double(node.bounds.max)];
    let key = match &node.kind {
        BoundKind::Alias(_) => return Err(fail(NoiseChunkErrorKind::InvalidGraph)),
        BoundKind::MulOrAdd {
            operation,
            input,
            argument,
        } => {
            let b = bounds();
            vec![
                1,
                *operation as u64,
                child(*input)?,
                b[0],
                b[1],
                double(*argument),
            ]
        }
        BoundKind::Original(kind) => match kind {
            NodeKind::Constant(value) => vec![2, double(*value)],
            NodeKind::Holder { target, .. } => vec![3, child(*target)?],
            NodeKind::Coordinate { target, .. } => vec![4, child(*target)?],
            NodeKind::Binary {
                operation,
                left,
                right,
            } => {
                let b = bounds();
                vec![
                    5,
                    *operation as u64,
                    child(*left)?,
                    child(*right)?,
                    b[0],
                    b[1],
                ]
            }
            NodeKind::Unary { operation, input } => {
                let b = bounds();
                vec![6, *operation as u64, child(*input)?, b[0], b[1]]
            }
            NodeKind::Marker { marker, input } => {
                if let Some(token) = identity {
                    vec![7, *marker as u64, token]
                } else {
                    vec![8, *marker as u64, child(*input)?]
                }
            }
            NodeKind::BlendDensity(input) => vec![9, child(*input)?],
            NodeKind::Clamp { input, min, max } => {
                vec![10, child(*input)?, double(*min), double(*max)]
            }
            NodeKind::RangeChoice {
                input,
                min,
                max,
                inside,
                outside,
            } => vec![
                11,
                child(*input)?,
                double(*min),
                double(*max),
                child(*inside)?,
                child(*outside)?,
            ],
            NodeKind::Gradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => vec![
                12,
                *from_y as u64,
                *to_y as u64,
                double(*from_value),
                double(*to_value),
            ],
            NodeKind::Noise { xz, y, .. } => vec![
                13,
                node.noise
                    .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))? as u64,
                noise,
                double(*xz),
                double(*y),
            ],
            NodeKind::Shift { shift, .. } => vec![
                14,
                *shift as u64,
                node.noise
                    .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))? as u64,
                noise,
            ],
            NodeKind::ShiftedNoise {
                shift_x,
                shift_y,
                shift_z,
                xz,
                y,
                ..
            } => vec![
                15,
                child(*shift_x)?,
                child(*shift_y)?,
                child(*shift_z)?,
                node.noise
                    .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))? as u64,
                noise,
                double(*xz),
                double(*y),
            ],
            NodeKind::Weird { input, rarity, .. } => vec![
                16,
                child(*input)?,
                node.noise
                    .ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))? as u64,
                noise,
                *rarity as u64,
            ],
            NodeKind::Blended { .. } => vec![
                17,
                identity.ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?,
            ],
            NodeKind::Unit(Unit::EndIslands) => vec![
                18,
                identity.ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?,
            ],
            NodeKind::Unit(unit) => vec![19, *unit as u64],
            NodeKind::Spline(root) => vec![20, child(*root)?],
            NodeKind::SplineConstant(value) => vec![21, float(*value)],
            NodeKind::SplineMultipoint { coordinate, points } => {
                let (locations, derivatives) =
                    arrays.ok_or_else(|| fail(NoiseChunkErrorKind::InvalidGraph))?;
                let b = bounds();
                let mut key = Vec::with_capacity(6 + points.len());
                key.extend([22, child(*coordinate)?, locations, derivatives, b[0], b[1]]);
                for point in points {
                    key.push(child(point.value)?);
                }
                key
            }
        },
    };
    Ok(key)
}
