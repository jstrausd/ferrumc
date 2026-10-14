use super::*;
use crate::{MobEvent, MobIdentity, MobKind, MobSpawnAttempt, MobSpawnLight};
use ferrumc_core::{EntityId, EntityUuid};

fn mob_id(n: u128) -> MobIdentity {
    MobIdentity::new(
        EntityUuid::from_bytes((0xABC0 + n).to_be_bytes()),
        EntityId::new(i32::try_from(n).expect("bounded id") + 800),
    )
}
fn spawn_attempt(id: MobIdentity, kind: MobKind, position: Vec3) -> MobSpawnAttempt {
    let light = if kind == MobKind::Zombie {
        MobSpawnLight::new(0, 0, 0)
    } else {
        MobSpawnLight::new(15, 0, 15)
    }
    .expect("trusted fixture light");
    MobSpawnAttempt::new(id, kind, position, light, 31, 7, 42)
}
async fn mob_owner(p: PlayerId) -> SimShard {
    let mut s = shard_with_player(p).await;
    for x in [6, 20, 34, 127, 128] {
        ground_load(&mut s, Vec3::new(f64::from(x), 64.0, 8.0)).await;
    }
    s.enable_survival(crate::Difficulty::Normal, false)
        .expect("survival");
    s.enable_mobs(false, Vec3::new(-100.0, 64.0, -100.0))
        .expect("mobs");
    s.set_mob_player_profile(p, EntityId::new(2000), 0.0, 0.0)
        .expect("genuine player registration");
    s
}
fn mob_hit(
    s: &mut SimShard,
    p: PlayerId,
    id: MobIdentity,
    damage: f32,
    token: u64,
) -> (bool, f32, u8) {
    s.enqueue(GameInput::PlayerMobAttackTracked {
        player: p,
        target: id.wire_id(),
        damage,
        attack_speed: 1.6,
        token,
    })
    .expect("hit");
    s.run_tick()
        .into_iter()
        .find_map(|out| match out {
            GameOutput::MobAttackResolved {
                accepted,
                damage,
                experience,
                ..
            } => Some((accepted, damage, experience)),
            _ => None,
        })
        .expect("always resolves")
}

#[tokio::test]
async fn mob_owner_real_spawn_movement_and_hostile_player_health_with_armor() {
    let p = player("mob_victim");
    let mut s = mob_owner(p).await;
    let id = mob_id(1);
    s.enqueue_mob_spawn(spawn_attempt(
        id,
        MobKind::Zombie,
        Vec3::new(34.5, 64.0, 8.5),
    ))
    .expect("natural attempt");
    let _ = s.run_tick();
    let before = s
        .mob_snapshots(None, 64)
        .expect("mobs")
        .pop()
        .expect("zombie");
    assert_eq!(before.base().uuid(), id.uuid());
    assert_eq!(before.kind(), MobKind::Zombie);
    assert_eq!(before.health(), 20.0);
    s.set_mob_player_profile(p, EntityId::new(2000), 20.0, 8.0)
        .expect("armor mirror");
    s.enqueue(GameInput::PlayerMove {
        player: p,
        position: Some(Vec3::new(33.5, 64.0, 8.5)),
        yaw: None,
        pitch: None,
    })
    .expect("contact");
    let outputs = s.run_tick();
    assert!(outputs.iter().any(|out|matches!(out,GameOutput::MobPlayerHurt {attacker,damage,raw_damage,..} if *attacker==id.wire_id() && *damage>0.0 && *damage<3.0 && raw_damage.to_bits()==3.0_f32.to_bits())));
    let health = s.player_vitals(p).expect("owner health").health();
    assert!(health < 20.0 && health > 17.0);
    for _ in 0..18 {
        assert!(!s
            .run_tick()
            .iter()
            .any(|out| matches!(out, GameOutput::MobPlayerHurt { .. })));
    }
    let mut damaged_again = false;
    for _ in 0..3 {
        damaged_again |= s
            .run_tick()
            .iter()
            .any(|out| matches!(out, GameOutput::MobPlayerHurt { .. }));
    }
    assert!(damaged_again);
    let events = s.take_mob_events(256).expect("events");
    assert!(events
        .iter()
        .any(|event| matches!(event, MobEvent::Spawn(_))));
    assert!(events
        .iter()
        .any(|event| matches!(event, MobEvent::Move(_))));
    s.clear_mob_player_profile(p)
        .expect("reserve armor witness");
    let reused_wire = MobIdentity::new(mob_id(999).uuid(), EntityId::new(2000));
    assert!(
        s.enqueue_mob_spawn(spawn_attempt(
            reused_wire,
            MobKind::Cow,
            Vec3::new(34.5, 64.0, 8.5)
        ))
        .is_err(),
        "withdrawn target retains the genuine player wire reservation"
    );
    let pinned_health = s.player_vitals(p).expect("health").health();
    for _ in 0..40 {
        assert!(!s
            .run_tick()
            .iter()
            .any(|out| matches!(out, GameOutput::MobPlayerHurt { .. })));
    }
    assert_eq!(s.player_vitals(p).expect("health").health(), pinned_health);
    s.set_mob_player_profile(p, EntityId::new(2000), 20.0, 8.0)
        .expect("fresh armor projection");
    let mut resumed_hit = false;
    for _ in 0..80 {
        let _ = s.take_mob_events(256).expect("observer room");
        resumed_hit |= s
            .run_tick()
            .iter()
            .any(|out| matches!(out, GameOutput::MobPlayerHurt { .. }));
        if resumed_hit {
            break;
        }
    }
    assert!(resumed_hit, "fresh profile resumes actual chase and damage");
}

#[tokio::test]
async fn mob_owner_spawn_light_distance_collision_and_stale_world_edit_refuse_without_registration()
{
    let p = player("mob_spawner");
    let mut s = mob_owner(p).await;
    s.enqueue_mob_spawn(spawn_attempt(
        mob_id(2),
        MobKind::Cow,
        Vec3::new(8.5, 64.0, 8.5),
    ))
    .expect("queue near");
    let _ = s.run_tick();
    assert!(s.mob_snapshots(None, 64).expect("snapshot").is_empty());
    let id = mob_id(3);
    s.enqueue_mob_spawn(spawn_attempt(id, MobKind::Cow, Vec3::new(34.5, 64.0, 8.5)))
        .expect("queue valid");
    s.enqueue(GameInput::SetBlockExact {
        player: p,
        position: BlockPos::new(8, 65, 8),
        sequence: 1,
        state: BlockStateId::new(1),
    })
    .expect("intervening light edit");
    let _ = s.run_tick();
    assert!(s
        .mob_snapshots(None, 64)
        .expect("stale light refused")
        .is_empty());
    s.enqueue_mob_spawn(spawn_attempt(id, MobKind::Cow, Vec3::new(34.5, 64.0, 8.5)))
        .expect("fresh validated light fixture");
    let _ = s.run_tick();
    assert_eq!(s.mob_snapshots(None, 64).expect("actual spawn").len(), 1);
    let id2 = mob_id(4);
    let position = s.mob_snapshots(None, 64).expect("mobs")[0]
        .base()
        .transform()
        .position();
    s.enqueue_mob_spawn(spawn_attempt(id2, MobKind::Pig, position))
        .expect("overlapping queued");
    let _ = s.run_tick();
    assert_eq!(
        s.mob_snapshots(None, 64).expect("closed obstruction").len(),
        1
    );
}

#[tokio::test]
async fn mob_owner_canonical_cross_region_migration_preserves_uuid_wire_and_health() {
    let p = player("mob_border");
    let mut s = mob_owner(p).await;
    s.enqueue(GameInput::PlayerMove {
        player: p,
        position: Some(Vec3::new(100.0, 64.0, 8.5)),
        yaw: None,
        pitch: None,
    })
    .expect("spawn eligible");
    let _ = s.run_tick();
    let id = mob_id(5);
    s.enqueue_mob_spawn(spawn_attempt(
        id,
        MobKind::Zombie,
        Vec3::new(127.98, 64.0, 8.5),
    ))
    .expect("border spawn");
    let _ = s.run_tick();
    let before = s.mob_snapshots(None, 64).expect("snapshot")[0];
    s.enqueue(GameInput::PlayerMove {
        player: p,
        position: Some(Vec3::new(132.0, 64.0, 8.5)),
        yaw: None,
        pitch: None,
    })
    .expect("target beyondborder");
    for _ in 0..40 {
        let _ = s.run_tick();
    }
    let after = s.mob_snapshots(None, 64).expect("snapshot")[0];
    assert!(after.base().transform().position().x >= 128.0);
    assert_ne!(before.base().owner(), after.base().owner());
    assert_eq!(after.base().uuid(), id.uuid());
    assert_eq!(after.base().wire_id(), id.wire_id());
    assert_eq!(after.health(), 20.0);
    let runtime = s.mobs.as_ref().expect("owner");
    assert_eq!(
        runtime.stores.values().map(EntityStore::len).sum::<usize>(),
        1
    );
}

#[tokio::test]
async fn mob_owner_ground_offer_credits_do_not_block_spawn_but_reserve_before_death() {
    let p = player("mob_looter");
    let mut s = mob_owner(p).await;
    enable_ground_runtime(&mut s, 2);
    s.refill_ground_item_identities(vec![ground_identity(77)])
        .expect("ground identity");
    s.enqueue_ground_item_drop(crate::GroundItemDrop::new(
        Vec3::new(6.5, 64.1, 8.5),
        stack("diamond", 3),
        0,
        None,
        None,
    ))
    .expect("drop");
    let id = mob_id(6);
    s.enqueue_mob_spawn(spawn_attempt(
        id,
        MobKind::Sheep,
        Vec3::new(34.5, 64.0, 8.5),
    ))
    .expect("mob separate capacity");
    let _ = s.run_tick();
    let (ticket, owned) = s
        .take_ground_item_events(256)
        .expect("events")
        .into_iter()
        .find_map(|event| match event {
            crate::GroundItemEvent::PickupOffer { ticket, stack, .. } => Some((ticket, stack)),
            _ => None,
        })
        .expect("suspended actual payload");
    assert_eq!(owned, stack("diamond", 3));
    assert_eq!(
        s.mob_snapshots(None, 64).expect("independent slot").len(),
        1
    );
    s.enqueue(GameInput::PlayerMove {
        player: p,
        position: Some(Vec3::new(34.5, 64.0, 8.5)),
        yaw: None,
        pitch: None,
    })
    .expect("hit reach");
    for _ in 0..25 {
        let _ = s.run_tick();
    }
    let before = s.mob_snapshots(None, 64).expect("sheep")[0];
    s.enqueue(GameInput::PlayerMove {
        player: p,
        position: Some(before.base().transform().position()),
        yaw: None,
        pitch: None,
    })
    .expect("current genuine contact");
    assert_eq!(mob_hit(&mut s, p, id, 100.0, 1), (false, 0.0, 0));
    assert_eq!(
        s.mob_snapshots(None, 64)
            .expect("loot refusal retains health")[0]
            .health(),
        before.health()
    );
    let deposited = owned;
    assert_eq!(deposited, stack("diamond", 3));
    s.enqueue_ground_item_pickup_result(crate::GroundItemPickupResult::new(
        ticket,
        ItemStack::empty(),
    ))
    .expect("actual full deposit receipt");
    let _ = s.run_tick();
    let (accepted, damage, xp) = mob_hit(&mut s, p, id, 100.0, 2);
    assert!(
        accepted && damage > 0.0 && (1..=3).contains(&xp),
        "accepted={accepted} damage={damage} xp={xp}"
    );
    assert!(s.mob_snapshots(None, 64).expect("dead removed").is_empty());
    let ground = s.ground_items.as_ref().expect("ground");
    assert_eq!(ground.pending.len(), 2);
    assert!(ground.pending.iter().any(|(_, drop)| drop
        .stack
        .item()
        .is_some_and(|item| item.name() == "minecraft:white_wool")));
    assert!(ground.pending.iter().any(|(_, drop)| drop
        .stack
        .item()
        .is_some_and(|item| item.name() == "minecraft:mutton")));
    assert_eq!(mob_hit(&mut s, p, id, 100.0, 3), (false, 0.0, 0));
    assert_eq!(s.ground_items.as_ref().expect("ground").pending.len(), 2);
    let events = s.take_mob_events(256).expect("death publication");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, MobEvent::Remove(_)))
            .count(),
        1
    );
}

#[tokio::test]
async fn mob_owner_duplicate_identity_checks_both_ground_and_mob_admission_directions() {
    let p = player("mob_identity");
    let mut s = mob_owner(p).await;
    enable_ground_runtime(&mut s, 4);
    let ground = ground_identity(78);
    s.refill_ground_item_identities(vec![ground])
        .expect("reserved ground identity");
    let reused = MobIdentity::new(ground.uuid(), ground.wire_id());
    assert!(s
        .enqueue_mob_spawn(spawn_attempt(
            reused,
            MobKind::Cow,
            Vec3::new(34.5, 64.0, 8.5)
        ))
        .is_err());
    let id = mob_id(7);
    s.enqueue_mob_spawn(spawn_attempt(id, MobKind::Cow, Vec3::new(34.5, 64.0, 8.5)))
        .expect("queued mob identity");
    let reused =
        crate::GroundItemIdentity::new(id.uuid(), id.wire_id()).expect("ground identity proposal");
    assert!(s.refill_ground_item_identities(vec![reused]).is_err());
    let _ = s.run_tick();
    assert!(s.refill_ground_item_identities(vec![reused]).is_err());
    assert!(s.set_mob_player_profile(p, id.wire_id(), 0.0, 0.0).is_err());
}

#[tokio::test]
async fn mob_owner_clone_has_no_health_tick_authority_before_inbox_drain() {
    let p = player("mob_clone");
    let s = mob_owner(p).await;
    let mut snapshot = s.clone();
    snapshot
        .enqueue(GameInput::PlayerLeave { player: p })
        .expect("pending");
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| snapshot.run_tick())).is_err()
    );
    assert_eq!(snapshot.inbox_len(), 1);
    assert!(snapshot.contains_player(p));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one complete original/restored owner chronology including loot conservation
async fn mob_persistence_owner_exact_health_transform_brain_timers_and_death_loot_after_restart() {
    let p = player("saved_mob");
    let mut original = mob_owner(p).await;
    enable_ground_runtime(&mut original, 8);
    let id = mob_id(80);
    original
        .enqueue_mob_spawn(spawn_attempt(id, MobKind::Pig, Vec3::new(34.5, 64.0, 8.5)))
        .expect("spawn");
    let _ = original.run_tick();
    for _ in 0..25 {
        let _ = original.run_tick();
    }
    let pos = original.mob_snapshots(None, 64).expect("mob")[0]
        .base()
        .transform()
        .position();
    original
        .enqueue(GameInput::PlayerMove {
            player: p,
            position: Some(pos),
            yaw: None,
            pitch: None,
        })
        .expect("contact");
    assert!(mob_hit(&mut original, p, id, 4.0, 1).0);
    let saved = original.capture_mob_records().expect("whole actual save");
    assert_eq!(saved.len(), 1);
    assert_eq!(
        saved[0].1.record().data().len(),
        crate::NATIVE_MOB_ENTITY_BYTES
    );
    let proposal = crate::decode_mob_entity(saved[0].0, &saved[0].1).expect("strict restore");
    assert_eq!(proposal.uuid(), id.uuid());
    assert!(proposal.health() > 0.0 && proposal.health() < 10.0);
    let fresh = MobIdentity::new(id.uuid(), EntityId::new(9080));
    let mut restored = mob_owner(p).await;
    enable_ground_runtime(&mut restored, 8);
    restored
        .enqueue_mob_restore(proposal, fresh)
        .expect("admit proposal");
    assert!(restored
        .mob_snapshots(None, 64)
        .expect("no enqueue mutation")
        .is_empty());
    assert!(restored.capture_mob_records().is_err());
    assert!(restored
        .mob_chunks()
        .expect("queued ticket")
        .contains(&proposal.transform().chunk()));
    let _ = original.take_mob_events(256).expect("observer");
    let _ = original.run_tick();
    let _ = restored.run_tick();
    let spawn = restored
        .take_mob_events(256)
        .expect("restored events")
        .into_iter()
        .find_map(|event| match event {
            MobEvent::Spawn(snapshot) => Some(snapshot),
            _ => None,
        })
        .expect("real restored registration");
    assert_eq!(spawn.base().uuid(), id.uuid());
    assert_eq!(spawn.base().wire_id(), fresh.wire_id());
    assert_eq!(spawn.base().transform(), proposal.transform());
    assert_eq!(spawn.health().to_bits(), proposal.health().to_bits());
    for _ in 0..25 {
        assert_eq!(
            original.capture_mob_records().expect("original state"),
            restored
                .capture_mob_records()
                .expect("restored exact state")
        );
        let _ = original.take_mob_events(256).expect("observer");
        let _ = restored.take_mob_events(256).expect("observer");
        let _ = original.run_tick();
        let _ = restored.run_tick();
    }
    for s in [&mut original, &mut restored] {
        let pos = s.mob_snapshots(None, 64).expect("actual mob")[0]
            .base()
            .transform()
            .position();
        s.enqueue(GameInput::PlayerMove {
            player: p,
            position: Some(pos),
            yaw: None,
            pitch: None,
        })
        .expect("current contact");
    }
    assert!(mob_hit(&mut original, p, id, 100.0, 2).0);
    assert!(mob_hit(&mut restored, p, fresh, 100.0, 1).0);
    assert_eq!(
        original
            .capture_mob_records()
            .expect("actual deletion")
            .len(),
        0
    );
    assert_eq!(
        restored
            .capture_mob_records()
            .expect("actual deletion")
            .len(),
        0
    );
    let a: Vec<_> = original
        .ground_items
        .as_ref()
        .expect("ground")
        .pending
        .iter()
        .map(|(_, d)| &d.stack)
        .collect();
    let b: Vec<_> = restored
        .ground_items
        .as_ref()
        .expect("ground")
        .pending
        .iter()
        .map(|(_, d)| &d.stack)
        .collect();
    assert_eq!(
        a, b,
        "persisted loot RNG produces the same real admitted drops"
    );
}

#[tokio::test]
async fn mob_persistence_owner_strict_malformed_records_and_outer_relationships() {
    use ferrumc_storage::{
        EntityRecord, PersistentEntityKey, PersistentEntityRecord, SchemaVersion,
    };
    let p = player("mob_codec");
    let mut s = mob_owner(p).await;
    s.enqueue_mob_spawn(spawn_attempt(
        mob_id(81),
        MobKind::Cow,
        Vec3::new(34.5, 64.0, 8.5),
    ))
    .expect("spawn");
    let _ = s.run_tick();
    let (key, record) = s
        .capture_mob_records()
        .expect("capture")
        .pop()
        .expect("record");
    let bytes = record.record().data();
    let decode = |data: Vec<u8>, schema: u32| {
        let invalid = PersistentEntityRecord::new(
            record.chunk(),
            EntityRecord::new(SchemaVersion::new(schema), data).expect("bounded"),
        )
        .expect("column");
        crate::decode_mob_entity(key, &invalid)
    };
    for len in 0..bytes.len() {
        assert!(decode(bytes[..len].to_vec(), 1).is_err(), "truncated {len}");
    }
    let mut trailing = bytes.to_vec();
    trailing.push(0);
    assert!(decode(trailing, 1).is_err());
    assert!(decode(bytes.to_vec(), 2).is_err());
    for at in [0, 5, 6, 10, 14, 18, 66] {
        let mut body = bytes.to_vec();
        body[at] ^= 255;
        assert!(decode(body, 1).is_err(), "malformed fixed field {at}");
    }
    for (at, value) in [(34, f64::NAN), (42, f64::INFINITY), (50, 30_000_001.0)] {
        let mut body = bytes.to_vec();
        body[at..at + 8].copy_from_slice(&value.to_bits().to_be_bytes());
        assert!(decode(body, 1).is_err());
    }
    for (at, value) in [
        (58, 360.0_f32),
        (62, 91.0),
        (67, 0.0),
        (67, 1000.0),
        (67, f32::NAN),
        (91, -1.0),
        (91, f32::INFINITY),
    ] {
        let mut body = bytes.to_vec();
        body[at..at + 4].copy_from_slice(&value.to_bits().to_be_bytes());
        assert!(decode(body, 1).is_err(), "malformed scalar {at} {value}");
    }
    for (at, value) in [(87, 9), (88, 41), (89, 21), (90, 11)] {
        let mut body = bytes.to_vec();
        body[at] = value;
        assert!(decode(body, 1).is_err());
    }
    let wrong = PersistentEntityKey::new(key.world(), key.dimension(), mob_id(82).uuid());
    assert!(crate::decode_mob_entity(wrong, &record).is_err());
    let wrong_column =
        PersistentEntityRecord::new(ChunkPos::new(7, 7), record.record().clone()).expect("column");
    assert!(crate::decode_mob_entity(key, &wrong_column).is_err());
    assert!(
        crate::decode_item_entity(key, &record).is_err(),
        "distinct native item domain"
    );
    assert_eq!(
        s.capture_mob_records().expect("unchanged actual owner"),
        vec![(key, record)]
    );
}

#[tokio::test]
async fn mob_persistence_owner_restore_queue16_identity_and_residency_preserve_whole_proposals() {
    use ferrumc_storage::{
        EntityRecord, PersistentEntityKey, PersistentEntityRecord, SchemaVersion,
    };
    let p = player("mob_restore_queue");
    let mut source = mob_owner(p).await;
    source
        .enqueue_mob_spawn(spawn_attempt(
            mob_id(90),
            MobKind::Cow,
            Vec3::new(34.5, 64.0, 8.5),
        ))
        .expect("spawn");
    let _ = source.run_tick();
    let (key, record) = source
        .capture_mob_records()
        .expect("saved")
        .pop()
        .expect("record");
    let mut target = shard_with_player(p).await;
    target
        .enable_survival(crate::Difficulty::Normal, false)
        .expect("survival");
    target
        .enable_mobs(false, Vec3::new(-100.0, 64.0, -100.0))
        .expect("mobs");
    enable_ground_runtime(&mut target, 4);
    let mut seventeenth = None;
    for n in 0..17 {
        let id = mob_id(100 + n);
        let newkey = PersistentEntityKey::new(key.world(), key.dimension(), id.uuid());
        let mut body = record.record().data().to_vec();
        body[18..34].copy_from_slice(&id.uuid().to_bytes());
        let newrecord = PersistentEntityRecord::new(
            record.chunk(),
            EntityRecord::new(SchemaVersion::new(1), body).expect("body"),
        )
        .expect("record");
        let proposal = crate::decode_mob_entity(newkey, &newrecord).expect("proposal");
        if n == 16 {
            let refused = target
                .enqueue_mob_restore(proposal, id)
                .expect_err("complete queue bound");
            let (_, retained, identity) = refused.into_parts();
            assert_eq!(retained.uuid(), proposal.uuid());
            assert_eq!(identity, id);
            seventeenth = Some((retained, identity));
        } else {
            target
                .enqueue_mob_restore(proposal, id)
                .expect("retain unloaded");
        }
    }
    assert_eq!(target.mobs.as_ref().expect("runtime").restores.len(), 16);
    let reused = crate::GroundItemIdentity::new(mob_id(100).uuid(), mob_id(100).wire_id())
        .expect("identity");
    assert!(target.refill_ground_item_identities(vec![reused]).is_err());
    let duplicate_player = PlayerId::from_uuid(mob_id(100).uuid().as_uuid());
    target
        .enqueue(GameInput::PlayerJoin {
            player: duplicate_player,
            position: Vec3::new(8.0, 64.0, 8.0),
        })
        .expect("queued join");
    let outputs = target.run_tick();
    assert!(!outputs.iter().any(
        |output| matches!(output,GameOutput::PlayerSpawned {player,..} if *player==duplicate_player)
    ));
    assert!(target
        .mob_snapshots(None, 64)
        .expect("no force load")
        .is_empty());
    assert_eq!(target.mobs.as_ref().expect("runtime").restores.len(), 16);
    ground_load(&mut target, Vec3::new(34.5, 64.0, 8.5)).await;
    let _ = target.run_tick();
    assert_eq!(
        target
            .mob_snapshots(None, 64)
            .expect("actual16 admission")
            .len(),
        16
    );
    assert!(target.mobs.as_ref().expect("runtime").restores.is_empty());
    let (proposal, id) = seventeenth.expect("retained tail");
    target
        .enqueue_mob_restore(proposal, id)
        .expect("retry after actual room");
    let _ = target.run_tick();
    assert_eq!(
        target.capture_mob_records().expect("complete set").len(),
        17
    );
    assert!(
        target.enqueue_mob_restore(proposal, id).is_err(),
        "replay UUID cannot duplicate live mob"
    );
    let snapshot = target.clone();
    assert!(
        snapshot.capture_mob_records().is_err(),
        "no cloned capture authority"
    );
}
