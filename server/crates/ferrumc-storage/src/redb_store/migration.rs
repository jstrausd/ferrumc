//! Explicit offline format migration; no startup path invokes this module.

use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ferrumc_world::{GenerationIdentity, WorldSeed};
use redb::{backends::FileBackend, Database, ReadableTable, StorageBackend};
use sha2::{Digest, Sha256};

use super::{
    RedbStore, CHUNK_OVERLAY_TABLE, CHUNK_TABLE, ENTITY_TABLE, GENERATION_STORE_FORMAT_VERSION,
    GENERATION_TABLE, META_FORMAT_KEY, META_TABLE, MUTATION_BATCH_TABLE, MUTATION_LOG_TABLE,
    PLAYER_TABLE, PLUGIN_TABLE, STORE_FORMAT_VERSION,
};
use crate::{GenerationKey, JournalBatchId, StorageError, MAX_GENERATION_IDENTITIES};

#[cfg(all(test, unix))]
mod tests;
mod validation;

const MIGRATION_CACHE_BYTES: usize = 64 * 1024 * 1024;
const FILE_HASH_BUFFER_BYTES: usize = 64 * 1024;

/// Explicit migration-only cap for a legacy plugin's previously unbounded ID.
pub const MAX_MIGRATION_PLUGIN_ID_LEN: usize = 4096;

type MigrationResult<T> = std::result::Result<T, GenerationMigrationError>;

/// A precise refusal or operational failure of an explicit offline migration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GenerationMigrationError {
    /// At least one operator-declared world/dimension scope is required.
    #[error("legacy migration requires at least one explicitly declared scope")]
    EmptyDeclaration,
    /// A declaration exceeded the same 4096-scope cap as durable metadata.
    #[error("legacy migration scope limit is {MAX_GENERATION_IDENTITIES}")]
    TooManyScopes,
    /// Duplicate scopes are refused rather than silently deduplicated.
    #[error("duplicate legacy migration scope {key:?}")]
    DuplicateScope {
        /// Repeated scope.
        key: GenerationKey,
    },
    /// This release cannot verify durable file identity on this platform.
    #[error("offline migration currently requires Unix inode identity and directory fsync")]
    UnsupportedPlatform,
    /// A final path component was a symlink.
    #[error("migration refuses a final symlink: {0}")]
    Symlink(PathBuf),
    /// Source/backup was not a nonempty regular file.
    #[error("migration requires a nonempty regular redb file: {0}")]
    InvalidFile(PathBuf),
    /// The path no longer names the locked file.
    #[error("migration path was replaced or aliased: {0}")]
    PathChanged(PathBuf),
    /// The pinned redb header is malformed or unsupported.
    #[error("invalid or unsupported redb header; no migration was performed")]
    InvalidRedbHeader,
    /// The source or backup requires separate recovery before this operation.
    #[error("redb recovery is required; recover and validate separately before migration")]
    RecoveryRequired,
    /// An existing file had a different `FerrumC` layout marker.
    #[error("unsupported FerrumC format marker {0:?}; expected format 3 or complete format 4")]
    UnsupportedFormat(Option<u64>),
    /// Table names/counts/kinds do not exactly match the declared format.
    #[error("incomplete, unexpected or multimap FerrumC table catalog")]
    CatalogMismatch,
    /// A metadata key cannot be interpreted by this bounded migration.
    #[error("unknown FerrumC metadata key; explicit schema support is required")]
    UnknownMetadata,
    /// A durable key has the wrong fixed grammar.
    #[error("malformed key in {table}: {len} bytes")]
    MalformedKey {
        /// Durable table containing the key.
        table: &'static str,
        /// Actual raw key length.
        len: usize,
    },
    /// A namespaced plugin key exceeded limits or contained invalid UTF-8.
    #[error("invalid or over-limit legacy plugin namespace/key")]
    InvalidPluginKey,
    /// A bounded existing record decoder refused a stored value.
    #[error("invalid record in {table}: {source}")]
    Record {
        /// Durable table containing the record.
        table: &'static str,
        /// Exact bounded storage-codec or descriptor failure.
        #[source]
        source: Box<StorageError>,
    },
    /// Stored scoped state has no explicit declaration.
    #[error("stored scope {key:?} is absent from the operator declaration")]
    UndeclaredScope {
        /// Uncovered stored world/dimension.
        key: GenerationKey,
    },
    /// Old player/journal state cannot be attributed among multiple scopes.
    #[error("unscoped player/journal/receipt/watermark state requires exactly one declared scope")]
    AmbiguousUnscopedState,
    /// A format-4 retry does not contain the complete exact declared baseline.
    #[error("format-4 descriptor set differs from the complete operator declaration")]
    DescriptorSetMismatch,
    /// Durable journal IDs were not contiguous starting at zero.
    #[error("journal sequence is not contiguous: expected {expected:?}, found {actual}")]
    JournalSequence {
        /// Next required ID; `None` means the u64 sequence is exhausted.
        expected: Option<u64>,
        /// Actual stored key ID.
        actual: u64,
    },
    /// The ID inside a journal record differs from its durable key.
    #[error("journal key ID {key} differs from record ID {record}")]
    JournalRecordId {
        /// Key ID.
        key: u64,
        /// Record ID.
        record: u64,
    },
    /// Journal watermark and greatest valid record disagree.
    #[error("journal watermark {stored:?} differs from last record {last:?}")]
    JournalWatermark {
        /// Value stored in the metadata table.
        stored: Option<u64>,
        /// Greatest validated journal ID, or absent for an empty journal.
        last: Option<u64>,
    },
    /// Two receipts claim overlapping durable journal ranges.
    #[error("journal receipts {first} and {second} overlap")]
    OverlappingReceipts {
        /// First conflicting batch.
        first: JournalBatchId,
        /// Second conflicting batch.
        second: JournalBatchId,
    },
    /// Existing backups are never silently overwritten.
    #[error("backup path already exists: {0}")]
    BackupExists(PathBuf),
    /// A proposed backup did not match its recorded hash or prepared snapshot.
    #[error("backup does not match the prepared snapshot or its expected closed-file SHA-256")]
    BackupMismatch,
    /// The source changed after preparation.
    #[error("source snapshot changed after preparation; migration refused")]
    SourceChanged,
    /// A u64 report counter could not represent the stored state.
    #[error("migration report counter overflow")]
    CounterOverflow,
    /// A synchronous filesystem action failed.
    #[error("{operation} failed for {path}: {source}")]
    Io {
        /// Action that failed.
        operation: &'static str,
        /// File or directory involved.
        path: PathBuf,
        /// Original filesystem failure.
        #[source]
        source: io::Error,
    },
    /// A redb operation failed; its structured original error is retained.
    #[error("redb {operation} failed: {source}")]
    Backend {
        /// Action that failed.
        operation: &'static str,
        /// Original database failure.
        #[source]
        source: Box<redb::Error>,
    },
}

/// Operator assertion of the known pre-metadata `FerrumC` flat baseline.
///
/// Seed zero is an explicitly assigned legacy identity, never a recovered seed.
/// This constructor does not prove that imported/edited chunks had this origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyGenerationDeclaration {
    scopes: Vec<GenerationKey>,
}

impl LegacyGenerationDeclaration {
    /// Declares flat-v1 / 1.21.8 / Overworld and deliberately assigns seed zero.
    ///
    /// Iteration stops after the 4097th item; no unbounded declaration is stored.
    pub fn ferrumc_flat_v1_seed_zero(
        scopes: impl IntoIterator<Item = GenerationKey>,
    ) -> MigrationResult<Self> {
        let mut declared = Vec::new();
        for key in scopes {
            if declared.len() == MAX_GENERATION_IDENTITIES {
                return Err(GenerationMigrationError::TooManyScopes);
            }
            match declared.binary_search(&key) {
                Ok(_) => return Err(GenerationMigrationError::DuplicateScope { key }),
                Err(index) => declared.insert(index, key),
            }
        }
        if declared.is_empty() {
            return Err(GenerationMigrationError::EmptyDeclaration);
        }
        Ok(Self { scopes: declared })
    }

    /// Returns the complete sorted set of explicitly declared typed scopes.
    pub fn scopes(&self) -> &[GenerationKey] {
        &self.scopes
    }

    /// Returns the fixed identity explicitly assigned by this declaration.
    pub const fn identity(&self) -> GenerationIdentity {
        GenerationIdentity::flat_overworld(WorldSeed::new(0))
    }
}

/// Durable categories counted in a preparation report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationMigrationTable {
    /// Typed format/watermark metadata entries.
    Metadata,
    /// Complete chunk snapshots.
    Chunks,
    /// Player-edited overlays, including empty overlays.
    ChunkOverlays,
    /// Opaque versioned entity records.
    Entities,
    /// Opaque versioned player records.
    Players,
    /// Namespaced plugin key/value entries.
    PluginValues,
    /// Ordered block mutation records.
    JournalRecords,
    /// Batch receipts, including empty receipts.
    JournalReceipts,
    /// Immutable generation descriptors; absent in format 3.
    GenerationIdentities,
}

impl GenerationMigrationTable {
    const fn index(self) -> usize {
        match self {
            Self::Metadata => 0,
            Self::Chunks => 1,
            Self::ChunkOverlays => 2,
            Self::Entities => 3,
            Self::Players => 4,
            Self::PluginValues => 5,
            Self::JournalRecords => 6,
            Self::JournalReceipts => 7,
            Self::GenerationIdentities => 8,
        }
    }
}

/// Complete streaming preflight result; cloning copies only bounded metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationMigrationReport {
    source_path: PathBuf,
    source_format: u64,
    source_file_len: u64,
    declaration: LegacyGenerationDeclaration,
    scan: validation::Scan,
}

impl GenerationMigrationReport {
    /// Returns the canonical source path observed during preparation.
    ///
    /// Only the prepared/backed-up typestate owns the source lock; a cloned
    /// report, an outcome or `AlreadyCompatible` report does not keep it held.
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }
    /// Returns the fully validated `FerrumC` source layout marker (3 or 4).
    pub const fn source_format(&self) -> u64 {
        self.source_format
    }
    /// Returns the only migration target layout, independently of record schema.
    pub const fn target_format(&self) -> u64 {
        GENERATION_STORE_FORMAT_VERSION
    }
    /// Returns the physical source file size observed during preparation.
    pub const fn source_file_len(&self) -> u64 {
        self.source_file_len
    }
    /// Returns the operator assertion; its seed is deliberately assigned.
    pub fn declaration(&self) -> &LegacyGenerationDeclaration {
        &self.declaration
    }
    /// Returns the number of completely validated entries in one category.
    pub const fn records(&self, table: GenerationMigrationTable) -> u64 {
        self.scan.counts[table.index()]
    }
    /// Returns summed logical key/value bytes, not physical redb pages or RSS.
    pub const fn logical_bytes(&self) -> u64 {
        self.scan.logical_bytes
    }
    /// Returns the table-/length-separated logical snapshot SHA-256.
    ///
    /// This includes all validated tables and the actual format marker. It is
    /// distinct from a physical closed-file hash and changes after migration.
    pub const fn snapshot_sha256(&self) -> &[u8; 32] {
        &self.scan.sha256
    }
}

/// Prepared migration or a fully validated idempotent format-4 retry.
#[derive(Debug)]
pub enum GenerationMigrationPreparation {
    /// Format 3 was validated; the returned value owns its exclusive file lock.
    Required(PreparedGenerationMigration),
    /// Every record and the complete exact descriptor/declaration set matched.
    AlreadyCompatible(GenerationMigrationReport),
}

/// Exclusive source preflight; only a verified backup enables the commit API.
pub struct PreparedGenerationMigration {
    // Database drops before the last FileBackend Arc, preserving the lock even
    // while redb performs its close-time allocator housekeeping.
    db: Database,
    source: LockedFile,
    report: GenerationMigrationReport,
}

impl fmt::Debug for PreparedGenerationMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedGenerationMigration")
            .field("report", &self.report)
            .finish_non_exhaustive()
    }
}

/// Durable separate backup evidence; it contains no writable handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationMigrationBackup {
    path: PathBuf,
    file_len: u64,
    file_sha256: [u8; 32],
    snapshot_sha256: [u8; 32],
}

impl GenerationMigrationBackup {
    /// Returns the separate canonical backup path; it is never overwritten.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Returns the closed physical backup file size.
    pub const fn file_len(&self) -> u64 {
        self.file_len
    }
    /// Returns SHA-256 after close and fsync of the logically verified backup.
    pub const fn file_sha256(&self) -> &[u8; 32] {
        &self.file_sha256
    }
    /// Returns the prepared logical snapshot hash reproduced by this backup.
    pub const fn snapshot_sha256(&self) -> &[u8; 32] {
        &self.snapshot_sha256
    }
}

/// Typestate proving the source has a separate fully verified backup.
pub struct BackedUpGenerationMigration {
    prepared: PreparedGenerationMigration,
    backup_file: LockedFile,
    backup: GenerationMigrationBackup,
}

impl fmt::Debug for BackedUpGenerationMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackedUpGenerationMigration")
            .field("report", &self.prepared.report)
            .field("backup", &self.backup)
            .finish_non_exhaustive()
    }
}

/// Evidence returned only after all descriptors and format 4 committed together.
#[derive(Debug, Clone)]
pub struct GenerationMigrationOutcome {
    report: GenerationMigrationReport,
    backup: GenerationMigrationBackup,
}

impl GenerationMigrationOutcome {
    /// Returns the pre-migration report describing the preserved source rows.
    pub fn report(&self) -> &GenerationMigrationReport {
        &self.report
    }
    /// Returns evidence for the separate pre-migration format-3 backup.
    pub fn backup(&self) -> &GenerationMigrationBackup {
        &self.backup
    }
}

fn backend(operation: &'static str, source: impl Into<redb::Error>) -> GenerationMigrationError {
    GenerationMigrationError::Backend {
        operation,
        source: Box::new(source.into()),
    }
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> GenerationMigrationError {
    GenerationMigrationError::Io {
        operation,
        path: path.to_owned(),
        source,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileIdentity {
    // The non-Unix branch classifies missing file-identity support.
    #[allow(clippy::unnecessary_wraps)]
    fn from_metadata(metadata: &Metadata) -> MigrationResult<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = metadata;
            Err(GenerationMigrationError::UnsupportedPlatform)
        }
    }
}

/// Delegation keeps the original redb file lock across Database open/close.
/// It neither intercepts nor buffers backend reads/writes.
#[derive(Debug)]
struct SharedBackend(Arc<FileBackend>);

impl StorageBackend for SharedBackend {
    fn len(&self) -> io::Result<u64> {
        self.0.len()
    }
    fn read(&self, offset: u64, len: usize) -> io::Result<Vec<u8>> {
        self.0.read(offset, len)
    }
    fn set_len(&self, len: u64) -> io::Result<()> {
        self.0.set_len(len)
    }
    fn sync_data(&self, eventual: bool) -> io::Result<()> {
        self.0.sync_data(eventual)
    }
    fn write(&self, offset: u64, data: &[u8]) -> io::Result<()> {
        self.0.write(offset, data)
    }
}

#[derive(Debug)]
struct LockedFile {
    backend: Arc<FileBackend>,
    file: File,
    path: PathBuf,
    identity: FileIdentity,
}

fn canonical_file_path(path: &Path) -> MigrationResult<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| GenerationMigrationError::InvalidFile(path.to_owned()))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|e| io_error("resolve parent", parent, e))?;
    Ok(parent.join(name))
}

impl LockedFile {
    fn open(path: &Path, create_new: bool) -> MigrationResult<Self> {
        let path = canonical_file_path(path)?;
        if !create_new {
            let meta = fs::symlink_metadata(&path).map_err(|e| io_error("stat", &path, e))?;
            if meta.file_type().is_symlink() {
                return Err(GenerationMigrationError::Symlink(path));
            }
            if !meta.is_file() || meta.len() == 0 {
                return Err(GenerationMigrationError::InvalidFile(path));
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(create_new)
            .open(&path)
            .map_err(|e| {
                if create_new && e.kind() == io::ErrorKind::AlreadyExists {
                    GenerationMigrationError::BackupExists(path.clone())
                } else {
                    io_error("open", &path, e)
                }
            })?;
        let metadata = file.metadata().map_err(|e| io_error("fstat", &path, e))?;
        if !metadata.is_file() {
            return Err(GenerationMigrationError::InvalidFile(path));
        }
        let identity = FileIdentity::from_metadata(&metadata)?;
        let clone = file
            .try_clone()
            .map_err(|e| io_error("clone handle", &path, e))?;
        let backend = Arc::new(FileBackend::new(clone).map_err(|e| backend("file lock", e))?);
        let binding = Self {
            backend,
            file,
            path,
            identity,
        };
        binding.verify_path()?;
        Ok(binding)
    }

    fn verify_path(&self) -> MigrationResult<()> {
        let metadata =
            fs::symlink_metadata(&self.path).map_err(|e| io_error("verify path", &self.path, e))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || FileIdentity::from_metadata(&metadata)? != self.identity
        {
            return Err(GenerationMigrationError::PathChanged(self.path.clone()));
        }
        Ok(())
    }

    fn open_database(&self, fresh: bool) -> MigrationResult<Database> {
        self.verify_path()?;
        if !fresh {
            require_clean_redb_header(&self.file, &self.path)?;
        }
        let mut builder = Database::builder();
        builder.set_cache_size(MIGRATION_CACHE_BYTES);
        builder.set_repair_callback(redb::RepairSession::abort);
        builder
            .create_with_backend(SharedBackend(Arc::clone(&self.backend)))
            .map_err(|e| match e {
                redb::DatabaseError::RepairAborted => GenerationMigrationError::RecoveryRequired,
                other => backend("open migration database", other),
            })
    }

    fn sync(&self) -> MigrationResult<()> {
        self.file
            .sync_all()
            .map_err(|e| io_error("fsync file", &self.path, e))?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| GenerationMigrationError::InvalidFile(self.path.clone()))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| io_error("fsync directory", parent, e))
    }

    fn file_hash(&self) -> MigrationResult<(u64, [u8; 32])> {
        self.verify_path()?;
        let mut reader = self
            .file
            .try_clone()
            .map_err(|e| io_error("clone hash handle", &self.path, e))?;
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| io_error("seek hash", &self.path, e))?;
        let expected_len = reader
            .metadata()
            .map_err(|e| io_error("stat hash", &self.path, e))?
            .len();
        let mut hash = Sha256::new();
        let mut buffer = vec![0; FILE_HASH_BUFFER_BYTES];
        let mut total = 0_u64;
        loop {
            let len = reader
                .read(&mut buffer)
                .map_err(|e| io_error("read hash", &self.path, e))?;
            if len == 0 {
                break;
            }
            total = total
                .checked_add(len as u64)
                .ok_or(GenerationMigrationError::CounterOverflow)?;
            hash.update(&buffer[..len]);
        }
        if total != expected_len {
            return Err(GenerationMigrationError::BackupMismatch);
        }
        Ok((total, hash.finalize().into()))
    }
}

/// redb 2.6.3's nine-byte magic and byte-nine recovery bit, checked under its
/// own `FileBackend` lock before opening. This refuses even fast allocator-state
/// recovery that would bypass the public repair callback. redb still validates
/// the remaining header, commit-slot checksums and physical database format.
fn require_clean_redb_header(file: &File, path: &Path) -> MigrationResult<()> {
    const MAGIC: [u8; 9] = [b'r', b'e', b'd', b'b', 0x1a, 0x0a, 0xa9, 0x0d, 0x0a];
    let mut reader = file
        .try_clone()
        .map_err(|e| io_error("clone header handle", path, e))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|e| io_error("seek header", path, e))?;
    // The fixed super-header lets us refuse impossible page/region geometry
    // before redb reaches assertion-based internal layout constructors.
    let mut prefix = [0; 320];
    match reader.read_exact(&mut prefix) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            return Err(GenerationMigrationError::InvalidRedbHeader)
        }
        Err(e) => return Err(io_error("read header", path, e)),
    }
    if prefix[..9] != MAGIC || prefix[9] & !7 != 0 {
        return Err(GenerationMigrationError::InvalidRedbHeader);
    }
    if prefix[9] & 2 != 0 {
        return Err(GenerationMigrationError::RecoveryRequired);
    }
    validate_redb_geometry(
        &prefix,
        file.metadata()
            .map_err(|e| io_error("stat header", path, e))?
            .len(),
    )?;
    Ok(())
}

fn validate_redb_geometry(header: &[u8; 320], file_len: u64) -> MigrationResult<()> {
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
    // All production stores use the default 4096-byte redb page. Unknown custom
    // geometry requires a separately reviewed physical-format adapter.
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
        return Err(GenerationMigrationError::InvalidRedbHeader);
    }
    let full_region_pages = u64::from(region_header_pages) + u64::from(max_data_pages);
    let trailing_region_pages = if trailing_pages == 0 {
        0
    } else {
        u64::from(region_header_pages) + u64::from(trailing_pages)
    };
    let expected_len = u64::from(full_regions)
        .checked_mul(full_region_pages)
        .and_then(|pages| pages.checked_add(trailing_region_pages))
        .and_then(|pages| pages.checked_add(1))
        .and_then(|pages| pages.checked_mul(u64::from(page_size)))
        .ok_or(GenerationMigrationError::InvalidRedbHeader)?;
    if expected_len != file_len {
        return Err(GenerationMigrationError::RecoveryRequired);
    }
    Ok(())
}

impl RedbStore {
    /// Prepares an explicitly requested offline format-3 to format-4 migration.
    ///
    /// Never creates a missing source, guesses a legacy seed, activates gameplay
    /// metadata, or changes `FerrumC` records/tables. redb housekeeping can change
    /// physical file bytes. The complete scan is blocking I/O/CPU work and must
    /// run outside async workers/ticks. Required preparations exclusively hold
    /// the source until backup/migration or drop; format-4 retries fully validate
    /// the complete caller declaration and every stored record before success.
    pub fn prepare_generation_migration(
        path: impl AsRef<Path>,
        declaration: LegacyGenerationDeclaration,
    ) -> MigrationResult<GenerationMigrationPreparation> {
        let source = LockedFile::open(path.as_ref(), false)?;
        let db = source.open_database(false)?;
        let txn = db
            .begin_write()
            .map_err(|e| backend("prepare transaction", e))?;
        let scan = validation::scan(&txn, &declaration, &source.path)?;
        txn.abort().map_err(|e| backend("abort preflight", e))?;
        let report = GenerationMigrationReport {
            source_path: source.path.clone(),
            source_format: scan.format,
            source_file_len: source
                .file
                .metadata()
                .map_err(|e| io_error("report file size", &source.path, e))?
                .len(),
            declaration,
            scan,
        };
        if report.source_format == GENERATION_STORE_FORMAT_VERSION {
            Ok(GenerationMigrationPreparation::AlreadyCompatible(report))
        } else {
            Ok(GenerationMigrationPreparation::Required(
                PreparedGenerationMigration { db, source, report },
            ))
        }
    }
}

impl PreparedGenerationMigration {
    /// Returns the completely validated report while keeping the source locked.
    pub fn report(&self) -> &GenerationMigrationReport {
        &self.report
    }

    /// Creates, closes, fsyncs and verifies a separate logically exact backup.
    ///
    /// This synchronous operation never overwrites a path. Errors may leave an
    /// incomplete own backup file for explicit operator inspection; it is never
    /// guessed reusable. Source raw records are copied without normalization.
    pub fn create_backup(
        self,
        path: impl AsRef<Path>,
    ) -> MigrationResult<BackedUpGenerationMigration> {
        self.source.verify_path()?;
        let backup_file = LockedFile::open(path.as_ref(), true)?;
        let backup_db = backup_file.open_database(true)?;
        copy_snapshot(&self.db, &backup_db)?;
        drop(backup_db);
        backup_file.sync()?;
        self.finish_backup(backup_file)
    }

    /// Explicitly reuses an existing backup with a previously recorded file hash.
    ///
    /// The hash is checked before redb opening, then all logical rows must equal
    /// the source preflight. Redb may change housekeeping bytes during open/close;
    /// the returned backup evidence includes the current closed-file hash.
    pub fn verify_existing_backup(
        self,
        path: impl AsRef<Path>,
        expected_closed_file_sha256: [u8; 32],
    ) -> MigrationResult<BackedUpGenerationMigration> {
        self.source.verify_path()?;
        let backup_file = LockedFile::open(path.as_ref(), false)?;
        if backup_file.identity == self.source.identity
            || backup_file.file_hash()?.1 != expected_closed_file_sha256
        {
            return Err(GenerationMigrationError::BackupMismatch);
        }
        self.finish_backup(backup_file)
    }

    fn finish_backup(
        self,
        backup_file: LockedFile,
    ) -> MigrationResult<BackedUpGenerationMigration> {
        let db = backup_file.open_database(false)?;
        let txn = db
            .begin_write()
            .map_err(|e| backend("verify backup transaction", e))?;
        let scan = validation::scan(&txn, &self.report.declaration, &backup_file.path)?;
        txn.abort()
            .map_err(|e| backend("abort backup validation", e))?;
        drop(db);
        if scan != self.report.scan {
            return Err(GenerationMigrationError::BackupMismatch);
        }
        backup_file.sync()?;
        require_clean_redb_header(&backup_file.file, &backup_file.path)?;
        let (file_len, file_sha256) = backup_file.file_hash()?;
        let backup = GenerationMigrationBackup {
            path: backup_file.path.clone(),
            file_len,
            file_sha256,
            snapshot_sha256: self.report.scan.sha256,
        };
        Ok(BackedUpGenerationMigration {
            prepared: self,
            backup_file,
            backup,
        })
    }
}

fn copy_snapshot(source: &Database, destination: &Database) -> MigrationResult<()> {
    let read = source.begin_read().map_err(|e| backend("backup read", e))?;
    let write = destination
        .begin_write()
        .map_err(|e| backend("backup write", e))?;
    {
        let source_meta = read
            .open_table(META_TABLE)
            .map_err(|e| backend("backup meta read", e))?;
        let mut dest_meta = write
            .open_table(META_TABLE)
            .map_err(|e| backend("backup meta write", e))?;
        for row in source_meta
            .iter()
            .map_err(|e| backend("backup meta iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("backup meta row", e))?;
            dest_meta
                .insert(key.value(), value.value())
                .map_err(|e| backend("backup meta insert", e))?;
        }
    }
    for definition in [
        CHUNK_TABLE,
        CHUNK_OVERLAY_TABLE,
        ENTITY_TABLE,
        PLAYER_TABLE,
        PLUGIN_TABLE,
        MUTATION_LOG_TABLE,
        MUTATION_BATCH_TABLE,
    ] {
        let source_table = read
            .open_table(definition)
            .map_err(|e| backend("backup table read", e))?;
        let mut dest_table = write
            .open_table(definition)
            .map_err(|e| backend("backup table write", e))?;
        for row in source_table
            .iter()
            .map_err(|e| backend("backup iterate", e))?
        {
            let (key, value) = row.map_err(|e| backend("backup row", e))?;
            dest_table
                .insert(key.value(), value.value())
                .map_err(|e| backend("backup insert", e))?;
        }
    }
    write.commit().map_err(|e| backend("backup commit", e))
}

impl BackedUpGenerationMigration {
    /// Returns the validated pre-migration source report.
    pub fn report(&self) -> &GenerationMigrationReport {
        &self.prepared.report
    }
    /// Returns durable closed-file and logical backup evidence.
    pub fn backup(&self) -> &GenerationMigrationBackup {
        &self.backup
    }

    /// Atomically commits the full descriptor set and format marker 4.
    ///
    /// Revalidates file identity, the closed backup hash and the entire source
    /// under this same writer transaction. All existing raw records and journal
    /// order remain unchanged. There is no automatic rollback on an ambiguous
    /// commit failure; retain the report/backup and retry preflight explicitly.
    pub fn migrate(self) -> MigrationResult<GenerationMigrationOutcome> {
        self.migrate_with_observer(|_| Ok(()))
    }

    fn migrate_with_observer(
        self,
        observe: impl FnMut(MigrationStage) -> MigrationResult<()>,
    ) -> MigrationResult<GenerationMigrationOutcome> {
        self.prepared.source.verify_path()?;
        if self.backup_file.file_hash()? != (self.backup.file_len, self.backup.file_sha256) {
            return Err(GenerationMigrationError::BackupMismatch);
        }
        let txn = self
            .prepared
            .db
            .begin_write()
            .map_err(|e| backend("migration transaction", e))?;
        stage_migration_with_observer(&txn, &self.prepared.report, observe)?;
        // A large source scan may take time. Recheck immediately before commit,
        // including path replacements and a raw writer bypassing advisory locks.
        self.prepared.source.verify_path()?;
        if self.backup_file.file_hash()? != (self.backup.file_len, self.backup.file_sha256) {
            return Err(GenerationMigrationError::BackupMismatch);
        }
        txn.commit().map_err(|e| backend("migration commit", e))?;
        Ok(GenerationMigrationOutcome {
            report: self.prepared.report,
            backup: self.backup,
        })
    }
}

/// Shared production/cutpoint helper: no caller reports success before commit.
#[cfg(test)]
fn stage_migration(
    txn: &redb::WriteTransaction,
    report: &GenerationMigrationReport,
) -> MigrationResult<()> {
    stage_migration_with_observer(txn, report, |_| Ok(()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationStage {
    TableCreated,
    DescriptorInserted(usize),
    MarkerWritten,
}

fn stage_migration_with_observer(
    txn: &redb::WriteTransaction,
    report: &GenerationMigrationReport,
    mut observe: impl FnMut(MigrationStage) -> MigrationResult<()>,
) -> MigrationResult<()> {
    let scan = validation::scan(txn, &report.declaration, &report.source_path)?;
    if scan != report.scan || scan.format != STORE_FORMAT_VERSION {
        return Err(GenerationMigrationError::SourceChanged);
    }
    {
        let mut table = txn
            .open_table(GENERATION_TABLE)
            .map_err(|e| backend("create identity table", e))?;
        observe(MigrationStage::TableCreated)?;
        let identity = report.declaration.identity().encode();
        for (index, key) in report.declaration.scopes().iter().enumerate() {
            let key = key.encode();
            table
                .insert(key.as_slice(), identity.as_slice())
                .map_err(|e| backend("insert migration identity", e))?;
            observe(MigrationStage::DescriptorInserted(index))?;
        }
    }
    txn.open_table(META_TABLE)
        .map_err(|e| backend("migration marker table", e))?
        .insert(META_FORMAT_KEY, GENERATION_STORE_FORMAT_VERSION)
        .map_err(|e| backend("migration marker", e))?;
    observe(MigrationStage::MarkerWritten)?;
    Ok(())
}
