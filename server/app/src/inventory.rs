//! Per-connection authoritative player inventory (window 0).
//!
//! The player inventory is connection-local state: the connection task is the
//! sole writer of every slot (it validates hostile creative-slot bytes through
//! [`ferrumc_items`] before storing) and owns the mandatory clientbound sends
//! that keep the client's view in sync. The simulation never sees the inventory;
//! it only receives the resolved block-state a place produces.
//!
//! The [`GameMode`] mirror held alongside is seeded to creative on join and
//! updated by the connection whenever it processes `/gamemode`. Because the
//! connection is the sole writer of mode changes, the mirror cannot drift, so the
//! "is this player authoritatively creative" gate the creative-slot handler needs
//! is a synchronous, lock-free local read instead of a per-packet driver
//! round-trip (creative-slot packets are frequent during menu editing).

#[cfg(test)]
use std::num::NonZeroU8;

use ferrumc_core::GameMode;
use ferrumc_items::{encode_container_content_payload, ItemStack, ItemValidationError};

#[cfg(test)]
use ferrumc_items::{left_click_exchange, ComponentPatch, ItemId};

type CraftPostimage = (Vec<ItemStack>, Vec<ItemStack>, ItemStack);

/// The player-inventory window id (window 0 is always open).
pub(crate) const WINDOW_ID: i32 = 0;

/// First main-inventory slot (the storage grid; armor/craft occupy `0..9`).
pub(crate) const MAIN_START: usize = 9;

/// One past the last hotbar slot (`45` is the offhand, excluded from a container
/// window). `MAIN_START..PLAYER_WINDOW_END` is the 36-slot main+hotbar region a
/// chest window mirrors after its own slots.
pub(crate) const PLAYER_WINDOW_END: usize = 45;

/// Number of slots in the player inventory window.
///
/// Layout: `0` craft-out, `1..=4` craft-in, `5..=8` armor, `9..=35` main,
/// `36..=44` hotbar, `45` offhand.
pub(crate) const SLOT_COUNT: usize = 46;

/// Index of the first hotbar slot (the slot for selected hotbar index `0`).
pub(crate) const HOTBAR_START: usize = 36;

/// Number of selectable hotbar slots (indices `0..=8`).
const HOTBAR_LEN: u8 = 9;

// Window-0 inventory indices of the slots mirrored into a player's *visible*
// equipment. The four armor pieces occupy `5..=8`; the off-hand is `45`; the
// main hand is the currently selected hotbar slot (`HOTBAR_START + selected`).
/// Inventory index of the worn helmet.
const HELMET_SLOT: usize = 5;
/// Inventory index of the worn chestplate.
const CHESTPLATE_SLOT: usize = 6;
/// Inventory index of the worn leggings.
const LEGGINGS_SLOT: usize = 7;
/// Inventory index of the worn boots.
const BOOTS_SLOT: usize = 8;
/// Inventory index of the off-hand slot.
pub(crate) const OFFHAND_SLOT: usize = 45;

// Equipment-slot ids carried in the clientbound `SetEquipment` packet: the low 7
// bits of each entry's slot byte. The high bit (`EQUIPMENT_CONTINUATION_BIT`)
// marks a non-terminal entry. These are the wire ids, distinct from the inventory
// indices above.
/// `SetEquipment` slot id for the main hand.
const EQUIP_MAIN_HAND: u8 = 0;
/// `SetEquipment` slot id for the off hand.
const EQUIP_OFF_HAND: u8 = 1;
/// `SetEquipment` slot id for the boots.
const EQUIP_BOOTS: u8 = 2;
/// `SetEquipment` slot id for the leggings.
const EQUIP_LEGGINGS: u8 = 3;
/// `SetEquipment` slot id for the chestplate.
const EQUIP_CHESTPLATE: u8 = 4;
/// `SetEquipment` slot id for the helmet.
const EQUIP_HELMET: u8 = 5;

/// High bit of a `SetEquipment` entry's slot byte: set on every entry except the
/// last to signal that another entry follows.
const EQUIPMENT_CONTINUATION_BIT: u8 = 0x80;

/// The starter creative kit placed in the hotbar (slots `36..=44`) on join: one
/// simple, placeable full block per hotbar slot. Every name is a placeable block
/// item in the pinned 1.21.8 registry.
#[cfg(test)]
const HOTBAR_KIT: [&str; HOTBAR_LEN as usize] = [
    "stone",
    "dirt",
    "oak_planks",
    "glass",
    "cobblestone",
    "bricks",
    "oak_log",
    "white_wool",
    "sand",
];

/// Count of each kit item placed in the hotbar.
#[cfg(test)]
const KIT_STACK_COUNT: u8 = 64;

/// The window state id carried by the first `SetContainerContent` on join. It is
/// bumped on every subsequent change so the client can validate clicks against it.
const INITIAL_STATE_ID: i32 = 1;

/// A server-authoritative 46-slot player inventory plus the selected hotbar index,
/// the sole carried stack, window state id, and connection-local game-mode mirror.
pub(crate) struct PlayerInventory {
    /// Every window-0 slot, indexed `0..SLOT_COUNT`.
    slots: [ItemStack; SLOT_COUNT],
    /// Player-owned cursor, retained even when no chest screen is open.
    carried: ItemStack,
    /// The window state id, sent on every clientbound inventory update and bumped
    /// on every change (wrapping).
    state_id: i32,
    /// The selected hotbar index, always `0..=8`.
    selected: u8,
    /// The connection-local mirror of the player's authoritative game mode.
    game_mode: GameMode,
    recipes: Option<ferrumc_items::NormalRecipes>,
    table_inputs: [ItemStack; 9],
}

/// Immutable prepared inventory state, pinned until a matching food owner ACK.
pub(crate) struct PreparedConsumption {
    slots: [ItemStack; SLOT_COUNT],
    carried: ItemStack,
}

impl PlayerInventory {
    /// Checks the complete projected-DAMAGE envelope for the current image.
    pub(crate) fn admit_budget(&self) -> anyhow::Result<()> {
        crate::inventory_budget::admit(
            self.slots
                .iter()
                .chain(std::iter::once(&self.carried))
                .chain(self.table_inputs.iter()),
        )
    }

    /// Preflights a container postimage before the world owner can commit.
    pub(crate) fn admit_container_candidate(
        &self,
        window: &[ItemStack],
        carried: &ItemStack,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            window.len() == PLAYER_WINDOW_END - MAIN_START,
            "invalid player window shape"
        );
        crate::inventory_budget::admit(
            self.slots
                .iter()
                .enumerate()
                .map(|(index, stack)| {
                    if (MAIN_START..PLAYER_WINDOW_END).contains(&index) {
                        &window[index - MAIN_START]
                    } else {
                        stack
                    }
                })
                .chain(std::iter::once(carried))
                .chain(self.table_inputs.iter()),
        )
    }

    /// Preflights the exact shared shift/number-key chest postimage before send.
    pub(crate) fn admit_chest_inventory_candidate(
        &self,
        chest: &[ItemStack],
        slot: usize,
        hotbar: Option<u8>,
    ) -> anyhow::Result<()> {
        let (_, after) =
            ferrumc_items::plan_chest_inventory_click(chest, self.window_slots(), slot, hotbar)?;
        self.admit_container_candidate(&after, &self.carried)
    }

    /// Preflights the exact shared cursor exchange before a chest owner command.
    pub(crate) fn admit_chest_cursor_candidate(
        &self,
        slot: &ItemStack,
        right: bool,
    ) -> anyhow::Result<()> {
        let mut slot = slot.clone();
        let mut cursor = self.carried.clone();
        ferrumc_items::exchange_slot(&mut slot, &mut cursor, right)?;
        self.admit_container_candidate(self.window_slots(), &cursor)
    }

    // Synchronous local planning cannot publish a candidate to any other lane.
    // A refusal retains the complete beforeimage, including the state ID.
    fn stage<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut candidate = Self {
            slots: self.slots.clone(),
            carried: self.carried.clone(),
            state_id: self.state_id,
            selected: self.selected,
            game_mode: self.game_mode,
            recipes: self.recipes.take(),
            table_inputs: self.table_inputs.clone(),
        };
        let result = operation(&mut candidate).and_then(|result| {
            candidate.admit_budget()?;
            Ok(result)
        });
        self.recipes = candidate.recipes.take();
        if result.is_ok() {
            self.slots = candidate.slots;
            self.carried = candidate.carried;
            self.table_inputs = candidate.table_inputs;
            self.state_id = candidate.state_id;
        }
        result
    }

    /// Builds an inventory seeded with the creative starter kit in the hotbar and
    /// the given game-mode mirror.
    #[cfg(test)]
    pub(crate) fn with_creative_kit(game_mode: GameMode) -> Self {
        let mut slots: [ItemStack; SLOT_COUNT] = std::array::from_fn(|_| ItemStack::empty());
        let count = NonZeroU8::new(KIT_STACK_COUNT).unwrap_or(NonZeroU8::MIN);
        for (offset, name) in HOTBAR_KIT.iter().enumerate() {
            // Every kit name is a known registry item, so this resolves; an
            // unexpected miss leaves the slot empty rather than panicking.
            if let Some(item) = ItemId::from_name(name) {
                slots[HOTBAR_START + offset] = ItemStack::new(item, count, ComponentPatch::empty());
            }
        }
        Self {
            slots,
            carried: ItemStack::empty(),
            state_id: INITIAL_STATE_ID,
            selected: 0,
            game_mode,
            recipes: ferrumc_items::NormalRecipes::builtin().ok(),
            table_inputs: std::array::from_fn(|_| ItemStack::empty()),
        }
    }

    /// Builds an inventory restored from persisted state: the given slots verbatim,
    /// the (range-checked) selected hotbar index, and the game-mode mirror, stamped
    /// with the initial window state id.
    ///
    /// An out-of-range `selected` (not `0..=8`) clamps to `0` rather than producing
    /// an invalid selection, so a corrupt persisted value can never desync the held
    /// slot.
    pub(crate) fn from_persisted(
        game_mode: GameMode,
        selected: u8,
        slots: [ItemStack; SLOT_COUNT],
        carried: ItemStack,
    ) -> Self {
        Self {
            slots,
            carried,
            state_id: INITIAL_STATE_ID,
            selected: if selected < HOTBAR_LEN { selected } else { 0 },
            game_mode,
            recipes: ferrumc_items::NormalRecipes::builtin().ok(),
            table_inputs: std::array::from_fn(|_| ItemStack::empty()),
        }
    }

    /// The current window state id.
    pub(crate) fn state_id(&self) -> i32 {
        self.state_id
    }

    /// The selected hotbar index (always `0..=8`).
    pub(crate) fn selected(&self) -> u8 {
        self.selected
    }

    /// The connection-local mirror of the player's game mode.
    pub(crate) fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    /// Updates the game-mode mirror (the connection is the sole writer, after it
    /// processes a `/gamemode`).
    pub(crate) fn set_game_mode(&mut self, mode: GameMode) {
        self.game_mode = mode;
    }

    /// The stack in slot `index`, or `None` if `index` is out of range.
    pub(crate) fn slot(&self, index: usize) -> Option<&ItemStack> {
        self.slots.get(index)
    }

    /// A mutable reference to slot `index`, or `None` if out of range.
    ///
    /// Test fixtures may seed a slot directly; production exchanges use
    /// [`Self::exchange_carried`] to keep cursor ownership inside this inventory.
    #[cfg(test)]
    pub(crate) fn slot_mut(&mut self, index: usize) -> Option<&mut ItemStack> {
        self.slots.get_mut(index)
    }

    /// The sole authoritative carried stack, shared by every inventory screen.
    pub(crate) fn carried(&self) -> &ItemStack {
        &self.carried
    }

    /// Adopts a completed container receipt whose full postimage was admitted
    /// before its owner command. Restored cursor data enters via `from_persisted`.
    pub(crate) fn set_carried(&mut self, carried: ItemStack) {
        self.carried = carried;
        self.bump_state_id();
    }

    /// Conserving exchange between one player slot and its carried stack.
    #[cfg(test)]
    pub(crate) fn exchange_carried(&mut self, index: usize) {
        if let Some(slot) = self.slots.get_mut(index) {
            left_click_exchange(slot, &mut self.carried);
            self.bump_state_id();
        }
    }

    /// Applies a normal left/right click to window 0 using only server state.
    ///
    /// The crafting result consumes a matching pinned normal recipe. Armor
    /// removal is allowed, but armor insertion is refused until equippable and
    /// binding rules are modeled. Main storage, hotbar, crafting inputs and
    /// offhand accept ordinary exchanges. Spectators cannot mutate any slot.
    /// Returns whether either stack changed, advancing the state ID once.
    pub(crate) fn normal_click(&mut self, index: usize, button: i8) -> anyhow::Result<bool> {
        self.stage(|candidate| candidate.normal_click_inner(index, button))
    }

    fn normal_click_inner(&mut self, index: usize, button: i8) -> anyhow::Result<bool> {
        if self.game_mode == GameMode::Spectator || !matches!(button, 0 | 1) {
            return Ok(false);
        }
        if index == 0 {
            return self.craft_once(false);
        }
        if (HELMET_SLOT..=BOOTS_SLOT).contains(&index) && self.carried.item().is_some() {
            return Ok(false);
        }
        let Some(slot) = self.slots.get_mut(index) else {
            return Ok(false);
        };
        let changed = ferrumc_items::exchange_slot(slot, &mut self.carried, button == 1)?;
        if changed {
            self.refresh_crafting_inner()?;
            self.bump_state_id();
        }
        Ok(changed)
    }

    /// Recomputes the derived 2×2 result; this preview owns no ingredients.
    #[cfg(test)]
    pub(crate) fn refresh_crafting(&mut self) -> anyhow::Result<()> {
        self.stage(Self::refresh_crafting_inner)
    }

    fn refresh_crafting_inner(&mut self) -> anyhow::Result<()> {
        let grid = ferrumc_items::CraftingGrid::new(2, &self.slots[1..5])?;
        self.slots[0] = self
            .recipes
            .as_ref()
            .and_then(|recipes| recipes.iter().find(|recipe| recipe.matches(&grid)))
            .map_or_else(ItemStack::empty, |recipe| recipe.output().clone());
        Ok(())
    }

    fn craft_once(&mut self, quick: bool) -> anyhow::Result<bool> {
        let Some((inputs, storage, cursor)) = self.craft_grid(2, &self.slots[1..5], quick)? else {
            return Ok(false);
        };
        self.slots[1..5].clone_from_slice(&inputs);
        self.slots[MAIN_START..PLAYER_WINDOW_END].clone_from_slice(&storage);
        self.carried = cursor;
        self.refresh_crafting_inner()?;
        self.bump_state_id();
        Ok(true)
    }

    fn craft_grid(
        &self,
        side: u8,
        inputs: &[ItemStack],
        quick: bool,
    ) -> anyhow::Result<Option<CraftPostimage>> {
        self.admit_table_stacks(inputs)?;
        let grid = ferrumc_items::CraftingGrid::new(side, inputs)?;
        let Some(recipe) = self
            .recipes
            .as_ref()
            .and_then(|recipes| recipes.iter().find(|recipe| recipe.matches(&grid)))
        else {
            return Ok(None);
        };
        let Some(plan) = recipe.plan_one(&grid)? else {
            return Ok(None);
        };
        let mut storage = self.window_slots().to_vec();
        let cursor_after = if quick {
            let mut output = plan.output().clone();
            ferrumc_items::transfer_into_slots(&mut output, &mut storage, true)?;
            if output.item().is_some() {
                return Ok(None);
            }
            self.carried.clone()
        } else {
            if self.carried.item().is_some()
                && !ferrumc_items::same_stack_representation(plan.output(), &self.carried)?
            {
                return Ok(None);
            }
            let transfer = ferrumc_items::plan_transfer(
                plan.output(),
                &self.carried,
                plan.output().count(),
                ferrumc_items::stack_capacity(plan.output())?,
            )?;
            if transfer.moved() != plan.output().count() {
                return Ok(None);
            }
            transfer.into_stacks().1
        };
        let (_, mut inputs, rests) = plan.into_parts();
        for (input, mut rest) in inputs.iter_mut().zip(rests) {
            let cap = if input.item().is_some() {
                ferrumc_items::stack_capacity(input)?
            } else {
                ferrumc_items::stack_capacity(&rest)?
            };
            let remainder = ferrumc_items::plan_transfer(&rest, input, rest.count(), cap)?;
            (rest, *input) = remainder.into_stacks();
            ferrumc_items::transfer_into_slots(&mut rest, &mut storage, false)?;
            if rest.item().is_some() {
                return Ok(None);
            }
        }
        Ok(Some((inputs, storage, cursor_after)))
    }

    fn admit_table_stacks(&self, inputs: &[ItemStack]) -> anyhow::Result<()> {
        let mut bytes = 0usize;
        for stack in inputs
            .iter()
            .chain(self.window_slots())
            .chain(std::iter::once(&self.carried))
        {
            bytes = bytes
                .checked_add(ferrumc_items::persisted_stack_size(stack)?)
                .ok_or_else(|| anyhow::anyhow!("crafting inventory budget overflow"))?;
            anyhow::ensure!(bytes <= 1024 * 1024, "crafting inventory budget exceeded");
            ferrumc_items::stack_capacity(stack)?;
        }
        Ok(())
    }

    /// Persisted 3×3 inputs, retained if returning them encounters a full bag.
    pub(crate) fn table_inputs(&self) -> &[ItemStack; 9] {
        &self.table_inputs
    }

    /// Restores the lossless persisted crafting-table inputs.
    pub(crate) fn restore_table_inputs(&mut self, inputs: [ItemStack; 9]) {
        self.table_inputs = inputs;
    }

    /// Builds a 46-slot crafting-table screen, including its derived result.
    pub(crate) fn table_content_payload(&self) -> anyhow::Result<Vec<u8>> {
        let grid = ferrumc_items::CraftingGrid::new(3, &self.table_inputs)?;
        let result = self
            .recipes
            .as_ref()
            .and_then(|recipes| recipes.iter().find(|recipe| recipe.matches(&grid)))
            .map_or_else(ItemStack::empty, |recipe| recipe.output().clone());
        let mut slots = Vec::with_capacity(46);
        slots.push(result);
        slots.extend_from_slice(&self.table_inputs);
        slots.extend_from_slice(self.window_slots());
        Ok(encode_container_content_payload(&slots, &self.carried)?)
    }

    /// Applies authoritative ordinary/shift/number-key crafting-table clicks.
    pub(crate) fn table_click(
        &mut self,
        slot: usize,
        button: i8,
        mode: i32,
    ) -> anyhow::Result<bool> {
        self.stage(|candidate| candidate.table_click_inner(slot, button, mode))
    }

    fn table_click_inner(&mut self, slot: usize, button: i8, mode: i32) -> anyhow::Result<bool> {
        if self.game_mode == GameMode::Spectator || slot >= 46 {
            return Ok(false);
        }
        if slot == 0 {
            if !matches!(mode, 0 | 1) || !matches!(button, 0 | 1) {
                return Ok(false);
            }
            let mut changed = false;
            for _ in 0..if mode == 1 { 64 } else { 1 } {
                let Some((inputs, storage, cursor)) =
                    self.craft_grid(3, &self.table_inputs, mode == 1)?
                else {
                    break;
                };
                self.table_inputs.clone_from_slice(&inputs);
                self.slots[MAIN_START..PLAYER_WINDOW_END].clone_from_slice(&storage);
                self.carried = cursor;
                changed = true;
            }
            if changed {
                self.bump_state_id();
            }
            return Ok(changed);
        }
        self.admit_table_stacks(&self.table_inputs)?;
        let mut inputs = self.table_inputs.clone();
        let mut storage = self.window_slots().to_vec();
        let mut cursor = self.carried.clone();
        let changed = match mode {
            0 if matches!(button, 0 | 1) => {
                let target = if slot <= 9 {
                    &mut inputs[slot - 1]
                } else {
                    &mut storage[slot - 10]
                };
                ferrumc_items::exchange_slot(target, &mut cursor, button == 1)?
            }
            1 if matches!(button, 0 | 1) => {
                if slot <= 9 {
                    ferrumc_items::transfer_into_slots(&mut inputs[slot - 1], &mut storage, false)?
                } else {
                    ferrumc_items::transfer_into_slots(&mut storage[slot - 10], &mut inputs, false)?
                }
            }
            2 if (0..9).contains(&button) => {
                let hotbar = 27 + button as usize;
                if slot <= 9 {
                    std::mem::swap(&mut inputs[slot - 1], &mut storage[hotbar]);
                } else {
                    storage.swap(slot - 10, hotbar);
                }
                true
            }
            _ => false,
        };
        if changed {
            self.table_inputs = inputs;
            self.slots[MAIN_START..PLAYER_WINDOW_END].clone_from_slice(&storage);
            self.carried = cursor;
            self.bump_state_id();
        }
        Ok(changed)
    }

    /// Shift-clicks the result repeatedly or transfers an ordinary slot.
    pub(crate) fn quick_move(&mut self, index: usize) -> anyhow::Result<bool> {
        self.stage(|candidate| candidate.quick_move_inner(index))
    }

    fn quick_move_inner(&mut self, index: usize) -> anyhow::Result<bool> {
        if self.game_mode == GameMode::Spectator || index >= SLOT_COUNT {
            return Ok(false);
        }
        if index == 0 {
            let mut changed = false;
            for _ in 0..64 {
                if !self.craft_once(true)? {
                    break;
                }
                changed = true;
            }
            return Ok(changed);
        }
        let range = if (MAIN_START..HOTBAR_START).contains(&index) {
            HOTBAR_START..PLAYER_WINDOW_END
        } else if (HOTBAR_START..PLAYER_WINDOW_END).contains(&index) {
            MAIN_START..HOTBAR_START
        } else {
            MAIN_START..PLAYER_WINDOW_END
        };
        let mut source = self.slots[index].clone();
        let mut destinations = self.slots[range.clone()].to_vec();
        if !ferrumc_items::transfer_into_slots(&mut source, &mut destinations, false)? {
            return Ok(false);
        }
        self.slots[index] = source;
        self.slots[range].clone_from_slice(&destinations);
        self.refresh_crafting_inner()?;
        self.bump_state_id();
        Ok(true)
    }

    /// Adopts a completed atomic container transaction's player section.
    /// The full postimage must pass `admit_container_candidate` before owner send.
    pub(crate) fn set_window_slots(&mut self, slots: Vec<ItemStack>) {
        if slots.len() == PLAYER_WINDOW_END - MAIN_START {
            for (destination, stack) in self.slots[MAIN_START..PLAYER_WINDOW_END]
                .iter_mut()
                .zip(slots)
            {
                *destination = stack;
            }
            self.bump_state_id();
        }
    }

    /// Number-key swap in writable window-0 slots; cursor is unchanged.
    pub(crate) fn hotbar_swap(&mut self, index: usize, hotbar: u8) -> anyhow::Result<bool> {
        self.stage(|candidate| candidate.hotbar_swap_inner(index, hotbar))
    }

    fn hotbar_swap_inner(&mut self, index: usize, hotbar: u8) -> anyhow::Result<bool> {
        if self.game_mode == GameMode::Spectator
            || hotbar >= 9
            || index == 0
            || index >= SLOT_COUNT
            || (HELMET_SLOT..=BOOTS_SLOT).contains(&index)
        {
            return Ok(false);
        }
        self.slots.swap(index, HOTBAR_START + usize::from(hotbar));
        self.refresh_crafting_inner()?;
        self.bump_state_id();
        Ok(true)
    }

    /// Returns unfinished crafting inputs to storage, retaining overflow in its
    /// original input slot because ground-item drops are not implemented yet.
    /// The result slot is never deposited: it has no authoritative recipe.
    pub(crate) fn return_crafting_inputs(&mut self) {
        let _ = self.stage(|candidate| {
            candidate.return_crafting_inputs_inner();
            Ok(())
        });
    }

    fn return_crafting_inputs_inner(&mut self) {
        self.slots[0] = ItemStack::empty();
        for index in 1..=4 {
            let input = std::mem::replace(&mut self.slots[index], ItemStack::empty());
            self.slots[index] = self.deposit_inner(input);
        }
    }

    /// Returns carried items to storage slots when possible, retaining overflow.
    /// Repeated attempts cannot create or discard a stack, including components.
    pub(crate) fn return_carried(&mut self) {
        let _ = self.stage(|candidate| {
            candidate.return_carried_inner();
            Ok(())
        });
    }

    fn return_carried_inner(&mut self) {
        for index in 0..9 {
            let input = std::mem::replace(&mut self.table_inputs[index], ItemStack::empty());
            self.table_inputs[index] = self.deposit_inner(input);
        }
        let carried = std::mem::replace(&mut self.carried, ItemStack::empty());
        self.carried = self.deposit_inner(carried);
    }

    /// The 36-slot main-inventory + hotbar region (`MAIN_START..PLAYER_WINDOW_END`)
    /// a container window mirrors after its own slots.
    ///
    /// The slice order — main grid then hotbar — is exactly the order the
    /// `generic_9x3` window expects for its player-inventory section, so the
    /// container payload builder can append it verbatim.
    pub(crate) fn window_slots(&self) -> &[ItemStack] {
        &self.slots[MAIN_START..PLAYER_WINDOW_END]
    }

    /// Deposits `cursor` back into the main inventory + hotbar, returning whatever
    /// could not fit. The sole cursor owner must retain this remainder.
    ///
    /// Used when a container window closes so a carried item is never lost: it
    /// first merges into matching stacks, then fills empty slots, applying the
    /// item-count-conserving [`left_click_exchange`] only to slots that are empty
    /// or hold the same item (never a different-item slot, which would swap an
    /// unrelated item onto the cursor). Bumps the window state id when anything was
    /// stored so the follow-up window-0 resync carries a fresh id. A non-empty
    /// return means the inventory was full; [`Self::return_carried`] retains it.
    #[must_use]
    pub(crate) fn deposit(&mut self, cursor: ItemStack) -> ItemStack {
        let original = cursor.clone();
        self.stage(|candidate| Ok(candidate.deposit_inner(cursor)))
            .unwrap_or(original)
    }

    fn deposit_inner(&mut self, mut cursor: ItemStack) -> ItemStack {
        if cursor.item().is_none() {
            return cursor;
        }
        if std::iter::once(&cursor)
            .chain(self.window_slots())
            .any(|stack| ferrumc_items::stack_capacity(stack).is_err())
        {
            return cursor;
        }
        let original = cursor.clone();
        let mut slots = self.window_slots().to_vec();
        match ferrumc_items::transfer_into_slots(&mut cursor, &mut slots, false) {
            Ok(true) => {
                self.slots[MAIN_START..PLAYER_WINDOW_END].clone_from_slice(&slots);
                self.bump_state_id();
                cursor
            }
            Ok(false) => cursor,
            Err(_) => original,
        }
    }

    /// The stack in the currently selected hotbar slot.
    ///
    /// `selected` is always `0..=8`, so the computed index is always a valid
    /// hotbar slot (`36..=44`).
    pub(crate) fn held(&self) -> &ItemStack {
        &self.slots[HOTBAR_START + self.selected as usize]
    }

    /// Sets the selected hotbar index, ignoring an out-of-range value.
    ///
    /// Returns `true` if `index` was a valid hotbar index (`0..=8`) and the
    /// selection was updated, `false` otherwise (selection unchanged).
    pub(crate) fn set_selected(&mut self, index: u8) -> bool {
        if index < HOTBAR_LEN {
            self.selected = index;
            true
        } else {
            false
        }
    }

    /// Preflights every persisted slot, cursor and table input before owner food
    /// admission. An overflowed bowl/bottle refuses; no remainder is discarded.
    pub(crate) fn prepare_consumption(
        &self,
        slot: usize,
        food: ferrumc_items::FoodProperties,
    ) -> anyhow::Result<Option<PreparedConsumption>> {
        let Some(stack) = self.slots.get(slot) else {
            return Ok(None);
        };
        if stack.item().is_none() || self.game_mode == GameMode::Spectator {
            return Ok(None);
        }
        let mut slots = self.slots.clone();
        let carried = self.carried.clone();
        if self.game_mode != GameMode::Creative {
            let (source, _) = ferrumc_items::plan_split(stack, 1)?.into_stacks();
            slots[slot] = source;
            if let Some(item) = food.remainder() {
                let mut remainder = ItemStack::new(
                    item,
                    std::num::NonZeroU8::MIN,
                    ferrumc_items::ComponentPatch::empty(),
                );
                if slots[slot].item().is_none() {
                    slots[slot] = remainder;
                } else {
                    ferrumc_items::transfer_into_slots(
                        &mut remainder,
                        &mut slots[MAIN_START..PLAYER_WINDOW_END],
                        false,
                    )?;
                    if remainder.item().is_some() {
                        return Ok(None);
                    }
                }
            }
        }
        crate::inventory_budget::admit(
            slots
                .iter()
                .chain(std::iter::once(&carried))
                .chain(self.table_inputs.iter()),
        )?;
        Ok(Some(PreparedConsumption { slots, carried }))
    }

    /// Prepares the exact one-item placement debit while the connection retains
    /// exclusive inventory ownership until the tracked world result arrives.
    pub(crate) fn prepare_placement(
        &self,
        hand: i32,
    ) -> anyhow::Result<Option<PreparedConsumption>> {
        let slot = match hand {
            0 => HOTBAR_START + usize::from(self.selected),
            1 => OFFHAND_SLOT,
            _ => return Ok(None),
        };
        if self.game_mode == GameMode::Spectator || self.slots[slot].item().is_none() {
            return Ok(None);
        }
        let mut slots = self.slots.clone();
        slots[slot] = ferrumc_items::plan_split(&self.slots[slot], 1)?
            .into_stacks()
            .0;
        let carried = self.carried.clone();
        crate::inventory_budget::admit(
            slots
                .iter()
                .chain(std::iter::once(&carried))
                .chain(self.table_inputs.iter()),
        )?;
        Ok(Some(PreparedConsumption { slots, carried }))
    }

    /// Prepares a complete bucket postimage before admitting any fluid change.
    /// A full destination refuses instead of discarding the filled result. The
    /// Creative duplicate policy follows ItemUtils.createFilledResult.
    pub(crate) fn prepare_bucket(
        &self,
        hand: i32,
        fill: bool,
    ) -> anyhow::Result<Option<PreparedConsumption>> {
        let slot = match hand {
            0 => HOTBAR_START + usize::from(self.selected),
            1 => OFFHAND_SLOT,
            _ => return Ok(None),
        };
        if matches!(self.game_mode, GameMode::Spectator | GameMode::Adventure) {
            return Ok(None);
        }
        let source_name = if fill { "bucket" } else { "water_bucket" };
        let destination_name = if fill { "water_bucket" } else { "bucket" };
        let source = ferrumc_items::ItemId::from_name(source_name)
            .ok_or_else(|| anyhow::anyhow!("missing bucket item"))?;
        if self.slots[slot].item() != Some(source) {
            return Ok(None);
        }
        // New vanilla bucket results carry default components; the residual
        // empty bucket stack retains its exact ordered patch via plan_split.
        let destination = ferrumc_items::ItemId::from_name(destination_name)
            .ok_or_else(|| anyhow::anyhow!("missing bucket result"))?;
        let mut result = ItemStack::new(
            destination,
            std::num::NonZeroU8::MIN,
            ferrumc_items::ComponentPatch::empty(),
        );
        let mut slots = self.slots.clone();
        if self.game_mode == GameMode::Creative {
            if fill
                && !slots[HELMET_SLOT..SLOT_COUNT].iter().any(|stack| {
                    stack.item() == result.item() && stack.components() == result.components()
                })
            {
                ferrumc_items::transfer_into_slots(
                    &mut result,
                    &mut slots[MAIN_START..PLAYER_WINDOW_END],
                    false,
                )?;
                if result.item().is_some() {
                    return Ok(None);
                }
            }
        } else {
            slots[slot] = ferrumc_items::plan_split(&self.slots[slot], 1)?
                .into_stacks()
                .0;
            if slots[slot].item().is_none() {
                slots[slot] = result;
            } else {
                ferrumc_items::transfer_into_slots(
                    &mut result,
                    &mut slots[MAIN_START..PLAYER_WINDOW_END],
                    false,
                )?;
                if result.item().is_some() {
                    return Ok(None);
                }
            }
        }
        let carried = self.carried.clone();
        crate::inventory_budget::admit(
            slots
                .iter()
                .chain(std::iter::once(&carried))
                .chain(self.table_inputs.iter()),
        )?;
        Ok(Some(PreparedConsumption { slots, carried }))
    }

    /// Publishes the exact preflighted snapshot after an accepted owner result.
    pub(crate) fn commit_consumption(&mut self, prepared: PreparedConsumption) {
        self.slots = prepared.slots;
        self.carried = prepared.carried;
        self.bump_state_id();
    }

    /// Adopts the already-budgeted held-drop postimage after genuine driver
    /// acceptance and exact beforeimage comparison in the serial handler.
    pub(crate) fn commit_held_drop(&mut self, slot: usize, stack: ItemStack) {
        self.slots[slot] = stack;
        self.bump_state_id();
    }

    /// Adopts the complete preflighted empty image after whole death-batch acceptance.
    pub(crate) fn commit_death(
        &mut self,
        cleared: crate::death_inventory::ClearedInventory,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            cleared.matches(self)?,
            "accepted death batch differs from pinned inventory"
        );
        let (slots, carried, table_inputs) = cleared.into_parts();
        self.slots = slots;
        self.carried = carried;
        self.table_inputs = table_inputs;
        self.bump_state_id();
        Ok(())
    }

    /// Applies wear only to the exact FCIS beforeimage retained for this attempt.
    /// An ambiguous moved stack refuses wear rather than choosing another tool.
    pub(crate) fn damage_witness(
        &mut self,
        witness: &crate::mining_witness::ToolWitness,
        amount: u32,
    ) -> anyhow::Result<bool> {
        let Some(plan) = crate::mining_wear::plan_wear(self, witness, amount)? else {
            return Ok(false);
        };
        self.apply_wear_plan(plan)?;
        Ok(true)
    }

    /// Captures combat only after admitting every future DAMAGE postimage.
    pub(crate) fn combat_snapshot(&self) -> anyhow::Result<crate::combat_wear::CombatWearSnapshot> {
        self.admit_budget()?;
        crate::combat_wear::CombatWearSnapshot::capture(self)
    }

    /// Checks the full persisted budget before adopting one exact wear postimage.
    pub(crate) fn apply_wear_plan(
        &mut self,
        plan: crate::mining_wear::MiningWearPlan,
    ) -> anyhow::Result<()> {
        self.apply_wear_plans(vec![plan])
    }

    /// Adopts all armor postimages only after the complete save budget is admitted.
    pub(crate) fn apply_armor_wear(
        &mut self,
        plan: crate::combat_wear::ArmorWearPlan,
    ) -> anyhow::Result<bool> {
        let changed = plan.iter().next().is_some();
        self.apply_wear_plans(plan.into_postimages().collect())?;
        Ok(changed)
    }

    fn apply_wear_plans(
        &mut self,
        plans: Vec<crate::mining_wear::MiningWearPlan>,
    ) -> anyhow::Result<()> {
        use crate::mining_wear::WearLocation;
        if plans.is_empty() {
            return Ok(());
        }
        let stacks = self
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| (WearLocation::Slot(i), s))
            .chain(std::iter::once((WearLocation::Cursor, &self.carried)))
            .chain(
                self.table_inputs
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (WearLocation::TableInput(i), s)),
            );
        crate::inventory_budget::admit(stacks.map(|(at, stack)| {
            plans
                .iter()
                .find(|p| p.location() == at)
                .map_or(stack, crate::mining_wear::MiningWearPlan::stack_after)
        }))?;
        for plan in plans {
            let location = plan.location();
            tracing::trace!(
                broken = plan.broken(),
                ?location,
                "applying committed exact item wear"
            );
            let after = plan.into_stack();
            match location {
                WearLocation::Slot(index) => self.slots[index] = after,
                WearLocation::Cursor => self.carried = after,
                WearLocation::TableInput(index) => self.table_inputs[index] = after,
            }
        }
        self.bump_state_id();
        Ok(())
    }

    /// Stores the (already-validated) `stack` into slot `index` and bumps the
    /// state id.
    ///
    /// The caller must bounds-check `index` (`0..SLOT_COUNT`) and validate `stack`
    /// through `ferrumc_items` first; an out-of-range index is a no-op.
    pub(crate) fn set_creative_slot(&mut self, index: usize, stack: ItemStack) {
        if index >= SLOT_COUNT {
            return;
        }
        let _ = self.stage(|candidate| {
            candidate.slots[index] = stack;
            candidate.bump_state_id();
            Ok(())
        });
    }

    /// Advances the window state id (wrapping), used after a resync so the client
    /// validates subsequent clicks against the new value.
    pub(crate) fn bump_state_id(&mut self) {
        self.state_id = self.state_id.wrapping_add(1);
    }

    /// Builds the `SetContainerContent` body for window 0: all [`SLOT_COUNT`]
    /// slots followed by the authoritative carried item.
    ///
    /// # Errors
    ///
    /// Propagates an NBT encoding error from a component (none in the starter
    /// kit), or [`ItemValidationError::WindowTooLarge`] — unreachable here since
    /// [`SLOT_COUNT`] is far below the window cap.
    pub(crate) fn container_content_payload(&self) -> Result<Vec<u8>, ItemValidationError> {
        encode_container_content_payload(&self.slots, &self.carried)
    }

    /// Whether inventory slot `index` is mirrored into the player's *visible*
    /// equipment — the held main hand, the off hand, or one of the four armor
    /// pieces — so a change to it must be rebroadcast to viewers via `SetEquipment`.
    ///
    /// The main-hand slot is the currently selected hotbar slot, so changing the
    /// selection (or the item in the selected slot) flips which slot this reports.
    pub(crate) fn is_equipment_slot(&self, index: usize) -> bool {
        matches!(
            index,
            HELMET_SLOT | CHESTPLATE_SLOT | LEGGINGS_SLOT | BOOTS_SLOT | OFFHAND_SLOT
        ) || index == HOTBAR_START + self.selected as usize
    }

    /// Builds the opaque `SetEquipment` body for the player's full visible
    /// equipment set: the main hand, the off hand, and the four armor pieces, in
    /// ascending equipment-slot-id order.
    ///
    /// Each entry is a slot byte (the equipment-slot id, with
    /// [`EQUIPMENT_CONTINUATION_BIT`] set on every entry except the last) followed
    /// by the trusted [`ItemStack::encode_slot`] of the source inventory slot. Every
    /// entry is *always* emitted: an empty source slot encodes as an air Slot, so
    /// removing a piece of armor (or the off-hand item) visibly clears it on viewers
    /// rather than leaving a stale render.
    ///
    /// The session/router layer carries this opaque (it has no `ferrumc-items`
    /// dependency) and only prepends the router-owned entity id.
    ///
    /// # Errors
    ///
    /// Propagates an [`ItemValidationError`] from encoding any source [`ItemStack`]
    /// (e.g. an NBT component failure); the default creative-kit items never error.
    pub(crate) fn equipment_body(&self) -> Result<Vec<u8>, ItemValidationError> {
        // (equipment-slot id, source inventory index) in ascending equipment-slot
        // order. The main hand tracks the selected hotbar slot; the rest are fixed.
        let entries = [
            (EQUIP_MAIN_HAND, HOTBAR_START + self.selected as usize),
            (EQUIP_OFF_HAND, OFFHAND_SLOT),
            (EQUIP_BOOTS, BOOTS_SLOT),
            (EQUIP_LEGGINGS, LEGGINGS_SLOT),
            (EQUIP_CHESTPLATE, CHESTPLATE_SLOT),
            (EQUIP_HELMET, HELMET_SLOT),
        ];
        let mut body = Vec::new();
        let last = entries.len() - 1;
        for (position, (equip_slot, inv_index)) in entries.iter().enumerate() {
            // Continuation bit on every entry but the last marks the terminator.
            let slot_byte = if position == last {
                *equip_slot
            } else {
                *equip_slot | EQUIPMENT_CONTINUATION_BIT
            };
            body.push(slot_byte);
            // Indices are all in `0..SLOT_COUNT`; a defensive miss encodes as air.
            match self.slots.get(*inv_index) {
                Some(stack) => stack.encode_slot(&mut body)?,
                None => ItemStack::empty().encode_slot(&mut body)?,
            }
        }
        Ok(body)
    }
}

/// Normal right-click: pick up the larger half, place one, or swap unlike items.
/// Both new stacks are built before assigning so even a validation error cannot
/// partially consume an item. Stack caps follow the existing registry contract.
#[cfg(test)]
fn right_click_exchange(slot: &mut ItemStack, cursor: &mut ItemStack) -> anyhow::Result<()> {
    ferrumc_items::exchange_slot(slot, cursor, true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stone is item id `1`, glass `195`, sand `59` in the pinned registry.
    const STONE_ITEM: i32 = 1;
    const GLASS_ITEM: i32 = 195;
    const SAND_ITEM: i32 = 59;

    fn item_id_at(inv: &PlayerInventory, slot: usize) -> Option<i32> {
        inv.slot(slot).and_then(ItemStack::item).map(ItemId::id)
    }

    fn named_stack(count: u8, name: &str) -> ItemStack {
        let mut text = ferrumc_nbt::NbtCompound::new();
        text.push("text", ferrumc_nbt::NbtTag::String(name.to_string()));
        ItemStack::try_new(
            ItemId::from_name("stone").expect("stone"),
            count,
            ComponentPatch::new(
                vec![ferrumc_items::ComponentValue::CustomName(
                    ferrumc_nbt::NbtTag::Compound(text),
                )],
                Vec::new(),
            ),
        )
        .expect("valid stack")
    }

    #[test]
    fn three_by_three_tools_and_bucket_remainders_are_atomic() {
        let mut inventory = PlayerInventory::from_persisted(
            GameMode::Survival,
            0,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        );
        let make = |name: &str, count| {
            ItemStack::try_new(
                ItemId::from_name(name).unwrap(),
                count,
                ComponentPatch::empty(),
            )
            .unwrap()
        };
        inventory.table_inputs[0] = make("iron_ingot", 2);
        inventory.table_inputs[1] = make("iron_ingot", 2);
        inventory.table_inputs[2] = make("iron_ingot", 2);
        inventory.table_inputs[4] = make("stick", 2);
        inventory.table_inputs[7] = make("stick", 2);
        inventory.carried = make("stone", 64);
        let before = inventory.table_inputs.clone();
        assert!(!inventory.table_click(0, 0, 0).unwrap());
        assert_eq!(inventory.table_inputs, before);
        inventory.carried = ItemStack::empty();
        assert!(inventory.table_click(0, 0, 1).unwrap());
        assert!(inventory
            .table_inputs
            .iter()
            .all(|item| item.item().is_none()));
        assert_eq!(
            inventory
                .window_slots()
                .iter()
                .filter(|item| item.item() == ItemId::from_name("iron_pickaxe"))
                .count(),
            2
        );
        inventory.table_inputs = [
            make("milk_bucket", 1),
            make("milk_bucket", 1),
            make("milk_bucket", 1),
            make("sugar", 1),
            make("egg", 1),
            make("sugar", 1),
            make("wheat", 1),
            make("wheat", 1),
            make("wheat", 1),
        ];
        assert!(inventory.table_click(0, 0, 0).unwrap());
        assert_eq!(inventory.carried.item(), ItemId::from_name("cake"));
        for index in 0..3 {
            assert_eq!(
                inventory.table_inputs[index].item(),
                ItemId::from_name("bucket")
            );
        }
        assert!(inventory.table_inputs[3..]
            .iter()
            .all(|stack| stack.item().is_none()));
        for index in MAIN_START..PLAYER_WINDOW_END {
            inventory.slots[index] = make("stone", 64);
        }
        let before = inventory.table_inputs.clone();
        inventory.return_carried();
        assert_eq!(inventory.table_inputs, before);
        assert_eq!(inventory.carried.item(), ItemId::from_name("cake"));
    }

    #[test]
    fn two_by_two_crafting_consumes_once_and_refuses_a_full_cursor() {
        let mut inventory = PlayerInventory::from_persisted(
            GameMode::Survival,
            0,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        );
        let log = ItemStack::try_new(
            ItemId::from_name("oak_log").unwrap(),
            2,
            ComponentPatch::new(vec![ferrumc_items::ComponentValue::Damage(3)], vec![]),
        )
        .unwrap();
        inventory.slots[4] = log.clone();
        inventory.refresh_crafting().unwrap();
        assert_eq!(inventory.slots[0].item(), ItemId::from_name("oak_planks"));
        assert_eq!(inventory.slots[0].count(), 4);
        assert!(inventory.normal_click(0, 0).unwrap());
        assert_eq!(inventory.slots[4].count(), 1);
        assert_eq!(inventory.slots[4].components(), log.components());
        assert_eq!(inventory.carried.count(), 4);
        inventory.carried = ItemStack::try_new(
            ItemId::from_name("oak_planks").unwrap(),
            64,
            ComponentPatch::empty(),
        )
        .unwrap();
        let before = inventory.slots.clone();
        assert!(!inventory.normal_click(0, 1).unwrap());
        assert_eq!(inventory.slots, before);
        inventory.carried = ItemStack::empty();
        assert!(inventory.normal_click(0, 1).unwrap());
        assert!(inventory.slots[4].item().is_none());
        assert!(inventory.slots[0].item().is_none());
        assert_eq!(inventory.carried.count(), 4);
    }

    #[test]
    fn right_click_splits_every_stack_count_and_preserves_components() {
        for count in 1..=64 {
            let original = named_stack(count, "source");
            let mut slot = original.clone();
            let mut cursor = ItemStack::empty();
            right_click_exchange(&mut slot, &mut cursor).expect("split");
            assert_eq!(slot.count(), count / 2);
            assert_eq!(cursor.count(), count.div_ceil(2));
            assert_eq!(cursor.components(), original.components());
            if slot.item().is_some() {
                assert_eq!(slot.components(), original.components());
            }
            left_click_exchange(&mut slot, &mut cursor);
            assert_eq!(slot, original);
            assert_eq!(cursor, ItemStack::empty());
        }
    }

    #[test]
    fn right_click_places_one_and_caps_merges_without_losing_the_cursor() {
        let original = named_stack(64, "source");
        let mut cursor = original.clone();
        let mut slot = ItemStack::empty();
        for placed in 1..=64 {
            right_click_exchange(&mut slot, &mut cursor).expect("place one");
            assert_eq!(slot.count(), placed);
            assert_eq!(cursor.count(), 64 - placed);
            assert_eq!(slot.components(), original.components());
            assert!(slot.is_valid() && cursor.is_valid());
        }
        let extra = named_stack(7, "source");
        cursor = extra.clone();
        right_click_exchange(&mut slot, &mut cursor).expect("full merge");
        assert_eq!(slot, original);
        assert_eq!(cursor, extra);
    }

    #[test]
    fn both_buttons_swap_differing_components_instead_of_merging_them() {
        for button in [0, 1] {
            let mut inventory = PlayerInventory::with_creative_kit(GameMode::Survival);
            let first = named_stack(4, "first");
            let second = named_stack(7, "second");
            inventory.slots[9] = first.clone();
            inventory.carried = second.clone();
            let state = inventory.state_id();
            assert!(inventory.normal_click(9, button).expect("swap"));
            assert_eq!(inventory.slot(9), Some(&second));
            assert_eq!(inventory.carried(), &first);
            assert_eq!(inventory.state_id(), state + 1);
        }
    }

    #[test]
    fn normal_click_slot_roles_and_spectator_gate_are_conservative() {
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Survival);
        let item = named_stack(3, "carried");
        inventory.carried = item.clone();
        for index in [
            0,
            HELMET_SLOT,
            CHESTPLATE_SLOT,
            LEGGINGS_SLOT,
            BOOTS_SLOT,
            SLOT_COUNT,
        ] {
            for button in [0, 1, -1, 2] {
                assert!(!inventory.normal_click(index, button).expect("refusal"));
                assert_eq!(inventory.carried(), &item);
                assert_eq!(inventory.state_id(), INITIAL_STATE_ID);
            }
        }
        assert!(inventory.normal_click(OFFHAND_SLOT, 1).expect("offhand"));
        assert_eq!(inventory.slot(OFFHAND_SLOT).expect("offhand").count(), 1);
        assert_eq!(inventory.carried().count(), 2);
        inventory.carried = ItemStack::empty();
        inventory.slots[HELMET_SLOT] = item.clone();
        assert!(inventory
            .normal_click(HELMET_SLOT, 0)
            .expect("armor removal"));
        assert_eq!(inventory.carried(), &item);
        inventory.set_game_mode(GameMode::Spectator);
        let before = inventory.slots.clone();
        let state = inventory.state_id();
        for index in 0..SLOT_COUNT {
            for button in [0, 1] {
                assert!(!inventory.normal_click(index, button).expect("spectator"));
            }
        }
        assert_eq!(inventory.slots, before);
        assert_eq!(inventory.carried(), &item);
        assert_eq!(inventory.state_id(), state);
    }

    #[test]
    fn crafting_inputs_return_once_and_full_bag_retains_every_component() {
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Survival);
        let input = named_stack(7, "crafting");
        inventory.slots[1] = input.clone();
        inventory.return_crafting_inputs();
        assert_eq!(inventory.slot(1), Some(&ItemStack::empty()));
        assert_eq!(inventory.slot(MAIN_START), Some(&input));
        let before = inventory.slots.clone();
        inventory.return_crafting_inputs();
        assert_eq!(inventory.slots, before);
        for index in MAIN_START..PLAYER_WINDOW_END {
            inventory.slots[index] = named_stack(64, "full");
        }
        inventory.slots[2] = input.clone();
        let before = inventory.slots.clone();
        for _ in 0..3 {
            inventory.return_crafting_inputs();
            assert_eq!(inventory.slots, before);
        }
    }

    #[test]
    fn creative_kit_populates_only_the_hotbar() {
        let inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        assert_eq!(inv.state_id(), INITIAL_STATE_ID);
        assert_eq!(inv.selected(), 0);
        // The hotbar carries the kit; the first and last entries pin the layout.
        assert_eq!(item_id_at(&inv, 36), Some(STONE_ITEM));
        assert_eq!(item_id_at(&inv, 44), Some(SAND_ITEM));
        assert_eq!(inv.slot(36).unwrap().count(), KIT_STACK_COUNT);
        // Non-hotbar slots stay empty.
        for slot in 0..HOTBAR_START {
            assert!(inv.slot(slot).unwrap().item().is_none(), "slot {slot}");
        }
        assert!(inv.slot(45).unwrap().item().is_none());
    }

    #[test]
    fn held_follows_selection_and_rejects_out_of_range() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        // Default selection is hotbar slot 0 -> inventory slot 36 (stone).
        assert_eq!(inv.held().item().map(ItemId::id), Some(STONE_ITEM));
        // Selecting hotbar index 3 -> inventory slot 39 (glass).
        assert!(inv.set_selected(3));
        assert_eq!(inv.held().item().map(ItemId::id), Some(GLASS_ITEM));
        // An out-of-range index is ignored and leaves the selection untouched.
        assert!(!inv.set_selected(9));
        assert_eq!(inv.selected(), 3);
        assert_eq!(inv.held().item().map(ItemId::id), Some(GLASS_ITEM));
    }

    #[test]
    fn set_creative_slot_stores_and_bumps_state() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        let before = inv.state_id();
        let glass = ItemStack::new(
            ItemId::new(GLASS_ITEM).unwrap(),
            NonZeroU8::new(64).unwrap(),
            ComponentPatch::empty(),
        );
        inv.set_creative_slot(9, glass);
        assert_eq!(item_id_at(&inv, 9), Some(GLASS_ITEM));
        assert_eq!(inv.state_id(), before + 1);
        // An out-of-range index is a no-op (no store, no bump).
        inv.set_creative_slot(SLOT_COUNT, ItemStack::empty());
        assert_eq!(inv.state_id(), before + 1);
    }

    #[test]
    fn container_payload_encodes_every_slot() {
        let inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        let payload = inv.container_content_payload().expect("payload encodes");
        // The body starts with the slot count varint (46 < 128 -> one byte).
        assert_eq!(payload[0], SLOT_COUNT as u8);
        // 46 slots + a carried item means more than just the count byte.
        assert!(payload.len() > SLOT_COUNT);
    }

    #[test]
    fn deposit_merges_then_fills_empties_and_conserves() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        let stone_id = ItemId::new(STONE_ITEM).unwrap();
        // Seed main slot 9 with 40 stone; the hotbar already holds a 64 stone (36).
        inv.set_creative_slot(9, ItemStack::new(stone_id, nz(40), ComponentPatch::empty()));
        let before = total_stone(&inv) + 30; // we will deposit 30 more stone

        // Depositing 30 stone tops up slot 9 (40 -> 64) and leaves 6 for an empty slot.
        let leftover = inv.deposit(ItemStack::new(stone_id, nz(30), ComponentPatch::empty()));
        assert!(leftover.item().is_none(), "all 30 stone were stored");
        assert_eq!(total_stone(&inv), before, "no stone duplicated or lost");
        assert_eq!(inv.slot(9).unwrap().count(), 64, "slot 9 topped to max");
    }

    #[test]
    fn deposit_of_empty_cursor_is_a_noop() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        let before = inv.state_id();
        let leftover = inv.deposit(ItemStack::empty());
        assert!(leftover.item().is_none());
        assert_eq!(
            inv.state_id(),
            before,
            "an empty deposit does not bump state"
        );
    }

    #[test]
    fn repeated_full_bag_returns_retain_the_exact_carried_stack() {
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let stone = ItemId::new(STONE_ITEM).expect("stone");
        for index in MAIN_START..PLAYER_WINDOW_END {
            inventory.set_creative_slot(
                index,
                ItemStack::new(stone, nz(64), ComponentPatch::empty()),
            );
        }
        let carried = ItemStack::new(
            stone,
            nz(7),
            ComponentPatch::new(vec![ferrumc_items::ComponentValue::Damage(3)], Vec::new()),
        );
        inventory.set_carried(carried.clone());
        let slots_before = inventory.slots.clone();
        for _ in 0..3 {
            inventory.return_carried();
            assert_eq!(inventory.carried(), &carried);
            assert_eq!(inventory.slots, slots_before);
        }
        // A local chest player-section exchange keeps the component patch too.
        inventory.exchange_carried(MAIN_START);
        assert_eq!(inventory.slot(MAIN_START), Some(&carried));
        assert_eq!(inventory.carried(), &slots_before[MAIN_START]);
    }

    #[test]
    fn partial_return_retains_overflow_and_later_free_space_accepts_it_once() {
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let stone = ItemId::new(STONE_ITEM).expect("stone");
        for index in MAIN_START..PLAYER_WINDOW_END {
            inventory.set_creative_slot(
                index,
                ItemStack::new(stone, nz(64), ComponentPatch::empty()),
            );
        }
        inventory.set_creative_slot(
            MAIN_START,
            ItemStack::new(stone, nz(60), ComponentPatch::empty()),
        );
        inventory.set_carried(ItemStack::new(stone, nz(7), ComponentPatch::empty()));
        let conserved = total_stone(&inventory) + u32::from(inventory.carried().count());
        inventory.return_carried();
        assert_eq!(inventory.slot(MAIN_START).expect("slot").count(), 64);
        assert_eq!(inventory.carried().count(), 3);
        inventory.return_carried();
        assert_eq!(inventory.carried().count(), 3);
        assert_eq!(
            total_stone(&inventory) + u32::from(inventory.carried().count()),
            conserved
        );
        inventory.set_creative_slot(MAIN_START + 1, ItemStack::empty());
        inventory.return_carried();
        assert!(inventory.carried().item().is_none());
        assert_eq!(
            inventory.slot(MAIN_START + 1).expect("free slot").count(),
            3
        );
        let final_payload = inventory.container_content_payload().expect("payload");
        inventory.return_carried();
        assert_eq!(
            inventory.container_content_payload().expect("payload"),
            final_payload
        );
    }

    /// Total count of stone across the whole inventory.
    fn total_stone(inv: &PlayerInventory) -> u32 {
        (0..SLOT_COUNT)
            .filter_map(|i| inv.slot(i))
            .filter(|s| s.item() == ItemId::new(STONE_ITEM))
            .map(|s| u32::from(s.count()))
            .sum()
    }

    fn nz(n: u8) -> NonZeroU8 {
        NonZeroU8::new(n).unwrap()
    }

    #[test]
    fn game_mode_mirror_round_trips() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        assert_eq!(inv.game_mode(), GameMode::Creative);
        inv.set_game_mode(GameMode::Survival);
        assert_eq!(inv.game_mode(), GameMode::Survival);
    }

    /// A single decoded `SetEquipment` entry: the equipment-slot id, whether the
    /// continuation bit was set (another entry follows), and the carried item id
    /// (`None` for an empty/air slot).
    #[derive(Debug, PartialEq, Eq)]
    struct EquipEntry {
        slot: u8,
        more: bool,
        item: Option<i32>,
    }

    /// Reads a Minecraft `VarInt` (LEB128) from `buf` at `*i`, advancing the cursor.
    fn read_varint(buf: &[u8], i: &mut usize) -> i32 {
        let mut value: i32 = 0;
        let mut shift = 0;
        loop {
            let byte = buf[*i];
            *i += 1;
            value |= i32::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        value
    }

    /// Walks an opaque equipment body into its entries. Test items use an empty
    /// `ComponentPatch`, so each non-air Slot is `count, item_id, 0 added, 0 removed`.
    fn parse_equipment(body: &[u8]) -> Vec<EquipEntry> {
        let mut entries = Vec::new();
        let mut i = 0;
        loop {
            let slot_byte = body[i];
            i += 1;
            let more = slot_byte & EQUIPMENT_CONTINUATION_BIT != 0;
            let slot = slot_byte & !EQUIPMENT_CONTINUATION_BIT;
            let count = read_varint(body, &mut i);
            let item = if count == 0 {
                None
            } else {
                let id = read_varint(body, &mut i);
                let _added = read_varint(body, &mut i);
                let _removed = read_varint(body, &mut i);
                Some(id)
            };
            entries.push(EquipEntry { slot, more, item });
            if !more {
                break;
            }
        }
        entries
    }

    fn stone_stack() -> ItemStack {
        ItemStack::new(
            ItemId::new(STONE_ITEM).unwrap(),
            nz(1),
            ComponentPatch::empty(),
        )
    }

    #[test]
    fn equipment_body_emits_full_set_in_order_with_continuation_bits() {
        // Default kit: hotbar slot 0 (stone) is held; off-hand and all armor empty.
        let inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        let body = inv.equipment_body().expect("body encodes");
        let entries = parse_equipment(&body);

        // Six entries, ascending equipment-slot id, continuation bit on all but last.
        let slots: Vec<u8> = entries.iter().map(|e| e.slot).collect();
        assert_eq!(
            slots,
            vec![
                EQUIP_MAIN_HAND,
                EQUIP_OFF_HAND,
                EQUIP_BOOTS,
                EQUIP_LEGGINGS,
                EQUIP_CHESTPLATE,
                EQUIP_HELMET,
            ]
        );
        let more: Vec<bool> = entries.iter().map(|e| e.more).collect();
        assert_eq!(more, vec![true, true, true, true, true, false]);

        // Main hand carries the held stone; off-hand and armor are air.
        assert_eq!(entries[0].item, Some(STONE_ITEM));
        for entry in &entries[1..] {
            assert_eq!(entry.item, None, "slot {} should be air", entry.slot);
        }
    }

    #[test]
    fn equipment_body_reflects_armor_and_offhand_set_then_clear() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        // Place a piece in the helmet (5) and the off-hand (45).
        inv.set_creative_slot(HELMET_SLOT, stone_stack());
        inv.set_creative_slot(OFFHAND_SLOT, stone_stack());

        let entries = parse_equipment(&inv.equipment_body().expect("body encodes"));
        let by_slot = |s: u8| entries.iter().find(|e| e.slot == s).unwrap().item;
        assert_eq!(by_slot(EQUIP_HELMET), Some(STONE_ITEM));
        assert_eq!(by_slot(EQUIP_OFF_HAND), Some(STONE_ITEM));
        // Untouched armor stays air.
        assert_eq!(by_slot(EQUIP_BOOTS), None);

        // Clearing the helmet sends an air entry, not a stale render.
        inv.set_creative_slot(HELMET_SLOT, ItemStack::empty());
        let cleared = parse_equipment(&inv.equipment_body().expect("body encodes"));
        let helmet = cleared.iter().find(|e| e.slot == EQUIP_HELMET).unwrap();
        assert_eq!(helmet.item, None);
    }

    #[test]
    fn equipment_body_main_hand_tracks_selection() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        // Hotbar slot 3 holds glass; selecting it makes it the main hand.
        assert!(inv.set_selected(3));
        let entries = parse_equipment(&inv.equipment_body().expect("body encodes"));
        let main = entries.iter().find(|e| e.slot == EQUIP_MAIN_HAND).unwrap();
        assert_eq!(main.item, Some(GLASS_ITEM));
    }

    #[test]
    fn is_equipment_slot_classifies_armor_offhand_and_selected_hotbar() {
        let mut inv = PlayerInventory::with_creative_kit(GameMode::Creative);
        // Armor and off-hand are always equipment slots.
        for slot in [
            HELMET_SLOT,
            CHESTPLATE_SLOT,
            LEGGINGS_SLOT,
            BOOTS_SLOT,
            OFFHAND_SLOT,
        ] {
            assert!(inv.is_equipment_slot(slot), "slot {slot}");
        }
        // The selected hotbar slot is the main hand; siblings are not equipment.
        assert!(inv.is_equipment_slot(HOTBAR_START)); // selected == 0
        assert!(!inv.is_equipment_slot(HOTBAR_START + 1));
        // Storage grid and crafting slots are never equipment.
        assert!(!inv.is_equipment_slot(MAIN_START));
        assert!(!inv.is_equipment_slot(0));
        // Changing the selection moves which hotbar slot counts as the main hand.
        assert!(inv.set_selected(4));
        assert!(inv.is_equipment_slot(HOTBAR_START + 4));
        assert!(!inv.is_equipment_slot(HOTBAR_START));
    }
}

#[cfg(test)]
mod bucket_tests {
    use super::*;
    fn stack(name: &str, count: u8) -> ItemStack {
        ItemStack::try_new(
            ItemId::from_name(name).expect("pinned item"),
            count,
            ComponentPatch::empty(),
        )
        .expect("valid stack")
    }
    fn inventory(mode: GameMode) -> PlayerInventory {
        PlayerInventory::from_persisted(
            mode,
            0,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        )
    }
    #[test]
    fn bucket_postimage_conserves_stacked_empty_buckets_and_offhand_results() {
        let mut inv = inventory(GameMode::Survival);
        inv.slots[HOTBAR_START] = stack("bucket", 2);
        let before = inv.container_content_payload().expect("before");
        let plan = inv
            .prepare_bucket(0, true)
            .expect("plan")
            .expect("admitted");
        assert_eq!(
            inv.container_content_payload().expect("uncommitted"),
            before
        );
        inv.commit_consumption(plan);
        assert_eq!(inv.slots[HOTBAR_START], stack("bucket", 1));
        assert_eq!(inv.slots[MAIN_START], stack("water_bucket", 1));
        inv.slots[OFFHAND_SLOT] = stack("water_bucket", 1);
        let plan = inv
            .prepare_bucket(1, false)
            .expect("pour")
            .expect("admitted");
        inv.commit_consumption(plan);
        assert_eq!(inv.slots[OFFHAND_SLOT], stack("bucket", 1));
    }
    #[test]
    fn full_inventory_refuses_before_world_change_and_single_bucket_can_replace() {
        let mut inv = inventory(GameMode::Survival);
        for slot in &mut inv.slots[MAIN_START..PLAYER_WINDOW_END] {
            *slot = stack("stone", 64);
        }
        inv.slots[HOTBAR_START] = stack("bucket", 2);
        let before = inv.container_content_payload().expect("before");
        assert!(inv.prepare_bucket(0, true).expect("full").is_none());
        assert_eq!(inv.container_content_payload().expect("unchanged"), before);
        inv.slots[HOTBAR_START] = stack("bucket", 1);
        let plan = inv
            .prepare_bucket(0, true)
            .expect("single")
            .expect("replace");
        inv.commit_consumption(plan);
        assert_eq!(inv.slots[HOTBAR_START], stack("water_bucket", 1));
    }
    #[test]
    fn creative_fill_deduplicates_and_pour_preserves_held_bucket() {
        let mut inv = inventory(GameMode::Creative);
        inv.slots[HOTBAR_START] = stack("bucket", 16);
        for _ in 0..2 {
            let plan = inv
                .prepare_bucket(0, true)
                .expect("fill")
                .expect("accepted");
            inv.commit_consumption(plan);
        }
        assert_eq!(inv.slots[HOTBAR_START], stack("bucket", 16));
        assert_eq!(
            inv.slots
                .iter()
                .filter(|s| s.item() == ItemId::from_name("water_bucket"))
                .count(),
            1
        );
        inv.slots[OFFHAND_SLOT] = stack("water_bucket", 1);
        let plan = inv
            .prepare_bucket(1, false)
            .expect("pour")
            .expect("accepted");
        inv.commit_consumption(plan);
        assert_eq!(inv.slots[OFFHAND_SLOT], stack("water_bucket", 1));
        inv.set_game_mode(GameMode::Adventure);
        assert!(inv.prepare_bucket(1, false).expect("adventure").is_none());
    }
}

/// Actual schema-5 boundary fixture with four pristine armor pieces and hand.
#[cfg(test)]
pub(crate) fn budget_boundary_fixture(headroom: usize, hand: bool) -> PlayerInventory {
    use ferrumc_items::{ComponentPatch, ComponentValue, ItemId, OpaqueComponent};
    let mut slots = std::array::from_fn(|_| ItemStack::empty());
    for (index, name) in [
        (5, "iron_helmet"),
        (6, "iron_chestplate"),
        (7, "iron_leggings"),
        (8, "iron_boots"),
    ] {
        slots[index] =
            ItemStack::try_new(ItemId::from_name(name).unwrap(), 1, ComponentPatch::empty())
                .unwrap();
    }
    if hand {
        slots[36] = ItemStack::try_new(
            ItemId::from_name("iron_sword").unwrap(),
            1,
            ComponentPatch::empty(),
        )
        .unwrap();
    }
    let large = |tail| {
        let components = (6..38)
            .map(|id| {
                ComponentValue::Opaque(
                    OpaqueComponent::new(id, vec![7; if id == 37 { tail } else { 8192 }]).unwrap(),
                )
            })
            .collect();
        ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            2,
            ComponentPatch::new(components, vec![]),
        )
        .unwrap()
    };
    let mut inventory =
        PlayerInventory::from_persisted(GameMode::Survival, 0, slots, ItemStack::empty());
    for stack in &mut inventory.table_inputs[..4] {
        *stack = large(8192);
    }
    let actual = 286
        + inventory
            .slots
            .iter()
            .chain(std::iter::once(&inventory.carried))
            .chain(inventory.table_inputs.iter())
            .map(|stack| ferrumc_items::persisted_stack_size(stack).unwrap())
            .sum::<usize>();
    let excess = actual - (ferrumc_storage::MAX_PLAYER_DATA_LEN - headroom);
    assert!(excess < 8192);
    inventory.table_inputs[3] = large(8192 - excess);
    inventory
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    use crate::player_data::PlayerData;
    use ferrumc_items::{ComponentPatch, ItemId};
    use ferrumc_math::Vec3;

    fn stack(name: &str, count: u8) -> ItemStack {
        ItemStack::try_new(
            ItemId::from_name(name).unwrap(),
            count,
            ComponentPatch::empty(),
        )
        .unwrap()
    }
    fn record(inventory: &PlayerInventory) -> ferrumc_storage::PlayerRecord {
        PlayerData::capture(Vec3::ZERO, 0.0, 0.0, inventory)
            .to_record(GameMode::Survival)
            .unwrap()
    }

    #[test]
    fn budget_real_four_armor_and_hand_exact_limits_and_wear() {
        for (hand, reserve) in [(false, 52), (true, 65)] {
            let mut inventory = budget_boundary_fixture(reserve, hand);
            assert_eq!(
                record(&inventory).data().len(),
                ferrumc_storage::MAX_PLAYER_DATA_LEN - reserve
            );
            let snapshot = inventory.combat_snapshot().unwrap();
            let armor =
                crate::combat_wear::plan_armor_wear(&snapshot, &inventory, true, 4.0).unwrap();
            assert_eq!(armor.iter().count(), 4);
            inventory.apply_armor_wear(armor).unwrap();
            assert_eq!(
                record(&inventory).data().len(),
                ferrumc_storage::MAX_PLAYER_DATA_LEN - reserve + 52
            );
            if hand {
                let weapon = crate::combat_wear::plan_weapon_wear(&snapshot, &inventory, true)
                    .unwrap()
                    .unwrap();
                inventory.apply_wear_plan(weapon).unwrap();
            }
            assert_eq!(
                record(&inventory).data().len(),
                ferrumc_storage::MAX_PLAYER_DATA_LEN
            );
            inventory.admit_budget().unwrap();
            // Once DAMAGE exists, further wear cannot consume any new save bytes.
            let later = inventory.combat_snapshot().unwrap();
            let armor = crate::combat_wear::plan_armor_wear(&later, &inventory, true, 8.0).unwrap();
            inventory.apply_armor_wear(armor).unwrap();
            assert_eq!(
                record(&inventory).data().len(),
                ferrumc_storage::MAX_PLAYER_DATA_LEN
            );
            let below = budget_boundary_fixture(reserve - 1, hand);
            assert!(below.combat_snapshot().is_err());
            assert_eq!(
                record(&below).data().len(),
                ferrumc_storage::MAX_PLAYER_DATA_LEN - reserve + 1
            );
        }
    }

    #[test]
    fn budget_delayed_snapshot_growth_cannot_consume_wear_envelope() {
        let mut inventory = budget_boundary_fixture(65, true);
        let snapshot = inventory.combat_snapshot().unwrap();
        let before = record(&inventory);
        let state = inventory.state_id();
        let incoming = stack("stone", 1);
        // This small pickup fits the actual cap, but would consume wear headroom.
        let actual_growth = ferrumc_items::persisted_stack_size(&incoming).unwrap() - 7;
        assert!(actual_growth < 65);
        assert_eq!(inventory.deposit(incoming.clone()), incoming);
        inventory.set_creative_slot(9, stack("stone", 1));
        assert!(inventory.slot(9).unwrap().item().is_none());
        assert_eq!(inventory.state_id(), state);
        assert_eq!(record(&inventory).data(), before.data());
        let armor = crate::combat_wear::plan_armor_wear(&snapshot, &inventory, true, 4.0).unwrap();
        inventory.apply_armor_wear(armor).unwrap();
        let weapon = crate::combat_wear::plan_weapon_wear(&snapshot, &inventory, true)
            .unwrap()
            .unwrap();
        inventory.apply_wear_plan(weapon).unwrap();
        assert_eq!(
            record(&inventory).data().len(),
            ferrumc_storage::MAX_PLAYER_DATA_LEN
        );
    }

    #[test]
    fn budget_legacy_maximum_record_loads_but_refuses_combat_and_growth() {
        let inventory = budget_boundary_fixture(0, true);
        let saved = record(&inventory);
        assert_eq!(saved.data().len(), ferrumc_storage::MAX_PLAYER_DATA_LEN);
        let loaded = PlayerData::from_record(&saved).unwrap();
        let mut restored = loaded.restore_inventory(GameMode::Survival);
        assert_eq!(record(&restored).data(), saved.data());
        assert!(restored.combat_snapshot().is_err());
        let before = record(&restored);
        let incoming = stack("stone", 1);
        assert_eq!(restored.deposit(incoming.clone()), incoming);
        restored.set_creative_slot(9, stack("stone", 1));
        assert_eq!(record(&restored).data(), before.data());
    }

    #[test]
    fn budget_table_split_is_staged_and_complete_container_previews_refuse() {
        let mut inventory = budget_boundary_fixture(65, true);
        let before = record(&inventory);
        let state = inventory.state_id();
        assert!(inventory.table_click(1, 1, 0).is_err());
        assert_eq!(record(&inventory).data(), before.data());
        assert_eq!(inventory.state_id(), state);
        let mut window = inventory.window_slots().to_vec();
        window[0] = stack("stone", 1);
        assert!(inventory
            .admit_container_candidate(&window, inventory.carried())
            .is_err());
        assert!(inventory
            .admit_container_candidate(inventory.window_slots(), &stack("stone", 1))
            .is_err());
        let slots: [ItemStack; 36] = window.try_into().unwrap();
        assert!(crate::furnace_window::prepare_player_candidate(
            &inventory,
            &slots,
            inventory.carried(),
            Vec3::ZERO,
            0.0,
            0.0,
            ferrumc_sim::PlayerVitals::new().snapshot()
        )
        .is_err());
    }

    #[test]
    fn budget_local_split_refuses_but_conserving_moves_and_close_keep_the_envelope() {
        let mut inventory = budget_boundary_fixture(65, true);
        inventory.slots[9] = std::mem::replace(&mut inventory.table_inputs[0], ItemStack::empty());
        let before = record(&inventory);
        let state = inventory.state_id();
        assert!(
            inventory.normal_click(9, 1).is_err(),
            "split duplicates the complete patch"
        );
        assert_eq!(record(&inventory).data(), before.data());
        assert_eq!(inventory.state_id(), state);
        assert!(inventory.normal_click(9, 0).unwrap());
        assert!(inventory.slot(9).unwrap().item().is_none());
        assert_eq!(inventory.carried().count(), 2);
        inventory.admit_budget().unwrap();
        inventory.return_carried();
        assert!(inventory.carried().item().is_none());
        inventory.admit_budget().unwrap();
        assert!(inventory.quick_move(9).unwrap());
        inventory.admit_budget().unwrap();
        assert!(inventory.hotbar_swap(9, 0).unwrap());
        inventory.admit_budget().unwrap();
        inventory.return_crafting_inputs();
        inventory.admit_budget().unwrap();
    }

    #[test]
    fn budget_prepared_food_bucket_and_placement_do_not_publish_a_refused_image() {
        let mut inventory = budget_boundary_fixture(0, true);
        inventory.slots[37] = stack("mushroom_stew", 1);
        inventory.slots[38] = stack("bucket", 1);
        inventory.slots[39] = stack("stone", 2);
        // Retune exactly to the durable ceiling after adding the fixture items.
        let over = 286
            + inventory
                .slots
                .iter()
                .chain(std::iter::once(&inventory.carried))
                .chain(inventory.table_inputs.iter())
                .map(|s| ferrumc_items::persisted_stack_size(s).unwrap())
                .sum::<usize>()
            - ferrumc_storage::MAX_PLAYER_DATA_LEN;
        let last = &mut inventory.table_inputs[3];
        let mut added = last.components().added().to_vec();
        let ferrumc_items::ComponentValue::Opaque(tail) = added.pop().unwrap() else {
            panic!("tail")
        };
        added.push(ferrumc_items::ComponentValue::Opaque(
            ferrumc_items::OpaqueComponent::new(
                tail.type_id(),
                tail.raw()[..tail.raw().len() - over].to_vec(),
            )
            .unwrap(),
        ));
        *last = ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            2,
            ComponentPatch::new(added, vec![]),
        )
        .unwrap();
        let before = record(&inventory);
        let food = ferrumc_items::food_properties(&inventory.slots[37])
            .unwrap()
            .unwrap();
        assert!(inventory.prepare_consumption(37, food).is_err());
        inventory.set_selected(2);
        assert!(inventory.prepare_bucket(0, true).is_err());
        inventory.set_selected(3);
        assert!(inventory.prepare_placement(0).is_err());
        // Preparation changes no item bytes; selected is the only deliberate edit.
        inventory.set_selected(0);
        assert_eq!(record(&inventory).data(), before.data());
    }
}
