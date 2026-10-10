//! Per-connection protocol driver: handshake -> login -> configuration -> play.
//!
//! A handshake with `next_state == 1` instead takes the status branch: the
//! server replies with a server-list status response and a ping/pong echo, then
//! closes — this is what makes the server visible (and "COMPATIBLE") in a real
//! client's multiplayer list.
//!
//! Each accepted socket runs one [`handle_connection`] task. It drives the login
//! handshake by hand over the `ferrumc-net` framing primitives (the crate's own
//! `LoginServer` ends at a keepalive shell and exposes no post-play hook, so the
//! app wires the play handoff itself). The instant the client acknowledges
//! configuration, the connection joins the simulation and replays the shared
//! [`JoinKit`](crate::world::JoinKit): `JoinGame`, a position sync, then the
//! spawn-area chunk packets. From there it pumps serverbound play packets into
//! the simulation and clientbound outputs back to the socket.

use std::time::Duration;

use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{oneshot, watch};
use tokio::time::{sleep_until, timeout, Instant};

use ferrumc_codec::BoundedString;
use ferrumc_core::PlayerId;
use ferrumc_math::Vec3;
use ferrumc_net::{
    CompressionState, ConnectionLiveness, ConnectionState, DecodeError, FrameDecodeError,
    InboundDecoder, InboundPacket, LivenessConfig, LivenessTimeout, OutboundEncoder,
    OutboundPacket, DEFAULT_FRAME_COMPLETION_TIMEOUT, DEFAULT_KEEP_ALIVE_TIMEOUT,
};
use ferrumc_observability::SessionDebug;
use ferrumc_proto::generated::configuration::{
    ClientboundConfigurationPacket, ClientboundKnownPacks, FinishConfiguration,
    ServerboundConfigurationPacket,
};
use ferrumc_proto::generated::handshake::ServerboundHandshakePacket;
use ferrumc_proto::generated::login::{
    ClientboundLoginPacket, LoginSuccess, ServerboundLoginPacket, SetCompression,
};
use ferrumc_proto::generated::play::SynchronizePlayerPosition;
use ferrumc_proto::generated::status::{
    ClientboundStatusPacket, PongResponse, ServerboundStatusPacket,
};
use ferrumc_registry::PROTOCOL_VERSION;

use crate::{driver::SimCommand, observe};

mod chunk_stream;
mod context;
mod handlers;
mod join;
mod outbound;
mod play;
mod rate_limiter;
mod serverbound_budget;

pub(crate) use context::{build_status_response, ConnContext};

use context::login_disconnect;
use play::enter_play;

/// Sends an authoritative driver command and waits until its bounded shard
/// admission is known.
///
/// The driver either accepts the exact input or explicitly tears down the
/// overloaded session before returning a classified rejection. Connection code
/// uses this barrier before publishing success feedback, client previews, or
/// plugin after-events.
async fn send_sim_command_accepted(
    ctx: &ConnContext,
    mut command: SimCommand,
) -> anyhow::Result<()> {
    let (reply_tx, reply_rx) = oneshot::channel();
    command
        .request_delivery_acceptance(reply_tx)
        .map_err(|_| anyhow::anyhow!("driver command has no shard-acceptance boundary"))?;
    ctx.commands
        .send(command)
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
    reply_rx
        .await
        .map_err(|_| anyhow::anyhow!("simulation driver dropped the acceptance reply"))?
        .map_err(|err| anyhow::anyhow!("simulation input rejected: {err}"))
}

/// The `next_state` value in a handshake that selects the status branch
/// (server-list ping).
const NEXT_STATE_STATUS: i32 = 1;

/// The `next_state` value in a handshake that selects the login branch.
const NEXT_STATE_LOGIN: i32 = 2;

/// Controlled Login Disconnect for clients on the wrong wire protocol.
const INCOMPATIBLE_PROTOCOL_REASON: &str =
    "Incompatible client: FerrumC requires Minecraft 1.21.8 (protocol 772).";

/// Controlled Login Disconnect for a syntactically invalid username.
const INVALID_USERNAME_REASON: &str =
    "Invalid username: use 1-16 ASCII letters, digits, or underscores.";

/// Bytes read off the socket per `read` call before decoding.
const READ_CHUNK: usize = 4096;

/// Teleport id carried by the join `SynchronizePlayerPosition`.
///
/// Must be non-zero: a real client replies with a `ConfirmTeleportation` echoing
/// it, which the server decodes and ignores.
const JOIN_TELEPORT_ID: i32 = 1;

/// `Game Event` reason `3`: "change game mode". The event `value` is the game-mode
/// id as a float; sending it switches the issuing client's mode (the carrier
/// `/gamemode` and the rejoin restore use, since there is no dedicated
/// set-game-mode packet). Shared by the join restore and the `/gamemode` handler.
pub(super) const GAME_EVENT_CHANGE_GAMEMODE: u8 = 3;

/// The login-handshake phase, one step finer than [`ConnectionState`] so an
/// out-of-order-but-valid packet can be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginPhase {
    /// Awaiting the initial handshake.
    Handshaking,
    /// Status branch (`next_state == 1`): awaiting the Status Request and Ping
    /// Request that drive the server-list exchange.
    Status,
    /// Status response sent; only the terminal Ping Request may advance now.
    AwaitingStatusPing,
    /// Awaiting Login Start.
    Login,
    /// Awaiting Login Acknowledged (Login Success already sent).
    AwaitingLoginAck,
    /// Awaiting the client's Known Packs echo (Clientbound Known Packs already
    /// sent); the registry data is not sent until it arrives.
    AwaitingKnownPacks,
    /// Awaiting Ack Finish Configuration.
    AwaitingFinishAck,
}

impl LoginPhase {
    /// The connection state the decoder/encoder use for the next frame.
    fn connection_state(self) -> ConnectionState {
        match self {
            Self::Handshaking => ConnectionState::Handshaking,
            Self::Status | Self::AwaitingStatusPing => ConnectionState::Status,
            Self::Login | Self::AwaitingLoginAck => ConnectionState::Login,
            Self::AwaitingKnownPacks | Self::AwaitingFinishAck => ConnectionState::Configuration,
        }
    }
}

/// Phase progress and frame completion have independent absolute clocks. The
/// frame clock also covers ignored frames, whose completion cannot refresh the
/// phase clock. The network policy owns the meaningful-progress contract.
struct LoginLiveness {
    progress: ConnectionLiveness,
    frame_window: Duration,
    partial_since: Option<Instant>,
}

impl LoginLiveness {
    fn new(now: Instant, window: Duration) -> Self {
        Self {
            progress: ConnectionLiveness::new(
                now,
                LivenessConfig::new(window, window, DEFAULT_KEEP_ALIVE_TIMEOUT),
            ),
            frame_window: window.min(DEFAULT_FRAME_COMPLETION_TIMEOUT),
            partial_since: None,
        }
    }

    fn deadline(&self) -> anyhow::Result<(Instant, LivenessTimeout)> {
        let progress = self
            .progress
            .next_deadline()
            .ok_or_else(|| anyhow::anyhow!("login liveness is closed"))?;
        if let Some(first_bytes) = self.partial_since {
            let frame_at = first_bytes
                .checked_add(self.frame_window)
                .unwrap_or(first_bytes);
            if frame_at < progress.at() {
                return Ok((
                    frame_at,
                    LivenessTimeout::FrameCompletion {
                        state: self.progress.state(),
                    },
                ));
            }
        }
        Ok((progress.at(), progress.timeout()))
    }

    fn ensure_live(&self, now: Instant) -> anyhow::Result<()> {
        let (at, reason) = self.deadline()?;
        if now >= at {
            return Err(reason.into());
        }
        Ok(())
    }

    fn partial_frame(&mut self, first_bytes: Instant) {
        self.partial_since.get_or_insert(first_bytes);
    }

    fn complete_frame(&mut self) {
        self.partial_since = None;
    }

    fn phase_advanced(&mut self, state: ConnectionState, now: Instant) -> anyhow::Result<()> {
        self.progress.enter_state(state, now)?;
        Ok(())
    }
}

/// Only pre-Play work is canceled here: no player/world transaction has begun.
/// Ready work and raw traffic never defeat shutdown or an expired deadline.
async fn guard_login_work<T>(
    deadline: (Instant, LivenessTimeout),
    shutdown: &mut watch::Receiver<bool>,
    work: impl std::future::Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<Option<T>> {
    if *shutdown.borrow() {
        return Ok(None);
    }
    if Instant::now() >= deadline.0 {
        return Err(deadline.1.into());
    }
    tokio::select! {
        biased;
        _ = shutdown.changed() => Ok(None),
        () = sleep_until(deadline.0) => Err(deadline.1.into()),
        result = work => result.map(Some),
    }
}

/// The outcome of feeding one decoded packet to the login state machine.
enum LoginProgress {
    /// Stay in login; keep reading.
    Continue,
    /// Close cleanly after a completed status flow or controlled rejection.
    Close,
    /// The client reached play.
    Play,
}

/// The canonical identity established once from Login Start and carried into
/// every later app-owned identity surface.
struct LoginIdentity {
    /// The exact, case-sensitive username after grammar validation.
    name: ValidatedUsername,
    /// A terminal/display-safe projection handed to every Play-era sink.
    ///
    /// For an admitted username this is byte-identical to `name`; retaining the
    /// projection separately makes that display boundary explicit and keeps raw
    /// hostile input away from logs, chat, metrics, plugins, and dashboards.
    safe_label: BoundedString<16>,
    /// The canonical offline-mode player identity derived from `name`.
    player: PlayerId,
}

impl LoginIdentity {
    /// Validates `name` before invoking the canonical identity derivation.
    fn try_offline(name: BoundedString<16>) -> Result<Self, InvalidUsername> {
        Self::try_offline_with(name, PlayerId::offline)
    }

    /// Validation/derivation seam used by the ordering regression.
    fn try_offline_with(
        name: BoundedString<16>,
        derive: impl FnOnce(&str) -> PlayerId,
    ) -> Result<Self, InvalidUsername> {
        let name = ValidatedUsername::new(name)?;
        let safe_label =
            BoundedString::<16>::new(display_safe_username(name.as_str())).map_err(|_| {
                // The validated grammar is ASCII and at most 16 characters, so
                // its unchanged safe projection necessarily retains this bound.
                InvalidUsername
            })?;
        let player = derive(name.as_str());
        debug_assert_eq!(safe_label.as_str(), name.as_str());
        Ok(Self {
            name,
            safe_label,
            player,
        })
    }

    /// Returns the UUID exposed in Login Success and player-facing packets.
    fn uuid(&self) -> uuid::Uuid {
        self.player.as_uuid()
    }
}

/// A Login Start name proven to match `[A-Za-z0-9_]{1,16}`.
struct ValidatedUsername(BoundedString<16>);

impl ValidatedUsername {
    /// Validates the exact wire spelling without trimming, case-folding, or
    /// Unicode normalization.
    fn new(name: BoundedString<16>) -> Result<Self, InvalidUsername> {
        let value = name.as_str();
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(InvalidUsername);
        }
        Ok(Self(name))
    }

    /// Returns the exact validated, case-sensitive spelling.
    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the bounded wire representation used by Login Success.
    fn wire(&self) -> &BoundedString<16> {
        &self.0
    }
}

/// Classified failure to satisfy the Minecraft username grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InvalidUsername;

/// Renders arbitrary untrusted username text as a single printable label.
///
/// Valid username characters are preserved. Every other Unicode scalar is
/// rendered with Rust's visible `\u{...}` notation, so controls, section signs,
/// and chat delimiters cannot become terminal or display instructions.
fn display_safe_username(name: &str) -> String {
    let mut label = String::with_capacity(name.len());
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            label.push(character);
        } else {
            label.extend(character.escape_unicode());
        }
    }
    label
}

/// One connection's mutable framing state during the login handshake.
struct Connection<'a> {
    /// The accepted socket.
    stream: TcpStream,
    /// Accumulating serverbound frame decoder.
    decoder: InboundDecoder,
    /// Clientbound frame encoder.
    encoder: OutboundEncoder,
    /// Shared (read + write) compression state, off until negotiated.
    compression: CompressionState,
    /// Shared connection context.
    ctx: &'a ConnContext,
    /// Per-connection rolling packet-trace holder, dumped on disconnect or a
    /// decode error. Boxed so the large trace rings live on the heap rather than
    /// bloating the connection task's stack/async frame.
    debug: Box<SessionDebug>,
}

impl<'a> Connection<'a> {
    /// Builds the framing state for a freshly accepted `stream`.
    fn new(stream: TcpStream, ctx: &'a ConnContext) -> Self {
        // Label the session by peer address until login upgrades it to the name.
        let label = stream
            .peer_addr()
            .map_or_else(|_| "unknown".to_string(), |addr| addr.to_string());
        Self {
            stream,
            decoder: InboundDecoder::new(ctx.limits),
            encoder: OutboundEncoder::new(ctx.limits),
            compression: CompressionState::disabled(),
            ctx,
            debug: Box::new(SessionDebug::new(label)),
        }
    }

    /// Encodes and writes one clientbound packet, bounded by the I/O timeout.
    async fn send(&mut self, packet: &OutboundPacket) -> anyhow::Result<()> {
        let mut buf = BytesMut::new();
        self.encoder
            .encode_compressed(packet, &mut buf, &self.compression)?;
        // Record an outbound trace with the exact on-wire frame size. This covers
        // the login / status / configuration phases; play frames are traced where
        // they are enqueued into the `PlayWriter`.
        let trace = observe::trace_outbound(packet, buf.len(), &self.compression, &self.ctx.clock);
        self.debug.record_outbound(trace);
        write_all(&mut self.stream, &buf, self.ctx.io_timeout).await
    }

    /// Sends the optional Set Compression and Login Success for `identity`.
    async fn send_login_success(&mut self, identity: &LoginIdentity) -> anyhow::Result<()> {
        if let Some(threshold) = self.ctx.enabled_threshold() {
            self.send(&OutboundPacket::Login(
                ClientboundLoginPacket::SetCompression(SetCompression::new(threshold)),
            ))
            .await?;
            // Set Compression itself goes out uncompressed; every later frame is
            // framed with the negotiated zlib threshold.
            self.compression = CompressionState::enabled(threshold as usize);
        }
        self.send(&OutboundPacket::Login(
            ClientboundLoginPacket::LoginSuccess(LoginSuccess::new(
                identity.uuid(),
                identity.name.wire().clone(),
                Vec::new(),
            )),
        ))
        .await
    }

    /// Advertises the built-in `minecraft:core` data pack so the client will
    /// accept NBT-omitted registry entries. The server then waits for the
    /// client's Known Packs echo before sending any registry data.
    async fn send_known_packs(&mut self) -> anyhow::Result<()> {
        let packs = self.ctx.config.known_packs().to_vec();
        self.send(&OutboundPacket::Configuration(
            ClientboundConfigurationPacket::ClientboundKnownPacks(ClientboundKnownPacks::new(
                packs,
            )),
        ))
        .await
    }

    /// Sends every enumerated registry (NBT omitted) in id-assignment order, then
    /// Finish Configuration to hand the client off to play.
    async fn send_registries_and_finish(&mut self) -> anyhow::Result<()> {
        // Detach the shared context reference so the registry borrow does not tie
        // up the `&mut self` each `send` needs.
        let ctx = self.ctx;
        for registry in ctx.config.registries() {
            self.send(&OutboundPacket::Configuration(
                ClientboundConfigurationPacket::RegistryData(registry.clone()),
            ))
            .await?;
        }
        self.send(&OutboundPacket::Configuration(
            ClientboundConfigurationPacket::FinishConfiguration(FinishConfiguration),
        ))
        .await
    }
}

/// Drives one accepted socket through login and, on success, play.
///
/// # Errors
///
/// Returns an error on a protocol violation, a framing/encode failure, a socket
/// I/O error or timeout, or loss of the simulation driver. A clean close before
/// play (a status handshake, EOF, or shutdown) is `Ok(())`.
pub(crate) async fn handle_connection(
    stream: TcpStream,
    ctx: &ConnContext,
    mut shutdown: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let mut conn = Connection::new(stream, ctx);
    match run_login(&mut conn, &mut shutdown).await? {
        Some(identity) => {
            let LoginIdentity {
                safe_label, player, ..
            } = identity;
            enter_play(conn, safe_label, player, &mut shutdown).await
        }
        None => Ok(()),
    }
}

/// Runs the login handshake, returning the canonical identity once play is
/// reached, or `None` if the connection closed cleanly first.
async fn run_login(
    conn: &mut Connection<'_>,
    shutdown: &mut watch::Receiver<bool>,
) -> anyhow::Result<Option<LoginIdentity>> {
    let mut phase = LoginPhase::Handshaking;
    let mut identity: Option<LoginIdentity> = None;
    let mut read_buf = [0u8; READ_CHUNK];
    let mut liveness = LoginLiveness::new(Instant::now(), conn.ctx.io_timeout);
    // A pipelined tail may have arrived before a slow server response. Retain
    // that read timestamp instead of starting its partial-frame clock later.
    let mut latest_read_at = Instant::now();

    loop {
        if *shutdown.borrow() {
            return Ok(None);
        }
        liveness.ensure_live(Instant::now())?;
        let state = phase.connection_state();
        // Drain everything buffered before blocking on another read so pipelined
        // frames progress without an extra round trip.
        match conn
            .decoder
            .next_packet_compressed(state, &conn.compression)
        {
            Ok(Some(packet)) => {
                // A bounded synchronous decode still consumes time; it cannot
                // retire a partial-frame clock that expired while decoding.
                liveness.ensure_live(Instant::now())?;
                liveness.complete_frame();
                // Record the inbound trace before dispatch. The login-phase
                // decoder does not surface the body length, so size is `0`
                // (documented); play-phase inbound records an exact size.
                let trace =
                    observe::trace_inbound_login(&packet, &conn.compression, &conn.ctx.clock);
                conn.debug.record_inbound(trace);
                let old_phase = phase;
                let Some(progress) = guard_login_work(
                    liveness.deadline()?,
                    shutdown,
                    advance(conn, &mut phase, &mut identity, &packet),
                )
                .await?
                else {
                    return Ok(None);
                };
                liveness.ensure_live(Instant::now())?;
                match progress {
                    LoginProgress::Continue => {
                        if phase != old_phase {
                            liveness.phase_advanced(phase.connection_state(), Instant::now())?;
                        }
                        continue;
                    }
                    LoginProgress::Close => return Ok(None),
                    LoginProgress::Play => {
                        return identity.take().map(Some).ok_or_else(|| {
                            anyhow::anyhow!("reached play without a login identity")
                        });
                    }
                }
            }
            // Buffer holds no complete frame yet: fall through and read more.
            Ok(None) => {
                if conn.decoder.buffered_len() > 0 {
                    liveness.partial_frame(latest_read_at);
                }
            }
            // A well-framed frame carrying an unmodelled packet id in login or
            // configuration is ignored, not fatal: the vanilla client sends
            // several configuration packets the slice does not react to (cookie
            // response 0x01, plugin message / brand 0x02, keep alive 0x04, pong
            // 0x05, resource pack response 0x06), and the spec requires a server
            // to skip unhandled configuration packets rather than disconnect. The
            // frame is length-delimited but `next_packet_compressed` leaves it
            // buffered on error, so it is explicitly skipped before continuing.
            // Only an unknown id is tolerated; a malformed *known* packet still
            // follows the disconnect policy below.
            Err(err) if is_ignorable_unknown_packet(state, &err) => {
                tracing::debug!(?state, %err, "ignoring unmodelled serverbound packet");
                conn.decoder.skip_frame(state)?;
                liveness.ensure_live(Instant::now())?;
                liveness.complete_frame();
                continue;
            }
            Err(err) => {
                // A fatal login/status/config decode error: count it and dump the
                // retained traces before propagating (acceptance: decode-error).
                conn.ctx.metrics.record_packet_decode_error(
                    observe::state_of(state),
                    observe::decode_error_label(&err),
                );
                conn.debug.dump("login_decode_error");
                return Err(err.into());
            }
        }

        match read_more(
            &mut conn.stream,
            &mut conn.decoder,
            &mut read_buf,
            liveness.deadline()?,
            shutdown,
        )
        .await?
        {
            ReadOutcome::Data => latest_read_at = Instant::now(),
            ReadOutcome::Eof | ReadOutcome::Shutdown => return Ok(None),
        }
    }
}

/// Whether `err` is a well-framed frame carrying an unknown (unmodelled) packet
/// id in a state where the server should ignore it rather than disconnect.
///
/// Tolerance is scoped to the login and configuration states: the vanilla client
/// legitimately sends configuration packets the slice does not model (e.g. the
/// `minecraft:brand` plugin message), and the protocol requires the server to
/// skip them. Handshaking and status remain strict — an unknown id in those
/// single-exchange states is a genuine protocol violation. A *malformed* known
/// packet ([`DecodeError::MalformedBody`] and friends) is never ignored here.
fn is_ignorable_unknown_packet(state: ConnectionState, err: &FrameDecodeError) -> bool {
    matches!(
        state,
        ConnectionState::Login | ConnectionState::Configuration
    ) && matches!(
        err,
        FrameDecodeError::Decode(DecodeError::UnknownPacket { .. })
    )
}

/// Feeds one decoded packet to the login state machine, sending replies.
async fn advance(
    conn: &mut Connection<'_>,
    phase: &mut LoginPhase,
    identity: &mut Option<LoginIdentity>,
    packet: &InboundPacket,
) -> anyhow::Result<LoginProgress> {
    match (*phase, packet) {
        (
            LoginPhase::Handshaking,
            InboundPacket::Handshake(ServerboundHandshakePacket::Handshake(handshake)),
        ) => {
            match handshake.next_state() {
                NEXT_STATE_LOGIN if handshake.protocol_version() != PROTOCOL_VERSION => {
                    let disconnect = login_disconnect(INCOMPATIBLE_PROTOCOL_REASON)?;
                    conn.send(&disconnect).await?;
                    Ok(LoginProgress::Close)
                }
                NEXT_STATE_LOGIN => {
                    *phase = LoginPhase::Login;
                    Ok(LoginProgress::Continue)
                }
                NEXT_STATE_STATUS => {
                    *phase = LoginPhase::Status;
                    Ok(LoginProgress::Continue)
                }
                // Transfer (3) or anything else is out of scope: close cleanly.
                _ => Ok(LoginProgress::Close),
            }
        }
        (LoginPhase::Status, InboundPacket::Status(ServerboundStatusPacket::StatusRequest(_))) => {
            // Copy the shared context reference out so the prebuilt response borrow
            // does not collide with the `&mut self` `send` needs.
            let ctx = conn.ctx;
            conn.send(&ctx.status_response).await?;
            *phase = LoginPhase::AwaitingStatusPing;
            Ok(LoginProgress::Continue)
        }
        (
            LoginPhase::AwaitingStatusPing,
            InboundPacket::Status(ServerboundStatusPacket::PingRequest(req)),
        ) => {
            // Echo the client's payload, then close: the status exchange ends here.
            let reply = OutboundPacket::Status(ClientboundStatusPacket::PongResponse(
                PongResponse::new(req.payload()),
            ));
            conn.send(&reply).await?;
            Ok(LoginProgress::Close)
        }
        (LoginPhase::Login, InboundPacket::Login(ServerboundLoginPacket::LoginStart(start))) => {
            let Ok(player_identity) = LoginIdentity::try_offline(start.name().clone()) else {
                let disconnect = login_disconnect(INVALID_USERNAME_REASON)?;
                conn.send(&disconnect).await?;
                return Ok(LoginProgress::Close);
            };
            // Beta-gate: reject banned / non-whitelisted players before login
            // completes (single additive access check; see ConnContext::login_denial).
            if let Some(disconnect) = conn
                .ctx
                .login_denial(player_identity.name.as_str(), player_identity.player)?
            {
                conn.send(&disconnect).await?;
                return Ok(LoginProgress::Close);
            }
            conn.send_login_success(&player_identity).await?;
            *identity = Some(player_identity);
            *phase = LoginPhase::AwaitingLoginAck;
            Ok(LoginProgress::Continue)
        }
        (
            LoginPhase::AwaitingLoginAck,
            InboundPacket::Login(ServerboundLoginPacket::LoginAcknowledged(_)),
        ) => {
            conn.send_known_packs().await?;
            *phase = LoginPhase::AwaitingKnownPacks;
            Ok(LoginProgress::Continue)
        }
        (LoginPhase::AwaitingKnownPacks, InboundPacket::Configuration(config)) => match config {
            // The client's Known Packs echo is the cue to send the registries.
            ServerboundConfigurationPacket::ServerboundKnownPacks(_) => {
                conn.send_registries_and_finish().await?;
                *phase = LoginPhase::AwaitingFinishAck;
                Ok(LoginProgress::Continue)
            }
            // Client Information (and anything else) is accepted without a reply.
            _ => Ok(LoginProgress::Continue),
        },
        (LoginPhase::AwaitingFinishAck, InboundPacket::Configuration(config)) => match config {
            ServerboundConfigurationPacket::AckFinishConfiguration(_) => Ok(LoginProgress::Play),
            // Late client settings / known packs are accepted but need no reply.
            _ => Ok(LoginProgress::Continue),
        },
        _ => anyhow::bail!("unexpected {:?} packet during {:?}", packet.state(), *phase),
    }
}

/// Builds an absolute position sync with the given teleport id, `yaw`, and `pitch`
/// (zero velocity deltas and flags), used both to spawn a joiner in at their
/// restored look and to snap a `/spawn`'d player back.
fn spawn_sync(teleport_id: i32, position: Vec3, yaw: f32, pitch: f32) -> SynchronizePlayerPosition {
    SynchronizePlayerPosition::new(
        teleport_id,
        position.x,
        position.y,
        position.z,
        0.0,
        0.0,
        0.0,
        yaw,
        pitch,
        0,
    )
}

/// The result of a single socket read during login.
enum ReadOutcome {
    /// Bytes were read and appended to the decoder.
    Data,
    /// The peer half-closed the connection.
    Eof,
    /// A shutdown was signalled while waiting to read.
    Shutdown,
}

/// Reads one chunk with the existing absolute phase/frame deadline. Partial
/// bytes do not create another I/O window.
async fn read_more(
    stream: &mut TcpStream,
    decoder: &mut InboundDecoder,
    read_buf: &mut [u8],
    deadline: (Instant, LivenessTimeout),
    shutdown: &mut watch::Receiver<bool>,
) -> anyhow::Result<ReadOutcome> {
    let Some(n) = guard_login_work(deadline, shutdown, async {
        stream.read(read_buf).await.map_err(Into::into)
    })
    .await?
    else {
        return Ok(ReadOutcome::Shutdown);
    };
    if n == 0 {
        return Ok(ReadOutcome::Eof);
    }
    decoder.push(&read_buf[..n])?;
    Ok(ReadOutcome::Data)
}

/// Writes every byte of `bytes` and flushes, each bounded by `io_timeout`.
async fn write_all(
    stream: &mut TcpStream,
    bytes: &[u8],
    io_timeout: Duration,
) -> anyhow::Result<()> {
    timeout(io_timeout, stream.write_all(bytes))
        .await
        .map_err(|_| anyhow::anyhow!("socket write timed out"))??;
    timeout(io_timeout, stream.flush())
        .await
        .map_err(|_| anyhow::anyhow!("socket flush timed out"))??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[tokio::test(start_paused = true)]
    async fn partial_login_drips_keep_the_first_byte_deadline() {
        let began = Instant::now();
        let mut live = LoginLiveness::new(began, Duration::from_secs(30));
        tokio::time::advance(Duration::from_secs(2)).await;
        live.partial_frame(Instant::now());
        for _ in 0..9 {
            tokio::time::advance(Duration::from_secs(1)).await;
            live.partial_frame(Instant::now());
            live.ensure_live(Instant::now())
                .expect("before first frame expiry");
        }
        assert_eq!(
            live.deadline().expect("active").0,
            began + Duration::from_secs(12)
        );
        tokio::time::advance(Duration::from_secs(1)).await;
        let error = live.ensure_live(Instant::now()).expect_err("exact expiry");
        assert_eq!(
            error.downcast_ref(),
            Some(&LivenessTimeout::FrameCompletion {
                state: ConnectionState::Handshaking,
            })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn ignored_login_frames_clear_partial_work_without_extending_phase_progress() {
        let began = Instant::now();
        let mut live = LoginLiveness::new(began, Duration::from_secs(30));
        live.phase_advanced(ConnectionState::Configuration, began)
            .expect("configuration");
        for _ in 0..29 {
            tokio::time::advance(Duration::from_secs(1)).await;
            live.partial_frame(Instant::now());
            live.complete_frame();
            live.ensure_live(Instant::now())
                .expect("still before phase expiry");
        }
        assert_eq!(
            live.deadline().expect("active").0,
            began + Duration::from_secs(30)
        );
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(
            live.ensure_live(Instant::now())
                .expect_err("unknown traffic is not progress")
                .downcast_ref(),
            Some(&LivenessTimeout::StateProgress {
                state: ConnectionState::Configuration
            }),
        );
    }

    #[tokio::test(start_paused = true)]
    async fn proper_login_phase_transition_grants_one_new_window() {
        let began = Instant::now();
        let mut live = LoginLiveness::new(began, Duration::from_secs(30));
        tokio::time::advance(Duration::from_secs(29)).await;
        live.complete_frame();
        live.phase_advanced(ConnectionState::Login, Instant::now())
            .expect("handshake completed");
        assert_eq!(
            live.deadline().expect("login").0,
            began + Duration::from_secs(59)
        );
        tokio::time::advance(Duration::from_secs(30)).await;
        assert!(live
            .phase_advanced(ConnectionState::Configuration, Instant::now())
            .is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn pipelined_partial_tail_uses_arrival_time_before_server_response() {
        let read_at = Instant::now();
        let mut live = LoginLiveness::new(read_at, Duration::from_secs(30));
        tokio::time::advance(Duration::from_secs(9)).await;
        live.complete_frame();
        live.phase_advanced(ConnectionState::Login, Instant::now())
            .expect("response completed");
        live.partial_frame(read_at);
        assert_eq!(
            live.deadline().expect("tail").0,
            read_at + Duration::from_secs(10)
        );
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(live.ensure_live(Instant::now()).is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn login_guard_prioritizes_exact_expiry_and_shutdown_over_ready_work() {
        let began = Instant::now();
        let live = LoginLiveness::new(began, Duration::from_secs(1));
        let (stop, mut shutdown) = watch::channel(false);
        tokio::time::advance(Duration::from_secs(1)).await;
        let called = Cell::new(false);
        let error = guard_login_work(live.deadline().expect("deadline"), &mut shutdown, async {
            called.set(true);
            Ok(())
        })
        .await
        .expect_err("already expired");
        assert_eq!(
            error.downcast_ref(),
            Some(&LivenessTimeout::StateProgress {
                state: ConnectionState::Handshaking,
            })
        );
        assert!(!called.get());
        stop.send(true).expect("stop");
        assert!(
            guard_login_work(live.deadline().expect("deadline"), &mut shutdown, async {
                panic!("stopped work must never run");
                #[allow(unreachable_code)]
                Ok(())
            })
            .await
            .expect("clean shutdown")
            .is_none()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn stalled_pre_play_output_is_canceled_at_the_phase_deadline() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        struct Dropped(Arc<AtomicBool>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        let live = LoginLiveness::new(Instant::now(), Duration::from_secs(1));
        let (_stop, mut shutdown) = watch::channel(false);
        let dropped = Arc::new(AtomicBool::new(false));
        let marker = Dropped(Arc::clone(&dropped));
        let deadline = live.deadline().expect("deadline");
        let task = tokio::spawn(async move {
            guard_login_work(deadline, &mut shutdown, async move {
                let _marker = marker;
                std::future::pending::<anyhow::Result<()>>().await
            })
            .await
        });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(task.await.expect("join").is_err());
        assert!(dropped.load(Ordering::Relaxed));
    }

    #[test]
    fn login_identity_is_derived_once() {
        let name = BoundedString::<16>::new("IdentityProbe".to_string())
            .expect("test username is within the protocol bound");
        let identity = LoginIdentity::try_offline(name).expect("test username is valid");

        assert_eq!(identity.player, PlayerId::offline("IdentityProbe"));
        assert_eq!(identity.uuid(), identity.player.as_uuid());
        assert_eq!(identity.name.as_str(), "IdentityProbe");
        assert_eq!(identity.safe_label.as_str(), "IdentityProbe");
    }

    #[test]
    fn username_grammar_covers_boundaries_and_classes() {
        for valid in ["A", "_", "Alpha_123", "abcdefghijklmnop"] {
            let name = BoundedString::<16>::new(valid.to_string()).expect("wire-bounded username");
            assert!(
                ValidatedUsername::new(name).is_ok(),
                "{valid:?} should be valid"
            );
        }

        for invalid in [
            "",
            "玩家",
            "Bad Name",
            "Bad\nName",
            "Bad\u{1b}Name",
            "<Admin>",
            "\u{a7}cAdmin",
            "Bad-Name",
        ] {
            let name =
                BoundedString::<16>::new(invalid.to_string()).expect("semantic invalid is bounded");
            assert_eq!(
                ValidatedUsername::new(name).err(),
                Some(InvalidUsername),
                "{invalid:?} should be invalid"
            );
        }
    }

    #[test]
    fn invalid_username_cannot_reach_identity_derivation() {
        let derived = Cell::new(false);
        let name =
            BoundedString::<16>::new("Bad Name".to_string()).expect("invalid test name is bounded");
        let result = LoginIdentity::try_offline_with(name, |value| {
            derived.set(true);
            PlayerId::offline(value)
        });

        assert_eq!(result.err(), Some(InvalidUsername));
        assert!(!derived.get(), "invalid input reached UUID derivation");
    }

    #[test]
    fn display_projection_escapes_every_hostile_name_character() {
        assert_eq!(
            display_safe_username("A\n\u{1b}\u{a7}<>_9"),
            "A\\u{a}\\u{1b}\\u{a7}\\u{3c}\\u{3e}_9"
        );
        assert_eq!(display_safe_username("Valid_Name9"), "Valid_Name9");
    }
}
