use super::*;
use crate::{ComponentValue, OpaqueComponent};

fn stack(name: &str, count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}

fn parts(input: &str, count: u8, fuel: &str, fuel_count: u8) -> [ItemStack; 3] {
    [
        stack(input, count),
        stack(fuel, fuel_count),
        ItemStack::empty(),
    ]
}

fn ticks(mut slots: [ItemStack; 3], count: u16) -> FurnaceTickPlan {
    let mut timers = (0, 0, 0, 200);
    let mut last = None;
    for _ in 0..count {
        let plan = plan_furnace_tick(&slots, timers.0, timers.1, timers.2, timers.3).unwrap();
        timers = (
            plan.burn_remaining(),
            plan.burn_duration(),
            plan.cook_progress(),
            plan.cook_total(),
        );
        slots = plan.slots().clone();
        last = Some(plan);
    }
    last.unwrap()
}

#[test]
fn complete_pinned_catalog_resolves_all_original_ingredients_and_fuels() {
    assert_eq!(data::RECIPES.len(), 140);
    assert_eq!(data::FUELS.len(), 269);
    let mut recipes = std::collections::BTreeSet::new();
    for (input, name, result, time, experience) in data::RECIPES {
        let found = smelting_recipe(&stack(input, 1)).unwrap().unwrap();
        assert_eq!(found.result(), ItemId::from_name(result).unwrap());
        assert_eq!(found.cooking_ticks(), *time);
        assert_eq!(found.experience().to_bits(), *experience);
        recipes.insert(*name);
    }
    assert_eq!(recipes.len(), 72);
    for (name, burn_ticks) in data::FUELS {
        assert_eq!(
            fuel_properties(&stack(name, 1))
                .unwrap()
                .unwrap()
                .burn_ticks(),
            *burn_ticks
        );
    }
    for name in [
        "crimson_planks",
        "warped_planks",
        "crimson_stem",
        "warped_stem",
        "stone",
        "bucket",
    ] {
        assert!(fuel_properties(&stack(name, 1)).unwrap().is_none());
    }
    assert!(smelting_recipe(&ItemStack::empty()).unwrap().is_none());
    assert!(fuel_properties(&ItemStack::empty()).unwrap().is_none());
}

#[test]
fn iron_progression_smelts_at_exact_two_hundred_ticks_and_coal_makes_eight() {
    let initial = parts("raw_iron", 8, "coal", 1);
    let before = ticks(initial.clone(), 199);
    assert_eq!(before.cook_progress(), 199);
    assert!(before.slots()[2].item().is_none());
    assert_eq!(before.slots()[0].count(), 8);
    assert!(before.slots()[1].item().is_none());
    let first = ticks(initial.clone(), 200);
    assert!(first.smelted());
    assert_eq!(first.slots()[0].count(), 7);
    assert_eq!(first.slots()[2], stack("iron_ingot", 1));
    assert_eq!(first.burn_remaining(), 1401);
    let eight = ticks(initial.clone(), 1600);
    assert_eq!(eight.slots()[2], stack("iron_ingot", 8));
    assert!(eight.slots()[0].item().is_none());
    assert_eq!(eight.burn_remaining(), 1);
    assert_eq!(initial, parts("raw_iron", 8, "coal", 1));
    assert!(!ticks(initial, 1601).is_lit());
}

#[test]
fn ores_food_logs_and_sponge_use_normal_recipe_outputs() {
    for (input, result) in [
        ("iron_ore", "iron_ingot"),
        ("deepslate_iron_ore", "iron_ingot"),
        ("raw_gold", "gold_ingot"),
        ("raw_copper", "copper_ingot"),
        ("sand", "glass"),
        ("oak_log", "charcoal"),
        ("potato", "baked_potato"),
        ("beef", "cooked_beef"),
    ] {
        assert_eq!(
            ticks(parts(input, 1, "coal", 1), 200).slots()[2],
            stack(result, 1)
        );
    }
    assert_eq!(smelting_recipe(&stack("crimson_stem", 1)).unwrap(), None);
}

#[test]
fn pinned_fuel_times_and_lava_bucket_remainder_are_exact() {
    for (name, expected) in [
        ("coal", 1600),
        ("charcoal", 1600),
        ("coal_block", 16000),
        ("blaze_rod", 2400),
        ("lava_bucket", 20000),
        ("oak_planks", 300),
        ("oak_slab", 150),
        ("stick", 100),
        ("bamboo", 50),
        ("dried_kelp_block", 4001),
        ("white_carpet", 67),
        ("oak_hanging_sign", 800),
        ("oak_boat", 1200),
    ] {
        assert_eq!(
            fuel_properties(&stack(name, 1))
                .unwrap()
                .unwrap()
                .burn_ticks(),
            expected
        );
    }
    let plan = ticks(parts("raw_iron", 1, "lava_bucket", 1), 1);
    assert!(plan.fuel_consumed());
    assert_eq!(plan.burn_remaining(), 20000);
    assert_eq!(plan.slots()[1], stack("bucket", 1));
    let done = ticks(parts("raw_iron", 1, "lava_bucket", 1), 200);
    assert_eq!(done.slots()[1], stack("bucket", 1));
    assert_eq!(done.slots()[2], stack("iron_ingot", 1));
}

#[test]
fn full_or_incompatible_output_cannot_consume_new_fuel_or_input() {
    for output in [
        stack("iron_ingot", 64),
        stack("gold_ingot", 1),
        ItemStack::try_new(
            ItemId::from_name("iron_ingot").unwrap(),
            1,
            ComponentPatch::new(vec![ComponentValue::Damage(3)], vec![]),
        )
        .unwrap(),
        ItemStack::try_new(
            ItemId::from_name("iron_ingot").unwrap(),
            16,
            ComponentPatch::new(vec![ComponentValue::MaxStackSize(16)], vec![]),
        )
        .unwrap(),
    ] {
        let mut initial = parts("raw_iron", 2, "coal", 2);
        initial[2] = output;
        let plan = plan_furnace_tick(&initial, 0, 0, 19, 200).unwrap();
        assert_eq!(plan.slots(), &initial);
        assert!(!plan.fuel_consumed());
        assert!(!plan.is_lit());
        assert_eq!(plan.cook_progress(), 0);
        let burning = plan_furnace_tick(&initial, 100, 1600, 19, 200).unwrap();
        assert_eq!(burning.burn_remaining(), 99);
        assert_eq!(burning.cook_progress(), 0);
        assert_eq!(burning.slots(), &initial);
    }
}

#[test]
fn final_output_space_is_used_once_and_then_burns_without_new_consumption() {
    let mut initial = parts("raw_iron", 2, "coal", 2);
    initial[2] = stack("iron_ingot", 63);
    let plan = plan_furnace_tick(&initial, 1, 1600, 199, 200).unwrap();
    assert!(plan.smelted());
    assert!(plan.fuel_consumed());
    assert_eq!(plan.slots()[2], stack("iron_ingot", 64));
    assert_eq!(plan.slots()[0], stack("raw_iron", 1));
    assert_eq!(plan.slots()[1], stack("coal", 1));
    let blocked = plan_furnace_tick(
        plan.slots(),
        plan.burn_remaining(),
        plan.burn_duration(),
        0,
        200,
    )
    .unwrap();
    assert!(!blocked.smelted());
    assert!(!blocked.fuel_consumed());
    assert_eq!(blocked.slots(), plan.slots());
    assert_eq!(blocked.burn_remaining(), 1599);
}

#[test]
fn surviving_input_and_fuel_components_are_preserved_exactly() {
    let mut initial = parts("raw_iron", 2, "coal", 2);
    for slot in &mut initial[..2] {
        *slot = ItemStack::try_new(
            slot.item().unwrap(),
            2,
            ComponentPatch::new(vec![ComponentValue::Damage(-7)], vec![]),
        )
        .unwrap();
    }
    let plan = ticks(initial.clone(), 200);
    for (index, original) in initial.iter().enumerate().take(2) {
        assert!(same_stack_representation(original, &plan.slots()[index]).unwrap());
        assert_eq!(plan.slots()[index].count(), 1);
    }
    assert!(plan.slots()[2].components().is_empty());
    assert_eq!(plan.slots()[2], stack("iron_ingot", 1));
}

#[test]
fn idle_cools_by_two_and_no_input_cannot_start_fuel() {
    let empty = std::array::from_fn(|_| ItemStack::empty());
    let cooling = plan_furnace_tick(&empty, 0, 1600, 199, 200).unwrap();
    assert_eq!(cooling.cook_progress(), 197);
    assert_eq!(
        plan_furnace_tick(&empty, 0, 1600, 1, 200)
            .unwrap()
            .cook_progress(),
        0
    );
    let no_input = [ItemStack::empty(), stack("coal", 2), ItemStack::empty()];
    assert_eq!(
        plan_furnace_tick(&no_input, 0, 0, 0, 200).unwrap().slots(),
        &no_input
    );
    let unburnable = parts("raw_iron", 1, "stone", 1);
    assert!(!plan_furnace_tick(&unburnable, 0, 0, 0, 200)
        .unwrap()
        .is_lit());
    let invalid_recipe = parts("cobweb", 1, "coal", 1);
    assert_eq!(
        plan_furnace_tick(&invalid_recipe, 0, 0, 0, 200)
            .unwrap()
            .slots(),
        &invalid_recipe
    );
}

#[test]
fn sponge_after_lava_turns_the_single_bucket_into_water() {
    let done = ticks(parts("wet_sponge", 1, "lava_bucket", 1), 200);
    assert_eq!(done.slots()[2], stack("sponge", 1));
    assert_eq!(done.slots()[1], stack("water_bucket", 1));
    let initial = [
        stack("wet_sponge", 1),
        stack("bucket", 2),
        ItemStack::empty(),
    ];
    assert!(matches!(
        plan_furnace_tick(&initial, 2, 20000, 199, 200),
        Err(SmeltingError::UnsupportedRemainder)
    ));
    assert_eq!(initial[1], stack("bucket", 2));
}

#[test]
fn malformed_counts_components_timers_air_and_aggregate_are_refused() {
    let valid = parts("raw_iron", 1, "coal", 1);
    for timers in [
        (1, 0, 0, 200),
        (0, 32768, 0, 200),
        (0, 0, 200, 200),
        (0, 0, 0, 0),
        (0, 0, 0, 32768),
    ] {
        assert!(matches!(
            plan_furnace_tick(&valid, timers.0, timers.1, timers.2, timers.3),
            Err(SmeltingError::InvalidTimers)
        ));
    }
    let mut invalid = valid.clone();
    invalid[0] = ItemStack::new(
        ItemId::from_name("raw_iron").unwrap(),
        NonZeroU8::new(65).unwrap(),
        ComponentPatch::empty(),
    );
    assert!(plan_furnace_tick(&invalid, 0, 0, 0, 200).is_err());
    invalid[0] = stack("air", 1);
    assert!(matches!(
        plan_furnace_tick(&invalid, 0, 0, 0, 200),
        Err(SmeltingError::PresentAir)
    ));
    invalid[0] = ItemStack::try_new(
        ItemId::from_name("raw_iron").unwrap(),
        2,
        ComponentPatch::new(vec![ComponentValue::MaxStackSize(1)], vec![]),
    )
    .unwrap();
    assert!(plan_furnace_tick(&invalid, 0, 0, 0, 200).is_err());
    let large = ComponentPatch::new(
        vec![ComponentValue::Opaque(
            OpaqueComponent::new(6, vec![0; 8192]).unwrap(),
        )],
        vec![],
    );
    let too_large = [
        ItemStack::try_new(ItemId::from_name("raw_iron").unwrap(), 1, large.clone()).unwrap(),
        ItemStack::try_new(ItemId::from_name("coal").unwrap(), 1, large).unwrap(),
        ItemStack::empty(),
    ];
    assert!(matches!(
        plan_furnace_tick(&too_large, 0, 0, 0, 200),
        Err(SmeltingError::BudgetExceeded)
    ));
}

#[test]
fn a_midcook_snapshot_can_resume_with_identical_tick_outputs() {
    let first = ticks(parts("raw_iron", 8, "coal", 1), 113);
    let mut a = first.slots().clone();
    let mut b = a.clone();
    let mut timers = (
        first.burn_remaining(),
        first.burn_duration(),
        first.cook_progress(),
        first.cook_total(),
    );
    for _ in 0..1487 {
        let left = plan_furnace_tick(&a, timers.0, timers.1, timers.2, timers.3).unwrap();
        let right = plan_furnace_tick(&b, timers.0, timers.1, timers.2, timers.3).unwrap();
        assert_eq!(left.slots(), right.slots());
        assert_eq!(left.smelted(), right.smelted());
        assert_eq!(left.fuel_consumed(), right.fuel_consumed());
        timers = (
            left.burn_remaining(),
            left.burn_duration(),
            left.cook_progress(),
            left.cook_total(),
        );
        assert_eq!(
            timers,
            (
                right.burn_remaining(),
                right.burn_duration(),
                right.cook_progress(),
                right.cook_total()
            )
        );
        a = left.into_slots();
        b = right.into_slots();
    }
    assert_eq!(a[2], stack("iron_ingot", 8));
}
