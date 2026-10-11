//! Borrowed complete stored-biome neighborhoods and the original stored lookup.
//!
//! This connects sealed GEN-05.5 [`ChunkBiomeSamples`] to the GEN-05.6 zoom
//! selector with the Minecraft 1.21.8 / Paper build 60 `getNoiseBiome` stored
//! lookup: signed chunk selection, local quart masking, vertical clamping and the
//! existing quart-Y/X/Z storage order. It performs no biome or density
//! generation, source re-evaluation, surface rule, fallback biome, chunk
//! installation, persistence or I/O. Missing data is a typed refusal.

use std::mem::size_of;

use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_registry::biome::BiomeResourceId;
use ferrumc_registry::worldgen::NoiseSettingsResourceId;

use crate::biome_selection::{
    BiomeQuartGeometry, BiomeSelection, BiomeSourceIdentity, ChunkBiomeSamples,
};
use crate::biome_zoom::{BiomeQuartPos, BiomeZoom, BiomeZoomSeed, BiomeZoomSemantics};
use crate::density::admit_chunk;
use crate::WorldSeed;

/// Maximum borrowed arrays: the centre chunk and its complete 3×3 window.
pub const MAX_STORED_BIOME_CHUNKS: usize = 9;
/// Maximum values per borrowed array (32 sections of 64 quarts).
pub const MAX_STORED_BIOME_VALUES_PER_CHUNK: usize = 2048;
/// Maximum borrowed values over all arrays of one neighborhood.
pub const MAX_STORED_BIOME_BORROWED_VALUES: usize = 18_432;
/// Maximum native bytes retained by one neighborhood or surface resolver value.
pub const MAX_STORED_BIOME_RETAINED_BYTES: usize = 512;
/// Maximum simultaneously live native construction scratch bytes.
pub const MAX_STORED_BIOME_SCRATCH_BYTES: usize = 4096;
/// Largest construction admission allowance.
pub const MAX_STORED_BIOME_BUILD_WORK: u32 = 1_048_576;
/// `FerrumC` policy bound for block X/Z queries, inclusive on both signs.
pub const MAX_STORED_BIOME_BLOCK_COORDINATE: i32 = 30_000_000;

/// Admission units for candidate start and publication.
const CANDIDATE_WORK: usize = 2;
/// Admission units per supplied array: window/key, duplicate, seed, settings,
/// source, geometry, shape, history, endpoint and installation.
const PER_ARRAY_WORK: usize = 10;
/// Admission units for the nine complete surface-halo slots.
const SURFACE_COVERAGE_WORK: usize = 9;
/// Reserved bytes for the one-block SHA-256 zoom-seed hash and call temporaries.
const HASH_AND_CALL_ALLOWANCE: usize = 1024;

/// Conservative simultaneously live typed construction objects, in bytes.
///
/// Candidate core, published surface value, staged slot array, refusal value,
/// copied identity and the reserved hash/call allowance are counted together.
const fn construction_scratch_bytes() -> usize {
    size_of::<Core<'static, ChunkBiomeSamples>>()
        + size_of::<SurfaceBiomeResolver<'static>>()
        + size_of::<[Option<&'static ChunkBiomeSamples>; MAX_STORED_BIOME_CHUNKS]>()
        + size_of::<StoredBiomeError>()
        + size_of::<StoredBiomeIdentity>()
        + HASH_AND_CALL_ALLOWANCE
}

/// Expected producer identity every borrowed array must match exactly.
///
/// Only existing checked types form this identity; there is no textual,
/// digest or raw-array authority, and an identity alone proves no completeness.
///
/// ```compile_fail
/// use ferrumc_registry::worldgen::NoiseSettingsResourceId;
/// use ferrumc_world::biome_selection::{BiomeQuartGeometry, BiomeSourceIdentity};
/// use ferrumc_world::biome_zoom::BiomeZoomSeed;
/// use ferrumc_world::stored_biome::StoredBiomeIdentity;
/// use ferrumc_world::WorldSeed;
/// // An already-obfuscated zoom seed is not a world seed.
/// let identity = StoredBiomeIdentity::new(
///     BiomeZoomSeed::from_world_seed(WorldSeed::new(0)),
///     NoiseSettingsResourceId::from_name("minecraft:overworld").unwrap(),
///     BiomeSourceIdentity::End,
///     BiomeQuartGeometry::new(0, 256).unwrap(),
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredBiomeIdentity {
    seed: WorldSeed,
    resource: NoiseSettingsResourceId,
    source: BiomeSourceIdentity,
    geometry: BiomeQuartGeometry,
}

impl StoredBiomeIdentity {
    /// Copies the expected world seed, settings, explicit source and geometry.
    #[must_use]
    pub const fn new(
        seed: WorldSeed,
        resource: NoiseSettingsResourceId,
        source: BiomeSourceIdentity,
        geometry: BiomeQuartGeometry,
    ) -> Self {
        Self {
            seed,
            resource,
            source,
            geometry,
        }
    }

    /// Expected world seed whose zoom seed is derived exactly once.
    #[must_use]
    pub const fn seed(self) -> WorldSeed {
        self.seed
    }

    /// Expected checked noise-settings resource.
    #[must_use]
    pub const fn resource(self) -> NoiseSettingsResourceId {
        self.resource
    }

    /// Expected explicit biome source; never inferred from settings.
    #[must_use]
    pub const fn source(self) -> BiomeSourceIdentity {
        self.source
    }

    /// Expected complete biome accessor geometry.
    #[must_use]
    pub const fn geometry(self) -> BiomeQuartGeometry {
        self.geometry
    }
}

/// Checked construction admission allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredBiomeBuildBudget(u32);

impl StoredBiomeBuildBudget {
    /// Admits exactly `1..=MAX_STORED_BIOME_BUILD_WORK` units.
    pub fn new(units: u32) -> Result<Self, StoredBiomeError> {
        if units == 0 || units > MAX_STORED_BIOME_BUILD_WORK {
            return Err(
                StoredBiomeError::new(StoredBiomeErrorKind::InvalidBudget).with_limit(
                    StoredBiomeLimit::ConstructionWork,
                    units as usize,
                    MAX_STORED_BIOME_BUILD_WORK as usize,
                ),
            );
        }
        Ok(Self(units))
    }

    /// Explicit allowance, never mutated by construction.
    #[must_use]
    pub const fn units(self) -> u32 {
        self.0
    }
}

/// Signed32 quart coordinates for the stored lookup.
///
/// Distinct from block positions and density's checked climate quarts; zoom
/// output converts only through [`StoredBiomeQuartPos::from_zoom`].
///
/// ```compile_fail
/// use ferrumc_world::density::ClimateQuartPos;
/// use ferrumc_world::stored_biome::StoredBiomeQuartPos;
/// fn stored(pos: ClimateQuartPos) -> StoredBiomeQuartPos { pos }
/// ```
/// ```compile_fail
/// use ferrumc_world::biome_zoom::BiomeQuartPos;
/// use ferrumc_world::stored_biome::StoredBiomeQuartPos;
/// fn stored(pos: BiomeQuartPos) -> StoredBiomeQuartPos { pos }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoredBiomeQuartPos {
    x: i32,
    y: i32,
    z: i32,
}

impl StoredBiomeQuartPos {
    /// Admits every signed32 quart axis without a domain check.
    #[must_use]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Preserves the selected zoom quart bits exactly.
    #[must_use]
    pub const fn from_zoom(pos: BiomeQuartPos) -> Self {
        Self {
            x: pos.x(),
            y: pos.y(),
            z: pos.z(),
        }
    }

    /// Quart X.
    #[must_use]
    pub const fn x(self) -> i32 {
        self.x
    }

    /// Quart Y before stored vertical clamping.
    #[must_use]
    pub const fn y(self) -> i32 {
        self.y
    }

    /// Quart Z.
    #[must_use]
    pub const fn z(self) -> i32 {
        self.z
    }
}

/// Independently admitted resource or count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredBiomeLimit {
    /// Supplied array references.
    Chunks,
    /// Values of one borrowed array.
    ValuesPerChunk,
    /// Values over all borrowed arrays.
    BorrowedValues,
    /// Native bytes retained by the published value.
    RetainedBytes,
    /// Simultaneously live native construction scratch bytes.
    ScratchBytes,
    /// Declared construction admission units.
    ConstructionWork,
}

/// Block axis refused by the `FerrumC` block-domain policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredBiomeAxis {
    /// Block X, checked first.
    X,
    /// Block Z, checked after X.
    Z,
}

/// Classified construction or query refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredBiomeErrorKind {
    /// Allowance outside `1..=MAX_STORED_BIOME_BUILD_WORK`.
    InvalidBudget,
    /// Supplied array count outside `1..=9`.
    InvalidChunkCount,
    /// A centre, supplied or selected chunk fails the existing chunk domain.
    CoordinateDomain,
    /// `FerrumC` policy: block X or Z outside ±30,000,000, not original behaviour.
    BlockCoordinateDomain {
        /// Refused axis; X is checked before Z.
        axis: StoredBiomeAxis,
        /// Offending block coordinate.
        coordinate: i32,
    },
    /// A supplied or selected chunk lies outside the centre's 3×3 window.
    OutsideNeighborhood,
    /// Two supplied arrays share a chunk key.
    DuplicateChunk,
    /// Array world seed differs from the expected identity.
    InconsistentSeed,
    /// Array noise settings differ from the expected identity.
    InconsistentSettings,
    /// Array biome source differs from the expected identity.
    InconsistentSource,
    /// Array accessor geometry differs from the expected identity.
    InconsistentGeometry,
    /// Array length differs from its complete geometry.
    IncompleteArray,
    /// Retained source history violates the source's history shape.
    InvalidHistory,
    /// A fixed-source array contains a different resource.
    InvalidFixedValue,
    /// The selected window slot holds no array; no fallback is substituted.
    MissingChunk,
    /// Surface query outside the centre's complete block column.
    OutsideSurfaceColumn,
    /// The allowance is below the declared construction requirement.
    WorkBudgetExceeded,
    /// A native capacity or count ceiling would be exceeded.
    NativeLimit,
    /// Defensive: a stored index escaped its array.
    InvalidStoredIndex,
}

/// Typed refusal; no partial neighborhood or resolver is published.
///
/// Optional typed context is set where applicable; `requested`/`maximum` are
/// zero unless the refusal concerns a count or index. No message is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("stored biome resolution refused: {kind:?}")]
pub struct StoredBiomeError {
    kind: StoredBiomeErrorKind,
    chunk: Option<ChunkPos>,
    quart: Option<StoredBiomeQuartPos>,
    block: Option<BlockPos>,
    input_index: Option<usize>,
    limit: Option<StoredBiomeLimit>,
    requested: usize,
    maximum: usize,
}

impl StoredBiomeError {
    const fn new(kind: StoredBiomeErrorKind) -> Self {
        Self {
            kind,
            chunk: None,
            quart: None,
            block: None,
            input_index: None,
            limit: None,
            requested: 0,
            maximum: 0,
        }
    }

    const fn with_chunk(mut self, chunk: ChunkPos) -> Self {
        self.chunk = Some(chunk);
        self
    }

    const fn with_quart(mut self, quart: StoredBiomeQuartPos) -> Self {
        self.quart = Some(quart);
        self
    }

    const fn with_block(mut self, block: BlockPos) -> Self {
        self.block = Some(block);
        self
    }

    const fn with_input(mut self, index: usize) -> Self {
        self.input_index = Some(index);
        self
    }

    const fn with_limit(
        mut self,
        limit: StoredBiomeLimit,
        requested: usize,
        maximum: usize,
    ) -> Self {
        self.limit = Some(limit);
        self.requested = requested;
        self.maximum = maximum;
        self
    }

    const fn with_counts(mut self, requested: usize, maximum: usize) -> Self {
        self.requested = requested;
        self.maximum = maximum;
        self
    }

    /// Classified refusal, without message matching.
    #[must_use]
    pub const fn kind(&self) -> StoredBiomeErrorKind {
        self.kind
    }

    /// Typed chunk key involved in the refusal.
    #[must_use]
    pub const fn chunk(&self) -> Option<ChunkPos> {
        self.chunk
    }

    /// Requested quart, for lookup refusals.
    #[must_use]
    pub const fn quart(&self) -> Option<StoredBiomeQuartPos> {
        self.quart
    }

    /// Requested block, for block-query refusals.
    #[must_use]
    pub const fn block(&self) -> Option<BlockPos> {
        self.block
    }

    /// Caller-order position of the refused supplied array.
    #[must_use]
    pub const fn input_index(&self) -> Option<usize> {
        self.input_index
    }

    /// Exceeded resource, for budget and capacity refusals.
    #[must_use]
    pub const fn limit(&self) -> Option<StoredBiomeLimit> {
        self.limit
    }

    /// Requested count, length, work or index.
    #[must_use]
    pub const fn requested(&self) -> usize {
        self.requested
    }

    /// Admitted maximum or expected count.
    #[must_use]
    pub const fn maximum(&self) -> usize {
        self.maximum
    }
}

/// One retained multinoise selection, independent of its producer type.
#[derive(Debug, Clone, Copy)]
struct HistoryPoint {
    ordinal: usize,
    biome: BiomeResourceId,
}

impl From<BiomeSelection> for HistoryPoint {
    fn from(selection: BiomeSelection) -> Self {
        Self {
            ordinal: selection.parameter_ordinal(),
            biome: selection.biome(),
        }
    }
}

/// Private read-only column view shared by production and `cfg(test)` controls.
///
/// Production implements it only for the sealed GEN-05.5 array; the lookup
/// therefore obtains geometry and values exclusively from an admitted borrow.
trait StoredColumn {
    fn column_seed(&self) -> WorldSeed;
    fn column_resource(&self) -> NoiseSettingsResourceId;
    fn column_chunk(&self) -> ChunkPos;
    fn column_geometry(&self) -> BiomeQuartGeometry;
    fn column_source(&self) -> BiomeSourceIdentity;
    fn column_values(&self) -> &[BiomeResourceId];
    fn column_initial(&self) -> Option<HistoryPoint>;
    fn column_final(&self) -> Option<HistoryPoint>;
}

impl StoredColumn for ChunkBiomeSamples {
    fn column_seed(&self) -> WorldSeed {
        self.seed()
    }
    fn column_resource(&self) -> NoiseSettingsResourceId {
        self.resource()
    }
    fn column_chunk(&self) -> ChunkPos {
        self.chunk_pos()
    }
    fn column_geometry(&self) -> BiomeQuartGeometry {
        self.geometry()
    }
    fn column_source(&self) -> BiomeSourceIdentity {
        self.source()
    }
    fn column_values(&self) -> &[BiomeResourceId] {
        self.values()
    }
    fn column_initial(&self) -> Option<HistoryPoint> {
        self.initial_selection().map(HistoryPoint::from)
    }
    fn column_final(&self) -> Option<HistoryPoint> {
        self.final_selection().map(HistoryPoint::from)
    }
}

/// Admitted immutable borrowed state; generic only over the column view.
#[derive(Debug)]
struct Core<'a, C> {
    identity: StoredBiomeIdentity,
    center: ChunkPos,
    chunks: [Option<&'a C>; MAX_STORED_BIOME_CHUNKS],
    zoom_seed: BiomeZoomSeed,
    count: u8,
    borrowed_values: u32,
    construction_work: u32,
}

impl<C> Clone for Core<'_, C> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<C> Copy for Core<'_, C> {}

#[cfg(test)]
thread_local! {
    /// Test-only observation of the exact zoom handed to the selector.
    static OBSERVED_ZOOM: std::cell::Cell<Option<BiomeZoom>> = const { std::cell::Cell::new(None) };
}

impl<'a, C: StoredColumn> Core<'a, C> {
    /// Complete admission in the documented order; nothing is published on failure.
    fn admit(
        identity: StoredBiomeIdentity,
        center: ChunkPos,
        chunks: &[&'a C],
        budget: StoredBiomeBuildBudget,
        retained_bytes: usize,
        extra_work: usize,
    ) -> Result<Self, StoredBiomeError> {
        let count = chunks.len();
        if !(1..=MAX_STORED_BIOME_CHUNKS).contains(&count) {
            return Err(
                StoredBiomeError::new(StoredBiomeErrorKind::InvalidChunkCount).with_limit(
                    StoredBiomeLimit::Chunks,
                    count,
                    MAX_STORED_BIOME_CHUNKS,
                ),
            );
        }
        admit_domain(center)?;
        native_limit(
            StoredBiomeLimit::RetainedBytes,
            retained_bytes,
            MAX_STORED_BIOME_RETAINED_BYTES,
        )?;
        native_limit(
            StoredBiomeLimit::ScratchBytes,
            construction_scratch_bytes(),
            MAX_STORED_BIOME_SCRATCH_BYTES,
        )?;
        let mut staged: [Option<&'a C>; MAX_STORED_BIOME_CHUNKS] = [None; MAX_STORED_BIOME_CHUNKS];
        let mut borrowed = 0_usize;
        for (index, &column) in chunks.iter().enumerate() {
            borrowed = admit_column(identity, center, column, &mut staged, borrowed)
                .map_err(|error| error.with_input(index))?;
        }
        // The whole requirement is refused before any value scan or seed hash.
        let required = CANDIDATE_WORK
            .saturating_add(PER_ARRAY_WORK.saturating_mul(count))
            .saturating_add(borrowed);
        let charged = required.saturating_add(extra_work);
        let allowance = budget.units() as usize;
        if charged > allowance {
            return Err(
                StoredBiomeError::new(StoredBiomeErrorKind::WorkBudgetExceeded).with_limit(
                    StoredBiomeLimit::ConstructionWork,
                    charged,
                    allowance,
                ),
            );
        }
        for (index, &column) in chunks.iter().enumerate() {
            scan_values(identity.source, column)
                .map_err(|error| error.with_chunk(column.column_chunk()).with_input(index))?;
        }
        let (Ok(count), Ok(borrowed_values), Ok(construction_work)) = (
            u8::try_from(count),
            u32::try_from(borrowed),
            u32::try_from(required),
        ) else {
            return Err(
                StoredBiomeError::new(StoredBiomeErrorKind::NativeLimit).with_limit(
                    StoredBiomeLimit::BorrowedValues,
                    borrowed,
                    MAX_STORED_BIOME_BORROWED_VALUES,
                ),
            );
        };
        Ok(Self {
            identity,
            center,
            chunks: staged,
            zoom_seed: BiomeZoomSeed::from_world_seed(identity.seed),
            count,
            borrowed_values,
            construction_work,
        })
    }

    /// Selects the signed window slot and reads the clamped stored slot.
    ///
    /// Refusal order: outside the window, selected chunk domain, missing slot,
    /// then the defensive stored-index check. Returns the selected chunk key.
    fn resolve(
        &self,
        quart: StoredBiomeQuartPos,
    ) -> Result<(ChunkPos, BiomeResourceId), StoredBiomeError> {
        let key = ChunkPos::new(quart.x >> 2, quart.z >> 2);
        let slot = window_slot(self.center, key).map_err(|error| error.with_quart(quart))?;
        admit_domain(key).map_err(|error| error.with_quart(quart))?;
        let refuse = |kind| {
            StoredBiomeError::new(kind)
                .with_chunk(key)
                .with_quart(quart)
        };
        let column = self
            .chunks
            .get(slot)
            .copied()
            .flatten()
            .ok_or(refuse(StoredBiomeErrorKind::MissingChunk))?;
        let geometry = column.column_geometry();
        let min_quart_y = i64::from(geometry.min_y()) >> 2;
        let max_quart_y = min_quart_y + i64::from(geometry.height() >> 2) - 1;
        // Only Y is clamped, to the closed accessor interval; max/min cannot panic.
        let quart_y = i64::from(quart.y).max(min_quart_y).min(max_quart_y);
        let index =
            ((quart_y - min_quart_y) * 4 + i64::from(quart.x & 3)) * 4 + i64::from(quart.z & 3);
        let values = column.column_values();
        let invalid = refuse(StoredBiomeErrorKind::InvalidStoredIndex);
        let index =
            usize::try_from(index).map_err(|_| invalid.with_counts(usize::MAX, values.len()))?;
        values
            .get(index)
            .map(|value| (key, *value))
            .ok_or(invalid.with_counts(index, values.len()))
    }

    fn noise_biome(&self, quart: StoredBiomeQuartPos) -> Result<BiomeResourceId, StoredBiomeError> {
        self.resolve(quart).map(|(_, value)| value)
    }

    fn noise_biome_at_block(&self, pos: BlockPos) -> Result<BiomeResourceId, StoredBiomeError> {
        admit_block(pos)?;
        self.noise_biome(block_quart(pos))
            .map_err(|error| error.with_block(pos))
    }

    /// The single zoom selection/lookup composition used by every public path.
    fn zoomed(&self, zoom: BiomeZoom, pos: BlockPos) -> Result<BiomeResourceId, StoredBiomeError> {
        #[cfg(test)]
        OBSERVED_ZOOM.with(|observed| observed.set(Some(zoom)));
        let quart = StoredBiomeQuartPos::from_zoom(zoom.select_quart(pos));
        self.noise_biome(quart)
            .map_err(|error| error.with_block(pos))
    }

    fn biome_at_block(
        &self,
        pos: BlockPos,
        semantics: BiomeZoomSemantics,
    ) -> Result<BiomeResourceId, StoredBiomeError> {
        admit_block(pos)?;
        self.zoomed(BiomeZoom::new(self.zoom_seed, semantics), pos)
    }
}

/// Per-array admission in caller order: window/key domain, duplicate, seed,
/// settings, source, geometry, complete length and history shape. Returns the
/// running borrowed-value total; the caller adds the input ordinal.
fn admit_column<'a, C: StoredColumn>(
    identity: StoredBiomeIdentity,
    center: ChunkPos,
    column: &'a C,
    staged: &mut [Option<&'a C>; MAX_STORED_BIOME_CHUNKS],
    borrowed: usize,
) -> Result<usize, StoredBiomeError> {
    let key = column.column_chunk();
    let refuse = |kind| StoredBiomeError::new(kind).with_chunk(key);
    let slot = window_slot(center, key)?;
    admit_domain(key)?;
    let Some(destination) = staged.get_mut(slot) else {
        return Err(refuse(StoredBiomeErrorKind::OutsideNeighborhood));
    };
    if destination.is_some() {
        return Err(refuse(StoredBiomeErrorKind::DuplicateChunk));
    }
    if column.column_seed() != identity.seed {
        return Err(refuse(StoredBiomeErrorKind::InconsistentSeed));
    }
    if column.column_resource() != identity.resource {
        return Err(refuse(StoredBiomeErrorKind::InconsistentSettings));
    }
    if column.column_source() != identity.source {
        return Err(refuse(StoredBiomeErrorKind::InconsistentSource));
    }
    if column.column_geometry() != identity.geometry {
        return Err(refuse(StoredBiomeErrorKind::InconsistentGeometry));
    }
    let length = column.column_values().len();
    native_limit(
        StoredBiomeLimit::ValuesPerChunk,
        length,
        MAX_STORED_BIOME_VALUES_PER_CHUNK,
    )
    .map_err(|error| error.with_chunk(key))?;
    let expected = identity.geometry.sample_count();
    if length != expected {
        return Err(refuse(StoredBiomeErrorKind::IncompleteArray).with_counts(length, expected));
    }
    let borrowed = borrowed.saturating_add(length);
    native_limit(
        StoredBiomeLimit::BorrowedValues,
        borrowed,
        MAX_STORED_BIOME_BORROWED_VALUES,
    )
    .map_err(|error| error.with_chunk(key))?;
    if !history_shape(identity.source, column) {
        return Err(refuse(StoredBiomeErrorKind::InvalidHistory));
    }
    *destination = Some(column);
    Ok(borrowed)
}

/// Original unzoomed `QuartPos.fromBlock` arithmetic shift on every axis.
const fn block_quart(pos: BlockPos) -> StoredBiomeQuartPos {
    StoredBiomeQuartPos::new(pos.x() >> 2, pos.y() >> 2, pos.z() >> 2)
}

fn admit_domain(chunk: ChunkPos) -> Result<(), StoredBiomeError> {
    admit_chunk(chunk).map_err(|_| {
        StoredBiomeError::new(StoredBiomeErrorKind::CoordinateDomain).with_chunk(chunk)
    })
}

/// Fixed slot `(dx + 1) * 3 + (dz + 1)` from window deltas computed in i64.
fn window_slot(center: ChunkPos, key: ChunkPos) -> Result<usize, StoredBiomeError> {
    let dx = i64::from(key.x()) - i64::from(center.x());
    let dz = i64::from(key.z()) - i64::from(center.z());
    let outside = StoredBiomeError::new(StoredBiomeErrorKind::OutsideNeighborhood).with_chunk(key);
    if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) {
        return Err(outside);
    }
    usize::try_from((dx + 1) * 3 + (dz + 1)).map_err(|_| outside)
}

/// Window key of one fixed slot, or a domain refusal when it leaves i32.
fn slot_key(center: ChunkPos, slot: usize) -> Result<ChunkPos, StoredBiomeError> {
    let offset = |value: usize| i64::try_from(value).map_or(i64::MAX, |value| value - 1);
    let x = i64::from(center.x()) + offset(slot / 3);
    let z = i64::from(center.z()) + offset(slot % 3);
    match (i32::try_from(x), i32::try_from(z)) {
        (Ok(x), Ok(z)) => Ok(ChunkPos::new(x, z)),
        _ => Err(StoredBiomeError::new(StoredBiomeErrorKind::CoordinateDomain).with_chunk(center)),
    }
}

/// `FerrumC` block-domain policy: X first, then Z, both inclusive ±30,000,000.
fn admit_block(pos: BlockPos) -> Result<(), StoredBiomeError> {
    let bound = -MAX_STORED_BIOME_BLOCK_COORDINATE..=MAX_STORED_BIOME_BLOCK_COORDINATE;
    for (axis, coordinate) in [(StoredBiomeAxis::X, pos.x()), (StoredBiomeAxis::Z, pos.z())] {
        if !bound.contains(&coordinate) {
            return Err(
                StoredBiomeError::new(StoredBiomeErrorKind::BlockCoordinateDomain {
                    axis,
                    coordinate,
                })
                .with_block(pos),
            );
        }
    }
    Ok(())
}

fn native_limit(
    limit: StoredBiomeLimit,
    requested: usize,
    maximum: usize,
) -> Result<(), StoredBiomeError> {
    if requested > maximum {
        Err(StoredBiomeError::new(StoredBiomeErrorKind::NativeLimit)
            .with_limit(limit, requested, maximum))
    } else {
        Ok(())
    }
}

/// O(1) history shape; the final endpoint is checked by the value scan.
fn history_shape<C: StoredColumn>(source: BiomeSourceIdentity, column: &C) -> bool {
    let (initial, last) = (column.column_initial(), column.column_final());
    match source {
        BiomeSourceIdentity::Fixed(_) | BiomeSourceIdentity::End => {
            initial.is_none() && last.is_none()
        }
        BiomeSourceIdentity::MultiNoise(preset) => {
            let parameters = preset.parameters();
            let valid = |point: HistoryPoint| {
                parameters
                    .get(point.ordinal)
                    .is_some_and(|entry| entry.biome() == point.biome)
            };
            last.is_some_and(valid) && initial.is_none_or(valid)
        }
    }
}

fn scan_values<C: StoredColumn>(
    source: BiomeSourceIdentity,
    column: &C,
) -> Result<(), StoredBiomeError> {
    let values = column.column_values();
    match source {
        BiomeSourceIdentity::Fixed(biome) => {
            match values.iter().position(|value| *value != biome) {
                Some(index) => Err(
                    StoredBiomeError::new(StoredBiomeErrorKind::InvalidFixedValue)
                        .with_counts(index, values.len()),
                ),
                None => Ok(()),
            }
        }
        BiomeSourceIdentity::MultiNoise(_) => {
            // Last execution slot (top quart Y, local X 3, Z 3) is also the last storage slot.
            match (values.last(), column.column_final()) {
                (Some(value), Some(last)) if *value == last.biome => Ok(()),
                _ => Err(StoredBiomeError::new(StoredBiomeErrorKind::InvalidHistory)),
            }
        }
        BiomeSourceIdentity::End => Ok(()),
    }
}

/// Bounded borrowed neighborhood of complete sealed biome arrays.
///
/// Retains at most nine immutable references and no slice, collection, `Arc`,
/// source session, callback or cache. Copying is a bounded value copy; every
/// query is constant-bounded, allocation-free and stateless.
///
/// ```compile_fail
/// use ferrumc_math::ChunkPos;
/// use ferrumc_world::biome_selection::ChunkBiomeSamples;
/// use ferrumc_world::stored_biome::{
///     StoredBiomeBuildBudget, StoredBiomeIdentity, StoredBiomeNeighborhood,
/// };
/// // The neighborhood cannot outlive the arrays it borrows.
/// fn detached(
///     samples: ChunkBiomeSamples,
///     identity: StoredBiomeIdentity,
///     budget: StoredBiomeBuildBudget,
/// ) -> StoredBiomeNeighborhood<'static> {
///     StoredBiomeNeighborhood::new(identity, ChunkPos::ORIGIN, &[&samples], budget).unwrap()
/// }
/// ```
/// ```compile_fail
/// use ferrumc_world::stored_biome::{StoredBiomeNeighborhood, StoredBiomeQuartPos};
/// // Results are checked resource identities, never wire IDs.
/// fn wire(neighborhood: &StoredBiomeNeighborhood<'_>) -> i32 {
///     neighborhood.noise_biome_at_quart(StoredBiomeQuartPos::new(0, 0, 0)).unwrap()
/// }
/// ```
/// ```compile_fail
/// use ferrumc_math::BlockPos;
/// use ferrumc_world::stored_biome::StoredBiomeNeighborhood;
/// // A block position is not a quart position.
/// fn quart(neighborhood: &StoredBiomeNeighborhood<'_>) {
///     let _ = neighborhood.noise_biome_at_quart(BlockPos::new(0, 0, 0));
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct StoredBiomeNeighborhood<'a> {
    core: Core<'a, ChunkBiomeSamples>,
}

impl<'a> StoredBiomeNeighborhood<'a> {
    /// Admits 1..=9 complete consistent arrays inside the centre's 3×3 window.
    ///
    /// Refusal order: chunk count, centre domain, native ceilings, then each
    /// array in caller order (window/key domain, duplicate, seed, settings,
    /// source, geometry, length, history shape), then the whole work
    /// requirement, then the value scan. The zoom seed is derived once.
    pub fn new(
        identity: StoredBiomeIdentity,
        center: ChunkPos,
        chunks: &[&'a ChunkBiomeSamples],
        budget: StoredBiomeBuildBudget,
    ) -> Result<Self, StoredBiomeError> {
        Core::admit(identity, center, chunks, budget, size_of::<Self>(), 0)
            .map(|core| Self { core })
    }

    /// Expected identity every borrowed array matched.
    #[must_use]
    pub const fn identity(&self) -> StoredBiomeIdentity {
        self.core.identity
    }

    /// Centre chunk of the 3×3 window.
    #[must_use]
    pub const fn center(&self) -> ChunkPos {
        self.core.center
    }

    /// Number of borrowed arrays.
    #[must_use]
    pub const fn chunk_count(&self) -> usize {
        self.core.count as usize
    }

    /// Borrowed values over all arrays; owned by the arrays, not retained here.
    #[must_use]
    pub const fn borrowed_value_count(&self) -> usize {
        self.core.borrowed_values as usize
    }

    /// Declared admission units `2 + 10·N + values` of this neighborhood.
    #[must_use]
    pub const fn construction_work(&self) -> usize {
        self.core.construction_work as usize
    }

    /// Native bytes of this value; it owns no heap allocation.
    #[must_use]
    pub const fn native_retained_bytes(&self) -> usize {
        size_of::<Self>()
    }

    /// Stored value of the selected signed chunk slot, with vertical clamping.
    pub fn noise_biome_at_quart(
        &self,
        quart: StoredBiomeQuartPos,
    ) -> Result<BiomeResourceId, StoredBiomeError> {
        self.core.noise_biome(quart)
    }

    /// Original unzoomed `>> 2` block conversion after the block-domain policy.
    pub fn noise_biome_at_block(&self, pos: BlockPos) -> Result<BiomeResourceId, StoredBiomeError> {
        self.core.noise_biome_at_block(pos)
    }

    /// Existing zoom selection with the derived seed, then the stored lookup.
    pub fn biome_at_block(
        &self,
        pos: BlockPos,
        semantics: BiomeZoomSemantics,
    ) -> Result<BiomeResourceId, StoredBiomeError> {
        self.core.biome_at_block(pos, semantics)
    }
}

/// Complete 3×3 surface-column resolver with explicit zoom semantics.
///
/// Requires all nine distinct arrays; queries admit only the centre's 16×16
/// block column and every signed32 Y.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceBiomeResolver<'a> {
    neighborhood: StoredBiomeNeighborhood<'a>,
    semantics: BiomeZoomSemantics,
}

impl<'a> SurfaceBiomeResolver<'a> {
    /// Admits as a neighborhood with nine surface coverage units, then checks
    /// all nine halo keys in slot order: every chunk domain, then every presence.
    pub fn new(
        identity: StoredBiomeIdentity,
        center: ChunkPos,
        chunks: &[&'a ChunkBiomeSamples],
        semantics: BiomeZoomSemantics,
        budget: StoredBiomeBuildBudget,
    ) -> Result<Self, StoredBiomeError> {
        let core = Core::admit(
            identity,
            center,
            chunks,
            budget,
            size_of::<Self>(),
            SURFACE_COVERAGE_WORK,
        )?;
        complete_halo(&core)?;
        Ok(Self {
            neighborhood: StoredBiomeNeighborhood { core },
            semantics,
        })
    }

    /// Read-only borrowed neighborhood, including quart lookup.
    #[must_use]
    pub const fn neighborhood(&self) -> &StoredBiomeNeighborhood<'a> {
        &self.neighborhood
    }

    /// Expected identity every borrowed array matched.
    #[must_use]
    pub const fn identity(&self) -> StoredBiomeIdentity {
        self.neighborhood.core.identity
    }

    /// Centre chunk whose block column is resolved.
    #[must_use]
    pub const fn center(&self) -> ChunkPos {
        self.neighborhood.core.center
    }

    /// Explicit zoom numerical semantics.
    #[must_use]
    pub const fn semantics(&self) -> BiomeZoomSemantics {
        self.semantics
    }

    /// Declared admission units `2 + 10·9 + values + 9`.
    #[must_use]
    pub const fn construction_work(&self) -> usize {
        self.neighborhood.core.construction_work as usize + SURFACE_COVERAGE_WORK
    }

    /// Zoomed biome of a centre-column block; X/Z policy and column first.
    pub fn biome_at_block(&self, pos: BlockPos) -> Result<BiomeResourceId, StoredBiomeError> {
        let core = &self.neighborhood.core;
        admit_block(pos)?;
        let column = pos.to_chunk_pos();
        if column.x() != core.center.x() || column.z() != core.center.z() {
            return Err(
                StoredBiomeError::new(StoredBiomeErrorKind::OutsideSurfaceColumn)
                    .with_chunk(core.center)
                    .with_block(pos),
            );
        }
        core.zoomed(BiomeZoom::new(core.zoom_seed, self.semantics), pos)
    }
}

/// All nine keys in slot order: every chunk domain first, then every presence.
fn complete_halo<C: StoredColumn>(core: &Core<'_, C>) -> Result<(), StoredBiomeError> {
    for slot in 0..MAX_STORED_BIOME_CHUNKS {
        admit_domain(slot_key(core.center, slot)?)?;
    }
    for (slot, column) in core.chunks.iter().enumerate() {
        if column.is_none() {
            let key = slot_key(core.center, slot)?;
            return Err(StoredBiomeError::new(StoredBiomeErrorKind::MissingChunk).with_chunk(key));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
