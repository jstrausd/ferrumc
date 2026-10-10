//! Real-socket chest regression: window/state guards and the driver's atomic
//! snapshot check reject stale clicks before changing either slot section or the
//! cursor. Two viewers deliberately hold conflicting snapshots; resync replies
//! provide deterministic barriers without sleeps or proactive subscriptions.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use ferrumc_app::AppConfig;
use ferrumc_codec::{write_var_int, BoundedReader, BoundedString};
use ferrumc_items::ItemId;
use ferrumc_proto::generated::play::{
    ChatCommand, ClientboundPlayPacket, CloseContainer, SetCreativeSlot, UseItemOn, WindowClick,
};
use ferrumc_proto::types::BlockPosition;
use tokio::time::timeout;

use common::{encode, login_to_play, TestClient};

const GUARD: Duration = Duration::from_secs(10);
const PLAYER_SECTION_START: usize = 27;

fn chest_position() -> BlockPosition {
    BlockPosition::new(10, 64, 8)
}

type Slot = Option<(i32, i32)>; // component-free (count, item id)

struct Contents {
    window: i32,
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
    let added = reader.read_var_int()?;
    let removed = reader.read_var_int()?;
    anyhow::ensure!(
        added == 0 && removed == 0,
        "test expects component-free slots"
    );
    Ok(Some((count, item)))
}

async fn next_contents(client: &mut TestClient) -> anyhow::Result<Contents> {
    loop {
        if let ClientboundPlayPacket::SetContainerContent(content) = client.next_play().await? {
            let mut reader = BoundedReader::new(content.payload());
            let count = usize::try_from(reader.read_var_int()?)?;
            let slots = (0..count)
                .map(|_| read_slot(&mut reader))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let cursor = read_slot(&mut reader)?;
            anyhow::ensure!(reader.remaining() == 0, "trailing container payload");
            return Ok(Contents {
                window: content.window_id(),
                state: content.state_id(),
                slots,
                cursor,
            });
        }
    }
}

async fn set_slot(client: &mut TestClient, slot: i16, item: i32, count: i32) -> anyhow::Result<()> {
    let mut stack = Vec::new();
    write_var_int(&mut stack, count);
    write_var_int(&mut stack, item);
    write_var_int(&mut stack, 0);
    write_var_int(&mut stack, 0);
    client
        .send_frame(&encode(|buf| SetCreativeSlot::new(slot, stack).encode(buf)))
        .await?;
    loop {
        if let ClientboundPlayPacket::SetContainerSlot(updated) = client.next_play().await? {
            if updated.slot() == slot {
                return Ok(());
            }
        }
    }
}

async fn interact(
    client: &mut TestClient,
    position: BlockPosition,
    sequence: i32,
) -> anyhow::Result<()> {
    client
        .send_frame(&encode(|buf| {
            UseItemOn::new(0, position, 1, 0.5, 1.0, 0.5, false, false, sequence).encode(buf)
        }))
        .await
}

async fn open(client: &mut TestClient, sequence: i32) -> anyhow::Result<Contents> {
    interact(client, chest_position(), sequence).await?;
    let window = loop {
        if let ClientboundPlayPacket::OpenScreen(screen) = client.next_play().await? {
            break screen.window_id();
        }
    };
    let contents = next_contents(client).await?;
    anyhow::ensure!(contents.window == window && window != 0, "new chest window");
    Ok(contents)
}

fn click_body(window: i32, state: i32, slot: i16, mode: i32) -> Vec<u8> {
    let mut rest = slot.to_be_bytes().to_vec();
    rest.push(0); // left button
    write_var_int(&mut rest, mode);
    write_var_int(&mut rest, 0); // no claimed changed slots
    write_var_int(&mut rest, 0); // claimed empty cursor; server ignores it
    encode(|buf| WindowClick::new(window, state, rest).encode(buf))
}

async fn click(
    client: &mut TestClient,
    window: i32,
    state: i32,
    slot: i16,
) -> anyhow::Result<Contents> {
    client
        .send_frame(&click_body(window, state, slot, 0))
        .await?;
    let content = next_contents(client).await?;
    anyhow::ensure!(
        content.window == window,
        "resync must target the active chest"
    );
    anyhow::ensure!(content.state != state, "resync must advance the state ID");
    Ok(content)
}

async fn run_flow(addr: SocketAddr) -> anyhow::Result<()> {
    let stone = ItemId::from_name("stone").expect("stone item").id();
    let glass = ItemId::from_name("glass").expect("glass item").id();
    let chest_item = ItemId::from_name("chest").expect("chest item").id();
    let mut actor = login_to_play(addr, "ChestActor").await?;
    let mut viewer = login_to_play(addr, "ChestViewer").await?;
    // Drain the join's inventory before later resync assertions.
    anyhow::ensure!(
        next_contents(&mut actor).await?.window == 0,
        "actor inventory"
    );
    anyhow::ensure!(
        next_contents(&mut viewer).await?.window == 0,
        "viewer inventory"
    );
    set_slot(&mut actor, 9, stone, 8).await?;
    set_slot(&mut actor, 10, glass, 5).await?;
    set_slot(&mut actor, 36, chest_item, 1).await?;
    interact(&mut actor, BlockPosition::new(10, 63, 8), 1).await?;
    loop {
        if let ClientboundPlayPacket::BlockUpdate(update) = actor.next_play().await? {
            if update.location() == chest_position() {
                anyhow::ensure!(update.block_state() != 0, "chest was placed");
                break;
            }
        }
    }
    let mut actor_view = open(&mut actor, 2).await?;
    let mut viewer_view = open(&mut viewer, 1).await?;
    let actor_window = actor_view.window;
    let viewer_window = viewer_view.window;
    anyhow::ensure!(
        actor_view.slots[0].is_none() && viewer_view.slots[0].is_none(),
        "empty chest"
    );

    // A valid player-slot pickup followed by a valid chest placement proves both
    // mutation branches work when IDs and the viewer snapshot are current.
    actor_view = click(&mut actor, actor_window, actor_view.state, 27).await?;
    anyhow::ensure!(
        actor_view.cursor == Some((8, stone)),
        "picked up inventory stack"
    );
    anyhow::ensure!(
        actor_view.slots[PLAYER_SECTION_START].is_none(),
        "inventory slot emptied"
    );
    actor_view = click(&mut actor, actor_window, actor_view.state, 0).await?;
    anyhow::ensure!(
        actor_view.slots[0] == Some((8, stone)) && actor_view.cursor.is_none(),
        "stack deposited"
    );

    // The viewer's own IDs still match its screen, but the other actor changed
    // the authoritative chest. Driver compare+exchange must refuse this pickup.
    viewer_view = click(&mut viewer, viewer_window, viewer_view.state, 0).await?;
    anyhow::ensure!(
        viewer_view.slots[0] == Some((8, stone)) && viewer_view.cursor.is_none(),
        "conflicting viewer resynced without pickup"
    );
    let pickup_state = viewer_view.state;
    viewer_view = click(&mut viewer, viewer_window, viewer_view.state, 0).await?;
    anyhow::ensure!(
        viewer_view.slots[0].is_none() && viewer_view.cursor == Some((8, stone)),
        "current viewer picks up once"
    );
    // The actor is now the stale viewer; it gets fresh empty chest contents.
    actor_view = click(&mut actor, actor_window, actor_view.state, 0).await?;
    anyhow::ensure!(
        actor_view.slots[0].is_none() && actor_view.cursor.is_none(),
        "actor conflict cannot dupe stack"
    );

    // Stale IDs guard the local player-slot branch, too: glass must stay put.
    actor_view = click(&mut actor, actor_window, actor_view.state - 1, 28).await?;
    anyhow::ensure!(
        actor_view.slots[PLAYER_SECTION_START + 1] == Some((5, glass))
            && actor_view.cursor.is_none(),
        "stale player-slot click cannot exchange"
    );
    // Replaying the pickup state, or forging a future state, must not put the
    // viewer's carried stack back into the chest.
    for state in [pickup_state, viewer_view.state + 100] {
        viewer_view = click(&mut viewer, viewer_window, state, 0).await?;
        anyhow::ensure!(
            viewer_view.slots[0].is_none() && viewer_view.cursor == Some((8, stone)),
            "invalid state leaves cursor and chest intact"
        );
    }
    for wrong_window in [viewer_window + 1, 0] {
        viewer
            .send_frame(&click_body(wrong_window, viewer_view.state, 0, 0))
            .await?;
        viewer_view = next_contents(&mut viewer).await?;
        anyhow::ensure!(
            viewer_view.window == viewer_window,
            "wrong window resyncs the active chest"
        );
        anyhow::ensure!(
            viewer_view.slots[0].is_none() && viewer_view.cursor == Some((8, stone)),
            "wrong window leaves items intact"
        );
    }

    verify_close_ids(&mut viewer, viewer_view, stone).await?;
    verify_spectator_chest(&mut actor, actor_view).await
}

async fn verify_spectator_chest(actor: &mut TestClient, mut view: Contents) -> anyhow::Result<()> {
    // Refresh the screen, then place the remaining glass in its chest section.
    actor
        .send_frame(&click_body(view.window, view.state, 0, 1))
        .await?;
    view = next_contents(actor).await?;
    view = click(actor, view.window, view.state, 28).await?;
    view = click(actor, view.window, view.state, 0).await?;
    anyhow::ensure!(
        view.slots[0].is_some() && view.cursor.is_none(),
        "seeded chest"
    );
    actor
        .send_frame(&encode(|out| {
            ChatCommand::new(BoundedString::new("gamemode 3".to_owned())?).encode(out)
        }))
        .await?;
    loop {
        if let ClientboundPlayPacket::GameEvent(event) = actor.next_play().await? {
            if event.reason() == 3 {
                anyhow::ensure!(
                    event.value().to_bits() == 3.0_f32.to_bits(),
                    "spectator mode"
                );
                break;
            }
        }
    }
    // Changing mode while a chest is open must protect both mutation branches.
    for slot in [0, 54] {
        let before = view;
        view = click(actor, before.window, before.state, slot).await?;
        anyhow::ensure!(
            view.slots == before.slots && view.cursor == before.cursor,
            "spectator exchanged an item in chest slot {slot}"
        );
    }
    Ok(())
}

async fn verify_close_ids(
    viewer: &mut TestClient,
    mut viewer_view: Contents,
    stone: i32,
) -> anyhow::Result<()> {
    let viewer_window = viewer_view.window;
    // A pipelined unsupported click is the barrier proving a wrong close ID did
    // not close the window or deposit its carried stack. No timeout-as-assertion.
    let wrong_close = encode(|buf| CloseContainer::new(viewer_window + 1).encode(buf));
    let barrier = click_body(viewer_window, viewer_view.state, 0, 1);
    viewer.send_frames(&[&wrong_close, &barrier]).await?;
    viewer_view = next_contents(viewer).await?;
    anyhow::ensure!(
        viewer_view.window == viewer_window && viewer_view.cursor == Some((8, stone)),
        "wrong close retains current window and cursor"
    );
    viewer
        .send_frame(&encode(|buf| {
            CloseContainer::new(viewer_window).encode(buf)
        }))
        .await?;
    let inventory = next_contents(viewer).await?;
    anyhow::ensure!(
        inventory.window == 0 && inventory.slots[9] == Some((8, stone)),
        "correct close returns cursor exactly once"
    );

    // A delayed close from the old window also cannot close a newly opened one.
    viewer_view = open(viewer, 2).await?;
    anyhow::ensure!(
        viewer_view.window != viewer_window,
        "new window ID allocated"
    );
    let old_close = encode(|buf| CloseContainer::new(viewer_window).encode(buf));
    let barrier = click_body(viewer_view.window, viewer_view.state, 0, 1);
    viewer.send_frames(&[&old_close, &barrier]).await?;
    let still_open = next_contents(viewer).await?;
    anyhow::ensure!(
        still_open.window == viewer_view.window && still_open.slots[0].is_none(),
        "delayed close cannot affect replacement window"
    );
    Ok(())
}

#[tokio::test]
async fn stale_clicks_and_closes_cannot_exchange_items_between_two_chest_viewers() {
    let config = AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 0\nview_distance = 0\nops = [\"ChestActor\"]\n",
    )
    .expect("small deterministic server config");
    let server = ferrumc_app::run(&config).await.expect("server starts");
    timeout(GUARD, run_flow(server.local_addr()))
        .await
        .expect("container regression completes")
        .expect("container flow succeeds");
    timeout(GUARD, server.shutdown())
        .await
        .expect("shutdown completes")
        .expect("clean shutdown");
}
