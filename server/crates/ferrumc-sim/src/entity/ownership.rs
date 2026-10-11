//! Private whole-store reservation and preflighted item publication.

use ferrumc_storage::{OwnedEntityExpectation, OwnershipTransferId, PersistentEntityRecord};

use super::item::{checked_stack_bytes, codec, require_item_type};
use super::{
    EntityData, EntityHandle, EntitySpawn, EntityStore, EntityStoreError, ItemEntitySpawn,
    ItemEntityState,
};
use crate::{LoadedChunkMap, PlayerOwnershipError};

#[derive(Debug)]
pub(crate) enum PreparedOwnedEntity {
    Spawn {
        base: EntitySpawn,
        item: ItemEntityState,
        bucket: usize,
        used: usize,
    },
    Replace {
        handle: EntityHandle,
        item: ItemEntityState,
        used: usize,
    },
    Remove {
        handle: EntityHandle,
        base: EntityData,
        bucket: usize,
        used: usize,
    },
}

#[derive(Debug)]
pub(crate) enum EntityPublication {
    Spawned(EntityHandle),
    Changed(EntityHandle),
    Removed(EntityData),
}

impl EntityStore {
    pub(super) fn require_legacy_write(&self) -> Result<(), EntityStoreError> {
        if self.ownership_mode || self.snapshot_only || self.reservation.is_some() {
            return Err(EntityStoreError::OwnershipRequired);
        }
        Ok(())
    }

    pub(crate) fn initialize_ownership(&mut self) -> Result<(), PlayerOwnershipError> {
        if self.ownership_mode || self.snapshot_only || !self.is_empty() {
            return Err(PlayerOwnershipError::AlreadyInitialized);
        }
        self.ownership_mode = true;
        Ok(())
    }

    fn require_owner(&self) -> Result<(), PlayerOwnershipError> {
        if self.snapshot_only {
            return Err(PlayerOwnershipError::SnapshotOnly);
        }
        if !self.ownership_mode {
            return Err(PlayerOwnershipError::NotInitialized);
        }
        if self.reservation.is_some() {
            return Err(PlayerOwnershipError::Reserved);
        }
        Ok(())
    }

    pub(crate) fn owned_expectation(
        &self,
        handle: &EntityHandle,
    ) -> Result<&OwnedEntityExpectation, PlayerOwnershipError> {
        let index = self.item_index(handle)?;
        self.slots[index]
            .ownership
            .as_ref()
            .ok_or(PlayerOwnershipError::StaleSnapshot)
    }

    pub(crate) fn prepare_owned_spawn(
        &self,
        chunks: &LoadedChunkMap,
        proposal: &ItemEntitySpawn,
    ) -> Result<(PreparedOwnedEntity, PersistentEntityRecord), PlayerOwnershipError> {
        self.require_owner()?;
        let base = proposal.base();
        require_item_type(base.entity_type)?;
        let bucket = self.validate_spawn(base)?;
        if !chunks.is_loaded(base.transform.chunk()) {
            return Err(EntityStoreError::ChunkNotResident {
                chunk: base.transform.chunk(),
            }
            .into());
        }
        proposal.properties().validate()?;
        let bytes = checked_stack_bytes(proposal.stack())?;
        let used =
            self.replaced_item_bytes(0, crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len())?;
        self.check_confirmed_replacement(0, crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len())?;
        let data = EntityData {
            owner: self.owner,
            uuid: base.uuid,
            wire_id: base.wire_id,
            entity_type: base.entity_type,
            transform: base.transform,
        };
        let (_, record) = codec::encode_record(data, proposal.properties(), &bytes)?;
        let item = ItemEntityState {
            stack: proposal.stack().clone(),
            properties: proposal.properties(),
            fcis_len: bytes.len(),
        };
        Ok((
            PreparedOwnedEntity::Spawn {
                base,
                item,
                bucket,
                used,
            },
            record,
        ))
    }

    pub(crate) fn prepare_owned_pickup(
        &self,
        chunks: &LoadedChunkMap,
        handle: &EntityHandle,
        stack: Option<&ferrumc_items::ItemStack>,
    ) -> Result<(PreparedOwnedEntity, Option<PersistentEntityRecord>), PlayerOwnershipError> {
        self.require_owner()?;
        let index = self.item_index(handle)?;
        let base = *self.get(handle)?;
        if !chunks.is_loaded(base.chunk()) {
            return Err(EntityStoreError::ChunkNotResident {
                chunk: base.chunk(),
            }
            .into());
        }
        let old_len =
            crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + self.checked_item_bytes(index)?.len();
        if let Some(stack) = stack {
            let bytes = checked_stack_bytes(stack)?;
            let original = self.item_state(handle)?.properties();
            let counters = original.counters();
            let properties = if counters.pickup_delay() < 0 {
                super::ItemEntityProperties::try_new(
                    original.motion(),
                    original.bob_offset(),
                    super::ItemEntityCounters::new(counters.age(), 0, counters.health()),
                    original.owner_target(),
                    original.thrower(),
                )?
            } else {
                original
            };
            let used = self.replaced_item_bytes(
                old_len,
                crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len(),
            )?;
            let confirmed_old = self.slots[index]
                .durability
                .as_ref()
                .map_or(0, |d| d.confirmed.record().data().len());
            self.check_confirmed_replacement(
                confirmed_old,
                crate::NATIVE_ITEM_ENTITY_HEADER_BYTES + bytes.len(),
            )?;
            let (_, record) = codec::encode_record(base, properties, &bytes)?;
            let item = ItemEntityState {
                stack: stack.clone(),
                properties,
                fcis_len: bytes.len(),
            };
            Ok((
                PreparedOwnedEntity::Replace {
                    handle: handle.clone(),
                    item,
                    used,
                },
                Some(record),
            ))
        } else {
            let used = self.replaced_item_bytes(old_len, 0)?;
            let bucket = self.bucket(base.chunk())?;
            Ok((
                PreparedOwnedEntity::Remove {
                    handle: handle.clone(),
                    base,
                    bucket,
                    used,
                },
                None,
            ))
        }
    }

    pub(crate) fn prepare_owned_item_save(
        &self,
        handle: &EntityHandle,
        delete: bool,
    ) -> Result<(PreparedOwnedEntity, Option<PersistentEntityRecord>), PlayerOwnershipError> {
        self.require_owner()?;
        let index = self.item_index(handle)?;
        let base = *self.get(handle)?;
        let item = self.item_state(handle)?;
        if delete {
            let used = self.replaced_item_bytes(item.native_body_len(), 0)?;
            return Ok((
                PreparedOwnedEntity::Remove {
                    handle: handle.clone(),
                    base,
                    bucket: self.bucket(base.chunk())?,
                    used,
                },
                None,
            ));
        }
        self.check_confirmed_replacement(
            self.slots[index]
                .durability
                .as_ref()
                .map_or(0, |d| d.confirmed.record().data().len()),
            item.native_body_len(),
        )?;
        let bytes = self.checked_item_bytes(index)?;
        let (_, record) = codec::encode_record(base, item.properties(), &bytes)?;
        Ok((
            PreparedOwnedEntity::Replace {
                handle: handle.clone(),
                item: item.snapshot_admitted(),
                used: self.item_bytes_used,
            },
            Some(record),
        ))
    }

    pub(crate) fn reserve_owned(
        &mut self,
        id: OwnershipTransferId,
        prepared: &PreparedOwnedEntity,
    ) -> Result<(), PlayerOwnershipError> {
        self.require_owner()?;
        if matches!(prepared, PreparedOwnedEntity::Spawn { .. }) {
            self.reserve_spawn_slot()?;
        }
        self.reservation = Some(id);
        Ok(())
    }

    pub(crate) fn check_reserved(
        &self,
        id: OwnershipTransferId,
        prepared: &PreparedOwnedEntity,
    ) -> Result<(), PlayerOwnershipError> {
        if self.snapshot_only {
            return Err(PlayerOwnershipError::SnapshotOnly);
        }
        if self.reservation != Some(id) {
            return Err(PlayerOwnershipError::CompletionMismatch);
        }
        match prepared {
            PreparedOwnedEntity::Spawn { base, .. } => {
                self.validate_spawn(*base)?;
            }
            PreparedOwnedEntity::Replace { handle, .. }
            | PreparedOwnedEntity::Remove { handle, .. } => {
                self.item_index(handle)?;
            }
        }
        Ok(())
    }

    pub(crate) fn release_owned(&mut self, id: OwnershipTransferId) {
        if self.reservation == Some(id) {
            self.reservation = None;
        }
    }

    // Caller checks both resources before this infallible synchronous publish.
    pub(crate) fn publish_owned(
        &mut self,
        id: OwnershipTransferId,
        prepared: PreparedOwnedEntity,
        expected: OwnedEntityExpectation,
    ) -> EntityPublication {
        let result = match prepared {
            PreparedOwnedEntity::Spawn {
                base,
                item,
                bucket,
                used,
            } => {
                let handle = self.commit_spawn(base, Some(item), bucket);
                self.slots[handle.slot].ownership = Some(expected);
                self.item_bytes_used = used;
                EntityPublication::Spawned(handle)
            }
            PreparedOwnedEntity::Replace { handle, item, used } => {
                self.slots[handle.slot].item = Some(item);
                self.slots[handle.slot].ownership = Some(expected);
                self.item_bytes_used = used;
                EntityPublication::Changed(handle)
            }
            PreparedOwnedEntity::Remove {
                handle,
                base,
                bucket,
                used,
            } => {
                self.remove_slot(handle.slot, base, bucket);
                self.item_bytes_used = used;
                EntityPublication::Removed(base)
            }
        };
        self.release_owned(id);
        result
    }
}
