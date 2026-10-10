//! UUID operations hold one consistent owned-state boundary; no guard crosses await.

use std::num::NonZeroU16;

use async_trait::async_trait;
use ferrumc_core::Result;
use ferrumc_world::GenerationIdentity;

use super::{poisoned, InMemoryStore};
use crate::persistent_entity::engine::{self, MemoryRead};
use crate::persistent_entity::{raw, require_mode};
use crate::{
    ChunkKey, PersistentEntityCursor, PersistentEntityKey, PersistentEntityPage,
    PersistentEntityRecord, PersistentEntityStore, PersistentEntityWrite, StorageError,
};

#[async_trait]
impl PersistentEntityStore for InMemoryStore {
    async fn load_persistent_entity(
        &self,
        key: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> Result<Option<PersistentEntityRecord>> {
        require_mode(self.generation_mode)?;
        let _boundary = self.mutation_guard()?;
        let generations = self
            .generations
            .read()
            .map_err(|_| poisoned("generation"))?;
        let state = self
            .uuid_entities
            .read()
            .map_err(|_| poisoned("UUID entity"))?;
        let reader = MemoryRead {
            state: &state,
            generations: &generations,
        };
        engine::require_identity(&reader, engine::scope(key), Some(expected))?;
        let entity = engine::lookup(&reader, key)?;
        entity
            .filter(|entity| entity.key == key)
            .map(|entity| raw::envelope(entity.bytes)?.owned())
            .transpose()
            .map_err(Into::into)
    }

    async fn save_persistent_entity(&self, write: PersistentEntityWrite) -> Result<()> {
        self.save_persistent_entities(vec![write]).await
    }

    async fn save_persistent_entities(&self, writes: Vec<PersistentEntityWrite>) -> Result<()> {
        require_mode(self.generation_mode)?;
        let writes = engine::prepare(writes)?;
        if writes.is_empty() {
            return Ok(());
        }
        let _boundary = self.mutation_guard()?;
        let generations = self
            .generations
            .read()
            .map_err(|_| poisoned("generation"))?;
        let mut state = self
            .uuid_entities
            .write()
            .map_err(|_| poisoned("UUID entity"))?;
        let plan = engine::save_plan(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            writes,
        )?;
        state.apply(plan);
        Ok(())
    }

    async fn delete_persistent_entity(
        &self,
        key: PersistentEntityKey,
        expected: GenerationIdentity,
    ) -> Result<bool> {
        require_mode(self.generation_mode)?;
        let _boundary = self.mutation_guard()?;
        let generations = self
            .generations
            .read()
            .map_err(|_| poisoned("generation"))?;
        let mut state = self
            .uuid_entities
            .write()
            .map_err(|_| poisoned("UUID entity"))?;
        let (plan, removed) = engine::delete_plan(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            key,
            expected,
        )?;
        state.apply(plan);
        Ok(removed)
    }

    async fn move_persistent_entity(
        &self,
        source: PersistentEntityKey,
        source_expected: GenerationIdentity,
        destination: PersistentEntityWrite,
    ) -> Result<()> {
        require_mode(self.generation_mode)?;
        let destination = engine::prepare(vec![destination])?.pop().ok_or(
            StorageError::InvalidPersistentEntityMove {
                reason: "missing destination proposal",
            },
        )?;
        let _boundary = self.mutation_guard()?;
        let generations = self
            .generations
            .read()
            .map_err(|_| poisoned("generation"))?;
        let mut state = self
            .uuid_entities
            .write()
            .map_err(|_| poisoned("UUID entity"))?;
        let plan = engine::move_plan(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            source,
            source_expected,
            destination,
        )?;
        state.apply(plan);
        Ok(())
    }

    async fn load_persistent_entity_page(
        &self,
        chunk: ChunkKey,
        expected: GenerationIdentity,
        cursor: Option<PersistentEntityCursor>,
        limit: NonZeroU16,
    ) -> Result<PersistentEntityPage> {
        require_mode(self.generation_mode)?;
        let _boundary = self.mutation_guard()?;
        let generations = self
            .generations
            .read()
            .map_err(|_| poisoned("generation"))?;
        let state = self
            .uuid_entities
            .read()
            .map_err(|_| poisoned("UUID entity"))?;
        engine::page(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            &self.entity_incarnation,
            chunk,
            expected,
            cursor.as_ref(),
            limit,
        )
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Barrier};

    use ferrumc_core::{DimensionId, EntityUuid, WorldId};
    use ferrumc_math::ChunkPos;
    use ferrumc_world::WorldSeed;

    use super::*;
    use crate::{EntityRecord, GenerationKey, SchemaVersion, WorldStore};

    fn identity() -> GenerationIdentity {
        GenerationIdentity::flat_overworld(WorldSeed::new(0))
    }
    fn key(dimension: u32, uuid: u8) -> PersistentEntityKey {
        PersistentEntityKey::new(
            WorldId::new(0),
            DimensionId::new(dimension),
            EntityUuid::from_bytes([uuid; 16]),
        )
    }
    fn write(dimension: u32, uuid: u8, chunk: i32, body: u8) -> PersistentEntityWrite {
        PersistentEntityWrite::new(
            key(dimension, uuid),
            identity(),
            PersistentEntityRecord::new(
                ChunkPos::new(chunk, 0),
                EntityRecord::new(SchemaVersion::new(0), vec![body]).unwrap(),
            )
            .unwrap(),
        )
    }
    fn chunk(dimension: u32, x: i32) -> ChunkKey {
        ChunkKey::new(
            WorldId::new(0),
            DimensionId::new(dimension),
            ChunkPos::new(x, 0),
        )
    }
    type Snapshot = [BTreeMap<Vec<u8>, Vec<u8>>; 4];
    fn snapshot(store: &InMemoryStore) -> Snapshot {
        let state = store.uuid_entities.read().unwrap();
        [
            state.entities.clone(),
            state.locators.clone(),
            state.indexes.clone(),
            state.epochs.clone(),
        ]
    }
    async fn setup() -> InMemoryStore {
        let store = InMemoryStore::with_uuid_entities();
        for dimension in 0..2 {
            store
                .ensure_generation_identity(
                    GenerationKey::new(WorldId::new(0), DimensionId::new(dimension)),
                    identity(),
                )
                .await
                .unwrap();
        }
        store
            .save_persistent_entity(write(0, 1, 0, 1))
            .await
            .unwrap();
        store
    }
    #[tokio::test]
    async fn encountered_corruption_refuses_all_operations_without_repair() {
        for case in 0..7 {
            let store = setup().await;
            {
                let mut state = store.uuid_entities.write().unwrap();
                match case {
                    0 => {
                        state
                            .locators
                            .remove(raw::locator_key(key(0, 1)).as_slice());
                    }
                    1 => {
                        state
                            .locators
                            .insert(raw::locator_key(key(0, 1)).to_vec(), vec![0]);
                    }
                    2 => {
                        state
                            .indexes
                            .remove(raw::index_key(key(0, 1), ChunkPos::new(0, 0)).as_slice());
                    }
                    3 => {
                        state.indexes.insert(
                            raw::index_key(key(0, 1), ChunkPos::new(0, 0)).to_vec(),
                            vec![0],
                        );
                    }
                    4 => {
                        state
                            .epochs
                            .remove(crate::codec::chunk_key_bytes(chunk(0, 0)).as_slice());
                    }
                    5 => {
                        state.epochs.insert(
                            crate::codec::chunk_key_bytes(chunk(0, 0)).to_vec(),
                            vec![0; 8],
                        );
                    }
                    6 => {
                        state
                            .entities
                            .get_mut(raw::key_bytes(key(0, 1)).as_slice())
                            .unwrap()[0] = 0;
                    }
                    _ => unreachable!(),
                }
            }
            let before = snapshot(&store);
            assert!(
                store
                    .load_persistent_entity(key(0, 1), identity())
                    .await
                    .is_err(),
                "case {case}"
            );
            assert!(store
                .save_persistent_entity(write(0, 1, 0, 1))
                .await
                .is_err());
            assert!(store
                .delete_persistent_entity(key(0, 1), identity())
                .await
                .is_err());
            assert!(store
                .move_persistent_entity(key(0, 1), identity(), write(1, 1, 1, 2))
                .await
                .is_err());
            // A missing index cannot be found by a prefix scan; complete validator is separate.
            if case != 2 {
                assert!(store
                    .load_persistent_entity_page(
                        chunk(0, 0),
                        identity(),
                        None,
                        NonZeroU16::new(1).unwrap()
                    )
                    .await
                    .is_err());
            }
            if case == 0 {
                // With no locator, a miss in another dimension cannot discover
                // an unknown record without the forbidden global runtime scan.
                assert!(store
                    .load_persistent_entity(key(1, 1), identity())
                    .await
                    .unwrap()
                    .is_none());
                assert!(!store
                    .delete_persistent_entity(key(1, 1), identity())
                    .await
                    .unwrap());
            } else {
                assert!(store
                    .load_persistent_entity(key(1, 1), identity())
                    .await
                    .is_err());
                assert!(store
                    .delete_persistent_entity(key(1, 1), identity())
                    .await
                    .is_err());
            }
            assert_eq!(snapshot(&store), before);
        }
    }
    #[tokio::test]
    async fn known_orphan_destination_index_is_not_repaired_unknown_orphans_need_full_scan() {
        let store = setup().await;
        {
            let mut state = store.uuid_entities.write().unwrap();
            state.indexes.insert(
                raw::index_key(key(0, 2), ChunkPos::new(2, 0)).to_vec(),
                vec![],
            );
        }
        let before = snapshot(&store);
        assert!(store
            .load_persistent_entity(key(0, 2), identity())
            .await
            .unwrap()
            .is_none());
        assert!(store
            .save_persistent_entity(write(0, 2, 2, 2))
            .await
            .is_err());
        assert_eq!(snapshot(&store), before);
    }
    #[tokio::test]
    async fn max_epoch_allows_only_consistent_raw_noop_and_refuses_whole_real_batch() {
        let store = setup().await;
        store.uuid_entities.write().unwrap().epochs.insert(
            crate::codec::chunk_key_bytes(chunk(0, 0)).to_vec(),
            u64::MAX.to_be_bytes().to_vec(),
        );
        let before = snapshot(&store);
        store
            .save_persistent_entity(write(0, 1, 0, 1))
            .await
            .unwrap();
        assert_eq!(snapshot(&store), before);
        assert!(store
            .save_persistent_entity(write(0, 1, 0, 2))
            .await
            .is_err());
        assert!(store
            .save_persistent_entities(vec![write(0, 2, 2, 2), write(0, 1, 0, 2)])
            .await
            .is_err());
        assert!(store
            .delete_persistent_entity(key(0, 1), identity())
            .await
            .is_err());
        assert!(store
            .move_persistent_entity(key(0, 1), identity(), write(1, 1, 1, 2))
            .await
            .is_err());
        assert_eq!(snapshot(&store), before);
        store
            .uuid_entities
            .write()
            .unwrap()
            .locators
            .remove(raw::locator_key(key(0, 1)).as_slice());
        assert!(store
            .save_persistent_entity(write(0, 1, 0, 1))
            .await
            .is_err());
    }
    #[tokio::test]
    async fn malformed_short_chunkprefix_keys_are_encountered_before_nil_uuid() {
        let store = setup().await;
        let prefix = crate::codec::chunk_key_bytes(chunk(0, 0)).to_vec();
        store
            .uuid_entities
            .write()
            .unwrap()
            .indexes
            .insert(prefix, vec![]);
        let before = snapshot(&store);
        assert!(store
            .load_persistent_entity_page(chunk(0, 0), identity(), None, NonZeroU16::new(1).unwrap())
            .await
            .is_err());
        assert_eq!(snapshot(&store), before);
    }
    #[tokio::test]
    async fn historical_empty_epoch_prevents_a_new_generation_baseline() {
        let store = setup().await;
        store
            .delete_persistent_entity(key(0, 1), identity())
            .await
            .unwrap();
        assert!(store
            .ensure_generation_identity(
                GenerationKey::new(WorldId::new(2), DimensionId::new(2)),
                identity()
            )
            .await
            .is_err());
    }
    #[tokio::test]
    async fn concurrent_reader_observes_only_complete_lock_boundary_states() {
        let store = Arc::new(setup().await);
        let barrier = Arc::new(Barrier::new(2));
        let cloned = store.clone();
        let rendezvous = barrier.clone();
        // Writer deliberately stops while the whole entity state remains exclusively owned.
        let writer = std::thread::spawn(move || {
            let _boundary = cloned.mutation_guard().unwrap();
            let generations = cloned.generations.read().unwrap();
            let mut state = cloned.uuid_entities.write().unwrap();
            let plan = engine::move_plan(
                &MemoryRead {
                    state: &state,
                    generations: &generations,
                },
                key(0, 1),
                identity(),
                engine::prepare(vec![write(1, 1, 1, 2)])
                    .unwrap()
                    .pop()
                    .unwrap(),
            )
            .unwrap();
            rendezvous.wait();
            state.apply(plan);
        });
        barrier.wait();
        // Reader either acquires after commit or blocks on the same owned state lock.
        assert!(store
            .load_persistent_entity(key(0, 1), identity())
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .load_persistent_entity(key(1, 1), identity())
                .await
                .unwrap()
                .unwrap()
                .record()
                .data(),
            &[2]
        );
        writer.join().unwrap();
    }
}
