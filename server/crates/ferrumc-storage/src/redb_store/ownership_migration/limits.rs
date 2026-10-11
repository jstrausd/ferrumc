//! Positive logical and physical offline budgets, separately enforced.

use std::num::{NonZeroU64, NonZeroUsize};

use super::{Error, Result};

/// Explicit positive per-operation budgets for the Format5-to-6 migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipMigrationLimits {
    pub(super) scan_rows: u64,
    pub(super) scan_bytes: u64,
    pub(super) mapping_rows: u64,
    pub(super) plan_rows: u64,
    pub(super) scratch_bytes: u64,
    pub(super) verification_copy_bytes: u64,
    pub(super) checkpoint_bytes: usize,
}

impl OwnershipMigrationLimits {
    /// Admits only positive budgets within the reviewed hard maximums.
    pub fn new(
        scan_rows: NonZeroU64,
        scan_bytes: NonZeroU64,
        mapping_rows: NonZeroU64,
        plan_rows: NonZeroU64,
        scratch_bytes: NonZeroU64,
        verification_copy_bytes: NonZeroU64,
        checkpoint_bytes: NonZeroUsize,
    ) -> Result<Self> {
        let limits = Self {
            scan_rows: scan_rows.get(),
            scan_bytes: scan_bytes.get(),
            mapping_rows: mapping_rows.get(),
            plan_rows: plan_rows.get(),
            scratch_bytes: scratch_bytes.get(),
            verification_copy_bytes: verification_copy_bytes.get(),
            checkpoint_bytes: checkpoint_bytes.get(),
        };
        limits.validate()?;
        Ok(limits)
    }

    pub(super) fn validate(self) -> Result<()> {
        let values = self.encoded();
        let maximums = [
            16_777_216,
            8 * 1024 * 1024 * 1024,
            1_048_576,
            1_048_576,
            512 * 1024 * 1024,
            32 * 1024 * 1024 * 1024,
            32 * 1024,
        ];
        if values
            .iter()
            .zip(maximums)
            .any(|(n, cap)| *n == 0 || *n > cap)
            || self
                .verification_copy_bytes
                .checked_mul(3)
                .and_then(|n| n.checked_add(self.scratch_bytes))
                .and_then(|n| n.checked_add(self.checkpoint_bytes as u64))
                .is_none()
            || usize::try_from(self.scan_bytes).is_err()
            || usize::try_from(self.scratch_bytes).is_err()
            || usize::try_from(self.verification_copy_bytes).is_err()
        {
            return Err(Error::InvalidLimit);
        }
        Ok(())
    }

    /// Returns the reviewed positive default budgets, with no unlimited setting.
    #[must_use]
    pub const fn defaults() -> Self {
        Self {
            scan_rows: 1_048_576,
            scan_bytes: 512 * 1024 * 1024,
            mapping_rows: 65_536,
            plan_rows: 131_072,
            scratch_bytes: 64 * 1024 * 1024,
            verification_copy_bytes: 2 * 1024 * 1024 * 1024,
            checkpoint_bytes: 32 * 1024,
        }
    }

    /// Maximum streamed logical rows in each complete database scan.
    pub const fn max_scan_rows(self) -> u64 {
        self.scan_rows
    }
    /// Maximum summed raw key/value bytes in each complete database scan.
    pub const fn max_scan_bytes(self) -> u64 {
        self.scan_bytes
    }
    /// Maximum explicit fixed-width player mapping rows.
    pub const fn max_mapping_rows(self) -> u64 {
        self.mapping_rows
    }
    /// Maximum immutable resource plan rows.
    pub const fn max_plan_rows(self) -> u64 {
        self.plan_rows
    }
    /// Maximum combined logical plan, count/index and metadata bytes.
    pub const fn max_scratch_bytes(self) -> u64 {
        self.scratch_bytes
    }
    /// Maximum actual physical extent of each Stage, Backup or verification copy.
    pub const fn max_verification_copy_bytes(self) -> u64 {
        self.verification_copy_bytes
    }
    /// Maximum actual immutable checkpoint envelope bytes.
    pub const fn max_checkpoint_bytes(self) -> usize {
        self.checkpoint_bytes
    }

    pub(super) const fn encoded(self) -> [u64; 7] {
        [
            self.scan_rows,
            self.scan_bytes,
            self.mapping_rows,
            self.plan_rows,
            self.scratch_bytes,
            self.verification_copy_bytes,
            self.checkpoint_bytes as u64,
        ]
    }

    pub(super) fn from_encoded(values: [u64; 7]) -> Result<Self> {
        let result = Self {
            scan_rows: values[0],
            scan_bytes: values[1],
            mapping_rows: values[2],
            plan_rows: values[3],
            scratch_bytes: values[4],
            verification_copy_bytes: values[5],
            checkpoint_bytes: usize::try_from(values[6]).map_err(|_| Error::InvalidLimit)?,
        };
        result.validate()?;
        Ok(result)
    }
}
