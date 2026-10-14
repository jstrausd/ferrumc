//! Complete schema-5 admission envelope, including future positive item wear.
//!
//! This is an App resource policy, not a change to the durable codec ceiling.
//! Reserving growth in every owned stack makes a delayed combat wear effect
//! safe against intervening ordinary inventory growth.

use ferrumc_items::{component::DAMAGE, ComponentValue, ItemStack};

/// Fixed bytes outside the 56 FCIS frames in a complete schema-5 record.
const SCHEMA_BYTES: usize = 41 + 56 * 4 + 21;
const STACK_COUNT: usize = 56;

/// Admits exactly the complete player image, without changing durable data.
pub(crate) fn admit<'a>(stacks: impl Iterator<Item = &'a ItemStack>) -> anyhow::Result<()> {
    let mut total = SCHEMA_BYTES;
    let mut count = 0;
    for stack in stacks {
        count += 1;
        anyhow::ensure!(count <= STACK_COUNT, "too many player budget stacks");
        total = total
            .checked_add(projected_stack_size(stack)?)
            .ok_or_else(|| anyhow::anyhow!("projected player record size overflow"))?;
        anyhow::ensure!(
            total <= ferrumc_storage::MAX_PLAYER_DATA_LEN,
            "future wear exceeds complete player save budget"
        );
    }
    anyhow::ensure!(count == STACK_COUNT, "incomplete player budget image");
    Ok(())
}

/// Reserves the largest FCIS image reachable by positive DAMAGE replacement.
/// All actual validation and effective durability rules remain authoritative.
fn projected_stack_size(stack: &ItemStack) -> anyhow::Result<usize> {
    let actual = ferrumc_items::persisted_stack_size(stack)?;
    ferrumc_items::stack_capacity(stack)?;
    let Some(_) = ferrumc_items::durability(stack)?.filter(|state| !state.unbreakable()) else {
        return Ok(actual);
    };
    let patch = stack.components();
    let damage = patch.added().iter().find(|value| value.type_id() == DAMAGE);
    let (growth, payload_growth) = match damage {
        None => {
            anyhow::ensure!(
                patch.added().len() + patch.removed().len() < ferrumc_items::MAX_COMPONENTS,
                "future DAMAGE exceeds component count"
            );
            (13, 4)
        }
        Some(ComponentValue::Damage(_)) => (0, 0),
        Some(ComponentValue::Opaque(value)) => {
            let growth = 4usize.saturating_sub(value.raw().len());
            (growth, growth)
        }
        Some(_) => anyhow::bail!("invalid effective DAMAGE representation"),
    };
    // FCIS preflight has already validated the exact header and all payloads.
    let item = stack
        .item()
        .ok_or_else(|| anyhow::anyhow!("damageable air"))?;
    let header = 12 + item.name().len() + patch.added().len() * 9 + patch.removed().len() * 4;
    let payload = actual
        .checked_sub(header)
        .ok_or_else(|| anyhow::anyhow!("invalid FCIS size accounting"))?;
    anyhow::ensure!(
        payload + payload_growth <= ferrumc_items::MAX_COMPONENTS_TOTAL_BYTES,
        "future DAMAGE exceeds component payload budget"
    );
    let projected = actual + growth;
    anyhow::ensure!(
        projected <= ferrumc_items::MAX_PERSISTED_STACK_BYTES,
        "future DAMAGE exceeds stack record budget"
    );
    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_items::{ComponentPatch, ComponentTypeId, ItemId, OpaqueComponent};

    fn tool(added: Vec<ComponentValue>, removed: Vec<ComponentTypeId>) -> ItemStack {
        ItemStack::try_new(
            ItemId::from_name("iron_sword").unwrap(),
            1,
            ComponentPatch::new(added, removed),
        )
        .unwrap()
    }

    fn opaque(id: i32, len: usize) -> ComponentValue {
        ComponentValue::Opaque(OpaqueComponent::new(id, vec![0; len]).unwrap())
    }

    #[test]
    fn budget_exact_first_damage_frame_and_opaque_conversion_growth() {
        let pristine = tool(vec![], vec![]);
        let after = ferrumc_items::plan_durability_damage(&pristine, 1, false).unwrap();
        let actual = ferrumc_items::persisted_stack_size(&pristine).unwrap();
        assert_eq!(
            ferrumc_items::persisted_stack_size(after.stack_after()).unwrap(),
            actual + 13
        );
        assert_eq!(projected_stack_size(&pristine).unwrap(), actual + 13);
        assert_eq!(
            projected_stack_size(after.stack_after()).unwrap(),
            actual + 13
        );
        for (raw, growth) in [
            (vec![0], 3),
            (vec![128, 1], 2),
            (vec![255, 255, 255, 255, 7], 0),
        ] {
            let stack = tool(
                vec![ComponentValue::Opaque(
                    OpaqueComponent::new(DAMAGE, raw).unwrap(),
                )],
                vec![],
            );
            let size = ferrumc_items::persisted_stack_size(&stack).unwrap();
            assert_eq!(projected_stack_size(&stack).unwrap(), size + growth);
            let after = ferrumc_items::plan_durability_damage(&stack, 1, false).unwrap();
            assert!(
                ferrumc_items::persisted_stack_size(after.stack_after()).unwrap() <= size + growth
            );
        }
    }

    #[test]
    fn budget_future_component_count_checks_added_and_removed() {
        let ids: Vec<_> = (6..=95)
            .filter(|id| *id != 10 && OpaqueComponent::new(*id, vec![]).is_some())
            .collect();
        let stack = tool(ids[..64].iter().map(|id| opaque(*id, 0)).collect(), vec![]);
        assert!(ferrumc_items::persisted_stack_size(&stack).is_ok());
        assert!(projected_stack_size(&stack).is_err());
        assert!(ferrumc_items::plan_durability_damage(&stack, 1, false).is_err());
        let stack = tool(
            ids[..63].iter().map(|id| opaque(*id, 0)).collect(),
            vec![ComponentTypeId::new(0)],
        );
        assert!(projected_stack_size(&stack).is_err());
        let mut components: Vec<_> = ids[..63].iter().map(|id| opaque(*id, 0)).collect();
        components.push(ComponentValue::Damage(0));
        let stack = tool(components, vec![]);
        assert_eq!(
            projected_stack_size(&stack).unwrap(),
            ferrumc_items::persisted_stack_size(&stack).unwrap()
        );
        assert!(ferrumc_items::plan_durability_damage(&stack, 1, false).is_ok());
    }

    #[test]
    fn budget_future_payload_ceiling_new_and_opaque_damage() {
        let ids: Vec<_> = (6..=38).filter(|id| *id != 10).collect();
        let added = |tail| {
            ids.iter()
                .enumerate()
                .map(|(index, id)| opaque(*id, if index == 31 { tail } else { 8192 }))
                .collect()
        };
        let stack = tool(added(8192), vec![]);
        assert!(ferrumc_items::persisted_stack_size(&stack).is_ok());
        assert!(projected_stack_size(&stack).is_err());
        assert!(ferrumc_items::plan_durability_damage(&stack, 1, false).is_err());
        let stack = tool(added(8188), vec![]);
        assert!(projected_stack_size(&stack).is_ok());
        assert!(ferrumc_items::plan_durability_damage(&stack, 1, false).is_ok());
        let mut components: Vec<_> = added(8191);
        components.push(opaque(DAMAGE, 1));
        let stack = tool(components, vec![]);
        assert!(ferrumc_items::persisted_stack_size(&stack).is_ok());
        assert!(projected_stack_size(&stack).is_err());
    }

    #[test]
    fn budget_removed_unbreakable_and_malformed_effective_damage() {
        for stack in [
            tool(vec![], vec![ComponentTypeId::new(DAMAGE)]),
            tool(vec![ComponentValue::Unbreakable], vec![]),
        ] {
            assert_eq!(
                projected_stack_size(&stack).unwrap(),
                ferrumc_items::persisted_stack_size(&stack).unwrap()
            );
        }
        let malformed = tool(vec![opaque(DAMAGE, 2)], vec![]);
        assert!(projected_stack_size(&malformed).is_err());
        let empty = ItemStack::empty();
        assert!(admit(std::iter::repeat_n(&empty, 55)).is_err());
        assert!(admit(std::iter::repeat_n(&empty, 56)).is_ok());
        assert!(admit(std::iter::repeat_n(&empty, 57)).is_err());
    }
}
