//! The join kit flushes world framing, then the actual destination column,
//! then the initial position sync. A client never starts gravity over a column
//! that the server has not yet delivered.

use tokio::net::TcpStream;
use tokio::sync::{oneshot, watch};
use tokio::time::{interval, sleep_until, Instant, MissedTickBehavior};

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
/// the permission-filtered `Commands` graph, the actual destination chunk, then
/// a non-zero `SynchronizePlayerPosition`. The chunk is flushed before the sync.
///
/// The spawn-area chunks are fetched LIVE from the resident shard chunks at join
/// time (a [`SimCommand::StreamChunks`] round-trip), not replayed from a cached
/// snapshot, so an edit a previous session placed in a spawn chunk is reflected on
/// this (re)join. The round-trip also gives this connection a player ticket on each
/// spawn column, released on disconnect via the normal `ReleaseChunks` path.
///
/// Natural chunks prepare asynchronously, so the actual destination must be
/// acquired under an absolute join deadline before the first teleport. Fresh
/// players retain the rest of their resident spawn batch; returning players
/// outside that batch receive their own destination instead of unrelated spawn
/// columns. Subsequent streaming fills the existing configured view.
///
/// Separate flushes are necessary because the writer drains State before World:
/// queuing a teleport beside a chunk would send the teleport first even if the
/// chunk was enqueued first. A failed initial-column preparation or delivery
/// runs the normal leave-save and actual-ticket release path without issuing
/// that teleport.
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

    // Frame the actual destination before its column is delivered; gravity must
    // not start at a restored position while that column is still preparing.
    let destination = chunk_stream.initialize_center(position);
    let deadline = Instant::now() + ctx.io_timeout;
    let mut shutdown = ctx.shutdown.clone();
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
            destination.x(),
            destination.z(),
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

    guard_join_stage(
        deadline,
        &mut shutdown,
        flush_writer(writer, stream, compression, ctx.io_timeout),
    )
    .await?;

    // The initial destination is mandatory even if generation has not completed
    // in the first StreamChunks reply. The driver stays free to run normal ticks
    // while this connection waits at a bounded cadence for background completion.
    let initial = await_destination_chunk(
        &ctx.commands,
        player,
        destination,
        &mut chunk_stream.loaded,
        &mut chunk_stream.ticket_handoff_pending,
        ctx.chunk_stream_interval
            .min(std::time::Duration::from_millis(250)),
        deadline,
        &mut shutdown,
    )
    .await?;
    queue_destination_chunk(ctx, writer, debug, compression, chunk_stream, initial)?;
    guard_join_stage(
        deadline,
        &mut shutdown,
        flush_writer(writer, stream, compression, ctx.io_timeout),
    )
    .await?;

    // This separate flush is the support-before-gravity boundary. State priority
    // cannot pull the teleport ahead of the already-flushed World column.
    send_mandatory(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::SynchronizePlayerPosition(
            pending_teleport.issue(&spawn_sync(0, position, yaw, pitch)),
        ),
    )?;
    guard_join_stage(
        deadline,
        &mut shutdown,
        flush_writer(writer, stream, compression, ctx.io_timeout),
    )
    .await?;

    let spawn_positions: Vec<ChunkPos> = kit.chunk_positions().collect();
    if spawn_positions.contains(&destination) {
        let remaining = spawn_positions
            .into_iter()
            .filter(|pos| *pos != destination)
            .collect::<Vec<_>>();
        if !remaining.is_empty() {
            let chunks = acquire_join_chunks_until(
                &ctx.commands,
                player,
                remaining,
                &mut chunk_stream.loaded,
                &mut chunk_stream.ticket_handoff_pending,
                deadline,
                &mut shutdown,
            )
            .await?;
            for streamed in chunks {
                super::chunk_stream::queue_streamed_chunk(
                    ctx,
                    writer,
                    debug,
                    compression,
                    chunk_stream,
                    streamed,
                );
            }
            guard_join_stage(
                deadline,
                &mut shutdown,
                flush_writer(writer, stream, compression, ctx.io_timeout),
            )
            .await?;
        }
    }
    Ok(())
}

/// Queue admission, preparation and socket writes share one absolute deadline.
/// Each empty producer reply must not grant another timeout window.
async fn guard_join_stage<T>(
    deadline: Instant,
    shutdown: &mut watch::Receiver<bool>,
    work: impl std::future::Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    anyhow::ensure!(
        !*shutdown.borrow(),
        "server stopped during destination join"
    );
    tokio::select! {
        biased;
        _ = shutdown.changed() => anyhow::bail!("server stopped during destination join"),
        () = sleep_until(deadline) => anyhow::bail!("join destination preparation or delivery timed out"),
        result = work => result,
    }
}

/// The first column must not be tail-dropped: unlike later view columns it is the
/// support for the first teleport and cannot be deferred to another stream pump.
fn queue_destination_chunk(
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    chunks: &mut ChunkStream,
    streamed: StreamedChunk,
) -> anyhow::Result<()> {
    let pos = queue_destination_packets(writer, debug, compression, &ctx.clock, streamed)?;
    ctx.metrics.incr_chunk_sent(1);
    chunks.sent.insert(pos);
    Ok(())
}

/// Context makes the initial destination mandatory even though ordinary view
/// columns are droppable. Packet-type criticality alone cannot express this.
fn queue_destination_packets(
    writer: &mut PlayWriter,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    clock: &ferrumc_observability::ServerClock,
    streamed: StreamedChunk,
) -> anyhow::Result<ChunkPos> {
    let pos = ChunkPos::new(streamed.chunk.x(), streamed.chunk.z());
    let outcome = enqueue_traced_classified(
        writer,
        debug,
        compression,
        clock,
        ClientboundPlayPacket::ChunkDataAndLight(streamed.chunk),
    );
    anyhow::ensure!(
        outcome.is_enqueued(),
        "outbound overflow while delivering initial destination column"
    );
    for packet in streamed.block_entities {
        let outcome = enqueue_traced_classified(writer, debug, compression, clock, packet);
        anyhow::ensure!(
            outcome.is_enqueued(),
            "outbound overflow while delivering initial destination block entity"
        );
    }
    Ok(pos)
}

/// Requests exactly one destination at a time, under one non-renewable deadline.
/// Empty replies mean the background producer is still running, not permission
/// to teleport the client into an absent column. No additional cache is retained.
#[allow(clippy::too_many_arguments)] // explicit destination, ticket ownership, pacing and cancellation
async fn await_destination_chunk(
    commands: &tokio::sync::mpsc::Sender<SimCommand>,
    viewer: PlayerId,
    destination: ChunkPos,
    acquired: &mut std::collections::BTreeSet<ChunkPos>,
    pending: &mut bool,
    cadence: std::time::Duration,
    deadline: Instant,
    shutdown: &mut watch::Receiver<bool>,
) -> anyhow::Result<StreamedChunk> {
    let mut retry = interval(cadence);
    retry.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        anyhow::ensure!(
            !*shutdown.borrow(),
            "server stopped during join chunk preparation"
        );
        tokio::select! {
            biased;
            _ = shutdown.changed() => anyhow::bail!("server stopped during join chunk preparation"),
            () = sleep_until(deadline) => anyhow::bail!("join destination chunk preparation timed out"),
            _ = retry.tick() => {}
        }
        let chunks = acquire_join_chunks_until(
            commands,
            viewer,
            vec![destination],
            acquired,
            pending,
            deadline,
            shutdown,
        )
        .await?;
        if let Some(chunk) = chunks
            .into_iter()
            .find(|chunk| ChunkPos::new(chunk.chunk.x(), chunk.chunk.z()) == destination)
        {
            return Ok(chunk);
        }
    }
}

/// Close canceled receivers before recording a ready reply so actual ownership
/// is never guessed. Successful partial batches remain owned even on deadline.
async fn acquire_join_chunks_until(
    commands: &tokio::sync::mpsc::Sender<SimCommand>,
    viewer: PlayerId,
    positions: Vec<ChunkPos>,
    acquired: &mut std::collections::BTreeSet<ChunkPos>,
    pending: &mut bool,
    deadline: Instant,
    shutdown: &mut watch::Receiver<bool>,
) -> anyhow::Result<Vec<StreamedChunk>> {
    let (reply, mut receive) = oneshot::channel();
    guard_join_stage(deadline, shutdown, async {
        commands
            .send(SimCommand::StreamChunks {
                viewer,
                load: positions,
                load_limit: crate::driver::MAX_STREAM_ADMISSIONS,
                unload: Vec::new(),
                prefetch: Vec::new(),
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver is gone"))
    })
    .await?;
    *pending = true;
    let mut canceled = false;
    let result = tokio::select! {
        biased;
        _ = shutdown.changed() => {
            canceled = true;
            Err(anyhow::anyhow!("server stopped during join chunk preparation"))
        },
        () = sleep_until(deadline) => {
            canceled = true;
            Err(anyhow::anyhow!("join destination chunk preparation timed out"))
        },
        chunks = &mut receive => chunks.map_err(|_| anyhow::anyhow!("simulation driver dropped the destination chunk reply")),
    };
    let chunks = match result {
        Ok(chunks) => chunks,
        Err(error) => {
            receive.close();
            if let Ok(chunks) = receive.try_recv() {
                record_acquired(acquired, &chunks);
            }
            if canceled {
                *pending = false;
            }
            return Err(error);
        }
    };
    record_acquired(acquired, &chunks);
    *pending = false;
    Ok(chunks)
}

fn record_acquired(acquired: &mut std::collections::BTreeSet<ChunkPos>, chunks: &[StreamedChunk]) {
    acquired.extend(
        chunks
            .iter()
            .map(|chunk| ChunkPos::new(chunk.chunk.x(), chunk.chunk.z())),
    );
}

/// Atomically records the actual ticket handoff before any encoding or socket
/// flush can fail. Failed load/encode entries were already released by the driver;
/// a lost receiver makes the driver release only its actual acquired batch.
#[cfg(test)]
async fn acquire_join_chunks(
    commands: &tokio::sync::mpsc::Sender<SimCommand>,
    viewer: PlayerId,
    positions: Vec<ChunkPos>,
    acquired: &mut std::collections::BTreeSet<ChunkPos>,
    pending: &mut bool,
) -> anyhow::Result<Vec<StreamedChunk>> {
    let (_stop, mut shutdown) = watch::channel(false);
    acquire_join_chunks_until(
        commands,
        viewer,
        positions,
        acquired,
        pending,
        Instant::now() + std::time::Duration::from_secs(30),
        &mut shutdown,
    )
    .await
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

    #[test]
    fn destination_world_queue_overflow_is_fatal_before_initial_sync() {
        use ferrumc_net::{BatchLimits, ConnectionLimits, OutboundPriority};
        use ferrumc_observability::ServerClock;
        let mut writer = PlayWriter::new(
            ConnectionLimits::default(),
            PlayWriter::capacities_from(4, 4, 0, 4),
            BatchLimits::default(),
        );
        let mut debug = SessionDebug::new("DroppedInitialDestination");
        let error = queue_destination_packets(
            &mut writer,
            &mut debug,
            &CompressionState::disabled(),
            &ServerClock::new(),
            streamed(ChunkPos::new(0, 0)),
        )
        .expect_err("context requires droppable-by-type column");
        assert!(error.to_string().contains("initial destination column"));
        assert_eq!(writer.total_queued(), 0, "no sync follows a missing column");
        assert_eq!(writer.queued_len(OutboundPriority::World), 0);
    }

    #[test]
    fn destination_block_entity_queue_overflow_is_also_fatal() {
        use ferrumc_net::{BatchLimits, ConnectionLimits};
        use ferrumc_observability::ServerClock;
        let mut writer = PlayWriter::new(
            ConnectionLimits::default(),
            PlayWriter::capacities_from(4, 0, 1, 4),
            BatchLimits::default(),
        );
        let mut debug = SessionDebug::new("DroppedInitialBlockEntity");
        let mut chunk = streamed(ChunkPos::new(0, 0));
        chunk
            .block_entities
            .push(ClientboundPlayPacket::SetCenterChunk(SetCenterChunk::new(
                0, 0,
            )));
        let error = queue_destination_packets(
            &mut writer,
            &mut debug,
            &CompressionState::disabled(),
            &ServerClock::new(),
            chunk,
        )
        .expect_err("complete initial destination required");
        assert!(error.to_string().contains("block entity"));
        assert_eq!(
            writer.total_queued(),
            1,
            "only the column entered; no teleport"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn destination_wait_retries_empty_preparation_without_acquiring_spawn_or_duplicate_tickets(
    ) {
        let destination = ChunkPos::new(-83, 91);
        let (commands, mut inbox) = mpsc::channel(1);
        let (_stop, mut shutdown) = watch::channel(false);
        let driver = tokio::spawn(async move {
            for attempt in 0..3 {
                let SimCommand::StreamChunks {
                    load,
                    unload,
                    prefetch,
                    reply,
                    ..
                } = inbox.recv().await.expect("single destination request")
                else {
                    panic!("destination request");
                };
                assert_eq!(load, vec![destination]);
                assert_eq!(unload, [] as [ChunkPos; 0]);
                assert_eq!(prefetch, [] as [ChunkPos; 0]);
                let chunks = if attempt == 2 {
                    vec![streamed(destination)]
                } else {
                    Vec::new()
                };
                assert!(reply.send(chunks).is_ok());
            }
            assert!(inbox.try_recv().is_err(), "no duplicate acquisition");
        });
        let mut acquired = BTreeSet::new();
        let mut pending = false;
        let started = Instant::now();
        let chunk = await_destination_chunk(
            &commands,
            PlayerId::offline("ColdSavedDestination"),
            destination,
            &mut acquired,
            &mut pending,
            std::time::Duration::from_millis(50),
            started + std::time::Duration::from_secs(1),
            &mut shutdown,
        )
        .await
        .expect("actual destination becomes ready");
        assert_eq!(
            Instant::now() - started,
            std::time::Duration::from_millis(100)
        );
        assert_eq!(ChunkPos::new(chunk.chunk.x(), chunk.chunk.z()), destination);
        assert_eq!(acquired, BTreeSet::from([destination]));
        assert!(!pending);
        driver.await.expect("driver");
    }

    #[tokio::test(start_paused = true)]
    async fn empty_destination_replies_do_not_extend_the_absolute_join_deadline() {
        let destination = ChunkPos::new(100, -100);
        let (commands, mut inbox) = mpsc::channel(1);
        let (_stop, mut shutdown) = watch::channel(false);
        let driver = tokio::spawn(async move {
            let mut attempts = 0;
            while let Some(SimCommand::StreamChunks { reply, .. }) = inbox.recv().await {
                assert!(reply.send(Vec::new()).is_ok());
                attempts += 1;
            }
            attempts
        });
        let mut acquired = BTreeSet::new();
        let mut pending = false;
        let started = Instant::now();
        let elapsed = std::time::Duration::from_millis(125);
        let error = await_destination_chunk(
            &commands,
            PlayerId::offline("NeverReady"),
            destination,
            &mut acquired,
            &mut pending,
            std::time::Duration::from_millis(50),
            started + elapsed,
            &mut shutdown,
        )
        .await
        .err()
        .expect("bounded preparation timeout");
        assert!(error.to_string().contains("timed out"));
        assert_eq!(Instant::now() - started, elapsed);
        assert!(acquired.is_empty());
        assert!(!pending);
        drop(commands);
        assert_eq!(driver.await.expect("driver"), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn destination_deadline_retains_a_delivered_ticket_before_returning_error() {
        let destination = ChunkPos::new(14, 15);
        let (commands, mut inbox) = mpsc::channel(1);
        let (_stop, mut shutdown) = watch::channel(false);
        let driver = tokio::spawn(async move {
            let SimCommand::StreamChunks { reply, .. } = inbox.recv().await.expect("request")
            else {
                panic!("destination request");
            };
            assert!(reply.send(vec![streamed(destination)]).is_ok());
            // Make the absolute deadline ready before the receiver is polled.
            tokio::time::advance(std::time::Duration::from_secs(1)).await;
        });
        let mut acquired = BTreeSet::new();
        let mut pending = false;
        let deadline = Instant::now() + std::time::Duration::from_secs(1);
        let error = await_destination_chunk(
            &commands,
            PlayerId::offline("ExpiredDelivered"),
            destination,
            &mut acquired,
            &mut pending,
            std::time::Duration::from_millis(50),
            deadline,
            &mut shutdown,
        )
        .await
        .err()
        .expect("deadline wins even over ready response");
        assert!(error.to_string().contains("timed out"));
        assert_eq!(
            acquired,
            BTreeSet::from([destination]),
            "teardown owns exact ticket"
        );
        assert!(!pending);
        driver.await.expect("driver");
    }

    #[tokio::test(start_paused = true)]
    async fn destination_shutdown_closes_late_handoff_for_driver_cleanup() {
        let destination = ChunkPos::new(11, 12);
        let (commands, mut inbox) = mpsc::channel(1);
        let (stop, mut shutdown) = watch::channel(false);
        let (return_sender, sender) = oneshot::channel();
        let driver = tokio::spawn(async move {
            let SimCommand::StreamChunks { reply, .. } = inbox.recv().await.expect("request")
            else {
                panic!("destination request");
            };
            stop.send(true).expect("stop");
            assert!(return_sender.send(reply).is_ok(), "retain reply sender");
        });
        let mut acquired = BTreeSet::new();
        let mut pending = false;
        let error = await_destination_chunk(
            &commands,
            PlayerId::offline("StopPreparing"),
            destination,
            &mut acquired,
            &mut pending,
            std::time::Duration::from_millis(50),
            Instant::now() + std::time::Duration::from_secs(30),
            &mut shutdown,
        )
        .await
        .err()
        .expect("stop cancels preparation");
        assert!(error.to_string().contains("stopped"));
        assert!(!pending);
        assert!(acquired.is_empty());
        let reply = sender.await.expect("sender");
        let undelivered = reply
            .send(vec![streamed(destination)])
            .expect_err("late delivery must fail");
        assert_eq!(
            undelivered.len(),
            1,
            "driver receives exact ticket for cleanup"
        );
        driver.await.expect("driver");
    }

    #[tokio::test(start_paused = true)]
    async fn destination_deadline_also_bounds_a_full_command_queue() {
        let destination = ChunkPos::new(9, 10);
        let (commands, _inbox) = mpsc::channel(1);
        commands
            .send(SimCommand::PrepareChunks {
                viewer: PlayerId::offline("QueueOwner"),
                positions: vec![destination],
            })
            .await
            .expect("fill bounded queue");
        let (_stop, mut shutdown) = watch::channel(false);
        let mut acquired = BTreeSet::new();
        let mut pending = false;
        let started = Instant::now();
        let duration = std::time::Duration::from_millis(125);
        let error = await_destination_chunk(
            &commands,
            PlayerId::offline("BlockedQueue"),
            destination,
            &mut acquired,
            &mut pending,
            std::time::Duration::from_millis(50),
            started + duration,
            &mut shutdown,
        )
        .await
        .err()
        .expect("queue admission times out");
        assert!(error.to_string().contains("timed out"));
        assert_eq!(Instant::now() - started, duration);
        assert!(!pending, "a canceled bounded send admitted no request");
        assert!(acquired.is_empty());
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
            let chunks = acquire_join_chunks(
                &commands,
                PlayerId::offline("join_fixture"),
                positions,
                &mut acquired,
                &mut pending,
            )
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
            PlayerId::offline("join_fixture"),
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
