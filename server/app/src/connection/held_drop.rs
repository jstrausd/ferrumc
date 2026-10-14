//! Serial held-drop transaction. Owner uncertainty ends the connection; it
//! cannot be treated as refusal/retry after payload admission.

use super::context::ConnContext;
use crate::{driver::SimCommand, held_drop::HeldDropPlan, inventory::PlayerInventory};
use ferrumc_core::PlayerId;
use tokio::sync::{mpsc, oneshot};

pub(super) async fn drop_held(
    ctx: &ConnContext,
    player: PlayerId,
    inventory: &mut PlayerInventory,
    status: i32,
) -> anyhow::Result<bool> {
    transact(&ctx.commands, player, inventory, status).await
}

async fn transact(
    commands: &mpsc::Sender<SimCommand>,
    player: PlayerId,
    inventory: &mut PlayerInventory,
    status: i32,
) -> anyhow::Result<bool> {
    let Some(plan) = HeldDropPlan::prepare(inventory, status)? else {
        return Ok(false);
    };
    let (stack, commit) = plan.into_parts();
    let (reply, result) = oneshot::channel();
    // Ordinary handler execution is not raced against shutdown/socket input.
    // Inventory/effects/mode changes cannot interleave through this await.
    commands
        .send(SimCommand::PlayerHeldDrop {
            player,
            stack,
            reply,
        })
        .await
        .map_err(|_| anyhow::anyhow!("driver closed while dropping held item"))?;
    let accepted = result
        .await
        .map_err(|_| anyhow::anyhow!("held drop outcome lost; ownership uncertain"))?;
    if accepted {
        commit.commit(inventory)?;
    }
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::{HOTBAR_START, SLOT_COUNT};
    use ferrumc_core::GameMode;
    use ferrumc_items::{ComponentPatch, ItemId, ItemStack};

    fn inventory() -> PlayerInventory {
        let mut slots: [ItemStack; SLOT_COUNT] = std::array::from_fn(|_| ItemStack::empty());
        slots[HOTBAR_START] = ItemStack::try_new(
            ItemId::from_name("stone").unwrap(),
            7,
            ComponentPatch::empty(),
        )
        .unwrap();
        PlayerInventory::from_persisted(GameMode::Survival, 0, slots, ItemStack::empty())
    }

    #[tokio::test]
    async fn held_drop_serial_reply_success_denial_and_lost_outcome() {
        for response in [Some(true), Some(false), None] {
            let (commands, mut receiver) = mpsc::channel(1);
            let mut inventory = inventory();
            let state = inventory.state_id();
            let transaction = transact(
                &commands,
                PlayerId::offline("serial-drop"),
                &mut inventory,
                4,
            );
            let owner = async {
                let SimCommand::PlayerHeldDrop { stack, reply, .. } =
                    receiver.recv().await.unwrap()
                else {
                    panic!("wrong command")
                };
                assert_eq!(stack.count(), 1);
                if let Some(response) = response {
                    reply.send(response).unwrap();
                }
            };
            let (result, ()) = tokio::join!(transaction, owner);
            match response {
                Some(true) => {
                    assert!(result.unwrap());
                    assert_eq!(inventory.slot(HOTBAR_START).unwrap().count(), 6);
                    assert_ne!(inventory.state_id(), state);
                }
                Some(false) => {
                    assert!(!result.unwrap());
                    assert_eq!(inventory.slot(HOTBAR_START).unwrap().count(), 7);
                    assert_eq!(inventory.state_id(), state);
                }
                None => {
                    assert!(result
                        .unwrap_err()
                        .to_string()
                        .contains("ownership uncertain"));
                    assert_eq!(inventory.slot(HOTBAR_START).unwrap().count(), 7);
                    assert_eq!(inventory.state_id(), state);
                }
            }
            assert!(receiver.try_recv().is_err());
        }
    }

    #[tokio::test]
    async fn held_drop_closed_command_channel_is_error_and_preserves_inventory() {
        let (commands, receiver) = mpsc::channel(1);
        drop(receiver);
        let mut inventory = inventory();
        let state = inventory.state_id();
        assert!(transact(
            &commands,
            PlayerId::offline("closed-drop"),
            &mut inventory,
            3
        )
        .await
        .is_err());
        assert_eq!(inventory.slot(HOTBAR_START).unwrap().count(), 7);
        assert_eq!(inventory.state_id(), state);
    }
}
