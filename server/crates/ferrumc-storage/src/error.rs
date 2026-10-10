//! The classifying error type for the storage layer.

use ferrumc_core::ServerError;
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_world::{BlockEntityCodecError, GenerationError, GenerationIdentity};

use crate::{GenerationKey, JournalBatchId};

/// A classified storage failure.
///
/// Every variant names *why* an operation failed so callers can react
/// programmatically instead of parsing message strings. Storage methods on the
/// public traits surface [`ferrumc_core::ServerError`]; this type is the
/// crate-internal classification that converts into it through
/// [`From<StorageError> for ServerError`](ServerError). Constructors such as
/// [`crate::StorageKey::new`] and [`crate::EntityRecord::new`] return it
/// directly so callers can match on the precise reason.
///
/// The enum is `#[non_exhaustive]`, so new variants may be added later and
/// downstream `match`es must include a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StorageError {
    /// This instance deliberately uses the legacy metadata-free contract.
    #[error("generation metadata is not supported by this storage mode")]
    GenerationMetadataUnsupported,

    /// The explicit format-4 opener cannot bind or migrate existing format 3.
    #[error("generation metadata requires explicit offline migration from database format 3 to 4")]
    GenerationMigrationRequired,

    /// A save or initialization lacks the required immutable baseline.
    #[error("missing generation identity for {key:?}; world state cannot be saved or rebound")]
    MissingGenerationIdentity {
        /// Requested scope, or `None` for an unscoped player/journal admission.
        key: Option<GenerationKey>,
    },

    /// A requested baseline disagrees with the immutable stored identity.
    #[error(
        "generation identity mismatch for {key:?}: stored {stored:?}, requested {requested:?}"
    )]
    GenerationIdentityMismatch {
        /// Scope whose baseline must remain unchanged.
        key: GenerationKey,
        /// Previously committed baseline.
        stored: GenerationIdentity,
        /// Incompatible caller proposal.
        requested: GenerationIdentity,
    },

    /// A durable metadata key is not exactly eight bytes.
    #[error("malformed generation key: {len} bytes (expected 8)")]
    MalformedGenerationKey {
        /// Actual durable key length.
        len: usize,
    },

    /// A durable identity is malformed or selects unsupported generation rules.
    #[error("cannot decode generation identity for {key:?}: {source}")]
    GenerationIdentityDecode {
        /// Scope whose persisted descriptor failed validation.
        key: GenerationKey,
        /// Exact WG-001 decoder failure.
        #[source]
        source: GenerationError,
    },

    /// Adding a new identity would exceed the store's metadata bound.
    #[error("generation identity limit reached (maximum {max})")]
    GenerationIdentityLimit {
        /// Maximum permitted descriptor count.
        max: usize,
    },
    /// A plugin storage key exceeded the maximum accepted byte length and was
    /// rejected before being stored, to bound work done on untrusted input.
    #[error("storage key too long: {len} bytes (maximum {max})")]
    KeyTooLong {
        /// Length of the rejected key, in bytes.
        len: usize,
        /// Maximum accepted key length, in bytes.
        max: usize,
    },

    /// A plugin storage value exceeded the maximum accepted byte length and was
    /// rejected before being stored.
    #[error("storage value too large: {len} bytes (maximum {max})")]
    ValueTooLarge {
        /// Length of the rejected value, in bytes.
        len: usize,
        /// Maximum accepted value length, in bytes.
        max: usize,
    },

    /// A record's serialized payload exceeded the maximum accepted byte length
    /// and was rejected when the record was constructed.
    #[error("record payload too large: {len} bytes (maximum {max})")]
    RecordTooLarge {
        /// Length of the rejected payload, in bytes.
        len: usize,
        /// Maximum accepted payload length, in bytes.
        max: usize,
    },

    /// A block entity in a full chunk snapshot exceeded the storage codec's
    /// maximum payload length.
    #[error("chunk block entity at {pos:?} is {len} bytes when encoded (maximum {max})")]
    ChunkBlockEntityTooLarge {
        /// Position of the block entity whose payload exceeded the bound.
        pos: BlockPos,
        /// Encoded payload length, in bytes.
        len: usize,
        /// Maximum accepted encoded payload length, in bytes.
        max: usize,
    },

    /// A block entity cannot be represented losslessly by the world-owned
    /// persistence payload codec.
    #[error("chunk block entity at {pos:?} cannot be represented losslessly by the storage codec")]
    ChunkBlockEntityNotRepresentable {
        /// Position of the block entity that would lose data.
        pos: BlockPos,
    },

    /// The checked world-owned codec rejected an entity without dropping data.
    #[error("invalid chunk block entity at {pos:?}: {source}")]
    BlockEntityCodec {
        /// Position of the rejected entity.
        pos: BlockPos,
        /// Classified world/item encoding or decoding failure.
        #[source]
        source: BlockEntityCodecError,
    },

    /// The chunk coordinate cannot be represented by this model's block positions.
    #[error("chunk position {pos:?} is outside the i32 block-coordinate domain")]
    ChunkPositionOutsideDomain {
        /// Rejected chunk column.
        pos: ChunkPos,
    },

    /// A capture, apply, or save paired data with another chunk column.
    #[error("chunk position mismatch: expected {expected:?}, found {actual:?}")]
    ChunkPositionMismatch {
        /// Column named by the operation or record.
        expected: ChunkPos,
        /// Column named by its source/target chunk.
        actual: ChunkPos,
    },

    /// A stored block entity lies outside its parent chunk's column or height.
    #[error("block entity at {pos:?} is outside chunk {chunk:?}")]
    BlockEntityOutsideChunk {
        /// Parent chunk column.
        chunk: ChunkPos,
        /// Rejected block entity position.
        pos: BlockPos,
    },

    /// An overlay supplied the same block entity position more than once.
    #[error("duplicate block entity at {pos:?}")]
    DuplicateBlockEntity {
        /// Position whose duplicate was rejected.
        pos: BlockPos,
    },

    /// Bytes remained after a complete full chunk record.
    #[error("trailing bytes after full chunk record: {remaining}")]
    TrailingChunkRecordBytes {
        /// Number of bytes left after the complete record.
        remaining: usize,
    },

    /// Persisted bytes were written by an incompatible pre-alpha format.
    #[error(
        "This data was created by an incompatible pre-alpha build. Back it up or delete it before starting this release."
    )]
    IncompatiblePreAlphaData,

    /// A batched save request carried more items than the backend accepts in one
    /// call and was rejected before any item was processed.
    #[error("save batch too large: {len} items (maximum {max})")]
    BatchTooLarge {
        /// Number of items in the rejected batch.
        len: usize,
        /// Maximum accepted number of items per batch.
        max: usize,
    },

    /// The durable block-mutation journal has no sequence IDs left for the
    /// requested batch. The append is rejected before any record is written.
    #[error(
        "mutation journal sequence exhausted after id {last_id} while allocating {requested} records"
    )]
    JournalSequenceExhausted {
        /// Greatest sequence ID that was already durably allocated.
        last_id: u64,
        /// Number of records the rejected append attempted to allocate.
        requested: usize,
    },

    /// A supposedly fresh journal sequence ID already existed on disk.
    ///
    /// This indicates corrupt or inconsistent sequence metadata. The whole
    /// append transaction is rejected rather than replacing history.
    #[error("mutation journal sequence id {id} already exists")]
    JournalIdCollision {
        /// Durable sequence ID that unexpectedly already existed.
        id: u64,
    },

    /// An idempotency token was reused with a different normalized mutation
    /// payload than the batch it already identifies.
    #[error("mutation journal batch id {batch_id} was reused with a different payload")]
    JournalBatchConflict {
        /// Reused idempotency token.
        batch_id: JournalBatchId,
    },

    /// A persisted journal receipt did not have the required fixed-width encoding.
    #[error(
        "malformed mutation journal receipt for batch {batch_id}: {len} bytes (expected {expected})"
    )]
    MalformedJournalReceipt {
        /// Batch whose receipt was malformed.
        batch_id: JournalBatchId,
        /// Length of the persisted receipt.
        len: usize,
        /// Required fixed-width receipt length.
        expected: usize,
    },

    /// A persisted journal receipt described an impossible or over-limit range.
    #[error(
        "invalid mutation journal receipt range for batch {batch_id}: first={first_id}, count={count}"
    )]
    InvalidJournalReceiptRange {
        /// Batch whose receipt was inconsistent.
        batch_id: JournalBatchId,
        /// First durable ID encoded by the receipt.
        first_id: u64,
        /// Number of records encoded by the receipt.
        count: u64,
    },

    /// A receipt referenced a journal record that was not present.
    #[error("mutation journal receipt for batch {batch_id} references missing record {id}")]
    JournalReceiptMissingRecord {
        /// Batch whose committed range was incomplete.
        batch_id: JournalBatchId,
        /// Missing durable journal sequence ID.
        id: u64,
    },

    /// A persisted mutation-journal key did not have the required eight-byte
    /// big-endian representation.
    #[error("malformed mutation journal key: {len} bytes (expected 8)")]
    MalformedJournalKey {
        /// Length of the malformed durable key, in bytes.
        len: usize,
    },

    /// The storage backend failed in an unexpected way (for example, an internal
    /// lock was poisoned by a panic in another thread). The payload describes
    /// the failure for diagnostics.
    #[error("storage backend failure: {0}")]
    Backend(String),
}

impl StorageError {
    /// Builds a [`StorageError::Backend`] describing an unexpected backend
    /// failure.
    pub fn backend(detail: impl Into<String>) -> Self {
        Self::Backend(detail.into())
    }
}

impl From<StorageError> for ServerError {
    fn from(err: StorageError) -> Self {
        let message = err.to_string();
        match err {
            // Every bound rejection is a capacity-class failure: the request
            // asked for more than a bounded resource permits.
            StorageError::KeyTooLong { .. }
            | StorageError::ValueTooLarge { .. }
            | StorageError::RecordTooLarge { .. }
            | StorageError::ChunkBlockEntityTooLarge { .. }
            | StorageError::BatchTooLarge { .. }
            | StorageError::GenerationIdentityLimit { .. }
            | StorageError::JournalSequenceExhausted { .. } => Self::capacity(message),
            // A backend failure is an internal invariant violation.
            StorageError::JournalIdCollision { .. }
            | StorageError::MalformedJournalReceipt { .. }
            | StorageError::InvalidJournalReceiptRange { .. }
            | StorageError::JournalReceiptMissingRecord { .. }
            | StorageError::MalformedJournalKey { .. }
            | StorageError::MalformedGenerationKey { .. }
            | StorageError::GenerationIdentityDecode { .. }
            | StorageError::ChunkBlockEntityNotRepresentable { .. }
            | StorageError::ChunkPositionOutsideDomain { .. }
            | StorageError::ChunkPositionMismatch { .. }
            | StorageError::BlockEntityOutsideChunk { .. }
            | StorageError::DuplicateBlockEntity { .. }
            | StorageError::TrailingChunkRecordBytes { .. }
            | StorageError::Backend(_) => Self::internal(message),
            StorageError::JournalBatchConflict { .. }
            | StorageError::IncompatiblePreAlphaData
            | StorageError::GenerationMigrationRequired
            | StorageError::MissingGenerationIdentity { .. }
            | StorageError::GenerationIdentityMismatch { .. } => Self::invalid_state(message),
            StorageError::GenerationMetadataUnsupported => Self::unsupported(message),
            StorageError::BlockEntityCodec { source, .. } => {
                if matches!(
                    source,
                    BlockEntityCodecError::PayloadTooLarge { .. }
                        | BlockEntityCodecError::StackTooLarge { .. }
                        | BlockEntityCodecError::StringTooLong { .. }
                ) {
                    Self::capacity(message)
                } else {
                    Self::internal(message)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_causes_keep_distinct_server_error_classes() {
        let key = GenerationKey::new(
            ferrumc_core::WorldId::new(1),
            ferrumc_core::DimensionId::new(2),
        );
        let stored = GenerationIdentity::flat_overworld(ferrumc_world::WorldSeed::new(0));
        let requested = GenerationIdentity::flat_overworld(ferrumc_world::WorldSeed::new(1));
        assert!(matches!(
            ServerError::from(StorageError::GenerationMetadataUnsupported),
            ServerError::Unsupported(_)
        ));
        for error in [
            StorageError::GenerationMigrationRequired,
            StorageError::MissingGenerationIdentity { key: Some(key) },
            StorageError::MissingGenerationIdentity { key: None },
            StorageError::GenerationIdentityMismatch {
                key,
                stored,
                requested,
            },
        ] {
            assert!(matches!(
                ServerError::from(error),
                ServerError::InvalidState(_)
            ));
        }
        assert!(matches!(
            ServerError::from(StorageError::GenerationIdentityLimit { max: 4096 }),
            ServerError::Capacity(_)
        ));
        let decode = StorageError::GenerationIdentityDecode {
            key,
            source: GenerationError::InvalidLength {
                actual: 26,
                expected: 27,
            },
        };
        assert!(std::error::Error::source(&decode).is_some());
        for error in [StorageError::MalformedGenerationKey { len: 7 }, decode] {
            assert!(matches!(
                ServerError::from(error),
                ServerError::Internal { .. }
            ));
        }
    }

    #[test]
    fn bound_errors_map_to_capacity() {
        for err in [
            StorageError::KeyTooLong { len: 9, max: 8 },
            StorageError::ValueTooLarge { len: 9, max: 8 },
            StorageError::RecordTooLarge { len: 9, max: 8 },
            StorageError::ChunkBlockEntityTooLarge {
                pos: BlockPos::new(0, 0, 0),
                len: 9,
                max: 8,
            },
            StorageError::BatchTooLarge { len: 9, max: 8 },
            StorageError::JournalSequenceExhausted {
                last_id: u64::MAX,
                requested: 1,
            },
        ] {
            let converted: ServerError = err.into();
            assert!(matches!(converted, ServerError::Capacity(_)));
        }
    }

    #[test]
    fn backend_error_maps_to_internal() {
        let err = StorageError::backend("lock poisoned");
        assert_eq!(err.to_string(), "storage backend failure: lock poisoned");
        let converted: ServerError = err.into();
        assert!(matches!(converted, ServerError::Internal { .. }));
    }

    #[test]
    fn journal_collision_maps_to_internal() {
        let err = StorageError::JournalIdCollision { id: 7 };
        let converted: ServerError = err.into();
        assert!(matches!(converted, ServerError::Internal { .. }));
    }

    #[test]
    fn journal_batch_conflict_maps_to_invalid_state() {
        let err = StorageError::JournalBatchConflict {
            batch_id: JournalBatchId::from_bytes([7; 16]),
        };
        let converted: ServerError = err.into();
        assert!(matches!(converted, ServerError::InvalidState(_)));
    }

    #[test]
    fn malformed_journal_key_maps_to_internal() {
        let err = StorageError::MalformedJournalKey { len: 7 };
        let converted: ServerError = err.into();
        assert!(matches!(converted, ServerError::Internal { .. }));
    }
}
