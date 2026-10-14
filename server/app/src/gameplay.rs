//! Trusted bounded simulation-to-inventory effects; no client-authored stack grants.
use ferrumc_math::BlockPos;

/// An effect is published only after the owner committed the block removal.
pub(crate) enum GameplayEffect {
    BlockBroken {
        position: BlockPos,
        sequence: i32,
        held_item: Option<i32>,
        experience: u32,
        tool_damage: u8,
        tool_token: u64,
    },
    Pickup {
        ticket: ferrumc_sim::GroundItemPickupTicket,
        wire_id: ferrumc_core::EntityId,
        stack: ferrumc_items::ItemStack,
    },
    MiningReleased {
        token: u64,
    },
    FoodResolved {
        token: u64,
        accepted: bool,
    },
    ArmorHurt {
        snapshot: std::sync::Arc<crate::combat_wear::CombatWearSnapshot>,
        raw_damage: f32,
    },
    Died {
        position: ferrumc_math::Vec3,
    },
    Respawned {
        position: ferrumc_math::Vec3,
    },
}

/// Thirty-two owner effects per connection; overflow closes the session rather
/// than acknowledging tool wear or inventory changes that were never delivered.
pub(crate) const EFFECT_CAPACITY: usize = 32;
