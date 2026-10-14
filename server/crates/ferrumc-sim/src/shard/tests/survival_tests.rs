use super::*;
use crate::{DamageRequest, Difficulty, VitalsSnapshot};

#[tokio::test]
async fn survival_owner_food_ack_is_authoritative_and_replay_safe() {
    let p = player("food_owner");
    let mut s = shard_with_player(p).await;
    s.enable_survival(Difficulty::Normal, true).expect("enable");
    let food =
        ferrumc_items::builtin_food(ferrumc_items::ItemId::from_name("apple").expect("item"))
            .expect("food");
    assert!(!s.player_can_eat(p, food));
    let original = s.player_vitals(p).expect("vitals");
    s.enqueue(GameInput::FoodConsumed {
        player: p,
        food,
        token: 1,
    })
    .expect("consume");
    assert_eq!(s.player_vitals(p), Some(original));
    assert!(s.run_tick().iter().any(|output| matches!(
        output,
        GameOutput::FoodConsumptionResolved {
            token: 1,
            accepted: false,
            ..
        }
    )));
    s.enqueue(GameInput::RestoreVitals {
        player: p,
        snapshot: VitalsSnapshot::new(20.0, 10, 0.0, 0.0).expect("saved"),
    })
    .expect("restore");
    let _ = s.run_tick();
    assert!(s.player_can_eat(p, food));
    for _ in 0..2 {
        s.enqueue(GameInput::FoodConsumed {
            player: p,
            food,
            token: 2,
        })
        .expect("consume");
    }
    let outputs = s.run_tick();
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(
                output,
                GameOutput::FoodConsumptionResolved { accepted: true, .. }
            ))
            .count(),
        1
    );
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(
                output,
                GameOutput::FoodConsumptionResolved {
                    accepted: false,
                    ..
                }
            ))
            .count(),
        1
    );
    assert_eq!(s.player_vitals(p).expect("vitals").food(), 14);
}

#[tokio::test]
async fn survival_owner_damage_death_and_respawn_mutate_real_owner() {
    let p = player("health_owner");
    let mut s = shard_with_player(p).await;
    s.enable_survival(Difficulty::Normal, false)
        .expect("enable");
    let lethal = DamageRequest::new(30.0, 0.0, 0.0).expect("source");
    s.enqueue(GameInput::PlayerDamage {
        player: p,
        request: lethal,
    })
    .expect("damage");
    assert_eq!(s.player_vitals(p).expect("vitals").health(), 20.0);
    let outputs = s.run_tick();
    assert_eq!(s.player_vitals(p).expect("vitals").health(), 0.0);
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(output, GameOutput::PlayerDied { .. }))
            .count(),
        1
    );
    let position = BlockPos::new(8, 65, 8);
    let outputs = place_block(&mut s, p, position, 1, Direction::Up, 1.0, 0.0, 1);
    assert!(outputs
        .iter()
        .any(|output| matches!(output, GameOutput::BlockChangeRejected { .. })));
    assert!(s.authoritative_state(position).is_air());
    s.enqueue(GameInput::PlayerDamage {
        player: p,
        request: lethal,
    })
    .expect("again");
    assert!(!s
        .run_tick()
        .iter()
        .any(|output| matches!(output, GameOutput::PlayerDied { .. })));
    s.enqueue(GameInput::Respawn {
        player: p,
        position: spawn(),
    })
    .expect("respawn");
    assert!(s.run_tick().iter().any(|output| matches!(output, GameOutput::PlayerRespawned { position, .. } if *position == spawn())));
    assert_eq!(s.player_position(p), Some(spawn()));
    assert_eq!(s.player_vitals(p).expect("vitals").health(), 20.0);
    assert_eq!(s.player_vitals(p).expect("vitals").food(), 20);
    s.enqueue(GameInput::Respawn {
        player: p,
        position: Vec3::ZERO,
    })
    .expect("duplicate");
    assert!(!s
        .run_tick()
        .iter()
        .any(|output| matches!(output, GameOutput::PlayerRespawned { .. })));
}

#[tokio::test]
async fn survival_owner_fall_requires_real_support_and_water_cancels_distance() {
    let p = player("fall_owner");
    let mut s = shard_with_player(p).await;
    s.enable_survival(Difficulty::Normal, false)
        .expect("enable");
    let floor = BlockPos::new(6, 63, 8);
    assert!(matches!(
        s.apply_block_edit(MutationCause::Test, floor, BlockStateId::new(1)),
        MutationResult::Applied { .. }
    ));
    for (y, on_ground) in [(74.0, false), (70.0, true), (64.0, true)] {
        s.enqueue(GameInput::FallMovement {
            player: p,
            position: Vec3::new(6.0, y, 8.0),
            on_ground,
            sprinting: false,
        })
        .expect("motion");
        let _ = s.run_tick();
    }
    assert_eq!(s.player_vitals(p).expect("vitals").health(), 13.0);
    let water = ferrumc_registry::block_state::block_default_state("water").expect("water");
    assert!(matches!(
        s.apply_block_edit(
            MutationCause::Test,
            BlockPos::new(6, 64, 8),
            BlockStateId::new(water)
        ),
        MutationResult::Applied { .. }
    ));
    for (y, on_ground) in [(74.0, false), (64.0, true)] {
        s.enqueue(GameInput::FallMovement {
            player: p,
            position: Vec3::new(6.0, y, 8.0),
            on_ground,
            sprinting: false,
        })
        .expect("motion");
        let _ = s.run_tick();
    }
    assert_eq!(s.player_vitals(p).expect("vitals").health(), 13.0);
}

#[tokio::test]
async fn survival_owner_tick_starvation_armor_and_creative_policy() {
    let p = player("starvation_owner");
    let mut s = shard_with_player(p).await;
    s.enable_survival(Difficulty::Normal, false)
        .expect("enable");
    s.enqueue(GameInput::RestoreVitals {
        player: p,
        snapshot: VitalsSnapshot::new(20.0, 0, 0.0, 0.0).expect("saved"),
    })
    .expect("restore");
    for _ in 0..80 {
        let _ = s.run_tick();
    }
    assert_eq!(s.player_vitals(p).expect("vitals").health(), 19.0);
    let armored = DamageRequest::new(10.0, 20.0, 8.0)
        .expect("armor")
        .with_bypasses(false, true);
    s.enqueue(GameInput::PlayerDamage {
        player: p,
        request: armored,
    })
    .expect("damage");
    let _ = s.run_tick();
    assert!((s.player_vitals(p).expect("vitals").health() - 16.0).abs() < 0.00001);
    s.enqueue(GameInput::SetGameMode {
        player: p,
        mode: GameMode::Creative,
    })
    .expect("creative");
    s.enqueue(GameInput::PlayerDamage {
        player: p,
        request: DamageRequest::new(100.0, 0.0, 0.0).expect("source"),
    })
    .expect("damage");
    for _ in 0..80 {
        let _ = s.run_tick();
    }
    assert!((s.player_vitals(p).expect("vitals").health() - 16.0).abs() < 0.00001);
}

#[tokio::test]
async fn survival_owner_snapshot_cannot_activate_competing_tick_producer() {
    let p = player("snapshot_vitals");
    let mut s = shard_with_player(p).await;
    s.enable_survival(Difficulty::Normal, true).expect("enable");
    assert!(s.enable_survival(Difficulty::Normal, true).is_err());
    let mut clone = s.clone();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| clone.run_tick())).is_err());
}

#[tokio::test]
async fn survival_owner_attack_uses_actual_fifo_move_prefix_and_cooldown() {
    let p = player("attacker_owner");
    let target = player("target_owner");
    let mut s = shard_with_player(p).await;
    s.enable_survival(Difficulty::Normal, false)
        .expect("enable");
    s.enqueue(GameInput::PlayerJoin {
        player: target,
        position: Vec3::new(8.0, 64.0, 8.0),
    })
    .expect("join");
    let _ = s.run_tick();
    let request = DamageRequest::new(3.0, 0.0, 0.0).expect("source");
    s.enqueue(GameInput::PlayerMove {
        player: target,
        position: Some(Vec3::new(80.0, 64.0, 8.0)),
        yaw: None,
        pitch: None,
    })
    .expect("far move");
    s.enqueue(GameInput::PlayerAttack {
        player: p,
        target,
        request,
    })
    .expect("attack");
    let _ = s.run_tick();
    assert_eq!(s.player_vitals(target).expect("health").health(), 20.0);
    s.enqueue(GameInput::PlayerMove {
        player: target,
        position: Some(Vec3::new(8.0, 64.0, 8.0)),
        yaw: None,
        pitch: None,
    })
    .expect("near move");
    s.enqueue(GameInput::PlayerAttack {
        player: p,
        target,
        request,
    })
    .expect("attack");
    s.enqueue(GameInput::PlayerAttack {
        player: p,
        target,
        request,
    })
    .expect("repeated attack");
    let outputs = s.run_tick();
    assert_eq!(s.player_vitals(target).expect("health").health(), 17.0);
    assert_eq!(
        outputs
            .iter()
            .filter(
                |output| matches!(output,GameOutput::PlayerHurt {player,..} if *player == target)
            )
            .count(),
        1
    );
    assert!(s.player_vitals(p).expect("vitals").exhaustion() > 0.0);
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // one token-release lifecycle across positive terminal paths
async fn survival_owner_mining_witness_releases_air_cancel_creative_and_capacity() {
    let p = player("witness_owner");
    let mut s = shard_with_player(p).await;
    let position = BlockPos::new(8, 65, 8);
    let tool = mining_tool("wooden_pickaxe");
    s.enqueue(GameInput::MiningStart {
        player: p,
        position,
        sequence: 1,
        tool: tool.with_token(101),
    })
    .expect("air");
    let outputs = s.run_tick();
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(
                output,
                GameOutput::MiningReleased {
                    tool_token: 101,
                    ..
                }
            ))
            .count(),
        1
    );
    let _ = place_block(&mut s, p, position, 1, Direction::Up, 1.0, 0.0, 2);
    s.enqueue(GameInput::MiningStart {
        player: p,
        position,
        sequence: 3,
        tool: tool.with_token(102),
    })
    .expect("start");
    assert!(!s
        .run_tick()
        .iter()
        .any(|output| matches!(output, GameOutput::MiningReleased { .. })));
    s.enqueue(GameInput::MiningToolChanged { player: p, tool })
        .expect("same id change");
    assert_eq!(
        s.run_tick()
            .iter()
            .filter(|output| matches!(
                output,
                GameOutput::MiningReleased {
                    tool_token: 102,
                    ..
                }
            ))
            .count(),
        1
    );
    s.enqueue(GameInput::SetGameMode {
        player: p,
        mode: GameMode::Creative,
    })
    .expect("creative");
    s.enqueue(GameInput::MiningStart {
        player: p,
        position,
        sequence: 4,
        tool: tool.with_token(103),
    })
    .expect("creative start");
    assert_eq!(
        s.run_tick()
            .iter()
            .filter(|output| matches!(
                output,
                GameOutput::MiningReleased {
                    tool_token: 103,
                    ..
                }
            ))
            .count(),
        1
    );
    s.enqueue(GameInput::SetGameMode {
        player: p,
        mode: GameMode::Survival,
    })
    .expect("survival");
    let _ = s.run_tick();
    let _ = place_block(&mut s, p, position, 1, Direction::Up, 1.0, 0.0, 5);
    for index in 0..129 {
        let actor = player(&format!("witness-cap-{index}"));
        s.enqueue(GameInput::PlayerJoin {
            player: actor,
            position: Vec3::new(6.0, 64.0, 8.0),
        })
        .expect("join");
        s.enqueue(GameInput::MiningStart {
            player: actor,
            position,
            sequence: index,
            tool: tool.with_token(200 + index as u64),
        })
        .expect("start");
    }
    let outputs = s.run_tick();
    assert_eq!(s.mining_work.len(), 128);
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(
                output,
                GameOutput::MiningReleased {
                    tool_token: 328,
                    ..
                }
            ))
            .count(),
        1
    );
}

async fn tracked_combat_shard() -> (SimShard, PlayerId, PlayerId) {
    let attacker = player("tracked_attacker");
    let victim = player("tracked_victim");
    let mut s = shard_with_player(attacker).await;
    s.enable_survival(Difficulty::Normal, false)
        .expect("survival");
    s.enqueue(GameInput::PlayerJoin {
        player: victim,
        position: Vec3::new(8.0, 64.0, 10.0),
    })
    .expect("victim");
    let _ = s.run_tick();
    for _ in 0..25 {
        let _ = s.run_tick();
    }
    (s, attacker, victim)
}
fn tracked_attack(
    s: &mut SimShard,
    a: PlayerId,
    v: PlayerId,
    request: DamageRequest,
    speed: f32,
    token: u64,
) -> (bool, f32, f32) {
    s.enqueue(GameInput::PlayerAttackTracked {
        player: a,
        target: v,
        request,
        attack_speed: speed,
        token,
    })
    .expect("attack");
    let outputs = s.run_tick();
    outputs
        .into_iter()
        .find_map(|out| match out {
            GameOutput::PlayerAttackResolved {
                accepted,
                damage,
                raw_damage,
                ..
            } => Some((accepted, damage, raw_damage)),
            _ => None,
        })
        .expect("always resolves")
}

#[tokio::test]
async fn tracked_combat_owner_full_strength_replay_and_rapid_hurt_immunity() {
    let (mut s, a, v) = tracked_combat_shard().await;
    let request = DamageRequest::new(4.0, 0.0, 0.0).expect("source");
    assert_eq!(
        tracked_attack(&mut s, a, v, request, 1.6, 1),
        (true, 4.0, 4.0)
    );
    let health = s.player_vitals(v).expect("health").health();
    assert_eq!(
        tracked_attack(&mut s, a, v, request, 1.6, 1),
        (false, 0.0, 0.0)
    );
    assert_eq!(
        tracked_attack(&mut s, a, v, request, 1.6, 2),
        (false, 0.0, 0.0)
    );
    assert_eq!(s.player_vitals(v).expect("unchanged").health(), health);
    assert!(s.player_vitals(a).expect("exhaustion").exhaustion() > 0.0);
}

#[tokio::test]
async fn tracked_combat_owner_no_fixed_gate_and_incremental_receipt_before_armor() {
    let (mut s, a, v) = tracked_combat_shard().await;
    assert_eq!(
        tracked_attack(
            &mut s,
            a,
            v,
            DamageRequest::new(4.0, 0.0, 0.0).expect("source"),
            1.6,
            1
        ),
        (true, 4.0, 4.0)
    );
    let (accepted, damage, raw) = tracked_attack(
        &mut s,
        a,
        v,
        DamageRequest::new(20.0, 0.0, 0.0).expect("source"),
        1.6,
        2,
    );
    let expected = 20.0 * (0.2 + 0.8 * (1.5_f32 / 12.5).powi(2)) - 4.0;
    assert!(accepted);
    assert!((damage - expected).abs() < 1.0e-5);
    assert!((raw - expected).abs() < 1.0e-5);
    // Armor absorption follows strength and hurt immunity; receipt preserves raw.
    let (mut s, a, v) = tracked_combat_shard().await;
    let (accepted, damage, raw) = tracked_attack(
        &mut s,
        a,
        v,
        DamageRequest::new(4.0, 20.0, 8.0).expect("armor"),
        1.6,
        1,
    );
    assert!(accepted);
    assert!((damage - 0.96).abs() < 1.0e-5);
    assert_eq!(raw, 4.0);
}

#[tokio::test]
async fn tracked_combat_owner_weapon_identity_strength_reset_and_same_item_preserves() {
    let (mut s, a, v) = tracked_combat_shard().await;
    s.enqueue(GameInput::MiningToolChanged {
        player: a,
        tool: mining_tool("wooden_sword"),
    })
    .expect("heldchanged");
    let _ = s.run_tick();
    let (_, damage, _) = tracked_attack(
        &mut s,
        a,
        v,
        DamageRequest::new(4.0, 0.0, 0.0).expect("source"),
        1.6,
        1,
    );
    assert!((damage - 4.0 * (0.2 + 0.8 * (1.5_f32 / 12.5).powi(2))).abs() < 1.0e-5);
    for _ in 0..25 {
        let _ = s.run_tick();
    }
    s.enqueue(GameInput::MiningToolChanged {
        player: a,
        tool: mining_tool("wooden_sword"),
    })
    .expect("samekind exactchange");
    let _ = s.run_tick();
    assert_eq!(
        tracked_attack(
            &mut s,
            a,
            v,
            DamageRequest::new(4.0, 0.0, 0.0).expect("source"),
            1.6,
            2
        ),
        (true, 4.0, 4.0)
    );
}

#[tokio::test]
async fn tracked_combat_owner_invalid_speed_creative_dead_and_reach_resolve_without_wear() {
    let (mut s, a, v) = tracked_combat_shard().await;
    let request = DamageRequest::new(4.0, 0.0, 0.0).expect("source");
    for (token, speed) in [(1, f32::NAN), (2, 0.0), (3, 1025.0), (0, 1.6)] {
        assert_eq!(
            tracked_attack(&mut s, a, v, request, speed, token),
            (false, 0.0, 0.0)
        );
    }
    s.enqueue(GameInput::SetGameMode {
        player: v,
        mode: GameMode::Creative,
    })
    .expect("creative");
    assert_eq!(
        tracked_attack(&mut s, a, v, request, 1.6, 4),
        (false, 0.0, 0.0)
    );
    s.enqueue(GameInput::SetGameMode {
        player: v,
        mode: GameMode::Survival,
    })
    .expect("survival");
    s.enqueue(GameInput::PlayerMove {
        player: v,
        position: Some(Vec3::new(30.0, 64.0, 8.0)),
        yaw: None,
        pitch: None,
    })
    .expect("outofreach");
    assert_eq!(
        tracked_attack(&mut s, a, v, request, 1.6, 5),
        (false, 0.0, 0.0)
    );
    s.enqueue(GameInput::RestoreVitals {
        player: a,
        snapshot: VitalsSnapshot::new(0.0, 20, 5.0, 0.0).expect("dead"),
    })
    .expect("deadrestore");
    assert_eq!(
        tracked_attack(&mut s, a, v, request, 1.6, 6),
        (false, 0.0, 0.0)
    );
}
