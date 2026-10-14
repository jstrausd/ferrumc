//! Real cold Overworld composition; every producer must succeed before publication.
use crate::biome_selection::{
    BiomeBuildBudget, BiomeQuartGeometry, BiomeSelectionSession, BiomeSourceIdentity,
    BiomeSourceSession, BiomeTree, BiomeWorkBudget, MAX_BIOME_BUILD_WORK, MAX_BIOME_SELECTION_WORK,
};
use crate::biome_temperature::{BiomeTemperatureBuildBudget, BuiltinBiomeTemperatures};
use crate::biome_zoom::BiomeZoomSemantics;
use crate::carver::{
    CarvedChunkBuffer, CarverBindBudget, CarverInput, CarverWorkBudget, SeededCarvers,
    CARVER_CHUNK_WORK,
};
use crate::density::{NoiseChunkWorkBudget, RouterTemplate, SeededRouter, MAX_NOISE_CHUNK_WORK};
use crate::stored_biome::{
    StoredBiomeBuildBudget, StoredBiomeIdentity, SurfaceBiomeResolver, MAX_STORED_BIOME_BUILD_WORK,
};
use crate::surface_rules::{
    PaperFlatBedrock, PreliminarySurfaceCorners, SeededSurfaceColumns, SeededSurfaceRules,
    SurfaceChunkBuffer, SurfaceChunkInput, SurfaceColumnBindBudget, SurfaceColumnWorkBudget,
    SurfaceNumericProfile, SurfaceRuleBindBudget, SurfaceRuleBuildBudget, SurfaceRuleTemplate,
    MAX_SURFACE_COLUMN_BIND_WORK, MAX_SURFACE_RULE_BIND_WORK, MAX_SURFACE_RULE_BUILD_WORK,
    SURFACE_CHUNK_WORK,
};
use crate::{
    Chunk, FallibleChunkGenerator, FlatWorldGenerator, GenerationIdentity, WorldError, WorldSeed,
};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::{biome::BiomeParametersResourceId, worldgen::NoiseSettingsResourceId};
use std::{
    collections::VecDeque,
    fmt::Display,
    sync::{Arc, Mutex},
};

/// Producer phase retained in a natural generation refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NaturalGenerationStage {
    /// Immutable registry/template/seed binding.
    Binding,
    /// Noise material and preliminary surface producer.
    Material,
    /// Complete stored-biome neighborhood and resolver.
    Biomes,
    /// Accepted surface column driver.
    Surface,
    /// Accepted cold carver runtime.
    Carvers,
    /// Explicit placed-feature subset preview.
    Features,
    /// Complete producer output installation into a chunk value.
    Installation,
}
fn refusal(stage: NaturalGenerationStage, error: impl Display) -> WorldError {
    WorldError::NaturalGeneration {
        stage,
        detail: error.to_string().chars().take(512).collect(),
    }
}
struct Binding {
    seed: WorldSeed,
    settings: NoiseSettingsResourceId,
    source: BiomeSourceIdentity,
    geometry: BiomeQuartGeometry,
    router: SeededRouter,
    noise_plan: Option<crate::density::NaturalNoisePlan>,
    tree: Arc<BiomeTree>,
    climate: ClimateCache,
    transitions: TransitionCache,
    temperatures: BuiltinBiomeTemperatures,
    rules: SeededSurfaceRules,
    columns: SeededSurfaceColumns,
    carvers: SeededCarvers,
}
// A target column is independent of the biome selector's history. Cache only
// the complete original fresh-noise targets; every nine-column source traversal
// still replays its own ordered RTree selection and exact cumulative work.
const CLIMATE_CACHE_COLUMNS: usize = 128;
const CLIMATE_CACHE_BYTES: usize = 10 * 1024 * 1024;
struct ClimateCache {
    values: Mutex<VecDeque<(ChunkPos, Arc<crate::biome_selection::NaturalClimateSamples>)>>,
    gates: [Mutex<()>; 32],
}
impl ClimateCache {
    fn new() -> Self {
        Self {
            values: Mutex::new(VecDeque::with_capacity(CLIMATE_CACHE_COLUMNS)),
            gates: std::array::from_fn(|_| Mutex::new(())),
        }
    }
    fn native_bytes(
        values: &VecDeque<(ChunkPos, Arc<crate::biome_selection::NaturalClimateSamples>)>,
    ) -> usize {
        std::mem::size_of::<Self>()
            + values.capacity()
                * std::mem::size_of::<(ChunkPos, Arc<crate::biome_selection::NaturalClimateSamples>)>(
                )
            + values
                .iter()
                .map(|(_, value)| value.native_bytes() + 2 * std::mem::size_of::<usize>())
                .sum::<usize>()
    }
    fn get(
        &self,
        pos: ChunkPos,
    ) -> Result<Option<Arc<crate::biome_selection::NaturalClimateSamples>>, WorldError> {
        let values = self
            .values
            .lock()
            .map_err(|_| refusal(NaturalGenerationStage::Biomes, "climate cache poisoned"))?;
        Ok(values
            .iter()
            .find(|(key, _)| *key == pos)
            .map(|(_, value)| Arc::clone(value)))
    }
}
// Optional bounded memo: cache failure must leave the original producer usable.
const TRANSITION_CACHE_ENTRIES: usize = 64;
const TRANSITION_CACHE_BYTES: usize = 128 * 1024;
struct TransitionCache {
    values: Mutex<VecDeque<crate::biome_selection::NaturalBiomeTransition>>,
}
impl TransitionCache {
    fn new() -> Self {
        let mut values = VecDeque::new();
        let _ = values.try_reserve_exact(TRANSITION_CACHE_ENTRIES);
        Self {
            values: Mutex::new(values),
        }
    }
    fn native_bytes(values: &VecDeque<crate::biome_selection::NaturalBiomeTransition>) -> usize {
        std::mem::size_of::<Self>()
            + values.capacity()
                * std::mem::size_of::<crate::biome_selection::NaturalBiomeTransition>()
            + values
                .iter()
                .map(crate::biome_selection::NaturalBiomeTransition::allocation_bytes)
                .sum::<usize>()
    }
    fn sample(
        &self,
        source: &mut BiomeSourceSession,
        climate: &crate::biome_selection::NaturalClimateSamples,
        pos: ChunkPos,
    ) -> Result<Arc<crate::biome_selection::ChunkBiomeSamples>, crate::biome_selection::BiomeError>
    {
        let key = source.natural_transition_key(pos);
        if let (Some(key), Ok(values)) = (key, self.values.lock()) {
            if let Some(saved) = values.iter().find(|saved| saved.key() == key) {
                if let Some(samples) = source.replay_natural_transition(climate, saved) {
                    return Ok(samples);
                }
            }
        }
        let saved = source.sample_natural_transition(climate)?;
        let samples = saved.samples();
        if let (Some(key), Ok(mut values)) = (key, self.values.lock()) {
            if values.capacity() >= TRANSITION_CACHE_ENTRIES
                && !values.iter().any(|saved| saved.key() == key)
            {
                if values.len() == TRANSITION_CACHE_ENTRIES {
                    values.pop_front();
                }
                while !values.is_empty()
                    && Self::native_bytes(&values) + saved.allocation_bytes()
                        > TRANSITION_CACHE_BYTES
                {
                    values.pop_front();
                }
                if Self::native_bytes(&values) + saved.allocation_bytes() <= TRANSITION_CACHE_BYTES
                {
                    values.push_back(saved);
                }
            }
        }
        Ok(samples)
    }
}
impl Binding {
    fn climate(
        &self,
        pos: ChunkPos,
    ) -> Result<Arc<crate::biome_selection::NaturalClimateSamples>, WorldError> {
        use NaturalGenerationStage::Biomes as S;
        if let Some(value) = self.climate.get(pos)? {
            return Ok(value);
        }
        let stripe = ((pos.x() as u32).wrapping_mul(31) ^ pos.z() as u32) as usize
            % self.climate.gates.len();
        let _gate = self.climate.gates[stripe]
            .lock()
            .map_err(|_| refusal(S, "climate producer gate poisoned"))?;
        if let Some(value) = self.climate.get(pos)? {
            return Ok(value);
        }
        let mut noise = self
            .router
            .fresh_natural_noise_chunk(
                self.noise_plan.as_ref(),
                pos,
                NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).map_err(|e| refusal(S, e))?,
            )
            .map_err(|e| refusal(S, e))?;
        let value = Arc::new(
            crate::biome_selection::NaturalClimateSamples::sample(&mut noise, self.geometry)
                .map_err(|e| refusal(S, e))?,
        );
        let mut values = self
            .climate
            .values
            .lock()
            .map_err(|_| refusal(S, "climate cache poisoned"))?;
        let bytes = value.native_bytes() + 2 * std::mem::size_of::<usize>();
        while values.len() >= CLIMATE_CACHE_COLUMNS
            || ClimateCache::native_bytes(&values) + bytes > CLIMATE_CACHE_BYTES
        {
            if values.pop_front().is_none() {
                return Ok(value);
            }
        }
        values.push_back((pos, Arc::clone(&value)));
        Ok(value)
    }
}
/// Deterministic real Overworld terrain through material, biomes, surface and carvers.
///
/// Uses the accepted cold Paper build-60 numerical profile and unchanged producer
/// budgets. Revision 1 omits placed features, trees, structures, fluid ticks and
/// full lighting; it is a terrain preview. Clones share immutable seed bindings.
#[derive(Clone)]
pub struct NaturalWorldGenerator(
    Arc<Binding>,
    Option<Arc<crate::grass::GrassPreview>>,
    Option<Arc<crate::trees::TreePreview>>,
    Option<Arc<crate::survival_ores::SurvivalOrePreview>>,
);
impl std::fmt::Debug for NaturalWorldGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NaturalWorldGenerator")
            .field("identity", &self.identity())
            .finish_non_exhaustive()
    }
}
impl NaturalWorldGenerator {
    /// Compiles and binds the accepted normal Overworld producers for this seed.
    pub fn new(seed: WorldSeed) -> Result<Self, WorldError> {
        use NaturalGenerationStage::Binding as S;
        let settings = NoiseSettingsResourceId::from_name("minecraft:overworld")
            .ok_or_else(|| refusal(S, "missing overworld settings"))?;
        let parameters = BiomeParametersResourceId::from_name("minecraft:overworld")
            .ok_or_else(|| refusal(S, "missing overworld biome parameters"))?;
        let source = BiomeSourceIdentity::MultiNoise(parameters);
        let geometry = BiomeQuartGeometry::new(-64, 384).map_err(|e| refusal(S, e))?;
        let router = RouterTemplate::compile_builtin(settings)
            .map_err(|e| refusal(S, e))?
            .bind_seed(seed)
            .map_err(|e| refusal(S, e))?;
        let tree = Arc::new(
            BiomeTree::from_builtin(
                parameters,
                BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).map_err(|e| refusal(S, e))?,
            )
            .map_err(|e| refusal(S, e))?,
        );
        let temperatures = BuiltinBiomeTemperatures::compile(
            BiomeTemperatureBuildBudget::new(1_048_576).map_err(|e| refusal(S, e))?,
        )
        .map_err(|e| refusal(S, e))?;
        let template = Arc::new(
            SurfaceRuleTemplate::compile_builtin(
                settings,
                SurfaceRuleBuildBudget::new(MAX_SURFACE_RULE_BUILD_WORK)
                    .map_err(|e| refusal(S, e))?,
            )
            .map_err(|e| refusal(S, e))?,
        );
        let rules = SeededSurfaceRules::bind(
            template,
            seed,
            SurfaceNumericProfile::PaperBuild60,
            PaperFlatBedrock::Disabled,
            SurfaceRuleBindBudget::new(MAX_SURFACE_RULE_BIND_WORK).map_err(|e| refusal(S, e))?,
        )
        .map_err(|e| refusal(S, e))?;
        let columns = SeededSurfaceColumns::bind(
            &rules,
            SurfaceColumnBindBudget::new(MAX_SURFACE_COLUMN_BIND_WORK)
                .map_err(|e| refusal(S, e))?,
        )
        .map_err(|e| refusal(S, e))?;
        let carvers = SeededCarvers::bind(
            &columns,
            CarverBindBudget::new(90).map_err(|e| refusal(S, e))?,
        )
        .map_err(|e| refusal(S, e))?;
        let noise_plan = router.compile_natural_noise_plan();
        Ok(Self(
            Arc::new(Binding {
                seed,
                settings,
                source,
                geometry,
                router,
                noise_plan,
                tree,
                climate: ClimateCache::new(),
                transitions: TransitionCache::new(),
                temperatures,
                rules,
                columns,
                carvers,
            }),
            None,
            None,
            None,
        ))
    }
    /// Binds revision 2: original plains/forest short-grass patches with isolated sources.
    ///
    /// The explicit subset omits trees, other plants, placed ores and structures.
    /// A bounded immutable raw-terrain cache is shared by clones. Revision 1 remains unchanged.
    pub fn with_grass_preview(seed: WorldSeed) -> Result<Self, WorldError> {
        let mut generator = Self::new(seed)?;
        generator.1 = Some(Arc::new(crate::grass::GrassPreview::new()?));
        Ok(generator)
    }
    /// Binds revision 3: original undecorated oak/birch shape subset and short grass.
    ///
    /// Full selectors, decorations and other tree species remain outside this preview.
    pub fn with_tree_preview(seed: WorldSeed) -> Result<Self, WorldError> {
        let mut generator = Self::with_grass_preview(seed)?;
        generator.2 = Some(Arc::new(crate::trees::TreePreview::new()?));
        Ok(generator)
    }
    /// Binds revision 4: oak/birch, grass and normal Overworld placed ore subsets.
    ///
    /// Coal, iron, copper, gold, redstone, lapis and diamond use pinned counts,
    /// height distributions, target tags and biome membership. Each source has
    /// isolated raw-terrain history; omitted geology/structures and the complete
    /// FEATURES stage prevent a Vanilla seed-parity claim. Revisions 1–3 retain
    /// their previous output and saved identity.
    pub fn with_survival_preview(seed: WorldSeed) -> Result<Self, WorldError> {
        let mut generator = Self::with_tree_preview(seed)?;
        generator.3 = Some(Arc::new(crate::survival_ores::SurvivalOrePreview::new()?));
        Ok(generator)
    }
    /// Returns the selected immutable terrain-preview revision identity.
    #[must_use]
    pub fn identity(&self) -> GenerationIdentity {
        if self.3.is_some() {
            GenerationIdentity::natural_overworld_survival(self.0.seed)
        } else if self.2.is_some() {
            GenerationIdentity::natural_overworld_with_trees(self.0.seed)
        } else if self.1.is_some() {
            GenerationIdentity::natural_overworld_with_grass(self.0.seed)
        } else {
            GenerationIdentity::natural_overworld(self.0.seed)
        }
    }
    /// Generates a complete real chunk or returns the refusing producer's error.
    pub fn try_generate_chunk(&self, center: ChunkPos) -> Result<Chunk, WorldError> {
        let mut chunk = if let Some(trees) = &self.2 {
            trees.generate(
                self,
                self.0.seed,
                center,
                self.1.as_deref().ok_or_else(|| {
                    refusal(NaturalGenerationStage::Features, "missing tree raw binding")
                })?,
            )?
        } else if let Some(grass) = &self.1 {
            grass.generate(self, self.0.seed, center)?
        } else {
            self.generate_raw_chunk(center)?
        };
        if let Some(ores) = &self.3 {
            ores.apply(
                self,
                self.1.as_deref().ok_or_else(|| {
                    refusal(NaturalGenerationStage::Features, "missing ore raw binding")
                })?,
                self.0.seed,
                &mut chunk,
            )?;
        }
        Ok(chunk)
    }

    #[cfg(test)]
    pub(crate) fn tree_preview(&self) -> Option<&crate::trees::TreePreview> {
        self.2.as_deref()
    }
    #[cfg(test)]
    pub(crate) fn grass_preview(&self) -> Option<&crate::grass::GrassPreview> {
        self.1.as_deref()
    }
    pub(crate) fn grass_plain_count(&self, source: ChunkPos) -> Result<usize, WorldError> {
        let noise = self
            .0
            .temperatures
            .info_noise(
                f64::from(source.x().wrapping_mul(16)) / 200.0,
                f64::from(source.z().wrapping_mul(16)) / 200.0,
            )
            .map_err(|e| refusal(NaturalGenerationStage::Features, e))?;
        Ok(if noise < -0.8 { 5 } else { 10 })
    }
    pub(crate) fn sample_center_biomes(
        &self,
        center: ChunkPos,
    ) -> Result<Vec<ferrumc_registry::biome::BiomeResourceId>, WorldError> {
        let b = &self.0;
        let mut source = BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
            Arc::clone(&b.tree),
            BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK)
                .map_err(|e| refusal(NaturalGenerationStage::Biomes, e))?,
        ));
        let mut result = None;
        for dx in -1..=1 {
            for dz in -1..=1 {
                let pos = ChunkPos::new(center.x().wrapping_add(dx), center.z().wrapping_add(dz));
                let climate = b.climate(pos)?;
                let array = b
                    .transitions
                    .sample(&mut source, &climate, pos)
                    .map_err(|e| refusal(NaturalGenerationStage::Biomes, e))?;
                if pos == center {
                    result = Some(array.values().to_vec());
                }
            }
        }
        result.ok_or_else(|| {
            refusal(
                NaturalGenerationStage::Biomes,
                "missing center biome projection",
            )
        })
    }
    pub(crate) fn generate_raw_chunk(&self, center: ChunkPos) -> Result<Chunk, WorldError> {
        use NaturalGenerationStage as S;
        let b = &self.0;
        let budget = || NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK);
        let material = b
            .router
            .fresh_natural_noise_chunk(
                b.noise_plan.as_ref(),
                center,
                budget().map_err(|e| refusal(S::Material, e))?,
            )
            .map_err(|e| refusal(S::Material, e))?
            .sample_chunk_materials()
            .map_err(|e| refusal(S::Material, e))?;
        let mut preliminary = b
            .router
            .fresh_natural_noise_chunk(
                b.noise_plan.as_ref(),
                center,
                budget().map_err(|e| refusal(S::Material, e))?,
            )
            .map_err(|e| refusal(S::Material, e))?;
        let mut corner = |dx: i32, dz: i32| {
            preliminary
                .preliminary_surface_level(BlockPos::new(
                    center.x().wrapping_add(dx).wrapping_mul(16),
                    0,
                    center.z().wrapping_add(dz).wrapping_mul(16),
                ))
                .map_err(|e| refusal(S::Material, e))
        };
        let corners = PreliminarySurfaceCorners::new(
            corner(0, 0)?,
            corner(1, 0)?,
            corner(0, 1)?,
            corner(1, 1)?,
        );
        let mut source = BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
            Arc::clone(&b.tree),
            BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).map_err(|e| refusal(S::Biomes, e))?,
        ));
        let mut arrays = Vec::with_capacity(9);
        for dx in -1..=1 {
            for dz in -1..=1 {
                let pos = ChunkPos::new(center.x().wrapping_add(dx), center.z().wrapping_add(dz));
                let climate = b.climate(pos)?;
                arrays.push(
                    b.transitions
                        .sample(&mut source, &climate, pos)
                        .map_err(|e| refusal(S::Biomes, e))?,
                );
            }
        }
        let refs: Vec<_> = arrays.iter().map(Arc::as_ref).collect();
        let resolver = SurfaceBiomeResolver::new(
            StoredBiomeIdentity::new(b.seed, b.settings, b.source, b.geometry),
            center,
            &refs,
            BiomeZoomSemantics::Paper1_21_8Build60,
            StoredBiomeBuildBudget::new(MAX_STORED_BIOME_BUILD_WORK)
                .map_err(|e| refusal(S::Biomes, e))?,
        )
        .map_err(|e| refusal(S::Biomes, e))?;
        let mut surface = SurfaceChunkBuffer::new().map_err(|e| refusal(S::Surface, e))?;
        b.columns
            .build_chunk(
                SurfaceChunkInput::new(&b.rules, &resolver, &b.temperatures, corners, &material),
                SurfaceColumnWorkBudget::new(SURFACE_CHUNK_WORK)
                    .map_err(|e| refusal(S::Surface, e))?,
                &mut surface,
            )
            .map_err(|e| refusal(S::Surface, e))?;
        let mut run = b
            .carvers
            .prepare_run_with_tree(
                &b.router,
                &b.rules,
                &b.temperatures,
                b.source,
                center,
                corners,
                &b.tree,
            )
            .map_err(|e| refusal(S::Carvers, e))?;
        let mut output = CarvedChunkBuffer::new().map_err(|e| refusal(S::Carvers, e))?;
        b.carvers
            .carve_chunk(
                CarverInput::new(&surface, &mut run),
                CarverWorkBudget::new(CARVER_CHUNK_WORK).map_err(|e| refusal(S::Carvers, e))?,
                &mut output,
            )
            .map_err(|e| refusal(S::Carvers, e))?;
        install_chunk(center, &output, &arrays)
    }
}
fn install_chunk(
    center: ChunkPos,
    output: &CarvedChunkBuffer,
    arrays: &[Arc<crate::biome_selection::ChunkBiomeSamples>],
) -> Result<Chunk, WorldError> {
    let mut chunk = Chunk::new(center);
    for y in -64..=319 {
        for x in 0..16 {
            for z in 0..16 {
                let pos = BlockPos::new(
                    center.x().wrapping_mul(16).wrapping_add(x),
                    y,
                    center.z().wrapping_mul(16).wrapping_add(z),
                );
                let state = output.state(pos).ok_or_else(|| {
                    refusal(
                        NaturalGenerationStage::Installation,
                        "unpublished carver output",
                    )
                })?;
                if !state.is_air() {
                    chunk.set_block(pos, state)?;
                }
            }
        }
    }
    let center_biomes = arrays
        .iter()
        .find(|array| array.chunk_pos() == center)
        .ok_or_else(|| {
            refusal(
                NaturalGenerationStage::Installation,
                "missing center biome array",
            )
        })?;
    chunk.install_generated_biomes(center_biomes.values().to_vec().into_boxed_slice())?;
    Ok(chunk)
}

impl FallibleChunkGenerator for NaturalWorldGenerator {
    fn identity(&self) -> GenerationIdentity {
        self.identity()
    }
    fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
        self.try_generate_chunk(pos)
    }
}
/// Cloneable configured generator; natural refusals never select the flat profile.
#[derive(Debug, Clone)]
pub enum RuntimeWorldGenerator {
    /// Existing seed-independent fixed-layer profile.
    Flat(FlatWorldGenerator),
    /// Real terrain preview through carvers, optionally with the explicit grass subset.
    Natural(NaturalWorldGenerator),
}
impl RuntimeWorldGenerator {
    /// Selects the existing flat profile and retains its numeric seed.
    #[must_use]
    pub const fn flat(seed: WorldSeed) -> Self {
        Self::Flat(FlatWorldGenerator::with_seed(seed))
    }
    /// Binds real normal Overworld terrain for the selected seed.
    pub fn natural(seed: WorldSeed) -> Result<Self, WorldError> {
        NaturalWorldGenerator::new(seed).map(Self::Natural)
    }
    /// Selects the explicit revision-2 original short-grass subset preview.
    pub fn natural_with_grass(seed: WorldSeed) -> Result<Self, WorldError> {
        NaturalWorldGenerator::with_grass_preview(seed).map(Self::Natural)
    }
    /// Selects the separately versioned oak/birch shape-and-grass preview.
    pub fn natural_with_trees(seed: WorldSeed) -> Result<Self, WorldError> {
        NaturalWorldGenerator::with_tree_preview(seed).map(Self::Natural)
    }
    /// Selects revision 4 with the explicit placed ore Survival subset.
    pub fn natural_survival(seed: WorldSeed) -> Result<Self, WorldError> {
        Ok(Self::Natural(NaturalWorldGenerator::with_survival_preview(
            seed,
        )?))
    }
    /// Returns the selected complete baseline identity.
    #[must_use]
    pub fn identity(&self) -> GenerationIdentity {
        match self {
            Self::Flat(g) => g.identity(),
            Self::Natural(g) => g.identity(),
        }
    }
    /// Generates selected terrain, propagating natural producer errors.
    pub fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
        match self {
            Self::Flat(g) => Ok(g.generate(pos)),
            Self::Natural(g) => g.try_generate_chunk(pos),
        }
    }
}
impl FallibleChunkGenerator for RuntimeWorldGenerator {
    fn identity(&self) -> GenerationIdentity {
        self.identity()
    }
    fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
        self.try_generate_chunk(pos)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeightmapKind;
    #[test]
    fn natural_origin_real_pipeline_smoke() {
        let start = std::time::Instant::now();
        let generator = NaturalWorldGenerator::new(WorldSeed::new(0)).unwrap();
        let chunk = generator.try_generate_chunk(ChunkPos::ORIGIN).unwrap();
        let heights = chunk.heightmap(HeightmapKind::WorldSurface);
        let mut all = Vec::new();
        for x in 0..16 {
            for z in 0..16 {
                all.push(heights.height(x, z).unwrap());
            }
        }
        println!(
            "natural seed0 origin time={:?} height={}..{} at8,8={:?}",
            start.elapsed(),
            all.iter().min().unwrap(),
            all.iter().max().unwrap(),
            heights.height(8, 8)
        );
        assert!(!chunk.persist_dirty_sections().any());
        assert!(!chunk.persist_edited_sections().any());
        assert!(all.iter().any(|y| *y != 63));
    }
}

#[cfg(test)]
mod acceptance {
    use super::*;

    const GROUPS: &str =
        include_str!("../tests/fixtures/carver-runtime-original-1.21.8/groups.tsv");
    const CHUNKS: &str =
        include_str!("../tests/fixtures/carver-runtime-original-1.21.8/chunks.tsv");
    const STATES: &str =
        include_str!("../tests/fixtures/carver-runtime-original-1.21.8/states.tsv");

    #[test]
    fn installed_chunks_match_original_final_states_at_seams_and_signed_seeds() {
        let generator = NaturalWorldGenerator::new(WorldSeed::new(0)).unwrap();
        let raw: Vec<u32> = STATES
            .lines()
            .skip(1)
            .map(|line| line.split('\t').nth(2).unwrap().parse().unwrap())
            .collect();
        for id in [
            "origin/normal/0",
            "seam/normal/0/+x",
            "seam/normal/0/+z",
            "negative/normal/0/-1,-1",
            "origin/normal/-1",
            "origin/normal/i64min",
            "origin/normal/i64max",
        ] {
            let group: Vec<_> = GROUPS
                .lines()
                .find(|line| line.split('\t').next() == Some(id))
                .unwrap()
                .split('\t')
                .collect();
            let center = ChunkPos::new(group[3].parse().unwrap(), group[4].parse().unwrap());
            let seed = WorldSeed::new(group[2].parse().unwrap());
            let generator = if seed.value() == 0 {
                generator.clone()
            } else {
                NaturalWorldGenerator::new(seed).unwrap()
            };
            let chunk = generator.try_generate_chunk(center).unwrap();
            let row: Vec<_> = CHUNKS
                .lines()
                .find(|line| {
                    let mut fields = line.split('\t');
                    fields.next() == Some(id) && fields.next() == Some("FINAL")
                })
                .unwrap()
                .split('\t')
                .collect();
            for (column, token) in row[7].split(';').enumerate() {
                let mut y = -64;
                for run in token.split('.') {
                    let (state, count) = run.split_once('*').unwrap();
                    let expected = raw[state.parse::<usize>().unwrap()];
                    for _ in 0..count.parse::<usize>().unwrap() {
                        let pos = BlockPos::new(
                            center.x() * 16 + i32::try_from(column / 16).unwrap(),
                            y,
                            center.z() * 16 + i32::try_from(column % 16).unwrap(),
                        );
                        assert_eq!(
                            chunk.get_block(pos).unwrap().as_u32(),
                            expected,
                            "{id} {pos:?}"
                        );
                        y += 1;
                    }
                }
                assert_eq!(y, 320);
            }
            let repeated = generator.clone().try_generate_chunk(center).unwrap();
            assert_eq!(chunk, repeated, "{id} request history and clone");
            assert_eq!(chunk.biomes().unwrap().len(), 1536);
            assert!(!chunk.persist_dirty_sections().any());
            assert!(!chunk.persist_edited_sections().any());
        }
    }

    #[test]
    fn real_out_of_domain_refusal_is_preserved_without_flat_substitution() {
        let generator = RuntimeWorldGenerator::natural(WorldSeed::new(0)).unwrap();
        let error = generator
            .try_generate_chunk(ChunkPos::new(i32::MAX, i32::MIN))
            .unwrap_err();
        assert!(matches!(
            error,
            WorldError::NaturalGeneration {
                stage: NaturalGenerationStage::Material,
                ..
            }
        ));
        assert_eq!(
            generator.identity(),
            GenerationIdentity::natural_overworld(WorldSeed::new(0))
        );
        let chunk = generator.try_generate_chunk(ChunkPos::ORIGIN).unwrap();
        assert_eq!(chunk.pos(), ChunkPos::ORIGIN);
    }
}

#[cfg(test)]
mod climate_cache_tests {
    use super::*;
    #[test]
    fn climate_failed_constructor_releases_gate_and_poison_preserves_completed_hits() {
        let generator = NaturalWorldGenerator::new(WorldSeed::new(0)).unwrap();
        assert!(generator.0.climate(ChunkPos::new(i32::MAX, 0)).is_err());
        assert!(generator.0.climate.values.lock().unwrap().is_empty());
        // i32::MAX and 31 hash to the same real producer stripe.
        assert!(!generator.0.climate.gates[1].is_poisoned());
        let completed = generator.0.climate(ChunkPos::new(31, 0)).unwrap();
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = generator.0.climate.gates[1].lock().unwrap();
            panic!("genuine private climate producer stripe poisoning");
        }));
        assert!(poisoned.is_err());
        assert!(generator.0.climate.gates[1].is_poisoned());
        let hit = generator.0.climate(ChunkPos::new(31, 0)).unwrap();
        assert!(Arc::ptr_eq(&completed, &hit));
        assert!(generator.0.climate(ChunkPos::new(63, 0)).is_err());
        let cache = generator.0.climate.values.lock().unwrap();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].0, ChunkPos::new(31, 0));
    }
    #[test]
    fn climate_cache_is_bounded_coalesced_and_recomputes_evicted_columns() {
        let generator = NaturalWorldGenerator::new(WorldSeed::new(0)).unwrap();
        let barrier = std::sync::Barrier::new(2);
        let (first, second) = std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                barrier.wait();
                generator.0.climate(ChunkPos::ORIGIN).unwrap()
            });
            let b = scope.spawn(|| {
                barrier.wait();
                generator.0.climate(ChunkPos::ORIGIN).unwrap()
            });
            (a.join().unwrap(), b.join().unwrap())
        });
        assert!(Arc::ptr_eq(&first, &second));
        for x in 1..=i32::try_from(CLIMATE_CACHE_COLUMNS).unwrap() {
            generator.0.climate(ChunkPos::new(x, 0)).unwrap();
        }
        let cache = generator.0.climate.values.lock().unwrap();
        assert_eq!(cache.len(), CLIMATE_CACHE_COLUMNS);
        assert_eq!(cache.capacity(), CLIMATE_CACHE_COLUMNS);
        assert!(cache.iter().all(|(pos, _)| *pos != ChunkPos::ORIGIN));
        let payload: usize = cache.iter().map(|(_, value)| value.native_bytes()).sum();
        assert_eq!(
            payload,
            CLIMATE_CACHE_COLUMNS
                * (73728 + std::mem::size_of::<crate::biome_selection::NaturalClimateSamples>())
        );
        let retained = ClimateCache::native_bytes(&cache);
        assert!(retained <= CLIMATE_CACHE_BYTES);
        println!("CLIMATE_NATIVE retained={retained} target_payload={} columns={} producer_stripes=32 cap={CLIMATE_CACHE_BYTES}; excludes allocator overhead and active worker leases", CLIMATE_CACHE_COLUMNS * 73728, CLIMATE_CACHE_COLUMNS);
        drop(cache);
        let replacement = generator.0.climate(ChunkPos::ORIGIN).unwrap();
        assert!(!Arc::ptr_eq(&first, &replacement));
        assert_eq!(first.targets(), replacement.targets());
    }
}

#[cfg(test)]
mod transition_cache_tests {
    use super::*;
    fn source(generator: &NaturalWorldGenerator, allowance: u32) -> BiomeSourceSession {
        BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
            Arc::clone(&generator.0.tree),
            BiomeWorkBudget::new(allowance).unwrap(),
        ))
    }
    #[test]
    fn transition_cache_bounds_fifo_eviction_and_concurrent_misses() {
        let generator = NaturalWorldGenerator::new(WorldSeed::new(-1)).unwrap();
        let climate = generator.0.climate(ChunkPos::ORIGIN).unwrap();
        let barrier = std::sync::Barrier::new(2);
        let run = || {
            let mut source = source(&generator, MAX_BIOME_SELECTION_WORK);
            barrier.wait();
            let samples = generator
                .0
                .transitions
                .sample(&mut source, &climate, ChunkPos::ORIGIN)
                .unwrap();
            (samples, source.work_used(), source.last_selection())
        };
        let (a, b) = std::thread::scope(|scope| {
            let a = scope.spawn(run);
            let b = scope.spawn(run);
            (a.join().unwrap(), b.join().unwrap())
        });
        assert_eq!(a.0.values(), b.0.values());
        assert_eq!((a.1, a.2), (b.1, b.2));
        assert_eq!(generator.0.transitions.values.lock().unwrap().len(), 1);
        let mut replay = source(&generator, MAX_BIOME_SELECTION_WORK);
        let first = generator
            .0
            .transitions
            .sample(&mut replay, &climate, ChunkPos::ORIGIN)
            .unwrap();
        for x in 1..=64 {
            let pos = ChunkPos::new(x, 0);
            let climate = generator.0.climate(pos).unwrap();
            generator
                .0
                .transitions
                .sample(
                    &mut source(&generator, MAX_BIOME_SELECTION_WORK),
                    &climate,
                    pos,
                )
                .unwrap();
        }
        let values = generator.0.transitions.values.lock().unwrap();
        assert_eq!(values.len(), 64);
        assert_eq!(values.capacity(), 64);
        assert_eq!(values.front().unwrap().key(), (ChunkPos::new(1, 0), None));
        assert_eq!(values.back().unwrap().key(), (ChunkPos::new(64, 0), None));
        let bytes = TransitionCache::native_bytes(&values);
        assert!(bytes <= TRANSITION_CACHE_BYTES);
        println!("TRANSITION_NATIVE retained={bytes} payload={} entries={} capacity={}; excludes shared binding tree, allocator overhead and active worker leases", 64 * 1536 * std::mem::size_of::<ferrumc_registry::biome::BiomeResourceId>(), values.len(), values.capacity());
        drop(values);
        let climate = generator.0.climate(ChunkPos::ORIGIN).unwrap();
        let replacement = generator
            .0
            .transitions
            .sample(
                &mut source(&generator, MAX_BIOME_SELECTION_WORK),
                &climate,
                ChunkPos::ORIGIN,
            )
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &replacement));
        assert_eq!(first.values(), replacement.values());
        assert_eq!(first.initial_selection(), replacement.initial_selection());
        assert_eq!(first.final_selection(), replacement.final_selection());
    }
    #[test]
    fn memo_hit_budget_fallback_preserves_original_error_work_and_history() {
        let generator = NaturalWorldGenerator::new(WorldSeed::new(17)).unwrap();
        let climate = generator.0.climate(ChunkPos::ORIGIN).unwrap();
        let mut full = source(&generator, MAX_BIOME_SELECTION_WORK);
        let saved = generator
            .0
            .transitions
            .sample(&mut full, &climate, ChunkPos::ORIGIN)
            .unwrap();
        let n = u32::try_from(full.work_used()).unwrap();
        for allowance in [n - 1, n, n + 1] {
            let mut actual = source(&generator, allowance);
            let mut expected = source(&generator, allowance);
            let actual_result =
                generator
                    .0
                    .transitions
                    .sample(&mut actual, &climate, ChunkPos::ORIGIN);
            let original = expected.sample_natural_climate(&climate);
            assert_eq!(
                actual_result
                    .as_ref()
                    .map(|v| v.values())
                    .map_err(crate::biome_selection::BiomeError::kind),
                original
                    .as_ref()
                    .map(crate::biome_selection::ChunkBiomeSamples::values)
                    .map_err(crate::biome_selection::BiomeError::kind)
            );
            assert_eq!(actual.work_used(), expected.work_used());
            assert_eq!(actual.last_selection(), expected.last_selection());
            assert_eq!(actual.is_poisoned(), expected.is_poisoned());
            if let Ok(hit) = actual_result {
                assert!(Arc::ptr_eq(&saved, &hit));
                assert_eq!(
                    hit.initial_selection(),
                    original.as_ref().unwrap().initial_selection()
                );
                assert_eq!(
                    hit.final_selection(),
                    original.as_ref().unwrap().final_selection()
                );
            }
            assert_eq!(generator.0.transitions.values.lock().unwrap().len(), 1);
        }
    }
    #[test]
    fn transition_hit_cannot_skip_upstream_climate_poison() {
        let generator = NaturalWorldGenerator::new(WorldSeed::new(17)).unwrap();
        generator.sample_center_biomes(ChunkPos::ORIGIN).unwrap();
        let retained = generator.0.transitions.values.lock().unwrap().len();
        assert_eq!(retained, 9);
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = generator.0.climate.values.lock().unwrap();
            panic!("genuine upstream climate cache poisoning");
        }));
        assert!(poisoned.is_err());
        let error = generator
            .sample_center_biomes(ChunkPos::ORIGIN)
            .unwrap_err();
        assert!(matches!(error, WorldError::NaturalGeneration {
            stage: NaturalGenerationStage::Biomes, detail
        } if detail == "climate cache poisoned"));
        assert_eq!(
            generator.0.transitions.values.lock().unwrap().len(),
            retained
        );
    }
    #[test]
    fn transition_failures_and_optional_cache_poison_use_original_producer() {
        let generator = NaturalWorldGenerator::new(WorldSeed::new(17)).unwrap();
        let climate = generator.0.climate(ChunkPos::ORIGIN).unwrap();
        let mut short = source(&generator, 1);
        assert!(generator
            .0
            .transitions
            .sample(&mut short, &climate, ChunkPos::ORIGIN)
            .is_err());
        assert!(short.is_poisoned());
        assert!(generator.0.transitions.values.lock().unwrap().is_empty());
        let mut complete = source(&generator, MAX_BIOME_SELECTION_WORK);
        let original = generator
            .0
            .transitions
            .sample(&mut complete, &climate, ChunkPos::ORIGIN)
            .unwrap();
        let used = complete.work_used();
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = generator.0.transitions.values.lock().unwrap();
            panic!("genuine optional transition cache poisoning");
        }));
        assert!(poisoned.is_err());
        let mut actual = source(&generator, MAX_BIOME_SELECTION_WORK);
        let replay = generator
            .0
            .transitions
            .sample(&mut actual, &climate, ChunkPos::ORIGIN)
            .unwrap();
        assert_eq!(replay.values(), original.values());
        assert_eq!(actual.work_used(), used);
        assert_eq!(actual.last_selection(), complete.last_selection());
        assert!(!actual.is_poisoned());
    }
}
