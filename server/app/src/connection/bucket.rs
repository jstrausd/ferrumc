//! Real air-use buckets with a connection-owned inventory pinned until owner ACK.
use super::{context::ConnContext, outbound::send_mandatory};
use crate::{driver::SimCommand, inventory::PlayerInventory, window::WindowState};
use ferrumc_core::PlayerId;
use ferrumc_math::Vec3;
use ferrumc_net::{CompressionState, PlayWriter};
use ferrumc_observability::SessionDebug;
use ferrumc_proto::generated::play::{
    AcknowledgeBlockChange, ClientboundPlayPacket, SetContainerContent, UseItem,
};
use tokio::sync::oneshot;

#[allow(clippy::too_many_arguments)]
pub(super) async fn use_bucket(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    windows: &mut WindowState,
    packet: &UseItem,
    position: Option<Vec3>,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<bool> {
    let slot = match packet.hand() {
        0 => crate::inventory::HOTBAR_START + usize::from(inventory.selected()),
        1 => crate::inventory::OFFHAND_SLOT,
        _ => return Ok(false),
    };
    let Some(item) = inventory
        .slot(slot)
        .and_then(ferrumc_items::ItemStack::item)
    else {
        return Ok(false);
    };
    let fill = if Some(item) == ferrumc_items::ItemId::from_name("bucket") {
        true
    } else if Some(item) == ferrumc_items::ItemId::from_name("water_bucket") {
        false
    } else {
        return Ok(false);
    };
    windows.consumption.cancel();
    let acknowledge = |writer: &mut PlayWriter, debug: &mut SessionDebug| {
        send_mandatory(
            writer,
            debug,
            compression,
            &ctx.clock,
            ClientboundPlayPacket::AcknowledgeBlockChange(AcknowledgeBlockChange::new(
                packet.sequence(),
            )),
        )
    };
    let Some(position) = position else {
        acknowledge(writer, debug)?;
        return Ok(true);
    };
    if !packet.yaw().is_finite() || !packet.pitch().is_finite() {
        acknowledge(writer, debug)?;
        return Ok(true);
    }
    let Some(prepared) = inventory.prepare_bucket(packet.hand(), fill)? else {
        acknowledge(writer, debug)?;
        return Ok(true);
    };
    let token = windows.bucket_token()?;
    let eye_height = if windows.secondary_use() {
        f64::from(1.27_f32)
    } else {
        f64::from(1.62_f32)
    };
    let (reply, result) = oneshot::channel();
    // This handler completes as one non-canceled inventory transaction. No other
    // inventory packet, pickup, or mode mutation can interleave with the await.
    ctx.commands
        .send(SimCommand::BucketUse {
            player,
            position,
            eye_height,
            yaw: packet.yaw(),
            pitch: packet.pitch(),
            sequence: packet.sequence(),
            fill,
            token,
            reply,
        })
        .await
        .map_err(|_| anyhow::anyhow!("driver closed while using bucket"))?;
    if result
        .await
        .map_err(|_| anyhow::anyhow!("bucket outcome lost"))??
        .is_some()
    {
        inventory.commit_consumption(prepared);
        ctx.commands
            .send(SimCommand::SetEquipment {
                player,
                equipment: inventory.equipment_body()?,
            })
            .await
            .map_err(|_| anyhow::anyhow!("driver closed after bucket commit"))?;
    }
    acknowledge(writer, debug)?;
    if windows.has_open() {
        super::handlers::resync_chest_window(ctx, writer, inventory, windows, debug, compression)?;
    } else {
        send_mandatory(
            writer,
            debug,
            compression,
            &ctx.clock,
            ClientboundPlayPacket::SetContainerContent(SetContainerContent::new(
                0,
                inventory.state_id(),
                inventory.container_content_payload()?,
            )),
        )?;
    }
    Ok(true)
}
