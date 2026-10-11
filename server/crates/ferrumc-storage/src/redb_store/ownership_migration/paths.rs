//! Caller authority is resolved before creating any offline evidence.

use super::super::uuid_migration::files;
use super::{io, Error, OwnershipMigrationFileRole as Role, Result};
use std::path::{Path, PathBuf};

/// Explicit destinations; all evidence must be new and independent of inputs.
#[derive(Debug)]
pub struct OwnershipMigrationOutputPaths {
    pub(super) backup: PathBuf,
    pub(super) plan: PathBuf,
    pub(super) stage: PathBuf,
    pub(super) checkpoint: PathBuf,
    pub(super) scratch: PathBuf,
}
impl OwnershipMigrationOutputPaths {
    /// Resolves exact caller-selected destinations and a local scratch directory.
    pub fn new(
        backup: impl AsRef<Path>,
        plan: impl AsRef<Path>,
        stage: impl AsRef<Path>,
        checkpoint: impl AsRef<Path>,
        scratch_directory: impl AsRef<Path>,
    ) -> Result<Self> {
        let paths = Self {
            backup: path(backup.as_ref())?,
            plan: path(plan.as_ref())?,
            stage: path(stage.as_ref())?,
            checkpoint: path(checkpoint.as_ref())?,
            scratch: directory(scratch_directory.as_ref())?,
        };
        distinct(&[&paths.backup, &paths.plan, &paths.stage, &paths.checkpoint])?;
        Ok(paths)
    }
}

/// Exact current caller paths required for resume; checkpoint paths grant no authority.
#[derive(Debug)]
pub struct OwnershipMigrationResumePaths {
    pub(super) source: PathBuf,
    pub(super) mapping: PathBuf,
    pub(super) output: OwnershipMigrationOutputPaths,
}
impl OwnershipMigrationResumePaths {
    /// Resolves all six paths without opening any checkpoint-supplied path.
    pub fn new(
        source: impl AsRef<Path>,
        mapping: impl AsRef<Path>,
        output: OwnershipMigrationOutputPaths,
    ) -> Result<Self> {
        let paths = Self {
            source: path(source.as_ref())?,
            mapping: path(mapping.as_ref())?,
            output,
        };
        distinct(&[
            &paths.source,
            &paths.mapping,
            &paths.output.backup,
            &paths.output.plan,
            &paths.output.stage,
            &paths.output.checkpoint,
        ])?;
        Ok(paths)
    }
}

pub(super) fn path(path: &Path) -> Result<PathBuf> {
    let path = files::canonical_path(path)?;
    path_bytes(&path)?;
    Ok(path)
}
pub(super) fn directory(path: &Path) -> Result<PathBuf> {
    let canonical =
        std::fs::canonicalize(path).map_err(|e| io("resolve directory", Role::Scratch, e))?;
    if !canonical.is_dir() {
        return Err(Error::MalformedCheckpoint("scratch is not a directory"));
    }
    path_bytes(&canonical)?;
    Ok(canonical)
}
pub(super) fn distinct(paths: &[&PathBuf]) -> Result<()> {
    for (index, path) in paths.iter().enumerate() {
        if paths[..index].contains(path) {
            return Err(Error::PathAlias);
        }
    }
    Ok(())
}
pub(super) fn path_bytes(path: &Path) -> Result<&[u8]> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes();
        if !path.is_absolute() || bytes.len() > 4096 || bytes.contains(&0) {
            return Err(Error::MalformedCheckpoint("canonical path extent"));
        }
        Ok(bytes)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(Error::UnsupportedPlatform)
    }
}
