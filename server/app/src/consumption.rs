//! Connection-owned timed use with a pinned inventory postimage until owner ACK.
use crate::inventory::PlayerInventory;
use ferrumc_items::FoodProperties;

pub(crate) struct FoodUse {
    pub(crate) slot: usize,
    pub(crate) before: Vec<u8>,
    pub(crate) due: u64,
    pub(crate) food: FoodProperties,
}

pub(crate) struct PendingFood {
    pub(crate) token: u64,
    pub(crate) inventory: crate::inventory::PreparedConsumption,
}

pub(crate) struct Consumption {
    pub(crate) using: Option<FoodUse>,
    pub(crate) pending: Option<PendingFood>,
    pub(crate) deferred_close: Option<i32>,
    // At most the existing64 exact mining witnesses can produce committed wear.
    pub(crate) deferred_breaks: Vec<crate::gameplay::GameplayEffect>,
    next: u64,
}

impl Consumption {
    pub(crate) fn new() -> Self {
        Self {
            using: None,
            pending: None,
            deferred_close: None,
            deferred_breaks: Vec::new(),
            next: 1,
        }
    }
    pub(crate) fn start(
        &mut self,
        inventory: &PlayerInventory,
        slot: usize,
        now: u64,
    ) -> anyhow::Result<()> {
        if self.pending.is_some() {
            return Ok(());
        }
        let Some(stack) = inventory.slot(slot) else {
            return Ok(());
        };
        let Some(food) = ferrumc_items::food_properties(stack)? else {
            return Ok(());
        };
        self.using = Some(FoodUse {
            slot,
            before: ferrumc_items::encode_persisted_stack(stack)?,
            due: now
                .checked_add(u64::from(food.consume_ticks()))
                .ok_or_else(|| anyhow::anyhow!("food use tick overflow"))?,
            food,
        });
        Ok(())
    }
    pub(crate) fn cancel(&mut self) {
        self.using = None;
    }
    pub(crate) fn token(&mut self) -> anyhow::Result<u64> {
        let token = self.next;
        self.next = token
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("food token exhausted"))?;
        Ok(token)
    }
}
