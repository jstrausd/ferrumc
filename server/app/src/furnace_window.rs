//! Connection-owned immutable mirror of one authoritative ordinary furnace.
//! The simulation owns furnace contents/timers; `PlayerInventory` owns the cursor.
//! Mirror refreshes and packet construction grant no exchange/commit authority.

use ferrumc_items::{
    encode_container_content_payload, persisted_stack_size, same_stack_representation,
    stack_capacity, ItemStack, ItemValidationError,
};
use ferrumc_math::{BlockPos, Vec3};
use ferrumc_nbt::{NbtCompound, NbtTag};
use ferrumc_proto::generated::play::{OpenScreen, SetContainerContent, SetContainerProperty};
use ferrumc_sim::VitalsSnapshot;
use ferrumc_storage::MAX_PLAYER_DATA_LEN;
use ferrumc_world::FurnaceInventory;

use crate::inventory::{PlayerInventory, MAIN_START, PLAYER_WINDOW_END, SLOT_COUNT};
use crate::player_data::PlayerData;

/// Original pinned 1.21.8 minecraft:menu registry id (enchantment is thirteen).
pub(crate) const FURNACE_TYPE: i32 = 14;
/// Three furnace slots followed by the main/hotbar player's thirty-six slots.
pub(crate) const FURNACE_WINDOW_SLOTS: usize = 39;

/// One raw furnace-window slot mapped to its authoritative owner's index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FurnaceWindowSlot {
    /// Input/fuel/output indices zero/one/two.
    Furnace(usize),
    /// Player window-0 main/hotbar index in nine through forty-four.
    Player(usize),
}

/// One viewer's furnace mirror and bounded protocol identity.
/// Contents are read-only snapshots, never a second mutable furnace owner.
pub(crate) struct FurnaceWindow {
    window_id: i32,
    position: BlockPos,
    snapshot: FurnaceInventory,
    state_id: i32,
}

impl FurnaceWindow {
    /// Adopt a validated authoritative snapshot under a Root-allocated id1..99.
    pub(crate) fn new(
        window_id: i32,
        position: BlockPos,
        snapshot: FurnaceInventory,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!((1..=99).contains(&window_id), "invalid furnace window id");
        Ok(Self {
            window_id,
            position,
            snapshot,
            state_id: 1,
        })
    }

    /// Identity echoed by client click/close and shared with all screen packets.
    pub(crate) fn window_id(&self) -> i32 {
        self.window_id
    }

    /// Authoritative furnace location whose screen this mirror describes.
    pub(crate) fn position(&self) -> BlockPos {
        self.position
    }

    /// Last published item-state revision; timer properties do not advance it.
    pub(crate) fn state_id(&self) -> i32 {
        self.state_id
    }

    /// The most recent authoritative contents and timer snapshot.
    pub(crate) fn snapshot(&self) -> &FurnaceInventory {
        &self.snapshot
    }

    /// Compare quantities and exact ordered component bits, excluding timers.
    /// Normal burn/cook timer ticks must not make every in-flight click stale.
    pub(crate) fn matches_contents(&self, snapshot: &FurnaceInventory) -> anyhow::Result<bool> {
        for (before, after) in self.snapshot.slots().iter().zip(snapshot.slots()) {
            if before.count() != after.count() || !same_stack_representation(before, after)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Refresh from the exclusive owner's reply, returning item-content change.
    /// Timer-only changes update progress properties without changing state id.
    /// Caller advances the item id only when publishing an item-content resync.
    pub(crate) fn set_snapshot(&mut self, snapshot: FurnaceInventory) -> anyhow::Result<bool> {
        let changed = !self.matches_contents(&snapshot)?;
        self.snapshot = snapshot;
        Ok(changed)
    }

    /// Advance a content revision in Paper's nonnegative fifteen-bit domain.
    pub(crate) fn bump_state_id(&mut self) {
        self.state_id = (self.state_id + 1) & 32767;
    }

    /// Map only valid 39-slot screen indices, excluding outside-drop sentinels.
    pub(crate) fn classify_slot(raw: i16) -> Option<FurnaceWindowSlot> {
        let raw = usize::try_from(raw).ok()?;
        if raw < 3 {
            Some(FurnaceWindowSlot::Furnace(raw))
        } else if raw < FURNACE_WINDOW_SLOTS {
            Some(FurnaceWindowSlot::Player(raw - 3 + MAIN_START))
        } else {
            None
        }
    }

    /// Open the genuine furnace menu with the client's translated title.
    pub(crate) fn open_screen(&self) -> OpenScreen {
        let mut title = NbtCompound::new();
        title.push("translate", NbtTag::String("container.furnace".to_string()));
        OpenScreen::new(self.window_id, FURNACE_TYPE, NbtTag::Compound(title))
    }

    /// Build native content: three furnace slots, player main/hotbar, cursor.
    /// The source inventory remains the sole cursor owner across every screen.
    pub(crate) fn content_payload(
        &self,
        player: &PlayerInventory,
    ) -> Result<Vec<u8>, ItemValidationError> {
        let mut combined = Vec::with_capacity(FURNACE_WINDOW_SLOTS);
        combined.extend_from_slice(self.snapshot.slots());
        combined.extend_from_slice(player.window_slots());
        encode_container_content_payload(&combined, player.carried())
    }

    /// Build the complete typed content packet at the currently published id.
    pub(crate) fn content_packet(
        &self,
        player: &PlayerInventory,
    ) -> Result<SetContainerContent, ItemValidationError> {
        Ok(SetContainerContent::new(
            self.window_id,
            self.state_id(),
            self.content_payload(player)?,
        ))
    }

    /// Build all four native timer/property packets without changing item state.
    pub(crate) fn property_packets(&self) -> [SetContainerProperty; 4] {
        self.properties()
            .map(|(property, value)| SetContainerProperty::new(self.window_id, property, value))
    }

    /// Data access0..3: burn remaining, burn duration, cook progress, cook total.
    /// Native furnace bounds ensure every value fits its signed network short.
    pub(crate) fn properties(&self) -> [(i16, i16); 4] {
        [
            (0, self.snapshot.burn_remaining().cast_signed()),
            (1, self.snapshot.burn_duration().cast_signed()),
            (2, self.snapshot.cook_progress().cast_signed()),
            (3, self.snapshot.cook_total().cast_signed()),
        ]
    }
}

/// Prepare and fully admit the player's proposed furnace-click postimage.
///
/// All ten hidden window-0 slots and nine table inputs are retained exactly;
/// only main/hotbar and cursor use the supplied owned-plan postimages. Exact
/// schema-5 full-record sizing/capacities admit before cloning, then the actual
/// `PlayerData` codec admits position/look, vitals and all lossless component
/// bytes. This is a preview, never a commit or save: its constructor resets the
/// ephemeral inventory state id. Caller retains the live inventory identity,
/// compares original player/furnace/revisions, and publishes both sides once.
/// A failed preview leaves the complete live inventory/cursor untouched.
pub(crate) fn prepare_player_candidate(
    player: &PlayerInventory,
    player_after: &[ItemStack; 36],
    cursor_after: &ItemStack,
    position: Vec3,
    yaw: f32,
    pitch: f32,
    vitals: VitalsSnapshot,
) -> anyhow::Result<PlayerInventory> {
    player.admit_container_candidate(player_after, cursor_after)?;
    let mut bytes = 41usize + 21 + 56 * 4;
    for index in 0..SLOT_COUNT {
        let stack = candidate_slot(player, player_after, index)?;
        admit_candidate_stack(stack, &mut bytes)?;
    }
    for stack in player
        .table_inputs()
        .iter()
        .chain(std::iter::once(cursor_after))
    {
        admit_candidate_stack(stack, &mut bytes)?;
    }
    let slots = std::array::from_fn(|index| {
        if (MAIN_START..PLAYER_WINDOW_END).contains(&index) {
            player_after[index - MAIN_START].clone()
        } else {
            player.slot(index).cloned().unwrap_or_else(ItemStack::empty)
        }
    });
    let mut candidate = PlayerInventory::from_persisted(
        player.game_mode(),
        player.selected(),
        slots,
        cursor_after.clone(),
    );
    candidate.restore_table_inputs(player.table_inputs().clone());
    let mut data = PlayerData::capture(position, yaw, pitch, &candidate);
    data.set_vitals(vitals)?;
    data.to_record(player.game_mode())?;
    Ok(candidate)
}

fn candidate_slot<'a>(
    player: &'a PlayerInventory,
    player_after: &'a [ItemStack; 36],
    index: usize,
) -> anyhow::Result<&'a ItemStack> {
    if (MAIN_START..PLAYER_WINDOW_END).contains(&index) {
        Ok(&player_after[index - MAIN_START])
    } else {
        player
            .slot(index)
            .ok_or_else(|| anyhow::anyhow!("missing player candidate slot"))
    }
}

fn admit_candidate_stack(stack: &ItemStack, bytes: &mut usize) -> anyhow::Result<()> {
    *bytes = bytes
        .checked_add(persisted_stack_size(stack)?)
        .ok_or_else(|| anyhow::anyhow!("player candidate budget overflow"))?;
    anyhow::ensure!(
        *bytes <= MAX_PLAYER_DATA_LEN,
        "player candidate budget exceeded"
    );
    stack_capacity(stack)?;
    anyhow::ensure!(
        !stack.item().is_some_and(ferrumc_items::ItemId::is_air),
        "present air in player candidate"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use ferrumc_core::GameMode;
    use ferrumc_items::{
        encode_persisted_stack, ComponentPatch, ComponentValue, ItemId, OpaqueComponent,
    };
    use ferrumc_sim::PlayerVitals;

    fn stack(name: &str, count: u8) -> ItemStack {
        ItemStack::try_new(
            ItemId::from_name(name).unwrap(),
            count,
            ComponentPatch::empty(),
        )
        .unwrap()
    }

    fn player() -> PlayerInventory {
        PlayerInventory::from_persisted(
            GameMode::Survival,
            4,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        )
    }

    fn snapshot(
        slots: [ItemStack; 3],
        remaining: u16,
        duration: u16,
        progress: u16,
        total: u16,
    ) -> FurnaceInventory {
        FurnaceInventory::try_from_parts(slots, remaining, duration, progress, total).unwrap()
    }

    fn window() -> FurnaceWindow {
        FurnaceWindow::new(7, BlockPos::new(-8, 64, 9), FurnaceInventory::new()).unwrap()
    }

    fn hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    // Independently serialized and completely decoded using installed pinned
    // minecraft-protocol1.66.2/minecraft-data3.111.0, protocol772, 1.21.8.
    #[test]
    fn native_open_screen_matches_prismarine_furnace_fourteen_and_translated_title() {
        let window = window();
        let mut bytes = BytesMut::new();
        window.open_screen().encode(&mut bytes).unwrap();
        assert_eq!(
            &bytes[..],
            hex("34070e0a0800097472616e736c6174650011636f6e7461696e65722e6675726e61636500")
        );
        assert_eq!(window.open_screen().window_type(), 14);
        assert_eq!(window.window_id(), 7);
        assert_eq!(window.position(), BlockPos::new(-8, 64, 9));
    }

    #[test]
    fn empty_native_content_matches_prismarine_thirty_nine_slots_and_one_cursor() {
        let window = window();
        let mut bytes = BytesMut::new();
        window
            .content_packet(&player())
            .unwrap()
            .encode(&mut bytes)
            .unwrap();
        assert_eq!(&bytes[..],hex("1207012700000000000000000000000000000000000000000000000000000000000000000000000000000000"));
        assert_eq!(window.content_payload(&player()).unwrap().len(), 41);
    }

    #[test]
    fn native_content_preserves_furnace_main_hotbar_order_and_inventory_owned_cursor() {
        let slots = [
            stack("iron_ore", 3),
            stack("coal", 2),
            stack("iron_ingot", 4),
        ];
        let window = FurnaceWindow::new(
            7,
            BlockPos::new(0, 64, 0),
            snapshot(slots, 1599, 1600, 37, 200),
        )
        .unwrap();
        let mut player = player();
        *player.slot_mut(MAIN_START).unwrap() = stack("diamond", 5);
        *player.slot_mut(PLAYER_WINDOW_END - 1).unwrap() = stack("stone", 6);
        player.set_carried(stack("stone", 7));
        let mut bytes = BytesMut::new();
        window
            .content_packet(&player)
            .unwrap()
            .encode(&mut bytes)
            .unwrap();
        assert_eq!(&bytes[..],hex("120701270342000002dc06000004e406000005de060000000000000000000000000000000000000000000000000000000000000000000000000601000007010000"));
        let before = encode_persisted_stack(player.carried()).unwrap();
        window.content_payload(&player).unwrap();
        assert_eq!(encode_persisted_stack(player.carried()).unwrap(), before);
    }

    #[test]
    fn native_timer_packets_match_prismarine_exact_short_fields_and_maximum() {
        let window = FurnaceWindow::new(
            7,
            BlockPos::new(0, 64, 0),
            snapshot(
                std::array::from_fn(|_| ItemStack::empty()),
                1599,
                1600,
                37,
                200,
            ),
        )
        .unwrap();
        assert_eq!(
            window.properties(),
            [(0, 1599), (1, 1600), (2, 37), (3, 200)]
        );
        for (packet, golden) in window.property_packets().into_iter().zip([
            "13070000063f",
            "130700010640",
            "130700020025",
            "1307000300c8",
        ]) {
            let mut bytes = BytesMut::new();
            packet.encode(&mut bytes).unwrap();
            assert_eq!(&bytes[..], hex(golden));
        }
        let max = FurnaceWindow::new(
            99,
            BlockPos::new(0, 64, 0),
            snapshot(
                std::array::from_fn(|_| ItemStack::empty()),
                32767,
                32767,
                32766,
                32767,
            ),
        )
        .unwrap();
        let mut bytes = BytesMut::new();
        max.property_packets()[3].encode(&mut bytes).unwrap();
        assert_eq!(&bytes[..], hex("136300037fff"));
    }

    #[test]
    fn ids_are_bounded_indices_are_exact_and_content_revision_wraps_fifteen_bits() {
        for id in [-1, 0, 100, i32::MAX] {
            assert!(
                FurnaceWindow::new(id, BlockPos::new(0, 0, 0), FurnaceInventory::new()).is_err()
            );
        }
        for id in [1, 99] {
            assert_eq!(
                FurnaceWindow::new(id, BlockPos::new(0, 0, 0), FurnaceInventory::new())
                    .unwrap()
                    .window_id(),
                id
            );
        }
        for (raw, expected) in [
            (0, Some(FurnaceWindowSlot::Furnace(0))),
            (2, Some(FurnaceWindowSlot::Furnace(2))),
            (3, Some(FurnaceWindowSlot::Player(9))),
            (29, Some(FurnaceWindowSlot::Player(35))),
            (30, Some(FurnaceWindowSlot::Player(36))),
            (38, Some(FurnaceWindowSlot::Player(44))),
            (39, None),
            (-1, None),
            (-999, None),
        ] {
            assert_eq!(FurnaceWindow::classify_slot(raw), expected);
        }
        let mut window = window();
        for _ in 1..32767 {
            window.bump_state_id();
        }
        assert_eq!(window.state_id(), 32767);
        window.bump_state_id();
        assert_eq!(window.state_id(), 0);
        window.bump_state_id();
        assert_eq!(window.state_id(), 1);
    }

    fn float_stack(bits: u32, count: u8) -> ItemStack {
        let mut data = NbtCompound::new();
        data.push("float", NbtTag::Float(f32::from_bits(bits)));
        ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            count,
            ComponentPatch::new(
                vec![ComponentValue::CustomData(NbtTag::Compound(data))],
                vec![],
            ),
        )
        .unwrap()
    }

    #[test]
    fn timer_refresh_is_not_stale_but_quantities_and_exact_nan_bits_are_content_changes() {
        let slots = [
            float_stack(0x7fc0_0011, 3),
            ItemStack::empty(),
            ItemStack::empty(),
        ];
        let mut window = FurnaceWindow::new(
            7,
            BlockPos::new(0, 64, 0),
            snapshot(slots.clone(), 100, 1600, 7, 200),
        )
        .unwrap();
        let state = window.state_id();
        assert!(!window
            .set_snapshot(snapshot(slots.clone(), 99, 1600, 8, 200))
            .unwrap());
        assert_eq!(window.state_id(), state);
        assert_eq!(window.snapshot().cook_progress(), 8);
        let mut changed = slots.clone();
        changed[0] = float_stack(0x7fc0_0012, 3);
        assert!(window
            .set_snapshot(snapshot(changed, 98, 1600, 9, 200))
            .unwrap());
        window.bump_state_id();
        assert_eq!(window.state_id(), state + 1);
        let mut changed = slots;
        changed[0] = float_stack(0x7fc0_0012, 4);
        assert!(!window
            .matches_contents(&snapshot(changed, 97, 1600, 10, 200))
            .unwrap());
    }

    #[test]
    fn full_player_candidate_retains_all_hidden_and_table_representations_without_mutation() {
        let mut player = player();
        for index in (0..9).chain(std::iter::once(45)) {
            *player.slot_mut(index).unwrap() = float_stack(0x7fc0_0011, index as u8 + 1);
        }
        player.restore_table_inputs(std::array::from_fn(|index| {
            float_stack(0x7fc0_0012, index as u8 + 1)
        }));
        player.set_carried(stack("diamond", 7));
        let source_state = player.state_id();
        let after = std::array::from_fn(|index| stack("stone", index as u8 + 1));
        let cursor = stack("coal", 6);
        let vitals = PlayerVitals::new().snapshot();
        let candidate =
            prepare_player_candidate(&player, &after, &cursor, Vec3::ZERO, 3.0, 4.0, vitals)
                .unwrap();
        for index in (0..9).chain(std::iter::once(45)) {
            assert_eq!(
                encode_persisted_stack(candidate.slot(index).unwrap()).unwrap(),
                encode_persisted_stack(player.slot(index).unwrap()).unwrap()
            );
        }
        for (a, b) in candidate.table_inputs().iter().zip(player.table_inputs()) {
            assert_eq!(
                encode_persisted_stack(a).unwrap(),
                encode_persisted_stack(b).unwrap()
            );
        }
        assert_eq!(candidate.window_slots(), after);
        assert_eq!(candidate.carried(), &cursor);
        assert_eq!(candidate.selected(), 4);
        assert_eq!(candidate.game_mode(), GameMode::Survival);
        assert_eq!(player.state_id(), source_state);
        assert_eq!(*player.carried(), stack("diamond", 7));
        assert!(player
            .window_slots()
            .iter()
            .all(|stack| stack.item().is_none()));
        let mut data = PlayerData::capture(Vec3::ZERO, 3.0, 4.0, &candidate);
        data.set_vitals(vitals).unwrap();
        assert!(data.to_record(GameMode::Survival).is_ok());
    }

    fn large_stack(last: usize) -> ItemStack {
        let components = (6..38)
            .map(|id| {
                ComponentValue::Opaque(
                    OpaqueComponent::new(id, vec![7; if id == 37 { last } else { 8192 }]).unwrap(),
                )
            })
            .collect();
        ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            1,
            ComponentPatch::new(components, vec![]),
        )
        .unwrap()
    }

    #[test]
    fn complete_player_candidate_counts_hidden_table_bytes_at_exact_limit_and_next_byte() {
        let mut player = player();
        let mut table = std::array::from_fn(|_| ItemStack::empty());
        for stack in &mut table[..4] {
            *stack = large_stack(8192);
        }
        let total = 41
            + 21
            + 56 * 4
            + 46 * 7
            + 7
            + table
                .iter()
                .map(|stack| persisted_stack_size(stack).unwrap())
                .sum::<usize>();
        let excess = total - MAX_PLAYER_DATA_LEN;
        assert!(excess < 8192);
        table[3] = large_stack(8192 - excess);
        player.restore_table_inputs(table.clone());
        let after = std::array::from_fn(|_| ItemStack::empty());
        let vitals = PlayerVitals::new().snapshot();
        let candidate = prepare_player_candidate(
            &player,
            &after,
            &ItemStack::empty(),
            Vec3::ZERO,
            0.0,
            0.0,
            vitals,
        )
        .unwrap();
        assert_eq!(
            PlayerData::capture(Vec3::ZERO, 0.0, 0.0, &candidate)
                .to_record(GameMode::Survival)
                .unwrap()
                .data()
                .len(),
            MAX_PLAYER_DATA_LEN
        );
        // The pure 36+cursor planner cannot see these nineteen hidden slots.
        let mut changed = after;
        changed[0] = stack("stone", 1);
        assert!(prepare_player_candidate(
            &player,
            &changed,
            &ItemStack::empty(),
            Vec3::ZERO,
            0.0,
            0.0,
            vitals
        )
        .is_err());
        assert!(player.window_slots()[0].item().is_none());
        table[3] = large_stack(8192 - excess + 1);
        player.restore_table_inputs(table);
        assert!(prepare_player_candidate(
            &player,
            &std::array::from_fn(|_| ItemStack::empty()),
            &ItemStack::empty(),
            Vec3::ZERO,
            0.0,
            0.0,
            vitals
        )
        .is_err());
    }

    #[test]
    fn full_candidate_codec_rejects_unsafe_position_look_and_invalid_hidden_capacity() {
        let mut player = player();
        let after = std::array::from_fn(|_| ItemStack::empty());
        let vitals = PlayerVitals::new().snapshot();
        for position in [
            Vec3::new(f64::NAN, 0.0, 0.0),
            Vec3::new(30_000_001.0, 64.0, 0.0),
        ] {
            assert!(prepare_player_candidate(
                &player,
                &after,
                &ItemStack::empty(),
                position,
                0.0,
                0.0,
                vitals
            )
            .is_err());
        }
        assert!(prepare_player_candidate(
            &player,
            &after,
            &ItemStack::empty(),
            Vec3::ZERO,
            f32::NAN,
            0.0,
            vitals
        )
        .is_err());
        *player.slot_mut(45).unwrap() = ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            2,
            ComponentPatch::new(vec![ComponentValue::MaxStackSize(1)], vec![]),
        )
        .unwrap();
        assert!(prepare_player_candidate(
            &player,
            &after,
            &ItemStack::empty(),
            Vec3::ZERO,
            0.0,
            0.0,
            vitals
        )
        .is_err());
        assert_eq!(player.slot(45).unwrap().count(), 2);
    }
}
