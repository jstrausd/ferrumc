//! Bounded ownership of CPU generation tasks outside async simulation work.

use std::num::NonZeroUsize;
use std::sync::Arc;

use ferrumc_math::ChunkPos;
use ferrumc_world::{Chunk, FallibleChunkGenerator};
use tokio::sync::Semaphore;

use crate::{GenerationBinding, SimError, SimResult};

/// Hard ceiling on admitted generation work, including cancelled callers.
pub const MAX_CONCURRENT_CHUNK_GENERATIONS: usize = 64;

/// Shared, non-waiting admission for CPU work on Tokio's blocking pool.
///
/// Clone this executor across all loaders in one world/server. Admission uses
/// an immediate try-acquire: a full executor returns `ChunkGenerationBusy`
/// without submitting work or queuing a waiter. The blocking task owns its
/// credit until it exits, even if the requesting future is cancelled. Tokio's
/// blocking-pool submission queue therefore contains at most `capacity()`
/// tasks from this executor. The caller controls retries and residency.
#[derive(Debug, Clone)]
pub struct ChunkGenerationExecutor {
    permits: Arc<Semaphore>,
    capacity: usize,
}

impl ChunkGenerationExecutor {
    /// Creates shared admission, clamping the requested capacity to 64 tasks.
    ///
    /// A small value (normally one or two) bounds concurrent chunk memory and
    /// leaves CPU available for ticks. This requires a running Tokio runtime
    /// when a loader submits generation.
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        let capacity = capacity.get().min(MAX_CONCURRENT_CHUNK_GENERATIONS);
        Self {
            permits: Arc::new(Semaphore::new(capacity)),
            capacity,
        }
    }

    /// Returns the fixed shared maximum number of admitted generation tasks.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub(crate) async fn generate(
        &self,
        generator: Arc<dyn FallibleChunkGenerator>,
        binding: GenerationBinding,
        pos: ChunkPos,
    ) -> SimResult<Chunk> {
        binding.validate_generator_identity(generator.identity())?;
        let permit = self.permits.clone().try_acquire_owned().map_err(|_| {
            SimError::ChunkGenerationBusy {
                pos,
                capacity: self.capacity,
            }
        })?;
        tokio::task::spawn_blocking(move || {
            // The task, rather than its waiting caller, retains bounded ownership.
            let _permit = permit;
            binding.validate_generator_identity(generator.identity())?;
            let chunk = generator
                .try_generate_chunk(pos)
                .map_err(|source| SimError::ChunkGeneration { pos, source })?;
            binding.validate_generator_identity(generator.identity())?;
            Ok(chunk)
        })
        .await
        .map_err(|error| SimError::ChunkGenerationTask {
            pos,
            panicked: error.is_panic(),
        })?
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    use ferrumc_core::{DimensionId, WorldId};
    use ferrumc_storage::GenerationKey;
    use ferrumc_world::{FlatWorldGenerator, GenerationIdentity, WorldError, WorldSeed};
    use tokio::sync::oneshot;

    use super::*;

    fn binding() -> GenerationBinding {
        GenerationBinding::new(
            GenerationKey::new(WorldId::new(0), DimensionId::new(0)),
            GenerationIdentity::flat_overworld(WorldSeed::new(0)),
        )
    }

    fn executor() -> ChunkGenerationExecutor {
        ChunkGenerationExecutor::new(NonZeroUsize::new(1).unwrap())
    }

    struct BlockingGenerator {
        started: Mutex<Option<oneshot::Sender<()>>>,
        release: Mutex<std::sync::mpsc::Receiver<()>>,
        calls: AtomicUsize,
    }

    impl FallibleChunkGenerator for BlockingGenerator {
        fn identity(&self) -> GenerationIdentity {
            binding().identity()
        }

        fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if let Some(started) = self.started.lock().unwrap().take() {
                started.send(()).unwrap();
            }
            self.release.lock().unwrap().recv().unwrap();
            Ok(Chunk::new(pos))
        }
    }

    #[tokio::test]
    async fn saturation_has_no_wait_queue_and_cancelled_caller_keeps_worker_credit() {
        let executor = executor();
        let (started_tx, started_rx) = oneshot::channel();
        // One release command is sufficient: there is exactly one admitted job.
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        let generator = Arc::new(BlockingGenerator {
            started: Mutex::new(Some(started_tx)),
            release: Mutex::new(release_rx),
            calls: AtomicUsize::new(0),
        });
        let job_executor = executor.clone();
        let job_generator = generator.clone();
        let job = tokio::spawn(async move {
            job_executor
                .generate(job_generator, binding(), ChunkPos::ORIGIN)
                .await
        });
        started_rx.await.unwrap();
        let first_busy = executor
            .generate(generator.clone(), binding(), ChunkPos::ORIGIN)
            .await;
        job.abort();
        let aborted = job.await.unwrap_err();
        let after_abort = executor
            .generate(generator.clone(), binding(), ChunkPos::ORIGIN)
            .await;
        release_tx.send(()).unwrap();
        // Wait for actual blocking-task exit, not just the producer's return.
        let released_credit = executor.permits.clone().acquire_owned().await.unwrap();
        drop(released_credit);
        assert!(aborted.is_cancelled());
        let expected = Err(SimError::ChunkGenerationBusy {
            pos: ChunkPos::ORIGIN,
            capacity: 1,
        });
        assert_eq!(first_busy, expected);
        assert_eq!(after_abort, expected);
        assert_eq!(generator.calls.load(Ordering::Relaxed), 1);
        assert!(executor
            .generate(
                Arc::new(FlatWorldGenerator::new()),
                binding(),
                ChunkPos::ORIGIN
            )
            .await
            .is_ok());
    }

    struct PanickingGenerator;

    impl FallibleChunkGenerator for PanickingGenerator {
        fn identity(&self) -> GenerationIdentity {
            binding().identity()
        }

        fn try_generate_chunk(&self, _pos: ChunkPos) -> Result<Chunk, WorldError> {
            panic!("deliberately broken producer")
        }
    }

    #[tokio::test]
    async fn producer_panic_is_classified_and_releases_credit() {
        let executor = executor();
        assert_eq!(
            executor
                .generate(Arc::new(PanickingGenerator), binding(), ChunkPos::ORIGIN)
                .await,
            Err(SimError::ChunkGenerationTask {
                pos: ChunkPos::ORIGIN,
                panicked: true
            })
        );
        assert!(executor
            .generate(
                Arc::new(FlatWorldGenerator::new()),
                binding(),
                ChunkPos::ORIGIN
            )
            .await
            .is_ok());
    }

    struct ChangingIdentity(AtomicBool);

    impl FallibleChunkGenerator for ChangingIdentity {
        fn identity(&self) -> GenerationIdentity {
            GenerationIdentity::flat_overworld(WorldSeed::new(i64::from(
                self.0.load(Ordering::Relaxed),
            )))
        }

        fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
            self.0.store(true, Ordering::Relaxed);
            Ok(Chunk::new(pos))
        }
    }

    #[tokio::test]
    async fn identity_change_inside_worker_refuses_successful_body() {
        assert!(matches!(
            executor()
                .generate(
                    Arc::new(ChangingIdentity(AtomicBool::new(false))),
                    binding(),
                    ChunkPos::ORIGIN
                )
                .await,
            Err(SimError::GeneratorIdentityMismatch { .. })
        ));
    }

    #[test]
    fn extreme_capacity_is_clamped_before_semaphore_construction() {
        assert_eq!(
            ChunkGenerationExecutor::new(NonZeroUsize::new(usize::MAX).unwrap()).capacity(),
            MAX_CONCURRENT_CHUNK_GENERATIONS
        );
    }
}
