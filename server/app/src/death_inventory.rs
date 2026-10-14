//! Pure whole-inventory death proposals; the exclusive owner commits only after
//! an actual whole-batch acceptance and an unchanged full beforeimage.

use ferrumc_items::{encode_persisted_stack, persisted_stack_size, stack_capacity, ItemStack};
use ferrumc_math::Vec3;
use ferrumc_sim::{EntityTransform, GroundItemDrop};
use ferrumc_storage::MAX_PLAYER_DATA_LEN;

use crate::inventory::{PlayerInventory, SLOT_COUNT};

const FRAME_COUNT: usize = SLOT_COUNT + 1 + 9;
// Exact schema-5 FCPS header + vitals suffix + 56 item-frame lengths.
const PLAYER_ENVELOPE_BYTES: usize = 41 + 21 + FRAME_COUNT * 4;
const ENCHANTMENTS: i32 = 10;

struct BeforeInventory {
    frames: Vec<u8>,
    ends: [usize; FRAME_COUNT],
}

impl BeforeInventory {
    fn matches(&self, inventory: &PlayerInventory) -> anyhow::Result<bool> {
        let mut matches = true;
        let mut start = 0;
        let mut bytes = PLAYER_ENVELOPE_BYTES;
        for (index, stack) in stacks(inventory).enumerate() {
            let stack = stack?;
            admit_stack(stack, &mut bytes)?;
            let encoded = encode_persisted_stack(stack)?;
            let end = self.ends[index];
            matches &= self.frames.get(start..end) == Some(encoded.as_slice());
            start = end;
        }
        Ok(matches)
    }
}

/// Move-owned proposed drops plus exact evidence and all-empty postimages.
/// This plan proves no death, drop admission, epoch or inventory commit.
pub(crate) struct DeathInventoryPlan {
    proposals: Vec<GroundItemDrop>,
    cleared: ClearedInventory,
}

impl DeathInventoryPlan {
    /// Prepare at the supplied actual owner death position without mutation.
    ///
    /// Preflight all 56 frames against the complete schema-5 encoded budget and
    /// effective stack capacities before cloning any proposal. Added complex
    /// enchantments refuse because `PREVENT_EQUIPMENT_DROP` cannot be resolved.
    /// Slot0 is a virtual result: compare and clear it, but never grant its item.
    /// Real slots1..45, cursor and nine table inputs each propose their complete
    /// nonempty stack with pickup delay40 and no target/thrower attribution.
    /// Caller owns death/keep-inventory policy, position authority and settlement.
    pub(crate) fn prepare(inventory: &PlayerInventory, position: Vec3) -> anyhow::Result<Self> {
        EntityTransform::try_new(position, 0.0, 0.0)?;
        let mut bytes = PLAYER_ENVELOPE_BYTES;
        for stack in stacks(inventory) {
            let stack = stack?;
            admit_stack(stack, &mut bytes)?;
            anyhow::ensure!(
                !stack
                    .components()
                    .added()
                    .iter()
                    .any(|value| value.type_id() == ENCHANTMENTS),
                "unsupported death enchantment override"
            );
        }
        let mut before = BeforeInventory {
            frames: Vec::with_capacity(bytes - PLAYER_ENVELOPE_BYTES),
            ends: [0; FRAME_COUNT],
        };
        let mut proposals = Vec::with_capacity(FRAME_COUNT - 1);
        for (index, stack) in stacks(inventory).enumerate() {
            let stack = stack?;
            before
                .frames
                .extend_from_slice(&encode_persisted_stack(stack)?);
            before.ends[index] = before.frames.len();
            if index != 0 && stack.item().is_some() {
                proposals.push(GroundItemDrop::new(position, stack.clone(), 40, None, None));
            }
        }
        Ok(Self {
            proposals,
            cleared: ClearedInventory {
                before,
                slots: std::array::from_fn(|_| ItemStack::empty()),
                carried: ItemStack::empty(),
                table_inputs: std::array::from_fn(|_| ItemStack::empty()),
            },
        })
    }

    /// Compare every exact FCIS frame, validating the whole current inventory.
    /// Selection, mode and state ID remain caller-owned; this compares items only.
    pub(crate) fn matches(&self, inventory: &PlayerInventory) -> anyhow::Result<bool> {
        self.cleared.matches(inventory)
    }

    /// Actual retained beforeimage payload bytes, excluding object/proposal heap.
    pub(crate) fn encoded_bytes(&self) -> usize {
        self.cleared.encoded_bytes()
    }

    /// Move proposals to the owner while retaining evidence with the postimages.
    /// A lost/refused/canceled response must never authorize the cleared values.
    pub(crate) fn into_parts(self) -> (Vec<GroundItemDrop>, ClearedInventory) {
        (self.proposals, self.cleared)
    }
}

/// Immutable whole beforeimage and full empty postimages retained during ACK.
/// The caller must pin its inventory until definitive whole-batch settlement.
pub(crate) struct ClearedInventory {
    before: BeforeInventory,
    slots: [ItemStack; SLOT_COUNT],
    carried: ItemStack,
    table_inputs: [ItemStack; 9],
}

impl ClearedInventory {
    /// Check the original 46 slots, carried stack and nine table inputs exactly.
    /// All current frames validate even if an earlier frame already mismatched.
    pub(crate) fn matches(&self, inventory: &PlayerInventory) -> anyhow::Result<bool> {
        self.before.matches(inventory)
    }

    /// Actual retained FCIS payload bytes, excluding metadata and allocator cost.
    pub(crate) fn encoded_bytes(&self) -> usize {
        self.before.frames.len()
    }

    /// Move all empty postimages after accepted settlement and exact comparison.
    /// Does not mutate selection/mode/state ID or perform any commit/publication.
    pub(crate) fn into_parts(self) -> ([ItemStack; SLOT_COUNT], ItemStack, [ItemStack; 9]) {
        (self.slots, self.carried, self.table_inputs)
    }
}

fn stacks(inventory: &PlayerInventory) -> impl Iterator<Item = anyhow::Result<&ItemStack>> {
    (0..SLOT_COUNT)
        .map(|index| {
            inventory
                .slot(index)
                .ok_or_else(|| anyhow::anyhow!("missing death inventory slot"))
        })
        .chain(std::iter::once(Ok(inventory.carried())))
        .chain(inventory.table_inputs().iter().map(Ok))
}

fn admit_stack(stack: &ItemStack, bytes: &mut usize) -> anyhow::Result<()> {
    *bytes = bytes
        .checked_add(persisted_stack_size(stack)?)
        .ok_or_else(|| anyhow::anyhow!("death inventory size overflow"))?;
    anyhow::ensure!(
        *bytes <= MAX_PLAYER_DATA_LEN,
        "death inventory budget exceeded"
    );
    stack_capacity(stack)?;
    anyhow::ensure!(
        !stack.item().is_some_and(ferrumc_items::ItemId::is_air),
        "present air in death inventory"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use ferrumc_core::GameMode;
    use ferrumc_items::{
        component, ComponentPatch, ComponentTypeId, ComponentValue, ItemId, OpaqueComponent,
    };
    use ferrumc_nbt::{NbtCompound, NbtTag};

    use super::*;

    fn item(count: u8, patch: ComponentPatch) -> ItemStack {
        ItemStack::new(
            ItemId::from_name("stone").expect("item"),
            NonZeroU8::new(count).expect("count"),
            patch,
        )
    }

    fn data(index: i32, bits: u32) -> NbtTag {
        let mut compound = NbtCompound::new();
        compound.push("nan", NbtTag::Float(f32::from_bits(bits)));
        compound.push("index", NbtTag::Int(index));
        compound.push("zero", NbtTag::Double(-0.0));
        NbtTag::Compound(compound)
    }

    fn rich(index: usize) -> ItemStack {
        let data = data(i32::try_from(index).expect("bounded index"), 0x7fc0_0001);
        item(
            (index % 64 + 1) as u8,
            ComponentPatch::new(
                vec![
                    ComponentValue::Opaque(
                        OpaqueComponent::new(42, vec![index as u8]).expect("opaque"),
                    ),
                    ComponentValue::CustomData(data),
                ],
                vec![
                    ComponentTypeId::new(component::DAMAGE),
                    ComponentTypeId::new(10),
                ],
            ),
        )
    }

    fn empty_inventory(mode: GameMode) -> PlayerInventory {
        PlayerInventory::from_persisted(
            mode,
            7,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        )
    }

    fn full_inventory(mode: GameMode) -> PlayerInventory {
        let mut inventory =
            PlayerInventory::from_persisted(mode, 7, std::array::from_fn(rich), rich(SLOT_COUNT));
        inventory.restore_table_inputs(std::array::from_fn(|i| rich(SLOT_COUNT + 1 + i)));
        inventory
    }

    fn snapshot(inventory: &PlayerInventory) -> Vec<Vec<u8>> {
        stacks(inventory)
            .map(|stack| encode_persisted_stack(stack.expect("stack")).expect("FCIS"))
            .collect()
    }

    fn replace(inventory: &mut PlayerInventory, index: usize, value: ItemStack) {
        match index.cmp(&SLOT_COUNT) {
            std::cmp::Ordering::Less => *inventory.slot_mut(index).expect("slot") = value,
            std::cmp::Ordering::Equal => inventory.set_carried(value),
            std::cmp::Ordering::Greater => {
                let mut table = inventory.table_inputs().clone();
                table[index - SLOT_COUNT - 1] = value;
                inventory.restore_table_inputs(table);
            }
        }
    }

    #[test]
    fn all_owned_locations_preserve_counts_components_and_exact_position() {
        let inventory = full_inventory(GameMode::Survival);
        let before = snapshot(&inventory);
        let position = Vec3::new(-12.25, 64.75, 31.5);
        let plan = DeathInventoryPlan::prepare(&inventory, position).expect("plan");
        assert!(plan.matches(&inventory).expect("compare"));
        assert_eq!(
            plan.encoded_bytes(),
            before.iter().map(Vec::len).sum::<usize>()
        );
        let (drops, cleared) = plan.into_parts();
        assert_eq!(drops.len(), FRAME_COUNT - 1);
        for (drop, expected) in drops.into_iter().zip(&before[1..]) {
            let (at, stack, delay, target, thrower) = drop.into_parts();
            assert_eq!(
                (at.x.to_bits(), at.y.to_bits(), at.z.to_bits()),
                (
                    position.x.to_bits(),
                    position.y.to_bits(),
                    position.z.to_bits()
                )
            );
            assert_eq!(encode_persisted_stack(&stack).expect("FCIS"), *expected);
            assert_eq!(delay, 40);
            assert!(target.is_none() && thrower.is_none());
        }
        assert!(cleared.matches(&inventory).expect("retained compare"));
        let (slots, carried, table) = cleared.into_parts();
        assert!(slots
            .iter()
            .chain(std::iter::once(&carried))
            .chain(&table)
            .all(|stack| stack.item().is_none()
                && stack.count() == 0
                && stack.components().is_empty()));
        assert_eq!(snapshot(&inventory), before);
        assert_eq!(inventory.selected(), 7);
        assert_eq!(inventory.game_mode(), GameMode::Survival);
        assert_eq!(inventory.state_id(), 1);
    }

    #[test]
    fn sparse_inventory_excludes_virtual_result_but_keeps_real_crafting_inputs() {
        let mut inventory = empty_inventory(GameMode::Survival);
        for index in [0, 1, 4, 5, 8, 9, 35, 36, 44, 45, 46, 47, 55] {
            replace(&mut inventory, index, rich(index));
        }
        let before = snapshot(&inventory);
        let plan = DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).expect("plan");
        let (drops, cleared) = plan.into_parts();
        let expected: Vec<_> = before
            .iter()
            .skip(1)
            .filter(|bytes| bytes.len() > 7)
            .collect();
        assert_eq!(drops.len(), expected.len());
        for (drop, expected) in drops.into_iter().zip(expected) {
            assert_eq!(
                encode_persisted_stack(&drop.into_parts().1).expect("FCIS"),
                *expected
            );
        }
        assert!(cleared.matches(&inventory).expect("matches"));
        assert_eq!(snapshot(&inventory), before);
    }

    #[test]
    fn virtual_result_alone_grants_nothing_and_all_empty_is_a_valid_zero_batch() {
        for present in [false, true] {
            let mut inventory = empty_inventory(GameMode::Survival);
            if present {
                replace(&mut inventory, 0, rich(0));
            }
            let plan = DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).expect("plan");
            let (drops, cleared) = plan.into_parts();
            assert!(drops.is_empty());
            assert!(cleared.matches(&inventory).expect("matches"));
            let (slots, carried, table) = cleared.into_parts();
            assert!(slots
                .iter()
                .chain(std::iter::once(&carried))
                .chain(&table)
                .all(|stack| stack.item().is_none()));
        }
    }

    #[test]
    fn every_beforeimage_change_refuses_both_before_and_after_proposals_move() {
        for index in 0..FRAME_COUNT {
            let mut inventory = full_inventory(GameMode::Survival);
            let plan = DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).expect("plan");
            replace(&mut inventory, index, ItemStack::empty());
            assert!(!plan.matches(&inventory).expect("changed"));
            let (_drops, cleared) = plan.into_parts();
            assert!(!cleared.matches(&inventory).expect("retained changed"));
        }
    }

    #[test]
    fn count_patch_order_and_float_bits_are_exact_compare_evidence() {
        for change in 0..3 {
            let mut inventory = empty_inventory(GameMode::Survival);
            replace(&mut inventory, 45, rich(45));
            let plan = DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).expect("plan");
            let old = inventory.slot(45).expect("slot");
            let mut added = old.components().added().to_vec();
            let count = if change == 0 {
                old.count() - 1
            } else {
                old.count()
            };
            if change == 1 {
                added.reverse();
            }
            if change == 2 {
                added[1] = ComponentValue::CustomData(data(45, 0x7fc0_0002));
            }
            let patch = ComponentPatch::new(added, old.components().removed().to_vec());
            replace(&mut inventory, 45, item(count, patch));
            assert!(!plan.matches(&inventory).expect("exact compare"));
        }
    }

    #[test]
    fn modes_selection_and_state_ids_do_not_change_item_evidence_or_postimages() {
        for mode in [
            GameMode::Survival,
            GameMode::Creative,
            GameMode::Adventure,
            GameMode::Spectator,
        ] {
            let mut inventory = full_inventory(mode);
            let before = snapshot(&inventory);
            let plan = DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).expect("plan");
            assert_eq!(inventory.game_mode(), mode);
            assert!(inventory.set_selected(2));
            inventory.set_game_mode(GameMode::Creative);
            inventory.bump_state_id();
            assert!(plan.matches(&inventory).expect("item evidence"));
            let (drops, cleared) = plan.into_parts();
            assert_eq!(drops.len(), 55);
            assert!(cleared.matches(&inventory).expect("retained"));
            assert_eq!(snapshot(&inventory), before);
        }
    }

    #[test]
    fn bad_positions_refuse_without_mutation_and_valid_bits_are_preserved() {
        let inventory = full_inventory(GameMode::Survival);
        let before = snapshot(&inventory);
        for position in [
            Vec3::new(f64::NAN, 0.0, 0.0),
            Vec3::new(0.0, f64::INFINITY, 0.0),
            Vec3::new(0.0, 0.0, f64::NEG_INFINITY),
            Vec3::new(30_000_001.0, 0.0, 0.0),
        ] {
            assert!(DeathInventoryPlan::prepare(&inventory, position).is_err());
            assert_eq!(snapshot(&inventory), before);
        }
        let position = Vec3::new(-0.0, -1.25, 0.0);
        let (drops, _) = DeathInventoryPlan::prepare(&inventory, position)
            .expect("valid")
            .into_parts();
        assert_eq!(
            drops
                .into_iter()
                .next()
                .expect("drop")
                .into_parts()
                .0
                .x
                .to_bits(),
            (-0.0_f64).to_bits()
        );
    }

    #[test]
    fn unsupported_enchantments_present_air_and_capacity_refuse_without_mutation() {
        for stack in [
            item(
                1,
                ComponentPatch::new(
                    vec![ComponentValue::Opaque(
                        OpaqueComponent::new(10, vec![0]).expect("enchantments"),
                    )],
                    vec![],
                ),
            ),
            item(
                2,
                ComponentPatch::new(vec![ComponentValue::MaxStackSize(1)], vec![]),
            ),
            ItemStack::new(
                ItemId::from_name("air").expect("air"),
                NonZeroU8::MIN,
                ComponentPatch::empty(),
            ),
        ] {
            let mut inventory = empty_inventory(GameMode::Survival);
            replace(&mut inventory, 55, stack);
            let before = snapshot(&inventory);
            assert!(DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).is_err());
            assert_eq!(snapshot(&inventory), before);
        }
    }

    #[test]
    fn late_invalid_current_stack_is_reported_even_after_an_early_mismatch() {
        let mut inventory = full_inventory(GameMode::Survival);
        let plan = DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).expect("plan");
        replace(&mut inventory, 0, ItemStack::empty());
        replace(
            &mut inventory,
            55,
            item(
                2,
                ComponentPatch::new(vec![ComponentValue::MaxStackSize(1)], vec![]),
            ),
        );
        let before = snapshot(&inventory);
        assert!(plan.matches(&inventory).is_err());
        let (_, cleared) = plan.into_parts();
        assert!(cleared.matches(&inventory).is_err());
        assert_eq!(snapshot(&inventory), before);
    }

    fn padded(last: usize) -> ItemStack {
        let ids: Vec<_> = (0..=95)
            .filter(|id| {
                !ComponentTypeId::new(*id).is_dangerous()
                    && ![
                        component::MAX_STACK_SIZE,
                        component::MAX_DAMAGE,
                        component::DAMAGE,
                        component::UNBREAKABLE,
                        component::CUSTOM_NAME,
                        component::CUSTOM_DATA,
                        10,
                    ]
                    .contains(id)
            })
            .take(32)
            .collect();
        assert_eq!(ids.len(), 32);
        item(
            1,
            ComponentPatch::new(
                ids.into_iter()
                    .enumerate()
                    .map(|(index, id)| {
                        ComponentValue::Opaque(
                            OpaqueComponent::new(
                                id,
                                vec![3; if index == 31 { last } else { 8192 }],
                            )
                            .expect("padding"),
                        )
                    })
                    .collect(),
                vec![],
            ),
        )
    }

    #[test]
    fn exact_schema_five_budget_admits_boundary_and_refuses_one_more_byte() {
        let mut inventory = empty_inventory(GameMode::Survival);
        for index in 1..=4 {
            replace(&mut inventory, index, padded(8192));
        }
        let total =
            PLAYER_ENVELOPE_BYTES + snapshot(&inventory).iter().map(Vec::len).sum::<usize>();
        let excess = total - MAX_PLAYER_DATA_LEN;
        assert!((1..8192).contains(&excess));
        replace(&mut inventory, 4, padded(8192 - excess));
        let before = snapshot(&inventory);
        let plan = DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).expect("exact cap");
        assert_eq!(
            plan.encoded_bytes() + PLAYER_ENVELOPE_BYTES,
            MAX_PLAYER_DATA_LEN
        );
        assert!(plan.matches(&inventory).expect("boundary compare"));
        assert_eq!(snapshot(&inventory), before);
        replace(&mut inventory, 4, padded(8193 - excess));
        let too_big = snapshot(&inventory);
        assert!(DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).is_err());
        assert_eq!(snapshot(&inventory), too_big);
    }

    #[test]
    fn malformed_component_patch_refuses_before_any_proposals_escape() {
        let mut inventory = full_inventory(GameMode::Survival);
        let invalid = item(
            1,
            ComponentPatch::new(
                vec![ComponentValue::Damage(0), ComponentValue::Damage(1)],
                vec![],
            ),
        );
        replace(&mut inventory, 55, invalid.clone());
        assert!(DeathInventoryPlan::prepare(&inventory, Vec3::ZERO).is_err());
        assert_eq!(inventory.table_inputs()[8], invalid);
        assert_eq!(inventory.selected(), 7);
        assert_eq!(inventory.state_id(), 1);
    }
}
