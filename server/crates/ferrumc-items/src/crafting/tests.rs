use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::num::NonZeroU8;

use ferrumc_nbt::{NbtCompound, NbtTag};

use super::*;
use crate::encode_persisted_stack;

const CASES: &str = include_str!("../../tests/fixtures/normal_crafting_1_21_8.tsv");

fn item(name: &str, count: u8) -> ItemStack {
    ItemStack::try_new(
        ItemId::from_name(name).unwrap(),
        count,
        ComponentPatch::empty(),
    )
    .unwrap()
}

fn hex(stack: &ItemStack) -> String {
    let mut bytes = Vec::new();
    stack.encode_slot(&mut bytes).unwrap();
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").unwrap();
        text
    })
}

fn patch_bytes(patch: &ComponentPatch) -> Vec<u8> {
    let stack = ItemStack::try_new(
        ItemId::from_name("minecraft:stone").unwrap(),
        1,
        patch.clone(),
    )
    .unwrap();
    encode_persisted_stack(&stack).unwrap()
}

fn input(value: &str) -> Vec<ItemStack> {
    value
        .split(',')
        .map(|slot| {
            if slot == "-" {
                return ItemStack::empty();
            }
            let parts: Vec<_> = slot.split(':').collect();
            assert_eq!(parts.len(), 3);
            let id = ItemId::new(parts[0].parse().unwrap()).unwrap();
            let count = parts[1].parse().unwrap();
            let patch = if parts[2] == "1" {
                // The actual original JsonOps-to-NBT codec produces TAG_Byte for 1.
                let mut data = NbtCompound::new();
                data.push("probe", NbtTag::Byte(1));
                ComponentPatch::new(
                    vec![ComponentValue::CustomData(NbtTag::Compound(data))],
                    vec![],
                )
            } else {
                assert_eq!(parts[2], "0");
                ComponentPatch::empty()
            };
            ItemStack::try_new(id, count, patch).unwrap()
        })
        .collect()
}

/// A separate exhaustive assignment model: remove one available position per
/// ingredient, selecting the smallest candidate set first. No augmenting paths.
fn assignment_model(recipe: &NormalRecipe, slots: &[ItemStack]) -> bool {
    let mut candidates: Vec<u16> = recipe
        .raw
        .slots
        .iter()
        .map(|index| {
            let allowed = recipe.ingredients[usize::from(*index)];
            slots.iter().enumerate().fold(0, |mask, (i, stack)| {
                if stack.item().is_some_and(|id| allowed.contains(&id.id())) {
                    mask | (1 << i)
                } else {
                    mask
                }
            })
        })
        .collect();
    candidates.sort_by_key(|mask| mask.count_ones());
    let occupied = slots.iter().enumerate().fold(0_u16, |mask, (i, stack)| {
        if stack.item().is_some() {
            mask | (1 << i)
        } else {
            mask
        }
    });
    occupied.count_ones() as usize == candidates.len() && assignment_search(&candidates, occupied)
}

fn assignment_search(candidates: &[u16], available: u16) -> bool {
    let Some((&first, rest)) = candidates.split_first() else {
        return true;
    };
    let mut choices = first & available;
    while choices != 0 {
        let selected = 1 << choices.trailing_zeros();
        choices &= !selected;
        if assignment_search(rest, available & !selected) {
            return true;
        }
    }
    false
}

#[test]
fn all_original_cases_match_assembly_remainders_and_independent_balance() {
    let catalog = NormalRecipes::builtin().unwrap();
    let mut matched = BTreeSet::new();
    let mut count = 0;
    let mut negatives = 0;
    for line in CASES.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 8);
        let recipe = catalog.recipe(fields[0]).unwrap();
        let slots = input(fields[3]);
        let snapshot: Vec<_> = slots
            .iter()
            .map(|stack| encode_persisted_stack(stack).unwrap())
            .collect();
        let grid = CraftingGrid::new(fields[2].parse().unwrap(), &slots).unwrap();
        let expected = fields[4] == "1";
        assert_eq!(
            recipe.matches(&grid),
            expected,
            "{} / {}",
            fields[0],
            fields[1]
        );
        if !recipe.is_shaped() {
            assert_eq!(
                recipe.matches(&grid),
                assignment_model(recipe, &slots),
                "independent assignment {} / {}",
                fields[0],
                fields[1]
            );
        }
        let plan = recipe.plan_one(&grid).unwrap();
        if expected {
            matched.insert(recipe.name());
            let plan = plan.unwrap();
            assert_eq!(plan.recipe(), fields[0]);
            assert_eq!(plan.side(), grid.side());
            assert_eq!(
                hex(plan.output()),
                fields[5],
                "original fixed output {}",
                fields[0]
            );
            assert_eq!(
                plan.grid_after()
                    .iter()
                    .map(hex)
                    .collect::<Vec<_>>()
                    .join(","),
                fields[6],
                "original postimages {} / {}",
                fields[0],
                fields[1]
            );
            assert_eq!(
                plan.crafting_remainders()
                    .iter()
                    .map(hex)
                    .collect::<Vec<_>>()
                    .join(","),
                fields[7],
                "original crafting rests {} / {}",
                fields[0],
                fields[1]
            );
            assert_balance(&slots, &plan);
        } else {
            negatives += 1;
            assert!(plan.is_none());
        }
        for (stack, original) in slots.iter().zip(snapshot) {
            assert_eq!(encode_persisted_stack(stack).unwrap(), original);
        }
        count += 1;
    }
    assert_eq!(count, 21_855);
    assert_eq!(matched.len(), 968);
    assert!(negatives > 4000);
}

fn assert_balance(slots: &[ItemStack], plan: &SingleCraftPlan) {
    let mut quantities_before = BTreeMap::<ItemId, u32>::new();
    let mut quantities_after = BTreeMap::<ItemId, u32>::new();
    let mut consumed = BTreeMap::<ItemId, u32>::new();
    for (i, (before, after)) in slots.iter().zip(plan.grid_after()).enumerate() {
        match before.item() {
            None => {
                assert!(after.item().is_none());
                assert_eq!(plan.consumed_mask() & (1 << i), 0);
            }
            Some(id) => {
                *quantities_before.entry(id).or_default() += u32::from(before.count());
                *consumed.entry(id).or_default() += 1;
                assert_ne!(plan.consumed_mask() & (1 << i), 0);
                assert_eq!(after.count(), before.count() - 1);
                if before.count() > 1 {
                    assert_eq!(after.item(), Some(id));
                    assert_eq!(
                        patch_bytes(before.components()),
                        patch_bytes(after.components())
                    );
                } else {
                    assert!(after.item().is_none());
                    assert!(after.components().is_empty());
                }
                *quantities_after.entry(id).or_default() += u32::from(after.count());
            }
        }
        if let Some(id) = plan.crafting_remainders()[i].item() {
            let name = before.item().unwrap().name();
            let actual = generated::REMAINDERS
                .iter()
                .find(|pair| pair.0 == name)
                .unwrap();
            assert_eq!(id.name(), actual.1);
            assert_eq!(plan.crafting_remainders()[i].count(), 1);
            assert!(plan.crafting_remainders()[i].components().is_empty());
        }
    }
    for (id, before) in quantities_before {
        assert_eq!(
            before,
            quantities_after.get(&id).copied().unwrap_or(0) + consumed[&id]
        );
    }
}

#[test]
fn complete_catalog_census_and_fixed_stew_components() {
    let catalog = NormalRecipes::builtin().unwrap();
    assert_eq!(catalog.iter().len(), 968);
    assert_eq!(
        catalog.iter().filter(|recipe| recipe.is_shaped()).count(),
        673
    );
    assert_eq!(
        catalog.iter().filter(|recipe| !recipe.is_shaped()).count(),
        295
    );
    assert_eq!(
        catalog
            .iter()
            .filter(|recipe| !recipe.output().components().is_empty())
            .count(),
        16
    );
    assert_eq!(generated::INGREDIENTS.len(), 479);
    assert_eq!(
        generated::INGREDIENTS
            .iter()
            .map(|set| set.len())
            .sum::<usize>(),
        1506
    );
    assert_eq!(
        generated::INGREDIENTS.iter().map(|set| set.len()).max(),
        Some(44)
    );
    assert_eq!(generated::REMAINDERS.len(), 5);
    assert!(catalog.recipe("acacia_boat").is_none());
    assert!(catalog
        .recipe("minecraft:crafting_special_repairitem")
        .is_none());
}

#[test]
fn a_full_stack_does_not_replace_two_ingredient_positions() {
    let catalog = NormalRecipes::builtin().unwrap();
    let recipe = catalog.recipe("minecraft:stick").unwrap();
    let slots = [
        item("minecraft:oak_planks", 64),
        ItemStack::empty(),
        ItemStack::empty(),
        ItemStack::empty(),
    ];
    let grid = CraftingGrid::new(2, &slots).unwrap();
    assert!(!recipe.matches(&grid));
    assert!(recipe.plan_one(&grid).unwrap().is_none());
}

#[test]
fn cake_keeps_all_three_buckets_separate_from_full_input_stacks() {
    let catalog = NormalRecipes::builtin().unwrap();
    let recipe = catalog.recipe("minecraft:cake").unwrap();
    let slots = [
        item("minecraft:milk_bucket", 1),
        item("minecraft:milk_bucket", 1),
        item("minecraft:milk_bucket", 1),
        item("minecraft:sugar", 64),
        item("minecraft:egg", 16),
        item("minecraft:sugar", 64),
        item("minecraft:wheat", 64),
        item("minecraft:wheat", 64),
        item("minecraft:wheat", 64),
    ];
    let grid = CraftingGrid::new(3, &slots).unwrap();
    let plan = recipe.plan_one(&grid).unwrap().unwrap();
    assert_eq!(plan.output().item().unwrap().name(), "minecraft:cake");
    for i in 0..3 {
        assert!(plan.grid_after()[i].item().is_none());
        assert_eq!(
            plan.crafting_remainders()[i].item().unwrap().name(),
            "minecraft:bucket"
        );
    }
    assert_eq!(plan.grid_after()[3].count(), 63);
}

#[test]
fn representative_greedy_trap_and_exhaustive_small_graph_assignments() {
    let stone = ItemId::from_name("minecraft:stone").unwrap();
    let cobble = ItemId::from_name("minecraft:cobblestone").unwrap();
    let mut both = vec![stone.id(), cobble.id()];
    both.sort_unstable();
    let sets: &'static [&'static [i32]] = Box::leak(
        vec![
            Box::leak(both.into_boxed_slice()) as &'static [i32],
            Box::leak(vec![stone.id()].into_boxed_slice()),
        ]
        .into_boxed_slice(),
    );
    let raw = Box::leak(Box::new(RawRecipe {
        name: "minecraft:test",
        shaped: false,
        width: 0,
        height: 0,
        slots: &[0, 1],
        result: RawOutput {
            item: "minecraft:stick",
            count: 1,
            added: &[],
            removed: &[],
        },
    }));
    let recipe = NormalRecipe {
        raw,
        ingredients: sets,
        remainders: &[],
        output: item("minecraft:stick", 1),
    };
    let slots = [
        item("minecraft:stone", 1),
        item("minecraft:cobblestone", 1),
        ItemStack::empty(),
        ItemStack::empty(),
    ];
    assert!(recipe.matches(&CraftingGrid::new(2, &slots).unwrap()));
    assert!(assignment_model(&recipe, &slots));
    // All 2^4 two-ingredient/two-item membership graphs, with repeated and
    // absent item positions. The separate model explores assignments directly.
    for graph in 0_u8..16 {
        let mut members = Vec::<&'static [i32]>::new();
        for ingredient in 0..2 {
            let mut ids = Vec::new();
            for (node, id) in [stone.id(), cobble.id()].into_iter().enumerate() {
                if graph & (1 << (ingredient * 2 + node)) != 0 {
                    ids.push(id);
                }
            }
            ids.sort_unstable();
            members.push(Box::leak(ids.into_boxed_slice()));
        }
        let sets = Box::leak(members.into_boxed_slice());
        let test = NormalRecipe {
            raw,
            ingredients: sets,
            remainders: &[],
            output: item("minecraft:stick", 1),
        };
        for first in 0..3 {
            for second in 0..3 {
                let select = |value| match value {
                    0 => ItemStack::empty(),
                    1 => item("minecraft:stone", 1),
                    _ => item("minecraft:cobblestone", 64),
                };
                let slots = [
                    select(first),
                    select(second),
                    ItemStack::empty(),
                    ItemStack::empty(),
                ];
                assert_eq!(
                    test.matches(&CraftingGrid::new(2, &slots).unwrap()),
                    assignment_model(&test, &slots),
                    "graph {graph}, slots {first}/{second}"
                );
            }
        }
    }
}

#[test]
fn lossless_admission_refuses_bad_shapes_air_invalid_stacks_and_aggregate_bytes() {
    assert!(matches!(
        CraftingGrid::new(1, &[]),
        Err(CraftingError::UnsupportedGridSide(1))
    ));
    assert!(matches!(
        CraftingGrid::new(3, &[]),
        Err(CraftingError::InvalidGridLength {
            expected: 9,
            actual: 0
        })
    ));
    let mut slots = vec![ItemStack::empty(); 4];
    slots[2] = item("minecraft:air", 1);
    let original = encode_persisted_stack(&slots[2]).unwrap();
    assert!(matches!(
        CraftingGrid::new(2, &slots),
        Err(CraftingError::PresentAirUnsupported { slot: 2 })
    ));
    assert_eq!(encode_persisted_stack(&slots[2]).unwrap(), original);
    slots[2] = ItemStack::new(
        ItemId::from_name("minecraft:milk_bucket").unwrap(),
        NonZeroU8::new(2).unwrap(),
        ComponentPatch::empty(),
    );
    assert!(matches!(
        CraftingGrid::new(2, &slots),
        Err(CraftingError::InvalidStack(_))
    ));
    let payloads: Vec<_> = (0..96)
        .filter(|id| !ComponentTypeId::new(*id).is_dangerous())
        .take(32)
        .map(|id| {
            ComponentValue::Opaque(OpaqueComponent::new(id, vec![0; MAX_COMPONENT_BYTES]).unwrap())
        })
        .collect();
    let large = ItemStack::try_new(
        ItemId::from_name("minecraft:stone").unwrap(),
        2,
        ComponentPatch::new(payloads, vec![]),
    )
    .unwrap();
    let mut slots = vec![large.clone(), large, ItemStack::empty(), ItemStack::empty()];
    assert!(matches!(
        CraftingGrid::new(2, &slots),
        Err(CraftingError::ResourceLimit {
            resource: "grid encoded",
            ..
        })
    ));
    slots[0] = ItemStack::empty();
    assert!(CraftingGrid::new(2, &slots).is_ok());
}

#[test]
fn logical_clone_budget_is_independent_from_encoded_byte_budget() {
    let mut compound = NbtCompound::new();
    compound.push(
        "list",
        NbtTag::List(vec![NbtTag::String(String::new()); 4000]),
    );
    let tree = NbtTag::Compound(compound);
    let large = ItemStack::try_new(
        ItemId::from_name("minecraft:stone").unwrap(),
        2,
        ComponentPatch::new(
            vec![
                ComponentValue::CustomData(tree.clone()),
                ComponentValue::CustomName(tree),
            ],
            vec![],
        ),
    )
    .unwrap();
    let slots = vec![large; 9];
    let encoded: usize = slots
        .iter()
        .map(|stack| authoritative_encoded_size(stack).unwrap())
        .sum();
    assert!(encoded < MAX_CRAFTING_GRID_BYTES);
    assert!(matches!(
        CraftingGrid::new(3, &slots),
        Err(CraftingError::ResourceLimit {
            resource: "grid clone",
            ..
        })
    ));
}

#[test]
fn retained_patches_preserve_variant_order_duplicate_nbt_names_and_float_bits() {
    let catalog = NormalRecipes::builtin().unwrap();
    let recipe = catalog.recipe("minecraft:stick").unwrap();
    let mut compound = NbtCompound::new();
    compound.push("x", NbtTag::Float(f32::from_bits(0x7fc0_1234)));
    compound.push("x", NbtTag::Double(-0.0));
    let patch = ComponentPatch::new(
        vec![
            ComponentValue::CustomData(NbtTag::Compound(compound)),
            ComponentValue::Opaque(OpaqueComponent::new(3, vec![0x81, 0]).unwrap()),
        ],
        vec![ComponentTypeId::new(49)],
    );
    let plank =
        ItemStack::try_new(ItemId::from_name("minecraft:oak_planks").unwrap(), 2, patch).unwrap();
    let slots = [plank.clone(), ItemStack::empty(), plank, ItemStack::empty()];
    let plan = recipe
        .plan_one(&CraftingGrid::new(2, &slots).unwrap())
        .unwrap()
        .unwrap();
    assert!(plan.output().components().is_empty());
    for i in [0, 2] {
        assert_eq!(
            patch_bytes(slots[i].components()),
            patch_bytes(plan.grid_after()[i].components())
        );
    }
}

#[test]
fn malformed_components_are_refused_without_normalization() {
    let bad = ItemStack::try_new(
        ItemId::from_name("minecraft:oak_planks").unwrap(),
        2,
        ComponentPatch::new(
            vec![ComponentValue::Damage(1), ComponentValue::Damage(2)],
            vec![],
        ),
    )
    .unwrap();
    let slots = [
        bad,
        ItemStack::empty(),
        ItemStack::empty(),
        ItemStack::empty(),
    ];
    assert!(matches!(
        CraftingGrid::new(2, &slots),
        Err(CraftingError::InvalidStack(
            ItemPersistenceError::DuplicateComponent(3)
        ))
    ));
    assert_eq!(slots[0].components().added().len(), 2);
}

#[test]
fn descriptor_construction_and_exact_resource_boundaries_refuse_invalid_inputs() {
    assert!(matches!(
        validate_catalog(&[], &[], &[], &[]),
        Err(CraftingError::InvalidCatalog(_))
    ));
    assert!(matches!(
        validate_catalog(&[&[0]], generated::RECIPES, &[], &[]),
        Err(CraftingError::InvalidCatalog("ingredient item"))
    ));
    assert!(matches!(
        validate_catalog(&[&[2, 1]], generated::RECIPES, &[], &[]),
        Err(CraftingError::InvalidCatalog("ingredient set"))
    ));
    assert!(matches!(
        validate_catalog(
            generated::INGREDIENTS,
            generated::RECIPES,
            &[("minecraft:stone", "minecraft:air")],
            &[]
        ),
        Err(CraftingError::InvalidCatalog(_))
    ));
    assert!(matches!(
        validate_catalog(
            generated::INGREDIENTS,
            generated::RECIPES,
            &[],
            &[("minecraft:stone", 1, 1)]
        ),
        Err(CraftingError::InvalidCatalog("item registry pin"))
    ));
    assert!(limit("test", MAX_CRAFT_PLAN_BYTES, MAX_CRAFT_PLAN_BYTES).is_ok());
    assert!(matches!(
        limit("test", MAX_CRAFT_PLAN_BYTES + 1, MAX_CRAFT_PLAN_BYTES),
        Err(CraftingError::ResourceLimit { .. })
    ));
    let mut sum = usize::MAX;
    assert_eq!(
        checked_add(&mut sum, 1),
        Err(CraftingError::ResourceOverflow)
    );
    assert_eq!(sum, usize::MAX);
}

fn descriptor() -> RawRecipe {
    RawRecipe {
        name: "minecraft:test",
        shaped: true,
        width: 1,
        height: 1,
        slots: &[0],
        result: RawOutput {
            item: "minecraft:stick",
            count: 1,
            added: &[],
            removed: &[],
        },
    }
}

#[test]
fn private_descriptor_shape_indices_and_output_bounds_fail_closed() {
    let check = |raw| validate_catalog(generated::INGREDIENTS, &[raw], &[], &[]);
    let mut raw = descriptor();
    raw.name = "minecraft:INVALID";
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("recipe descriptor"))
    );
    let mut raw = descriptor();
    raw.width = 0;
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("shaped dimensions"))
    );
    let mut raw = descriptor();
    raw.width = 2;
    raw.slots = &[0, EMPTY_INGREDIENT];
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("untrimmed pattern"))
    );
    let mut raw = descriptor();
    raw.slots = &[1024];
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("ingredient index"))
    );
    let mut raw = descriptor();
    raw.shaped = false;
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("shapeless descriptor"))
    );
    let mut raw = descriptor();
    raw.result.count = 0;
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("fixed output"))
    );
    let mut raw = descriptor();
    raw.result.count = 65;
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("fixed output"))
    );
}

#[test]
fn private_fixed_components_validate_before_copying_any_payload() {
    let check = |raw| validate_catalog(generated::INGREDIENTS, &[raw], &[], &[]);
    let mut raw = descriptor();
    raw.result.added = &[(96, &[])];
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("fixed component"))
    );
    let mut raw = descriptor();
    raw.result.added = &[(49, &[])];
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("fixed component"))
    );
    let mut raw = descriptor();
    raw.result.added = &[(3, &[]), (3, &[])];
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("duplicate fixed component"))
    );
    let mut raw = descriptor();
    raw.result.added = &[(3, &[])];
    raw.result.removed = &[3];
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("fixed removal"))
    );
    let mut raw = descriptor();
    raw.result.removed = &[-1];
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("fixed removal"))
    );
    let oversized = Box::leak(vec![0_u8; MAX_COMPONENT_BYTES + 1].into_boxed_slice());
    let mut raw = descriptor();
    raw.result.added = Box::leak(vec![(42, oversized as &'static [u8])].into_boxed_slice());
    assert_eq!(
        check(raw),
        Err(CraftingError::InvalidCatalog("fixed component"))
    );
}
