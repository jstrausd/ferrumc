//! Bounded immutable network projection of canonical simulation mob owners.
use ferrumc_codec::BoundedBytes;
use ferrumc_core::{EntityId, PlayerId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, EntityTeleport, EntityVelocity, RemoveEntities, SetEntityData,
    SpawnEntity,
};
use ferrumc_session::{SessionError, SessionRouter};
use ferrumc_sim::{EntityData, EntityTransform, MobKind, MobRules, MobSnapshot};
use std::collections::{BTreeMap, BTreeSet};

const MAX_ITEMS: usize = 64;
const MAX_BYTES: usize = 64 * 128;

struct Projection {
    base: EntityData,
    metadata: ClientboundPlayPacket,
    bytes: usize,
}

/// Wire metadata is an immutable observation of the genuine canonical owner.
/// Views are bounded by the router connection cap and 64 active mob identities.
pub(crate) struct MobView {
    items: BTreeMap<EntityId, Projection>,
    rules: MobRules,
    views: BTreeMap<PlayerId, BTreeMap<EntityId, EntityTransform>>,
    chunks: BTreeMap<PlayerId, BTreeSet<ChunkPos>>,
    bytes: usize,
    failed: BTreeSet<PlayerId>,
}

impl MobView {
    pub(crate) fn new() -> anyhow::Result<Self> {
        Ok(Self {
            rules: MobRules::builtin()?,
            items: BTreeMap::new(),
            views: BTreeMap::new(),
            chunks: BTreeMap::new(),
            bytes: 0,
            failed: BTreeSet::new(),
        })
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

    pub(crate) fn snapshot(&mut self, snapshot: &MobSnapshot) -> anyhow::Result<()> {
        self.observe(snapshot).map(|_| ())
    }

    fn observe(&mut self, snapshot: &MobSnapshot) -> anyhow::Result<bool> {
        let base = snapshot.base();
        let bytes = metadata(&self.rules, *snapshot);
        let size = bytes.len();
        let old = self
            .items
            .get(&base.wire_id())
            .map_or(0, |entry| entry.bytes);
        let required = self
            .bytes
            .checked_sub(old)
            .and_then(|sum| sum.checked_add(size))
            .ok_or_else(|| anyhow::anyhow!("mob metadata accounting overflow"))?;
        anyhow::ensure!(
            required <= MAX_BYTES
                && (self.items.contains_key(&base.wire_id()) || self.items.len() < MAX_ITEMS),
            "mob metadata projection capacity exceeded"
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
        snapshot: &MobSnapshot,
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

    /// Reconciles current owner observations against actual admitted player position.
    /// No mob is respawned merely because its health metadata changed.
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
                "mob mandatory entity publication refused"
            );
            failed.insert(player);
            false
        }
    }
}

fn metadata(rules: &MobRules, snapshot: MobSnapshot) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    let mut entry = |accessor: ferrumc_sim::mobs::MobMetadataAccessor, value: &[u8]| {
        out.push(accessor.index());
        ferrumc_codec::write_var_int(&mut out, accessor.serializer());
        out.extend_from_slice(value);
    };
    entry(rules.health_metadata(), &snapshot.health().to_be_bytes());
    entry(rules.flags_metadata(), &[0]);
    entry(rules.baby_metadata(snapshot.kind()), &[0]);
    match snapshot.kind() {
        MobKind::Sheep => entry(rules.sheep_wool_metadata(), &[0]),
        MobKind::Pig => entry(rules.pig_boost_metadata(), &[0]),
        MobKind::Zombie => {
            entry(rules.zombie_special_metadata(), &[0]);
            entry(rules.zombie_conversion_metadata(), &[0]);
        }
        MobKind::Cow | MobKind::Chicken => {}
    }
    out.push(0xff);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground_persistence::tests::fixture;
    use crate::ground_persistence::RestoredEntity;
    use crate::ground_runtime::GroundRuntime;
    use ferrumc_math::{ShardPos, Vec3};
    use ferrumc_proto::generated::play::GameEvent;
    use ferrumc_sim::{ChunkTicket, GameInput, SimShard, TicketLevel, TicketReason};
    use ferrumc_storage::InMemoryStore;
    use ferrumc_world::FlatWorldGenerator;

    #[tokio::test]
    async fn actual_health_change_still_requires_mandatory_delivery_after_unchanged_observation() {
        let mut shard = SimShard::new(ShardPos::new(0, 0));
        GroundRuntime::initialize(&mut shard).unwrap();
        shard
            .enable_mobs(false, Vec3::new(-100.0, 64.0, -100.0))
            .unwrap();
        shard
            .loaded_chunks_mut()
            .acquire(
                &InMemoryStore::new(),
                &FlatWorldGenerator::new(),
                ChunkPos::new(0, 0),
                ChunkTicket::new(TicketReason::Forced, TicketLevel::ENTITY_TICKING),
            )
            .await
            .unwrap();
        let mut router = SessionRouter::new();
        let mut inbox = router.register_shard(ShardPos::new(0, 0));
        let player = PlayerId::offline("mob-view-observer");
        let mut handle = router
            .join_player(player, "mob-view-observer", Vec3::new(8.0, 64.0, 8.0))
            .unwrap();
        shard.enqueue(inbox.try_recv().unwrap()).unwrap();
        shard.run_tick();
        let (key, record) = fixture(true, 123, Vec3::new(9.0, 64.0, 8.0));
        let proposal = ferrumc_sim::decode_mob_entity(key, &record).unwrap();
        GroundRuntime::restore(RestoredEntity::Mob(proposal), &mut router, &mut shard).unwrap();
        shard.run_tick();
        let snapshot = shard.mob_snapshots(None, 1).unwrap().remove(0);
        let wire = snapshot.base().wire_id();
        let mut view = MobView::new().unwrap();
        view.snapshot(&snapshot).unwrap();
        view.update_chunks(player, vec![ChunkPos::new(0, 0)], vec![]);
        let online = BTreeMap::from([(player, "mob-view-observer".to_owned())]);
        view.reconcile(&mut router, &online);
        let ClientboundPlayPacket::SpawnEntity(spawn) = handle.try_recv().unwrap().into_packet()
        else {
            panic!("real restored owner must spawn first");
        };
        assert_eq!(spawn.entity_id(), wire.get());
        assert!(matches!(
            handle.try_recv().unwrap().into_packet(),
            ClientboundPlayPacket::SetEntityData(_)
        ));
        view.changed(&mut router, &snapshot).unwrap();
        assert!(
            handle.try_recv().is_none(),
            "unchanged metadata must not fill the FIFO"
        );

        shard
            .enqueue(GameInput::PlayerMobAttackTracked {
                player,
                target: wire,
                damage: 2.0,
                attack_speed: 4.0,
                token: 1,
            })
            .unwrap();
        shard.run_tick();
        let changed = shard.mob_snapshots(None, 1).unwrap().remove(0);
        assert!(
            changed.health() < snapshot.health(),
            "actual owner attack must change health"
        );
        for _ in 0..router.outbound_capacity() {
            router
                .send_gameplay_packet_to(
                    player,
                    ClientboundPlayPacket::GameEvent(GameEvent::new(1, 0.0)),
                )
                .unwrap();
        }
        let delivered = view.views[&player][&wire];
        view.changed(&mut router, &changed).unwrap();
        assert_eq!(view.take_failed(), vec![player]);
        assert_eq!(view.views[&player][&wire], delivered);
        router.disconnect_player(player).unwrap();
        assert!(!router.is_player_connected(player));
    }
}
