//! Seed binding of the column driver: the six `SurfaceSystem` extension noises
//! and the closed state alphabet (policy D1), derived from the producers.

use std::mem::size_of;

use ferrumc_registry::biome::BiomeResourceId;
use ferrumc_registry::block_state::{block_default_state, block_metadata, state_id_to_block_name};
use ferrumc_registry::worldgen::{NoiseResourceId, NoiseSettingsResourceId};

use super::super::{SeededSurfaceRules, SurfaceRuleErrorKind};
use super::error::limit;
use super::{
    SurfaceColumnBindBudget, SurfaceColumnError, SurfaceColumnErrorKind as Kind,
    SurfaceColumnLimit, MAX_SURFACE_COLUMN_ALPHABET, MAX_SURFACE_COLUMN_BINDING_BYTES,
};
use crate::density::{
    material_alphabet, parse_definition, JsonValueKind, MATERIAL_ALPHABET_STATES,
};
use crate::noise::NormalNoise;
use crate::sample_work::SampleWork;
use crate::{BlockStateId, WorldSeed};

/// Original `Noises` keys of the six extension noises, in binding slot order.
pub(super) const EXTENSION_NOISES: [&str; 6] = [
    "minecraft:badlands_surface",
    "minecraft:badlands_pillar",
    "minecraft:badlands_pillar_roof",
    "minecraft:iceberg_surface",
    "minecraft:iceberg_pillar",
    "minecraft:iceberg_pillar_roof",
];
pub(super) const BADLANDS_SURFACE: usize = 0;
pub(super) const BADLANDS_PILLAR: usize = 1;
pub(super) const BADLANDS_PILLAR_ROOF: usize = 2;
pub(super) const ICEBERG_SURFACE: usize = 3;
pub(super) const ICEBERG_PILLAR: usize = 4;
pub(super) const ICEBERG_PILLAR_ROOF: usize = 5;
/// Existing `SampleWork` charge of one `ImprovedNoise` Paper-basis copy.
const IMPROVED_BASIS_WORK: usize = 259;
/// Original air blocks (`BlockState.isAir`): `air`, `cave_air`, `void_air`.
const AIR_BLOCKS: [&str; 3] = ["air", "cave_air", "void_air"];

/// Driver classification of one alphabet state, derived from its block name
/// and the dense water/lava ranges (never from solidity or fixture flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StateClass(u8);

impl StateClass {
    const AIR: u8 = 1;
    const FLUID: u8 = 2;
    const WATER_BLOCK: u8 = 4;
    const DEFAULT_BLOCK: u8 = 8;
    const DEFAULT_STATE: u8 = 16;
    /// `getBlock` outside the build height: `VOID_AIR`.
    pub(super) const VOID_AIR: Self = Self(Self::AIR);

    /// Original `BlockState.isAir`.
    pub(super) const fn air(self) -> bool {
        self.0 & Self::AIR != 0
    }
    /// Non-empty original fluid state (any water or lava state).
    pub(super) const fn fluid(self) -> bool {
        self.0 & Self::FLUID != 0
    }
    /// Original `is(Blocks.WATER)`.
    pub(super) const fn water_block(self) -> bool {
        self.0 & Self::WATER_BLOCK != 0
    }
    /// Original `is(defaultBlock.getBlock())`.
    pub(super) const fn default_block(self) -> bool {
        self.0 & Self::DEFAULT_BLOCK != 0
    }
    /// Original `state == defaultBlock`.
    pub(super) const fn default_state(self) -> bool {
        self.0 & Self::DEFAULT_STATE != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    state: u32,
    class: StateClass,
}

/// Sorted closed alphabet with inline storage (no heap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Alphabet {
    entries: [Entry; MAX_SURFACE_COLUMN_ALPHABET],
    len: usize,
}

impl Alphabet {
    const EMPTY: Self = Self {
        entries: [Entry {
            state: 0,
            class: StateClass(0),
        }; MAX_SURFACE_COLUMN_ALPHABET],
        len: 0,
    };

    /// Class of an alphabet state; `None` outside the alphabet (D1).
    pub(super) fn classify(&self, state: BlockStateId) -> Option<StateClass> {
        let entries = &self.entries[..self.len];
        entries
            .binary_search_by_key(&state.as_u32(), |entry| entry.state)
            .ok()
            .map(|index| entries[index].class)
    }

    /// Alphabet states in ascending raw order.
    #[cfg(test)]
    pub(super) fn states(&self) -> impl Iterator<Item = BlockStateId> + '_ {
        self.entries[..self.len]
            .iter()
            .map(|entry| BlockStateId::new(entry.state))
    }

    fn insert(&mut self, state: BlockStateId, cap: usize) -> Result<(), SurfaceColumnError> {
        let raw = state.as_u32();
        let entries = &self.entries[..self.len];
        let Err(at) = entries.binary_search_by_key(&raw, |entry| entry.state) else {
            return Ok(());
        };
        limit(SurfaceColumnLimit::AlphabetStates, self.len + 1, cap)?;
        self.entries.copy_within(at..self.len, at + 1);
        self.entries[at] = Entry {
            state: raw,
            class: StateClass(0),
        };
        self.len += 1;
        Ok(())
    }

    /// Classifies every entry by its block name (D1, review condition A6).
    fn classify_by_name(&mut self, default_block: BlockStateId) -> Result<(), SurfaceColumnError> {
        let unclassified = || SurfaceColumnError::new(Kind::UnclassifiedState);
        let range = |name: &str| {
            block_metadata(name)
                .map(|meta| meta.min_state..=meta.max_state)
                .ok_or_else(unclassified)
        };
        let (water, lava) = (range("water")?, range("lava")?);
        let default_name =
            state_id_to_block_name(default_block.as_u32()).ok_or_else(unclassified)?;
        for entry in &mut self.entries[..self.len] {
            let name = state_id_to_block_name(entry.state)
                .ok_or_else(|| unclassified().with_counts(entry.state as usize, 0))?;
            let mut class = 0;
            if AIR_BLOCKS.contains(&name) {
                class |= StateClass::AIR;
            }
            if water.contains(&entry.state) {
                class |= StateClass::FLUID | StateClass::WATER_BLOCK;
            }
            if lava.contains(&entry.state) {
                class |= StateClass::FLUID;
            }
            if name == default_name {
                class |= StateClass::DEFAULT_BLOCK;
            }
            if entry.state == default_block.as_u32() {
                class |= StateClass::DEFAULT_STATE;
            }
            entry.class = StateClass(class);
        }
        Ok(())
    }
}

/// An immutable column-driver binding for one world seed and settings resource
/// (Paper build 60 only; P1, P2 and P3 are inherited from the rules binding).
///
/// Holds the six extension noises (`badlands_surface/pillar/pillar_roof`,
/// `iceberg_surface/pillar/pillar_roof`) in the Paper `ImprovedNoise` basis,
/// bound from the rules' positional root, and the closed state alphabet (D1):
/// the GEN-05.4 material states of the settings, the template's rule-result
/// states, the clay band states and the extension states (default block,
/// `snow_block`, `packed_ice`), classified by block name.
#[derive(Debug)]
pub struct SeededSurfaceColumns {
    seed: WorldSeed,
    resource: NoiseSettingsResourceId,
    pub(super) sea_level: i32,
    pub(super) default_block: BlockStateId,
    pub(super) snow_block: BlockStateId,
    pub(super) packed_ice: BlockStateId,
    pub(super) eroded_badlands: BiomeResourceId,
    pub(super) frozen_ocean: BiomeResourceId,
    pub(super) deep_frozen_ocean: BiomeResourceId,
    pub(super) noises: Box<[NormalNoise]>,
    pub(super) alphabet: Alphabet,
    construction_work: usize,
}

impl SeededSurfaceColumns {
    pub(crate) fn carver_states(&self) -> impl Iterator<Item = BlockStateId> + '_ {
        self.alphabet.entries[..self.alphabet.len]
            .iter()
            .map(|entry| BlockStateId::new(entry.state))
    }
    /// Binds the column driver to the seed and settings of `rules`.
    ///
    /// Order: the allowance domain is checked by the budget type; then the
    /// closed alphabet is derived (pure registry and settings lookups, no
    /// sampler), the bind census (Paper-basis noise slots + one unit per
    /// alphabet state) is checked (`WorkBudgetExceeded`), the samplers are
    /// built and the retained-byte cap is checked. Nothing is returned on
    /// failure.
    pub fn bind(
        rules: &SeededSurfaceRules,
        budget: SurfaceColumnBindBudget,
    ) -> Result<Self, SurfaceColumnError> {
        Self::bind_with(
            rules,
            budget,
            MAX_SURFACE_COLUMN_BINDING_BYTES,
            MAX_SURFACE_COLUMN_ALPHABET,
        )
    }

    pub(super) fn bind_with(
        rules: &SeededSurfaceRules,
        budget: SurfaceColumnBindBudget,
        retained_cap: usize,
        alphabet_cap: usize,
    ) -> Result<Self, SurfaceColumnError> {
        let template = rules.template();
        let resource = template.resource();
        let (default_block, alphabet) = derive_alphabet(rules, alphabet_cap)?;
        let (basis_work, parameters) = noise_census()?;
        let requirement = basis_work + alphabet.len;
        if (budget.units() as usize) < requirement {
            return Err(SurfaceColumnError::new(Kind::WorkBudgetExceeded)
                .with_counts(requirement, budget.units() as usize));
        }
        let mut work = SampleWork::new(basis_work);
        let mut noises = Vec::with_capacity(EXTENSION_NOISES.len());
        for noise in parameters {
            let parameters = noise.parameters();
            let vanilla = NormalNoise::new(
                &mut rules.root.from_hash_of(noise.resource_location()),
                parameters.first_octave(),
                parameters.amplitudes(),
            )
            .map_err(|_| SurfaceColumnError::new(Kind::Noise))?;
            noises.push(
                vanilla
                    .paper_build60_basis(&mut work)
                    .map_err(|_| SurfaceColumnError::new(Kind::WorkBudgetExceeded))?,
            );
        }
        let biome = |name: &str| {
            BiomeResourceId::from_name(name).ok_or_else(|| {
                SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::UnknownBiome))
            })
        };
        let extension = |name: &str| {
            block_default_state(name)
                .map(BlockStateId::new)
                .ok_or_else(|| {
                    SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::InvalidBlockState))
                })
        };
        let bound = Self {
            seed: rules.seed(),
            resource,
            sea_level: template.sea_level(),
            default_block,
            snow_block: extension("minecraft:snow_block")?,
            packed_ice: extension("minecraft:packed_ice")?,
            eroded_badlands: biome("minecraft:eroded_badlands")?,
            frozen_ocean: biome("minecraft:frozen_ocean")?,
            deep_frozen_ocean: biome("minecraft:deep_frozen_ocean")?,
            noises: noises.into_boxed_slice(),
            alphabet,
            construction_work: requirement,
        };
        limit(
            SurfaceColumnLimit::BindingBytes,
            bound.retained_bytes(),
            retained_cap,
        )?;
        Ok(bound)
    }

    /// Bound world seed.
    pub const fn seed(&self) -> WorldSeed {
        self.seed
    }
    /// Bound settings resource.
    pub const fn resource(&self) -> NoiseSettingsResourceId {
        self.resource
    }
    /// Spent bind census: Paper-basis noise slots (8320) + alphabet states.
    pub const fn construction_work(&self) -> usize {
        self.construction_work
    }
    /// Number of states of the derived closed alphabet (35 for the admitted
    /// overworld settings).
    pub const fn alphabet_len(&self) -> usize {
        self.alphabet.len
    }
    /// Measured retained native bytes; the rules binding is not included.
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>()
            + self
                .noises
                .iter()
                .map(NormalNoise::retained_bytes)
                .sum::<usize>()
    }
}

/// `(Paper-basis SampleWork of the six noises, their registry keys)`.
fn noise_census() -> Result<(usize, [NoiseResourceId; 6]), SurfaceColumnError> {
    let lookup = |slot: usize| {
        NoiseResourceId::from_name(EXTENSION_NOISES[slot])
            .ok_or_else(|| SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::UnknownNoise)))
    };
    let ids = [
        lookup(BADLANDS_SURFACE)?,
        lookup(BADLANDS_PILLAR)?,
        lookup(BADLANDS_PILLAR_ROOF)?,
        lookup(ICEBERG_SURFACE)?,
        lookup(ICEBERG_PILLAR)?,
        lookup(ICEBERG_PILLAR_ROOF)?,
    ];
    let basis = ids
        .iter()
        .map(|noise| {
            let amplitudes = noise.parameters().amplitudes();
            let nonzero = amplitudes.iter().filter(|a| **a != 0.0).count();
            // Two stacks per NormalNoise: one charge per octave slot, 259 per Improved.
            2 * (amplitudes.len() + nonzero * IMPROVED_BASIS_WORK)
        })
        .sum::<usize>();
    Ok((basis, ids))
}

/// The settings' default block and the classified closed alphabet (A5): the
/// GEN-05.4 material states (resolved through the material producer), the
/// template's result states, the clay bands and the extension states.
fn derive_alphabet(
    rules: &SeededSurfaceRules,
    cap: usize,
) -> Result<(BlockStateId, Alphabet), SurfaceColumnError> {
    let resource = rules.template().resource();
    let malformed =
        || SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::MalformedDefinition));
    let json = resource.definition().json();
    let (root, _) =
        parse_definition(json, resource.resource_location()).map_err(|_| malformed())?;
    let JsonValueKind::Object(map) = &root.kind else {
        return Err(malformed());
    };
    let span = |field: &str| {
        map.get(field)
            .and_then(|value| json.get(value.span.clone()))
            .ok_or_else(malformed)
    };
    let material: [BlockStateId; MATERIAL_ALPHABET_STATES] =
        material_alphabet(span("default_block")?, span("default_fluid")?).map_err(|_| {
            SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::InvalidBlockState))
        })?;
    let default_block = material[0];
    let extension = |name: &str| {
        block_default_state(name)
            .map(BlockStateId::new)
            .ok_or_else(|| {
                SurfaceColumnError::new(Kind::Rules(SurfaceRuleErrorKind::InvalidBlockState))
            })
    };
    let mut alphabet = Alphabet::EMPTY;
    for state in material
        .into_iter()
        .chain(rules.template().states.iter().copied())
        .chain(rules.bands.iter().copied())
        .chain([
            default_block,
            extension("minecraft:snow_block")?,
            extension("minecraft:packed_ice")?,
        ])
    {
        alphabet.insert(state, cap)?;
    }
    alphabet.classify_by_name(default_block)?;
    Ok((default_block, alphabet))
}

/// Private test controls.
#[cfg(test)]
impl SeededSurfaceColumns {
    /// Removes one state from the alphabet to reach `UnclassifiedState` (A5).
    pub(super) fn without_state(&mut self, state: BlockStateId) {
        let len = self.alphabet.len;
        if let Some(at) = self.alphabet.entries[..len]
            .iter()
            .position(|entry| entry.state == state.as_u32())
        {
            self.alphabet.entries.copy_within(at + 1..len, at);
            self.alphabet.len -= 1;
        }
    }
}
