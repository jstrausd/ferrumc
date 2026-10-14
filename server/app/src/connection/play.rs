//! The play phase: join the simulation, replay the join kit, then pump
//! serverbound packets and clientbound outputs until disconnect.

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, watch};
use tokio::time::{interval_at, sleep_until, Instant, Interval, MissedTickBehavior};
enum ConnectionWake {
    Play(PlayWake),
    Effect(Option<crate::gameplay::GameplayEffect>),
}

use ferrumc_codec::BoundedString;
use ferrumc_core::{GameMode, PlayerId};
use ferrumc_math::Vec3;
use ferrumc_net::{
    CompressionState, ConnectionLiveness, ConnectionState, Criticality, DecodeError,
    DisconnectReason, FrameDecodeError, InboundDecoder, InboundPacket, KeepAliveError,
    LivenessActivityError, LivenessConfig, LivenessDeadline, LivenessTimeout, PlayWriter,
    DEFAULT_KEEP_ALIVE_TIMEOUT,
};
use ferrumc_observability::{PacketState, SessionDebug};
use ferrumc_proto::generated::play::{
    ClientboundKeepAlive, ClientboundPlayPacket, GameEvent, PlayDisconnect,
};
use ferrumc_session::{NetEvent, OutboundMessage, PlayerSessionHandle, SessionError};

use crate::driver::SimCommand;
use crate::inventory::PlayerInventory;
use crate::observe;
use crate::player_admission::AdmissionError;
use crate::player_data::{is_valid_player_position, load_player_for_join, PlayerData, PlayerLoad};
use crate::window::WindowState;

use super::chunk_stream::{apply_chunk_stream, mirror_server_teleport, ChunkStream};
use super::context::ConnContext;
use super::handlers::{handle_gameplay_effect, handle_play_body, resume_pending_mining};
use super::ingress::{is_exact_keep_alive, IngressFence};
use super::join::send_join_kit;
use super::outbound::{
    enqueue_traced, enqueue_traced_classified, flush_writer, is_mandatory_overflow,
    observe_queue_len, send_mandatory,
};
use super::rate_limiter::ChatRateLimiter;
use super::serverbound_budget::{BudgetAdmission, ServerboundBudget, ServerboundBudgetExceeded};
use super::teleport::{PendingTeleport, TeleportConfirmationError};
use super::{send_sim_command_accepted, Connection, GAME_EVENT_CHANGE_GAMEMODE, READ_CHUNK};

/// Joins the simulation and replays the join kit, then pumps the play link until
/// the client disconnects or the server shuts down.
#[allow(clippy::too_many_lines)] // one cohesive lifecycle: join, replay, pump, dump
pub(super) async fn enter_play(
    conn: Connection<'_>,
    name: BoundedString<16>,
    player: PlayerId,
    shutdown: &mut watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let Connection {
        mut stream,
        mut decoder,
        compression,
        ctx,
        mut debug,
        client_view_distance,
        ..
    } = conn;
    // Upgrade the session label from the peer address to the player name now that
    // login has completed.
    debug.set_session(name.as_str());

    // Claim before any storage read. The old connection owns this UUID until
    // its durable save and acknowledged simulation leave have both completed.
    let mut reservation = match ctx.player_admissions.reserve(player) {
        Ok(reservation) => reservation,
        Err(error) => {
            tracing::warn!(%error, %player, name = name.as_str(), "rejecting overlapping player admission");
            let mut writer = PlayWriter::with_defaults(ctx.limits);
            let refusal = send_mandatory(
                &mut writer,
                &mut debug,
                &compression,
                &ctx.clock,
                admission_refusal(error),
            );
            if let Err(send_error) = refusal {
                tracing::debug!(%send_error, "could not enqueue player admission refusal");
            } else if let Err(send_error) =
                flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout).await
            {
                tracing::debug!(%send_error, "could not flush player admission refusal");
            }
            return Err(error.into());
        }
    };

    // Resolve storage admission before joining simulation or constructing any
    // teardown state. Only a confirmed store miss may start fresh; backend,
    // schema, and payload failures return from `enter_play`, close the connection,
    // and therefore cannot reach the leave-save below.
    let player_load = load_player_for_join(ctx.player_store.as_ref(), player)
        .await
        .map_err(|error| {
            tracing::warn!(
                %error,
                player = name.as_str(),
                "rejecting Play admission because persisted player state is unreadable"
            );
            error
        })?;
    let restored = match player_load {
        PlayerLoad::Restored { data, game_mode } => Some((data, game_mode)),
        PlayerLoad::NotFound => None,
    };
    let is_returning = restored.is_some();
    let restored_vitals = restored.as_ref().map_or_else(
        || ferrumc_sim::PlayerVitals::new().snapshot(),
        |(data, _)| data.vitals(),
    );

    // Resolve the join state from the restored record or the spawn defaults. The
    // authoritative server-side inventory is built here, BEFORE the join, so the
    // joiner's (restored) held item is cached in the router as part of the join —
    // viewers entering view then see it on the spawn rather than only after the next
    // hotbar change. The inventory mirrors the (restored or default-creative) game
    // mode; the connection is the sole writer of both, so the mirror cannot drift.
    let (position, mut player_yaw, mut player_pitch, game_mode, mut inventory) =
        if let Some((data, game_mode)) = restored {
            let stored_position = data.position();
            let position = if is_valid_player_position(stored_position) {
                stored_position
            } else {
                // A current-schema record with an unsafe coordinate is recoverable:
                // retain its decoded inventory/mode/look, join at the configured
                // spawn, and let the normal leave-save write a normalized snapshot
                // with the recovered position. This is an explicit reset policy,
                // not the unreadable-record fallback Packet 24 forbids. If even the
                // configured recovery point is unsafe, reject admission rather
                // than route saturated integer coordinates.
                let recovery = ctx.join_kit.spawn_position();
                anyhow::ensure!(
                    is_valid_player_position(recovery),
                    "cannot recover invalid player position because configured spawn is unsafe"
                );
                tracing::warn!(
                    player = name.as_str(),
                    stored_x = stored_position.x,
                    stored_y = stored_position.y,
                    stored_z = stored_position.z,
                    recovery_x = recovery.x,
                    recovery_y = recovery.y,
                    recovery_z = recovery.z,
                    "recovering an invalid persisted player position at spawn"
                );
                recovery
            };
            (
                position,
                data.yaw(),
                data.pitch(),
                game_mode,
                data.restore_inventory(game_mode),
            )
        } else {
            let position = ctx.join_kit.spawn_position();
            anyhow::ensure!(
                is_valid_player_position(position),
                "configured spawn position is unsafe"
            );
            (
                position,
                0.0_f32,
                0.0_f32,
                GameMode::Survival,
                PlayerInventory::from_persisted(
                    GameMode::Survival,
                    0,
                    std::array::from_fn(|_| ferrumc_items::ItemStack::empty()),
                    ferrumc_items::ItemStack::empty(),
                ),
            )
        };

    // The opaque equipment body threaded into the join: the player's full visible
    // set (main hand, off hand, and armor), reflecting the restored slots. The kit
    // never fails to encode; on the off chance it does, fall back to empty (no
    // equipment shown) rather than aborting the join.
    let equipment = inventory.equipment_body().unwrap_or_else(|err| {
        tracing::warn!(%err, "failed to encode initial equipment; joining without it");
        Vec::new()
    });
    // Validate structural deadline setup before joining. Every later error must
    // pass through the shared leave-save path, even a failed initial kit send.
    let mut liveness = play_liveness(Instant::now(), ctx.io_timeout)?;
    let mut handle = join_simulation(ctx, player, name.as_str(), position, equipment).await?;

    // The client holds the spawn batch after the join kit; the stream tracks it from
    // there so it never re-sends a spawn chunk and knows what to unload. Seed it with
    // the player's actual (restored) position so the first streaming pass centres on
    // where they rejoin: if they left far from spawn, their chunk is then streamed in
    // (centre-first) and the loading screen releases without waiting for a move.
    let mut chunk_stream = ChunkStream::before_join(ctx, player);
    chunk_stream.limit_view_distance(ctx.view_distance, client_view_distance);
    chunk_stream.observe(position);

    // Per-connection chat rate limiter, seeded at the current server tick. Lives
    // here (the connection task may use a non-deterministic time source) and is
    // never touched by the sim/session deterministic tick path.
    let mut chat_limiter = ChatRateLimiter::new(ctx.clock.now());

    // Per-connection serverbound packet budget: a token bucket (~300 frames/sec
    // sustained, 600 burst by default) charged one token per serverbound play
    // frame. A sustained flood drains the bucket and the connection is dropped with
    // `BudgetExceeded`, so a hostile client cannot exhaust the server by spamming
    // play packets. Like the chat limiter, it lives in the connection task (allowed
    // a non-deterministic clock) and never touches the deterministic tick path.
    let mut budget = ServerboundBudget::new(
        Instant::now().into_std(),
        ctx.budget.sustained_rate,
        ctx.budget.burst,
    );

    // Make the sim's authoritative mode match the restored (or default-creative)
    // mode so the creative-slot gate accepts this player and later enforcement
    // (creative no-decrement, break speed, flight) reads the right mode.
    let mut writer = PlayWriter::with_defaults(ctx.limits);
    let mut pending_teleport = PendingTeleport::default();
    let mut ingress = IngressFence::default();
    // Subscribe before the initial graph. Any concurrent mutation during the
    // join kit remains pending and triggers a fresh graph after the handoff.
    let mut operator_changes = ctx.policy.operators().subscribe();
    let (effect_tx, mut effect_rx) = tokio::sync::mpsc::channel(crate::gameplay::EFFECT_CAPACITY);
    let setup: anyhow::Result<()> = async {
        let (reply, registered) = oneshot::channel();
        ctx.commands
            .send(SimCommand::RegisterGameplay {
                player,
                sender: effect_tx,
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
        anyhow::ensure!(
            registered.await?,
            "trusted gameplay receiver registration refused"
        );
        send_sim_command_accepted(
            ctx,
            SimCommand::SetGameMode {
                player,
                mode: game_mode,
                acceptance: None,
            },
        )
        .await?;
        // Restore mode, look, inventory and cursor before buffered Play packets.
        send_join_kit(
            &mut writer,
            &mut stream,
            &compression,
            ctx,
            &mut debug,
            player,
            handle.entity_id(),
            name.as_str(),
            position,
            player_yaw,
            player_pitch,
            is_returning,
            &inventory,
            &mut chunk_stream,
            &mut pending_teleport,
        )
        .await?;
        // Start absolute deadlines only after the kit reached the socket.
        liveness = play_liveness(Instant::now(), ctx.io_timeout)?;
        Ok(())
    }
    .await;
    // Drain the frames already buffered before the loop. Any error here — a
    // serverbound budget kick on a flood pipelined during the login handoff, or a
    // decode/socket failure — is deferred to a break inside the loop rather than
    // returned via `?`, so this connection still runs the shared leave-save and
    // chunk-ticket release teardown below instead of leaking the join's tickets.
    let mut deferred_break = setup.err();
    if deferred_break.is_none() {
        if let Err(error) = send_sim_command_accepted(
            ctx,
            SimCommand::GameplayInput {
                player,
                input: ferrumc_sim::GameInput::RestoreVitals {
                    player,
                    snapshot: restored_vitals,
                },
                acceptance: None,
            },
        )
        .await
        {
            deferred_break = Some(error);
        }
    }
    // Per-connection open-container window state (a chest screen, when open). The
    // always-open player inventory lives in `inventory`; this tracks the at-most-one
    // container window layered on top of it.
    let mut window_state = WindowState::new();
    let mut java_join_attempted = false;
    if let Some(bridge) = ctx
        .java_bridge
        .as_ref()
        .filter(|_| deferred_break.is_none())
    {
        java_join_attempted = true;
        let outcome = bridge
            .event(true, ctx.java_player(player, name.as_str()))
            .await;
        match outcome {
            Ok(messages) => {
                if let Err(error) = super::outbound::send_java_messages(
                    ctx,
                    &mut writer,
                    &mut debug,
                    &compression,
                    messages,
                ) {
                    deferred_break = Some(error);
                }
            }
            Err(error) => {
                tracing::warn!(%error, player = name.as_str(), "Java join callback failed");
                deferred_break = Some(error.into());
            }
        }
    }
    if deferred_break.is_none() {
        if let Err(err) = pump_serverbound(
            &mut decoder,
            &compression,
            ctx,
            player,
            name.as_str(),
            &mut writer,
            &mut chunk_stream,
            &mut chat_limiter,
            &mut budget,
            &mut inventory,
            &mut window_state,
            &mut player_yaw,
            &mut player_pitch,
            &mut debug,
            &mut liveness,
            &mut pending_teleport,
            &mut ingress,
            &mut handle,
        )
        .await
        {
            deferred_break = Some(err);
        }
    }
    if deferred_break.is_none() {
        // The join position (or the latest valid movement pipelined during the
        // handoff) is the first fixed-cadence stream target. This one eager pass
        // releases the loading screen; steady-state socket reads only update the
        // coalesced pending target and never run chunk work themselves.
        if let Err(err) = apply_chunk_stream(
            ctx,
            &mut writer,
            &mut chunk_stream,
            &mut debug,
            &compression,
            liveness.next_deadline(),
        )
        .await
        {
            deferred_break = Some(err);
        }
    }
    if deferred_break.is_none() {
        match guard_play_work(
            liveness.next_deadline(),
            flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout),
        )
        .await
        {
            Ok(()) => observe_queue_len(&mut debug, ctx, &writer, budget.over_budget()),
            Err(err) => deferred_break = Some(err),
        }
    }

    // Keep one unanswered Keep Alive at a time. A shorter configured cadence must
    // never replace its ID or extend its absolute response deadline.
    let mut keep_alive = interval_at(
        Instant::now() + ctx.keep_alive_interval,
        ctx.keep_alive_interval,
    );
    keep_alive.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut keep_alive_id: i64 = 0;

    // Chunk-stream pump: advance a standing player's view toward the full
    // advertised view distance without waiting for a movement packet. The initial
    // fill already ran in the eager post-handoff stream pass above; this drains
    // the remaining backlog one bounded batch per interval and consumes only the
    // latest accepted target. `Delay` skips missed ticks under load rather than
    // bursting to catch up.
    let mut chunk_pump = interval_at(
        Instant::now() + ctx.chunk_stream_interval,
        ctx.chunk_stream_interval,
    );
    chunk_pump.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let mut read_buf = [0u8; READ_CHUNK];
    let mut next_work = 0;
    let result = loop {
        // Immutable ordinary equipment facts are refreshed before any later
        // interaction command, including changes caused by owner pickup effects.
        let profile = crate::combat::CombatProfile::from_inventory(&inventory).ok();
        let combat_revision = (
            inventory.state_id(),
            inventory.selected(),
            inventory.game_mode(),
        );
        if window_state.combat_revision != Some(combat_revision)
            || window_state.combat_profile != profile
        {
            if ctx
                .commands
                .send(SimCommand::CombatProfile {
                    player,
                    profile: inventory
                        .combat_snapshot()
                        .ok()
                        .filter(|p| p.encoded_bytes() <= 64 * 1024)
                        .map(std::sync::Arc::new),
                })
                .await
                .is_err()
            {
                break Err(anyhow::anyhow!(
                    "driver closed while refreshing combat profile"
                ));
            }
            window_state.combat_profile = profile;
            window_state.combat_revision = Some(combat_revision);
        }
        if window_state.consumption.pending.is_none() {
            if let Err(error) = super::furnace::poll(
                ctx,
                player,
                &mut writer,
                &inventory,
                &mut window_state,
                &mut debug,
                &compression,
            )
            .await
            {
                break Err(error);
            }
        }
        if let Err(error) =
            super::handlers::advance_consumption(ctx, player, &inventory, &mut window_state).await
        {
            break Err(error);
        }
        if let Err(error) = chunk_stream.report_delivered(&ctx.commands, player).await {
            break Err(error);
        }
        // A deferred failure from the pre-loop drain (e.g. a budget kick on a
        // flood pipelined during the login handoff) breaks here so the teardown
        // below still runs — the same save + ticket-release path a steady-state
        // disconnect takes.
        if let Some(err) = deferred_break.take() {
            break Err(err);
        }
        if let Err(error) = guard_play_work(
            liveness.next_deadline(),
            resume_pending_mining(
                ctx,
                player,
                &mut writer,
                &inventory,
                &mut window_state,
                &mut debug,
                &compression,
            ),
        )
        .await
        {
            break Err(error);
        }
        let Some(deadline) = liveness.next_deadline() else {
            break Err(anyhow::anyhow!(
                "Play liveness closed without a disconnect reason"
            ));
        };
        let quota_ready = ingress.quota_ready();
        let fenced = ingress.is_fenced() || quota_ready.is_some();
        let wake = tokio::select! {
            biased;
            _ = shutdown.changed() => break Ok(()),
            () = sleep_until(deadline.at()) => break Err(deadline.timeout().into()),
            effect = effect_rx.recv() => ConnectionWake::Effect(effect),
            wake = next_play_wake(
                &mut next_work,
                &mut keep_alive,
                &mut chunk_pump,
                async {
                    if let Some(message) = ingress.pop_outbound() {
                        Some(message)
                    } else {
                        handle.recv().await
                    }
                },
                async {
                    if fenced {
                        std::future::pending().await
                    } else {
                        stream.read(&mut read_buf).await
                    }
                },
                operator_changes.changed(),
                quota_ready,
            ) => ConnectionWake::Play(wake),
        };
        let wake = match wake {
            ConnectionWake::Play(wake) => wake,
            ConnectionWake::Effect(Some(effect)) => {
                if let crate::gameplay::GameplayEffect::Respawned { position } = &effect {
                    let world = ctx.join_kit.join_game().world_state();
                    let spawn = ferrumc_proto::generated::play::SpawnInfo::new(
                        world.dimension_type(),
                        world.dimension_name().clone(),
                        world.hashed_seed(),
                        inventory.game_mode().as_id(),
                        inventory.game_mode().as_id(),
                        world.is_debug(),
                        world.is_flat(),
                        None,
                        world.portal_cooldown(),
                        world.sea_level(),
                    );
                    let respawn = ClientboundPlayPacket::Respawn(
                        ferrumc_proto::generated::play::Respawn::new(spawn, 0),
                    );
                    let outcome = async {
                        send_mandatory(&mut writer, &mut debug, &compression, &ctx.clock, respawn)?;
                        flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout)
                            .await?;
                        let destination = ferrumc_math::BlockPos::new(
                            position.x.floor() as i32,
                            0,
                            position.z.floor() as i32,
                        )
                        .to_chunk_pos();
                        let (reply, receive) = oneshot::channel();
                        if chunk_stream.loaded.contains(&destination) {
                            ctx.commands
                                .send(SimCommand::ResendChunks {
                                    positions: vec![destination],
                                    reply,
                                })
                                .await
                                .map_err(|_| {
                                    anyhow::anyhow!("driver closed preparing respawn ground")
                                })?;
                            let columns = receive
                                .await
                                .map_err(|_| anyhow::anyhow!("respawn ground snapshot canceled"))?;
                            let column = columns.into_iter().next().ok_or_else(|| {
                                anyhow::anyhow!("respawn support is no longer resident")
                            })?;
                            super::join::queue_destination_chunk(
                                ctx,
                                &mut writer,
                                &mut debug,
                                &compression,
                                &mut chunk_stream,
                                column,
                            )?;
                        } else {
                            let column = super::join::await_destination_chunk(
                                &ctx.commands,
                                player,
                                destination,
                                &mut chunk_stream.loaded,
                                &mut chunk_stream.ticket_handoff_pending,
                                ctx.chunk_stream_interval,
                                Instant::now() + ctx.io_timeout,
                                shutdown,
                            )
                            .await?;
                            super::join::queue_destination_chunk(
                                ctx,
                                &mut writer,
                                &mut debug,
                                &compression,
                                &mut chunk_stream,
                                column,
                            )?;
                        }
                        flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout)
                            .await?;
                        let sync =
                            pending_teleport.issue(&super::spawn_sync(0, *position, 0.0, 0.0));
                        mirror_server_teleport(
                            &mut chunk_stream,
                            &mut player_yaw,
                            &mut player_pitch,
                            &sync,
                        );
                        send_mandatory(
                            &mut writer,
                            &mut debug,
                            &compression,
                            &ctx.clock,
                            ClientboundPlayPacket::SynchronizePlayerPosition(sync),
                        )?;
                        Ok::<(), anyhow::Error>(())
                    }
                    .await;
                    if let Err(error) = outcome {
                        break Err(error);
                    }
                }
                if let Err(error) = handle_gameplay_effect(
                    ctx,
                    player,
                    handle.entity_id(),
                    &mut writer,
                    &mut inventory,
                    &mut window_state,
                    effect,
                    &mut debug,
                    &compression,
                )
                .await
                {
                    break Err(error);
                }
                if let Err(error) = guard_play_work(
                    liveness.next_deadline(),
                    flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout),
                )
                .await
                {
                    break Err(error);
                }
                continue;
            }
            ConnectionWake::Effect(None) => {
                break Err(anyhow::anyhow!("trusted gameplay producer closed"))
            }
        };
        match wake {
            PlayWake::Operators(changed) => {
                if let Err(error) = changed {
                    break Err(error.into());
                }
                let graph =
                    ferrumc_proto::generated::play::Commands::new(ctx.command_graph(player));
                if let Err(error) = send_mandatory(
                    &mut writer,
                    &mut debug,
                    &compression,
                    &ctx.clock,
                    ClientboundPlayPacket::Commands(graph),
                ) {
                    break Err(error);
                }
                if let Err(error) = guard_play_work(
                    liveness.next_deadline(),
                    flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout),
                )
                .await
                {
                    break Err(error);
                }
                observe_queue_len(&mut debug, ctx, &writer, budget.over_budget());
            }
            PlayWake::KeepAlive => {
                if liveness.outstanding_keep_alive().is_some() {
                    continue;
                }
                keep_alive_id = keep_alive_id.wrapping_add(1);
                let packet = ClientboundPlayPacket::ClientboundKeepAlive(
                    ClientboundKeepAlive::new(keep_alive_id),
                );
                let criticality = Criticality::for_packet(&packet);
                let outcome = enqueue_traced_classified(
                    &mut writer,
                    &mut debug,
                    &compression,
                    &ctx.clock,
                    packet,
                );
                if is_mandatory_overflow(criticality, outcome) {
                    // A full Critical queue means the client cannot drain even
                    // keep-alives: the Layer-B mirror of the router's mandatory
                    // slow-client policy (DisconnectReason::OutboundOverflow).
                    break Err(anyhow::anyhow!(
                        "outbound overflow: a mandatory keep-alive was dropped at the connection writer"
                    ));
                }
                if let Err(err) = guard_play_work(
                    liveness.next_deadline(),
                    flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout),
                )
                .await
                {
                    break Err(err);
                }
                if let Err(err) = liveness.keep_alive_sent(keep_alive_id, Instant::now()) {
                    break Err(err.into());
                }
                observe_queue_len(&mut debug, ctx, &writer, budget.over_budget());
            }
            PlayWake::Chunks => {
                // Advance the view one bounded batch toward full view distance from
                // the current center, even if the player never moved. Bounded per
                // pump, so this paces the backlog out without flooding the socket.
                if let Err(err) = apply_chunk_stream(
                    ctx,
                    &mut writer,
                    &mut chunk_stream,
                    &mut debug,
                    &compression,
                    liveness.next_deadline(),
                )
                .await
                {
                    break Err(err);
                }
                if let Err(err) = guard_play_work(
                    liveness.next_deadline(),
                    flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout),
                )
                .await
                {
                    break Err(err);
                }
                observe_queue_len(&mut debug, ctx, &writer, budget.over_budget());
            }
            PlayWake::Outbound(outbound) => match outbound {
                // Clientbound simulation output: queue and flush to the socket.
                Some(msg) => {
                    // The envelope carries the criticality AND priority the router
                    // (Layer A) assigned at the send site, so Layer B honors that
                    // intent instead of re-inferring it from packet type — which is
                    // wrong for context-dependent packets (an actor-resync BlockUpdate
                    // is mandatory, a viewer-broadcast BlockUpdate is droppable). The
                    // router already disconnects a slow client rather than silently
                    // drop a mandatory packet; this mirrors that here so a full
                    // priority queue can never silently drop a mandatory frame
                    // (despawn/spawn/ack/correction/resync) either.
                    if let Err(error) = queue_outbound(
                        msg,
                        ctx,
                        &mut writer,
                        &mut chunk_stream,
                        &mut inventory,
                        &mut player_yaw,
                        &mut player_pitch,
                        &mut pending_teleport,
                        &mut debug,
                        &compression,
                    ) {
                        break Err(error);
                    }
                    // One channel message is enqueued and flushed per loop turn.
                    // The router's atomic resync+ack group relies on this: the FIFO
                    // channel yields the (Mandatory, State) resync before the ack, so
                    // batching several messages before a flush must NOT be introduced
                    // here without re-establishing that ordering, or a dropped resync
                    // could leave the ack behind.
                    if let Err(err) = guard_play_work(
                        liveness.next_deadline(),
                        flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout),
                    )
                    .await
                    {
                        break Err(err);
                    }
                    observe_queue_len(&mut debug, ctx, &writer, budget.over_budget());
                    // A finite prefix has made progress. Once it is empty, the
                    // held frame runs even if new outputs remain continuously ready.
                    if let Err(error) = pump_serverbound(
                        &mut decoder,
                        &compression,
                        ctx,
                        player,
                        name.as_str(),
                        &mut writer,
                        &mut chunk_stream,
                        &mut chat_limiter,
                        &mut budget,
                        &mut inventory,
                        &mut window_state,
                        &mut player_yaw,
                        &mut player_pitch,
                        &mut debug,
                        &mut liveness,
                        &mut pending_teleport,
                        &mut ingress,
                        &mut handle,
                    )
                    .await
                    {
                        break Err(error);
                    }
                    // Releasing the held input can enqueue its own response or
                    // inline teleport. Flush that work now: rare chunk/Keep
                    // Alive timers and a silent client cannot be its trigger.
                    // The FIFO message above has already had its separate flush.
                    if let Err(error) = guard_play_work(
                        liveness.next_deadline(),
                        flush_writer(&mut writer, &mut stream, &compression, ctx.io_timeout),
                    )
                    .await
                    {
                        break Err(error);
                    }
                    observe_queue_len(&mut debug, ctx, &writer, budget.over_budget());
                }
                // The router dropped the session.
                None => break Ok(()),
            },
            PlayWake::Quota => {
                if let Err(err) = read_and_pump(
                    &mut decoder,
                    &compression,
                    ctx,
                    player,
                    name.as_str(),
                    &mut writer,
                    &mut stream,
                    &mut chunk_stream,
                    &mut chat_limiter,
                    &mut budget,
                    &mut inventory,
                    &mut window_state,
                    &mut player_yaw,
                    &mut player_pitch,
                    &mut debug,
                    &[],
                    &mut liveness,
                    &mut pending_teleport,
                    &mut ingress,
                    &mut handle,
                )
                .await
                {
                    break Err(err);
                }
            }
            PlayWake::Read(read) => {
                let outcome = match read {
                    Err(err) => break Err(err.into()),
                    Ok(0) => break Ok(()),
                    Ok(n) => {
                        read_and_pump(
                            &mut decoder,
                            &compression,
                            ctx,
                            player,
                            name.as_str(),
                            &mut writer,
                            &mut stream,
                            &mut chunk_stream,
                            &mut chat_limiter,
                            &mut budget,
                            &mut inventory,
                            &mut window_state,
                            &mut player_yaw,
                            &mut player_pitch,
                            &mut debug,
                            &read_buf[..n],
                            &mut liveness,
                            &mut pending_teleport,
                            &mut ingress,
                            &mut handle,
                        )
                        .await
                    }
                };
                if let Err(err) = outcome {
                    break Err(err);
                }
            }
        }
    };

    // Deadlines close the peer before persistence teardown, whose durability
    // awaits may legitimately take longer than a network response window.
    drop(stream);
    mirror_queued_for_leave(
        &mut handle,
        &mut ingress,
        &mut chunk_stream,
        &mut inventory,
        &mut player_yaw,
        &mut player_pitch,
        &mut pending_teleport,
    );

    if let Some(bridge) = ctx.java_bridge.as_ref().filter(|_| java_join_attempted) {
        match bridge
            .event(false, ctx.java_player(player, name.as_str()))
            .await
        {
            Ok(messages) => {
                for message in messages {
                    // The peer has already closed; quit callbacks run, but a message
                    // addressed to that departed player cannot be sent to a socket.
                    if message.player.is_none() {
                        tracing::info!(message = %message.text, "Java plugin console");
                    }
                }
            }
            Err(error) => {
                tracing::warn!(%error, player = name.as_str(), "Java quit callback failed");
            }
        }
    }

    // The play link has ended (clean close, EOF, shutdown, or error). Sample the
    // final outbound queue depth and dump the retained traces (acceptance:
    // disconnect).
    observe_queue_len(&mut debug, ctx, &writer, budget.over_budget());
    debug.dump("disconnect");

    // Cancel pending raw work before save/leave, even when the join kit failed.
    // Already running immutable baselines may finish into the bounded LRU.
    let _ = ctx
        .commands
        .send(SimCommand::CancelChunkPreparation { viewer: player })
        .await;

    // Persist this player's state BEFORE releasing their chunk tickets, mirroring
    // the chunk-edit flush-before-release discipline (`release_chunks_acked` in the
    // driver): the save is durable on the shared store the instant it returns (the
    // in-memory backend inserts inline; a redb backend commits), so a fast rejoin —
    // and the post-shutdown drain that waits on this connection's concurrency permit
    // — reads the just-saved state rather than a stale one. The UUID lease also
    // prevents a competing connection from reading while this save is pending.
    // The saved position is
    // the latest accepted client move or authoritative server teleport, falling
    // back to the join position if neither occurred. Failed saves quarantine this
    // UUID, but a connection ending must still run its remaining teardown.
    // Return the carried item even without an open screen; full-bag overflow
    // stays player-owned and is persisted alongside the inventory. The
    // client cleared its own cursor on disconnect, so no clientbound sync is needed.
    // Remove the producer before draining: FIFO acknowledgment proves no further
    // owner offer can enter this connection. Even a failed socket must apply committed
    // tool wear and return owned pickup remainders before its durable save.
    while window_state.consumption.pending.is_some() {
        if let Some(effect) = effect_rx.recv().await {
            if let Err(error) = handle_gameplay_effect(
                ctx,
                player,
                handle.entity_id(),
                &mut writer,
                &mut inventory,
                &mut window_state,
                effect,
                &mut debug,
                &compression,
            )
            .await
            {
                reservation.quarantine();
                tracing::warn!(%error, "pending food result failed during leave");
            }
        } else {
            reservation.quarantine();
            break;
        }
    }
    let (reply, unregistered) = oneshot::channel();
    match ctx
        .commands
        .send(SimCommand::UnregisterGameplay { player, reply })
        .await
    {
        Ok(()) => {
            if unregistered.await.is_err() {
                reservation.quarantine();
            }
        }
        Err(_) => reservation.quarantine(),
    }
    effect_rx.close();
    while let Some(effect) = effect_rx.recv().await {
        if let Err(error) = handle_gameplay_effect(
            ctx,
            player,
            handle.entity_id(),
            &mut writer,
            &mut inventory,
            &mut window_state,
            effect,
            &mut debug,
            &compression,
        )
        .await
        {
            reservation.quarantine();
            tracing::warn!(%error, %player, "could not apply committed gameplay effect during leave");
        }
    }
    window_state.take();
    inventory.return_carried();

    if chunk_stream.ticket_handoff_pending {
        reservation.quarantine();
        tracing::warn!(%player, name = name.as_str(), "unconfirmed join chunk-ticket ownership; UUID quarantined until server restart");
    }

    let save_position = chunk_stream.last_position().unwrap_or(position);
    let mut player_data = PlayerData::capture(save_position, player_yaw, player_pitch, &inventory);
    let (reply, vitals) = oneshot::channel();
    let captured = if ctx
        .commands
        .send(SimCommand::ReadVitals { player, reply })
        .await
        .is_ok()
    {
        match vitals.await {
            Ok(Some(snapshot)) => player_data.set_vitals(snapshot).is_ok(),
            _ => false,
        }
    } else {
        false
    };
    if !captured {
        reservation.quarantine();
        tracing::warn!(%player, "live vitals unavailable; preserving previous player record");
    }
    if captured {
        match player_data.to_record(inventory.game_mode()) {
            Ok(record) => {
                if let Err(err) = ctx.player_store.save_player(player, record).await {
                    reservation.quarantine();
                    tracing::warn!(%err, %player, name = name.as_str(), "failed to save player state on leave; UUID quarantined until server restart");
                }
            }
            Err(err) => {
                reservation.quarantine();
                tracing::warn!(%err, %player, name = name.as_str(), "failed to encode player state on leave; UUID quarantined until server restart");
            }
        }
    }

    // Release every chunk this connection had the client holding so its player
    // tickets stop pinning chunks resident after it leaves. Best-effort: a gone
    // driver just means the whole simulation is winding down anyway.
    if let Err(error) = release_owned_chunks(&ctx.commands, &chunk_stream.loaded).await {
        reservation.quarantine();
        tracing::warn!(%error, %player, name = name.as_str(), "failed to queue acquired chunk-ticket release; UUID quarantined until server restart");
    }

    // Preserve a typed peer-decode reason through the session leave event. Both
    // malformed and protocol-violation paths close immediately on the wire, but
    // routing them as `ServerShutdown` would erase the classification established
    // at the app boundary.
    let disconnect_reason = disconnect_reason_for_result(&result);

    // Enqueuing Leave is insufficient: the driver must remove the old binding
    // before a fresh connection can load and join with this UUID.
    if let Err(error) = acknowledge_leave(&ctx.commands, player, disconnect_reason).await {
        reservation.quarantine();
        tracing::warn!(%error, %player, name = name.as_str(), "failed to acknowledge player leave; UUID quarantined until server restart");
    }
    result
}

/// Releases only confirmed tickets; configured spawn columns alone confer no
/// ownership. FIFO command handling completes this release before Leave ack.
pub(super) async fn release_owned_chunks(
    commands: &tokio::sync::mpsc::Sender<SimCommand>,
    acquired: &std::collections::BTreeSet<ferrumc_math::ChunkPos>,
) -> anyhow::Result<()> {
    if acquired.is_empty() {
        return Ok(());
    }
    commands
        .send(SimCommand::ReleaseChunks {
            positions: acquired.iter().copied().collect(),
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!("simulation driver closed before acquired chunk-ticket release")
        })
}

/// Only classified admission refusals produce this small mandatory packet;
/// no storage read, simulation join or Java callback has happened at this point.
fn admission_refusal(error: AdmissionError) -> ClientboundPlayPacket {
    let mut reason = ferrumc_nbt::NbtCompound::new();
    reason.push("text", ferrumc_nbt::NbtTag::String(error.to_string()));
    ClientboundPlayPacket::PlayDisconnect(PlayDisconnect::new(ferrumc_nbt::NbtTag::Compound(
        reason,
    )))
}

/// Awaits the driver's Leave acceptance. An already absent player is safe too:
/// overload handling may have removed its binding before connection teardown.
async fn acknowledge_leave(
    commands: &tokio::sync::mpsc::Sender<SimCommand>,
    player: PlayerId,
    reason: DisconnectReason,
) -> anyhow::Result<()> {
    let (reply, acknowledgment) = oneshot::channel();
    commands
        .send(SimCommand::Event {
            event: NetEvent::disconnected(player, reason),
            acceptance: Some(reply),
        })
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver closed before player leave"))?;
    match acknowledgment
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver dropped player leave acknowledgment"))?
    {
        Ok(()) => Ok(()),
        Err(SessionError::UnknownPlayer { player: absent }) if absent == player => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Maps a typed decode or liveness failure to its session leave reason.
///
/// Unclassified exits keep the pre-existing cooperative `ServerShutdown` reason.
/// `anyhow` preserves concrete decode and liveness errors in its chain, retaining
/// their peer-failure classification through connection teardown.
fn disconnect_reason_for_result(result: &anyhow::Result<()>) -> DisconnectReason {
    let Err(error) = result else {
        return DisconnectReason::ServerShutdown;
    };
    if error.downcast_ref::<ServerboundBudgetExceeded>().is_some() {
        return DisconnectReason::BudgetExceeded;
    }
    if let Some(error) = error.downcast_ref::<DecodeError>() {
        return DisconnectReason::from_disconnect_class(error.disconnect_class());
    }
    if let Some(error) = error.downcast_ref::<FrameDecodeError>() {
        return DisconnectReason::from_disconnect_class(error.disconnect_class());
    }
    if let Some(error) = error.downcast_ref::<KeepAliveError>() {
        return error
            .disconnect_reason()
            .unwrap_or(DisconnectReason::ServerShutdown);
    }
    if let Some(error) = error.downcast_ref::<LivenessActivityError>() {
        return error
            .disconnect_reason()
            .unwrap_or(DisconnectReason::ServerShutdown);
    }
    if error.downcast_ref::<TeleportConfirmationError>().is_some() {
        return DisconnectReason::ProtocolViolation;
    }
    if let Some(error) = error.downcast_ref::<LivenessTimeout>() {
        return error.disconnect_reason();
    }
    DisconnectReason::ServerShutdown
}

/// Starts Play progress tracking with the configured I/O window and vanilla's
/// fifteen-second Keep Alive response limit. A shorter I/O window still wins.
pub(super) fn play_liveness(
    now: Instant,
    io_timeout: std::time::Duration,
) -> anyhow::Result<ConnectionLiveness> {
    let config = LivenessConfig::new(io_timeout, io_timeout, DEFAULT_KEEP_ALIVE_TIMEOUT);
    let mut liveness = ConnectionLiveness::new(now, config);
    liveness.enter_state(ConnectionState::Play, now)?;
    Ok(liveness)
}

/// Bounds cancellation-safe chunk or socket work by the currently active
/// absolute deadline. Capture this after packet handling refreshes progress;
/// inventory transactions are never interrupted by this guard.
pub(super) async fn guard_play_work<T>(
    deadline: Option<LivenessDeadline>,
    work: impl std::future::Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    let deadline = deadline.ok_or_else(|| anyhow::anyhow!("Play liveness is closed"))?;
    tokio::select! {
        biased;
        () = sleep_until(deadline.at()) => Err(deadline.timeout().into()),
        result = work => result,
    }
}

/// A selected work source; shutdown and deadlines have already won priority.
#[allow(clippy::large_enum_variant)] // bounded stack-local result; avoid per-packet allocation
enum PlayWake {
    Read(std::io::Result<usize>),
    Outbound(Option<OutboundMessage>),
    KeepAlive,
    Chunks,
    Operators(Result<(), watch::error::RecvError>),
    Quota,
}

/// Rotates the first-polled source after every selection. A continuously ready
/// outbound route therefore cannot starve a ready inbound Keep Alive echo, and
/// timers and operator changes remain serviceable under continuous traffic.
async fn next_play_wake<O, R, P>(
    next_work: &mut u8,
    keep_alive: &mut Interval,
    chunks: &mut Interval,
    outbound: O,
    read: R,
    operators: P,
    quota_ready: Option<Instant>,
) -> PlayWake
where
    O: std::future::Future<Output = Option<OutboundMessage>>,
    R: std::future::Future<Output = std::io::Result<usize>>,
    P: std::future::Future<Output = Result<(), watch::error::RecvError>>,
{
    let quota = async {
        match quota_ready {
            Some(ready) => sleep_until(ready).await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(outbound, read, operators, quota);
    let wake = match *next_work {
        0 => tokio::select! {
            biased;
            result = &mut read => PlayWake::Read(result),
            result = &mut outbound => PlayWake::Outbound(result),
            _ = keep_alive.tick() => PlayWake::KeepAlive,
            _ = chunks.tick() => PlayWake::Chunks,
            result = &mut operators => PlayWake::Operators(result),
            () = &mut quota => PlayWake::Quota,
        },
        1 => tokio::select! {
            biased;
            result = &mut outbound => PlayWake::Outbound(result),
            _ = keep_alive.tick() => PlayWake::KeepAlive,
            _ = chunks.tick() => PlayWake::Chunks,
            result = &mut operators => PlayWake::Operators(result),
            () = &mut quota => PlayWake::Quota,
            result = &mut read => PlayWake::Read(result),
        },
        2 => tokio::select! {
            biased;
            _ = keep_alive.tick() => PlayWake::KeepAlive,
            _ = chunks.tick() => PlayWake::Chunks,
            result = &mut operators => PlayWake::Operators(result),
            () = &mut quota => PlayWake::Quota,
            result = &mut read => PlayWake::Read(result),
            result = &mut outbound => PlayWake::Outbound(result),
        },
        3 => tokio::select! {
            biased;
            _ = chunks.tick() => PlayWake::Chunks,
            result = &mut operators => PlayWake::Operators(result),
            () = &mut quota => PlayWake::Quota,
            result = &mut read => PlayWake::Read(result),
            result = &mut outbound => PlayWake::Outbound(result),
            _ = keep_alive.tick() => PlayWake::KeepAlive,
        },
        4 => tokio::select! {
            biased;
            result = &mut operators => PlayWake::Operators(result),
            () = &mut quota => PlayWake::Quota,
            result = &mut read => PlayWake::Read(result),
            result = &mut outbound => PlayWake::Outbound(result),
            _ = keep_alive.tick() => PlayWake::KeepAlive,
            _ = chunks.tick() => PlayWake::Chunks,
        },
        _ => tokio::select! {
            biased;
            () = &mut quota => PlayWake::Quota,
            result = &mut read => PlayWake::Read(result),
            result = &mut outbound => PlayWake::Outbound(result),
            _ = keep_alive.tick() => PlayWake::KeepAlive,
            _ = chunks.tick() => PlayWake::Chunks,
            result = &mut operators => PlayWake::Operators(result),
        },
    };
    *next_work = match wake {
        PlayWake::Read(_) => 1,
        PlayWake::Outbound(_) => 2,
        PlayWake::KeepAlive => 3,
        PlayWake::Chunks => 4,
        PlayWake::Operators(_) => 5,
        PlayWake::Quota => 0,
    };
    wake
}

/// Sends a join request to the driver and awaits the session handle.
///
/// `equipment` is the joiner's pre-encoded main-hand `SetEquipment` body, cached
/// by the router at join so viewers entering view see the held item immediately.
async fn join_simulation(
    ctx: &ConnContext,
    player: PlayerId,
    name: &str,
    position: Vec3,
    equipment: Vec<u8>,
) -> anyhow::Result<PlayerSessionHandle> {
    let (reply_tx, reply_rx) = oneshot::channel();
    ctx.commands
        .send(SimCommand::Join {
            player,
            name: name.to_owned(),
            position,
            equipment,
            reply: reply_tx,
        })
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
    reply_rx
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver dropped the join reply"))?
        .map_err(|err| anyhow::anyhow!("join rejected: {err}"))
}

/// Pushes freshly read bytes through the decoder, handles every complete play
/// frame, then flushes any clientbound responses queued while handling them.
///
/// Movement only replaces a coalesced pending stream target here. Chunk
/// load/unload work runs on the fixed stream interval, so attacker-controlled TCP
/// read boundaries cannot multiply driver/store work.
#[allow(clippy::too_many_arguments)] // one play step: framing + policy + I/O + trace state
async fn read_and_pump(
    decoder: &mut InboundDecoder,
    compression: &CompressionState,
    ctx: &ConnContext,
    player: PlayerId,
    name: &str,
    writer: &mut PlayWriter,
    stream: &mut TcpStream,
    chunk_stream: &mut ChunkStream,
    chat_limiter: &mut ChatRateLimiter,
    budget: &mut ServerboundBudget,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    player_yaw: &mut f32,
    player_pitch: &mut f32,
    debug: &mut SessionDebug,
    bytes: &[u8],
    liveness: &mut ConnectionLiveness,
    pending_teleport: &mut PendingTeleport,
    ingress: &mut IngressFence,
    handle: &mut PlayerSessionHandle,
) -> anyhow::Result<()> {
    decoder.push(bytes)?;
    pump_serverbound(
        decoder,
        compression,
        ctx,
        player,
        name,
        writer,
        chunk_stream,
        chat_limiter,
        budget,
        inventory,
        window_state,
        player_yaw,
        player_pitch,
        debug,
        liveness,
        pending_teleport,
        ingress,
        handle,
    )
    .await?;
    guard_play_work(
        liveness.next_deadline(),
        flush_writer(writer, stream, compression, ctx.io_timeout),
    )
    .await
}

/// Drains every buffered serverbound play frame and handles each: a
/// `ChatCommand` runs through the command tree (queuing any clientbound response
/// into `writer`), a spawn-protected break/place is vetoed, and anything else is
/// forwarded to the simulation as a [`NetEvent`].
///
/// Valid movement is admitted before it replaces the connection's one pending
/// stream target. The fixed-cadence chunk timer consumes that latest target;
/// this drain never performs chunk work itself.
#[allow(clippy::too_many_arguments)] // one play drain: framing + policy + trace state
pub(super) async fn pump_serverbound(
    decoder: &mut InboundDecoder,
    compression: &CompressionState,
    ctx: &ConnContext,
    player: PlayerId,
    name: &str,
    writer: &mut PlayWriter,
    chunk_stream: &mut ChunkStream,
    chat_limiter: &mut ChatRateLimiter,
    budget: &mut ServerboundBudget,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    player_yaw: &mut f32,
    player_pitch: &mut f32,
    debug: &mut SessionDebug,
    liveness: &mut ConnectionLiveness,
    pending_teleport: &mut PendingTeleport,
    ingress: &mut IngressFence,
    handle: &mut PlayerSessionHandle,
) -> anyhow::Result<()> {
    // Refresh actual admission time per attempt; Tokio clock supports paused tests.
    loop {
        // A previous callback may have consumed the deadline; even an
        // unmodelled buffered packet must not postpone its enforcement.
        let deadline = liveness
            .next_deadline()
            .ok_or_else(|| anyhow::anyhow!("Play liveness is closed"))?;
        if Instant::now() >= deadline.at() {
            return Err(deadline.timeout().into());
        }
        if ingress.is_fenced()
            || ingress
                .quota_ready()
                .is_some_and(|ready| Instant::now() < ready)
        {
            return Ok(());
        }
        let quota_retry = ingress.quota_ready().is_some();
        let body = if let Some(body) = ingress.take_ready_body() {
            body
        } else {
            let next = match decoder.next_packet_compressed(ConnectionState::Play, compression) {
                Ok(next) => next,
                Err(err) => {
                    ctx.metrics.record_packet_decode_error(
                        PacketState::Play,
                        observe::decode_error_label(&err),
                    );
                    debug.dump("play_decode_error");
                    return Err(err.into());
                }
            };
            let Some(packet) = next else { break };
            let InboundPacket::Play(body) = packet else {
                anyhow::bail!("non-play frame received in the play phase");
            };
            if is_exact_keep_alive(&body) {
                body
            } else {
                ingress.hold(body, handle);
                let Some(body) = ingress.take_ready_body() else {
                    return Ok(());
                };
                body
            }
        };
        match budget.admit_or_defer(Instant::now().into_std(), quota_retry) {
            Ok(BudgetAdmission::Admitted) => {}
            Ok(BudgetAdmission::Deferred(ready)) => {
                ingress.defer_for_quota(body, Instant::from_std(ready));
                return Ok(());
            }
            Err(reason) => {
                tracing::warn!(
                    player = name,
                    ?reason,
                    "serverbound packet budget exceeded; dropping connection"
                );
                debug.dump("play_budget_exceeded");
                return Err(ServerboundBudgetExceeded.into());
            }
        }
        handle_play_body(
            ctx,
            player,
            name,
            writer,
            chunk_stream,
            chat_limiter,
            inventory,
            window_state,
            player_yaw,
            player_pitch,
            &body,
            debug,
            compression,
            liveness,
            pending_teleport,
        )
        .await?;
    }
    // A frozen prefix may have returned before this point. Keep every initial
    // buffered frame (including a partial tail) strict until it actually drains.
    if decoder.buffered_len() == 0 {
        budget.establish_play();
    }
    Ok(())
}

/// An accepted teleport stays authoritative when EOF/stop/deadline wins before
/// its socket turn. Retained and channel messages keep their bounded FIFO order.
#[allow(clippy::too_many_arguments)] // one cohesive save candidate across connection mirrors
pub(super) fn mirror_queued_for_leave(
    handle: &mut PlayerSessionHandle,
    ingress: &mut IngressFence,
    chunk_stream: &mut ChunkStream,
    inventory: &mut PlayerInventory,
    yaw: &mut f32,
    pitch: &mut f32,
    pending_teleport: &mut PendingTeleport,
) {
    while let Some(message) = ingress.pop_outbound() {
        mirror_outbound(
            message.into_packet(),
            chunk_stream,
            inventory,
            yaw,
            pitch,
            pending_teleport,
        );
    }
    for _ in 0..ferrumc_session::DEFAULT_OUTBOUND_CAPACITY {
        let Some(message) = handle.try_recv() else {
            break;
        };
        mirror_outbound(
            message.into_packet(),
            chunk_stream,
            inventory,
            yaw,
            pitch,
            pending_teleport,
        );
    }
}

/// Stamps teleports at the sole connection-owned ID authority, then mirrors
/// already accepted changes independently of whether the socket can flush.
fn mirror_outbound(
    mut packet: ClientboundPlayPacket,
    chunk_stream: &mut ChunkStream,
    inventory: &mut PlayerInventory,
    yaw: &mut f32,
    pitch: &mut f32,
    pending_teleport: &mut PendingTeleport,
) -> ClientboundPlayPacket {
    if let ClientboundPlayPacket::SynchronizePlayerPosition(sync) = &mut packet {
        *sync = pending_teleport.issue(sync);
        mirror_server_teleport(chunk_stream, yaw, pitch, sync);
    }
    if let ClientboundPlayPacket::GameEvent(event) = &packet {
        mirror_game_mode_change(event, inventory);
    }
    packet
}

/// One FIFO output per socket flush keeps router resync-before-ack ordering.
#[allow(clippy::too_many_arguments)] // cohesive output admission and connection mirrors
fn queue_outbound(
    message: OutboundMessage,
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    chunk_stream: &mut ChunkStream,
    inventory: &mut PlayerInventory,
    yaw: &mut f32,
    pitch: &mut f32,
    pending_teleport: &mut PendingTeleport,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    let criticality = message.criticality();
    let priority = message.priority();
    let packet = mirror_outbound(
        message.into_packet(),
        chunk_stream,
        inventory,
        yaw,
        pitch,
        pending_teleport,
    );
    let outcome = enqueue_traced(writer, debug, compression, &ctx.clock, priority, packet);
    anyhow::ensure!(
        !is_mandatory_overflow(criticality, outcome),
        "outbound overflow: a mandatory clientbound packet was dropped at the connection writer"
    );
    Ok(())
}

/// Mirrors a server-driven game-mode change into the connection-local game-mode.
///
/// A targeted `/gamemode <mode> <player>` (and the self-form aimed at one's own
/// name) switches the target by routing a `change_game_mode` [`GameEvent`] through
/// the session channel — unlike the self-form's inline switch, this never touches
/// this task's mirror on its own. The creative-slot gate and the leave-save both
/// read that mirror, so keep it in lockstep with the sim's authoritative mode
/// here; otherwise a targeted change would block the target from authoring its
/// creative inventory and be lost on relog. A non `change_game_mode` event, or one
/// carrying an unknown mode id, is ignored.
fn mirror_game_mode_change(event: &GameEvent, inventory: &mut PlayerInventory) {
    if event.reason() != GAME_EVENT_CHANGE_GAMEMODE {
        return;
    }
    if let Some(mode) = GameMode::from_id(event.value() as u8) {
        inventory.set_game_mode(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refusals_have_distinct_stable_standard_protocol_reasons() {
        let mut seen = std::collections::BTreeSet::new();
        for error in [
            AdmissionError::AlreadyReserved,
            AdmissionError::Quarantined,
            AdmissionError::AtCapacity,
        ] {
            let packet = admission_refusal(error);
            assert_eq!(Criticality::for_packet(&packet), Criticality::Mandatory);
            let ClientboundPlayPacket::PlayDisconnect(refusal) = packet else {
                panic!("standard Play Disconnect");
            };
            let ferrumc_nbt::NbtTag::Compound(reason) = refusal.reason() else {
                panic!("text component root");
            };
            assert_eq!(
                reason.get("text"),
                Some(&ferrumc_nbt::NbtTag::String(error.to_string()))
            );
            assert!(seen.insert(error.to_string()));
        }
        assert_eq!(
            AdmissionError::AlreadyReserved.to_string(),
            "player UUID is already active or finishing leave"
        );
    }

    #[tokio::test]
    async fn leave_keeps_uuid_reserved_until_the_actual_driver_acknowledgment() {
        use crate::player_admission::{AdmissionError, PlayerAdmissions};
        let player = PlayerId::offline("LeaveFence");
        let registry = PlayerAdmissions::new(1);
        let reservation = registry.reserve(player).expect("claim");
        let (commands, mut inbox) = tokio::sync::mpsc::channel(1);
        let leaving = tokio::spawn(async move {
            acknowledge_leave(&commands, player, DisconnectReason::ServerShutdown)
                .await
                .expect("leave acknowledgment");
            drop(reservation);
        });
        let SimCommand::Event {
            event:
                NetEvent::Disconnected {
                    player: leaving_player,
                    ..
                },
            acceptance: Some(reply),
        } = inbox.recv().await.expect("Leave queued")
        else {
            panic!("typed leave must request an acknowledgment");
        };
        assert_eq!(leaving_player, player);
        assert_eq!(
            registry
                .reserve(player)
                .expect_err("old driver mapping still exists"),
            AdmissionError::AlreadyReserved
        );
        assert!(
            !leaving.is_finished(),
            "queue admission alone must not release UUID"
        );
        reply.send(Ok(())).expect("driver removes mapping");
        leaving.await.expect("cleanup returns");
        assert!(registry.reserve(player).is_ok());
    }

    #[tokio::test]
    async fn leave_accepts_only_the_exact_already_absent_identity_or_success() {
        for outcome in [
            Err(SessionError::UnknownPlayer {
                player: PlayerId::offline("Absent"),
            }),
            Err(SessionError::UnknownPlayer {
                player: PlayerId::offline("Different"),
            }),
            Err(SessionError::ShardInboxFull {
                shard: ferrumc_math::ShardPos::new(0, 0),
            }),
        ] {
            let player = PlayerId::offline("Absent");
            let accepted_absent = matches!(&outcome, Err(SessionError::UnknownPlayer { player: absent }) if *absent == player);
            let (commands, mut inbox) = tokio::sync::mpsc::channel(1);
            let leaving = tokio::spawn(async move {
                acknowledge_leave(&commands, player, DisconnectReason::ServerShutdown).await
            });
            let SimCommand::Event {
                acceptance: Some(reply),
                ..
            } = inbox.recv().await.expect("Leave")
            else {
                panic!("typed ack");
            };
            reply.send(outcome).expect("reply");
            assert_eq!(leaving.await.expect("task").is_ok(), accepted_absent);
        }
        let (commands, inbox) = tokio::sync::mpsc::channel(1);
        drop(inbox);
        assert!(acknowledge_leave(
            &commands,
            PlayerId::offline("GoneDriver"),
            DisconnectReason::ServerShutdown
        )
        .await
        .is_err());
        let (commands, mut inbox) = tokio::sync::mpsc::channel(1);
        let leaving = tokio::spawn(async move {
            acknowledge_leave(
                &commands,
                PlayerId::offline("GoneReply"),
                DisconnectReason::ServerShutdown,
            )
            .await
        });
        drop(inbox.recv().await.expect("queued Leave loses reply sender"));
        assert!(leaving.await.expect("task").is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn ready_outbound_work_cannot_starve_ready_inbound_echoes_or_timers() {
        use std::future::{pending, ready};
        use std::time::Duration;

        let mut liveness =
            play_liveness(Instant::now(), Duration::from_secs(30)).expect("Play tracker");
        liveness
            .keep_alive_sent(7, Instant::now())
            .expect("request sent");
        let mut keep_alive = interval_at(Instant::now(), Duration::from_secs(1));
        let mut chunks = interval_at(Instant::now(), Duration::from_secs(1));
        let mut next_work = 1; // An outbound burst starts ahead of the echo.
        let mut choices = Vec::new();
        for _ in 0..4 {
            match next_play_wake(
                &mut next_work,
                &mut keep_alive,
                &mut chunks,
                ready(None),
                ready(Ok(9)),
                pending(),
                None,
            )
            .await
            {
                PlayWake::Outbound(_) => choices.push("outbound"),
                PlayWake::KeepAlive => choices.push("keep-alive"),
                PlayWake::Chunks => choices.push("chunks"),
                PlayWake::Operators(_) => panic!("no operator changes"),
                PlayWake::Quota => panic!("no quota wait"),
                PlayWake::Read(_) => {
                    liveness
                        .keep_alive_received(7, Instant::now())
                        .expect("echo serviced");
                    choices.push("read");
                }
            }
        }
        assert_eq!(choices, ["outbound", "keep-alive", "chunks", "read"]);
        assert_eq!(liveness.outstanding_keep_alive(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn continuous_operator_changes_cannot_starve_ready_echoes_or_other_work() {
        use std::future::ready;
        use std::time::Duration;

        let mut liveness =
            play_liveness(Instant::now(), Duration::from_secs(30)).expect("Play tracker");
        liveness.keep_alive_sent(7, Instant::now()).expect("sent");
        let mut keep_alive = interval_at(Instant::now(), Duration::from_secs(1));
        let mut chunks = interval_at(Instant::now(), Duration::from_secs(1));
        let (revisions, mut changes) = watch::channel(0_u64);
        let mut next_work = 4;
        let mut choices = Vec::new();
        for _ in 0..5 {
            revisions.send_modify(|revision| *revision += 1);
            match next_play_wake(
                &mut next_work,
                &mut keep_alive,
                &mut chunks,
                ready(None),
                ready(Ok(9)),
                changes.changed(),
                None,
            )
            .await
            {
                PlayWake::Operators(Ok(())) => choices.push("operators"),
                PlayWake::Operators(Err(_)) => panic!("revision sender live"),
                PlayWake::Quota => panic!("no quota wait"),
                PlayWake::Read(_) => {
                    liveness
                        .keep_alive_received(7, Instant::now())
                        .expect("echo serviced");
                    choices.push("read");
                }
                PlayWake::Outbound(_) => choices.push("outbound"),
                PlayWake::KeepAlive => choices.push("keep-alive"),
                PlayWake::Chunks => choices.push("chunks"),
            }
        }
        assert_eq!(
            choices,
            ["operators", "read", "outbound", "keep-alive", "chunks"]
        );
        assert_eq!(liveness.outstanding_keep_alive(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn quota_wake_rotates_with_all_ready_sources_without_starvation() {
        use std::future::ready;
        use std::time::Duration;
        let mut keep_alive = interval_at(Instant::now(), Duration::from_secs(1));
        let mut chunks = interval_at(Instant::now(), Duration::from_secs(1));
        let mut next = 1;
        let mut choices = Vec::new();
        for _ in 0..6 {
            let wake = next_play_wake(
                &mut next,
                &mut keep_alive,
                &mut chunks,
                ready(None),
                ready(Ok(1)),
                ready(Ok(())),
                Some(Instant::now()),
            )
            .await;
            choices.push(match wake {
                PlayWake::Outbound(_) => "outbound",
                PlayWake::KeepAlive => "keep-alive",
                PlayWake::Chunks => "chunks",
                PlayWake::Operators(_) => "operators",
                PlayWake::Quota => "quota",
                PlayWake::Read(_) => "read",
            });
        }
        assert_eq!(
            choices,
            [
                "outbound",
                "keep-alive",
                "chunks",
                "operators",
                "quota",
                "read"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn continuous_outbound_cannot_starve_a_due_quota_retry() {
        use std::future::{pending, ready};
        use std::time::Duration;
        let mut keep_alive = interval_at(
            Instant::now() + Duration::from_secs(10),
            Duration::from_secs(10),
        );
        let mut chunks = interval_at(
            Instant::now() + Duration::from_secs(10),
            Duration::from_secs(10),
        );
        let mut next = 1;
        assert!(matches!(
            next_play_wake(
                &mut next,
                &mut keep_alive,
                &mut chunks,
                ready(None),
                pending(),
                pending(),
                Some(Instant::now())
            )
            .await,
            PlayWake::Outbound(_)
        ));
        assert!(matches!(
            next_play_wake(
                &mut next,
                &mut keep_alive,
                &mut chunks,
                ready(None),
                pending(),
                pending(),
                Some(Instant::now())
            )
            .await,
            PlayWake::Quota
        ));
    }
    #[tokio::test(start_paused = true)]
    async fn held_quota_does_not_renew_absolute_progress_deadline() {
        use std::future::pending;
        use std::time::Duration;
        let liveness = play_liveness(Instant::now(), Duration::from_secs(1)).expect("tracker");
        let mut keep_alive = interval_at(
            Instant::now() + Duration::from_secs(10),
            Duration::from_secs(10),
        );
        let mut chunks = interval_at(
            Instant::now() + Duration::from_secs(10),
            Duration::from_secs(10),
        );
        let mut next = 5;
        let result = guard_play_work(liveness.next_deadline(), async {
            Ok(next_play_wake(
                &mut next,
                &mut keep_alive,
                &mut chunks,
                pending(),
                pending(),
                pending(),
                Some(Instant::now() + Duration::from_secs(2)),
            )
            .await)
        })
        .await;
        assert!(result
            .err()
            .expect("unchanged one-second deadline wins")
            .downcast_ref::<LivenessTimeout>()
            .is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn active_response_deadline_interrupts_stalled_cancellation_safe_work() {
        use std::future::poll_fn;
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        use std::task::Poll;
        use std::time::Duration;
        use tokio::time::advance;

        let mut liveness =
            play_liveness(Instant::now(), Duration::from_secs(30)).expect("Play tracker");
        liveness
            .keep_alive_sent(7, Instant::now())
            .expect("request sent");
        let started = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&started);
        let stalled = tokio::spawn(guard_play_work(
            liveness.next_deadline(),
            poll_fn(move |_| {
                observed.store(true, Ordering::Relaxed);
                Poll::<anyhow::Result<()>>::Pending
            }),
        ));
        tokio::task::yield_now().await;
        assert!(
            started.load(Ordering::Relaxed),
            "fixture actually started work"
        );
        advance(Duration::from_secs(14)).await;
        assert!(!stalled.is_finished());
        advance(Duration::from_secs(1)).await;
        let error = stalled
            .await
            .expect("guard task returned")
            .expect_err("stalled work timed out");
        assert_eq!(
            error.downcast_ref(),
            Some(&LivenessTimeout::KeepAlive { id: 7 })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn work_after_valid_activity_uses_the_refreshed_progress_deadline() {
        use std::future::ready;
        use std::time::Duration;
        use tokio::time::advance;

        let mut liveness =
            play_liveness(Instant::now(), Duration::from_secs(5)).expect("Play tracker");
        advance(Duration::from_secs(4)).await;
        liveness
            .valid_packet_observed(Instant::now())
            .expect("valid packet refreshes progress");
        advance(Duration::from_secs(2)).await;
        guard_play_work(liveness.next_deadline(), ready(Ok(())))
            .await
            .expect("the expired pre-packet deadline is not reused");
    }

    #[test]
    fn typed_decode_failures_reach_the_session_leave_reason() {
        let malformed: anyhow::Result<()> = Err(DecodeError::MalformedBody {
            state: ConnectionState::Play,
        }
        .into());
        assert_eq!(
            disconnect_reason_for_result(&malformed),
            DisconnectReason::MalformedPacket,
        );

        let trailing: anyhow::Result<()> = Err(DecodeError::TrailingBytes {
            state: ConnectionState::Play,
            trailing: 2,
        }
        .into());
        assert_eq!(
            disconnect_reason_for_result(&trailing),
            DisconnectReason::ProtocolViolation,
        );

        let oversized: anyhow::Result<()> =
            Err(FrameDecodeError::Decode(DecodeError::FrameTooLarge {
                state: ConnectionState::Play,
                length: 2,
                max: 1,
            })
            .into());
        assert_eq!(
            disconnect_reason_for_result(&oversized),
            DisconnectReason::FrameTooLarge,
        );
    }

    #[test]
    fn non_decode_exit_keeps_the_cooperative_leave_reason() {
        let result: anyhow::Result<()> = Err(anyhow::anyhow!("socket closed"));
        assert_eq!(
            disconnect_reason_for_result(&result),
            DisconnectReason::ServerShutdown,
        );
        assert_eq!(
            disconnect_reason_for_result(&Ok(())),
            DisconnectReason::ServerShutdown,
        );
    }

    #[test]
    fn liveness_failures_retain_their_session_leave_reason() {
        for (error, reason) in [
            (
                anyhow::Error::new(KeepAliveError::Mismatched {
                    expected: 1,
                    received: 2,
                }),
                DisconnectReason::ProtocolViolation,
            ),
            (
                anyhow::Error::new(KeepAliveError::Unexpected { received: 1 }),
                DisconnectReason::ProtocolViolation,
            ),
            (
                anyhow::Error::new(LivenessTimeout::KeepAlive { id: 1 }),
                DisconnectReason::KeepAliveTimeout,
            ),
            (
                anyhow::Error::new(LivenessActivityError::TimedOut(
                    LivenessTimeout::StateProgress {
                        state: ConnectionState::Play,
                    },
                )),
                DisconnectReason::ProgressTimeout,
            ),
        ] {
            assert_eq!(disconnect_reason_for_result(&Err(error)), reason);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn configured_io_window_is_an_absolute_play_progress_limit() {
        use std::time::Duration;
        use tokio::time::advance;

        let mut liveness =
            play_liveness(Instant::now(), Duration::from_secs(1)).expect("Play tracker");
        liveness
            .keep_alive_sent(1, Instant::now())
            .expect("request sent");
        advance(Duration::from_secs(1)).await;
        assert_eq!(
            liveness.poll_timeout(Instant::now()),
            Some(LivenessTimeout::StateProgress {
                state: ConnectionState::Play
            })
        );
    }

    /// A routed `change_game_mode` event updates the connection-local mode that
    /// both the creative-slot gate (`inventory.game_mode() == Creative`) and the
    /// leave-save (`to_record(inventory.game_mode())`) read, so a targeted
    /// `/gamemode creative <player>` both unblocks creative authoring in-session
    /// and persists across relog.
    #[test]
    fn change_game_mode_event_updates_the_connection_mirror() {
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Survival);
        mirror_game_mode_change(
            &GameEvent::new(
                GAME_EVENT_CHANGE_GAMEMODE,
                f32::from(GameMode::Creative.as_id()),
            ),
            &mut inventory,
        );
        assert_eq!(inventory.game_mode(), GameMode::Creative);
    }

    /// A non `change_game_mode` event (e.g. start-raining) never touches the mode.
    #[test]
    fn unrelated_game_event_leaves_the_mode_untouched() {
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        mirror_game_mode_change(&GameEvent::new(1, 0.0), &mut inventory);
        assert_eq!(inventory.game_mode(), GameMode::Creative);
    }

    /// A `change_game_mode` event carrying an unknown mode id is ignored rather
    /// than corrupting the mirror.
    #[test]
    fn change_game_mode_event_with_unknown_id_is_ignored() {
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Adventure);
        mirror_game_mode_change(
            &GameEvent::new(GAME_EVENT_CHANGE_GAMEMODE, 9.0),
            &mut inventory,
        );
        assert_eq!(inventory.game_mode(), GameMode::Adventure);
    }
}
