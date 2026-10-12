//! The bounded native session lane between the router and one native owner.
//!
//! # Backpressure
//!
//! Each native registration owns exactly one `tokio::sync::mpsc` channel of the
//! router's shard-input capacity (nonzero, at most
//! [`MAX_NATIVE_LANE_CAPACITY`](super::MAX_NATIVE_LANE_CAPACITY)). The router is
//! the only sender. Data items stop before the router's control reserve;
//! control items (disconnects) may use it. A full lane never blocks: data for an
//! active session turns into a controlled session leave, a disconnect is
//! retained (deduplicated) in the session and retried. The receiver belongs to
//! the single driver task that owns the simulation shard.
//!
//! # Movement coalescing (Root R3-A2)
//!
//! An mpsc entry cannot be replaced once it was sent. A movement item therefore
//! carries a per-session [`MovementCell`]: a `std::sync::Mutex` leaf shared by
//! the router (merging newer movement) and the receiver (taking it). While the
//! newest pending lane item of a session is this cell and the receiver has not
//! taken it, newer movement replaces its content without a new lane slot. Every
//! other item of the session, a teleport confirmation, a resync, and a region
//! correction close the cell; the next movement then gets a new item, so no
//! movement is ever reordered across a barrier. The lock is held only for O(1)
//! work without allocation, never across `.await`, and never together with
//! another lock. Router and receiver live in the same driver task, so the lock
//! is uncontended in production; it exists because the queued entry is shared.
//!
//! # Take sequence (§7.3 amendment V1, Root)
//!
//! Every lane generation counts the items the router put into it and the items
//! the receiver took out of it. The channel is FIFO and the router is its only
//! sender, so the `n`-th sent item has left the lane exactly when the take
//! count reaches `n`. A disconnect remembers its position as a [`LaneMark`];
//! the leave-save state machine counts a disconnect resend only after the
//! previous disconnect was taken (handed to the simulation, §6.2), never while
//! a backlog of other sessions' items still holds it in the lane.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{self, error::TryRecvError};

use ferrumc_math::Vec3;
use ferrumc_sim::{OwnedPlayerHandle, OwnedPlayerInput, ShardId};

use super::PlayerSessionKey;
use crate::directory::{ShardCoverage, ShardLease};

/// Identity of one native lane generation; compared by `Arc::ptr_eq`.
///
/// It also carries the lane's send and take sequence (§7.3 amendment V1).
/// Both counters only grow; a `u64` cannot wrap at any real item rate.
#[derive(Debug)]
pub(crate) struct NativeLaneIdentity {
    pub(crate) coverage: ShardCoverage,
    /// Items the router (the only sender) put into this lane.
    sent: AtomicU64,
    /// Items the receiver took out of this lane.
    taken: AtomicU64,
}

impl NativeLaneIdentity {
    pub(crate) const fn new(coverage: ShardCoverage) -> Self {
        Self {
            coverage,
            sent: AtomicU64::new(0),
            taken: AtomicU64::new(0),
        }
    }

    /// Records one successful send and returns the item's lane position.
    pub(crate) fn record_send(&self) -> u64 {
        self.sent.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }

    fn record_take(&self) {
        self.taken.fetch_add(1, Ordering::Relaxed);
    }
}

/// Lane position of one sent disconnect (§7.3 amendment V1, Root).
#[derive(Debug, Clone)]
pub(crate) struct LaneMark {
    lane: Arc<NativeLaneIdentity>,
    position: u64,
}

impl LaneMark {
    pub(crate) fn new(lane: &Arc<NativeLaneIdentity>, position: u64) -> Self {
        Self {
            lane: Arc::clone(lane),
            position,
        }
    }

    /// Whether the receiver took the marked item out of its lane.
    pub(crate) fn taken(&self) -> bool {
        self.lane.taken.load(Ordering::Relaxed) >= self.position
    }
}

/// The lane class of a native input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeInputClass {
    /// Gameplay data (movement, mode, selection) of an active session.
    Data,
    /// Lifecycle control (disconnect) that may use the reserved lane tail.
    Control,
}

/// One routed native input: the session key, its class, and the simulation
/// input the router built from the session's stored admission handle.
///
/// Only the router constructs it; there is no public constructor:
///
/// ```compile_fail
/// use ferrumc_session::{NativeInputClass, NativeShardInput, PlayerSessionKey};
/// use ferrumc_sim::OwnedPlayerInput;
/// fn forge(key: PlayerSessionKey) -> NativeShardInput {
///     NativeShardInput { key, class: NativeInputClass::Data, input: OwnedPlayerInput::drain() }
/// }
/// ```
#[derive(Debug)]
pub struct NativeShardInput {
    pub(crate) key: PlayerSessionKey,
    pub(crate) class: NativeInputClass,
    pub(crate) input: OwnedPlayerInput,
}

impl NativeShardInput {
    /// The session this input was routed for.
    pub const fn session(&self) -> &PlayerSessionKey {
        &self.key
    }

    /// The lane class of this input.
    pub const fn class(&self) -> NativeInputClass {
        self.class
    }

    /// Splits the input so the driver keeps key and class while the simulation
    /// may refuse (and return) only the [`OwnedPlayerInput`].
    pub fn into_parts(self) -> (PlayerSessionKey, NativeInputClass, OwnedPlayerInput) {
        (self.key, self.class, self.input)
    }
}

/// Why [`NativeShardReceiver::try_recv`] produced no input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum NativeLaneRecv {
    /// The lane is currently empty.
    #[error("native lane is empty")]
    Empty,
    /// The router dropped this lane (replacement or unregistration) and it is drained.
    #[error("native lane is disconnected")]
    Disconnected,
    /// The movement cell of `session` was poisoned by a panic in another
    /// holder; its coalesced movement is explicitly discarded.
    #[error("native movement cell is poisoned")]
    MovementCellPoisoned {
        /// The session whose coalesced movement was discarded.
        session: PlayerSessionKey,
    },
}

/// The receiving half of one native lane generation.
///
/// It is neither `Clone` nor publicly constructible; the session compares its
/// identity with the current registration before every binding:
///
/// ```compile_fail
/// fn copy(lane: &ferrumc_session::NativeShardReceiver) -> ferrumc_session::NativeShardReceiver {
///     lane.clone()
/// }
/// ```
#[derive(Debug)]
pub struct NativeShardReceiver {
    pub(crate) identity: Arc<NativeLaneIdentity>,
    pub(crate) receiver: mpsc::Receiver<LaneItem>,
}

impl NativeShardReceiver {
    /// Receives the next routed input without waiting.
    ///
    /// A coalesced movement is taken atomically: the router can no longer merge
    /// into it, and the returned input carries the newest merged pose.
    ///
    /// # Errors
    ///
    /// [`NativeLaneRecv::Empty`] / [`NativeLaneRecv::Disconnected`] mirror the
    /// bounded channel; [`NativeLaneRecv::MovementCellPoisoned`] reports a
    /// discarded movement whose cell lock was poisoned.
    pub fn try_recv(&mut self) -> Result<NativeShardInput, NativeLaneRecv> {
        let item = self.receiver.try_recv();
        if item.is_ok() {
            // Every entry that leaves the lane counts, a poisoned movement too.
            self.identity.record_take();
        }
        match item {
            Ok(LaneItem::Input(input)) => Ok(input),
            Ok(LaneItem::Movement { key, cell }) => match cell.take() {
                Ok(input) => Ok(NativeShardInput {
                    key,
                    class: NativeInputClass::Data,
                    input,
                }),
                Err(CellPoisoned) => Err(NativeLaneRecv::MovementCellPoisoned { session: key }),
            },
            Err(TryRecvError::Empty) => Err(NativeLaneRecv::Empty),
            Err(TryRecvError::Disconnected) => Err(NativeLaneRecv::Disconnected),
        }
    }
}

/// A freshly registered (or replaced) native endpoint: home, lease and the
/// lane receiver for the driver that owns the native simulation shard.
#[derive(Debug)]
pub struct NativeShardRegistration {
    pub(crate) home: ShardId,
    pub(crate) lease: ShardLease,
    pub(crate) receiver: NativeShardReceiver,
}

impl NativeShardRegistration {
    /// The canonical home shard of the endpoint.
    pub const fn shard_id(&self) -> ShardId {
        self.home
    }

    /// The lease authorizing replacement or unregistration.
    pub fn lease(&self) -> ShardLease {
        self.lease.clone()
    }

    /// Consumes the registration into home, lease and lane receiver.
    pub fn into_parts(self) -> (ShardId, ShardLease, NativeShardReceiver) {
        (self.home, self.lease, self.receiver)
    }
}

/// One queued lane entry.
///
/// Inputs stay inline rather than boxed: the channel is bounded, so the slot
/// size bounds memory (capacity × entry size) and routing allocates nothing
/// per item.
#[derive(Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "bounded channel slots; avoids one heap allocation per routed input"
)]
pub(crate) enum LaneItem {
    /// A fully built input.
    Input(NativeShardInput),
    /// A coalescing movement entry, built when the receiver takes it.
    Movement {
        key: PlayerSessionKey,
        cell: Arc<MovementCell>,
    },
}

/// Coalesced movement state of one session; see the module documentation.
#[derive(Debug)]
pub(crate) struct MovementCell(Mutex<MovementSlot>);

#[derive(Debug)]
struct MovementSlot {
    handle: OwnedPlayerHandle,
    position: Option<Vec3>,
    look: [Option<f32>; 2],
    taken: bool,
}

/// Marker for a poisoned movement-cell lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CellPoisoned;

/// Outcome of offering newer movement to an existing cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeOutcome {
    /// The pending entry now carries the newer movement.
    Merged,
    /// The receiver already took the entry; the newer movement needs its own.
    Taken,
    /// The lock is poisoned; the newer movement needs its own entry.
    Poisoned,
}

impl MovementCell {
    pub(crate) fn new(
        handle: OwnedPlayerHandle,
        position: Option<Vec3>,
        look: [Option<f32>; 2],
    ) -> Self {
        Self(Mutex::new(MovementSlot {
            handle,
            position,
            look,
            taken: false,
        }))
    }

    /// Merges newer movement into an untaken entry (latest value per field).
    pub(crate) fn merge(&self, position: Option<Vec3>, look: [Option<f32>; 2]) -> MergeOutcome {
        let Ok(mut slot) = self.0.lock() else {
            return MergeOutcome::Poisoned;
        };
        if slot.taken {
            return MergeOutcome::Taken;
        }
        if position.is_some() {
            slot.position = position;
        }
        for (current, newer) in slot.look.iter_mut().zip(look) {
            if newer.is_some() {
                *current = newer;
            }
        }
        MergeOutcome::Merged
    }

    /// Takes the entry exactly once and builds the movement input.
    fn take(&self) -> Result<OwnedPlayerInput, CellPoisoned> {
        let (handle, position, look) = {
            let mut slot = self.0.lock().map_err(|_| CellPoisoned)?;
            slot.taken = true;
            (slot.handle.clone(), slot.position, slot.look)
        };
        Ok(OwnedPlayerInput::move_player(handle, position, look))
    }

    /// Poisons the lock by panicking while it is held (test support only).
    #[cfg(test)]
    pub(crate) fn poison_for_test(self: &Arc<Self>) {
        let cell = Arc::clone(self);
        let _ = std::thread::spawn(move || {
            let _guard = cell.0.lock();
            panic!("poison the movement cell");
        })
        .join();
    }
}

pub(crate) fn lane_channel(capacity: usize) -> (mpsc::Sender<LaneItem>, mpsc::Receiver<LaneItem>) {
    mpsc::channel(capacity)
}
