//! Public owner/handle/index and real shard-boundary entity contracts.

use std::num::NonZeroUsize;

use ferrumc_core::{DimensionId, EntityId, EntityUuid, PlayerId, WorldId};
use ferrumc_math::{ChunkPos, ShardPos, Vec3};
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_sim::{
    ChunkTicket, EntityData, EntitySpawn, EntityStore, EntityStoreError, EntityTransform,
    GameInput, GameOutput, LoadedChunkMap, ShardId, SimShard, TicketReason, MAX_ENTITIES_PER_SHARD,
    MAX_ENTITY_HORIZONTAL_POSITION, MAX_ENTITY_VERTICAL_POSITION,
};
use ferrumc_storage::InMemoryStore;
use ferrumc_world::FlatWorldGenerator;

fn owner(x: i32, z: i32) -> ShardId {
    ShardId::try_new(WorldId::new(7), DimensionId::new(13), ShardPos::new(x, z)).unwrap()
}

fn uuid(id: u16) -> EntityUuid {
    EntityUuid::from_bytes(u128::from(id).to_be_bytes())
}

fn transform(x: f64, y: f64, z: f64) -> EntityTransform {
    EntityTransform::try_new(Vec3::new(x, y, z), 0.0, 0.0).unwrap()
}

fn request(owner: ShardId, id: u16, kind: &str, position: Vec3) -> EntitySpawn {
    EntitySpawn::new(
        owner.world(),
        owner.dimension(),
        uuid(id),
        EntityId::new(i32::from(id)),
        EntityTypeId::from_name(kind).unwrap(),
        EntityTransform::try_new(position, 0.0, 0.0).unwrap(),
    )
}

fn store(owner: ShardId, capacity: usize) -> EntityStore {
    EntityStore::new(owner, NonZeroUsize::new(capacity).unwrap()).unwrap()
}

fn snapshot(store: &EntityStore) -> Vec<EntityData> {
    let min = store.owner().region().min_chunk();
    let mut entities = Vec::new();
    for z in min.z()..min.z() + 8 {
        for x in min.x()..min.x() + 8 {
            let chunk = ChunkPos::new(x, z);
            let occupants = store.in_chunk(chunk).unwrap();
            let count = occupants.len();
            let mut visited = 0;
            for entity in occupants {
                assert_eq!(entity.owner(), store.owner());
                assert_eq!(entity.chunk(), chunk);
                let by_uuid = store.handle_for_uuid(entity.uuid()).unwrap();
                let by_wire = store.handle_for_wire_id(entity.wire_id()).unwrap();
                assert_eq!(by_uuid, by_wire);
                assert_eq!(store.get(&by_uuid).unwrap(), entity);
                entities.push(*entity);
                visited += 1;
            }
            assert_eq!(visited, count);
        }
    }
    assert_eq!(entities.len(), store.len());
    entities
}

#[test]
fn different_registered_types_keep_uuid_wire_type_and_owner_distinct() {
    let owner = owner(0, 0);
    let mut entities = store(owner, 4);
    for (id, kind, position) in [
        (1, "item", Vec3::new(0.5, 64.0, 0.5)),
        (2, "zombie", Vec3::new(16.0, 65.0, 1.0)),
        (3, "cow", Vec3::new(127.99, 66.0, 127.99)),
        (4, "arrow", Vec3::new(4.0, 67.0, 17.0)),
    ] {
        let handle = entities.spawn(request(owner, id, kind, position)).unwrap();
        let entity = entities.get(&handle).unwrap();
        assert_eq!(entity.uuid(), uuid(id));
        assert_eq!(entity.wire_id(), EntityId::new(i32::from(id)));
        assert_eq!(
            entity.entity_type().resource_location(),
            format!("minecraft:{kind}")
        );
        assert_eq!(entity.transform().position(), position);
    }
    assert_eq!(snapshot(&entities).len(), 4);
    assert!(entities.is_full());
}

#[test]
fn rejected_duplicates_scope_owner_type_and_capacity_preserve_all_indexes() {
    let owner = owner(0, 0);
    let mut entities = store(owner, 2);
    let first = request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0));
    let handle = entities.spawn(first).unwrap();
    let before = snapshot(&entities);
    assert_eq!(
        entities.spawn(first),
        Err(EntityStoreError::DuplicateUuid { uuid: uuid(1) })
    );
    let wire_duplicate = EntitySpawn::new(
        owner.world(),
        owner.dimension(),
        uuid(2),
        EntityId::new(1),
        EntityTypeId::from_name("cow").unwrap(),
        transform(2.0, 64.0, 2.0),
    );
    assert_eq!(
        entities.spawn(wire_duplicate),
        Err(EntityStoreError::DuplicateWireId {
            wire_id: EntityId::new(1)
        })
    );
    for other in [
        ShardId::try_new(WorldId::new(8), owner.dimension(), owner.position()).unwrap(),
        ShardId::try_new(owner.world(), DimensionId::new(14), owner.position()).unwrap(),
    ] {
        assert!(matches!(
            entities.spawn(request(other, 2, "cow", Vec3::new(1.0, 64.0, 1.0))),
            Err(EntityStoreError::ScopeMismatch { .. })
        ));
    }
    assert!(matches!(
        entities.spawn(request(owner, 2, "cow", Vec3::new(128.0, 64.0, 1.0))),
        Err(EntityStoreError::OutsideOwner { .. })
    ));
    assert_eq!(
        entities.spawn(request(owner, 2, "player", Vec3::new(1.0, 64.0, 1.0))),
        Err(EntityStoreError::PlayerType)
    );
    assert_eq!(snapshot(&entities), before);
    assert_eq!(entities.handle_for_uuid(uuid(1)), Some(handle.clone()));
    entities
        .spawn(request(owner, 2, "cow", Vec3::new(2.0, 64.0, 2.0)))
        .unwrap();
    let full = snapshot(&entities);
    assert_eq!(
        entities.spawn(request(owner, 3, "zombie", Vec3::new(3.0, 64.0, 3.0))),
        Err(EntityStoreError::Full { capacity: 2 })
    );
    assert_eq!(snapshot(&entities), full);
    assert!(entities.handle_for_uuid(uuid(3)).is_none());
    assert!(entities.handle_for_wire_id(EntityId::new(3)).is_none());
}

#[test]
fn capacity_is_explicit_bounded_and_lazy() {
    let owner = owner(0, 0);
    assert!(
        EntityStore::new(owner, NonZeroUsize::new(MAX_ENTITIES_PER_SHARD).unwrap())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        EntityStore::new(
            owner,
            NonZeroUsize::new(MAX_ENTITIES_PER_SHARD + 1).unwrap()
        ),
        Err(EntityStoreError::InvalidCapacity { .. })
    ));
}

#[test]
fn finite_position_ceiling_and_original_rotation_path_are_explicit() {
    for position in [
        Vec3::new(f64::NAN, 0.0, 0.0),
        Vec3::new(0.0, f64::INFINITY, 0.0),
        Vec3::new(0.0, 0.0, f64::NEG_INFINITY),
        Vec3::new(MAX_ENTITY_HORIZONTAL_POSITION + 1.0, 0.0, 0.0),
        Vec3::new(-MAX_ENTITY_HORIZONTAL_POSITION - 1.0, 0.0, 0.0),
        Vec3::new(0.0, MAX_ENTITY_VERTICAL_POSITION + 1.0, 0.0),
        Vec3::new(0.0, 0.0, -MAX_ENTITY_HORIZONTAL_POSITION - 1.0),
    ] {
        assert!(matches!(
            EntityTransform::try_new(position, 0.0, 0.0),
            Err(EntityStoreError::InvalidTransform { .. })
        ));
    }
    for (yaw, pitch) in [
        (f32::NAN, 0.0),
        (f32::INFINITY, 0.0),
        (0.0, f32::NEG_INFINITY),
        (0.0, f32::NAN),
    ] {
        assert!(EntityTransform::try_new(Vec3::ZERO, yaw, pitch).is_err());
    }
    for sign in [1.0, -1.0] {
        assert!(EntityTransform::try_new(
            Vec3::new(
                sign * MAX_ENTITY_HORIZONTAL_POSITION,
                sign * MAX_ENTITY_VERTICAL_POSITION,
                sign * MAX_ENTITY_HORIZONTAL_POSITION
            ),
            0.0,
            0.0
        )
        .is_ok());
    }
    let normalized = EntityTransform::try_new(Vec3::ZERO, -725.0, 451.0).unwrap();
    assert!((normalized.yaw() + 5.0).abs() < f32::EPSILON);
    assert!((normalized.pitch() - 90.0).abs() < f32::EPSILON);
    let normalized = EntityTransform::try_new(Vec3::ZERO, 725.0, -451.0).unwrap();
    assert!((normalized.yaw() - 5.0).abs() < f32::EPSILON);
    assert!((normalized.pitch() + 90.0).abs() < f32::EPSILON);
}

#[test]
fn negative_chunk_and_shard_boundaries_use_floor_and_refuse_atomic_crossing() {
    let owner = owner(-1, -1);
    let mut entities = store(owner, 2);
    let first = entities
        .spawn(request(owner, 1, "item", Vec3::new(-128.0, 64.0, -128.0)))
        .unwrap();
    let second = entities
        .spawn(request(owner, 2, "cow", Vec3::new(-0.001, 64.0, -0.001)))
        .unwrap();
    assert_eq!(entities.get(&first).unwrap().chunk(), ChunkPos::new(-8, -8));
    assert_eq!(
        entities.get(&second).unwrap().chunk(),
        ChunkPos::new(-1, -1)
    );
    for target in [
        transform(-128.001, 64.0, -1.0),
        transform(0.0, 64.0, -1.0),
        transform(-1.0, 64.0, 0.0),
    ] {
        let before = snapshot(&entities);
        assert!(matches!(
            entities.update_transform(&second, target),
            Err(EntityStoreError::OutsideOwner { .. })
        ));
        assert_eq!(snapshot(&entities), before);
    }
    entities
        .update_transform(&second, transform(-16.001, 64.0, -16.0))
        .unwrap();
    assert_eq!(
        entities.get(&second).unwrap().chunk(),
        ChunkPos::new(-2, -1)
    );
    assert_eq!(snapshot(&entities).len(), 2);
    assert!(entities.in_chunk(ChunkPos::new(0, -1)).is_err());
}

#[test]
fn move_remove_reuse_and_borrowed_budgeting_preserve_chunk_and_identity_indexes() {
    let owner = owner(0, 0);
    let mut entities = store(owner, 3);
    let a = entities
        .spawn(request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0)))
        .unwrap();
    let b = entities
        .spawn(request(owner, 2, "cow", Vec3::new(2.0, 64.0, 2.0)))
        .unwrap();
    let c = entities
        .spawn(request(owner, 3, "arrow", Vec3::new(3.0, 64.0, 3.0)))
        .unwrap();
    let origin = ChunkPos::new(0, 0);
    assert_eq!(
        entities
            .in_chunk(origin)
            .unwrap()
            .map(|e| e.uuid())
            .collect::<Vec<_>>(),
        [uuid(1), uuid(2), uuid(3)]
    );
    assert_eq!(
        entities
            .in_chunk(origin)
            .unwrap()
            .skip(1)
            .take(1)
            .next()
            .unwrap()
            .uuid(),
        uuid(2)
    );
    assert_eq!(entities.in_chunk(origin).unwrap().take(0).count(), 0);
    entities
        .update_transform(&b, transform(4.0, 80.0, 4.0))
        .unwrap();
    assert_eq!(
        entities.in_chunk(origin).unwrap().nth(1).unwrap().uuid(),
        uuid(2)
    );
    entities
        .update_transform(&b, transform(16.0, 64.0, 1.0))
        .unwrap();
    assert_eq!(
        entities
            .in_chunk(origin)
            .unwrap()
            .map(|e| e.uuid())
            .collect::<Vec<_>>(),
        [uuid(1), uuid(3)]
    );
    assert_eq!(
        entities
            .in_chunk(ChunkPos::new(1, 0))
            .unwrap()
            .next()
            .unwrap()
            .uuid(),
        uuid(2)
    );
    assert_eq!(entities.remove(&a).unwrap().uuid(), uuid(1));
    assert_eq!(entities.get(&a), Err(EntityStoreError::StaleHandle));
    assert_eq!(entities.remove(&a), Err(EntityStoreError::StaleHandle));
    let reused = entities
        .spawn(request(owner, 1, "zombie", Vec3::new(1.0, 64.0, 1.0)))
        .unwrap();
    assert_ne!(a, reused);
    assert_eq!(
        entities.handle_for_wire_id(EntityId::new(1)),
        Some(reused.clone())
    );
    assert_eq!(
        entities.update_transform(&a, transform(1.0, 64.0, 1.0)),
        Err(EntityStoreError::StaleHandle)
    );
    assert_eq!(snapshot(&entities).len(), 3);
    entities.remove(&c).unwrap();
    entities.remove(&reused).unwrap();
    assert_eq!(entities.in_chunk(origin).unwrap().len(), 0);
    entities.remove(&b).unwrap();
    assert!(entities.is_empty());
    assert!(entities.handle_for_uuid(uuid(1)).is_none());
    let mut empty = entities.in_chunk(origin).unwrap();
    assert_eq!(empty.next(), None);
    assert_eq!(empty.next(), None);
}

#[test]
fn handles_refuse_other_owners_recreation_and_fresh_snapshot_incarnations() {
    let owner = owner(0, 0);
    let mut original = store(owner, 2);
    let handle = original
        .spawn(request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0)))
        .unwrap();
    let mut copy = original.clone();
    let from_copy = copy.handle_for_uuid(uuid(1)).unwrap();
    assert_ne!(handle, from_copy);
    assert_eq!(copy.get(&handle), Err(EntityStoreError::ForeignHandle));
    assert_eq!(
        original.get(&from_copy),
        Err(EntityStoreError::ForeignHandle)
    );
    assert_eq!(snapshot(&copy), snapshot(&original));
    copy.update_transform(&from_copy, transform(16.0, 70.0, 1.0))
        .unwrap();
    assert_eq!(original.get(&handle).unwrap().chunk(), ChunkPos::new(0, 0));
    assert_eq!(
        store(self::owner(1, 0), 1).get(&handle),
        Err(EntityStoreError::ForeignHandle)
    );
    drop(original);
    let mut recreated = store(owner, 2);
    let new = recreated
        .spawn(request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0)))
        .unwrap();
    assert_ne!(new, handle);
    let before = snapshot(&recreated);
    assert_eq!(recreated.get(&handle), Err(EntityStoreError::ForeignHandle));
    assert_eq!(
        recreated.remove(&handle),
        Err(EntityStoreError::ForeignHandle)
    );
    assert_eq!(
        recreated.update_transform(&handle, transform(16.0, 64.0, 1.0)),
        Err(EntityStoreError::ForeignHandle)
    );
    assert_eq!(snapshot(&recreated), before);
}

#[test]
fn deterministic_interleavings_cover_all_64_buckets_without_index_aliasing() {
    let owner = owner(-1, 0);
    let mut left = store(owner, 64);
    let mut right = store(owner, 64);
    let min = owner.region().min_chunk();
    let mut handles = Vec::new();
    for id in 0_u16..64 {
        let chunk = ChunkPos::new(min.x() + i32::from(id % 8), min.z() + i32::from(id / 8));
        let position = Vec3::new(
            f64::from(chunk.x() * 16) + 0.5,
            64.0,
            f64::from(chunk.z() * 16) + 0.5,
        );
        handles.push((
            left.spawn(request(owner, id, "item", position)).unwrap(),
            right.spawn(request(owner, id, "item", position)).unwrap(),
        ));
    }
    assert_eq!(snapshot(&left), snapshot(&right));
    for (id, (a, b)) in handles.iter().enumerate() {
        let local = i32::try_from(id).unwrap();
        let chunk = ChunkPos::new(min.x() + (7 - local % 8), min.z() + (7 - local / 8));
        let moved = transform(f64::from(chunk.x() * 16), 80.0, f64::from(chunk.z() * 16));
        left.update_transform(a, moved).unwrap();
        right.update_transform(b, moved).unwrap();
        if id % 3 == 0 {
            left.remove(a).unwrap();
            right.remove(b).unwrap();
        }
        assert_eq!(snapshot(&left), snapshot(&right));
    }
}

#[test]
fn shard_initialization_and_missing_context_refusal_preserve_ordinary_inputs() {
    let mut shard = SimShard::new(ShardPos::new(0, 0));
    let player = PlayerId::offline("EntityFixture");
    shard
        .enqueue(GameInput::PlayerJoin {
            player,
            position: Vec3::new(1.0, 64.0, 1.0),
        })
        .unwrap();
    let refused = shard.run_tick_with_entity_work(|_| panic!("uninitialized work must not run"));
    assert_eq!(refused, Err(EntityStoreError::NotInitialized));
    assert_eq!(shard.inbox_len(), 1);
    assert_eq!(shard.player_count(), 0);
    assert!(matches!(
        shard.initialize_entities(NonZeroUsize::new(MAX_ENTITIES_PER_SHARD + 1).unwrap()),
        Err(EntityStoreError::InvalidCapacity { .. })
    ));
    assert!(shard.entities().is_none());
    shard.initialize_entities(NonZeroUsize::MIN).unwrap();
    assert_eq!(
        shard.initialize_entities(NonZeroUsize::MIN),
        Err(EntityStoreError::AlreadyInitialized)
    );
    let ((), outputs) = shard
        .run_tick_with_entity_work(|ctx| {
            assert!(ctx.entities().is_empty());
            assert!(!ctx.is_chunk_resident(ChunkPos::new(0, 0)).unwrap());
        })
        .unwrap();
    assert!(matches!(
        outputs.as_slice(),
        [GameOutput::PlayerSpawned { .. }]
    ));
    assert_eq!(shard.player_count(), 1);
    let mut bad = SimShard::new(ShardPos::new(i32::MAX, 0));
    assert!(matches!(
        bad.initialize_entities(NonZeroUsize::MIN),
        Err(EntityStoreError::InvalidOwner { .. })
    ));
    assert!(bad.entities().is_none());
}

#[test]
fn context_owner_and_handle_refusals_precede_nonresident_target_checks() {
    let owner = owner(0, 0);
    let mut shard = SimShard::in_dimension(owner.position(), owner.world(), owner.dimension());
    shard.initialize_entities(NonZeroUsize::MIN).unwrap();
    let mut foreign = store(owner, 1);
    let foreign_handle = foreign
        .spawn(request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0)))
        .unwrap();
    let other_scope =
        ShardId::try_new(WorldId::new(8), owner.dimension(), owner.position()).unwrap();
    shard
        .run_tick_with_entity_work(|ctx| {
            assert!(matches!(
                ctx.spawn(request(other_scope, 2, "item", Vec3::new(1.0, 64.0, 1.0))),
                Err(EntityStoreError::ScopeMismatch { .. })
            ));
            assert!(matches!(
                ctx.spawn(request(owner, 2, "item", Vec3::new(128.0, 64.0, 1.0))),
                Err(EntityStoreError::OutsideOwner { .. })
            ));
            assert!(matches!(
                ctx.is_chunk_resident(ChunkPos::new(8, 0)),
                Err(EntityStoreError::OutsideOwner { .. })
            ));
            assert_eq!(
                ctx.update_transform(&foreign_handle, transform(16.0, 64.0, 1.0)),
                Err(EntityStoreError::ForeignHandle)
            );
            assert_eq!(
                ctx.remove(&foreign_handle),
                Err(EntityStoreError::ForeignHandle)
            );
        })
        .unwrap();
    assert!(shard.entities().unwrap().is_empty());
    assert_eq!(shard.loaded_chunks().loaded_count(), 0);
    assert_eq!(foreign.get(&foreign_handle).unwrap().uuid(), uuid(1));
}

#[tokio::test]
async fn shard_tick_context_enforces_residency_and_keeps_registered_lifetime_separate() {
    let owner = owner(0, 0);
    let mut shard = SimShard::in_dimension(owner.position(), owner.world(), owner.dimension());
    shard
        .initialize_entities(NonZeroUsize::new(3).unwrap())
        .unwrap();
    let missing = request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0));
    let (refused, _) = shard
        .run_tick_with_entity_work(|ctx| ctx.spawn(missing))
        .unwrap();
    assert_eq!(
        refused,
        Err(EntityStoreError::ChunkNotResident {
            chunk: ChunkPos::new(0, 0)
        })
    );
    assert!(shard.entities().unwrap().is_empty());
    let storage = InMemoryStore::new();
    let generator = FlatWorldGenerator::new();
    let ticket = ChunkTicket::of(TicketReason::Spawn);
    shard
        .loaded_chunks_mut()
        .acquire(&storage, &generator, ChunkPos::new(0, 0), ticket)
        .await
        .unwrap();
    let (handle, _) = shard
        .run_tick_with_entity_work(|ctx| ctx.spawn(missing).unwrap())
        .unwrap();
    let before = snapshot(shard.entities().unwrap());
    let (refused, _) = shard
        .run_tick_with_entity_work(|ctx| ctx.update_transform(&handle, transform(16.0, 64.0, 1.0)))
        .unwrap();
    assert_eq!(
        refused,
        Err(EntityStoreError::ChunkNotResident {
            chunk: ChunkPos::new(1, 0)
        })
    );
    assert_eq!(snapshot(shard.entities().unwrap()), before);
    shard
        .loaded_chunks_mut()
        .acquire(&storage, &generator, ChunkPos::new(1, 0), ticket)
        .await
        .unwrap();
    shard
        .run_tick_with_entity_work(|ctx| {
            ctx.update_transform(&handle, transform(16.0, 64.0, 1.0))
                .unwrap();
        })
        .unwrap();
    let before_unload = snapshot(shard.entities().unwrap());
    shard
        .loaded_chunks_mut()
        .release(ChunkPos::new(1, 0), ticket);
    assert_eq!(snapshot(shard.entities().unwrap()), before_unload);
    let (refused, _) = shard
        .run_tick_with_entity_work(|ctx| {
            assert!(!ctx.is_chunk_resident(ChunkPos::new(1, 0)).unwrap());
            assert!(ctx.entities().get(&handle).is_ok());
            ctx.update_transform(&handle, transform(17.0, 64.0, 1.0))
        })
        .unwrap();
    assert!(matches!(
        refused,
        Err(EntityStoreError::ChunkNotResident { .. })
    ));
    shard
        .run_tick_with_entity_work(|ctx| {
            ctx.update_transform(&handle, transform(1.0, 64.0, 1.0))
                .unwrap();
        })
        .unwrap();
    shard
        .loaded_chunks_mut()
        .release(ChunkPos::new(0, 0), ticket);
    shard
        .run_tick_with_entity_work(|ctx| ctx.remove(&handle).unwrap())
        .unwrap();
    assert!(shard.entities().unwrap().is_empty());
}

#[tokio::test]
async fn shard_copy_gets_fresh_handles_and_entity_work_does_not_own_players() {
    let owner = owner(0, 0);
    let mut shard = SimShard::in_dimension(owner.position(), owner.world(), owner.dimension());
    shard
        .initialize_entities(NonZeroUsize::new(2).unwrap())
        .unwrap();
    let storage = InMemoryStore::new();
    shard
        .loaded_chunks_mut()
        .acquire(
            &storage,
            &FlatWorldGenerator::new(),
            ChunkPos::new(0, 0),
            ChunkTicket::of(TicketReason::Spawn),
        )
        .await
        .unwrap();
    let player = PlayerId::offline("EntityFixture");
    shard
        .enqueue(GameInput::PlayerJoin {
            player,
            position: Vec3::new(1.0, 64.0, 1.0),
        })
        .unwrap();
    let (handle, outputs) = shard
        .run_tick_with_entity_work(|ctx| {
            assert_eq!(
                ctx.spawn(request(owner, 2, "player", Vec3::new(1.0, 64.0, 1.0))),
                Err(EntityStoreError::PlayerType)
            );
            ctx.spawn(request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0)))
                .unwrap()
        })
        .unwrap();
    assert_eq!(shard.player_count(), 1);
    assert!(matches!(
        outputs.as_slice(),
        [GameOutput::PlayerSpawned { .. }]
    ));
    let mut copy = shard.clone();
    assert_eq!(
        copy.entities().unwrap().get(&handle),
        Err(EntityStoreError::ForeignHandle)
    );
    let copied = copy.entities().unwrap().handle_for_uuid(uuid(1)).unwrap();
    copy.run_tick_with_entity_work(|ctx| ctx.remove(&copied).unwrap())
        .unwrap();
    assert_eq!(shard.entities().unwrap().len(), 1);
    assert_eq!(copy.entities().unwrap().len(), 0);
    assert_eq!(copy.player_count(), 1);
}

async fn assert_replaced_chunk_map_refuses_scope(world: WorldId, dimension: DimensionId) {
    let owner = owner(0, 0);
    let mut shard = SimShard::in_dimension(owner.position(), owner.world(), owner.dimension());
    shard
        .initialize_entities(NonZeroUsize::new(2).unwrap())
        .unwrap();
    let storage = InMemoryStore::new();
    let generator = FlatWorldGenerator::new();
    let chunk = ChunkPos::new(0, 0);
    let ticket = ChunkTicket::of(TicketReason::Spawn);
    shard
        .loaded_chunks_mut()
        .acquire(&storage, &generator, chunk, ticket)
        .await
        .unwrap();
    let (handle, _) = shard
        .run_tick_with_entity_work(|ctx| {
            ctx.spawn(request(owner, 1, "item", Vec3::new(1.0, 64.0, 1.0)))
                .unwrap()
        })
        .unwrap();
    let records = snapshot(shard.entities().unwrap());
    let original_map = std::mem::replace(
        shard.loaded_chunks_mut(),
        LoadedChunkMap::new(world, dimension),
    );
    shard
        .loaded_chunks_mut()
        .acquire(&storage, &generator, chunk, ticket)
        .await
        .unwrap();
    let player = PlayerId::offline("ReplacedScopeFixture");
    shard
        .enqueue(GameInput::PlayerJoin {
            player,
            position: Vec3::new(1.0, 64.0, 1.0),
        })
        .unwrap();
    let attempted = request(owner, 2, "cow", Vec3::new(2.0, 64.0, 2.0));
    let result = shard.run_tick_with_entity_work(|ctx| ctx.spawn(attempted));
    assert_eq!(
        result,
        Err(EntityStoreError::ScopeMismatch {
            owner,
            world,
            dimension
        })
    );
    let never = shard
        .run_tick_with_entity_work(|_| panic!("foreign map must refuse before executing work"));
    assert_eq!(
        never,
        Err(EntityStoreError::ScopeMismatch {
            owner,
            world,
            dimension
        })
    );
    assert_eq!(shard.inbox_len(), 1);
    assert_eq!(shard.player_count(), 0);
    assert_eq!(snapshot(shard.entities().unwrap()), records);
    assert_eq!(
        shard.entities().unwrap().handle_for_uuid(uuid(1)),
        Some(handle.clone())
    );
    assert_eq!(
        shard.entities().unwrap().get(&handle).unwrap().uuid(),
        uuid(1)
    );
    assert!(shard.entities().unwrap().handle_for_uuid(uuid(2)).is_none());
    assert_eq!(shard.loaded_chunks().world(), world);
    assert_eq!(shard.loaded_chunks().dimension(), dimension);
    assert_eq!(shard.loaded_chunks().loaded_count(), 1);
    assert_eq!(shard.loaded_chunks().ticket_count(chunk), 1);
    *shard.loaded_chunks_mut() = original_map;
    let (accepted, outputs) = shard
        .run_tick_with_entity_work(|ctx| ctx.spawn(attempted))
        .unwrap();
    assert!(accepted.is_ok());
    assert!(matches!(
        outputs.as_slice(),
        [GameOutput::PlayerSpawned { .. }]
    ));
    assert_eq!(shard.inbox_len(), 0);
    assert_eq!(shard.player_count(), 1);
    assert_eq!(shard.entities().unwrap().len(), 2);
    assert_eq!(
        shard.entities().unwrap().get(&handle).unwrap().uuid(),
        uuid(1)
    );
}

#[tokio::test]
async fn replacing_chunk_map_world_refuses_entity_entry_without_mutating_owner_or_inbox() {
    assert_replaced_chunk_map_refuses_scope(WorldId::new(8), DimensionId::new(13)).await;
}

#[tokio::test]
async fn replacing_chunk_map_dimension_refuses_entity_entry_without_mutating_owner_or_inbox() {
    assert_replaced_chunk_map_refuses_scope(WorldId::new(7), DimensionId::new(14)).await;
}
