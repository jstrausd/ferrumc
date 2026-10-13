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

#[derive(Default)]
struct ViewerTargets {
    // Unlike the consumed queues, this bounded set preserves the viewer's
    // complete current intent so other viewers' useful warm work is protected.
    wanted: BTreeSet<ChunkPos>,
    visible: VecDeque<ChunkPos>,
    speculative: VecDeque<ChunkPos>,
}

impl ViewerTargets {
    fn retain(&mut self, keep: impl Fn(&ChunkPos) -> bool) {
        self.wanted.retain(&keep);
        self.visible.retain(&keep);
        self.speculative.retain(keep);
    }
}

type Completion = Result<(ChunkPos, Result<Chunk, WorldError>), JoinError>;

/// Driver-owned bounded work and raw baseline cache. None of these own tickets.
pub(crate) struct ChunkPreparation {
    generator: Arc<dyn FallibleChunkGenerator>,
    targets: BTreeMap<PlayerId, ViewerTargets>,
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

    /// Replace this viewer's priorities, keeping all visible work ahead of
    /// speculative work across viewers without changing the producer bound.
    pub(crate) fn request(
        &mut self,
        viewer: PlayerId,
        visible: impl IntoIterator<Item = ChunkPos>,
        speculative: impl IntoIterator<Item = ChunkPos>,
    ) {
        if !self.targets.contains_key(&viewer) && self.targets.len() >= MAX_PREPARATION_VIEWERS {
            return;
        }
        let mut unique = BTreeSet::new();
        let mut targets = ViewerTargets::default();
        for (pos, visible) in visible
            .into_iter()
            .map(|pos| (pos, true))
            .chain(speculative.into_iter().map(|pos| (pos, false)))
        {
            if unique.len() >= MAX_PREPARATION_TARGETS {
                break;
            }
            if unique.insert(pos) && !self.failed.contains(&pos) {
                if visible {
                    targets.visible.push_back(pos);
                } else {
                    targets.speculative.push_back(pos);
                }
            }
        }
        targets.wanted = unique;
        if self.targets.insert(viewer, targets).is_none() {
            self.viewers.push_back(viewer);
        }
        self.discard_unwanted_ready();
        self.start_jobs();
    }

    fn discard_unwanted_ready(&mut self) {
        self.ready.retain(|pos, (_, bytes)| {
            let wanted = self
                .targets
                .values()
                .any(|targets| targets.wanted.contains(pos));
            if !wanted {
                self.ready_bytes -= *bytes;
            }
            wanted
        });
        self.recent.retain(|pos| self.ready.contains_key(pos));
    }

    /// Forget departed or failed-join priorities; admitted producers remain bounded.
    pub(crate) fn cancel(&mut self, viewer: PlayerId) {
        self.targets.remove(&viewer);
        self.viewers.retain(|queued| *queued != viewer);
        self.discard_unwanted_ready();
    }

    pub(crate) fn retain_viewers(&mut self, active: &BTreeMap<PlayerId, String>) {
        self.targets.retain(|viewer, _| active.contains_key(viewer));
        self.viewers.retain(|viewer| active.contains_key(viewer));
        self.discard_unwanted_ready();
    }

    fn next_target(&mut self) -> Option<ChunkPos> {
        // Round-robin fairness applies within each class. A viewer's speculative
        // queue cannot repeatedly occupy slots ahead of another viewer's view.
        for visible in [true, false] {
            if !visible && !self.has_speculative_capacity() {
                // A stable view may repeatedly publish more outer targets than
                // fit in the byte-bound cache. Stop before unused warm terrain
                // is evicted and generated again; real visible demand may evict.
                return None;
            }
            for _ in 0..self.viewers.len() {
                let viewer = self.viewers.pop_front()?;
                self.viewers.push_back(viewer);
                let Some(targets) = self.targets.get_mut(&viewer) else {
                    continue;
                };
                let queue = if visible {
                    &mut targets.visible
                } else {
                    &mut targets.speculative
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
        }
        None
    }

    fn has_speculative_capacity(&self) -> bool {
        self.ready.len() + self.jobs.len() < MAX_WARM_BASELINES
            && self.ready_bytes + (self.jobs.len() + 1) * MAX_BASELINE_BYTES <= MAX_WARM_BYTES
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

    pub(crate) fn completed(
        &mut self,
        completion: Completion,
        resident: impl Fn(ChunkPos) -> bool,
    ) -> anyhow::Result<()> {
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
                if resident(pos)
                    || !self
                        .targets
                        .values()
                        .any(|targets| targets.wanted.contains(&pos))
                {
                    // Another owner path admitted the column, or every viewer
                    // replaced/canceled its intent while this producer ran.
                    // Validate producer shape first, then drop obsolete output
                    // so it cannot saturate new directional preparation.
                    self.start_jobs();
                    return Ok(());
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

    /// Successful owner admission makes the raw cache copy redundant. Remove
    /// only those exact resident positions; unrelated viewer demand is preserved.
    pub(crate) fn admitted(&mut self, positions: &BTreeSet<ChunkPos>) {
        for pos in positions {
            if let Some((_, bytes)) = self.ready.remove(pos) {
                self.ready_bytes -= bytes;
            }
        }
        self.recent.retain(|pos| !positions.contains(pos));
        for targets in self.targets.values_mut() {
            targets.retain(|pos| !positions.contains(pos));
        }
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
        preparation.request(first, (0..10).map(|x| ChunkPos::new(x, 0)), []);
        preparation.request(second, (100..110).map(|x| ChunkPos::new(x, 0)), []);
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
            preparation
                .completed(completion, |_| false)
                .expect("valid baseline");
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
        preparation.request(viewer, (0..200).flat_map(|x| [ChunkPos::new(x, 0); 2]), []);
        assert!(
            preparation.targets[&viewer].visible.len() + preparation.running.len()
                <= MAX_PREPARATION_TARGETS
        );
        assert_eq!(preparation.running.len(), 2);
        preparation.cancel(viewer);
        assert!(!preparation.targets.contains_key(&viewer));
        assert!(!preparation.viewers.contains(&viewer));
        preparation.drain().await.expect("drain");
        for index in 0..MAX_PREPARATION_VIEWERS {
            let viewer = PlayerId::offline(&format!("viewer{index}"));
            preparation.request(viewer, [], []);
        }
        let extra = PlayerId::offline("extra");
        preparation.request(extra, [], []);
        assert!(!preparation.targets.contains_key(&extra));
        preparation.cancel(viewer);
        preparation.retain_viewers(&BTreeMap::new());
        preparation.request(extra, [], []);
        assert!(preparation.targets.contains_key(&extra));
    }

    #[tokio::test]
    async fn requested_results_have_no_owner_and_cache_count_bytes_and_lru_are_bounded() {
        let mut preparation = fixture();
        for viewer in 0..4 {
            preparation.targets.insert(
                PlayerId::offline(&format!("lru{viewer}")),
                ViewerTargets {
                    wanted: (viewer * 64..(viewer + 1) * 64)
                        .map(|x| ChunkPos::new(x, 0))
                        .collect(),
                    ..ViewerTargets::default()
                },
            );
        }
        for x in 0..i32::try_from(MAX_WARM_BASELINES).expect("bounded constant") {
            let pos = ChunkPos::new(x, 0);
            preparation
                .completed(Ok((pos, Ok(Chunk::new(pos)))), |_| false)
                .expect("raw completion");
        }
        preparation.ready(ChunkPos::ORIGIN).expect("refresh oldest");
        let pos = ChunkPos::new(64, 0);
        preparation
            .completed(Ok((pos, Ok(Chunk::new(pos)))), |_| false)
            .expect("late completion");
        assert_eq!(preparation.ready.len(), MAX_WARM_BASELINES);
        assert!(preparation.ready.contains_key(&ChunkPos::ORIGIN));
        assert!(!preparation.ready.contains_key(&ChunkPos::new(1, 0)));
        for x in 100..200 {
            let pos = ChunkPos::new(x, 0);
            preparation
                .completed(
                    Ok((pos, Ok(FlatWorldGenerator::new().generate(pos)))),
                    |_| false,
                )
                .expect("populated completion");
            assert!(preparation.ready_bytes <= MAX_WARM_BYTES);
            assert!(preparation.ready.len() <= MAX_WARM_BASELINES);
        }
        assert!(
            preparation
                .targets
                .values()
                .all(|targets| targets.visible.is_empty() && targets.speculative.is_empty()),
            "cached outputs confer no queued work or ticket ownership"
        );
        assert!(preparation.jobs.is_empty());
    }

    #[tokio::test]
    async fn deterministic_failures_are_memoized_and_failure_history_is_bounded() {
        let mut preparation = fixture();
        let refused = ChunkPos::ORIGIN;
        preparation
            .completed(
                Ok((refused, Err(WorldError::InvalidBitsPerEntry { bits: 0 }))),
                |_| false,
            )
            .expect("producer refusal is retained");
        preparation.request(PlayerId::offline("failure"), [refused], []);
        assert!(!preparation.has_jobs());
        for x in 0..200 {
            preparation.remember_failure(ChunkPos::new(x, 0));
        }
        assert_eq!(preparation.failed.len(), MAX_FAILED_POSITIONS);
    }

    #[test]
    fn visible_demand_precedes_other_viewers_speculation_and_rotates_fairly() {
        let mut preparation = fixture();
        let outer_viewer = PlayerId::offline("outer");
        let first = PlayerId::offline("first_visible");
        let second = PlayerId::offline("second_visible");
        preparation.viewers = VecDeque::from([outer_viewer, first, second]);
        preparation.targets.insert(
            outer_viewer,
            ViewerTargets {
                wanted: BTreeSet::new(),
                visible: VecDeque::new(),
                speculative: VecDeque::from([ChunkPos::new(100, 0)]),
            },
        );
        preparation.targets.insert(
            first,
            ViewerTargets {
                wanted: BTreeSet::new(),
                visible: VecDeque::from([ChunkPos::new(1, 0), ChunkPos::new(2, 0)]),
                speculative: VecDeque::new(),
            },
        );
        preparation.targets.insert(
            second,
            ViewerTargets {
                wanted: BTreeSet::new(),
                visible: VecDeque::from([ChunkPos::new(3, 0)]),
                speculative: VecDeque::new(),
            },
        );
        assert_eq!(preparation.next_target(), Some(ChunkPos::new(1, 0)));
        assert_eq!(preparation.next_target(), Some(ChunkPos::new(3, 0)));
        assert_eq!(preparation.next_target(), Some(ChunkPos::new(2, 0)));
        assert_eq!(preparation.next_target(), Some(ChunkPos::new(100, 0)));
        assert_eq!(preparation.next_target(), None);
    }

    #[test]
    fn full_warm_cache_stops_speculative_churn_but_not_visible_demand() {
        let mut preparation = fixture();
        let viewer = PlayerId::offline("capacity");
        preparation.viewers.push_back(viewer);
        preparation.targets.insert(
            viewer,
            ViewerTargets {
                wanted: BTreeSet::new(),
                visible: VecDeque::new(),
                speculative: VecDeque::from([ChunkPos::new(100, 0)]),
            },
        );
        // Exercise the byte-reservation boundary independently of terrain size.
        preparation.ready_bytes = MAX_WARM_BYTES - MAX_BASELINE_BYTES + 1;
        assert_eq!(preparation.next_target(), None);
        assert_eq!(
            preparation.targets[&viewer].speculative.len(),
            1,
            "a full cache neither evicts nor consumes the deferred outer target"
        );
        preparation
            .targets
            .get_mut(&viewer)
            .expect("viewer")
            .visible
            .push_back(ChunkPos::ORIGIN);
        assert_eq!(preparation.next_target(), Some(ChunkPos::ORIGIN));
        preparation.ready_bytes -= 1;
        assert_eq!(preparation.next_target(), Some(ChunkPos::new(100, 0)));
    }

    #[tokio::test]
    async fn direction_replacement_reclaims_stale_full_cache_and_protects_other_viewers() {
        let mut preparation = fixture();
        let moving = PlayerId::offline("moving");
        let other = PlayerId::offline("other");
        let old: Vec<_> = (0..16).map(|x| ChunkPos::new(x, 0)).collect();
        for &pos in &old {
            preparation
                .ready
                .insert(pos, (Arc::new(Chunk::new(pos)), MAX_BASELINE_BYTES));
            preparation.recent.push_back(pos);
            preparation.ready_bytes += MAX_BASELINE_BYTES;
        }
        preparation.request(moving, [], old.iter().copied());
        preparation.request(other, [], [old[0]]);
        assert!(
            !preparation.has_jobs(),
            "a stable saturated speculative cache pauses"
        );
        let new = [ChunkPos::new(-1, 0), ChunkPos::new(-2, 0)];
        preparation.request(moving, [], new);
        assert_eq!(preparation.ready.len(), 1);
        assert!(
            preparation.ready.contains_key(&old[0]),
            "another viewer's current intent is protected"
        );
        assert_eq!(preparation.ready_bytes, MAX_BASELINE_BYTES);
        assert_eq!(preparation.recent, VecDeque::from([old[0]]));
        assert_eq!(
            preparation.running,
            BTreeSet::from(new),
            "latest direction starts within the existing two producer slots"
        );
        preparation.cancel(moving);
        preparation.cancel(other);
        preparation
            .drain()
            .await
            .expect("bounded admitted jobs drain");
    }

    #[tokio::test]
    async fn canceled_and_replaced_plan_drops_late_outputs_after_shape_validation() {
        let mut preparation = fixture();
        let viewer = PlayerId::offline("late");
        let old = ChunkPos::ORIGIN;
        preparation.targets.insert(
            viewer,
            ViewerTargets {
                wanted: BTreeSet::from([old]),
                ..ViewerTargets::default()
            },
        );
        preparation.cancel(viewer);
        preparation
            .completed(Ok((old, Ok(Chunk::new(old)))), |_| false)
            .expect("obsolete valid producer is discarded");
        assert!(preparation.ready.is_empty());
        assert_eq!(preparation.ready_bytes, 0);
        preparation.targets.insert(
            viewer,
            ViewerTargets {
                wanted: BTreeSet::from([ChunkPos::new(1, 0)]),
                ..ViewerTargets::default()
            },
        );
        preparation
            .completed(Ok((old, Ok(Chunk::new(old)))), |_| false)
            .expect("replaced valid producer is discarded");
        assert!(preparation.ready.is_empty());
        assert!(
            preparation
                .completed(Ok((old, Ok(Chunk::new(ChunkPos::new(99, 0))))), |_| false)
                .is_err(),
            "obsolete output still validates exact producer position"
        );
        let mut invalid = Chunk::new(old);
        invalid
            .set_block_entity(
                ferrumc_math::BlockPos::new(1, 64, 1),
                ferrumc_world::BlockEntity::Chest(ferrumc_world::ChestInventory::new()),
            )
            .expect("test block entity");
        assert!(
            preparation
                .completed(Ok((old, Ok(invalid))), |_| false)
                .is_err(),
            "obsolete output still rejects gameplay block entities"
        );
        assert!(preparation.ready.is_empty());
    }

    #[tokio::test]
    async fn admission_releases_only_exact_raw_copies_and_shared_resident_targets() {
        let mut preparation = fixture();
        let resident = ChunkPos::ORIGIN;
        let untouched = ChunkPos::new(1, 0);
        preparation.targets.insert(
            PlayerId::offline("first"),
            ViewerTargets {
                wanted: BTreeSet::from([resident, untouched]),
                ..ViewerTargets::default()
            },
        );
        for pos in [resident, untouched] {
            preparation
                .completed(Ok((pos, Ok(Chunk::new(pos)))), |_| false)
                .expect("baseline");
        }
        let retained_bytes = preparation.ready[&untouched].1;
        for name in ["first", "second"] {
            let viewer = PlayerId::offline(name);
            preparation.viewers.push_back(viewer);
            preparation.targets.insert(
                viewer,
                ViewerTargets {
                    wanted: BTreeSet::from([resident, untouched, ChunkPos::new(2, 0)]),
                    visible: VecDeque::from([resident, untouched]),
                    speculative: VecDeque::from([ChunkPos::new(2, 0)]),
                },
            );
        }
        preparation.admitted(&BTreeSet::from([resident]));
        assert!(!preparation.ready.contains_key(&resident));
        assert_eq!(preparation.ready_bytes, retained_bytes);
        assert_eq!(preparation.recent, VecDeque::from([untouched]));
        for targets in preparation.targets.values() {
            assert_eq!(targets.visible, VecDeque::from([untouched]));
            assert_eq!(targets.speculative, VecDeque::from([ChunkPos::new(2, 0)]));
        }
        assert_eq!(
            preparation.next_target(),
            Some(ChunkPos::new(2, 0)),
            "a consumed resident baseline cannot be scheduled again from stale viewer priorities"
        );
        preparation.running.insert(resident);
        preparation
            .completed(Ok((resident, Ok(Chunk::new(resident)))), |pos| {
                pos == resident
            })
            .expect("late completion");
        assert!(!preparation.ready.contains_key(&resident));
        assert!(!preparation.running.contains(&resident));
        assert_eq!(preparation.ready_bytes, retained_bytes);
        preparation.cancel(PlayerId::offline("first"));
        preparation.cancel(PlayerId::offline("second"));
        preparation.drain().await.expect("no owner work remains");
    }
}
