use super::*;
use crate::{
    encode_persisted_stack, same_stack_representation, ComponentPatch, ComponentValue, ItemId,
    OpaqueComponent,
};
use std::collections::BTreeMap;

fn stack(name: &str, count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}

fn empty_furnace() -> [ItemStack; 3] {
    std::array::from_fn(|_| ItemStack::empty())
}

fn empty_player() -> [ItemStack; 36] {
    std::array::from_fn(|_| ItemStack::empty())
}

fn quantities<'a>(stacks: impl Iterator<Item = &'a ItemStack>) -> BTreeMap<Vec<u8>, u32> {
    let mut quantities = BTreeMap::new();
    for stack in stacks {
        if let Some(item) = stack.item() {
            let one = ItemStack::try_new(item, 1, stack.components().clone()).unwrap();
            *quantities
                .entry(encode_persisted_stack(&one).unwrap())
                .or_default() += u32::from(stack.count());
        }
    }
    quantities
}

fn checked(
    furnace: &[ItemStack; 3],
    player: &[ItemStack; 36],
    cursor: &ItemStack,
    slot: usize,
    button: i8,
    mode: i32,
) -> FurnaceClickPlan {
    let before: Vec<_> = furnace
        .iter()
        .chain(player)
        .chain(std::iter::once(cursor))
        .map(|stack| encode_persisted_stack(stack).unwrap())
        .collect();
    let plan = plan_furnace_click(furnace, player, cursor, slot, button, mode).unwrap();
    assert_eq!(
        quantities(furnace.iter().chain(player).chain(std::iter::once(cursor))),
        quantities(
            plan.slots()
                .iter()
                .chain(plan.player())
                .chain(std::iter::once(plan.cursor()))
        )
    );
    let after: Vec<_> = furnace
        .iter()
        .chain(player)
        .chain(std::iter::once(cursor))
        .map(|stack| encode_persisted_stack(stack).unwrap())
        .collect();
    assert_eq!(before, after, "borrowed source is unchanged");
    plan
}

#[test]
fn left_and_right_pickup_preserve_every_quantity_and_exact_components() {
    for count in 1..=64 {
        let mut furnace = empty_furnace();
        furnace[0] = ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            count,
            ComponentPatch::new(
                vec![ComponentValue::Opaque(
                    OpaqueComponent::new(20, vec![1, 2, 3]).unwrap(),
                )],
                vec![],
            ),
        )
        .unwrap();
        for button in [0, 1] {
            let plan = checked(&furnace, &empty_player(), &ItemStack::empty(), 0, button, 0);
            let taken = if button == 0 {
                count
            } else {
                count.div_ceil(2)
            };
            assert_eq!(plan.cursor().count(), taken);
            assert_eq!(plan.slots()[0].count(), count - taken);
            assert!(plan.changed());
        }
    }
}

#[test]
fn manual_input_accepts_nonrecipe_items_and_preserves_incompatible_swap() {
    let furnace = empty_furnace();
    let player = empty_player();
    let plan = checked(&furnace, &player, &stack("diamond", 7), 0, 0, 0);
    assert_eq!(plan.slots()[0], stack("diamond", 7));
    let plan = checked(plan.slots(), &player, &stack("coal", 5), 0, 1, 0);
    assert_eq!(plan.slots()[0], stack("coal", 5));
    assert_eq!(*plan.cursor(), stack("diamond", 7));
}

#[test]
fn fuel_accepts_catalog_fuel_or_exactly_one_bucket_and_refuses_unrelated_items() {
    let furnace = empty_furnace();
    let player = empty_player();
    for button in [0, 1] {
        let plan = checked(&furnace, &player, &stack("bucket", 16), 1, button, 0);
        assert_eq!(plan.slots()[1], stack("bucket", 1));
        assert_eq!(*plan.cursor(), stack("bucket", 15));
    }
    let fuel = checked(&furnace, &player, &stack("lava_bucket", 1), 1, 0, 0);
    assert_eq!(fuel.slots()[1], stack("lava_bucket", 1));
    let rejected = checked(&furnace, &player, &stack("diamond", 7), 1, 0, 0);
    assert!(!rejected.changed());
    assert_eq!(*rejected.cursor(), stack("diamond", 7));
}

#[test]
fn output_never_accepts_insert_or_swap_and_matching_cursor_requires_whole_result_room() {
    let mut furnace = empty_furnace();
    let player = empty_player();
    furnace[2] = stack("iron_ingot", 9);
    for button in [0, 1] {
        let incompatible = checked(&furnace, &player, &stack("coal", 3), 2, button, 0);
        assert!(!incompatible.changed());
        let full = checked(&furnace, &player, &stack("iron_ingot", 56), 2, button, 0);
        assert!(
            !full.changed(),
            "Slot.tryRemove forbids partial output into an existing cursor"
        );
        let fitting = checked(&furnace, &player, &stack("iron_ingot", 55), 2, button, 0);
        assert_eq!(fitting.cursor().count(), 64);
        assert_eq!(fitting.slots()[2], ItemStack::empty());
    }
    let right = checked(&furnace, &player, &ItemStack::empty(), 2, 1, 0);
    assert_eq!(right.cursor().count(), 5);
    assert_eq!(right.slots()[2].count(), 4);
    let empty = checked(&empty_furnace(), &player, &stack("iron_ingot", 1), 2, 0, 0);
    assert!(!empty.changed());
}

#[test]
fn shift_result_reverse_but_input_and_fuel_forward_with_full_destination_conservation() {
    for slot in 0..3 {
        let mut furnace = empty_furnace();
        furnace[slot] = stack("coal", 7);
        let mut player = empty_player();
        let expected = if slot == 2 { 35 } else { 0 };
        player[expected] = stack("coal", 60);
        let plan = checked(&furnace, &player, &stack("diamond", 3), slot, 0, 1);
        assert_eq!(plan.player()[expected].count(), 64);
        assert_eq!(plan.player()[if slot == 2 { 34 } else { 1 }].count(), 3);
        assert!(plan.slots()[slot].item().is_none());
        let full = std::array::from_fn(|_| stack("diamond", 64));
        let no_op = checked(&furnace, &full, &ItemStack::empty(), slot, 1, 1);
        assert!(!no_op.changed());
    }
}

#[test]
fn player_shift_prioritizes_smelting_over_fuel_and_never_falls_back_when_blocked() {
    let mut player = empty_player();
    player[0] = stack("oak_log", 12);
    let furnace = empty_furnace();
    let plan = checked(&furnace, &player, &ItemStack::empty(), 3, 0, 1);
    assert_eq!(plan.slots()[0], stack("oak_log", 12));
    assert!(plan.slots()[1].item().is_none());
    let mut blocked = furnace;
    blocked[0] = stack("oak_log", 64);
    let plan = checked(&blocked, &player, &ItemStack::empty(), 3, 1, 1);
    assert!(!plan.changed());
    assert!(plan.slots()[1].item().is_none());
    player[0] = stack("coal", 8);
    let plan = checked(&empty_furnace(), &player, &ItemStack::empty(), 3, 0, 1);
    assert_eq!(plan.slots()[1], stack("coal", 8));
}

#[test]
fn bucket_shift_is_player_relocation_and_main_hotbar_both_directions_are_forward() {
    let mut player = empty_player();
    player[0] = stack("bucket", 7);
    let plan = checked(&empty_furnace(), &player, &ItemStack::empty(), 3, 0, 1);
    assert_eq!(plan.player()[27], stack("bucket", 7));
    assert!(plan.slots()[1].item().is_none());
    let plan = checked(plan.slots(), plan.player(), plan.cursor(), 30, 1, 1);
    assert_eq!(plan.player()[0], stack("bucket", 7));
    assert!(plan.player()[27].item().is_none());
}

#[test]
fn all_hotbar_buttons_swap_and_output_extracts_only_to_an_empty_hotbar() {
    for button in 0..9 {
        let mut furnace = empty_furnace();
        furnace[0] = stack("iron_ore", 11);
        let mut player = empty_player();
        player[27 + button] = stack("diamond", 4);
        let plan = checked(&furnace, &player, &stack("coal", 3), 0, button as i8, 2);
        assert_eq!(plan.slots()[0], stack("diamond", 4));
        assert_eq!(plan.player()[27 + button], stack("iron_ore", 11));
        furnace[2] = stack("iron_ingot", 8);
        let refused = checked(&furnace, &player, &ItemStack::empty(), 2, button as i8, 2);
        assert!(!refused.changed());
        player[27 + button] = ItemStack::empty();
        let taken = checked(&furnace, &player, &ItemStack::empty(), 2, button as i8, 2);
        assert_eq!(taken.player()[27 + button], stack("iron_ingot", 8));
        assert!(taken.slots()[2].item().is_none());
    }
}

#[test]
fn hotbar_bucket_splits_into_empty_fuel_and_refuses_unrepresented_overflow_branch() {
    let mut player = empty_player();
    player[27] = stack("bucket", 7);
    let furnace = empty_furnace();
    let plan = checked(&furnace, &player, &ItemStack::empty(), 1, 0, 2);
    assert_eq!(plan.slots()[1], stack("bucket", 1));
    assert_eq!(plan.player()[27], stack("bucket", 6));
    let mut occupied = furnace;
    occupied[1] = stack("coal", 1);
    assert!(matches!(
        plan_furnace_click(&occupied, &player, &ItemStack::empty(), 1, 0, 2),
        Err(FurnaceClickError::UnsupportedSwap)
    ));
    assert_eq!(occupied[1], stack("coal", 1));
    assert_eq!(player[27], stack("bucket", 7));
}

#[test]
fn same_hotbar_target_and_identical_representations_do_not_change() {
    let mut player = empty_player();
    player[27] = stack("stone", 1);
    player[0] = stack("stone", 1);
    for slot in [3, 30] {
        assert!(!checked(&empty_furnace(), &player, &ItemStack::empty(), slot, 0, 2).changed());
    }
}

#[test]
fn malformed_actions_stacks_capacities_and_present_air_refuse_even_noops() {
    for (slot, button, mode) in [
        (39, 0, 0),
        (usize::MAX, 0, 0),
        (0, -1, 0),
        (0, 2, 0),
        (0, -1, 1),
        (0, 2, 1),
        (0, -1, 2),
        (0, 9, 2),
        (0, 40, 2),
        (0, 0, -1),
        (0, 0, 3),
    ] {
        assert!(matches!(
            plan_furnace_click(
                &empty_furnace(),
                &empty_player(),
                &ItemStack::empty(),
                slot,
                button,
                mode
            ),
            Err(FurnaceClickError::InvalidAction)
        ));
    }
    let mut player = empty_player();
    player[35] = ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        2,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(1)], vec![]),
    )
    .unwrap();
    assert!(matches!(
        plan_furnace_click(&empty_furnace(), &player, &ItemStack::empty(), 0, 0, 0),
        Err(FurnaceClickError::Capacity(_))
    ));
    player[35] = stack("air", 1);
    assert!(matches!(
        plan_furnace_click(&empty_furnace(), &player, &ItemStack::empty(), 0, 0, 0),
        Err(FurnaceClickError::PresentAir)
    ));
    player[35] = ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        1,
        ComponentPatch::new(
            vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
            vec![],
        ),
    )
    .unwrap();
    assert!(matches!(
        plan_furnace_click(&empty_furnace(), &player, &ItemStack::empty(), 0, 0, 0),
        Err(FurnaceClickError::Stack(_))
    ));
}

fn large_stack(count: u8, last: usize) -> ItemStack {
    let components = (6..38)
        .map(|id| {
            ComponentValue::Opaque(
                OpaqueComponent::new(id, vec![7; if id == 37 { last } else { 8192 }]).unwrap(),
            )
        })
        .collect();
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        count,
        ComponentPatch::new(components, vec![]),
    )
    .unwrap()
}

#[test]
fn exact_player_budget_passes_but_split_postimage_duplicate_payload_is_refused() {
    let mut player = empty_player();
    for stack in &mut player[..4] {
        *stack = large_stack(1, 8192);
    }
    player[0] = large_stack(2, 8192);
    let bytes = PLAYER_FRAMING_MIN
        + player
            .iter()
            .map(|stack| persisted_stack_size(stack).unwrap())
            .sum::<usize>()
        + 7;
    let excess = bytes - PLAYER_BYTES;
    assert!(excess < 8192);
    player[3] = large_stack(1, 8192 - excess);
    let plan = checked(&empty_furnace(), &player, &ItemStack::empty(), 3, 0, 0);
    assert_eq!(plan.cursor().count(), 2);
    assert!(matches!(
        plan_furnace_click(&empty_furnace(), &player, &ItemStack::empty(), 3, 1, 0),
        Err(FurnaceClickError::PlayerBudgetExceeded)
    ));
    assert_eq!(player[0].count(), 2);
    player[3] = large_stack(1, 8192 - excess + 1);
    assert!(matches!(
        plan_furnace_click(&empty_furnace(), &player, &ItemStack::empty(), 0, 0, 0),
        Err(FurnaceClickError::PlayerBudgetExceeded)
    ));
}

#[test]
fn furnace_candidate_budget_is_checked_before_returning_any_player_postimage() {
    let mut player = empty_player();
    player[27] = large_stack(1, 8192);
    let furnace = empty_furnace();
    assert!(matches!(
        plan_furnace_click(&furnace, &player, &ItemStack::empty(), 0, 0, 2),
        Err(FurnaceClickError::Furnace(SmeltingError::BudgetExceeded))
    ));
    assert!(furnace[0].item().is_none());
    assert_eq!(player[27].count(), 1);
}

#[test]
fn stale_output_plan_has_no_snapshot_exchange_authority() {
    let mut original = empty_furnace();
    original[2] = stack("iron_ingot", 6);
    let plan = checked(&original, &empty_player(), &ItemStack::empty(), 2, 0, 0);
    let mut current = original.clone();
    current[2] = stack("iron_ingot", 7);
    // The real owner must compare original counts AND exact representation;
    // this planner never authorizes overwriting a concurrent smelt or extraction.
    let snapshot_matches = original
        .iter()
        .zip(&current)
        .all(|(a, b)| a.count() == b.count() && same_stack_representation(a, b).unwrap());
    assert!(!snapshot_matches);
    assert_eq!(current[2].count(), 7);
    assert_eq!(plan.cursor().count(), 6);
}

fn furnace_bound_stack(last: usize) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name("stone").unwrap(),
        1,
        ComponentPatch::new(
            vec![
                ComponentValue::Opaque(OpaqueComponent::new(20, vec![1; 8192]).unwrap()),
                ComponentValue::Opaque(OpaqueComponent::new(21, vec![2; last]).unwrap()),
            ],
            vec![],
        ),
    )
    .unwrap()
}

#[test]
fn complete_furnace_exact_sixteen_kib_passes_and_next_byte_refuses_on_both_sides() {
    let base_size = persisted_stack_size(&furnace_bound_stack(1)).unwrap();
    let last = crate::MAX_FURNACE_BYTES - crate::FURNACE_FRAMING_BYTES - 14 - (base_size - 1);
    assert!(last < 8192);
    let mut player = empty_player();
    player[27] = furnace_bound_stack(last);
    let plan = checked(&empty_furnace(), &player, &ItemStack::empty(), 0, 0, 2);
    assert_eq!(
        crate::FURNACE_FRAMING_BYTES
            + plan
                .slots()
                .iter()
                .map(|stack| persisted_stack_size(stack).unwrap())
                .sum::<usize>(),
        crate::MAX_FURNACE_BYTES
    );
    let unchanged = checked(plan.slots(), plan.player(), plan.cursor(), 2, 0, 0);
    assert!(!unchanged.changed());
    player[27] = furnace_bound_stack(last + 1);
    assert!(matches!(
        plan_furnace_click(&empty_furnace(), &player, &ItemStack::empty(), 0, 0, 2),
        Err(FurnaceClickError::Furnace(SmeltingError::BudgetExceeded))
    ));
    let mut too_large = empty_furnace();
    too_large[0] = player[27].clone();
    assert!(matches!(
        plan_furnace_click(&too_large, &empty_player(), &ItemStack::empty(), 2, 0, 0),
        Err(FurnaceClickError::Furnace(SmeltingError::BudgetExceeded))
    ));
}
