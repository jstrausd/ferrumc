//! Bounded speculative raw terrain preparation, separate from owner admission.
//!
//! This cache contains immutable generated baselines only. Stored snapshots and
//! gameplay overlays are read again when the driver acquires a real ticket.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use ferrumc_core::PlayerId;
use ferrumc_math::ChunkPos;
use ferrumc_world::{Chunk, FallibleChunkGenerator, WorldError};
use tokio::task::{JoinError, JoinSet};

use crate::world::RuntimeGeneration;

/// At most two blocking producers plus 64 completed raw baseline chunks.
pub(crate) const MAX_PREPARATION_JOBS: usize = 2;
pub(crate) const MAX_WARM_BASELINES: usize = 64;
pub(crate) const MAX_PREPARATION_TARGETS: usize = 64;
const MAX_PREPARATION_VIEWERS: usize = 1024; // AppConfig's validated connection ceiling.
const MAX_WARM_BYTES: usize = 8 * 1024 * 1024;
const MAX_BASELINE_BYTES: usize = 512 * 1024;
const MAX_FAILED_POSITIONS: usize = 128;

type Completion = Result<(ChunkPos, Result<Chunk, WorldError>), JoinError>;

/// Driver-owned bounded work and raw baseline cache. None of these own tickets.
pub(crate) struct ChunkPreparation {
    generator: Arc<dyn FallibleChunkGenerator>,
    targets: BTreeMap<PlayerId, VecDeque<ChunkPos>>,
    viewers: VecDeque<PlayerId>,
    running: BTreeSet<ChunkPos>,
    ready: BTreeMap<ChunkPos, (Arc<Chunk>, usize)>,
    ready_bytes: usize,
    failed: VecDeque<ChunkPos>,
    recent: VecDeque<ChunkPos>,
    jobs: JoinSet<(ChunkPos, Result<Chunk, WorldError>)>,
}

impl ChunkPreparation {
    pub(crate) fn new(generator: &RuntimeGeneration) -> Self {
        Self {
            generator: generator.baseline_generator(),
            targets: BTreeMap::new(),
            viewers: VecDeque::new(),
            running: BTreeSet::new(),
            ready: BTreeMap::new(),
            ready_bytes: 0,
            failed: VecDeque::new(),
            recent: VecDeque::new(),
            jobs: JoinSet::new(),
        }
    }

    /// Replace only this viewer's bounded priorities; other viewers keep their work.
    pub(crate) fn request(
        &mut self,
        viewer: PlayerId,
        positions: impl IntoIterator<Item = ChunkPos>,
    ) {
        if !self.targets.contains_key(&viewer) && self.targets.len() >= MAX_PREPARATION_VIEWERS {
            return;
        }
        let mut unique = BTreeSet::new();
        let mut queue = VecDeque::new();
        for pos in positions {
            if unique.len() >= MAX_PREPARATION_TARGETS {
                break;
            }
            if unique.insert(pos) && !self.failed.contains(&pos) {
                queue.push_back(pos);
            }
        }
        if self.targets.insert(viewer, queue).is_none() {
            self.viewers.push_back(viewer);
        }
        self.start_jobs();
    }

    /// Forget departed or failed-join priorities; admitted producers remain bounded.
    pub(crate) fn cancel(&mut self, viewer: PlayerId) {
        self.targets.remove(&viewer);
        self.viewers.retain(|queued| *queued != viewer);
    }

    pub(crate) fn retain_viewers(&mut self, active: &BTreeMap<PlayerId, String>) {
        self.targets.retain(|viewer, _| active.contains_key(viewer));
        self.viewers.retain(|viewer| active.contains_key(viewer));
    }

    fn next_target(&mut self) -> Option<ChunkPos> {
        for _ in 0..self.viewers.len() {
            let viewer = self.viewers.pop_front()?;
            self.viewers.push_back(viewer);
            let Some(queue) = self.targets.get_mut(&viewer) else {
                continue;
            };
            while let Some(pos) = queue.pop_front() {
                if !self.running.contains(&pos)
                    && !self.ready.contains_key(&pos)
                    && !self.failed.contains(&pos)
                {
                    return Some(pos);
                }
            }
        }
        None
    }

    fn start_jobs(&mut self) {
        while self.jobs.len() < MAX_PREPARATION_JOBS {
            let Some(pos) = self.next_target() else { break };
            let generator = Arc::clone(&self.generator);
            self.running.insert(pos);
            self.jobs
                .spawn_blocking(move || (pos, generator.try_generate_chunk(pos)));
        }
    }

    pub(crate) fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    pub(crate) async fn next_completion(&mut self) -> Option<Completion> {
        self.jobs.join_next().await
    }

    pub(crate) fn completed(&mut self, completion: Completion) -> anyhow::Result<()> {
        let (pos, result) = completion
            .map_err(|error| anyhow::anyhow!("raw chunk preparation worker failed: {error}"))?;
        self.running.remove(&pos);
        match result {
            Ok(chunk) => {
                if chunk.block_entity_count() != 0 {
                    anyhow::bail!("raw terrain baseline contains gameplay block entities");
                }
                if chunk.pos() != pos {
                    anyhow::bail!("prepared terrain column differs from requested column");
                }
                let bytes = match baseline_storage_charge(&chunk) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        self.remember_failure(pos);
                        tracing::warn!(%error, "prepared baseline is not encodable");
                        self.start_jobs();
                        return Ok(());
                    }
                };
                if bytes <= MAX_BASELINE_BYTES {
                    while self.ready.len() >= MAX_WARM_BASELINES
                        || self.ready_bytes + bytes > MAX_WARM_BYTES
                    {
                        let Some(oldest) = self.recent.pop_front() else {
                            break;
                        };
                        if let Some((_, released)) = self.ready.remove(&oldest) {
                            self.ready_bytes -= released;
                        }
                    }
                    self.ready_bytes += bytes;
                    self.ready.insert(pos, (Arc::new(chunk), bytes));
                    self.recent.retain(|old| *old != pos);
                    self.recent.push_back(pos);
                } else {
                    self.remember_failure(pos);
                    tracing::warn!(
                        bytes,
                        x = pos.x(),
                        z = pos.z(),
                        "prepared baseline exceeds cache allocation bound"
                    );
                }
            }
            Err(error) => {
                self.remember_failure(pos);
                tracing::warn!(%error, x = pos.x(), z = pos.z(), "raw chunk preparation refused; position memoized");
            }
        }
        self.start_jobs();
        Ok(())
    }

    fn remember_failure(&mut self, pos: ChunkPos) {
        if self.failed.len() >= MAX_FAILED_POSITIONS {
            self.failed.pop_front();
        }
        self.failed.push_back(pos);
    }

    pub(crate) fn ready(&mut self, pos: ChunkPos) -> Option<Arc<Chunk>> {
        let chunk = Arc::clone(&self.ready.get(&pos)?.0);
        self.recent.retain(|old| *old != pos);
        self.recent.push_back(pos);
        Some(chunk)
    }

    /// Await admitted blocking producers during stop; no replacement is started.
    pub(crate) async fn drain(&mut self) -> anyhow::Result<()> {
        self.targets.clear();
        self.viewers.clear();
        let mut failure = None;
        while let Some(result) = self.jobs.join_next().await {
            match result {
                Ok((pos, Err(error))) => {
                    tracing::warn!(%error, x = pos.x(), z = pos.z(), "raw producer refused during shutdown");
                }
                Ok((_, Ok(_))) => {}
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
        self.running.clear();
        if let Some(error) = failure {
            anyhow::bail!("chunk preparation shutdown failed: {error}");
        }
        Ok(())
    }
}

/// The network blob preserves palette representation, including formerly
/// populated sections that are now all air. Triple its size covers 32-bit direct
/// arrays (15-bit on wire) and indirect palette capacity growth. Fixed structs,
/// allocator allowance and biome storage are additional. This is not RSS.
fn baseline_storage_charge(chunk: &Chunk) -> Result<usize, WorldError> {
    let encoded = ferrumc_world::encode_chunk_section_data(chunk)?;
    Ok(std::mem::size_of::<Chunk>()
        + 4096
        + chunk.biomes().map_or(0, std::mem::size_of_val)
        + encoded.len() * 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_world::{FlatWorldGenerator, GenerationIdentity};
    use std::sync::{Condvar, Mutex};

    struct BlockingGenerator {
        entered: tokio::sync::mpsc::Sender<ChunkPos>,
        gate: Arc<(Mutex<bool>, Condvar)>,
    }
    impl FallibleChunkGenerator for BlockingGenerator {
        fn identity(&self) -> GenerationIdentity {
            FlatWorldGenerator::new().identity()
        }
        fn try_generate_chunk(&self, pos: ChunkPos) -> Result<Chunk, WorldError> {
            self.entered
                .blocking_send(pos)
                .expect("bounded entry observation");
            let (lock, ready) = &*self.gate;
            let mut released = lock.lock().expect("gate");
            while !*released {
                released = ready.wait(released).expect("gate wait");
            }
            Ok(Chunk::new(pos))
        }
    }

    fn fixture() -> ChunkPreparation {
        ChunkPreparation::new(&RuntimeGeneration::new(Arc::new(FlatWorldGenerator::new())))
    }

    #[tokio::test(start_paused = true)]
    async fn blocked_workers_are_bounded_and_allow_timer_progress_and_fair_viewers() {
        let (entered, mut entries) = tokio::sync::mpsc::channel(16);
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let mut preparation =
            ChunkPreparation::new(&RuntimeGeneration::new(Arc::new(BlockingGenerator {
                entered,
                gate: Arc::clone(&gate),
            })));
        let first = PlayerId::offline("first");
        let second = PlayerId::offline("second");
        preparation.request(first, (0..10).map(|x| ChunkPos::new(x, 0)));
        preparation.request(second, (100..110).map(|x| ChunkPos::new(x, 0)));
        assert_eq!(preparation.jobs.len(), MAX_PREPARATION_JOBS);
        entries.recv().await.expect("first running");
        entries.recv().await.expect("second running");
        assert!(entries.try_recv().is_err());
        let mut ticks = tokio::time::interval(std::time::Duration::from_millis(50));
        ticks.tick().await;
        tokio::time::advance(std::time::Duration::from_millis(250)).await;
        ticks.tick().await;
        assert_eq!(
            preparation.running.len(),
            2,
            "CPU cannot occupy the async timer lane"
        );
        *gate.0.lock().expect("release") = true;
        gate.1.notify_all();
        for _ in 0..6 {
            let completion = preparation.next_completion().await.expect("completion");
            preparation.completed(completion).expect("valid baseline");
        }
        assert!(
            preparation.ready.keys().any(|pos| pos.x() >= 100),
            "distant viewer progresses without replacing first viewer"
        );
        assert!(preparation.ready.keys().any(|pos| pos.x() < 100));
        preparation.cancel(first);
        preparation.cancel(second);
        preparation.drain().await.expect("admitted producers drain");
        assert!(preparation.targets.is_empty());
        assert!(preparation.viewers.is_empty());
    }

    #[tokio::test]
    async fn shutdown_drains_other_blocking_jobs_after_a_producer_panic() {
        let (entered, mut entries) = tokio::sync::mpsc::channel(2);
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let producer = BlockingGenerator {
            entered,
            gate: Arc::clone(&gate),
        };
        let mut preparation = fixture();
        let failed = preparation
            .jobs
            .spawn(async { panic!("controlled producer failure") });
        while !failed.is_finished() {
            tokio::task::yield_now().await;
        }
        preparation.jobs.spawn_blocking(move || {
            (
                ChunkPos::ORIGIN,
                producer.try_generate_chunk(ChunkPos::ORIGIN),
            )
        });
        entries.recv().await.expect("second producer admitted");
        let mut drain = Box::pin(preparation.drain());
        tokio::select! {
            biased;
            result = &mut drain => {
                *gate.0.lock().expect("release on regression") = true;
                gate.1.notify_all();
                panic!("drain returned before blocked producer: {result:?}");
            },
            () = tokio::task::yield_now() => {}
        }
        *gate.0.lock().expect("release second") = true;
        gate.1.notify_all();
        assert!(
            drain.await.is_err(),
            "panic stays observable after every producer is joined"
        );
        assert_eq!(preparation.jobs.len(), 0);
        assert_eq!(preparation.running.len(), 0);
    }

    #[tokio::test]
    async fn priorities_deduplicate_and_are_bounded_and_cancel_reclaims_viewer_capacity() {
        let mut preparation = fixture();
        let viewer = PlayerId::offline("bounded");
        preparation.request(viewer, (0..200).flat_map(|x| [ChunkPos::new(x, 0); 2]));
        assert!(
            preparation.targets[&viewer].len() + preparation.running.len()
                <= MAX_PREPARATION_TARGETS
        );
        assert_eq!(preparation.running.len(), 2);
        preparation.cancel(viewer);
        assert!(!preparation.targets.contains_key(&viewer));
        assert!(!preparation.viewers.contains(&viewer));
        preparation.drain().await.expect("drain");
        for index in 0..MAX_PREPARATION_VIEWERS {
            let viewer = PlayerId::offline(&format!("viewer{index}"));
            preparation.request(viewer, []);
        }
        let extra = PlayerId::offline("extra");
        preparation.request(extra, []);
        assert!(!preparation.targets.contains_key(&extra));
        preparation.cancel(viewer);
        preparation.retain_viewers(&BTreeMap::new());
        preparation.request(extra, []);
        assert!(preparation.targets.contains_key(&extra));
    }

    #[tokio::test]
    async fn late_results_have_no_owner_and_cache_count_bytes_and_lru_are_bounded() {
        let mut preparation = fixture();
        for x in 0..i32::try_from(MAX_WARM_BASELINES).expect("bounded constant") {
            let pos = ChunkPos::new(x, 0);
            preparation
                .completed(Ok((pos, Ok(Chunk::new(pos)))))
                .expect("raw completion");
        }
        preparation.ready(ChunkPos::ORIGIN).expect("refresh oldest");
        let pos = ChunkPos::new(64, 0);
        preparation
            .completed(Ok((pos, Ok(Chunk::new(pos)))))
            .expect("late completion");
        assert_eq!(preparation.ready.len(), MAX_WARM_BASELINES);
        assert!(preparation.ready.contains_key(&ChunkPos::ORIGIN));
        assert!(!preparation.ready.contains_key(&ChunkPos::new(1, 0)));
        for x in 100..200 {
            let pos = ChunkPos::new(x, 0);
            preparation
                .completed(Ok((pos, Ok(FlatWorldGenerator::new().generate(pos)))))
                .expect("populated completion");
            assert!(preparation.ready_bytes <= MAX_WARM_BYTES);
            assert!(preparation.ready.len() <= MAX_WARM_BASELINES);
        }
        assert!(
            preparation.targets.is_empty(),
            "late raw outputs confer no viewer ownership"
        );
        assert!(preparation.jobs.is_empty());
    }

    #[tokio::test]
    async fn deterministic_failures_are_memoized_and_failure_history_is_bounded() {
        let mut preparation = fixture();
        let refused = ChunkPos::ORIGIN;
        preparation
            .completed(Ok((
                refused,
                Err(WorldError::InvalidBitsPerEntry { bits: 0 }),
            )))
            .expect("producer refusal is retained");
        preparation.request(PlayerId::offline("failure"), [refused]);
        assert!(!preparation.has_jobs());
        for x in 0..200 {
            preparation.remember_failure(ChunkPos::new(x, 0));
        }
        assert_eq!(preparation.failed.len(), MAX_FAILED_POSITIONS);
    }
}
