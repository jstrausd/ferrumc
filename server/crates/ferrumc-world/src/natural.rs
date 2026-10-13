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
use std::{fmt::Display, sync::Arc};

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
    tree: Arc<BiomeTree>,
    temperatures: BuiltinBiomeTemperatures,
    rules: SeededSurfaceRules,
    columns: SeededSurfaceColumns,
    carvers: SeededCarvers,
}
/// Deterministic real Overworld terrain through material, biomes, surface and carvers.
///
/// Uses the accepted cold Paper build-60 numerical profile and unchanged producer
/// budgets. Revision 1 omits placed features, trees, structures, fluid ticks and
/// full lighting; it is a terrain preview. Clones share immutable seed bindings.
#[derive(Clone)]
pub struct NaturalWorldGenerator(Arc<Binding>, Option<Arc<crate::grass::GrassPreview>>);
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
        Ok(Self(
            Arc::new(Binding {
                seed,
                settings,
                source,
                geometry,
                router,
                tree,
                temperatures,
                rules,
                columns,
                carvers,
            }),
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
    /// Returns the selected immutable terrain-preview revision identity.
    #[must_use]
    pub fn identity(&self) -> GenerationIdentity {
        if self.1.is_some() {
            GenerationIdentity::natural_overworld_with_grass(self.0.seed)
        } else {
            GenerationIdentity::natural_overworld(self.0.seed)
        }
    }
    /// Generates a complete real chunk or returns the refusing producer's error.
    pub fn try_generate_chunk(&self, center: ChunkPos) -> Result<Chunk, WorldError> {
        if let Some(grass) = &self.1 {
            return grass.generate(self, self.0.seed, center);
        }
        self.generate_raw_chunk(center)
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
                let mut noise = b
                    .router
                    .fresh_paper_noise_chunk(
                        pos,
                        NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK)
                            .map_err(|e| refusal(NaturalGenerationStage::Biomes, e))?,
                    )
                    .map_err(|e| refusal(NaturalGenerationStage::Biomes, e))?;
                let array = source
                    .sample_chunk_biomes(&mut noise, b.geometry)
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
            .fresh_paper_noise_chunk(center, budget().map_err(|e| refusal(S::Material, e))?)
            .map_err(|e| refusal(S::Material, e))?
            .sample_chunk_materials()
            .map_err(|e| refusal(S::Material, e))?;
        let mut preliminary = b
            .router
            .fresh_paper_noise_chunk(center, budget().map_err(|e| refusal(S::Material, e))?)
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
                let mut noise = b
                    .router
                    .fresh_paper_noise_chunk(pos, budget().map_err(|e| refusal(S::Biomes, e))?)
                    .map_err(|e| refusal(S::Biomes, e))?;
                arrays.push(
                    source
                        .sample_chunk_biomes(&mut noise, b.geometry)
                        .map_err(|e| refusal(S::Biomes, e))?,
                );
            }
        }
        let refs: Vec<_> = arrays.iter().collect();
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
            .prepare_run(
                &b.router,
                &b.rules,
                &b.temperatures,
                b.source,
                center,
                corners,
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
    arrays: &[crate::biome_selection::ChunkBiomeSamples],
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
