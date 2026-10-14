use super::*;
use ferrumc_items::{ComponentPatch, ComponentTypeId, ComponentValue, ItemId};
use ferrumc_nbt::{NbtCompound, NbtTag};

fn stack(count: u8, bits: u32) -> ItemStack {
    let mut data = NbtCompound::new();
    data.push("ordered_first", NbtTag::Float(f32::from_bits(bits)));
    data.push("ordered_second", NbtTag::Int(7));
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::new(
            vec![ComponentValue::CustomData(NbtTag::Compound(data))],
            vec![ComponentTypeId::new(5)],
        ),
    )
    .unwrap()
}

fn inventory(changed: Option<usize>) -> PlayerInventory {
    let mut slots = std::array::from_fn(|_| ItemStack::empty());
    slots[HOTBAR_START + 2] = stack(7, 0x7fc0_0011);
    let mut cursor = ItemStack::empty();
    let mut table = std::array::from_fn(|_| ItemStack::empty());
    if let Some(index) = changed {
        match index.cmp(&SLOT_COUNT) {
            std::cmp::Ordering::Less => slots[index] = stack(2, 0x7fc0_0012),
            std::cmp::Ordering::Equal => cursor = stack(2, 0x7fc0_0012),
            std::cmp::Ordering::Greater => table[index - SLOT_COUNT - 1] = stack(2, 0x7fc0_0012),
        }
    }
    let mut inventory = PlayerInventory::from_persisted(GameMode::Survival, 2, slots, cursor);
    inventory.restore_table_inputs(table);
    inventory
}

fn frames(inventory: &PlayerInventory) -> Vec<Vec<u8>> {
    stacks(inventory)
        .unwrap()
        .into_iter()
        .map(|stack| encode_persisted_stack(stack).unwrap())
        .collect()
}

#[test]
fn held_drop_one_and_whole_preserve_complete_components_and_only_selected_slot() {
    for (status, count, remaining) in [(4, 1, 6), (3, 7, 0)] {
        let mut inventory = inventory(None);
        let before = frames(&inventory);
        let state = inventory.state_id();
        let plan = HeldDropPlan::prepare(&inventory, status).unwrap().unwrap();
        let (dropped, commit) = plan.into_parts();
        assert_eq!(frames(&inventory), before);
        assert_eq!(
            encode_persisted_stack(&dropped).unwrap(),
            encode_persisted_stack(&stack(count, 0x7fc0_0011)).unwrap()
        );
        assert!(commit.matches(&inventory).unwrap());
        commit.commit(&mut inventory).unwrap();
        assert_eq!(inventory.slot(HOTBAR_START + 2).unwrap().count(), remaining);
        assert_ne!(inventory.state_id(), state);
        let after = frames(&inventory);
        for index in 0..56 {
            if index != HOTBAR_START + 2 {
                assert_eq!(before[index], after[index]);
            }
        }
    }
}

#[test]
fn held_drop_beforeimages_cover_every_slot_cursor_table_and_mode_selection() {
    let original = inventory(None);
    let (_, commit) = HeldDropPlan::prepare(&original, 4)
        .unwrap()
        .unwrap()
        .into_parts();
    for index in 0..56 {
        assert!(
            !commit.matches(&inventory(Some(index))).unwrap(),
            "index {index}"
        );
    }
    let mut selected = inventory(None);
    selected.set_selected(1);
    assert!(!commit.matches(&selected).unwrap());
    let mut mode = inventory(None);
    mode.set_game_mode(GameMode::Creative);
    assert!(!commit.matches(&mode).unwrap());
}

#[test]
fn held_drop_inactive_or_invalid_actions_do_not_prepare() {
    for status in [-999, 0, 1, 2, 5, 6] {
        assert!(HeldDropPlan::prepare(&inventory(None), status)
            .unwrap()
            .is_none());
    }
    let mut spectator = inventory(None);
    spectator.set_game_mode(GameMode::Spectator);
    assert!(HeldDropPlan::prepare(&spectator, 4).unwrap().is_none());
    spectator.set_game_mode(GameMode::Creative);
    assert!(HeldDropPlan::prepare(&spectator, 3).unwrap().is_some());
    spectator.set_selected(0);
    assert!(HeldDropPlan::prepare(&spectator, 3).unwrap().is_none());
}

#[test]
fn held_drop_complete_postimage_budget_refuses_single_but_whole_can_repair_restored_image() {
    let fixture = crate::inventory::budget_boundary_fixture(0, false);
    let mut slots = std::array::from_fn(|index| fixture.slot(index).unwrap().clone());
    let mut table = fixture.table_inputs().clone();
    slots[HOTBAR_START] = std::mem::replace(&mut table[0], ItemStack::empty());
    let mut inventory =
        PlayerInventory::from_persisted(GameMode::Survival, 0, slots, ItemStack::empty());
    inventory.restore_table_inputs(table);
    assert!(inventory.admit_budget().is_err());
    let before = frames(&inventory);
    assert!(HeldDropPlan::prepare(&inventory, 4).is_err());
    assert_eq!(frames(&inventory), before);
    let (_, commit) = HeldDropPlan::prepare(&inventory, 3)
        .unwrap()
        .unwrap()
        .into_parts();
    commit.commit(&mut inventory).unwrap();
    inventory.admit_budget().unwrap();
}
