use super::*;
use ferrumc_core::GameMode;
use ferrumc_items::{ComponentPatch, ItemId};

fn stack(name: &str, count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}

#[test]
fn unrelated_and_refused_pickups_preserve_active_mining_and_its_exact_witness() {
    let mut slots = std::array::from_fn(|_| ItemStack::empty());
    slots[36] = stack("iron_pickaxe", 1);
    let mut inventory =
        PlayerInventory::from_persisted(GameMode::Survival, 0, slots, ItemStack::empty());
    let mut windows = WindowState::new();
    let token = windows.mining.start(&inventory).unwrap();
    let tool_before = ferrumc_items::encode_persisted_stack(inventory.held()).unwrap();
    let (remainder, changed) =
        deposit_ground_pickup(&mut inventory, &mut windows, stack("cobblestone", 1)).unwrap();
    assert!(remainder.item().is_none());
    assert!(!changed);
    assert_eq!(windows.mining.active(), token);
    assert_eq!(inventory.slot(37), Some(&stack("cobblestone", 1)));
    assert_eq!(
        ferrumc_items::encode_persisted_stack(inventory.held()).unwrap(),
        tool_before
    );
    for slot in 37..45 {
        inventory.set_creative_slot(slot, stack("cobblestone", 64));
    }
    for slot in 9..36 {
        inventory.set_creative_slot(slot, stack("cobblestone", 64));
    }
    let incoming = stack("dirt", 1);
    let (remainder, changed) =
        deposit_ground_pickup(&mut inventory, &mut windows, incoming.clone()).unwrap();
    assert_eq!(remainder, incoming);
    assert!(!changed);
    assert_eq!(windows.mining.active(), token);
    assert_eq!(windows.mining.release(token).unwrap().bytes, tool_before);
}

#[test]
fn genuine_held_pickup_change_cancels_work_but_retains_terminal_witness() {
    let mut inventory = PlayerInventory::from_persisted(
        GameMode::Survival,
        0,
        std::array::from_fn(|_| ItemStack::empty()),
        ItemStack::empty(),
    );
    let mut windows = WindowState::new();
    let token = windows.mining.start(&inventory).unwrap();
    let (remainder, changed) =
        deposit_ground_pickup(&mut inventory, &mut windows, stack("dirt", 1)).unwrap();
    assert!(remainder.item().is_none());
    assert!(changed);
    assert_eq!(windows.mining.active(), 0);
    assert!(windows.mining.has_unsettled());
    assert!(windows.mining.release(token).is_some());
}
