//! Real-socket window-0 transactions, resync barriers and cursor conservation.
mod common;

use std::net::SocketAddr;
use std::time::Duration;

use ferrumc_app::AppConfig;
use ferrumc_codec::{write_var_int, BoundedReader, BoundedString};
use ferrumc_proto::generated::play::{
    ChatCommand, ClientboundPlayPacket, CloseContainer, SetCreativeSlot, WindowClick,
};
use tokio::time::timeout;

use common::{encode, login_to_play, TestClient};

type Slot = Option<(i32, i32)>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct InventoryView {
    state: i32,
    slots: Vec<Slot>,
    cursor: Slot,
}

fn read_slot(reader: &mut BoundedReader<'_>) -> anyhow::Result<Slot> {
    let count = reader.read_var_int()?;
    if count == 0 {
        return Ok(None);
    }
    let item = reader.read_var_int()?;
    anyhow::ensure!(reader.read_var_int()? == 0, "unexpected component");
    anyhow::ensure!(reader.read_var_int()? == 0, "unexpected removed component");
    Ok(Some((count, item)))
}

async fn content(client: &mut TestClient) -> anyhow::Result<InventoryView> {
    loop {
        if let ClientboundPlayPacket::SetContainerContent(packet) = client.next_play().await? {
            anyhow::ensure!(packet.window_id() == 0, "expected player window");
            let mut reader = BoundedReader::new(packet.payload());
            anyhow::ensure!(reader.read_var_int()? == 46, "expected 46 slots");
            let slots = (0..46)
                .map(|_| read_slot(&mut reader))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let cursor = read_slot(&mut reader)?;
            anyhow::ensure!(reader.remaining() == 0, "trailing inventory bytes");
            return Ok(InventoryView {
                state: packet.state_id(),
                slots,
                cursor,
            });
        }
    }
}

async fn click(
    client: &mut TestClient,
    view: &InventoryView,
    window: i32,
    slot: i16,
    button: i8,
    mode: i32,
) -> anyhow::Result<InventoryView> {
    let mut rest = slot.to_be_bytes().to_vec();
    rest.push(button.to_be_bytes()[0]);
    write_var_int(&mut rest, mode);
    write_var_int(&mut rest, 0); // client claims no changed slots
    write_var_int(&mut rest, 0); // client falsely claims an empty cursor
    client
        .send_frame(&encode(|out| {
            WindowClick::new(window, view.state, rest).encode(out)
        }))
        .await?;
    content(client).await
}

async fn game_mode(client: &mut TestClient, mode: i32) -> anyhow::Result<()> {
    client
        .send_frame(&encode(|out| {
            ChatCommand::new(BoundedString::new(format!("gamemode {mode}"))?).encode(out)
        }))
        .await?;
    loop {
        if let ClientboundPlayPacket::GameEvent(packet) = client.next_play().await? {
            if packet.reason() == 3 {
                anyhow::ensure!(
                    packet.value().to_bits() == f32::from(u8::try_from(mode)?).to_bits(),
                    "unexpected mode"
                );
                return Ok(());
            }
        }
    }
}

async fn creative(client: &mut TestClient, slot: i16, count: i32, item: i32) -> anyhow::Result<()> {
    let mut bytes = Vec::new();
    write_var_int(&mut bytes, count);
    if count > 0 {
        for value in [item, 0, 0] {
            write_var_int(&mut bytes, value);
        }
    }
    client
        .send_frame(&encode(|out| SetCreativeSlot::new(slot, bytes).encode(out)))
        .await?;
    loop {
        if let ClientboundPlayPacket::SetContainerSlot(packet) = client.next_play().await? {
            anyhow::ensure!(packet.slot() == slot, "unexpected creative echo");
            return Ok(());
        }
    }
}

async fn close(client: &mut TestClient) -> anyhow::Result<InventoryView> {
    client
        .send_frame(&encode(|out| CloseContainer::new(0).encode(out)))
        .await?;
    content(client).await
}

async fn start() -> (ferrumc_app::RunningServer, SocketAddr) {
    let config = AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 1\nview_distance = 1\nbuiltin_plugins = false\nops = [\"InventoryTester\"]",
    )
    .expect("config");
    let server = ferrumc_app::run(&config).await.expect("start server");
    let addr = server.local_addr();
    (server, addr)
}

async fn ordinary_flow(addr: SocketAddr) -> anyhow::Result<()> {
    let mut client = login_to_play(addr, "InventoryTester").await?;
    let mut view = content(&mut client).await?;
    anyhow::ensure!(
        view.slots.iter().all(Option::is_none) && view.cursor.is_none(),
        "fresh Survival inventory is empty"
    );
    game_mode(&mut client, 1).await?;
    creative(&mut client, 36, 64, 1).await?;
    view = click(&mut client, &view, 0, -999, 0, 0).await?;
    game_mode(&mut client, 0).await?;
    view = click(&mut client, &view, 0, 36, 1, 0).await?;
    anyhow::ensure!(view.slots[36] == Some((32, 1)) && view.cursor == Some((32, 1)));
    view = click(&mut client, &view, 0, 9, 1, 0).await?;
    anyhow::ensure!(view.slots[9] == Some((1, 1)) && view.cursor == Some((31, 1)));
    let stale = view.clone();
    view = click(&mut client, &view, 0, 45, 0, 0).await?;
    anyhow::ensure!(view.slots[45] == Some((31, 1)) && view.cursor.is_none());
    let before = view.clone();
    view = click(&mut client, &stale, 0, 45, 0, 0).await?;
    anyhow::ensure!(
        view.slots == before.slots && view.cursor == before.cursor,
        "stale mutated"
    );
    view = click(&mut client, &view, 0, 45, 0, 0).await?;
    for (window, slot, button, mode) in [
        (99, 9, 0, 0),
        (0, 0, 0, 0),
        (0, 5, 0, 0),
        (0, -999, 0, 0),
        (0, 46, 0, 0),
        (0, 9, 0, 5),
    ] {
        let before = view.clone();
        view = click(&mut client, &view, window, slot, button, mode).await?;
        anyhow::ensure!(
            view.slots == before.slots && view.cursor == before.cursor,
            "refused click mutated"
        );
        anyhow::ensure!(view.state != before.state, "resync needs fresh state");
    }
    view = close(&mut client).await?;
    anyhow::ensure!(view.cursor.is_none() && view.slots[9] == Some((32, 1)));
    // Shift transfer merges the 32 storage items into the held stack of 32.
    let mut expected = view.clone();
    expected.slots[9] = None;
    expected.slots[36] = Some((64, 1));
    view = click(&mut client, &view, 0, 9, 0, 1).await?;
    anyhow::ensure!(
        view.slots == expected.slots && view.cursor == expected.cursor,
        "supported shift conserves all 64 items and every other owned slot"
    );
    game_mode(&mut client, 3).await?;
    let before = view.clone();
    // A normal pickup would move this nonempty stack onto the cursor.
    view = click(&mut client, &view, 0, 36, 0, 0).await?;
    anyhow::ensure!(
        view.slots == before.slots && view.cursor == before.cursor,
        "spectator mutated"
    );
    Ok(())
}

async fn full_bag_flow(addr: SocketAddr) -> anyhow::Result<()> {
    let mut client = login_to_play(addr, "InventoryTester").await?;
    let initial = content(&mut client).await?;
    game_mode(&mut client, 1).await?;
    for slot in 9..45 {
        creative(&mut client, slot, 64, 1).await?;
    }
    creative(&mut client, 1, 7, 1).await?;
    creative(&mut client, 2, 3, 1).await?;
    // A deliberately stale click yields a fresh authoritative barrier after seeding.
    let mut view = click(&mut client, &initial, 0, -999, 0, 0).await?;
    game_mode(&mut client, 0).await?;
    view = click(&mut client, &view, 0, 1, 0, 0).await?;
    anyhow::ensure!(view.cursor == Some((7, 1)) && view.slots[1].is_none());
    for _ in 0..2 {
        view = close(&mut client).await?;
        anyhow::ensure!(view.cursor == Some((7, 1)), "full bag discarded cursor");
        anyhow::ensure!(
            view.slots[2] == Some((3, 1)),
            "full bag discarded crafting input"
        );
        anyhow::ensure!(view.slots[9..45].iter().all(|slot| *slot == Some((64, 1))));
    }
    Ok(())
}

#[tokio::test]
async fn normal_clicks_survival_resync_and_spectator_policy() {
    let (server, addr) = start().await;
    let result = timeout(Duration::from_secs(10), ordinary_flow(addr)).await;
    server.shutdown().await.expect("shutdown");
    result.expect("bounded flow").expect("ordinary flow");
}

#[tokio::test]
async fn close_window_zero_with_a_full_bag_preserves_cursor_and_crafting_input() {
    let (server, addr) = start().await;
    let result = timeout(Duration::from_secs(10), full_bag_flow(addr)).await;
    server.shutdown().await.expect("shutdown");
    result.expect("bounded flow").expect("full bag flow");
}
