//! Narrow owned offline files; legacy migration helpers remain unchanged.

use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use redb::{backends::FileBackend, Database, StorageBackend};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    backend, io_error, MigrationResult, UuidEntityMigrationError as Error,
    UuidEntityMigrationScratchKind,
};

pub(super) const CACHE_BYTES: usize = 64 * 1024 * 1024;
const BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::redb_store) struct FileIdentity {
    pub(in crate::redb_store) device: u64,
    pub(in crate::redb_store) inode: u64,
}

impl FileIdentity {
    // The non-Unix implementation must return an explicit platform refusal.
    #[allow(clippy::unnecessary_wraps)]
    pub(in crate::redb_store) fn of(meta: &Metadata) -> MigrationResult<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(Self {
                device: meta.dev(),
                inode: meta.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = meta;
            Err(Error::UnsupportedPlatform)
        }
    }
}

// The non-Unix implementation must return an explicit platform refusal.
#[allow(clippy::unnecessary_wraps)]
pub(in crate::redb_store) fn nlink(meta: &Metadata) -> MigrationResult<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(meta.nlink())
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        Err(Error::UnsupportedPlatform)
    }
}

pub(in crate::redb_store) fn canonical_path(path: &Path) -> MigrationResult<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::InvalidFile(path.to_owned()))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|e| io_error("resolve parent", parent, e))?;
    Ok(parent.join(name))
}

#[derive(Debug)]
pub(in crate::redb_store) struct DiskBudget {
    kind: UuidEntityMigrationScratchKind,
    limit: u64,
    exceeded: AtomicBool,
}

impl DiskBudget {
    pub(in crate::redb_store) fn new(
        kind: UuidEntityMigrationScratchKind,
        limit: u64,
    ) -> Arc<Self> {
        Arc::new(Self {
            kind,
            limit,
            exceeded: AtomicBool::new(false),
        })
    }
    pub(in crate::redb_store) fn check(&self, extent: Option<u64>) -> io::Result<()> {
        if extent.is_none_or(|n| n > self.limit) {
            self.exceeded.store(true, Ordering::Relaxed);
            return Err(io::Error::other(
                "UUID migration scratch byte budget exceeded",
            ));
        }
        Ok(())
    }
    pub(in crate::redb_store) fn error(&self) -> Option<Error> {
        self.exceeded
            .load(Ordering::Relaxed)
            .then_some(Error::ScratchBudgetExceeded {
                kind: self.kind,
                limit: self.limit,
            })
    }
    pub(in crate::redb_store) fn map(
        &self,
        operation: &'static str,
        e: impl Into<redb::Error>,
    ) -> Error {
        self.error().unwrap_or_else(|| backend(operation, e))
    }
    pub(in crate::redb_store) fn admit(&self, n: u64) -> MigrationResult<()> {
        self.check(Some(n))
            .map_err(|_| Error::ScratchBudgetExceeded {
                kind: self.kind,
                limit: self.limit,
            })
    }
}

#[derive(Debug)]
pub(super) struct SharedBackend {
    backend: Arc<FileBackend>,
    budget: Option<Arc<DiskBudget>>,
    #[cfg(test)]
    fail_sync: Option<Arc<AtomicBool>>,
}
impl SharedBackend {
    pub(super) fn new(backend: Arc<FileBackend>, budget: Option<Arc<DiskBudget>>) -> Self {
        Self {
            backend,
            budget,
            #[cfg(test)]
            fail_sync: None,
        }
    }
}
impl StorageBackend for SharedBackend {
    fn len(&self) -> io::Result<u64> {
        self.backend.len()
    }
    fn read(&self, offset: u64, len: usize) -> io::Result<Vec<u8>> {
        self.backend.read(offset, len)
    }
    fn set_len(&self, len: u64) -> io::Result<()> {
        if let Some(b) = &self.budget {
            b.check(Some(len))?;
        }
        self.backend.set_len(len)
    }
    fn sync_data(&self, eventual: bool) -> io::Result<()> {
        #[cfg(test)]
        if self
            .fail_sync
            .as_ref()
            .is_some_and(|flag| flag.swap(false, Ordering::Relaxed))
        {
            return Err(io::Error::other("injected actual backend sync failure"));
        }
        self.backend.sync_data(eventual)
    }
    fn write(&self, offset: u64, data: &[u8]) -> io::Result<()> {
        if let Some(b) = &self.budget {
            b.check(offset.checked_add(data.len() as u64))?;
        }
        self.backend.write(offset, data)
    }
}

pub(in crate::redb_store) struct LockedFile {
    backend: Arc<FileBackend>,
    pub(in crate::redb_store) file: File,
    pub(in crate::redb_store) path: PathBuf,
    pub(in crate::redb_store) identity: FileIdentity,
    #[cfg(test)]
    pub(in crate::redb_store) fail_sync: Arc<AtomicBool>,
}
impl fmt::Debug for LockedFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockedFile")
            .field("path", &self.path)
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

impl LockedFile {
    pub(in crate::redb_store) fn backend(&self) -> Arc<FileBackend> {
        Arc::clone(&self.backend)
    }
    pub(in crate::redb_store) fn open(
        path: &Path,
        create_new: bool,
        checkpoint_links: bool,
    ) -> MigrationResult<Self> {
        Self::open_mode(path, create_new, checkpoint_links, true)
    }
    pub(in crate::redb_store) fn open_readonly(path: &Path) -> MigrationResult<Self> {
        Self::open_mode(path, false, false, false)
    }
    fn open_mode(
        path: &Path,
        create_new: bool,
        checkpoint_links: bool,
        writable: bool,
    ) -> MigrationResult<Self> {
        #[cfg(not(unix))]
        {
            let _ = (path, create_new, checkpoint_links, writable);
            return Err(Error::UnsupportedPlatform);
        }
        #[cfg(unix)]
        {
            let path = canonical_path(path)?;
            if !create_new {
                let meta = fs::symlink_metadata(&path).map_err(|e| io_error("stat", &path, e))?;
                if meta.file_type().is_symlink() {
                    return Err(Error::Symlink(path));
                }
                if !meta.is_file() || meta.len() == 0 {
                    return Err(Error::InvalidFile(path));
                }
            }
            let file = OpenOptions::new()
                .read(true)
                .write(writable)
                .create_new(create_new)
                .open(&path)
                .map_err(|e| {
                    if create_new && e.kind() == io::ErrorKind::AlreadyExists {
                        Error::EvidenceExists(path.clone())
                    } else {
                        io_error("open", &path, e)
                    }
                })?;
            let meta = file.metadata().map_err(|e| {
                if create_new {
                    // Without a handle identity it is unsafe to unlink the path.
                    Error::Cleanup {
                        path: path.clone(),
                        source: e,
                    }
                } else {
                    io_error("fstat", &path, e)
                }
            })?;
            if !meta.is_file() {
                return Err(Error::InvalidFile(path));
            }
            let identity = FileIdentity::of(&meta)?;
            let result = (|| {
                if !checkpoint_links && nlink(&meta)? != 1 {
                    return Err(Error::UnexpectedFileAlias(path.clone()));
                }
                let clone = file
                    .try_clone()
                    .map_err(|e| io_error("clone lock handle", &path, e))?;
                let backend = Arc::new(FileBackend::new(clone).map_err(|e| match e {
                    redb::DatabaseError::DatabaseAlreadyOpen => Error::Busy(path.clone()),
                    other => super::backend("file lock", other),
                })?);
                let locked = Self {
                    backend,
                    file,
                    path: path.clone(),
                    identity,
                    #[cfg(test)]
                    fail_sync: Arc::new(AtomicBool::new(false)),
                };
                locked.verify_path()?;
                Ok(locked)
            })();
            if create_new && result.is_err() {
                remove_owned_path(&path, identity)?;
            }
            result
        }
    }
    pub(in crate::redb_store) fn verify_path(&self) -> MigrationResult<()> {
        let meta =
            fs::symlink_metadata(&self.path).map_err(|e| io_error("verify path", &self.path, e))?;
        if meta.file_type().is_symlink()
            || !meta.is_file()
            || FileIdentity::of(&meta)? != self.identity
        {
            return Err(Error::PathChanged(self.path.clone()));
        }
        Ok(())
    }
    pub(in crate::redb_store) fn verify_single_link(&self) -> MigrationResult<()> {
        self.verify_path()?;
        if nlink(
            &self
                .file
                .metadata()
                .map_err(|e| io_error("fstat links", &self.path, e))?,
        )? != 1
        {
            return Err(Error::UnexpectedFileAlias(self.path.clone()));
        }
        Ok(())
    }
    pub(in crate::redb_store) fn separate_from(&self, other: &Self) -> MigrationResult<()> {
        if self.identity == other.identity || self.path == other.path {
            return Err(Error::UnexpectedFileAlias(self.path.clone()));
        }
        Ok(())
    }
    pub(in crate::redb_store) fn len(&self) -> MigrationResult<u64> {
        self.file
            .metadata()
            .map(|m| m.len())
            .map_err(|e| io_error("file length", &self.path, e))
    }
    pub(in crate::redb_store) fn open_database(
        &self,
        fresh: bool,
        budget: Option<&Arc<DiskBudget>>,
    ) -> MigrationResult<Database> {
        self.open_database_with_cache(fresh, budget, CACHE_BYTES)
    }
    pub(in crate::redb_store) fn open_database_with_cache(
        &self,
        fresh: bool,
        budget: Option<&Arc<DiskBudget>>,
        cache_bytes: usize,
    ) -> MigrationResult<Database> {
        self.verify_single_link()?;
        if !fresh {
            require_clean_header(&self.file, &self.path)?;
        }
        if let Some(b) = &budget {
            b.admit(self.len()?)?;
        }
        let mut builder = Database::builder();
        builder.set_cache_size(cache_bytes);
        builder.set_repair_callback(redb::RepairSession::abort);
        let shared = SharedBackend::new(Arc::clone(&self.backend), budget.cloned());
        #[cfg(test)]
        let shared = SharedBackend {
            fail_sync: Some(Arc::clone(&self.fail_sync)),
            ..shared
        };
        builder.create_with_backend(shared).map_err(|e| {
            if let Some(error) = budget.as_ref().and_then(|b| b.error()) {
                return error;
            }
            match e {
                redb::DatabaseError::RepairAborted => Error::RecoveryRequired,
                other => backend("open offline database", other),
            }
        })
    }
    pub(in crate::redb_store) fn sync(&self) -> MigrationResult<()> {
        self.verify_path()?;
        self.file
            .sync_all()
            .map_err(|e| io_error("fsync file", &self.path, e))?;
        sync_parent(&self.path)
    }
    pub(in crate::redb_store) fn hash(&self) -> MigrationResult<(u64, [u8; 32])> {
        self.hash_expected_len(self.len()?)
    }
    /// Hash only the admitted closed extent and one byte of growth lookahead.
    pub(in crate::redb_store) fn hash_expected_len(
        &self,
        expected: u64,
    ) -> MigrationResult<(u64, [u8; 32])> {
        self.verify_path()?;
        if self.len()? != expected {
            return Err(Error::BackupMismatch);
        }
        let mut reader = self
            .file
            .try_clone()
            .map_err(|e| io_error("clone hash", &self.path, e))?;
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| io_error("seek hash", &self.path, e))?;
        let mut hash = Sha256::new();
        let mut buffer = vec![0; BUFFER_BYTES];
        let mut count = 0u64;
        while count < expected {
            let extent = (expected - count).min(BUFFER_BYTES as u64) as usize;
            let n = reader
                .read(&mut buffer[..extent])
                .map_err(|e| io_error("read hash", &self.path, e))?;
            if n == 0 {
                return Err(Error::BackupMismatch);
            }
            count = count.checked_add(n as u64).ok_or(Error::CounterOverflow)?;
            hash.update(&buffer[..n]);
        }
        if reader
            .read(&mut buffer[..1])
            .map_err(|e| io_error("read hash extent", &self.path, e))?
            != 0
            || self.len()? != expected
        {
            return Err(Error::BackupMismatch);
        }
        self.verify_path()?;
        Ok((count, hash.finalize().into()))
    }
    pub(in crate::redb_store) fn read_exact<const N: usize>(&self) -> MigrationResult<[u8; N]> {
        self.verify_path()?;
        if self.len()? != N as u64 {
            return Err(Error::InvalidFile(self.path.clone()));
        }
        let mut reader = self
            .file
            .try_clone()
            .map_err(|e| io_error("clone record", &self.path, e))?;
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| io_error("seek record", &self.path, e))?;
        let mut bytes = [0; N];
        reader
            .read_exact(&mut bytes)
            .map_err(|e| io_error("read record", &self.path, e))?;
        Ok(bytes)
    }
    pub(in crate::redb_store) fn write_exact(&self, bytes: &[u8]) -> MigrationResult<()> {
        self.verify_single_link()?;
        let mut file = self
            .file
            .try_clone()
            .map_err(|e| io_error("clone record writer", &self.path, e))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| io_error("seek record writer", &self.path, e))?;
        file.write_all(bytes)
            .map_err(|e| io_error("write record", &self.path, e))
    }
    pub(in crate::redb_store) fn rebind(&mut self, new_path: PathBuf) -> MigrationResult<()> {
        let meta = fs::symlink_metadata(&new_path)
            .map_err(|e| io_error("verify published record", &new_path, e))?;
        if !meta.is_file() || FileIdentity::of(&meta)? != self.identity {
            return Err(Error::PathChanged(new_path));
        }
        self.path = new_path;
        self.verify_path()
    }
}

pub(in crate::redb_store) fn sync_parent(path: &Path) -> MigrationResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::InvalidFile(path.to_owned()))?;
    File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| io_error("fsync directory", parent, e))
}

pub(in crate::redb_store) fn require_clean_header(file: &File, path: &Path) -> MigrationResult<()> {
    const MAGIC: [u8; 9] = [b'r', b'e', b'd', b'b', 0x1a, 0x0a, 0xa9, 0x0d, 0x0a];
    let mut reader = file
        .try_clone()
        .map_err(|e| io_error("clone header", path, e))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|e| io_error("seek header", path, e))?;
    let mut prefix = [0; 320];
    match reader.read_exact(&mut prefix) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Err(Error::InvalidRedbHeader),
        Err(e) => return Err(io_error("read header", path, e)),
    }
    if prefix[..9] != MAGIC || prefix[9] & !7 != 0 {
        return Err(Error::InvalidRedbHeader);
    }
    if prefix[9] & 2 != 0 {
        return Err(Error::RecoveryRequired);
    }
    validate_geometry(
        &prefix,
        file.metadata()
            .map_err(|e| io_error("stat header", path, e))?
            .len(),
    )
}

fn validate_geometry(header: &[u8; 320], file_len: u64) -> MigrationResult<()> {
    let field = |offset| {
        u32::from_le_bytes([
            header[offset],
            header[offset + 1],
            header[offset + 2],
            header[offset + 3],
        ])
    };
    let page_size = field(12);
    let region_header_pages = field(16);
    let max_data_pages = field(20);
    let full_regions = field(24);
    let trailing_pages = field(28);
    if page_size != 4096
        || region_header_pages == 0
        || region_header_pages > 1024
        || !max_data_pages.is_power_of_two()
        || max_data_pages > 1_048_576
        || trailing_pages >= max_data_pages
        || (full_regions == 0 && trailing_pages == 0)
        || full_regions
            .checked_add(u32::from(trailing_pages > 0))
            .is_none()
    {
        return Err(Error::InvalidRedbHeader);
    }
    let full = u64::from(region_header_pages) + u64::from(max_data_pages);
    let trailing = if trailing_pages == 0 {
        0
    } else {
        u64::from(region_header_pages) + u64::from(trailing_pages)
    };
    let expected = u64::from(full_regions)
        .checked_mul(full)
        .and_then(|n| n.checked_add(trailing))
        .and_then(|n| n.checked_add(1))
        .and_then(|n| n.checked_mul(u64::from(page_size)))
        .ok_or(Error::InvalidRedbHeader)?;
    if expected != file_len {
        return Err(Error::RecoveryRequired);
    }
    Ok(())
}

#[derive(Debug)]
pub(in crate::redb_store) struct TemporaryFile {
    file: Option<LockedFile>,
    path: PathBuf,
    identity: FileIdentity,
}

pub(in crate::redb_store) fn remove_owned_path(
    path: &Path,
    identity: FileIdentity,
) -> MigrationResult<()> {
    let owned = fs::symlink_metadata(path)
        .is_ok_and(|m| m.is_file() && FileIdentity::of(&m).ok() == Some(identity));
    if !owned {
        return Err(Error::Cleanup {
            path: path.to_owned(),
            source: io::Error::other("owned temporary path replaced or unverifiable"),
        });
    }
    fs::remove_file(path).map_err(|source| Error::Cleanup {
        path: path.to_owned(),
        source,
    })
}

impl TemporaryFile {
    pub(in crate::redb_store) fn create(parent: &Path, prefix: &str) -> MigrationResult<Self> {
        for _ in 0..16 {
            let path = parent.join(format!(".{prefix}-{}.redb", Uuid::new_v4()));
            match LockedFile::open(&path, true, false) {
                Ok(file) => {
                    return Ok(Self {
                        path: file.path.clone(),
                        identity: file.identity,
                        file: Some(file),
                    });
                }
                Err(Error::EvidenceExists(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Err(io_error(
            "create temporary file",
            parent,
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "temporary filename collision bound reached",
            ),
        ))
    }
    pub(in crate::redb_store) fn file(&self) -> MigrationResult<&LockedFile> {
        self.file
            .as_ref()
            .ok_or(Error::InvalidFile(self.path.clone()))
    }
    pub(in crate::redb_store) fn cleanup(mut self) -> MigrationResult<()> {
        self.remove()
    }
    fn remove(&mut self) -> MigrationResult<()> {
        if self.file.take().is_none() {
            return Ok(());
        }
        remove_owned_path(&self.path, self.identity)
    }
}
impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if let Err(error) = self.remove() {
            tracing::warn!(%error,"UUID migration owned temporary cleanup failed");
        }
    }
}

pub(in crate::redb_store) fn copy_closed(
    source: &LockedFile,
    destination: &LockedFile,
    budget: &DiskBudget,
    expected: (u64, [u8; 32]),
) -> MigrationResult<()> {
    source.verify_single_link()?;
    destination.verify_single_link()?;
    destination.separate_from(source)?;
    budget.admit(expected.0)?;
    if source.hash_expected_len(expected.0)? != expected {
        return Err(Error::BackupMismatch);
    }
    let mut reader = source
        .file
        .try_clone()
        .map_err(|e| io_error("clone physical copy reader", &source.path, e))?;
    let mut writer = destination
        .file
        .try_clone()
        .map_err(|e| io_error("clone physical copy writer", &destination.path, e))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|e| io_error("seek physical copy reader", &source.path, e))?;
    writer
        .seek(SeekFrom::Start(0))
        .map_err(|e| io_error("seek physical copy writer", &destination.path, e))?;
    let mut buffer = vec![0; BUFFER_BYTES];
    let mut total = 0u64;
    let mut hash = Sha256::new();
    while total < expected.0 {
        let extent = (expected.0 - total).min(BUFFER_BYTES as u64) as usize;
        let n = reader
            .read(&mut buffer[..extent])
            .map_err(|e| io_error("read physical copy", &source.path, e))?;
        if n == 0 {
            return Err(Error::BackupMismatch);
        }
        total = total.checked_add(n as u64).ok_or(Error::CounterOverflow)?;
        budget.admit(total)?;
        writer
            .write_all(&buffer[..n])
            .map_err(|e| io_error("write physical copy", &destination.path, e))?;
        hash.update(&buffer[..n]);
    }
    if reader
        .read(&mut buffer[..1])
        .map_err(|e| io_error("read physical copy extent", &source.path, e))?
        != 0
        || source.len()? != expected.0
    {
        return Err(Error::BackupMismatch);
    }
    let actual: [u8; 32] = hash.finalize().into();
    if (total, actual) != expected {
        return Err(Error::BackupMismatch);
    }
    destination.sync()?;
    require_clean_header(&destination.file, &destination.path)?;
    if destination.hash_expected_len(expected.0)? != expected
        || source.hash_expected_len(expected.0)? != expected
    {
        return Err(Error::BackupMismatch);
    }
    Ok(())
}
