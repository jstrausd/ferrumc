//! Connection-owned furnace screen transactions; simulation owns stored slots.
use super::context::ConnContext;
use super::outbound::send_mandatory;
use crate::driver::SimCommand;
use crate::furnace_window::{prepare_player_candidate, FurnaceWindow};
use crate::inventory::PlayerInventory;
use crate::window::WindowState;
use ferrumc_core::{GameMode, PlayerId};
use ferrumc_items::{plan_furnace_click, ItemStack};
use ferrumc_math::{BlockPos, Vec3};
use ferrumc_net::{CompressionState, PlayWriter};
use ferrumc_observability::SessionDebug;
use ferrumc_proto::generated::play::{ClientboundPlayPacket, WindowClick};
use ferrumc_world::FurnaceInventory;
use tokio::sync::oneshot;

pub(super) async fn snapshot(
    ctx: &ConnContext,
    player: PlayerId,
    position: BlockPos,
) -> anyhow::Result<Option<FurnaceInventory>> {
    let (reply, receive) = oneshot::channel();
    ctx.commands
        .send(SimCommand::ProbeFurnace {
            player,
            position,
            reply,
        })
        .await
        .map_err(|_| anyhow::anyhow!("driver closed while opening furnace"))?;
    receive
        .await
        .map_err(|_| anyhow::anyhow!("furnace snapshot canceled"))
}

pub(super) fn resync(
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    inventory: &PlayerInventory,
    windows: &mut WindowState,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    bump: bool,
) -> anyhow::Result<()> {
    let Some(open) = windows.furnace.as_mut() else {
        return Ok(());
    };
    if bump {
        open.bump_state_id();
    }
    send_mandatory(
        writer,
        debug,
        compression,
        &ctx.clock,
        ClientboundPlayPacket::SetContainerContent(open.content_packet(inventory)?),
    )?;
    for packet in open.property_packets() {
        send_mandatory(
            writer,
            debug,
            compression,
            &ctx.clock,
            ClientboundPlayPacket::SetContainerProperty(packet),
        )?;
    }
    Ok(())
}

/// One outstanding query at most; timer changes do not invalidate item revisions.
pub(super) async fn poll(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &PlayerInventory,
    windows: &mut WindowState,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    let Some(open) = windows.furnace.as_ref() else {
        return Ok(());
    };
    let now = ctx.clock.now().get();
    if now < windows.furnace_poll {
        return Ok(());
    }
    let pos = open.position();
    windows.furnace_poll = now.saturating_add(5);
    let Some(current) = snapshot(ctx, player, pos).await? else {
        return Ok(());
    };
    let Some(open) = windows.furnace.as_mut() else {
        return Ok(());
    };
    let old = open.properties();
    if open.set_snapshot(current)? {
        resync(ctx, writer, inventory, windows, debug, compression, true)?;
    } else if open.properties() != old {
        for packet in open.property_packets() {
            send_mandatory(
                writer,
                debug,
                compression,
                &ctx.clock,
                ClientboundPlayPacket::SetContainerProperty(packet),
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn open(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    windows: &mut WindowState,
    position: BlockPos,
    current: FurnaceInventory,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    if windows.has_open() {
        super::handlers::close_open_container(
            ctx,
            player,
            writer,
            inventory,
            windows,
            debug,
            compression,
        )
        .await?;
    }
    let id = windows.allocate_window_id();
    let open = FurnaceWindow::new(id, position, current)?;
    send_mandatory(
        writer,
        debug,
        compression,
        &ctx.clock,
        ClientboundPlayPacket::OpenScreen(open.open_screen()),
    )?;
    windows.furnace = Some(open);
    windows.furnace_poll = ctx.clock.now().get().saturating_add(5);
    resync(ctx, writer, inventory, windows, debug, compression, false)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn click(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    windows: &mut WindowState,
    packet: &WindowClick,
    position: Option<Vec3>,
    yaw: f32,
    pitch: f32,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    let Some(open) = windows.furnace.as_ref() else {
        return Ok(());
    };
    if packet.window_id() != open.window_id()
        || packet.state_id() != open.state_id()
        || inventory.game_mode() == GameMode::Spectator
    {
        return resync(ctx, writer, inventory, windows, debug, compression, true);
    }
    let Some((slot, button, mode)) = super::handlers::decode_click_prefix(packet.rest()) else {
        return resync(ctx, writer, inventory, windows, debug, compression, true);
    };
    let slot = match FurnaceWindow::classify_slot(slot) {
        Some(crate::furnace_window::FurnaceWindowSlot::Furnace(index)) => index,
        Some(crate::furnace_window::FurnaceWindowSlot::Player(index)) => {
            index - crate::inventory::MAIN_START + 3
        }
        None => return resync(ctx, writer, inventory, windows, debug, compression, true),
    };
    let expected = open.snapshot().slots().clone();
    let pos = open.position();
    anyhow::ensure!(
        inventory.window_slots().len() == 36,
        "player section changed shape"
    );
    let slots: [ItemStack; 36] =
        std::array::from_fn(|index| inventory.window_slots()[index].clone());
    let cursor = inventory.carried().clone();
    let Ok(plan) = plan_furnace_click(&expected, &slots, &cursor, slot, button, mode) else {
        return resync(ctx, writer, inventory, windows, debug, compression, true);
    };
    let (_, player_after, cursor_after) = plan.into_parts();
    let (reply, receive) = oneshot::channel();
    ctx.commands
        .send(SimCommand::ReadVitals { player, reply })
        .await
        .map_err(|_| anyhow::anyhow!("driver closed preparing furnace inventory"))?;
    let vitals = receive
        .await
        .map_err(|_| anyhow::anyhow!("furnace vitals preflight canceled"))?
        .ok_or_else(|| anyhow::anyhow!("furnace viewer has no health owner"))?;
    let position =
        position.ok_or_else(|| anyhow::anyhow!("furnace viewer has no admitted position"))?;
    // This proves the complete hidden slots/table/cursor budget before either
    // side commits. A fresh preview's ephemeral revision is never adopted.
    if prepare_player_candidate(
        inventory,
        &player_after,
        &cursor_after,
        position,
        yaw,
        pitch,
        vitals,
    )
    .is_err()
    {
        return resync(ctx, writer, inventory, windows, debug, compression, true);
    }
    let (reply, receive) = oneshot::channel();
    ctx.commands
        .send(SimCommand::FurnaceClick {
            player,
            position: pos,
            expected,
            player_slots: Box::new(slots),
            cursor,
            slot,
            button,
            mode,
            reply,
        })
        .await
        .map_err(|_| anyhow::anyhow!("driver closed before furnace exchange"))?;
    let outcome = receive
        .await
        .map_err(|_| anyhow::anyhow!("furnace exchange receipt canceled"))?;
    if let Some((snapshot, slots, cursor)) = outcome {
        inventory.set_window_slots(slots.to_vec());
        inventory.set_carried(cursor);
        if let Some(open) = windows.furnace.as_mut() {
            open.set_snapshot(snapshot)?;
        }
        ctx.commands
            .send(SimCommand::SetEquipment {
                player,
                equipment: inventory.equipment_body()?,
            })
            .await
            .map_err(|_| anyhow::anyhow!("driver closed after furnace exchange"))?;
    }
    resync(ctx, writer, inventory, windows, debug, compression, true)
}
