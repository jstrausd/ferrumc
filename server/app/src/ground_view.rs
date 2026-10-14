//! Bounded immutable network projection of canonical simulation item owners.
use ferrumc_codec::BoundedBytes;
use ferrumc_core::{EntityId, PlayerId};
use ferrumc_items::encode_item_entity_metadata;
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, EntityTeleport, EntityVelocity, RemoveEntities, SetEntityData,
    SpawnEntity, TakeItemEntity,
};
use ferrumc_session::{SessionError, SessionRouter};
use ferrumc_sim::{EntityData, EntityTransform, ItemEntitySnapshot};
use std::collections::{BTreeMap, BTreeSet};

const MAX_ITEMS: usize = 256;
const MAX_BYTES: usize = 4 * 1024 * 1024;

struct Projection {
    base: EntityData,
    metadata: ClientboundPlayPacket,
    bytes: usize,
}

/// Wire metadata is an immutable observation, never a live `ItemStack` owner.
/// Views are bounded by the router connection cap and 256 active item identities.
pub(crate) struct GroundView {
    items: BTreeMap<EntityId, Projection>,
    views: BTreeMap<PlayerId, BTreeMap<EntityId, EntityTransform>>,
    chunks: BTreeMap<PlayerId, BTreeSet<ChunkPos>>,
    bytes: usize,
    failed: BTreeSet<PlayerId>,
}

impl GroundView {
    pub(crate) fn new() -> Self {
        Self {
            items: BTreeMap::new(),
            views: BTreeMap::new(),
            chunks: BTreeMap::new(),
            bytes: 0,
            failed: BTreeSet::new(),
        }
    }

    /// Admitted column deliveries are distinct from simulation residency tickets.
    pub(crate) fn update_chunks(
        &mut self,
        player: PlayerId,
        added: Vec<ChunkPos>,
        removed: Vec<ChunkPos>,
    ) {
        let view = self.chunks.entry(player).or_default();
        for pos in removed {
            view.remove(&pos);
        }
        view.extend(added);
    }

    pub(crate) fn take_failed(&mut self) -> Vec<PlayerId> {
        std::mem::take(&mut self.failed).into_iter().collect()
    }

    pub(crate) fn forget(&mut self, player: PlayerId) {
        self.views.remove(&player);
        self.chunks.remove(&player);
    }

    pub(crate) fn snapshot(&mut self, snapshot: &ItemEntitySnapshot) -> anyhow::Result<()> {
        self.observe(snapshot).map(|_| ())
    }

    fn observe(&mut self, snapshot: &ItemEntitySnapshot) -> anyhow::Result<bool> {
        let base = snapshot.base();
        let bytes = encode_item_entity_metadata(snapshot.item_state().stack())?.into_bytes();
        let size = bytes.len();
        let old = self
            .items
            .get(&base.wire_id())
            .map_or(0, |entry| entry.bytes);
        let required = self
            .bytes
            .checked_sub(old)
            .and_then(|sum| sum.checked_add(size))
            .ok_or_else(|| anyhow::anyhow!("ground metadata accounting overflow"))?;
        anyhow::ensure!(
            required <= MAX_BYTES
                && (self.items.contains_key(&base.wire_id()) || self.items.len() < MAX_ITEMS),
            "ground metadata projection capacity exceeded"
        );
        let metadata = ClientboundPlayPacket::SetEntityData(SetEntityData::new(
            base.wire_id().get(),
            BoundedBytes::new(bytes)?,
        ));
        let changed = self
            .items
            .get(&base.wire_id())
            .is_none_or(|entry| entry.metadata != metadata);
        self.items.insert(
            base.wire_id(),
            Projection {
                base,
                metadata,
                bytes: size,
            },
        );
        self.bytes = required;
        Ok(changed)
    }

    pub(crate) fn changed(
        &mut self,
        router: &mut SessionRouter,
        snapshot: &ItemEntitySnapshot,
    ) -> anyhow::Result<()> {
        if !self.observe(snapshot)? {
            return Ok(());
        }
        let Some(entry) = self.items.get(&snapshot.base().wire_id()) else {
            return Ok(());
        };
        for (player, known) in &self.views {
            if known.contains_key(&entry.base.wire_id()) {
                mandatory(router, &mut self.failed, *player, entry.metadata.clone());
            }
        }
        Ok(())
    }

    pub(crate) fn moved(&mut self, _router: &mut SessionRouter, base: EntityData) {
        if let Some(entry) = self.items.get_mut(&base.wire_id()) {
            entry.base = base;
        }
    }

    pub(crate) fn remove(&mut self, router: &mut SessionRouter, base: EntityData) {
        if let Some(entry) = self.items.remove(&base.wire_id()) {
            self.bytes -= entry.bytes;
        }
        for (player, known) in &mut self.views {
            if known.remove(&base.wire_id()).is_some() {
                mandatory(
                    router,
                    &mut self.failed,
                    *player,
                    ClientboundPlayPacket::RemoveEntities(RemoveEntities::new(vec![base
                        .wire_id()
                        .get()])),
                );
            }
        }
    }

    pub(crate) fn pickup(
        &mut self,
        router: &mut SessionRouter,
        player: PlayerId,
        wire_id: EntityId,
        count: u8,
    ) {
        let Some(collector) = router.player_entity_id(player) else {
            return;
        };
        let packet = ClientboundPlayPacket::TakeItemEntity(TakeItemEntity::new(
            wire_id.get(),
            collector,
            i32::from(count),
        ));
        for (viewer, known) in &self.views {
            if known.contains_key(&wire_id) {
                mandatory(router, &mut self.failed, *viewer, packet.clone());
            }
        }
    }

    /// Reconciles current owner observations against actual admitted player position.
    /// No item is respawned merely because its stack metadata changed.
    pub(crate) fn reconcile(
        &mut self,
        router: &mut SessionRouter,
        online: &BTreeMap<PlayerId, String>,
    ) {
        self.views.retain(|player, _| online.contains_key(player));
        self.chunks.retain(|player, _| online.contains_key(player));
        for player in online.keys().copied() {
            let Some(pos) = router.player_position(player) else {
                continue;
            };
            let center =
                BlockPos::new(pos.x.floor() as i32, 0, pos.z.floor() as i32).to_chunk_pos();
            let radius = i64::from(router.view_distance().min(6));
            let admitted = self.chunks.get(&player);
            let wanted: BTreeSet<_> = self
                .items
                .iter()
                .filter(|(_, entry)| {
                    let chunk = entry.base.chunk();
                    admitted.is_some_and(|view| view.contains(&chunk))
                        && (i64::from(center.x()) - i64::from(chunk.x())).abs() <= radius
                        && (i64::from(center.z()) - i64::from(chunk.z())).abs() <= radius
                })
                .map(|(wire, _)| *wire)
                .collect();
            let known = self.views.entry(player).or_default();
            let removed: Vec<_> = known
                .keys()
                .filter(|wire| !wanted.contains(*wire))
                .map(|wire| wire.get())
                .collect();
            if !removed.is_empty()
                && !mandatory(
                    router,
                    &mut self.failed,
                    player,
                    ClientboundPlayPacket::RemoveEntities(RemoveEntities::new(removed)),
                )
            {
                continue;
            }
            known.retain(|wire, _| wanted.contains(wire));
            let added: Vec<_> = wanted
                .iter()
                .filter(|wire| !known.contains_key(*wire))
                .take(16)
                .copied()
                .collect();
            for wire in added {
                if let Some(entry) = self.items.get(&wire) {
                    match router.send_gameplay_pair_to(
                        player,
                        spawn_packet(entry.base),
                        entry.metadata.clone(),
                    ) {
                        Ok(()) => {
                            known.insert(wire, entry.base.transform());
                        }
                        // Neither packet was admitted. The bounded projection
                        // retries this still-unseen entity on a later boundary.
                        Err(SessionError::OutboundFull { .. }) => break,
                        Err(_) => {
                            self.failed.insert(player);
                            break;
                        }
                    }
                }
            }
            for (wire, delivered) in known.iter_mut() {
                let Some(entry) = self.items.get(wire) else {
                    continue;
                };
                let current = entry.base.transform();
                if *delivered == current {
                    continue;
                }
                match router.try_send_play_packet_to(player, move_packet(entry.base)) {
                    Ok(true) => *delivered = current,
                    // A later absolute update heals the previous baseline even
                    // when the entity stops moving after this refused send.
                    Ok(false) => {}
                    Err(_) => {
                        self.failed.insert(player);
                        break;
                    }
                }
            }
        }
    }
}

fn move_packet(base: EntityData) -> ClientboundPlayPacket {
    let pos = base.transform().position();
    ClientboundPlayPacket::EntityTeleport(EntityTeleport::new(
        base.wire_id().get(),
        pos.x,
        pos.y,
        pos.z,
        0.0,
        0.0,
        0.0,
        base.transform().yaw(),
        base.transform().pitch(),
        false,
    ))
}

fn spawn_packet(base: EntityData) -> ClientboundPlayPacket {
    let pos = base.transform().position();
    ClientboundPlayPacket::SpawnEntity(SpawnEntity::new(
        base.wire_id().get(),
        base.uuid().into(),
        base.entity_type().protocol_id(),
        pos.x,
        pos.y,
        pos.z,
        pack_angle(base.transform().pitch()),
        pack_angle(base.transform().yaw()),
        pack_angle(base.transform().yaw()),
        0,
        EntityVelocity::new(0, 0, 0),
    ))
}

fn pack_angle(degrees: f32) -> i8 {
    (degrees * (256.0 / 360.0)).floor() as i32 as i8
}

fn mandatory(
    router: &SessionRouter,
    failed: &mut BTreeSet<PlayerId>,
    player: PlayerId,
    packet: ClientboundPlayPacket,
) -> bool {
    let packet_id = packet.packet_id();
    match router.send_gameplay_packet_to(player, packet) {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(
                %player,
                %error,
                packet_id,
                outbound_queue_len = router.player_outbound_queue_len(player),
                "ground item mandatory entity publication refused"
            );
            failed.insert(player);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground_persistence::tests::fixture;
    use crate::ground_persistence::RestoredEntity;
    use crate::ground_runtime::GroundRuntime;
    use crate::mob_view::MobView;
    use ferrumc_math::{ShardPos, Vec3};
    use ferrumc_net::Criticality;
    use ferrumc_proto::generated::play::GameEvent;
    use ferrumc_session::PlayerSessionHandle;
    use ferrumc_sim::{
        ChunkTicket, GameOutput, GroundItemEvent, SimShard, TicketLevel, TicketReason,
    };
    use ferrumc_storage::InMemoryStore;
    use ferrumc_world::FlatWorldGenerator;

    fn observe_tick(shard: &mut SimShard, view: &mut GroundView, router: &mut SessionRouter) {
        shard.run_tick();
        for event in shard.take_ground_item_events(256).unwrap() {
            match event {
                GroundItemEvent::Spawn { snapshot } => view.snapshot(&snapshot).unwrap(),
                GroundItemEvent::Move { base } => view.moved(router, base),
                GroundItemEvent::StackChanged { snapshot } => {
                    view.changed(router, &snapshot).unwrap();
                }
                GroundItemEvent::Remove { base } => view.remove(router, base),
                _ => {}
            }
        }
    }

    async fn owners(
        count: usize,
        y: f64,
    ) -> (
        SimShard,
        SessionRouter,
        PlayerSessionHandle,
        PlayerId,
        GroundView,
        BTreeMap<PlayerId, String>,
    ) {
        let mut shard = SimShard::new(ShardPos::new(0, 0));
        GroundRuntime::initialize(&mut shard).unwrap();
        let store = InMemoryStore::new();
        for chunk in [ChunkPos::new(0, 0), ChunkPos::new(4, 0)] {
            shard
                .loaded_chunks_mut()
                .acquire(
                    &store,
                    &FlatWorldGenerator::new(),
                    chunk,
                    ChunkTicket::new(TicketReason::Forced, TicketLevel::ENTITY_TICKING),
                )
                .await
                .unwrap();
        }
        let mut router = SessionRouter::new();
        let mut inbox = router.register_shard(ShardPos::new(0, 0));
        let mut view = GroundView::new();
        for start in (0..count).step_by(32) {
            for i in start..(start + 32).min(count) {
                let (key, record) = fixture(
                    false,
                    i as u128 + 100,
                    Vec3::new((i % 16) as f64 + 0.5, y, (i / 16) as f64 + 0.5),
                );
                let proposal = ferrumc_sim::decode_item_entity(key, &record).unwrap();
                GroundRuntime::restore(RestoredEntity::Item(proposal), &mut router, &mut shard)
                    .unwrap();
            }
            observe_tick(&mut shard, &mut view, &mut router);
        }
        let player = PlayerId::offline("ground-view-observer");
        let handle = router
            .join_player(player, "ground-view-observer", Vec3::new(64.5, 64.0, 8.5))
            .unwrap();
        shard.enqueue(inbox.try_recv().unwrap()).unwrap();
        observe_tick(&mut shard, &mut view, &mut router);
        view.update_chunks(player, vec![ChunkPos::new(0, 0)], vec![]);
        let online = BTreeMap::from([(player, "ground-view-observer".to_owned())]);
        (shard, router, handle, player, view, online)
    }

    #[tokio::test]
    async fn real_256_item_owners_reconcile_without_partial_spawn_or_burst_disconnect() {
        let (_shard, mut router, mut handle, player, mut view, online) = owners(256, 64.0).await;
        assert_eq!(view.items.len(), 256);
        view.reconcile(&mut router, &online);
        assert_eq!(view.views[&player].len(), 16);
        for _ in 0..8 {
            view.reconcile(&mut router, &online);
        }
        assert_eq!(router.player_outbound_queue_len(player), Some(192));
        assert_eq!(view.views[&player].len(), 96);
        assert_eq!(view.take_failed(), Vec::<PlayerId>::new());
        let mut spawned = BTreeSet::new();
        for _ in 0..17 {
            while let Some(spawn) = handle.try_recv() {
                assert_eq!(spawn.criticality(), Criticality::Mandatory);
                let ClientboundPlayPacket::SpawnEntity(spawn) = spawn.into_packet() else {
                    panic!("spawn must lead each admitted pair");
                };
                assert!(spawned.insert(spawn.entity_id()));
                let metadata = handle
                    .try_recv()
                    .expect("metadata must be admitted with spawn");
                assert_eq!(metadata.criticality(), Criticality::Mandatory);
                let ClientboundPlayPacket::SetEntityData(metadata) = metadata.into_packet() else {
                    panic!("metadata must follow its own spawn");
                };
                assert_eq!(metadata.entity_id(), spawn.entity_id());
            }
            if spawned.len() == 256 {
                break;
            }
            view.reconcile(&mut router, &online);
            assert_eq!(view.take_failed(), Vec::<PlayerId>::new());
        }
        assert_eq!(spawned.len(), 256);
        assert_eq!(view.views[&player].len(), 256);
        view.reconcile(&mut router, &online);
        assert!(
            handle.try_recv().is_none(),
            "stationary owners do not create more motion"
        );
        assert!(router.is_player_connected(player));
    }

    #[tokio::test]
    async fn both_real_entity_views_defer_new_pairs_for_late_mandatory_output() {
        let (mut shard, mut router, mut handle, player, mut ground, online) =
            owners(16, 64.0).await;
        shard
            .enable_mobs(false, Vec3::new(-100.0, 64.0, -100.0))
            .unwrap();
        for id in 0..16 {
            let (key, record) = fixture(
                true,
                id + 1000,
                Vec3::new((id % 8) as f64 + 0.5, 64.0, (id / 8) as f64 + 4.5),
            );
            let proposal = ferrumc_sim::decode_mob_entity(key, &record).unwrap();
            GroundRuntime::restore(RestoredEntity::Mob(proposal), &mut router, &mut shard).unwrap();
        }
        observe_tick(&mut shard, &mut ground, &mut router);
        let snapshots = shard.mob_snapshots(None, 64).unwrap();
        assert_eq!(snapshots.len(), 16);
        let mut mobs = MobView::new().unwrap();
        for snapshot in &snapshots {
            mobs.snapshot(snapshot).unwrap();
        }
        mobs.update_chunks(player, vec![ChunkPos::new(0, 0)], vec![]);
        while router
            .try_send_play_packet_to(
                player,
                ClientboundPlayPacket::GameEvent(GameEvent::new(1, 0.0)),
            )
            .unwrap()
        {}
        assert_eq!(router.player_outbound_queue_len(player), Some(192));
        ground.reconcile(&mut router, &online);
        mobs.reconcile(&mut router, &online);
        assert_eq!(ground.take_failed(), Vec::<PlayerId>::new());
        assert_eq!(mobs.take_failed(), Vec::<PlayerId>::new());
        assert_eq!(router.player_outbound_queue_len(player), Some(192));
        // The old two independent 16-entity bursts consumed all 64 remaining
        // slots. A genuine later mandatory output then removed a healthy viewer.
        for sequence in 0..64 {
            assert_eq!(
                router.route_output(&GameOutput::BlockActionAcknowledged { player, sequence }),
                Vec::<PlayerId>::new()
            );
        }
        for _ in 0..192 {
            assert_eq!(
                handle.try_recv().unwrap().criticality(),
                Criticality::Droppable
            );
        }
        for sequence in 0..64 {
            let message = handle.try_recv().unwrap();
            assert_eq!(message.criticality(), Criticality::Mandatory);
            let ClientboundPlayPacket::AcknowledgeBlockChange(ack) = message.into_packet() else {
                panic!("deferred new entities must not consume late mandatory headroom");
            };
            assert_eq!(ack.sequence(), sequence);
        }
        assert!(handle.try_recv().is_none());
        ground.reconcile(&mut router, &online);
        mobs.reconcile(&mut router, &online);
        let expected: BTreeSet<_> = ground
            .items
            .keys()
            .copied()
            .chain(snapshots.iter().map(|snapshot| snapshot.base().wire_id()))
            .map(EntityId::get)
            .collect();
        let mut spawned = BTreeSet::new();
        while let Some(message) = handle.try_recv() {
            let ClientboundPlayPacket::SpawnEntity(spawn) = message.into_packet() else {
                panic!("retry must begin with the whole ordered spawn pair");
            };
            assert!(spawned.insert(spawn.entity_id()));
            let metadata = handle.try_recv().unwrap();
            assert_eq!(metadata.criticality(), Criticality::Mandatory);
            let ClientboundPlayPacket::SetEntityData(metadata) = metadata.into_packet() else {
                panic!("retry must retain its matching metadata");
            };
            assert_eq!(metadata.entity_id(), spawn.entity_id());
        }
        assert_eq!(spawned, expected);
        assert_eq!(spawned.len(), 32);
        ground.reconcile(&mut router, &online);
        mobs.reconcile(&mut router, &online);
        assert!(handle.try_recv().is_none());
        assert!(router.is_player_connected(player));
    }

    #[tokio::test]
    async fn failed_final_motion_heals_without_new_event_and_unchanged_metadata_stays_quiet() {
        let (mut shard, mut router, mut handle, player, mut view, online) = owners(1, 70.0).await;
        view.reconcile(&mut router, &online);
        while handle.try_recv().is_some() {}
        let wire = *view.items.keys().next().unwrap();
        let delivered = view.views[&player][&wire];
        for _ in 0..2 {
            observe_tick(&mut shard, &mut view, &mut router);
        }
        assert!(
            handle.try_recv().is_none(),
            "move events coalesce until reconcile"
        );
        let current = view.items[&wire].base.transform();
        assert_ne!(current, delivered);
        while router
            .try_send_play_packet_to(
                player,
                ClientboundPlayPacket::GameEvent(GameEvent::new(1, 0.0)),
            )
            .unwrap()
        {}
        view.reconcile(&mut router, &online);
        assert_eq!(view.views[&player][&wire], delivered);
        assert_eq!(view.take_failed(), Vec::<PlayerId>::new());
        while handle.try_recv().is_some() {}
        // No subsequent tick or Move event: the latest canonical observation
        // must still heal the refused final pose after the viewer catches up.
        view.reconcile(&mut router, &online);
        let ClientboundPlayPacket::EntityTeleport(motion) =
            handle.try_recv().unwrap().into_packet()
        else {
            panic!("a refused final movement must be retried");
        };
        assert_eq!(motion.y().to_bits(), current.position().y.to_bits());
        assert_eq!(view.views[&player][&wire], current);
        assert!(handle.try_recv().is_none());
        let snapshot = shard.ground_item_snapshots(None, 1).unwrap().remove(0);
        view.changed(&mut router, &snapshot).unwrap();
        view.reconcile(&mut router, &online);
        assert!(
            handle.try_recv().is_none(),
            "same encoded stack creates no mandatory metadata"
        );
    }
}
