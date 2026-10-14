use super::*;
use crate::{ComponentTypeId, OpaqueComponent};

fn stack(name: &str, added: Vec<ComponentValue>, removed: Vec<i32>) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).unwrap(),
        1,
        ComponentPatch::new(
            added,
            removed.into_iter().map(ComponentTypeId::new).collect(),
        ),
    )
    .unwrap()
}
fn scalar_food(raw: Vec<u8>) -> ItemStack {
    stack(
        "apple",
        vec![ComponentValue::Opaque(
            OpaqueComponent::new(FOOD, raw).unwrap(),
        )],
        vec![],
    )
}

#[test]
fn closed_catalog_covers_all_foods_and_all_original_durability() {
    assert_eq!(data::FOODS.len(), 40);
    assert_eq!(data::DURABILITY.len(), 68);
    for (name, _) in data::FOODS {
        let item = ItemId::from_name(name).unwrap();
        let value = food_properties(&stack(name, vec![], vec![]))
            .unwrap()
            .unwrap();
        assert_eq!(builtin_food(item).unwrap(), value);
        assert!(value.consume_ticks() > 0);
        if let Some(remainder) = value.remainder() {
            assert!(matches!(
                remainder.name(),
                "minecraft:bowl" | "minecraft:glass_bottle"
            ));
        }
    }
    for (name, maximum) in data::DURABILITY {
        let state = durability(&stack(name, vec![], vec![])).unwrap().unwrap();
        assert_eq!(state.max_damage(), *maximum);
        assert_eq!(state.damage(), 0);
    }
    assert_eq!(
        builtin_max_damage(ItemId::from_name("wooden_pickaxe").unwrap()),
        Some(59)
    );
    assert_eq!(
        builtin_max_damage(ItemId::from_name("diamond_sword").unwrap()),
        Some(1561)
    );
    assert_eq!(
        builtin_max_damage(ItemId::from_name("turtle_helmet").unwrap()),
        Some(275)
    );
    assert_eq!(
        builtin_max_damage(ItemId::from_name("elytra").unwrap()),
        Some(432)
    );
    assert_eq!(
        builtin_max_damage(ItemId::from_name("mace").unwrap()),
        Some(500)
    );
}

#[test]
fn food_has_absolute_saturation_correct_durations_remainders_and_effects() {
    let apple = builtin_food(ItemId::from_name("apple").unwrap()).unwrap();
    assert_eq!(apple.nutrition(), 4);
    assert_eq!(apple.saturation().to_bits(), 2.4_f32.to_bits());
    assert_eq!(apple.consume_ticks(), 32);
    assert!(!apple.always_edible());
    let honey = builtin_food(ItemId::from_name("honey_bottle").unwrap()).unwrap();
    assert_eq!(honey.consume_ticks(), 40);
    assert!(honey.always_edible());
    assert_eq!(honey.remainder(), ItemId::from_name("glass_bottle"));
    assert_eq!(honey.effects()[0].kind(), FoodEffectKind::ClearPoison);
    let kelp = builtin_food(ItemId::from_name("dried_kelp").unwrap()).unwrap();
    assert_eq!(kelp.consume_ticks(), 16);
    let stew = builtin_food(ItemId::from_name("rabbit_stew").unwrap()).unwrap();
    assert_eq!(stew.nutrition(), 10);
    assert_eq!(stew.remainder(), ItemId::from_name("bowl"));
    let pufferfish = builtin_food(ItemId::from_name("pufferfish").unwrap()).unwrap();
    assert_eq!(pufferfish.effects().len(), 3);
    assert_eq!(pufferfish.effects()[0].kind(), FoodEffectKind::Poison);
    assert_eq!(pufferfish.effects()[0].ticks(), 1200);
    assert_eq!(pufferfish.effects()[0].amplifier(), 1);
    assert_eq!(pufferfish.effects()[0].probability(), 1.0);
    let chicken = builtin_food(ItemId::from_name("chicken").unwrap()).unwrap();
    assert_eq!(chicken.effects()[0].probability(), 0.3);
}

#[test]
fn food_scalar_parser_refuses_every_truncation_malformed_bool_float_and_tail() {
    let mut valid = vec![7];
    valid.extend_from_slice(&5.5_f32.to_be_bytes());
    valid.push(1);
    let food = food_properties(&scalar_food(valid.clone()))
        .unwrap()
        .unwrap();
    assert_eq!(food.nutrition(), 7);
    assert_eq!(food.saturation(), 5.5);
    assert!(food.always_edible());
    for end in 0..valid.len() {
        assert!(food_properties(&scalar_food(valid[..end].to_vec())).is_err());
    }
    let mut tail = valid.clone();
    tail.push(0);
    assert!(food_properties(&scalar_food(tail)).is_err());
    let mut boolean = valid.clone();
    boolean[5] = 2;
    assert!(food_properties(&scalar_food(boolean)).is_err());
    for value in [f32::NAN, f32::INFINITY, -1.0] {
        let mut bad = valid.clone();
        bad[1..5].copy_from_slice(&value.to_be_bytes());
        assert!(food_properties(&scalar_food(bad)).is_err());
    }
    let mut negative = vec![0xff, 0xff, 0xff, 0xff, 0x0f];
    negative.extend_from_slice(&1.0_f32.to_be_bytes());
    negative.push(0);
    assert!(food_properties(&scalar_food(negative)).is_err());
    assert!(food_properties(&stack("apple", vec![], vec![FOOD]))
        .unwrap()
        .is_none());
    assert!(food_properties(&stack("apple", vec![], vec![CONSUMABLE]))
        .unwrap()
        .is_none());
    assert!(
        food_properties(&stack("honey_bottle", vec![], vec![USE_REMAINDER]))
            .unwrap()
            .unwrap()
            .remainder()
            .is_none()
    );
}

#[test]
fn durability_preserves_other_patches_suppresses_creative_and_unbreakable_and_breaks_once() {
    let original = stack(
        "wooden_pickaxe",
        vec![
            ComponentValue::Damage(57),
            ComponentValue::CustomData(ferrumc_nbt::NbtTag::Compound(
                ferrumc_nbt::NbtCompound::new(),
            )),
        ],
        vec![],
    );
    let surviving = plan_durability_damage(&original, 1, false).unwrap();
    assert!(!surviving.broken());
    assert_eq!(
        durability(surviving.stack_after())
            .unwrap()
            .unwrap()
            .damage(),
        58
    );
    assert_eq!(
        surviving.stack_after().components().added()[1],
        original.components().added()[1]
    );
    assert_eq!(durability(&original).unwrap().unwrap().damage(), 57);
    let broken = plan_durability_damage(surviving.stack_after(), 1, false).unwrap();
    assert!(broken.broken());
    assert!(broken.stack_after().item().is_none());
    assert_eq!(
        plan_durability_damage(&original, u32::MAX, true)
            .unwrap()
            .stack_after(),
        &original
    );
    let unbreakable = stack(
        "wooden_pickaxe",
        vec![ComponentValue::Damage(58), ComponentValue::Unbreakable],
        vec![],
    );
    assert!(durability(&unbreakable).unwrap().unwrap().unbreakable());
    assert_eq!(
        plan_durability_damage(&unbreakable, u32::MAX, false)
            .unwrap()
            .stack_after(),
        &unbreakable
    );
    assert!(plan_durability_damage(&original, u32::MAX, false)
        .unwrap()
        .broken());
}

#[test]
fn durability_respects_max_damage_removals_signed_damage_and_opaque_scalars() {
    let custom = stack(
        "stone",
        vec![ComponentValue::MaxDamage(5), ComponentValue::Damage(-7)],
        vec![],
    );
    assert_eq!(durability(&custom).unwrap().unwrap().damage(), 0);
    assert_eq!(
        durability(
            plan_durability_damage(&custom, 2, false)
                .unwrap()
                .stack_after()
        )
        .unwrap()
        .unwrap()
        .damage(),
        2
    );
    assert!(
        durability(&stack("iron_pickaxe", vec![], vec![component::MAX_DAMAGE]))
            .unwrap()
            .is_none()
    );
    assert!(
        durability(&stack("iron_pickaxe", vec![], vec![component::DAMAGE]))
            .unwrap()
            .is_none()
    );
    let opaque = stack(
        "stone",
        vec![
            ComponentValue::Opaque(OpaqueComponent::new(2, vec![5]).unwrap()),
            ComponentValue::Opaque(OpaqueComponent::new(3, vec![4]).unwrap()),
        ],
        vec![],
    );
    assert!(plan_durability_damage(&opaque, 1, false).unwrap().broken());
    for raw in [vec![], vec![0x80], vec![1, 0], vec![0]] {
        let invalid = stack(
            "iron_pickaxe",
            vec![ComponentValue::Opaque(
                OpaqueComponent::new(2, raw).unwrap(),
            )],
            vec![],
        );
        assert!(durability(&invalid).is_err());
    }
}

#[test]
fn armor_tool_values_and_component_removals_match_original_defaults() {
    let diamond = armor_properties(&stack("diamond_chestplate", vec![], vec![]))
        .unwrap()
        .unwrap();
    assert_eq!(diamond.inventory_slot(), 6);
    assert_eq!(diamond.armor(), 8.0);
    assert_eq!(diamond.toughness(), 2.0);
    let netherite = armor_properties(&stack("netherite_boots", vec![], vec![]))
        .unwrap()
        .unwrap();
    assert_eq!(netherite.inventory_slot(), 8);
    assert_eq!(netherite.armor(), 3.0);
    assert_eq!(netherite.knockback_resistance(), 0.1);
    assert!(!armor_properties(&stack("elytra", vec![], vec![]))
        .unwrap()
        .unwrap()
        .damage_on_hurt());
    assert!(!armor_properties(&stack("carved_pumpkin", vec![], vec![]))
        .unwrap()
        .unwrap()
        .swappable());
    assert!(
        armor_properties(&stack("diamond_chestplate", vec![], vec![EQUIPPABLE]))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        armor_properties(&stack(
            "diamond_chestplate",
            vec![],
            vec![ATTRIBUTE_MODIFIERS]
        ))
        .unwrap()
        .unwrap()
        .armor(),
        0.0
    );
    let pickaxe = tool_properties(&stack("iron_pickaxe", vec![], vec![]))
        .unwrap()
        .unwrap();
    assert_eq!(pickaxe.attack_damage(), 4.0);
    assert_eq!(pickaxe.block_durability_cost(), 1);
    assert_eq!(pickaxe.attack_durability_cost(), 2);
    let sword = tool_properties(&stack("diamond_sword", vec![], vec![]))
        .unwrap()
        .unwrap();
    assert_eq!(sword.attack_damage(), 7.0);
    assert_eq!(sword.block_durability_cost(), 2);
    assert_eq!(sword.attack_durability_cost(), 1);
    assert_eq!(
        tool_properties(&stack("iron_pickaxe", vec![], vec![25]))
            .unwrap()
            .unwrap()
            .block_durability_cost(),
        0
    );
}

#[test]
fn armor_formula_has_damage_toughness_response_and_refuses_nonfinite_input() {
    assert_eq!(damage_after_armor(10.0, 0.0, 0.0).unwrap(), 10.0);
    assert_eq!(
        damage_after_armor(10.0, 20.0, 0.0).unwrap().to_bits(),
        3.999_999_8_f32.to_bits()
    );
    assert_eq!(damage_after_armor(10.0, 20.0, 8.0).unwrap(), 3.0);
    assert_eq!(damage_after_armor(50.0, 20.0, 0.0).unwrap(), 42.0);
    for args in [
        (f32::NAN, 20.0, 0.0),
        (1.0, f32::INFINITY, 0.0),
        (-1.0, 0.0, 0.0),
        (1.0, 31.0, 0.0),
        (1.0, 0.0, 21.0),
    ] {
        assert!(damage_after_armor(args.0, args.1, args.2).is_err());
    }
}
