//! Immutable ordinary player-attack equipment evidence and checked wear plans.
//! Accepted receipts, retained-evidence limits, exact player identity, epochs,
//! deduplication, lineage, commits and publication belong to runtime owners.

use ferrumc_items::{durability, encode_persisted_stack, ItemRuleError, MAX_PERSISTED_STACK_BYTES};

use crate::combat::CombatProfile;
use crate::inventory::{PlayerInventory, HOTBAR_START};
use crate::mining_wear::{plan_wear, MiningWearPlan};
use crate::mining_witness::ToolWitness;

const EQUIPPABLE: i32 = 28;
const DAMAGE_RESISTANT: i32 = 24;
const MAX_SNAPSHOT_BYTES: usize = 5 * MAX_PERSISTED_STACK_BYTES;

// Actual original Item.components().EQUIPPABLE.damageOnHurt values, exported
// from guarded Paper1.21.8 build60 classes; no equip-slot compatibility gate.
const DAMAGE_ON_HURT: &[&str] = &[
    "minecraft:black_carpet",
    "minecraft:black_harness",
    "minecraft:blue_carpet",
    "minecraft:blue_harness",
    "minecraft:brown_carpet",
    "minecraft:brown_harness",
    "minecraft:carved_pumpkin",
    "minecraft:chainmail_boots",
    "minecraft:chainmail_chestplate",
    "minecraft:chainmail_helmet",
    "minecraft:chainmail_leggings",
    "minecraft:creeper_head",
    "minecraft:cyan_carpet",
    "minecraft:cyan_harness",
    "minecraft:diamond_boots",
    "minecraft:diamond_chestplate",
    "minecraft:diamond_helmet",
    "minecraft:diamond_leggings",
    "minecraft:dragon_head",
    "minecraft:golden_boots",
    "minecraft:golden_chestplate",
    "minecraft:golden_helmet",
    "minecraft:golden_leggings",
    "minecraft:gray_carpet",
    "minecraft:gray_harness",
    "minecraft:green_carpet",
    "minecraft:green_harness",
    "minecraft:iron_boots",
    "minecraft:iron_chestplate",
    "minecraft:iron_helmet",
    "minecraft:iron_leggings",
    "minecraft:leather_boots",
    "minecraft:leather_chestplate",
    "minecraft:leather_helmet",
    "minecraft:leather_leggings",
    "minecraft:light_blue_carpet",
    "minecraft:light_blue_harness",
    "minecraft:light_gray_carpet",
    "minecraft:light_gray_harness",
    "minecraft:lime_carpet",
    "minecraft:lime_harness",
    "minecraft:magenta_carpet",
    "minecraft:magenta_harness",
    "minecraft:netherite_boots",
    "minecraft:netherite_chestplate",
    "minecraft:netherite_helmet",
    "minecraft:netherite_leggings",
    "minecraft:orange_carpet",
    "minecraft:orange_harness",
    "minecraft:piglin_head",
    "minecraft:pink_carpet",
    "minecraft:pink_harness",
    "minecraft:player_head",
    "minecraft:purple_carpet",
    "minecraft:purple_harness",
    "minecraft:red_carpet",
    "minecraft:red_harness",
    "minecraft:saddle",
    "minecraft:shield",
    "minecraft:skeleton_skull",
    "minecraft:turtle_helmet",
    "minecraft:white_carpet",
    "minecraft:white_harness",
    "minecraft:wither_skeleton_skull",
    "minecraft:wolf_armor",
    "minecraft:yellow_carpet",
    "minecraft:yellow_harness",
    "minecraft:zombie_head",
];

struct ArmorWitness {
    witness: ToolWitness,
    damage_on_hurt: bool,
}

/// Fixed five-location FCIS evidence with the exact captured ordinary profile.
/// Share this immutable value instead of cloning its retained component bytes.
pub(crate) struct CombatWearSnapshot {
    profile: CombatProfile,
    hand: ToolWitness,
    armor: [ArmorWitness; 4],
    encoded_bytes: usize,
}

impl CombatWearSnapshot {
    /// Capture selected main hand and armor slots5..8 without changing inventory.
    /// The existing profile admits capacities/durability and refuses unsupported
    /// combat semantics. Added armor `DAMAGE_RESISTANT` is additionally refused;
    /// static defaults resist fire only, which ordinary player attacks do not use.
    /// Each FCIS is independently bounded; exactly five fit the fixed total cap.
    pub(crate) fn capture(inventory: &PlayerInventory) -> anyhow::Result<Self> {
        let profile = CombatProfile::from_inventory(inventory)?;
        let hand = capture_witness(inventory, HOTBAR_START + usize::from(inventory.selected()))?;
        let armor = [
            capture_armor(inventory, 5)?,
            capture_armor(inventory, 6)?,
            capture_armor(inventory, 7)?,
            capture_armor(inventory, 8)?,
        ];
        let encoded_bytes = hand.bytes.len()
            + armor
                .iter()
                .map(|armor| armor.witness.bytes.len())
                .sum::<usize>();
        anyhow::ensure!(
            encoded_bytes <= MAX_SNAPSHOT_BYTES,
            "combat equipment witness budget exceeded"
        );
        Ok(Self {
            profile,
            hand,
            armor,
            encoded_bytes,
        })
    }

    /// Exact ordinary attributes resolved from the captured equipment.
    pub(crate) fn profile(&self) -> CombatProfile {
        self.profile
    }

    /// Borrow the exact selected-hand beforeimage without copying its FCIS bytes.
    pub(crate) fn hand_witness(&self) -> &ToolWitness {
        &self.hand
    }

    /// Sum of the five retained FCIS record lengths, excluding allocation overhead.
    /// Runtime owners must separately cap pending counts and decoded/heap costs.
    pub(crate) fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
}

fn capture_witness(inventory: &PlayerInventory, slot: usize) -> anyhow::Result<ToolWitness> {
    let stack = inventory
        .slot(slot)
        .ok_or_else(|| anyhow::anyhow!("combat equipment slot unavailable"))?;
    let bytes = encode_persisted_stack(stack)?;
    anyhow::ensure!(
        bytes.len() <= MAX_PERSISTED_STACK_BYTES,
        "combat equipment record budget exceeded"
    );
    Ok(ToolWitness { slot, bytes })
}

fn capture_armor(inventory: &PlayerInventory, slot: usize) -> anyhow::Result<ArmorWitness> {
    let stack = inventory
        .slot(slot)
        .ok_or_else(|| anyhow::anyhow!("combat armor slot unavailable"))?;
    for added in stack.components().added() {
        if added.type_id() == DAMAGE_RESISTANT {
            return Err(ItemRuleError::UnsupportedComponent {
                id: DAMAGE_RESISTANT,
            }
            .into());
        }
    }
    let equippable_removed = stack
        .components()
        .removed()
        .iter()
        .any(|removed| removed.get() == EQUIPPABLE);
    let damage_on_hurt = !equippable_removed
        && stack
            .item()
            .is_some_and(|item| DAMAGE_ON_HURT.binary_search(&item.name()).is_ok())
        && durability(stack)?.is_some_and(|state| !state.unbreakable());
    Ok(ArmorWitness {
        witness: capture_witness(inventory, slot)?,
        damage_on_hurt,
    })
}

/// Prepare captured main-hand wear only for the caller's accepted outcome.
/// Acceptance is supplied by the tracked Sim receipt, never inferred here.
/// The exact beforeimage may have moved to any player/cursor/table location.
/// Missing/ambiguous matches return None; current Creative policy is inherited
/// from the shared Items planner. No target-armor or cross-player commit occurs.
pub(crate) fn plan_weapon_wear(
    snapshot: &CombatWearSnapshot,
    current: &PlayerInventory,
    accepted: bool,
) -> anyhow::Result<Option<MiningWearPlan>> {
    if !accepted || snapshot.profile.attack_wear() == 0 {
        return Ok(None);
    }
    plan_wear(current, &snapshot.hand, snapshot.profile.attack_wear())
}

/// All checked armor postimages, prepared before any can be published.
/// An absent exact witness is omitted; duplicate output locations refuse the
/// whole plan so overlapping witnesses cannot overwrite or double-wear an item.
#[derive(Debug)]
pub(crate) struct ArmorWearPlan {
    postimages: [Option<MiningWearPlan>; 4],
}

impl ArmorWearPlan {
    /// Inspect every checked postimage in original FEET/LEGS/CHEST/HEAD order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &MiningWearPlan> {
        self.postimages.iter().filter_map(Option::as_ref)
    }

    /// Move the complete private plan into its exclusive inventory owner.
    pub(crate) fn into_postimages(self) -> impl Iterator<Item = MiningWearPlan> {
        self.postimages.into_iter().flatten()
    }
}

/// Plan ordinary player-attack armor wear using the accepted pre-armor increment.
///
/// `raw_damage` must come from `PlayerAttackResolved`, after protected hurt-cooldown
/// reduction. It is never full attempted damage, final health loss or overkill.
/// Finite nonnegative receipts are required; accepted receipts require positive
/// raw damage. Rejected receipts yield no wear. The original positive amount is
/// `(int) max(1f, raw_damage / 4f)`, including Java's saturating f32-to-i32 cast.
///
/// Frozen static EQUIPPABLE.damageOnHurt, damageability and unbreakable rules
/// govern eligibility even in a mismatched armor slot. Exact moved-stack lookup
/// uses `mining_wear`. Every selected postimage passes Items planning before return.
/// Runtime owners commit only against the inspected immutable current inventory;
/// this function cannot authorize, deduplicate or atomically combine both players.
pub(crate) fn plan_armor_wear(
    snapshot: &CombatWearSnapshot,
    current: &PlayerInventory,
    accepted: bool,
    raw_damage: f32,
) -> anyhow::Result<ArmorWearPlan> {
    anyhow::ensure!(
        raw_damage.is_finite() && raw_damage >= 0.0,
        "invalid pre-armor combat damage receipt"
    );
    let mut result = ArmorWearPlan {
        postimages: std::array::from_fn(|_| None),
    };
    if !accepted {
        return Ok(result);
    }
    anyhow::ensure!(
        raw_damage > 0.0,
        "accepted combat receipt has no raw damage"
    );
    let amount = ordinary_armor_wear(raw_damage)?;
    for (output, source) in [3, 2, 1, 0].into_iter().enumerate() {
        let armor = &snapshot.armor[source];
        if !armor.damage_on_hurt {
            continue;
        }
        let Some(postimage) = plan_wear(current, &armor.witness, amount)? else {
            continue;
        };
        anyhow::ensure!(
            result
                .iter()
                .all(|earlier| earlier.location() != postimage.location()),
            "overlapping combat armor wear witnesses"
        );
        result.postimages[output] = Some(postimage);
    }
    Ok(result)
}

fn ordinary_armor_wear(raw_damage: f32) -> anyhow::Result<u32> {
    let amount = (raw_damage / 4.0).max(1.0) as i32;
    u32::try_from(amount).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use ferrumc_core::GameMode;
    use ferrumc_items::{
        ComponentPatch, ComponentTypeId, ComponentValue, ItemId, ItemStack, OpaqueComponent,
    };
    use ferrumc_nbt::{NbtCompound, NbtTag};

    use super::*;
    use crate::inventory::SLOT_COUNT;
    use crate::mining_wear::WearLocation;

    // Actual guarded JVM probe includes all true and false static equippables.
    const JVM_EQUIPPABLE: &[(&str, bool)] = &[
        ("minecraft:black_carpet", true),
        ("minecraft:black_harness", true),
        ("minecraft:blue_carpet", true),
        ("minecraft:blue_harness", true),
        ("minecraft:brown_carpet", true),
        ("minecraft:brown_harness", true),
        ("minecraft:carved_pumpkin", true),
        ("minecraft:chainmail_boots", true),
        ("minecraft:chainmail_chestplate", true),
        ("minecraft:chainmail_helmet", true),
        ("minecraft:chainmail_leggings", true),
        ("minecraft:creeper_head", true),
        ("minecraft:cyan_carpet", true),
        ("minecraft:cyan_harness", true),
        ("minecraft:diamond_boots", true),
        ("minecraft:diamond_chestplate", true),
        ("minecraft:diamond_helmet", true),
        ("minecraft:diamond_horse_armor", false),
        ("minecraft:diamond_leggings", true),
        ("minecraft:dragon_head", true),
        ("minecraft:elytra", false),
        ("minecraft:golden_boots", true),
        ("minecraft:golden_chestplate", true),
        ("minecraft:golden_helmet", true),
        ("minecraft:golden_horse_armor", false),
        ("minecraft:golden_leggings", true),
        ("minecraft:gray_carpet", true),
        ("minecraft:gray_harness", true),
        ("minecraft:green_carpet", true),
        ("minecraft:green_harness", true),
        ("minecraft:iron_boots", true),
        ("minecraft:iron_chestplate", true),
        ("minecraft:iron_helmet", true),
        ("minecraft:iron_horse_armor", false),
        ("minecraft:iron_leggings", true),
        ("minecraft:leather_boots", true),
        ("minecraft:leather_chestplate", true),
        ("minecraft:leather_helmet", true),
        ("minecraft:leather_horse_armor", false),
        ("minecraft:leather_leggings", true),
        ("minecraft:light_blue_carpet", true),
        ("minecraft:light_blue_harness", true),
        ("minecraft:light_gray_carpet", true),
        ("minecraft:light_gray_harness", true),
        ("minecraft:lime_carpet", true),
        ("minecraft:lime_harness", true),
        ("minecraft:magenta_carpet", true),
        ("minecraft:magenta_harness", true),
        ("minecraft:netherite_boots", true),
        ("minecraft:netherite_chestplate", true),
        ("minecraft:netherite_helmet", true),
        ("minecraft:netherite_leggings", true),
        ("minecraft:orange_carpet", true),
        ("minecraft:orange_harness", true),
        ("minecraft:piglin_head", true),
        ("minecraft:pink_carpet", true),
        ("minecraft:pink_harness", true),
        ("minecraft:player_head", true),
        ("minecraft:purple_carpet", true),
        ("minecraft:purple_harness", true),
        ("minecraft:red_carpet", true),
        ("minecraft:red_harness", true),
        ("minecraft:saddle", true),
        ("minecraft:shield", true),
        ("minecraft:skeleton_skull", true),
        ("minecraft:turtle_helmet", true),
        ("minecraft:white_carpet", true),
        ("minecraft:white_harness", true),
        ("minecraft:wither_skeleton_skull", true),
        ("minecraft:wolf_armor", true),
        ("minecraft:yellow_carpet", true),
        ("minecraft:yellow_harness", true),
        ("minecraft:zombie_head", true),
    ];

    fn stack(name: &str, added: Vec<ComponentValue>, removed: &[i32]) -> ItemStack {
        ItemStack::new(
            ItemId::from_name(name).expect("pinned item"),
            NonZeroU8::MIN,
            ComponentPatch::new(
                added,
                removed.iter().copied().map(ComponentTypeId::new).collect(),
            ),
        )
    }

    fn inventory(mode: GameMode) -> PlayerInventory {
        PlayerInventory::from_persisted(
            mode,
            0,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        )
    }

    fn iron_equipment() -> PlayerInventory {
        let mut inventory = inventory(GameMode::Survival);
        for (slot, name) in [
            (5, "iron_helmet"),
            (6, "iron_chestplate"),
            (7, "iron_leggings"),
            (8, "iron_boots"),
            (36, "iron_sword"),
        ] {
            inventory.set_creative_slot(slot, stack(name, vec![], &[]));
        }
        inventory
    }

    fn damage(plan: &MiningWearPlan) -> i32 {
        durability(plan.stack_after())
            .expect("valid durability")
            .expect("damageable")
            .damage()
    }

    fn inventory_bytes(inventory: &PlayerInventory) -> Vec<Vec<u8>> {
        (0..SLOT_COUNT)
            .map(|index| inventory.slot(index).expect("slot"))
            .chain(std::iter::once(inventory.carried()))
            .chain(inventory.table_inputs())
            .map(|stack| encode_persisted_stack(stack).expect("current FCIS"))
            .collect()
    }

    #[test]
    fn capture_freezes_exact_selected_hand_profile_and_five_bounded_records() {
        let mut inventory = iron_equipment();
        inventory.set_creative_slot(
            40,
            stack("diamond_axe", vec![ComponentValue::Damage(3)], &[]),
        );
        inventory.set_selected(4);
        let before = inventory_bytes(&inventory);
        let snapshot = CombatWearSnapshot::capture(&inventory).expect("capture");
        assert_eq!(snapshot.profile().attack_damage(), 9.0);
        assert_eq!(snapshot.profile().attack_wear(), 2);
        assert_eq!(snapshot.hand_witness().slot, 40);
        assert_eq!(
            snapshot.hand_witness().bytes,
            encode_persisted_stack(inventory.held()).expect("hand FCIS")
        );
        assert_eq!(
            snapshot.encoded_bytes(),
            snapshot.hand.bytes.len()
                + snapshot
                    .armor
                    .iter()
                    .map(|witness| witness.witness.bytes.len())
                    .sum::<usize>()
        );
        assert!(snapshot.encoded_bytes() <= MAX_SNAPSHOT_BYTES);
        inventory.set_selected(0);
        assert_eq!(snapshot.profile().attack_damage(), 9.0);
        assert_eq!(snapshot.hand_witness().slot, 40);
        assert_eq!(inventory_bytes(&inventory), before);
    }

    #[test]
    fn weapon_requires_acceptance_and_exact_moved_beforeimage() {
        let mut current = iron_equipment();
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        assert!(plan_weapon_wear(&snapshot, &current, false)
            .expect("rejected")
            .is_none());
        let original = current.held().clone();
        current.set_creative_slot(36, ItemStack::empty());
        current.set_carried(original);
        let before = inventory_bytes(&current);
        let plan = plan_weapon_wear(&snapshot, &current, true)
            .expect("accepted")
            .expect("cursor");
        assert_eq!(plan.location(), WearLocation::Cursor);
        assert_eq!(damage(&plan), 1);
        assert_eq!(inventory_bytes(&current), before);
        current.set_creative_slot(9, current.carried().clone());
        assert!(plan_weapon_wear(&snapshot, &current, true)
            .expect("ambiguity")
            .is_none());
    }

    #[test]
    fn armor_uses_incremental_pre_armor_amount_and_original_equipment_order() {
        let current = iron_equipment();
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        let before = inventory_bytes(&current);
        let plan = plan_armor_wear(&snapshot, &current, true, 8.0).expect("armor plan");
        assert_eq!(
            plan.iter()
                .map(MiningWearPlan::location)
                .collect::<Vec<_>>(),
            vec![
                WearLocation::Slot(8),
                WearLocation::Slot(7),
                WearLocation::Slot(6),
                WearLocation::Slot(5)
            ]
        );
        assert!(plan.iter().all(|postimage| damage(postimage) == 2));
        let protected_increment =
            plan_armor_wear(&snapshot, &current, true, 3.0).expect("protected increment");
        assert!(protected_increment
            .iter()
            .all(|postimage| damage(postimage) == 1));
        assert_eq!(plan.into_postimages().count(), 4);
        assert_eq!(inventory_bytes(&current), before);
    }

    #[test]
    fn positive_fractional_and_large_armor_amounts_match_java_cast_boundaries() {
        for (raw, expected) in [
            (f32::MIN_POSITIVE, 1),
            (0.1, 1),
            (3.999_999, 1),
            (4.0, 1),
            (7.999_999, 1),
            (8.0, 2),
            (15.999_999, 3),
            (16.0, 4),
            (f32::MAX, i32::MAX as u32),
        ] {
            assert_eq!(ordinary_armor_wear(raw).expect("finite positive"), expected);
        }
    }

    #[test]
    fn rejected_invalid_and_zero_accepted_receipts_produce_no_postimages() {
        let current = iron_equipment();
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        for raw in [0.0, -0.0, 8.0] {
            assert_eq!(
                plan_armor_wear(&snapshot, &current, false, raw)
                    .expect("rejected")
                    .iter()
                    .count(),
                0
            );
        }
        for raw in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for accepted in [false, true] {
                assert!(plan_armor_wear(&snapshot, &current, accepted, raw).is_err());
            }
        }
        assert!(plan_armor_wear(&snapshot, &current, true, 0.0).is_err());
    }

    #[test]
    fn each_actual_equippable_default_has_its_genuine_damage_on_hurt_rule() {
        assert_eq!(JVM_EQUIPPABLE.len(), 73);
        assert_eq!(DAMAGE_ON_HURT.len(), 68);
        assert!(DAMAGE_ON_HURT.windows(2).all(|pair| pair[0] < pair[1]));
        for &(name, eligible) in JVM_EQUIPPABLE {
            let mut current = inventory(GameMode::Survival);
            current.set_creative_slot(
                8,
                stack(
                    name,
                    vec![ComponentValue::MaxDamage(50), ComponentValue::Damage(0)],
                    &[],
                ),
            );
            let snapshot = CombatWearSnapshot::capture(&current).expect("scalar custom durability");
            let plan =
                plan_armor_wear(&snapshot, &current, true, 8.0).expect("ordinary player source");
            assert_eq!(plan.iter().count(), usize::from(eligible), "{name}");
            if eligible {
                assert_eq!(damage(plan.iter().next().expect("wear")), 2, "{name}");
            }
        }
    }

    #[test]
    fn mismatched_static_slot_still_wears_without_contributing_profile_armor() {
        for name in ["diamond_chestplate", "shield", "wolf_armor"] {
            let mut current = inventory(GameMode::Survival);
            current.set_creative_slot(8, stack(name, vec![], &[]));
            let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
            assert_eq!(snapshot.profile().armor(), 0.0);
            let plan = plan_armor_wear(&snapshot, &current, true, 8.0).expect("mismatched wear");
            assert_eq!(plan.iter().count(), 1);
            assert_eq!(damage(plan.iter().next().expect("wear")), 2);
        }
    }

    #[test]
    fn armor_may_move_to_cursor_and_table_before_receipt_without_losing_patch_data() {
        let mut name = NbtCompound::new();
        name.push("text", NbtTag::String("named helmet".into()));
        let original = stack(
            "iron_helmet",
            vec![
                ComponentValue::CustomName(NbtTag::Compound(name)),
                ComponentValue::Damage(4),
            ],
            &[13],
        );
        let mut current = inventory(GameMode::Survival);
        current.set_creative_slot(5, original.clone());
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        current.set_creative_slot(5, ItemStack::empty());
        current.set_carried(original.clone());
        let plan = plan_armor_wear(&snapshot, &current, true, 8.0).expect("cursor plan");
        let cursor = plan.iter().next().expect("cursor");
        assert_eq!(cursor.location(), WearLocation::Cursor);
        assert_eq!(damage(cursor), 6);
        assert_eq!(
            cursor.stack_after().components().added()[0],
            original.components().added()[0]
        );
        assert_eq!(
            cursor.stack_after().components().removed(),
            original.components().removed()
        );
        current.set_carried(ItemStack::empty());
        let mut table = std::array::from_fn(|_| ItemStack::empty());
        table[8] = original;
        current.restore_table_inputs(table);
        let plan = plan_armor_wear(&snapshot, &current, true, 8.0).expect("table plan");
        assert_eq!(
            plan.iter().next().expect("table").location(),
            WearLocation::TableInput(8)
        );
    }

    #[test]
    fn overlapping_frozen_armor_witnesses_refuse_every_postimage_before_mutation() {
        let original = stack("iron_helmet", vec![], &[]);
        let mut current = inventory(GameMode::Survival);
        current.set_creative_slot(5, original.clone());
        current.set_creative_slot(6, original.clone());
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        current.set_creative_slot(5, ItemStack::empty());
        current.set_creative_slot(6, ItemStack::empty());
        current.set_carried(original);
        let before = inventory_bytes(&current);
        assert!(plan_armor_wear(&snapshot, &current, true, 8.0).is_err());
        assert_eq!(inventory_bytes(&current), before);
    }

    #[test]
    fn static_and_removed_equippable_damageability_and_unbreakable_rules_suppress_wear() {
        for original in [
            stack("elytra", vec![], &[]),
            stack("iron_helmet", vec![], &[28]),
            stack("iron_helmet", vec![], &[3]),
            stack("iron_helmet", vec![], &[2]),
            stack("iron_helmet", vec![ComponentValue::Unbreakable], &[]),
            stack("iron_pickaxe", vec![], &[]),
        ] {
            let mut current = inventory(GameMode::Survival);
            current.set_creative_slot(5, original);
            let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
            assert_eq!(
                plan_armor_wear(&snapshot, &current, true, 8.0)
                    .expect("no armor wear")
                    .iter()
                    .count(),
                0
            );
        }
    }

    #[test]
    fn current_creative_policy_returns_unchanged_postimages_and_captured_creative_weapon_has_no_cost(
    ) {
        let mut current = iron_equipment();
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        current.set_game_mode(GameMode::Creative);
        let weapon = plan_weapon_wear(&snapshot, &current, true)
            .expect("creative weapon")
            .expect("located");
        assert_eq!(damage(&weapon), 0);
        let armor = plan_armor_wear(&snapshot, &current, true, 100.0).expect("creative armor");
        assert!(armor
            .iter()
            .all(|postimage| damage(postimage) == 0 && !postimage.broken()));
        let creative = CombatWearSnapshot::capture(&current).expect("creative capture");
        current.set_game_mode(GameMode::Survival);
        assert!(plan_weapon_wear(&creative, &current, true)
            .expect("frozen zero weapon cost")
            .is_none());
    }

    #[test]
    fn armor_break_and_missing_or_ambiguous_witnesses_never_retarget_other_items() {
        let mut current = iron_equipment();
        current.set_creative_slot(
            5,
            stack("iron_helmet", vec![ComponentValue::Damage(164)], &[]),
        );
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        let plan = plan_armor_wear(&snapshot, &current, true, 8.0).expect("break plan");
        let helmet = plan
            .iter()
            .find(|plan| plan.location() == WearLocation::Slot(5))
            .expect("helmet");
        assert!(helmet.broken());
        assert_eq!(helmet.stack_after(), &ItemStack::empty());
        let original = current.slot(5).expect("helmet").clone();
        current.set_creative_slot(5, ItemStack::empty());
        assert_eq!(
            plan_armor_wear(&snapshot, &current, true, 8.0)
                .expect("missing helmet")
                .iter()
                .count(),
            3
        );
        current.set_carried(original.clone());
        current.set_creative_slot(9, original);
        assert_eq!(
            plan_armor_wear(&snapshot, &current, true, 8.0)
                .expect("ambiguous helmet")
                .iter()
                .count(),
            3
        );
    }

    #[test]
    fn semantic_overrides_and_bad_durability_refuse_capture_without_rewriting_source() {
        for (id, raw) in [
            (10, vec![0]),
            (13, vec![0]),
            (24, vec![0]),
            (28, vec![0]),
            (2, vec![0]),
        ] {
            let original = stack(
                "iron_helmet",
                vec![ComponentValue::Opaque(
                    OpaqueComponent::new(id, raw).expect("trusted opaque"),
                )],
                &[],
            );
            let mut current = inventory(GameMode::Survival);
            // Corrupt restored data bypasses the ordinary writer admission.
            *current.slot_mut(5).expect("fixture helmet") = original.clone();
            assert!(
                CombatWearSnapshot::capture(&current).is_err(),
                "component {id}"
            );
            assert_eq!(current.slot(5), Some(&original));
        }
        let mut current = inventory(GameMode::Survival);
        current.set_creative_slot(5, stack("netherite_helmet", vec![], &[24]));
        let snapshot = CombatWearSnapshot::capture(&current).expect("known resistance removal");
        assert_eq!(
            plan_armor_wear(&snapshot, &current, true, 8.0)
                .expect("ordinary source")
                .iter()
                .count(),
            1
        );
    }

    #[test]
    fn later_inventory_validation_error_discards_earlier_private_postimages() {
        let mut current = iron_equipment();
        let snapshot = CombatWearSnapshot::capture(&current).expect("capture");
        // Earlier feet/legs/chest witnesses still match; the final head witness
        // becomes an invalid FCIS. No caller can receive a partial armor result.
        let mut corrupted = snapshot;
        corrupted.armor[0].witness.bytes.push(0);
        let before = inventory_bytes(&current);
        let state = current.state_id();
        assert!(plan_armor_wear(&corrupted, &current, true, 8.0).is_err());
        assert_eq!(inventory_bytes(&current), before);
        assert_eq!(current.state_id(), state);
        // Deliberately invalid restored FCIS must reach the planner unchanged.
        *current.slot_mut(9).expect("fixture slot") = stack(
            "iron_pickaxe",
            vec![ComponentValue::Damage(0), ComponentValue::Damage(1)],
            &[],
        );
        assert!(plan_weapon_wear(&corrupted, &current, true).is_err());
    }
}
