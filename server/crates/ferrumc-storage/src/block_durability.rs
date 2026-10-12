//! Format7 joint Player/full-overlay Chunk durability, with backend-issued authority.

use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use ferrumc_core::PlayerId;
use ferrumc_math::BlockPos;
use ferrumc_world::{BlockStateId, Chunk, GenerationIdentity};

use crate::{
    ChunkKey, ChunkOverlayRecord, ChunkRecord, GenerationKey, MutationActor, OwnershipCapacity,
    OwnershipCommitDisposition, OwnershipConfig, OwnershipError, OwnershipStamp,
    OwnershipTransferId, PlayerRecord, SchemaVersion, StorageError,
};

pub(crate) mod engine;
pub(crate) mod raw;

/// Maximum retained canonical request body (32 MiB).
pub const MAX_BLOCK_REQUEST_BYTES: usize = 32 * 1024 * 1024;
/// Maximum coherent union size.
pub const MAX_BLOCK_CHUNKS: usize = 8;
/// Maximum distinct cell read dependencies.
pub const MAX_BLOCK_GUARDS: usize = 4096;
/// Maximum ordered cell effects.
pub const MAX_BLOCK_EFFECTS: usize = 1024;
/// Typed results preserve conflicts and uncertainty.
pub type BlockResult<T> = Result<T, BlockDurabilityError>;

/// Refusals never imply rollback after a commit attempt or started-worker failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BlockDurabilityError {
    /// The explicit mode was not selected.
    #[error("block durability unsupported")]
    Unsupported,
    /// Upgrade requires a separate offline ticket.
    #[error("explicit migration from format {actual} to7 required")]
    MigrationRequired {
        /// Stored marker.
        actual: u64,
    },
    /// Invalid positive agreement or arithmetic overflow.
    #[error("invalid block configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// Missing or extra table, type or metadata.
    #[error("invalid Format7 catalog: {0}")]
    CatalogMismatch(&'static str),
    /// Immutable durable agreement differs.
    #[error("Format7 capacity agreement differs")]
    CapacityContractMismatch,
    /// Malformed grammar or encountered relationship.
    #[error("malformed {table}: {reason}")]
    MalformedData {
        /// Grammar/table.
        table: &'static str,
        /// Refusal.
        reason: &'static str,
    },
    /// Existing bounded record refused.
    #[error("block record: {0}")]
    Record(#[source] StorageError),
    /// Genuine shared Player/Entity authority refused.
    #[error("shared ownership: {0}")]
    Ownership(#[source] OwnershipError),
    /// A declared key/effect belongs to a different scope.
    #[error("block scope mismatch")]
    ScopeMismatch,
    /// Actual immutable generation differs.
    #[error("block generation mismatch")]
    GenerationMismatch,
    /// Foreign/reopened authority cannot issue new writes.
    #[error("block snapshot belongs to another backend")]
    BackendOwnerMismatch,
    /// Joint gameplay requires an existing live player.
    #[error("joint block player is not live")]
    PlayerNotLive,
    /// A sorted resource's history changed.
    #[error("block CAS conflict for {resource}: expected {expected:?}, actual {actual:?}")]
    CasConflict {
        /// Player or Chunk.
        resource: &'static str,
        /// Optional chunk.
        key: Option<ChunkKey>,
        /// Issued stamp.
        expected: OwnershipStamp,
        /// Current stamp.
        actual: OwnershipStamp,
    },
    /// Matching stamp with different exact stored bytes.
    #[error("exact before image mismatch for {resource}")]
    BeforeImageMismatch {
        /// Domain.
        resource: &'static str,
        /// Optional chunk.
        key: Option<ChunkKey>,
    },
    /// Fresh/tombstone erase cannot invent history.
    #[error("chunk is already absent: {key:?}")]
    AlreadyAbsent {
        /// Erased column.
        key: ChunkKey,
    },
    /// Operation IDs share FCTR/FCBR namespace.
    #[error("operation ID collision: {id:?}")]
    ReceiptCollision {
        /// Retained operation.
        id: OwnershipTransferId,
    },
    /// A read-only lookup asked for the other receipt family.
    #[error("receipt kind mismatch: {id:?}")]
    ReceiptKindMismatch {
        /// Retained operation.
        id: OwnershipTransferId,
    },
    /// A hard shape bound refused before work.
    #[error("{resource} batch size {len} exceeds {max}")]
    BatchCapacity {
        /// Shape.
        resource: &'static str,
        /// Actual count.
        len: usize,
        /// Bound.
        max: usize,
    },
    /// Request or snapshot encoded admission refused.
    #[error("{resource} bytes {len} exceed {max}")]
    ByteCapacity {
        /// Admission domain.
        resource: &'static str,
        /// Actual bytes.
        len: u64,
        /// Bound.
        max: u64,
    },
    /// Invalid chain, unchanged effect, duplicate or misplaced cell.
    #[error("invalid effect/dependency at ordinal {ordinal}: {reason}")]
    InvalidEffect {
        /// Semantic ordinal.
        ordinal: usize,
        /// Refusal.
        reason: &'static str,
    },
    /// Effects did not prove the complete endpoint.
    #[error("incomplete block delta: {key:?}")]
    IncompleteDelta {
        /// Column.
        key: ChunkKey,
    },
    /// Permanent logical capacity exhausted.
    #[error("{resource} history capacity exhausted ({max})")]
    HistoryCapacity {
        /// Domain.
        resource: &'static str,
        /// Persisted bound.
        max: u64,
    },
    /// Actual change cannot increment MAX.
    #[error("{resource} revision exhausted")]
    RevisionExhausted {
        /// Player/Chunk.
        resource: &'static str,
        /// Column if applicable.
        key: Option<ChunkKey>,
    },
    /// Scoped count/ID arithmetic cannot wrap.
    #[error("scoped journal sequence exhausted")]
    SequenceExhausted,
    /// The next actual row already exists.
    #[error("scoped journal ID collision: {id}")]
    JournalIdCollision {
        /// Actual next ID.
        id: u64,
    },
    /// Cumulative inspected bytes or cell work exhausted.
    #[error("validation {0} budget exceeded")]
    ValidationCapacity(&'static str),
    /// Complete startup scan exhausted.
    #[error("startup {0} budget exceeded")]
    ScanCapacity(&'static str),
    /// Actual disposable index extent refused.
    #[error("scratch extent exceeded")]
    ScratchCapacity,
    /// No shared permit; worker never started.
    #[error("shared storage workers busy")]
    Busy,
    /// Precommit operational failure.
    #[error("block backend: {0}")]
    Backend(String),
    /// Actual commit failed; resolve by original query and exclusive reopen.
    #[error("block commit uncertain for {query:?}: {cause}")]
    CommitUncertain {
        /// Original query.
        query: BlockReceiptQuery,
        /// Actual cause.
        cause: String,
    },
    /// Started worker/postcommit reply failed.
    #[error("block execution uncertain for {query:?}: {cause}")]
    ExecutionUncertain {
        /// Original query.
        query: BlockReceiptQuery,
        /// Actual cause.
        cause: String,
    },
    /// Exact file identity currently requires Unix.
    #[error("Format7 requires Unix file identity")]
    UnsupportedPlatform,
    /// Owned scratch could not be removed; foreign replacements survive.
    #[error("scratch cleanup: {0}")]
    ScratchCleanup(String),
    /// The explicit scratch directory cannot host the owned index; checked before any
    /// database path is created, so nothing was written.
    #[error("Format7 scratch directory unusable: {0}")]
    ScratchUnavailable(String),
}
// Typed adapters carry Format7 refusals through shared v1/UUID readers; unwrap
// them structurally so callers see e.g. `ValidationCapacity`, never a string.
impl From<StorageError> for BlockDurabilityError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::BlockValidation(inner) => *inner,
            other => Self::Record(other),
        }
    }
}
impl From<OwnershipError> for BlockDurabilityError {
    fn from(e: OwnershipError) -> Self {
        match e {
            OwnershipError::Block(inner)
            | OwnershipError::Record(StorageError::BlockValidation(inner)) => *inner,
            other => Self::Ownership(other),
        }
    }
}
impl From<BlockDurabilityError> for ferrumc_core::ServerError {
    fn from(e: BlockDurabilityError) -> Self {
        match e {
            BlockDurabilityError::Unsupported => Self::unsupported("Player/block durability"),
            BlockDurabilityError::Record(e) => e.into(),
            BlockDurabilityError::Ownership(e) => e.into(),
            other => Self::internal(other.to_string()),
        }
    }
}

/// Immutable positive Format7 retained logical-byte/row agreement, without deployment defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDurabilityCapacity {
    pub(crate) ownership: OwnershipCapacity,
    pub(crate) request: usize,
    pub(crate) chunks: u64,
    pub(crate) receipt_bytes: u64,
    pub(crate) journal_rows: u64,
    pub(crate) journal_bytes: u64,
}
impl BlockDurabilityCapacity {
    /// Combines the shared v1 agreement with independent positive block bounds.
    pub fn new(
        ownership: OwnershipCapacity,
        request: NonZeroUsize,
        chunks: NonZeroU64,
        receipt_bytes: NonZeroU64,
        journal_rows: NonZeroU64,
        journal_bytes: NonZeroU64,
    ) -> BlockResult<Self> {
        if !(144..=MAX_BLOCK_REQUEST_BYTES).contains(&request.get()) {
            return Err(BlockDurabilityError::InvalidConfiguration(
                "request outside144..=32MiB",
            ));
        }
        receipt_bytes
            .get()
            .checked_add(ownership.max_revision_rows().checked_mul(55).ok_or(
                BlockDurabilityError::InvalidConfiguration("revision bytes overflow"),
            )?)
            .and_then(|n| chunks.get().checked_mul(47).and_then(|c| n.checked_add(c)))
            .and_then(|n| n.checked_add(journal_bytes.get()))
            .ok_or(BlockDurabilityError::InvalidConfiguration(
                "retained logical bytes overflow",
            ))?;
        Ok(Self {
            ownership,
            request: request.get(),
            chunks: chunks.get(),
            receipt_bytes: receipt_bytes.get(),
            journal_rows: journal_rows.get(),
            journal_bytes: journal_bytes.get(),
        })
    }
    /// Returns the exact shared Player/Entity capacity.
    pub const fn ownership(self) -> OwnershipCapacity {
        self.ownership
    }
    /// Returns the permanent canonical body limit.
    pub const fn max_request_bytes(self) -> usize {
        self.request
    }
    /// Returns the permanent Chunk header row limit.
    pub const fn max_chunk_revision_rows(self) -> u64 {
        self.chunks
    }
    /// Returns the common FCTR/FCBR key/value byte limit.
    pub const fn max_receipt_bytes(self) -> u64 {
        self.receipt_bytes
    }
    /// Returns the permanent scoped journal row limit.
    pub const fn max_journal_rows(self) -> u64 {
        self.journal_rows
    }
    /// Returns the permanent scoped journal key/value byte limit.
    pub const fn max_journal_bytes(self) -> u64 {
        self.journal_bytes
    }
}
/// Positive local cumulative validation/output limits; encoded limits are not RSS quotas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDurabilityLimits {
    pub(crate) bytes: u64,
    pub(crate) cells: u64,
    pub(crate) snapshot: u64,
    pub(crate) relation_bytes: u64,
    pub(crate) relation_cells: u64,
}
impl BlockDurabilityLimits {
    /// Caps each runtime call at512MiB inspected bytes/16,777,216 cells and32MiB output.
    /// Startup relations have independent positive cumulative bounds.
    pub fn new(
        bytes: NonZeroU64,
        cells: NonZeroU64,
        snapshot: NonZeroU64,
        relation_bytes: NonZeroU64,
        relation_cells: NonZeroU64,
    ) -> BlockResult<Self> {
        if bytes.get() > 512 * 1024 * 1024
            || cells.get() > 16_777_216
            || snapshot.get() > MAX_BLOCK_REQUEST_BYTES as u64
        {
            return Err(BlockDurabilityError::InvalidConfiguration(
                "runtime hard limit exceeded",
            ));
        }
        Ok(Self {
            bytes: bytes.get(),
            cells: cells.get(),
            snapshot: snapshot.get(),
            relation_bytes: relation_bytes.get(),
            relation_cells: relation_cells.get(),
        })
    }
    /// Returns cumulative runtime byte admission.
    pub const fn max_validation_bytes(self) -> u64 {
        self.bytes
    }
    /// Returns cumulative runtime cell-work admission.
    pub const fn max_validation_cells(self) -> u64 {
        self.cells
    }
    /// Returns coherent snapshot encoded-output admission.
    pub const fn max_snapshot_bytes(self) -> u64 {
        self.snapshot
    }
    /// Returns complete startup relation byte admission.
    pub const fn max_relation_bytes(self) -> u64 {
        self.relation_bytes
    }
    /// Returns complete startup relation cell-work admission.
    pub const fn max_relation_cells(self) -> u64 {
        self.relation_cells
    }
}
/// Explicit Format7 agreement plus shared startup/worker and local validation limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDurabilityConfig {
    pub(crate) capacity: BlockDurabilityCapacity,
    pub(crate) ownership: OwnershipConfig,
    pub(crate) limits: BlockDurabilityLimits,
}
impl BlockDurabilityConfig {
    /// Requires the same immutable v1 agreement for both API families.
    pub fn new(
        capacity: BlockDurabilityCapacity,
        ownership: OwnershipConfig,
        limits: BlockDurabilityLimits,
    ) -> BlockResult<Self> {
        if capacity.ownership != ownership.capacity() {
            return Err(BlockDurabilityError::CapacityContractMismatch);
        }
        Ok(Self {
            capacity,
            ownership,
            limits,
        })
    }
    /// Returns the permanent agreement.
    pub const fn capacity(self) -> BlockDurabilityCapacity {
        self.capacity
    }
    /// Returns shared scan/scratch/worker settings.
    pub const fn ownership_config(self) -> OwnershipConfig {
        self.ownership
    }
    /// Returns local cumulative allowances.
    pub const fn limits(self) -> BlockDurabilityLimits {
        self.limits
    }
}

/// A declaration, never authority: storage checks the actual descriptor and originals coherently.
#[derive(Debug, Clone)]
pub struct BlockSnapshotSpec {
    pub(crate) scope: GenerationKey,
    pub(crate) identity: GenerationIdentity,
    pub(crate) player: Option<PlayerId>,
    pub(crate) keys: Vec<ChunkKey>,
}
impl BlockSnapshotSpec {
    /// Admits1..8 distinct bounded columns in one explicit scope before I/O.
    pub fn new(
        scope: GenerationKey,
        identity: GenerationIdentity,
        player: Option<PlayerId>,
        mut keys: Vec<ChunkKey>,
    ) -> BlockResult<Self> {
        raw::shape("chunks", keys.len(), 1, MAX_BLOCK_CHUNKS)?;
        keys.sort();
        for (i, k) in keys.iter().enumerate() {
            if GenerationKey::new(k.world(), k.dimension()) != scope {
                return Err(BlockDurabilityError::ScopeMismatch);
            }
            crate::record::validate_chunk_position(k.pos())?;
            if i > 0 && keys[i - 1] == *k {
                return Err(raw::bad("snapshot", "duplicate chunk"));
            }
        }
        Ok(Self {
            scope,
            identity,
            player,
            keys,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Pair {
    pub(crate) full: Option<Arc<[u8]>>,
    pub(crate) overlay: Option<Arc<[u8]>>,
}
impl Pair {
    pub(crate) fn len(&self) -> usize {
        self.full.as_ref().map_or(0, |b| b.len()) + self.overlay.as_ref().map_or(0, |b| b.len())
    }
}
/// Checked complete full/overlay proposal; schemas are preserved verbatim.
#[derive(Debug, Clone)]
pub struct ChunkImage(pub(crate) Pair);
impl ChunkImage {
    /// Losslessly captures current records or refuses; at least one record is required.
    pub fn new(
        full: Option<ChunkRecord>,
        overlay: Option<ChunkOverlayRecord>,
    ) -> BlockResult<Self> {
        if full.is_none() && overlay.is_none() {
            return Err(raw::bad("image", "empty Replace"));
        }
        let mut budget = engine::Budget::hard();
        let full = full
            .map(|r| {
                budget.cells(98_304)?;
                let bytes = crate::codec::encode_chunk_record(&r)?;
                Ok::<_, BlockDurabilityError>(Arc::from(bytes))
            })
            .transpose()?;
        let overlay = overlay
            .map(|r| {
                budget.cells(98_304)?;
                Ok::<_, BlockDurabilityError>(Arc::from(crate::codec::encode_chunk_overlay_record(
                    &r,
                )?))
            })
            .transpose()?;
        Ok(Self(Pair { full, overlay }))
    }
    /// Borrows exact checked full-envelope bytes.
    pub fn full_bytes(&self) -> Option<&[u8]> {
        self.0.full.as_deref()
    }
    /// Borrows exact checked overlay-envelope bytes.
    pub fn overlay_bytes(&self) -> Option<&[u8]> {
        self.0.overlay.as_deref()
    }
}
/// Explicit action for exactly one snapshot column.
#[derive(Debug, Clone)]
pub enum ChunkImageAction {
    /// Retain exact original bytes, including readable legacy spellings.
    Keep,
    /// Replace with a checked nonempty complete pair.
    Replace(ChunkImage),
    /// Delete both full and overlay while retaining positive history.
    Erase,
}
/// A key and action; purported originals cannot be supplied here.
#[derive(Debug, Clone)]
pub struct ChunkImageChange {
    pub(crate) key: ChunkKey,
    pub(crate) action: ChunkImageAction,
}
impl ChunkImageChange {
    /// Proposes one action against a genuine snapshot's column.
    pub const fn new(key: ChunkKey, action: ChunkImageAction) -> Self {
        Self { key, action }
    }
}
/// New scoped cause domain, independent of legacy `PlayerCreative`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopedBlockCause {
    /// Actual player action, policy owned by gameplay.
    PlayerAction,
    /// Command mutation.
    Command,
    /// Plugin mutation.
    Plugin,
    /// Deterministic fixture.
    Test,
}
/// Complete ordered cell effect; scope is explicitly declared, never inferred.
#[derive(Debug, Clone)]
pub struct ScopedBlockEffect {
    pub(crate) scope: GenerationKey,
    pub(crate) schema: SchemaVersion,
    pub(crate) tick: u64,
    pub(crate) actor: MutationActor,
    pub(crate) cause: ScopedBlockCause,
    pub(crate) pos: BlockPos,
    pub(crate) old: BlockStateId,
    pub(crate) new: BlockStateId,
    pub(crate) old_entity: Option<Arc<[u8]>>,
    pub(crate) new_entity: Option<Arc<[u8]>>,
}
impl ScopedBlockEffect {
    /// Accepts complete canonical current entity envelopes; execution proves the chain.
    // One argument per retained Effect grammar field, in grammar order; a builder would allow
    // a partially specified effect to exist.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scope: GenerationKey,
        schema: SchemaVersion,
        tick: u64,
        actor: MutationActor,
        cause: ScopedBlockCause,
        pos: BlockPos,
        old: BlockStateId,
        new: BlockStateId,
        old_entity: Option<Vec<u8>>,
        new_entity: Option<Vec<u8>>,
    ) -> BlockResult<Self> {
        for b in [&old_entity, &new_entity].into_iter().flatten() {
            raw::canonical_entity(b)?;
        }
        Ok(Self {
            scope,
            schema,
            tick,
            actor,
            cause,
            pos,
            old,
            new,
            old_entity: old_entity.map(Arc::from),
            new_entity: new_entity.map(Arc::from),
        })
    }
    /// Returns the explicit scope.
    pub const fn scope(&self) -> GenerationKey {
        self.scope
    }
    /// Returns the effect position.
    pub const fn position(&self) -> BlockPos {
        self.pos
    }
}
/// Immutable coherent column view; cloning a view creates no write expectation.
#[derive(Debug, Clone)]
pub struct OwnedBlockColumn {
    pub(crate) key: ChunkKey,
    pub(crate) stamp: OwnershipStamp,
    pub(crate) pair: Pair,
    pub(crate) view: Chunk,
}
impl OwnedBlockColumn {
    /// Returns the captured column key.
    pub const fn key(&self) -> ChunkKey {
        self.key
    }
    /// Returns historical state, without independently writable authority.
    pub const fn stamp(&self) -> OwnershipStamp {
        self.stamp
    }
    /// Borrows the complete resolved cell/entity view.
    pub const fn chunk(&self) -> &Chunk {
        &self.view
    }
    /// Borrows exact stored full bytes.
    pub fn full_bytes(&self) -> Option<&[u8]> {
        self.pair.full.as_deref()
    }
    /// Borrows exact stored overlay bytes.
    pub fn overlay_bytes(&self) -> Option<&[u8]> {
        self.pair.overlay.as_deref()
    }
}
/// Private backend-issued originals, captured under one actual read boundary.
#[derive(Debug, Clone)]
pub struct OwnedBlockSnapshot {
    pub(crate) spec: BlockSnapshotSpec,
    pub(crate) owner: Weak<()>,
    pub(crate) player_stamp: OwnershipStamp,
    pub(crate) player_raw: Option<Arc<[u8]>>,
    pub(crate) player_record: Option<PlayerRecord>,
    pub(crate) columns: Vec<OwnedBlockColumn>,
    // Issued alongside originals so persisted admission precedes Q allocation/hash.
    pub(crate) request_limit: usize,
}
impl OwnedBlockSnapshot {
    /// Borrows complete immutable column views from the same boundary.
    pub fn columns(&self) -> &[OwnedBlockColumn] {
        &self.columns
    }
    /// Borrows the complete opaque Player record from the same boundary.
    pub fn player_record(&self) -> Option<&PlayerRecord> {
        self.player_record.as_ref()
    }
    /// Returns the captured Player stamp.
    pub const fn player_stamp(&self) -> OwnershipStamp {
        self.player_stamp
    }
    /// Returns the verified scope.
    pub const fn scope(&self) -> GenerationKey {
        self.spec.scope
    }
    /// Returns the verified actual identity.
    pub const fn generation_identity(&self) -> GenerationIdentity {
        self.spec.identity
    }
}
#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub(crate) snapshot: OwnedBlockSnapshot,
    pub(crate) q: Arc<[u8]>,
    pub(crate) query: BlockReceiptQuery,
}
macro_rules! request_api {
    ($name:ident,$kind:expr,$player:ident) => {
        impl $name {
            /// Builds a bounded canonical operation from genuine issued originals.
            pub fn new(
                id: OwnershipTransferId,
                snapshot: OwnedBlockSnapshot,
                $player: PlayerRecord,
                images: Vec<ChunkImageChange>,
                guards: Vec<BlockPos>,
                effects: Vec<ScopedBlockEffect>,
            ) -> BlockResult<Self> {
                raw::request(id, $kind, snapshot, Some(&$player), images, guards, effects).map(Self)
            }
            /// Returns reconstructible read-only retry identity; no write authority.
            pub const fn receipt_query(&self) -> BlockReceiptQuery {
                self.0.query
            }
        }
    };
}
/// Exactly one live Player plus1..8 complete columns.
#[derive(Debug, Clone)]
pub struct PlayerBlockRequest(pub(crate) Request);
request_api!(PlayerBlockRequest, 1, player);
/// Scoped gameplay without Player; it still proves complete endpoints.
#[derive(Debug, Clone)]
pub struct BlockMutationRequest(pub(crate) Request);
impl BlockMutationRequest {
    /// Builds a complete scoped block operation; snapshots with Player refuse.
    pub fn new(
        id: OwnershipTransferId,
        snapshot: OwnedBlockSnapshot,
        images: Vec<ChunkImageChange>,
        guards: Vec<BlockPos>,
        effects: Vec<ScopedBlockEffect>,
    ) -> BlockResult<Self> {
        raw::request(id, 2, snapshot, None, images, guards, effects).map(Self)
    }
    /// Returns original read-only query.
    pub const fn receipt_query(&self) -> BlockReceiptQuery {
        self.0.query
    }
}
/// Arbitrary complete storage image CAS; never certifies a gameplay delta.
#[derive(Debug, Clone)]
pub struct ChunkImageCasRequest(pub(crate) Request);
impl ChunkImageCasRequest {
    /// Uses the same history with no Player, guards or journal effects.
    pub fn new(
        id: OwnershipTransferId,
        snapshot: OwnedBlockSnapshot,
        images: Vec<ChunkImageChange>,
    ) -> BlockResult<Self> {
        raw::request(id, 3, snapshot, None, images, vec![], vec![]).map(Self)
    }
    /// Returns original read-only query.
    pub const fn receipt_query(&self) -> BlockReceiptQuery {
        self.0.query
    }
}
/// Reconstructible ID/SHA256 query; caller digests issue no expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockReceiptQuery {
    pub(crate) id: OwnershipTransferId,
    pub(crate) digest: [u8; 32],
}
impl BlockReceiptQuery {
    /// Retains every supplied bit for read-only reconciliation.
    pub const fn from_bytes(id: OwnershipTransferId, digest: [u8; 32]) -> Self {
        Self { id, digest }
    }
    /// Returns the retained operation ID.
    pub const fn id(self) -> OwnershipTransferId {
        self.id
    }
    /// Returns canonical domain-separated digest.
    pub const fn sha256(self) -> [u8; 32] {
        self.digest
    }
}
/// Actual committed row; fields borrow one Arc-backed retained Q buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockReceipt {
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) query: BlockReceiptQuery,
    pub(crate) q: std::ops::Range<usize>,
    pub(crate) player: OwnershipStamp,
    pub(crate) chunks: Vec<(ChunkKey, OwnershipStamp)>,
    pub(crate) first: Option<u64>,
    pub(crate) count: u16,
}
impl BlockReceipt {
    /// Returns original read-only query.
    pub const fn query(&self) -> BlockReceiptQuery {
        self.query
    }
    /// Borrows exact original FCBQ body, without a second full copy.
    pub fn request_bytes(&self) -> &[u8] {
        &self.bytes[self.q.clone()]
    }
    /// Borrows the exact committed FCBR row value.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns original Player outcome, Fresh for inactive Player.
    pub const fn after_player_stamp(&self) -> OwnershipStamp {
        self.player
    }
    /// Borrows original sorted column outcomes.
    pub fn chunks(&self) -> &[(ChunkKey, OwnershipStamp)] {
        &self.chunks
    }
    /// Returns actual first journal sequence and count, absent for an empty chain.
    pub const fn journal_range(&self) -> Option<(u64, u16)> {
        match self.first {
            Some(n) => Some((n, self.count)),
            None => None,
        }
    }
}
/// Only committed/replayed historical evidence; a new snapshot is required for further writes.
#[derive(Debug, Clone)]
pub struct BlockCommitOutcome {
    pub(crate) receipt: BlockReceipt,
    pub(crate) disposition: OwnershipCommitDisposition,
}
impl BlockCommitOutcome {
    /// Borrows actual original committed evidence.
    pub const fn receipt(&self) -> &BlockReceipt {
        &self.receipt
    }
    /// Distinguishes new commit and byte-exact replay.
    pub const fn disposition(&self) -> OwnershipCommitDisposition {
        self.disposition
    }
}
/// Separate dyn-compatible opt-in contract; unsupported defaults refuse even empty calls.
// async_trait adds must_use to boxed futures that already carry that contract.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait PlayerBlockDurabilityStore: Send + Sync {
    /// Issues coherent authority from actual stored originals and generation.
    async fn load_owned_blocks(&self, _spec: BlockSnapshotSpec) -> BlockResult<OwnedBlockSnapshot> {
        Err(BlockDurabilityError::Unsupported)
    }
    /// Atomically commits complete Player/world images, ordered effects and evidence.
    async fn commit_player_blocks(
        &self,
        _request: PlayerBlockRequest,
    ) -> BlockResult<BlockCommitOutcome> {
        Err(BlockDurabilityError::Unsupported)
    }
    /// Atomically commits complete scoped gameplay without Player.
    async fn commit_blocks(
        &self,
        _request: BlockMutationRequest,
    ) -> BlockResult<BlockCommitOutcome> {
        Err(BlockDurabilityError::Unsupported)
    }
    /// Atomically replaces storage images against the same Chunk authority.
    async fn compare_exchange_chunk_images(
        &self,
        _request: ChunkImageCasRequest,
    ) -> BlockResult<BlockCommitOutcome> {
        Err(BlockDurabilityError::Unsupported)
    }
    /// Resolves original historical evidence; absence during execution proves no abort.
    async fn load_block_receipt(
        &self,
        _query: BlockReceiptQuery,
    ) -> BlockResult<Option<BlockReceipt>> {
        Err(BlockDurabilityError::Unsupported)
    }
}

pub(crate) fn ownership_adapter(e: BlockDurabilityError) -> OwnershipError {
    match e {
        BlockDurabilityError::Ownership(e) => e,
        BlockDurabilityError::Busy => OwnershipError::Busy,
        BlockDurabilityError::ReceiptKindMismatch { id } => {
            OwnershipError::ReceiptKindMismatch { id }
        }
        other => OwnershipError::Block(Box::new(other)),
    }
}

#[cfg(test)]
mod tests;
