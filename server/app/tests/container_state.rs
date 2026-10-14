//! Real-socket chest regression: window guards and the driver's atomic
//! snapshot check reject conflicting views before changing either slot section
//! or the cursor. State IDs may lag during vanilla prediction. Two viewers deliberately hold conflicting snapshots; resync replies
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
    click_button_body(window, state, slot, 0, mode)
}

fn click_button_body(window: i32, state: i32, slot: i16, button: u8, mode: i32) -> Vec<u8> {
    let mut rest = slot.to_be_bytes().to_vec();
    rest.push(button);
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
    common::make_creative(&mut actor).await?;
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

    let (actor_after, viewer_after) = verify_predicted_state_clicks(
        &mut actor,
        actor_view,
        &mut viewer,
        viewer_view,
        pickup_state,
        stone,
        glass,
    )
    .await?;
    actor_view = actor_after;
    viewer_view = viewer_after;
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

async fn verify_predicted_state_clicks(
    actor: &mut TestClient,
    mut actor_view: Contents,
    viewer: &mut TestClient,
    mut viewer_view: Contents,
    pickup_state: i32,
    stone: i32,
    glass: i32,
) -> anyhow::Result<(Contents, Contents)> {
    // State IDs are the last received server revision, not replay tokens.
    // Late/predicted actions run against authoritative current stacks, once each.
    actor_view = click(actor, actor_view.window, actor_view.state - 1, 28).await?;
    anyhow::ensure!(
        actor_view.cursor == Some((5, glass)),
        "lagging local pickup accepted"
    );
    actor_view = click(actor, actor_view.window, actor_view.state + 100, 28).await?;
    anyhow::ensure!(
        actor_view.cursor.is_none() && actor_view.slots[28] == Some((5, glass)),
        "future claimed state grants no client item authority"
    );
    viewer_view = click(viewer, viewer_view.window, pickup_state, 0).await?;
    anyhow::ensure!(
        viewer_view.slots[0] == Some((8, stone)) && viewer_view.cursor.is_none(),
        "repeated action deposits exactly the current carried items"
    );
    viewer_view = click(viewer, viewer_view.window, viewer_view.state + 100, 0).await?;
    anyhow::ensure!(
        viewer_view.slots[0].is_none() && viewer_view.cursor == Some((8, stone)),
        "next action picks up once without duplicate inventory"
    );
    Ok((actor_view, viewer_view))
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
async fn conflicting_chest_snapshots_and_wrong_window_ids_cannot_exchange_items() {
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

fn item_total(view: &Contents, item: i32) -> i32 {
    view.slots
        .iter()
        .chain(std::iter::once(&view.cursor))
        .flatten()
        .filter(|(_, id)| *id == item)
        .map(|(count, _)| *count)
        .sum()
}

async fn run_rapid_flow(addr: SocketAddr) -> anyhow::Result<()> {
    let stone = ItemId::from_name("stone").expect("stone").id();
    let glass = ItemId::from_name("glass").expect("glass").id();
    let chest = ItemId::from_name("chest").expect("chest").id();
    let mut client = login_to_play(addr, "RapidChest").await?;
    common::make_creative(&mut client).await?;
    next_contents(&mut client).await?;
    set_slot(&mut client, 9, stone, 64).await?;
    set_slot(&mut client, 10, glass, 16).await?;
    set_slot(&mut client, 36, chest, 1).await?;
    interact(&mut client, BlockPosition::new(10, 63, 8), 1).await?;
    loop {
        if let ClientboundPlayPacket::BlockUpdate(update) = client.next_play().await? {
            if update.location() == chest_position() {
                break;
            }
        }
    }
    let mut view = open(&mut client, 2).await?;
    // One TCP write, no read/ACK barrier: real vanilla prediction shares the
    // last received state ID across pickup plus rapid single-item placements.
    let mut bodies = vec![click_button_body(view.window, view.state, 27, 0, 0)];
    for slot in 0..8 {
        bodies.push(click_button_body(view.window, view.state, slot, 1, 0));
    }
    let refs: Vec<_> = bodies.iter().map(Vec::as_slice).collect();
    client.send_frames(&refs).await?;
    for deposited in 0..=8 {
        view = next_contents(&mut client).await?;
        anyhow::ensure!(
            view.cursor == Some((64 - deposited, stone)),
            "rapid cursor {deposited}"
        );
        anyhow::ensure!(
            item_total(&view, stone) == 64 && item_total(&view, glass) == 16,
            "rapid click conservation"
        );
        for slot in 0..deposited as usize {
            anyhow::ensure!(view.slots[slot] == Some((1, stone)));
        }
    }
    // Held right-button drag: select eight chest slots, one player slot, an
    // incompatible glass slot, and duplicate one slot. Finish atomically.
    bodies = vec![click_button_body(view.window, view.state, -999, 4, 5)];
    for slot in [8, 9, 10, 11, 12, 13, 14, 15, 27, 28, 8] {
        bodies.push(click_button_body(view.window, view.state, slot, 5, 5));
    }
    bodies.push(click_button_body(view.window, view.state, -999, 6, 5));
    let refs: Vec<_> = bodies.iter().map(Vec::as_slice).collect();
    client.send_frames(&refs).await?;
    view = next_contents(&mut client).await?;
    anyhow::ensure!(
        view.cursor == Some((47, stone)) && view.slots[27] == Some((1, stone)),
        "right drag deposits exactly once in each eligible selected slot"
    );
    for slot in 0..16 {
        anyhow::ensure!(view.slots[slot] == Some((1, stone)));
    }
    anyhow::ensure!(
        view.slots[28] == Some((16, glass)) && item_total(&view, stone) == 64,
        "incompatible and repeated drag slots conserve"
    );
    // A left placement followed by both directions of shift transfer in one
    // wire burst uses the authoritative result of each previous action.
    bodies = vec![
        click_button_body(view.window, view.state, 16, 0, 0),
        click_button_body(view.window, view.state, 28, 0, 1),
        click_button_body(view.window, view.state, 16, 0, 1),
    ];
    let refs: Vec<_> = bodies.iter().map(Vec::as_slice).collect();
    client.send_frames(&refs).await?;
    for _ in 0..3 {
        view = next_contents(&mut client).await?;
        anyhow::ensure!(
            item_total(&view, stone) == 64 && item_total(&view, glass) == 16,
            "rapid shift conservation"
        );
    }
    anyhow::ensure!(
        view.cursor.is_none()
            && view.slots[16].is_none()
            && view.slots[17] == Some((16, glass))
            && view.slots[27] == Some((48, stone)),
        "rapid shift completed in both directions"
    );
    // Shift first merged into the existing player stack. Left drag shares that
    // complete 48-stack across five slots; the remainder stays carried.
    view = click(&mut client, view.window, view.state, 27).await?;
    bodies = vec![click_button_body(view.window, view.state, -999, 0, 5)];
    for slot in [18, 19, 20, 21, 22] {
        bodies.push(click_button_body(view.window, view.state, slot, 1, 5));
    }
    bodies.push(click_button_body(view.window, view.state, -999, 2, 5));
    let refs: Vec<_> = bodies.iter().map(Vec::as_slice).collect();
    client.send_frames(&refs).await?;
    view = next_contents(&mut client).await?;
    anyhow::ensure!(
        view.cursor == Some((3, stone)) && item_total(&view, stone) == 64,
        "left drag remainder conserved"
    );
    for slot in 18..23 {
        anyhow::ensure!(view.slots[slot] == Some((9, stone)));
    }
    Ok(())
}

#[tokio::test]
async fn rapid_predicted_right_clicks_drag_and_shift_transfers_preserve_every_item() {
    let config = AppConfig::from_toml_str(
        "bind = \"127.0.0.1:0\"\nspawn_chunk_radius = 0\nview_distance = 0\nops = [\"RapidChest\"]\n",
    ).expect("config");
    let server = ferrumc_app::run(&config).await.expect("server starts");
    timeout(GUARD, run_rapid_flow(server.local_addr()))
        .await
        .expect("rapid flow finishes")
        .expect("rapid flow succeeds");
    timeout(GUARD, server.shutdown())
        .await
        .expect("stop finishes")
        .expect("stop succeeds");
}
