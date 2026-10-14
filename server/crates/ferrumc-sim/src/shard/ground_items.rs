//! Exclusive legacy ground-item consumer, entered only from the ordinary tick.

#![allow(clippy::result_large_err)]

use std::ops::Bound::{Excluded, Included, Unbounded};
use std::sync::Arc;

// Refusals retain complete bounded requests without a second allocation.

use super::{
    block_metadata, encode_persisted_stack, state_id_to_block_name, BTreeMap, BTreeSet, BlockPos,
    EntityStore, EntityStoreError, EntityTickContext, GameMode, LoadedChunkMap, PlayerId,
    PlayerState, SimShard, Vec3,
};
use crate::ground_items::{
    regional_store, regional_store_mut, GroundItemBatchRefusal, GroundItemDrop,
    GroundItemDropRefusal, GroundItemError, GroundItemEvent, GroundItemIdentity,
    GroundItemIdentityRefusal, GroundItemLimits, GroundItemPickupRefusal, GroundItemPickupResult,
    GroundItemPickupTicket, GroundItemRestore, GroundItemRestoreRefusal, GroundItems, GroundOffer,
    GROUND_TICK_BUDGET, MAX_GROUND_EVENTS, MAX_GROUND_EVENT_BYTES, MAX_GROUND_IDENTITIES,
    MAX_GROUND_OFFERS,
};
use crate::{
    EntityHandle, EntityTransform, ItemEntityCounters, ItemEntityError, ItemEntityProperties,
    ItemEntitySnapshot, ItemEntitySpawn,
};
use ferrumc_core::EntityUuid;

impl SimShard {
    /// Selects one bounded legacy consumer, adopting existing concrete item slots.
    ///
    /// Restore native records through `run_tick_with_entity_work` before this
    /// selection. After selection, generic entity work and native player-owner
    /// initialization refuse, so suspended pickup slots retain their credits.
    pub fn enable_ground_items(&mut self, limits: GroundItemLimits) -> Result<(), GroundItemError> {
        if self.player_ownership.is_some() {
            return Err(GroundItemError::OwnershipRequired);
        }
        if self.ground_items.is_some() {
            return Err(GroundItemError::AlreadyInitialized);
        }
        let entities = self
            .entities
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        if entities.item_byte_budget() == 0 {
            return Err(GroundItemError::NotInitialized);
        }
        if entities.owner().world() != self.chunks.world()
            || entities.owner().dimension() != self.chunks.dimension()
        {
            return Err(GroundItemError::ScopeMismatch);
        }
        // Whole-store preflight precedes runtime adoption; this existing checked
        // snapshot has a single explicit entity/native-byte ceiling.
        drop(entities.try_snapshot()?);
        let mut ground = GroundItems::new(entities.owner(), limits);
        ground.item_bytes = entities.item_bytes_used();
        ground.owner_byte_budget = entities.item_byte_budget();
        if ground.item_bytes > limits.pending_bytes {
            return Err(GroundItemError::Capacity("reserved entity bytes"));
        }
        for handle in entities.concrete_item_handles() {
            let base = entities.get(&handle).map_err(ItemEntityError::from)?;
            let item = entities.item_state(&handle)?;
            if self
                .mobs
                .as_ref()
                .is_some_and(|runtime| runtime.identity_used(base.uuid(), base.wire_id()))
            {
                return Err(GroundItemError::InvalidIdentity);
            }
            if ferrumc_items::encode_item_entity_metadata(item.stack())?
                .as_bytes()
                .len()
                > item.native_body_len()
            {
                return Err(GroundItemError::Capacity("metadata reservation"));
            }
            ground.unpublished.insert(base.uuid());
            ground.active.insert(base.uuid(), handle);
        }
        let other_slots = entities.len() - ground.active.len();
        if ground.active.len() > limits.items
            || limits.items > entities.capacity().saturating_sub(other_slots)
        {
            return Err(GroundItemError::InvalidLimit);
        }
        self.ground_items = Some(ground);
        Ok(())
    }

    /// Returns free slots in the bounded driver identity pool before allocation.
    pub fn ground_item_identity_room(&self) -> Result<usize, GroundItemError> {
        let ground = self
            .ground_items
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        ground.check_live()?;
        Ok(MAX_GROUND_IDENTITIES - ground.identities.len())
    }

    /// Refills explicit driver UUID/wire identities without spawning an entity.
    /// Any refusal returns the complete original vector, with no consumed id.
    pub fn refill_ground_item_identities(
        &mut self,
        identities: Vec<GroundItemIdentity>,
    ) -> Result<(), GroundItemIdentityRefusal> {
        let checked = (|| {
            let ground = self
                .ground_items
                .as_ref()
                .ok_or(GroundItemError::NotInitialized)?;
            ground.check_live()?;
            if identities.len() > MAX_GROUND_IDENTITIES - ground.identities.len() {
                return Err(GroundItemError::Capacity("identity pool"));
            }
            let entities = self
                .entities
                .as_ref()
                .ok_or(GroundItemError::NotInitialized)?;
            let mut uuids = BTreeSet::new();
            let mut wires = BTreeSet::new();
            for identity in ground.identities.iter().chain(&identities) {
                if identity.wire_id.get() < 2
                    || !uuids.insert(identity.uuid)
                    || !wires.insert(identity.wire_id)
                    || self.mobs.as_ref().is_some_and(|runtime| {
                        runtime.identity_used(identity.uuid, identity.wire_id)
                    })
                    || entities.handle_for_uuid(identity.uuid).is_some()
                    || entities.handle_for_wire_id(identity.wire_id).is_some()
                    || ground.regions.values().any(|store| {
                        store.handle_for_uuid(identity.uuid).is_some()
                            || store.handle_for_wire_id(identity.wire_id).is_some()
                    })
                    || ground.restores.iter().any(|(_, restore)| {
                        restore.proposal.key().uuid() == identity.uuid
                            || restore.wire_id == identity.wire_id
                    })
                    || ground.offers.values().any(|offer| {
                        offer.base.uuid() == identity.uuid
                            || offer.base.wire_id() == identity.wire_id
                    })
                {
                    return Err(GroundItemError::InvalidIdentity);
                }
            }
            Ok(())
        })();
        if let Err(reason) = checked {
            return Err(GroundItemIdentityRefusal { reason, identities });
        }
        let Some(ground) = self.ground_items.as_mut() else {
            unreachable!("checked ground owner");
        };
        ground.identities.extend(identities);
        Ok(())
    }

    /// Enqueues a complete owned drop; entity admission occurs at the next tick.
    /// A full owner/byte budget returns the unchanged request rather than losing
    /// an already removed player-inventory stack. Unloaded targets stay queued.
    pub fn enqueue_ground_item_drop(
        &mut self,
        drop: GroundItemDrop,
    ) -> Result<(), GroundItemDropRefusal> {
        let checked = (|| {
            let ground = self
                .ground_items
                .as_ref()
                .ok_or(GroundItemError::NotInitialized)?;
            let transform =
                EntityTransform::try_new(drop.position, 0.0, 0.0).map_err(ItemEntityError::from)?;
            let owner = crate::ShardPartitioner::for_chunk(
                ground.owner.world(),
                ground.owner.dimension(),
                transform.chunk(),
            );
            ground.check_region(owner)?;
            ground.reserve_drop_bytes(std::slice::from_ref(&drop.stack))
        })();
        let size = match checked {
            Ok(size) => size,
            Err(reason) => return Err(GroundItemDropRefusal { reason, drop }),
        };
        let Some(ground) = self.ground_items.as_mut() else {
            unreachable!("checked ground owner");
        };
        ground.admit_drop(size, drop);
        Ok(())
    }

    /// Admits a whole move-owned drop batch atomically (at most56 members).
    /// All native, metadata, pending slot/byte and aggregate regional preflights
    /// precede the first admission. Unloaded positions retain pending payloads;
    /// publication pressure delays spawning without discarding accepted drops.
    pub fn enqueue_ground_item_drops(
        &mut self,
        drops: Vec<GroundItemDrop>,
    ) -> Result<(), GroundItemBatchRefusal> {
        let prepared = (|| {
            if drops.len() > 56 {
                return Err(GroundItemError::Capacity("drop batch56"));
            }
            let ground = self
                .ground_items
                .as_ref()
                .ok_or(GroundItemError::NotInitialized)?;
            ground.reserve_drop_iter(drops.iter().map(|drop| &drop.stack))?;
            let mut owners = BTreeSet::new();
            let mut charges = Vec::with_capacity(drops.len());
            for drop in &drops {
                let transform = EntityTransform::try_new(drop.position, 0.0, 0.0)
                    .map_err(ItemEntityError::from)?;
                let owner = crate::ShardPartitioner::for_chunk(
                    ground.owner.world(),
                    ground.owner.dimension(),
                    transform.chunk(),
                );
                owners.insert(owner);
                charges.push(
                    ferrumc_items::persisted_stack_size(&drop.stack)?
                        + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES,
                );
            }
            let added = owners
                .iter()
                .filter(|owner| !ground.region_claims.contains(owner))
                .count();
            if ground.region_claims.len() + added > crate::ground_items::MAX_GROUND_REGIONS {
                return Err(GroundItemError::Capacity("canonical region owners"));
            }
            let publication = charges.iter().sum::<usize>() + drops.len() * 128;
            if !ground.can_publish(publication, drops.len()) {
                return Err(GroundItemError::Capacity("batch publication room"));
            }
            Ok(charges)
        })();
        let charges = match prepared {
            Ok(charges) => charges,
            Err(reason) => return Err(GroundItemBatchRefusal { reason, drops }),
        };
        let ground = self
            .ground_items
            .as_mut()
            .unwrap_or_else(|| panic!("prevalidated batch ground owner missing"));
        for (drop, charge) in drops.into_iter().zip(charges) {
            ground.admit_drop(charge, drop);
        }
        Ok(())
    }

    /// Admits exactly one complete inventory-death batch for the actual dead life.
    /// The caller must freeze its full original inventory and clear only after
    /// this successful whole-batch acknowledgement. Every member uses the actual
    /// owner death position. A repeated call cannot mint another death grant.
    pub fn enqueue_player_death_drops(
        &mut self,
        player: PlayerId,
        drops: Vec<GroundItemDrop>,
    ) -> Result<(), GroundItemBatchRefusal> {
        let checked = (|| {
            if self.player_ownership.is_some()
                || !self.survival.as_ref().is_some_and(|runtime| runtime.live)
            {
                return Err(GroundItemError::OwnershipRequired);
            }
            let actor = self
                .players
                .get(&player)
                .ok_or(GroundItemError::InvalidDeath)?;
            let state = actor
                .survival
                .as_ref()
                .ok_or(GroundItemError::InvalidDeath)?;
            if !state.vitals.is_dead() || drops.iter().any(|drop| drop.position != actor.position) {
                return Err(GroundItemError::InvalidDeath);
            }
            if state.death_drops_admitted {
                return Err(GroundItemError::DeathAlreadyAdmitted);
            }
            Ok(())
        })();
        if let Err(reason) = checked {
            return Err(GroundItemBatchRefusal { reason, drops });
        }
        self.enqueue_ground_item_drops(drops)?;
        let state = self
            .players
            .get_mut(&player)
            .and_then(|actor| actor.survival.as_mut())
            .unwrap_or_else(|| panic!("prevalidated dead inventory owner missing"));
        state.death_drops_admitted = true;
        Ok(())
    }

    /// Reads the actual once-per-death admission fact for a still-pinned caller
    /// reconciling its first prepared batch after a lost response. This is not an
    /// inventory projection, durable receipt or permission to replay a grant.
    pub fn player_death_drops_admitted(&self, player: PlayerId) -> Option<bool> {
        Some(
            self.players
                .get(&player)?
                .survival
                .as_ref()?
                .death_drops_admitted,
        )
    }

    /// Enqueues an exact decoded native item into its canonical region owner.
    /// Queues are bounded to 32 restores; unloaded targets retain their original
    /// proposal until residency permits a tick-context restore. Refusal returns
    /// the complete request, never a normalized/default entity.
    pub fn enqueue_ground_item_restore(
        &mut self,
        restore: GroundItemRestore,
    ) -> Result<(), GroundItemRestoreRefusal> {
        let checked = (|| {
            let ground = self
                .ground_items
                .as_ref()
                .ok_or(GroundItemError::NotInitialized)?;
            ground.check_live()?;
            if restore.wire_id.get() < 2 {
                return Err(GroundItemError::InvalidIdentity);
            }
            let key = restore.proposal.key();
            if key.world() != ground.owner.world() || key.dimension() != ground.owner.dimension() {
                return Err(GroundItemError::ScopeMismatch);
            }
            let owner = crate::ShardPartitioner::for_chunk(
                key.world(),
                key.dimension(),
                restore.proposal.transform().chunk(),
            );
            ground.check_region(owner)?;
            if ground.restores.len() >= GROUND_TICK_BUDGET {
                return Err(GroundItemError::Capacity("restore queue"));
            }
            let size = ground
                .reserve_drop_bytes(std::slice::from_ref(restore.proposal.item_state().stack()))?;
            if size + ground.restore_bytes + ground.pending_bytes > ground.limits.pending_bytes {
                return Err(GroundItemError::Capacity("restore bytes"));
            }
            let primary = self
                .entities
                .as_ref()
                .ok_or(GroundItemError::NotInitialized)?;
            if self
                .mobs
                .as_ref()
                .is_some_and(|runtime| runtime.identity_used(key.uuid(), restore.wire_id))
                || primary.handle_for_uuid(key.uuid()).is_some()
                || primary.handle_for_wire_id(restore.wire_id).is_some()
                || ground.regions.values().any(|store| {
                    store.handle_for_uuid(key.uuid()).is_some()
                        || store.handle_for_wire_id(restore.wire_id).is_some()
                })
                || ground.offers.values().any(|offer| {
                    offer.base.uuid() == key.uuid() || offer.base.wire_id() == restore.wire_id
                })
                || ground.identities.iter().any(|identity| {
                    identity.uuid == key.uuid() || identity.wire_id == restore.wire_id
                })
                || ground.restores.iter().any(|(_, queued)| {
                    queued.proposal.key().uuid() == key.uuid() || queued.wire_id == restore.wire_id
                })
            {
                return Err(GroundItemError::InvalidIdentity);
            }
            Ok((size, owner))
        })();
        let (size, owner) = match checked {
            Ok(value) => value,
            Err(reason) => return Err(GroundItemRestoreRefusal { reason, restore }),
        };
        let Some(ground) = self.ground_items.as_mut() else {
            unreachable!("checked ground owner");
        };
        ground.region_claims.insert(owner);
        ground.item_bytes += size;
        ground.restore_bytes += size;
        ground.restores.push_back((size, restore));
        Ok(())
    }

    /// Borrows the exact canonical entity store for a published item's owner,
    /// including regional stores used by the legacy world-cover application.
    /// Existing `capture_item`/`handle_for_uuid` methods supply checked save records.
    pub fn ground_entity_store(&self, owner: crate::ShardId) -> Option<&EntityStore> {
        let ground = self.ground_items.as_ref()?;
        regional_store(self.entities.as_ref()?, &ground.regions, owner)
    }

    /// Enumerates every chunk retaining a live, pending, restored or offered
    /// payload. App may acquire bounded tickets before releasing player tickets;
    /// this readonly query neither acquires residency nor proves persistence.
    pub fn ground_item_chunks(&self) -> Result<BTreeSet<ferrumc_math::ChunkPos>, GroundItemError> {
        let ground = self
            .ground_items
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        ground.check_live()?;
        let primary = self
            .entities
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        let mut chunks = BTreeSet::new();
        for handle in ground.active.values() {
            let store = regional_store(primary, &ground.regions, handle.owner())
                .ok_or(GroundItemError::ScopeMismatch)?;
            chunks.insert(store.get(handle).map_err(ItemEntityError::from)?.chunk());
        }
        for (_, drop) in &ground.pending {
            let transform =
                EntityTransform::try_new(drop.position, 0.0, 0.0).map_err(ItemEntityError::from)?;
            chunks.insert(transform.chunk());
        }
        chunks.extend(
            ground
                .restores
                .iter()
                .map(|(_, restore)| restore.proposal.transform().chunk()),
        );
        chunks.extend(ground.offers.values().map(|offer| offer.base.chunk()));
        Ok(chunks)
    }

    /// Captures bounded exact UUID records from their real canonical owners.
    /// Refuses while any payload is pending or offered. This performs no I/O,
    /// acknowledges no save and clears no dirty state.
    pub fn capture_ground_item_records(
        &self,
    ) -> Result<
        Vec<(
            ferrumc_storage::PersistentEntityKey,
            ferrumc_storage::PersistentEntityRecord,
        )>,
        GroundItemError,
    > {
        let ground = self
            .ground_items
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        ground.check_live()?;
        if !ground.pending.is_empty()
            || !ground.restores.is_empty()
            || !ground.offers.is_empty()
            || !ground.responses.is_empty()
        {
            return Err(GroundItemError::Capacity("unsettled payloads"));
        }
        let primary = self
            .entities
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        ground
            .active
            .values()
            .map(|handle| {
                regional_store(primary, &ground.regions, handle.owner())
                    .ok_or(GroundItemError::ScopeMismatch)?
                    .capture_item(handle)
                    .map_err(GroundItemError::from)
            })
            .collect()
    }

    /// Enqueues the genuine offer response after exact lossless conservation
    /// preflight. Refusal retains the ticket and every returned unit for retry.
    /// No player slot or entity payload changes during this call.
    pub fn enqueue_ground_item_pickup_result(
        &mut self,
        result: GroundItemPickupResult,
    ) -> Result<(), GroundItemPickupRefusal> {
        let checked = (|| {
            let ground = self
                .ground_items
                .as_ref()
                .ok_or(GroundItemError::NotInitialized)?;
            ground.check_live()?;
            let authority = ground
                .authority
                .as_ref()
                .ok_or(GroundItemError::SnapshotOnly)?;
            if !result.ticket.owner.ptr_eq(&Arc::downgrade(authority)) {
                return Err(GroundItemError::StaleTicket);
            }
            let offer = ground
                .offers
                .get(&result.ticket.sequence)
                .ok_or(GroundItemError::StaleTicket)?;
            if offer.response_queued {
                return Err(GroundItemError::StaleTicket);
            }
            if ground.responses.len() >= MAX_GROUND_OFFERS {
                return Err(GroundItemError::Capacity("pickup response queue"));
            }
            GroundItems::remainder_matches(offer, &result.remainder)
        })();
        if let Err(reason) = checked {
            return Err(GroundItemPickupRefusal { reason, result });
        }
        let Some(ground) = self.ground_items.as_mut() else {
            unreachable!("checked ground owner");
        };
        let Some(offer) = ground.offers.get_mut(&result.ticket.sequence) else {
            unreachable!("checked offer");
        };
        offer.response_queued = true;
        ground.responses.push_back(result);
        Ok(())
    }

    /// Drains at most 256 move-owned publications, releasing their outbox credit.
    /// Pickup offers must be routed or returned in full, including on disconnect.
    pub fn take_ground_item_events(
        &mut self,
        max: usize,
    ) -> Result<Vec<GroundItemEvent>, GroundItemError> {
        let ground = self
            .ground_items
            .as_mut()
            .ok_or(GroundItemError::NotInitialized)?;
        ground.check_live()?;
        let mut events = Vec::with_capacity(max.min(MAX_GROUND_EVENTS).min(ground.events.len()));
        for _ in 0..max.min(MAX_GROUND_EVENTS) {
            let Some((size, event)) = ground.events.pop_front() else {
                break;
            };
            ground.event_bytes -= size;
            events.push(event);
        }
        Ok(events)
    }

    /// Pages at most 32 readonly canonical item snapshots in UUID order, for a
    /// newly joined viewer. Continue after the last returned snapshot's UUID.
    /// Offered stacks remain exclusively with App until a genuine response.
    pub fn ground_item_snapshots(
        &self,
        after: Option<EntityUuid>,
        max: usize,
    ) -> Result<Vec<ItemEntitySnapshot>, GroundItemError> {
        let ground = self
            .ground_items
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        ground.check_live()?;
        let entities = self
            .entities
            .as_ref()
            .ok_or(GroundItemError::NotInitialized)?;
        let range = (after.map_or(Unbounded, Excluded), Unbounded);
        ground
            .active
            .range(range)
            .take(max.min(GROUND_TICK_BUDGET))
            .map(|(_, handle)| {
                regional_store(entities, &ground.regions, handle.owner())
                    .ok_or(GroundItemError::ScopeMismatch)?
                    .snapshot_item(handle)
                    .map_err(GroundItemError::from)
            })
            .collect()
    }

    /// Reports whether no queued stack, offer, response or publication needs
    /// completion before a controlled stop. Resident items can continue ticking.
    pub fn ground_items_quiescent(&self) -> bool {
        self.ground_items.as_ref().is_none_or(|ground| {
            ground.authority.is_some()
                && ground.pending.is_empty()
                && ground.restores.is_empty()
                && ground.offers.is_empty()
                && ground.responses.is_empty()
                && ground.events.is_empty()
        })
    }

    pub(super) fn drive_ground_items(&mut self) {
        let Some(mut ground) = self.ground_items.take() else {
            return;
        };
        let Some(entities) = self.entities.as_mut() else {
            panic!("ground runtime lost its entity owner");
        };
        assert!(
            ground.check_live().is_ok(),
            "ground snapshot has no tick authority"
        );
        assert!(
            ground.owner == entities.owner()
                && ground.owner.world() == self.chunks.world()
                && ground.owner.dimension() == self.chunks.dimension(),
            "ground owner scope changed"
        );
        assert_eq!(
            ground.item_bytes,
            entities.item_bytes_used()
                + ground
                    .regions
                    .values()
                    .map(EntityStore::item_bytes_used)
                    .sum::<usize>()
                + ground.pending_bytes
                + ground.restore_bytes
                + ground
                    .offers
                    .values()
                    .map(|offer| offer.before.len() + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES)
                    .sum::<usize>(),
            "ground item byte reservation differs from its owners"
        );
        ground.apply_responses(entities, &self.chunks);
        ground.restore_pending(entities, &self.chunks);
        ground.spawn_pending(entities, &self.chunks);
        ground.step_items(entities, &self.chunks, &self.players);
        ground.release_empty_regions();
        self.ground_items = Some(ground);
    }
}

impl GroundItems {
    fn restore_pending(&mut self, entities: &mut EntityStore, chunks: &LoadedChunkMap) {
        for _ in 0..self.restores.len().min(GROUND_TICK_BUDGET) {
            let Some((size, restore)) = self.restores.pop_front() else {
                break;
            };
            let owner = crate::ShardPartitioner::for_chunk(
                self.owner.world(),
                self.owner.dimension(),
                restore.proposal.transform().chunk(),
            );
            if !self.can_publish(size + 128, 1) || self.ensure_region(owner).is_err() {
                self.restores.push_back((size, restore));
                continue;
            }
            let uuid = restore.proposal.key().uuid();
            let Some(store) = regional_store_mut(entities, &mut self.regions, owner) else {
                panic!("restore region missing");
            };
            let handle = match EntityTickContext::new(store, chunks)
                .restore_item(restore.proposal, restore.wire_id)
            {
                Ok(handle) => handle,
                Err(refused) => {
                    let (_, proposal, wire_id) = refused.into_parts();
                    self.restores
                        .push_back((size, GroundItemRestore::new(proposal, wire_id)));
                    continue;
                }
            };
            self.restore_bytes -= size;
            self.active.insert(uuid, handle.clone());
            let snapshot = match store.snapshot_item(&handle) {
                Ok(snapshot) => snapshot,
                Err(error) => panic!("admitted restore snapshot failed: {error}"),
            };
            self.publish(size + 128, GroundItemEvent::Spawn { snapshot });
        }
    }

    fn release_empty_regions(&mut self) {
        let mut needed = BTreeSet::from([self.owner]);
        needed.extend(self.active.values().map(EntityHandle::owner));
        needed.extend(self.offers.values().map(|offer| offer.base.owner()));
        needed.extend(
            self.pending
                .iter()
                .map(|(_, drop)| self.owner_for_position(drop.position)),
        );
        needed.extend(self.restores.iter().map(|(_, restore)| {
            crate::ShardPartitioner::for_chunk(
                self.owner.world(),
                self.owner.dimension(),
                restore.proposal.transform().chunk(),
            )
        }));
        self.regions
            .retain(|owner, store| !store.is_empty() || needed.contains(owner));
        needed.extend(self.regions.keys().copied());
        self.region_claims = needed;
    }

    fn spawn_pending(&mut self, entities: &mut EntityStore, chunks: &LoadedChunkMap) {
        for _ in 0..self.pending.len().min(GROUND_TICK_BUDGET) {
            let Some(&(size, _)) = self.pending.front() else {
                break;
            };
            if !self.can_publish(size + 128, 1) {
                break;
            }
            let Some(identity) = self.identities.front().copied() else {
                break;
            };
            let Some((size, drop)) = self.pending.pop_front() else {
                break;
            };
            let owner = self.owner_for_position(drop.position);
            if self.ensure_region(owner).is_err() {
                self.pending.push_back((size, drop));
                continue;
            }
            let proposal = match self.proposal(identity, drop) {
                Ok(proposal) => proposal,
                Err(refused) => {
                    self.pending.push_front((size, refused.drop));
                    break;
                }
            };
            let Some(store) = regional_store_mut(entities, &mut self.regions, owner) else {
                panic!("admitted ground region is missing");
            };
            let handle = match EntityTickContext::new(store, chunks).spawn_item(proposal) {
                Ok(handle) => handle,
                Err(refused) => {
                    let (_, proposal) = refused.into_parts();
                    let (base, stack, properties) = proposal.into_parts();
                    self.pending.push_back((
                        size,
                        GroundItemDrop::new(
                            base.transform.position(),
                            stack,
                            properties.counters().pickup_delay(),
                            properties.owner_target(),
                            properties.thrower(),
                        ),
                    ));
                    continue;
                }
            };
            self.pending_bytes -= size;
            self.identities.pop_front();
            self.active.insert(identity.uuid, handle.clone());
            let snapshot = match store.snapshot_item(&handle) {
                Ok(snapshot) => snapshot,
                Err(error) => panic!("admitted ground snapshot failed: {error}"),
            };
            self.publish(size + 128, GroundItemEvent::Spawn { snapshot });
        }
    }

    fn apply_responses(&mut self, entities: &mut EntityStore, chunks: &LoadedChunkMap) {
        for _ in 0..GROUND_TICK_BUDGET {
            let Some(result) = self.responses.pop_front() else {
                break;
            };
            let sequence = result.ticket.sequence;
            let Some(offer) = self.offers.get(&sequence) else {
                panic!("admitted pickup response lost its reservation");
            };
            let accepted = offer.count - result.remainder.count();
            let size = offer.before.len() + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + 256;
            if !self.can_publish(size, 2) {
                self.responses.push_front(result);
                break;
            }
            let base = offer.base;
            let player = offer.player;
            let changed = if result.remainder.item().is_some() {
                let proposal = ItemEntitySpawn::new(
                    self.spawn_base(
                        GroundItemIdentity {
                            uuid: base.uuid(),
                            wire_id: base.wire_id(),
                        },
                        base.transform(),
                    ),
                    result.remainder,
                    match ItemEntityProperties::try_new(
                        offer.properties.motion(),
                        offer.properties.bob_offset(),
                        ItemEntityCounters::new(
                            offer.properties.counters().age(),
                            10,
                            offer.properties.counters().health(),
                        ),
                        offer.properties.owner_target(),
                        offer.properties.thrower(),
                    ) {
                        Ok(properties) => properties,
                        Err(error) => panic!("checked pickup cooldown failed: {error}"),
                    },
                );
                let owner = base.owner();
                if let Err(error) = self.ensure_region(owner) {
                    panic!("reserved pickup region disappeared: {error}");
                }
                let Some(store) = regional_store_mut(entities, &mut self.regions, owner) else {
                    panic!("pickup region missing");
                };
                let handle = match EntityTickContext::new(store, chunks).spawn_item(proposal) {
                    Ok(handle) => handle,
                    Err(refused) => {
                        let (_, proposal) = refused.into_parts();
                        let (_, remainder, _) = proposal.into_parts();
                        self.responses
                            .push_front(GroundItemPickupResult::new(result.ticket, remainder));
                        break;
                    }
                };
                self.active.insert(base.uuid(), handle.clone());
                let snapshot = match store.snapshot_item(&handle) {
                    Ok(snapshot) => snapshot,
                    Err(error) => panic!("restored ground snapshot failed: {error}"),
                };
                Some(snapshot)
            } else {
                None
            };
            let Some(offer) = self.offers.remove(&sequence) else {
                unreachable!("checked pickup offer");
            };
            self.offer_bytes -= offer.before.len();
            if accepted == offer.count {
                self.item_bytes -= offer.before.len() + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES;
            }
            if accepted != 0 {
                self.publish(
                    128,
                    GroundItemEvent::PickupAccepted {
                        player,
                        wire_id: base.wire_id(),
                        count: accepted,
                    },
                );
            }
            // Collect must precede both removal and a partial-stack metadata
            // update: the client animates and subtracts the accepted quantity
            // using its previous entity stack, just as original playerTouch.
            match changed {
                Some(snapshot) => {
                    self.publish(size - 128, GroundItemEvent::StackChanged { snapshot });
                }
                None => self.publish(128, GroundItemEvent::Remove { base }),
            }
        }
    }

    // One phase keeps publication and move-owned pickup reservations visibly ordered.
    #[allow(clippy::too_many_lines)]
    fn step_items(
        &mut self,
        entities: &mut EntityStore,
        chunks: &LoadedChunkMap,
        players: &BTreeMap<PlayerId, PlayerState>,
    ) {
        // Counter work is globally capped by the admitted 4096 slots. Timers
        // advance every logical tick independently of publication backpressure.
        for handle in self.active.values() {
            let store = regional_store_mut(entities, &mut self.regions, handle.owner())
                .unwrap_or_else(|| panic!("active counter region missing"));
            let base = *store
                .get(handle)
                .unwrap_or_else(|error| panic!("active counter handle invalid: {error}"));
            if !chunks.is_loaded(base.chunk()) {
                continue;
            }
            let old = store
                .item_state(handle)
                .unwrap_or_else(|error| panic!("active counter payload invalid: {error}"))
                .properties();
            let stepped = crate::step_active_item_counters(old.counters(), 6000);
            let properties = ItemEntityProperties::try_new(
                old.motion(),
                old.bob_offset(),
                stepped.counters(),
                old.owner_target(),
                old.thrower(),
            )
            .unwrap_or_else(|error| panic!("admitted counter properties invalid: {error}"));
            EntityTickContext::new(store, chunks)
                .set_item_properties(handle, properties)
                .unwrap_or_else(|error| panic!("checked active counter update failed: {error}"));
        }
        let mut keys: Vec<_> = self
            .active
            .range((self.cursor.map_or(Unbounded, Excluded), Unbounded))
            .take(256)
            .map(|(&uuid, _)| uuid)
            .collect();
        if keys.len() < 256 {
            if let Some(cursor) = self.cursor {
                keys.extend(
                    self.active
                        .range((Unbounded, Included(cursor)))
                        .take(256 - keys.len())
                        .map(|(&uuid, _)| uuid),
                );
            }
        }
        for uuid in keys {
            if !self.can_publish(256, 2) {
                break;
            }
            self.cursor = Some(uuid);
            let Some(mut handle) = self.active.get(&uuid).cloned() else {
                continue;
            };
            let owner = handle.owner();
            let Some(store) = regional_store_mut(entities, &mut self.regions, owner) else {
                panic!("active ground region is missing");
            };
            let base = match store.get(&handle) {
                Ok(base) => *base,
                Err(error) => panic!("ground handle invalidated outside its owner: {error}"),
            };
            if !chunks.is_loaded(base.chunk()) {
                continue;
            }
            if self.unpublished.contains(&uuid) {
                let size = match store.item_state(&handle) {
                    Ok(item) => item.native_body_len() + 128,
                    Err(error) => panic!("ground payload missing: {error}"),
                };
                if self.events.len() + 1 > MAX_GROUND_EVENTS
                    || self.event_bytes + size > MAX_GROUND_EVENT_BYTES
                {
                    break;
                }
                let snapshot = match store.snapshot_item(&handle) {
                    Ok(snapshot) => snapshot,
                    Err(error) => panic!("ground snapshot failed: {error}"),
                };
                self.unpublished.remove(&uuid);
                self.publish(size, GroundItemEvent::Spawn { snapshot });
                continue;
            }
            let properties = match store.item_state(&handle) {
                Ok(item) => item.properties(),
                Err(error) => panic!("ground payload missing: {error}"),
            };
            if properties.counters().age() >= 6000 {
                let removed = match EntityTickContext::new(store, chunks).remove_owned(&handle) {
                    Ok(removed) => removed,
                    Err(error) => panic!("checked ground despawn failed: {error}"),
                };
                if let Some(item) = removed.item_state() {
                    self.item_bytes -= item.native_body_len();
                }
                self.active.remove(&uuid);
                self.publish(
                    128,
                    GroundItemEvent::Remove {
                        base: removed.base(),
                    },
                );
                continue;
            }
            let (position, motion) =
                ground_physics(chunks, base.transform().position(), properties.motion());
            let transform = EntityTransform::try_new(
                position,
                base.transform().yaw(),
                base.transform().pitch(),
            );
            let moved = if let Ok(transform) = transform {
                let destination = crate::ShardPartitioner::for_chunk(
                    self.owner.world(),
                    self.owner.dimension(),
                    transform.chunk(),
                );
                if destination == owner {
                    EntityTickContext::new(store, chunks)
                        .update_transform(&handle, transform)
                        .is_ok()
                } else {
                    let properties = match ItemEntityProperties::try_new(
                        motion,
                        properties.bob_offset(),
                        properties.counters(),
                        properties.owner_target(),
                        properties.thrower(),
                    ) {
                        Ok(properties) => properties,
                        Err(error) => panic!("finite migration properties failed: {error}"),
                    };
                    if self.ensure_region(destination).is_err() {
                        false
                    } else {
                        match migrate_between_regions(
                            entities,
                            &mut self.regions,
                            chunks,
                            &handle,
                            destination,
                            transform,
                            properties,
                        ) {
                            Ok(new_handle) => {
                                handle = new_handle;
                                self.active.insert(uuid, handle.clone());
                                true
                            }
                            Err(_) => false,
                        }
                    }
                }
            } else {
                false
            };
            let properties = match ItemEntityProperties::try_new(
                if moved { motion } else { properties.motion() },
                properties.bob_offset(),
                properties.counters(),
                properties.owner_target(),
                properties.thrower(),
            ) {
                Ok(properties) => properties,
                Err(error) => panic!("finite ground properties failed: {error}"),
            };
            let Some(store) = regional_store_mut(entities, &mut self.regions, handle.owner())
            else {
                panic!("moved ground region is missing");
            };
            if let Err(error) =
                EntityTickContext::new(store, chunks).set_item_properties(&handle, properties)
            {
                panic!("checked ground counter update failed: {error}");
            }
            let base = match store.get(&handle) {
                Ok(base) => *base,
                Err(error) => panic!("ground handle lost: {error}"),
            };
            if moved {
                self.publish(128, GroundItemEvent::Move { base });
            }
            if properties.counters().pickup_delay() > 0 || self.offers.len() >= MAX_GROUND_OFFERS {
                continue;
            }
            let position = base.transform().position();
            let player = players.iter().find_map(|(&player, actor)| {
                let eligible = actor.is_alive()
                    && actor.game_mode != GameMode::Spectator
                    && properties
                        .owner_target()
                        .is_none_or(|target| target == EntityUuid::from_uuid(player.as_uuid()))
                    && (actor.position.x - position.x).abs() <= 1.0
                    && (actor.position.z - position.z).abs() <= 1.0
                    && position.y >= actor.position.y - 0.5
                    && position.y <= actor.position.y + 2.0;
                eligible.then_some(player)
            });
            let Some(player) = player else {
                continue;
            };
            let Some(sequence) = self.last_offer.checked_add(1) else {
                continue;
            };
            let Some(store) = regional_store_mut(entities, &mut self.regions, handle.owner())
            else {
                panic!("pickup ground region is missing");
            };
            let item = match store.item_state(&handle) {
                Ok(item) => item,
                Err(error) => panic!("ground item disappeared: {error}"),
            };
            let before = match encode_persisted_stack(item.stack()) {
                Ok(bytes) => bytes,
                Err(error) => panic!("admitted ground bytes invalid: {error}"),
            };
            let count = item.stack().count();
            if self.offer_bytes + before.len() > self.limits.pending_bytes
                || self.events.len() + 1 > MAX_GROUND_EVENTS
                || self.event_bytes + before.len() + 256 > MAX_GROUND_EVENT_BYTES
            {
                continue;
            }
            let removed = match EntityTickContext::new(store, chunks).remove_owned(&handle) {
                Ok(removed) => removed,
                Err(error) => panic!("checked ground pickup failed: {error}"),
            };
            let (base, item) = removed.into_parts();
            let Some(item) = item else {
                panic!("ground payload removal returned a placeholder");
            };
            let (stack, properties) = item.into_parts();
            self.active.remove(&uuid);
            self.offer_bytes += before.len();
            let size = before.len() + 256;
            self.offers.insert(
                sequence,
                GroundOffer {
                    base,
                    properties,
                    player,
                    before,
                    count,
                    response_queued: false,
                },
            );
            self.last_offer = sequence;
            let Some(authority) = self.authority.as_ref() else {
                panic!("ground snapshot attempted pickup");
            };
            let ticket = GroundItemPickupTicket {
                owner: Arc::downgrade(authority),
                sequence,
            };
            self.publish(
                size,
                GroundItemEvent::PickupOffer {
                    ticket,
                    player,
                    wire_id: base.wire_id(),
                    stack,
                },
            );
        }
    }
}

fn migrate_between_regions(
    primary: &mut EntityStore,
    regions: &mut BTreeMap<crate::ShardId, EntityStore>,
    chunks: &LoadedChunkMap,
    handle: &EntityHandle,
    destination: crate::ShardId,
    transform: EntityTransform,
    properties: ItemEntityProperties,
) -> Result<EntityHandle, ItemEntityError> {
    if destination == primary.owner() {
        let source = regions
            .get_mut(&handle.owner())
            .ok_or(EntityStoreError::NotInitialized)?;
        EntityTickContext::new(source, chunks).migrate_item_to(
            &mut EntityTickContext::new(primary, chunks),
            handle,
            transform,
            properties,
        )
    } else {
        let mut target = regions
            .remove(&destination)
            .ok_or(EntityStoreError::NotInitialized)?;
        let result = if let Some(source) = regional_store_mut(primary, regions, handle.owner()) {
            EntityTickContext::new(source, chunks).migrate_item_to(
                &mut EntityTickContext::new(&mut target, chunks),
                handle,
                transform,
                properties,
            )
        } else {
            Err(EntityStoreError::NotInitialized.into())
        };
        regions.insert(destination, target);
        result
    }
}

fn solid_at(chunks: &LoadedChunkMap, position: Vec3) -> bool {
    let block = BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    chunks
        .get(block.to_chunk_pos())
        .and_then(|chunk| chunk.get_block(block))
        .and_then(|state| state_id_to_block_name(state.as_u32()))
        .and_then(block_metadata)
        .is_some_and(|metadata| metadata.is_solid_cube)
}

fn ground_physics(chunks: &LoadedChunkMap, position: Vec3, motion: Vec3) -> (Vec3, Vec3) {
    let block = BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    let in_water = chunks
        .get(block.to_chunk_pos())
        .and_then(|chunk| chunk.get_block(block))
        .and_then(|state| state_id_to_block_name(state.as_u32()))
        == Some("water");
    let mut velocity = if in_water {
        Vec3::new(
            motion.x * 0.99,
            motion.y + if motion.y < 0.06 { 0.0005 } else { 0.0 },
            motion.z * 0.99,
        )
    } else {
        Vec3::new(motion.x, motion.y - 0.04, motion.z)
    };
    let distance = velocity.x.abs().max(velocity.y.abs()).max(velocity.z.abs());
    if !distance.is_finite() || distance > 4.0 {
        return (position, motion);
    }
    let steps = (distance / 0.25).ceil().max(1.0) as usize;
    let mut next = position;
    let mut grounded = false;
    for _ in 0..steps {
        let x = Vec3::new(next.x + velocity.x / steps as f64, next.y, next.z);
        if solid_at(chunks, x) {
            velocity.x = 0.0;
        } else {
            next.x = x.x;
        }
        let z = Vec3::new(next.x, next.y, next.z + velocity.z / steps as f64);
        if solid_at(chunks, z) {
            velocity.z = 0.0;
        } else {
            next.z = z.z;
        }
        let y = Vec3::new(next.x, next.y + velocity.y / steps as f64, next.z);
        if !solid_at(chunks, y) {
            next.y = y.y;
        } else if velocity.y < 0.0 {
            next.y = y.y.floor() + 1.0;
            grounded = true;
        } else {
            velocity.y = 0.0;
        }
    }
    let drag = if grounded { 0.6 * 0.98 } else { 0.98 };
    (
        next,
        Vec3::new(
            velocity.x * drag,
            velocity.y * if grounded { -0.5 * 0.98 } else { 0.98 },
            velocity.z * drag,
        ),
    )
}
