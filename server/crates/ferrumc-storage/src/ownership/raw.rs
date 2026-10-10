//! Exact Format6 fixed headers, receipts and bounded streaming request identity.

use ferrumc_core::{EntityUuid, PlayerId};
use ferrumc_math::ChunkPos;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    ChunkKey, GenerationKey, OwnershipEpochChange, OwnershipError, OwnershipReceipt,
    OwnershipReceiptQuery, OwnershipRequestIdentity, OwnershipResult, OwnershipStamp,
    OwnershipTransferId, PersistentEntityKey, Request, ScopedPlayerKey,
};

pub(crate) const PLAYER_OWNER: &str = "ferrumc:player_ownership";
pub(crate) const ENTITY_OWNER: &str = "ferrumc:entity_ownership";
pub(crate) const RECEIPT: &str = "ferrumc:ownership_receipt";

pub(crate) fn bad(table: &'static str, reason: &'static str) -> OwnershipError {
    OwnershipError::MalformedOwnershipData { table, reason }
}
pub(crate) fn array<const N: usize>(bytes: &[u8], table: &'static str) -> OwnershipResult<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| bad(table, "invalid fixed width"))
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Header {
    pub(crate) scope: GenerationKey,
    pub(crate) stamp: OwnershipStamp,
    pub(crate) receipt: OwnershipTransferId,
}
pub(crate) fn player_key(key: ScopedPlayerKey) -> [u8; 16] {
    *key.id.as_uuid().as_bytes()
}
pub(crate) fn entity_key(key: PersistentEntityKey) -> [u8; 20] {
    crate::persistent_entity::raw::locator_key(key)
}
pub(crate) fn encode_player_header(h: Header) -> [u8; 39] {
    let mut b = [0; 39];
    b[..4].copy_from_slice(b"FCOP");
    b[4..6].copy_from_slice(&1u16.to_be_bytes());
    b[6] = h.stamp.tag;
    b[7..15].copy_from_slice(&h.scope.encode());
    b[15..23].copy_from_slice(&h.stamp.revision.to_be_bytes());
    b[23..].copy_from_slice(&h.receipt.0);
    b
}
pub(crate) fn player_header(b: &[u8]) -> OwnershipResult<Header> {
    let b = array::<39>(b, PLAYER_OWNER)?;
    if &b[..4] != b"FCOP" || b[4..6] != 1u16.to_be_bytes() {
        return Err(bad(PLAYER_OWNER, "unknown magic/schema"));
    }
    let stamp =
        OwnershipStamp::checked(b[6], u64::from_be_bytes(array(&b[15..23], PLAYER_OWNER)?))?;
    if stamp.is_fresh() {
        return Err(bad(PLAYER_OWNER, "fresh header is not stored"));
    }
    Ok(Header {
        scope: GenerationKey::decode(&b[7..15])?,
        stamp,
        receipt: OwnershipTransferId(array(&b[23..], PLAYER_OWNER)?),
    })
}
pub(crate) fn encode_entity_header(h: Header) -> [u8; 35] {
    let mut b = [0; 35];
    b[..4].copy_from_slice(b"FCOR");
    b[4..6].copy_from_slice(&1u16.to_be_bytes());
    b[6] = h.stamp.tag;
    b[7..11].copy_from_slice(&h.scope.dimension().get().to_be_bytes());
    b[11..19].copy_from_slice(&h.stamp.revision.to_be_bytes());
    b[19..].copy_from_slice(&h.receipt.0);
    b
}
pub(crate) fn entity_header(key: &[u8], b: &[u8]) -> OwnershipResult<Header> {
    let (world, _) = crate::persistent_entity::raw::decode_locator_key(key)?;
    let b = array::<35>(b, ENTITY_OWNER)?;
    if &b[..4] != b"FCOR" || b[4..6] != 1u16.to_be_bytes() {
        return Err(bad(ENTITY_OWNER, "unknown magic/schema"));
    }
    let stamp =
        OwnershipStamp::checked(b[6], u64::from_be_bytes(array(&b[11..19], ENTITY_OWNER)?))?;
    if stamp.is_fresh() {
        return Err(bad(ENTITY_OWNER, "fresh header is not stored"));
    }
    Ok(Header {
        scope: GenerationKey::new(
            world,
            ferrumc_core::DimensionId::new(u32::from_be_bytes(array(&b[7..11], ENTITY_OWNER)?)),
        ),
        stamp,
        receipt: OwnershipTransferId(array(&b[19..], ENTITY_OWNER)?),
    })
}

pub(crate) fn request_identity(r: &Request) -> OwnershipRequestIdentity {
    let mut h = Sha256::new();
    h.update(b"FerrumC ownership request v1\0");
    h.update([r.kind]);
    h.update(r.id.0);
    h.update(r.scope.encode());
    h.update(r.identity.encode());
    h.update(
        r.player
            .as_ref()
            .map_or([0; 16], |(p, _)| player_key(p.key)),
    );
    h.update(
        r.entity
            .as_ref()
            .map_or([0; 16], |(e, _)| e.key.uuid().to_bytes()),
    );
    h.update(
        r.player
            .as_ref()
            .map_or(OwnershipStamp::FRESH, |(p, _)| p.stamp)
            .encode(),
    );
    h.update(
        r.entity
            .as_ref()
            .map_or(OwnershipStamp::FRESH, |(e, _)| e.stamp)
            .encode(),
    );
    match &r.player {
        None => {
            h.update([0]);
            h.update(0u64.to_be_bytes());
        }
        Some((_, proposal)) => match &proposal.record {
            None => {
                h.update([2]);
                h.update(0u64.to_be_bytes());
            }
            Some(record) => {
                h.update([1]);
                h.update((5 + record.data().len() as u64).to_be_bytes());
                h.update(record.schema_version().get().to_be_bytes());
                h.update([record.game_mode().as_id()]);
                h.update(record.data());
            }
        },
    }
    match &r.entity {
        None => {
            h.update([0]);
            h.update(0u64.to_be_bytes());
        }
        Some((_, proposal)) => match &proposal.record {
            None => {
                h.update([2]);
                h.update(0u64.to_be_bytes());
            }
            Some(record) => {
                h.update([1]);
                h.update((22 + record.record().data().len() as u64).to_be_bytes());
                h.update(b"FCUE");
                h.update(1u16.to_be_bytes());
                h.update(record.chunk().x().to_be_bytes());
                h.update(record.chunk().z().to_be_bytes());
                h.update(record.record().schema_version().get().to_be_bytes());
                h.update((record.record().data().len() as u32).to_be_bytes());
                h.update(record.record().data());
            }
        },
    }
    OwnershipRequestIdentity(h.finalize().into())
}

pub(crate) fn encode_receipt(r: &OwnershipReceipt) -> [u8; 256] {
    let mut b = [0; 256];
    b[..4].copy_from_slice(b"FCTR");
    b[4..6].copy_from_slice(&1u16.to_be_bytes());
    b[6] = r.kind;
    b[7..23].copy_from_slice(&r.query.id.0);
    b[23..55].copy_from_slice(&r.query.identity.0);
    b[55..63].copy_from_slice(&r.scope.encode());
    b[63..90].copy_from_slice(&r.identity.encode());
    if let Some(id) = r.player {
        b[90..106].copy_from_slice(id.as_uuid().as_bytes());
    }
    if let Some(id) = r.entity {
        b[106..122].copy_from_slice(&id.to_bytes());
    }
    b[122..131].copy_from_slice(&r.before_player.encode());
    b[131..140].copy_from_slice(&r.after_player.encode());
    b[140..149].copy_from_slice(&r.before_entity.encode());
    b[149..158].copy_from_slice(&r.after_entity.encode());
    b[158] = r.epochs.len() as u8;
    for (n, e) in r.epochs.iter().enumerate() {
        let start = 159 + n * 32;
        b[start..start + 16].copy_from_slice(&crate::codec::chunk_key_bytes(e.chunk));
        b[start + 16..start + 24].copy_from_slice(&e.before.to_be_bytes());
        b[start + 24..start + 32].copy_from_slice(&e.after.to_be_bytes());
    }
    encode_chunk(&mut b[223..232], r.before_chunk);
    encode_chunk(&mut b[232..241], r.after_chunk);
    b
}
fn encode_chunk(b: &mut [u8], chunk: Option<ChunkPos>) {
    if let Some(c) = chunk {
        b[0] = 1;
        b[1..5].copy_from_slice(&c.x().to_be_bytes());
        b[5..].copy_from_slice(&c.z().to_be_bytes());
    }
}
fn chunk(b: &[u8]) -> OwnershipResult<Option<ChunkPos>> {
    match b[0] {
        0 if b[1..].iter().all(|n| *n == 0) => Ok(None),
        1 => {
            let c = ChunkPos::new(
                i32::from_be_bytes(array(&b[1..5], RECEIPT)?),
                i32::from_be_bytes(array(&b[5..], RECEIPT)?),
            );
            crate::record::validate_chunk_position(c)?;
            Ok(Some(c))
        }
        _ => Err(bad(RECEIPT, "invalid chunk flag/inactive bytes")),
    }
}
fn transition(before: OwnershipStamp, after: OwnershipStamp) -> OwnershipResult<()> {
    let valid = if before.is_fresh() {
        after.tag == 1 && after.revision == 1
    } else if before.is_tombstone() {
        after.tag == 1 && before.revision.checked_add(1) == Some(after.revision)
    } else {
        after.tag == 1 && after.revision == before.revision
            || (after.tag == 1 || after.tag == 2)
                && before.revision.checked_add(1) == Some(after.revision)
    };
    if valid {
        Ok(())
    } else {
        Err(bad(RECEIPT, "invalid resource transition"))
    }
}
fn receipt_epochs(
    b: &[u8; 256],
    scope: GenerationKey,
    before_entity: OwnershipStamp,
    after_entity: OwnershipStamp,
    before_chunk: Option<ChunkPos>,
    after_chunk: Option<ChunkPos>,
) -> OwnershipResult<Vec<OwnershipEpochChange>> {
    let mut epochs = Vec::with_capacity(2);
    for n in 0..2 {
        let start = 159 + n * 32;
        let entry = &b[start..start + 32];
        if n >= usize::from(b[158]) {
            if entry.iter().any(|n| *n != 0) {
                return Err(bad(RECEIPT, "nonzero inactive epoch"));
            }
            continue;
        }
        let chunk = crate::persistent_entity::raw::decode_chunk(&entry[..16], RECEIPT)?;
        let before = u64::from_be_bytes(array(&entry[16..24], RECEIPT)?);
        let after = u64::from_be_bytes(array(&entry[24..], RECEIPT)?);
        if GenerationKey::new(chunk.world(), chunk.dimension()) != scope
            || before.checked_add(1) != Some(after)
            || epochs
                .last()
                .is_some_and(|e: &OwnershipEpochChange| e.chunk >= chunk)
        {
            return Err(bad(RECEIPT, "invalid epoch transition/sort/scope"));
        }
        epochs.push(OwnershipEpochChange {
            chunk,
            before,
            after,
        });
    }
    if before_entity == after_entity {
        if before_chunk != after_chunk || !epochs.is_empty() {
            return Err(bad(RECEIPT, "no-op entity changed chunks/epochs"));
        }
    } else {
        if before_chunk
            .is_some_and(|old| epochs.iter().any(|e| e.chunk.pos() == old && e.before == 0))
        {
            return Err(bad(RECEIPT, "live source chunk had no historical epoch"));
        }
        let mut expected = std::collections::BTreeSet::new();
        for c in [before_chunk, after_chunk].into_iter().flatten() {
            expected.insert(ChunkKey::new(scope.world(), scope.dimension(), c));
        }
        if expected.len() != epochs.len()
            || !expected.into_iter().eq(epochs.iter().map(|e| e.chunk))
        {
            return Err(bad(RECEIPT, "epochs disagree with changed chunk union"));
        }
    }
    Ok(epochs)
}
pub(crate) fn receipt(key: &[u8], bytes: &[u8]) -> OwnershipResult<OwnershipReceipt> {
    let key = array::<16>(key, RECEIPT)?;
    let b = array::<256>(bytes, RECEIPT)?;
    if &b[..4] != b"FCTR" || b[4..6] != 1u16.to_be_bytes() {
        return Err(bad(RECEIPT, "unknown magic/schema"));
    }
    if !(1..=3).contains(&b[6]) || b[7..23] != key || b[241..].iter().any(|n| *n != 0) || b[158] > 2
    {
        return Err(bad(RECEIPT, "invalid kind/ID/reserved/count"));
    }
    let scope = GenerationKey::decode(&b[55..63])?;
    let identity = crate::generation::decode_identity(scope, &b[63..90])?;
    let before_player = OwnershipStamp::decode(&b[122..131])?;
    let after_player = OwnershipStamp::decode(&b[131..140])?;
    let before_entity = OwnershipStamp::decode(&b[140..149])?;
    let after_entity = OwnershipStamp::decode(&b[149..158])?;
    let player = if b[6] == 3 {
        if b[90..106].iter().any(|n| *n != 0)
            || !before_player.is_fresh()
            || !after_player.is_fresh()
        {
            return Err(bad(RECEIPT, "nonzero inactive player"));
        }
        None
    } else {
        transition(before_player, after_player)?;
        Some(PlayerId::from_uuid(Uuid::from_bytes(array(
            &b[90..106],
            RECEIPT,
        )?)))
    };
    let entity = if b[6] == 2 {
        if b[106..122].iter().any(|n| *n != 0)
            || !before_entity.is_fresh()
            || !after_entity.is_fresh()
        {
            return Err(bad(RECEIPT, "nonzero inactive entity"));
        }
        None
    } else {
        transition(before_entity, after_entity)?;
        Some(EntityUuid::from_bytes(array(&b[106..122], RECEIPT)?))
    };
    let before_chunk = chunk(&b[223..232])?;
    let after_chunk = chunk(&b[232..241])?;
    if before_entity.is_live() != before_chunk.is_some()
        || after_entity.is_live() != after_chunk.is_some()
    {
        return Err(bad(RECEIPT, "chunk/state mismatch"));
    }
    let epochs = receipt_epochs(
        &b,
        scope,
        before_entity,
        after_entity,
        before_chunk,
        after_chunk,
    )?;
    Ok(OwnershipReceipt {
        kind: b[6],
        query: OwnershipReceiptQuery {
            id: OwnershipTransferId(key),
            identity: OwnershipRequestIdentity(array(&b[23..55], RECEIPT)?),
        },
        scope,
        identity,
        player,
        entity,
        before_player,
        after_player,
        before_entity,
        after_entity,
        epochs,
        before_chunk,
        after_chunk,
    })
}
