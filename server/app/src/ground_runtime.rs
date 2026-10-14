//! Legacy item-plus-mob persistence with ground-item wiring: bounded publications, exact pickup replies and one
//! asynchronous sidecar writer. This is graceful persistence, not a transaction
//! coupling block removal, entity ownership and player inventory.
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;

use ferrumc_core::{EntityId, EntityUuid, PlayerId};
use ferrumc_math::ChunkPos;
use ferrumc_session::SessionRouter;
use ferrumc_sim::{
    GroundItemEvent, GroundItemIdentity, GroundItemLimits, GroundItemPickupResult,
    GroundItemRestore, MobIdentity, MobRuntimeError, SimShard,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::gameplay::GameplayEffect;
use crate::ground_persistence::{EntityRecords, GroundPersistence, RestoredEntity};
use crate::ground_view::GroundView;

type SaveJob = JoinHandle<anyhow::Result<GroundPersistence>>;

pub(crate) struct GroundRuntime {
    pub(crate) view: GroundView,
    pub(crate) pinned: BTreeSet<ChunkPos>,
    writer: Option<GroundPersistence>,
    job: Option<SaveJob>,
    next_save: u64,
}

impl GroundRuntime {
    pub(crate) fn new(writer: GroundPersistence, pinned: BTreeSet<ChunkPos>) -> Self {
        Self {
            view: GroundView::new(),
            pinned,
            writer: Some(writer),
            job: None,
            next_save: 400,
        }
    }

    pub(crate) fn initialize(shard: &mut SimShard) -> anyhow::Result<()> {
        let items = NonZeroUsize::new(256).ok_or_else(|| anyhow::anyhow!("invalid item cap"))?;
        let bytes = NonZeroUsize::new(4 * 1024 * 1024)
            .ok_or_else(|| anyhow::anyhow!("invalid item byte cap"))?;
        shard.initialize_entities_with_item_budget(items, bytes)?;
        shard.enable_ground_items(GroundItemLimits::new(items, bytes)?)?;
        shard.enable_survival(ferrumc_sim::Difficulty::Normal, true)?;
        shard.enable_water(4096, true)?;
        shard.enable_furnaces(256)?;
        Ok(())
    }

    pub(crate) fn refill(router: &mut SessionRouter, shard: &mut SimShard) -> anyhow::Result<()> {
        // UUIDs are minted outside simulation; transient IDs share the router's
        // nonreusing player/entity allocator and are never reconstructed from UUIDs.
        let room = shard.ground_item_identity_room()?;
        let mut identities = Vec::with_capacity(room);
        for _ in 0..room {
            let wire = EntityId::new(router.allocate_non_player_entity_id()?);
            identities.push(GroundItemIdentity::new(
                EntityUuid::from(uuid::Uuid::new_v4()),
                wire,
            )?);
        }
        shard
            .refill_ground_item_identities(identities)
            .map_err(|refusal| anyhow::anyhow!("ground identity admission: {}", refusal.reason()))
    }

    /// Publishes move-owned offers or returns their exact whole stack when no
    /// connection can accept them. A failure remains fatal, never a cloned grant.
    pub(crate) fn events(
        &mut self,
        router: &mut SessionRouter,
        shard: &mut SimShard,
        effects: &BTreeMap<PlayerId, mpsc::Sender<GameplayEffect>>,
    ) -> anyhow::Result<()> {
        for event in shard.take_ground_item_events(256)? {
            match event {
                GroundItemEvent::Spawn { snapshot } => self.view.snapshot(&snapshot)?,
                GroundItemEvent::StackChanged { snapshot } => {
                    self.view.changed(router, &snapshot)?;
                }
                GroundItemEvent::Move { base } => self.view.moved(router, base),
                GroundItemEvent::Remove { base } => self.view.remove(router, base),
                GroundItemEvent::PickupAccepted {
                    player,
                    wire_id,
                    count,
                } => self.view.pickup(router, player, wire_id, count),
                GroundItemEvent::PickupOffer {
                    ticket,
                    player,
                    wire_id,
                    stack,
                } => {
                    let effect = GameplayEffect::Pickup {
                        ticket,
                        wire_id,
                        stack,
                    };
                    let returned = match effects.get(&player) {
                        Some(sender) => sender
                            .try_send(effect)
                            .err()
                            .map(mpsc::error::TrySendError::into_inner),
                        None => Some(effect),
                    };
                    if let Some(GameplayEffect::Pickup { ticket, stack, .. }) = returned {
                        shard
                            .enqueue_ground_item_pickup_result(GroundItemPickupResult::new(
                                ticket, stack,
                            ))
                            .map_err(|refusal| {
                                anyhow::anyhow!(
                                    "undelivered ground pickup return: {}",
                                    refusal.reason()
                                )
                            })?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Captures both canonical sets behind one serial save/delete writer.
    /// At most one item4MiB plus mob6080-byte snapshot is held by the I/O task;
    /// busy writers or unsettled proposals leave the next capture due.
    pub(crate) async fn save_if_due(&mut self, shard: &SimShard, tick: u64) -> anyhow::Result<()> {
        if self.job.as_ref().is_some_and(JoinHandle::is_finished) {
            if let Some(job) = self.job.take() {
                self.writer = Some(job.await??);
            }
        }
        if tick < self.next_save || self.job.is_some() {
            return Ok(());
        }
        let Some(records) = capture_if_settled(shard)? else {
            return Ok(());
        };
        let mut writer = self
            .writer
            .take()
            .ok_or_else(|| anyhow::anyhow!("ground writer unavailable"))?;
        self.job = Some(tokio::spawn(async move {
            writer.flush(records).await?;
            Ok(writer)
        }));
        self.next_save = tick.saturating_add(400);
        Ok(())
    }

    pub(crate) async fn finish(&mut self, shard: &SimShard) -> anyhow::Result<()> {
        if let Some(job) = self.job.take() {
            self.writer = Some(job.await??);
        }
        let records = capture_if_settled(shard)?
            .ok_or_else(|| anyhow::anyhow!("unsettled entity runtime at stop"))?;
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("ground writer unavailable at stop"))?;
        writer.flush(records).await
    }

    /// Union of actual live/pending item and mob columns for genuine tickets.
    pub(crate) fn entity_chunks(shard: &SimShard) -> anyhow::Result<BTreeSet<ChunkPos>> {
        let mut chunks = shard.ground_item_chunks()?;
        chunks.extend(shard.mob_chunks()?);
        Ok(chunks)
    }

    /// True only when both item payloads and mob proposals can be captured.
    /// Mob events remain a separate publication queue; Root must drain them too.
    pub(crate) fn settled(shard: &SimShard) -> anyhow::Result<bool> {
        if !shard.ground_items_quiescent() {
            return Ok(false);
        }
        Ok(capture_settled_mobs(shard)?.is_some())
    }

    pub(crate) fn restore(
        proposal: RestoredEntity,
        router: &mut SessionRouter,
        shard: &mut SimShard,
    ) -> anyhow::Result<()> {
        let wire = EntityId::new(router.allocate_non_player_entity_id()?);
        match proposal {
            RestoredEntity::Item(proposal) => shard
                .enqueue_ground_item_restore(GroundItemRestore::new(proposal, wire))
                .map_err(|refusal| {
                    anyhow::anyhow!("ground restore admission: {}", refusal.reason())
                }),
            RestoredEntity::Mob(proposal) => shard
                .enqueue_mob_restore(proposal, MobIdentity::new(proposal.uuid(), wire))
                .map_err(|refusal| anyhow::anyhow!("mob restore admission: {}", refusal.reason())),
        }
    }
}

fn capture_settled_mobs(shard: &SimShard) -> anyhow::Result<Option<EntityRecords>> {
    match shard.capture_mob_records() {
        Ok(records) => Ok(Some(records)),
        Err(MobRuntimeError::Capacity("unsettled mob proposals")) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn capture_if_settled(shard: &SimShard) -> anyhow::Result<Option<EntityRecords>> {
    if !shard.ground_items_quiescent() {
        return Ok(None);
    }
    let Some(mobs) = capture_settled_mobs(shard)? else {
        return Ok(None);
    };
    let mut records = shard.capture_ground_item_records()?;
    records.extend(mobs);
    Ok(Some(records))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground_persistence::tests::{fixture, identity};
    use ferrumc_math::{ShardPos, Vec3};
    use ferrumc_sim::{ChunkTicket, TicketLevel, TicketReason};
    use ferrumc_storage::InMemoryStore;
    use ferrumc_world::FlatWorldGenerator;

    fn shard() -> SimShard {
        let mut shard = SimShard::new(ShardPos::new(0, 0));
        GroundRuntime::initialize(&mut shard).expect("ground init");
        shard
            .enable_mobs(false, Vec3::new(-100.0, 64.0, -100.0))
            .expect("mob init");
        shard
    }

    async fn load(shard: &mut SimShard, chunk: ChunkPos) {
        shard
            .loaded_chunks_mut()
            .acquire(
                &InMemoryStore::new(),
                &FlatWorldGenerator::new(),
                chunk,
                ChunkTicket::new(TicketReason::Forced, TicketLevel::ENTITY_TICKING),
            )
            .await
            .expect("genuine ticket");
    }

    fn proposals() -> (RestoredEntity, RestoredEntity) {
        let (key, record) = fixture(false, 100, Vec3::new(-16.5, 64.0, 1.5));
        let item =
            RestoredEntity::Item(ferrumc_sim::decode_item_entity(key, &record).expect("item"));
        let (key, record) = fixture(true, 101, Vec3::new(32.5, 64.0, 1.5));
        let mob = RestoredEntity::Mob(ferrumc_sim::decode_mob_entity(key, &record).expect("mob"));
        (item, mob)
    }

    #[tokio::test]
    async fn real_mixed_restore_keeps_uuid_uses_fresh_wires_and_union_entity_tickets() {
        let mut shard = shard();
        let mut router = SessionRouter::new();
        let retired = router.allocate_non_player_entity_id().expect("prior wire");
        let (item, mob) = proposals();
        let chunks = [item.transform().chunk(), mob.transform().chunk()]
            .into_iter()
            .collect::<BTreeSet<_>>();
        for chunk in &chunks {
            load(&mut shard, *chunk).await;
        }
        GroundRuntime::restore(item, &mut router, &mut shard).expect("item queue");
        GroundRuntime::restore(mob, &mut router, &mut shard).expect("mob queue");
        assert_eq!(
            GroundRuntime::entity_chunks(&shard).expect("pending union"),
            chunks
        );
        assert!(!GroundRuntime::settled(&shard).expect("pending"));
        shard.run_tick();
        let item_wire = shard
            .take_ground_item_events(256)
            .expect("item events")
            .into_iter()
            .find_map(|event| match event {
                GroundItemEvent::Spawn { snapshot } => {
                    Some((snapshot.base().uuid(), snapshot.base().wire_id()))
                }
                _ => None,
            })
            .expect("item admission");
        let mobs = shard.mob_snapshots(None, 64).expect("mob snapshots");
        assert_eq!(mobs.len(), 1);
        assert_eq!(item_wire.0, EntityUuid::from_bytes(100_u128.to_be_bytes()));
        assert_eq!(
            mobs[0].base().uuid(),
            EntityUuid::from_bytes(101_u128.to_be_bytes())
        );
        assert!(item_wire.1.get() > retired && mobs[0].base().wire_id().get() > item_wire.1.get());
        assert_eq!(mobs[0].health().to_bits(), 7.0_f32.to_bits());
        assert!(GroundRuntime::settled(&shard).expect("settled"));
        assert_eq!(
            GroundRuntime::entity_chunks(&shard).expect("live union"),
            chunks
        );
        let records = capture_if_settled(&shard)
            .expect("capture")
            .expect("settled set");
        assert_eq!(records.len(), 2);
        assert!(records
            .iter()
            .any(|(_, record)| record.record().data().starts_with(b"FCIE")));
        assert!(records
            .iter()
            .any(|(_, record)| record.record().data().starts_with(b"FCMB")));
    }

    #[tokio::test]
    async fn unsettled_mob_defers_periodic_and_refuses_final_capture_without_deleting_items() {
        let directory = tempfile::tempdir().expect("directory");
        let (mut writer, _) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("open");
        let item = fixture(false, 100, Vec3::new(-16.5, 64.0, 1.5));
        let mob = fixture(true, 101, Vec3::new(32.5, 64.0, 1.5));
        writer
            .flush(vec![item, mob])
            .await
            .expect("complete prior snapshot");
        let mut runtime = GroundRuntime::new(writer, BTreeSet::new());
        let mut shard = shard();
        let mut router = SessionRouter::new();
        let (_, mob) = proposals();
        GroundRuntime::restore(mob, &mut router, &mut shard).expect("pending mob");
        assert!(shard.ground_items_quiescent());
        assert!(!GroundRuntime::settled(&shard).expect("mob unsettled"));
        runtime.save_if_due(&shard, 500).await.expect("defer");
        assert!(runtime.job.is_none());
        assert_eq!(runtime.next_save, 400);
        assert!(runtime.finish(&shard).await.is_err());
        assert!(runtime.writer.is_some());
        drop(runtime);
        let (_, restored) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("prior disk snapshot");
        assert_eq!(restored.len(), 2);
    }

    #[tokio::test]
    async fn periodic_then_final_mixed_capture_restores_both_types_after_restart() {
        let directory = tempfile::tempdir().expect("directory");
        let (writer, _) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("open");
        let mut runtime = GroundRuntime::new(writer, BTreeSet::new());
        let mut shard = shard();
        let mut router = SessionRouter::new();
        let (item, mob) = proposals();
        for chunk in [item.transform().chunk(), mob.transform().chunk()] {
            load(&mut shard, chunk).await;
        }
        GroundRuntime::restore(item, &mut router, &mut shard).expect("item");
        GroundRuntime::restore(mob, &mut router, &mut shard).expect("mob");
        shard.run_tick();
        runtime
            .events(&mut router, &mut shard, &BTreeMap::new())
            .expect("ground publication");
        shard.take_mob_events(256).expect("mob publication");
        runtime.save_if_due(&shard, 400).await.expect("periodic");
        assert!(runtime.job.is_some());
        assert!(runtime.writer.is_none());
        runtime
            .finish(&shard)
            .await
            .expect("serial join plus final capture");
        assert!(runtime.job.is_none());
        assert!(runtime.writer.is_some());
        drop(runtime);
        let (_, restored) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("restart");
        assert_eq!(restored.len(), 2);
        assert_eq!(
            restored
                .iter()
                .filter(|entity| matches!(entity, RestoredEntity::Mob(_)))
                .count(),
            1
        );
        assert_eq!(
            restored
                .iter()
                .filter(|entity| matches!(entity, RestoredEntity::Item(_)))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn saved_mob_restore_retains_unloaded_proposal_until_genuine_ticket_arrives() {
        let mut shard = shard();
        let mut router = SessionRouter::new();
        let (_, mob) = proposals();
        let chunk = mob.transform().chunk();
        GroundRuntime::restore(mob, &mut router, &mut shard).expect("queue");
        shard.run_tick();
        assert!(!GroundRuntime::settled(&shard).expect("still pending"));
        assert!(shard
            .mob_snapshots(None, 64)
            .expect("no fabricated resident actor")
            .is_empty());
        load(&mut shard, chunk).await;
        shard.run_tick();
        assert!(GroundRuntime::settled(&shard).expect("now admitted"));
        assert_eq!(shard.mob_snapshots(None, 64).expect("real actor").len(), 1);
    }
}
