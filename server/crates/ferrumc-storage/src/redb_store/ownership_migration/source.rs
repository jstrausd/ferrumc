//! Full existing Format5 validation after raw admission, using explicit scratch authority.

use std::ops::Bound;
use std::path::Path;

use redb::{ReadableTable, TableHandle, WriteTransaction};

use super::super::uuid_migration::files::LockedFile;
use super::super::{
    uuid_entities, uuid_validation, ENTITY_TABLE, GENERATION_TABLE, META_FORMAT_KEY, META_TABLE,
    PLAYER_TABLE,
};
use super::{backend, mapping, Error, OwnershipMigrationLimits, Result, TABLES};
use crate::persistent_entity::raw;
use crate::{GenerationKey, OwnershipConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawScan {
    pub(super) counts: [u64; 15],
    pub(super) rows: u64,
    pub(super) bytes: u64,
}

pub(super) fn prescan(
    txn: &WriteTransaction,
    format: u64,
    limits: OwnershipMigrationLimits,
) -> Result<RawScan> {
    let count = if format == 5 { 12 } else { 15 };
    let mut actual = 0;
    let mut metadata_present = false;
    let mut unknown = false;
    for table in txn.list_tables().map_err(|e| backend("catalog", e))? {
        actual += 1;
        if actual > 15 {
            return Err(Error::CatalogMismatch);
        }
        metadata_present |= table.name() == META_TABLE.name();
        if table.name() != META_TABLE.name()
            && !TABLES[..count - 1].iter().any(|t| t.name() == table.name())
        {
            unknown = true;
        }
    }
    if !metadata_present {
        return Err(Error::CatalogMismatch);
    }
    {
        let meta = txn
            .open_table(META_TABLE)
            .map_err(|_| Error::CatalogMismatch)?;
        let version = meta
            .get(META_FORMAT_KEY)
            .map_err(|e| backend("format", e))?
            .map(|v| v.value());
        if version != Some(format) {
            return Err(Error::UnsupportedSourceFormat(version));
        }
    }
    if actual != count
        || unknown
        || txn
            .list_multimap_tables()
            .map_err(|e| backend("multimap catalog", e))?
            .next()
            .is_some()
    {
        return Err(Error::CatalogMismatch);
    }
    let mut scan = RawScan {
        counts: [0; 15],
        rows: 0,
        bytes: 0,
    };
    {
        let meta = txn
            .open_table(META_TABLE)
            .map_err(|_| Error::CatalogMismatch)?;
        let version = meta
            .get(META_FORMAT_KEY)
            .map_err(|e| backend("format", e))?
            .map(|v| v.value());
        if version != Some(format) {
            return Err(Error::UnsupportedSourceFormat(version));
        }
        for row in meta.iter().map_err(|e| backend("meta prescan", e))? {
            let (key, _) = row.map_err(|e| backend("meta prescan row", e))?;
            charge(&mut scan, 0, key.value().len() as u64 + 8, limits)?;
        }
    }
    for (slot, definition) in TABLES[..count - 1].iter().enumerate() {
        let table = txn
            .open_table(*definition)
            .map_err(|_| Error::CatalogMismatch)?;
        for row in table.iter().map_err(|e| backend("raw prescan", e))? {
            let (key, value) = row.map_err(|e| backend("raw prescan row", e))?;
            charge(
                &mut scan,
                slot + 1,
                (key.value().len() as u64)
                    .checked_add(value.value().len() as u64)
                    .ok_or(Error::PlanCapacity)?,
                limits,
            )?;
        }
    }
    Ok(scan)
}

fn charge(
    scan: &mut RawScan,
    slot: usize,
    bytes: u64,
    limits: OwnershipMigrationLimits,
) -> Result<()> {
    scan.rows = scan.rows.checked_add(1).ok_or(Error::PlanCapacity)?;
    scan.bytes = scan.bytes.checked_add(bytes).ok_or(Error::PlanCapacity)?;
    for (kind, required, limit) in [
        ("rows", scan.rows, limits.scan_rows),
        ("bytes", scan.bytes, limits.scan_bytes),
    ] {
        if required > limit {
            return Err(Error::ScanCapacity {
                kind,
                required,
                limit,
            });
        }
    }
    scan.counts[slot] += 1;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Scan {
    pub(super) raw: RawScan,
    pub(super) sha: [u8; 32],
    pub(super) players: u64,
    pub(super) entities: u64,
    pub(super) chunks: u64,
}

pub(super) fn scan(
    txn: &WriteTransaction,
    scratch_anchor: &Path,
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
) -> Result<Scan> {
    let raw = prescan(txn, 5, limits)?;
    let validated = uuid_validation::validate_complete_migration(
        txn,
        scratch_anchor,
        journal_cap(config, limits),
    )?;
    if raw.counts[..12] != validated.counts || raw.bytes != validated.logical_bytes {
        return Err(Error::SourceDrift);
    }
    Ok(Scan {
        raw,
        sha: validated.sha256,
        players: raw.counts[8],
        entities: raw.counts[4],
        chunks: raw.counts[7],
    })
}

pub(super) fn journal_cap(config: OwnershipConfig, limits: OwnershipMigrationLimits) -> u64 {
    config
        .open_limits()
        .max_journal_index_bytes()
        .min(limits.scratch_bytes)
}

pub(super) fn validate_inputs(
    txn: &WriteTransaction,
    mapping_file: &LockedFile,
    original: Scan,
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
) -> Result<()> {
    let resources = original
        .players
        .checked_add(original.entities)
        .ok_or(Error::PlanCapacity)?;
    if resources > limits.plan_rows {
        return Err(Error::PlanCapacity);
    }
    if resources > config.capacity().max_receipts()
        || resources > config.capacity().max_revision_rows()
    {
        return Err(Error::HistoryCapacity);
    }
    // Admission uses original encoded proposal lengths, before record cloning.
    for definition in [PLAYER_TABLE, ENTITY_TABLE] {
        let table = txn
            .open_table(definition)
            .map_err(|e| backend("request preflight table", e))?;
        for row in table.iter().map_err(|e| backend("request preflight", e))? {
            let (_, bytes) = row.map_err(|e| backend("request preflight row", e))?;
            if bytes
                .value()
                .len()
                .checked_add(149)
                .is_none_or(|n| n > config.capacity().max_request_bytes())
            {
                return Err(Error::RequestCapacity);
            }
        }
    }
    validate_mapping(txn, mapping_file, limits)?;
    let epochs = txn
        .open_table(uuid_entities::EPOCH_TABLE)
        .map_err(|e| backend("epoch preflight", e))?;
    let index = txn
        .open_table(uuid_entities::INDEX_TABLE)
        .map_err(|e| backend("count preflight", e))?;
    let mut total = 0u64;
    for row in epochs
        .iter()
        .map_err(|e| backend("epoch preflight iterate", e))?
    {
        let (key, value) = row.map_err(|e| backend("epoch preflight row", e))?;
        let chunk = raw::decode_chunk(key.value(), raw::EPOCH)?;
        let epoch = raw::decode_epoch(value.value())?;
        let count = chunk_count(&index, key.value())?;
        if epoch.checked_add(count).is_none() {
            return Err(Error::EpochExhausted {
                chunk,
                source_epoch: epoch,
                count,
            });
        }
        total = total.checked_add(count).ok_or(Error::PlanCapacity)?;
    }
    if total != original.entities {
        return Err(Error::SourceDrift);
    }
    Ok(())
}

pub(super) fn chunk_count(
    index: &impl ReadableTable<&'static [u8], &'static [u8]>,
    key: &[u8],
) -> Result<u64> {
    let prefix: [u8; 16] = key.try_into().map_err(|_| Error::CatalogMismatch)?;
    let upperraw = raw::prefix_upper(prefix);
    let upper = upperraw
        .as_ref()
        .map_or(Bound::Unbounded, |b| Bound::Excluded(b.as_slice()));
    let mut count = 0u64;
    for row in index
        .range::<&[u8]>((Bound::Included(prefix.as_slice()), upper))
        .map_err(|e| backend("chunk count range", e))?
    {
        row.map_err(|e| backend("chunk count row", e))?;
        count = count.checked_add(1).ok_or(Error::PlanCapacity)?;
    }
    Ok(count)
}

pub(super) fn identity(
    txn: &WriteTransaction,
    scope: GenerationKey,
) -> Result<ferrumc_world::GenerationIdentity> {
    let generations = txn
        .open_table(GENERATION_TABLE)
        .map_err(|e| backend("generation table", e))?;
    let bytes = generations
        .get(scope.encode().as_slice())
        .map_err(|e| backend("generation lookup", e))?
        .ok_or(Error::GenerationMismatch(scope))?;
    Ok(crate::generation::decode_identity(scope, bytes.value())?)
}

fn validate_mapping(
    txn: &WriteTransaction,
    file: &LockedFile,
    limits: OwnershipMigrationLimits,
) -> Result<()> {
    let players = txn
        .open_table(PLAYER_TABLE)
        .map_err(|e| backend("mapping players", e))?;
    let mut mapping = mapping::Reader::open(file, limits)?;
    let mut next = mapping.next()?;
    for row in players
        .iter()
        .map_err(|e| backend("mapping player iterate", e))?
    {
        let (key, _) = row.map_err(|e| backend("mapping player row", e))?;
        let id = ferrumc_core::PlayerId::from_uuid(uuid::Uuid::from_bytes(
            key.value().try_into().map_err(|_| Error::CatalogMismatch)?,
        ));
        let mapped = next.ok_or(Error::MappingMissingPlayer(id))?;
        match mapped
            .player
            .as_uuid()
            .as_bytes()
            .as_slice()
            .cmp(key.value())
        {
            std::cmp::Ordering::Less => return Err(Error::MappingUnusedPlayer(mapped.player)),
            std::cmp::Ordering::Greater => return Err(Error::MappingMissingPlayer(id)),
            std::cmp::Ordering::Equal => {}
        }
        if identity(txn, mapped.scope)? != mapped.identity {
            return Err(Error::GenerationMismatch(mapped.scope));
        }
        next = mapping.next()?;
    }
    if let Some(unused) = next {
        return Err(Error::MappingUnusedPlayer(unused.player));
    }
    Ok(())
}
