//! Actual Fresh reads and actual Singles CAS; no manufactured bootstrap evidence.

use ferrumc_core::PlayerId;
use redb::{Database, Durability, ReadableTable, WriteTransaction};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use super::super::uuid_migration::files::LockedFile;
use super::super::{
    ownership, ownership_validation, ENTITY_TABLE, META_FORMAT_KEY, META_TABLE, PLAYER_TABLE,
};
use super::{
    backend, index::Index, mapping, plan, source, Cutpoint, Error, Hook, OwnershipMigrationLimits,
    Result, TABLES,
};
use crate::ownership::{engine, Request, Runtime};
use crate::persistent_entity::{engine as entity_engine, raw as entity_raw};
use crate::{
    EntityOwnershipCasRequest, EntityOwnershipProposal, OwnershipCommitDisposition,
    OwnershipConfig, OwnershipTransferId, PlayerOwnershipCasRequest, PlayerOwnershipProposal,
    ScopedPlayerKey,
};

pub(super) fn copy_original(
    source: &Database,
    target: &Database,
    stage: bool,
    config: OwnershipConfig,
    hook: Hook<'_>,
) -> Result<()> {
    let from = source
        .begin_read()
        .map_err(|e| backend("copy original read", e))?;
    let mut to = target
        .begin_write()
        .map_err(|e| backend("copy original write", e))?;
    to.set_durability(Durability::Immediate);
    {
        let original = from
            .open_table(META_TABLE)
            .map_err(|e| backend("copy original metadata", e))?;
        let mut meta = to
            .open_table(META_TABLE)
            .map_err(|e| backend("copy destination metadata", e))?;
        for row in original
            .iter()
            .map_err(|e| backend("copy metadata iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("copy metadata row", e))?;
            meta.insert(key.value(), value.value())
                .map_err(|e| backend("copy metadata insert", e))?;
        }
        if stage {
            for (key, value) in super::metadata(config, 0) {
                meta.insert(key, value)
                    .map_err(|e| backend("initialize Stage agreement", e))?;
            }
            meta.insert(META_FORMAT_KEY, 6)
                .map_err(|e| backend("initialize Stage marker", e))?;
        }
    }
    for (slot, definition) in TABLES.iter().take(if stage { 14 } else { 11 }).enumerate() {
        let mut destination = to
            .open_table(*definition)
            .map_err(|e| backend("copy destination table", e))?;
        if stage && [3, 4, 5, 7, 11, 12, 13].contains(&slot) {
            continue;
        }
        let original = from
            .open_table(*definition)
            .map_err(|e| backend("copy original table", e))?;
        for row in original
            .iter()
            .map_err(|e| backend("copy data iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("copy data row", e))?;
            hook(Cutpoint::BeforePreparationRow)?;
            destination
                .insert(key.value(), value.value())
                .map_err(|e| backend("copy data insert", e))?;
            hook(Cutpoint::AfterPreparationRow)?;
        }
    }
    to.commit()
        .map_err(|e| backend("copy original commit", e))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Each borrowed file/DB identity remains explicit in the offline authority boundary.
pub(super) fn make_plan(
    source: &Database,
    stage: &Database,
    mapping: &LockedFile,
    file: &LockedFile,
    index: &Index,
    header: plan::Header,
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
    owner: &Arc<()>,
    hook: Hook<'_>,
) -> Result<()> {
    plan::write_header(file, header)?;
    let original = source
        .begin_write()
        .map_err(|e| backend("plan source read", e))?;
    let mut map = mapping::Reader::open(mapping, limits)?;
    let mut ordinal = 0;
    for (kind, definition) in [(2, PLAYER_TABLE), (3, ENTITY_TABLE)] {
        let table = original
            .open_table(definition)
            .map_err(|e| backend("plan resources", e))?;
        for entry in table.iter().map_err(|e| backend("plan iterate", e))? {
            let (key, value) = entry.map_err(|e| backend("plan source row", e))?;
            let mut sourcekey = [0; 24];
            sourcekey
                .get_mut(..key.value().len())
                .ok_or(Error::MalformedPlan("original source key extent"))?
                .copy_from_slice(key.value());
            let (scope, identity) = if kind == 2 {
                let row = map.next()?.ok_or(Error::MalformedPlan("mapping ended"))?;
                if row.player.as_uuid().as_bytes().as_slice() != key.value() {
                    return Err(Error::SourceDrift);
                }
                (row.scope, row.identity)
            } else {
                let scope = entity_engine::scope(entity_raw::decode_key(key.value())?);
                (scope, source::identity(&original, scope)?)
            };
            let id = OwnershipTransferId::from_bytes(*Uuid::new_v4().as_bytes());
            index.insert_id(id.to_bytes(), ordinal)?;
            let mut row = plan::Row {
                kind,
                key: sourcekey,
                scope,
                identity,
                id,
                query: [0; 32],
                proposal: Sha256::digest(value.value()).into(),
                proposal_bytes: value.value().len() as u64,
            };
            let request = request(stage, owner, row, value.value(), config)?;
            row.query = request.query().identity().to_bytes();
            hook(Cutpoint::BeforePlanRow)?;
            plan::append(file, ordinal, row)?;
            hook(Cutpoint::AfterPlanRow)?;
            ordinal += 1;
        }
    }
    if ordinal != header.count()? || map.next()?.is_some() {
        return Err(Error::SourceDrift);
    }
    original
        .abort()
        .map_err(|e| backend("abort plan source read", e))?;
    file.sync()?;
    Ok(())
}

fn request(
    stage: &Database,
    owner: &Arc<()>,
    row: plan::Row,
    bytes: &[u8],
    config: OwnershipConfig,
) -> Result<Request> {
    if bytes
        .len()
        .checked_add(149)
        .is_none_or(|n| n > config.capacity().max_request_bytes())
    {
        return Err(Error::RequestCapacity);
    }
    let txn = stage
        .begin_read()
        .map_err(|e| backend("Stage Fresh read", e))?;
    let reader = ownership::read_reader(&txn, config.capacity())?;
    let r = if row.kind == 2 {
        let id = PlayerId::from_uuid(Uuid::from_bytes(
            row.key[..16]
                .try_into()
                .map_err(|_| Error::MalformedPlan("player UUID"))?,
        ));
        let snapshot = engine::load_player(
            &reader,
            owner,
            ScopedPlayerKey::new(row.scope, id),
            row.identity,
        )?;
        if !snapshot.expectation().stamp().is_fresh() || snapshot.record().is_some() {
            return Err(Error::StageOutcomeMismatch);
        }
        PlayerOwnershipCasRequest::new(
            row.id,
            snapshot.expectation().clone(),
            PlayerOwnershipProposal::put(crate::codec::decode_player_record(bytes)?),
        )?
        .0
    } else {
        let key = entity_raw::decode_key(&row.key)?;
        let snapshot = engine::load_entity(&reader, owner, key, row.identity)?;
        if !snapshot.expectation().stamp().is_fresh() || snapshot.record().is_some() {
            return Err(Error::StageOutcomeMismatch);
        }
        EntityOwnershipCasRequest::new(
            row.id,
            snapshot.expectation().clone(),
            EntityOwnershipProposal::put(entity_raw::envelope(bytes)?.owned()?),
        )?
        .0
    };
    r.admission(config.capacity().max_request_bytes())?;
    Ok(r)
}

pub(super) fn bootstrap(
    source: &Database,
    stage: &Database,
    file: &LockedFile,
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
    owner: &Arc<()>,
    hook: Hook<'_>,
) -> Result<()> {
    let runtime = Runtime::new(config);
    let mut plan = plan::Reader::open(file, limits)?;
    let original = source
        .begin_read()
        .map_err(|e| backend("bootstrap original read", e))?;
    while let Some(row) = plan.next()? {
        let definition = if row.kind == 2 {
            PLAYER_TABLE
        } else {
            ENTITY_TABLE
        };
        let table = original
            .open_table(definition)
            .map_err(|e| backend("bootstrap original table", e))?;
        let bytes = table
            .get(if row.kind == 2 {
                &row.key[..16]
            } else {
                &row.key[..]
            })
            .map_err(|e| backend("bootstrap payload lookup", e))?
            .ok_or(Error::SourceDrift)?;
        if row.proposal_bytes != bytes.value().len() as u64
            || row.proposal != Sha256::digest(bytes.value()).as_slice()
        {
            return Err(Error::SourceDrift);
        }
        let r = request(stage, owner, row, bytes.value(), config)?;
        if r.query().identity().to_bytes() != row.query {
            return Err(Error::StageOutcomeMismatch);
        }
        let _permit = runtime.permit()?;
        hook(Cutpoint::BeforeStageCas)?;
        let outcome = ownership::commit(stage, owner, config.capacity(), r, |_| Ok(()))?;
        if outcome.disposition() != OwnershipCommitDisposition::Committed
            || outcome.receipt().query().identity().to_bytes() != row.query
        {
            return Err(Error::StageOutcomeMismatch);
        }
        hook(Cutpoint::AfterStageCas)?;
    }
    Ok(())
}

pub(super) fn target_scan(
    txn: &WriteTransaction,
    anchor: &std::path::Path,
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
) -> Result<(source::RawScan, [u8; 32])> {
    let raw = source::prescan(txn, 6, limits)?;
    let local = crate::OwnershipOpenLimits::new(
        std::num::NonZeroU64::new(config.open_limits().max_scan_rows().min(limits.scan_rows))
            .ok_or(Error::InvalidLimit)?,
        std::num::NonZeroU64::new(config.open_limits().max_scan_bytes().min(limits.scan_bytes))
            .ok_or(Error::InvalidLimit)?,
        std::num::NonZeroU64::new(source::journal_cap(config, limits))
            .ok_or(Error::InvalidLimit)?,
        std::num::NonZeroU16::new(config.open_limits().max_inflight())
            .ok_or(Error::InvalidLimit)?,
    )?;
    let hash =
        ownership_validation::scan(txn, anchor, OwnershipConfig::new(config.capacity(), local))?;
    Ok((raw, hash))
}
