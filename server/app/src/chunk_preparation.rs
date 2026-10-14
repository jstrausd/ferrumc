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
                validate_baseline(&chunk)?;
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
                    // Census below measures the clone's lengths. Cloning removes spare
                    // producer Vec/String capacity without altering item/component bytes.
                    self.ready.insert(pos, (Arc::new(chunk.clone()), bytes));
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

/// A generated chest (including its loot) belongs to an immutable baseline.
/// Persist-dirty/edited chunks and malformed/mismatched entities do not.
fn validate_baseline(chunk: &Chunk) -> anyhow::Result<()> {
    use ferrumc_world::{is_chest_state, is_furnace_state, sign_kind_for_state, BlockEntity};
    if chunk.persist_dirty_sections().any() || chunk.persist_edited_sections().any() {
        anyhow::bail!("prepared terrain baseline contains persisted gameplay edits");
    }
    if chunk.light().is_some() {
        anyhow::bail!("prepared terrain baseline contains gameplay lighting");
    }
    let mut encoded = Vec::new();
    for (pos, entity) in chunk.block_entities() {
        let state = chunk.get_block(pos).ok_or_else(|| {
            anyhow::anyhow!("prepared baseline block entity is outside its column")
        })?;
        let matches = match entity {
            BlockEntity::Chest(_) => is_chest_state(state.into()),
            BlockEntity::Furnace(_) => is_furnace_state(state.into()),
            BlockEntity::Sign(sign) => sign_kind_for_state(state.into()) == Some(sign.kind()),
            _ => anyhow::bail!("prepared baseline has an unsupported block entity"),
        };
        if !matches {
            anyhow::bail!(
                "prepared baseline block entity disagrees with its block state at {pos:?}"
            );
        }
        encoded.clear();
        ferrumc_world::try_encode_block_entity(entity, &mut encoded)?;
    }
    Ok(())
}

/// Palette-preserving wire size ×3 covers cloned direct/indirect arrays; fixed
/// objects/allocator allowance, biome bytes and complete decoded entity clone
/// allocations are additional. This is a conservative cache charge, not RSS.
fn baseline_storage_charge(chunk: &Chunk) -> anyhow::Result<usize> {
    let encoded = ferrumc_world::encode_chunk_section_data(chunk)?;
    let mut bytes = std::mem::size_of::<Chunk>()
        + 4096
        + chunk.biomes().map_or(0, std::mem::size_of_val)
        + encoded.len() * 3;
    for (_, entity) in chunk.block_entities() {
        // Conservative single-entry worst BTree node (eleven inline entries,
        // twelve edges, header+allocator allowance) exceeds the amortized node.
        charge(
            &mut bytes,
            11 * (std::mem::size_of::<ferrumc_math::BlockPos>()
                + std::mem::size_of::<ferrumc_world::BlockEntity>())
                + 16 * std::mem::size_of::<usize>()
                + 64,
        )?;
        match entity {
            ferrumc_world::BlockEntity::Chest(chest) => {
                charge(&mut bytes, std::mem::size_of_val(chest.slots()) + 32)?;
                for stack in chest.slots() {
                    charge_stack_clone(&mut bytes, stack)?;
                }
            }
            ferrumc_world::BlockEntity::Furnace(furnace) => {
                for stack in furnace.slots() {
                    charge_stack_clone(&mut bytes, stack)?;
                }
            }
            ferrumc_world::BlockEntity::Sign(sign) => {
                for face in [sign.front(), sign.back()] {
                    charge(&mut bytes, face.color().len() + 32)?;
                    for line in face.lines() {
                        charge(&mut bytes, line.len() + 32)?;
                    }
                }
            }
            _ => anyhow::bail!("prepared baseline has an unsupported block entity"),
        }
    }
    Ok(bytes)
}
fn charge(bytes: &mut usize, amount: usize) -> anyhow::Result<()> {
    *bytes = bytes
        .checked_add(amount)
        .ok_or_else(|| anyhow::anyhow!("prepared baseline clone charge overflow"))?;
    Ok(())
}
fn charge_stack_clone(bytes: &mut usize, stack: &ferrumc_items::ItemStack) -> anyhow::Result<()> {
    use ferrumc_items::ComponentValue;
    charge(
        bytes,
        std::mem::size_of_val(stack.components().added())
            + std::mem::size_of_val(stack.components().removed())
            + 64,
    )?;
    for value in stack.components().added() {
        match value {
            ComponentValue::CustomData(tag) | ComponentValue::CustomName(tag) => {
                charge_nbt_clone(bytes, tag)?;
            }
            ComponentValue::Opaque(value) => charge(bytes, value.raw().len() + 32)?,
            _ => {}
        }
    }
    Ok(())
}
fn charge_nbt_clone(bytes: &mut usize, tag: &ferrumc_nbt::NbtTag) -> anyhow::Result<()> {
    use ferrumc_nbt::NbtTag;
    match tag {
        NbtTag::ByteArray(values) => charge(bytes, values.len() + 32)?,
        NbtTag::IntArray(values) => charge(bytes, values.len() * 4 + 32)?,
        NbtTag::LongArray(values) => charge(bytes, values.len() * 8 + 32)?,
        NbtTag::String(value) => charge(bytes, value.len() + 32)?,
        NbtTag::List(values) => {
            charge(bytes, values.len() * std::mem::size_of::<NbtTag>() + 32)?;
            for value in values {
                charge_nbt_clone(bytes, value)?;
            }
        }
        NbtTag::Compound(values) => {
            charge(
                bytes,
                values.len() * std::mem::size_of::<(String, NbtTag)>() + 32,
            )?;
            for (name, value) in values.iter() {
                charge(bytes, name.len() + 32)?;
                charge_nbt_clone(bytes, value)?;
            }
        }
        _ => {}
    }
    Ok(())
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

    fn generated_chest(stack: ferrumc_items::ItemStack) -> Chunk {
        use ferrumc_world::{BlockEntity, BlockStateId, ChestInventory};
        let mut chunk = Chunk::new(ChunkPos::ORIGIN);
        let at = ferrumc_math::BlockPos::new(1, 64, 1);
        chunk
            .set_block(
                at,
                BlockStateId::new(
                    ferrumc_registry::block_state::block_default_state("chest").unwrap(),
                ),
            )
            .unwrap();
        let mut chest = ChestInventory::new();
        *chest.slot_mut(0).unwrap() = stack;
        chunk
            .set_block_entity(at, BlockEntity::Chest(chest))
            .unwrap();
        chunk
    }

    #[test]
    fn generated_entities_validate_losslessly_but_gameplay_and_malformed_payloads_refuse() {
        use ferrumc_items::{ComponentPatch, ComponentValue, ItemId, ItemStack};
        use ferrumc_nbt::NbtTag;
        use ferrumc_world::{BlockEntity, BlockStateId, Sign, SignKind};
        let at = ferrumc_math::BlockPos::new(1, 64, 1);
        let baseline = generated_chest(ItemStack::empty());
        validate_baseline(&baseline).unwrap();
        let mut wrong_state = baseline.clone();
        wrong_state.set_block(at, BlockStateId::AIR).unwrap();
        assert!(validate_baseline(&wrong_state).is_err());
        let mut gameplay = baseline.clone();
        gameplay.mark_persist_dirty(at);
        assert!(validate_baseline(&gameplay).is_err());
        gameplay.clear_persist_dirty();
        assert!(gameplay.persist_edited_sections().any());
        assert!(validate_baseline(&gameplay).is_err());
        let malformed_stack = ItemStack::try_new(
            ItemId::from_name("bread").unwrap(),
            1,
            ComponentPatch::new(vec![ComponentValue::CustomData(NbtTag::Int(3))], vec![]),
        )
        .unwrap();
        assert!(validate_baseline(&generated_chest(malformed_stack)).is_err());
        let mut invalid_sign = Chunk::new(ChunkPos::ORIGIN);
        invalid_sign
            .set_block(
                at,
                BlockStateId::new(
                    ferrumc_registry::block_state::block_default_state("oak_sign").unwrap(),
                ),
            )
            .unwrap();
        let mut sign = Sign::new(SignKind::Sign);
        sign.set_face_lines(true, std::array::from_fn(|_| "x".repeat(1025)));
        invalid_sign
            .set_block_entity(at, BlockEntity::Sign(sign))
            .unwrap();
        assert!(validate_baseline(&invalid_sign).is_err());
    }

    #[tokio::test]
    async fn cache_charges_decoded_nbt_and_opaque_components_and_preserves_the_complete_clone() {
        use ferrumc_items::{
            ComponentPatch, ComponentTypeId, ComponentValue, ItemId, ItemStack, OpaqueComponent,
        };
        use ferrumc_nbt::{NbtCompound, NbtTag};
        let mut data = NbtCompound::new();
        data.push(
            "many",
            NbtTag::List(vec![NbtTag::Compound(NbtCompound::new()); 128]),
        );
        data.push("ints", NbtTag::IntArray(vec![7; 256]));
        data.push("longs", NbtTag::LongArray(vec![13; 128]));
        let mut label = String::with_capacity(64 * 1024);
        label.push_str("complete");
        data.push("label", NbtTag::String(label));
        let stack = ItemStack::try_new(
            ItemId::from_name("bread").unwrap(),
            1,
            ComponentPatch::new(
                vec![
                    ComponentValue::CustomData(NbtTag::Compound(data)),
                    ComponentValue::Opaque(OpaqueComponent::new(6, vec![0xa5; 512]).unwrap()),
                ],
                vec![ComponentTypeId::new(95)],
            ),
        )
        .unwrap();
        let source = generated_chest(stack);
        validate_baseline(&source).unwrap();
        let blank_charge = baseline_storage_charge(&generated_chest(ItemStack::empty())).unwrap();
        let charge = baseline_storage_charge(&source).unwrap();
        assert!(charge > blank_charge + 128 * std::mem::size_of::<NbtTag>() + 2048 + 512);
        let mut preparation = fixture();
        preparation.targets.insert(
            PlayerId::offline("components"),
            ViewerTargets {
                wanted: BTreeSet::from([source.pos()]),
                ..ViewerTargets::default()
            },
        );
        preparation
            .completed(Ok((source.pos(), Ok(source.clone()))), |_| false)
            .unwrap();
        let retained = preparation.ready(source.pos()).unwrap();
        assert_eq!(*retained, source);
        assert_eq!(preparation.ready_bytes, charge);
        let ferrumc_world::BlockEntity::Chest(chest) = retained
            .block_entity(ferrumc_math::BlockPos::new(1, 64, 1))
            .unwrap()
        else {
            panic!("chest")
        };
        let ComponentValue::CustomData(NbtTag::Compound(data)) =
            &chest.slots()[0].components().added()[0]
        else {
            panic!("complete data")
        };
        let (_, NbtTag::String(label)) = data.iter().find(|(name, _)| *name == "label").unwrap()
        else {
            panic!("label")
        };
        assert_eq!(
            label.capacity(),
            label.len(),
            "retained clone removes producer spare capacity"
        );
    }

    #[tokio::test]
    async fn oversized_valid_entity_baseline_is_memoized_without_a_driver_failure() {
        use ferrumc_math::BlockPos;
        use ferrumc_world::{BlockEntity, BlockStateId, ChestInventory};
        let mut source = Chunk::new(ChunkPos::ORIGIN);
        let chest_state =
            BlockStateId::new(ferrumc_registry::block_state::block_default_state("chest").unwrap());
        for index in 0..512 {
            let at = BlockPos::new(index % 16, 64 + index / 256, (index / 16) % 16);
            source.set_block(at, chest_state).unwrap();
            source
                .set_block_entity(at, BlockEntity::Chest(ChestInventory::new()))
                .unwrap();
        }
        validate_baseline(&source).unwrap();
        assert!(baseline_storage_charge(&source).unwrap() > MAX_BASELINE_BYTES);
        let mut preparation = fixture();
        preparation.targets.insert(
            PlayerId::offline("oversized"),
            ViewerTargets {
                wanted: BTreeSet::from([source.pos()]),
                ..ViewerTargets::default()
            },
        );
        preparation
            .completed(Ok((source.pos(), Ok(source))), |_| false)
            .unwrap();
        assert!(preparation.ready.is_empty());
        assert_eq!(preparation.ready_bytes, 0);
        assert!(preparation.failed.contains(&ChunkPos::ORIGIN));
        preparation.request(PlayerId::offline("oversized"), [ChunkPos::ORIGIN], []);
        assert!(!preparation.has_jobs());
    }

    fn assert_cabin_loot(baseline: &Chunk, at: ferrumc_math::BlockPos) {
        use ferrumc_items::ItemId;
        let ferrumc_world::BlockEntity::Chest(chest) = baseline.block_entity(at).unwrap() else {
            panic!("real seed0 cabin chest")
        };
        for (slot, (name, count)) in [
            ("bread", 1),
            ("coal", 8),
            ("iron_nugget", 3),
            ("wheat", 5),
            ("oak_sapling", 1),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                chest.slots()[slot].item(),
                Some(ItemId::from_name(name).unwrap())
            );
            assert_eq!(chest.slots()[slot].count(), count);
        }
        assert!(chest.slots()[5..].iter().all(|s| s.item().is_none()));
    }

    #[tokio::test]
    async fn natural_cabin_warms_admits_and_redb_restart_keeps_edited_loot() {
        use ferrumc_core::{DimensionId, WorldId};
        use ferrumc_math::BlockPos;
        use ferrumc_sim::{ChunkTicket, GenerationBinding, LoadedChunkMap, TicketReason};
        use ferrumc_storage::{GenerationKey, RedbStore, WorldStore};
        use ferrumc_world::{BlockEntity, NaturalWorldGenerator, WorldSeed};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cabin.redb");
        let open = |path| {
            tokio::task::spawn_blocking(move || {
                RedbStore::open_with_generation_metadata(path).unwrap()
            })
        };
        let store = open(path.clone()).await.unwrap();
        let generation = RuntimeGeneration::new(Arc::new(
            NaturalWorldGenerator::with_playable(WorldSeed::new(0)).unwrap(),
        ));
        let key = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
        store
            .ensure_generation_identity(key, generation.identity())
            .await
            .unwrap();
        let binding = GenerationBinding::new(key, generation.identity());
        let mut chunks = LoadedChunkMap::with_generation(binding);
        let pos = ChunkPos::new(-10, -10);
        let at = BlockPos::new(-150, 97, -149);
        let mut preparation = ChunkPreparation::new(&generation);
        preparation.request(PlayerId::offline("cabin"), [pos], []);
        let completion = preparation.next_completion().await.unwrap();
        preparation.completed(completion, |_| false).unwrap();
        assert_eq!(
            chunks.ticket_count(pos),
            0,
            "warm loot owns no resident ticket"
        );
        let baseline = preparation.ready(pos).unwrap();
        assert_cabin_loot(&baseline, at);
        generation
            .acquire_warmed(
                &mut chunks,
                &store,
                Arc::clone(&baseline),
                ChunkTicket::of(TicketReason::Player),
            )
            .await
            .unwrap();
        assert_eq!(chunks.ticket_count(pos), 1);
        assert_eq!(
            chunks.get(pos).unwrap().block_entity(at),
            baseline.block_entity(at)
        );
        preparation.admitted(&BTreeSet::from([pos]));
        assert!(preparation.ready(pos).is_none());
        assert_eq!(preparation.ready_bytes, 0);
        let edited = chunks.get_mut(pos).unwrap();
        let Some(BlockEntity::Chest(chest)) = edited.block_entity_mut(at) else {
            panic!("admitted chest")
        };
        *chest.slot_mut(0).unwrap() = ferrumc_items::ItemStack::empty();
        *chest.slot_mut(1).unwrap() = ferrumc_items::ItemStack::empty();
        edited.mark_persist_dirty(at);
        let expected = edited.block_entity(at).unwrap().clone();
        let batch = chunks
            .try_take_persist_dirty_bound(&store, 7, std::num::NonZeroUsize::MIN)
            .await
            .unwrap();
        assert_eq!(batch.len(), 1);
        store.save_chunk_overlays(batch).await.unwrap();
        drop(store);
        let store = open(path).await.unwrap();
        let fresh_generation = RuntimeGeneration::new(Arc::new(
            NaturalWorldGenerator::with_playable(WorldSeed::new(0)).unwrap(),
        ));
        let mut fresh_preparation = ChunkPreparation::new(&fresh_generation);
        fresh_preparation.request(PlayerId::offline("rejoin"), [pos], []);
        let completion = fresh_preparation.next_completion().await.unwrap();
        fresh_preparation.completed(completion, |_| false).unwrap();
        let fresh_baseline = fresh_preparation.ready(pos).unwrap();
        assert_eq!(
            fresh_baseline.block_entity(at),
            baseline.block_entity(at),
            "new generator repeats original loot"
        );
        let mut restored = LoadedChunkMap::with_generation(binding);
        fresh_generation
            .acquire_warmed(
                &mut restored,
                &store,
                fresh_baseline,
                ChunkTicket::of(TicketReason::Player),
            )
            .await
            .unwrap();
        assert_eq!(
            restored.get(pos).unwrap().block_entity(at),
            Some(&expected),
            "persisted complete inventory wins over regenerated loot"
        );
        assert_eq!(restored.ticket_count(pos), 1);
        fresh_preparation.admitted(&BTreeSet::from([pos]));
        assert_eq!(fresh_preparation.ready_bytes, 0);
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
            "obsolete output still rejects mismatched block entities"
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
