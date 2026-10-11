//! Exact fixed-width `FCOPlan1`. Mutable construction precedes immutable seal/CAS.

use super::super::uuid_migration::files::LockedFile;
use super::{io, OwnershipMigrationFileRole as Role, OwnershipMigrationLimits};
use ferrumc_world::GenerationIdentity;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use uuid::{Uuid, Variant, Version};

use super::{Error, Result};
use crate::{GenerationKey, OwnershipCapacity, OwnershipTransferId};

pub(super) const HEADER_BYTES: u64 = 128;
pub(super) const ROW_BYTES: u64 = 148;

pub(super) fn capacity_hash(capacity: OwnershipCapacity) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"FerrumC ownership migration capacity v1\0");
    hash.update((capacity.max_request_bytes() as u64).to_be_bytes());
    hash.update(capacity.max_receipts().to_be_bytes());
    hash.update(capacity.max_revision_rows().to_be_bytes());
    hash.finalize().into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Header {
    pub(super) source: [u8; 32],
    pub(super) mapping: [u8; 32],
    pub(super) capacity: [u8; 32],
    pub(super) players: u64,
    pub(super) entities: u64,
}
impl Header {
    pub(super) fn count(self) -> Result<u64> {
        self.players
            .checked_add(self.entities)
            .ok_or(Error::PlanCapacity)
    }
    pub(super) fn encode(self) -> Result<[u8; HEADER_BYTES as usize]> {
        let mut raw = [0; HEADER_BYTES as usize];
        raw[..4].copy_from_slice(b"FCIP");
        raw[4..6].copy_from_slice(&1u16.to_be_bytes());
        raw[8..16].copy_from_slice(&self.count()?.to_be_bytes());
        raw[16..48].copy_from_slice(&self.source);
        raw[48..80].copy_from_slice(&self.mapping);
        raw[80..112].copy_from_slice(&self.capacity);
        raw[112..120].copy_from_slice(&self.players.to_be_bytes());
        raw[120..128].copy_from_slice(&self.entities.to_be_bytes());
        Ok(raw)
    }
    pub(super) fn decode(raw: &[u8; HEADER_BYTES as usize]) -> Result<Self> {
        if &raw[..4] != b"FCIP" || raw[4..6] != 1u16.to_be_bytes() || raw[6..8] != [0; 2] {
            return Err(Error::MalformedPlan("magic/schema/reserved"));
        }
        let result = Self {
            source: raw[16..48]
                .try_into()
                .map_err(|_| Error::MalformedPlan("header field"))?,
            mapping: raw[48..80]
                .try_into()
                .map_err(|_| Error::MalformedPlan("header field"))?,
            capacity: raw[80..112]
                .try_into()
                .map_err(|_| Error::MalformedPlan("header field"))?,
            players: u64::from_be_bytes(
                raw[112..120]
                    .try_into()
                    .map_err(|_| Error::MalformedPlan("header field"))?,
            ),
            entities: u64::from_be_bytes(
                raw[120..128]
                    .try_into()
                    .map_err(|_| Error::MalformedPlan("header field"))?,
            ),
        };
        if result.count()?
            != u64::from_be_bytes(
                raw[8..16]
                    .try_into()
                    .map_err(|_| Error::MalformedPlan("header field"))?,
            )
        {
            return Err(Error::MalformedPlan("resource count"));
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) kind: u8,
    pub(super) key: [u8; 24],
    pub(super) scope: GenerationKey,
    pub(super) identity: GenerationIdentity,
    pub(super) id: OwnershipTransferId,
    pub(super) query: [u8; 32],
    pub(super) proposal: [u8; 32],
    pub(super) proposal_bytes: u64,
}
impl Row {
    pub(super) fn encode(self) -> [u8; ROW_BYTES as usize] {
        let mut raw = [0; ROW_BYTES as usize];
        raw[0] = self.kind;
        raw[1..25].copy_from_slice(&self.key);
        raw[25..33].copy_from_slice(&self.scope.encode());
        raw[33..60].copy_from_slice(&self.identity.encode());
        raw[60..76].copy_from_slice(&self.id.to_bytes());
        raw[76..108].copy_from_slice(&self.query);
        raw[108..140].copy_from_slice(&self.proposal);
        raw[140..148].copy_from_slice(&self.proposal_bytes.to_be_bytes());
        raw
    }
    pub(super) fn decode(raw: &[u8; ROW_BYTES as usize]) -> Result<Self> {
        let kind = raw[0];
        let key: [u8; 24] = raw[1..25]
            .try_into()
            .map_err(|_| Error::MalformedPlan("row field"))?;
        let scope =
            GenerationKey::decode(&raw[25..33]).map_err(|_| Error::MalformedPlan("scope"))?;
        let identity = crate::generation::decode_identity(scope, &raw[33..60])
            .map_err(|_| Error::MalformedPlan("identity"))?;
        let id = Uuid::from_bytes(
            raw[60..76]
                .try_into()
                .map_err(|_| Error::MalformedPlan("row field"))?,
        );
        if id.get_version() != Some(Version::Random)
            || id.get_variant() != Variant::RFC4122
            || !matches!(kind, 2 | 3)
            || (kind == 2 && key[16..].iter().any(|b| *b != 0))
            || (kind == 3 && key[..8] != scope.encode())
        {
            return Err(Error::MalformedPlan("kind/key/import UUIDv4"));
        }
        let proposal_bytes = u64::from_be_bytes(
            raw[140..148]
                .try_into()
                .map_err(|_| Error::MalformedPlan("row field"))?,
        );
        if proposal_bytes < if kind == 2 { 5 } else { 22 } {
            return Err(Error::MalformedPlan("proposal extent"));
        }
        Ok(Self {
            kind,
            key,
            scope,
            identity,
            id: OwnershipTransferId::from_bytes(*id.as_bytes()),
            query: raw[76..108]
                .try_into()
                .map_err(|_| Error::MalformedPlan("row field"))?,
            proposal: raw[108..140]
                .try_into()
                .map_err(|_| Error::MalformedPlan("row field"))?,
            proposal_bytes,
        })
    }
}

pub(super) fn write_header(file: &LockedFile, header: Header) -> Result<()> {
    file.write_exact(&header.encode()?)?;
    Ok(())
}
pub(super) fn append(file: &LockedFile, ordinal: u64, row: Row) -> Result<()> {
    let offset = ordinal
        .checked_mul(ROW_BYTES)
        .and_then(|n| n.checked_add(HEADER_BYTES))
        .ok_or(Error::PlanCapacity)?;
    let mut writer = file
        .file
        .try_clone()
        .map_err(|e| io("clone plan writer", Role::Plan, e))?;
    if file.len()? != offset {
        return Err(Error::MalformedPlan("append offset"));
    }
    writer
        .seek(SeekFrom::Start(offset))
        .map_err(|e| io("seek plan row", Role::Plan, e))?;
    writer
        .write_all(&row.encode())
        .map_err(|e| io("write plan row", Role::Plan, e))?;
    Ok(())
}
pub(super) struct Reader {
    file: File,
    pub(super) header: Header,
    remaining: u64,
    previous: Option<(u8, [u8; 24])>,
    players: u64,
    entities: u64,
}
impl Reader {
    pub(super) fn open(file: &LockedFile, limits: OwnershipMigrationLimits) -> Result<Self> {
        let mut reader = file
            .file
            .try_clone()
            .map_err(|e| io("clone plan reader", Role::Plan, e))?;
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| io("seek plan", Role::Plan, e))?;
        let mut raw = [0; HEADER_BYTES as usize];
        reader
            .read_exact(&mut raw)
            .map_err(|e| io("read plan header", Role::Plan, e))?;
        let header = Header::decode(&raw)?;
        let count = header.count()?;
        if count > limits.plan_rows {
            return Err(Error::PlanCapacity);
        }
        let expected = count
            .checked_mul(ROW_BYTES)
            .and_then(|n| n.checked_add(HEADER_BYTES))
            .ok_or(Error::PlanCapacity)?;
        if expected > limits.scratch_bytes {
            return Err(Error::ScratchCapacity);
        }
        if file.len()? != expected {
            return Err(Error::MalformedPlan("count/extent"));
        }
        Ok(Self {
            file: reader,
            header,
            remaining: count,
            previous: None,
            players: 0,
            entities: 0,
        })
    }
    pub(super) fn next(&mut self) -> Result<Option<Row>> {
        if self.remaining == 0 {
            if self.players != self.header.players || self.entities != self.header.entities {
                return Err(Error::MalformedPlan("kind counts"));
            }
            return Ok(None);
        }
        let mut bytes = [0; ROW_BYTES as usize];
        self.file
            .read_exact(&mut bytes)
            .map_err(|e| io("read plan row", Role::Plan, e))?;
        let row = Row::decode(&bytes)?;
        if self
            .previous
            .is_some_and(|prev| prev >= (row.kind, row.key))
        {
            return Err(Error::MalformedPlan("resource order/duplicate"));
        }
        self.previous = Some((row.kind, row.key));
        self.remaining -= 1;
        if row.kind == 2 {
            self.players += 1;
        } else {
            self.entities += 1;
        }
        Ok(Some(row))
    }
}
