//! Bounded immutable network projection of canonical simulation mob owners.
use ferrumc_codec::BoundedBytes;
use ferrumc_core::{EntityId, PlayerId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, EntityTeleport, EntityVelocity, RemoveEntities, SetEntityData,
    SpawnEntity,
};
use ferrumc_session::SessionRouter;
use ferrumc_sim::{EntityData, MobKind, MobRules, MobSnapshot};
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
    views: BTreeMap<PlayerId, BTreeSet<EntityId>>,
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
        self.items.insert(
            base.wire_id(),
            Projection {
                base,
                metadata,
                bytes: size,
            },
        );
        self.bytes = required;
        Ok(())
    }

    pub(crate) fn changed(
        &mut self,
        router: &mut SessionRouter,
        snapshot: &MobSnapshot,
    ) -> anyhow::Result<()> {
        self.snapshot(snapshot)?;
        let Some(entry) = self.items.get(&snapshot.base().wire_id()) else {
            return Ok(());
        };
        for (player, known) in &self.views {
            if known.contains(&entry.base.wire_id()) {
                mandatory(router, &mut self.failed, *player, entry.metadata.clone());
            }
        }
        Ok(())
    }

    pub(crate) fn moved(&mut self, router: &mut SessionRouter, base: EntityData) {
        let Some(entry) = self.items.get_mut(&base.wire_id()) else {
            return;
        };
        entry.base = base;
        let pos = base.transform().position();
        let packet = ClientboundPlayPacket::EntityTeleport(EntityTeleport::new(
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
        ));
        for (player, known) in &self.views {
            if known.contains(&base.wire_id()) {
                router.send_play_packet_to(*player, packet.clone());
            }
        }
    }

    pub(crate) fn remove(&mut self, router: &mut SessionRouter, base: EntityData) {
        if let Some(entry) = self.items.remove(&base.wire_id()) {
            self.bytes -= entry.bytes;
        }
        for (player, known) in &mut self.views {
            if known.remove(&base.wire_id()) {
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
            let removed: Vec<_> = known.difference(&wanted).map(|wire| wire.get()).collect();
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
            known.retain(|wire| wanted.contains(wire));
            let added: Vec<_> = wanted.difference(known).copied().collect();
            for wire in added {
                if let Some(entry) = self.items.get(&wire) {
                    if !mandatory(router, &mut self.failed, player, spawn_packet(entry.base))
                        || !mandatory(router, &mut self.failed, player, entry.metadata.clone())
                    {
                        break;
                    }
                    known.insert(wire);
                }
            }
        }
    }
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
        0,
        0,
        0,
        0,
        EntityVelocity::new(0, 0, 0),
    ))
}

fn mandatory(
    router: &SessionRouter,
    failed: &mut BTreeSet<PlayerId>,
    player: PlayerId,
    packet: ClientboundPlayPacket,
) -> bool {
    if router.send_gameplay_packet_to(player, packet).is_err() {
        failed.insert(player);
        false
    } else {
        true
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
