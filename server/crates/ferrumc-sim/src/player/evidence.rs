//! Concrete off-tick evidence factories, bound to the actual borrowed backend.

use std::fmt;
use std::sync::{Arc, Weak};

use ferrumc_core::{EntityUuid, PlayerId};
use ferrumc_storage::{
    EntityOwnershipCasRequest, OwnedEntitySnapshot, OwnedPlayerEntitySnapshot, OwnedPlayerSnapshot,
    OwnershipError, OwnershipReceipt, OwnershipReceiptQuery, PersistentEntityKey,
    PersistentEntityRecord, PlayerEntityOwnershipStore, PlayerEntityTransferRequest,
    PlayerOwnershipCasRequest, ScopedPlayerKey,
};

use super::{PlayerLocalGeneration, PlayerOwnershipError};
use crate::GenerationBinding;

/// One explicit weak backend allocation and immutable generation declaration.
///
/// This holds no database handle and does not extend its lifetime. All actual
/// calls below are made by an external driver, outside a simulation tick.
#[derive(Clone)]
pub struct OwnershipStoreBinding {
    pub(crate) store: Weak<dyn PlayerEntityOwnershipStore>,
    pub(crate) generation: GenerationBinding,
    pub(crate) identity: Arc<()>,
}

impl fmt::Debug for OwnershipStoreBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnershipStoreBinding")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl OwnershipStoreBinding {
    /// Binds an explicitly selected backend; its real calls must support DUR03.
    pub fn new(store: &Arc<dyn PlayerEntityOwnershipStore>, generation: GenerationBinding) -> Self {
        Self {
            store: Arc::downgrade(store),
            generation,
            identity: Arc::new(()),
        }
    }
    /// Returns the declared scope and full generator identity, never inferred data.
    pub const fn generation(&self) -> GenerationBinding {
        self.generation
    }
    pub(crate) fn check_store(
        &self,
        store: &Arc<dyn PlayerEntityOwnershipStore>,
    ) -> Result<(), PlayerOwnershipError> {
        if self.store.strong_count() == 0 || !Weak::ptr_eq(&self.store, &Arc::downgrade(store)) {
            return Err(PlayerOwnershipError::ForeignStore);
        }
        Ok(())
    }
    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.generation == other.generation
            && Arc::ptr_eq(&self.identity, &other.identity)
            && Weak::ptr_eq(&self.store, &other.store)
    }
    fn player_key(&self, player: PlayerId) -> ScopedPlayerKey {
        ScopedPlayerKey::new(self.generation.key(), player)
    }
    fn entity_key(&self, entity: EntityUuid) -> PersistentEntityKey {
        PersistentEntityKey::new(
            self.generation.key().world(),
            self.generation.key().dimension(),
            entity,
        )
    }
    /// Obtains real player admission evidence from the exact bound store.
    pub async fn read_player(
        &self,
        store: &Arc<dyn PlayerEntityOwnershipStore>,
        player: PlayerId,
    ) -> Result<OwnedPlayerRead, PlayerOwnershipError> {
        self.check_store(store)?;
        let snapshot = store
            .load_owned_player(self.player_key(player), self.generation.identity())
            .await?;
        self.validate_player(&snapshot)?;
        let native_position = checked_player_position(&snapshot);
        Ok(OwnedPlayerRead {
            binding: self.clone(),
            snapshot,
            native_position,
        })
    }
    /// Obtains real entity admission evidence, without inventing a wire identity.
    pub async fn read_entity(
        &self,
        store: &Arc<dyn PlayerEntityOwnershipStore>,
        entity: EntityUuid,
    ) -> Result<OwnedEntityRead, PlayerOwnershipError> {
        self.check_store(store)?;
        let snapshot = store
            .load_owned_entity(self.entity_key(entity), self.generation.identity())
            .await?;
        self.validate_entity(&snapshot)?;
        let native_item_checked = snapshot.record().is_some_and(|record| {
            crate::decode_item_entity(snapshot.expectation().key(), record).is_ok()
        });
        Ok(OwnedEntityRead {
            binding: self.clone(),
            snapshot,
            native_item_checked,
        })
    }
    /// Reads the actual coherent pair used by drop and pickup reservation.
    pub async fn read_pair(
        &self,
        store: &Arc<dyn PlayerEntityOwnershipStore>,
        player: PlayerId,
        entity: EntityUuid,
    ) -> Result<OwnedPairRead, PlayerOwnershipError> {
        self.check_store(store)?;
        let snapshot = store
            .load_owned_pair(
                self.player_key(player),
                self.entity_key(entity),
                self.generation.identity(),
            )
            .await?;
        self.validate_player(snapshot.player())?;
        self.validate_entity(snapshot.entity())?;
        Ok(OwnedPairRead {
            binding: self.clone(),
            snapshot,
        })
    }
    fn validate_player(&self, snapshot: &OwnedPlayerSnapshot) -> Result<(), PlayerOwnershipError> {
        let expected = snapshot.expectation();
        if expected.key().scope() != self.generation.key()
            || expected.generation_identity() != self.generation.identity()
        {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        Ok(())
    }
    fn validate_entity(&self, snapshot: &OwnedEntitySnapshot) -> Result<(), PlayerOwnershipError> {
        let expected = snapshot.expectation();
        let key = expected.key();
        if key.world() != self.generation.key().world()
            || key.dimension() != self.generation.key().dimension()
            || expected.generation_identity() != self.generation.identity()
        {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        Ok(())
    }
}

/// Sealed readonly player evidence, created only by a real bound backend call.
#[derive(Debug)]
pub struct OwnedPlayerRead {
    pub(crate) native_position: Option<ferrumc_math::Vec3>,
    pub(crate) binding: OwnershipStoreBinding,
    pub(crate) snapshot: OwnedPlayerSnapshot,
}

pub(crate) fn checked_player_position(
    snapshot: &OwnedPlayerSnapshot,
) -> Option<ferrumc_math::Vec3> {
    snapshot
        .record()
        .and_then(|record| super::NativePlayerSnapshot::try_from_record(record).ok())
        .map(|native| native.position())
}
impl OwnedPlayerRead {
    /// Borrows the real snapshot; cloning its data cannot forge this evidence.
    pub const fn snapshot(&self) -> &OwnedPlayerSnapshot {
        &self.snapshot
    }
}
/// Sealed readonly entity evidence for restore or receipt-backed spawn.
#[derive(Debug)]
pub struct OwnedEntityRead {
    pub(crate) native_item_checked: bool,
    pub(crate) binding: OwnershipStoreBinding,
    pub(crate) snapshot: OwnedEntitySnapshot,
}
impl OwnedEntityRead {
    /// Borrows the real immutable storage snapshot.
    pub const fn snapshot(&self) -> &OwnedEntitySnapshot {
        &self.snapshot
    }
}
/// Sealed real shared player/entity snapshot for the concrete transfer consumer.
#[derive(Debug)]
pub struct OwnedPairRead {
    pub(crate) binding: OwnershipStoreBinding,
    pub(crate) snapshot: OwnedPlayerEntitySnapshot,
}
impl OwnedPairRead {
    /// Borrows the coherent actual pair without exposing mutation authority.
    pub const fn snapshot(&self) -> &OwnedPlayerEntitySnapshot {
        &self.snapshot
    }
}

#[derive(Debug, Clone)]
pub(crate) enum WorkRequest {
    Pair(PlayerEntityTransferRequest),
    Player(PlayerOwnershipCasRequest),
    Entity(EntityOwnershipCasRequest),
}
impl WorkRequest {
    pub(crate) fn query(&self) -> OwnershipReceiptQuery {
        match self {
            Self::Pair(r) => r.receipt_query(),
            Self::Player(r) => r.receipt_query(),
            Self::Entity(r) => r.receipt_query(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkKeys {
    pub(crate) player: Option<ScopedPlayerKey>,
    pub(crate) entity: Option<PersistentEntityKey>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionTicket {
    pub(crate) binding: OwnershipStoreBinding,
    pub(crate) owner: Weak<()>,
    pub(crate) generation: PlayerLocalGeneration,
    pub(crate) query: OwnershipReceiptQuery,
    pub(crate) keys: WorkKeys,
}

/// Move-owned exact storage work, with an unforgeable local completion ticket.
///
/// Only the external bounded driver executes this object. Cancellation produces
/// no completion and consequently cannot release the shard's reservation.
#[derive(Debug)]
pub struct PlayerOwnershipStorageWork {
    pub(crate) buffer: Arc<()>,
    pub(crate) ticket: CompletionTicket,
    pub(crate) request: Option<WorkRequest>,
}

/// A work refusal that retains the whole request for the original driver.
#[derive(Debug)]
pub struct PlayerOwnershipWorkRefusal {
    reason: PlayerOwnershipError,
    work: Box<PlayerOwnershipStorageWork>,
}
impl PlayerOwnershipWorkRefusal {
    /// Returns both the typed refusal and all original work.
    pub fn into_parts(self) -> (PlayerOwnershipError, PlayerOwnershipStorageWork) {
        (self.reason, *self.work)
    }
}

#[derive(Debug)]
pub(crate) enum CurrentResources {
    Pair(OwnedPlayerEntitySnapshot),
    Player(OwnedPlayerSnapshot),
    Entity(OwnedEntitySnapshot),
}

#[derive(Debug)]
pub(crate) enum CompletionEvidence {
    Confirmed {
        receipt: Box<OwnershipReceipt>,
        current: Result<CurrentResources, OwnershipError>,
    },
    Refused(OwnershipError),
    Unresolved(OwnershipError),
    ReceiptAbsent,
}

/// Sealed result of the actual bound store call/receipt/current-read sequence.
///
/// No public constructor accepts arbitrary outcomes, receipts or lifecycle bools.
#[derive(Debug)]
pub struct PlayerOwnershipCompletion {
    pub(crate) buffer: Arc<()>,
    pub(crate) ticket: CompletionTicket,
    pub(crate) evidence: CompletionEvidence,
}
impl PlayerOwnershipCompletion {
    /// Returns the original ID/digest, which is data rather than publication authority.
    pub const fn query(&self) -> OwnershipReceiptQuery {
        self.ticket.query
    }
    pub(crate) fn charged_bytes(&self) -> usize {
        match &self.evidence {
            CompletionEvidence::Confirmed {
                current: Ok(resources),
                ..
            } => {
                512 + match resources {
                    CurrentResources::Pair(p) => {
                        p.player().record().map_or(0, |r| r.data().len())
                            + p.entity().record().map_or(0, |r| r.record().data().len())
                    }
                    CurrentResources::Player(p) => p.record().map_or(0, |r| r.data().len()),
                    CurrentResources::Entity(e) => {
                        e.record().map_or(0, |r| r.record().data().len())
                    }
                }
            }
            _ => 512,
        }
    }
}

impl PlayerOwnershipStorageWork {
    /// Converts this same undispatched buffer into readonly receipt clarification.
    /// This grants no abort evidence and creates no second worker buffer.
    #[must_use]
    pub fn into_receipt_clarification(mut self) -> Self {
        self.request = None;
        self
    }
    /// Returns the exact reconstructible query without replanning the transfer.
    pub const fn query(&self) -> OwnershipReceiptQuery {
        self.ticket.query
    }
    /// Executes one exact DUR03 operation or readonly receipt clarification off-tick.
    ///
    /// Wrong-backend refusal returns all work before starting any operation.
    /// A confirmed commit followed by a failed current read remains confirmed
    /// pending evidence, never a definitive abort or permission to free items.
    pub async fn execute(
        self,
        store: &Arc<dyn PlayerEntityOwnershipStore>,
    ) -> Result<PlayerOwnershipCompletion, PlayerOwnershipWorkRefusal> {
        if let Err(reason) = self.ticket.binding.check_store(store) {
            return Err(PlayerOwnershipWorkRefusal {
                reason,
                work: Box::new(self),
            });
        }
        let Self {
            ticket,
            request,
            buffer,
        } = self;
        let result = match request {
            Some(WorkRequest::Pair(r)) => Some(store.commit_player_entity_transfer(r).await),
            Some(WorkRequest::Player(r)) => Some(store.compare_exchange_owned_player(r).await),
            Some(WorkRequest::Entity(r)) => Some(store.compare_exchange_owned_entity(r).await),
            None => None,
        };
        let evidence = match result {
            Some(Ok(outcome)) => confirmed(store, &ticket, outcome.receipt().clone()).await,
            Some(Err(
                source @ (OwnershipError::CommitUncertain { .. }
                | OwnershipError::ExecutionUncertain { .. }),
            )) => match store.load_ownership_receipt(ticket.query).await {
                Ok(Some(receipt)) => confirmed(store, &ticket, receipt).await,
                _ => CompletionEvidence::Unresolved(source),
            },
            Some(Err(source)) => CompletionEvidence::Refused(source),
            None => match store.load_ownership_receipt(ticket.query).await {
                Ok(Some(receipt)) => confirmed(store, &ticket, receipt).await,
                Ok(None) => CompletionEvidence::ReceiptAbsent,
                Err(source) => CompletionEvidence::Unresolved(source),
            },
        };
        Ok(PlayerOwnershipCompletion {
            buffer,
            ticket,
            evidence,
        })
    }
}

async fn confirmed(
    store: &Arc<dyn PlayerEntityOwnershipStore>,
    ticket: &CompletionTicket,
    receipt: OwnershipReceipt,
) -> CompletionEvidence {
    let identity = ticket.binding.generation.identity();
    let current = match (ticket.keys.player, ticket.keys.entity) {
        (Some(p), Some(e)) => store
            .load_owned_pair(p, e, identity)
            .await
            .map(CurrentResources::Pair),
        (Some(p), None) => store
            .load_owned_player(p, identity)
            .await
            .map(CurrentResources::Player),
        (None, Some(e)) => store
            .load_owned_entity(e, identity)
            .await
            .map(CurrentResources::Entity),
        (None, None) => Err(OwnershipError::InvalidConfiguration("empty native work")),
    };
    CompletionEvidence::Confirmed {
        receipt: Box::new(receipt),
        current,
    }
}

// A real receipt is additionally checked against the concrete native affected
// chunks; a matching query alone is not enough for local publication/recovery.
pub(crate) fn check_receipt_epochs(
    receipt: &OwnershipReceipt,
    binding: GenerationBinding,
    before: Option<&PersistentEntityRecord>,
    after: Option<&PersistentEntityRecord>,
) -> Result<(), PlayerOwnershipError> {
    if before.zip(after).is_some_and(|(a, b)| {
        a.chunk() == b.chunk()
            && a.record().schema_version() == b.record().schema_version()
            && a.record().data() == b.record().data()
    }) {
        return if receipt.epochs().is_empty() {
            Ok(())
        } else {
            Err(PlayerOwnershipError::CompletionMismatch)
        };
    }
    let affected: std::collections::BTreeSet<_> = before
        .map(PersistentEntityRecord::chunk)
        .into_iter()
        .chain(after.map(PersistentEntityRecord::chunk))
        .collect();
    if receipt.epochs().len() != affected.len()
        || receipt.epochs().iter().zip(affected).any(|(epoch, chunk)| {
            epoch.chunk().world() != binding.key().world()
                || epoch.chunk().dimension() != binding.key().dimension()
                || epoch.chunk().pos() != chunk
                || epoch.before().checked_add(1) != Some(epoch.after())
        })
    {
        return Err(PlayerOwnershipError::CompletionMismatch);
    }
    Ok(())
}
