//! Original builtin biome trees and explicitly owned ordered selection history.
//!
//! Minecraft 1.21.8 / Paper build 60 compute data only. This does not assign
//! client registry IDs, activate a natural generator or define worker scheduling.

use std::sync::Arc;

use ferrumc_registry::biome::{BiomeParametersResourceId, BiomeResourceId};

use crate::density::{ClimateSample, NoiseChunkError};
use crate::sample_work::{CarverCumulativeMeter, ChargeFault, CoreError, SampleWork};

mod arrays;
mod build;
mod end;
mod search;

pub use arrays::{BiomeQuartGeometry, BiomeSourceIdentity, BiomeSourceSession, ChunkBiomeSamples};

/// Largest work allowance for building one complete tree candidate.
pub const MAX_BIOME_BUILD_WORK: u32 = 67_108_864;
/// Largest cumulative selection allowance until explicit successful reset.
pub const MAX_BIOME_SELECTION_WORK: u32 = 268_435_456;
/// Maximum admitted builtin parameter entries.
pub const MAX_BIOME_PARAMETERS: usize = 8192;
/// Maximum retained tree nodes, independently of byte admission.
pub const MAX_BIOME_TREE_NODES: usize = 10_000;
/// Maximum retained tree edges, independently of byte admission.
pub const MAX_BIOME_TREE_EDGES: usize = 9999;
/// Maximum tree depth, including root and leaf.
pub const MAX_BIOME_TREE_DEPTH: usize = 8;
/// Maximum native retained tree/capacity bytes, excluding allocator overhead.
pub const MAX_BIOME_TREE_BYTES: usize = 2 * 1024 * 1024;
/// Maximum simultaneously leased construction scratch capacity bytes.
pub const MAX_BIOME_BUILD_SCRATCH_BYTES: usize = 1024 * 1024;
/// Largest full height-accessor quart array, independently of noise height.
pub const MAX_BIOME_QUART_SAMPLES: usize = 2048;
/// Simultaneous native staged/final output capacity bytes, excluding allocator overhead.
pub const MAX_BIOME_OUTPUT_BYTES: usize = 8192;

/// Independently admitted storage or shape resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeLimit {
    /// Ordered input entries.
    Parameters,
    /// Retained nodes.
    Nodes,
    /// Retained edges.
    Edges,
    /// Root-to-leaf depth.
    Depth,
    /// Retained native capacity bytes.
    TreeBytes,
    /// Simultaneously leased build scratch capacity bytes.
    ScratchBytes,
    /// Complete staged quart output and conversion bytes.
    OutputBytes,
}

/// Classified build, selection, geometry or full-array refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiomeErrorKind {
    /// Budget is outside its explicitly supported closed range.
    InvalidBudget { units: u32, maximum: u32 },
    /// The next work operation was refused before execution.
    WorkExceeded { limit: usize, used: usize },
    /// A separate native capacity or shape cap would be exceeded.
    Limit {
        limit: BiomeLimit,
        count: usize,
        maximum: usize,
    },
    /// A checked reservation failed before a candidate was published.
    Allocation { limit: BiomeLimit },
    /// A private producer violated its graph or resource contract.
    InvalidTree,
    /// Original strict search produced no leaf, rather than a fallback biome.
    NoSelectableLeaf,
    /// A failed operation poisoned this session; an explicit reset is needed.
    Poisoned,
    /// Existing checked noise-chunk computation rejected a query.
    Noise(NoiseChunkError),
    /// The height accessor is not section-aligned, bounded or contains overflow.
    InvalidGeometry,
    /// The accessor would clamp the existing complete noise geometry.
    UnsupportedHeightClamping,
    /// A transformed original sample coordinate escapes the supported domain.
    PositionOutOfRange { axis: &'static str, coordinate: i64 },
}

/// Typed rejection; no partial array or new history is published on failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("original biome computation failed: {kind:?}")]
pub struct BiomeError {
    kind: BiomeErrorKind,
}
impl BiomeError {
    /// Classified failure, without message matching.
    pub const fn kind(&self) -> &BiomeErrorKind {
        &self.kind
    }
}
impl From<NoiseChunkError> for BiomeError {
    fn from(error: NoiseChunkError) -> Self {
        fail(BiomeErrorKind::Noise(error))
    }
}
impl From<crate::sample_work::SampleWorkExceeded> for BiomeError {
    fn from(error: crate::sample_work::SampleWorkExceeded) -> Self {
        fail(BiomeErrorKind::WorkExceeded {
            limit: error.limit,
            used: error.used,
        })
    }
}

pub(crate) type BiomeCoreError = CoreError<BiomeError>;
impl From<BiomeError> for BiomeCoreError {
    fn from(error: BiomeError) -> Self {
        Self::Error(error)
    }
}
fn core_charge(work: &mut SampleWork) -> Result<(), BiomeCoreError> {
    work.charge_core().map_err(|fault| match fault {
        ChargeFault::Public { limit, used } => {
            CoreError::Error(fail(BiomeErrorKind::WorkExceeded { limit, used }))
        }
        fault @ ChargeFault::Carver { .. } => CoreError::Work(fault),
    })
}
fn core_fail(kind: BiomeErrorKind) -> BiomeCoreError {
    fail(kind).into()
}

/// Checked work allowance for one complete immutable tree construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeBuildBudget(u32);
impl BiomeBuildBudget {
    /// Admits exactly `1..=MAX_BIOME_BUILD_WORK` units.
    pub fn new(units: u32) -> Result<Self, BiomeError> {
        budget(units, MAX_BIOME_BUILD_WORK)?;
        Ok(Self(units))
    }
    /// Explicit construction allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

/// Checked cumulative allowance for ordered selection until reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeWorkBudget(u32);
impl BiomeWorkBudget {
    /// Admits exactly `1..=MAX_BIOME_SELECTION_WORK` units.
    pub fn new(units: u32) -> Result<Self, BiomeError> {
        budget(units, MAX_BIOME_SELECTION_WORK)?;
        Ok(Self(units))
    }
    /// Explicit cumulative allowance.
    pub const fn units(self) -> u32 {
        self.0
    }
}

/// All six original quantized Java-long climate coordinates.
///
/// The complete i64 domain is supported using explicit Java wrapping distance
/// arithmetic. The seventh search coordinate is always original offset zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeClimateTarget([i64; 6]);
impl BiomeClimateTarget {
    /// Preserves the six original quantized coordinates exactly.
    pub const fn from_quantized(coordinates: [i64; 6]) -> Self {
        Self(coordinates)
    }
    /// Preserves the existing original f32-to-Java-long quantization results.
    pub const fn from_climate(sample: ClimateSample) -> Self {
        Self(sample.quantized())
    }
    /// Temperature, humidity, continentalness, erosion, depth, weirdness order.
    pub const fn coordinates(self) -> [i64; 6] {
        self.0
    }
}

/// One selected resource and its original ordered parameter position.
/// The parameter position is not a tree-node, wire or persisted biome ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeSelection {
    biome: BiomeResourceId,
    ordinal: u16,
}
impl BiomeSelection {
    /// Checked original biome resource identity.
    pub const fn biome(self) -> BiomeResourceId {
        self.biome
    }
    /// Original parameter-list ordinal, retaining all repeated entries.
    pub const fn parameter_ordinal(self) -> usize {
        self.ordinal as usize
    }
}

#[derive(Clone, Copy)]
struct Node {
    axes: [[i64; 2]; 7],
    kind: NodeKind,
}
#[derive(Clone, Copy)]
enum NodeKind {
    Leaf(u16),
    Branch { children: [u16; 6], len: u8 },
}

/// Immutable bounded original tree compiled from the checked Registry producer.
pub struct BiomeTree {
    preset: BiomeParametersResourceId,
    nodes: Vec<Node>,
    root: u16,
    leaf_nodes: Vec<u16>,
    edges: usize,
    depth: usize,
    retained_bytes: usize,
    build_work: usize,
    build_scratch_peak: usize,
}
impl BiomeTree {
    /// Builds the actual stable seven-axis tree, without a linear fallback.
    /// Construction failure publishes no partial tree or selection history.
    pub fn from_builtin(
        preset: BiomeParametersResourceId,
        allowance: BiomeBuildBudget,
    ) -> Result<Self, BiomeError> {
        build::builtin(preset, allowance)
    }
    /// Checked original multinoise preset.
    pub const fn preset(&self) -> BiomeParametersResourceId {
        self.preset
    }
    /// Actual retained original tree nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// Original ordered leaves, without deduplication.
    pub fn leaf_count(&self) -> usize {
        self.leaf_nodes.len()
    }
    /// Actual retained child edges.
    pub const fn edge_count(&self) -> usize {
        self.edges
    }
    /// Actual root-to-leaf depth, including both ends.
    pub const fn depth(&self) -> usize {
        self.depth
    }
    /// Admitted native capacity bytes, excluding allocator overhead and RSS.
    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    /// Charged construction work for this successful tree.
    pub const fn build_work_used(&self) -> usize {
        self.build_work
    }
    /// Maximum simultaneously leased native build scratch capacity bytes.
    /// This excludes immutable tree bytes and allocator metadata.
    pub const fn build_scratch_peak_bytes(&self) -> usize {
        self.build_scratch_peak
    }
    fn selection(&self, node: u16) -> Result<BiomeSelection, BiomeError> {
        let Some(Node {
            kind: NodeKind::Leaf(ordinal),
            ..
        }) = self.nodes.get(usize::from(node))
        else {
            return Err(fail(BiomeErrorKind::InvalidTree));
        };
        let Some(entry) = self.preset.parameters().get(usize::from(*ordinal)) else {
            return Err(fail(BiomeErrorKind::InvalidTree));
        };
        Ok(BiomeSelection {
            ordinal: *ordinal,
            biome: entry.biome(),
        })
    }
}

/// Explicitly owned ordered history; no thread-local or mutable global cache.
///
/// Moving this session preserves history. Independent sessions sharing a tree
/// remain independent. Work refusal retains prior history and poisons queries
/// until explicit successful cold reset. Scheduler/restart policy is external.
pub struct BiomeSelectionSession {
    tree: Arc<BiomeTree>,
    last: Option<u16>,
    work: SampleWork,
    poisoned: bool,
}
impl BiomeSelectionSession {
    #[cfg(test)]
    pub(crate) fn carver_native_tree(&self) -> (usize, usize) {
        (self.tree.retained_bytes(), Arc::strong_count(&self.tree))
    }
    pub(crate) fn new_with_carver_meter(
        tree: Arc<BiomeTree>,
        meter: CarverCumulativeMeter,
    ) -> Self {
        Self {
            tree,
            last: None,
            work: SampleWork::with_carver_meter(meter),
            poisoned: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn carver_meter_mut(&mut self) -> &mut crate::sample_work::CarverCumulativeMeter {
        self.work.carver_meter_mut()
    }
    pub(crate) fn carver_meter(&self) -> &CarverCumulativeMeter {
        self.work.carver_meter()
    }
    /// Starts deliberately cold with the chosen aggregate allowance.
    pub fn new(tree: Arc<BiomeTree>, allowance: BiomeWorkBudget) -> Self {
        Self {
            tree,
            last: None,
            work: SampleWork::new(allowance.0 as usize),
            poisoned: false,
        }
    }
    /// Checked original source preset.
    pub fn preset(&self) -> BiomeParametersResourceId {
        self.tree.preset
    }
    /// Work already consumed by construction-independent ordered queries.
    pub const fn work_used(&self) -> usize {
        self.work.used()
    }
    /// Whether a previous failed operation requires explicit reset.
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    /// Last fully published original selection, or cold `None`.
    pub fn last_selection(&self) -> Option<BiomeSelection> {
        self.last.and_then(|node| self.tree.selection(node).ok())
    }
    /// Runs the original strict tree search using retained leaf history.
    /// Failure publishes no new leaf and poisons the session.
    pub fn select(&mut self, target: BiomeClimateTarget) -> Result<BiomeSelection, BiomeError> {
        self.select_core(target).map_err(CoreError::into_public)
    }
    pub(crate) fn select_core(
        &mut self,
        target: BiomeClimateTarget,
    ) -> Result<BiomeSelection, BiomeCoreError> {
        if self.poisoned {
            return Err(core_fail(BiomeErrorKind::Poisoned));
        }
        let result = search::select_core(&self.tree, target, self.last, &mut self.work);
        match result {
            Ok((node, selection)) => {
                self.last = Some(node);
                Ok(selection)
            }
            Err(error) => {
                self.poisoned = true;
                Err(error)
            }
        }
    }
    /// Deliberately returns to cold history with a fresh validated allowance.
    /// The immutable tree is retained; no implicit reset occurs on chunk changes.
    pub fn reset(&mut self, allowance: BiomeWorkBudget) {
        self.last = None;
        self.work = SampleWork::new(allowance.0 as usize);
        self.poisoned = false;
    }
}

fn fail(kind: BiomeErrorKind) -> BiomeError {
    BiomeError { kind }
}
fn budget(units: u32, maximum: u32) -> Result<(), BiomeError> {
    if units == 0 || units > maximum {
        Err(fail(BiomeErrorKind::InvalidBudget { units, maximum }))
    } else {
        Ok(())
    }
}
fn limit(resource: BiomeLimit, count: usize, maximum: usize) -> Result<(), BiomeError> {
    if count > maximum {
        Err(fail(BiomeErrorKind::Limit {
            limit: resource,
            count,
            maximum,
        }))
    } else {
        Ok(())
    }
}
fn charge(work: &mut SampleWork) -> Result<(), BiomeError> {
    work.charge().map_err(Into::into)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
impl BiomeSelectionSession {
    pub(crate) fn carver_history(&self) -> (Option<u16>, bool) {
        (self.last, self.poisoned)
    }
}
