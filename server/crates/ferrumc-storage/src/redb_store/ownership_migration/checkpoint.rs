//! Exact FCMC1 checkpoint; input paths never become filesystem authority.

use redb::Database;
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use uuid::{Uuid, Variant, Version};

use super::super::uuid_migration::files::{FileIdentity, LockedFile};
use super::{
    io, paths, source, Error, OwnershipMigrationFileRole as Role, OwnershipMigrationLimits, Result,
};
use crate::OwnershipConfig;

const HEADER: usize = 376;
const MAXIMUM: usize = 21_188;
const CONTRACT: [u8; 32] = [
    0x4e, 0x66, 0xfd, 0x1b, 0x7b, 0xac, 0x1a, 0x9b, 0x00, 0xfc, 0x66, 0xc7, 0xb6, 0xc1, 0x76, 0x53,
    0xbc, 0x77, 0xec, 0x69, 0x22, 0xcb, 0x20, 0xc6, 0x60, 0x93, 0x02, 0xf9, 0xf4, 0x0d, 0x73, 0x77,
];
const REFERENCE: [u8; 32] = [
    0x64, 0x6f, 0xf5, 0x1b, 0x92, 0x93, 0x85, 0x8e, 0x76, 0x84, 0x9e, 0x3e, 0x0d, 0x7f, 0x38, 0xbc,
    0x37, 0x90, 0x17, 0x69, 0x35, 0xc1, 0xf8, 0xa2, 0xe9, 0x93, 0x04, 0x27, 0x0f, 0xa0, 0xce, 0x51,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Evidence {
    pub(super) path: PathBuf,
    pub(super) identity: FileIdentity,
    pub(super) physical: (u64, [u8; 32]),
}
impl Evidence {
    pub(super) fn capture(file: &LockedFile, closed: bool, limit_len: u64) -> Result<Self> {
        file.verify_single_link()?;
        let len = file.len()?;
        if closed && len > limit_len {
            return Err(Error::ScanCapacity {
                kind: "closed evidence bytes",
                required: len,
                limit: limit_len,
            });
        }
        let physical = if closed {
            file.hash_expected_len(len)?
        } else {
            (len, [0; 32])
        };
        Ok(Self {
            path: file.path.clone(),
            identity: file.identity,
            physical,
        })
    }
    pub(super) fn verify(&self, file: &LockedFile, role: Role, closed: bool) -> Result<()> {
        file.verify_single_link()
            .map_err(|_| Error::FileReplaced(role))?;
        if file.path != self.path || file.identity != self.identity {
            return Err(Error::FileReplaced(role));
        }
        if closed {
            let physical =
                file.hash_expected_len(self.physical.0)
                    .map_err(|error| match error {
                        super::super::uuid_migration::UuidEntityMigrationError::BackupMismatch => {
                            Error::EvidenceDrift(role)
                        }
                        other => other.into(),
                    })?;
            if physical != self.physical {
                return Err(Error::EvidenceDrift(role));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Record {
    pub(super) id: [u8; 16],
    pub(super) source: [u8; 32],
    pub(super) target: [u8; 32],
    pub(super) mapping: [u8; 32],
    pub(super) plan: [u8; 32],
    pub(super) counts: [u64; 9],
    pub(super) evidence: [Evidence; 5],
    pub(super) limits: OwnershipMigrationLimits,
    pub(super) capacity: [u64; 3],
}
pub(super) fn capacity(config: OwnershipConfig) -> [u64; 3] {
    let c = config.capacity();
    [
        c.max_request_bytes() as u64,
        c.max_receipts(),
        c.max_revision_rows(),
    ]
}
pub(super) fn extent(
    paths: &[&std::path::Path; 5],
    limits: OwnershipMigrationLimits,
) -> Result<usize> {
    let mut len = HEADER + 32;
    for path in paths {
        len = len
            .checked_add(60 + paths::path_bytes(path)?.len())
            .ok_or(Error::CheckpointCapacity)?;
    }
    if len > MAXIMUM || len > limits.checkpoint_bytes {
        return Err(Error::CheckpointCapacity);
    }
    Ok(len)
}
impl Record {
    pub(super) fn validate_shape(&self, config: OwnershipConfig) -> Result<()> {
        if self.capacity != capacity(config) {
            return Err(Error::MalformedCheckpoint("capacity agreement"));
        }
        let n = self.counts[4]
            .checked_add(self.counts[5])
            .ok_or(Error::PlanCapacity)?;
        let rows = n
            .checked_mul(2)
            .and_then(|delta| delta.checked_add(5))
            .and_then(|delta| self.counts[0].checked_add(delta))
            .ok_or(Error::PlanCapacity)?;
        let meta = super::metadata(config, n)
            .iter()
            .try_fold(0u64, |sum, (key, _)| {
                sum.checked_add(key.len() as u64 + 8)
                    .ok_or(Error::PlanCapacity)
            })?;
        let bytes = n
            .checked_mul(327)
            .and_then(|delta| delta.checked_add(meta))
            .and_then(|delta| self.counts[1].checked_add(delta))
            .ok_or(Error::PlanCapacity)?;
        if n > self.limits.plan_rows
            || self.counts[4] > self.limits.mapping_rows
            || n > config.capacity().max_receipts()
            || n > config.capacity().max_revision_rows()
            || self.counts[7] != n
            || self.counts[8] != n
            || self.counts[2] != rows
            || self.counts[3] != bytes
            || rows > self.limits.scan_rows
            || bytes > self.limits.scan_bytes
        {
            return Err(Error::MalformedCheckpoint("counts/target delta"));
        }
        if self.evidence[1].physical.0
            != super::mapping::HEADER_BYTES + self.counts[4] * super::mapping::ROW_BYTES
            || self.evidence[3].physical.0 != super::plan::HEADER_BYTES + n * super::plan::ROW_BYTES
        {
            return Err(Error::MalformedCheckpoint("mapping/plan extent"));
        }
        for index in [2, 4] {
            if self.evidence[index].physical.0 > self.limits.verification_copy_bytes {
                return Err(Error::VerificationCopyCapacity);
            }
        }
        if self.evidence[3].physical.0 > self.limits.scratch_bytes {
            return Err(Error::ScratchCapacity);
        }
        Ok(())
    }
    pub(super) fn encode(&self) -> Result<Vec<u8>> {
        let len = extent(
            &std::array::from_fn(|i| self.evidence[i].path.as_path()),
            self.limits,
        )?;
        let mut bytes = vec![0; HEADER];
        bytes.reserve_exact(len - HEADER);
        bytes[..4].copy_from_slice(b"FCMC");
        bytes[4..6].copy_from_slice(&1u16.to_be_bytes());
        bytes[6] = 5;
        bytes[7] = 6;
        bytes[8..24].copy_from_slice(&self.id);
        bytes[24..56].copy_from_slice(&self.source);
        bytes[56..88].copy_from_slice(&self.target);
        bytes[88..120].copy_from_slice(&self.mapping);
        bytes[120..152].copy_from_slice(&self.plan);
        for (i, n) in self.capacity.iter().enumerate() {
            bytes[152 + i * 8..160 + i * 8].copy_from_slice(&n.to_be_bytes());
        }
        for (i, n) in self.limits.encoded().iter().enumerate() {
            bytes[176 + i * 8..184 + i * 8].copy_from_slice(&n.to_be_bytes());
        }
        for (i, n) in self.counts.iter().enumerate() {
            bytes[232 + i * 8..240 + i * 8].copy_from_slice(&n.to_be_bytes());
        }
        bytes[304..336].copy_from_slice(&CONTRACT);
        bytes[336..368].copy_from_slice(&REFERENCE);
        bytes[368..370].copy_from_slice(&5u16.to_be_bytes());
        for (i, e) in self.evidence.iter().enumerate() {
            let path = paths::path_bytes(&e.path)?;
            bytes.push(i as u8 + 1);
            bytes.push(u8::from(i != 0));
            bytes.extend_from_slice(&(path.len() as u16).to_be_bytes());
            bytes.extend_from_slice(&e.identity.device.to_be_bytes());
            bytes.extend_from_slice(&e.identity.inode.to_be_bytes());
            bytes.extend_from_slice(&e.physical.0.to_be_bytes());
            bytes.extend_from_slice(&e.physical.1);
            bytes.extend_from_slice(path);
        }
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        bytes.extend_from_slice(&digest);
        Ok(bytes)
    }
    pub(super) fn read(file: &LockedFile, limits: OwnershipMigrationLimits) -> Result<Self> {
        let len = usize::try_from(file.len()?).map_err(|_| Error::CheckpointCapacity)?;
        if len > limits.checkpoint_bytes || len > MAXIMUM {
            return Err(Error::CheckpointCapacity);
        }
        if len < HEADER + 5 * 60 + 32 {
            return Err(Error::MalformedCheckpoint("truncated"));
        }
        let mut input = file
            .file
            .try_clone()
            .map_err(|e| io("clone checkpoint reader", Role::Checkpoint, e))?;
        input
            .seek(SeekFrom::Start(0))
            .map_err(|e| io("seek checkpoint", Role::Checkpoint, e))?;
        let mut bytes = vec![0; len];
        input
            .read_exact(&mut bytes)
            .map_err(|e| io("read checkpoint", Role::Checkpoint, e))?;
        Self::decode(&bytes, limits)
    }
    #[allow(clippy::too_many_lines)] // Keep the exact fixed-offset grammar and all refusal checks adjacent.
    fn decode(bytes: &[u8], limits: OwnershipMigrationLimits) -> Result<Self> {
        if bytes.len() < HEADER + 5 * 60 + 32
            || bytes.len() > limits.checkpoint_bytes
            || bytes.len() > MAXIMUM
        {
            return Err(Error::MalformedCheckpoint("extent"));
        }
        let footer = bytes.len() - 32;
        if &bytes[..4] != b"FCMC"
            || bytes[4..6] != 1u16.to_be_bytes()
            || bytes[6..8] != [5, 6]
            || bytes[304..336] != CONTRACT
            || bytes[336..368] != REFERENCE
            || bytes[368..370] != 5u16.to_be_bytes()
            || bytes[370..376] != [0; 6]
            || Sha256::digest(&bytes[..footer]).as_slice() != &bytes[footer..]
        {
            return Err(Error::MalformedCheckpoint(
                "version/contracts/reserved/checksum",
            ));
        }
        let id = array(&bytes[8..24])?;
        let uuid = Uuid::from_bytes(id);
        if uuid.get_version() != Some(Version::Random) || uuid.get_variant() != Variant::RFC4122 {
            return Err(Error::MalformedCheckpoint("checkpoint UUIDv4"));
        }
        let mut encoded = [0; 7];
        for (i, n) in encoded.iter_mut().enumerate() {
            *n = u64::from_be_bytes(array(&bytes[176 + i * 8..184 + i * 8])?);
        }
        let stored_limits = OwnershipMigrationLimits::from_encoded(encoded)?;
        if stored_limits != limits {
            return Err(Error::MalformedCheckpoint("limit agreement"));
        }
        let mut capacities = [0; 3];
        for (i, n) in capacities.iter_mut().enumerate() {
            *n = u64::from_be_bytes(array(&bytes[152 + i * 8..160 + i * 8])?);
        }
        let mut counts = [0; 9];
        for (i, n) in counts.iter_mut().enumerate() {
            *n = u64::from_be_bytes(array(&bytes[232 + i * 8..240 + i * 8])?);
        }
        let mut evidence = Vec::with_capacity(5);
        let mut offset = HEADER;
        for i in 0..5 {
            let raw = bytes
                .get(offset..offset + 60)
                .filter(|_| offset + 60 <= footer)
                .ok_or(Error::MalformedCheckpoint("file record"))?;
            if raw[0] != i + 1 || raw[1] != u8::from(i != 0) {
                return Err(Error::MalformedCheckpoint("file role/flags"));
            }
            let pathlen = u16::from_be_bytes(array(&raw[2..4])?) as usize;
            if pathlen == 0 || pathlen > 4096 {
                return Err(Error::MalformedCheckpoint("path length"));
            }
            let pathbytes = bytes
                .get(offset + 60..offset + 60 + pathlen)
                .filter(|_| offset + 60 + pathlen <= footer)
                .ok_or(Error::MalformedCheckpoint("truncated path"))?;
            if pathbytes.contains(&0) {
                return Err(Error::MalformedCheckpoint("path NUL"));
            }
            #[cfg(unix)]
            let path = {
                use std::os::unix::ffi::OsStrExt;
                PathBuf::from(std::ffi::OsStr::from_bytes(pathbytes))
            };
            #[cfg(not(unix))]
            let path = {
                let _ = pathbytes;
                return Err(Error::UnsupportedPlatform);
            };
            if !path.is_absolute()
                || path.components().any(|c| {
                    matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )
                })
            {
                return Err(Error::MalformedCheckpoint("noncanonical path"));
            }
            let physical = (
                u64::from_be_bytes(array(&raw[20..28])?),
                array(&raw[28..60])?,
            );
            if i == 0 && physical.1 != [0; 32] {
                return Err(Error::MalformedCheckpoint("Source physical hash"));
            }
            evidence.push(Evidence {
                path,
                identity: FileIdentity {
                    device: u64::from_be_bytes(array(&raw[4..12])?),
                    inode: u64::from_be_bytes(array(&raw[12..20])?),
                },
                physical,
            });
            offset += 60 + pathlen;
        }
        if offset != footer {
            return Err(Error::MalformedCheckpoint("trailing bytes"));
        }
        Ok(Self {
            id,
            source: array(&bytes[24..56])?,
            target: array(&bytes[56..88])?,
            mapping: array(&bytes[88..120])?,
            plan: array(&bytes[120..152])?,
            counts,
            evidence: evidence
                .try_into()
                .map_err(|_| Error::MalformedCheckpoint("file count"))?,
            limits: stored_limits,
            capacity: capacities,
        })
    }
    pub(super) fn source_scan(
        &self,
        db: &Database,
        scratch_anchor: &std::path::Path,
        config: OwnershipConfig,
    ) -> Result<source::Scan> {
        let txn = db
            .begin_write()
            .map_err(|e| super::backend("resume source scan", e))?;
        let scan = source::scan(&txn, scratch_anchor, config, self.limits)?;
        if scan.sha != self.source
            || scan.raw.rows != self.counts[0]
            || scan.raw.bytes != self.counts[1]
            || scan.players != self.counts[4]
            || scan.entities != self.counts[5]
            || scan.chunks != self.counts[6]
        {
            return Err(Error::SourceDrift);
        }
        txn.abort()
            .map_err(|e| super::backend("abort source scan", e))?;
        Ok(scan)
    }
}
fn array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| Error::MalformedCheckpoint("fixed field"))
}
