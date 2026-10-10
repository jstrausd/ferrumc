//! Explicit, scope-bound admission of an immutable generated baseline.

use std::sync::{Arc, Weak};

use ferrumc_storage::{ChunkKey, GenerationKey, WorldStore};
use ferrumc_world::{ChunkGenerator, GenerationIdentity};

use crate::{SimError, SimResult};

/// The caller's explicitly selected baseline for one world/dimension scope.
///
/// This is a declaration, **not** proof that a store has accepted the identity.
/// The simulation never initializes or guesses missing metadata. Bootstrap or
/// an explicit offline migration must persist it first. Every bound load checks
/// it against the borrowed store; checked capture either checks it afresh or
/// uses a [`VerifiedGenerationBinding`] for the exact immutable target store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationBinding {
    key: GenerationKey,
    identity: GenerationIdentity,
}

impl GenerationBinding {
    /// Declares the desired identity and storage scope without any storage I/O.
    #[must_use]
    pub const fn new(key: GenerationKey, identity: GenerationIdentity) -> Self {
        Self { key, identity }
    }

    /// Returns the exact runtime/storage scope, independently of dimension kind.
    #[must_use]
    pub const fn key(self) -> GenerationKey {
        self.key
    }

    /// Returns the explicit generated baseline expected by this scope.
    #[must_use]
    pub const fn identity(self) -> GenerationIdentity {
        self.identity
    }

    /// Reads and checks existing immutable metadata, never initializing it.
    ///
    /// Missing metadata, an incompatible descriptor, or an unsupported/corrupt
    /// backend refuses admission. Backend failures keep their classification
    /// inside [`SimError::GenerationStore`], independently of a chunk miss.
    pub async fn validate_store(self, store: &dyn WorldStore) -> SimResult<()> {
        let stored = store
            .load_generation_identity(self.key)
            .await
            .map_err(|source| SimError::GenerationStore {
                key: self.key,
                source,
            })?
            .ok_or(SimError::GenerationIdentityMissing { key: self.key })?;
        if stored != self.identity {
            return Err(SimError::GenerationIdentityMismatch {
                key: self.key,
                expected: self.identity,
                stored,
            });
        }
        Ok(())
    }

    /// Checks metadata off the tick path and authorizes one exact store instance.
    ///
    /// The metadata contract of [`WorldStore`] is immutable. Consequently this
    /// authorization may be reused for synchronous captures into this same
    /// `Arc` allocation; it cannot authorize another store, scope or identity.
    /// Keep the driver and writer on clones of the same `Arc`. A custom backend
    /// that mutates or deletes confirmed metadata violates that contract.
    ///
    /// The returned token holds only a weak allocation identity. It neither
    /// owns a database handle nor extends the backend's lifetime.
    pub async fn verify_store(
        self,
        store: &Arc<dyn WorldStore>,
    ) -> SimResult<VerifiedGenerationBinding> {
        self.validate_store(store.as_ref()).await?;
        Ok(VerifiedGenerationBinding {
            binding: self,
            store: Arc::downgrade(store),
        })
    }

    pub(crate) fn validate_key(self, key: ChunkKey) -> SimResult<()> {
        let actual = GenerationKey::new(key.world(), key.dimension());
        if actual != self.key {
            return Err(SimError::GenerationScopeMismatch {
                expected: self.key,
                actual,
            });
        }
        Ok(())
    }

    pub(crate) fn validate_generator(self, generator: &dyn ChunkGenerator) -> SimResult<()> {
        let actual = generator.identity();
        if actual != self.identity {
            return Err(SimError::GeneratorIdentityMismatch {
                key: self.key,
                expected: self.identity,
                actual,
            });
        }
        Ok(())
    }
}

/// Opaque capture authorization for one verified, immutable target store.
///
/// Constructed only by [`GenerationBinding::verify_store`]. The private weak
/// store identity prevents a token from authorizing a different allocation,
/// even if it contains the same descriptor. Maps/shards never retain this token
/// or a database handle. Application code owns durable batch retries and must
/// preserve the same target store throughout capture, enqueue and save.
#[derive(Debug, Clone)]
pub struct VerifiedGenerationBinding {
    binding: GenerationBinding,
    store: Weak<dyn WorldStore>,
}

impl VerifiedGenerationBinding {
    /// Returns the declaration whose persisted identity was confirmed.
    #[must_use]
    pub const fn binding(&self) -> GenerationBinding {
        self.binding
    }

    pub(crate) fn validate(
        &self,
        binding: GenerationBinding,
        store: &Arc<dyn WorldStore>,
    ) -> SimResult<()> {
        if binding.key != self.binding.key {
            return Err(SimError::GenerationScopeMismatch {
                expected: binding.key,
                actual: self.binding.key,
            });
        }
        if binding.identity != self.binding.identity {
            return Err(SimError::GenerationIdentityMismatch {
                key: binding.key,
                expected: binding.identity,
                stored: self.binding.identity,
            });
        }
        if !Weak::ptr_eq(&self.store, &Arc::downgrade(store)) {
            return Err(SimError::GenerationStoreAuthorizationMismatch { key: binding.key });
        }
        Ok(())
    }
}
