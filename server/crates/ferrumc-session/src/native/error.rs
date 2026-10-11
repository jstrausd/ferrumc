//! Classified refusals of the native collector-session binding.

use ferrumc_core::{DimensionId, PlayerId, ServerError, WorldId};
use ferrumc_sim::ShardId;

use crate::directory::{ShardDirectoryError, ShardRegistrationId};

/// A classifying error returned by the native collector-session APIs of
/// [`SessionRouter`](crate::SessionRouter).
///
/// Every refusal is decided before the operation's first side effect unless the
/// variant documentation says otherwise (a region correction or a controlled
/// session leave is part of the documented disposition, never a partial
/// mutation).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CollectorSessionError {
    /// Native collector sessions were not initialized on this router.
    #[error("native collector sessions are not initialized")]
    NotInitialized,
    /// Native collector sessions were already initialized; there is no reset.
    #[error("native collector sessions are already initialized")]
    AlreadyInitialized,
    /// Initialization was attempted after sessions or leaves already existed.
    #[error("native collector sessions must be initialized before any session")]
    NotEmpty,
    /// A positive limit exceeded its hard maximum or a cross-limit rule.
    #[error("invalid collector session limit: {0}")]
    InvalidLimit(&'static str),
    /// The supplied registry or known-pack data is not the one supported layout.
    #[error("unsupported native registry layout: {0}")]
    UnsupportedRegistryLayout(&'static str),
    /// The configured simulation distance lies outside `0..=32`.
    #[error("invalid simulation distance {distance}")]
    InvalidSimulationDistance {
        /// The rejected configured value.
        distance: i32,
    },
    /// A generation, scope, or position is outside the supported native layout.
    #[error("unsupported native scope: {0}")]
    Unsupported(&'static str),
    /// Legacy and native coverage cannot coexist in one world/dimension.
    #[error("endpoint kind conflict in world {world}, dimension {dimension}")]
    EndpointKindConflict {
        /// The contested world.
        world: WorldId,
        /// The contested dimension.
        dimension: DimensionId,
    },
    /// A native operation resolved to a legacy endpoint; nothing is translated.
    #[error("native operation resolved to legacy endpoint {home}")]
    LegacyEndpoint {
        /// Home of the legacy endpoint.
        home: ShardId,
    },
    /// No endpoint covers the requested target.
    #[error("no native endpoint covers {target}")]
    UnknownShard {
        /// The logical target that has no coverage.
        target: ShardId,
    },
    /// A directory lease mutation or validation failed.
    #[error("shard directory rejected the native operation: {0}")]
    Directory(#[from] ShardDirectoryError),
    /// The borrowed receiver is not the current receiver of its registration.
    #[error("native receiver is foreign or no longer current")]
    StaleReceiver,
    /// A session, reservation, or record still references the registration.
    #[error(
        "native registration {registration_id} is in use by {sessions} sessions and {reservations} reservations"
    )]
    NativeEndpointInUse {
        /// The registration lineage that is still referenced.
        registration_id: ShardRegistrationId,
        /// Native sessions (active or leaving) referencing it.
        sessions: usize,
        /// Provisional join reservations referencing it.
        reservations: usize,
    },
    /// Installed sessions plus provisional reservations reached the session limit.
    #[error("collector session capacity {limit} reached")]
    SessionCapacity {
        /// The configured session limit.
        limit: usize,
    },
    /// The router's monotonic session incarnation counter is exhausted.
    #[error("session incarnations exhausted")]
    SessionIdsExhausted,
    /// The router's shared entity wire allocator is exhausted; it never wraps.
    #[error("entity wire ids exhausted")]
    EntityIdsExhausted,
    /// The UUID already has a legacy session, a native session, or a reservation.
    #[error("player {player} already has a session or reservation")]
    DuplicatePlayer {
        /// The duplicate player.
        player: PlayerId,
    },
    /// The reservation was minted by another router.
    #[error("join reservation belongs to another router")]
    ForeignReservation,
    /// The reservation was already admitted or cancelled.
    #[error("join reservation is no longer pending")]
    StaleReservation,
    /// Handle, view, scope, or declared position do not describe one admission.
    #[error("native admission mismatch: {0}")]
    AdmissionMismatch(&'static str),
    /// The typed `JoinGame` differs from the validated native layout.
    #[error("join game mismatch: {0}")]
    JoinGameMismatch(&'static str),
    /// The live pose cannot be projected without normalization.
    #[error("invalid native spawn projection: {0}")]
    InvalidSpawnProjection(&'static str),
    /// The session's own outbound lane refused the first mandatory packet.
    #[error("native session outbound lane refused the mandatory join packet")]
    OutboundRefused,
    /// The session key is foreign, stale, or names no current session.
    #[error("stale or foreign native session key")]
    StaleSession,
    /// The session is leaving; no new gameplay input is accepted.
    #[error("native session is leaving")]
    SessionNotActive,
    /// The session's endpoint was replaced and is not yet revalidated.
    #[error("native session endpoint is suspended")]
    EndpointSuspended,
    /// A movement crossed the owner region; a correction was queued instead.
    #[error("native movement crosses the region boundary")]
    NativeRegionBoundary,
    /// A position or look failed the simulation's finite admission bounds.
    #[error("native movement is not a valid simulation transform")]
    InvalidTransform,
    /// A declared admission position lies outside the owner region.
    #[error("native admission lies outside the owner region")]
    NativeAdmissionOutsideRegion,
    /// A hotbar selection outside `0..=8`.
    #[error("invalid hotbar slot {slot}")]
    InvalidHotbarSlot {
        /// The rejected wire value.
        slot: i16,
    },
    /// Block, command, container and item actions have no native consumer yet.
    #[error("native action has no consumer: {0}")]
    UnsupportedNativeAction(&'static str),
    /// The native lane had no data capacity; the session was set leaving.
    #[error("native lane of {home} is full")]
    NativeLaneFull {
        /// Home of the full lane.
        home: ShardId,
    },
    /// The native lane receiver is gone; the session was set leaving.
    #[error("native lane of {home} is closed")]
    NativeLaneClosed {
        /// Home of the closed lane.
        home: ShardId,
    },
    /// A mandatory correction could not be queued; the session was set leaving.
    #[error("native correction could not be delivered")]
    CorrectionUndeliverable,
    /// A bounded worklist could not be allocated.
    #[error("bounded native session allocation failed")]
    AllocationFailed,
}

impl From<CollectorSessionError> for ServerError {
    fn from(error: CollectorSessionError) -> Self {
        match error {
            CollectorSessionError::SessionCapacity { .. }
            | CollectorSessionError::SessionIdsExhausted
            | CollectorSessionError::EntityIdsExhausted
            | CollectorSessionError::NativeLaneFull { .. }
            | CollectorSessionError::AllocationFailed => {
                ServerError::capacity(format!("native session: {error}"))
            }
            CollectorSessionError::UnknownShard { .. } | CollectorSessionError::StaleSession => {
                ServerError::not_found(format!("native session: {error}"))
            }
            _ => ServerError::invalid_state(format!("native session: {error}")),
        }
    }
}
