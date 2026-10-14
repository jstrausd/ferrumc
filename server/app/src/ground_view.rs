//! Bounded immutable network projection of canonical simulation item owners.
use ferrumc_codec::BoundedBytes;
use ferrumc_core::{EntityId, PlayerId};
use ferrumc_items::encode_item_entity_metadata;
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, EntityTeleport, EntityVelocity, RemoveEntities, SetEntityData,
    SpawnEntity, TakeItemEntity,
};
use ferrumc_session::SessionRouter;
use ferrumc_sim::{EntityData, ItemEntitySnapshot};
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
    views: BTreeMap<PlayerId, BTreeSet<EntityId>>,
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
        snapshot: &ItemEntitySnapshot,
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
            if known.contains(&wire_id) {
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
