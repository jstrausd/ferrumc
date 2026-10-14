//! Pure, fixed-size player vitals stepped at an explicit 20 TPS.
//!
//! Baseline rules were reviewed against Paper 1.21.8 build 60's complete
//! `FoodData`, `ServerPlayer.tickRegeneration` and `LivingEntity` damage/fall
//! methods. This module owns no inventory, clock, events, RNG or persistence
//! codec. Consumers own source tags, creative authority, armor durability,
//! consume timers, food effects, death handling and packet publication.

use ferrumc_items::{damage_after_armor, FoodProperties};
use std::fmt;

/// A rejected vitals input; refusal leaves the authoritative state unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VitalsError {
    /// Health must be finite and between zero and twenty.
    InvalidHealth,
    /// Hunger must be between zero and twenty.
    InvalidFood,
    /// Saturation must be finite and between zero and the current hunger.
    InvalidSaturation,
    /// Exhaustion must be finite and nonnegative (snapshots also cap it at forty).
    InvalidExhaustion,
    /// A saved timer or last-damage value exceeds its supported range.
    InvalidTimers,
    /// Damage must be finite and nonnegative; armor/toughness must fit Items' bounds.
    InvalidDamage,
    /// Healing must be finite and nonnegative.
    InvalidHealing,
    /// Fall distance/multiplier must be finite and nonnegative with a finite f32 result.
    InvalidFall,
    /// Full hunger refuses ordinary food, and dead players cannot consume food.
    CannotEat,
    /// The resolved food has invalid nutrition or saturation values.
    InvalidFoodProperties,
}

impl fmt::Display for VitalsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidHealth => "invalid player health",
            Self::InvalidFood => "invalid player hunger",
            Self::InvalidSaturation => "invalid player saturation",
            Self::InvalidExhaustion => "invalid player exhaustion",
            Self::InvalidTimers => "invalid player vitals timers",
            Self::InvalidDamage => "invalid player damage or armor",
            Self::InvalidHealing => "invalid player healing",
            Self::InvalidFall => "invalid player fall input",
            Self::CannotEat => "player cannot eat this food",
            Self::InvalidFoodProperties => "invalid resolved food properties",
        };
        f.write_str(message)
    }
}

impl std::error::Error for VitalsError {}

/// The two explicitly supported baseline difficulties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    /// Exhaustion does not lower hunger; natural recovery has extra 10/20 tick phases.
    Peaceful,
    /// Starvation applies while health is greater than one.
    Normal,
}

/// World/game-mode authority supplied for one simulation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VitalsTickPolicy {
    difficulty: Difficulty,
    natural_regeneration: bool,
    invulnerable: bool,
}

impl VitalsTickPolicy {
    /// Select difficulty and the natural-regeneration gamerule explicitly.
    pub const fn new(difficulty: Difficulty, natural_regeneration: bool) -> Self {
        Self {
            difficulty,
            natural_regeneration,
            invulnerable: false,
        }
    }

    /// Suppress starvation damage and regeneration exhaustion for invulnerable players.
    /// Existing exhaustion still drains, as in `FoodData.tick`.
    #[must_use]
    pub const fn with_invulnerable(mut self, invulnerable: bool) -> Self {
        self.invulnerable = invulnerable;
        self
    }
}

/// A checked source-policy input, with ordinary unenchanted armor attributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageRequest {
    amount: f32,
    armor: f32,
    toughness: f32,
    bypass_armor: bool,
    bypass_cooldown: bool,
    invulnerable: bool,
    exhaustion: f32,
}

impl DamageRequest {
    /// Validate damage and armor; the default uses both armor and hit cooldown.
    /// Source-specific exhaustion defaults to zero and must be supplied explicitly.
    pub fn new(amount: f32, armor: f32, toughness: f32) -> Result<Self, VitalsError> {
        damage_after_armor(amount, armor, toughness).map_err(|_| VitalsError::InvalidDamage)?;
        Ok(Self {
            amount,
            armor,
            toughness,
            bypass_armor: false,
            bypass_cooldown: false,
            invulnerable: false,
            exhaustion: 0.0,
        })
    }

    pub(crate) fn scaled_amount(mut self, factor: f32) -> Result<Self, VitalsError> {
        if !factor.is_finite() || !(0.0..=1.0).contains(&factor) {
            return Err(VitalsError::InvalidDamage);
        }
        self.amount *= factor;
        Ok(self)
    }

    pub(crate) fn incremental_amount(self, snapshot: VitalsSnapshot) -> f32 {
        if snapshot.invulnerable_ticks() > 10 && !self.bypass_cooldown {
            (self.amount - snapshot.last_hurt()).max(0.0)
        } else {
            self.amount
        }
    }

    /// Supply the authoritative source's armor and cooldown bypass flags.
    #[must_use]
    pub const fn with_bypasses(mut self, armor: bool, cooldown: bool) -> Self {
        self.bypass_armor = armor;
        self.bypass_cooldown = cooldown;
        self
    }

    /// Refuse damage when the caller's source/game-mode policy makes it invulnerable.
    #[must_use]
    pub const fn with_invulnerable(mut self, invulnerable: bool) -> Self {
        self.invulnerable = invulnerable;
        self
    }

    /// Supply finite nonnegative source exhaustion, added only for effective damage.
    pub fn with_exhaustion(mut self, exhaustion: f32) -> Result<Self, VitalsError> {
        if !finite_nonnegative(exhaustion) {
            return Err(VitalsError::InvalidExhaustion);
        }
        self.exhaustion = exhaustion;
        Ok(self)
    }
}

/// Structurally bounded snapshot; all fields are scalars and constructors validate.
/// It includes transient timers for deterministic native restores, unlike Vanilla NBT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VitalsSnapshot {
    health: f32,
    food: u8,
    saturation: f32,
    exhaustion: f32,
    food_tick_timer: u8,
    invulnerable_ticks: u8,
    hurt_ticks: u8,
    last_hurt: f32,
    peaceful_phase: u8,
}

impl VitalsSnapshot {
    /// Validate scalar vitals with all timers initially zero.
    pub fn new(
        health: f32,
        food: u8,
        saturation: f32,
        exhaustion: f32,
    ) -> Result<Self, VitalsError> {
        let snapshot = Self {
            health,
            food,
            saturation,
            exhaustion,
            food_tick_timer: 0,
            invulnerable_ticks: 0,
            hurt_ticks: 0,
            last_hurt: 0.0,
            peaceful_phase: 0,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Restore the shared food timer (0..79), cooldown (0..20), hurt animation
    /// (0..10), last raw hit (finite/nonnegative), and tick phase (0..19).
    pub fn with_timers(
        mut self,
        food_tick_timer: u8,
        invulnerable_ticks: u8,
        hurt_ticks: u8,
        last_hurt: f32,
        peaceful_phase: u8,
    ) -> Result<Self, VitalsError> {
        self.food_tick_timer = food_tick_timer;
        self.invulnerable_ticks = invulnerable_ticks;
        self.hurt_ticks = hurt_ticks;
        self.last_hurt = last_hurt;
        self.peaceful_phase = peaceful_phase;
        self.validate()?;
        Ok(self)
    }

    fn validate(self) -> Result<(), VitalsError> {
        if !finite_nonnegative(self.health) || self.health > 20.0 {
            return Err(VitalsError::InvalidHealth);
        }
        if self.food > 20 {
            return Err(VitalsError::InvalidFood);
        }
        if !finite_nonnegative(self.saturation) || self.saturation > f32::from(self.food) {
            return Err(VitalsError::InvalidSaturation);
        }
        if !finite_nonnegative(self.exhaustion) || self.exhaustion > 40.0 {
            return Err(VitalsError::InvalidExhaustion);
        }
        if self.food_tick_timer > 79
            || self.invulnerable_ticks > 20
            || self.hurt_ticks > 10
            || !finite_nonnegative(self.last_hurt)
            || self.peaceful_phase > 19
        {
            return Err(VitalsError::InvalidTimers);
        }
        Ok(())
    }

    /// Current health in half-heart units.
    pub const fn health(self) -> f32 {
        self.health
    }
    /// Current hunger points.
    pub const fn food(self) -> u8 {
        self.food
    }
    /// Current saturation points.
    pub const fn saturation(self) -> f32 {
        self.saturation
    }
    /// Current exhaustion points.
    pub const fn exhaustion(self) -> f32 {
        self.exhaustion
    }
    /// Shared regeneration/starvation timer.
    pub const fn food_tick_timer(self) -> u8 {
        self.food_tick_timer
    }
    /// Remaining hit cooldown ticks.
    pub const fn invulnerable_ticks(self) -> u8 {
        self.invulnerable_ticks
    }
    /// Remaining hurt animation ticks.
    pub const fn hurt_ticks(self) -> u8 {
        self.hurt_ticks
    }
    /// Raw amount of the last accepted hit, before armor.
    pub const fn last_hurt(self) -> f32 {
        self.last_hurt
    }
    /// Completed ticks modulo twenty; advances even in Normal difficulty.
    pub const fn peaceful_phase(self) -> u8 {
        self.peaceful_phase
    }
}

/// Fixed-size outcome with effective health changes and publication predicates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VitalsChange {
    flags: u8,
    damaged: f32,
    healed: f32,
}

impl VitalsChange {
    fn between(before: VitalsSnapshot, after: VitalsSnapshot, damaged: f32, healed: f32) -> Self {
        // Publication detects exact scalar changes, including sub-heart fractions.
        let flags = u8::from(before != after)
            | (u8::from(before.health.to_bits() != after.health.to_bits()) << 1)
            | (u8::from(
                before.food != after.food
                    || before.saturation.to_bits() != after.saturation.to_bits(),
            ) << 2)
            | (u8::from(before.health > 0.0 && after.health == 0.0) << 3);
        Self {
            flags,
            damaged,
            healed,
        }
    }
    /// Any scalar or transient timer changed (suitable for native save dirtiness).
    pub const fn changed(self) -> bool {
        self.flags & 1 != 0
    }
    /// The health scalar changed.
    pub const fn health_changed(self) -> bool {
        self.flags & 2 != 0
    }
    /// Hunger or saturation changed (exhaustion alone does not require a health packet).
    pub const fn food_changed(self) -> bool {
        self.flags & 4 != 0
    }
    /// Health actually lost, excluding overkill damage.
    pub const fn damaged(self) -> f32 {
        self.damaged
    }
    /// Health actually regained, excluding the amount beyond maximum health.
    pub const fn healed(self) -> f32 {
        self.healed
    }
    /// This operation transitioned a living player to zero health.
    pub const fn died(self) -> bool {
        self.flags & 8 != 0
    }
}

/// Exclusive authoritative health/hunger owner, with no dynamic allocation.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerVitals {
    state: VitalsSnapshot,
}

impl Default for PlayerVitals {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerVitals {
    /// Start a fresh player at health 20, food 20, saturation 5 and exhaustion 0.
    pub const fn new() -> Self {
        Self {
            state: VitalsSnapshot {
                health: 20.0,
                food: 20,
                saturation: 5.0,
                exhaustion: 0.0,
                food_tick_timer: 0,
                invulnerable_ticks: 0,
                hurt_ticks: 0,
                last_hurt: 0.0,
                peaceful_phase: 0,
            },
        }
    }

    /// Admit a complete validated snapshot, preserving every timer.
    pub fn from_snapshot(snapshot: VitalsSnapshot) -> Result<Self, VitalsError> {
        snapshot.validate()?;
        Ok(Self { state: snapshot })
    }

    /// Atomically replace all vitals; invalid input preserves the current state.
    pub fn restore(&mut self, snapshot: VitalsSnapshot) -> Result<(), VitalsError> {
        snapshot.validate()?;
        self.state = snapshot;
        Ok(())
    }

    /// Capture all scalar state without allocation.
    pub const fn snapshot(&self) -> VitalsSnapshot {
        self.state
    }
    /// Current health in half-heart units.
    pub const fn health(&self) -> f32 {
        self.state.health
    }
    /// Current hunger points.
    pub const fn food(&self) -> u8 {
        self.state.food
    }
    /// Current saturation points.
    pub const fn saturation(&self) -> f32 {
        self.state.saturation
    }
    /// Current exhaustion points.
    pub const fn exhaustion(&self) -> f32 {
        self.state.exhaustion
    }
    /// Whether health is zero; ordinary healing cannot resurrect a dead player.
    pub fn is_dead(&self) -> bool {
        self.state.health == 0.0
    }
    /// Remaining hurt animation ticks.
    pub const fn hurt_ticks(&self) -> u8 {
        self.state.hurt_ticks
    }
    /// Remaining hit cooldown ticks.
    pub const fn invulnerable_ticks(&self) -> u8 {
        self.state.invulnerable_ticks
    }

    /// Add finite nonnegative exhaustion, capped at forty. Consumers must omit
    /// movement/effect exhaustion when their game-mode authority exempts it.
    pub fn add_exhaustion(&mut self, amount: f32) -> Result<(), VitalsError> {
        if !finite_nonnegative(amount) {
            return Err(VitalsError::InvalidExhaustion);
        }
        self.add_exhaustion_validated(amount);
        Ok(())
    }

    fn add_exhaustion_validated(&mut self, amount: f32) {
        self.state.exhaustion += amount.min(40.0 - self.state.exhaustion);
    }

    /// Heal a living player up to twenty; zero health stays dead until respawn.
    pub fn heal(&mut self, amount: f32) -> Result<VitalsChange, VitalsError> {
        if !finite_nonnegative(amount) {
            return Err(VitalsError::InvalidHealing);
        }
        let before = self.state;
        let healed = self.heal_validated(amount);
        Ok(VitalsChange::between(before, self.state, 0.0, healed))
    }

    fn heal_validated(&mut self, amount: f32) -> f32 {
        if self.is_dead() {
            return 0.0;
        }
        let before = self.state.health;
        self.state.health += amount.min(20.0 - self.state.health);
        self.state.health - before
    }

    /// Check hunger eligibility; the bool is caller-supplied creative/invulnerable authority.
    pub fn can_eat(&self, food: FoodProperties, invulnerable: bool) -> bool {
        !self.is_dead() && (invulnerable || food.always_edible() || self.state.food < 20)
    }

    /// Apply resolved nutrition/saturation after the consumer's checked use timer
    /// and inventory transaction. This does not decrement stacks or execute effects.
    pub fn apply_food(
        &mut self,
        food: FoodProperties,
        invulnerable: bool,
    ) -> Result<VitalsChange, VitalsError> {
        if food.nutrition() < 0 || !finite_nonnegative(food.saturation()) {
            return Err(VitalsError::InvalidFoodProperties);
        }
        if !self.can_eat(food, invulnerable) {
            return Err(VitalsError::CannotEat);
        }
        let before = self.state;
        self.state.food = (i64::from(self.state.food) + i64::from(food.nutrition())).min(20) as u8;
        self.state.saturation += food
            .saturation()
            .min(f32::from(self.state.food) - self.state.saturation);
        Ok(VitalsChange::between(before, self.state, 0.0, 0.0))
    }

    /// Apply source policy, then incremental hit cooldown reduction, then Items'
    /// ordinary armor absorption. Enchantments, effects and absorption hearts
    /// require a separate consumer; no armor item is mutated here.
    pub fn hurt(&mut self, request: DamageRequest) -> Result<VitalsChange, VitalsError> {
        let incremental = if self.state.invulnerable_ticks > 10 && !request.bypass_cooldown {
            (request.amount - self.state.last_hurt).max(0.0)
        } else {
            request.amount
        };
        let effective = if request.bypass_armor {
            incremental
        } else {
            damage_after_armor(incremental, request.armor, request.toughness)
                .map_err(|_| VitalsError::InvalidDamage)?
        };
        Ok(self.hurt_validated(request, effective))
    }

    fn hurt_validated(&mut self, request: DamageRequest, effective: f32) -> VitalsChange {
        let before = self.state;
        if !request.invulnerable && !self.is_dead() && effective > 0.0 {
            let in_cooldown = self.state.invulnerable_ticks > 10 && !request.bypass_cooldown;
            self.state.health = (self.state.health - effective).max(0.0);
            self.state.last_hurt = request.amount;
            if !in_cooldown {
                self.state.invulnerable_ticks = 20;
                self.state.hurt_ticks = 10;
            }
            self.add_exhaustion_validated(request.exhaustion);
        }
        VitalsChange::between(before, self.state, before.health - self.state.health, 0.0)
    }

    /// Advance exactly one 20 TPS tick: decrement hurt/cooldown timers, run
    /// Peaceful recovery, drain exhaustion once, then run the shared food timer.
    /// Dead players only advance timers/phase and cannot regenerate or starve.
    pub fn tick(&mut self, policy: VitalsTickPolicy) -> VitalsChange {
        let before = self.state;
        let mut healed = 0.0;
        let mut damaged = 0.0;
        self.state.hurt_ticks = self.state.hurt_ticks.saturating_sub(1);
        self.state.invulnerable_ticks = self.state.invulnerable_ticks.saturating_sub(1);
        self.state.peaceful_phase = (self.state.peaceful_phase + 1) % 20;
        if self.is_dead() {
            self.state.food_tick_timer = 0;
            return VitalsChange::between(before, self.state, 0.0, 0.0);
        }

        if policy.difficulty == Difficulty::Peaceful && policy.natural_regeneration {
            if self.state.peaceful_phase == 0 || self.state.peaceful_phase == 10 {
                self.state.food = self.state.food.saturating_add(1).min(20);
            }
            if self.state.peaceful_phase == 0 {
                healed += self.heal_validated(1.0);
                self.state.saturation =
                    (self.state.saturation + 1.0).min(f32::from(self.state.food));
            }
        }

        // A strict threshold and a single drain preserve the reference cadence.
        if self.state.exhaustion > 4.0 {
            self.state.exhaustion -= 4.0;
            if self.state.saturation > 0.0 {
                self.state.saturation = (self.state.saturation - 1.0).max(0.0);
            } else if policy.difficulty != Difficulty::Peaceful {
                self.state.food = self.state.food.saturating_sub(1);
            }
        }

        let hurt = self.state.health < 20.0;
        if policy.natural_regeneration
            && self.state.saturation > 0.0
            && hurt
            && self.state.food == 20
        {
            self.state.food_tick_timer += 1;
            if self.state.food_tick_timer >= 10 {
                let gain = self.state.saturation.min(6.0);
                healed += self.heal_validated(gain / 6.0);
                if !policy.invulnerable {
                    self.add_exhaustion_validated(gain);
                }
                self.state.food_tick_timer = 0;
            }
        } else if policy.natural_regeneration && self.state.food >= 18 && hurt {
            self.state.food_tick_timer += 1;
            if self.state.food_tick_timer >= 80 {
                healed += self.heal_validated(1.0);
                if !policy.invulnerable {
                    self.add_exhaustion_validated(6.0);
                }
                self.state.food_tick_timer = 0;
            }
        } else if self.state.food == 0 {
            self.state.food_tick_timer += 1;
            if self.state.food_tick_timer >= 80 {
                if self.state.health > 10.0
                    || (policy.difficulty == Difficulty::Normal && self.state.health > 1.0)
                {
                    let request = DamageRequest {
                        amount: 1.0,
                        armor: 0.0,
                        toughness: 0.0,
                        bypass_armor: true,
                        bypass_cooldown: false,
                        invulnerable: policy.invulnerable,
                        exhaustion: 0.0,
                    };
                    let effective = if self.state.invulnerable_ticks > 10 {
                        (1.0 - self.state.last_hurt).max(0.0)
                    } else {
                        1.0
                    };
                    damaged += self.hurt_validated(request, effective).damaged();
                }
                self.state.food_tick_timer = 0;
            }
        } else {
            self.state.food_tick_timer = 0;
        }
        VitalsChange::between(before, self.state, damaged, healed)
    }

    /// Reset health, hunger and every transient timer to fresh-player defaults.
    /// Inventory, location, respawn authority and events belong to the consumer.
    pub fn respawn(&mut self) {
        *self = Self::new();
    }
}

/// Calculate ordinary fall damage using default safe distance three and the
/// 1.21.8 floor/epsilon rule. Consumers supply flying/gamerule/source authority
/// and submit the returned amount with an armor bypass, zero source exhaustion.
pub fn fall_damage(distance: f64, multiplier: f32) -> Result<f32, VitalsError> {
    if !distance.is_finite() || distance < 0.0 || !finite_nonnegative(multiplier) {
        return Err(VitalsError::InvalidFall);
    }
    let damage = ((distance + 1.0e-6 - 3.0) * f64::from(multiplier))
        .floor()
        .max(0.0);
    if !damage.is_finite() || damage > f64::from(f32::MAX) {
        return Err(VitalsError::InvalidFall);
    }
    Ok(damage as f32)
}

fn finite_nonnegative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

#[cfg(test)]
// Golden scalar values deliberately check exact boundaries and cadence.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use ferrumc_items::{builtin_food, ItemId};

    const NORMAL: VitalsTickPolicy = VitalsTickPolicy::new(Difficulty::Normal, true);
    const NO_REGEN: VitalsTickPolicy = VitalsTickPolicy::new(Difficulty::Normal, false);
    const PEACEFUL: VitalsTickPolicy = VitalsTickPolicy::new(Difficulty::Peaceful, true);

    fn vitals(health: f32, food: u8, saturation: f32, exhaustion: f32) -> PlayerVitals {
        PlayerVitals::from_snapshot(
            VitalsSnapshot::new(health, food, saturation, exhaustion).unwrap(),
        )
        .unwrap()
    }

    fn food(name: &str) -> FoodProperties {
        builtin_food(ItemId::from_name(name).unwrap()).unwrap()
    }

    fn hit(amount: f32) -> DamageRequest {
        DamageRequest::new(amount, 0.0, 0.0).unwrap()
    }

    fn advance(state: &mut PlayerVitals, count: u16, policy: VitalsTickPolicy) {
        for _ in 0..count {
            state.tick(policy);
        }
    }

    #[test]
    fn fresh_defaults_and_respawn_reset_every_saved_field() {
        let fresh = PlayerVitals::new();
        assert_eq!(
            (
                fresh.health(),
                fresh.food(),
                fresh.saturation(),
                fresh.exhaustion()
            ),
            (20.0, 20, 5.0, 0.0)
        );
        let mut player = vitals(2.0, 1, 0.0, 39.0);
        player.tick(NO_REGEN);
        assert!(player.hurt(hit(100.0)).unwrap().died());
        player.respawn();
        assert_eq!(player, fresh);
    }

    #[test]
    fn snapshot_refuses_nonfinite_and_out_of_range_values() {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
            assert_eq!(
                VitalsSnapshot::new(invalid, 20, 5.0, 0.0),
                Err(VitalsError::InvalidHealth)
            );
            assert_eq!(
                VitalsSnapshot::new(20.0, 20, invalid, 0.0),
                Err(VitalsError::InvalidSaturation)
            );
            assert_eq!(
                VitalsSnapshot::new(20.0, 20, 5.0, invalid),
                Err(VitalsError::InvalidExhaustion)
            );
        }
        assert_eq!(
            VitalsSnapshot::new(20.01, 20, 5.0, 0.0),
            Err(VitalsError::InvalidHealth)
        );
        assert_eq!(
            VitalsSnapshot::new(20.0, 21, 5.0, 0.0),
            Err(VitalsError::InvalidFood)
        );
        assert_eq!(
            VitalsSnapshot::new(20.0, 3, 3.1, 0.0),
            Err(VitalsError::InvalidSaturation)
        );
        assert_eq!(
            VitalsSnapshot::new(20.0, 20, 5.0, 40.1),
            Err(VitalsError::InvalidExhaustion)
        );
        let valid = VitalsSnapshot::new(20.0, 20, 20.0, 40.0).unwrap();
        assert!(valid.with_timers(79, 20, 10, f32::MAX, 19).is_ok());
        for timers in [
            (80, 20, 10, 0.0, 19),
            (79, 21, 10, 0.0, 19),
            (79, 20, 11, 0.0, 19),
            (79, 20, 10, f32::NAN, 19),
            (79, 20, 10, -1.0, 19),
            (79, 20, 10, 0.0, 20),
        ] {
            assert_eq!(
                valid.with_timers(timers.0, timers.1, timers.2, timers.3, timers.4),
                Err(VitalsError::InvalidTimers)
            );
        }
    }

    #[test]
    fn invalid_restore_is_atomic_and_capture_is_fixed_size() {
        let mut player = PlayerVitals::new();
        let before = player.snapshot();
        let mut invalid = before;
        invalid.health = f32::NAN;
        assert_eq!(player.restore(invalid), Err(VitalsError::InvalidHealth));
        assert_eq!(player.snapshot(), before);
        assert!(PlayerVitals::from_snapshot(invalid).is_err());
        assert!(std::mem::size_of::<VitalsSnapshot>() <= 32);
        assert_eq!(
            std::mem::size_of::<PlayerVitals>(),
            std::mem::size_of::<VitalsSnapshot>()
        );
    }

    #[test]
    fn exhaustion_strict_threshold_and_single_drain_preserve_cadence() {
        let mut player = vitals(20.0, 20, 5.0, 4.0);
        player.tick(NO_REGEN);
        assert_eq!((player.saturation(), player.exhaustion()), (5.0, 4.0));
        player.add_exhaustion(0.5).unwrap();
        player.tick(NO_REGEN);
        assert_eq!((player.saturation(), player.exhaustion()), (4.0, 0.5));
        player.add_exhaustion(f32::MAX).unwrap();
        assert_eq!(player.exhaustion(), 40.0);
        player.tick(NO_REGEN);
        assert_eq!((player.saturation(), player.exhaustion()), (3.0, 36.0));
        advance(&mut player, 3, NO_REGEN);
        assert_eq!(player.saturation(), 0.0);
        assert_eq!(player.food(), 20);
        player.tick(NO_REGEN);
        assert_eq!(player.food(), 19);
    }

    #[test]
    fn peaceful_preserves_food_during_exhaustion_and_recovers_at_exact_phases() {
        let mut player = vitals(10.0, 10, 0.0, 40.0);
        advance(&mut player, 9, PEACEFUL);
        assert_eq!(
            (player.health(), player.food(), player.saturation()),
            (10.0, 10, 0.0)
        );
        player.tick(PEACEFUL);
        assert_eq!(
            (player.health(), player.food(), player.saturation()),
            (10.0, 11, 0.0)
        );
        advance(&mut player, 9, PEACEFUL);
        assert_eq!(player.health(), 10.0);
        player.tick(PEACEFUL);
        assert_eq!(
            (player.health(), player.food(), player.saturation()),
            (11.0, 12, 1.0)
        );
    }

    #[test]
    fn natural_regeneration_gamerule_disables_both_recovery_paths() {
        for difficulty in [Difficulty::Normal, Difficulty::Peaceful] {
            let mut player = vitals(10.0, 20, 5.0, 0.0);
            advance(&mut player, 160, VitalsTickPolicy::new(difficulty, false));
            assert_eq!(
                (player.health(), player.food(), player.saturation()),
                (10.0, 20, 5.0)
            );
            assert_eq!(player.snapshot().food_tick_timer(), 0);
        }
    }

    #[test]
    fn saturated_regeneration_uses_fraction_and_exhaustion_not_direct_saturation_loss() {
        let mut player = vitals(10.0, 20, 3.0, 0.0);
        advance(&mut player, 9, NORMAL);
        assert_eq!(player.health(), 10.0);
        let result = player.tick(NORMAL);
        assert_eq!(result.healed(), 0.5);
        assert_eq!(
            (player.health(), player.saturation(), player.exhaustion()),
            (10.5, 3.0, 3.0)
        );
        advance(&mut player, 10, NORMAL);
        assert_eq!(player.exhaustion(), 6.0);
        player.tick(NORMAL);
        assert_eq!((player.saturation(), player.exhaustion()), (2.0, 2.0));
    }

    #[test]
    fn ordinary_regeneration_requires_eighteen_food_and_eighty_ticks() {
        let mut player = vitals(10.0, 18, 0.0, 0.0);
        advance(&mut player, 79, NORMAL);
        assert_eq!(player.health(), 10.0);
        assert_eq!(player.tick(NORMAL).healed(), 1.0);
        assert_eq!(player.exhaustion(), 6.0);
        player.tick(NORMAL);
        assert_eq!(player.food(), 17);
        advance(&mut player, 160, NORMAL);
        assert_eq!(player.health(), 11.0);
    }

    #[test]
    fn shared_food_timer_carries_between_regeneration_modes_and_resets_when_ineligible() {
        let mut player = vitals(10.0, 18, 0.0, 0.0);
        advance(&mut player, 79, NORMAL);
        player.apply_food(food("apple"), false).unwrap();
        assert!(player.tick(NORMAL).healed() > 0.0);
        assert_eq!(player.snapshot().food_tick_timer(), 0);
        advance(&mut player, 5, NORMAL);
        player.tick(NO_REGEN);
        assert_eq!(player.snapshot().food_tick_timer(), 0);
    }

    #[test]
    fn starvation_exact_cadence_threshold_and_cooldown_are_deterministic() {
        let mut player = vitals(3.0, 0, 0.0, 0.0);
        advance(&mut player, 79, NO_REGEN);
        assert_eq!(player.health(), 3.0);
        assert_eq!(player.tick(NO_REGEN).damaged(), 1.0);
        assert_eq!(
            (
                player.health(),
                player.invulnerable_ticks(),
                player.hurt_ticks()
            ),
            (2.0, 20, 10)
        );
        advance(&mut player, 160, NO_REGEN);
        assert_eq!(player.health(), 1.0);
        let mut fractional = vitals(1.5, 0, 0.0, 0.0);
        advance(&mut fractional, 80, NO_REGEN);
        assert_eq!(fractional.health(), 0.5);
        advance(&mut fractional, 80, NO_REGEN);
        assert_eq!(fractional.health(), 0.5);
        let mut recent = vitals(10.0, 0, 0.0, 0.0);
        let snapshot = recent.snapshot().with_timers(79, 0, 0, 0.0, 0).unwrap();
        recent.restore(snapshot).unwrap();
        recent.hurt(hit(2.0)).unwrap();
        assert_eq!(recent.tick(NO_REGEN).damaged(), 0.0);
    }

    #[test]
    fn peaceful_zero_food_without_regeneration_uses_reference_ten_health_threshold() {
        let mut player = vitals(11.0, 0, 0.0, 0.0);
        let policy = VitalsTickPolicy::new(Difficulty::Peaceful, false);
        advance(&mut player, 160, policy);
        assert_eq!(player.health(), 10.0);
        assert_eq!(player.food(), 0);
    }

    #[test]
    fn invulnerable_policy_prevents_starvation_and_regeneration_exhaustion() {
        let mut hungry = vitals(10.0, 0, 0.0, 0.0);
        advance(&mut hungry, 160, NO_REGEN.with_invulnerable(true));
        assert_eq!(hungry.health(), 10.0);
        let mut hurt = vitals(10.0, 20, 5.0, 0.0);
        advance(&mut hurt, 10, NORMAL.with_invulnerable(true));
        assert!(hurt.health() > 10.0);
        assert_eq!(hurt.exhaustion(), 0.0);
        assert!(!hurt
            .hurt(hit(100.0).with_invulnerable(true))
            .unwrap()
            .changed());
    }

    #[test]
    fn hits_apply_only_incremental_raw_damage_in_first_half_of_cooldown() {
        let mut player = PlayerVitals::new();
        assert_eq!(player.hurt(hit(4.0)).unwrap().damaged(), 4.0);
        assert!(!player.hurt(hit(4.0)).unwrap().changed());
        assert!(!player.hurt(hit(3.0)).unwrap().changed());
        player.tick(NO_REGEN);
        assert_eq!(player.hurt(hit(7.0)).unwrap().damaged(), 3.0);
        assert_eq!((player.invulnerable_ticks(), player.hurt_ticks()), (19, 9));
        advance(&mut player, 9, NO_REGEN);
        assert_eq!(player.invulnerable_ticks(), 10);
        assert_eq!(player.hurt(hit(7.0)).unwrap().damaged(), 7.0);
        assert_eq!((player.invulnerable_ticks(), player.hurt_ticks()), (20, 10));
    }

    #[test]
    fn armor_is_applied_after_incremental_cooldown_reduction() {
        let mut player = PlayerVitals::new();
        let first = player
            .hurt(DamageRequest::new(10.0, 20.0, 0.0).unwrap())
            .unwrap();
        assert!((first.damaged() - 4.0).abs() < 0.00001);
        let second = player
            .hurt(DamageRequest::new(15.0, 20.0, 0.0).unwrap())
            .unwrap();
        assert!((second.damaged() - 1.5).abs() < 0.00001);
        assert_eq!(player.snapshot().last_hurt(), 15.0);
        assert!(second.damaged() < 3.0);
    }

    #[test]
    fn source_bypasses_are_independent_and_exhaustion_only_follows_effective_damage() {
        let mut player = PlayerVitals::new();
        let attack = DamageRequest::new(4.0, 20.0, 8.0)
            .unwrap()
            .with_bypasses(true, false)
            .with_exhaustion(0.1)
            .unwrap();
        assert_eq!(player.hurt(attack).unwrap().damaged(), 4.0);
        assert_eq!(player.exhaustion(), 0.1);
        assert!(!player.hurt(attack).unwrap().changed());
        assert_eq!(player.exhaustion(), 0.1);
        let bypass_cooldown = DamageRequest::new(4.0, 20.0, 8.0)
            .unwrap()
            .with_bypasses(false, true);
        assert!((player.hurt(bypass_cooldown).unwrap().damaged() - 0.96).abs() < 0.00001);
        assert_eq!(player.invulnerable_ticks(), 20);
    }

    #[test]
    fn food_uses_absolute_saturation_and_caps_at_new_food_level() {
        let mut player = vitals(20.0, 17, 16.0, 1.0);
        let result = player.apply_food(food("apple"), false).unwrap();
        assert_eq!(player.food(), 20);
        assert!((player.saturation() - 18.4).abs() < 0.00001);
        assert_eq!(player.exhaustion(), 1.0);
        assert!(result.food_changed());
        assert!(!result.health_changed());
        assert_eq!(
            player.apply_food(food("apple"), false),
            Err(VitalsError::CannotEat)
        );
        player.apply_food(food("golden_carrot"), true).unwrap();
        assert_eq!(player.saturation(), 20.0);
        player.apply_food(food("honey_bottle"), false).unwrap();
        assert_eq!(player.food(), 20);
    }

    #[test]
    fn fall_boundaries_use_floor_not_legacy_ceil_and_bypass_armor_when_submitted() {
        for (distance, expected) in [
            (0.0, 0.0),
            (3.0, 0.0),
            (3.5, 0.0),
            (3.999, 0.0),
            (4.0, 1.0),
            (4.25, 1.0),
            (5.0, 2.0),
            (12.0, 9.0),
        ] {
            assert_eq!(fall_damage(distance, 1.0).unwrap(), expected);
        }
        assert_eq!(fall_damage(5.0, 0.5).unwrap(), 1.0);
        assert_eq!(fall_damage(5.0, 0.0).unwrap(), 0.0);
        let mut player = PlayerVitals::new();
        let amount = fall_damage(12.0, 1.0).unwrap();
        let result = player
            .hurt(
                DamageRequest::new(amount, 20.0, 8.0)
                    .unwrap()
                    .with_bypasses(true, false),
            )
            .unwrap();
        assert_eq!(result.damaged(), 9.0);
        assert_eq!(player.exhaustion(), 0.0);
    }

    #[test]
    fn invalid_external_amounts_are_refused_without_mutation() {
        let mut player = PlayerVitals::new();
        let before = player.snapshot();
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
            assert!(player.add_exhaustion(value).is_err());
            assert!(player.heal(value).is_err());
            assert!(DamageRequest::new(value, 0.0, 0.0).is_err());
            assert!(hit(1.0).with_exhaustion(value).is_err());
            assert!(fall_damage(5.0, value).is_err());
            assert_eq!(player.snapshot(), before);
        }
        for (armor, toughness) in [(31.0, 0.0), (0.0, 21.0), (-1.0, 0.0), (0.0, f32::NAN)] {
            assert!(DamageRequest::new(1.0, armor, toughness).is_err());
        }
        for distance in [f64::NAN, f64::INFINITY, -1.0, f64::MAX] {
            assert!(fall_damage(distance, 1.0).is_err());
        }
    }

    #[test]
    fn death_is_one_transition_and_cannot_be_reversed_by_food_or_healing() {
        let mut player = vitals(2.0, 0, 0.0, 0.0);
        let result = player.hurt(hit(f32::MAX)).unwrap();
        assert_eq!(result.damaged(), 2.0);
        assert!(result.died());
        assert!(!player.hurt(hit(1.0)).unwrap().died());
        assert!(!player.heal(20.0).unwrap().health_changed());
        assert_eq!(
            player.apply_food(food("honey_bottle"), true),
            Err(VitalsError::CannotEat)
        );
        advance(&mut player, 160, PEACEFUL);
        assert_eq!(player.health(), 0.0);
        assert_eq!(player.food(), 0);
    }

    #[test]
    fn saved_midcycle_restore_replays_identical_outcomes_and_stays_bounded() {
        let mut original = vitals(13.0, 20, 5.0, 0.0);
        advance(&mut original, 7, NORMAL);
        original.hurt(hit(2.0)).unwrap();
        let mut restored = PlayerVitals::from_snapshot(original.snapshot()).unwrap();
        for tick in 0..2000 {
            let policy = if tick % 137 < 30 { PEACEFUL } else { NORMAL };
            if tick % 17 == 0 {
                original.add_exhaustion(0.6).unwrap();
                restored.add_exhaustion(0.6).unwrap();
            }
            if tick % 113 == 0 {
                assert_eq!(
                    original.hurt(hit(1.0)).unwrap(),
                    restored.hurt(hit(1.0)).unwrap()
                );
            }
            if tick % 151 == 0 && original.can_eat(food("apple"), false) {
                assert_eq!(
                    original.apply_food(food("apple"), false).unwrap(),
                    restored.apply_food(food("apple"), false).unwrap()
                );
            }
            assert_eq!(original.tick(policy), restored.tick(policy));
            assert_eq!(original.snapshot(), restored.snapshot());
            assert!(original.snapshot().validate().is_ok());
        }
    }
}
