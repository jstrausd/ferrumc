//! The two-stage join kit: framing + position sync first (flushed), then the
//! spawn-area chunk columns (flushed), in the order a real client needs to leave
//! the loading screen.

use tokio::net::TcpStream;
use tokio::sync::oneshot;

use ferrumc_core::PlayerId;
use ferrumc_math::{ChunkPos, Vec3};
use ferrumc_net::{CompressionState, PlayWriter};
use ferrumc_observability::SessionDebug;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, ClientboundSetHeldItem, Commands, GameEvent, PlayerAbilities,
    SetCenterChunk, SetContainerContent, SetDefaultSpawnPosition,
};

use crate::driver::{SimCommand, StreamedChunk};
use crate::inventory::{PlayerInventory, WINDOW_ID};

use super::chunk_stream::ChunkStream;
use super::context::ConnContext;
use super::outbound::{enqueue_traced_classified, flush_writer, send_mandatory};
use super::teleport::PendingTeleport;
use super::{spawn_sync, GAME_EVENT_CHANGE_GAMEMODE};

/// `Game Event` reason `13`: "level chunks load start". Sent right after
/// `JoinGame` to tell the client the spawn chunks are on their way; without it
/// the client never leaves the "Loading terrain" screen.
const GAME_EVENT_LEVEL_CHUNKS_LOAD_START: u8 = 13;

/// Player Abilities `flags` bits sent to a creative client on join: invulnerable
/// (`0x01`) | allow flying (`0x04`) | instabuild/creative (`0x08`). Flying itself
/// (`0x02`) is left off so the player starts grounded but may take off.
const CREATIVE_ABILITY_FLAGS: i8 = 0x01 | 0x04 | 0x08;

/// Flying speed sent in Player Abilities (the vanilla creative default).
const ABILITY_FLYING_SPEED: f32 = 0.05;

/// Walking speed (field-of-view modifier base) sent in Player Abilities.
const ABILITY_WALKING_SPEED: f32 = 0.1;

/// Sends the join kit in the order a real client needs to leave the loading
/// screen: `JoinGame`, `GameEvent(13)`, `SetCenterChunk`, `SetDefaultSpawnPosition`,
/// the permission-filtered `Commands` graph, a non-zero `SynchronizePlayerPosition`,
/// then the spawn-area chunks.
///
/// The spawn-area chunks are fetched LIVE from the resident shard chunks at join
/// time (a [`SimCommand::StreamChunks`] round-trip), not replayed from a cached
/// snapshot, so an edit a previous session placed in a spawn chunk is reflected on
/// this (re)join. The round-trip also gives this connection a player ticket on each
/// spawn column, released on disconnect via the normal `ReleaseChunks` path.
///
/// The position sync goes out *before* the chunks so the client's position is
/// fixed first. A fresh player's chunk is in the spawn batch that follows. For a
/// returning player outside that batch, the immediate post-kit streaming pump
/// recentres and sends the restored-position chunk before waiting for client
/// movement.
///
/// The sequence is flushed in two stages because the [`PlayWriter`] drains by
/// priority (State before World): flushing the framing-and-position packets, then
/// the chunks, guarantees the position sync lands ahead of the chunk column
/// rather than being reordered after it.
///
/// # Errors
///
/// Returns an error if any stage fails to encode or write to the socket.
#[allow(clippy::too_many_arguments)] // one cohesive step: framing + self player-info + I/O + trace state
#[allow(clippy::too_many_lines)] // one join sequence: framing, abilities, inventory, chunks
pub(super) async fn send_join_kit(
    writer: &mut PlayWriter,
    stream: &mut TcpStream,
    compression: &CompressionState,
    ctx: &ConnContext,
    debug: &mut SessionDebug,
    player: PlayerId,
    name: &str,
    position: Vec3,
    yaw: f32,
    pitch: f32,
    is_returning: bool,
    inventory: &PlayerInventory,
    chunk_stream: &mut ChunkStream,
    pending_teleport: &mut PendingTeleport,
) -> anyhow::Result<()> {
    let kit = &ctx.join_kit;
    let clock = &ctx.clock;

    // Stage 1: enter play, cue the client to expect spawn chunks, fix the world
    // spawn, and teleport the player in — all before any chunk is sent.
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::JoinGame(kit.join_game().clone()),
    );
    // Put the local player on their own tab list: a Player Info Update "Add
    // Player" for themselves. Other players' entries arrive from the session
    // router's join-visibility broadcast; this is the one entry the router cannot
    // send (a player is not in their own viewer set).
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ferrumc_session::player_info_add(player, name),
    );
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::GameEvent(GameEvent::new(GAME_EVENT_LEVEL_CHUNKS_LOAD_START, 0.0)),
    );
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::SetCenterChunk(SetCenterChunk::new(
            kit.spawn_chunk().x(),
            kit.spawn_chunk().z(),
        )),
    );
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::SetDefaultSpawnPosition(SetDefaultSpawnPosition::new(
            kit.spawn_block(),
            0.0,
        )),
    );
    // Declare the command graph so the client renders `/spawn` and `/gamemode` as
    // valid (not red) and offers autocomplete for them. The graph is filtered to
    // this player's permission level AND their granted permission nodes, so a
    // non-operator never receives the level-gated `/gamemode` subtree and a player
    // without a permission-gated command's node never receives that command.
    let command_body = ctx.command_graph(player);
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::Commands(Commands::new(command_body)),
    );
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::SynchronizePlayerPosition(
            pending_teleport.issue(&spawn_sync(0, position, yaw, pitch)),
        ),
    );
    // Player Abilities: tell a creative client it may fly and instabuild, so the
    // flight + creative-reach UX matches the creative mode JoinGame advertised.
    send_mandatory(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::PlayerAbilities(PlayerAbilities::new(
            CREATIVE_ABILITY_FLAGS,
            ABILITY_FLYING_SPEED,
            ABILITY_WALKING_SPEED,
        )),
    )?;
    // Initialize window 0 with the full 46-slot inventory (the starter kit in the
    // hotbar) and the persisted player-owned cursor. Mandatory: a dropped container-content leaves the
    // client's inventory view desynced.
    let container_payload = inventory
        .container_content_payload()
        .map_err(|err| anyhow::anyhow!("encoding join container content: {err}"))?;
    send_mandatory(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::SetContainerContent(SetContainerContent::new(
            WINDOW_ID,
            inventory.state_id(),
            container_payload,
        )),
    )?;

    // For a RETURNING player, restore the saved game mode and held hotbar slot. The
    // JoinGame above always advertises creative and the client defaults to hotbar
    // slot 0, so a fresh joiner needs neither packet; a returning one is corrected
    // here. The GameEvent (reason 3 = change game mode) switches the client's mode to
    // the inventory's authoritative mirror, and `ClientboundSetHeldItem` moves the
    // selector to the restored slot so the held item matches the broadcast equipment.
    if is_returning {
        enqueue_traced_classified(
            writer,
            debug,
            compression,
            clock,
            ClientboundPlayPacket::GameEvent(GameEvent::new(
                GAME_EVENT_CHANGE_GAMEMODE,
                f32::from(inventory.game_mode().as_id()),
            )),
        );
        enqueue_traced_classified(
            writer,
            debug,
            compression,
            clock,
            ClientboundPlayPacket::ClientboundSetHeldItem(ClientboundSetHeldItem::new(i32::from(
                inventory.selected(),
            ))),
        );
    }

    flush_writer(writer, stream, compression, ctx.io_timeout).await?;

    // Stage 2: the spawn-area chunk column packets, fetched LIVE from the resident
    // shard chunks rather than replayed from a cached snapshot. A `StreamChunks`
    // round-trip acquires a player ticket on each spawn column and builds its
    // packet from the current chunk state, so a block a previous session placed in
    // a spawn chunk is present on this (re)join. A restored player outside this
    // batch is recentered by the immediate streaming pump after this function
    // returns. The spawn batch remains up-front rather than paced through that
    // pump so the ordinary spawn join path is unchanged.
    let spawn_positions: Vec<ChunkPos> = kit.chunk_positions().collect();
    let chunks = acquire_join_chunks(
        &ctx.commands,
        spawn_positions,
        &mut chunk_stream.loaded,
        &mut chunk_stream.ticket_handoff_pending,
    )
    .await?;
    for streamed in chunks {
        let outcome = enqueue_traced_classified(
            writer,
            debug,
            compression,
            clock,
            ClientboundPlayPacket::ChunkDataAndLight(streamed.chunk),
        );
        // Count only chunks that actually entered the queue; a tail-dropped chunk
        // never reaches the wire and must not inflate the counter.
        if outcome.is_enqueued() {
            ctx.metrics.incr_chunk_sent(1);
        }
        // Render any signs in the column right after it, so a (re)joining player
        // sees text a previous session left on spawn-area signs.
        for block_entity in streamed.block_entities {
            enqueue_traced_classified(writer, debug, compression, clock, block_entity);
        }
    }
    flush_writer(writer, stream, compression, ctx.io_timeout).await
}

/// Atomically records the actual ticket handoff before any encoding or socket
/// flush can fail. Failed load/encode entries were already released by the driver;
/// a lost receiver makes the driver release only its actual acquired batch.
async fn acquire_join_chunks(
    commands: &tokio::sync::mpsc::Sender<SimCommand>,
    positions: Vec<ChunkPos>,
    acquired: &mut std::collections::BTreeSet<ChunkPos>,
    pending: &mut bool,
) -> anyhow::Result<Vec<StreamedChunk>> {
    let (reply, receive) = oneshot::channel();
    commands
        .send(SimCommand::StreamChunks {
            load: positions,
            unload: Vec::new(),
            reply,
        })
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
    *pending = true;
    let chunks = receive
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver dropped the spawn chunk reply"))?;
    acquired.extend(
        chunks
            .iter()
            .map(|chunk| ChunkPos::new(chunk.chunk.x(), chunk.chunk.z())),
    );
    *pending = false;
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use tokio::sync::mpsc;

    fn streamed(pos: ChunkPos) -> StreamedChunk {
        let chunk = ferrumc_world::FlatWorldGenerator::new().generate(pos);
        StreamedChunk {
            chunk: crate::world::chunk_packet(pos, &chunk).expect("chunk"),
            block_entities: Vec::new(),
        }
    }

    #[tokio::test]
    async fn failed_setup_before_acquisition_never_releases_another_players_tickets() {
        let (commands, mut inbox) = mpsc::channel(1);
        let acquired = BTreeSet::new();
        super::super::play::release_owned_chunks(&commands, &acquired)
            .await
            .expect("nothing owned");
        assert!(
            inbox.try_recv().is_err(),
            "no release for configured but unacquired spawn columns"
        );
    }

    #[tokio::test]
    async fn partial_reply_is_owned_before_any_later_encoding_or_flush_failure() {
        let positions = vec![
            ChunkPos::new(0, 0),
            ChunkPos::new(1, 0),
            ChunkPos::new(2, 0),
        ];
        let returned = vec![positions[0], positions[2]];
        let (commands, mut inbox) = mpsc::channel(1);
        let (acquired_ready, inspect) = oneshot::channel();
        let connection = tokio::spawn(async move {
            let mut acquired = BTreeSet::new();
            let mut pending = false;
            let chunks = acquire_join_chunks(&commands, positions, &mut acquired, &mut pending)
                .await
                .expect("partial successful reply");
            acquired_ready
                .send((acquired.clone(), pending))
                .expect("inspect handoff");
            // Socket failure after receipt cannot undo this already-owned batch.
            drop(chunks);
            super::super::play::release_owned_chunks(&commands, &acquired)
                .await
                .expect("error teardown release");
        });
        let SimCommand::StreamChunks { reply, .. } = inbox.recv().await.expect("acquire") else {
            panic!("acquire command");
        };
        assert!(
            reply
                .send(returned.iter().copied().map(streamed).collect())
                .is_ok(),
            "handoff"
        );
        let (owned, pending) = inspect
            .await
            .expect("ownership recorded before socket stage");
        assert_eq!(owned, returned.iter().copied().collect());
        assert!(!pending);
        let SimCommand::ReleaseChunks { positions } = inbox.recv().await.expect("release") else {
            panic!("release command");
        };
        assert_eq!(
            positions, returned,
            "skipped column was never acquired and cannot be released"
        );
        connection.await.expect("cleanup");
    }

    #[tokio::test]
    async fn missing_join_reply_retains_uncertainty_without_guessing_ticket_positions() {
        let (commands, mut inbox) = mpsc::channel(1);
        let driver = tokio::spawn(async move {
            drop(inbox.recv().await.expect("admitted chunk request"));
        });
        let mut acquired = BTreeSet::new();
        let mut pending = false;
        assert!(acquire_join_chunks(
            &commands,
            vec![ChunkPos::new(0, 0)],
            &mut acquired,
            &mut pending
        )
        .await
        .is_err());
        assert!(
            pending,
            "caller must quarantine an unconfirmed admitted batch"
        );
        assert!(acquired.is_empty(), "no guessed releases");
        driver.await.expect("driver ends");
    }
}
