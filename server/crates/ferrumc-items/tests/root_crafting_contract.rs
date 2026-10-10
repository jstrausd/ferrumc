use ferrumc_items::{
    encode_persisted_stack, ComponentPatch, ComponentValue, CraftingGrid, ItemId, ItemStack,
    NormalRecipes,
};
use ferrumc_nbt::{NbtCompound, NbtTag};

fn stack(name: &str, count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}

#[test]
fn exhaustive_three_by_three_shapes_follow_fixed_original_slot_masks() {
    let recipes = NormalRecipes::builtin().unwrap();
    let sticks = recipes.recipe("minecraft:stick").unwrap();
    let axe = recipes.recipe("minecraft:wooden_axe").unwrap();
    // Independent concrete positions derived from the original resource's
    // XX / X# / _# pattern, including both horizontal placements and mirrors.
    let axe_masks = [(11, 144), (22, 288), (19, 72), (38, 144)];
    let stick_masks = [9, 18, 36, 72, 144, 288];
    for mut case in 0..3_u32.pow(9) {
        let mut planks = 0_u16;
        let mut handles = 0_u16;
        let slots: Vec<_> = (0..9)
            .map(|i| {
                let choice = case % 3;
                case /= 3;
                match choice {
                    1 => {
                        planks |= 1 << i;
                        stack(
                            if i % 2 == 0 {
                                "minecraft:oak_planks"
                            } else {
                                "minecraft:crimson_planks"
                            },
                            if i % 2 == 0 { 64 } else { 1 },
                        )
                    }
                    2 => {
                        handles |= 1 << i;
                        stack("minecraft:stick", 64)
                    }
                    _ => ItemStack::empty(),
                }
            })
            .collect();
        let grid = CraftingGrid::new(3, &slots).unwrap();
        let expected_sticks = handles == 0 && stick_masks.contains(&planks);
        let expected_axe = axe_masks.contains(&(planks, handles));
        assert_eq!(
            sticks.matches(&grid),
            expected_sticks,
            "planks={planks},handles={handles}"
        );
        assert_eq!(
            axe.matches(&grid),
            expected_axe,
            "planks={planks},handles={handles}"
        );
        for (recipe, matched) in [(sticks, expected_sticks), (axe, expected_axe)] {
            let plan = recipe.plan_one(&grid).unwrap();
            assert_eq!(plan.is_some(), matched);
            if let Some(plan) = plan {
                assert_eq!(plan.consumed_mask(), planks | handles);
                for (before, after) in slots.iter().zip(plan.grid_after()) {
                    assert_eq!(after.count(), before.count().saturating_sub(1));
                }
                assert!(plan
                    .crafting_remainders()
                    .iter()
                    .all(|stack| stack.item().is_none()));
            }
        }
    }
}

#[test]
fn exhaustive_book_positions_count_occupied_slots_and_forbid_extra_items() {
    let recipes = NormalRecipes::builtin().unwrap();
    let book = recipes.recipe("minecraft:book").unwrap();
    for mut case in 0..4_u32.pow(9) {
        let (mut papers, mut leather, mut foreign) = (0, 0, 0);
        let slots: Vec<_> = (0..9)
            .map(|_| {
                let choice = case % 4;
                case /= 4;
                match choice {
                    1 => {
                        papers += 1;
                        stack("minecraft:paper", 64)
                    }
                    2 => {
                        leather += 1;
                        stack("minecraft:leather", 1)
                    }
                    3 => {
                        foreign += 1;
                        stack("minecraft:stick", 64)
                    }
                    _ => ItemStack::empty(),
                }
            })
            .collect();
        let expected = papers == 3 && leather == 1 && foreign == 0;
        let grid = CraftingGrid::new(3, &slots).unwrap();
        assert_eq!(book.matches(&grid), expected);
        if expected {
            let plan = book.plan_one(&grid).unwrap().unwrap();
            assert_eq!(plan.output().item().unwrap().name(), "minecraft:book");
            assert_eq!(plan.output().count(), 1);
            assert_eq!(plan.consumed_mask().count_ones(), 4);
            assert!(plan
                .crafting_remainders()
                .iter()
                .all(|stack| stack.item().is_none()));
        }
    }
}

#[test]
fn owned_plans_survive_input_and_catalog_drop_without_losing_patch_bits() {
    let mut compound = NbtCompound::new();
    compound.push(
        "duplicate",
        NbtTag::Double(f64::from_bits(0x7ff8_0000_0000_0321)),
    );
    compound.push("duplicate", NbtTag::Float(-0.0));
    let patched = ItemStack::try_new(
        ItemId::from_name("minecraft:oak_planks").unwrap(),
        2,
        ComponentPatch::new(
            vec![ComponentValue::CustomData(NbtTag::Compound(compound))],
            vec![],
        ),
    )
    .unwrap();
    let original = encode_persisted_stack(&patched).unwrap();
    let survivor =
        ItemStack::try_new(patched.item().unwrap(), 1, patched.components().clone()).unwrap();
    let survivor_bytes = encode_persisted_stack(&survivor).unwrap();
    let plan = {
        let recipes = NormalRecipes::builtin().unwrap();
        let slots = [
            ItemStack::empty(),
            patched.clone(),
            ItemStack::empty(),
            patched,
        ];
        let plan = recipes
            .recipe("minecraft:stick")
            .unwrap()
            .plan_one(&CraftingGrid::new(2, &slots).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(encode_persisted_stack(&slots[1]).unwrap(), original);
        plan
    };
    assert_eq!(plan.consumed_mask(), 10);
    assert_eq!(plan.output().count(), 4);
    assert!(plan.output().components().is_empty());
    for index in [1, 3] {
        assert_eq!(
            encode_persisted_stack(&plan.grid_after()[index]).unwrap(),
            survivor_bytes
        );
    }
}
