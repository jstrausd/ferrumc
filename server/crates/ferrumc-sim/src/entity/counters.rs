//! Narrow scalar mutation authority; never changes a reserved native postimage.

use ferrumc_core::{EntityUuid, Tick};
use ferrumc_math::ChunkPos;
use ferrumc_storage::PersistentEntityRecord;

use super::{EntityHandle, EntityStore, ItemEntityError};
use crate::item_counters::{CounterBoundaryEntry, DeferredOwnedCounters, OwnedItemDurability};
use crate::{LoadedChunkMap, PlayerOwnershipError};

impl EntityStore {
    pub(crate) fn initialize_counters(&mut self, cap: usize) -> Result<(), PlayerOwnershipError> {
        if !self.ownership_mode || self.confirmed_item_cap.is_some() || !self.is_empty() {
            return Err(PlayerOwnershipError::AlreadyInitialized);
        }
        self.confirmed_item_cap = Some(cap);
        Ok(())
    }

    pub(crate) fn item_durability(
        &self,
        handle: &EntityHandle,
    ) -> Result<&OwnedItemDurability, PlayerOwnershipError> {
        self.slots[self.item_index(handle)?]
            .durability
            .as_ref()
            .ok_or(PlayerOwnershipError::NotInitialized)
    }

    pub(crate) fn check_confirmed_replacement(
        &self,
        old: usize,
        new: usize,
    ) -> Result<(), PlayerOwnershipError> {
        if let Some(cap) = self.confirmed_item_cap {
            let used = self
                .confirmed_item_bytes
                .checked_sub(old)
                .and_then(|n| n.checked_add(new))
                .ok_or(PlayerOwnershipError::Capacity("confirmed item bytes"))?;
            if used > cap {
                return Err(PlayerOwnershipError::Capacity("confirmed item bytes"));
            }
        }
        Ok(())
    }

    pub(crate) fn validate_confirmed_accounting(&self) -> Result<(), ItemEntityError> {
        let mut actual = 0usize;
        for durable in self.slots.iter().filter_map(|s| s.durability.as_ref()) {
            actual = actual
                .checked_add(durable.confirmed.record().data().len())
                .ok_or(ItemEntityError::ItemByteBudgetExceeded {
                    required: usize::MAX,
                    limit: self.confirmed_item_cap.unwrap_or(0),
                })?;
        }
        if actual > self.confirmed_item_cap.unwrap_or(0) {
            return Err(ItemEntityError::ItemByteBudgetExceeded {
                required: actual,
                limit: self.confirmed_item_cap.unwrap_or(0),
            });
        }
        if actual != self.confirmed_item_bytes {
            return Err(ItemEntityError::AggregateItemBytesMismatch {
                cached: self.confirmed_item_bytes,
                actual,
            });
        }
        // The complete retained aggregate is checked before decoding any row
        // or cloning either native body in a snapshot.
        for slot in &self.slots {
            if let Some(durable) = &slot.durability {
                let Some(expected) = &slot.ownership else {
                    return Err(ItemEntityError::MissingItemState);
                };
                super::decode_item_entity(expected.key(), &durable.confirmed)?;
            }
        }
        Ok(())
    }

    pub(crate) fn install_confirmed(
        &mut self,
        handle: &EntityHandle,
        record: PersistentEntityRecord,
        generation: u64,
        tick: Tick,
    ) {
        if self.confirmed_item_cap.is_none() {
            return;
        }
        let old = self.slots[handle.slot]
            .durability
            .as_ref()
            .map_or(0, |d| d.confirmed.record().data().len());
        self.confirmed_item_bytes = self.confirmed_item_bytes - old + record.record().data().len();
        self.slots[handle.slot].durability = Some(OwnedItemDurability {
            confirmed: record,
            generation,
            saved: generation,
            saved_tick: tick,
            due: false,
            priority: false,
            first_due: None,
        });
    }

    pub(crate) fn apply_counter_publication(
        &mut self,
        uuid: EntityUuid,
        projection: DeferredOwnedCounters,
    ) {
        let Some(&index) = self.by_uuid.get(&uuid) else {
            panic!("preflighted item defer publication disappeared");
        };
        let slot = &mut self.slots[index];
        if let (Some(item), Some(durable)) = (&mut slot.item, &mut slot.durability) {
            item.properties = item.properties.with_counters(projection.counters);
            durable.generation = projection.generation;
            durable.due = projection.due;
            durable.first_due = projection.first_due;
            durable.priority |= projection.due;
        } else {
            panic!("preflighted item defer state disappeared");
        }
    }

    pub(crate) fn append_counter_entries(
        &self,
        chunks: &LoadedChunkMap,
        entries: &mut Vec<CounterBoundaryEntry>,
    ) {
        for (index, slot) in self.slots.iter().enumerate() {
            if let (Some(base), Some(item), Some(durable)) =
                (slot.entity, &slot.item, &slot.durability)
            {
                entries.push(CounterBoundaryEntry {
                    original_counters: item.properties().counters(),
                    original_generation: durable.generation,
                    slot: index,
                    uuid: base.uuid(),
                    counters: item.properties().counters(),
                    generation: durable.generation,
                    deferred: None,
                    claims: 0,
                    due: durable.due,
                    resident: chunks.is_loaded(base.chunk()),
                    present: true,

                    saved: durable.saved,
                    wire: base.wire_id(),
                    body_len: item.native_body_len(),
                    confirmed_len: durable.confirmed.record().data().len(),
                    first_due: durable.first_due,
                });
            }
        }
    }

    pub(crate) fn apply_counter_entry(&mut self, entry: &CounterBoundaryEntry) {
        let index = if entry.slot == usize::MAX {
            match self.by_uuid.get(&entry.uuid) {
                Some(index) => *index,
                None => panic!("preflighted item publication disappeared"),
            }
        } else {
            entry.slot
        };
        let Some(slot) = self.slots.get_mut(index) else {
            panic!("preflighted item slot disappeared");
        };
        assert!(
            slot.entity.is_some_and(|base| base.uuid() == entry.uuid),
            "preflighted item slot identity changed"
        );
        if let (Some(item), Some(durable)) = (&mut slot.item, &mut slot.durability) {
            item.properties = item.properties.with_counters(entry.counters);
            durable.generation = entry.generation;
            durable.due = entry.due;
            durable.first_due = entry.first_due;
            durable.priority |= entry.due;
        } else {
            panic!("preflighted item state disappeared");
        }
    }

    pub(crate) fn reserve_counter_boundary_slots(
        &mut self,
        extra: usize,
    ) -> Result<(), PlayerOwnershipError> {
        let needed = self
            .slots
            .len()
            .checked_add(extra)
            .ok_or(PlayerOwnershipError::Capacity("future item slots"))?
            .min(self.capacity);
        if needed > self.slots.capacity() {
            self.slots
                .try_reserve_exact(needed - self.slots.len())
                .map_err(|_| PlayerOwnershipError::AllocationFailed)?;
        }
        Ok(())
    }

    pub(crate) fn counter_effective_capacity(&self) -> usize {
        self.capacity - self.retired
    }
    pub(crate) fn counter_slot_retires(&self, index: usize) -> bool {
        self.slots
            .get(index)
            .is_some_and(|s| s.generation == u64::MAX)
    }
    #[cfg(test)]
    pub(crate) fn inject_counter_generation(&mut self, handle: &EntityHandle, generation: u64) {
        self.slots[handle.slot]
            .durability
            .as_mut()
            .unwrap()
            .generation = generation;
    }
    #[cfg(test)]
    pub(crate) fn inject_confirmed_state(
        &mut self,
        handle: &EntityHandle,
        record: PersistentEntityRecord,
        cap: usize,
    ) {
        self.slots[handle.slot]
            .durability
            .as_mut()
            .unwrap()
            .confirmed = record;
        self.confirmed_item_cap = Some(cap);
    }

    pub(crate) fn slot_count(&self) -> usize {
        self.slots.len()
    }
    pub(crate) fn slot_handle(&self, index: usize) -> Option<EntityHandle> {
        self.slots
            .get(index)
            .and_then(|s| s.entity.map(|_| self.handle(index)))
    }
    pub(crate) fn counter_entity_reserved(&self) -> bool {
        self.reservation.is_some()
    }
    pub(crate) fn counter_quiescent(&self) -> bool {
        self.slots
            .iter()
            .filter_map(|s| s.durability.as_ref())
            .all(|d| d.saved == d.generation && !d.due)
    }
    pub(crate) fn restore_counter_flush(&mut self, handle: &EntityHandle, priority: bool) {
        if let Some(d) = &mut self.slots[handle.slot].durability {
            if priority && d.generation != d.saved {
                d.priority = true;
            }
        }
    }

    pub(crate) fn mark_counter_flush(&mut self, chunk: Option<ChunkPos>) {
        for slot in &mut self.slots {
            if slot
                .entity
                .is_some_and(|b| chunk.is_none_or(|c| b.chunk() == c))
            {
                if let Some(d) = &mut slot.durability {
                    d.priority = true;
                }
            }
        }
    }
}
