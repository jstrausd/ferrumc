//! Immutable, explicitly enabled generation-baseline metadata.

use ferrumc_core::{DimensionId, WorldId};
use ferrumc_world::GenerationIdentity;

use crate::StorageError;

/// Maximum number of immutable world/dimension identities in one store.
pub const MAX_GENERATION_IDENTITIES: usize = 4096;

/// Addresses the generator baseline of one world/dimension scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GenerationKey {
    world: WorldId,
    dimension: DimensionId,
}

impl GenerationKey {
    /// Builds the metadata key for the supplied typed scope handles.
    #[must_use]
    pub const fn new(world: WorldId, dimension: DimensionId) -> Self {
        Self { world, dimension }
    }

    /// Returns the world whose generated baseline is bound.
    #[must_use]
    pub const fn world(self) -> WorldId {
        self.world
    }

    /// Returns the dimension handle, independently of the identity's dimension kind.
    #[must_use]
    pub const fn dimension(self) -> DimensionId {
        self.dimension
    }

    pub(crate) fn encode(self) -> [u8; 8] {
        let mut encoded = [0; 8];
        encoded[..4].copy_from_slice(&self.world.get().to_be_bytes());
        encoded[4..].copy_from_slice(&self.dimension.get().to_be_bytes());
        encoded
    }

    pub(crate) fn decode(encoded: &[u8]) -> std::result::Result<Self, StorageError> {
        let bytes: &[u8; 8] = encoded
            .try_into()
            .map_err(|_| StorageError::MalformedGenerationKey { len: encoded.len() })?;
        Ok(Self::new(
            WorldId::new(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
            DimensionId::new(u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])),
        ))
    }
}

/// Result of confirming an immutable generation baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationInitialization {
    /// The requested identity was first inserted and committed successfully.
    Initialized,
    /// The exact requested identity had already been committed.
    AlreadyPresent,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationMode {
    #[default]
    Legacy,
    Metadata,
}

pub(crate) fn decode_identity(
    key: GenerationKey,
    bytes: &[u8],
) -> std::result::Result<GenerationIdentity, StorageError> {
    GenerationIdentity::decode(bytes)
        .map_err(|source| StorageError::GenerationIdentityDecode { key, source })
}

pub(crate) fn compare_identity(
    key: GenerationKey,
    stored: GenerationIdentity,
    requested: GenerationIdentity,
) -> std::result::Result<GenerationInitialization, StorageError> {
    if stored == requested {
        Ok(GenerationInitialization::AlreadyPresent)
    } else {
        Err(StorageError::GenerationIdentityMismatch {
            key,
            stored,
            requested,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_codec_is_exact_big_endian_and_bounded() {
        let key = GenerationKey::new(WorldId::new(0x0102_0304), DimensionId::new(u32::MAX));
        assert_eq!(key.encode(), [1, 2, 3, 4, 255, 255, 255, 255]);
        assert_eq!(GenerationKey::decode(&key.encode()).unwrap(), key);
        assert_eq!(
            GenerationKey::decode(&[0; 8]).unwrap(),
            GenerationKey::new(WorldId::new(0), DimensionId::new(0))
        );
        for len in [0, 1, 7, 9, 32] {
            assert_eq!(
                GenerationKey::decode(&vec![0; len]),
                Err(StorageError::MalformedGenerationKey { len })
            );
        }
    }
}
