//! Independent original-plus-declared-delta proof, including exact serial epoch receipts.

use super::super::uuid_migration::files::LockedFile;
use super::super::{
    ownership, uuid_entities, ENTITY_TABLE, META_FORMAT_KEY, META_TABLE, PLAYER_TABLE,
};
use super::{
    backend, index::Index, mapping, plan, source, stage, Error, OwnershipMigrationLimits, Result,
    TABLES,
};
use crate::ownership::raw;
use crate::persistent_entity::raw as entity_raw;
use crate::{OwnershipConfig, OwnershipStamp};
use ferrumc_core::PlayerId;
use redb::{Database, ReadableTable, TableHandle, WriteTransaction};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(super) fn forecast(
    scan: source::Scan,
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
) -> Result<(u64, u64)> {
    let n = scan
        .players
        .checked_add(scan.entities)
        .ok_or(Error::PlanCapacity)?;
    let rows = n
        .checked_mul(2)
        .and_then(|d| d.checked_add(5))
        .and_then(|d| d.checked_add(scan.raw.rows))
        .ok_or(Error::PlanCapacity)?;
    let extra = super::metadata(config, n)
        .iter()
        .try_fold(0u64, |sum, (key, _)| {
            sum.checked_add(key.len() as u64 + 8)
                .ok_or(Error::PlanCapacity)
        })?;
    let bytes = n
        .checked_mul(327)
        .and_then(|d| d.checked_add(extra))
        .and_then(|d| d.checked_add(scan.raw.bytes))
        .ok_or(Error::PlanCapacity)?;
    for (kind, required, limit) in [
        (
            "target rows",
            rows,
            limits.scan_rows.min(config.open_limits().max_scan_rows()),
        ),
        (
            "target bytes",
            bytes,
            limits.scan_bytes.min(config.open_limits().max_scan_bytes()),
        ),
    ] {
        if required > limit {
            return Err(Error::ScanCapacity {
                kind,
                required,
                limit,
            });
        }
    }
    Ok((rows, bytes))
}

#[allow(clippy::too_many_arguments)] // Source, target and each immutable evidence role stay independently bound.
pub(super) fn target(
    source: &Database,
    target: &WriteTransaction,
    file: &LockedFile,
    mapfile: &LockedFile,
    index: &Index,
    expected: source::Scan,
    mapping_sha: [u8; 32],
    config: OwnershipConfig,
    limits: OwnershipMigrationLimits,
    anchor: &std::path::Path,
) -> Result<(source::RawScan, [u8; 32])> {
    let original = source
        .begin_write()
        .map_err(|e| backend("delta source proof", e))?;
    if source::scan(&original, anchor, config, limits)? != expected {
        return Err(Error::SourceDrift);
    }
    source::validate_inputs(&original, mapfile, expected, config, limits)?;
    let (targetraw, targethash) = stage::target_scan(target, anchor, config, limits)?;
    let (rows, bytes) = forecast(expected, config, limits)?;
    let n = expected
        .players
        .checked_add(expected.entities)
        .ok_or(Error::PlanCapacity)?;
    let mut expectedcounts = expected.raw.counts;
    expectedcounts[0] += 5;
    expectedcounts[12] = expected.players;
    expectedcounts[13] = expected.entities;
    expectedcounts[14] = n;
    if targetraw.rows != rows || targetraw.bytes != bytes || targetraw.counts != expectedcounts {
        return Err(Error::TargetDeltaMismatch);
    }
    compare_common(&original, target, config, n)?;
    index.reset_counts(&original)?;
    index.clear_ids()?;
    let mut plan = plan::Reader::open(file, limits)?;
    if plan.header
        != (plan::Header {
            source: expected.sha,
            mapping: mapping_sha,
            capacity: plan::capacity_hash(config.capacity()),
            players: expected.players,
            entities: expected.entities,
        })
    {
        return Err(Error::MalformedPlan("binding"));
    }
    let mut mapping = mapping::Reader::open(mapfile, limits)?;
    let mut ordinal = 0;
    while let Some(row) = plan.next()? {
        index.insert_id(row.id.to_bytes(), ordinal)?;
        ordinal += 1;
        let definition = if row.kind == 2 {
            PLAYER_TABLE
        } else {
            ENTITY_TABLE
        };
        let table = original
            .open_table(definition)
            .map_err(|e| backend("delta original resource", e))?;
        let bytes = table
            .get(if row.kind == 2 {
                &row.key[..16]
            } else {
                &row.key[..]
            })
            .map_err(|e| backend("delta original payload", e))?
            .ok_or(Error::TargetDeltaMismatch)?;
        let identity = source::identity(&original, row.scope)?;
        if identity != row.identity
            || row.proposal_bytes != bytes.value().len() as u64
            || row.proposal != Sha256::digest(bytes.value()).as_slice()
            || row.query != fresh_query(row, bytes.value())
        {
            return Err(Error::TargetDeltaMismatch);
        }
        if row.kind == 2 {
            let mapped = mapping.next()?.ok_or(Error::TargetDeltaMismatch)?;
            if mapped.player.as_uuid().as_bytes().as_slice() != &row.key[..16]
                || mapped.scope != row.scope
                || mapped.identity != row.identity
            {
                return Err(Error::TargetDeltaMismatch);
            }
        }
        verify_receipt(target, index, row, bytes.value())?;
    }
    if ordinal != n || mapping.next()?.is_some() {
        return Err(Error::TargetDeltaMismatch);
    }
    index.verify_final(target)?;
    original
        .abort()
        .map_err(|e| backend("abort delta original proof", e))?;
    Ok((targetraw, targethash))
}

fn compare_common(
    source: &WriteTransaction,
    target: &WriteTransaction,
    config: OwnershipConfig,
    n: u64,
) -> Result<()> {
    {
        let from = source
            .open_table(META_TABLE)
            .map_err(|e| backend("original metadata compare", e))?;
        let to = target
            .open_table(META_TABLE)
            .map_err(|e| backend("target metadata compare", e))?;
        for row in from
            .iter()
            .map_err(|e| backend("original metadata iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("original metadata row", e))?;
            let expected = if key.value() == META_FORMAT_KEY {
                6
            } else {
                value.value()
            };
            if to
                .get(key.value())
                .map_err(|e| backend("target metadata lookup", e))?
                .is_none_or(|v| v.value() != expected)
            {
                return Err(Error::TargetDeltaMismatch);
            }
        }
        for (key, value) in super::metadata(config, n) {
            if to
                .get(key)
                .map_err(|e| backend("agreement lookup", e))?
                .is_none_or(|v| v.value() != value)
            {
                return Err(Error::TargetDeltaMismatch);
            }
        }
    }
    for definition in TABLES
        .iter()
        .take(11)
        .filter(|t| t.name() != uuid_entities::EPOCH_TABLE.name())
    {
        let from = source
            .open_table(*definition)
            .map_err(|e| backend("original table compare", e))?;
        let to = target
            .open_table(*definition)
            .map_err(|e| backend("target table compare", e))?;
        let mut from = from
            .iter()
            .map_err(|e| backend("original rows compare", e))?;
        let mut to = to.iter().map_err(|e| backend("target rows compare", e))?;
        loop {
            match (from.next(), to.next()) {
                (None, None) => break,
                (Some(a), Some(b)) => {
                    let (ak, av) = a.map_err(|e| backend("original compare row", e))?;
                    let (bk, bv) = b.map_err(|e| backend("target compare row", e))?;
                    if ak.value() != bk.value() || av.value() != bv.value() {
                        return Err(Error::TargetDeltaMismatch);
                    }
                }
                _ => return Err(Error::TargetDeltaMismatch),
            }
        }
    }
    Ok(())
}

/// Read-only canonical algebra validates the genuine Query recorded before CAS.
/// This computes no expectation, header, receipt or write authority.
fn fresh_query(row: plan::Row, bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"FerrumC ownership request v1\0");
    hash.update([row.kind]);
    hash.update(row.id.to_bytes());
    hash.update(row.scope.encode());
    hash.update(row.identity.encode());
    if row.kind == 2 {
        hash.update(&row.key[..16]);
        hash.update([0; 16]);
    } else {
        hash.update([0; 16]);
        hash.update(&row.key[8..]);
    }
    hash.update([0; 18]);
    if row.kind == 2 {
        hash.update([1]);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
        hash.update([0]);
        hash.update([0; 8]);
    } else {
        hash.update([0]);
        hash.update([0; 8]);
        hash.update([1]);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    hash.finalize().into()
}

fn verify_receipt(
    target: &WriteTransaction,
    index: &Index,
    row: plan::Row,
    proposal: &[u8],
) -> Result<()> {
    let table = target
        .open_table(ownership::RECEIPT)
        .map_err(|e| backend("receipt proof table", e))?;
    let bytes = table
        .get(row.id.to_bytes().as_slice())
        .map_err(|e| backend("receipt proof lookup", e))?
        .ok_or(Error::TargetDeltaMismatch)?;
    let receipt = raw::receipt(&row.id.to_bytes(), bytes.value())?;
    if receipt.kind != row.kind
        || receipt.query.id() != row.id
        || receipt.query.identity().to_bytes() != row.query
        || receipt.scope != row.scope
        || receipt.identity != row.identity
        || receipt.before_player != OwnershipStamp::FRESH
        || receipt.before_entity != OwnershipStamp::FRESH
        || receipt.before_chunk.is_some()
    {
        return Err(Error::TargetDeltaMismatch);
    }
    if row.kind == 2 {
        let id = PlayerId::from_uuid(Uuid::from_bytes(
            row.key[..16]
                .try_into()
                .map_err(|_| Error::TargetDeltaMismatch)?,
        ));
        let headers = target
            .open_table(ownership::PLAYER_OWNER)
            .map_err(|e| backend("player header proof", e))?;
        let bytes = headers
            .get(&row.key[..16])
            .map_err(|e| backend("player header lookup", e))?
            .ok_or(Error::TargetDeltaMismatch)?;
        let header = raw::player_header(bytes.value())?;
        if receipt.player != Some(id)
            || receipt.entity.is_some()
            || !receipt.after_player.is_live()
            || receipt
                .after_player
                .revision()
                .map(crate::OwnershipRevision::get)
                != Some(1)
            || receipt.after_entity != OwnershipStamp::FRESH
            || !receipt.epochs.is_empty()
            || receipt.after_chunk.is_some()
            || header.scope != row.scope
            || header.stamp != receipt.after_player
            || header.receipt != row.id
        {
            return Err(Error::TargetDeltaMismatch);
        }
    } else {
        let key = entity_raw::decode_key(&row.key)?;
        let envelope = entity_raw::envelope(proposal)?;
        let headerkey = raw::entity_key(key);
        let headers = target
            .open_table(ownership::ENTITY_OWNER)
            .map_err(|e| backend("entity header proof", e))?;
        let bytes = headers
            .get(headerkey.as_slice())
            .map_err(|e| backend("entity header lookup", e))?
            .ok_or(Error::TargetDeltaMismatch)?;
        let header = raw::entity_header(&headerkey, bytes.value())?;
        if receipt.player.is_some()
            || receipt.entity != Some(key.uuid())
            || receipt.after_player != OwnershipStamp::FRESH
            || !receipt.after_entity.is_live()
            || receipt
                .after_entity
                .revision()
                .map(crate::OwnershipRevision::get)
                != Some(1)
            || receipt.after_chunk != Some(envelope.chunk)
            || receipt.epochs.len() != 1
            || header.scope != row.scope
            || header.stamp != receipt.after_entity
            || header.receipt != row.id
        {
            return Err(Error::TargetDeltaMismatch);
        }
        let epoch = &receipt.epochs[0];
        let chunk = crate::ChunkKey::new(row.scope.world(), row.scope.dimension(), envelope.chunk);
        if epoch.chunk() != chunk {
            return Err(Error::TargetDeltaMismatch);
        }
        index.advance(
            &crate::codec::chunk_key_bytes(chunk),
            epoch.before(),
            epoch.after(),
        )?;
    }
    Ok(())
}
