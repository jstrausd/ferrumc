//! Immutable location and durability planning for one delayed mining witness.
//! The caller owns accepted receipts, their lineage, exclusive inventory
//! commits, state IDs and publication; a plan grants none of that authority.

use ferrumc_core::GameMode;
use ferrumc_items::{
    decode_persisted_stack, encode_persisted_stack, plan_durability_damage, ItemStack,
};

use crate::inventory::{PlayerInventory, SLOT_COUNT};
use crate::mining_witness::ToolWitness;

/// A player-owned stack location; indices retain the existing inventory layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WearLocation {
    /// One of the 46 window-0 slots, including offhand and crafting slots.
    Slot(usize),
    /// The player's sole carried stack, shared by every open screen.
    Cursor,
    /// One of the nine persisted crafting-table inputs.
    TableInput(usize),
}

/// One checked private postimage awaiting its inventory owner's decision.
#[derive(Debug)]
pub(crate) struct MiningWearPlan {
    location: WearLocation,
    stack_after: ItemStack,
    broken: bool,
}

impl MiningWearPlan {
    /// Where the exact beforeimage was found when this immutable plan was built.
    pub(crate) fn location(&self) -> WearLocation {
        self.location
    }

    /// The checked full postimage, preserving unrelated component representation.
    pub(crate) fn stack_after(&self) -> &ItemStack {
        &self.stack_after
    }

    /// Whether this operation consumed exactly one broken item.
    pub(crate) fn broken(&self) -> bool {
        self.broken
    }

    /// Moves the checked stack to the exclusive owner without making a commit.
    pub(crate) fn into_stack(self) -> ItemStack {
        self.stack_after
    }
}

/// Find an exact FCIS beforeimage and plan its deterministic durability change.
///
/// The original slot wins only when it still contains the exact witness bytes.
/// Otherwise all 46 player slots, the cursor and nine table inputs must contain
/// exactly one matching present stack. Missing or ambiguous matches return
/// `None`; count, patch order and typed/opaque representations are never ignored.
///
/// The bounded shared FCIS decoder validates the witness. Every current stack
/// passes the shared encoder before selection; only one encoded stack buffer is
/// retained at a time, and at most 56 locations are inspected. The shared Items
/// planner validates durability and handles Creative, unbreakable, zero-cost,
/// absent damageability, unsupported enchantment overrides and break-one rules.
/// Nothing mutates even when validation fails. A repeated receipt whose earlier
/// commit changed its beforeimage needs caller-owned lineage handling.
pub(crate) fn plan_wear(
    inventory: &PlayerInventory,
    witness: &ToolWitness,
    amount: u32,
) -> anyhow::Result<Option<MiningWearPlan>> {
    anyhow::ensure!(
        witness.slot < SLOT_COUNT,
        "mining witness slot out of range"
    );
    // Empty witnesses cannot identify a tool among interchangeable empty cells.
    if decode_persisted_stack(&witness.bytes)?.item().is_none() {
        return Ok(None);
    }

    let slots = (0..SLOT_COUNT).map(|index| (WearLocation::Slot(index), inventory.slot(index)));
    let cursor = std::iter::once((WearLocation::Cursor, Some(inventory.carried())));
    let table = inventory
        .table_inputs()
        .iter()
        .enumerate()
        .map(|(index, stack)| (WearLocation::TableInput(index), Some(stack)));
    let mut preferred = None;
    let mut candidate = None;
    let mut matches = 0;
    for (location, stack) in slots.chain(cursor).chain(table) {
        let stack = stack.ok_or_else(|| anyhow::anyhow!("player inventory slot unavailable"))?;
        if encode_persisted_stack(stack)? != witness.bytes {
            continue;
        }
        matches += 1;
        candidate = Some((location, stack));
        if location == WearLocation::Slot(witness.slot) {
            preferred = candidate;
        }
    }
    let chosen = preferred.or(if matches == 1 { candidate } else { None });
    let Some((location, stack)) = chosen else {
        return Ok(None);
    };
    let durability =
        plan_durability_damage(stack, amount, inventory.game_mode() == GameMode::Creative)?;
    let broken = durability.broken();
    Ok(Some(MiningWearPlan {
        location,
        stack_after: durability.into_stack(),
        broken,
    }))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use ferrumc_items::{
        component, durability, ComponentPatch, ComponentTypeId, ComponentValue, ItemId,
        ItemPersistenceError, ItemRuleError, OpaqueComponent, MAX_PERSISTED_STACK_BYTES,
    };
    use ferrumc_nbt::{NbtCompound, NbtTag};

    use super::*;

    fn stack(name: &str, count: u8, added: Vec<ComponentValue>, removed: &[i32]) -> ItemStack {
        ItemStack::new(
            ItemId::from_name(name).expect("pinned item"),
            NonZeroU8::new(count).expect("nonempty count"),
            ComponentPatch::new(
                added,
                removed.iter().copied().map(ComponentTypeId::new).collect(),
            ),
        )
    }

    fn tool() -> ItemStack {
        stack("iron_pickaxe", 1, vec![ComponentValue::Damage(7)], &[])
    }

    fn inventory(mode: GameMode) -> PlayerInventory {
        PlayerInventory::from_persisted(
            mode,
            0,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        )
    }

    fn witness(stack: &ItemStack) -> ToolWitness {
        ToolWitness {
            slot: 36,
            bytes: encode_persisted_stack(stack).expect("witness FCIS"),
        }
    }

    fn snapshot(inventory: &PlayerInventory) -> Vec<Vec<u8>> {
        (0..SLOT_COUNT)
            .map(|index| inventory.slot(index).expect("slot"))
            .chain(std::iter::once(inventory.carried()))
            .chain(inventory.table_inputs())
            .map(|stack| encode_persisted_stack(stack).expect("valid snapshot"))
            .collect()
    }

    fn damage(plan: &MiningWearPlan) -> i32 {
        durability(plan.stack_after())
            .expect("valid durability")
            .expect("damageable")
            .damage()
    }

    #[test]
    fn original_exact_slot_wins_even_with_identical_cursor_and_table_candidates() {
        let original = tool();
        let witness = witness(&original);
        let mut inventory = inventory(GameMode::Survival);
        inventory.set_creative_slot(36, original.clone());
        inventory.set_creative_slot(9, original.clone());
        inventory.set_carried(original.clone());
        let mut table = std::array::from_fn(|_| ItemStack::empty());
        table[8] = original;
        inventory.restore_table_inputs(table);
        inventory.set_selected(5);
        let before = snapshot(&inventory);
        let plan = plan_wear(&inventory, &witness, 2)
            .expect("valid plan")
            .expect("preferred location");
        assert_eq!(plan.location(), WearLocation::Slot(36));
        assert_eq!(damage(&plan), 9);
        assert!(!plan.broken());
        assert_eq!(snapshot(&inventory), before);
    }

    #[test]
    fn moved_tool_can_be_found_in_every_player_slot() {
        let original = tool();
        let witness = witness(&original);
        for index in 0..SLOT_COUNT {
            let mut inventory = inventory(GameMode::Survival);
            inventory.set_creative_slot(index, original.clone());
            let before = snapshot(&inventory);
            let plan = plan_wear(&inventory, &witness, 1)
                .expect("valid plan")
                .expect("unique location");
            assert_eq!(plan.location(), WearLocation::Slot(index));
            assert_eq!(damage(&plan), 8);
            assert_eq!(snapshot(&inventory), before);
        }
    }

    #[test]
    fn moved_cursor_and_each_table_input_are_found_without_mutation() {
        let original = tool();
        let witness = witness(&original);
        let mut inventory = inventory(GameMode::Survival);
        inventory.set_carried(original.clone());
        let before = snapshot(&inventory);
        let plan = plan_wear(&inventory, &witness, 3)
            .expect("valid cursor plan")
            .expect("cursor location");
        assert_eq!(plan.location(), WearLocation::Cursor);
        assert_eq!(damage(&plan), 10);
        assert_eq!(snapshot(&inventory), before);
        inventory.set_carried(ItemStack::empty());
        for index in 0..9 {
            let mut table = std::array::from_fn(|_| ItemStack::empty());
            table[index] = original.clone();
            inventory.restore_table_inputs(table);
            let before = snapshot(&inventory);
            let plan = plan_wear(&inventory, &witness, 3)
                .expect("valid table plan")
                .expect("table location");
            assert_eq!(plan.location(), WearLocation::TableInput(index));
            assert_eq!(damage(&plan), 10);
            assert_eq!(snapshot(&inventory), before);
        }
    }

    #[test]
    fn ambiguous_moved_duplicates_refuse_even_when_original_slot_holds_another_tool() {
        let original = tool();
        let witness = witness(&original);
        for duplicate in [
            WearLocation::Slot(10),
            WearLocation::Cursor,
            WearLocation::TableInput(3),
        ] {
            let mut inventory = inventory(GameMode::Survival);
            inventory.set_creative_slot(9, original.clone());
            inventory.set_creative_slot(36, stack("iron_axe", 1, vec![], &[]));
            match duplicate {
                WearLocation::Slot(index) => inventory.set_creative_slot(index, original.clone()),
                WearLocation::Cursor => inventory.set_carried(original.clone()),
                WearLocation::TableInput(index) => {
                    let mut table = std::array::from_fn(|_| ItemStack::empty());
                    table[index] = original.clone();
                    inventory.restore_table_inputs(table);
                }
            }
            let before = snapshot(&inventory);
            assert!(plan_wear(&inventory, &witness, 1)
                .expect("valid search")
                .is_none());
            assert_eq!(snapshot(&inventory), before);
        }
    }

    #[test]
    fn fc_is_identity_preserves_count_patch_order_and_typed_opaque_distinction() {
        let original = stack(
            "stone",
            2,
            vec![ComponentValue::MaxDamage(20), ComponentValue::Damage(7)],
            &[],
        );
        let witness = witness(&original);
        for different in [
            stack("stone", 1, original.components().added().to_vec(), &[]),
            stack(
                "stone",
                2,
                vec![ComponentValue::Damage(7), ComponentValue::MaxDamage(20)],
                &[],
            ),
            stack(
                "stone",
                2,
                vec![
                    ComponentValue::MaxDamage(20),
                    ComponentValue::Opaque(
                        OpaqueComponent::new(3, vec![7]).expect("opaque damage"),
                    ),
                ],
                &[],
            ),
        ] {
            let mut inventory = inventory(GameMode::Survival);
            inventory.set_creative_slot(36, different);
            assert!(plan_wear(&inventory, &witness, 1)
                .expect("valid search")
                .is_none());
        }
    }

    #[test]
    fn surviving_postimage_preserves_unrelated_ordered_additions_and_removals() {
        let mut name = NbtCompound::new();
        name.push("text", NbtTag::String("named tool".into()));
        let original = stack(
            "iron_pickaxe",
            1,
            vec![
                ComponentValue::CustomName(NbtTag::Compound(name)),
                ComponentValue::Opaque(OpaqueComponent::new(3, vec![7]).expect("opaque damage")),
                ComponentValue::Opaque(
                    OpaqueComponent::new(14, vec![1, 2, 3]).expect("trusted opaque"),
                ),
            ],
            &[24, 25],
        );
        let witness = witness(&original);
        let mut inventory = inventory(GameMode::Survival);
        inventory.set_carried(original.clone());
        let plan = plan_wear(&inventory, &witness, 2)
            .expect("plan")
            .expect("cursor");
        assert_eq!(
            plan.stack_after().components().added()[0],
            original.components().added()[0]
        );
        assert_eq!(
            plan.stack_after().components().added()[1],
            ComponentValue::Damage(9)
        );
        assert_eq!(
            plan.stack_after().components().added()[2],
            original.components().added()[2]
        );
        assert_eq!(
            plan.stack_after().components().removed(),
            original.components().removed()
        );
        assert_eq!(inventory.carried(), &original);
        assert_eq!(plan.into_stack().count(), 1);
    }

    #[test]
    fn broken_single_tool_is_empty_and_multi_item_model_consumes_only_one() {
        for original in [
            stack("wooden_pickaxe", 1, vec![ComponentValue::Damage(58)], &[]),
            stack(
                "stone",
                3,
                vec![ComponentValue::MaxDamage(5), ComponentValue::Damage(4)],
                &[],
            ),
        ] {
            let witness = witness(&original);
            let mut inventory = inventory(GameMode::Survival);
            inventory.set_creative_slot(36, original.clone());
            let plan = plan_wear(&inventory, &witness, u32::MAX)
                .expect("plan")
                .expect("slot");
            assert!(plan.broken());
            assert_eq!(plan.stack_after().count(), original.count() - 1);
            if original.count() == 1 {
                assert_eq!(plan.stack_after(), &ItemStack::empty());
            } else {
                assert_eq!(plan.stack_after().item(), original.item());
                assert_eq!(damage(&plan), 5);
            }
            assert_eq!(inventory.slot(36), Some(&original));
        }
    }

    #[test]
    fn creative_unbreakable_missing_damageability_and_zero_wear_are_byte_identical() {
        for (mode, original, amount) in [
            (GameMode::Creative, tool(), u32::MAX),
            (
                GameMode::Survival,
                stack("iron_pickaxe", 1, vec![ComponentValue::Unbreakable], &[]),
                u32::MAX,
            ),
            (
                GameMode::Survival,
                stack("iron_pickaxe", 1, vec![], &[component::DAMAGE]),
                1,
            ),
            (
                GameMode::Survival,
                stack("iron_pickaxe", 1, vec![], &[component::MAX_DAMAGE]),
                1,
            ),
            (GameMode::Survival, tool(), 0),
        ] {
            let witness = witness(&original);
            let mut inventory = inventory(mode);
            inventory.set_carried(original.clone());
            let plan = plan_wear(&inventory, &witness, amount)
                .expect("plan")
                .expect("cursor");
            assert!(!plan.broken());
            assert_eq!(
                encode_persisted_stack(plan.stack_after()).expect("postimage"),
                witness.bytes
            );
            assert_eq!(inventory.carried(), &original);
        }
    }

    #[test]
    fn no_match_empty_witness_and_changed_beforeimage_do_not_retarget_an_item() {
        let original = tool();
        let witness = witness(&original);
        let mut inventory = inventory(GameMode::Survival);
        assert!(plan_wear(&inventory, &witness, 1)
            .expect("empty search")
            .is_none());
        inventory.set_carried(original);
        let plan = plan_wear(&inventory, &witness, 1)
            .expect("first plan")
            .expect("cursor");
        inventory.set_carried(plan.into_stack());
        assert!(plan_wear(&inventory, &witness, 1)
            .expect("changed search")
            .is_none());
        let empty = ToolWitness {
            slot: 36,
            bytes: encode_persisted_stack(&ItemStack::empty()).expect("empty FCIS"),
        };
        assert!(plan_wear(&inventory, &empty, 1)
            .expect("empty witness")
            .is_none());
    }

    #[test]
    fn malformed_oversized_and_invalid_slot_witnesses_refuse_without_mutation() {
        let mut inventory = inventory(GameMode::Survival);
        inventory.set_carried(tool());
        let before = snapshot(&inventory);
        let valid = witness(inventory.carried());
        for length in 0..valid.bytes.len() {
            let truncated = ToolWitness {
                slot: 36,
                bytes: valid.bytes[..length].to_vec(),
            };
            assert!(
                plan_wear(&inventory, &truncated, 1).is_err(),
                "prefix {length}"
            );
        }
        let mut trailing = valid.bytes.clone();
        trailing.push(0);
        assert!(plan_wear(
            &inventory,
            &ToolWitness {
                slot: 36,
                bytes: trailing
            },
            1
        )
        .is_err());
        let oversized = ToolWitness {
            slot: 36,
            bytes: vec![0; MAX_PERSISTED_STACK_BYTES + 1],
        };
        assert!(matches!(
            plan_wear(&inventory, &oversized, 1)
                .expect_err("oversize")
                .downcast_ref::<ItemPersistenceError>(),
            Some(ItemPersistenceError::RecordTooLarge { .. })
        ));
        assert!(plan_wear(
            &inventory,
            &ToolWitness {
                slot: SLOT_COUNT,
                bytes: valid.bytes
            },
            1
        )
        .is_err());
        assert_eq!(snapshot(&inventory), before);
    }

    #[test]
    fn unsupported_enchantments_and_malformed_durability_keep_all_beforeimages() {
        for original in [
            stack(
                "iron_pickaxe",
                1,
                vec![ComponentValue::Opaque(
                    OpaqueComponent::new(10, vec![0]).expect("enchantments"),
                )],
                &[],
            ),
            stack(
                "iron_pickaxe",
                1,
                vec![ComponentValue::Opaque(
                    OpaqueComponent::new(2, vec![0]).expect("invalid maximum"),
                )],
                &[],
            ),
        ] {
            let witness = witness(&original);
            let mut inventory = inventory(GameMode::Survival);
            // Seed corrupt restored data without going through ordinary admission.
            *inventory.slot_mut(36).expect("fixture slot") = original;
            let before = snapshot(&inventory);
            assert!(plan_wear(&inventory, &witness, 1)
                .expect_err("semantic refusal")
                .downcast_ref::<ItemRuleError>()
                .is_some());
            assert_eq!(snapshot(&inventory), before);
        }
    }

    #[test]
    fn invalid_current_stack_refuses_even_when_an_exact_preferred_tool_exists() {
        let original = tool();
        let witness = witness(&original);
        let mut inventory = inventory(GameMode::Survival);
        inventory.set_creative_slot(36, original.clone());
        let invalid = stack(
            "iron_pickaxe",
            1,
            vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
            &[],
        );
        let mut table = std::array::from_fn(|_| ItemStack::empty());
        table[8] = invalid.clone();
        inventory.restore_table_inputs(table);
        let state = inventory.state_id();
        assert!(plan_wear(&inventory, &witness, 1).is_err());
        assert_eq!(inventory.slot(36), Some(&original));
        assert_eq!(inventory.table_inputs()[8], invalid);
        assert_eq!(inventory.state_id(), state);
    }
}
