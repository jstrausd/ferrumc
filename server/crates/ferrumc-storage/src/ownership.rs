//! Explicit, receipt-backed Player/UUID-entity ownership; unrelated to chunk epochs.

use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use ferrumc_core::{PlayerId, ServerError};
use ferrumc_math::ChunkPos;
use ferrumc_world::GenerationIdentity;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::{
    ChunkKey, GenerationKey, PersistentEntityKey, PersistentEntityRecord, PlayerRecord,
    StorageError,
};

pub(crate) mod engine;
pub(crate) mod raw;
#[cfg(test)]
pub(crate) mod tests;

/// Maximum canonical bytes accepted by any ownership request (2 MiB).
pub const MAX_OWNERSHIP_REQUEST_BYTES: usize = 2 * 1024 * 1024;
/// Maximum concurrently admitted ownership workers per backend owner.
pub const MAX_OWNERSHIP_INFLIGHT: u16 = 64;
/// Typed ownership results preserve uncertainty and conflicts across crate boundaries.
pub type OwnershipResult<T> = std::result::Result<T, OwnershipError>;

/// A precise ownership refusal; an uncertain operation must retain its pending authority.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OwnershipError {
    /// This mode has not adopted the ownership contract.
    #[error("Player/entity ownership is not supported by this storage mode")]
    Unsupported,
    /// A configuration would remove bounds or overflow accounting.
    #[error("invalid ownership configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// Existing records need an explicit, separately specified scope migration.
    #[error("ownership requires explicit offline migration from format {actual} to 6")]
    MigrationRequired {
        /// Actual durable format.
        actual: u64,
    },
    /// Ordinary reopen cannot reset or modify the durable capacity agreement.
    #[error("ownership capacity agreement differs from the persisted contract")]
    CapacityContractMismatch,
    /// Durable table names, types, marker or metadata are incomplete or unknown.
    #[error("invalid ownership catalog: {0}")]
    CatalogMismatch(&'static str),
    /// A fixed record or encountered relationship is inconsistent.
    #[error("corrupt ownership storage in {table}: {reason}")]
    MalformedOwnershipData {
        /// Table or grammar.
        table: &'static str,
        /// Exact refusal.
        reason: &'static str,
    },
    /// A reused bounded record or generation contract refused.
    #[error("ownership record or generation failure: {0}")]
    Record(#[from] StorageError),
    /// Pair resources or expectations have different scopes or generator identities.
    #[error("ownership pair has incompatible scopes or generation identities")]
    ScopeMismatch,
    /// A global player or world/UUID already belongs to a different scope.
    #[error("ownership resource {resource:?} is bound to another scope")]
    ScopeConflict {
        /// Conflicting resource domain.
        resource: OwnershipResource,
    },
    /// New CAS requests require expectations from this live backend incarnation.
    #[error("ownership expectation belongs to another backend incarnation")]
    BackendOwnerMismatch,
    /// Current durable state differs from the supplied CAS expectation.
    #[error("ownership CAS conflict for {resource:?}: expected {expected:?}, actual {actual:?}")]
    CasConflict {
        /// Conflicting resource.
        resource: OwnershipResource,
        /// Supplied stamp.
        expected: OwnershipStamp,
        /// Current durable stamp.
        actual: OwnershipStamp,
    },
    /// Deleting fresh or tombstone absence is not a new logical operation.
    #[error("ownership resource {resource:?} is already absent ({stamp:?})")]
    AlreadyAbsent {
        /// Resource being deleted.
        resource: OwnershipResource,
        /// Observed fresh/tombstone stamp.
        stamp: OwnershipStamp,
    },
    /// One operation ID cannot represent a different canonical request.
    #[error("ownership receipt ID {id:?} collides with another request")]
    ReceiptCollision {
        /// Supplied operation ID.
        id: OwnershipTransferId,
        /// Requested digest.
        expected: OwnershipRequestIdentity,
        /// Persisted digest.
        stored: OwnershipRequestIdentity,
    },
    /// Canonical bytes exceed the hard or persisted admission bound.
    #[error("ownership request is {len} bytes (maximum {max})")]
    RequestBytesExceeded {
        /// Requested bytes.
        len: usize,
        /// Allowed bytes.
        max: usize,
    },
    /// Permanent receipts or revision histories have exhausted their admission.
    #[error("ownership {kind:?} history capacity exhausted (maximum {max})")]
    HistoryCapacity {
        /// Exhausted history.
        kind: OwnershipHistory,
        /// Persisted row limit.
        max: u64,
    },
    /// Startup cannot completely validate within its supplied work limit.
    #[error("ownership startup scan exceeded its {0} limit")]
    ScanCapacity(&'static str),
    /// Bounded journal validation scratch refused further growth.
    #[error("ownership journal validation scratch exceeded its limit")]
    ScratchCapacity,
    /// No worker permit was available; this operation did not start.
    #[error("ownership worker admission is full")]
    Busy,
    /// A real change cannot increment the retained positive revision.
    #[error("ownership revision exhausted for {resource:?}")]
    RevisionExhausted {
        /// Resource at u64MAX.
        resource: OwnershipResource,
    },
    /// Operational I/O, locking or worker failure before a commit attempt.
    #[error("ownership backend failure: {0}")]
    Backend(String),
    /// Actual commit returned an error; absence or success must be resolved by receipt.
    #[error("ownership commit is uncertain for {query:?}: {cause}")]
    CommitUncertain {
        /// Durable retry identity.
        query: OwnershipReceiptQuery,
        /// Actual backend cause.
        cause: String,
    },
    /// A started worker failed or lost its reply; it may already have committed.
    #[error("ownership execution is uncertain for {query:?}: {cause}")]
    ExecutionUncertain {
        /// Durable retry identity.
        query: OwnershipReceiptQuery,
        /// Actual worker/reply cause.
        cause: String,
    },
}

impl From<OwnershipError> for ServerError {
    fn from(error: OwnershipError) -> Self {
        match error {
            OwnershipError::Unsupported => Self::unsupported("Player/entity ownership"),
            OwnershipError::Record(source) => source.into(),
            other => Self::internal(other.to_string()),
        }
    }
}

/// The resource whose ownership was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipResource {
    /// Globally UUID-keyed player.
    Player,
    /// World/UUID-keyed entity.
    Entity,
}
/// Permanent history admission domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipHistory {
    /// Idempotency receipts, including no-ops.
    Receipts,
    /// Live and deleted resource headers.
    Revisions,
}

/// Explicit player scope; the payload remains keyed globally by the full player UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopedPlayerKey {
    scope: GenerationKey,
    id: PlayerId,
}
impl ScopedPlayerKey {
    /// Binds a global player UUID to the explicit requested world/dimension.
    #[must_use]
    pub const fn new(scope: GenerationKey, id: PlayerId) -> Self {
        Self { scope, id }
    }
    /// Returns the explicit scope.
    #[must_use]
    pub const fn scope(self) -> GenerationKey {
        self.scope
    }
    /// Returns the full global player identity.
    #[must_use]
    pub const fn id(self) -> PlayerId {
        self.id
    }
}

/// Caller-supplied operation identity, retained forever within the capacity agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OwnershipTransferId([u8; 16]);
impl OwnershipTransferId {
    /// Preserves all128 supplied bits, including zero.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    /// Returns every operation identity bit.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}
/// Positive persistent resource revision assigned by storage, never a chunk epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnershipRevision(NonZeroU64);
impl OwnershipRevision {
    /// Returns the positive revision number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}
/// Fresh, live or deleted state; decoding a stamp does not create CAS authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipStamp {
    tag: u8,
    revision: u64,
}
impl OwnershipStamp {
    pub(crate) const FRESH: Self = Self {
        tag: 0,
        revision: 0,
    };
    pub(crate) fn checked(tag: u8, revision: u64) -> OwnershipResult<Self> {
        if tag > 2 || (tag == 0) != (revision == 0) {
            return Err(raw::bad("ownership_stamp", "invalid state/revision"));
        }
        Ok(Self { tag, revision })
    }
    /// True only for a resource with no retained ownership history.
    #[must_use]
    pub const fn is_fresh(self) -> bool {
        self.tag == 0
    }
    /// True for a retained live resource.
    #[must_use]
    pub const fn is_live(self) -> bool {
        self.tag == 1
    }
    /// True for retained deletion history.
    #[must_use]
    pub const fn is_tombstone(self) -> bool {
        self.tag == 2
    }
    /// Returns the retained positive revision, or none for never-seen absence.
    #[must_use]
    pub fn revision(self) -> Option<OwnershipRevision> {
        NonZeroU64::new(self.revision).map(OwnershipRevision)
    }
    /// Encodes the exact tag/u64 big-endian9-byte grammar, without owner authority.
    #[must_use]
    pub fn encode(self) -> [u8; 9] {
        let mut out = [0; 9];
        out[0] = self.tag;
        out[1..].copy_from_slice(&self.revision.to_be_bytes());
        out
    }
    /// Rejects unknown tags, zero live revisions, nonzero fresh revisions and trailing bytes.
    pub fn decode(bytes: &[u8]) -> OwnershipResult<Self> {
        let b = raw::array::<9>(bytes, "ownership_stamp")?;
        Self::checked(
            b[0],
            u64::from_be_bytes(raw::array(&b[1..], "ownership_stamp")?),
        )
    }
}

/// A snapshot-issued player expectation bound to one live owner, key and generation.
///
/// Raw fields cannot be supplied by a consumer:
/// ```compile_fail,E0451
/// use ferrumc_storage::{OwnedPlayerExpectation, ScopedPlayerKey, OwnershipStamp};
/// use ferrumc_world::GenerationIdentity;
/// fn forge(key: ScopedPlayerKey, identity: GenerationIdentity, stamp: OwnershipStamp) {
///     OwnedPlayerExpectation { key, identity, stamp, owner: std::sync::Weak::new() };
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OwnedPlayerExpectation {
    pub(crate) key: ScopedPlayerKey,
    pub(crate) identity: GenerationIdentity,
    pub(crate) stamp: OwnershipStamp,
    pub(crate) owner: Weak<()>,
}
impl OwnedPlayerExpectation {
    /// Returns the explicit player key.
    #[must_use]
    pub const fn key(&self) -> ScopedPlayerKey {
        self.key
    }
    /// Returns the complete immutable generator identity.
    #[must_use]
    pub const fn generation_identity(&self) -> GenerationIdentity {
        self.identity
    }
    /// Returns the expected state; it is not independently writable authority.
    #[must_use]
    pub const fn stamp(&self) -> OwnershipStamp {
        self.stamp
    }
}
/// A snapshot-issued UUID-entity expectation bound to one live backend incarnation.
#[derive(Debug, Clone)]
pub struct OwnedEntityExpectation {
    pub(crate) key: PersistentEntityKey,
    pub(crate) identity: GenerationIdentity,
    pub(crate) stamp: OwnershipStamp,
    pub(crate) owner: Weak<()>,
}
impl OwnedEntityExpectation {
    /// Returns the explicit UUID-entity key.
    #[must_use]
    pub const fn key(&self) -> PersistentEntityKey {
        self.key
    }
    /// Returns the complete immutable generator identity.
    #[must_use]
    pub const fn generation_identity(&self) -> GenerationIdentity {
        self.identity
    }
    /// Returns the expected state.
    #[must_use]
    pub const fn stamp(&self) -> OwnershipStamp {
        self.stamp
    }
}
/// Coherent player payload and revision; missing history and deletion differ.
#[derive(Debug, Clone)]
pub struct OwnedPlayerSnapshot {
    pub(crate) expected: OwnedPlayerExpectation,
    pub(crate) record: Option<PlayerRecord>,
}
impl OwnedPlayerSnapshot {
    /// Borrows the live payload, absent for fresh or tombstone state.
    #[must_use]
    pub fn record(&self) -> Option<&PlayerRecord> {
        self.record.as_ref()
    }
    /// Borrows this snapshot's private owner-bound expectation.
    #[must_use]
    pub const fn expectation(&self) -> &OwnedPlayerExpectation {
        &self.expected
    }
    /// Returns the state captured with the payload.
    #[must_use]
    pub const fn stamp(&self) -> OwnershipStamp {
        self.expected.stamp
    }
}
/// Coherent UUID-entity payload, scope and persistent ownership revision.
#[derive(Debug, Clone)]
pub struct OwnedEntitySnapshot {
    pub(crate) expected: OwnedEntityExpectation,
    pub(crate) record: Option<PersistentEntityRecord>,
}
impl OwnedEntitySnapshot {
    /// Borrows the live payload, absent for fresh or tombstone state.
    #[must_use]
    pub fn record(&self) -> Option<&PersistentEntityRecord> {
        self.record.as_ref()
    }
    /// Borrows this snapshot's private owner-bound expectation.
    #[must_use]
    pub const fn expectation(&self) -> &OwnedEntityExpectation {
        &self.expected
    }
    /// Returns the state captured with the payload.
    #[must_use]
    pub const fn stamp(&self) -> OwnershipStamp {
        self.expected.stamp
    }
}
/// Both resource snapshots captured within one real backend read boundary.
#[derive(Debug, Clone)]
pub struct OwnedPlayerEntitySnapshot {
    pub(crate) player: OwnedPlayerSnapshot,
    pub(crate) entity: OwnedEntitySnapshot,
}
impl OwnedPlayerEntitySnapshot {
    /// Borrows the player snapshot.
    #[must_use]
    pub const fn player(&self) -> &OwnedPlayerSnapshot {
        &self.player
    }
    /// Borrows the entity snapshot from the same read boundary.
    #[must_use]
    pub const fn entity(&self) -> &OwnedEntitySnapshot {
        &self.entity
    }
    /// Moves both coherent snapshots to the caller.
    #[must_use]
    pub fn into_parts(self) -> (OwnedPlayerSnapshot, OwnedEntitySnapshot) {
        (self.player, self.entity)
    }
}
/// One bounded player put or live deletion; no generic storage actions.
#[derive(Debug, Clone)]
pub struct PlayerOwnershipProposal {
    pub(crate) record: Option<PlayerRecord>,
}
impl PlayerOwnershipProposal {
    /// Moves a prevalidated bounded player record into this proposal.
    #[must_use]
    pub fn put(record: PlayerRecord) -> Self {
        Self {
            record: Some(record),
        }
    }
    /// Requests deletion of a live player while retaining its revision history.
    #[must_use]
    pub const fn delete() -> Self {
        Self { record: None }
    }
}
/// One bounded entity put or live deletion; dimension moves are unsupported.
#[derive(Debug, Clone)]
pub struct EntityOwnershipProposal {
    pub(crate) record: Option<PersistentEntityRecord>,
}
impl EntityOwnershipProposal {
    /// Moves a prevalidated bounded UUID-entity record into this proposal.
    #[must_use]
    pub fn put(record: PersistentEntityRecord) -> Self {
        Self {
            record: Some(record),
        }
    }
    /// Requests live deletion with a permanent tombstone.
    #[must_use]
    pub const fn delete() -> Self {
        Self { record: None }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Request {
    digest: OwnershipRequestIdentity,
    pub(crate) kind: u8,
    pub(crate) id: OwnershipTransferId,
    pub(crate) scope: GenerationKey,
    pub(crate) identity: GenerationIdentity,
    pub(crate) player: Option<(OwnedPlayerExpectation, PlayerOwnershipProposal)>,
    pub(crate) entity: Option<(OwnedEntityExpectation, EntityOwnershipProposal)>,
}
impl Request {
    pub(crate) fn len(&self) -> OwnershipResult<usize> {
        let p = self
            .player
            .as_ref()
            .and_then(|(_, p)| p.record.as_ref())
            .map_or(0, |r| 5 + r.data().len());
        let e = self
            .entity
            .as_ref()
            .and_then(|(_, p)| p.record.as_ref())
            .map_or(0, |r| 22 + r.record().data().len());
        149usize
            .checked_add(p)
            .and_then(|n| n.checked_add(e))
            .ok_or(OwnershipError::RequestBytesExceeded {
                len: usize::MAX,
                max: MAX_OWNERSHIP_REQUEST_BYTES,
            })
    }
    pub(crate) fn admission(&self, max: usize) -> OwnershipResult<()> {
        let len = self.len()?;
        if len > max {
            Err(OwnershipError::RequestBytesExceeded { len, max })
        } else {
            Ok(())
        }
    }
    pub(crate) fn query(&self) -> OwnershipReceiptQuery {
        OwnershipReceiptQuery {
            id: self.id,
            identity: self.digest,
        }
    }
}
/// Exactly one player plus one UUID entity in the same explicitly verified scope.
#[derive(Debug, Clone)]
pub struct PlayerEntityTransferRequest(pub(crate) Request);
impl PlayerEntityTransferRequest {
    /// Builds a bounded pair; live-owner/CAS checks follow receipt lookup at execution.
    pub fn new(
        id: OwnershipTransferId,
        player: OwnedPlayerExpectation,
        entity: OwnedEntityExpectation,
        player_proposal: PlayerOwnershipProposal,
        entity_proposal: EntityOwnershipProposal,
    ) -> OwnershipResult<Self> {
        let scope = player.key.scope;
        if scope != crate::persistent_entity::engine::scope(entity.key)
            || player.identity != entity.identity
        {
            return Err(OwnershipError::ScopeMismatch);
        }
        let mut r = Request {
            digest: OwnershipRequestIdentity([0; 32]),
            kind: 1,
            id,
            scope,
            identity: player.identity,
            player: Some((player, player_proposal)),
            entity: Some((entity, entity_proposal)),
        };
        r.admission(MAX_OWNERSHIP_REQUEST_BYTES)?;
        r.digest = raw::request_identity(&r);
        Ok(Self(r))
    }
    /// Returns the reconstructible read-only receipt identity.
    #[must_use]
    pub fn receipt_query(&self) -> OwnershipReceiptQuery {
        self.0.query()
    }
}
/// Receipt-backed player bootstrap/autosave CAS, without a dummy entity.
#[derive(Debug, Clone)]
pub struct PlayerOwnershipCasRequest(pub(crate) Request);
impl PlayerOwnershipCasRequest {
    /// Builds one bounded player CAS request.
    pub fn new(
        id: OwnershipTransferId,
        expected: OwnedPlayerExpectation,
        proposal: PlayerOwnershipProposal,
    ) -> OwnershipResult<Self> {
        let mut r = Request {
            digest: OwnershipRequestIdentity([0; 32]),
            kind: 2,
            id,
            scope: expected.key.scope,
            identity: expected.identity,
            player: Some((expected, proposal)),
            entity: None,
        };
        r.admission(MAX_OWNERSHIP_REQUEST_BYTES)?;
        r.digest = raw::request_identity(&r);
        Ok(Self(r))
    }
    /// Returns the reconstructible read-only receipt identity.
    #[must_use]
    pub fn receipt_query(&self) -> OwnershipReceiptQuery {
        self.0.query()
    }
}
/// Receipt-backed entity spawn/autosave CAS, without a dummy player.
#[derive(Debug, Clone)]
pub struct EntityOwnershipCasRequest(pub(crate) Request);
impl EntityOwnershipCasRequest {
    /// Builds one bounded entity CAS request.
    pub fn new(
        id: OwnershipTransferId,
        expected: OwnedEntityExpectation,
        proposal: EntityOwnershipProposal,
    ) -> OwnershipResult<Self> {
        let mut r = Request {
            digest: OwnershipRequestIdentity([0; 32]),
            kind: 3,
            id,
            scope: crate::persistent_entity::engine::scope(expected.key),
            identity: expected.identity,
            player: None,
            entity: Some((expected, proposal)),
        };
        r.admission(MAX_OWNERSHIP_REQUEST_BYTES)?;
        r.digest = raw::request_identity(&r);
        Ok(Self(r))
    }
    /// Returns the reconstructible read-only receipt identity.
    #[must_use]
    pub fn receipt_query(&self) -> OwnershipReceiptQuery {
        self.0.query()
    }
}

/// Explicit SHA256 equivalence for the bounded canonical ownership request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipRequestIdentity(pub(crate) [u8; 32]);
impl OwnershipRequestIdentity {
    /// Returns the fixed digest, which does not grant write authority.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}
/// Durable ID/digest query for retry/restart clarification; not a live CAS token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipReceiptQuery {
    pub(crate) id: OwnershipTransferId,
    pub(crate) identity: OwnershipRequestIdentity,
}
impl OwnershipReceiptQuery {
    /// Reconstructs a fixed read-only query from a caller's pending record.
    #[must_use]
    pub const fn from_bytes(id: OwnershipTransferId, sha256: [u8; 32]) -> Self {
        Self {
            id,
            identity: OwnershipRequestIdentity(sha256),
        }
    }
    /// Returns the supplied logical operation ID.
    #[must_use]
    pub const fn id(self) -> OwnershipTransferId {
        self.id
    }
    /// Returns its original canonical request digest.
    #[must_use]
    pub const fn identity(self) -> OwnershipRequestIdentity {
        self.identity
    }
}
/// Historical entity-set epoch evidence for one changed chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipEpochChange {
    pub(crate) chunk: ChunkKey,
    pub(crate) before: u64,
    pub(crate) after: u64,
}
impl OwnershipEpochChange {
    /// Returns the changed chunk column.
    #[must_use]
    pub const fn chunk(self) -> ChunkKey {
        self.chunk
    }
    /// Returns its epoch before the committed operation.
    #[must_use]
    pub const fn before(self) -> u64 {
        self.before
    }
    /// Returns its positive epoch after the committed operation.
    #[must_use]
    pub const fn after(self) -> u64 {
        self.after
    }
}
/// Original fixed-size committed outcome evidence; historical states remain historical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipReceipt {
    pub(crate) kind: u8,
    pub(crate) query: OwnershipReceiptQuery,
    pub(crate) scope: GenerationKey,
    pub(crate) identity: GenerationIdentity,
    pub(crate) player: Option<PlayerId>,
    pub(crate) entity: Option<ferrumc_core::EntityUuid>,
    pub(crate) before_player: OwnershipStamp,
    pub(crate) after_player: OwnershipStamp,
    pub(crate) before_entity: OwnershipStamp,
    pub(crate) after_entity: OwnershipStamp,
    pub(crate) epochs: Vec<OwnershipEpochChange>,
    pub(crate) before_chunk: Option<ChunkPos>,
    pub(crate) after_chunk: Option<ChunkPos>,
}
impl OwnershipReceipt {
    /// Returns the durable ID/digest query.
    #[must_use]
    pub const fn query(&self) -> OwnershipReceiptQuery {
        self.query
    }
    /// Returns the explicit shared world/dimension.
    #[must_use]
    pub const fn scope(&self) -> GenerationKey {
        self.scope
    }
    /// Returns the full immutable generator identity.
    #[must_use]
    pub const fn generation_identity(&self) -> GenerationIdentity {
        self.identity
    }
    /// Returns active player identity and its before/after state, or none for entity-only CAS.
    #[must_use]
    pub fn player(&self) -> Option<(PlayerId, OwnershipStamp, OwnershipStamp)> {
        self.player
            .map(|id| (id, self.before_player, self.after_player))
    }
    /// Returns active entity identity and its before/after state, or none for player-only CAS.
    #[must_use]
    pub fn entity(&self) -> Option<(ferrumc_core::EntityUuid, OwnershipStamp, OwnershipStamp)> {
        self.entity
            .map(|id| (id, self.before_entity, self.after_entity))
    }
    /// Borrows at most two original epoch transitions.
    #[must_use]
    pub fn epochs(&self) -> &[OwnershipEpochChange] {
        &self.epochs
    }
    /// Returns the original live entity's old chunk, if any.
    #[must_use]
    pub const fn before_entity_chunk(&self) -> Option<ChunkPos> {
        self.before_chunk
    }
    /// Returns the original live entity's new chunk, if any.
    #[must_use]
    pub const fn after_entity_chunk(&self) -> Option<ChunkPos> {
        self.after_chunk
    }
}
/// Whether this invocation committed or returned the unchanged prior evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipCommitDisposition {
    /// New operation durably confirmed.
    Committed,
    /// Identical operation already committed.
    Replayed,
}
/// Committed evidence plus owner-bound original after-expectations; replays may be stale.
#[derive(Debug, Clone)]
pub struct OwnershipCommitOutcome {
    pub(crate) receipt: OwnershipReceipt,
    pub(crate) disposition: OwnershipCommitDisposition,
    pub(crate) player: Option<OwnedPlayerExpectation>,
    pub(crate) entity: Option<OwnedEntityExpectation>,
}
impl OwnershipCommitOutcome {
    /// Borrows the original outcome evidence.
    #[must_use]
    pub const fn receipt(&self) -> &OwnershipReceipt {
        &self.receipt
    }
    /// Distinguishes a new confirmed commit from an identical replay.
    #[must_use]
    pub const fn disposition(&self) -> OwnershipCommitDisposition {
        self.disposition
    }
    /// Borrows the original player after-expectation; later writes still cause CAS conflict.
    #[must_use]
    pub fn player_expectation(&self) -> Option<&OwnedPlayerExpectation> {
        self.player.as_ref()
    }
    /// Borrows the original entity after-expectation; it does not claim the state is still live.
    #[must_use]
    pub fn entity_expectation(&self) -> Option<&OwnedEntityExpectation> {
        self.entity.as_ref()
    }
}

/// Immutable persisted request/receipt/revision admission agreement; history never expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipCapacity {
    request: usize,
    receipts: u64,
    revisions: u64,
}
impl OwnershipCapacity {
    /// Rejects missing bounds, too-small/large requests and overflow of retained logical bytes.
    pub fn new(
        request: NonZeroUsize,
        receipts: NonZeroU64,
        revisions: NonZeroU64,
    ) -> OwnershipResult<Self> {
        if !(149..=MAX_OWNERSHIP_REQUEST_BYTES).contains(&request.get()) {
            return Err(OwnershipError::InvalidConfiguration(
                "request limit outside149..=2MiB",
            ));
        }
        receipts
            .get()
            .checked_mul(272)
            .and_then(|n| {
                revisions
                    .get()
                    .checked_mul(55)
                    .and_then(|r| n.checked_add(r))
            })
            .ok_or(OwnershipError::InvalidConfiguration(
                "history logical byte limit overflow",
            ))?;
        Ok(Self {
            request: request.get(),
            receipts: receipts.get(),
            revisions: revisions.get(),
        })
    }
    /// Returns the persisted canonical request limit.
    #[must_use]
    pub const fn max_request_bytes(self) -> usize {
        self.request
    }
    /// Returns the immutable maximum number of permanent receipts.
    #[must_use]
    pub const fn max_receipts(self) -> u64 {
        self.receipts
    }
    /// Returns the immutable maximum number of live/tombstone headers.
    #[must_use]
    pub const fn max_revision_rows(self) -> u64 {
        self.revisions
    }
}
/// Local, explicit startup work/scratch and concurrent-worker limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipOpenLimits {
    rows: u64,
    bytes: u64,
    scratch: u64,
    inflight: u16,
}
impl OwnershipOpenLimits {
    /// Keeps every scan/scratch bound positive and worker admission within1..=64.
    pub fn new(
        rows: NonZeroU64,
        bytes: NonZeroU64,
        scratch: NonZeroU64,
        inflight: NonZeroU16,
    ) -> OwnershipResult<Self> {
        if inflight.get() > MAX_OWNERSHIP_INFLIGHT {
            return Err(OwnershipError::InvalidConfiguration("inflight exceeds64"));
        }
        Ok(Self {
            rows: rows.get(),
            bytes: bytes.get(),
            scratch: scratch.get(),
            inflight: inflight.get(),
        })
    }
    /// Returns the complete-startup streamed row bound.
    #[must_use]
    pub const fn max_scan_rows(self) -> u64 {
        self.rows
    }
    /// Returns the complete-startup encoded key/value byte bound.
    #[must_use]
    pub const fn max_scan_bytes(self) -> u64 {
        self.bytes
    }
    /// Returns the physical disposable journal range-index extent bound.
    #[must_use]
    pub const fn max_journal_index_bytes(self) -> u64 {
        self.scratch
    }
    /// Returns the non-waiting admission limit shared by all owner clones.
    #[must_use]
    pub const fn max_inflight(self) -> u16 {
        self.inflight
    }
}
/// Explicit ownership admission; no implicit default or silent capacity reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipConfig {
    capacity: OwnershipCapacity,
    limits: OwnershipOpenLimits,
}
impl OwnershipConfig {
    /// Combines prevalidated persistent and local resource contracts.
    #[must_use]
    pub const fn new(capacity: OwnershipCapacity, limits: OwnershipOpenLimits) -> Self {
        Self { capacity, limits }
    }
    /// Returns the immutable persisted admission agreement.
    #[must_use]
    pub const fn capacity(self) -> OwnershipCapacity {
        self.capacity
    }
    /// Returns local scan/scratch/concurrency limits.
    #[must_use]
    pub const fn open_limits(self) -> OwnershipOpenLimits {
        self.limits
    }
}
#[derive(Debug)]
pub(crate) struct Runtime {
    pub(crate) config: OwnershipConfig,
    permits: Arc<Semaphore>,
}
impl Runtime {
    pub(crate) fn new(config: OwnershipConfig) -> Self {
        Self {
            config,
            permits: Arc::new(Semaphore::new(usize::from(config.limits.inflight))),
        }
    }
    pub(crate) fn permit(&self) -> OwnershipResult<OwnedSemaphorePermit> {
        self.permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| OwnershipError::Busy)
    }
}

/// Separate dyn-compatible ownership authority; defaults never delegate to blind writes.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait PlayerEntityOwnershipStore: Send + Sync {
    /// Loads a coherent explicitly scoped player payload and history stamp.
    async fn load_owned_player(
        &self,
        _key: ScopedPlayerKey,
        _expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerSnapshot> {
        Err(OwnershipError::Unsupported)
    }
    /// Loads a coherent scoped UUID entity and history stamp.
    async fn load_owned_entity(
        &self,
        _key: PersistentEntityKey,
        _expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedEntitySnapshot> {
        Err(OwnershipError::Unsupported)
    }
    /// Captures both payloads and revisions under one actual read boundary.
    async fn load_owned_pair(
        &self,
        _player: ScopedPlayerKey,
        _entity: PersistentEntityKey,
        _expected: GenerationIdentity,
    ) -> OwnershipResult<OwnedPlayerEntitySnapshot> {
        Err(OwnershipError::Unsupported)
    }
    /// Atomically CASes exactly one player and entity, including permanent receipt/history.
    async fn commit_player_entity_transfer(
        &self,
        _request: PlayerEntityTransferRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        Err(OwnershipError::Unsupported)
    }
    /// Versioned receipt-backed player bootstrap/autosave, without an entity placeholder.
    async fn compare_exchange_owned_player(
        &self,
        _request: PlayerOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        Err(OwnershipError::Unsupported)
    }
    /// Versioned receipt-backed entity spawn/autosave, without a player placeholder.
    async fn compare_exchange_owned_entity(
        &self,
        _request: EntityOwnershipCasRequest,
    ) -> OwnershipResult<OwnershipCommitOutcome> {
        Err(OwnershipError::Unsupported)
    }
    /// Clarifies a historical operation; none means absent in this snapshot, not worker abort.
    async fn load_ownership_receipt(
        &self,
        _query: OwnershipReceiptQuery,
    ) -> OwnershipResult<Option<OwnershipReceipt>> {
        Err(OwnershipError::Unsupported)
    }
}
