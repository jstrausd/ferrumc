//! Complete original section traversal with transactionally retained history.

use super::{
    charge, end, fail, limit, search, Arc, BiomeClimateTarget, BiomeError, BiomeErrorKind,
    BiomeLimit, BiomeParametersResourceId, BiomeResourceId, BiomeSelection, BiomeSelectionSession,
    BiomeTree, BiomeWorkBudget, NoiseChunkError, SampleWork, MAX_BIOME_OUTPUT_BYTES,
    MAX_BIOME_QUART_SAMPLES,
};
use crate::density::{ClimateQuartPos, NoiseChunkSession};
use crate::WorldSeed;
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::worldgen::NoiseSettingsResourceId;
use std::mem::{size_of, size_of_val};

/// Checked complete section-aligned biome height accessor, distinct from noise height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeQuartGeometry {
    min_y: i32,
    height: u32,
}
impl BiomeQuartGeometry {
    /// Admits 1..=32 complete sections and every checked internal block coordinate.
    /// A later consumer independently rejects accessors that clamp its noise height.
    pub fn new(min_y: i32, height: u32) -> Result<Self, BiomeError> {
        let top = i64::from(min_y) + i64::from(height) - 1;
        if min_y % 16 != 0
            || !(16..=512).contains(&height)
            || !height.is_multiple_of(16)
            || top > i64::from(i32::MAX)
        {
            return Err(fail(BiomeErrorKind::InvalidGeometry));
        }
        Ok(Self { min_y, height })
    }
    /// Absolute minimum block Y, aligned to a section boundary.
    pub const fn min_y(self) -> i32 {
        self.min_y
    }
    /// Full biome accessor height, which can exceed the noise height.
    pub const fn height(self) -> u32 {
        self.height
    }
    /// Complete quart values in the 4×4 horizontal chunk column.
    pub const fn sample_count(self) -> usize {
        self.height as usize * 4
    }
    fn admit_noise(self, noise: &NoiseChunkSession) -> Result<(), BiomeError> {
        let actual = noise.geometry();
        if i64::from(self.min_y) > i64::from(actual.min_y())
            || i64::from(self.min_y) + i64::from(self.height)
                < i64::from(actual.min_y()) + i64::from(actual.height())
        {
            return Err(fail(BiomeErrorKind::UnsupportedHeightClamping));
        }
        limit(
            BiomeLimit::OutputBytes,
            self.sample_count(),
            MAX_BIOME_QUART_SAMPLES,
        )
    }
}

/// Explicit biome source identity; settings alone never choose this source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeSourceIdentity {
    /// Checked original multinoise parameter resource.
    MultiNoise(BiomeParametersResourceId),
    /// Checked fixed biome resource; no climate sampling is performed.
    Fixed(BiomeResourceId),
    /// Original five-biome End source using cached f64 erosion.
    End,
}

/// Complete owned biome resource array; local identities are not registry wire IDs.
#[derive(Debug)]
pub struct ChunkBiomeSamples {
    seed: WorldSeed,
    resource: NoiseSettingsResourceId,
    chunk: ChunkPos,
    geometry: BiomeQuartGeometry,
    source: BiomeSourceIdentity,
    values: Box<[BiomeResourceId]>,
    initial: Option<BiomeSelection>,
    final_selection: Option<BiomeSelection>,
}
impl ChunkBiomeSamples {
    /// Full original signed seed bits.
    pub const fn seed(&self) -> WorldSeed {
        self.seed
    }
    /// Explicit noise-settings resource; it does not imply the biome source.
    pub const fn resource(&self) -> NoiseSettingsResourceId {
        self.resource
    }
    /// Admitted complete horizontal chunk column.
    pub const fn chunk_pos(&self) -> ChunkPos {
        self.chunk
    }
    /// Complete biome accessor geometry, independently of noise height.
    pub const fn geometry(&self) -> BiomeQuartGeometry {
        self.geometry
    }
    /// Explicit source identity chosen by the consumer.
    pub const fn source(&self) -> BiomeSourceIdentity {
        self.source
    }
    /// Ascending-quart-Y/X/Z storage; index `((quartY-minQuartY)*4+localX)*4+localZ`.
    /// Original execution order is section-Y then local-X/Y/Z, retaining history.
    pub fn values(&self) -> &[BiomeResourceId] {
        &self.values
    }
    /// Original multinoise history before this array; fixed/End have none.
    pub const fn initial_selection(&self) -> Option<BiomeSelection> {
        self.initial
    }
    /// Original multinoise history after complete publication; fixed/End have none.
    pub const fn final_selection(&self) -> Option<BiomeSelection> {
        self.final_selection
    }
    /// Biome of the enclosing quart only inside this column and full biome height.
    pub fn biome_at(&self, pos: BlockPos) -> Option<BiomeResourceId> {
        if pos.to_chunk_pos() != self.chunk {
            return None;
        }
        let relative = i64::from(pos.y()) - i64::from(self.geometry.min_y);
        if !(0..i64::from(self.geometry.height)).contains(&relative) {
            return None;
        }
        let y = usize::try_from(relative / 4).ok()?;
        let x = usize::try_from((pos.x() & 15) >> 2).ok()?;
        let z = usize::try_from((pos.z() & 15) >> 2).ok()?;
        self.values.get((y * 4 + x) * 4 + z).copied()
    }
}

// A sealed successful private composition transition. Its tree lease prevents a
// foreign tree with the same preset from supplying a different tie history.
pub(crate) struct NaturalBiomeTransition {
    tree: Arc<BiomeTree>,
    initial: Option<u16>,
    final_leaf: Option<u16>,
    work: usize,
    samples: Arc<ChunkBiomeSamples>,
}
impl NaturalBiomeTransition {
    pub(crate) fn key(&self) -> (ChunkPos, Option<u16>) {
        (self.samples.chunk, self.initial)
    }
    pub(crate) fn samples(&self) -> Arc<ChunkBiomeSamples> {
        Arc::clone(&self.samples)
    }
    // The record itself is already charged in the cache's deque capacity.
    pub(crate) fn allocation_bytes(&self) -> usize {
        size_of::<ChunkBiomeSamples>()
            + size_of_val(self.samples.values.as_ref())
            + 2 * size_of::<usize>()
    }
}

enum Source {
    MultiNoise(Arc<BiomeTree>),
    Fixed(BiomeResourceId),
    End([BiomeResourceId; 5]),
}

/// Exclusively owned explicit source, cumulative work and multinoise history.
/// Arrays commit history only after full completion. Executed failure poisons
/// both this source and its noise-chunk biome phase; both require explicit reset.
pub struct BiomeSourceSession {
    source: Source,
    last: Option<u16>,
    work: SampleWork,
    poisoned: bool,
    #[cfg(test)]
    trace: Option<Vec<(ClimateQuartPos, BiomeResourceId, Option<BiomeSelection>)>>,
}
impl BiomeSourceSession {
    #[cfg(test)]
    pub(super) fn enable_trace(&mut self) {
        self.trace = Some(Vec::with_capacity(MAX_BIOME_QUART_SAMPLES));
    }
    #[cfg(test)]
    pub(super) fn take_trace(
        &mut self,
    ) -> Vec<(ClimateQuartPos, BiomeResourceId, Option<BiomeSelection>)> {
        self.trace.take().unwrap()
    }
    /// Transfers the scalar session's exact tree, history, cumulative meter and status.
    pub fn from_multinoise(session: BiomeSelectionSession) -> Self {
        Self {
            source: Source::MultiNoise(session.tree),
            last: session.last,
            work: session.work,
            poisoned: session.poisoned,
            #[cfg(test)]
            trace: None,
        }
    }
    /// Starts one fixed checked resource, with no climate evaluation or history.
    pub fn fixed(biome: BiomeResourceId, allowance: BiomeWorkBudget) -> Self {
        Self {
            source: Source::Fixed(biome),
            last: None,
            work: SampleWork::new(allowance.0 as usize),
            poisoned: false,
            #[cfg(test)]
            trace: None,
        }
    }
    /// Starts the exact five original End resource keys, without a fallback.
    pub fn the_end(allowance: BiomeWorkBudget) -> Result<Self, BiomeError> {
        let [Some(end), Some(highlands), Some(midlands), Some(islands), Some(barrens)] = [
            "minecraft:the_end",
            "minecraft:end_highlands",
            "minecraft:end_midlands",
            "minecraft:small_end_islands",
            "minecraft:end_barrens",
        ]
        .map(BiomeResourceId::from_name) else {
            return Err(fail(BiomeErrorKind::InvalidTree));
        };
        let values = [end, highlands, midlands, islands, barrens];
        Ok(Self {
            source: Source::End(values),
            last: None,
            work: SampleWork::new(allowance.0 as usize),
            poisoned: false,
            #[cfg(test)]
            trace: None,
        })
    }
    /// Explicit source identity, preserved across reset and unrelated chunk changes.
    pub fn identity(&self) -> BiomeSourceIdentity {
        match &self.source {
            Source::MultiNoise(tree) => BiomeSourceIdentity::MultiNoise(tree.preset),
            Source::Fixed(biome) => BiomeSourceIdentity::Fixed(*biome),
            Source::End(_) => BiomeSourceIdentity::End,
        }
    }
    /// Shared source work consumed since the last explicit reset.
    pub const fn work_used(&self) -> usize {
        self.work.used()
    }
    /// Executed failure status; no automatic recovery occurs on a new chunk.
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    /// Last complete published multinoise leaf; fixed and End have no leaf history.
    pub fn last_selection(&self) -> Option<BiomeSelection> {
        match &self.source {
            Source::MultiNoise(tree) => self.last.and_then(|n| tree.selection(n).ok()),
            _ => None,
        }
    }
    /// Resets to live/cold/zero-work using a validated allowance, without allocation.
    pub fn reset(&mut self, allowance: BiomeWorkBudget) {
        self.work = SampleWork::new(allowance.0 as usize);
        self.last = None;
        self.poisoned = false;
    }
    /// Runs exactly one complete biome phase before the noise-chunk material phase.
    /// Admission rejection leaves live work/history unchanged. Executed failures
    /// drop partial output, retain prior history and require both sessions' reset.
    pub fn sample_chunk_biomes(
        &mut self,
        noise: &mut NoiseChunkSession,
        geometry: BiomeQuartGeometry,
    ) -> Result<ChunkBiomeSamples, BiomeError> {
        if self.poisoned {
            return Err(fail(BiomeErrorKind::Poisoned));
        }
        geometry.admit_noise(noise)?;
        noise.begin_biome_array()?;
        let initial = self.last_selection();
        let mut last = self.last;
        let result = self.fill(Some(noise), None, geometry, &mut last);
        noise.finish_biome_array(result.is_ok());
        match result {
            Ok(values) => {
                self.last = last;
                Ok(ChunkBiomeSamples {
                    seed: noise.seed(),
                    resource: noise.resource(),
                    chunk: noise.chunk_pos(),
                    geometry,
                    source: self.identity(),
                    values,
                    initial,
                    final_selection: self.last_selection(),
                })
            }
            Err(error) => {
                self.poisoned = true;
                Err(error)
            }
        }
    }
    /// Replays a sealed natural producer's targets with exclusively owned history.
    pub(crate) fn sample_natural_climate(
        &mut self,
        cached: &NaturalClimateSamples,
    ) -> Result<ChunkBiomeSamples, BiomeError> {
        if self.poisoned {
            return Err(fail(BiomeErrorKind::Poisoned));
        }
        if !matches!(self.source, Source::MultiNoise(_)) {
            return Err(fail(BiomeErrorKind::InvalidTree));
        }
        let initial = self.last_selection();
        let mut last = self.last;
        match self.fill(None, Some(cached), cached.geometry, &mut last) {
            Ok(values) => {
                self.last = last;
                Ok(ChunkBiomeSamples {
                    seed: cached.seed,
                    resource: cached.resource,
                    chunk: cached.chunk,
                    geometry: cached.geometry,
                    source: self.identity(),
                    values,
                    initial,
                    final_selection: self.last_selection(),
                })
            }
            Err(error) => {
                self.poisoned = true;
                Err(error)
            }
        }
    }
    pub(crate) fn natural_transition_key(&self, pos: ChunkPos) -> Option<(ChunkPos, Option<u16>)> {
        #[cfg(test)]
        if self.trace.is_some() {
            return None;
        }
        if !self.poisoned && matches!(self.source, Source::MultiNoise(_)) {
            Some((pos, self.last))
        } else {
            None
        }
    }
    pub(crate) fn sample_natural_transition(
        &mut self,
        climate: &NaturalClimateSamples,
    ) -> Result<NaturalBiomeTransition, BiomeError> {
        let initial = self.last;
        let before = self.work.used();
        let samples = Arc::new(self.sample_natural_climate(climate)?);
        let Source::MultiNoise(tree) = &self.source else {
            return Err(fail(BiomeErrorKind::InvalidTree));
        };
        Ok(NaturalBiomeTransition {
            tree: Arc::clone(tree),
            initial,
            final_leaf: self.last,
            work: self.work.used() - before,
            samples,
        })
    }
    pub(crate) fn replay_natural_transition(
        &mut self,
        climate: &NaturalClimateSamples,
        saved: &NaturalBiomeTransition,
    ) -> Option<Arc<ChunkBiomeSamples>> {
        if self.natural_transition_key(climate.chunk)? != saved.key() {
            return None;
        }
        let Source::MultiNoise(tree) = &self.source else {
            return None;
        };
        let samples = &saved.samples;
        if !Arc::ptr_eq(tree, &saved.tree)
            || samples.seed != climate.seed
            || samples.resource != climate.resource
            || samples.chunk != climate.chunk
            || samples.geometry != climate.geometry
            || samples.source != self.identity()
            || !self.work.try_charge_public_group(saved.work)
        {
            return None;
        }
        self.last = saved.final_leaf;
        Some(Arc::clone(samples))
    }
    // Keep the original charge/selection/staging transaction in one visible traversal.
    #[allow(clippy::too_many_lines)]
    fn fill(
        &mut self,
        mut noise: Option<&mut NoiseChunkSession>,
        cached: Option<&NaturalClimateSamples>,
        geometry: BiomeQuartGeometry,
        last: &mut Option<u16>,
    ) -> Result<Box<[BiomeResourceId]>, BiomeError> {
        let count = geometry.sample_count();
        let admitted = count
            .checked_mul(size_of::<Option<BiomeResourceId>>() + size_of::<BiomeResourceId>())
            .ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))?;
        limit(BiomeLimit::OutputBytes, admitted, MAX_BIOME_OUTPUT_BYTES)?;
        charge(&mut self.work)?;
        let mut staged = reserve::<Option<BiomeResourceId>>(count)?;
        staged.resize(count, None);
        charge(&mut self.work)?;
        let mut values = reserve::<BiomeResourceId>(count)?;
        let capacities = staged.capacity() * size_of::<Option<BiomeResourceId>>()
            + values.capacity() * size_of::<BiomeResourceId>();
        limit(BiomeLimit::OutputBytes, capacities, MAX_BIOME_OUTPUT_BYTES)?;
        let origin = if let Some(cached) = cached {
            cached.chunk
        } else {
            require_noise(&mut noise)?.chunk_pos()
        };
        let quart_x = i64::from(origin.x()) * 4;
        let quart_z = i64::from(origin.z()) * 4;
        let min_quart_y = i64::from(geometry.min_y) / 4;
        for section in 0..geometry.height / 16 {
            charge(&mut self.work)?;
            for x in 0..4_usize {
                charge(&mut self.work)?;
                for y in 0..4_usize {
                    charge(&mut self.work)?;
                    for z in 0..4_usize {
                        charge(&mut self.work)?;
                        let absolute_y = min_quart_y
                            + i64::from(section) * 4
                            + i64::try_from(y)
                                .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?;
                        let pos = ClimateQuartPos::new(
                            i32::try_from(
                                quart_x
                                    + i64::try_from(x)
                                        .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?,
                            )
                            .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?,
                            i32::try_from(absolute_y)
                                .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?,
                            i32::try_from(
                                quart_z
                                    + i64::try_from(z)
                                        .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?,
                            )
                            .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?,
                        )
                        .map_err(|error| BiomeError::from(NoiseChunkError::from(error)))?;
                        charge(&mut self.work)?;
                        let biome = match &self.source {
                            Source::MultiNoise(tree) => {
                                let target = if let Some(cached) = cached {
                                    let quart_y = usize::try_from(absolute_y - min_quart_y)
                                        .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?;
                                    cached.values[(quart_y * 4 + x) * 4 + z]
                                } else {
                                    BiomeClimateTarget::from_climate(
                                        require_noise(&mut noise)?.sample_climate(pos)?,
                                    )
                                };
                                let (node, selection) =
                                    search::select(tree, target, *last, &mut self.work)?;
                                *last = Some(node);
                                selection.biome()
                            }
                            Source::Fixed(biome) => *biome,
                            Source::End(biomes) => end::select(
                                *biomes,
                                pos,
                                require_noise(&mut noise)?,
                                &mut self.work,
                            )?,
                        };
                        charge(&mut self.work)?;
                        let quart_y = usize::try_from(absolute_y - min_quart_y)
                            .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?;
                        let index = (quart_y * 4 + x) * 4 + z;
                        let destination = staged
                            .get_mut(index)
                            .ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))?;
                        if destination.is_some() {
                            return Err(fail(BiomeErrorKind::InvalidGeometry));
                        }
                        *destination = Some(biome);
                        #[cfg(test)]
                        if let Some(trace) = &mut self.trace {
                            assert!(trace.len() < MAX_BIOME_QUART_SAMPLES);
                            let selection = match &self.source {
                                Source::MultiNoise(tree) => {
                                    last.and_then(|node| tree.selection(node).ok())
                                }
                                _ => None,
                            };
                            trace.push((pos, biome, selection));
                        }
                    }
                }
            }
        }
        for biome in staged {
            charge(&mut self.work)?;
            values.push(biome.ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))?);
        }
        charge(&mut self.work)?;
        Ok(values.into_boxed_slice())
    }
}

fn require_noise<'a>(
    noise: &'a mut Option<&mut NoiseChunkSession>,
) -> Result<&'a mut NoiseChunkSession, BiomeError> {
    noise
        .as_deref_mut()
        .ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))
}

fn reserve<T>(count: usize) -> Result<Vec<T>, BiomeError> {
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| {
        fail(BiomeErrorKind::Allocation {
            limit: BiomeLimit::OutputBytes,
        })
    })?;
    Ok(values)
}

/// Complete immutable targets from one genuinely fresh noise chunk. This type is
/// private to natural composition: no foreign biome selection or mutable cache
/// history is captured, and the noise producer must complete before publication.
pub(crate) struct NaturalClimateSamples {
    seed: WorldSeed,
    resource: NoiseSettingsResourceId,
    chunk: ChunkPos,
    geometry: BiomeQuartGeometry,
    values: Box<[BiomeClimateTarget]>,
}
impl NaturalClimateSamples {
    pub(crate) fn sample(
        noise: &mut NoiseChunkSession,
        geometry: BiomeQuartGeometry,
    ) -> Result<Self, BiomeError> {
        geometry.admit_noise(noise)?;
        noise.begin_biome_array()?;
        let result = Self::fill(noise, geometry);
        noise.finish_biome_array(result.is_ok());
        result
    }
    fn fill(
        noise: &mut NoiseChunkSession,
        geometry: BiomeQuartGeometry,
    ) -> Result<Self, BiomeError> {
        let count = geometry.sample_count();
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|_| {
            fail(BiomeErrorKind::Allocation {
                limit: BiomeLimit::OutputBytes,
            })
        })?;
        values.resize(count, BiomeClimateTarget::from_quantized([0; 6]));
        let origin = noise.chunk_pos();
        let min_y = geometry.min_y / 4;
        for section in 0..geometry.height / 16 {
            for x in 0..4_usize {
                for y in 0..4_usize {
                    for z in 0..4_usize {
                        let quart_y = section as usize * 4 + y;
                        let pos = ClimateQuartPos::new(
                            origin
                                .x()
                                .checked_mul(4)
                                .and_then(|v| v.checked_add(i32::try_from(x).ok()?))
                                .ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))?,
                            min_y
                                + i32::try_from(quart_y)
                                    .map_err(|_| fail(BiomeErrorKind::InvalidGeometry))?,
                            origin
                                .z()
                                .checked_mul(4)
                                .and_then(|v| v.checked_add(i32::try_from(z).ok()?))
                                .ok_or_else(|| fail(BiomeErrorKind::InvalidGeometry))?,
                        )
                        .map_err(|error| BiomeError::from(NoiseChunkError::from(error)))?;
                        values[(quart_y * 4 + x) * 4 + z] =
                            BiomeClimateTarget::from_climate(noise.sample_climate(pos)?);
                    }
                }
            }
        }
        Ok(Self {
            seed: noise.seed(),
            resource: noise.resource(),
            chunk: origin,
            geometry,
            values: values.into_boxed_slice(),
        })
    }
}

impl NaturalClimateSamples {
    pub(crate) fn native_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + std::mem::size_of_val(self.values.as_ref())
    }
    #[cfg(test)]
    pub(crate) fn targets(&self) -> &[BiomeClimateTarget] {
        &self.values
    }
}

#[cfg(test)]
mod natural_replay_tests {
    use super::*;
    use crate::biome_selection::{
        BiomeBuildBudget, MAX_BIOME_BUILD_WORK, MAX_BIOME_SELECTION_WORK,
    };
    use crate::density::{NoiseChunkWorkBudget, RouterTemplate, MAX_NOISE_CHUNK_WORK};

    #[test]
    fn sealed_climate_replays_full_centered_history_and_work() {
        let resource = NoiseSettingsResourceId::from_name("minecraft:overworld").unwrap();
        let router = RouterTemplate::compile_builtin(resource)
            .unwrap()
            .bind_seed(WorldSeed::new(17))
            .unwrap();
        let tree = Arc::new(
            BiomeTree::from_builtin(
                BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
                BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
            )
            .unwrap(),
        );
        let make = |allowance| {
            BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
                Arc::clone(&tree),
                BiomeWorkBudget::new(allowance).unwrap(),
            ))
        };
        let mut original = make(MAX_BIOME_SELECTION_WORK);
        let mut replay = make(MAX_BIOME_SELECTION_WORK);
        let geometry = BiomeQuartGeometry::new(-64, 384).unwrap();
        let noise = |pos| {
            router
                .fresh_paper_noise_chunk(
                    pos,
                    NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
                )
                .unwrap()
        };
        for x in -2..=0 {
            for z in -2..=0 {
                let pos = ChunkPos::new(x, z);
                let mut producer = noise(pos);
                let cached = NaturalClimateSamples::sample(&mut producer, geometry).unwrap();
                assert_eq!(cached.values.len(), 1536);
                assert_eq!(std::mem::size_of_val(cached.values.as_ref()), 73728);
                let actual = replay.sample_natural_climate(&cached).unwrap();
                let expected = original
                    .sample_chunk_biomes(&mut noise(pos), geometry)
                    .unwrap();
                assert_eq!(actual.values(), expected.values());
                assert_eq!(actual.initial_selection(), expected.initial_selection());
                assert_eq!(actual.final_selection(), expected.final_selection());
                assert_eq!(replay.work_used(), original.work_used());
            }
        }
        let cached = NaturalClimateSamples::sample(&mut noise(ChunkPos::ORIGIN), geometry).unwrap();
        let mut complete = make(MAX_BIOME_SELECTION_WORK);
        complete.sample_natural_climate(&cached).unwrap();
        let required = u32::try_from(complete.work_used()).unwrap();
        for allowance in [1, 2, 3, 64, required - 1, required, required + 1] {
            let mut actual = make(allowance);
            let mut expected = make(allowance);
            let a = actual.sample_natural_climate(&cached);
            let e = expected.sample_chunk_biomes(&mut noise(ChunkPos::ORIGIN), geometry);
            assert_eq!(
                a.as_ref().map(ChunkBiomeSamples::values),
                e.as_ref().map(ChunkBiomeSamples::values)
            );
            assert_eq!(actual.work_used(), expected.work_used());
            assert_eq!(actual.last_selection(), expected.last_selection());
            assert_eq!(actual.is_poisoned(), expected.is_poisoned());
        }
    }
}

#[cfg(test)]
mod natural_transition_tests {
    use super::*;
    use crate::biome_selection::{
        BiomeBuildBudget, MAX_BIOME_BUILD_WORK, MAX_BIOME_SELECTION_WORK,
    };
    use crate::density::{NoiseChunkWorkBudget, RouterTemplate, MAX_NOISE_CHUNK_WORK};

    fn setup() -> (Arc<BiomeTree>, NaturalClimateSamples, NaturalClimateSamples) {
        let resource = NoiseSettingsResourceId::from_name("minecraft:overworld").unwrap();
        let router = RouterTemplate::compile_builtin(resource)
            .unwrap()
            .bind_seed(WorldSeed::new(17))
            .unwrap();
        let tree = Arc::new(
            BiomeTree::from_builtin(
                BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
                BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
            )
            .unwrap(),
        );
        let climate = |pos| {
            NaturalClimateSamples::sample(
                &mut router
                    .fresh_paper_noise_chunk(
                        pos,
                        NoiseChunkWorkBudget::new(MAX_NOISE_CHUNK_WORK).unwrap(),
                    )
                    .unwrap(),
                BiomeQuartGeometry::new(-64, 384).unwrap(),
            )
            .unwrap()
        };
        (
            tree,
            climate(ChunkPos::new(-1, -1)),
            climate(ChunkPos::ORIGIN),
        )
    }
    fn source(tree: &Arc<BiomeTree>, allowance: u32) -> BiomeSourceSession {
        BiomeSourceSession::from_multinoise(BiomeSelectionSession::new(
            Arc::clone(tree),
            BiomeWorkBudget::new(allowance).unwrap(),
        ))
    }
    fn compare(actual: &BiomeSourceSession, expected: &BiomeSourceSession) {
        assert_eq!(actual.work_used(), expected.work_used());
        assert_eq!(actual.last, expected.last);
        assert_eq!(actual.last_selection(), expected.last_selection());
        assert_eq!(actual.is_poisoned(), expected.is_poisoned());
    }
    #[test]
    fn transition_exact_allowance_history_and_first_refusal() {
        let (tree, prefix, climate) = setup();
        for warm in [false, true] {
            let mut capture = source(&tree, MAX_BIOME_SELECTION_WORK);
            if warm {
                capture.sample_natural_climate(&prefix).unwrap();
            }
            let prefix_work = capture.work_used();
            let saved = capture.sample_natural_transition(&climate).unwrap();
            let required = u32::try_from(prefix_work + saved.work).unwrap();
            for allowance in [required - 1, required, required + 1] {
                let mut actual = source(&tree, allowance);
                let mut expected = source(&tree, allowance);
                if warm {
                    actual.sample_natural_climate(&prefix).unwrap();
                    expected.sample_natural_climate(&prefix).unwrap();
                }
                let before = actual.work_used();
                let initial = actual.last;
                let hit = actual.replay_natural_transition(&climate, &saved);
                assert_eq!(hit.is_some(), allowance >= required);
                if hit.is_none() {
                    assert_eq!(actual.work_used(), before);
                    assert_eq!(actual.last, initial);
                    assert!(!actual.is_poisoned());
                }
                let result =
                    hit.map_or_else(|| actual.sample_natural_climate(&climate).map(Arc::new), Ok);
                let original = expected.sample_natural_climate(&climate);
                assert_eq!(
                    result
                        .as_ref()
                        .map(|v| v.values())
                        .map_err(BiomeError::kind),
                    original
                        .as_ref()
                        .map(ChunkBiomeSamples::values)
                        .map_err(BiomeError::kind)
                );
                if let (Ok(a), Ok(e)) = (&result, &original) {
                    assert_eq!(a.initial_selection(), e.initial_selection());
                    assert_eq!(a.final_selection(), e.final_selection());
                    assert_eq!(a.seed(), e.seed());
                    assert_eq!(a.source(), e.source());
                    assert_eq!(a.chunk_pos(), e.chunk_pos());
                    assert_eq!(a.geometry(), e.geometry());
                }
                compare(&actual, &expected);
                if result.is_err() {
                    assert!(actual.replay_natural_transition(&climate, &saved).is_none());
                    assert_eq!(
                        actual.sample_natural_climate(&climate).unwrap_err().kind(),
                        &BiomeErrorKind::Poisoned
                    );
                    compare(&actual, &expected);
                }
            }
        }
    }
    #[test]
    fn transition_rejects_foreign_context_history_trace_and_poison() {
        let (tree, prefix, climate) = setup();
        let saved = source(&tree, MAX_BIOME_SELECTION_WORK)
            .sample_natural_transition(&climate)
            .unwrap();
        let (foreign, _, _) = setup();
        let mut different_tree = source(&foreign, MAX_BIOME_SELECTION_WORK);
        assert!(different_tree
            .replay_natural_transition(&climate, &saved)
            .is_none());
        assert_eq!(different_tree.work_used(), 0);
        let mut warm = source(&tree, MAX_BIOME_SELECTION_WORK);
        warm.sample_natural_climate(&prefix).unwrap();
        assert!(warm.last.is_some());
        let before = warm.work_used();
        let last = warm.last;
        assert!(warm.replay_natural_transition(&climate, &saved).is_none());
        assert_eq!(warm.work_used(), before);
        assert_eq!(warm.last, last);
        for mismatch in 0..4 {
            let mut altered = NaturalClimateSamples {
                seed: climate.seed,
                resource: climate.resource,
                chunk: climate.chunk,
                geometry: climate.geometry,
                values: climate.values.clone(),
            };
            match mismatch {
                0 => altered.seed = WorldSeed::new(-17),
                1 => {
                    altered.resource =
                        NoiseSettingsResourceId::from_name("minecraft:nether").unwrap();
                }
                2 => altered.chunk = ChunkPos::new(1, 0),
                _ => altered.geometry = BiomeQuartGeometry::new(-64, 400).unwrap(),
            }
            let mut actual = source(&tree, MAX_BIOME_SELECTION_WORK);
            assert!(actual.replay_natural_transition(&altered, &saved).is_none());
            assert_eq!(actual.work_used(), 0);
            assert_eq!(actual.last, None);
        }
        let mut fixed = BiomeSourceSession::fixed(
            saved.samples.values[0],
            BiomeWorkBudget::new(MAX_BIOME_SELECTION_WORK).unwrap(),
        );
        assert!(fixed.replay_natural_transition(&climate, &saved).is_none());
        assert_eq!(fixed.work_used(), 0);
        let mut traced = source(&tree, MAX_BIOME_SELECTION_WORK);
        traced.enable_trace();
        assert!(traced.replay_natural_transition(&climate, &saved).is_none());
        assert_eq!(traced.work_used(), 0);
        traced.sample_natural_climate(&climate).unwrap();
        assert_eq!(traced.take_trace().len(), 1536);
        let mut poisoned = source(&tree, 1);
        assert!(poisoned.sample_natural_climate(&climate).is_err());
        assert!(poisoned
            .replay_natural_transition(&climate, &saved)
            .is_none());
        assert_eq!(poisoned.work_used(), 1);
    }
}
