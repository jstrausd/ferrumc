//! Shared bounded preflight and encountered-relation checks for both backends.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::num::NonZeroU16;
use std::ops::Bound;
use std::sync::Arc;

use ferrumc_world::GenerationIdentity;

use super::raw;
use super::{
    validate_cursor, PersistentEntityCursor, PersistentEntityEntry, PersistentEntityEpoch,
    PersistentEntityKey, PersistentEntityPage, PersistentEntityWrite,
    MAX_PERSISTENT_ENTITY_BATCH_BYTES, MAX_PERSISTENT_ENTITY_PAGE_BYTES,
    MAX_PERSISTENT_ENTITY_PAGE_ENTRIES, MAX_PERSISTENT_ENTITY_WRITES,
};
use crate::{ChunkKey, GenerationKey, StorageError};

type Outcome<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Clone, Copy)]
pub(crate) enum RawTable {
    Entity,
    Locator,
    Index,
    Epoch,
}

pub(crate) type RawRows<'a, B> = Box<dyn Iterator<Item = Outcome<(B, B)>> + 'a>;

pub(crate) trait EntityRead {
    type Bytes<'a>: AsRef<[u8]>
    where
        Self: 'a;
    fn get(&self, table: RawTable, key: &[u8]) -> Outcome<Option<Self::Bytes<'_>>>;
    fn identity(&self, key: GenerationKey) -> Outcome<Option<GenerationIdentity>>;
    fn index_rows(
        &self,
        lower: Bound<Vec<u8>>,
        upper: Bound<Vec<u8>>,
    ) -> Outcome<RawRows<'_, Self::Bytes<'_>>>;
}

pub(crate) fn scope(key: PersistentEntityKey) -> GenerationKey {
    GenerationKey::new(key.world(), key.dimension())
}

pub(crate) fn chunk_key(key: PersistentEntityKey, pos: ferrumc_math::ChunkPos) -> ChunkKey {
    ChunkKey::new(key.world(), key.dimension(), pos)
}

pub(crate) fn require_identity(
    reader: &impl EntityRead,
    key: GenerationKey,
    expected: Option<GenerationIdentity>,
) -> Outcome<()> {
    let stored = reader
        .identity(key)?
        .ok_or(StorageError::MissingGenerationIdentity { key: Some(key) })?;
    if let Some(requested) = expected {
        crate::generation::compare_identity(key, stored, requested)?;
    }
    Ok(())
}

pub(crate) fn epoch(reader: &impl EntityRead, chunk: ChunkKey) -> Outcome<PersistentEntityEpoch> {
    crate::record::validate_chunk_position(chunk.pos())?;
    require_identity(
        reader,
        GenerationKey::new(chunk.world(), chunk.dimension()),
        None,
    )?;
    Ok(PersistentEntityEpoch(
        match reader.get(RawTable::Epoch, &crate::codec::chunk_key_bytes(chunk))? {
            Some(bytes) => raw::decode_epoch(bytes.as_ref())?,
            None => 0,
        },
    ))
}

/// Does not copy payloads, including raw no-op comparisons and page lookahead.
pub(crate) struct CheckedEntity<B> {
    pub(crate) key: PersistentEntityKey,
    pub(crate) chunk: ChunkKey,
    pub(crate) bytes: B,
}

pub(crate) fn verify<'a, R: EntityRead>(
    reader: &'a R,
    key: PersistentEntityKey,
    bytes: R::Bytes<'a>,
) -> Outcome<CheckedEntity<R::Bytes<'a>>> {
    let header = raw::envelope(bytes.as_ref())?;
    let chunk = chunk_key(key, header.chunk);
    require_identity(reader, scope(key), None)?;
    let locator = reader
        .get(RawTable::Locator, &raw::locator_key(key))?
        .ok_or_else(|| raw::corrupt(raw::LOCATOR, "record has no locator"))?;
    if raw::decode_locator(locator.as_ref())? != (key.dimension(), header.chunk) {
        return Err(raw::corrupt(
            raw::LOCATOR,
            "locator disagrees with entity envelope",
        ));
    }
    let index = reader
        .get(RawTable::Index, &raw::index_key(key, header.chunk))?
        .ok_or_else(|| raw::corrupt(raw::INDEX, "record has no chunk index"))?;
    if !index.as_ref().is_empty() {
        return Err(raw::corrupt(raw::INDEX, "index value must be empty"));
    }
    if epoch(reader, chunk)?.get() == 0 {
        return Err(raw::corrupt(raw::EPOCH, "indexed chunk has no epoch"));
    }
    Ok(CheckedEntity { key, chunk, bytes })
}

/// Resolves directly addressable local records and world-wide UUID locators.
/// An absent local key can still encounter a known record in another dimension.
pub(crate) fn lookup<R: EntityRead>(
    reader: &R,
    key: PersistentEntityKey,
) -> Outcome<Option<CheckedEntity<R::Bytes<'_>>>> {
    if let Some(bytes) = reader.get(RawTable::Entity, &raw::key_bytes(key))? {
        return verify(reader, key, bytes).map(Some);
    }
    let Some(locator) = reader.get(RawTable::Locator, &raw::locator_key(key))? else {
        return Ok(None);
    };
    let (dimension, chunk) = raw::decode_locator(locator.as_ref())?;
    let actual = PersistentEntityKey::new(key.world(), dimension, key.uuid());
    let bytes = reader
        .get(RawTable::Entity, &raw::key_bytes(actual))?
        .ok_or_else(|| raw::corrupt(raw::LOCATOR, "locator points to a missing record"))?;
    let checked = verify(reader, actual, bytes)?;
    if checked.chunk.pos() != chunk {
        return Err(raw::corrupt(
            raw::LOCATOR,
            "locator points to another chunk",
        ));
    }
    Ok(Some(checked))
}

/// Prepared encodings are bounded together before either backend mutates.
pub(crate) struct PreparedWrite {
    pub(crate) key: PersistentEntityKey,
    expected: GenerationIdentity,
    chunk: ChunkKey,
    bytes: Vec<u8>,
}

pub(crate) fn prepare(writes: Vec<PersistentEntityWrite>) -> Outcome<Vec<PreparedWrite>> {
    if writes.len() > MAX_PERSISTENT_ENTITY_WRITES {
        return Err(StorageError::BatchTooLarge {
            len: writes.len(),
            max: MAX_PERSISTENT_ENTITY_WRITES,
        });
    }
    let mut bytes = 0usize;
    let mut keys = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for write in &writes {
        bytes += raw::KEY_LEN + raw::HEADER_LEN + write.record().record().data().len();
        if bytes > MAX_PERSISTENT_ENTITY_BATCH_BYTES {
            return Err(StorageError::PersistentEntityBytesExceeded {
                len: bytes,
                max: MAX_PERSISTENT_ENTITY_BATCH_BYTES,
            });
        }
        if !keys.insert(write.key())
            || !identities.insert((write.key().world(), write.key().uuid()))
        {
            return Err(StorageError::DuplicatePersistentEntityWrite { key: write.key() });
        }
    }
    Ok(writes
        .into_iter()
        .map(|write| PreparedWrite {
            key: write.key(),
            expected: write.expected_identity(),
            chunk: chunk_key(write.key(), write.record().chunk()),
            bytes: raw::encode(write.record()),
        })
        .collect())
}

pub(crate) struct Change {
    pub(crate) key: PersistentEntityKey,
    pub(crate) old_chunk: Option<ChunkKey>,
    pub(crate) new: Option<(ChunkKey, Vec<u8>)>,
}

#[derive(Default)]
pub(crate) struct Plan {
    pub(crate) changes: Vec<Change>,
    pub(crate) epochs: BTreeMap<ChunkKey, u64>,
}

impl Plan {
    fn finish(reader: &impl EntityRead, changes: Vec<Change>) -> Outcome<Self> {
        let mut chunks = BTreeSet::new();
        for change in &changes {
            if let Some(old) = change.old_chunk {
                chunks.insert(old);
            }
            if let Some((new, _)) = &change.new {
                chunks.insert(*new);
            }
        }
        let mut epochs = BTreeMap::new();
        for chunk in chunks {
            let next = epoch(reader, chunk)?
                .get()
                .checked_add(1)
                .ok_or(StorageError::PersistentEntityEpochExhausted { chunk })?;
            epochs.insert(chunk, next);
        }
        Ok(Self { changes, epochs })
    }
}

fn require_free_index(
    reader: &impl EntityRead,
    key: PersistentEntityKey,
    chunk: ChunkKey,
) -> Outcome<()> {
    if reader
        .get(RawTable::Index, &raw::index_key(key, chunk.pos()))?
        .is_some()
    {
        return Err(raw::corrupt(
            raw::INDEX,
            "unexpected existing destination chunk index",
        ));
    }
    // A directly known malformed epoch is corruption even if this is an empty chunk.
    epoch(reader, chunk)?;
    Ok(())
}

pub(crate) fn save_plan(reader: &impl EntityRead, writes: Vec<PreparedWrite>) -> Outcome<Plan> {
    let mut changes = Vec::with_capacity(writes.len());
    for write in writes {
        require_identity(reader, scope(write.key), Some(write.expected))?;
        let old = lookup(reader, write.key)?;
        if let Some(old) = &old {
            if old.key != write.key {
                return Err(StorageError::PersistentEntityWorldUuidConflict { key: write.key });
            }
            if old.bytes.as_ref() == write.bytes.as_slice() {
                continue;
            }
        }
        let old_chunk = old.as_ref().map(|old| old.chunk);
        if old_chunk != Some(write.chunk) {
            require_free_index(reader, write.key, write.chunk)?;
        }
        changes.push(Change {
            key: write.key,
            old_chunk,
            new: Some((write.chunk, write.bytes)),
        });
    }
    Plan::finish(reader, changes)
}

pub(crate) fn delete_plan(
    reader: &impl EntityRead,
    key: PersistentEntityKey,
    expected: GenerationIdentity,
) -> Outcome<(Plan, bool)> {
    require_identity(reader, scope(key), Some(expected))?;
    let Some(old) = lookup(reader, key)? else {
        return Ok((Plan::default(), false));
    };
    if old.key != key {
        return Ok((Plan::default(), false));
    }
    Ok((
        Plan::finish(
            reader,
            vec![Change {
                key,
                old_chunk: Some(old.chunk),
                new: None,
            }],
        )?,
        true,
    ))
}

pub(crate) fn move_plan(
    reader: &impl EntityRead,
    source: PersistentEntityKey,
    source_expected: GenerationIdentity,
    destination: PreparedWrite,
) -> Outcome<Plan> {
    if source.world() != destination.key.world()
        || source.uuid() != destination.key.uuid()
        || source.dimension() == destination.key.dimension()
    {
        return Err(StorageError::InvalidPersistentEntityMove {
            reason: "move requires same world/UUID and different dimensions",
        });
    }
    require_identity(reader, scope(source), Some(source_expected))?;
    require_identity(reader, scope(destination.key), Some(destination.expected))?;
    let old = lookup(reader, source)?
        .filter(|old| old.key == source)
        .ok_or(StorageError::InvalidPersistentEntityMove {
            reason: "source does not exist",
        })?;
    if let Some(target) = lookup(reader, destination.key)? {
        if target.key != source {
            return Err(StorageError::InvalidPersistentEntityMove {
                reason: "destination is occupied",
            });
        }
    }
    require_free_index(reader, destination.key, destination.chunk)?;
    Plan::finish(
        reader,
        vec![
            Change {
                key: source,
                old_chunk: Some(old.chunk),
                new: None,
            },
            Change {
                key: destination.key,
                old_chunk: None,
                new: Some((destination.chunk, destination.bytes)),
            },
        ],
    )
}

pub(crate) fn page(
    reader: &impl EntityRead,
    owner: &Arc<()>,
    chunk: ChunkKey,
    expected: GenerationIdentity,
    cursor: Option<&PersistentEntityCursor>,
    limit: NonZeroU16,
) -> Outcome<PersistentEntityPage> {
    if limit.get() > MAX_PERSISTENT_ENTITY_PAGE_ENTRIES {
        return Err(StorageError::PersistentEntityPageLimit { limit: limit.get() });
    }
    if let Some(cursor) = cursor {
        validate_cursor(cursor, owner, chunk, expected, None)?;
    }
    require_identity(
        reader,
        GenerationKey::new(chunk.world(), chunk.dimension()),
        Some(expected),
    )?;
    let epoch = epoch(reader, chunk)?;
    if let Some(cursor) = cursor {
        validate_cursor(cursor, owner, chunk, expected, Some(epoch))?;
    }
    let prefix = crate::codec::chunk_key_bytes(chunk);
    let lower = match cursor {
        Some(cursor) => Bound::Excluded(
            raw::index_key(
                PersistentEntityKey::new(chunk.world(), chunk.dimension(), cursor.last),
                chunk.pos(),
            )
            .to_vec(),
        ),
        None => Bound::Included(prefix.to_vec()),
    };
    let upper =
        raw::prefix_upper(prefix).map_or(Bound::Unbounded, |upper| Bound::Excluded(upper.to_vec()));
    let mut entries = Vec::with_capacity(usize::from(limit.get()));
    let mut total = 0usize;
    let mut more = false;
    // One borrowed, bounded lookahead proves either continuation or exhaustion.
    for row in reader
        .index_rows(lower, upper)?
        .take(usize::from(limit.get()) + 1)
    {
        let (index, value) = row?;
        let (indexed_chunk, uuid) = raw::decode_index(index.as_ref(), value.as_ref())?;
        if indexed_chunk != chunk {
            return Err(raw::corrupt(raw::INDEX, "index escaped chunk prefix"));
        }
        let key = PersistentEntityKey::new(chunk.world(), chunk.dimension(), uuid);
        let bytes = reader
            .get(RawTable::Entity, &raw::key_bytes(key))?
            .ok_or_else(|| raw::corrupt(raw::INDEX, "chunk index points to missing record"))?;
        let checked = verify(reader, key, bytes)?;
        if checked.chunk != chunk {
            return Err(raw::corrupt(
                raw::INDEX,
                "chunk index points to another envelope chunk",
            ));
        }
        let size = raw::KEY_LEN + checked.bytes.as_ref().len();
        if entries.len() == usize::from(limit.get())
            || total + size > MAX_PERSISTENT_ENTITY_PAGE_BYTES
        {
            more = true;
            break;
        }
        total += size;
        entries.push(PersistentEntityEntry {
            key,
            record: raw::envelope(checked.bytes.as_ref())?.owned()?,
        });
    }
    let next = if more {
        let last = entries
            .last()
            .ok_or_else(|| raw::corrupt(raw::ENTITY, "one entity exceeds page byte bound"))?
            .key
            .uuid();
        Some(PersistentEntityCursor {
            owner: Arc::downgrade(owner),
            chunk,
            expected,
            epoch,
            last,
        })
    } else {
        None
    };
    Ok(PersistentEntityPage {
        entries,
        epoch,
        next,
    })
}

#[derive(Debug, Default)]
pub(crate) struct MemoryState {
    pub(crate) entities: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(crate) locators: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(crate) indexes: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(crate) epochs: BTreeMap<Vec<u8>, Vec<u8>>,
}
impl MemoryState {
    pub(crate) fn is_empty(&self) -> bool {
        self.entities.is_empty()
            && self.locators.is_empty()
            && self.indexes.is_empty()
            && self.epochs.is_empty()
    }
    /// No fallible admission or returned-error cutpoint exists once applying starts.
    pub(crate) fn apply(&mut self, plan: Plan) {
        for change in plan.changes {
            if let Some(old) = change.old_chunk {
                self.indexes
                    .remove(raw::index_key(change.key, old.pos()).as_slice());
            }
            if let Some((chunk, bytes)) = change.new {
                self.entities
                    .insert(raw::key_bytes(change.key).to_vec(), bytes);
                self.locators.insert(
                    raw::locator_key(change.key).to_vec(),
                    raw::locator_value(change.key, chunk.pos()).to_vec(),
                );
                self.indexes
                    .insert(raw::index_key(change.key, chunk.pos()).to_vec(), Vec::new());
            } else {
                self.entities.remove(raw::key_bytes(change.key).as_slice());
                self.locators
                    .remove(raw::locator_key(change.key).as_slice());
            }
        }
        for (chunk, epoch) in plan.epochs {
            self.epochs.insert(
                crate::codec::chunk_key_bytes(chunk).to_vec(),
                epoch.to_be_bytes().to_vec(),
            );
        }
    }
}

pub(crate) struct MemoryRead<'a> {
    pub(crate) state: &'a MemoryState,
    pub(crate) generations: &'a HashMap<GenerationKey, GenerationIdentity>,
}
impl EntityRead for MemoryRead<'_> {
    type Bytes<'a>
        = &'a [u8]
    where
        Self: 'a;
    fn get(&self, table: RawTable, key: &[u8]) -> Outcome<Option<Self::Bytes<'_>>> {
        let table = match table {
            RawTable::Entity => &self.state.entities,
            RawTable::Locator => &self.state.locators,
            RawTable::Index => &self.state.indexes,
            RawTable::Epoch => &self.state.epochs,
        };
        Ok(table.get(key).map(Vec::as_slice))
    }
    fn identity(&self, key: GenerationKey) -> Outcome<Option<GenerationIdentity>> {
        Ok(self.generations.get(&key).copied())
    }
    fn index_rows(
        &self,
        lower: Bound<Vec<u8>>,
        upper: Bound<Vec<u8>>,
    ) -> Outcome<RawRows<'_, Self::Bytes<'_>>> {
        Ok(Box::new(self.state.indexes.range((lower, upper)).map(
            |(key, value)| Ok((key.as_slice(), value.as_slice())),
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use ferrumc_core::{DimensionId, EntityUuid, WorldId};
    use ferrumc_math::ChunkPos;
    use ferrumc_world::WorldSeed;

    use super::*;
    use crate::{
        EntityRecord, PersistentEntityCursorRejection as Rejection, PersistentEntityRecord,
        SchemaVersion,
    };

    fn identity(seed: i64) -> GenerationIdentity {
        GenerationIdentity::flat_overworld(WorldSeed::new(seed))
    }
    fn key(uuid: u128) -> PersistentEntityKey {
        PersistentEntityKey::new(
            WorldId::new(0),
            DimensionId::new(0),
            EntityUuid::from_bytes(uuid.to_be_bytes()),
        )
    }
    fn proposal(uuid: u128, len: usize) -> PersistentEntityWrite {
        PersistentEntityWrite::new(
            key(uuid),
            identity(0),
            PersistentEntityRecord::new(
                ChunkPos::new(0, 0),
                EntityRecord::new(SchemaVersion::new(0), vec![0; len]).unwrap(),
            )
            .unwrap(),
        )
    }
    struct CountRead<'a> {
        reader: MemoryRead<'a>,
        rows: Cell<usize>,
        gets: Cell<usize>,
    }
    impl EntityRead for CountRead<'_> {
        type Bytes<'a>
            = &'a [u8]
        where
            Self: 'a;
        fn get(&self, table: RawTable, key: &[u8]) -> Outcome<Option<Self::Bytes<'_>>> {
            self.gets.set(self.gets.get() + 1);
            self.reader.get(table, key)
        }
        fn identity(&self, key: GenerationKey) -> Outcome<Option<GenerationIdentity>> {
            self.reader.identity(key)
        }
        fn index_rows(
            &self,
            lower: Bound<Vec<u8>>,
            upper: Bound<Vec<u8>>,
        ) -> Outcome<RawRows<'_, Self::Bytes<'_>>> {
            Ok(Box::new(
                self.reader
                    .index_rows(lower, upper)?
                    .inspect(|_| self.rows.set(self.rows.get() + 1)),
            ))
        }
    }
    fn generations() -> HashMap<GenerationKey, GenerationIdentity> {
        [(scope(key(0)), identity(0))].into_iter().collect()
    }
    #[test]
    fn pagination_work_is_count_bounded_and_byte_lookahead_is_borrowed() {
        let generations = generations();
        let mut state = MemoryState::default();
        for start in (0..1024).step_by(64) {
            let plan = save_plan(
                &MemoryRead {
                    state: &state,
                    generations: &generations,
                },
                prepare((start..start + 64).map(|id| proposal(id, 0)).collect()).unwrap(),
            )
            .unwrap();
            state.apply(plan);
        }
        let reader = CountRead {
            reader: MemoryRead {
                state: &state,
                generations: &generations,
            },
            rows: Cell::new(0),
            gets: Cell::new(0),
        };
        let result = page(
            &reader,
            &Arc::new(()),
            chunk_key(key(0), ChunkPos::new(0, 0)),
            identity(0),
            None,
            NonZeroU16::new(64).unwrap(),
        )
        .unwrap();
        assert_eq!(result.entries().len(), 64);
        assert!(result.next().is_some());
        assert_eq!(reader.rows.get(), 65);
        assert_eq!(reader.gets.get(), 1 + 4 * 65);
        let proposals = (0..8)
            .map(|id| proposal(id, crate::MAX_ENTITY_DATA_LEN))
            .collect();
        state.apply(
            save_plan(
                &MemoryRead {
                    state: &state,
                    generations: &generations,
                },
                prepare(proposals).unwrap(),
            )
            .unwrap(),
        );
        let reader = CountRead {
            reader: MemoryRead {
                state: &state,
                generations: &generations,
            },
            rows: Cell::new(0),
            gets: Cell::new(0),
        };
        let result = page(
            &reader,
            &Arc::new(()),
            chunk_key(key(0), ChunkPos::new(0, 0)),
            identity(0),
            None,
            NonZeroU16::new(64).unwrap(),
        )
        .unwrap();
        assert_eq!(result.entries().len(), 3);
        assert_eq!(reader.rows.get(), 4);
        assert_eq!(reader.gets.get(), 17);
    }
    #[test]
    fn cursor_failures_have_exact_reasons_and_publish_no_entries() {
        let generations = generations();
        let mut state = MemoryState::default();
        state.apply(
            save_plan(
                &MemoryRead {
                    state: &state,
                    generations: &generations,
                },
                prepare(vec![proposal(0, 0), proposal(1, 0)]).unwrap(),
            )
            .unwrap(),
        );
        let owner = Arc::new(());
        let chunk = chunk_key(key(0), ChunkPos::new(0, 0));
        let cursor = page(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            &owner,
            chunk,
            identity(0),
            None,
            NonZeroU16::new(1).unwrap(),
        )
        .unwrap()
        .next()
        .unwrap()
        .clone();
        for reason in [
            Rejection::Backend,
            Rejection::Chunk,
            Rejection::GenerationIdentity,
            Rejection::Epoch,
        ] {
            let reader = CountRead {
                reader: MemoryRead {
                    state: &state,
                    generations: &generations,
                },
                rows: Cell::new(0),
                gets: Cell::new(0),
            };
            let foreign = Arc::new(());
            let mut candidate = cursor.clone();
            if reason == Rejection::Epoch {
                candidate.epoch = PersistentEntityEpoch(0);
            }
            let target = if reason == Rejection::Chunk {
                ChunkKey::new(chunk.world(), chunk.dimension(), ChunkPos::new(1, 0))
            } else {
                chunk
            };
            let expected = if reason == Rejection::GenerationIdentity {
                identity(1)
            } else {
                identity(0)
            };
            let authority = if reason == Rejection::Backend {
                &foreign
            } else {
                &owner
            };
            assert!(
                matches!(page(&reader, authority, target, expected, Some(&candidate), NonZeroU16::new(1).unwrap()), Err(StorageError::InvalidPersistentEntityCursor { reason: actual }) if actual == reason)
            );
            assert_eq!(reader.rows.get(), 0);
            assert_eq!(reader.gets.get(), usize::from(reason == Rejection::Epoch));
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use crate::{EntityRecord, PersistentEntityRecord, SchemaVersion};
    use ferrumc_core::{DimensionId, EntityUuid, WorldId};
    use ferrumc_math::ChunkPos;
    use ferrumc_world::WorldSeed;

    fn proposal(
        world: u32,
        dimension: u32,
        id: u128,
        chunk: i32,
        len: usize,
    ) -> PersistentEntityWrite {
        PersistentEntityWrite::new(
            PersistentEntityKey::new(
                WorldId::new(world),
                DimensionId::new(dimension),
                EntityUuid::from_bytes(id.to_be_bytes()),
            ),
            identity(),
            PersistentEntityRecord::new(
                ChunkPos::new(chunk, 0),
                EntityRecord::new(SchemaVersion::new(0), vec![0; len]).unwrap(),
            )
            .unwrap(),
        )
    }
    fn identity() -> GenerationIdentity {
        GenerationIdentity::flat_overworld(WorldSeed::new(0))
    }
    #[test]
    fn sixty_four_chunk_changes_increment_exactly128_distinct_epochs_once() {
        let generations = [(scope(proposal(0, 0, 0, 0, 0).key()), identity())]
            .into_iter()
            .collect();
        let mut state = MemoryState::default();
        state.apply(
            save_plan(
                &MemoryRead {
                    state: &state,
                    generations: &generations,
                },
                prepare((0..64).map(|id| proposal(0, 0, id, id as i32, 0)).collect()).unwrap(),
            )
            .unwrap(),
        );
        let plan = save_plan(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            prepare(
                (0..64)
                    .map(|id| proposal(0, 0, id, 64 + id as i32, 0))
                    .collect(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(plan.changes.len(), 64);
        assert_eq!(plan.epochs.len(), 128);
        for (chunk, value) in &plan.epochs {
            assert_eq!(*value, if chunk.pos().x() < 64 { 2 } else { 1 });
        }
        state.apply(plan);
    }
    #[test]
    fn duplicate_world_uuid_proposals_refuse_across_dimensions_but_other_worlds_are_independent() {
        assert!(matches!(
            prepare(vec![proposal(0, 0, 1, 0, 0), proposal(0, 1, 1, 0, 0)]),
            Err(StorageError::DuplicatePersistentEntityWrite { .. })
        ));
        assert_eq!(
            prepare(vec![proposal(0, 0, 1, 0, 0), proposal(1, 0, 1, 0, 0)])
                .unwrap()
                .len(),
            2
        );
    }
    #[test]
    fn page_exact_encoded_byte_boundary_and_one_more_entry_yield_valid_continuation() {
        let generations = [(scope(proposal(0, 0, 0, 0, 0).key()), identity())]
            .into_iter()
            .collect();
        let mut state = MemoryState::default();
        let mut writes: Vec<_> = (0..3)
            .map(|id| proposal(0, 0, id, 0, crate::MAX_ENTITY_DATA_LEN))
            .collect();
        writes.push(proposal(
            0,
            0,
            3,
            0,
            MAX_PERSISTENT_ENTITY_PAGE_BYTES - 4 * 46 - 3 * crate::MAX_ENTITY_DATA_LEN,
        ));
        writes.push(proposal(0, 0, 4, 0, 0));
        state.apply(
            save_plan(
                &MemoryRead {
                    state: &state,
                    generations: &generations,
                },
                prepare(writes).unwrap(),
            )
            .unwrap(),
        );
        let owner = Arc::new(());
        let chunk = chunk_key(proposal(0, 0, 0, 0, 0).key(), ChunkPos::new(0, 0));
        let first = page(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            &owner,
            chunk,
            identity(),
            None,
            NonZeroU16::new(64).unwrap(),
        )
        .unwrap();
        assert_eq!(first.entries().len(), 4);
        assert_eq!(
            first
                .entries()
                .iter()
                .map(|entry| 46 + entry.record().record().data().len())
                .sum::<usize>(),
            MAX_PERSISTENT_ENTITY_PAGE_BYTES
        );
        let last = page(
            &MemoryRead {
                state: &state,
                generations: &generations,
            },
            &owner,
            chunk,
            identity(),
            first.next(),
            NonZeroU16::new(64).unwrap(),
        )
        .unwrap();
        assert_eq!(last.entries().len(), 1);
        assert!(last.next().is_none());
    }
}
