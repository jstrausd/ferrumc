//! Native collector-session types (NET-04.2e1): sealed session keys, positive
//! limits, the native endpoint lane, join reservations and closure-scoped
//! endpoint revalidation.
//!
//! The router never holds a `SimShard`. It stores the genuine admission handle
//! the driver passed to [`admit_native_join`](crate::SessionRouter::admit_native_join)
//! and builds every native gameplay input from it.

mod config;
mod error;
mod lane;
mod revalidation;

use std::sync::Weak;

use ferrumc_core::{GameMode, PlayerId};
use ferrumc_math::Vec3;
use ferrumc_sim::ShardId;

pub use config::{
    CollectorSessionLimits, PlayerSessionKey, MAX_BASELINES_PER_VIEWER, MAX_COLLECTOR_AUDIENCE,
    MAX_COLLECTOR_SCRATCH, MAX_COLLECTOR_SESSIONS, MAX_COLLECTOR_WORK, MAX_NATIVE_LANE_CAPACITY,
    MAX_SIMULATION_DISTANCE, MAX_TOTAL_BASELINES,
};
pub(crate) use config::{
    NativeRegistryLayout, RouterIdentity, OVERWORLD, OVERWORLD_DIMENSION_TYPE,
};
pub use error::CollectorSessionError;
pub(crate) use lane::{lane_channel, LaneItem, MergeOutcome, MovementCell, NativeLaneIdentity};
pub use lane::{
    NativeInputClass, NativeLaneRecv, NativeShardInput, NativeShardReceiver,
    NativeShardRegistration,
};
pub(crate) use revalidation::{LivePose, VerdictOutcome};
pub use revalidation::{NativeRevalidation, NativeRevalidationProbe, NativeRevalidationVerdict};

/// Read-only spawn pose minted from the genuine live view at admission.
///
/// The app uses exactly these bits for the first `SynchronizePlayerPosition`;
/// no axis or angle is normalized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeSpawnProjection {
    pub(crate) position: Vec3,
    pub(crate) look: [f32; 2],
    pub(crate) game_mode: GameMode,
}

impl NativeSpawnProjection {
    /// The admitted position (finite, every axis at most `3.0e7` in magnitude).
    pub const fn position(self) -> Vec3 {
        self.position
    }

    /// The admitted yaw and pitch, finite and unnormalized.
    pub const fn look(self) -> [f32; 2] {
        self.look
    }

    /// The admitted native game mode.
    pub const fn game_mode(self) -> GameMode {
        self.game_mode
    }
}

/// A provisional native join slot: a session incarnation, a burned wire id and
/// one counted session-table entry. It is not a simulation admission.
///
/// Sealed and not `Clone`:
///
/// ```compile_fail
/// fn copy(r: &ferrumc_session::NativeJoinReservation) -> ferrumc_session::NativeJoinReservation {
///     r.clone()
/// }
/// ```
#[derive(Debug)]
pub struct NativeJoinReservation {
    pub(crate) router: Weak<RouterIdentity>,
    pub(crate) key: PlayerSessionKey,
}

impl NativeJoinReservation {
    /// The reserved player.
    pub const fn player(&self) -> PlayerId {
        self.key.player
    }
}

/// Successful disposition of one routed native client event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRouted {
    /// A new lane entry was queued.
    Queued,
    /// The movement replaced the session's pending, untaken movement entry.
    Coalesced,
    /// A teleport confirmation closed the movement cell; nothing was queued.
    Barrier,
    /// The event has no simulation effect (for example a keep-alive).
    Ignored,
}

/// Disposition of [`disconnect_native`](crate::SessionRouter::disconnect_native).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDisconnect {
    /// The disconnect entered the native lane.
    Sent,
    /// The disconnect is retained (lane full/closed or endpoint suspended) and
    /// retried by [`retry_native_disconnects`](crate::SessionRouter::retry_native_disconnects)
    /// or sent first after revalidation.
    Retained,
}

/// Lifecycle phase of a native session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSessionPhase {
    /// Admitted and accepting gameplay input.
    Active,
    /// Leaving; no new gameplay input, a disconnect is sent or retained.
    Leaving,
}

/// Endpoint binding of a native session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEndpointState {
    /// Bound to the current receiver of its registration.
    Bound,
    /// `EndpointSuspended`: the registration was replaced; awaiting revalidation.
    Suspended,
    /// Revalidation reported the simulation player detached; no lane is needed.
    Detached,
    /// `NativeEndpointRevalidationRefused`: the submitted view did not match.
    RevalidationRefused,
}

/// Read-only diagnostics of one native session; grants no authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSessionStatus {
    pub(crate) phase: NativeSessionPhase,
    pub(crate) endpoint: NativeEndpointState,
    pub(crate) ever_active: bool,
    pub(crate) disconnect: Option<NativeDisconnect>,
    pub(crate) remote_wire_id: i32,
    pub(crate) self_wire_id: i32,
    pub(crate) game_mode: GameMode,
}

impl NativeSessionStatus {
    /// The lifecycle phase.
    pub const fn phase(self) -> NativeSessionPhase {
        self.phase
    }

    /// The endpoint binding.
    pub const fn endpoint(self) -> NativeEndpointState {
        self.endpoint
    }

    /// `true` once the session was admitted active (never reset).
    pub const fn ever_active(self) -> bool {
        self.ever_active
    }

    /// Disconnect progress, or `None` before a leave was requested.
    pub const fn disconnect(self) -> Option<NativeDisconnect> {
        self.disconnect
    }

    /// Wire id other viewers will see this session as (from the shared allocator).
    pub const fn remote_wire_id(self) -> i32 {
        self.remote_wire_id
    }

    /// Wire id the session's own `JoinGame` assigned to itself.
    pub const fn self_wire_id(self) -> i32 {
        self.self_wire_id
    }

    /// The last genuine native mode the session observed (admission or
    /// revalidation view); not a replacement for the simulation's state.
    pub const fn game_mode(self) -> GameMode {
        self.game_mode
    }
}

/// A refused native unregistration; the receiver is returned intact.
#[derive(Debug)]
pub struct NativeUnregisterError {
    pub(crate) error: CollectorSessionError,
    pub(crate) receiver: NativeShardReceiver,
}

impl NativeUnregisterError {
    /// The classified refusal.
    pub const fn error(&self) -> &CollectorSessionError {
        &self.error
    }

    /// Returns the refusal and the untouched receiver.
    pub fn into_parts(self) -> (CollectorSessionError, NativeShardReceiver) {
        (self.error, self.receiver)
    }
}

impl std::fmt::Display for NativeUnregisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for NativeUnregisterError {}

/// Region check with the simulation's own derivation (A10):
/// `EntityTransform::try_new(position, 0, 0)?.chunk()` inside the home region.
pub(crate) fn region_contains(
    home: ShardId,
    position: Vec3,
) -> Result<bool, CollectorSessionError> {
    let transform = ferrumc_sim::EntityTransform::try_new(position, 0.0, 0.0)
        .map_err(|_| CollectorSessionError::InvalidTransform)?;
    Ok(home
        .region()
        .contains_chunk(home.world(), home.dimension(), transform.chunk()))
}
