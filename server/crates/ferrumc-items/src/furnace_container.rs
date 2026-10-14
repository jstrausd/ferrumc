//! Pure ordinary-furnace menu postimages over exact supported item models.
//!
//! Consumers must compare the original furnace snapshot/revision and publish
//! furnace, player and cursor together under one exclusive owner. These plans
//! grant no commit, stale-state, replay, drop, event or recipe/XP authority.

mod exchange;
#[cfg(test)]
mod tests;

use crate::{
    persisted_stack_size, stack_capacity, validate_furnace_parts, ContainerTransferError, ItemId,
    ItemPersistenceError, ItemStack, SmeltingError, StackTransferError,
};

// Schema-5 header/suffix, all 56 item lengths, and the nineteen unrepresented
// slots at their minimum seven-byte FCIS size. The consumer must additionally
// admit its actual armor/craft/offhand/table state before committing this plan.
const PLAYER_FRAMING_MIN: usize = 41 + 21 + 56 * 4 + 19 * 7;
const PLAYER_BYTES: usize = 1024 * 1024;

/// Refusal before an owned furnace click postimage can be published.
#[derive(Debug, thiserror::Error)]
pub enum FurnaceClickError {
    /// Unsupported mode/button or an index outside the 39-slot menu.
    #[error("invalid furnace click action")]
    InvalidAction,
    /// Complete native furnace representation exceeds its budget or admission.
    #[error(transparent)]
    Furnace(#[from] SmeltingError),
    /// One stack fails lossless storage admission.
    #[error(transparent)]
    Stack(#[from] ItemPersistenceError),
    /// A stack has invalid effective capacity or an exchange failed.
    #[error(transparent)]
    Capacity(#[from] ContainerTransferError),
    /// One checked separation or transfer failed.
    #[error(transparent)]
    Transfer(#[from] StackTransferError),
    /// Player postimage plus the minimum complete-record framing exceeds 1 MiB.
    #[error("furnace click player budget exceeded")]
    PlayerBudgetExceeded,
    /// A present air stack cannot participate in an ordinary menu.
    #[error("present air in furnace click")]
    PresentAir,
    /// An over-capacity hotbar replacement requires selected/offhand/drop state
    /// not represented by this API; no source may be discarded or overwritten.
    #[error("unsupported oversized occupied furnace hotbar swap")]
    UnsupportedSwap,
}

/// Owned exact furnace/player/cursor postimages from one immutable snapshot.
/// The plan intentionally has no `Clone` or mutable slot escape.
#[derive(Debug)]
pub struct FurnaceClickPlan {
    slots: [ItemStack; 3],
    player: [ItemStack; 36],
    cursor: ItemStack,
    changed: bool,
}

impl FurnaceClickPlan {
    /// Furnace postimage in input/fuel/result order.
    pub fn slots(&self) -> &[ItemStack; 3] {
        &self.slots
    }
    /// Player postimage: 27 main slots followed by nine hotbar slots.
    pub fn player(&self) -> &[ItemStack; 36] {
        &self.player
    }
    /// Exact carried-item postimage.
    pub fn cursor(&self) -> &ItemStack {
        &self.cursor
    }
    /// Whether at least one item quantity or representation changes.
    pub const fn changed(&self) -> bool {
        self.changed
    }
    /// Move the complete postimage into the one authorized consumer.
    pub fn into_parts(self) -> ([ItemStack; 3], [ItemStack; 36], ItemStack) {
        (self.slots, self.player, self.cursor)
    }
}

/// Plan supported ordinary-furnace clicks without changing any borrowed input.
///
/// Menu slots 0/1/2 are input/fuel/output; 3..29 are player main and 30..38
/// hotbar. Mode 0 is left/right pickup, mode 1 shift (buttons 0/1), and mode 2
/// number-key swap (buttons 0..8). Manual input accepts any admitted item; fuel
/// accepts actual fuel or one bucket; output only permits extraction. Shift
/// selects smelting input first, then actual fuel, then main/hotbar relocation;
/// a full selected furnace destination does not fall back. Result shifts visit
/// player slots backwards, while input/fuel shifts visit them forwards.
///
/// Every source and complete three-slot furnace candidate passes the 16 KiB
/// entity admission. Player/cursor candidates plus schema-5 framing and minimum
/// omitted-slot bytes pass 1 MiB admission. The consumer must validate its full
/// player snapshot, including actual omitted slots, before committing. Exact
/// ordered components define merge equality; item-specific overrides, plugins,
/// XP and oversized occupied swaps requiring inventory/drop state are absent.
/// Timers stay consumer-owned, including input-identity cook-progress resets.
pub fn plan_furnace_click(
    slots: &[ItemStack; 3],
    player: &[ItemStack; 36],
    cursor: &ItemStack,
    slot: usize,
    button: i8,
    mode: i32,
) -> Result<FurnaceClickPlan, FurnaceClickError> {
    if slot >= 39 || !matches!((mode, button), (0 | 1, 0 | 1) | (2, 0..=8)) {
        return Err(FurnaceClickError::InvalidAction);
    }
    admit(slots, player, cursor)?;
    let mut plan = FurnaceClickPlan {
        slots: slots.clone(),
        player: player.clone(),
        cursor: cursor.clone(),
        changed: false,
    };
    plan.changed = match mode {
        0 => exchange::pickup(&mut plan, slot, button == 1)?,
        1 => exchange::shift(&mut plan, slot)?,
        2 => exchange::hotbar(&mut plan, slot, button as usize)?,
        _ => return Err(FurnaceClickError::InvalidAction),
    };
    admit(&plan.slots, &plan.player, &plan.cursor)?;
    Ok(plan)
}

fn admit(
    slots: &[ItemStack; 3],
    player: &[ItemStack; 36],
    cursor: &ItemStack,
) -> Result<(), FurnaceClickError> {
    // Timers have fixed encoded size, so legal placeholder values admit the
    // complete entity bytes without granting authority over live timer state.
    validate_furnace_parts(slots, 0, 0, 0, 200)?;
    let mut bytes = PLAYER_FRAMING_MIN;
    for stack in player.iter().chain(std::iter::once(cursor)) {
        bytes = bytes
            .checked_add(persisted_stack_size(stack)?)
            .ok_or(FurnaceClickError::PlayerBudgetExceeded)?;
        if bytes > PLAYER_BYTES {
            return Err(FurnaceClickError::PlayerBudgetExceeded);
        }
        stack_capacity(stack)?;
        if stack.item().is_some_and(ItemId::is_air) {
            return Err(FurnaceClickError::PresentAir);
        }
    }
    Ok(())
}
