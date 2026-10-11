//! Explicit native item counter/dirty-state limits; no physics or plugin events.

use crate::{
    ItemEntityCounters, PlayerOwnershipError, MAX_ENTITIES_PER_SHARD, MAX_PLAYER_OWNERSHIP_INPUTS,
};
use ferrumc_core::{EntityUuid, Tick};
use ferrumc_storage::{OwnershipTransferId, PersistentEntityRecord};
use std::num::NonZeroUsize;

/// Maximum additional retained confirmed FCIE bodies (16 MiB per shard).
pub const MAX_CONFIRMED_ITEM_BYTES: usize = 16 * 1024 * 1024;
/// Maximum unconfirmed logical counter steps on one existing reserved item.
pub const MAX_OWNED_COUNTER_DEFER: usize = 1024;
/// Maximum checked scalar scratch bytes, independently of native body budgets.
pub const MAX_OWNED_COUNTER_PLAN_BYTES: usize = 9 * 1024 * 1024;
/// Maximum scalar descriptors including current slots and bounded new admissions.
pub const MAX_OWNED_COUNTER_PLAN_ENTRIES: usize =
    MAX_ENTITIES_PER_SHARD + MAX_PLAYER_OWNERSHIP_INPUTS + 1;
/// Maximum logical encoded charge of one entity save request and retained images.
pub const MAX_OWNED_ENTITY_SAVE_CHARGE: usize = 2 * 1024 * 1024;

/// Explicit positive limits for the opt-in native counter and save consumer.
///
/// The only despawn policy in this stage is Vanilla without plugins. All resident
/// owned items step each tick; `save_scan` limits only save selection work.
#[derive(Debug, Clone, Copy)]
pub struct OwnedItemCounterLimits {
    pub(crate) confirmed_bytes: usize,
    pub(crate) defer: usize,
    pub(crate) save_scan: usize,
    pub(crate) plan_entries: usize,
    pub(crate) plan_bytes: usize,
    pub(crate) save_charge: usize,
    pub(crate) save_period: u32,
    pub(crate) despawn_rate: i32,
    pub(crate) save_ids: usize,
}
impl OwnedItemCounterLimits {
    /// Creates checked positive resource limits and the exact signed despawn rate.
    #[allow(
        clippy::too_many_arguments,
        reason = "all separate positive limits are explicit operator agreements"
    )]
    pub fn new(
        confirmed_bytes: NonZeroUsize,
        defer: NonZeroUsize,
        save_scan: NonZeroUsize,
        plan_entries: NonZeroUsize,
        plan_bytes: NonZeroUsize,
        save_charge: NonZeroUsize,
        save_period: std::num::NonZeroU32,
        despawn_rate: i32,
        save_ids: NonZeroUsize,
    ) -> Result<Self, PlayerOwnershipError> {
        for (value, max, name) in [
            (
                confirmed_bytes.get(),
                MAX_CONFIRMED_ITEM_BYTES,
                "confirmed item bytes",
            ),
            (defer.get(), MAX_OWNED_COUNTER_DEFER, "counter defer"),
            (save_scan.get(), 256, "save scan"),
            (
                plan_entries.get(),
                MAX_OWNED_COUNTER_PLAN_ENTRIES,
                "counter plan count",
            ),
            (
                plan_bytes.get(),
                MAX_OWNED_COUNTER_PLAN_BYTES,
                "counter plan bytes",
            ),
            (
                save_charge.get(),
                MAX_OWNED_ENTITY_SAVE_CHARGE,
                "entity save charge",
            ),
            (save_ids.get(), 32, "save IDs"),
        ] {
            if value > max {
                return Err(PlayerOwnershipError::InvalidLimit(name));
            }
        }
        Ok(Self {
            confirmed_bytes: confirmed_bytes.get(),
            defer: defer.get(),
            save_scan: save_scan.get(),
            plan_entries: plan_entries.get(),
            plan_bytes: plan_bytes.get(),
            save_charge: save_charge.get(),
            save_period: save_period.get(),
            despawn_rate,
            save_ids: save_ids.get(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OwnedItemDurability {
    pub(crate) confirmed: PersistentEntityRecord,
    pub(crate) generation: u64,
    pub(crate) saved: u64,
    pub(crate) saved_tick: Tick,
    pub(crate) due: bool,
    pub(crate) priority: bool,
    pub(crate) first_due: Option<Tick>,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeferredOwnedCounters {
    pub(crate) original_counters: ItemEntityCounters,
    pub(crate) original_generation: u64,
    pub(crate) counters: ItemEntityCounters,
    pub(crate) generation: u64,
    pub(crate) claims: usize,
    pub(crate) first: Option<Tick>,
    pub(crate) last: Option<Tick>,
    pub(crate) due: bool,
    pub(crate) first_due: Option<Tick>,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct CounterBoundaryEntry {
    pub(crate) original_counters: ItemEntityCounters,
    pub(crate) original_generation: u64,
    pub(crate) slot: usize,
    pub(crate) uuid: EntityUuid,
    pub(crate) counters: ItemEntityCounters,
    pub(crate) generation: u64,
    pub(crate) deferred: Option<OwnershipTransferId>,
    pub(crate) claims: usize,
    pub(crate) due: bool,
    pub(crate) resident: bool,
    pub(crate) present: bool,

    pub(crate) saved: u64,
    pub(crate) wire: ferrumc_core::EntityId,
    pub(crate) body_len: usize,
    pub(crate) confirmed_len: usize,
    pub(crate) first_due: Option<Tick>,
}
#[derive(Debug)]
pub(crate) struct OwnedItemCounterState {
    pub(crate) limits: OwnedItemCounterLimits,
    pub(crate) tick: Tick,
    pub(crate) processing_tick: Option<Tick>,
    pub(crate) scratch: Vec<CounterBoundaryEntry>,
    pub(crate) normal_cursor: usize,
    pub(crate) priority_cursor: usize,
    pub(crate) selection_turn: u8,
    pub(crate) save_ids: Vec<OwnershipTransferId>,
}
impl OwnedItemCounterState {
    pub(crate) fn new(limits: OwnedItemCounterLimits, tick: Tick) -> Self {
        Self {
            limits,
            tick,
            processing_tick: None,
            scratch: Vec::new(),
            normal_cursor: 0,
            priority_cursor: 0,
            selection_turn: 0,
            save_ids: Vec::new(),
        }
    }
    pub(crate) fn snapshot(&self) -> Self {
        Self {
            limits: self.limits,
            tick: self.tick,
            processing_tick: None,
            scratch: Vec::new(),
            normal_cursor: self.normal_cursor,
            priority_cursor: self.priority_cursor,
            selection_turn: self.selection_turn,
            save_ids: Vec::new(),
        }
    }
}

/// Opaque binding for move-owned save ID refills to this concrete counter owner.
#[derive(Debug, Clone)]
pub struct OwnedItemCounterHandle {
    pub(crate) owner: std::sync::Weak<()>,
}
