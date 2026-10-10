//! Immutable344-byte checkpoints, atomic no-clobber publication and alias recovery.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::files::{self, FileIdentity, LockedFile};
use super::{
    io_error, validation, BackedUpUuidEntityMigration, CheckpointedUuidEntityMigration,
    MigrationResult, Observer, Stage, UuidEntityMigrationError as Error,
};

pub(super) const LENGTH: usize = 344;
const BODY: usize = 312;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Data {
    pub(super) source: FileIdentity,
    pub(super) source_file_len: u64,
    pub(super) backup: FileIdentity,
    pub(super) backup_len: u64,
    pub(super) backup_hash: [u8; 32],
    pub(super) scan: validation::Scan,
    identity: FileIdentity,
    staging: Uuid,
}

fn number(bytes: &[u8; LENGTH], offset: usize) -> u64 {
    u64::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

impl Data {
    fn encode(&self) -> [u8; LENGTH] {
        let mut bytes = [0; LENGTH];
        bytes[..8].copy_from_slice(b"FCUMIG01");
        bytes[8..16].copy_from_slice(&[0, 1, 0, 1, 0, 1, 0, 0]);
        for (offset, value) in [
            (16, 4),
            (24, 5),
            (32, self.source.device),
            (40, self.source.inode),
            (48, self.source_file_len),
            (56, self.backup.device),
            (64, self.backup.inode),
            (72, self.backup_len),
            (208, self.scan.logical_bytes),
            (280, self.identity.device),
            (288, self.identity.inode),
        ] {
            bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
        }
        bytes[80..112].copy_from_slice(&self.backup_hash);
        for (slot, count) in self.scan.counts.iter().enumerate() {
            let offset = 112 + slot * 8;
            bytes[offset..offset + 8].copy_from_slice(&count.to_be_bytes());
        }
        bytes[216..248].copy_from_slice(&self.scan.sha256);
        bytes[248..280].copy_from_slice(&self.scan.target_sha256);
        bytes[296..312].copy_from_slice(self.staging.as_bytes());
        let checksum: [u8; 32] = Sha256::digest(&bytes[..BODY]).into();
        bytes[BODY..].copy_from_slice(&checksum);
        bytes
    }
    pub(super) fn decode(input: &[u8]) -> MigrationResult<Self> {
        let bytes: &[u8; LENGTH] = input
            .try_into()
            .map_err(|_| Error::CheckpointMalformed("length"))?;
        if &bytes[..8] != b"FCUMIG01" {
            return Err(Error::CheckpointMalformed("magic"));
        }
        if bytes[8..16] != [0, 1, 0, 1, 0, 1, 0, 0] {
            return Err(Error::CheckpointMalformed("schema/reserved"));
        }
        let checksum: [u8; 32] = Sha256::digest(&bytes[..BODY]).into();
        if bytes[BODY..] != checksum {
            return Err(Error::CheckpointMalformed("checksum"));
        }
        if number(bytes, 16) != 4 || number(bytes, 24) != 5 {
            return Err(Error::CheckpointMalformed("format delta"));
        }
        let mut counts = [0u64; 12];
        for (slot, count) in counts.iter_mut().enumerate() {
            *count = number(bytes, 112 + slot * 8);
        }
        if !(1..=2).contains(&counts[0])
            || counts[1] > crate::MAX_GENERATION_IDENTITIES as u64
            || counts[4..8] != [0; 4]
        {
            return Err(Error::CheckpointMalformed("row counts"));
        }
        if counts
            .iter()
            .try_fold(0u64, |sum, count| sum.checked_add(*count))
            .is_none()
        {
            return Err(Error::CheckpointMalformed("row counter overflow"));
        }
        if number(bytes, 72) == 0 {
            return Err(Error::CheckpointMalformed("empty backup"));
        }
        let source = FileIdentity {
            device: number(bytes, 32),
            inode: number(bytes, 40),
        };
        let backup = FileIdentity {
            device: number(bytes, 56),
            inode: number(bytes, 64),
        };
        let identity = FileIdentity {
            device: number(bytes, 280),
            inode: number(bytes, 288),
        };
        if source == backup || source == identity || backup == identity {
            return Err(Error::CheckpointMalformed("role identity alias"));
        }
        let mut backup_hash = [0; 32];
        backup_hash.copy_from_slice(&bytes[80..112]);
        let mut sha256 = [0; 32];
        sha256.copy_from_slice(&bytes[216..248]);
        let mut target_sha256 = [0; 32];
        target_sha256.copy_from_slice(&bytes[248..280]);
        let mut uuid = [0; 16];
        uuid.copy_from_slice(&bytes[296..312]);
        Ok(Self {
            source,
            source_file_len: number(bytes, 48),
            backup,
            backup_len: number(bytes, 72),
            backup_hash,
            scan: validation::Scan {
                format: 4,
                counts,
                logical_bytes: number(bytes, 208),
                sha256,
                target_sha256,
            },
            identity,
            staging: Uuid::from_bytes(uuid),
        })
    }
    fn staging_path(&self, parent: &Path) -> PathBuf {
        parent.join(format!(".ferrumc-uuid-checkpoint-{}.tmp", self.staging))
    }
}

#[derive(Debug)]
pub(super) struct Authority {
    file: LockedFile,
    pub(super) data: Data,
    bytes: [u8; LENGTH],
    staging_path: PathBuf,
}

impl Authority {
    pub(super) fn path(&self) -> &Path {
        &self.file.path
    }
    pub(super) fn identity(&self) -> FileIdentity {
        self.file.identity
    }
    pub(super) fn hash(&self) -> [u8; 32] {
        Sha256::digest(self.bytes).into()
    }
    pub(super) fn load(path: &Path) -> MigrationResult<Self> {
        let file = LockedFile::open(path, false, true)?;
        let bytes = file.read_exact::<LENGTH>()?;
        let data = Data::decode(&bytes)?;
        if data.identity != file.identity {
            return Err(Error::CheckpointChanged);
        }
        let parent = file
            .path
            .parent()
            .ok_or_else(|| Error::InvalidFile(file.path.clone()))?;
        let staging_path = data.staging_path(parent);
        let authority = Self {
            file,
            data,
            bytes,
            staging_path,
        };
        authority.verify(false)?;
        Ok(authority)
    }
    pub(super) fn verify(&self, require_single: bool) -> MigrationResult<()> {
        self.file.verify_path()?;
        if self.file.read_exact::<LENGTH>()? != self.bytes
            || self.file.identity != self.data.identity
        {
            return Err(Error::CheckpointChanged);
        }
        if self.file.path == self.staging_path {
            return Err(Error::CheckpointStagingChanged(self.staging_path.clone()));
        }
        let links = files::nlink(
            &self
                .file
                .file
                .metadata()
                .map_err(|e| io_error("checkpoint fstat", &self.file.path, e))?,
        )?;
        let stage = match fs::symlink_metadata(&self.staging_path) {
            Ok(meta) => Some(meta),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(io_error("checkpoint staging stat", &self.staging_path, e)),
        };
        match (links, stage) {
            (1, None) => Ok(()),
            (2, Some(meta))
                if !require_single
                    && meta.is_file()
                    && files::FileIdentity::of(&meta)? == self.data.identity =>
            {
                Ok(())
            }
            (1 | 2, Some(_)) => Err(Error::CheckpointStagingChanged(self.staging_path.clone())),
            _ => Err(Error::UnexpectedFileAlias(self.file.path.clone())),
        }
    }
    pub(super) fn stabilize(&self, observe: Observer<'_>) -> MigrationResult<()> {
        self.verify(false)?;
        self.file
            .file
            .sync_all()
            .map_err(|e| io_error("fsync checkpoint", &self.file.path, e))?;
        observe(Stage::CheckpointFileSynced)?;
        files::sync_parent(&self.file.path)?;
        observe(Stage::CheckpointDirectorySynced)?;
        if files::nlink(
            &self
                .file
                .file
                .metadata()
                .map_err(|e| io_error("checkpoint links", &self.file.path, e))?,
        )? == 2
        {
            self.verify(false)?;
            fs::remove_file(&self.staging_path)
                .map_err(|e| io_error("unlink checkpoint staging", &self.staging_path, e))?;
            observe(Stage::CheckpointStagingUnlinked)?;
        }
        files::sync_parent(&self.file.path)?;
        self.verify(true)?;
        observe(Stage::CheckpointReady)
    }
}

impl BackedUpUuidEntityMigration {
    /// Atomically publishes an immutable fixed checkpoint before enabling commit.
    ///
    /// Uses create-new staging, fsync and no-clobber hard-link publication in one
    /// parent directory. Errors may preserve known staging/final evidence; resume
    /// verifies it fully rather than guessing aliases or overwriting an old file.
    pub fn write_checkpoint(
        self,
        path: impl AsRef<Path>,
    ) -> MigrationResult<CheckpointedUuidEntityMigration> {
        self.write_checkpoint_observed(path.as_ref(), &mut |_| Ok(()))
    }

    pub(super) fn write_checkpoint_observed(
        self,
        path: &Path,
        observe: Observer<'_>,
    ) -> MigrationResult<CheckpointedUuidEntityMigration> {
        self.prepared.source.verify_single_link()?;
        self.backup_file.verify_single_link()?;
        if self.backup_file.hash()? != (self.backup.file_len, self.backup.file_sha256) {
            return Err(Error::BackupMismatch);
        }
        let final_path = files::canonical_path(path)?;
        let parent = final_path
            .parent()
            .ok_or_else(|| Error::InvalidFile(final_path.clone()))?;
        let mut stage = None;
        for _ in 0..16 {
            let uuid = Uuid::new_v4();
            let staging_path = parent.join(format!(".ferrumc-uuid-checkpoint-{uuid}.tmp"));
            match LockedFile::open(&staging_path, true, false) {
                Ok(file) => {
                    stage = Some((uuid, file));
                    break;
                }
                Err(Error::EvidenceExists(_)) => {}
                Err(e) => return Err(e),
            }
        }
        let (uuid, mut file) = stage.ok_or_else(|| {
            io_error(
                "create checkpoint staging",
                parent,
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "checkpoint collision bound reached",
                ),
            )
        })?;
        file.separate_from(&self.prepared.source)?;
        file.separate_from(&self.backup_file)?;
        observe(Stage::CheckpointCreated)?;
        let data = Data {
            source: self.prepared.source.identity,
            source_file_len: self.prepared.report.source_file_len,
            backup: self.backup_file.identity,
            backup_len: self.backup.file_len,
            backup_hash: self.backup.file_sha256,
            scan: self.prepared.report.scan.clone(),
            identity: file.identity,
            staging: uuid,
        };
        let bytes = data.encode();
        file.write_exact(&bytes)?;
        if file.read_exact::<LENGTH>()? != bytes || Data::decode(&bytes)? != data {
            return Err(Error::CheckpointChanged);
        }
        observe(Stage::CheckpointWritten)?;
        file.file
            .sync_all()
            .map_err(|e| io_error("fsync checkpoint staging", &file.path, e))?;
        observe(Stage::CheckpointFileSynced)?;
        file.verify_single_link()?;
        fs::hard_link(&file.path, &final_path).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                Error::EvidenceExists(final_path.clone())
            } else {
                io_error("publish checkpoint", &final_path, e)
            }
        })?;
        let staging_path = file.path.clone();
        file.rebind(final_path)?;
        let checkpoint = Authority {
            file,
            data,
            bytes,
            staging_path,
        };
        checkpoint.verify(false)?;
        observe(Stage::CheckpointLinked)?;
        checkpoint.stabilize(observe)?;
        Ok(CheckpointedUuidEntityMigration {
            backed: Box::new(self),
            checkpoint: Box::new(checkpoint),
        })
    }
}
