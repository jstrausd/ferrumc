//! Shared bounded ownership preflight; application belongs to one backend transaction.

use std::ops::Bound;

use crate::persistent_entity::engine::{EntityRead, RawRows, RawTable};
use crate::persistent_entity::{engine as uuid_engine, raw as uuid_raw};
use crate::{PersistentEntityWrite, MAX_PLAYER_DATA_LEN};

use super::{
    raw, Arc, ChunkPos, EntityOwnershipProposal, GenerationIdentity, GenerationKey,
    OwnedEntityExpectation, OwnedEntitySnapshot, OwnedPlayerEntitySnapshot, OwnedPlayerExpectation,
    OwnedPlayerSnapshot, OwnershipCapacity, OwnershipCommitDisposition, OwnershipCommitOutcome,
    OwnershipEpochChange, OwnershipError, OwnershipHistory, OwnershipReceipt,
    OwnershipReceiptQuery, OwnershipResource, OwnershipResult, OwnershipStamp, PersistentEntityKey,
    PersistentEntityRecord, PlayerOwnershipProposal, Request, ScopedPlayerKey, StorageError, Weak,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Table {
    Player,
    PlayerHeader,
    EntityHeader,
    Receipt,
}
pub(crate) trait Read: EntityRead {
    fn owned(&self, table: Table, key: &[u8]) -> OwnershipResult<Option<Self::Bytes<'_>>>;
    fn counts(&self) -> OwnershipResult<(u64, u64)>;
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct State {
    pub(crate) stamp: OwnershipStamp,
    pub(crate) chunk: Option<ChunkPos>,
}

fn identity(
    reader: &impl EntityRead,
    scope: GenerationKey,
    expected: GenerationIdentity,
) -> OwnershipResult<()> {
    uuid_engine::require_identity(reader, scope, Some(expected))?;
    Ok(())
}
fn player_bytes(bytes: &[u8]) -> OwnershipResult<()> {
    if bytes.len() < 5
        || bytes.len() > 5 + MAX_PLAYER_DATA_LEN
        || ferrumc_core::GameMode::from_id(bytes[4]).is_none()
    {
        return Err(raw::bad("ferrumc:player", "invalid bounded player record"));
    }
    Ok(())
}
fn raw_header(
    reader: &impl Read,
    resource: OwnershipResource,
    key: &[u8],
) -> OwnershipResult<Option<raw::Header>> {
    reader
        .owned(
            match resource {
                OwnershipResource::Player => Table::PlayerHeader,
                OwnershipResource::Entity => Table::EntityHeader,
            },
            key,
        )?
        .map(|b| match resource {
            OwnershipResource::Player => raw::player_header(b.as_ref()),
            OwnershipResource::Entity => raw::entity_header(key, b.as_ref()),
        })
        .transpose()
}
fn historical_stamp(
    reader: &impl Read,
    resource: OwnershipResource,
    key: &[u8],
    scope: GenerationKey,
    after: OwnershipStamp,
) -> OwnershipResult<()> {
    let header = raw_header(reader, resource, key)?
        .ok_or_else(|| raw::bad(raw::RECEIPT, "receipt has no retained ownership history"))?;
    if header.scope != scope
        || header.stamp.revision < after.revision
        || (header.stamp.revision == after.revision && header.stamp != after)
    {
        return Err(raw::bad(
            raw::RECEIPT,
            "receipt disagrees with retained ownership history",
        ));
    }
    Ok(())
}
/// Historical checks read raw headers; they deliberately do not recurse through last receipts.
pub(crate) fn receipt_relations(
    reader: &impl Read,
    receipt: &OwnershipReceipt,
) -> OwnershipResult<()> {
    identity(reader, receipt.scope, receipt.identity)?;
    if let Some(id) = receipt.player {
        historical_stamp(
            reader,
            OwnershipResource::Player,
            &raw::player_key(ScopedPlayerKey::new(receipt.scope, id)),
            receipt.scope,
            receipt.after_player,
        )?;
    }
    if let Some(id) = receipt.entity {
        let key = PersistentEntityKey::new(receipt.scope.world(), receipt.scope.dimension(), id);
        historical_stamp(
            reader,
            OwnershipResource::Entity,
            &raw::entity_key(key),
            receipt.scope,
            receipt.after_entity,
        )?;
        let header = raw_header(reader, OwnershipResource::Entity, &raw::entity_key(key))?
            .ok_or_else(|| raw::bad(raw::RECEIPT, "missing retained entity history"))?;
        if header.stamp == receipt.after_entity && header.stamp.is_live() {
            let bytes = reader
                .get(RawTable::Entity, &uuid_raw::key_bytes(key))?
                .ok_or_else(|| raw::bad(raw::RECEIPT, "current receipt has no entity payload"))?;
            if Some(uuid_raw::envelope(bytes.as_ref())?.chunk) != receipt.after_chunk {
                return Err(raw::bad(
                    raw::RECEIPT,
                    "current receipt chunk disagrees with entity payload",
                ));
            }
        }
    }
    for e in &receipt.epochs {
        if uuid_engine::epoch(reader, e.chunk)?.get() < e.after {
            return Err(raw::bad(
                raw::RECEIPT,
                "receipt epoch exceeds retained epoch",
            ));
        }
    }
    Ok(())
}
fn last_receipt(
    reader: &impl Read,
    header: raw::Header,
    resource: OwnershipResource,
    key: &[u8],
    expected: GenerationIdentity,
    chunk: Option<ChunkPos>,
) -> OwnershipResult<()> {
    let bytes = reader
        .owned(Table::Receipt, &header.receipt.0)?
        .ok_or_else(|| raw::bad(raw::RECEIPT, "owner last-changing receipt missing"))?;
    let r = raw::receipt(&header.receipt.0, bytes.as_ref())?;
    if r.scope != header.scope || r.identity != expected {
        return Err(raw::bad(
            raw::RECEIPT,
            "owner last receipt scope/identity mismatch",
        ));
    }
    let matches = match resource {
        OwnershipResource::Player => {
            r.player.is_some_and(|id| id.as_uuid().as_bytes() == key)
                && r.after_player == header.stamp
                && r.before_player != r.after_player
        }
        OwnershipResource::Entity => {
            r.entity
                .is_some_and(|id| id.to_bytes().as_slice() == &key[4..])
                && r.after_entity == header.stamp
                && r.before_entity != r.after_entity
                && r.after_chunk == chunk
        }
    };
    if !matches {
        return Err(raw::bad(
            raw::RECEIPT,
            "owner last receipt resource/revision/chunk mismatch",
        ));
    }
    receipt_relations(reader, &r)
}
pub(crate) fn player_state(
    reader: &impl Read,
    key: ScopedPlayerKey,
    expected: GenerationIdentity,
) -> OwnershipResult<State> {
    identity(reader, key.scope, expected)?;
    let k = raw::player_key(key);
    let header = raw_header(reader, OwnershipResource::Player, &k)?;
    let payload = reader.owned(Table::Player, &k)?;
    if let Some(p) = &payload {
        player_bytes(p.as_ref())?;
    }
    let Some(h) = header else {
        if payload.is_some() {
            return Err(raw::bad(raw::PLAYER_OWNER, "player record has no owner"));
        }
        return Ok(State {
            stamp: OwnershipStamp::FRESH,
            chunk: None,
        });
    };
    if h.scope != key.scope {
        return Err(OwnershipError::ScopeConflict {
            resource: OwnershipResource::Player,
        });
    }
    if h.stamp.is_live() != payload.is_some() {
        return Err(raw::bad(
            raw::PLAYER_OWNER,
            "player live/tombstone payload mismatch",
        ));
    }
    last_receipt(reader, h, OwnershipResource::Player, &k, expected, None)?;
    Ok(State {
        stamp: h.stamp,
        chunk: None,
    })
}
pub(crate) fn entity_state(
    reader: &impl Read,
    key: PersistentEntityKey,
    expected: GenerationIdentity,
) -> OwnershipResult<State> {
    let scope = uuid_engine::scope(key);
    identity(reader, scope, expected)?;
    let k = raw::entity_key(key);
    let header = raw_header(reader, OwnershipResource::Entity, &k)?;
    if header.is_some_and(|h| h.scope != scope) {
        return Err(OwnershipError::ScopeConflict {
            resource: OwnershipResource::Entity,
        });
    }
    let payload = uuid_engine::lookup(reader, key)?;
    let Some(h) = header else {
        if payload.is_some() {
            return Err(raw::bad(
                raw::ENTITY_OWNER,
                "entity record/locator has no owner",
            ));
        }
        return Ok(State {
            stamp: OwnershipStamp::FRESH,
            chunk: None,
        });
    };
    if h.stamp.is_live() != payload.is_some() || payload.as_ref().is_some_and(|p| p.key != key) {
        return Err(raw::bad(
            raw::ENTITY_OWNER,
            "entity live/tombstone payload mismatch",
        ));
    }
    let chunk = payload.map(|p| p.chunk.pos());
    last_receipt(reader, h, OwnershipResource::Entity, &k, expected, chunk)?;
    Ok(State {
        stamp: h.stamp,
        chunk,
    })
}
pub(crate) fn load_player(
    reader: &impl Read,
    owner: &Arc<()>,
    key: ScopedPlayerKey,
    expected: GenerationIdentity,
) -> OwnershipResult<OwnedPlayerSnapshot> {
    let state = player_state(reader, key, expected)?;
    let record = reader
        .owned(Table::Player, &raw::player_key(key))?
        .map(|b| crate::codec::decode_player_record(b.as_ref()))
        .transpose()?;
    Ok(OwnedPlayerSnapshot {
        expected: OwnedPlayerExpectation {
            key,
            identity: expected,
            stamp: state.stamp,
            owner: Arc::downgrade(owner),
        },
        record,
    })
}
pub(crate) fn load_entity(
    reader: &impl Read,
    owner: &Arc<()>,
    key: PersistentEntityKey,
    expected: GenerationIdentity,
) -> OwnershipResult<OwnedEntitySnapshot> {
    let state = entity_state(reader, key, expected)?;
    let record = reader
        .get(RawTable::Entity, &uuid_raw::key_bytes(key))?
        .map(|b| uuid_raw::envelope(b.as_ref())?.owned())
        .transpose()?;
    Ok(OwnedEntitySnapshot {
        expected: OwnedEntityExpectation {
            key,
            identity: expected,
            stamp: state.stamp,
            owner: Arc::downgrade(owner),
        },
        record,
    })
}
pub(crate) fn load_pair(
    reader: &impl Read,
    owner: &Arc<()>,
    player: ScopedPlayerKey,
    entity: PersistentEntityKey,
    expected: GenerationIdentity,
) -> OwnershipResult<OwnedPlayerEntitySnapshot> {
    if player.scope != uuid_engine::scope(entity) {
        return Err(OwnershipError::ScopeMismatch);
    }
    Ok(OwnedPlayerEntitySnapshot {
        player: load_player(reader, owner, player, expected)?,
        entity: load_entity(reader, owner, entity, expected)?,
    })
}
pub(crate) fn lookup_receipt(
    reader: &impl Read,
    query: OwnershipReceiptQuery,
) -> OwnershipResult<Option<OwnershipReceipt>> {
    let Some(bytes) = reader.owned(Table::Receipt, &query.id.0)? else {
        return Ok(None);
    };
    let r = raw::receipt(&query.id.0, bytes.as_ref())?;
    if r.query.identity != query.identity {
        return Err(OwnershipError::ReceiptCollision {
            id: query.id,
            expected: query.identity,
            stored: r.query.identity,
        });
    }
    receipt_relations(reader, &r)?;
    Ok(Some(r))
}
pub(crate) fn outcome(
    receipt: OwnershipReceipt,
    owner: &Arc<()>,
    disposition: OwnershipCommitDisposition,
) -> OwnershipCommitOutcome {
    let player = receipt.player.map(|id| OwnedPlayerExpectation {
        key: ScopedPlayerKey::new(receipt.scope, id),
        identity: receipt.identity,
        stamp: receipt.after_player,
        owner: Arc::downgrade(owner),
    });
    let entity = receipt.entity.map(|uuid| OwnedEntityExpectation {
        key: PersistentEntityKey::new(receipt.scope.world(), receipt.scope.dimension(), uuid),
        identity: receipt.identity,
        stamp: receipt.after_entity,
        owner: Arc::downgrade(owner),
    });
    OwnershipCommitOutcome {
        receipt,
        disposition,
        player,
        entity,
    }
}

pub(crate) struct PlayerChange {
    pub(crate) key: [u8; 16],
    pub(crate) record: Option<Vec<u8>>,
    pub(crate) header: raw::Header,
}
pub(crate) struct Plan {
    pub(crate) player: Option<PlayerChange>,
    pub(crate) entity: uuid_engine::Plan,
    pub(crate) entity_header: Option<([u8; 20], raw::Header)>,
    pub(crate) receipt: OwnershipReceipt,
    pub(crate) receipt_count: u64,
    pub(crate) revision_count: u64,
}
pub(crate) enum Prepared {
    Replay(Box<OwnershipReceipt>),
    Commit(Box<Plan>),
}
fn check_cas(
    owner: &Arc<()>,
    expected_owner: &Weak<()>,
    resource: OwnershipResource,
    expected: OwnershipStamp,
    actual: OwnershipStamp,
) -> OwnershipResult<()> {
    if !Weak::ptr_eq(expected_owner, &Arc::downgrade(owner)) || expected_owner.upgrade().is_none() {
        return Err(OwnershipError::BackendOwnerMismatch);
    }
    if expected != actual {
        return Err(OwnershipError::CasConflict {
            resource,
            expected,
            actual,
        });
    }
    Ok(())
}
fn next_stamp(
    resource: OwnershipResource,
    before: OwnershipStamp,
    put: bool,
    changed: bool,
) -> OwnershipResult<OwnershipStamp> {
    if !put && !before.is_live() {
        return Err(OwnershipError::AlreadyAbsent {
            resource,
            stamp: before,
        });
    }
    if !changed {
        return Ok(before);
    }
    let revision = before
        .revision
        .checked_add(1)
        .ok_or(OwnershipError::RevisionExhausted { resource })?;
    OwnershipStamp::checked(if put { 1 } else { 2 }, revision)
}
fn prepare_player(
    reader: &impl Read,
    owner: &Arc<()>,
    expected: &OwnedPlayerExpectation,
    proposal: &PlayerOwnershipProposal,
    receipt: &mut OwnershipReceipt,
) -> OwnershipResult<Option<PlayerChange>> {
    let state = player_state(reader, expected.key, receipt.identity)?;
    check_cas(
        owner,
        &expected.owner,
        OwnershipResource::Player,
        expected.stamp,
        state.stamp,
    )?;
    let key = raw::player_key(expected.key);
    let encoded = proposal
        .record
        .as_ref()
        .map(crate::codec::encode_player_record);
    let old = reader.owned(Table::Player, &key)?;
    let changed = match (&old, &encoded) {
        (Some(a), Some(b)) => a.as_ref() != b.as_slice(),
        (None, None) => false,
        _ => true,
    };
    let after = next_stamp(
        OwnershipResource::Player,
        state.stamp,
        encoded.is_some(),
        changed,
    )?;
    receipt.player = Some(expected.key.id);
    receipt.before_player = state.stamp;
    receipt.after_player = after;
    if changed {
        Ok(Some(PlayerChange {
            key,
            record: encoded,
            header: raw::Header {
                scope: receipt.scope,
                stamp: after,
                receipt: receipt.query.id,
            },
        }))
    } else {
        Ok(None)
    }
}
struct EntityChange {
    plan: uuid_engine::Plan,
    header: Option<([u8; 20], raw::Header)>,
}
fn prepare_entity(
    reader: &impl Read,
    owner: &Arc<()>,
    expected: &OwnedEntityExpectation,
    proposal: EntityOwnershipProposal,
    receipt: &mut OwnershipReceipt,
) -> OwnershipResult<EntityChange> {
    let state = entity_state(reader, expected.key, receipt.identity)?;
    check_cas(
        owner,
        &expected.owner,
        OwnershipResource::Entity,
        expected.stamp,
        state.stamp,
    )?;
    // The existing bounded engine owns all locator/index/epoch preflight, within this reader.
    let put = proposal.record.is_some();
    let after_chunk = proposal.record.as_ref().map(PersistentEntityRecord::chunk);
    let plan = if let Some(record) = proposal.record {
        uuid_engine::save_plan(
            reader,
            uuid_engine::prepare(vec![PersistentEntityWrite::new(
                expected.key,
                receipt.identity,
                record,
            )])?,
        )?
    } else {
        uuid_engine::delete_plan(reader, expected.key, receipt.identity)?.0
    };
    let changed = !plan.changes.is_empty();
    let after = next_stamp(OwnershipResource::Entity, state.stamp, put, changed)?;
    receipt.entity = Some(expected.key.uuid());
    receipt.before_entity = state.stamp;
    receipt.after_entity = after;
    receipt.before_chunk = state.chunk;
    receipt.after_chunk = after_chunk;
    for (chunk, after) in &plan.epochs {
        receipt.epochs.push(OwnershipEpochChange {
            chunk: *chunk,
            before: uuid_engine::epoch(reader, *chunk)?.get(),
            after: *after,
        });
    }
    let header = if changed {
        Some((
            raw::entity_key(expected.key),
            raw::Header {
                scope: receipt.scope,
                stamp: after,
                receipt: receipt.query.id,
            },
        ))
    } else {
        None
    };
    Ok(EntityChange { plan, header })
}
pub(crate) fn prepare(
    reader: &impl Read,
    owner: &Arc<()>,
    capacity: OwnershipCapacity,
    r: Request,
) -> OwnershipResult<Prepared> {
    r.admission(capacity.request)?;
    let query = r.query();
    // An old expectation or full admission history cannot hide an already committed retry.
    if let Some(receipt) = lookup_receipt(reader, query)? {
        return Ok(Prepared::Replay(Box::new(receipt)));
    }
    identity(reader, r.scope, r.identity)?;
    let (receipt_count, revision_count) = reader.counts()?;
    let mut receipt = OwnershipReceipt {
        kind: r.kind,
        query,
        scope: r.scope,
        identity: r.identity,
        player: None,
        entity: None,
        before_player: OwnershipStamp::FRESH,
        after_player: OwnershipStamp::FRESH,
        before_entity: OwnershipStamp::FRESH,
        after_entity: OwnershipStamp::FRESH,
        epochs: Vec::with_capacity(2),
        before_chunk: None,
        after_chunk: None,
    };
    let player_change = r
        .player
        .map(|(expected, proposal)| {
            prepare_player(reader, owner, &expected, &proposal, &mut receipt)
        })
        .transpose()?
        .flatten();
    let EntityChange {
        plan: entity_plan,
        header: entity_header,
    } = match r.entity {
        Some((expected, proposal)) => {
            prepare_entity(reader, owner, &expected, proposal, &mut receipt)?
        }
        None => EntityChange {
            plan: uuid_engine::Plan::default(),
            header: None,
        },
    };
    let additional = u64::from(player_change.is_some() && receipt.before_player.is_fresh())
        + u64::from(entity_header.is_some() && receipt.before_entity.is_fresh());
    if receipt_count >= capacity.receipts {
        return Err(OwnershipError::HistoryCapacity {
            kind: OwnershipHistory::Receipts,
            max: capacity.receipts,
        });
    }
    if revision_count > capacity.revisions {
        return Err(raw::bad("ferrumc:meta", "revision count exceeds capacity"));
    }
    let revision_count =
        revision_count
            .checked_add(additional)
            .ok_or(OwnershipError::HistoryCapacity {
                kind: OwnershipHistory::Revisions,
                max: capacity.revisions,
            })?;
    if revision_count > capacity.revisions {
        return Err(OwnershipError::HistoryCapacity {
            kind: OwnershipHistory::Revisions,
            max: capacity.revisions,
        });
    }
    Ok(Prepared::Commit(Box::new(Plan {
        player: player_change,
        entity: entity_plan,
        entity_header,
        receipt,
        receipt_count: receipt_count + 1,
        revision_count,
    })))
}

/// Read-only UUID compatibility uses the same owner state, including bounded lookahead.
pub(crate) struct EntityAdapter<'a, R: Read>(pub(crate) &'a R);
impl<R: Read> EntityRead for EntityAdapter<'_, R> {
    type Bytes<'a>
        = R::Bytes<'a>
    where
        Self: 'a;
    fn get(
        &self,
        table: RawTable,
        key: &[u8],
    ) -> std::result::Result<Option<Self::Bytes<'_>>, StorageError> {
        if matches!(table, RawTable::Entity) {
            let k = uuid_raw::decode_key(key)?;
            let scope = uuid_engine::scope(k);
            let expected = self
                .0
                .identity(scope)?
                .ok_or(StorageError::MissingGenerationIdentity { key: Some(scope) })?;
            entity_state(self.0, k, expected).map_err(adapter_error)?;
        }
        self.0.get(table, key)
    }
    fn identity(
        &self,
        key: GenerationKey,
    ) -> std::result::Result<Option<GenerationIdentity>, StorageError> {
        self.0.identity(key)
    }
    fn index_rows(
        &self,
        lower: Bound<Vec<u8>>,
        upper: Bound<Vec<u8>>,
    ) -> std::result::Result<RawRows<'_, Self::Bytes<'_>>, StorageError> {
        self.0.index_rows(lower, upper)
    }
}
fn adapter_error(e: OwnershipError) -> StorageError {
    match e {
        OwnershipError::Record(e) => e,
        OwnershipError::MalformedOwnershipData { table, reason } => {
            StorageError::PersistentEntityCorruption { table, reason }
        }
        other => StorageError::backend(other.to_string()),
    }
}
