//! Seed binding: positional root, Paper-basis noises, gradient factories, bands.

use std::mem::{size_of, size_of_val};
use std::sync::Arc;

use ferrumc_registry::block_state::block_default_state;

use super::compile::{SurfaceRuleTemplate, CLAY_NOISE, SECONDARY_NOISE, SURFACE_NOISE};
use super::error::limit;
use super::{
    PaperFlatBedrock, SurfaceNumericProfile, SurfaceRuleBindBudget, SurfaceRuleError,
    SurfaceRuleErrorKind as Kind, SurfaceRuleLimit, MAX_SURFACE_RULE_BINDING_BYTES,
    SURFACE_RULE_CLAY_BANDS,
};
use crate::noise::NormalNoise;
use crate::random::{PositionalRandomFactory, RandomAlgorithm, WorldgenRandomSource};
use crate::sample_work::SampleWork;
use crate::{BlockStateId, WorldSeed};

/// Existing `SampleWork` charge of one `ImprovedNoise` Paper-basis copy:
/// 256 permutation entries and three offsets.
const IMPROVED_BASIS_WORK: usize = 259;
/// Original `ResourceLocation.withDefaultNamespace("clay_bands")`.
const CLAY_BANDS_NAME: &str = "minecraft:clay_bands";

/// An immutable surface-rule binding for one world seed (Paper build 60 only).
///
/// Holds the positional root `Xoroshiro(seed).forkPositional()` (the original
/// `RandomState.random`, `SurfaceSystem.noiseRandom` and density root), one
/// Paper-basis `NormalNoise` per distinct bound noise, one positional factory
/// per gradient random name and the 192 original clay bands.
#[derive(Debug)]
pub struct SeededSurfaceRules {
    template: Arc<SurfaceRuleTemplate>,
    seed: WorldSeed,
    pub(super) root: PositionalRandomFactory,
    pub(super) noises: Box<[NormalNoise]>,
    pub(super) factories: Box<[PositionalRandomFactory]>,
    pub(super) bands: [BlockStateId; SURFACE_RULE_CLAY_BANDS],
    construction_work: usize,
}

impl SeededSurfaceRules {
    /// Binds a template to one seed.
    ///
    /// Refusal order, before any sampler is built: numeric profile (P1),
    /// flat-bedrock mode (P2), then the bind census (`WorkBudgetExceeded`).
    /// The retained-byte cap is checked before publication.
    pub fn bind(
        template: Arc<SurfaceRuleTemplate>,
        seed: WorldSeed,
        profile: SurfaceNumericProfile,
        flat_bedrock: PaperFlatBedrock,
        budget: SurfaceRuleBindBudget,
    ) -> Result<Self, SurfaceRuleError> {
        Self::bind_with(
            template,
            seed,
            profile,
            flat_bedrock,
            budget,
            MAX_SURFACE_RULE_BINDING_BYTES,
        )
    }

    pub(super) fn bind_with(
        template: Arc<SurfaceRuleTemplate>,
        seed: WorldSeed,
        profile: SurfaceNumericProfile,
        flat_bedrock: PaperFlatBedrock,
        budget: SurfaceRuleBindBudget,
        retained_cap: usize,
    ) -> Result<Self, SurfaceRuleError> {
        if profile != SurfaceNumericProfile::PaperBuild60 {
            return Err(SurfaceRuleError::new(Kind::UnsupportedNumericProfile));
        }
        if flat_bedrock != PaperFlatBedrock::Disabled {
            return Err(SurfaceRuleError::new(Kind::UnsupportedFlatBedrock));
        }
        let (basis_work, requirement) = bind_census(&template);
        if (budget.units() as usize) < requirement {
            return Err(SurfaceRuleError::new(Kind::WorkBudgetExceeded)
                .with_counts(requirement, budget.units() as usize));
        }
        let root = WorldgenRandomSource::new(RandomAlgorithm::Xoroshiro128PlusPlus, seed)
            .fork_positional();
        let mut work = SampleWork::new(basis_work);
        let mut noises = Vec::with_capacity(template.noises.len());
        for noise in &*template.noises {
            let parameters = noise.parameters();
            let vanilla = NormalNoise::new(
                &mut root.from_hash_of(noise.resource_location()),
                parameters.first_octave(),
                parameters.amplitudes(),
            )
            .map_err(|_| SurfaceRuleError::new(Kind::Noise))?;
            noises.push(
                vanilla
                    .paper_build60_basis(&mut work)
                    .map_err(|_| SurfaceRuleError::new(Kind::WorkBudgetExceeded))?,
            );
        }
        let factories = template
            .random_names
            .iter()
            .map(|name| root.from_hash_of(name).fork_positional())
            .collect::<Vec<_>>();
        let bands = generate_bands(&mut root.from_hash_of(CLAY_BANDS_NAME))?;
        let bound = Self {
            template,
            seed,
            root,
            noises: noises.into_boxed_slice(),
            factories: factories.into_boxed_slice(),
            bands,
            construction_work: requirement,
        };
        limit(
            SurfaceRuleLimit::BindingBytes,
            bound.retained_bytes(),
            retained_cap,
        )?;
        Ok(bound)
    }

    /// Bound template.
    pub fn template(&self) -> &SurfaceRuleTemplate {
        &self.template
    }
    /// Bound world seed.
    pub const fn seed(&self) -> WorldSeed {
        self.seed
    }
    /// Spent bind census: octave slots + 259 per Paper-basis `ImprovedNoise`
    /// + 192 band slots + positional root and gradient factories.
    pub const fn construction_work(&self) -> usize {
        self.construction_work
    }
    /// Measured retained native bytes; the shared template is not included.
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + self
                .noises
                .iter()
                .map(NormalNoise::retained_bytes)
                .sum::<usize>()
            + size_of_val(&*self.factories)
    }
    pub(super) fn surface_noise(&self) -> &NormalNoise {
        &self.noises[SURFACE_NOISE]
    }
    pub(super) fn secondary_noise(&self) -> &NormalNoise {
        &self.noises[SECONDARY_NOISE]
    }
    pub(super) fn clay_noise(&self) -> &NormalNoise {
        &self.noises[CLAY_NOISE]
    }
}

/// `(SampleWork of the Paper-basis copies, whole bind requirement)`.
fn bind_census(template: &SurfaceRuleTemplate) -> (usize, usize) {
    let basis = template
        .noises
        .iter()
        .map(|noise| {
            let amplitudes = noise.parameters().amplitudes();
            let nonzero = amplitudes.iter().filter(|a| **a != 0.0).count();
            // Two stacks per NormalNoise: one charge per octave slot, 259 per Improved.
            2 * (amplitudes.len() + nonzero * IMPROVED_BASIS_WORK)
        })
        .sum::<usize>();
    let factories = 1 + template.random_names.len();
    (basis, basis + SURFACE_RULE_CLAY_BANDS + factories)
}

/// Default state of a `SurfaceSystem` band block (canonical `minecraft:` name).
fn band_state(name: &str) -> Result<BlockStateId, SurfaceRuleError> {
    block_default_state(name)
        .map(BlockStateId::new)
        .ok_or_else(|| SurfaceRuleError::new(Kind::InvalidBlockState))
}

/// Original `SurfaceSystem.generateBands` with the original draw order.
fn generate_bands(
    random: &mut WorldgenRandomSource,
) -> Result<[BlockStateId; SURFACE_RULE_CLAY_BANDS], SurfaceRuleError> {
    let terracotta = band_state("minecraft:terracotta")?;
    let orange = band_state("minecraft:orange_terracotta")?;
    let white = band_state("minecraft:white_terracotta")?;
    let light_gray = band_state("minecraft:light_gray_terracotta")?;
    let mut bands = [terracotta; SURFACE_RULE_CLAY_BANDS];
    let length = SURFACE_RULE_CLAY_BANDS as u32;
    let mut i = 0_u32;
    while i < length {
        i += random.bounded_positive(5) + 1;
        if i < length {
            bands[i as usize] = orange;
        }
        i += 1;
    }
    make_bands(
        random,
        &mut bands,
        1,
        band_state("minecraft:yellow_terracotta")?,
    );
    make_bands(
        random,
        &mut bands,
        2,
        band_state("minecraft:brown_terracotta")?,
    );
    make_bands(
        random,
        &mut bands,
        1,
        band_state("minecraft:red_terracotta")?,
    );
    let count = between_inclusive(random, 9, 15);
    let (mut placed, mut index) = (0, 0_u32);
    while placed < count && index < length {
        bands[index as usize] = white;
        if index > 1 && random.next_boolean() {
            bands[index as usize - 1] = light_gray;
        }
        if index + 1 < length && random.next_boolean() {
            bands[index as usize + 1] = light_gray;
        }
        placed += 1;
        index += random.bounded_positive(16) + 4;
    }
    Ok(bands)
}

/// Original `RandomSource.nextIntBetweenInclusive` for the positive constants used here.
fn between_inclusive(random: &mut WorldgenRandomSource, min: u32, max: u32) -> u32 {
    random.bounded_positive(max - min + 1) + min
}

/// Original `SurfaceSystem.makeBands`.
fn make_bands(
    random: &mut WorldgenRandomSource,
    bands: &mut [BlockStateId; SURFACE_RULE_CLAY_BANDS],
    min_size: u32,
    state: BlockStateId,
) {
    let length = SURFACE_RULE_CLAY_BANDS as u32;
    let count = between_inclusive(random, 6, 15);
    for _ in 0..count {
        let size = min_size + random.bounded_positive(3);
        let start = random.bounded_positive(length);
        let mut offset = 0;
        while start + offset < length && offset < size {
            bands[(start + offset) as usize] = state;
            offset += 1;
        }
    }
}

/// Synthetic source controls; never a natural-world expectation or public tree API.
#[cfg(test)]
impl SeededSurfaceRules {
    pub(crate) fn carver_test_rule(&mut self, control: u8) {
        use super::compile::{Node, NodeKind};
        let template = Arc::get_mut(&mut self.template).unwrap();
        let node = |kind, parent, depth| Node {
            kind,
            parent,
            depth,
        };
        match control {
            0 => {
                template.nodes = vec![node(NodeKind::Sequence { first: 0, len: 0 }, u16::MAX, 0)]
                    .into_boxed_slice();
                template.children = Box::new([]);
            }
            1 => {
                template.nodes = vec![node(NodeKind::Block(0), u16::MAX, 0)].into_boxed_slice();
                template.children = Box::new([]);
            }
            2 => {
                template.nodes = vec![
                    node(NodeKind::Sequence { first: 0, len: 2 }, u16::MAX, 0),
                    node(NodeKind::Condition { test: 2, then: 3 }, 0, 1),
                    node(
                        NodeKind::Biome {
                            first: 0,
                            len: template.biomes.len() as u16,
                        },
                        1,
                        2,
                    ),
                    node(NodeKind::Sequence { first: 0, len: 0 }, 1, 2),
                    node(NodeKind::Condition { test: 5, then: 6 }, 0, 1),
                    node(NodeKind::Temperature, 4, 2),
                    node(NodeKind::Sequence { first: 0, len: 0 }, 4, 2),
                ]
                .into_boxed_slice();
                template.children = vec![1, 4].into_boxed_slice();
            }
            3 => {
                let water = template
                    .states
                    .iter()
                    .position(|s| {
                        ferrumc_registry::block_state::state_id_to_block_name(s.as_u32())
                            == Some("water")
                    })
                    .unwrap();
                template.nodes =
                    vec![node(NodeKind::Block(water as u16), u16::MAX, 0)].into_boxed_slice();
                template.children = Box::new([]);
            }
            _ => unreachable!(),
        }
    }
}
