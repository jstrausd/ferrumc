//! Focused pure ownership, bounds, exact-fact and resident-terrain tests.
use super::*;
use ferrumc_registry::block_state::compute_state_id;
fn state(name: &str) -> BlockStateId {
    BlockStateId::new(compute_state_id(name, &BTreeMap::new()).unwrap())
}
fn special(name: &str, props: &[(&str, &str)]) -> BlockStateId {
    BlockStateId::new(compute_state_id(name, &props.iter().copied().collect()).unwrap())
}
fn actor(id: i32, x: f64) -> MobActor {
    MobActor::new(EntityId::new(id), Vec3::new(x, 1.0, 0.5), 0.0).unwrap()
}
#[allow(clippy::unnecessary_wraps)] // Same resident-reader boundary as production.
fn ground(pos: BlockPos) -> Option<BlockStateId> {
    Some(if pos.y() == 0 {
        state("grass_block")
    } else {
        BlockStateId::AIR
    })
}
fn context(
    raw: u8,
    sky: u8,
    block: u8,
    distance: f64,
    peaceful: bool,
    sky_roll: u8,
    dark_roll: u8,
) -> MobSpawnContext {
    MobSpawnContext::new(
        MobSpawnLight::new(sky, block, raw).unwrap(),
        distance,
        peaceful,
        sky_roll,
        dark_roll,
    )
    .unwrap()
}

#[test]
fn pinned_jvm_attributes_metadata_and_armor_are_exact() {
    let rules = MobRules::builtin().unwrap();
    let expected = [
        (MobKind::Sheep, 106, 8.0),
        (MobKind::Cow, 28, 10.0),
        (MobKind::Pig, 95, 10.0),
        (MobKind::Chicken, 25, 4.0),
        (MobKind::Zombie, 145, 20.0),
    ];
    for (kind, id, health) in expected {
        let a = rules.attributes(kind);
        assert_eq!(a.entity_type().protocol_id(), id);
        assert_eq!(a.max_health(), health);
    }
    assert_eq!(rules.attributes(MobKind::Zombie).armor(), 2.0);
    assert_eq!(
        rules.attributes(MobKind::Zombie).movement_speed().to_bits(),
        f64::from(0.23_f32).to_bits()
    );
    assert_eq!(
        rules.attributes(MobKind::Cow).width().to_bits(),
        f64::from(0.9_f32).to_bits()
    );
    assert_eq!(
        rules.health_metadata(),
        MobMetadataAccessor {
            index: 9,
            serializer: 3
        }
    );
    assert_eq!(
        rules.flags_metadata(),
        MobMetadataAccessor {
            index: 15,
            serializer: 0
        }
    );
    assert_eq!(
        rules.baby_metadata(MobKind::Zombie),
        MobMetadataAccessor {
            index: 16,
            serializer: 8
        }
    );
    assert_eq!(
        rules.sheep_wool_metadata(),
        MobMetadataAccessor {
            index: 17,
            serializer: 0
        }
    );
    assert_eq!(
        rules.zombie_conversion_metadata(),
        MobMetadataAccessor {
            index: 18,
            serializer: 8
        }
    );
    assert_eq!(
        rules.attack_reach().to_bits(),
        (f64::from(2.04_f32).sqrt() - f64::from(0.6_f32)).to_bits()
    );
    for line in include_str!("data/armor.tsv").lines() {
        let (damage, result) = line.split_once('\t').unwrap();
        let damage = f32::from_bits(damage.parse().unwrap());
        let result: u32 = result.parse().unwrap();
        assert_eq!(ordinary_armor_damage(damage, 2.0).to_bits(), result);
    }
    assert_eq!(rules.facts.states.len(), 27946);
    assert_eq!(rules.facts.shapes.len(), 321);
    assert!(rules
        .facts
        .states
        .iter()
        .filter(|s| s.flags & 1 != 0)
        .all(|s| rules.facts.shapes[s.shape].len() <= 64));
}

#[test]
fn spawn_checks_exact_light_distance_floor_headroom_and_residency() {
    let rules = MobRules::builtin().unwrap();
    let position = Vec3::new(0.5, 1.0, 0.5);
    for kind in MobKind::ALL {
        let valid = if kind == MobKind::Zombie {
            context(7, 0, 0, 625.0, false, 0, 7)
        } else {
            context(9, 15, 0, 625.0, false, 0, 0)
        };
        assert_eq!(rules.validate_spawn(kind, position, valid, ground), Ok(()));
        assert_eq!(
            rules.validate_spawn(kind, position, valid, |p| if p.y() == 2 {
                Some(state("stone"))
            } else {
                ground(p)
            }),
            Err(MobError::SpawnRefused)
        );
        assert_eq!(
            rules.validate_spawn(kind, position, valid, |_| None),
            Err(MobError::IncompleteTerrain)
        );
        assert_eq!(
            rules.validate_spawn(kind, position, valid, |p| if p.y() == 1 {
                Some(state("water"))
            } else {
                ground(p)
            }),
            Err(MobError::SpawnRefused)
        );
    }
    for distance in [576.0, 16384.1] {
        assert_eq!(
            rules.validate_spawn(
                MobKind::Pig,
                position,
                context(9, 0, 0, distance, false, 0, 0),
                ground
            ),
            Err(MobError::SpawnRefused)
        );
    }
    assert_eq!(
        rules.validate_spawn(
            MobKind::Pig,
            position,
            context(9, 0, 0, 16384.0, false, 0, 0),
            ground
        ),
        Ok(())
    );
    assert_eq!(
        rules.validate_spawn(
            MobKind::Pig,
            position,
            context(8, 0, 0, 625.0, false, 0, 0),
            ground
        ),
        Err(MobError::SpawnRefused)
    );
    assert_eq!(
        rules.validate_spawn(
            MobKind::Pig,
            position,
            context(9, 0, 0, 625.0, false, 0, 0),
            |p| Some(if p.y() == 0 {
                state("stone")
            } else {
                BlockStateId::AIR
            })
        ),
        Err(MobError::SpawnRefused)
    );
    for c in [
        context(0, 0, 0, 625.0, true, 0, 0),
        context(8, 0, 0, 625.0, false, 0, 7),
        context(0, 1, 0, 625.0, false, 0, 0),
        context(0, 0, 1, 625.0, false, 0, 0),
    ] {
        assert_eq!(
            rules.validate_spawn(MobKind::Zombie, position, c, ground),
            Err(MobError::SpawnRefused)
        );
    }
    assert!(MobSpawnLight::new(16, 0, 0).is_err());
    assert!(
        MobSpawnContext::new(MobSpawnLight::new(0, 0, 0).unwrap(), f64::NAN, false, 0, 0).is_err()
    );
}

#[test]
fn pinned_hazards_and_dynamic_context_shapes_close_spawns() {
    let rules = MobRules::builtin().unwrap();
    let position = Vec3::new(0.5, 1.0, 0.5);
    for hazard in [
        state("fire"),
        state("soul_fire"),
        state("sweet_berry_bush"),
        state("cactus"),
    ] {
        assert_eq!(
            rules.validate_spawn(
                MobKind::Pig,
                position,
                context(9, 0, 0, 625.0, false, 0, 0),
                |p| if p == BlockPos::new(0, 1, 0) {
                    Some(hazard)
                } else {
                    ground(p)
                }
            ),
            Err(MobError::SpawnRefused)
        );
    }
    for dynamic in [
        state("scaffolding"),
        state("powder_snow"),
        state("moving_piston"),
    ] {
        assert_eq!(
            rules.validate_spawn(
                MobKind::Pig,
                position,
                context(9, 0, 0, 625.0, false, 0, 0),
                |p| if p == BlockPos::new(0, 1, 0) {
                    Some(dynamic)
                } else {
                    ground(p)
                }
            ),
            Err(MobError::UnsupportedState)
        );
    }
}

#[test]
fn native_grounded_motion_blocks_walls_fences_thin_shapes_and_unknown_cells() {
    let rules = MobRules::builtin().unwrap();
    let attributes = rules.attributes(MobKind::Pig);
    let before = Vec3::new(0.5, 1.0, 0.5);
    let after = Vec3::new(0.625, 1.0, 0.5);
    let mut read = ground;
    let mut terrain = physics::Terrain::new(&rules.facts, &mut read);
    assert_eq!(
        terrain.grounded_move(before, after, attributes).unwrap(),
        Some(after)
    );
    let mut read = |p: BlockPos| {
        if p == BlockPos::new(1, 1, 0) {
            Some(special(
                "minecraft:stone_slab",
                &[("type", "bottom"), ("waterlogged", "false")],
            ))
        } else {
            ground(p)
        }
    };
    let mut terrain = physics::Terrain::new(&rules.facts, &mut read);
    assert_eq!(
        terrain.grounded_move(before, after, attributes).unwrap(),
        Some(Vec3::new(0.625, 1.5, 0.5))
    );
    for obstacle in [
        state("stone"),
        state("oak_fence"),
        special(
            "minecraft:oak_trapdoor",
            &[
                ("facing", "west"),
                ("half", "bottom"),
                ("open", "true"),
                ("powered", "false"),
                ("waterlogged", "false"),
            ],
        ),
    ] {
        let mut read = |p: BlockPos| {
            if p == BlockPos::new(1, 1, 0) {
                Some(obstacle)
            } else {
                ground(p)
            }
        };
        let mut terrain = physics::Terrain::new(&rules.facts, &mut read);
        // Approach the genuine leading collision face for each shape.
        let fact = rules.facts.states[obstacle.as_u32() as usize];
        let min_x = rules.facts.shapes[fact.shape]
            .iter()
            .map(|b| b.min[0])
            .min_by(f64::total_cmp)
            .unwrap();
        let x = 1.0 + min_x - attributes.width() / 2.0 - 0.01;
        assert!(terrain
            .grounded_move(
                Vec3::new(x, 1.0, 0.5),
                Vec3::new(x + 0.125, 1.0, 0.5),
                attributes
            )
            .unwrap()
            .is_none());
    }
    let mut read = |p: BlockPos| if p.x() >= 16 { None } else { ground(p) };
    let mut terrain = physics::Terrain::new(&rules.facts, &mut read);
    assert_eq!(
        terrain.grounded_move(
            Vec3::new(15.0, 1.0, 0.5),
            Vec3::new(15.125, 1.0, 0.5),
            attributes
        ),
        Err(MobError::IncompleteTerrain)
    );
    let mut read = |p: BlockPos| {
        if p.y() == 0 {
            Some(BlockStateId::AIR)
        } else {
            ground(p)
        }
    };
    let mut terrain = physics::Terrain::new(&rules.facts, &mut read);
    assert_eq!(
        terrain.grounded_move(before, after, attributes).unwrap(),
        None
    );
    let mut read = |p: BlockPos| {
        if p == BlockPos::new(1, 1, 0) {
            Some(BlockStateId::new(u32::MAX))
        } else {
            ground(p)
        }
    };
    let mut terrain = physics::Terrain::new(&rules.facts, &mut read);
    assert_eq!(
        terrain.grounded_move(before, after, attributes),
        Err(MobError::UnsupportedState)
    );
}

#[test]
fn tick_preflight_caps_atomicity_and_deterministic_resident_chase() {
    let rules = MobRules::builtin().unwrap();
    let mut a = MobController::new(rules.clone());
    let mut b = MobController::new(rules);
    let id = EntityId::new(1);
    a.admit(id, MobKind::Zombie, 42).unwrap();
    b.admit(id, MobKind::Zombie, 42).unwrap();
    let actors = [actor(1, 0.5)];
    let targets = [MobTarget::new(EntityId::new(100), Vec3::new(5.0, 1.0, 0.5), true).unwrap()];
    assert_eq!(
        a.step(10, &[actors[0], actors[0]], &targets, ground)
            .unwrap_err(),
        MobError::DuplicateEntity
    );
    let mut reads = 0;
    let out = a
        .step(10, &actors, &targets, |p| {
            reads += 1;
            ground(p)
        })
        .unwrap();
    assert!(reads <= MAX_MOB_READS);
    assert_eq!(out.movements().len(), 1);
    assert!(out.movements()[0].after().x > actors[0].position().x);
    assert_eq!(out.movements()[0].after().y, 1.0);
    let other = b.step(10, &actors, &targets, ground).unwrap();
    assert_eq!(out.movements()[0].after(), other.movements()[0].after());
    assert_eq!(
        a.step(10, &actors, &targets, ground).unwrap_err(),
        MobError::InvalidTick
    );
    assert_eq!(
        a.step(12, &actors, &targets, ground).unwrap_err(),
        MobError::InvalidTick
    );
    let blocked = a.step(11, &actors, &targets, |_| None).unwrap();
    assert!(blocked.movements().is_empty());
    assert!(blocked.attacks().is_empty());
    for n in 2..=64 {
        a.admit(EntityId::new(n), MobKind::Chicken, n as u64)
            .unwrap();
    }
    assert_eq!(a.len(), 64);
    assert_eq!(
        a.admit(EntityId::new(65), MobKind::Pig, 1).unwrap_err(),
        MobError::Capacity
    );
    assert_eq!(
        a.step(12, &vec![actor(1, 0.5); 65], &[], ground)
            .unwrap_err(),
        MobError::Capacity
    );
    assert!(a.step(12, &actors, &[], ground).is_ok());
}

#[test]
fn zombie_attack_requires_target_eligibility_original_range_sight_and_twenty_ticks() {
    let rules = MobRules::builtin().unwrap();
    let mut c = MobController::new(rules);
    c.admit(EntityId::new(1), MobKind::Zombie, 1).unwrap();
    let actors = [actor(1, 0.5)];
    let target = MobTarget::new(EntityId::new(100), Vec3::new(1.5, 1.0, 0.5), true).unwrap();
    for now in 0..=40 {
        let out = c.step(now, &actors, &[target], ground).unwrap();
        assert_eq!(out.attacks().len(), usize::from(now % 20 == 0));
        if let Some(hit) = out.attacks().first() {
            assert_eq!(hit.base_damage(), 3.0);
            assert_eq!(hit.attacker(), EntityId::new(1));
            assert_eq!(hit.target(), EntityId::new(100));
        }
    }
    let invisible = MobTarget::new(EntityId::new(100), Vec3::new(1.5, 1.0, 0.5), false).unwrap();
    assert!(c
        .step(41, &actors, &[invisible], ground)
        .unwrap()
        .attacks()
        .is_empty());
    let mut c = MobController::new(MobRules::builtin().unwrap());
    c.admit(EntityId::new(1), MobKind::Zombie, 1).unwrap();
    let obstacle = |p: BlockPos| {
        if p.x() == 1 && p.y() >= 1 {
            Some(state("stone"))
        } else {
            ground(p)
        }
    };
    assert!(c
        .step(0, &actors, &[target], obstacle)
        .unwrap()
        .attacks()
        .is_empty());
    let floating = [MobActor::new(EntityId::new(1), Vec3::new(0.5, 2.0, 0.5), 0.0).unwrap()];
    let nearby = MobTarget::new(EntityId::new(100), Vec3::new(1.0, 2.0, 0.5), true).unwrap();
    let denied = c.step(1, &floating, &[nearby], ground).unwrap();
    assert!(denied.attacks().is_empty());
    assert!(denied.movements().is_empty());
    let high = MobTarget::new(EntityId::new(100), Vec3::new(0.5, 3.0, 0.5), true).unwrap();
    assert!(c
        .step(2, &actors, &[high], ground)
        .unwrap()
        .attacks()
        .is_empty());
}

#[test]
fn staged_damage_owns_drops_and_refuses_stale_foreign_reused_and_repeated_death() {
    let rules = MobRules::builtin().unwrap();
    let id = EntityId::new(1);
    let mut c = MobController::new(rules.clone());
    let initial = c.admit(id, MobKind::Sheep, 5).unwrap();
    assert_eq!(initial.health(), 8.0);
    let plan = c
        .prepare_damage(0, id, 100.0, MobDamageContext::new(true, true))
        .unwrap();
    assert!(plan.is_death());
    assert_eq!(plan.drops().len(), 2);
    assert_eq!(c.health(id).unwrap(), initial);
    assert_eq!(c.dirty_health().count(), 1);
    let mut foreign = MobController::new(rules);
    foreign.admit(id, MobKind::Sheep, 5).unwrap();
    let refusal = foreign.apply_damage(plan).unwrap_err();
    assert_eq!(refusal.error(), MobError::ForeignPlan);
    let plan = refusal.into_plan();
    assert_eq!(plan.drops().len(), 2);
    let second = c
        .prepare_damage(0, id, 100.0, MobDamageContext::new(true, true))
        .unwrap();
    let receipt = c.apply_damage(plan).unwrap();
    assert_eq!(receipt.fact().health(), 0.0);
    assert!(receipt.drops().iter().all(ItemStack::is_valid));
    assert!((1..=3).contains(&receipt.experience()));
    assert_eq!(
        receipt.drops()[0].item().unwrap().name(),
        "minecraft:cooked_mutton"
    );
    assert_eq!(
        c.prepare_damage(1, id, 1.0, MobDamageContext::new(false, true))
            .unwrap_err(),
        MobError::Dead
    );
    let refusal = c.apply_damage(second).unwrap_err();
    assert_eq!(refusal.error(), MobError::StalePlan);
    assert_eq!(refusal.into_plan().drops().len(), 2);
    assert!(!c.ack_health_saved(initial));
    let current = c.health(id).unwrap();
    assert!(c.ack_health_saved(current));
    assert_eq!(c.dirty_health().count(), 0);
    let drops = receipt.into_drops();
    assert_eq!(drops.len(), 2);
    c.remove(id);
    c.admit(id, MobKind::Sheep, 5).unwrap();
    let plan = c
        .prepare_damage(2, id, 100.0, MobDamageContext::new(false, false))
        .unwrap();
    c.remove(id);
    c.admit(id, MobKind::Sheep, 5).unwrap();
    assert_eq!(
        c.apply_damage(plan).unwrap_err().error(),
        MobError::StalePlan
    );
    assert!(!c.ack_health_saved(current));
}

#[test]
fn ordinary_hurt_window_and_dirty_generations_are_exact() {
    let mut c = MobController::new(MobRules::builtin().unwrap());
    let id = EntityId::new(1);
    c.admit(id, MobKind::Pig, 1).unwrap();
    let ctx = MobDamageContext::new(false, false);
    let plan = c.prepare_damage(0, id, 3.0, ctx).unwrap();
    assert_eq!(plan.health_after(), 7.0);
    let r = c.apply_damage(plan).unwrap();
    assert_eq!(r.fact().generation(), 2);
    for now in [0, 1, 9] {
        assert_eq!(
            c.prepare_damage(now, id, 3.0, ctx).unwrap_err(),
            MobError::HurtCooldown
        );
    }
    let p = c.prepare_damage(5, id, 5.0, ctx).unwrap();
    assert_eq!(p.health_after(), 5.0);
    c.apply_damage(p).unwrap();
    assert_eq!(
        c.prepare_damage(9, id, 5.0, ctx).unwrap_err(),
        MobError::HurtCooldown
    );
    let p = c.prepare_damage(10, id, 1.0, ctx).unwrap();
    assert_eq!(p.health_after(), 4.0);
    let new = c.apply_damage(p).unwrap().fact();
    assert!(!c.ack_health_saved(r.fact()));
    assert!(c.ack_health_saved(new));
    assert!(c.prepare_damage(10, id, f32::NAN, ctx).is_err());
    assert!(c.prepare_damage(10, id, 0.0, ctx).is_err());
    let p = c.prepare_damage(20, id, 1.0, ctx).unwrap();
    c.step(21, &[], &[], ground).unwrap();
    assert_eq!(c.apply_damage(p).unwrap_err().error(), MobError::StalePlan);
}

#[test]
fn all_normal_loot_is_valid_bounded_and_native_rng_is_reproducible() {
    let rules = MobRules::builtin().unwrap();
    let mut rare = 0;
    for seed in 0..256 {
        for kind in MobKind::ALL {
            let mut a = MobController::new(rules.clone());
            let mut b = MobController::new(rules.clone());
            let id = EntityId::new(1);
            a.admit(id, kind, seed).unwrap();
            b.admit(id, kind, seed).unwrap();
            let ctx = MobDamageContext::new(seed % 2 == 0, true);
            let p = a.prepare_damage(0, id, 100.0, ctx).unwrap();
            let q = b.prepare_damage(0, id, 100.0, ctx).unwrap();
            assert_eq!(p.drops(), q.drops());
            assert_eq!(p.experience(), q.experience());
            assert!(p.drops().len() <= 2);
            assert!(p.drops().iter().all(ItemStack::is_valid));
            for stack in p.drops() {
                assert!((1..=3).contains(&stack.count()));
                if kind == MobKind::Zombie
                    && stack.item().unwrap().name() != "minecraft:rotten_flesh"
                {
                    rare += 1;
                    assert_eq!(stack.count(), 1);
                }
            }
            assert_eq!(
                a.health(id).unwrap().health(),
                rules.attributes(kind).max_health()
            );
            if kind == MobKind::Zombie {
                assert_eq!(p.experience(), 5);
            } else {
                assert!((1..=3).contains(&p.experience()));
            }
            let none = a
                .prepare_damage(0, id, 100.0, MobDamageContext::new(false, false))
                .unwrap();
            assert_eq!(none.experience(), 0);
        }
    }
    assert!(rare > 0);
}
