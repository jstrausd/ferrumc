//! Per-packet serverbound play handlers: chat/commands, inventory, the block
//! break/place plugin-intent boundary and its single rejection funnel, and tab
//! completion.

use ferrumc_codec::{BoundedReader, BoundedString};
use ferrumc_command::{CommandSource, CommandTree};
use ferrumc_core::{GameMode, PlayerId, TextColor, TextComponent};
use ferrumc_items::{ItemStack, UntrustedItemStack};
use ferrumc_math::{BlockPos, Direction, Vec3, WorldIntent};
use ferrumc_net::{CompressionState, ConnectionLiveness, ConnectionState, DecodeError, PlayWriter};
use ferrumc_observability::{PacketState, SessionDebug};
use ferrumc_plugin_api::{
    BlockBreakAttempt, BlockPlaceAttempt, ChatAttempt, InteractAttempt, InteractHand,
    InteractTarget, PluginEventDecision,
};
use ferrumc_plugin_host::ResolvedDecision;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, CloseContainer, CommandSuggestionMatch, GameEvent,
    ServerboundPlayPacket, ServerboundSetHeldItem, SetContainerContent, SetContainerSlot,
    SetCreativeSlot, SetPlayerPosition, SetPlayerRotation, TabCompleteResponse, UpdateSign,
    UseItemOn, WindowClick,
};
use ferrumc_proto::ProtoError;
use ferrumc_session::{
    net_event_to_input, open_screen, use_item_on_block, use_item_on_face, use_item_on_target,
    NetEvent,
};
use ferrumc_sim::{BlockStateId, GameInput};
use tokio::sync::oneshot;

use crate::command::{parse_gamemode, GAMEMODE_COMMAND, SPAWN_COMMAND};
use crate::driver::SimCommand;
use crate::inventory::{PlayerInventory, SLOT_COUNT, WINDOW_ID};
use crate::observe;
use crate::player_data::is_valid_player_position;
use crate::plugins::PermissionFacade;
use crate::window::{OpenContainer, WindowSlot, WindowState, GENERIC_9X3_TYPE};

use super::chunk_stream::{mirror_server_teleport, ChunkStream};
use super::context::ConnContext;
use super::outbound::{ack_sequence, enqueue_traced_classified, send_mandatory};
use super::rate_limiter::ChatRateLimiter;
use super::teleport::PendingTeleport;
use super::{send_sim_command_accepted, spawn_sync, GAME_EVENT_CHANGE_GAMEMODE};

/// Last serverbound Play packet id in protocol 772.
///
/// The protocol's ids are contiguous from `0x00` through `0x41`. `FerrumC` models
/// only the subset it handles, so an in-range generated-enum miss is compatible
/// unmodelled traffic; an id outside this range is a protocol violation.
const MAX_PROTOCOL_772_SERVERBOUND_PLAY_ID: i32 = 0x41;

/// One decoded frame body plus its validated nested creative-item payload.
struct DecodedPlayBody {
    packet: ServerboundPlayPacket,
    creative_item: Option<ItemStack>,
}

/// Result of classifying a complete protocol-772 Play frame body.
enum PlayBodyDecode {
    /// A packet `FerrumC` models and can dispatch.
    Modelled(DecodedPlayBody),
    /// A protocol-valid id omitted from `FerrumC`'s deliberately partial enum.
    Unmodelled { id: i32 },
}

/// Handles one decoded serverbound play-frame body.
///
/// Protocol-valid but unmodelled packets are ignored because the generated slice
/// intentionally covers only what `FerrumC` handles. A malformed modelled packet,
/// an impossible protocol-772 id, or trailing outer/nested bytes is a classified
/// fatal decode error. Keep Alive echoes must match the one outstanding request
/// before its deadline; teleport confirmations release only the newest issued ID.
/// A `ChatCommand` is dispatched locally; every other
/// modelled packet is forwarded to the simulation unless policy vetoes it.
/// Movement is validated atomically and admitted to the bounded shard route
/// before it may update any connection-local mirror.
#[allow(clippy::too_many_arguments)] // one play step: framing + policy + trace state
#[allow(clippy::too_many_lines)] // one dispatch: chat, inventory, place, movement fallthrough
pub(super) async fn handle_play_body(
    ctx: &ConnContext,
    player: PlayerId,
    name: &str,
    writer: &mut PlayWriter,
    chunk_stream: &mut ChunkStream,
    chat_limiter: &mut ChatRateLimiter,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    player_yaw: &mut f32,
    player_pitch: &mut f32,
    body: &[u8],
    debug: &mut SessionDebug,
    compression: &CompressionState,
    liveness: &mut ConnectionLiveness,
    pending_teleport: &mut PendingTeleport,
) -> anyhow::Result<()> {
    let decoded = match decode_play_body(body) {
        Ok(PlayBodyDecode::Modelled(decoded)) => decoded,
        Ok(PlayBodyDecode::Unmodelled { id }) => {
            // A conforming 1.21.8 client can send packets the deliberately partial
            // generated enum does not model. The complete raw frame has already
            // been consumed, so skip only this body and keep the next frame intact.
            ctx.metrics
                .record_packet_decode_error(PacketState::Play, "unknown_play");
            tracing::trace!(
                packet_id = id,
                "ignoring unmodelled protocol-772 Play packet"
            );
            return Ok(());
        }
        Err(err) => {
            ctx.metrics.record_packet_decode_error(
                PacketState::Play,
                observe::body_decode_error_label(&err),
            );
            debug.dump("play_packet_decode_error");
            return Err(err.into());
        }
    };
    let packet = decoded.packet;
    let creative_item = decoded.creative_item;

    // A movement packet is one atomic hostile observation. Reject it before even
    // tracing it as accepted when any carried coordinate or look component is
    // unsafe: a valid position paired with NaN yaw (or an invalid position paired
    // with finite look) must not partially update a mirror, call a plugin, route
    // to the shard, or trigger chunk work.
    let movement = match validated_movement(&packet) {
        MovementValidation::NotMovement => None,
        MovementValidation::Valid(movement) => Some(movement),
        MovementValidation::Invalid => return Ok(()),
    };

    // Validate liveness only after strict body and movement validation, before any
    // observable packet effect. Ordinary traffic cannot satisfy a Keep Alive.
    let now = tokio::time::Instant::now();
    if let ServerboundPlayPacket::ServerboundKeepAlive(keep_alive) = &packet {
        liveness.keep_alive_received(keep_alive.keep_alive_id(), now)?;
    } else {
        liveness.valid_packet_observed(now)?;
    }

    // Record the inbound play trace with the exact frame-body size.
    debug.record_inbound(observe::trace_inbound_play(
        &packet,
        body.len(),
        compression,
        &ctx.clock,
    ));

    if let ServerboundPlayPacket::ConfirmTeleportation(confirmation) = &packet {
        pending_teleport.confirm(confirmation.teleport_id())?;
        return Ok(());
    }

    if let Some(movement) = movement {
        // Paper permits looking around while a teleport is pending, but neither
        // its positional component nor plugin movement/stream effects run.
        if pending_teleport.is_pending() {
            if let (Some(yaw), Some(pitch)) = (movement.yaw, movement.pitch) {
                send_sim_command_accepted(
                    ctx,
                    SimCommand::Event {
                        event: NetEvent::play(
                            player,
                            ServerboundPlayPacket::SetPlayerRotation(SetPlayerRotation::new(
                                yaw, pitch, 0,
                            )),
                        ),
                        acceptance: None,
                    },
                )
                .await?;
                *player_yaw = yaw;
                *player_pitch = pitch;
            }
            return Ok(());
        }
        // The router must own the exact movement before any connection-local
        // state advances. A full/closed shard route returns a classified failure
        // (and may terminate the overloaded session), leaving the stream centre,
        // plugin baseline, and leave-save candidate untouched.
        send_sim_command_accepted(
            ctx,
            SimCommand::Event {
                event: NetEvent::play(player, packet),
                acceptance: None,
            },
        )
        .await?;

        if let Some(position) = movement.position {
            chunk_stream.observe(position);
            // Fire the observe-only PlayerMove plugin event only for a validated,
            // admitted move, throttled to block granularity. Movement cannot be
            // vetoed through this surface; only emitted intents are routed.
            if let Some((from, to)) = chunk_stream.block_crossing(position) {
                let perms = PermissionFacade::new(ctx.policy.permissions());
                let intents =
                    ctx.block_events
                        .player_move(player, from, to, Some(position), &perms);
                route_emitted_intents(ctx, player, writer, 0, intents, debug, compression).await?;
            }
        }
        if let Some(yaw) = movement.yaw {
            *player_yaw = yaw;
        }
        if let Some(pitch) = movement.pitch {
            *player_pitch = pitch;
        }
        return Ok(());
    }

    match &packet {
        ServerboundPlayPacket::ChatCommand(command) => {
            let command = command.command().as_str().to_owned();
            return handle_command(
                ctx,
                player,
                name,
                writer,
                chunk_stream,
                inventory,
                player_yaw,
                player_pitch,
                &command,
                debug,
                compression,
                liveness.next_deadline(),
                pending_teleport,
            )
            .await;
        }
        ServerboundPlayPacket::ChatMessage(chat) => {
            // Rate-limit at the SOURCE before relaying: one spammer must not fan
            // spam into every recipient's bounded outbound channel and starve legit
            // packets. The per-connection token bucket is driven by the server tick
            // (connection task, allowed a non-deterministic clock). Over budget ->
            // drop the line (logged, not relayed); the sender is not disconnected
            // for a transient burst.
            if !chat_limiter.try_consume(ctx.clock.now()) {
                tracing::debug!(player = name, "dropping over-budget chat line");
                return Ok(());
            }
            // Consult the loaded plugins at the chat intent boundary (off the tick,
            // under the host mutex, no lock across an `.await`) BEFORE broadcasting.
            // A Deny drops the line (it is never relayed) and shows the sender the
            // reason, if any; a chat filter rides this path. The plugins see the raw
            // message the player typed and the sender's last known position.
            let perms = PermissionFacade::new(ctx.policy.permissions());
            let (chat_decision, chat_intents) = ctx.block_events.before_chat(
                &ChatAttempt::new(player, chat.message().as_str()),
                chunk_stream.last_position(),
                &perms,
            );
            if let PluginEventDecision::Deny { message } = chat_decision {
                // `chat_intents` is empty on a Deny (the dispatcher drops them so a
                // dropped message cannot still trigger a side effect); routing it is
                // a no-op kept for symmetry with the Allow path.
                deliver_deny_message(ctx, writer, debug, compression, message);
                return route_emitted_intents(
                    ctx,
                    player,
                    writer,
                    0,
                    chat_intents,
                    debug,
                    compression,
                )
                .await;
            }
            // Allowed: route any intents the plugins emitted, then broadcast.
            route_emitted_intents(ctx, player, writer, 0, chat_intents, debug, compression).await?;
            // Relay unsigned player chat to everyone as a system message: format it
            // "<name> message" and hand it to the driver, the only owner of every
            // player's outbound channel. enforces_secure_chat = false, so no 1.19
            // signing apparatus is needed (the signature tail was decoded into the
            // ignored `rest` field). The relay reaches the sender via its own
            // session outbound channel, so it is NOT also enqueued on the writer.
            //
            // Strip legacy section-sign (§, U+00A7) codes from the untrusted
            // message first: a client still interprets `§k`/`§l`/§<colour> inside
            // a component's `text`, so leaving them in would let a player inject
            // colour/obfuscation formatting into the relayed line. `name` is the
            // connection boundary's display-safe projection of an already
            // validated `[A-Za-z0-9_]{1,16}` identity.
            let sanitized = chat.message().as_str().replace('\u{00A7}', "");
            let line = format!("<{name}> {sanitized}");
            let content = TextComponent::text(line);
            return ctx
                .commands
                .send(SimCommand::BroadcastSystemChat {
                    content,
                    overlay: false,
                })
                .await
                .map_err(|_| anyhow::anyhow!("simulation driver is gone"));
        }
        // Root literals are local; Java argument suggestions use the bounded actor.
        ServerboundPlayPacket::TabCompleteRequest(req) => {
            return handle_tab_complete(
                ctx,
                player,
                name,
                writer,
                req.transaction_id(),
                req.text().as_str(),
                debug,
                compression,
                liveness.next_deadline(),
            )
            .await;
        }
        // Place-with-held-item: resolve the held hotbar stack to a block-state and
        // route a place carrying it (or just ack, on empty hand / non-placeable /
        // veto). Handled here, not via the generic NetEvent path, because the place
        // needs the inventory the session layer cannot see.
        ServerboundPlayPacket::UseItemOn(p) => {
            return handle_use_item_on(
                ctx,
                player,
                writer,
                inventory,
                window_state,
                p,
                *player_yaw,
                chunk_stream.last_position(),
                debug,
                compression,
            )
            .await;
        }
        // Update Sign: the player confirmed text in the sign editor. Route it to
        // the block's owning shard, which validates and stores it (net never writes
        // the world directly) and broadcasts the rendered sign to viewers.
        ServerboundPlayPacket::UpdateSign(p) => {
            return handle_update_sign(ctx, player, p).await;
        }
        // Set Creative Slot (untrusted): validate the hostile item bytes, store the
        // slot, and echo it back so the client view matches the server.
        ServerboundPlayPacket::SetCreativeSlot(p) => {
            let Some(stack) = creative_item else {
                anyhow::bail!("strict creative-slot decode omitted its validated item");
            };
            return handle_set_creative_slot(
                ctx,
                player,
                name,
                writer,
                inventory,
                p,
                stack,
                debug,
                compression,
            )
            .await;
        }
        // Set Held Item (serverbound): update the selected hotbar index and, on a
        // real change, broadcast the new held item to viewers.
        ServerboundPlayPacket::ServerboundSetHeldItem(p) => {
            return handle_set_held_item(ctx, player, inventory, p).await;
        }
        // Click Container: an open chest window applies the safe left-click subset
        // (everything else resyncs); a click on window 0 resyncs the inventory.
        ServerboundPlayPacket::WindowClick(p) => {
            return handle_window_click(
                ctx,
                player,
                writer,
                inventory,
                window_state,
                p,
                debug,
                compression,
            )
            .await;
        }
        // Close Container: return any carried item to the inventory and resync.
        ServerboundPlayPacket::CloseContainer(p) => {
            return handle_close_container(
                ctx,
                player,
                writer,
                inventory,
                window_state,
                p,
                debug,
                compression,
            )
            .await;
        }
        // Connection-only control packets never reach simulation.
        ServerboundPlayPacket::ServerboundKeepAlive(_) => return Ok(()),
        _ => {}
    }

    let event = NetEvent::play(player, packet);
    // A block break crosses the plugin intent boundary: the loaded plugins'
    // `before_block_break` decision hooks decide whether (and how) it proceeds.
    // Every other event reaching this fallthrough carries no block decision and
    // routes straight to the simulation; movement returned through its validated
    // admission path above.
    if let Some(GameInput::BlockBreak {
        position, sequence, ..
    }) = net_event_to_input(&event)
    {
        return handle_block_break(
            ctx,
            player,
            writer,
            position,
            sequence,
            event,
            debug,
            compression,
        )
        .await;
    }
    ctx.commands
        .send(SimCommand::Event {
            event,
            acceptance: None,
        })
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))
}

/// Decodes one complete serverbound Play body to the generated schema's precision.
///
/// Generated packet structs decode fields but deliberately leave frame
/// exhaustion to their caller. Fields explicitly modelled as opaque remainders,
/// such as signed-chat tails, remain the packet's payload. `SetCreativeSlot` is
/// the nested format this boundary owns: its generated item field consumes the
/// opaque remainder, so this function also decodes, validates, and finishes that
/// untrusted-slot reader before the packet can reach authorization or inventory
/// effects.
fn decode_play_body(body: &[u8]) -> Result<PlayBodyDecode, DecodeError> {
    let state = ConnectionState::Play;
    let mut reader = BoundedReader::new(body);
    let id = reader
        .read_var_int()
        .map_err(|_| DecodeError::MalformedBody { state })?;

    let packet = match ServerboundPlayPacket::decode(id, &mut reader) {
        Ok(packet) => packet,
        Err(ProtoError::UnknownPacketId { .. })
            if (0..=MAX_PROTOCOL_772_SERVERBOUND_PLAY_ID).contains(&id) =>
        {
            return Ok(PlayBodyDecode::Unmodelled { id });
        }
        Err(ProtoError::UnknownPacketId { .. }) => {
            return Err(DecodeError::UnknownPacket { state, id });
        }
        Err(_) => return Err(DecodeError::MalformedBody { state }),
    };

    match reader.finish() {
        Ok(()) => {}
        Err(ferrumc_codec::CodecError::TrailingBytes { remaining }) => {
            return Err(DecodeError::TrailingBytes {
                state,
                trailing: remaining,
            });
        }
        Err(_) => return Err(DecodeError::MalformedBody { state }),
    }

    let creative_item = if let ServerboundPlayPacket::SetCreativeSlot(packet) = &packet {
        let mut item_reader = BoundedReader::new(packet.item());
        let untrusted = UntrustedItemStack::decode(&mut item_reader)
            .map_err(|_| DecodeError::MalformedBody { state })?;
        match item_reader.finish() {
            Ok(()) => {}
            Err(ferrumc_codec::CodecError::TrailingBytes { remaining }) => {
                return Err(DecodeError::TrailingBytes {
                    state,
                    trailing: remaining,
                });
            }
            Err(_) => return Err(DecodeError::MalformedBody { state }),
        }
        Some(
            untrusted
                .into_validated()
                .map_err(|_| DecodeError::MalformedBody { state })?,
        )
    } else {
        None
    };

    Ok(PlayBodyDecode::Modelled(DecodedPlayBody {
        packet,
        creative_item,
    }))
}

/// One validated, atomically admitted movement observation.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MovementObservation {
    /// Absolute position, absent on a rotation-only packet.
    position: Option<Vec3>,
    /// Body yaw, absent on a position-only packet.
    yaw: Option<f32>,
    /// Pitch, absent on a position-only packet.
    pitch: Option<f32>,
}

/// Classification of one decoded packet at the client-movement trust boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
enum MovementValidation {
    /// This packet carries no player movement.
    NotMovement,
    /// Every carried component is finite and any position is in range.
    Valid(MovementObservation),
    /// At least one carried component is unsafe; the whole packet is rejected.
    Invalid,
}

/// Validates one decoded movement packet atomically.
///
/// Coordinates use the same inclusive finite/range predicate as restoration and
/// the simulation. Look angles may be arbitrary finite degrees—the protocol and
/// simulation allow wrapping—but NaN and infinities are rejected so they cannot
/// poison routing, placement, or persistence. A combined packet is all-or-nothing.
fn validated_movement(packet: &ServerboundPlayPacket) -> MovementValidation {
    let movement = match packet {
        ServerboundPlayPacket::SetPlayerPosition(p) => MovementObservation {
            position: Some(Vec3::new(p.x(), p.y(), p.z())),
            yaw: None,
            pitch: None,
        },
        ServerboundPlayPacket::SetPlayerPositionAndRotation(p) => MovementObservation {
            position: Some(Vec3::new(p.x(), p.y(), p.z())),
            yaw: Some(p.yaw()),
            pitch: Some(p.pitch()),
        },
        ServerboundPlayPacket::SetPlayerRotation(p) => MovementObservation {
            position: None,
            yaw: Some(p.yaw()),
            pitch: Some(p.pitch()),
        },
        _ => return MovementValidation::NotMovement,
    };

    let valid_position = match movement.position {
        Some(position) => is_valid_player_position(position),
        None => true,
    };
    let valid_yaw = match movement.yaw {
        Some(yaw) => yaw.is_finite(),
        None => true,
    };
    let valid_pitch = match movement.pitch {
        Some(pitch) => pitch.is_finite(),
        None => true,
    };
    if valid_position && valid_yaw && valid_pitch {
        MovementValidation::Valid(movement)
    } else {
        MovementValidation::Invalid
    }
}

/// Maps the wire hand index of a use-item packet to the typed [`InteractHand`].
///
/// The protocol encodes `0` as the main hand and `1` as the off-hand; any other
/// value is treated as the off-hand (the only non-main option), so a malformed
/// index can never be mistaken for a main-hand action.
fn interact_hand(hand: i32) -> InteractHand {
    if hand == 0 {
        InteractHand::Main
    } else {
        InteractHand::Off
    }
}

/// Handles a block break at the plugin intent boundary.
///
/// Consults the loaded plugins' `before_block_break` hooks (off the tick, under
/// the host mutex with no lock held across an `.await`) and resolves the combined
/// decision:
///
/// - [`Deny`](ResolvedDecision::Deny): the edit is dropped (the world is never
///   modified) and the actor's optimistic client-side prediction is healed through
///   the single reject funnel ([`reject_block_edit`]) — a [`SimCommand::RejectBlockEdit`]
///   to the block's owning shard, which reads the authoritative state and emits the
///   actor's mandatory resync `BlockUpdate` + `AcknowledgeBlockChange` (the same
///   path a sim-side rejection uses, so a Deny no longer leaves a ghost block). The
///   refused break heals to the block the client predicted removing
///   ([`BlockStateId::AIR`] is the predicted state passed for metric classification).
///   The rejection is counted once, on the sim's resulting `BlockChangeRejected`. If
///   the decision carries a message it is delivered as a system chat.
/// - [`Replace`](ResolvedDecision::Replace): the broken block is set to the
///   replacement state instead of air, by routing a [`SimCommand::SetBlockExact`]
///   at the break position — an exact write applied verbatim (never re-run through
///   `compute_placement`, so a rotated replacement survives).
/// - [`Allow`](ResolvedDecision::Allow): the original break routes to the
///   simulation as before.
///
/// Any emitted [`WorldIntent`]s are routed, and on a non-denied edit the
/// `after_block_break` notification fires.
#[allow(clippy::too_many_arguments)] // the connection threads its writer + trace context through
async fn handle_block_break(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    position: BlockPos,
    sequence: i32,
    event: NetEvent,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    let perms = PermissionFacade::new(ctx.policy.permissions());
    let (decision, emitted) = ctx
        .block_events
        .before_block_break(&BlockBreakAttempt::new(player, position), &perms);

    match decision {
        ResolvedDecision::Deny { message } => {
            // Heal the actor through the single reject funnel: the sim reads the
            // authoritative state and sends the mandatory resync + ack (no ghost).
            // The predicted state for a break is air. The metric is counted on the
            // resulting BlockChangeRejected, so it is NOT recorded here too.
            reject_block_edit(ctx, player, position, sequence, BlockStateId::AIR).await?;
            deliver_deny_message(ctx, writer, debug, compression, message);
            route_emitted_intents(ctx, player, writer, sequence, emitted, debug, compression)
                .await?;
            return Ok(());
        }
        ResolvedDecision::Replace { block_state_id } => {
            // A plugin Replace supplies an EXACT state: write it verbatim through
            // the exact-write path so the simulation never re-runs it through
            // compute_placement (a rotated replacement must survive byte-for-byte).
            // Reject an unrepresentable state id (> i32::MAX) before it can be
            // stored — the mandatory resync must never meet a state the wire cannot
            // carry — and heal the actor, which predicted the break to air.
            if i32::try_from(block_state_id).is_err() {
                tracing::warn!(
                    block_state_id,
                    "plugin replace supplied an out-of-range block state id; healing actor and skipping the write"
                );
                reject_block_edit(ctx, player, position, sequence, BlockStateId::AIR).await?;
                route_emitted_intents(ctx, player, writer, sequence, emitted, debug, compression)
                    .await?;
                return Ok(());
            }
            send_sim_command_accepted(
                ctx,
                SimCommand::SetBlockExact {
                    player,
                    position,
                    sequence,
                    state: BlockStateId::new(block_state_id),
                    acceptance: None,
                },
            )
            .await?;
        }
        _ => {
            send_sim_command_accepted(
                ctx,
                SimCommand::Event {
                    event,
                    acceptance: None,
                },
            )
            .await?;
        }
    }

    route_emitted_intents(ctx, player, writer, sequence, emitted, debug, compression).await?;
    // The edit was accepted at the intent boundary and routed: notify after_*.
    let after = ctx.block_events.after_block_break(player, position, &perms);
    route_emitted_intents(ctx, player, writer, sequence, after, debug, compression).await
}

/// Delivers a plugin Deny message (if any) to the acting player as a system chat.
fn deliver_deny_message(
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    message: Option<TextComponent>,
) {
    if let Some(message) = message {
        enqueue_traced_classified(
            writer,
            debug,
            compression,
            &ctx.clock,
            ferrumc_session::system_chat(&message, false),
        );
    }
}

/// The single funnel that heals a block edit refused at the connection (a plugin
/// `Deny` / spawn-protection veto).
///
/// The net layer has no world access, so it cannot read the authoritative block
/// state needed to undo the client's optimistic prediction. Instead of authoring an
/// ack-only heal (which leaves a ghost block, since a `BlockUpdate` is swallowed
/// while a prediction is pending and an ack alone heals to the *predicted* state),
/// it routes a [`SimCommand::RejectBlockEdit`] to the block's owning shard. The
/// shard reads the authoritative state and emits a `BlockChangeRejected`, which the
/// router turns into the actor's mandatory resync `BlockUpdate` + `AcknowledgeBlockChange`
/// — the exact same path a sim-side rejection (out of reach, unloaded chunk) uses.
///
/// `requested_state` is the state the client predicted (air for a refused break,
/// the held block for a refused place); it is used only to classify the block-edit
/// metric on the resulting rejection, never applied to the world.
///
/// The call waits for bounded shard admission. Rejection healing uses the
/// control-reserved lane; if even that lane is physically full, the driver fails
/// closed and retains a `PlayerLeave` retry rather than acknowledging or silently
/// dropping the heal.
async fn reject_block_edit(
    ctx: &ConnContext,
    player: PlayerId,
    position: BlockPos,
    sequence: i32,
    requested_state: BlockStateId,
) -> anyhow::Result<()> {
    send_sim_command_accepted(
        ctx,
        SimCommand::RejectBlockEdit {
            player,
            position,
            sequence,
            requested_state,
            acceptance: None,
        },
    )
    .await
}

/// Routes the [`WorldIntent`]s a plugin emitted from a block decision (or an
/// after-* notification).
///
/// Mapping (bounded; world-changing intents wait for shard admission):
/// - [`WorldIntent::SetBlock`] -> [`SimCommand::SetBlockExact`] by the acting
///   player (an exact write applied verbatim, never refined by `compute_placement`).
/// - [`WorldIntent::Message`] -> a system chat to the acting player's own writer
///   when it targets them, otherwise a targeted [`SimCommand::SendSystemChat`] to
///   the named recipient (the connection task cannot reach another player's
///   outbound channel directly, so the driver-owned router delivers it).
/// - [`WorldIntent::Teleport`] -> a [`SimCommand::TeleportPlayer`], which the
///   driver-owned router fulfils (snap the target's client + route an
///   authoritative move).
async fn route_emitted_intents(
    ctx: &ConnContext,
    actor: PlayerId,
    writer: &mut PlayWriter,
    sequence: i32,
    intents: Vec<WorldIntent>,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    for intent in intents {
        match intent {
            WorldIntent::SetBlock {
                pos,
                block_state_id,
            } => {
                // A plugin SetBlock supplies an EXACT state: write it verbatim
                // (bypassing compute_placement) so a rotated state survives. Skip an
                // unrepresentable state id (> i32::MAX); an arbitrary plugin
                // set-block carries no client prediction to heal, so log and drop it
                // rather than store a state the wire cannot carry.
                if i32::try_from(block_state_id).is_err() {
                    tracing::warn!(
                        block_state_id,
                        "plugin set-block supplied an out-of-range block state id; skipping the write"
                    );
                    continue;
                }
                send_sim_command_accepted(
                    ctx,
                    SimCommand::SetBlockExact {
                        player: actor,
                        position: pos,
                        sequence,
                        state: BlockStateId::new(block_state_id),
                        acceptance: None,
                    },
                )
                .await?;
            }
            WorldIntent::Message { player, message } => {
                if player == actor {
                    // The actor is this connection: write straight to its socket.
                    enqueue_traced_classified(
                        writer,
                        debug,
                        compression,
                        &ctx.clock,
                        ferrumc_session::system_chat(&message, false),
                    );
                } else {
                    // A different recipient: route a TARGETED chat through the
                    // driver-owned router (only it holds other players' channels).
                    ctx.commands
                        .send(SimCommand::SendSystemChat {
                            player,
                            content: message,
                            overlay: false,
                        })
                        .await
                        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
                }
            }
            WorldIntent::Teleport { player, position } => {
                if !is_valid_player_position(position) {
                    tracing::warn!(
                        %player,
                        x = position.x,
                        y = position.y,
                        z = position.z,
                        "plugin supplied an unsafe teleport destination; skipping the teleport"
                    );
                    continue;
                }
                // The connection cannot reach another player's channel; the
                // driver-owned router snaps the target and routes the authoritative
                // move.
                send_sim_command_accepted(
                    ctx,
                    SimCommand::TeleportPlayer {
                        player,
                        position,
                        acceptance: None,
                    },
                )
                .await?;
            }
            _ => {
                tracing::debug!("plugin emitted an intent with no connection-side route; skipping");
            }
        }
    }
    Ok(())
}

/// Dispatches a `/command` for `player`, reports the outcome to them, and applies
/// its side effect.
///
/// Dispatch goes through the shared command tree with the player's per-player
/// permission level and a node-string checker backed by the permission registry.
/// Every outcome is now reported to the issuer as a `SystemChat` on their writer:
/// a dispatch failure (unknown command, bad argument, permission denied) becomes a
/// red error line and the command stops there, while a handler that ran reports
/// its [`CommandResult`](ferrumc_command::CommandResult) feedback (for success or
/// logical failure).
///
/// On a successful `/spawn`, the authoritative move is admitted before
/// `SynchronizePlayerPosition` is queued to the socket. On a successful
/// `/gamemode <id>`, the authoritative mode change is admitted before the
/// `GameEvent` with reason `3` (`change_game_mode`) switches the client. Command
/// success feedback and plugin after-effects follow the same admission barrier.
#[allow(clippy::too_many_arguments)] // one command step: dispatch + feedback + side effects + I/O
#[allow(clippy::too_many_lines)] // one ordered transaction keeps admission before every visible success
async fn handle_command(
    ctx: &ConnContext,
    player: PlayerId,
    name: &str,
    writer: &mut PlayWriter,
    chunk_stream: &mut ChunkStream,
    inventory: &mut PlayerInventory,
    player_yaw: &mut f32,
    player_pitch: &mut f32,
    command: &str,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    deadline: Option<ferrumc_net::LivenessDeadline>,
    pending_teleport: &mut PendingTeleport,
) -> anyhow::Result<()> {
    let policy = &ctx.policy;
    if let Some(bridge) = &ctx.java_bridge {
        let mut tokens = command.split_whitespace();
        if let Some(label) = tokens
            .next()
            .filter(|label| bridge.find_command(label).is_some())
        {
            let descriptor = bridge
                .find_command(label)
                .ok_or_else(|| anyhow::anyhow!("Java command metadata disappeared"))?;
            if descriptor
                .permission
                .as_ref()
                .is_some_and(|node| !policy.java_permission_allowed(player, node))
            {
                return super::outbound::send_java_feedback(
                    ctx,
                    writer,
                    debug,
                    compression,
                    "You do not have permission to run this command",
                );
            }
            match bridge
                .command(
                    label,
                    tokens.take(65).map(str::to_owned).collect(),
                    ctx.java_player(player, name),
                )
                .await
            {
                Ok(messages) => {
                    super::outbound::send_java_messages(ctx, writer, debug, compression, messages)?;
                }
                Err(error) => {
                    tracing::warn!(%error, player = name, "Java command failed");
                    super::outbound::send_java_feedback(
                        ctx,
                        writer,
                        debug,
                        compression,
                        &error.to_string(),
                    )?;
                }
            }
            return Ok(());
        }
    }
    let source = CommandSource::for_player(player, name, policy.permission_level(player));
    let allowed = |node: &str| ctx.command_permission_allowed(player, node);
    let result = match policy
        .command_tree()
        .dispatch_with(command, &source, &allowed)
    {
        Ok(result) => result,
        Err(err) => {
            // The handler never ran (unknown command / bad argument / permission
            // denied): report why to the issuer as a red system-chat line.
            let message = TextComponent::text(err.to_string()).with_color(TextColor::Red);
            enqueue_traced_classified(
                writer,
                debug,
                compression,
                &ctx.clock,
                ferrumc_session::system_chat(&message, false),
            );
            return Ok(());
        }
    };

    if !result.is_success() {
        if !result.feedback().to_plain_string().is_empty() {
            enqueue_traced_classified(
                writer,
                debug,
                compression,
                &ctx.clock,
                ferrumc_session::system_chat(result.feedback(), false),
            );
        }
        return Ok(());
    }

    if matches!(command.split_whitespace().next(), Some("op" | "deop")) {
        let changed = match crate::operators::parse_command(command) {
            Ok((target, operator)) => {
                crate::operators::guard_change(
                    deadline,
                    ctx.shutdown.clone(),
                    policy.operators().change(Some(player), target, operator),
                )
                .await?
            }
            Err(error) => Err(error),
        };
        let feedback = match changed {
            Ok(message) => TextComponent::text(message),
            Err(error) => TextComponent::text(error.to_string()).with_color(TextColor::Red),
        };
        send_mandatory(
            writer,
            debug,
            compression,
            &ctx.clock,
            ferrumc_session::system_chat(&feedback, false),
        )?;
        return Ok(());
    }

    // Route every driver-owned command effect before publishing success feedback.
    // Shard-mutating commands wait for their explicit bounded admission; pure
    // driver-owned presentation/time commands need only cross the already-bounded
    // app command channel.
    for sim_command in crate::command::region_commands(command, player) {
        if sim_command.supports_delivery_acceptance() {
            send_sim_command_accepted(ctx, sim_command).await?;
        } else {
            ctx.commands
                .send(sim_command)
                .await
                .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
        }
    }

    let first_token = command.split_whitespace().next();
    if first_token == Some(SPAWN_COMMAND) {
        let spawn = policy.spawn();
        // Snap the player to spawn at their CURRENT look: the previous `0.0/0.0`
        // reset them to facing south and level on every `/spawn`.
        let sync = pending_teleport.issue(&spawn_sync(0, spawn, *player_yaw, *player_pitch));
        let move_event = NetEvent::play(
            player,
            ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(
                spawn.x, spawn.y, spawn.z, 0,
            )),
        );
        send_sim_command_accepted(
            ctx,
            SimCommand::Event {
                event: move_event,
                acceptance: None,
            },
        )
        .await?;
        // Only after admission may the client/persistence mirrors preview the
        // teleport. A rejected route terminates the overloaded session above and
        // never reaches these effects.
        mirror_server_teleport(chunk_stream, player_yaw, player_pitch, &sync);
        send_mandatory(
            writer,
            debug,
            compression,
            &ctx.clock,
            ClientboundPlayPacket::SynchronizePlayerPosition(sync),
        )?;
    } else if first_token == Some(GAMEMODE_COMMAND) {
        if let Some(mode) = parse_gamemode(command) {
            send_sim_command_accepted(
                ctx,
                SimCommand::SetGameMode {
                    player,
                    mode,
                    acceptance: None,
                },
            )
            .await?;
            // Switch both client-side mirrors only after authoritative shard
            // admission, so saturation cannot split visual and simulation modes.
            enqueue_traced_classified(
                writer,
                debug,
                compression,
                &ctx.clock,
                ClientboundPlayPacket::GameEvent(GameEvent::new(
                    GAME_EVENT_CHANGE_GAMEMODE,
                    f32::from(mode.as_id()),
                )),
            );
            inventory.set_game_mode(mode);
        }
    }

    // The handler ran and every authoritative side effect above was accepted:
    // only now may the issuer receive success feedback.
    if !result.feedback().to_plain_string().is_empty() {
        enqueue_traced_classified(
            writer,
            debug,
            compression,
            &ctx.clock,
            ferrumc_session::system_chat(result.feedback(), false),
        );
    }

    // Presentation and scoreboard commands have no shard mutation. Enqueue their
    // clientbound effects after the same successful dispatch point.
    for packet in crate::command::presentation_packets(command, policy.spawn()) {
        enqueue_traced_classified(writer, debug, compression, &ctx.clock, packet);
    }
    for packet in crate::command::scoreboard_packets(command, name)? {
        enqueue_traced_classified(writer, debug, compression, &ctx.clock, packet);
    }
    Ok(())
}

/// Validates an untrusted `UseItemOn` hit point before any interaction effect.
///
/// The placement rules currently read only the vertical component for some
/// families, but all three coordinates cross the same protocol trust boundary.
/// Rejecting rather than clamping keeps malformed input from selecting a valid
/// placement state the client did not actually send.
fn validated_placement_cursor(packet: &UseItemOn) -> Option<Vec3> {
    let [x, y, z] = [packet.cursor_x(), packet.cursor_y(), packet.cursor_z()];
    [x, y, z]
        .into_iter()
        .all(|component| component.is_finite() && (0.0..=1.0).contains(&component))
        .then(|| Vec3::new(f64::from(x), f64::from(y), f64::from(z)))
}

/// Handles a serverbound `UseItemOn`: place the held block at the targeted cell,
/// after consulting the loaded plugins at the intent boundary.
///
/// Resolves the held hotbar stack to a block-state. An empty hand, a non-placeable
/// item, or a malformed face places nothing but still acknowledges the block-action
/// sequence so the client's optimistic prediction ends. A non-finite or
/// out-of-range cursor is discarded earlier, before any acknowledgement, plugin
/// effect, preview, or simulation command. A placeable block is offered to the
/// plugins' `before_block_place` hooks (off the tick); the combined decision is
/// then resolved:
///
/// - [`Deny`](ResolvedDecision::Deny): nothing is placed; the actor is healed
///   through the single reject funnel ([`reject_block_edit`]) — the sim reads the
///   authoritative state at the target (usually air) and sends the mandatory resync
///   and ack, so no ghost block remains — and any Deny message is delivered as a
///   system chat. The rejection is counted on the sim's resulting
///   `BlockChangeRejected`.
/// - [`Replace`](ResolvedDecision::Replace): the replacement block-state is written
///   verbatim (an exact write via [`SimCommand::SetBlockExact`], never refined by
///   `compute_placement`) instead of the held one.
/// - [`Allow`](ResolvedDecision::Allow): the held block is placed (creative never
///   decrements the stack) and refined by `compute_placement` — the only path that
///   runs it.
///
/// On a non-denied placement any emitted intents are routed and the
/// `after_block_place` notification fires with the FINAL state: the exact
/// replacement for a Replace, or the driver-previewed computed state for an Allow.
#[allow(clippy::too_many_arguments)] // one place step: inventory + plugin policy + placement context + I/O
#[allow(clippy::too_many_lines)] // one place dispatch: deny + exact replace + refined allow arms
async fn handle_use_item_on(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    packet: &UseItemOn,
    player_yaw: f32,
    player_position: Option<Vec3>,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    let Some(cursor_position) = validated_placement_cursor(packet) else {
        return Ok(());
    };
    let sequence = packet.sequence();
    // A malformed face index yields no target: just ack so the prediction ends.
    let Some(position) = use_item_on_target(packet) else {
        ack_sequence(writer, debug, compression, &ctx.clock, sequence)?;
        return Ok(());
    };
    // The clicked face shares `use_item_on_target`'s face decoding, so a `Some`
    // target guarantees a `Some` face; the fallback is unreachable but fails safe.
    let clicked_face = use_item_on_face(packet).unwrap_or(Direction::Up);

    let perms = PermissionFacade::new(ctx.policy.permissions());

    // Consult the plugins at the interaction intent boundary FIRST: a right-click on
    // a block is an interaction regardless of what (if anything) it would place. A
    // Deny stops the interaction here — heal the client's prediction, show the
    // reason, and do not place — before the block-place decision even runs. (The
    // current protocol slice only carries use-item-on-block, so the target is always
    // a `Block`; air/entity interactions are a future wiring.)
    let (interact_decision, interact_intents) = ctx.block_events.before_interact(
        &InteractAttempt::new(
            player,
            interact_hand(packet.hand()),
            InteractTarget::Block {
                pos: position,
                face: clicked_face,
            },
        ),
        player_position,
        &perms,
    );
    if let PluginEventDecision::Deny { message } = interact_decision {
        // The interaction is refused. If the held item would have placed a block,
        // the client already predicted that placement at `position`; an ack alone
        // heals to the predicted state and leaves a ghost block (see
        // `reject_block_edit`), so route the same heal a refused place uses. A
        // non-placeable interaction predicts no block, so a plain ack ends it.
        if let Some(held_state) = inventory.held().placeable_block() {
            reject_block_edit(
                ctx,
                player,
                position,
                sequence,
                BlockStateId::new(held_state),
            )
            .await?;
        } else {
            ack_sequence(writer, debug, compression, &ctx.clock, sequence)?;
        }
        deliver_deny_message(ctx, writer, debug, compression, message);
        return route_emitted_intents(
            ctx,
            player,
            writer,
            sequence,
            interact_intents,
            debug,
            compression,
        )
        .await;
    }
    // Allowed: route any intents the interaction emitted, then continue to placement.
    route_emitted_intents(
        ctx,
        player,
        writer,
        sequence,
        interact_intents,
        debug,
        compression,
    )
    .await?;

    // Right-clicking a chest opens its container instead of placing (vanilla opens
    // regardless of a held placeable block when not sneaking). Net never reads the
    // world, so ask the clicked block's owning shard whether it is an openable chest
    // and, if so, snapshot its contents. A non-chest falls through to placement.
    let clicked = use_item_on_block(packet);
    let (open_tx, open_rx) = oneshot::channel();
    ctx.commands
        .send(SimCommand::OpenContainer {
            player,
            position: clicked,
            reply: open_tx,
        })
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
    let chest_open = open_rx
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver dropped the open-container reply"))?;
    if let Some(slots) = chest_open {
        // End the client's interaction prediction, then show the container screen.
        ack_sequence(writer, debug, compression, &ctx.clock, sequence)?;
        return open_chest_window(
            ctx,
            player,
            writer,
            inventory,
            window_state,
            clicked,
            slots,
            debug,
            compression,
        )
        .await;
    }

    // Empty hand or non-placeable item: nothing to place, just ack. The block-place
    // plugins are not consulted for a no-op placement (the interaction already was).
    let Some(held_state) = inventory.held().placeable_block() else {
        ack_sequence(writer, debug, compression, &ctx.clock, sequence)?;
        return Ok(());
    };

    let (decision, emitted) = ctx.block_events.before_block_place(
        &BlockPlaceAttempt::new(player, position, held_state),
        &perms,
    );

    match decision {
        ResolvedDecision::Deny { message } => {
            // Heal the actor through the single reject funnel: the sim reads the
            // authoritative state at the target (usually air) and sends the
            // mandatory resync + ack (no ghost). The predicted state for a place is
            // the held block; it heals the target back to its current block. The
            // metric is counted on the resulting BlockChangeRejected, not here.
            reject_block_edit(
                ctx,
                player,
                position,
                sequence,
                BlockStateId::new(held_state),
            )
            .await?;
            deliver_deny_message(ctx, writer, debug, compression, message);
            route_emitted_intents(ctx, player, writer, sequence, emitted, debug, compression).await
        }
        ResolvedDecision::Replace { block_state_id } => {
            // A plugin Replace supplies an EXACT state: write it verbatim through
            // the exact-write path so compute_placement never re-derives it (a
            // rotated replacement must survive). The clicked face / cursor / yaw are
            // deliberately NOT passed — the plugin already chose the final state.
            // Reject an unrepresentable id (> i32::MAX) and heal the actor (it
            // predicted the held block).
            if i32::try_from(block_state_id).is_err() {
                tracing::warn!(
                    block_state_id,
                    "plugin replace supplied an out-of-range block state id; healing actor and skipping the place"
                );
                reject_block_edit(
                    ctx,
                    player,
                    position,
                    sequence,
                    BlockStateId::new(held_state),
                )
                .await?;
                return route_emitted_intents(
                    ctx,
                    player,
                    writer,
                    sequence,
                    emitted,
                    debug,
                    compression,
                )
                .await;
            }
            send_sim_command_accepted(
                ctx,
                SimCommand::SetBlockExact {
                    player,
                    position,
                    sequence,
                    state: BlockStateId::new(block_state_id),
                    acceptance: None,
                },
            )
            .await?;
            route_emitted_intents(ctx, player, writer, sequence, emitted, debug, compression)
                .await?;
            // The exact replacement state IS the final state, so the after-hook
            // observes it directly (no refinement to wait on).
            let after =
                ctx.block_events
                    .after_block_place(player, position, block_state_id, &perms);
            route_emitted_intents(ctx, player, writer, sequence, after, debug, compression).await
        }
        _ => {
            // Allow: a player placement refined by compute_placement (the ONLY path
            // that runs it). Creative places the held block without touching the
            // stack; the clicked face, cursor hit point, and player yaw ride along
            // so the sim derives the correct rotated/faced/halved state. After
            // bounded shard admission, the driver previews that final state and
            // replies before the tick, so the after-hook fires with the state the
            // world will hold (e.g. a side-faced log's axis=x), not the held
            // default.
            let (reply_tx, reply_rx) = oneshot::channel();
            ctx.commands
                .send(SimCommand::PlaceBlock {
                    player,
                    position,
                    sequence,
                    state: BlockStateId::new(held_state),
                    clicked_face,
                    cursor_position,
                    player_yaw,
                    reply: reply_tx,
                })
                .await
                .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
            let computed = reply_rx
                .await
                .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?
                .map_err(|err| anyhow::anyhow!("block placement rejected: {err}"))?;
            route_emitted_intents(ctx, player, writer, sequence, emitted, debug, compression)
                .await?;
            let after =
                ctx.block_events
                    .after_block_place(player, position, computed.as_u32(), &perms);
            route_emitted_intents(ctx, player, writer, sequence, after, debug, compression).await
        }
    }
}

/// Handles a serverbound Update Sign: route the player's new sign text to the
/// block's owning shard as a [`SimCommand::UpdateSign`].
///
/// All validation — the editor is within reach and a non-waxed sign block-entity
/// exists at the target — happens in the simulation at the tick boundary, since
/// net never reads or writes the world directly. This only converts the wire
/// fields (the packed location and the four bounded line strings) and forwards
/// them; an edit the sim refuses is a silent no-op.
async fn handle_update_sign(
    ctx: &ConnContext,
    player: PlayerId,
    packet: &UpdateSign,
) -> anyhow::Result<()> {
    let loc = packet.location();
    let position = BlockPos::new(loc.x(), loc.y(), loc.z());
    let lines = [
        packet.line_1().as_str().to_owned(),
        packet.line_2().as_str().to_owned(),
        packet.line_3().as_str().to_owned(),
        packet.line_4().as_str().to_owned(),
    ];
    send_sim_command_accepted(
        ctx,
        SimCommand::UpdateSign {
            player,
            position,
            is_front: packet.is_front_text(),
            lines,
            acceptance: None,
        },
    )
    .await
}

/// Handles a serverbound Set Creative Slot: validate the untrusted item bytes,
/// store the slot, and echo it.
///
/// Requires the player to be authoritatively creative (read from the connection's
/// drift-free game-mode mirror); a non-creative sender is ignored. The `slot` must
/// be in `0..=45` (a `-1` "drop outside" or any other out-of-range value is
/// ignored). The item bytes have already gone through strict
/// [`UntrustedItemStack::decode`], nested reader exhaustion, and `into_validated`
/// before this policy handler runs. Validation clamps the count and strips
/// dangerous/unknown components. A malformed item is therefore fatal before
/// either the game-mode/slot gate or any inventory effect. On success the slot
/// is stored, the state id bumped, and a mandatory `SetContainerSlot` echoes the
/// authoritative slot back so the client view matches the server.
///
/// If the changed slot is mirrored into the player's visible equipment (held main
/// hand, off hand, or worn armor — see [`PlayerInventory::is_equipment_slot`]), the
/// full equipment set is rebroadcast to viewers via [`SimCommand::SetEquipment`] so
/// they see the new piece — or its removal, since empty slots encode as air. An
/// encode failure for the broadcast is logged and skipped (never fatal).
#[allow(clippy::too_many_arguments)] // one creative-slot step: identity + window state + I/O + trace
async fn handle_set_creative_slot(
    ctx: &ConnContext,
    player: PlayerId,
    name: &str,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    packet: &SetCreativeSlot,
    stack: ItemStack,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    // Authoritative-creative gate: only a creative player may author slots.
    if inventory.game_mode() != GameMode::Creative {
        tracing::debug!(
            player = name,
            "ignoring set-creative-slot from non-creative player"
        );
        return Ok(());
    }
    // Bounds: -1 (drop) or anything outside 0..=45 is ignored.
    let Ok(index) = usize::try_from(packet.slot()) else {
        return Ok(());
    };
    if index >= SLOT_COUNT {
        return Ok(());
    }
    inventory.set_creative_slot(index, stack);

    // Echo the authoritative slot (mandatory) so the client view matches.
    let mut item_bytes = Vec::new();
    let Some(stored) = inventory.slot(index) else {
        return Ok(());
    };
    if let Err(err) = stored.encode_slot(&mut item_bytes) {
        tracing::warn!(player = name, %err, "failed to encode creative-slot echo");
        return Ok(());
    }
    send_mandatory(
        writer,
        debug,
        compression,
        &ctx.clock,
        ClientboundPlayPacket::SetContainerSlot(SetContainerSlot::new(
            WINDOW_ID,
            inventory.state_id(),
            packet.slot(),
            item_bytes,
        )),
    )?;

    // If the changed slot is mirrored into the player's visible equipment (held
    // main hand, off hand, or worn armor), rebroadcast the full equipment set so
    // viewers see the new piece — or its removal, since empty slots encode as air.
    // Non-equipment slots (storage grid, crafting) need no broadcast.
    if inventory.is_equipment_slot(index) {
        match inventory.equipment_body() {
            Ok(equipment) => {
                ctx.commands
                    .send(SimCommand::SetEquipment { player, equipment })
                    .await
                    .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
            }
            Err(err) => {
                tracing::warn!(
                    player = name,
                    %err,
                    "failed to encode equipment after creative slot set; skipping broadcast"
                );
            }
        }
    }
    Ok(())
}

/// Handles a serverbound Set Held Item: update the selected hotbar index and, on a
/// real change, broadcast the new main-hand item to viewers.
///
/// The wire slot is an `i16`; values outside `0..=8` are ignored (the client
/// already moved its own selector, so no clientbound reply is needed and nothing
/// is broadcast). On a valid change the new main-hand equipment body is encoded
/// from the connection-local inventory and routed as a [`SimCommand::SetEquipment`]
/// so the driver-owned router relays it (droppable) to the viewers that have this
/// player spawned — the inventory stays connection-local; only the opaque body
/// crosses to the router. A failed encode is logged and skipped (never fatal).
async fn handle_set_held_item(
    ctx: &ConnContext,
    player: PlayerId,
    inventory: &mut PlayerInventory,
    packet: &ServerboundSetHeldItem,
) -> anyhow::Result<()> {
    let Ok(slot) = u8::try_from(packet.slot()) else {
        return Ok(());
    };
    // Only broadcast when the selection actually changed (an out-of-range slot
    // leaves `selected` untouched and returns false).
    if !inventory.set_selected(slot) {
        return Ok(());
    }
    let equipment = match inventory.equipment_body() {
        Ok(body) => body,
        Err(err) => {
            tracing::warn!(%err, "failed to encode held-item equipment; skipping broadcast");
            return Ok(());
        }
    };
    ctx.commands
        .send(SimCommand::SetEquipment { player, equipment })
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))
}

/// Handles a serverbound Click Container.
///
/// On the open chest window, the SAFE click subset is applied directly:
/// **normal-mode left-click** (`mode = 0`, `button = 0`) on a chest or
/// player-inventory slot runs the item-count-conserving exchange (pickup / place /
/// merge / swap — chest mutations round-trip atomically to the simulation). EVERY
/// other case — right-click, shift-click, drag, number-key swap, double-click,
/// drop-outside, an out-of-window slot, or a malformed body — is treated as a
/// no-op and the whole window is resynced (authoritative `SetContainerContent`)
/// rather than guessed. Window and state IDs must match before either slot
/// section can mutate. Stale
/// IDs refresh the chest snapshot and resync the current window; clicks for an
/// unknown window without an open chest resync the player inventory. The client's
/// claimed result is never trusted. With no chest open, window 0 accepts normal
/// left/right clicks in writable player slots; result and armor insertion remain
/// unavailable, and unsupported actions resync. If the chest becomes inaccessible, exchanges
/// are refused and the last snapshot/cursor are retained; server-driven closure
/// of that screen is not yet modelled.
#[allow(clippy::too_many_arguments)] // one click step: identity + window state + I/O + trace
async fn handle_window_click(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    packet: &WindowClick,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    // Validate both IDs before parsing or mutating either chest or player slots.
    // Even window 0 is stale while a chest is the active container screen.
    if let Some(open) = window_state.open() {
        if inventory.game_mode() != GameMode::Spectator
            && packet.window_id() == open.window_id()
            && packet.state_id() == open.state_id()
        {
            apply_chest_window_click(ctx, player, inventory, window_state, packet).await?;
        } else {
            refresh_chest_window(ctx, player, window_state).await?;
        }
        return resync_chest_window(ctx, writer, inventory, window_state, debug, compression);
    }
    // Window 0 exchanges use server-owned slots/cursor, never client hashes.
    let changed_slot = apply_player_window_click(inventory, packet)?;
    if changed_slot.is_none() {
        inventory.bump_state_id();
    }
    if changed_slot.is_some_and(|index| inventory.is_equipment_slot(index)) {
        let equipment = inventory.equipment_body()?;
        ctx.commands
            .send(SimCommand::SetEquipment { player, equipment })
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
    }
    let payload = match inventory.container_content_payload() {
        Ok(payload) => payload,
        Err(err) => {
            tracing::warn!(%err, "failed to encode container-content resync");
            return Ok(());
        }
    };
    send_mandatory(
        writer,
        debug,
        compression,
        &ctx.clock,
        ClientboundPlayPacket::SetContainerContent(SetContainerContent::new(
            WINDOW_ID,
            inventory.state_id(),
            payload,
        )),
    )
}

/// Validates window/state and supports only ordinary left/right player clicks.
/// A malformed prefix, unsupported mode, result/armor insertion, outside slot or
/// spectator action leaves all items untouched for the caller's full resync.
fn apply_player_window_click(
    inventory: &mut PlayerInventory,
    packet: &WindowClick,
) -> Result<Option<usize>, ferrumc_items::ItemValidationError> {
    if packet.window_id() != WINDOW_ID || packet.state_id() != inventory.state_id() {
        return Ok(None);
    }
    let Some((slot, button, 0)) = decode_click_prefix(packet.rest()) else {
        return Ok(None);
    };
    let Ok(index) = usize::try_from(slot) else {
        return Ok(None);
    };
    Ok(inventory.normal_click(index, button)?.then_some(index))
}

/// Applies the SAFE subset of a click on the open chest window to the
/// authoritative state, mutating the chest mirror / cursor / player inventory.
///
/// Only normal-mode left-click is applied; any other mode/button, an out-of-window
/// slot, or a malformed body is a no-op (the caller's resync heals the client).
/// The server recomputes the outcome from its own authoritative state and never
/// trusts the click body's trailing changed-slots / cursor (`HashedSlot`) fields,
/// which are not even parsed.
async fn apply_chest_window_click(
    ctx: &ConnContext,
    player: PlayerId,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    packet: &WindowClick,
) -> anyhow::Result<()> {
    // Parse only the fixed prefix: slot (i16), button (i8), mode (varint). A
    // truncated/malformed body resyncs (no-op here).
    let Some((slot, button, mode)) = decode_click_prefix(packet.rest()) else {
        refresh_chest_window(ctx, player, window_state).await?;
        return Ok(());
    };
    // Normal-mode left-click only; everything else resyncs.
    if mode != 0 || button != 0 {
        refresh_chest_window(ctx, player, window_state).await?;
        return Ok(());
    }
    let Some(target) = OpenContainer::classify_slot(slot) else {
        refresh_chest_window(ctx, player, window_state).await?;
        return Ok(());
    };
    match target {
        WindowSlot::Chest(chest_slot) => {
            // Round-trip to the sim for an atomic, conserving chest mutation. Send
            // the current cursor; adopt the authoritative result.
            let Some(open) = window_state.open() else {
                return Ok(());
            };
            let position = open.position();
            let cursor = inventory.carried().clone();
            let expected_slots = open.chest_slots().to_vec();
            let (reply_tx, reply_rx) = oneshot::channel();
            ctx.commands
                .send(SimCommand::ContainerLeftClick {
                    player,
                    position,
                    slot: chest_slot,
                    cursor,
                    expected_slots,
                    reply: reply_tx,
                })
                .await
                .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
            let outcome = reply_rx
                .await
                .map_err(|_| anyhow::anyhow!("simulation driver dropped the container reply"))?;
            // An inaccessible chest has no fresh snapshot. Keep the last mirror
            // and cursor intact; automatic screen closure is not modelled yet.
            if let (Some((new_cursor, snapshot)), Some(open)) = (outcome, window_state.open_mut()) {
                inventory.set_carried(new_cursor);
                open.set_chest_slots(snapshot);
            }
        }
        WindowSlot::Player(index) => {
            // Another viewer may have changed the chest since this state ID was
            // sent. Heal that stale screen before allowing its local slot click.
            if !refresh_chest_window(ctx, player, window_state).await? {
                return Ok(());
            }
            // A player-inventory slot shown in the window: a purely local conserving
            // exchange against the connection-owned inventory and cursor.
            inventory.exchange_carried(index);
        }
    }
    Ok(())
}

/// Refreshes the open chest mirror through its owning driver. Returns true only
/// when the viewer's snapshot was current, so a stale player-slot click is a
/// no-op. An inaccessible chest preserves the cursor and refuses the exchange.
async fn refresh_chest_window(
    ctx: &ConnContext,
    player: PlayerId,
    window_state: &mut WindowState,
) -> anyhow::Result<bool> {
    let Some(open) = window_state.open() else {
        return Ok(false);
    };
    let (reply, response) = oneshot::channel();
    ctx.commands
        .send(SimCommand::OpenContainer {
            player,
            position: open.position(),
            reply,
        })
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
    let snapshot = response
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver dropped the container refresh reply"))?;
    if let (Some(snapshot), Some(open)) = (snapshot, window_state.open_mut()) {
        let unchanged = open.chest_slots() == snapshot;
        open.set_chest_slots(snapshot);
        return Ok(unchanged);
    }
    Ok(false)
}

/// Decodes the fixed prefix of a Click Container body: slot (`i16`), button
/// (`i8`), and mode (`VarInt`).
///
/// Returns `None` for a truncated or malformed body (the caller then resyncs). It
/// never panics on hostile input: [`BoundedReader`] rejects a short read or an
/// over-long `VarInt` with an error rather than reading out of bounds. The trailing
/// changed-slots / cursor (`HashedSlot`) fields are deliberately NOT decoded — the
/// server recomputes the click outcome from its own authoritative state and never
/// trusts the client's claimed result.
fn decode_click_prefix(rest: &[u8]) -> Option<(i16, i8, i32)> {
    let mut reader = BoundedReader::new(rest);
    let slot = reader.read_i16().ok()?;
    let button = reader.read_i8().ok()?;
    let mode = reader.read_var_int().ok()?;
    Some((slot, button, mode))
}

/// Resyncs the open chest window: bumps its state id and re-sends the full
/// authoritative `SetContainerContent` (chest slots, player section, cursor).
///
/// This is the single confirm/heal path after handled or refused clicks. Callers
/// refresh the snapshot while the chest is accessible; an inaccessible chest
/// retains its last mirror and cursor until the client closes the screen.
fn resync_chest_window(
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    inventory: &PlayerInventory,
    window_state: &mut WindowState,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    let Some(open) = window_state.open_mut() else {
        return Ok(());
    };
    open.bump_state_id();
    let payload = match open.content_payload(inventory) {
        Ok(payload) => payload,
        Err(err) => {
            tracing::warn!(%err, "failed to encode chest-window resync");
            return Ok(());
        }
    };
    send_mandatory(
        writer,
        debug,
        compression,
        &ctx.clock,
        ClientboundPlayPacket::SetContainerContent(SetContainerContent::new(
            open.window_id(),
            open.state_id(),
            payload,
        )),
    )
}

/// Opens a chest window: assigns a window id, stores the open state seeded with the
/// sim's slot snapshot, and sends `OpenScreen` + the initial `SetContainerContent`.
///
/// If a container is already open, it is closed first (returning its cursor) so a
/// carried item is never discarded when one window replaces another.
#[allow(clippy::too_many_arguments)] // one open step: identity + I/O + the chest snapshot
async fn open_chest_window(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    position: BlockPos,
    slots: Vec<ItemStack>,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    if window_state.open().is_some() {
        close_open_container(
            ctx,
            player,
            writer,
            inventory,
            window_state,
            debug,
            compression,
        )
        .await?;
    }
    let window_id = window_state.open_chest(position, slots);
    // OpenScreen first (generic_9x3, titled "Chest"), then the initial contents.
    send_mandatory(
        writer,
        debug,
        compression,
        &ctx.clock,
        open_screen(window_id, GENERIC_9X3_TYPE, &TextComponent::text("Chest")),
    )?;
    resync_chest_window(ctx, writer, inventory, window_state, debug, compression)
}

/// Handles a serverbound Close Container: only the current window's exact ID may
/// close it and return its cursor. A delayed close for an earlier window is a no-op.
#[allow(clippy::too_many_arguments)] // one close step: identity + window state + I/O + trace
async fn handle_close_container(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    packet: &CloseContainer,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    match window_state.open() {
        Some(open) if open.window_id() == packet.window_id() => {}
        None if packet.window_id() == WINDOW_ID => {}
        _ => return Ok(()),
    }
    close_open_container(
        ctx,
        player,
        writer,
        inventory,
        window_state,
        debug,
        compression,
    )
    .await
}

/// Closes window metadata, attempts to return the player-owned carried stack,
/// retains overflow, and resyncs window 0 and the broadcast equipment.
async fn close_open_container(
    ctx: &ConnContext,
    player: PlayerId,
    writer: &mut PlayWriter,
    inventory: &mut PlayerInventory,
    window_state: &mut WindowState,
    debug: &mut SessionDebug,
    compression: &CompressionState,
) -> anyhow::Result<()> {
    window_state.take();
    // The inventory owns the cursor independently of this retired screen. A full
    // bag retains its remainder for the next screen or durable leave snapshot.
    inventory.return_carried();
    inventory.return_crafting_inputs();
    // Resync window 0 so the client sees both the returned cursor and any inventory
    // changes the session's player-slot clicks made.
    inventory.bump_state_id();
    let payload = match inventory.container_content_payload() {
        Ok(payload) => payload,
        Err(err) => {
            tracing::warn!(%err, "failed to encode inventory resync on container close");
            return Ok(());
        }
    };
    send_mandatory(
        writer,
        debug,
        compression,
        &ctx.clock,
        ClientboundPlayPacket::SetContainerContent(SetContainerContent::new(
            WINDOW_ID,
            inventory.state_id(),
            payload,
        )),
    )?;
    // The held hotbar slot may have changed during the session; refresh the
    // broadcast equipment so viewers see the right main-hand item (the full set is
    // sent, so off-hand/armor stay in sync too).
    match inventory.equipment_body() {
        Ok(equipment) => ctx
            .commands
            .send(SimCommand::SetEquipment { player, equipment })
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver is gone")),
        Err(err) => {
            tracing::warn!(%err, "failed to encode equipment after container close");
            Ok(())
        }
    }
}

/// Answers a serverbound tab-complete request, enqueuing a `TabCompleteResponse`
/// built from the command tree's suggestion engine.
///
/// The request `text` is the full chat-box content including the leading `/`; the
/// slash is stripped before suggesting, and `start`/`length` are computed so the
/// client replaces exactly the in-progress token. Suggestions are filtered to the
/// literals the player's permission level *and* granted permission nodes allow
/// (matching the permission-filtered command graph the join kit declared), and
/// argument *hints* such as `<mode: 0..3>` are dropped — only concrete literal
/// completions are sent, never placeholder text the client would insert verbatim.
/// The offsets are UTF-16 code units (the positions used by Java and Brigadier), so a non-ASCII prefix is indexed correctly.
#[allow(clippy::too_many_arguments)] // one requester context, absolute deadline and correlated reply
async fn handle_tab_complete(
    ctx: &ConnContext,
    player: PlayerId,
    name: &str,
    writer: &mut PlayWriter,
    transaction_id: i32,
    text: &str,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    deadline: Option<ferrumc_net::LivenessDeadline>,
) -> anyhow::Result<()> {
    let level = ctx.policy.permission_level(player);
    let allowed = |node: &str| ctx.command_permission_allowed(player, node);
    let (start, length, mut suggestions) =
        tab_complete_reply(ctx.policy.command_tree(), level, &allowed, text);
    if let Some((label, args)) = java_completion_input(text) {
        if let Some(bridge) = &ctx.java_bridge {
            if let Some(descriptor) = bridge.find_command(label) {
                suggestions.clear();
                if descriptor
                    .permission
                    .as_ref()
                    .is_none_or(|node| ctx.policy.java_permission_allowed(player, node))
                {
                    match guard_java_completion(
                        deadline,
                        ctx.shutdown.clone(),
                        bridge.complete(label, args, ctx.java_player(player, name)),
                    )
                    .await?
                    {
                        Ok(completion) => {
                            suggestions = completion.suggestions;
                            super::outbound::send_java_messages(
                                ctx,
                                writer,
                                debug,
                                compression,
                                completion.messages,
                            )?;
                        }
                        Err(error) => {
                            tracing::warn!(%error, player = name, "Java completion failed");
                            super::outbound::send_java_feedback(
                                ctx,
                                writer,
                                debug,
                                compression,
                                &error.to_string(),
                            )?;
                        }
                    }
                }
            }
        }
    }

    let matches: Vec<CommandSuggestionMatch> = suggestions
        .into_iter()
        .filter_map(|suggestion| {
            BoundedString::<32_767>::new(suggestion)
                .ok()
                // MVP: no hover tooltip (a single absent-flag byte on the wire).
                .map(|suggestion| CommandSuggestionMatch::new(suggestion, None))
        })
        .collect();

    let response = TabCompleteResponse::new(
        transaction_id,
        i32::try_from(start).unwrap_or(i32::MAX),
        i32::try_from(length).unwrap_or(i32::MAX),
        matches,
    );
    enqueue_traced_classified(
        writer,
        debug,
        compression,
        &ctx.clock,
        ClientboundPlayPacket::TabCompleteResponse(response),
    );
    Ok(())
}

/// Completion changes no Rust inventory transaction, so cancellation may drop
/// its RPC. The actor then kills/reaps active work before accepting a new ID.
async fn guard_java_completion<T>(
    deadline: Option<ferrumc_net::LivenessDeadline>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    work: impl std::future::Future<Output = Result<T, crate::java_bridge::BridgeError>>,
) -> anyhow::Result<Result<T, crate::java_bridge::BridgeError>> {
    anyhow::ensure!(
        !*shutdown.borrow(),
        "server shutdown during Java completion"
    );
    tokio::select! {
        biased;
        _ = shutdown.changed() => Err(anyhow::anyhow!("server shutdown during Java completion")),
        result = super::play::guard_play_work(deadline, async { Ok(work.await) }) => result,
    }
}

/// Matches Bukkit's literal-space split, retaining interior/trailing empty args.
fn java_completion_input(text: &str) -> Option<(&str, Vec<String>)> {
    let input = text.strip_prefix('/').unwrap_or(text);
    let boundary = input.find(' ')?;
    let label = &input[..boundary];
    if label.is_empty() {
        return None;
    }
    let args: Vec<String> = input[boundary + 1..]
        .split(' ')
        .take(65)
        .map(str::to_owned)
        .collect();
    Some((label, args))
}

/// Computes the tab-complete reply for `text` at permission `level` against
/// `tree`, gating permission-node-declared commands through `is_allowed`: the
/// `(start, length)` UTF-16 span of `text` the matches replace, and the
/// filtered list of literal completions.
///
/// Pure (no I/O), so it is unit-tested directly. The leading `/` is stripped
/// before suggesting; `start`/`length` delimit the in-progress token (from after
/// the last whitespace to the end of `text`). They are reported in UTF-16 code units, so non-BMP characters use the same
/// indices as the Java client rather than UTF-8 bytes or Unicode scalar counts. Matches are filtered to
/// the literals the player's `level` and granted permission nodes allow (the
/// declared graph is filtered the same way), and argument *hints* (which begin with
/// `<`) are dropped so the client is never sent placeholder text to insert verbatim.
fn tab_complete_reply(
    tree: &CommandTree,
    level: u8,
    is_allowed: &dyn Fn(&str) -> bool,
    text: &str,
) -> (usize, usize, Vec<String>) {
    let input = text.strip_prefix('/').unwrap_or(text);
    let offset = text.len() - input.len();
    // Bukkit and Brigadier separate argument tokens with a literal ASCII space.
    let token_start = input.rfind(' ').map_or(0, |idx| idx + 1);
    let start_bytes = offset + token_start;
    // Brigadier ranges use Java UTF-16 indices, including surrogate pairs.
    let start = text[..start_bytes].encode_utf16().count();
    let length = text[start_bytes..].encode_utf16().count();

    let graph = tree.to_brigadier(level, is_allowed);
    let allowed: Vec<&str> = graph
        .nodes()
        .iter()
        .filter_map(|node| node.name())
        .collect();
    let matches = tree
        .suggest(input)
        .into_iter()
        .filter(|suggestion| !suggestion.starts_with('<'))
        .filter(|suggestion| allowed.contains(&suggestion.as_str()))
        .collect();
    (start, length, matches)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use bytes::BytesMut;
    use ferrumc_codec::{write_var_int, BoundedReader, BoundedString};
    use ferrumc_config::AccessConfig;
    use ferrumc_core::{GameMode, PlayerId, PluginId};
    use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
    use ferrumc_math::{BlockPos, Direction, ShardPos, Vec3, WorldIntent};
    use ferrumc_net::{CompressionState, ConnectionLimits, DisconnectReason, PlayWriter};
    use ferrumc_observability::{CounterRegistry, NetTelemetryHub, ServerClock, SessionDebug};
    use ferrumc_plugin_api::{
        BlockPlaceAttempt, Capability, CapabilityManifest, EventContext, EventKind,
        InteractAttempt, Plugin, PluginBlockDecision, PluginError, PluginEvent,
        PluginEventDecision, PluginMetadata, SetupContext, Version,
    };
    use ferrumc_plugin_host::PluginHost;
    use ferrumc_proto::generated::play::{
        ClientboundPlayPacket, CloseContainer, ConfirmTeleportation, GameEvent,
        ServerboundKeepAlive, ServerboundPlayPacket, SetCenterChunk, SetCreativeSlot,
        SetPlayerPosition, SetPlayerPositionAndRotation, SetPlayerRotation, UpdateSign, UseItemOn,
        WindowClick,
    };
    use ferrumc_proto::types::BlockPosition;
    use ferrumc_sim::SimShard;
    use tokio::sync::mpsc;

    use super::{
        decode_click_prefix, decode_play_body, handle_play_body, route_emitted_intents,
        tab_complete_reply, validated_movement, MovementObservation, MovementValidation,
        PlayBodyDecode,
    };
    use crate::command::{build_command_tree, GAMEMODE_COMMAND, SPAWN_COMMAND};
    use crate::config::AppConfig;
    use crate::connection::chunk_stream::{apply_chunk_stream, ChunkStream};
    use crate::connection::context::{build_status_response, ConnContext};
    use crate::connection::ingress::IngressFence;
    use crate::connection::play::{play_liveness, pump_serverbound};
    use crate::connection::rate_limiter::ChatRateLimiter;
    use crate::connection::serverbound_budget::ServerboundBudget;
    use crate::connection::teleport::PendingTeleport;
    use crate::driver::SimCommand;
    use crate::inventory::{PlayerInventory, HOTBAR_START};
    use crate::plugins::{build_play_policy, BlockEventDispatcher};
    use crate::registries::ConfigRegistries;
    use crate::window::WindowState;
    use crate::world::build_world;

    fn strict_body_disconnect_reason(body: &[u8]) -> DisconnectReason {
        let Err(error) = decode_play_body(body) else {
            panic!("body unexpectedly decoded");
        };
        DisconnectReason::from_disconnect_class(error.disconnect_class())
    }

    #[test]
    fn strict_body_decode_preserves_disconnect_classes_and_compatibility() {
        assert_eq!(
            strict_body_disconnect_reason(&[]),
            DisconnectReason::MalformedPacket,
        );

        let mut trailing = BytesMut::new();
        ServerboundKeepAlive::new(9)
            .encode(&mut trailing)
            .expect("Keep Alive encodes");
        trailing.extend_from_slice(&[0xDE, 0xAD]);
        assert_eq!(
            strict_body_disconnect_reason(&trailing),
            DisconnectReason::ProtocolViolation,
        );

        let mut invalid_id = Vec::new();
        write_var_int(&mut invalid_id, 0x77);
        assert_eq!(
            strict_body_disconnect_reason(&invalid_id),
            DisconnectReason::ProtocolViolation,
        );

        let mut nested_item = Vec::new();
        write_var_int(&mut nested_item, 1);
        write_var_int(&mut nested_item, 1);
        write_var_int(&mut nested_item, 0);
        write_var_int(&mut nested_item, 0);
        nested_item.extend_from_slice(&[0xDE, 0xAD]);
        let mut creative = BytesMut::new();
        SetCreativeSlot::new(9, nested_item)
            .encode(&mut creative)
            .expect("creative slot encodes");
        assert_eq!(
            strict_body_disconnect_reason(&creative),
            DisconnectReason::ProtocolViolation,
        );

        let mut tick_end = Vec::new();
        write_var_int(&mut tick_end, 0x0c);
        assert!(matches!(
            decode_play_body(&tick_end),
            Ok(PlayBodyDecode::Unmodelled { id: 0x0c })
        ));
    }

    /// Runs the real packet handler with a socket-free fixture. These packets
    /// should never enqueue simulation work or outbound gameplay effects.
    fn handle_liveness_packet<'call>(
        ctx: &'call ConnContext,
        liveness: &'call mut ferrumc_net::ConnectionLiveness,
        packet: ServerboundPlayPacket,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + 'call {
        // The complete handler fixture owns an inventory and session trace;
        // keep that future off the test task's stack.
        Box::pin(async move {
            let mut body = BytesMut::new();
            packet.encode(&mut body)?;
            let mut writer = PlayWriter::with_defaults(ctx.limits);
            let result = handle_play_body(
                ctx,
                PlayerId::offline("LivenessProbe"),
                "LivenessProbe",
                &mut writer,
                &mut ChunkStream::new(ctx),
                &mut ChatRateLimiter::new(ctx.clock.now()),
                &mut PlayerInventory::with_creative_kit(GameMode::Creative),
                &mut WindowState::new(),
                &mut 0.0,
                &mut 0.0,
                &body,
                &mut SessionDebug::new("LivenessProbe"),
                &CompressionState::disabled(),
                liveness,
                &mut PendingTeleport::default(),
            )
            .await;
            assert_eq!(writer.total_queued(), 0);
            result
        })
    }

    #[tokio::test(start_paused = true)]
    async fn keep_alive_echo_is_accepted_once_and_must_match() {
        use ferrumc_net::KeepAliveError;
        use tokio::time::Instant;

        let (commands, mut inbox) = mpsc::channel(8);
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let echo = ServerboundPlayPacket::ServerboundKeepAlive(ServerboundKeepAlive::new(7));
        let mut liveness = play_liveness(Instant::now(), ctx.io_timeout).expect("Play tracker");
        let error = handle_liveness_packet(&ctx, &mut liveness, echo.clone())
            .await
            .expect_err("unsolicited Keep Alive must terminate the connection");
        assert_eq!(
            error.downcast_ref(),
            Some(&KeepAliveError::Unexpected { received: 7 })
        );

        let mut liveness = play_liveness(Instant::now(), ctx.io_timeout).expect("Play tracker");
        liveness
            .keep_alive_sent(8, Instant::now())
            .expect("request sent");
        let error = handle_liveness_packet(&ctx, &mut liveness, echo.clone())
            .await
            .expect_err("wrong ID must terminate the connection");
        assert_eq!(
            error.downcast_ref(),
            Some(&KeepAliveError::Mismatched {
                expected: 8,
                received: 7
            })
        );

        let mut liveness = play_liveness(Instant::now(), ctx.io_timeout).expect("Play tracker");
        liveness
            .keep_alive_sent(7, Instant::now())
            .expect("request sent");
        handle_liveness_packet(&ctx, &mut liveness, echo.clone())
            .await
            .expect("matching echo");
        assert_eq!(liveness.outstanding_keep_alive(), None);
        let error = handle_liveness_packet(&ctx, &mut liveness, echo)
            .await
            .expect_err("replay must terminate the connection");
        assert_eq!(
            error.downcast_ref(),
            Some(&KeepAliveError::Unexpected { received: 7 })
        );
        assert!(
            inbox.try_recv().is_err(),
            "Keep Alive never reaches simulation"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn ordinary_packet_cannot_extend_or_satisfy_keep_alive_deadline() {
        use ferrumc_net::{KeepAliveError, LivenessTimeout};
        use std::time::Duration;
        use tokio::time::{advance, Instant};

        let (commands, mut inbox) = mpsc::channel(8);
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let mut liveness = play_liveness(Instant::now(), ctx.io_timeout).expect("Play tracker");
        liveness
            .keep_alive_sent(7, Instant::now())
            .expect("request sent");
        let deadline = liveness.next_deadline().expect("response deadline");
        advance(Duration::from_secs(14)).await;
        handle_liveness_packet(
            &ctx,
            &mut liveness,
            ServerboundPlayPacket::ConfirmTeleportation(ConfirmTeleportation::new(1)),
        )
        .await
        .expect("ordinary valid packet");
        assert_eq!(liveness.next_deadline(), Some(deadline));
        assert!(liveness.outstanding_keep_alive().is_some());
        advance(Duration::from_secs(1)).await;
        let error = handle_liveness_packet(
            &ctx,
            &mut liveness,
            ServerboundPlayPacket::ServerboundKeepAlive(ServerboundKeepAlive::new(7)),
        )
        .await
        .expect_err("an echo at the deadline is late");
        assert_eq!(
            error.downcast_ref(),
            Some(&KeepAliveError::TimedOut {
                timeout: LivenessTimeout::KeepAlive { id: 7 },
            })
        );
        assert!(inbox.try_recv().is_err());
    }

    /// Invokes the production frame pump; the fixture's outgoing producer stays
    /// continuously ready while exact Keep Alive handling is inspected.
    #[allow(clippy::too_many_arguments)] // actual pump state retained across each finite output turn
    async fn pump_probe(
        ctx: &ConnContext,
        decoder: &mut ferrumc_net::InboundDecoder,
        liveness: &mut ferrumc_net::ConnectionLiveness,
        pending: &mut PendingTeleport,
        ingress: &mut IngressFence,
        handle: &mut ferrumc_session::PlayerSessionHandle,
        writer: &mut PlayWriter,
    ) {
        pump_serverbound(
            decoder,
            &CompressionState::disabled(),
            ctx,
            PlayerId::offline("PumpProbe"),
            "PumpProbe",
            writer,
            &mut ChunkStream::new(ctx),
            &mut ChatRateLimiter::new(ctx.clock.now()),
            &mut ServerboundBudget::new(
                std::time::Instant::now(),
                ctx.budget.sustained_rate,
                ctx.budget.burst,
            ),
            &mut PlayerInventory::with_creative_kit(GameMode::Creative),
            &mut WindowState::new(),
            &mut 0.0,
            &mut 0.0,
            &mut SessionDebug::new("PumpProbe"),
            liveness,
            pending,
            ingress,
            handle,
        )
        .await
        .expect("production pump");
    }

    fn frame_packet(packet: &ServerboundPlayPacket) -> Vec<u8> {
        let mut body = BytesMut::new();
        packet.encode(&mut body).expect("packet encodes");
        let mut frame = Vec::new();
        write_var_int(
            &mut frame,
            i32::try_from(body.len()).expect("bounded packet"),
        );
        frame.extend_from_slice(&body);
        frame
    }

    #[tokio::test(start_paused = true)]
    async fn exact_keep_alive_head_bypasses_ready_output_with_or_without_pending_teleport() {
        use ferrumc_session::{SessionRouter, DEFAULT_OUTBOUND_CAPACITY};
        for teleport_pending in [false, true] {
            let (commands, mut inbox) = mpsc::channel(8);
            let ctx = handler_context(commands, PluginHost::in_memory()).await;
            let player = PlayerId::offline("PumpProbe");
            let mut router = SessionRouter::new();
            let _shard = router.register_shard(ShardPos::new(0, 0));
            let mut handle = router
                .join_player(player, "PumpProbe", Vec3::new(8.0, 64.0, 8.0))
                .expect("join");
            for _ in 0..DEFAULT_OUTBOUND_CAPACITY {
                router.send_play_packet_to(
                    player,
                    ClientboundPlayPacket::GameEvent(GameEvent::new(1, 0.0)),
                );
            }
            let mut pending = PendingTeleport::default();
            if teleport_pending {
                pending.issue(&super::super::spawn_sync(
                    0,
                    Vec3::new(8.0, 64.0, 8.0),
                    0.0,
                    0.0,
                ));
            }
            let mut liveness =
                play_liveness(tokio::time::Instant::now(), ctx.io_timeout).expect("tracker");
            liveness
                .keep_alive_sent(77, tokio::time::Instant::now())
                .expect("sent");
            let mut decoder = ferrumc_net::InboundDecoder::new(ctx.limits);
            decoder
                .push(&frame_packet(&ServerboundPlayPacket::ServerboundKeepAlive(
                    ServerboundKeepAlive::new(77),
                )))
                .expect("buffer echo");
            let mut ingress = IngressFence::default();
            let mut writer = PlayWriter::with_defaults(ctx.limits);
            Box::pin(pump_probe(
                &ctx,
                &mut decoder,
                &mut liveness,
                &mut pending,
                &mut ingress,
                &mut handle,
                &mut writer,
            ))
            .await;
            assert_eq!(liveness.outstanding_keep_alive(), None);
            assert_eq!(pending.is_pending(), teleport_pending);
            assert!(!ingress.is_fenced());
            assert!(
                handle.try_recv().is_some(),
                "ready producer output stayed queued"
            );
            assert!(inbox.try_recv().is_err());
            assert_eq!(writer.total_queued(), 0);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn producer_refills_cannot_extend_frame_fence_or_starve_buffered_echo() {
        use ferrumc_proto::generated::play::TabCompleteRequest;
        use ferrumc_session::{SessionRouter, DEFAULT_OUTBOUND_CAPACITY};
        let (commands, mut inbox) = mpsc::channel(8);
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let player = PlayerId::offline("PumpProbe");
        let mut router = SessionRouter::new();
        let _shard = router.register_shard(ShardPos::new(0, 0));
        let mut handle = router
            .join_player(player, "PumpProbe", Vec3::new(8.0, 64.0, 8.0))
            .expect("join");
        let output = ClientboundPlayPacket::GameEvent(GameEvent::new(1, 0.0));
        for _ in 0..DEFAULT_OUTBOUND_CAPACITY {
            router.send_play_packet_to(player, output.clone());
        }
        let mut decoder = ferrumc_net::InboundDecoder::new(ctx.limits);
        decoder
            .push(&frame_packet(&ServerboundPlayPacket::TabCompleteRequest(
                TabCompleteRequest::new(17, BoundedString::new("/sp".to_owned()).expect("query")),
            )))
            .expect("first input");
        decoder
            .push(&frame_packet(&ServerboundPlayPacket::ServerboundKeepAlive(
                ServerboundKeepAlive::new(77),
            )))
            .expect("buffered echo");
        let mut liveness =
            play_liveness(tokio::time::Instant::now(), ctx.io_timeout).expect("tracker");
        liveness
            .keep_alive_sent(77, tokio::time::Instant::now())
            .expect("sent");
        let mut pending = PendingTeleport::default();
        let mut ingress = IngressFence::default();
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        Box::pin(pump_probe(
            &ctx,
            &mut decoder,
            &mut liveness,
            &mut pending,
            &mut ingress,
            &mut handle,
            &mut writer,
        ))
        .await;
        assert!(ingress.is_fenced());
        for turn in 0..DEFAULT_OUTBOUND_CAPACITY {
            assert!(ingress.pop_outbound().is_some(), "prefix turn {turn}");
            // Replace each consumed message with a new one in the live channel.
            // A drain-until-empty strategy could never reach the buffered echo.
            router.send_play_packet_to(player, output.clone());
            Box::pin(pump_probe(
                &ctx,
                &mut decoder,
                &mut liveness,
                &mut pending,
                &mut ingress,
                &mut handle,
                &mut writer,
            ))
            .await;
            if turn + 1 < DEFAULT_OUTBOUND_CAPACITY {
                assert!(liveness.outstanding_keep_alive().is_some());
            }
        }
        assert_eq!(liveness.outstanding_keep_alive(), None);
        assert!(!ingress.is_fenced());
        assert!(
            handle.try_recv().is_some(),
            "producer is still ready after bounded fence"
        );
        assert_eq!(
            writer.total_queued(),
            1,
            "held tab request was actually dispatched"
        );
        assert!(inbox.try_recv().is_err());
    }

    #[tokio::test]
    async fn leave_mirror_applies_retained_then_queued_teleports_before_save_without_ack() {
        use ferrumc_session::SessionRouter;
        let (commands, _inbox) = mpsc::channel(8);
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let player = PlayerId::offline("LeaveTeleport");
        let mut router = SessionRouter::new();
        let _shard = router.register_shard(ShardPos::new(0, 0));
        let mut handle = router
            .join_player(player, "LeaveTeleport", Vec3::new(8.0, 64.0, 8.0))
            .expect("join");
        let earlier = Vec3::new(16.0, 64.0, 8.0);
        router.send_play_packet_to(
            player,
            ClientboundPlayPacket::SynchronizePlayerPosition(super::super::spawn_sync(
                1, earlier, 31.0, -17.0,
            )),
        );
        let mut ingress = IngressFence::default();
        ingress.hold(bytes::Bytes::from_static(&[0, 1]), &mut handle);
        assert!(ingress.is_fenced());
        let latest = Vec3::new(20.0, 64.0, 8.0);
        router.send_play_packet_to(
            player,
            ClientboundPlayPacket::SynchronizePlayerPosition(super::super::spawn_sync(
                1, latest, 45.0, -25.0,
            )),
        );
        let mut stream = ChunkStream::new(&ctx);
        stream.observe(Vec3::new(8.0, 64.0, 8.0));
        let mut yaw = 0.0;
        let mut pitch = 0.0;
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let mut pending = PendingTeleport::default();
        super::super::play::mirror_queued_for_leave(
            &mut handle,
            &mut ingress,
            &mut stream,
            &mut inventory,
            &mut yaw,
            &mut pitch,
            &mut pending,
        );
        assert_eq!(stream.last_position(), Some(latest));
        assert_eq!(stream.pending_position_for_test(), Some(latest));
        assert_eq!((yaw, pitch), (45.0, -25.0));
        assert!(pending.is_pending());
        assert!(!ingress.is_fenced());
        assert!(handle.try_recv().is_none());
    }

    /// Test plugin that makes a `PlayerMove` callback observable without changing
    /// production metrics or relying on captured logs.
    struct MoveProbe {
        calls: Arc<AtomicUsize>,
    }

    impl Plugin for MoveProbe {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new(
                PluginId::new("packet26-move-probe"),
                "packet26-move-probe",
                Version::new(0, 1, 0),
                CapabilityManifest::empty().with(Capability::ReceiveEvents),
            )
        }

        fn on_enable(&mut self, ctx: &mut SetupContext<'_>) -> Result<(), PluginError> {
            ctx.events()?.subscribe(EventKind::PlayerMove);
            Ok(())
        }

        fn on_event(&mut self, event: &PluginEvent, _ctx: &mut EventContext<'_>) {
            if matches!(event, PluginEvent::PlayerMove { .. }) {
                self.calls.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Builds the real handler context around a caller-supplied bounded command
    /// route and plugin host.
    async fn handler_context(commands: mpsc::Sender<SimCommand>, host: PluginHost) -> ConnContext {
        let config = AppConfig::from_toml_str("spawn_chunk_radius = 0\nview_distance = 0\n")
            .expect("handler test config is valid");
        let setup = build_world(&config, ShardPos::new(0, 0))
            .await
            .expect("build one-column handler world");
        let (policy, _default_dispatcher) =
            build_play_policy(&config).expect("build handler policy");

        let access = AccessConfig::default()
            .resolve(Path::new("."))
            .expect("resolve default access policy");
        ConnContext {
            shutdown: tokio::sync::watch::channel(false).1,
            limits: ConnectionLimits::default(),
            io_timeout: config.io_timeout(),
            compression_threshold: config.compression_threshold(),
            join_kit: setup.join_kit,
            config: Arc::new(ConfigRegistries::build().expect("build registries")),
            keep_alive_interval: config.keep_alive_interval(),
            chunk_stream_interval: config.chunk_stream_interval(),
            commands,
            player_store: setup.player_store,
            player_admissions: crate::player_admission::PlayerAdmissions::new(
                config.max_connections(),
            ),
            policy: Arc::new(policy),
            block_events: Arc::new(BlockEventDispatcher::new(host)),
            java_bridge: None,
            status_response: Arc::new(build_status_response(1).expect("build status response")),
            view_distance: config.view_distance(),
            metrics: Arc::new(CounterRegistry::new()),
            clock: ServerClock::new(),
            net_telemetry: Arc::new(NetTelemetryHub::new()),
            access: Arc::new(access),
            budget: config.budget(),
        }
    }

    /// Drives the complete decode/dispatch path without sockets while retaining
    /// the inventory, cursor, window, and encoded outbound resync for inspection.
    fn handle_container_packet<'call>(
        ctx: &'call ConnContext,
        inventory: &'call mut PlayerInventory,
        windows: &'call mut WindowState,
        writer: &'call mut PlayWriter,
        packet: ServerboundPlayPacket,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + 'call {
        Box::pin(async move {
            let mut body = BytesMut::new();
            packet.encode(&mut body)?;
            handle_play_body(
                ctx,
                PlayerId::offline("ContainerProbe"),
                "ContainerProbe",
                writer,
                &mut ChunkStream::new(ctx),
                &mut ChatRateLimiter::new(ctx.clock.now()),
                inventory,
                windows,
                &mut 0.0,
                &mut 0.0,
                &body,
                &mut SessionDebug::new("ContainerProbe"),
                &CompressionState::disabled(),
                &mut play_liveness(tokio::time::Instant::now(), ctx.io_timeout)?,
                &mut PendingTeleport::default(),
            )
            .await
        })
    }

    fn container_probe_stack(count: u8) -> ItemStack {
        ItemStack::try_new(
            ItemId::from_name("stone").expect("stone item"),
            count,
            ComponentPatch::empty(),
        )
        .expect("valid probe stack")
    }

    fn container_probe_click(window: i32, state: i32, slot: i16) -> ServerboundPlayPacket {
        let mut rest = slot.to_be_bytes().to_vec();
        rest.push(0); // left button
        write_var_int(&mut rest, 0); // normal click
        write_var_int(&mut rest, 0); // no claimed changed slots
        write_var_int(&mut rest, 0); // claimed empty cursor is never trusted
        ServerboundPlayPacket::WindowClick(WindowClick::new(window, state, rest))
    }

    fn drain_container_resync(
        writer: &mut PlayWriter,
    ) -> ferrumc_proto::generated::play::SetContainerContent {
        let batch = writer
            .drain_batch(&CompressionState::disabled())
            .expect("resync encodes");
        assert_eq!(batch.frame_count(), 1, "exactly one mandatory resync");
        let mut framed = BoundedReader::new(batch.bytes());
        let length = usize::try_from(framed.read_var_int().expect("frame length"))
            .expect("positive frame length");
        let body = framed.read_bytes(length).expect("whole frame");
        assert_eq!(framed.remaining(), 0);
        let mut reader = BoundedReader::new(body);
        let id = reader.read_var_int().expect("packet id");
        let ClientboundPlayPacket::SetContainerContent(content) =
            ClientboundPlayPacket::decode(id, &mut reader).expect("resync packet")
        else {
            panic!("expected container-content resync");
        };
        assert_eq!(reader.remaining(), 0);
        content
    }

    #[tokio::test]
    async fn player_click_rejections_resync_without_mutating_or_broadcasting() {
        let (commands, mut inbox) = mpsc::channel(1); // rejects send no simulation work
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Survival);
        inventory.set_carried(container_probe_stack(5));
        let mut windows = WindowState::new();
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        let claims = [
            (99, 0, 9, 0, 0),   // wrong window
            (0, -1, 9, 0, 0),   // stale state
            (0, 0, -999, 0, 0), // outside drop unsupported
            (0, 0, 46, 0, 0),   // outside player window
            (0, 0, 9, 2, 0),    // unsupported middle button
            (0, 0, 9, 0, 1),    // shift
            (0, 0, 9, 0, 2),    // number key
            (0, 0, 9, 0, 5),    // drag
            (0, 0, 9, 0, 6),    // double click
            (0, 0, 0, 0, 0),    // crafting result read-only
            (0, 0, 5, 1, 0),    // armor insertion unsupported
        ];
        for (window, offset, slot, button, mode) in claims {
            let before = inventory.container_content_payload().expect("before");
            let state = inventory.state_id();
            let mut rest = i16::try_from(slot)
                .expect("slot fits")
                .to_be_bytes()
                .to_vec();
            rest.push(u8::try_from(button).expect("button fits"));
            write_var_int(&mut rest, mode);
            // Fabricated claimed items are deliberately ignored.
            rest.extend_from_slice(&[0xFF, 0x01, 0x7F]);
            handle_container_packet(
                &ctx,
                &mut inventory,
                &mut windows,
                &mut writer,
                ServerboundPlayPacket::WindowClick(WindowClick::new(window, state + offset, rest)),
            )
            .await
            .expect("reject resync");
            assert_eq!(
                inventory.container_content_payload().expect("after"),
                before
            );
            let content = drain_container_resync(&mut writer);
            assert_eq!(content.window_id(), 0);
            assert_eq!(content.state_id(), state + 1);
            assert_eq!(content.payload(), before);
            assert!(inbox.try_recv().is_err());
        }
    }

    #[tokio::test]
    async fn player_equipment_click_broadcasts_the_new_authoritative_stack() {
        let (commands, mut inbox) = mpsc::channel(1); // one equipment update per click
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Survival);
        let mut windows = WindowState::new();
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        let state = inventory.state_id();
        handle_container_packet(
            &ctx,
            &mut inventory,
            &mut windows,
            &mut writer,
            container_probe_click(0, state, 36),
        )
        .await
        .expect("pickup held");
        assert!(inventory.held().item().is_none());
        assert_eq!(inventory.carried().count(), 64);
        assert_eq!(drain_container_resync(&mut writer).state_id(), state + 1);
        match inbox.try_recv().expect("equipment update") {
            SimCommand::SetEquipment { equipment, .. } => {
                assert_eq!(equipment, inventory.equipment_body().expect("equipment"));
                assert_eq!(equipment[1], 0, "main hand cleared");
            }
            _ => panic!("unexpected command"),
        }
    }

    #[tokio::test]
    async fn window_zero_close_returns_cursor_and_unfinished_inputs_once() {
        let (commands, mut inbox) = mpsc::channel(1); // one equipment refresh per close
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Survival);
        inventory.set_creative_slot(1, container_probe_stack(7));
        inventory.set_carried(container_probe_stack(5));
        let mut windows = WindowState::new();
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        let mut after_first = None;
        for _ in 0..2 {
            handle_container_packet(
                &ctx,
                &mut inventory,
                &mut windows,
                &mut writer,
                ServerboundPlayPacket::CloseContainer(CloseContainer::new(0)),
            )
            .await
            .expect("close player window");
            let content = drain_container_resync(&mut writer);
            if let Some(previous) = &after_first {
                assert_eq!(content.payload(), previous);
            } else {
                after_first = Some(content.payload().to_vec());
            }
            assert!(inventory.carried().item().is_none());
            assert!(inventory.slot(1).expect("crafting input").item().is_none());
            assert_eq!(inventory.slot(9).expect("deposit").count(), 12);
            assert!(matches!(
                inbox.try_recv(),
                Ok(SimCommand::SetEquipment { .. })
            ));
        }
    }

    #[tokio::test]
    async fn stale_container_ids_reject_both_slot_sections_before_mutation_and_refresh() {
        use ferrumc_world::CHEST_SLOTS;

        // Each wrong ID is tested against chest AND local player-slot mutations.
        // The refresh differs from the mirror, proving the reply is authoritative.
        let mut fresh = vec![ItemStack::empty(); CHEST_SLOTS];
        fresh[0] = container_probe_stack(7);
        let returned = fresh.clone();
        let (commands, mut inbox) = mpsc::channel(1); // one request at a time
        let probe = tokio::spawn(async move {
            let mut refreshes = 0;
            while let Some(command) = inbox.recv().await {
                let SimCommand::OpenContainer { reply, .. } = command else {
                    panic!("invalid IDs must not dispatch any mutation");
                };
                refreshes += 1;
                reply.send(Some(returned.clone())).expect("live handler");
            }
            refreshes
        });
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        for slot in [0, 27] {
            for (window_id, state_id) in [(1, 0), (1, 2), (1, -1), (2, 1), (0, 1)] {
                let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
                *inventory.slot_mut(9).expect("main slot") = container_probe_stack(4);
                let mut windows = WindowState::new();
                windows.open_chest(
                    BlockPos::new(8, 65, 8),
                    vec![ItemStack::empty(); CHEST_SLOTS],
                );
                inventory.set_carried(container_probe_stack(5));
                let inventory_before = inventory.container_content_payload().expect("inventory");
                let mut writer = PlayWriter::with_defaults(ctx.limits);
                handle_container_packet(
                    &ctx,
                    &mut inventory,
                    &mut windows,
                    &mut writer,
                    container_probe_click(window_id, state_id, slot),
                )
                .await
                .expect("stale click resyncs");
                assert_eq!(
                    inventory.container_content_payload().expect("inventory"),
                    inventory_before
                );
                let open = windows.open().expect("same window stays open");
                assert_eq!(inventory.carried(), &container_probe_stack(5));
                assert_eq!(open.chest_slots(), fresh);
                let content = drain_container_resync(&mut writer);
                assert_eq!((content.window_id(), content.state_id()), (1, 2));
                assert_eq!(
                    content.payload(),
                    open.content_payload(&inventory)
                        .expect("full authoritative window")
                );
            }
        }
        drop(ctx);
        assert_eq!(probe.await.expect("request probe finished"), 10);
    }

    #[tokio::test]
    async fn player_slot_click_requires_a_current_accessible_chest_snapshot() {
        use ferrumc_world::CHEST_SLOTS;

        for (conflicting_view, accessible) in [(false, true), (true, true), (true, false)] {
            let mut returned = vec![ItemStack::empty(); CHEST_SLOTS];
            if conflicting_view {
                returned[0] = container_probe_stack(7);
            }
            let (commands, mut inbox) = mpsc::channel(1);
            let probe = tokio::spawn(async move {
                let SimCommand::OpenContainer { reply, .. } = inbox.recv().await.expect("refresh")
                else {
                    panic!("local player-slot click only refreshes the chest");
                };
                reply
                    .send(accessible.then_some(returned))
                    .expect("live handler");
            });
            let ctx = handler_context(commands, PluginHost::in_memory()).await;
            let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
            *inventory.slot_mut(9).expect("main slot") = container_probe_stack(4);
            let mut windows = WindowState::new();
            windows.open_chest(
                BlockPos::new(8, 65, 8),
                vec![ItemStack::empty(); CHEST_SLOTS],
            );
            inventory.set_carried(container_probe_stack(5));
            let mut writer = PlayWriter::with_defaults(ctx.limits);
            handle_container_packet(
                &ctx,
                &mut inventory,
                &mut windows,
                &mut writer,
                container_probe_click(1, 1, 27),
            )
            .await
            .expect("current click handled");
            assert!(windows.open().is_some(), "window stays open");
            assert_eq!(
                inventory.slot(9).expect("main slot").count(),
                if conflicting_view { 4 } else { 9 }
            );
            assert_eq!(
                inventory.carried().count(),
                if conflicting_view { 5 } else { 0 }
            );
            assert_eq!(drain_container_resync(&mut writer).window_id(), 1);
            probe.await.expect("refresh probe finished");
        }
    }

    #[tokio::test]
    async fn unknown_window_without_an_open_chest_resyncs_the_player_inventory() {
        let (commands, mut inbox) = mpsc::channel(1);
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let before = inventory.slot(9).expect("main slot").clone();
        let mut windows = WindowState::new();
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        handle_container_packet(
            &ctx,
            &mut inventory,
            &mut windows,
            &mut writer,
            container_probe_click(99, 42, 9),
        )
        .await
        .expect("unknown window resyncs");
        assert_eq!(inventory.slot(9), Some(&before));
        let content = drain_container_resync(&mut writer);
        assert_eq!(content.window_id(), 0);
        assert_eq!(
            content.payload(),
            inventory.container_content_payload().expect("inventory")
        );
        assert!(
            inbox.try_recv().is_err(),
            "no world work for a nonexistent window"
        );
    }

    #[tokio::test]
    async fn wrong_close_id_cannot_close_a_newer_window_or_deposit_its_cursor() {
        use ferrumc_world::CHEST_SLOTS;

        let (commands, mut inbox) = mpsc::channel(1);
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let mut windows = WindowState::new();
        windows.open_chest(
            BlockPos::new(8, 65, 8),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        windows.take(); // the old window was already closed
        let current = windows.open_chest(
            BlockPos::new(8, 65, 8),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        assert_eq!(current, 2);
        inventory.set_carried(container_probe_stack(5));
        let inventory_before = inventory.container_content_payload().expect("inventory");
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        for wrong_id in [0, 1, 3] {
            handle_container_packet(
                &ctx,
                &mut inventory,
                &mut windows,
                &mut writer,
                ServerboundPlayPacket::CloseContainer(CloseContainer::new(wrong_id)),
            )
            .await
            .expect("stale close is harmless");
            assert_eq!(inventory.carried(), &container_probe_stack(5));
            assert_eq!(
                inventory.container_content_payload().expect("inventory"),
                inventory_before
            );
            assert_eq!(writer.total_queued(), 0);
            assert!(
                inbox.try_recv().is_err(),
                "wrong close never reaches the driver"
            );
        }
    }

    #[tokio::test]
    async fn full_bag_close_duplicate_and_replacement_keep_one_player_owned_cursor() {
        use ferrumc_world::CHEST_SLOTS;

        let (commands, mut inbox) = mpsc::channel(1); // exactly one equipment refresh
        let ctx = handler_context(commands, PluginHost::in_memory()).await;
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        for index in crate::inventory::MAIN_START..crate::inventory::PLAYER_WINDOW_END {
            inventory.set_creative_slot(index, container_probe_stack(64));
        }
        let carried = container_probe_stack(7);
        inventory.set_carried(carried.clone());
        let slots_before = (0..crate::inventory::SLOT_COUNT)
            .map(|index| inventory.slot(index).expect("slot").clone())
            .collect::<Vec<_>>();
        let mut windows = WindowState::new();
        let old = windows.open_chest(
            BlockPos::new(8, 65, 8),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        handle_container_packet(
            &ctx,
            &mut inventory,
            &mut windows,
            &mut writer,
            ServerboundPlayPacket::CloseContainer(CloseContainer::new(old)),
        )
        .await
        .expect("close");
        assert!(windows.open().is_none());
        assert_eq!(inventory.carried(), &carried);
        assert_eq!(
            (0..crate::inventory::SLOT_COUNT)
                .map(|index| inventory.slot(index).expect("slot").clone())
                .collect::<Vec<_>>(),
            slots_before
        );
        let content = drain_container_resync(&mut writer);
        assert_eq!(content.window_id(), 0);
        assert_eq!(
            content.payload(),
            inventory
                .container_content_payload()
                .expect("window0 payload")
        );
        assert!(matches!(
            inbox.try_recv().expect("equipment update"),
            SimCommand::SetEquipment { .. }
        ));
        handle_container_packet(
            &ctx,
            &mut inventory,
            &mut windows,
            &mut writer,
            ServerboundPlayPacket::CloseContainer(CloseContainer::new(old)),
        )
        .await
        .expect("duplicate close");
        assert_eq!(writer.total_queued(), 0);
        assert!(inbox.try_recv().is_err());
        let replacement = windows.open_chest(
            BlockPos::new(9, 65, 8),
            vec![ItemStack::empty(); CHEST_SLOTS],
        );
        assert_ne!(old, replacement);
        handle_container_packet(
            &ctx,
            &mut inventory,
            &mut windows,
            &mut writer,
            ServerboundPlayPacket::CloseContainer(CloseContainer::new(old)),
        )
        .await
        .expect("stale old close");
        assert_eq!(
            windows.open().expect("new window retained").window_id(),
            replacement
        );
        assert_eq!(inventory.carried(), &carried);
        assert_eq!(writer.total_queued(), 0);
        assert!(inbox.try_recv().is_err());
        let mut carried_bytes = Vec::new();
        carried.encode_slot(&mut carried_bytes).expect("cursor");
        assert!(windows
            .open()
            .expect("new window")
            .content_payload(&inventory)
            .expect("content")
            .ends_with(&carried_bytes));
    }

    /// Builds the real handler context around a bounded command spy and a
    /// movement-notification probe.
    async fn movement_handler_context(
        commands: mpsc::Sender<SimCommand>,
        move_calls: Arc<AtomicUsize>,
    ) -> ConnContext {
        let mut host = PluginHost::in_memory();
        let probe = host
            .register(Box::new(MoveProbe { calls: move_calls }))
            .expect("register movement probe");
        host.enable(&probe).expect("enable movement probe");

        handler_context(commands, host).await
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // one hostile-input fixture and its atomic admission/side-effect assertions
    async fn pending_movement_keeps_position_stream_and_plugin_baseline_but_admits_only_look() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (commands, mut inbox) = mpsc::channel(8);
        let ctx = movement_handler_context(commands, Arc::clone(&calls)).await;
        let player = PlayerId::offline("PendingMovement");
        let base = Vec3::new(8.0, 64.0, 8.0);
        let mut stream = ChunkStream::new(&ctx);
        stream.observe(base);
        stream.block_crossing(base);
        let mut yaw = 10.0;
        let mut pitch = -10.0;
        let mut pending = PendingTeleport::default();
        pending.issue(&super::super::spawn_sync(0, base, yaw, pitch));
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        let mut limiter = ChatRateLimiter::new(ctx.clock.now());
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let mut window = WindowState::new();
        let mut debug = SessionDebug::new("PendingMovement");
        let mut liveness =
            play_liveness(tokio::time::Instant::now(), ctx.io_timeout).expect("tracker");
        for packet in
            [
                ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(
                    40.0, 64.0, 8.0, 0,
                )),
                ServerboundPlayPacket::SetPlayerPositionAndRotation(
                    SetPlayerPositionAndRotation::new(40.0, 64.0, 8.0, f32::NAN, -20.0, 0),
                ),
            ]
        {
            let mut body = BytesMut::new();
            packet.encode(&mut body).expect("encode pending move");
            handle_play_body(
                &ctx,
                player,
                "PendingMovement",
                &mut writer,
                &mut stream,
                &mut limiter,
                &mut inventory,
                &mut window,
                &mut yaw,
                &mut pitch,
                &body,
                &mut debug,
                &CompressionState::disabled(),
                &mut liveness,
                &mut pending,
            )
            .await
            .expect("ignore coordinate");
        }
        assert!(
            inbox.try_recv().is_err(),
            "position-only/unsafe look never reaches sim"
        );
        assert_eq!((yaw, pitch), (10.0, -10.0));
        let mut body = BytesMut::new();
        SetPlayerPositionAndRotation::new(40.0, 64.0, 8.0, 70.0, -20.0, 0)
            .encode(&mut body)
            .expect("combined move");
        let compression = CompressionState::disabled();
        let (result, ()) = tokio::join!(
            handle_play_body(
                &ctx,
                player,
                "PendingMovement",
                &mut writer,
                &mut stream,
                &mut limiter,
                &mut inventory,
                &mut window,
                &mut yaw,
                &mut pitch,
                &body,
                &mut debug,
                &compression,
                &mut liveness,
                &mut pending
            ),
            async {
                let SimCommand::Event {
                    event,
                    acceptance: Some(reply),
                } = inbox.recv().await.expect("look admission")
                else {
                    panic!("only an accepted look event")
                };
                assert_eq!(
                    ferrumc_session::net_event_to_input(&event),
                    Some(ferrumc_sim::GameInput::PlayerMove {
                        player,
                        position: None,
                        yaw: Some(70.0),
                        pitch: Some(-20.0),
                    })
                );
                reply.send(Ok(())).expect("admission reply");
            },
        );
        result.expect("look-only admitted");
        assert_eq!((yaw, pitch), (70.0, -20.0));
        assert_eq!(stream.last_position(), Some(base));
        assert_eq!(stream.pending_position_for_test(), Some(base));
        assert_eq!(
            stream.block_crossing(base),
            None,
            "plugin baseline is unchanged"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(writer.total_queued(), 0);
        assert!(pending.is_pending());
        assert!(inbox.try_recv().is_err());
    }

    #[derive(Default)]
    struct PlacementProbeCalls {
        interact: AtomicUsize,
        before_place: AtomicUsize,
        after_place: AtomicUsize,
        after_states: Mutex<Vec<u32>>,
    }

    /// Makes every plugin boundary in a placement observable without emitting
    /// additional intents.
    struct PlacementProbe {
        calls: Arc<PlacementProbeCalls>,
    }

    impl Plugin for PlacementProbe {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new(
                PluginId::new("packet35-placement-probe"),
                "packet35-placement-probe",
                Version::new(0, 1, 0),
                CapabilityManifest::empty()
                    .with(Capability::ReceiveEvents)
                    .with(Capability::VetoEvents)
                    .with(Capability::VetoBlockEdits),
            )
        }

        fn on_enable(&mut self, ctx: &mut SetupContext<'_>) -> Result<(), PluginError> {
            ctx.events()?.subscribe(EventKind::AfterBlockPlace);
            Ok(())
        }

        fn on_event(&mut self, event: &PluginEvent, _ctx: &mut EventContext<'_>) {
            if let PluginEvent::AfterBlockPlace { block_state_id, .. } = event {
                self.calls.after_place.fetch_add(1, Ordering::Relaxed);
                self.calls
                    .after_states
                    .lock()
                    .expect("placement-probe state lock")
                    .push(*block_state_id);
            }
        }

        fn before_block_place(
            &mut self,
            _event: &BlockPlaceAttempt,
            _ctx: &mut EventContext<'_>,
        ) -> PluginBlockDecision {
            self.calls.before_place.fetch_add(1, Ordering::Relaxed);
            PluginBlockDecision::Allow
        }

        fn before_interact(
            &mut self,
            _event: &InteractAttempt,
            _ctx: &mut EventContext<'_>,
        ) -> PluginEventDecision {
            self.calls.interact.fetch_add(1, Ordering::Relaxed);
            PluginEventDecision::Allow
        }
    }

    async fn placement_handler_context(
        commands: mpsc::Sender<SimCommand>,
        calls: Arc<PlacementProbeCalls>,
    ) -> ConnContext {
        let mut host = PluginHost::in_memory();
        let probe = host
            .register(Box::new(PlacementProbe { calls }))
            .expect("register placement probe");
        host.enable(&probe).expect("enable placement probe");
        handler_context(commands, host).await
    }

    #[derive(Default)]
    struct PlacementCommandCalls {
        total: AtomicUsize,
        open_container: AtomicUsize,
        place_block: AtomicUsize,
        preview: AtomicUsize,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct PlacementPreview {
        requested: u32,
        computed: u32,
        position: BlockPos,
        face: Direction,
        cursor: Vec3,
        yaw: f32,
    }

    #[test]
    fn decode_click_prefix_parses_valid_and_rejects_malformed_without_panicking() {
        // Valid: slot 5 (i16 big-endian), button 0, mode 0; trailing bytes ignored.
        assert_eq!(
            decode_click_prefix(&[0x00, 0x05, 0x00, 0x00]),
            Some((5, 0, 0))
        );
        assert_eq!(
            decode_click_prefix(&[0x00, 0x05, 0x01, 0x01, 0xDE, 0xAD]),
            Some((5, 1, 1)),
            "trailing changed-slots/cursor bytes are ignored, not trusted"
        );
        // Truncated bodies are rejected (no panic), not read out of bounds.
        assert_eq!(decode_click_prefix(&[]), None);
        assert_eq!(decode_click_prefix(&[0x00]), None); // half a slot
        assert_eq!(decode_click_prefix(&[0x00, 0x05]), None); // no button
        assert_eq!(decode_click_prefix(&[0x00, 0x05, 0x00]), None); // no mode
                                                                    // An over-long (6-byte) VarInt for mode is rejected by the bounded reader.
        assert_eq!(
            decode_click_prefix(&[0x00, 0x05, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]),
            None
        );
    }

    const OP_LEVEL: u8 = 4;
    const MEMBER_LEVEL: u8 = 0;

    #[test]
    fn movement_validation_preserves_packet_component_shape() {
        // A turn-in-place (SetPlayerRotation) updates the yaw the placement path
        // reads, so rotating then placing orients stairs/furnaces correctly. This
        // locks in that the rotation-only packet feeds the place yaw.
        let turn = ServerboundPlayPacket::SetPlayerRotation(SetPlayerRotation::new(90.0, -10.0, 1));
        assert_eq!(
            validated_movement(&turn),
            MovementValidation::Valid(MovementObservation {
                position: None,
                yaw: Some(90.0),
                pitch: Some(-10.0),
            })
        );

        // A position-only move carries no yaw, so it must leave the mirrored yaw
        // untouched (None), not reset it to 0.
        let strafe =
            ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(1.0, 2.0, 3.0, 0));
        assert_eq!(
            validated_movement(&strafe),
            MovementValidation::Valid(MovementObservation {
                position: Some(Vec3::new(1.0, 2.0, 3.0)),
                yaw: None,
                pitch: None,
            })
        );
    }

    #[test]
    fn invalid_movement_components_are_rejected_before_observation() {
        for (label, coordinate) in [
            ("NaN", f64::NAN),
            ("positive infinity", f64::INFINITY),
            ("negative infinity", f64::NEG_INFINITY),
            ("above the positive boundary", 30_000_001.0),
            ("below the negative boundary", -30_000_001.0),
        ] {
            let packet = ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(
                coordinate, 64.0, 8.0, 0,
            ));
            assert_eq!(
                validated_movement(&packet),
                MovementValidation::Invalid,
                "{label} reached the connection's stateful movement path"
            );
        }

        for coordinate in [-30_000_000.0, 30_000_000.0] {
            let packet = ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(
                coordinate, 64.0, 8.0, 0,
            ));
            assert_eq!(
                validated_movement(&packet),
                MovementValidation::Valid(MovementObservation {
                    position: Some(Vec3::new(coordinate, 64.0, 8.0)),
                    yaw: None,
                    pitch: None,
                }),
                "the inclusive simulation boundary must stay accepted",
            );
        }

        let invalid_yaw =
            ServerboundPlayPacket::SetPlayerRotation(SetPlayerRotation::new(f32::NAN, 0.0, 0));
        assert_eq!(
            validated_movement(&invalid_yaw),
            MovementValidation::Invalid,
            "a non-finite yaw must not enter the persistence mirror"
        );

        let invalid_pitch = ServerboundPlayPacket::SetPlayerPositionAndRotation(
            SetPlayerPositionAndRotation::new(8.0, 64.0, 8.0, 0.0, f32::NEG_INFINITY, 0),
        );
        assert_eq!(
            validated_movement(&invalid_pitch),
            MovementValidation::Invalid,
            "a non-finite pitch must not enter the persistence mirror"
        );

        let valid_position_invalid_look = ServerboundPlayPacket::SetPlayerPositionAndRotation(
            SetPlayerPositionAndRotation::new(8.0, 64.0, 8.0, f32::INFINITY, 15.0, 0),
        );
        assert_eq!(
            validated_movement(&valid_position_invalid_look),
            MovementValidation::Invalid,
            "a combined packet is rejected atomically instead of applying its position"
        );

        let invalid_position_valid_look = ServerboundPlayPacket::SetPlayerPositionAndRotation(
            SetPlayerPositionAndRotation::new(30_000_001.0, 64.0, 8.0, 90.0, 15.0, 0),
        );
        assert_eq!(
            validated_movement(&invalid_position_valid_look),
            MovementValidation::Invalid,
            "a combined packet is rejected atomically instead of applying its look"
        );

        let wrapped_finite_look = ServerboundPlayPacket::SetPlayerRotation(SetPlayerRotation::new(
            100_000.0, -100_000.0, 0,
        ));
        assert_eq!(
            validated_movement(&wrapped_finite_look),
            MovementValidation::Valid(MovementObservation {
                position: None,
                yaw: Some(100_000.0),
                pitch: Some(-100_000.0),
            }),
            "finite angles remain protocol-valid and may wrap"
        );
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // one ordered trust-boundary transaction and its spies
    async fn movement_handler_rejects_before_every_observable_side_effect() {
        // At most two sequential positive controls are emitted. Eight slots keep
        // diagnostic headroom while the active receiver makes queue saturation
        // impossible, and the capacity remains explicitly bounded.
        const COMMAND_CAPACITY: usize = 8;

        let command_calls = Arc::new(AtomicUsize::new(0));
        let move_calls = Arc::new(AtomicUsize::new(0));
        let (commands_tx, mut commands_rx) = mpsc::channel(COMMAND_CAPACITY);
        let command_calls_for_task = Arc::clone(&command_calls);
        let command_task = tokio::spawn(async move {
            while let Some(command) = commands_rx.recv().await {
                command_calls_for_task.fetch_add(1, Ordering::Relaxed);
                let (SimCommand::Event { acceptance, .. }
                | SimCommand::TeleportPlayer { acceptance, .. }) = command
                else {
                    panic!("movement test emitted an unexpected command");
                };
                if let Some(reply) = acceptance {
                    let _ = reply.send(Ok(()));
                }
            }
        });

        let ctx = movement_handler_context(commands_tx, Arc::clone(&move_calls)).await;
        let player = PlayerId::offline("MovementHandlerProbe");
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        let mut chunk_stream = ChunkStream::new(&ctx);
        let base = Vec3::new(8.5, 64.0, 8.5);
        chunk_stream.observe(base);
        assert_eq!(chunk_stream.block_crossing(base), None);
        let mut chat_limiter = ChatRateLimiter::new(ctx.clock.now());
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let mut window_state = WindowState::new();
        let mut yaw = 15.0;
        let mut pitch = -10.0;
        let mut debug = SessionDebug::new("movement-handler-probe");
        let compression = CompressionState::disabled();

        let invalid =
            [
                ServerboundPlayPacket::SetPlayerPosition(SetPlayerPosition::new(
                    f64::NAN,
                    64.0,
                    8.5,
                    0,
                )),
                ServerboundPlayPacket::SetPlayerRotation(SetPlayerRotation::new(
                    f32::INFINITY,
                    20.0,
                    0,
                )),
                ServerboundPlayPacket::SetPlayerPositionAndRotation(
                    SetPlayerPositionAndRotation::new(9.5, 64.0, 8.5, 30.0, f32::NEG_INFINITY, 0),
                ),
                ServerboundPlayPacket::SetPlayerPositionAndRotation(
                    SetPlayerPositionAndRotation::new(30_000_001.0, 64.0, 8.5, 120.0, 30.0, 0),
                ),
            ];
        for packet in invalid {
            let mut body = BytesMut::new();
            packet.encode(&mut body).expect("encode hostile movement");
            handle_play_body(
                &ctx,
                player,
                "MovementHandlerProbe",
                &mut writer,
                &mut chunk_stream,
                &mut chat_limiter,
                &mut inventory,
                &mut window_state,
                &mut yaw,
                &mut pitch,
                &body,
                &mut debug,
                &compression,
                &mut play_liveness(tokio::time::Instant::now(), ctx.io_timeout)
                    .expect("Play liveness"),
                &mut PendingTeleport::default(),
            )
            .await
            .expect("reject hostile movement cleanly");
        }
        assert_eq!(command_calls.load(Ordering::Relaxed), 0);
        assert_eq!(move_calls.load(Ordering::Relaxed), 0);
        assert_eq!(chunk_stream.last_position(), Some(base));
        assert_eq!(chunk_stream.pending_position_for_test(), Some(base));
        assert_eq!((yaw, pitch), (15.0, -10.0));
        assert_eq!(writer.total_queued(), 0);

        // The probe is not vacuous: a valid admitted crossing reaches exactly one
        // driver event and one plugin notification, and updates every local mirror.
        let accepted = Vec3::new(9.5, 64.0, 8.5);
        let packet =
            SetPlayerPositionAndRotation::new(accepted.x, accepted.y, accepted.z, 45.0, -25.0, 0);
        let mut body = BytesMut::new();
        packet.encode(&mut body).expect("encode accepted movement");
        handle_play_body(
            &ctx,
            player,
            "MovementHandlerProbe",
            &mut writer,
            &mut chunk_stream,
            &mut chat_limiter,
            &mut inventory,
            &mut window_state,
            &mut yaw,
            &mut pitch,
            &body,
            &mut debug,
            &compression,
            &mut play_liveness(tokio::time::Instant::now(), ctx.io_timeout).expect("Play liveness"),
            &mut PendingTeleport::default(),
        )
        .await
        .expect("admit valid movement");
        assert_eq!(command_calls.load(Ordering::Relaxed), 1);
        assert_eq!(move_calls.load(Ordering::Relaxed), 1);
        assert_eq!(chunk_stream.last_position(), Some(accepted));
        assert_eq!(chunk_stream.pending_position_for_test(), Some(accepted));
        assert_eq!((yaw, pitch), (45.0, -25.0));

        // A later hostile combined packet cannot partially replace the accepted
        // position, look, plugin baseline, or routed event.
        let packet = SetPlayerPositionAndRotation::new(10.5, 64.0, 8.5, f32::NAN, -30.0, 0);
        let mut body = BytesMut::new();
        packet
            .encode(&mut body)
            .expect("encode later hostile movement");
        handle_play_body(
            &ctx,
            player,
            "MovementHandlerProbe",
            &mut writer,
            &mut chunk_stream,
            &mut chat_limiter,
            &mut inventory,
            &mut window_state,
            &mut yaw,
            &mut pitch,
            &body,
            &mut debug,
            &compression,
            &mut play_liveness(tokio::time::Instant::now(), ctx.io_timeout).expect("Play liveness"),
            &mut PendingTeleport::default(),
        )
        .await
        .expect("reject later hostile movement");
        assert_eq!(command_calls.load(Ordering::Relaxed), 1);
        assert_eq!(move_calls.load(Ordering::Relaxed), 1);
        assert_eq!(chunk_stream.last_position(), Some(accepted));
        assert_eq!(chunk_stream.pending_position_for_test(), Some(accepted));
        assert_eq!((yaw, pitch), (45.0, -25.0));
        assert_eq!(writer.total_queued(), 0);

        for position in [
            Vec3::new(f64::NAN, 64.0, 8.0),
            Vec3::new(30_000_001.0, 64.0, 8.0),
        ] {
            route_emitted_intents(
                &ctx,
                player,
                &mut writer,
                0,
                vec![WorldIntent::Teleport { player, position }],
                &mut debug,
                &compression,
            )
            .await
            .expect("skip unsafe plugin teleport cleanly");
        }
        assert_eq!(
            command_calls.load(Ordering::Relaxed),
            1,
            "unsafe plugin teleports never reach the driver"
        );

        route_emitted_intents(
            &ctx,
            player,
            &mut writer,
            0,
            vec![WorldIntent::Teleport {
                player,
                position: Vec3::new(20.0, 70.0, -5.0),
            }],
            &mut debug,
            &compression,
        )
        .await
        .expect("route a safe plugin teleport");
        assert_eq!(
            command_calls.load(Ordering::Relaxed),
            2,
            "the plugin teleport gate is not vacuously dropping every intent"
        );

        let mut overflow_stream = ChunkStream::new(&ctx);
        overflow_stream.observe(Vec3::new(40.5, 64.0, 8.5));
        let mut fills = 0usize;
        loop {
            let outcome = writer.enqueue_classified(ClientboundPlayPacket::SetCenterChunk(
                SetCenterChunk::new(0, 0),
            ));
            if outcome.is_dropped() {
                break;
            }
            fills += 1;
            assert!(
                fills < 1_000,
                "the bounded State queue must eventually fill"
            );
        }
        let error = apply_chunk_stream(
            &ctx,
            &mut writer,
            &mut overflow_stream,
            &mut debug,
            &compression,
            None,
        )
        .await
        .expect_err("a dropped mandatory stream centre terminates the session");
        assert!(error.to_string().contains("mandatory clientbound packet"));
        assert_eq!(
            overflow_stream.center_for_test(),
            ctx.join_kit.spawn_chunk(),
            "a dropped centre cannot commit app-side stream ownership"
        );
        assert_eq!(
            command_calls.load(Ordering::Relaxed),
            2,
            "a dropped centre cannot proceed into chunk ticket work"
        );

        drop(ctx);
        command_task
            .await
            .expect("bounded movement command spy shuts down");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // one ordered trust-boundary battery and its effect spies
    async fn use_item_on_rejects_invalid_cursor_before_every_observable_effect() {
        // Commands are consumed one at a time as each packet waits for its reply.
        // Eight bounded slots leave diagnostic headroom for the two valid controls
        // without weakening the production backpressure contract.
        const COMMAND_CAPACITY: usize = 8;

        let plugin_calls = Arc::new(PlacementProbeCalls::default());
        let command_calls = Arc::new(PlacementCommandCalls::default());
        let previews = Arc::new(Mutex::new(Vec::new()));
        let (commands_tx, mut commands_rx) = mpsc::channel(COMMAND_CAPACITY);
        let command_calls_for_task = Arc::clone(&command_calls);
        let previews_for_task = Arc::clone(&previews);
        let command_task = tokio::spawn(async move {
            let shard = SimShard::new(ShardPos::new(0, 0));
            while let Some(command) = commands_rx.recv().await {
                command_calls_for_task.total.fetch_add(1, Ordering::Relaxed);
                match command {
                    SimCommand::OpenContainer { reply, .. } => {
                        command_calls_for_task
                            .open_container
                            .fetch_add(1, Ordering::Relaxed);
                        let _ = reply.send(None);
                    }
                    SimCommand::PlaceBlock {
                        position,
                        state,
                        clicked_face,
                        cursor_position,
                        player_yaw,
                        reply,
                        ..
                    } => {
                        command_calls_for_task
                            .place_block
                            .fetch_add(1, Ordering::Relaxed);
                        let computed = shard.preview_placement(
                            state,
                            clicked_face,
                            cursor_position,
                            player_yaw,
                            position,
                        );
                        command_calls_for_task
                            .preview
                            .fetch_add(1, Ordering::Relaxed);
                        previews_for_task
                            .lock()
                            .expect("placement preview lock")
                            .push(PlacementPreview {
                                requested: state.as_u32(),
                                computed: computed.as_u32(),
                                position,
                                face: clicked_face,
                                cursor: cursor_position,
                                yaw: player_yaw,
                            });
                        let _ = reply.send(Ok(computed));
                    }
                    _ => panic!("placement test emitted an unexpected command"),
                }
            }
        });

        let ctx = placement_handler_context(commands_tx, Arc::clone(&plugin_calls)).await;
        let player = PlayerId::offline("PlacementHandlerProbe");
        let mut writer = PlayWriter::with_defaults(ctx.limits);
        let mut chunk_stream = ChunkStream::new(&ctx);
        let player_position = Vec3::new(8.5, 64.0, 8.5);
        chunk_stream.observe(player_position);
        let mut chat_limiter = ChatRateLimiter::new(ctx.clock.now());
        let mut inventory = PlayerInventory::with_creative_kit(GameMode::Creative);
        let mut window_state = WindowState::new();
        let mut yaw = 0.0;
        let mut pitch = 0.0;
        let mut debug = SessionDebug::new("placement-handler-probe");
        let compression = CompressionState::disabled();

        // Every hostile value is injected independently into x, y, and z. The
        // other two components remain valid so a missing per-axis check is visible.
        let mut sequence = 1;
        for axis in 0..3 {
            for invalid in [
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
                -f32::EPSILON,
                1.0 + f32::EPSILON,
            ] {
                let mut cursor = [0.5; 3];
                cursor[axis] = invalid;
                let packet = UseItemOn::new(
                    0,
                    BlockPosition::new(8, 63, 8),
                    1,
                    cursor[0],
                    cursor[1],
                    cursor[2],
                    false,
                    false,
                    sequence,
                );
                sequence += 1;
                let mut body = BytesMut::new();
                packet
                    .encode(&mut body)
                    .expect("encode hostile placement cursor");
                handle_play_body(
                    &ctx,
                    player,
                    "PlacementHandlerProbe",
                    &mut writer,
                    &mut chunk_stream,
                    &mut chat_limiter,
                    &mut inventory,
                    &mut window_state,
                    &mut yaw,
                    &mut pitch,
                    &body,
                    &mut debug,
                    &compression,
                    &mut play_liveness(tokio::time::Instant::now(), ctx.io_timeout)
                        .expect("Play liveness"),
                    &mut PendingTeleport::default(),
                )
                .await
                .expect("reject hostile placement cursor cleanly");
            }
        }

        // Cursor validation also precedes the malformed-face acknowledgement.
        // Otherwise a packet invalid in both fields would still create an
        // observable clientbound effect.
        let packet = UseItemOn::new(
            0,
            BlockPosition::new(8, 63, 8),
            99,
            f32::NAN,
            0.5,
            0.5,
            false,
            false,
            sequence,
        );
        sequence += 1;
        let mut body = BytesMut::new();
        packet
            .encode(&mut body)
            .expect("encode invalid cursor with malformed face");
        handle_play_body(
            &ctx,
            player,
            "PlacementHandlerProbe",
            &mut writer,
            &mut chunk_stream,
            &mut chat_limiter,
            &mut inventory,
            &mut window_state,
            &mut yaw,
            &mut pitch,
            &body,
            &mut debug,
            &compression,
            &mut play_liveness(tokio::time::Instant::now(), ctx.io_timeout).expect("Play liveness"),
            &mut PendingTeleport::default(),
        )
        .await
        .expect("cursor rejection precedes malformed-face acknowledgement");

        assert_eq!(plugin_calls.interact.load(Ordering::Relaxed), 0);
        assert_eq!(plugin_calls.before_place.load(Ordering::Relaxed), 0);
        assert_eq!(plugin_calls.after_place.load(Ordering::Relaxed), 0);
        assert_eq!(command_calls.total.load(Ordering::Relaxed), 0);
        assert_eq!(command_calls.open_container.load(Ordering::Relaxed), 0);
        assert_eq!(command_calls.place_block.load(Ordering::Relaxed), 0);
        assert_eq!(command_calls.preview.load(Ordering::Relaxed), 0);
        assert!(previews.lock().expect("placement preview lock").is_empty());
        assert!(plugin_calls
            .after_states
            .lock()
            .expect("placement-probe state lock")
            .is_empty());
        assert_eq!(writer.total_queued(), 0);
        assert!(window_state.open().is_none());
        assert_eq!(chunk_stream.last_position(), Some(player_position));

        // Positive controls cover all inclusive endpoints across the two packets
        // and run the real placement preview: a high side-click makes a top slab,
        // while yaw zero plus a low east-face click makes south/bottom stairs.
        for (item_name, location, direction, cursor) in [
            ("oak_slab", BlockPosition::new(8, 64, 9), 2, [0.0, 1.0, 1.0]),
            (
                "oak_stairs",
                BlockPosition::new(7, 64, 8),
                5,
                [1.0, 0.0, 0.0],
            ),
        ] {
            let item = ItemId::from_name(item_name).expect("known placeable item");
            let stack = ItemStack::try_new(item, 1, ComponentPatch::empty())
                .expect("one-item stack is valid");
            inventory.set_creative_slot(HOTBAR_START, stack);
            let packet = UseItemOn::new(
                0, location, direction, cursor[0], cursor[1], cursor[2], false, false, sequence,
            );
            sequence += 1;
            let mut body = BytesMut::new();
            packet
                .encode(&mut body)
                .expect("encode accepted placement cursor");
            handle_play_body(
                &ctx,
                player,
                "PlacementHandlerProbe",
                &mut writer,
                &mut chunk_stream,
                &mut chat_limiter,
                &mut inventory,
                &mut window_state,
                &mut yaw,
                &mut pitch,
                &body,
                &mut debug,
                &compression,
                &mut play_liveness(tokio::time::Instant::now(), ctx.io_timeout)
                    .expect("Play liveness"),
                &mut PendingTeleport::default(),
            )
            .await
            .expect("admit valid placement cursor");
        }

        assert_eq!(plugin_calls.interact.load(Ordering::Relaxed), 2);
        assert_eq!(plugin_calls.before_place.load(Ordering::Relaxed), 2);
        assert_eq!(plugin_calls.after_place.load(Ordering::Relaxed), 2);
        assert_eq!(command_calls.total.load(Ordering::Relaxed), 4);
        assert_eq!(command_calls.open_container.load(Ordering::Relaxed), 2);
        assert_eq!(command_calls.place_block.load(Ordering::Relaxed), 2);
        assert_eq!(command_calls.preview.load(Ordering::Relaxed), 2);
        assert_eq!(
            *previews.lock().expect("placement preview lock"),
            [
                PlacementPreview {
                    requested: 12054,
                    computed: 12052,
                    position: BlockPos::new(8, 64, 8),
                    face: Direction::North,
                    cursor: Vec3::new(0.0, 1.0, 1.0),
                    yaw: 0.0,
                },
                PlacementPreview {
                    requested: 2949,
                    computed: 2969,
                    position: BlockPos::new(8, 64, 8),
                    face: Direction::East,
                    cursor: Vec3::new(1.0, 0.0, 0.0),
                    yaw: 0.0,
                },
            ]
        );
        assert_eq!(
            *plugin_calls
                .after_states
                .lock()
                .expect("placement-probe state lock"),
            [12052, 2969],
        );
        assert_eq!(writer.total_queued(), 0);
        assert!(window_state.open().is_none());

        drop(ctx);
        command_task
            .await
            .expect("bounded placement command spy shuts down");
    }

    #[test]
    fn java_completion_keeps_trailing_empty_argument_and_bounded_token_count() {
        assert_eq!(super::java_completion_input("/bfixture"), None);
        assert_eq!(super::java_completion_input("/ "), None);
        assert_eq!(
            super::java_completion_input("/BFIXTURE probe  "),
            Some((
                "BFIXTURE",
                vec!["probe".to_owned(), String::new(), String::new()]
            )),
        );
        assert_eq!(super::java_completion_input("/bfixture\targ"), None);
        assert_eq!(
            super::java_completion_input("/bfixture \u{2003}\t"),
            Some(("bfixture", vec!["\u{2003}\t".to_owned()]))
        );
        assert_eq!(
            super::java_completion_input("/bridgefixture:bfixture Grüße 🌍"),
            Some((
                "bridgefixture:bfixture",
                vec!["Grüße".to_owned(), "🌍".to_owned()]
            )),
        );
        let hostile = format!("/bfixture {}", "x ".repeat(1000));
        assert_eq!(
            super::java_completion_input(&hostile)
                .expect("arguments")
                .1
                .len(),
            65
        );
    }

    #[tokio::test(start_paused = true)]
    async fn java_completion_cancels_at_the_exact_outstanding_keep_alive_deadline() {
        use ferrumc_net::LivenessTimeout;
        use std::sync::atomic::AtomicBool;
        use std::time::Duration;
        use tokio::time::{advance, Instant};

        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        let mut liveness =
            super::super::play::play_liveness(Instant::now(), Duration::from_secs(30))
                .expect("Play tracker");
        liveness
            .keep_alive_sent(77, Instant::now())
            .expect("request sent");
        let (_signal, shutdown) = tokio::sync::watch::channel(false);
        let dropped = Arc::new(AtomicBool::new(false));
        let marker = DropFlag(Arc::clone(&dropped));
        let task = tokio::spawn(super::guard_java_completion(
            liveness.next_deadline(),
            shutdown,
            async move {
                let _marker = marker;
                std::future::pending::<Result<(), crate::java_bridge::BridgeError>>().await
            },
        ));
        tokio::task::yield_now().await;
        advance(Duration::from_secs(14)).await;
        assert!(!task.is_finished());
        advance(Duration::from_secs(1)).await;
        let error = task
            .await
            .expect("guard finished")
            .expect_err("response deadline");
        assert_eq!(
            error.downcast_ref(),
            Some(&LivenessTimeout::KeepAlive { id: 77 })
        );
        assert!(
            dropped.load(Ordering::Relaxed),
            "RPC was cancelled at the deadline"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn java_completion_observes_shutdown_and_preserves_remote_errors() {
        use std::time::Duration;
        use tokio::time::Instant;
        let liveness = super::super::play::play_liveness(Instant::now(), Duration::from_secs(30))
            .expect("Play tracker");
        let (signal, shutdown) = tokio::sync::watch::channel(false);
        let remote = crate::java_bridge::BridgeError::Remote("completer failed".to_owned());
        let result = super::guard_java_completion(
            liveness.next_deadline(),
            shutdown.clone(),
            std::future::ready(Err::<(), _>(remote.clone())),
        )
        .await
        .expect("recoverable callback result");
        assert_eq!(result.expect_err("remote error"), remote);
        let task = tokio::spawn(super::guard_java_completion(
            liveness.next_deadline(),
            shutdown,
            std::future::pending::<Result<(), crate::java_bridge::BridgeError>>(),
        ));
        tokio::task::yield_now().await;
        signal.send(true).expect("server shutdown");
        assert!(task
            .await
            .expect("guard finished")
            .expect_err("cancelled")
            .to_string()
            .contains("server shutdown"));
    }

    #[test]
    fn tab_complete_offsets_match_javas_utf16_cursor_positions() {
        let tree = build_command_tree();
        let (start, length, _) = tab_complete_reply(&tree, OP_LEVEL, &|_| true, "/say 🌍 🌍a");
        assert_eq!((start, length), (8, 3));
    }

    #[test]
    fn tab_complete_offers_literal_completion_for_a_prefix() {
        let tree = build_command_tree();
        // "/sp" -> the in-progress token "sp" (char 1..3) completes to "spawn".
        let (start, length, matches) = tab_complete_reply(&tree, OP_LEVEL, &|_| true, "/sp");
        assert_eq!((start, length), (1, 2));
        assert_eq!(matches, vec![SPAWN_COMMAND.to_string()]);
    }

    #[test]
    fn tab_complete_lists_all_commands_after_the_slash() {
        let tree = build_command_tree();
        let (start, length, matches) = tab_complete_reply(&tree, OP_LEVEL, &|_| true, "/");
        assert_eq!((start, length), (1, 0));
        assert!(matches.contains(&SPAWN_COMMAND.to_string()));
        assert!(matches.contains(&GAMEMODE_COMMAND.to_string()));
    }

    #[test]
    fn tab_complete_hides_gated_commands_from_low_level_players() {
        let tree = build_command_tree();
        // A level-0 player gets `/spawn` but never `/gamemode`.
        let (_, _, op_matches) = tab_complete_reply(&tree, OP_LEVEL, &|_| true, "/ga");
        assert_eq!(op_matches, vec![GAMEMODE_COMMAND.to_string()]);
        let (_, _, member_matches) = tab_complete_reply(&tree, MEMBER_LEVEL, &|_| true, "/ga");
        assert_eq!(member_matches, [] as [String; 0]);
    }

    #[test]
    fn tab_complete_drops_argument_hints() {
        let tree = build_command_tree();
        // After "/gamemode " the only candidate is the `<mode: 0..3>` hint, which is
        // display-only and must not be sent as an insertable match.
        let (start, length, matches) = tab_complete_reply(&tree, OP_LEVEL, &|_| true, "/gamemode ");
        assert_eq!((start, length), (10, 0));
        assert_eq!(matches, [] as [String; 0]);
    }

    #[test]
    fn tab_complete_range_is_in_char_units_for_non_ascii() {
        let tree = build_command_tree();
        // "/éa": the accented `é` is two UTF-8 bytes, so the in-progress token "éa"
        // begins at character 1 (right after the slash) and is two characters long.
        // The old byte computation would report length 3, mis-indexing the client.
        let (start, length, _) = tab_complete_reply(&tree, OP_LEVEL, &|_| true, "/\u{e9}a");
        assert_eq!((start, length), (1, 2));
    }

    /// Decodes a serverbound play body the same way `handle_play_body` does:
    /// read the `VarInt` id, then dispatch to the typed packet.
    fn decode_play(body: &[u8]) -> Result<ServerboundPlayPacket, ()> {
        let mut reader = BoundedReader::new(body);
        let id = reader.read_var_int().map_err(|_| ())?;
        ServerboundPlayPacket::decode(id, &mut reader).map_err(|_| ())
    }

    #[test]
    fn update_sign_decodes_a_well_formed_body() {
        // id + 8-byte packed location (all zero -> (0,0,0)) + is_front(1) + four
        // zero-length line strings.
        let mut body = Vec::new();
        write_var_int(&mut body, UpdateSign::PACKET_ID);
        body.extend_from_slice(&[0u8; 8]); // packed BlockPosition (0,0,0)
        body.push(0x01); // is_front_text = true
        body.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // four empty VarInt-prefixed strings

        let ServerboundPlayPacket::UpdateSign(packet) = decode_play(&body).expect("valid body")
        else {
            panic!("expected an UpdateSign");
        };
        let loc = packet.location();
        assert_eq!((loc.x(), loc.y(), loc.z()), (0, 0, 0));
        assert!(packet.is_front_text());
        assert_eq!(packet.line_1().as_str(), "");
        assert_eq!(packet.line_4().as_str(), "");
    }

    #[test]
    fn update_sign_rejects_a_truncated_body() {
        // Only the location is present: the decoder needs the is_front bool and four
        // line strings next, so a body that stops here is malformed and rejected
        // rather than silently accepted with garbage.
        let mut truncated = Vec::new();
        write_var_int(&mut truncated, UpdateSign::PACKET_ID);
        truncated.extend_from_slice(&[0u8; 8]); // location only, nothing after
        assert!(decode_play(&truncated).is_err());

        // Even shorter: the location itself is incomplete (4 of 8 bytes).
        let mut shorter = Vec::new();
        write_var_int(&mut shorter, UpdateSign::PACKET_ID);
        shorter.extend_from_slice(&[0u8; 4]);
        assert!(decode_play(&shorter).is_err());

        // A line string claims a length far past the remaining bytes.
        let mut bad_string = Vec::new();
        write_var_int(&mut bad_string, UpdateSign::PACKET_ID);
        bad_string.extend_from_slice(&[0u8; 8]); // location
        bad_string.push(0x00); // is_front = false
        write_var_int(&mut bad_string, 4096); // line_1 claims 4096 bytes...
        bad_string.extend_from_slice(b"hi"); // ...but only 2 follow
        assert!(decode_play(&bad_string).is_err());
    }
}
