//! Streaming exact `FCOMapping1`: fixed positive caps precede every owned row.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use ferrumc_core::PlayerId;
use ferrumc_world::GenerationIdentity;
use uuid::Uuid;

use super::super::uuid_migration::files::LockedFile;
use super::{io, Error, OwnershipMigrationFileRole as Role, OwnershipMigrationLimits, Result};
use crate::GenerationKey;

pub(super) const HEADER_BYTES: u64 = 64;
pub(super) const ROW_BYTES: u64 = 51;

#[derive(Debug, Clone, Copy)]
pub(super) struct Row {
    pub(super) player: PlayerId,
    pub(super) scope: GenerationKey,
    pub(super) identity: GenerationIdentity,
}

pub(super) struct Reader {
    file: File,
    extent: u64,
    remaining: u64,
    previous: Option<[u8; 16]>,
}

impl Reader {
    pub(super) fn open(file: &LockedFile, limits: OwnershipMigrationLimits) -> Result<Self> {
        file.verify_single_link()?;
        let len = file.len()?;
        if len < HEADER_BYTES {
            return Err(Error::MalformedMapping("truncated header"));
        }
        let mut reader = file
            .file
            .try_clone()
            .map_err(|e| io("clone mapping reader", Role::Mapping, e))?;
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| io("seek mapping", Role::Mapping, e))?;
        let mut header = [0; HEADER_BYTES as usize];
        reader
            .read_exact(&mut header)
            .map_err(|e| io("read mapping header", Role::Mapping, e))?;
        if &header[..4] != b"FCOM"
            || header[4..6] != 1u16.to_be_bytes()
            || header[6..8] != [0; 2]
            || header[16..].iter().any(|b| *b != 0)
        {
            return Err(Error::MalformedMapping("magic/schema/reserved"));
        }
        let count = u64::from_be_bytes(
            header[8..16]
                .try_into()
                .map_err(|_| Error::MalformedMapping("header field"))?,
        );
        if count > limits.mapping_rows {
            return Err(Error::ScanCapacity {
                kind: "mapping rows",
                required: count,
                limit: limits.mapping_rows,
            });
        }
        if count
            .checked_mul(ROW_BYTES)
            .and_then(|n| n.checked_add(HEADER_BYTES))
            != Some(len)
        {
            return Err(Error::MalformedMapping("row count/extent/trailing bytes"));
        }
        Ok(Self {
            file: reader,
            extent: len,
            remaining: count,
            previous: None,
        })
    }

    pub(super) const fn extent(&self) -> u64 {
        self.extent
    }

    pub(super) fn next(&mut self) -> Result<Option<Row>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        let mut raw = [0; ROW_BYTES as usize];
        self.file
            .read_exact(&mut raw)
            .map_err(|e| io("read mapping row", Role::Mapping, e))?;
        let player_bytes: [u8; 16] = raw[..16]
            .try_into()
            .map_err(|_| Error::MalformedMapping("player UUID field"))?;
        if let Some(previous) = self.previous {
            if previous == player_bytes {
                return Err(Error::MappingDuplicate);
            }
            if previous > player_bytes {
                return Err(Error::MappingUnsorted);
            }
        }
        let scope =
            GenerationKey::decode(&raw[16..24]).map_err(|_| Error::MalformedMapping("scope"))?;
        let identity = crate::generation::decode_identity(scope, &raw[24..])
            .map_err(|_| Error::MalformedMapping("generation identity"))?;
        self.previous = Some(player_bytes);
        self.remaining -= 1;
        Ok(Some(Row {
            player: PlayerId::from_uuid(Uuid::from_bytes(player_bytes)),
            scope,
            identity,
        }))
    }
}
