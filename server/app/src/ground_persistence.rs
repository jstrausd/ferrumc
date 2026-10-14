//! Explicit item-plus-mob UUID entity sidecar; the existing world database is never migrated.
//!
//! This owner is independent of block/player saves. It provides graceful restart
//! persistence, not a transaction coupling a pickup to a player/world record.
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU16;
use std::path::Path;
use std::sync::Arc;

use ferrumc_core::{DimensionId, EntityUuid, PluginId, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_sim::{
    decode_item_entity, decode_mob_entity, EntityTransform, MobRestoreProposal, RestoredItemEntity,
    NATIVE_MOB_ENTITY_BYTES,
};
use ferrumc_storage::{
    ChunkKey, GenerationKey, InMemoryStore, PersistentEntityKey, PersistentEntityRecord,
    PersistentEntityStore, PersistentEntityWrite, PluginStore, RedbStore, StorageKey, WorldStore,
};
use ferrumc_world::GenerationIdentity;

const MAX_ITEMS: usize = 256;
const MAX_ITEM_BYTES: usize = 4 * 1024 * 1024;
const MAX_MOBS: usize = 64;
const MAX_MOB_BYTES: usize = MAX_MOBS * NATIVE_MOB_ENTITY_BYTES;
const MAX_MANIFEST_CHUNKS: usize = 512;
const MANIFEST_MAGIC: &[u8; 6] = b"FCGI\0\x01";

/// One complete item-plus-mob canonical snapshot for the sole serial writer.
pub(crate) type EntityRecords = Vec<(PersistentEntityKey, PersistentEntityRecord)>;

/// Strict decoded startup proposal, without a runtime wire ID or live authority.
#[derive(Debug)]
pub(crate) enum RestoredEntity {
    Item(RestoredItemEntity),
    Mob(MobRestoreProposal),
}

impl RestoredEntity {
    /// Exact saved canonical transform whose genuine chunk ticket must be held.
    pub(crate) fn transform(&self) -> EntityTransform {
        match self {
            Self::Item(item) => item.transform(),
            Self::Mob(mob) => mob.transform(),
        }
    }
}

#[derive(Default)]
struct RecordBudget {
    items: usize,
    item_bytes: usize,
    mobs: usize,
    mob_bytes: usize,
}

impl RecordBudget {
    fn admit(
        &mut self,
        key: PersistentEntityKey,
        record: &PersistentEntityRecord,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            key.world() == WorldId::new(0) && key.dimension() == DimensionId::new(0),
            "entity runtime foreign scope"
        );
        let bytes = record.record().data();
        match bytes.get(..4) {
            Some(b"FCIE") => {
                self.items += 1;
                self.item_bytes = self
                    .item_bytes
                    .checked_add(bytes.len())
                    .ok_or_else(|| anyhow::anyhow!("item restore size overflow"))?;
                anyhow::ensure!(
                    self.items <= MAX_ITEMS && self.item_bytes <= MAX_ITEM_BYTES,
                    "entity runtime item allowance exceeded"
                );
            }
            Some(b"FCMB") => {
                self.mobs += 1;
                self.mob_bytes = self
                    .mob_bytes
                    .checked_add(bytes.len())
                    .ok_or_else(|| anyhow::anyhow!("mob restore size overflow"))?;
                anyhow::ensure!(
                    bytes.len() == NATIVE_MOB_ENTITY_BYTES
                        && self.mobs <= MAX_MOBS
                        && self.mob_bytes <= MAX_MOB_BYTES,
                    "entity runtime mob allowance or fixed length exceeded"
                );
            }
            _ => anyhow::bail!("unknown entity runtime native record"),
        }
        Ok(())
    }
}

fn decode_record(
    key: PersistentEntityKey,
    record: &PersistentEntityRecord,
) -> anyhow::Result<RestoredEntity> {
    match record.record().data().get(..4) {
        Some(b"FCIE") => Ok(RestoredEntity::Item(decode_item_entity(key, record)?)),
        Some(b"FCMB") => Ok(RestoredEntity::Mob(decode_mob_entity(key, record)?)),
        _ => anyhow::bail!("unknown entity runtime native record"),
    }
}
trait GroundBackend: WorldStore + PersistentEntityStore + PluginStore {}
impl GroundBackend for RedbStore {}
impl GroundBackend for InMemoryStore {}

/// One serial I/O owner, moved into at most one off-tick flush task at a time.
pub(crate) struct GroundPersistence {
    backend: Arc<dyn GroundBackend>,
    identity: GenerationIdentity,
    previous: BTreeMap<EntityUuid, ChunkPos>,
}

impl GroundPersistence {
    /// Opens a separate Format5 sidecar and verifies its exact generator identity.
    /// Malformed or incompatible data refuses startup; it never becomes a miss.
    pub(crate) async fn open(
        world_dir: Option<&Path>,
        identity: GenerationIdentity,
    ) -> anyhow::Result<(Self, Vec<RestoredEntity>)> {
        let path = world_dir.map(|dir| dir.join("entity-runtime.redb"));
        let backend: Arc<dyn GroundBackend> =
            tokio::task::spawn_blocking(move || -> anyhow::Result<Arc<dyn GroundBackend>> {
                match path {
                    Some(path) => Ok(Arc::new(RedbStore::open_with_uuid_entities(path)?)),
                    None => Ok(Arc::new(InMemoryStore::with_uuid_entities())),
                }
            })
            .await??;
        backend
            .ensure_generation_identity(
                GenerationKey::new(WorldId::new(0), DimensionId::new(0)),
                identity,
            )
            .await?;
        let plugin = PluginId::new("ferrumc-ground-item-runtime");
        let key = StorageKey::new("resident-chunk-manifest-v1")?;
        let raw = backend.get(&plugin, &key).await?;
        let chunks = match raw {
            Some(bytes) => decode_manifest(&bytes)?,
            None => BTreeSet::new(),
        };
        let mut records = Vec::new();
        let mut previous = BTreeMap::new();
        let mut budget = RecordBudget::default();
        let page_size =
            NonZeroU16::new(64).ok_or_else(|| anyhow::anyhow!("invalid fixed page size"))?;
        for chunk in chunks {
            let key = ChunkKey::new(WorldId::new(0), DimensionId::new(0), chunk);
            let mut cursor = None;
            loop {
                let page = backend
                    .load_persistent_entity_page(key, identity, cursor, page_size)
                    .await?;
                let (entries, next) = page.into_parts();
                for entry in entries {
                    let key = entry.key();
                    let record = entry.into_record();
                    budget.admit(key, &record)?;
                    anyhow::ensure!(
                        previous.insert(key.uuid(), record.chunk()).is_none(),
                        "duplicate entity UUID in manifest"
                    );
                    records.push((key, record));
                }
                cursor = next;
                if cursor.is_none() {
                    break;
                }
            }
        }
        // Complete encoded admission precedes component decoding. The existing
        // FCIE/FCMB decoders validate every identity/header/body relation exactly.
        let restored = records
            .into_iter()
            .map(|(key, record)| decode_record(key, &record))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            Self {
                backend,
                identity,
                previous,
            },
            restored,
        ))
    }

    /// Writes one complete item-plus-mob owner snapshot. The union manifest lands first so a
    /// crash cannot hide a newly saved chunk; completed deletions then permit the
    /// current manifest to shrink. A failure is retained/fail-stop, never retried
    /// as an inferred transaction receipt or silently acknowledged.
    pub(crate) async fn flush(&mut self, records: EntityRecords) -> anyhow::Result<()> {
        let mut budget = RecordBudget::default();
        let mut current = BTreeMap::new();
        for (key, record) in &records {
            budget.admit(*key, record)?;
            // Every native codec and cross-record UUID relation validates before
            // the first manifest/save/delete write. Invalid sets preserve storage.
            decode_record(*key, record)?;
            anyhow::ensure!(
                current.insert(key.uuid(), record.chunk()).is_none(),
                "entity runtime flush duplicate UUID"
            );
        }
        let union = self
            .previous
            .values()
            .chain(current.values())
            .copied()
            .collect();
        self.write_manifest(&union).await?;
        let mut batch = Vec::new();
        let mut batch_bytes = 0;
        for (key, record) in records {
            let size = record.record().data().len() + 46;
            if batch.len() == 64 || batch_bytes + size > 4 * 1024 * 1024 {
                self.backend
                    .save_persistent_entities(std::mem::take(&mut batch))
                    .await?;
                batch_bytes = 0;
            }
            batch_bytes += size;
            batch.push(PersistentEntityWrite::new(key, self.identity, record));
        }
        if !batch.is_empty() {
            self.backend.save_persistent_entities(batch).await?;
        }
        for uuid in self
            .previous
            .keys()
            .filter(|uuid| !current.contains_key(uuid))
        {
            self.backend
                .delete_persistent_entity(
                    PersistentEntityKey::new(WorldId::new(0), DimensionId::new(0), *uuid),
                    self.identity,
                )
                .await?;
        }
        self.write_manifest(&current.values().copied().collect())
            .await?;
        self.previous = current;
        Ok(())
    }

    async fn write_manifest(&self, chunks: &BTreeSet<ChunkPos>) -> anyhow::Result<()> {
        let bytes = encode_manifest(chunks)?;
        self.backend
            .put(
                &PluginId::new("ferrumc-ground-item-runtime"),
                StorageKey::new("resident-chunk-manifest-v1")?,
                bytes,
            )
            .await?;
        Ok(())
    }
}

fn encode_manifest(chunks: &BTreeSet<ChunkPos>) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        chunks.len() <= MAX_MANIFEST_CHUNKS,
        "ground manifest chunk allowance exceeded"
    );
    let mut bytes = Vec::with_capacity(8 + chunks.len() * 8);
    bytes.extend_from_slice(MANIFEST_MAGIC);
    bytes.extend_from_slice(&(chunks.len() as u16).to_be_bytes());
    for chunk in chunks {
        bytes.extend_from_slice(&chunk.x().to_be_bytes());
        bytes.extend_from_slice(&chunk.z().to_be_bytes());
    }
    Ok(bytes)
}

fn decode_manifest(bytes: &[u8]) -> anyhow::Result<BTreeSet<ChunkPos>> {
    anyhow::ensure!(
        bytes.len() >= 8 && &bytes[..6] == MANIFEST_MAGIC,
        "invalid ground item chunk manifest"
    );
    let count = usize::from(u16::from_be_bytes([bytes[6], bytes[7]]));
    anyhow::ensure!(
        count <= MAX_MANIFEST_CHUNKS && bytes.len() == 8 + count * 8,
        "invalid ground manifest length or count"
    );
    let mut chunks = BTreeSet::new();
    for cell in bytes[8..].as_chunks::<8>().0 {
        let x = i32::from_be_bytes(cell[..4].try_into()?);
        let z = i32::from_be_bytes(cell[4..].try_into()?);
        anyhow::ensure!(
            chunks.insert(ChunkPos::new(x, z)),
            "duplicate ground manifest chunk"
        );
    }
    Ok(chunks)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
    use ferrumc_math::Vec3;
    use ferrumc_sim::MobKind;
    use ferrumc_storage::{EntityRecord, SchemaVersion};
    use ferrumc_world::WorldSeed;
    use std::num::NonZeroU8;

    /// Deterministic independent native-format fixture, checked by real codecs.
    pub(crate) fn fixture(
        mob: bool,
        id: u128,
        position: Vec3,
    ) -> (PersistentEntityKey, PersistentEntityRecord) {
        let key = PersistentEntityKey::new(
            WorldId::new(0),
            DimensionId::new(0),
            EntityUuid::from_bytes(id.to_be_bytes()),
        );
        let kind = if mob { "cow" } else { "item" };
        let mut body = if mob {
            b"FCMB".to_vec()
        } else {
            b"FCIE".to_vec()
        };
        body.extend_from_slice(&1_u16.to_be_bytes());
        body.extend_from_slice(
            &ferrumc_registry::entity_type::EntityTypeId::from_name(kind)
                .expect("type")
                .protocol_id()
                .to_be_bytes(),
        );
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&key.uuid().to_bytes());
        for value in [position.x, position.y, position.z] {
            body.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        body.extend_from_slice(&0_f32.to_bits().to_be_bytes());
        body.extend_from_slice(&0_f32.to_bits().to_be_bytes());
        if mob {
            body.push(1); // Supported adult cow, no enchantment/plugin state.
            body.extend_from_slice(&7.0_f32.to_bits().to_be_bytes());
            body.extend_from_slice(&42_u64.to_be_bytes());
            body.extend_from_slice(&84_u64.to_be_bytes());
            body.extend_from_slice(&[0; 4]);
            body.extend_from_slice(&0_f32.to_bits().to_be_bytes());
        } else {
            body.extend_from_slice(&[0; 24 + 4]);
            for value in [0_i32, 40, 5] {
                body.extend_from_slice(&value.to_be_bytes());
            }
            body.extend_from_slice(&[0; 34]);
            let stack = ItemStack::new(
                ItemId::from_name("stone").expect("item"),
                NonZeroU8::new(16).expect("count"),
                ComponentPatch::empty(),
            );
            let encoded = ferrumc_items::encode_persisted_stack(&stack).expect("FCIS");
            body.extend_from_slice(&u32::try_from(encoded.len()).expect("bounded").to_be_bytes());
            body.extend_from_slice(&encoded);
        }
        let record = PersistentEntityRecord::new(
            EntityTransform::try_new(position, 0.0, 0.0)
                .expect("position")
                .chunk(),
            EntityRecord::new(SchemaVersion::new(1), body).expect("body"),
        )
        .expect("record");
        decode_record(key, &record).expect("independent valid fixture");
        (key, record)
    }

    /// Explicit unchanged generation binding shared by sidecar and test fixtures.
    pub(crate) fn identity() -> GenerationIdentity {
        GenerationIdentity::flat_overworld(WorldSeed::new(0))
    }

    fn rewrite(record: &PersistentEntityRecord, bytes: Vec<u8>) -> PersistentEntityRecord {
        PersistentEntityRecord::new(
            record.chunk(),
            EntityRecord::new(record.record().schema_version(), bytes).expect("record body"),
        )
        .expect("chunk")
    }

    #[tokio::test]
    async fn mixed_disk_restart_deletes_absent_item_without_deleting_present_mob() {
        let directory = tempfile::tempdir().expect("directory");
        let item = fixture(false, 1, Vec3::new(-16.5, 64.0, 1.5));
        let mob = fixture(true, 2, Vec3::new(32.5, 64.0, 8.5));
        let (mut writer, initial) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("open");
        assert!(initial.is_empty());
        writer
            .flush(vec![item.clone(), mob.clone()])
            .await
            .expect("mixed save");
        drop(writer);
        let (mut writer, restored) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("restart");
        assert_eq!(restored.len(), 2);
        assert!(restored.iter().any(|entity| matches!(entity,RestoredEntity::Item(proposal) if proposal.key()==item.0 && proposal.item_state().stack().count()==16)));
        assert!(restored.iter().any(|entity| matches!(entity,RestoredEntity::Mob(proposal) if proposal.key()==mob.0 && proposal.kind()==MobKind::Cow && proposal.health().to_bits()==7.0_f32.to_bits())));
        writer
            .flush(vec![mob.clone()])
            .await
            .expect("delete only item");
        assert!(writer
            .backend
            .load_persistent_entity(item.0, identity())
            .await
            .expect("item read")
            .is_none());
        assert_eq!(
            writer
                .backend
                .load_persistent_entity(mob.0, identity())
                .await
                .expect("mob read"),
            Some(mob.1)
        );
        drop(writer);
        let (mut writer, restored) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("restart deletion");
        assert_eq!(restored.len(), 1);
        assert!(matches!(restored[0], RestoredEntity::Mob(_)));
        writer.flush(vec![]).await.expect("delete all");
        drop(writer);
        let (_, restored) = GroundPersistence::open(Some(directory.path()), identity())
            .await
            .expect("empty restart");
        assert!(restored.is_empty());
    }

    #[tokio::test]
    async fn mixed_uuid_migration_and_item_only_replacement_preserve_single_writer_set() {
        let (mut writer, _) = GroundPersistence::open(None, identity())
            .await
            .expect("open");
        let old_mob = fixture(true, 20, Vec3::new(-16.5, 64.0, 1.0));
        let item = fixture(false, 21, Vec3::new(0.5, 64.0, 1.0));
        writer
            .flush(vec![old_mob.clone(), item.clone()])
            .await
            .expect("save");
        let moved_mob = fixture(true, 20, Vec3::new(64.5, 64.0, 1.0));
        writer
            .flush(vec![moved_mob.clone(), item.clone()])
            .await
            .expect("move");
        assert_eq!(
            writer.previous.get(&old_mob.0.uuid()),
            Some(&moved_mob.1.chunk())
        );
        assert_eq!(
            writer
                .backend
                .load_persistent_entity(moved_mob.0, identity())
                .await
                .expect("mob"),
            Some(moved_mob.1)
        );
        writer
            .flush(vec![item.clone()])
            .await
            .expect("mob deletion");
        assert!(writer
            .backend
            .load_persistent_entity(old_mob.0, identity())
            .await
            .expect("removed")
            .is_none());
        assert_eq!(writer.previous.len(), 1);
        assert_eq!(
            writer
                .backend
                .load_persistent_entity(item.0, identity())
                .await
                .expect("item"),
            Some(item.1)
        );
    }

    #[tokio::test]
    async fn invalid_late_record_duplicate_or_foreign_scope_cannot_write_manifest_or_delete() {
        let (mut writer, _) = GroundPersistence::open(None, identity())
            .await
            .expect("open");
        let item = fixture(false, 30, Vec3::new(0.5, 64.0, 1.0));
        let mob = fixture(true, 31, Vec3::new(32.5, 64.0, 1.0));
        writer
            .flush(vec![item.clone(), mob.clone()])
            .await
            .expect("save");
        let previous = writer.previous.clone();
        let before = writer
            .backend
            .get(
                &PluginId::new("ferrumc-ground-item-runtime"),
                &StorageKey::new("resident-chunk-manifest-v1").expect("key"),
            )
            .await
            .expect("manifest");
        let duplicate = fixture(true, 30, Vec3::new(64.5, 64.0, 1.0));
        let mut bad_body = mob.1.record().data().to_vec();
        bad_body[67..71].copy_from_slice(&0_f32.to_bits().to_be_bytes());
        let malformed = (mob.0, rewrite(&mob.1, bad_body));
        let foreign = (
            PersistentEntityKey::new(WorldId::new(1), DimensionId::new(0), mob.0.uuid()),
            mob.1.clone(),
        );
        let unknown = (mob.0, rewrite(&mob.1, b"XXXX".to_vec()));
        for bad in [duplicate, malformed, foreign, unknown] {
            assert!(writer.flush(vec![item.clone(), bad]).await.is_err());
            assert_eq!(writer.previous, previous);
            assert_eq!(
                writer
                    .backend
                    .get(
                        &PluginId::new("ferrumc-ground-item-runtime"),
                        &StorageKey::new("resident-chunk-manifest-v1").expect("key")
                    )
                    .await
                    .expect("manifest"),
                before
            );
            assert_eq!(
                writer
                    .backend
                    .load_persistent_entity(item.0, identity())
                    .await
                    .expect("item"),
                Some(item.1.clone())
            );
            assert_eq!(
                writer
                    .backend
                    .load_persistent_entity(mob.0, identity())
                    .await
                    .expect("mob"),
                Some(mob.1.clone())
            );
        }
    }

    #[test]
    fn complete_mixed_counts_and_separate_byte_budgets_are_bounded() {
        let item = fixture(false, 40, Vec3::ZERO);
        let mob = fixture(true, 41, Vec3::ZERO);
        let mut budget = RecordBudget::default();
        for _ in 0..MAX_ITEMS {
            budget.admit(item.0, &item.1).expect("item cap");
        }
        for _ in 0..MAX_MOBS {
            budget.admit(mob.0, &mob.1).expect("mob cap");
        }
        assert_eq!(budget.items + budget.mobs, 320);
        assert_eq!(budget.mob_bytes, 6080);
        assert!(budget.admit(item.0, &item.1).is_err());
        let mut budget = RecordBudget {
            item_bytes: MAX_ITEM_BYTES - item.1.record().data().len(),
            ..RecordBudget::default()
        };
        budget.admit(item.0, &item.1).expect("exact bytes");
        assert!(budget.admit(item.0, &item.1).is_err());
        let mut budget = RecordBudget::default();
        for _ in 0..MAX_MOBS {
            budget.admit(mob.0, &mob.1).expect("mob cap");
        }
        assert!(budget.admit(mob.0, &mob.1).is_err());
    }

    #[tokio::test]
    async fn malformed_known_unknown_or_mixed_oversized_sidecar_refuses_startup() {
        for mode in 0..4 {
            let directory = tempfile::tempdir().expect("directory");
            let (writer, _) = GroundPersistence::open(Some(directory.path()), identity())
                .await
                .expect("open");
            let mob = fixture(true, 50, Vec3::ZERO);
            let body = match mode {
                0 => b"XXXX".to_vec(),
                1 => mob.1.record().data()[..94].to_vec(),
                2 => {
                    let mut b = mob.1.record().data().to_vec();
                    b.push(0);
                    b
                }
                _ => mob.1.record().data().to_vec(),
            };
            let record = rewrite(&mob.1, body);
            if mode == 3 {
                for id in 1..=65 {
                    let entry = fixture(true, id, Vec3::ZERO);
                    writer
                        .backend
                        .save_persistent_entity(PersistentEntityWrite::new(
                            entry.0,
                            identity(),
                            entry.1,
                        ))
                        .await
                        .expect("raw row");
                }
            } else {
                writer
                    .backend
                    .save_persistent_entity(PersistentEntityWrite::new(mob.0, identity(), record))
                    .await
                    .expect("raw row");
            }
            writer
                .write_manifest(&[ChunkPos::ORIGIN].into_iter().collect())
                .await
                .expect("manifest");
            drop(writer);
            assert!(GroundPersistence::open(Some(directory.path()), identity())
                .await
                .is_err());
        }
    }

    #[test]
    fn manifest_requires_complete_unique_bounded_membership() {
        let chunks = [ChunkPos::new(-1, 0), ChunkPos::new(8, -8)]
            .into_iter()
            .collect();
        let bytes = encode_manifest(&chunks).unwrap();
        assert_eq!(decode_manifest(&bytes).unwrap(), chunks);
        for n in 0..bytes.len() {
            assert!(decode_manifest(&bytes[..n]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode_manifest(&trailing).is_err());
        let mut repeated = bytes.clone();
        repeated[16..24].copy_from_slice(&bytes[8..16]);
        assert!(decode_manifest(&repeated).is_err());
        let mut oversized = MANIFEST_MAGIC.to_vec();
        oversized.extend_from_slice(&513u16.to_be_bytes());
        assert!(decode_manifest(&oversized).is_err());
        assert!(encode_manifest(&(0..513).map(|x| ChunkPos::new(x, 0)).collect()).is_err());
    }
}
