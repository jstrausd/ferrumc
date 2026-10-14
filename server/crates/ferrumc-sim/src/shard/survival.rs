//! Legacy player health, hunger and death are mutated only by the tick owner.

use super::{is_valid_position, SimShard};
use crate::survival::{SurvivalPlayer, SurvivalRuntime};
use crate::{
    DamageRequest, Difficulty, GameOutput, SurvivalError, VitalsChange, VitalsSnapshot,
    VitalsTickPolicy,
};
use ferrumc_core::{GameMode, PlayerId};
use ferrumc_items::FoodProperties;
use ferrumc_math::{BlockPos, Vec3};
use ferrumc_registry::block_state::{block_metadata, state_id_to_block_name};

impl SimShard {
    /// Selects actual legacy health/hunger gameplay, independent of native DUR.
    /// Default-off preserves old replay contracts. Existing players are seeded
    /// once; restore is a subsequent tick input. Snapshot copies cannot tick.
    pub fn enable_survival(
        &mut self,
        difficulty: Difficulty,
        natural_regeneration: bool,
    ) -> Result<(), SurvivalError> {
        if self.player_ownership.is_some() {
            return Err(SurvivalError::NativeOwnership);
        }
        if self.survival.is_some() {
            return Err(SurvivalError::AlreadyInitialized);
        }
        self.survival = Some(SurvivalRuntime {
            live: true,
            policy: VitalsTickPolicy::new(difficulty, natural_regeneration),
        });
        for player in self.players.values_mut() {
            player.survival = Some(SurvivalPlayer::new(player.position));
        }
        Ok(())
    }

    /// Borrows all nine exact current health/hunger/timer save fields.
    pub fn player_vitals(&self, player: PlayerId) -> Option<VitalsSnapshot> {
        Some(
            self.players
                .get(&player)?
                .survival
                .as_ref()?
                .vitals
                .snapshot(),
        )
    }

    /// Probes current authoritative hunger; this is not a consume reservation.
    /// App must pin its complete prepared inventory mutation until the matching
    /// `FoodConsumptionResolved` result, and commit only an accepted result.
    pub fn player_can_eat(&self, player: PlayerId, food: FoodProperties) -> bool {
        self.survival.as_ref().is_some_and(|runtime| runtime.live)
            && self.players.get(&player).is_some_and(|actor| {
                actor.game_mode != GameMode::Spectator
                    && actor.survival.as_ref().is_some_and(|state| {
                        state
                            .vitals
                            .can_eat(food, actor.game_mode == GameMode::Creative)
                    })
            })
    }

    pub(super) fn restore_vitals(
        &mut self,
        player: PlayerId,
        snapshot: VitalsSnapshot,
        outputs: &mut Vec<GameOutput>,
    ) {
        let Some(actor) = self.players.get_mut(&player) else {
            return;
        };
        let Some(state) = actor.survival.as_mut() else {
            return;
        };
        let was_alive = !state.vitals.is_dead();
        if state.vitals.restore(snapshot).is_err() {
            return;
        }
        let died = was_alive && state.vitals.is_dead();
        if died {
            state.death_drops_admitted = false;
        }
        state.movement_position = actor.position;
        state.fall_distance = 0.0;
        outputs.push(GameOutput::PlayerVitalsChanged { player, snapshot });
        if died {
            outputs.push(GameOutput::PlayerDied { player });
            self.cancel_mining(outputs, player);
        }
    }

    pub(super) fn consume_food(
        &mut self,
        player: PlayerId,
        food: FoodProperties,
        token: u64,
        outputs: &mut Vec<GameOutput>,
    ) {
        let mut accepted = false;
        if token != 0 {
            if let Some(actor) = self.players.get_mut(&player) {
                if let Some(state) = actor.survival.as_mut() {
                    if token > state.last_food_token {
                        state.last_food_token = token;
                        if actor.game_mode != GameMode::Spectator {
                            if let Ok(change) = state
                                .vitals
                                .apply_food(food, actor.game_mode == GameMode::Creative)
                            {
                                accepted = true;
                                publish_change(outputs, player, state.vitals.snapshot(), change);
                            }
                        }
                    }
                }
            }
        }
        outputs.push(GameOutput::FoodConsumptionResolved {
            player,
            token,
            accepted,
        });
    }

    pub(super) fn damage_player(
        &mut self,
        player: PlayerId,
        request: DamageRequest,
        outputs: &mut Vec<GameOutput>,
    ) -> f32 {
        let Some(actor) = self.players.get_mut(&player) else {
            return 0.0;
        };
        let Some(state) = actor.survival.as_mut() else {
            return 0.0;
        };
        let request = if matches!(actor.game_mode, GameMode::Creative | GameMode::Spectator) {
            request.with_invulnerable(true)
        } else {
            request
        };
        let Ok(change) = state.vitals.hurt(request) else {
            return 0.0;
        };
        if change.died() {
            state.death_drops_admitted = false;
        }
        publish_change(outputs, player, state.vitals.snapshot(), change);
        if change.died() {
            self.cancel_mining(outputs, player);
        }
        change.damaged()
    }

    pub(super) fn attack_player(
        &mut self,
        player: PlayerId,
        target: PlayerId,
        request: DamageRequest,
        outputs: &mut Vec<GameOutput>,
    ) {
        if player == target {
            return;
        }
        let Some(attacker) = self.players.get(&player) else {
            return;
        };
        let Some(state) = attacker.survival.as_ref() else {
            return;
        };
        if !attacker.is_alive()
            || attacker.game_mode == GameMode::Spectator
            || state.attack_cooldown != 0
        {
            return;
        }
        let Some(victim) = self.players.get(&target) else {
            return;
        };
        let exhausts = attacker.game_mode != GameMode::Creative;
        let delta = attacker.position - victim.position;
        if delta.length_squared() > 3.0 * 3.0 {
            return;
        }
        if let Some(state) = self
            .players
            .get_mut(&player)
            .and_then(|actor| actor.survival.as_mut())
        {
            state.attack_cooldown = 10;
            if exhausts {
                if let Err(error) = state.vitals.add_exhaustion(0.1) {
                    panic!("finite attack exhaustion failed: {error}");
                }
            }
        }
        self.damage_player(target, request, outputs);
    }

    pub(super) fn attack_player_tracked(
        &mut self,
        player: PlayerId,
        target: PlayerId,
        request: DamageRequest,
        attack_speed: f32,
        token: u64,
        outputs: &mut Vec<GameOutput>,
    ) {
        let mut damage = 0.0;
        let mut raw_damage = 0.0;
        let attempted = (|| {
            let attacker = self.players.get_mut(&player)?;
            let state = attacker.survival.as_mut()?;
            if token == 0 || token <= state.last_attack_token {
                return None;
            }
            state.last_attack_token = token;
            if state.vitals.is_dead()
                || attacker.game_mode == GameMode::Spectator
                || player == target
                || !attack_speed.is_finite()
                || !(0.1..=1024.0).contains(&attack_speed)
            {
                return None;
            }
            let strength = ((f32::from(state.attack_strength_ticks) + 0.5) / (20.0 / attack_speed))
                .clamp(0.0, 1.0);
            let source = attacker.position;
            let exhausts = attacker.game_mode != GameMode::Creative;
            let request = request
                .scaled_amount(0.2 + strength * strength * 0.8)
                .ok()?;
            let victim = self.players.get(&target)?;
            let snapshot = victim.survival.as_ref()?.vitals.snapshot();
            if !victim.is_alive() || (source - victim.position).length_squared() > 9.0 {
                return None;
            }
            let incoming = request.incremental_amount(snapshot);
            let committed = self.damage_player(target, request, outputs);
            if committed <= 0.0 {
                return None;
            }
            let state = self.players.get_mut(&player)?.survival.as_mut()?;
            state.attack_strength_ticks = 0;
            if exhausts {
                if let Err(error) = state.vitals.add_exhaustion(0.1) {
                    panic!("validated finite attack exhaustion failed: {error}");
                }
            }
            Some((committed, incoming))
        })();
        if let Some((committed, incoming)) = attempted {
            damage = committed;
            raw_damage = incoming;
        }
        outputs.push(GameOutput::PlayerAttackResolved {
            player,
            target,
            token,
            accepted: damage > 0.0,
            damage,
            raw_damage,
        });
    }

    pub(super) fn update_attack_item(&mut self, player: PlayerId, held: Option<i32>) {
        if let Some(state) = self
            .players
            .get_mut(&player)
            .and_then(|actor| actor.survival.as_mut())
        {
            if state.last_attack_item != held {
                state.attack_strength_ticks = 0;
                state.last_attack_item = held;
            }
        }
    }

    pub(super) fn fall_movement(
        &mut self,
        player: PlayerId,
        position: Vec3,
        on_ground: bool,
        sprinting: bool,
        outputs: &mut Vec<GameOutput>,
    ) {
        if !is_valid_position(position) {
            return;
        }
        let feet = BlockPos::new(
            position.x.floor() as i32,
            (position.y - 0.2).floor() as i32,
            position.z.floor() as i32,
        );
        let support = self
            .chunks
            .get(feet.to_chunk_pos())
            .and_then(|chunk| chunk.get_block(feet))
            .and_then(|state| state_id_to_block_name(state.as_u32()))
            .and_then(block_metadata)
            .is_some_and(|metadata| metadata.is_solid_cube);
        let in_water = [position.y, position.y + 1.5].into_iter().any(|y| {
            let cell = BlockPos::new(
                position.x.floor() as i32,
                y.floor() as i32,
                position.z.floor() as i32,
            );
            self.chunks
                .get(cell.to_chunk_pos())
                .and_then(|chunk| chunk.get_block(cell))
                .and_then(|state| state_id_to_block_name(state.as_u32()))
                == Some("water")
        });
        let Some(actor) = self.players.get_mut(&player) else {
            return;
        };
        if !actor.is_alive() {
            return;
        }
        let Some(state) = actor.survival.as_mut() else {
            return;
        };
        let previous = state.movement_position;
        state.movement_position = position;
        let invulnerable = matches!(actor.game_mode, GameMode::Creative | GameMode::Spectator);
        if invulnerable || in_water {
            state.fall_distance = 0.0;
            return;
        }
        let delta = position - previous;
        // Legacy movement permits teleports; no large jump manufactures fall or
        // hunger damage. This is a coarse motion consumer, not anti-cheat parity.
        if delta.length_squared() > 64.0 * 64.0 {
            state.fall_distance = 0.0;
            return;
        }
        if delta.y < 0.0 {
            state.fall_distance += -delta.y;
        }
        let horizontal = delta.x.hypot(delta.z);
        if sprinting && on_ground && support {
            if let Err(error) = state.vitals.add_exhaustion((horizontal * 0.1) as f32) {
                panic!("bounded sprint exhaustion failed: {error}");
            }
        }
        if !on_ground || !support {
            return;
        }
        let distance = std::mem::take(&mut state.fall_distance);
        let Ok(amount) = crate::fall_damage(distance, 1.0) else {
            return;
        };
        if amount <= 0.0 {
            return;
        }
        let Ok(request) = DamageRequest::new(amount, 0.0, 0.0) else {
            return;
        };
        self.damage_player(player, request.with_bypasses(true, false), outputs);
    }

    pub(super) fn respawn_player(
        &mut self,
        player: PlayerId,
        position: Vec3,
        outputs: &mut Vec<GameOutput>,
    ) -> bool {
        if !is_valid_position(position) {
            return false;
        }
        let Some(actor) = self.players.get_mut(&player) else {
            return false;
        };
        let Some(state) = actor.survival.as_mut() else {
            return false;
        };
        if !state.vitals.is_dead() {
            return false;
        }
        state.vitals.respawn();
        state.movement_position = position;
        state.fall_distance = 0.0;
        state.attack_cooldown = 0;
        state.attack_strength_ticks = 0;
        state.death_drops_admitted = false;
        actor.position = position;
        outputs.push(GameOutput::PlayerRespawned { player, position });
        outputs.push(GameOutput::PlayerVitalsChanged {
            player,
            snapshot: state.vitals.snapshot(),
        });
        true
    }

    pub(super) fn drive_survival(&mut self, outputs: &mut Vec<GameOutput>) {
        let Some(runtime) = self.survival.as_ref() else {
            return;
        };
        assert!(runtime.live, "Survival snapshot has no tick authority");
        for (&player, actor) in &mut self.players {
            let Some(state) = actor.survival.as_mut() else {
                continue;
            };
            state.attack_cooldown = state.attack_cooldown.saturating_sub(1);
            state.attack_strength_ticks = state.attack_strength_ticks.saturating_add(1);
            let policy = runtime.policy.with_invulnerable(matches!(
                actor.game_mode,
                GameMode::Creative | GameMode::Spectator
            ));
            let change = state.vitals.tick(policy);
            if change.died() {
                state.death_drops_admitted = false;
            }
            publish_change(outputs, player, state.vitals.snapshot(), change);
        }
    }
}

fn publish_change(
    outputs: &mut Vec<GameOutput>,
    player: PlayerId,
    snapshot: VitalsSnapshot,
    change: VitalsChange,
) {
    if change.health_changed() || change.food_changed() {
        outputs.push(GameOutput::PlayerVitalsChanged { player, snapshot });
    }
    if change.damaged() > 0.0 {
        outputs.push(GameOutput::PlayerHurt {
            player,
            amount: change.damaged(),
        });
    }
    if change.died() {
        outputs.push(GameOutput::PlayerDied { player });
    }
}
