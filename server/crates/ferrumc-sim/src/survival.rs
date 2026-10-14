//! Explicit legacy gameplay selection and fixed-size player runtime facts.

use crate::{PlayerVitals, VitalsTickPolicy};
use ferrumc_math::Vec3;

/// Refused legacy gameplay activation; existing state is preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SurvivalError {
    /// The separate native durable owner already owns player gameplay.
    #[error("Survival gameplay requires legacy player ownership")]
    NativeOwnership,
    /// The legacy gameplay selection is once-only.
    #[error("Survival gameplay already initialized")]
    AlreadyInitialized,
}

#[derive(Debug)]
pub(crate) struct SurvivalRuntime {
    pub(crate) live: bool,
    pub(crate) policy: VitalsTickPolicy,
}
impl Clone for SurvivalRuntime {
    fn clone(&self) -> Self {
        Self {
            live: false,
            policy: self.policy,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SurvivalPlayer {
    pub(crate) vitals: PlayerVitals,
    pub(crate) last_food_token: u64,
    pub(crate) last_bucket_token: u64,
    pub(crate) movement_position: Vec3,
    pub(crate) fall_distance: f64,
    pub(crate) attack_cooldown: u8,
    pub(crate) attack_strength_ticks: u16,
    pub(crate) last_attack_token: u64,
    pub(crate) last_attack_item: Option<i32>,
    pub(crate) death_drops_admitted: bool,
}
impl SurvivalPlayer {
    pub(crate) fn new(position: Vec3) -> Self {
        Self {
            vitals: PlayerVitals::new(),
            last_food_token: 0,
            last_bucket_token: 0,
            movement_position: position,
            fall_distance: 0.0,
            attack_cooldown: 0,
            attack_strength_ticks: 0,
            last_attack_token: 0,
            last_attack_item: None,
            death_drops_admitted: false,
        }
    }
}
