//! Per-connection chunk-streaming state and the bounded, center-out pump that
//! keeps a client's loaded chunk square in sync with its position.

use std::collections::BTreeSet;

use tokio::sync::oneshot;
use tokio::time::sleep_until;

use ferrumc_core::PlayerId;
use ferrumc_math::{BlockPos, ChunkPos, Vec3};
use ferrumc_net::{CompressionState, LivenessDeadline, OutboundPriority, PlayWriter};
use ferrumc_observability::SessionDebug;
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, SetCenterChunk, SynchronizePlayerPosition, UnloadChunk,
};

use crate::driver::{SimCommand, StreamedChunk};
use crate::player_data::is_valid_player_position;

use super::context::ConnContext;
use super::outbound::{enqueue_traced, send_mandatory};

/// Maximum number of `ChunkDataAndLight` packets a single streaming evaluation
/// will request and send for one player.
///
/// Each chunk packet carries the full section + light payload (tens of KiB), so
/// an uncapped burst — a teleport that makes the whole view square new, or the
/// initial gap between the small spawn batch and a large view distance — could
/// dump megabytes onto one socket at once. Capping the per-update load count
/// paces the stream: the leftover chunks are picked up on the next stream
/// cadence (the desired-vs-loaded diff is recomputed every pump), nearest-first,
/// so the player always gets the chunks closest to them first. `16` keeps a
/// normal single-chunk step fully served in one update while bounding a teleport
/// flood.
const MAX_CHUNK_LOADS_PER_UPDATE: usize = crate::driver::MAX_STREAM_ADMISSIONS;

/// Maximum number of departed chunk columns released by one stream pump.
///
/// Releasing a player ticket crosses the driver's durability barrier and can
/// evict a chunk, so the tiny `UnloadChunk` wire size is irrelevant to the true
/// cost. Matching the load bound limits one pump to at most 32 chunk mutations.
/// The remainder is recomputed from the latest centre on the next cadence tick.
const MAX_CHUNK_UNLOADS_PER_UPDATE: usize = 16;

/// Upper bound applied to the configured view distance when streaming chunks.
///
/// The streamed view is a `(2 * r + 1)` square, so the per-player loaded-chunk
/// set is bounded by `(2 * r + 1)^2`. Clamping `r` here (to the vanilla view-
/// distance ceiling) keeps that set — and the work a single boundary crossing can
/// request — bounded even if the config carries an absurd view distance.
pub(super) const STREAM_VIEW_DISTANCE_MAX: i32 = 32;

/// Per-connection chunk-streaming state: which chunk the client is centred on and
/// which chunk columns it currently holds.
///
/// This is connection/session bookkeeping, not world state: it records owned
/// player tickets before their packets are sent, so any later encoding or socket
/// failure still releases exactly those tickets. The authoritative chunk data still lives in the simulation shard;
/// the connection only ever asks the driver (via [`SimCommand::StreamChunks`]) to
/// load-or-generate and never touches the chunk map itself.
pub(super) struct ChunkStream {
    /// The chunk the client is currently centred on (the last `Set Center Chunk`).
    center: ChunkPos,
    /// Canonical connection identity, used to keep background priorities fair.
    viewer: PlayerId,
    /// The square radius, in chunks, of the streamed view (already clamped).
    view_distance: i32,
    /// Actual acquired player tickets, recorded before packet encoding/flush.
    /// Tail-dropped or unsent packets still require this owned ticket release.
    pub(super) loaded: BTreeSet<ChunkPos>,
    /// Columns whose chunk and block-entity packets actually entered the writer.
    pub(super) sent: BTreeSet<ChunkPos>,
    /// Recent movement direction prioritizes a small visible corridor and outer baselines.
    movement_hint: ChunkPos,
    /// A join request was admitted but its exact acquired batch is unconfirmed.
    /// Failed handoff quarantines the UUID; guessed ticket releases are unsafe.
    pub(super) ticket_handoff_pending: bool,
    /// The latest accepted client position or authoritative server teleport since
    /// the last fixed-cadence evaluation. Replacing this slot coalesces every
    /// superseded target into one streaming pass.
    pending_position: Option<Vec3>,
    /// The latest accepted client position or authoritative server teleport
    /// (never cleared), used to persist where the player was standing when they
    /// leave. If neither occurs, it remains `None` and the join position is
    /// persisted instead.
    last_position: Option<Vec3>,
    /// The integer block of the latest accepted client position or authoritative
    /// teleport, used to throttle the `PlayerMove` plugin event to block
    /// granularity. `None` until the first accepted position establishes a baseline
    /// (so the very first packet only seeds it and does not fire), see
    /// [`block_crossing`](ChunkStream::block_crossing).
    last_move_block: Option<BlockPos>,
}

impl ChunkStream {
    /// Starts before join-kit acquisition with no owned player tickets.
    pub(super) fn before_join(ctx: &ConnContext, viewer: PlayerId) -> Self {
        Self {
            center: ctx.join_kit.spawn_chunk(),
            viewer,
            view_distance: ctx.view_distance.clamp(0, STREAM_VIEW_DISTANCE_MAX),
            loaded: BTreeSet::new(),
            sent: BTreeSet::new(),
            movement_hint: ChunkPos::new(0, 0),
            ticket_handoff_pending: false,
            pending_position: None,
            last_position: None,
            last_move_block: None,
        }
    }

    /// Test fixture representing a client that has received the whole spawn kit.
    #[cfg(test)]
    pub(super) fn new(ctx: &ConnContext) -> Self {
        let mut stream = Self::before_join(ctx, PlayerId::offline("chunk_fixture"));
        stream.loaded = ctx.join_kit.chunk_positions().collect();
        stream.sent = stream.loaded.clone();
        stream
    }

    /// Establishes the initial wire centre at the actual restored or fresh position.
    /// No world-spawn centre is sent for a player returning elsewhere.
    pub(super) fn initialize_center(&mut self, position: Vec3) -> ChunkPos {
        self.center = chunk_of(position);
        self.observe(position);
        self.center
    }

    /// Client render radius is capped by the configured server view; an absent
    /// or negative client sentinel retains that configured cap.
    pub(super) fn limit_view_distance(&mut self, server_radius: i32, client_radius: Option<i8>) {
        self.view_distance = effective_view_distance(server_radius, client_radius);
    }

    /// Records the client's latest validated and admitted position.
    pub(super) fn observe(&mut self, position: Vec3) {
        if let Some(previous) = self.last_position {
            let direction = |delta: f64| {
                if delta > 0.01 {
                    1
                } else if delta < -0.01 {
                    -1
                } else {
                    0
                }
            };
            let hint = ChunkPos::new(
                direction(position.x - previous.x),
                direction(position.z - previous.z),
            );
            // Vertical movement and idle X/Z packets must not erase the last
            // useful horizontal direction before a boundary crossing.
            if hint != ChunkPos::ORIGIN {
                self.movement_hint = hint;
            }
        }
        self.pending_position = Some(position);
        self.last_position = Some(position);
    }

    /// The latest accepted client position or authoritative server teleport, or
    /// `None` if neither occurred (in which case the caller persists the join
    /// position).
    pub(super) fn last_position(&self) -> Option<Vec3> {
        self.last_position
    }

    /// Returns the coalesced target for handler-boundary regression tests.
    #[cfg(test)]
    pub(super) fn pending_position_for_test(&self) -> Option<Vec3> {
        self.pending_position
    }

    /// Returns the delivered stream centre for queue-overflow regression tests.
    #[cfg(test)]
    pub(super) fn center_for_test(&self) -> ChunkPos {
        self.center
    }

    /// Records `position` and returns the `(from, to)` block crossing if the player
    /// entered a *new* integer block since the last reported crossing, else `None`.
    ///
    /// This throttles the `PlayerMove` plugin event to block granularity: many
    /// sub-block movement packets within one block collapse to no event, and the
    /// event fires once per block the player enters. The first call after join only
    /// seeds the baseline (there is no prior block to report a crossing *from*), so
    /// it returns `None`. Coordinates are floored to block space (matching vanilla:
    /// block `x = -0.5` is block `-1`).
    pub(super) fn block_crossing(&mut self, position: Vec3) -> Option<(BlockPos, BlockPos)> {
        let to = BlockPos::new(
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        );
        let crossing = match self.last_move_block {
            Some(from) if from == to => None,
            Some(from) => Some((from, to)),
            None => None,
        };
        self.last_move_block = Some(to);
        crossing
    }

    /// Records one authoritative server teleport for persistence and streaming.
    ///
    /// A server-driven teleport is already authoritative, so waiting for a client
    /// echo would leave simulation/router/save at the destination while tickets
    /// remain at the old centre. The fixed-cadence pump coalesces this pending
    /// destination with any later accepted observation.
    pub(super) fn mirror_teleport(&mut self, position: Vec3) {
        self.pending_position = Some(position);
        self.last_position = Some(position);
        // A server teleport establishes a new plugin movement baseline. Retaining
        // the pre-teleport block would turn the first client echo or tiny step at
        // the destination into one false world-spanning PlayerMove notification.
        self.last_move_block = Some(block_of(position));
        self.movement_hint = ChunkPos::new(0, 0);
    }
}

/// Mirrors a server-driven absolute teleport into the persistence state so a later
/// leave-save captures the teleported position and look, not the stale
/// client-reported one.
///
/// The server snaps a player with an authoritative `SynchronizePlayerPosition` for
/// `/spawn`, a routed plugin `Teleport`, or an anti-cheat correction. The mirror is
/// otherwise advanced only by validated client movement packets. Only safe,
/// fully-absolute syncs (`flags == 0`, the only kind the server emits) are
/// mirrored; a partially-relative or non-finite/out-of-range sync is left
/// untouched because it is not a trustworthy absolute destination.
pub(super) fn mirror_server_teleport(
    chunk_stream: &mut ChunkStream,
    player_yaw: &mut f32,
    player_pitch: &mut f32,
    sync: &SynchronizePlayerPosition,
) {
    let position = Vec3::new(sync.x(), sync.y(), sync.z());
    if sync.flags() != 0
        || !is_valid_player_position(position)
        || !sync.yaw().is_finite()
        || !sync.pitch().is_finite()
    {
        return;
    }
    chunk_stream.mirror_teleport(position);
    *player_yaw = sync.yaw();
    *player_pitch = sync.pitch();
}

/// Streams chunks toward the latest accepted movement or authoritative teleport,
/// then advances the view toward the full view distance.
///
/// If a new target arrived since the last call, this recenters on it (sending
/// `Set Center Chunk` on a chunk-boundary crossing). Either way it then runs
/// [`pump_chunk_stream`] against the current center, so the view advances even
/// when no target arrived (a freshly-joined or standing player). A chunk already
/// in the loaded set is never re-requested or re-sent.
///
/// The connection never generates chunks itself: it only decides the desired set
/// and renders the packets the driver returns.
pub(super) async fn apply_chunk_stream(
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    chunk_stream: &mut ChunkStream,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    deadline: Option<LivenessDeadline>,
) -> anyhow::Result<()> {
    // Position path: recenter on the latest accepted target, if any.
    if let Some(position) = chunk_stream.pending_position.take() {
        let new_center = chunk_of(position);
        if new_center != chunk_stream.center {
            // The centre is mandatory client state. Do not advance internal
            // ownership/tickets if a full State queue drops it; fail the session
            // before the load/unload command instead.
            send_mandatory(
                writer,
                debug,
                compression,
                &ctx.clock,
                ClientboundPlayPacket::SetCenterChunk(SetCenterChunk::new(
                    new_center.x(),
                    new_center.z(),
                )),
            )?;
            chunk_stream.center = new_center;
        }
    }

    // Always advance the view toward full view distance from the current center,
    // whether or not a position packet moved it.
    pump_chunk_stream(ctx, writer, chunk_stream, debug, compression, deadline).await
}

/// Advances the chunk view one bounded batch toward the full view distance around
/// the stream's current center, independent of any position packet.
///
/// Diffs the `(2 * view_distance + 1)` square against the per-player loaded set,
/// sends `Unload Chunk` for any column that left the radius, and asks the driver
/// (via [`SimCommand::StreamChunks`]) to load-or-generate the columns newly in
/// range — nearest-first and capped at [`MAX_CHUNK_LOADS_PER_UPDATE`] per call (see
/// [`next_chunk_transition`]), with the remainder caught on a later pump. Driven
/// at the fixed stream cadence, so a non-moving joiner still fills out to the
/// advertised view distance while a packet flood cannot run this work once per
/// socket read.
pub(super) async fn pump_chunk_stream(
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    chunk_stream: &mut ChunkStream,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    deadline: Option<LivenessDeadline>,
) -> anyhow::Result<()> {
    let clock = &ctx.clock;
    let center = chunk_stream.center;

    let desired = desired_chunks(center, chunk_stream.view_distance);
    let mut to_retry: Vec<_> = desired
        .intersection(&chunk_stream.loaded)
        .filter(|pos| !chunk_stream.sent.contains(pos))
        .copied()
        .collect();
    to_retry.sort_by_key(|pos| (chebyshev_distance(center, *pos), pos.x(), pos.z()));
    to_retry.truncate(MAX_CHUNK_LOADS_PER_UPDATE);
    let transition = next_chunk_transition(
        center,
        &desired,
        &chunk_stream.loaded,
        crate::chunk_preparation::MAX_PREPARATION_TARGETS,
        MAX_CHUNK_UNLOADS_PER_UPDATE,
        chunk_stream.movement_hint,
    );
    let to_load = transition.load;
    // One extra retained ring prevents immediate unload/regenerate churn when
    // hovering at a chunk boundary. It is not an expanded client send radius.
    let retained = desired_chunks(center, chunk_stream.view_distance + 1);
    let to_unload: Vec<_> = chunk_stream
        .loaded
        .difference(&retained)
        .take(MAX_CHUNK_UNLOADS_PER_UPDATE)
        .copied()
        .collect();
    let prefetch = prefetch_targets(
        center,
        chunk_stream.view_distance,
        chunk_stream.movement_hint,
        &chunk_stream.loaded,
    );
    let prefetch: Vec<_> = prefetch
        .into_iter()
        .filter(|pos| chebyshev_distance(center, *pos) > i64::from(chunk_stream.view_distance))
        .take(crate::chunk_preparation::MAX_PREPARATION_TARGETS.saturating_sub(to_load.len()))
        .collect();
    if to_load.is_empty() && to_unload.is_empty() && to_retry.is_empty() {
        // Even a complete visible view prepares an outer ring before movement.
        ctx.commands
            .try_send(SimCommand::PrepareChunks {
                viewer: chunk_stream.viewer,
                positions: prefetch,
            })
            .ok(); // A full bounded command queue defers speculative work.
        return Ok(());
    }
    let retry_count = to_retry.len();
    let mut packets = Vec::new();
    if !to_retry.is_empty() {
        packets = resend_chunk_snapshots(ctx, to_retry, deadline).await?;
    }

    // Queue departed columns now, but retain their ticket bookkeeping until the
    // command is admitted. An absolute deadline may cancel the bounded send;
    // teardown must then still release every ticket the driver has not seen.
    for pos in &to_unload {
        let outcome = enqueue_traced(
            writer,
            debug,
            compression,
            clock,
            OutboundPriority::World,
            ClientboundPlayPacket::UnloadChunk(UnloadChunk::new(pos.z(), pos.x())),
        );
        anyhow::ensure!(
            outcome.is_enqueued(),
            "outbound overflow while retiring a chunk column"
        );
        ctx.metrics.incr_chunk_unloaded(1);
    }

    let (reply_tx, reply_rx) = oneshot::channel();
    let send = async {
        ctx.commands
            .send(SimCommand::StreamChunks {
                viewer: chunk_stream.viewer,
                load: to_load,
                load_limit: MAX_CHUNK_LOADS_PER_UPDATE - retry_count,
                unload: to_unload.clone(),
                prefetch,
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver is gone"))
    };
    if deadline.is_some() {
        super::play::guard_play_work(deadline, send).await?;
    } else {
        send.await?;
    }
    for pos in &to_unload {
        chunk_stream.loaded.remove(pos);
        chunk_stream.sent.remove(pos);
    }
    packets.extend(receive_stream_reply(reply_rx, deadline, &mut chunk_stream.loaded).await?);

    for streamed in packets {
        queue_streamed_chunk(ctx, writer, debug, compression, chunk_stream, streamed);
    }
    Ok(())
}

/// Snapshot retries never acquire or transfer tickets, including canceled replies.
async fn resend_chunk_snapshots(
    ctx: &ConnContext,
    positions: Vec<ChunkPos>,
    deadline: Option<LivenessDeadline>,
) -> anyhow::Result<Vec<StreamedChunk>> {
    let (reply, receive) = oneshot::channel();
    let retry = async {
        ctx.commands
            .send(SimCommand::ResendChunks { positions, reply })
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver is gone"))?;
        receive
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver dropped snapshot reply"))
    };
    if deadline.is_some() {
        super::play::guard_play_work(deadline, retry).await
    } else {
        retry.await
    }
}

/// Ticket ownership and packet admission are separate facts. Backpressure retains
/// the ticket while the next pump retries a fresh resident snapshot.
pub(super) fn queue_streamed_chunk(
    ctx: &ConnContext,
    writer: &mut PlayWriter,
    debug: &mut SessionDebug,
    compression: &CompressionState,
    chunk_stream: &mut ChunkStream,
    streamed: StreamedChunk,
) {
    let pos = ChunkPos::new(streamed.chunk.x(), streamed.chunk.z());
    chunk_stream.loaded.insert(pos);
    let outcome = enqueue_traced(
        writer,
        debug,
        compression,
        &ctx.clock,
        OutboundPriority::World,
        ClientboundPlayPacket::ChunkDataAndLight(streamed.chunk),
    );
    let mut complete = outcome.is_enqueued();
    if complete {
        ctx.metrics.incr_chunk_sent(1);
    }
    for block_entity in streamed.block_entities {
        complete &= enqueue_traced(
            writer,
            debug,
            compression,
            &ctx.clock,
            OutboundPriority::World,
            block_entity,
        )
        .is_enqueued();
    }
    if complete {
        chunk_stream.sent.insert(pos);
    } else {
        chunk_stream.sent.remove(&pos);
    }
}

/// Visible targets precede an outer speculative ring, biased toward movement.
fn prefetch_targets(
    center: ChunkPos,
    radius: i32,
    direction: ChunkPos,
    held: &BTreeSet<ChunkPos>,
) -> Vec<ChunkPos> {
    let lead = ChunkPos::new(
        center.x().saturating_add(direction.x()),
        center.z().saturating_add(direction.z()),
    );
    let mut targets: Vec<_> = desired_chunks(center, radius + 2)
        .into_iter()
        .filter(|pos| !held.contains(pos))
        .collect();
    targets.sort_by_key(|pos| {
        (
            chebyshev_distance(center, *pos) > i64::from(radius),
            chebyshev_distance(lead, *pos),
            chebyshev_distance(center, *pos),
            pos.x(),
            pos.z(),
        )
    });
    targets.truncate(crate::chunk_preparation::MAX_PREPARATION_TARGETS);
    targets
}

/// Closing before inspecting a timed-out receiver makes the ticket handoff
/// atomic: an already delivered batch is retained for teardown; every later
/// driver send fails and releases its own actual acquired tickets.
async fn receive_stream_reply(
    mut reply: oneshot::Receiver<Vec<StreamedChunk>>,
    deadline: Option<LivenessDeadline>,
    loaded: &mut BTreeSet<ChunkPos>,
) -> anyhow::Result<Vec<StreamedChunk>> {
    if let Some(deadline) = deadline {
        tokio::select! {
            biased;
            () = sleep_until(deadline.at()) => {
                reply.close();
                if let Ok(packets) = reply.try_recv() {
                    for packet in packets {
                        loaded.insert(ChunkPos::new(packet.chunk.x(), packet.chunk.z()));
                    }
                }
                Err(deadline.timeout().into())
            }
            packets = &mut reply => packets.map_err(|_| anyhow::anyhow!("simulation driver dropped the chunk-stream reply")),
        }
    } else {
        reply
            .await
            .map_err(|_| anyhow::anyhow!("simulation driver dropped the chunk-stream reply"))
    }
}

fn effective_view_distance(server_radius: i32, client_radius: Option<i8>) -> i32 {
    let server = server_radius.clamp(0, STREAM_VIEW_DISTANCE_MAX);
    client_radius
        .filter(|radius| *radius >= 0)
        .map_or(server, |radius| server.min(i32::from(radius)))
}

/// The chunk column the world `position` falls in (flooring so negatives land on
/// the correct column).
fn chunk_of(position: Vec3) -> ChunkPos {
    block_of(position).to_chunk_pos()
}

/// The integer block containing a validated world position.
fn block_of(position: Vec3) -> BlockPos {
    BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

/// The `(2 * radius + 1)` square of chunk columns centred on `center`.
///
/// Coordinates are added saturating so a centre at the edge of the coordinate
/// range can never overflow; any realistic position is exact.
fn desired_chunks(center: ChunkPos, radius: i32) -> BTreeSet<ChunkPos> {
    let mut set = BTreeSet::new();
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            set.insert(ChunkPos::new(
                center.x().saturating_add(dx),
                center.z().saturating_add(dz),
            ));
        }
    }
    set
}

/// The Chebyshev (square/king-move) chunk distance between `a` and `b`.
///
/// Computed in `i64` so the difference cannot overflow at the coordinate
/// extremes.
fn chebyshev_distance(a: ChunkPos, b: ChunkPos) -> i64 {
    let dx = (i64::from(a.x()) - i64::from(b.x())).abs();
    let dz = (i64::from(a.z()) - i64::from(b.z())).abs();
    dx.max(dz)
}

fn manhattan_distance(a: ChunkPos, b: ChunkPos) -> i64 {
    (i64::from(a.x()) - i64::from(b.x())).abs() + (i64::from(a.z()) - i64::from(b.z())).abs()
}

/// The current column stays first. A moving client gets its next two central
/// columns, then their one-column lateral margins before whole side/rear rings.
/// This is a bounded `FerrumC` latency policy, not Paper's loader comparator.
fn movement_corridor_rank(center: ChunkPos, direction: ChunkPos, pos: ChunkPos) -> u8 {
    if pos == center {
        return 0;
    }
    if direction == ChunkPos::ORIGIN {
        return 3;
    }
    let dx = direction.x().signum();
    let dz = direction.z().signum();
    for step in 1..=2 {
        let ahead = ChunkPos::new(
            center.x().saturating_add(dx * step),
            center.z().saturating_add(dz * step),
        );
        if pos == ahead {
            return 1;
        }
    }
    for step in 1..=2 {
        for side in [-1, 1] {
            let margin = ChunkPos::new(
                center
                    .x()
                    .saturating_add(dx * step)
                    .saturating_add(-dz * side),
                center
                    .z()
                    .saturating_add(dz * step)
                    .saturating_add(dx * side),
            );
            if pos == margin {
                return 2;
            }
        }
    }
    3
}

/// One bounded transition toward the latest desired chunk square.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChunkTransition {
    /// Missing desired columns to acquire, center/corridor first then nearest.
    load: Vec<ChunkPos>,
    /// Stale held columns to release, in deterministic coordinate order.
    unload: Vec<ChunkPos>,
}

/// Computes one bounded load/unload transition toward `desired`.
///
/// The desired set is built once by the caller. Missing columns are sorted
/// center-out for useful visual progress; stale columns already come from a
/// `BTreeSet` difference, so taking its prefix is deterministic without sorting
/// all 4,225 maximum-radius candidates. No queue is retained: every call
/// recomputes against the newest centre, which coalesces superseded teleports.
fn next_chunk_transition(
    center: ChunkPos,
    desired: &BTreeSet<ChunkPos>,
    loaded: &BTreeSet<ChunkPos>,
    load_bound: usize,
    unload_bound: usize,
    direction: ChunkPos,
) -> ChunkTransition {
    let mut to_load: Vec<ChunkPos> = desired.difference(loaded).copied().collect();
    let lead = ChunkPos::new(
        center.x().saturating_add(direction.x()),
        center.z().saturating_add(direction.z()),
    );
    to_load.sort_by_key(|pos| {
        (
            movement_corridor_rank(center, direction, *pos),
            chebyshev_distance(center, *pos),
            // Only the first unheld spawn ring needs unknown-heading cardinal
            // support. Keep later rings contiguous in the original x/z order
            // to preserve overlapping terrain halos and bounded-cache reuse.
            if direction == ChunkPos::ORIGIN && chebyshev_distance(center, *pos) == 2 {
                manhattan_distance(center, *pos)
            } else {
                0
            },
            chebyshev_distance(lead, *pos),
            pos.x(),
            pos.z(),
        )
    });
    to_load.truncate(load_bound);
    let to_unload = loaded
        .difference(desired)
        .take(unload_bound)
        .copied()
        .collect();
    ChunkTransition {
        load: to_load,
        unload: to_unload,
    }
}

#[cfg(test)]
mod tests {
    fn streamed_fixture(pos: ferrumc_math::ChunkPos) -> crate::driver::StreamedChunk {
        let chunk = ferrumc_world::FlatWorldGenerator::new().generate(pos);
        crate::driver::StreamedChunk {
            chunk: crate::world::chunk_packet(pos, &chunk).expect("fixture chunk encodes"),
            block_entities: Vec::new(),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_reply_recovers_an_already_delivered_unconsumed_ticket() {
        use ferrumc_math::ChunkPos;
        use ferrumc_net::LivenessTimeout;
        use std::collections::BTreeSet;
        use std::time::Duration;
        use tokio::sync::oneshot;
        use tokio::time::{advance, Instant};

        let mut liveness =
            super::super::play::play_liveness(Instant::now(), Duration::from_secs(30))
                .expect("Play tracker");
        liveness
            .keep_alive_sent(7, Instant::now())
            .expect("request sent");
        let pos = ChunkPos::new(4, 5);
        let (sender, receiver) = oneshot::channel();
        assert!(sender.send(vec![streamed_fixture(pos)]).is_ok());
        // Delivery succeeded, but the connection has not yet polled the reply.
        advance(Duration::from_secs(15)).await;
        let mut loaded = BTreeSet::new();
        let error = super::receive_stream_reply(receiver, liveness.next_deadline(), &mut loaded)
            .await
            .err()
            .expect("the response deadline still wins");
        assert_eq!(
            error.downcast_ref(),
            Some(&LivenessTimeout::KeepAlive { id: 7 })
        );
        assert_eq!(
            loaded,
            BTreeSet::from([pos]),
            "teardown retains the actual acquired ticket"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_reply_closes_before_a_later_driver_delivery() {
        use ferrumc_math::ChunkPos;
        use std::collections::BTreeSet;
        use std::time::Duration;
        use tokio::sync::oneshot;
        use tokio::time::{advance, Instant};

        let mut liveness =
            super::super::play::play_liveness(Instant::now(), Duration::from_secs(30))
                .expect("Play tracker");
        liveness
            .keep_alive_sent(7, Instant::now())
            .expect("request sent");
        let (sender, receiver) = oneshot::channel();
        advance(Duration::from_secs(15)).await;
        let mut loaded = BTreeSet::new();
        super::receive_stream_reply(receiver, liveness.next_deadline(), &mut loaded)
            .await
            .err()
            .expect("undelivered reply timed out");
        let returned = sender
            .send(vec![streamed_fixture(ChunkPos::new(4, 5))])
            .expect_err("closed receiver returns the driver's acquired ticket for durable cleanup");
        assert_eq!(returned.len(), 1);
        assert_eq!(
            loaded.len(),
            0,
            "the driver owns cleanup of the undelivered ticket"
        );
    }

    #[test]
    fn chunk_pump_makes_center_out_progress_without_a_position_packet() {
        use std::collections::BTreeSet;

        use ferrumc_math::ChunkPos;

        use super::{
            chebyshev_distance, desired_chunks, next_chunk_transition, MAX_CHUNK_LOADS_PER_UPDATE,
            MAX_CHUNK_UNLOADS_PER_UPDATE,
        };

        let center = ChunkPos::new(0, 0);
        let view_distance = 10;
        // A fresh joiner holds only the spawn batch (radius-2 square, 25 columns)
        // before sending any movement packet.
        let mut loaded: BTreeSet<ChunkPos> = desired_chunks(center, 2);

        // The first batch is the nearest ring (center-out): every column sits at the
        // closest missing Chebyshev distance — 3, just outside the spawn batch.
        let desired = desired_chunks(center, view_distance);
        let first = next_chunk_transition(
            center,
            &desired,
            &loaded,
            MAX_CHUNK_LOADS_PER_UPDATE,
            MAX_CHUNK_UNLOADS_PER_UPDATE,
            ChunkPos::ORIGIN,
        );
        assert_eq!(first.load.len(), MAX_CHUNK_LOADS_PER_UPDATE);
        assert_eq!(first.unload, []);
        assert!(first
            .load
            .iter()
            .all(|pos| chebyshev_distance(center, *pos) == 3));

        // Driving the pump repeatedly — with NO position packet ever — fills the
        // whole advertised view square. Each call simulates the driver loading the
        // returned batch.
        let mut iterations = 0;
        loop {
            let transition = next_chunk_transition(
                center,
                &desired,
                &loaded,
                MAX_CHUNK_LOADS_PER_UPDATE,
                MAX_CHUNK_UNLOADS_PER_UPDATE,
                ChunkPos::ORIGIN,
            );
            if transition.load.is_empty() {
                break;
            }
            assert_eq!(transition.unload, []);
            for pos in transition.load {
                loaded.insert(pos);
            }
            iterations += 1;
            assert!(iterations < 1000, "the pump must terminate");
        }
        assert!(
            desired.is_subset(&loaded),
            "the pump fills out to the full advertised view distance"
        );
        // 441 desired - 25 seeded = 416 columns, in bounded batches of 16 -> 26 pumps.
        assert_eq!(iterations, 26);
    }

    #[test]
    fn maximum_loaded_square_has_a_bounded_unload_batch() {
        use ferrumc_math::ChunkPos;

        use super::{
            desired_chunks, next_chunk_transition, MAX_CHUNK_LOADS_PER_UPDATE,
            MAX_CHUNK_UNLOADS_PER_UPDATE, STREAM_VIEW_DISTANCE_MAX,
        };

        let mut loaded = desired_chunks(ChunkPos::ORIGIN, STREAM_VIEW_DISTANCE_MAX);
        let superseding_center = ChunkPos::new(10_000, -10_000);
        let desired = desired_chunks(superseding_center, STREAM_VIEW_DISTANCE_MAX);

        assert_eq!(
            loaded.len(),
            65 * 65,
            "the fixture exercises the maximum streamed square"
        );
        let first = next_chunk_transition(
            superseding_center,
            &desired,
            &loaded,
            MAX_CHUNK_LOADS_PER_UPDATE,
            MAX_CHUNK_UNLOADS_PER_UPDATE,
            ChunkPos::ORIGIN,
        );
        assert_eq!(first.load.len(), MAX_CHUNK_LOADS_PER_UPDATE);
        assert_eq!(first.unload.len(), MAX_CHUNK_UNLOADS_PER_UPDATE);
        assert_eq!(
            first,
            next_chunk_transition(
                superseding_center,
                &desired,
                &loaded,
                MAX_CHUNK_LOADS_PER_UPDATE,
                MAX_CHUNK_UNLOADS_PER_UPDATE,
                ChunkPos::ORIGIN,
            ),
            "the same state always plans the same bounded transition"
        );

        let mut pumps = 0usize;
        while loaded != desired {
            let transition = next_chunk_transition(
                superseding_center,
                &desired,
                &loaded,
                MAX_CHUNK_LOADS_PER_UPDATE,
                MAX_CHUNK_UNLOADS_PER_UPDATE,
                ChunkPos::ORIGIN,
            );
            assert!(transition.load.len() <= MAX_CHUNK_LOADS_PER_UPDATE);
            assert!(transition.unload.len() <= MAX_CHUNK_UNLOADS_PER_UPDATE);
            assert!(
                transition.load.len() + transition.unload.len()
                    <= MAX_CHUNK_LOADS_PER_UPDATE + MAX_CHUNK_UNLOADS_PER_UPDATE
            );
            assert!(
                !transition.load.is_empty() || !transition.unload.is_empty(),
                "a non-converged stream must make progress"
            );
            for pos in transition.unload {
                assert!(loaded.remove(&pos), "only held columns may be released");
            }
            for pos in transition.load {
                assert!(desired.contains(&pos), "only desired columns may load");
                assert!(loaded.insert(pos), "a load cannot duplicate a held column");
            }
            assert!(
                loaded.len() <= 65 * 65,
                "equal load/unload caps prevent ticket-set growth"
            );
            pumps += 1;
            assert!(pumps <= 265, "the bounded transition must converge");
        }
        assert_eq!(pumps, 265, "4,225 disjoint columns drain in ceil(n/16)");
    }

    #[test]
    fn superseding_center_discards_the_stale_transition() {
        use ferrumc_math::ChunkPos;

        use super::{
            desired_chunks, next_chunk_transition, MAX_CHUNK_LOADS_PER_UPDATE,
            MAX_CHUNK_UNLOADS_PER_UPDATE, STREAM_VIEW_DISTANCE_MAX,
        };

        let loaded = desired_chunks(ChunkPos::ORIGIN, STREAM_VIEW_DISTANCE_MAX);
        let stale_center = ChunkPos::new(10_000, 10_000);
        let stale_desired = desired_chunks(stale_center, STREAM_VIEW_DISTANCE_MAX);
        let stale = next_chunk_transition(
            stale_center,
            &stale_desired,
            &loaded,
            MAX_CHUNK_LOADS_PER_UPDATE,
            MAX_CHUNK_UNLOADS_PER_UPDATE,
            ChunkPos::ORIGIN,
        );
        assert!(!stale.load.is_empty() && !stale.unload.is_empty());

        // Before any stale plan is applied, a latest-wins observation returns to
        // the already-loaded origin. Recomputing has no old backlog to drain.
        let latest_desired = desired_chunks(ChunkPos::ORIGIN, STREAM_VIEW_DISTANCE_MAX);
        let latest = next_chunk_transition(
            ChunkPos::ORIGIN,
            &latest_desired,
            &loaded,
            MAX_CHUNK_LOADS_PER_UPDATE,
            MAX_CHUNK_UNLOADS_PER_UPDATE,
            ChunkPos::ORIGIN,
        );
        assert_eq!(latest.load, []);
        assert_eq!(latest.unload, []);
    }

    /// Builds a bare [`ChunkStream`] for the block-crossing test without a
    /// [`ConnContext`](super::super::context::ConnContext): only `last_move_block`
    /// matters here, so the rest is seeded empty.
    fn bare_stream() -> super::ChunkStream {
        use std::collections::BTreeSet;

        use ferrumc_math::ChunkPos;

        super::ChunkStream {
            ticket_handoff_pending: false,
            center: ChunkPos::ORIGIN,
            viewer: ferrumc_core::PlayerId::offline("chunk_fixture"),
            view_distance: 0,
            loaded: BTreeSet::new(),
            sent: BTreeSet::new(),
            movement_hint: ChunkPos::new(0, 0),
            pending_position: None,
            last_position: None,
            last_move_block: None,
        }
    }

    #[test]
    fn client_distance_is_capped_and_prefetch_is_bounded_visible_first_and_directional() {
        use ferrumc_math::ChunkPos;
        assert_eq!(super::effective_view_distance(6, Some(3)), 3);
        assert_eq!(super::effective_view_distance(6, Some(12)), 6);
        assert_eq!(super::effective_view_distance(6, Some(-1)), 6);
        assert_eq!(super::effective_view_distance(100, None), 32);
        assert_eq!(super::effective_view_distance(6, Some(0)), 0);
        let targets = super::prefetch_targets(
            ChunkPos::ORIGIN,
            3,
            ChunkPos::new(1, 0),
            &std::collections::BTreeSet::new(),
        );
        assert_eq!(targets.len(), 64);
        assert!(targets[..49]
            .iter()
            .all(|pos| super::chebyshev_distance(ChunkPos::ORIGIN, *pos) <= 3));
        assert!(targets[49..]
            .iter()
            .all(|pos| super::chebyshev_distance(ChunkPos::ORIGIN, *pos) > 3));
        assert!(
            targets[49..].iter().filter(|pos| pos.x() > 0).count()
                > targets[49..].iter().filter(|pos| pos.x() < 0).count()
        );
        let complete_view = super::desired_chunks(ChunkPos::ORIGIN, 6);
        let outer =
            super::prefetch_targets(ChunkPos::ORIGIN, 6, ChunkPos::new(1, 0), &complete_view);
        assert_eq!(
            outer.len(),
            64,
            "full views larger than target limit still prepare outer columns"
        );
        assert!(outer
            .iter()
            .all(|pos| super::chebyshev_distance(ChunkPos::ORIGIN, *pos) > 6));
        let retained = super::desired_chunks(ChunkPos::ORIGIN, 4);
        assert_eq!(retained.len(), 81);
        assert!(super::desired_chunks(ChunkPos::new(1, 0), 3).is_subset(&retained));
    }

    #[test]
    fn block_crossing_fires_only_on_a_new_block() {
        use ferrumc_math::{BlockPos, Vec3};

        let mut stream = bare_stream();

        // The first reported position only seeds the baseline; there is no prior
        // block to report a crossing from.
        assert_eq!(stream.block_crossing(Vec3::new(0.5, 64.0, 0.5)), None);

        // Sub-block jitter within the same block does not refire.
        assert_eq!(stream.block_crossing(Vec3::new(0.9, 64.2, 0.1)), None);

        // Stepping into the next block fires exactly one crossing.
        assert_eq!(
            stream.block_crossing(Vec3::new(1.1, 64.0, 0.5)),
            Some((BlockPos::new(0, 64, 0), BlockPos::new(1, 64, 0)))
        );
        // Staying put after the crossing does not refire.
        assert_eq!(stream.block_crossing(Vec3::new(1.4, 64.0, 0.6)), None);

        // Negative coordinates floor toward -inf (vanilla block space).
        assert_eq!(
            stream.block_crossing(Vec3::new(-0.1, 64.0, 0.5)),
            Some((BlockPos::new(1, 64, 0), BlockPos::new(-1, 64, 0)))
        );
    }

    #[test]
    fn pending_stream_position_is_latest_wins() {
        use ferrumc_math::Vec3;

        let mut stream = bare_stream();
        let first = Vec3::new(128.5, 64.0, 8.5);
        let second = Vec3::new(-128.5, 64.0, 8.5);
        let latest = Vec3::new(8.5, 64.0, 256.5);

        stream.observe(first);
        stream.observe(second);
        stream.observe(latest);

        assert_eq!(stream.pending_position, Some(latest));
        assert_eq!(stream.last_position(), Some(latest));
    }

    #[test]
    fn horizontal_prefetch_direction_survives_idle_and_vertical_packets() {
        use ferrumc_math::{ChunkPos, Vec3};
        let mut stream = bare_stream();
        stream.observe(Vec3::new(0.5, 64.0, 0.5));
        stream.observe(Vec3::new(1.0, 64.0, 0.5));
        stream.observe(Vec3::new(1.0, 65.0, 0.5));
        stream.observe(Vec3::new(1.0, 65.0, 0.5));
        assert_eq!(stream.movement_hint, ChunkPos::new(1, 0));
        stream.observe(Vec3::new(0.5, 65.0, 0.5));
        assert_eq!(stream.movement_hint, ChunkPos::new(-1, 0));
        stream.mirror_teleport(Vec3::new(128.0, 64.0, 0.0));
        assert_eq!(stream.movement_hint, ChunkPos::ORIGIN);
    }

    #[test]
    fn visible_candidate_scan_keeps_center_and_two_forward_columns_before_side_rings() {
        use ferrumc_math::ChunkPos;
        let desired = super::desired_chunks(ChunkPos::ORIGIN, 6);
        let transition = super::next_chunk_transition(
            ChunkPos::ORIGIN,
            &desired,
            &std::collections::BTreeSet::new(),
            64,
            16,
            ChunkPos::new(1, 0),
        );
        assert_eq!(transition.load.len(), 64);
        assert_eq!(transition.load[0], ChunkPos::ORIGIN);
        assert_eq!(transition.load[1], ChunkPos::new(1, 0));
        assert!(transition.load.iter().all(|pos| desired.contains(pos)));
        assert_eq!(transition.load[2], ChunkPos::new(2, 0));
        let rest: Vec<_> = transition
            .load
            .iter()
            .filter(|pos| {
                super::movement_corridor_rank(ChunkPos::ORIGIN, ChunkPos::new(1, 0), **pos) == 3
            })
            .copied()
            .collect();
        assert!(rest
            .windows(2)
            .all(|pair| super::chebyshev_distance(ChunkPos::ORIGIN, pair[0])
                <= super::chebyshev_distance(ChunkPos::ORIGIN, pair[1])));
    }

    #[test]
    fn stationary_unknown_heading_prepares_all_next_two_axis_columns_before_corners() {
        use ferrumc_math::ChunkPos;
        for center in [ChunkPos::ORIGIN, ChunkPos::new(-5, -7)] {
            let desired = super::desired_chunks(center, 6);
            let held = super::desired_chunks(center, 1);
            let transition =
                super::next_chunk_transition(center, &desired, &held, 64, 16, ChunkPos::ORIGIN);
            assert_eq!(
                &transition.load[..4],
                &[
                    ChunkPos::new(center.x() - 2, center.z()),
                    ChunkPos::new(center.x(), center.z() - 2),
                    ChunkPos::new(center.x(), center.z() + 2),
                    ChunkPos::new(center.x() + 2, center.z()),
                ]
            );
            assert!(transition.load.iter().all(|pos| desired.contains(pos)));
            assert_eq!(transition.load.len(), 64);
            assert!(transition.load[..4]
                .iter()
                .all(|pos| super::manhattan_distance(center, *pos) == 2));
            assert_eq!(
                super::manhattan_distance(
                    ChunkPos::new(i32::MIN, i32::MIN),
                    ChunkPos::new(i32::MAX, i32::MAX)
                ),
                8_589_934_590
            );
        }
    }

    #[test]
    fn stationary_inner_and_outer_rings_keep_original_contiguous_order() {
        use ferrumc_math::ChunkPos;
        use std::collections::BTreeSet;
        for center in [ChunkPos::ORIGIN, ChunkPos::new(-5, -7)] {
            let desired = super::desired_chunks(center, 6);
            let transition = super::next_chunk_transition(
                center,
                &desired,
                &BTreeSet::new(),
                desired.len(),
                16,
                ChunkPos::ORIGIN,
            );
            let actual: Vec<_> = transition
                .load
                .into_iter()
                .filter(|pos| super::chebyshev_distance(center, *pos) != 2)
                .collect();
            let mut original: Vec<_> = desired
                .into_iter()
                .filter(|pos| super::chebyshev_distance(center, *pos) != 2)
                .collect();
            original.sort_by_key(|pos| (super::chebyshev_distance(center, *pos), pos.x(), pos.z()));
            assert_eq!(
                actual, original,
                "all stationary rings except ring2 keep the exact prior locality order"
            );
        }
    }

    #[test]
    fn corridor_reversal_diagonal_and_small_view_remain_visible_and_bounded() {
        use ferrumc_math::ChunkPos;
        use std::collections::BTreeSet;
        let center = ChunkPos::new(-5, -7);
        for direction in [
            ChunkPos::new(1, 0),
            ChunkPos::new(-1, 0),
            ChunkPos::new(1, -1),
        ] {
            let desired = super::desired_chunks(center, 6);
            let transition =
                super::next_chunk_transition(center, &desired, &BTreeSet::new(), 64, 16, direction);
            assert_eq!(
                &transition.load[..3],
                &[
                    center,
                    ChunkPos::new(center.x() + direction.x(), center.z() + direction.z()),
                    ChunkPos::new(
                        center.x() + 2 * direction.x(),
                        center.z() + 2 * direction.z()
                    )
                ]
            );
            assert_eq!(transition.load.len(), 64);
            assert!(transition.load.iter().all(|pos| desired.contains(pos)));
            assert_eq!(
                transition
                    .load
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    .len(),
                64
            );
        }
        let desired = super::desired_chunks(center, 1);
        let transition = super::next_chunk_transition(
            center,
            &desired,
            &BTreeSet::new(),
            64,
            16,
            ChunkPos::new(1, 0),
        );
        assert_eq!(transition.load.len(), 9);
        assert!(
            transition.load.iter().all(|pos| desired.contains(pos)),
            "corridor prediction never expands client-visible admission"
        );
    }

    #[test]
    fn authoritative_teleport_recenters_only_when_fully_safe() {
        use ferrumc_math::Vec3;
        use ferrumc_proto::generated::play::SynchronizePlayerPosition;

        use super::mirror_server_teleport;

        let mut stream = bare_stream();
        let mut yaw = 5.0;
        let mut pitch = -5.0;
        let target = Vec3::new(128.5, 70.0, -64.5);
        let safe = SynchronizePlayerPosition::new(
            7, target.x, target.y, target.z, 0.0, 0.0, 0.0, 90.0, -30.0, 0,
        );
        mirror_server_teleport(&mut stream, &mut yaw, &mut pitch, &safe);
        assert_eq!(stream.pending_position, Some(target));
        assert_eq!(stream.last_position(), Some(target));
        assert_eq!((yaw, pitch), (90.0, -30.0));
        assert_eq!(
            stream.block_crossing(Vec3::new(128.9, 70.1, -64.1)),
            None,
            "a client echo in the destination block is not a giant move event",
        );
        assert_eq!(
            stream.block_crossing(Vec3::new(129.1, 70.1, -64.1)),
            Some((
                ferrumc_math::BlockPos::new(128, 70, -65),
                ferrumc_math::BlockPos::new(129, 70, -65),
            )),
            "the next real block crossing starts at the teleport destination",
        );

        let invalid = [
            SynchronizePlayerPosition::new(8, f64::NAN, 64.0, 8.0, 0.0, 0.0, 0.0, 10.0, 20.0, 0),
            SynchronizePlayerPosition::new(
                9,
                8.0,
                64.0,
                8.0,
                0.0,
                0.0,
                0.0,
                f32::INFINITY,
                20.0,
                0,
            ),
            SynchronizePlayerPosition::new(10, 8.0, 64.0, 8.0, 0.0, 0.0, 0.0, 10.0, 20.0, 1),
        ];
        for sync in invalid {
            mirror_server_teleport(&mut stream, &mut yaw, &mut pitch, &sync);
            assert_eq!(stream.pending_position, Some(target));
            assert_eq!(stream.last_position(), Some(target));
            assert_eq!((yaw, pitch), (90.0, -30.0));
        }
    }
}
