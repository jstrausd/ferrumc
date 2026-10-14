//! Bounded immutable FCIS witnesses for delayed owner-committed tool wear.
use crate::inventory::{PlayerInventory, HOTBAR_START};
use std::collections::BTreeMap;

const MAX_WITNESSES: usize = 64;
const MAX_BYTES: usize = 4 * 1024 * 1024;

pub(crate) struct ToolWitness {
    pub(crate) slot: usize,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) struct MiningWitnesses {
    next: u64,
    active: Option<u64>,
    records: BTreeMap<u64, ToolWitness>,
    bytes: usize,
}

impl MiningWitnesses {
    pub(crate) fn new() -> Self {
        Self {
            next: 1,
            active: None,
            records: BTreeMap::new(),
            bytes: 0,
        }
    }

    /// Every attempt gets a nonreused token. Full admission refuses before any
    /// mining input is sent; cancellation retirement comes from the owner.
    pub(crate) fn start(&mut self, inventory: &PlayerInventory) -> anyhow::Result<u64> {
        anyhow::ensure!(
            !self.has_unsettled(),
            "previous mining tool witness is unsettled"
        );
        let bytes = ferrumc_items::encode_persisted_stack(inventory.held())?;
        anyhow::ensure!(
            self.records.len() < MAX_WITNESSES && self.bytes + bytes.len() <= MAX_BYTES,
            "too many unsettled mining tool witnesses"
        );
        let token = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("mining witness token exhausted"))?;
        self.bytes += bytes.len();
        self.records.insert(
            token,
            ToolWitness {
                slot: HOTBAR_START + usize::from(inventory.selected()),
                bytes,
            },
        );
        self.active = Some(token);
        Ok(token)
    }

    pub(crate) fn active(&self) -> u64 {
        self.active.unwrap_or(0)
    }

    /// Only a processed terminal owner receipt retires an exact beforeimage.
    /// Selection changes can clear the active token while its wear is in flight.
    pub(crate) fn has_unsettled(&self) -> bool {
        !self.records.is_empty()
    }
    pub(crate) fn changed(&mut self) {
        self.active = None;
    }

    pub(crate) fn release(&mut self, token: u64) -> Option<ToolWitness> {
        if self.active == Some(token) {
            self.active = None;
        }
        let witness = self.records.remove(&token)?;
        self.bytes -= witness.bytes.len();
        Some(witness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_core::GameMode;
    use ferrumc_items::{ComponentPatch, ItemId, ItemStack};

    #[test]
    fn changed_selection_retains_receipt_gate_and_serial_wear_follows_moved_tool() {
        for use_table in [false, true] {
            let mut inventory = PlayerInventory::from_persisted(
                GameMode::Survival,
                0,
                std::array::from_fn(|_| ItemStack::empty()),
                ItemStack::empty(),
            );
            let tool = ItemStack::try_new(
                ItemId::from_name("golden_shovel").expect("tool"),
                1,
                ComponentPatch::empty(),
            )
            .expect("one tool");
            inventory.set_creative_slot(36, tool.clone());
            let mut witnesses = MiningWitnesses::new();
            let first = witnesses.start(&inventory).expect("first witness");
            witnesses.changed();
            assert_eq!(witnesses.active(), 0);
            inventory.set_creative_slot(36, ItemStack::empty());
            if use_table {
                let mut inputs = std::array::from_fn(|_| ItemStack::empty());
                inputs[8] = tool;
                inventory.restore_table_inputs(inputs);
            } else {
                inventory.set_carried(tool);
            }
            assert!(witnesses.has_unsettled());
            assert!(witnesses.start(&inventory).is_err());
            assert!(inventory
                .damage_witness(&witnesses.release(first).expect("positive receipt"), 1)
                .expect("exact moved-tool wear"));
            let worn = if use_table {
                inventory.table_inputs()[8].clone()
            } else {
                inventory.carried().clone()
            };
            assert_eq!(
                ferrumc_items::durability(&worn)
                    .expect("valid")
                    .expect("tool")
                    .damage(),
                1
            );
            if use_table {
                inventory.restore_table_inputs(std::array::from_fn(|_| ItemStack::empty()));
            } else {
                inventory.set_carried(ItemStack::empty());
            }
            inventory.set_creative_slot(36, worn);
            let second = witnesses.start(&inventory).expect("settled retry");
            assert_ne!(first, second);
            assert!(inventory
                .damage_witness(&witnesses.release(second).expect("second receipt"), 1)
                .expect("fresh exact beforeimage"));
            assert_eq!(
                ferrumc_items::durability(inventory.held())
                    .expect("valid")
                    .expect("tool")
                    .damage(),
                2
            );
            assert!(!witnesses.has_unsettled());
        }
    }
}
