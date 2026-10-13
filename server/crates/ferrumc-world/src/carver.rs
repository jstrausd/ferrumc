//! Original cold Overworld CARVERS computation after an independently published Surface.
//! No live chunk, generator identity, persistence or App activation is introduced.

use crate::biome_selection::{
    BiomeBuildBudget, BiomeCoreError, BiomeSelectionSession, BiomeSourceIdentity, BiomeTree,
    MAX_BIOME_BUILD_WORK,
};
use crate::biome_temperature::BuiltinBiomeTemperatures;
use crate::biome_zoom::{BiomeZoom, BiomeZoomSeed, BiomeZoomSemantics};
use crate::density::{CarverAquifer, NoiseChunkCoreError, NoiseChunkSession, SeededRouter};
use crate::sample_work::{CarverCumulativeMeter, ChargeFault, CoreError, Unsigned192};
use crate::surface_rules::{
    PreliminarySurfaceCorners, SeededSurfaceRules, SurfaceRuleSession, SurfaceRuleWorkBudget,
};
use crate::surface_rules::{SeededSurfaceColumns, SurfaceChunkBuffer};
use crate::{BlockStateId, WorldSeed};
#[cfg(test)]
use ferrumc_math::BlockPos;
use ferrumc_math::ChunkPos;
use ferrumc_registry::biome::BiomeParametersResourceId;
use ferrumc_registry::block_state::state_id_to_block_name;
use ferrumc_registry::worldgen::NoiseSettingsResourceId;
use std::mem::size_of;
use std::sync::Arc;

mod biomes;
mod buffer;
mod error;
mod geometry;
mod replaceable;
mod sin;
pub(crate) use biomes::{CarverClimateQuart, DirectCarverBiomes};
pub use buffer::CarvedChunkBuffer;
pub use error::{
    BindingMismatch, CarverError, CarverErrorKind, CarverLimit, CarverProducer, ProducerMismatch,
    SurfaceMismatch,
};

/// Structural geometry, block, copy/reset and direct-provider requirement.
pub const CARVER_CHUNK_WORK: u64 = 63_912_717_682;
/// Closed public geometry work cap, unchanged from the accepted contract.
pub const MAX_CARVER_WORK: u64 = 1 << 36;
/// Closed public binding census cap.
pub const MAX_CARVER_BIND_WORK: u32 = 65_536;
/// Immutable binding native cap, excluding the separately shared SIN static.
pub const MAX_CARVER_BINDING_BYTES: usize = 4096;
/// Output value and all three owned box payloads.
pub const MAX_CARVED_BUFFER_BYTES: usize = 448 * 1024;
const RUN_VALUE_CAP: usize = 4096;
const NOISE_TOTAL: Unsigned192 = Unsigned192::from_limbs([117_840_306_519, 0, 945]);
const NOISE_REMAINING: Unsigned192 = Unsigned192::from_limbs([96_128_152, 0, 945]);
const PREPARE_RESERVED: u64 = 117_744_178_367;
const SELECTOR_WORK: u64 = 18_716_174_375;
const RULE_WORK: u32 = 45_120_000;

/// Validated structural geometry work budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarverWorkBudget(u64);
impl CarverWorkBudget {
    /// Admits exactly 1 through 2^36.
    pub fn new(units: u64) -> Result<Self, CarverError> {
        if units == 0 || units > MAX_CARVER_WORK {
            return Err(CarverError::new(CarverErrorKind::InvalidBudget {
                units,
                maximum: MAX_CARVER_WORK,
            }));
        }
        Ok(Self(units))
    }
    /// Chosen allowance.
    pub const fn units(self) -> u64 {
        self.0
    }
}
/// Validated immutable binding census budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarverBindBudget(u32);
impl CarverBindBudget {
    /// Admits exactly 1 through 65,536.
    pub fn new(units: u32) -> Result<Self, CarverError> {
        if units == 0 || units > MAX_CARVER_BIND_WORK {
            return Err(CarverError::new(CarverErrorKind::InvalidBudget {
                units: u64::from(units),
                maximum: u64::from(MAX_CARVER_BIND_WORK),
            }));
        }
        Ok(Self(units))
    }
    /// Chosen allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent immutable block classification flags can overlap."
)]
struct Entry {
    state: BlockStateId,
    air: bool,
    fluid: bool,
    replaceable: bool,
    surface: bool,
    dirt: bool,
}
const EMPTY_ENTRY: Entry = Entry {
    state: BlockStateId::AIR,
    air: false,
    fluid: false,
    replaceable: false,
    surface: false,
    dirt: false,
};

/// Immutable closed 35-state binding derived from the accepted Surface producer.
#[derive(Debug)]
pub struct SeededCarvers {
    seed: WorldSeed,
    resource: NoiseSettingsResourceId,
    entries: [Entry; 128],
    len: usize,
}
impl SeededCarvers {
    /// Inherits seed, settings and the genuine producer-derived alphabet; census 90.
    pub fn bind(
        columns: &SeededSurfaceColumns,
        budget: CarverBindBudget,
    ) -> Result<Self, CarverError> {
        Self::bind_with(columns, budget, MAX_CARVER_BINDING_BYTES)
    }
    fn bind_with(
        columns: &SeededSurfaceColumns,
        budget: CarverBindBudget,
        cap: usize,
    ) -> Result<Self, CarverError> {
        let required = columns.alphabet_len() + replaceable::REPLACEABLE.len() + 3;
        if u64::from(budget.0) < required as u64 {
            return Err(CarverError::new(CarverErrorKind::WorkBudgetExceeded {
                required: required as u64,
                available: u64::from(budget.0),
            }));
        }
        if size_of::<Self>() > cap {
            return Err(CarverError::new(CarverErrorKind::NativeLimit {
                category: CarverLimit::Binding,
                required: size_of::<Self>(),
                available: cap,
            }));
        }
        let mut entries = [EMPTY_ENTRY; 128];
        let mut len = 0;
        for state in columns.carver_states() {
            let name = state_id_to_block_name(state.as_u32())
                .ok_or_else(|| CarverError::new(CarverErrorKind::UnclassifiedState))?;
            let slot = entries
                .get_mut(len)
                .ok_or_else(|| CarverError::new(CarverErrorKind::UnclassifiedState))?;
            *slot = Entry {
                state,
                air: ["air", "cave_air", "void_air"].contains(&name),
                fluid: ["water", "lava"].contains(&name),
                replaceable: replaceable::REPLACEABLE.contains(&name),
                surface: ["grass_block", "mycelium"].contains(&name),
                dirt: name == "dirt",
            };
            len += 1;
        }
        Ok(Self {
            seed: columns.seed(),
            resource: columns.resource(),
            entries,
            len,
        })
    }
    /// Bound full signed world seed.
    pub const fn seed(&self) -> WorldSeed {
        self.seed
    }
    /// Bound original noise settings.
    pub const fn resource(&self) -> NoiseSettingsResourceId {
        self.resource
    }
    /// Value bytes; no heap or SIN copy is retained by this binding.
    pub const fn retained_bytes(&self) -> usize {
        size_of::<Self>()
    }
    fn classify(&self, state: BlockStateId) -> Result<Entry, CarverError> {
        self.entries[..self.len]
            .binary_search_by_key(&state.as_u32(), |entry| entry.state.as_u32())
            .ok()
            .map(|i| self.entries[i])
            .ok_or_else(|| CarverError::new(CarverErrorKind::UnclassifiedState))
    }
    /// Admits and constructs one fresh cold run before any source or geometry work.
    ///
    /// `corners` must come from GEN-05.3 preliminary surfaces for this exact
    /// seed, settings and chunk. Their provenance remains caller-trusted, as in
    /// the accepted Surface producer; preparation does not regenerate them.
    pub fn prepare_run<'a>(
        &self,
        router: &SeededRouter,
        rules: &'a SeededSurfaceRules,
        temperatures: &'a BuiltinBiomeTemperatures,
        source: BiomeSourceIdentity,
        chunk: ChunkPos,
        corners: PreliminarySurfaceCorners,
    ) -> Result<PreparedCarverRun<'a>, CarverError> {
        self.prepare_with(
            router,
            rules,
            temperatures,
            source,
            chunk,
            corners,
            NOISE_TOTAL,
            Unsigned192::from_u64(SELECTOR_WORK),
            RULE_WORK,
            RUN_VALUE_CAP,
        )
    }
    /// Reuses a checked immutable builtin tree while constructing fresh run state.
    #[expect(
        clippy::too_many_arguments,
        reason = "Internal preparation also accepts the immutable source binding."
    )]
    pub(crate) fn prepare_run_with_tree<'a>(
        &self,
        router: &SeededRouter,
        rules: &'a SeededSurfaceRules,
        temperatures: &'a BuiltinBiomeTemperatures,
        source: BiomeSourceIdentity,
        chunk: ChunkPos,
        corners: PreliminarySurfaceCorners,
        tree: &Arc<BiomeTree>,
    ) -> Result<PreparedCarverRun<'a>, CarverError> {
        self.prepare_with_tree(
            router,
            rules,
            temperatures,
            source,
            chunk,
            corners,
            NOISE_TOTAL,
            Unsigned192::from_u64(SELECTOR_WORK),
            RULE_WORK,
            RUN_VALUE_CAP,
            Some(tree),
        )
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Private admission controls mirror separate contractual authorities."
    )]
    fn prepare_with<'a>(
        &self,
        router: &SeededRouter,
        rules: &'a SeededSurfaceRules,
        temperatures: &'a BuiltinBiomeTemperatures,
        source: BiomeSourceIdentity,
        chunk: ChunkPos,
        corners: PreliminarySurfaceCorners,
        noise_limit: Unsigned192,
        selector_limit: Unsigned192,
        rule_limit: u32,
        native_cap: usize,
    ) -> Result<PreparedCarverRun<'a>, CarverError> {
        self.prepare_with_tree(
            router,
            rules,
            temperatures,
            source,
            chunk,
            corners,
            noise_limit,
            selector_limit,
            rule_limit,
            native_cap,
            None,
        )
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Private admission controls and immutable tree remain separate authorities."
    )]
    fn prepare_with_tree<'a>(
        &self,
        router: &SeededRouter,
        rules: &'a SeededSurfaceRules,
        temperatures: &'a BuiltinBiomeTemperatures,
        source: BiomeSourceIdentity,
        chunk: ChunkPos,
        corners: PreliminarySurfaceCorners,
        noise_limit: Unsigned192,
        selector_limit: Unsigned192,
        rule_limit: u32,
        native_cap: usize,
        tree: Option<&Arc<BiomeTree>>,
    ) -> Result<PreparedCarverRun<'a>, CarverError> {
        self.admit_binding(rules)?;
        let preset = BiomeParametersResourceId::from_name("minecraft:overworld")
            .ok_or_else(|| producer_mismatch(ProducerMismatch::Source))?;
        if router.seed() != self.seed {
            return Err(producer_mismatch(ProducerMismatch::Seed));
        }
        if router.resource() != self.resource {
            return Err(producer_mismatch(ProducerMismatch::Settings));
        }
        if source != BiomeSourceIdentity::MultiNoise(preset) {
            return Err(producer_mismatch(ProducerMismatch::Source));
        }
        let geometry = router.carver_geometry();
        if geometry.min_y() != -64 || geometry.height() != 384 {
            return Err(producer_mismatch(ProducerMismatch::Geometry));
        }
        for (dx, dz) in [(-8, -8), (-8, 8), (8, -8), (8, 8)] {
            CarverClimateQuart::source(chunk, dx, dz)?;
        }
        if size_of::<PreparedCarverRun<'a>>() > native_cap {
            return Err(CarverError::new(CarverErrorKind::NativeLimit {
                category: CarverLimit::Run,
                required: size_of::<PreparedCarverRun<'a>>(),
                available: native_cap,
            }));
        }
        admit_allowance(
            CarverProducer::Noise,
            NOISE_TOTAL,
            noise_limit,
            Unsigned192::ZERO,
        )?;
        admit_allowance(
            CarverProducer::Selector,
            Unsigned192::from_u64(SELECTOR_WORK),
            selector_limit,
            Unsigned192::ZERO,
        )?;
        admit_allowance(
            CarverProducer::Surface,
            Unsigned192::from_u64(u64::from(RULE_WORK)),
            Unsigned192::from_u64(u64::from(rule_limit)),
            Unsigned192::ZERO,
        )?;
        let tree = if let Some(tree) = tree {
            // BiomeTree has only the checked builtin constructor and no mutable
            // access; its preset proves provenance independently of run history.
            // Check at the original tree-binding stage to preserve refusal order.
            if tree.preset() != preset {
                return Err(producer_mismatch(ProducerMismatch::Source));
            }
            Arc::clone(tree)
        } else {
            Arc::new(
                BiomeTree::from_builtin(
                    preset,
                    BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK)
                        .map_err(|e| CarverError::new(CarverErrorKind::Biome(e.kind().clone())))?,
                )
                .map_err(|e| CarverError::new(CarverErrorKind::Biome(e.kind().clone())))?,
            )
        };
        let mut noise = router
            .fresh_carver_noise_chunk(chunk, CarverCumulativeMeter::new(noise_limit))
            .map_err(noise_error)?;
        let aquifer = CarverAquifer::new(&mut noise).map_err(noise_error)?;
        let used = noise.carver_meter().used();
        if used > Unsigned192::from_u64(PREPARE_RESERVED) {
            return Err(work_error(
                CarverProducer::Noise,
                Unsigned192::from_u64(PREPARE_RESERVED),
                noise_limit,
                used,
            ));
        }
        let selector = BiomeSelectionSession::new_with_carver_meter(
            tree,
            CarverCumulativeMeter::new(selector_limit),
        );
        let context = SurfaceRuleSession::new_carver(
            rules,
            temperatures,
            corners,
            chunk,
            SurfaceRuleWorkBudget::new(rule_limit).map_err(rules_error)?,
        )
        .map_err(rules_error)?;
        Ok(PreparedCarverRun {
            noise,
            aquifer,
            selector,
            context,
            chunk,
            source,
            zoom: BiomeZoom::new(
                BiomeZoomSeed::from_world_seed(self.seed),
                BiomeZoomSemantics::Paper1_21_8Build60,
            ),
            lifecycle: PreparedCarverState::Ready,
            #[cfg(test)]
            trace: None,
        })
    }
    fn admit_binding(&self, rules: &SeededSurfaceRules) -> Result<(), CarverError> {
        if self.seed != rules.seed() {
            return Err(CarverError::new(CarverErrorKind::InconsistentBinding(
                BindingMismatch::Seed,
            )));
        }
        if self.resource != rules.template().resource() {
            return Err(CarverError::new(CarverErrorKind::InconsistentBinding(
                BindingMismatch::Settings,
            )));
        }
        Ok(())
    }
    /// Executes exactly one fully admitted run, with all-or-nothing publication.
    pub fn carve_chunk(
        &self,
        input: CarverInput<'_, '_>,
        budget: CarverWorkBudget,
        output: &mut CarvedChunkBuffer,
    ) -> Result<CarverReport, CarverError> {
        output.identity = None;
        let run = input.run;
        self.admit_binding(run.context.carver_rules())?;
        let (chunk, seed, resource) = input
            .surface
            .published()
            .ok_or_else(|| surface_mismatch(SurfaceMismatch::NotPublished))?;
        if seed != self.seed {
            return Err(surface_mismatch(SurfaceMismatch::Seed));
        }
        if resource != self.resource {
            return Err(surface_mismatch(SurfaceMismatch::Settings));
        }
        if chunk != run.chunk {
            return Err(surface_mismatch(SurfaceMismatch::Chunk));
        }
        run.admit(self)?;
        let geometry = crate::biome_selection::BiomeQuartGeometry::new(
            run.noise.geometry().min_y(),
            run.noise.geometry().height(),
        )
        .map_err(|error| CarverError::new(CarverErrorKind::Biome(error.kind().clone())))?;
        run.context
            .admit_carver(
                crate::stored_biome::StoredBiomeIdentity::new(
                    run.noise.seed(),
                    run.noise.resource(),
                    run.source,
                    geometry,
                ),
                run.zoom.semantics(),
                run.chunk,
            )
            .map_err(rules_error)?;
        admit_allowance(
            CarverProducer::Noise,
            NOISE_REMAINING,
            run.noise.carver_meter().limit(),
            run.noise.carver_meter().used(),
        )?;
        admit_allowance(
            CarverProducer::Selector,
            Unsigned192::from_u64(SELECTOR_WORK),
            run.selector.carver_meter().limit(),
            run.selector.carver_meter().used(),
        )?;
        let rule_remaining = run.context.carver_remaining();
        admit_allowance(
            CarverProducer::Surface,
            Unsigned192::from_u64(u64::from(RULE_WORK)),
            Unsigned192::from_u64(u64::from(rule_remaining)),
            Unsigned192::ZERO,
        )?;
        if budget.0 < CARVER_CHUNK_WORK {
            return Err(CarverError::new(CarverErrorKind::WorkBudgetExceeded {
                required: CARVER_CHUNK_WORK,
                available: budget.0,
            }));
        }
        run.lifecycle = PreparedCarverState::Running;
        let result = geometry::execute(self, input.surface, run, budget, output);
        match result {
            Ok(mut report) => {
                report.noise_work = run.noise.carver_meter().used().limbs();
                report.selector_work = run.selector.carver_meter().used().limbs();
                report.surface_work = run.context.work_used();
                output.identity = Some((chunk, seed, resource));
                run.lifecycle = PreparedCarverState::Complete;
                Ok(report)
            }
            Err(error) => {
                run.lifecycle = PreparedCarverState::Poisoned;
                Err(error)
            }
        }
    }
}
/// One-shot cold run state; no reset or continuation is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedCarverState {
    /// Fully admitted fresh history and producers.
    Ready,
    /// Invocation is executing privately.
    Running,
    /// Successful whole target was published.
    Complete,
    /// Defensive failure prevented publication permanently.
    Poisoned,
}
/// Run-owned Noise, Aquifer and selector plus a separate borrowed immutable rule binding.
pub struct PreparedCarverRun<'a> {
    noise: NoiseChunkSession,
    aquifer: CarverAquifer,
    selector: BiomeSelectionSession,
    context: SurfaceRuleSession<'a>,
    chunk: ChunkPos,
    source: BiomeSourceIdentity,
    zoom: BiomeZoom,
    lifecycle: PreparedCarverState,
    #[cfg(test)]
    pub(crate) trace: Option<Trace>,
}
impl PreparedCarverRun<'_> {
    /// Current one-shot lifecycle.
    pub const fn state(&self) -> PreparedCarverState {
        self.lifecycle
    }
    fn admit(&self, binding: &SeededCarvers) -> Result<(), CarverError> {
        if self.noise.seed() != binding.seed
            || self.zoom.seed() != BiomeZoomSeed::from_world_seed(binding.seed)
        {
            return Err(producer_mismatch(ProducerMismatch::Seed));
        }
        if self.noise.resource() != binding.resource {
            return Err(producer_mismatch(ProducerMismatch::Settings));
        }
        if self.source != BiomeSourceIdentity::MultiNoise(self.selector.preset()) {
            return Err(producer_mismatch(ProducerMismatch::Source));
        }
        if self.noise.geometry().min_y() != -64 || self.noise.geometry().height() != 384 {
            return Err(producer_mismatch(ProducerMismatch::Geometry));
        }
        if self.zoom.semantics() != BiomeZoomSemantics::Paper1_21_8Build60 {
            return Err(producer_mismatch(ProducerMismatch::ZoomSemantics));
        }
        if self.noise.chunk_pos() != self.chunk {
            return Err(producer_mismatch(ProducerMismatch::Chunk));
        }
        if self.lifecycle != PreparedCarverState::Ready {
            return Err(producer_mismatch(ProducerMismatch::Lifecycle));
        }
        Ok(())
    }
}
/// Borrowed independently published Surface and exclusive fresh run authority.
pub struct CarverInput<'call, 'prepared> {
    surface: &'call SurfaceChunkBuffer,
    run: &'call mut PreparedCarverRun<'prepared>,
}
impl<'call, 'prepared> CarverInput<'call, 'prepared> {
    /// No raw state, biome, callback or foreign mutable producer is accepted.
    pub const fn new(
        surface: &'call SurfaceChunkBuffer,
        run: &'call mut PreparedCarverRun<'prepared>,
    ) -> Self {
        Self { surface, run }
    }
}
/// Actual successful counters; private producer totals never truncate to usize.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CarverReport {
    work: u64,
    noise_work: [u64; 3],
    selector_work: [u64; 3],
    surface_work: u32,
    starts: u64,
    writes: u64,
    mask_bits: u64,
    fixups: u64,
    aquifer_calls: u64,
    #[cfg(test)]
    fixup_writes: u64,
    #[cfg(test)]
    schedule_reads: u64,
}
impl CarverReport {
    /// Actual geometry/copy/provider units.
    pub const fn work_used(self) -> u64 {
        self.work
    }
    /// Exact Noise constructor and run total, little-endian limbs.
    pub const fn noise_work_used(self) -> [u64; 3] {
        self.noise_work
    }
    /// Exact selector run total, little-endian limbs.
    pub const fn selector_work_used(self) -> [u64; 3] {
        self.selector_work
    }
    /// Existing cumulative rule work.
    pub const fn surface_work_used(self) -> u32 {
        self.surface_work
    }
    /// Started configured carvers.
    pub const fn starts(self) -> u64 {
        self.starts
    }
    /// Ordered writes, including fix-ups and unchanged states.
    pub const fn writes(self) -> u64 {
        self.writes
    }
    /// New carving-mask bits.
    pub const fn mask_bits(self) -> u64 {
        self.mask_bits
    }
    /// Actual topMaterial invocations.
    pub const fn fixups(self) -> u64 {
        self.fixups
    }
    /// Actual Aquifer invocations, excluding Lava-level bypasses.
    pub const fn aquifer_calls(self) -> u64 {
        self.aquifer_calls
    }
}
pub(crate) struct GeometryWork {
    limit: u64,
    used: u64,
}
impl GeometryWork {
    pub(crate) fn charge(&mut self) -> Result<(), CarverError> {
        if self.used == self.limit {
            return Err(CarverError::new(CarverErrorKind::WorkBudgetExceeded {
                required: self.used + 1,
                available: self.limit,
            }));
        }
        self.used += 1;
        Ok(())
    }
}
fn producer_mismatch(kind: ProducerMismatch) -> CarverError {
    CarverError::new(CarverErrorKind::InconsistentProducer(kind))
}
fn surface_mismatch(kind: SurfaceMismatch) -> CarverError {
    CarverError::new(CarverErrorKind::InconsistentSurface(kind))
}
fn work_error(
    producer: CarverProducer,
    required: Unsigned192,
    limit: Unsigned192,
    used: Unsigned192,
) -> CarverError {
    CarverError::new(CarverErrorKind::ProducerWorkExceeded {
        producer,
        required: required.limbs(),
        limit: limit.limbs(),
        used: used.limbs(),
    })
}
fn admit_allowance(
    producer: CarverProducer,
    required: Unsigned192,
    limit: Unsigned192,
    used: Unsigned192,
) -> Result<(), CarverError> {
    if limit
        .checked_sub(used)
        .is_none_or(|remaining| remaining < required)
    {
        return Err(work_error(producer, required, limit, used));
    }
    Ok(())
}
fn fault_error(producer: CarverProducer, fault: ChargeFault) -> CarverError {
    match fault {
        ChargeFault::Carver { limit, used } => {
            CarverError::new(CarverErrorKind::ProducerWorkExceeded {
                producer,
                required: [1, 0, 0],
                limit,
                used,
            })
        }
        ChargeFault::Public { .. } => unreachable!("closed private run cannot carry public work"),
    }
}
fn noise_error(error: NoiseChunkCoreError) -> CarverError {
    match error {
        CoreError::Error(error) => {
            CarverError::new(CarverErrorKind::Producer(error.kind().clone()))
        }
        CoreError::Work(fault) => fault_error(CarverProducer::Noise, fault),
    }
}
fn biome_error(error: BiomeCoreError) -> CarverError {
    match error {
        CoreError::Error(error) => CarverError::new(CarverErrorKind::Biome(error.kind().clone())),
        CoreError::Work(fault) => fault_error(CarverProducer::Selector, fault),
    }
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Owned error adapter is passed directly to map_err."
)]
fn rules_error(error: crate::surface_rules::SurfaceRuleError) -> CarverError {
    CarverError::new(CarverErrorKind::Rules(error.kind().clone()))
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct Trace {
    pub(crate) events: Vec<String>,
    pub(crate) queries: Vec<(
        bool,
        BlockPos,
        BlockPos,
        ferrumc_registry::biome::BiomeResourceId,
    )>,
    pub(crate) starts: String,
    pub(crate) carves: Vec<(ChunkPos, usize, [u64; 10])>,
    pub(crate) writes: Vec<(BlockPos, BlockStateId)>,
    pub(crate) shadow: Vec<BlockStateId>,
    pub(crate) tunnels: Vec<f32>,
}

// Literal production layouts exclude test observers and independently counted
// retained payloads. They are enforced on the pinned x86-64 toolchain.
#[cfg(not(test))]
const _: () = {
    assert!(size_of::<CarverCumulativeMeter>() == 56);
    assert!(size_of::<crate::sample_work::SampleWork>() == 64);
    assert!(size_of::<DirectCarverBiomes<'_>>() == 48);
    assert!(size_of::<CarverAquifer>() == 144);
};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "carver/tests/tree_reuse.rs"]
mod tree_reuse_tests;
