//! Per-connection open-container window state.
//!
//! The player inventory (window 0) is always open and modelled by
//! [`PlayerInventory`](crate::inventory::PlayerInventory). A *container* window —
//! a chest's `generic_9x3` screen — is opened on demand and tracked here while it
//! is on screen: the assigned window id, the chest's world position, a mirror of
//! the chest's [`CHEST_SLOTS`] item slots and the
//! window's own state id.
//!
//! The chest contents are authoritative in the simulation; this mirror exists so
//! the connection can render the window and resolve clicks. Every chest-slot
//! mutation round-trips to the simulation and the mirror is refreshed from the
//! authoritative reply, so the mirror can never drift into a dupe.

use ferrumc_items::{encode_container_content_payload, ItemStack, ItemValidationError};
use ferrumc_math::BlockPos;
use ferrumc_world::CHEST_SLOTS;

use crate::inventory::{PlayerInventory, MAIN_START, PLAYER_WINDOW_END};

/// The `minecraft:menu` registry id for a single chest's `generic_9x3` GUI.
///
/// Verified against the pinned 1.21.8 menu registry (`generic_9x3` = 2).
pub(crate) const GENERIC_9X3_TYPE: i32 = 2;

/// Pinned minecraft:crafting menu id.
pub(crate) const CRAFTING_TYPE: i32 = 12;

/// The window state id stamped on the first `SetContainerContent`, bumped on every
/// subsequent send so the client can report its latest received revision.
const INITIAL_STATE_ID: i32 = 1;

/// The number of player-inventory slots shown in a container window (main grid +
/// hotbar), appended after the container's own slots.
const PLAYER_SECTION: usize = PLAYER_WINDOW_END - MAIN_START;

/// Total slot count of an open chest window: the chest plus the player section.
pub(crate) const CHEST_WINDOW_SLOTS: usize = CHEST_SLOTS + PLAYER_SECTION;

/// Which logical slot a raw chest-window slot index addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowSlot {
    /// A chest container slot, `0..`[`CHEST_SLOTS`].
    Chest(usize),
    /// A player-inventory slot (an index into [`PlayerInventory`]).
    Player(usize),
}

/// One bounded, connection-owned ordinary chest drag; no world mutation yet.
pub(crate) struct ChestDrag {
    pub(crate) right: bool,
    pub(crate) selection: Vec<usize>,
    pub(crate) cursor: ItemStack,
}

/// A container window the player currently has open.
pub(crate) struct OpenContainer {
    window_id: i32,
    position: BlockPos,
    /// Mirror of the chest's [`CHEST_SLOTS`] slots (authoritative copy is in the sim).
    slots: Vec<ItemStack>,
    state_id: i32,
    crafting: bool,
    pub(crate) drag: Option<ChestDrag>,
}

impl OpenContainer {
    /// Opens a chest window `window_id` over the chest at `position`, seeded with
    /// the simulation's `slots` snapshot. The player inventory owns the cursor.
    fn new(window_id: i32, position: BlockPos, slots: Vec<ItemStack>) -> Self {
        Self {
            window_id,
            position,
            slots,
            state_id: INITIAL_STATE_ID,
            crafting: false,
            drag: None,
        }
    }

    /// Whether this screen is a local 3×3 crafting table.
    pub(crate) fn is_crafting(&self) -> bool {
        self.crafting
    }

    /// The assigned window id (the client echoes it on click / close).
    pub(crate) fn window_id(&self) -> i32 {
        self.window_id
    }

    /// The world position of the open chest.
    pub(crate) fn position(&self) -> BlockPos {
        self.position
    }

    /// The current window state id.
    pub(crate) fn state_id(&self) -> i32 {
        self.state_id
    }

    /// The chest snapshot last sent to this viewer, used to reject conflicting
    /// clicks before the authoritative simulation exchanges any items.
    pub(crate) fn chest_slots(&self) -> &[ItemStack] {
        &self.slots
    }

    /// Refreshes the chest-slot mirror from an authoritative sim snapshot.
    pub(crate) fn set_chest_slots(&mut self, slots: Vec<ItemStack>) {
        self.slots = slots;
    }

    /// Advances the window revision within the protocol's nonnegative 15-bit domain.
    pub(crate) fn bump_state_id(&mut self) {
        self.state_id = (self.state_id + 1) & 32767;
    }

    /// Maps a raw wire slot index to the [`WindowSlot`] it addresses, or `None`
    /// for an out-of-window index (including the `-999` click-outside sentinel).
    ///
    /// Layout (`generic_9x3`): `0..27` chest, `27..63` the player section, which
    /// maps to player-inventory indices `MAIN_START..PLAYER_WINDOW_END`.
    pub(crate) fn classify_slot(raw: i16) -> Option<WindowSlot> {
        let raw = usize::try_from(raw).ok()?;
        if raw < CHEST_SLOTS {
            Some(WindowSlot::Chest(raw))
        } else if raw < CHEST_WINDOW_SLOTS {
            // Shift past the chest section into the player-inventory window region.
            Some(WindowSlot::Player(raw - CHEST_SLOTS + MAIN_START))
        } else {
            None
        }
    }

    /// Builds the `SetContainerContent` body for this window: the chest slots, then
    /// the player's main+hotbar section, then the carried cursor item.
    ///
    /// # Errors
    ///
    /// Propagates an NBT encode error from a slot component, or
    /// [`ItemValidationError::WindowTooLarge`] — unreachable here since
    /// [`CHEST_WINDOW_SLOTS`] is far below the window cap.
    pub(crate) fn content_payload(
        &self,
        inventory: &PlayerInventory,
    ) -> Result<Vec<u8>, ItemValidationError> {
        let mut combined = Vec::with_capacity(CHEST_WINDOW_SLOTS);
        combined.extend_from_slice(&self.slots);
        combined.extend_from_slice(inventory.window_slots());
        encode_container_content_payload(&combined, inventory.carried())
    }
}

/// The connection's open-window state: at most one container window plus the
/// rolling window-id allocator.
pub(crate) struct WindowState {
    open: Option<OpenContainer>,
    pub(crate) furnace: Option<crate::furnace_window::FurnaceWindow>,
    pub(crate) furnace_poll: u64,
    next_window_id: i32,
    next_bucket_token: u64,
    next_attack_token: u64,
    /// Last admitted client ground flag; server collision verification remains separate.
    on_ground: bool,
    secondary_use: bool,
    pub(crate) consumption: crate::consumption::Consumption,
    pub(crate) sprinting: bool,
    pub(crate) mining: crate::mining_witness::MiningWitnesses,
    pub(crate) combat_profile: Option<crate::combat::CombatProfile>,
    pub(crate) combat_revision: Option<(i32, u8, ferrumc_core::GameMode)>,
}

impl WindowState {
    /// No container window open; the allocator starts at window id 1 (0 is the
    /// always-open player inventory).
    pub(crate) fn new() -> Self {
        Self {
            open: None,
            furnace: None,
            furnace_poll: 0,
            next_window_id: 1,
            next_bucket_token: 0,
            next_attack_token: 0,
            on_ground: true,
            secondary_use: false,
            consumption: crate::consumption::Consumption::new(),
            sprinting: false,
            mining: crate::mining_witness::MiningWitnesses::new(),
            combat_profile: None,
            combat_revision: None,
        }
    }

    /// Monotonic identity for one serial connection-pinned attack.
    pub(crate) fn attack_token(&mut self) -> anyhow::Result<u64> {
        self.next_attack_token = self
            .next_attack_token
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("attack token exhausted"))?;
        Ok(self.next_attack_token)
    }

    /// Monotonic identity for one pinned, serial bucket transaction.
    pub(crate) fn bucket_token(&mut self) -> anyhow::Result<u64> {
        self.next_bucket_token = self
            .next_bucket_token
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("bucket token exhausted"))?;
        Ok(self.next_bucket_token)
    }

    /// Secondary-use input is the protocol-772 shift bit, not an obsolete action ID.
    pub(crate) fn set_secondary_use(&mut self, value: bool) {
        self.secondary_use = value;
    }
    pub(crate) fn secondary_use(&self) -> bool {
        self.secondary_use
    }

    /// Records the ground flag only after validated movement admission.
    pub(crate) fn set_on_ground(&mut self, value: bool) {
        self.on_ground = value;
    }

    /// Last admitted ground flag for ordinary mining penalties.
    pub(crate) fn on_ground(&self) -> bool {
        self.on_ground
    }

    /// Allocates the next container window id, cycling `1..=99` so a click for a
    /// just-closed window cannot be confused with a freshly opened one.
    pub(crate) fn allocate_window_id(&mut self) -> i32 {
        let id = self.next_window_id;
        self.next_window_id = if id >= 99 { 1 } else { id + 1 };
        id
    }

    /// Opens a chest window over `position` seeded with `slots`, returning its
    /// assigned id. Replacing window metadata never changes the player-owned
    /// carried item; the caller resynchronizes that same item into the new screen.
    pub(crate) fn open_chest(&mut self, position: BlockPos, slots: Vec<ItemStack>) -> i32 {
        let window_id = self.allocate_window_id();
        self.open = Some(OpenContainer::new(window_id, position, slots));
        window_id
    }

    /// Opens a crafting-table screen over its validated world position.
    pub(crate) fn open_crafting(&mut self, position: BlockPos) -> i32 {
        let window_id = self.allocate_window_id();
        let mut open = OpenContainer::new(window_id, position, Vec::new());
        open.crafting = true;
        self.open = Some(open);
        window_id
    }

    /// Whether a chest, crafting table or furnace screen is active.
    pub(crate) fn has_open(&self) -> bool {
        self.open.is_some() || self.furnace.is_some()
    }

    /// Current shared screen id, including the always-open player inventory.
    pub(crate) fn current_window_id(&self) -> i32 {
        self.furnace.as_ref().map_or_else(
            || self.open.as_ref().map_or(0, OpenContainer::window_id),
            crate::furnace_window::FurnaceWindow::window_id,
        )
    }

    /// The open container window, if any.
    pub(crate) fn open(&self) -> Option<&OpenContainer> {
        self.open.as_ref()
    }

    /// The open container window mutably, if any.
    pub(crate) fn open_mut(&mut self) -> Option<&mut OpenContainer> {
        self.open.as_mut()
    }

    /// Takes (closes) the open container window, if any.
    pub(crate) fn take(&mut self) -> Option<OpenContainer> {
        self.furnace = None;
        self.open.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_slot_splits_chest_and_player_sections() {
        // Chest slots map 1:1.
        assert_eq!(OpenContainer::classify_slot(0), Some(WindowSlot::Chest(0)));
        assert_eq!(
            OpenContainer::classify_slot(26),
            Some(WindowSlot::Chest(26))
        );
        // The player section starts right after the chest at inventory MAIN_START.
        assert_eq!(
            OpenContainer::classify_slot(27),
            Some(WindowSlot::Player(MAIN_START))
        );
        // The last hotbar slot is inventory PLAYER_WINDOW_END - 1.
        let last = i16::try_from(CHEST_WINDOW_SLOTS - 1).unwrap();
        assert_eq!(
            OpenContainer::classify_slot(last),
            Some(WindowSlot::Player(PLAYER_WINDOW_END - 1))
        );
        // Out-of-window indices (and the -999 click-outside sentinel) are rejected.
        let past_end = i16::try_from(CHEST_WINDOW_SLOTS).unwrap();
        assert_eq!(OpenContainer::classify_slot(past_end), None);
        assert_eq!(OpenContainer::classify_slot(-1), None);
        assert_eq!(OpenContainer::classify_slot(-999), None);
    }

    #[test]
    fn window_ids_cycle_and_skip_zero() {
        let mut state = WindowState::new();
        let first = state.open_chest(
            BlockPos::new(0, 0, 0),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        assert_eq!(first, 1);
        let second = state.open_chest(
            BlockPos::new(0, 0, 0),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        assert_eq!(second, 2, "each open allocates a fresh id");
    }

    #[test]
    fn content_payload_has_all_window_slots_plus_cursor() {
        use ferrumc_core::GameMode;

        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let carried = ItemStack::try_new(
            ferrumc_items::ItemId::from_name("stone").expect("stone"),
            7,
            ferrumc_items::ComponentPatch::empty(),
        )
        .expect("carried");
        inventory.set_carried(carried.clone());
        let open = OpenContainer::new(
            1,
            BlockPos::new(0, 0, 0),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        let payload = open.content_payload(&inventory).expect("payload encodes");
        // The body starts with the slot-count varint (63 < 128 -> one byte).
        assert_eq!(payload[0], CHEST_WINDOW_SLOTS as u8);
        let mut encoded_carried = Vec::new();
        carried
            .encode_slot(&mut encoded_carried)
            .expect("cursor encodes");
        assert!(payload.ends_with(&encoded_carried));
        assert!(inventory
            .container_content_payload()
            .expect("window zero")
            .ends_with(&encoded_carried));
        let mut windows = WindowState::new();
        windows.open_chest(
            BlockPos::new(0, 0, 0),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        windows.take();
        windows.open_chest(
            BlockPos::new(1, 0, 0),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        assert!(windows
            .open()
            .expect("replacement")
            .content_payload(&inventory)
            .expect("payload")
            .ends_with(&encoded_carried));
    }
}
