//! Read-only collector delivery dispositions (.2c "readonly Audience-API"),
//! the G3 adoption invariant and the lifecycle reports (NET-04.2e2).
//!
//! An audience lists exactly the active native viewers whose admitted spawn
//! baseline matches the collector's session, plus the collector itself with
//! its self wire id. A retirement proof states that the original collector
//! tracker genuinely ended (D5 or unbound retirement) and its take audience is
//! provably empty. Neither payload has a public constructor, `Clone`, packet
//! bytes, senders or acknowledgement authority; both borrow the router.

use std::marker::PhantomData;

use ferrumc_core::PlayerId;

use super::PlayerSessionKey;

/// The disposition of one confirmed take's collector.
#[derive(Debug)]
pub enum CollectorDeliveryDisposition<'a> {
    /// The collector is a current active native session with live tracking.
    Tracked(CollectorAudience<'a>),
    /// The original collector session was genuinely retired; no take and no
    /// self packet is produced, the rest of the group follows normally.
    Retired(CollectorRetirementProof<'a>),
}

/// The bounded take audience of one tracked collector.
#[derive(Debug)]
pub struct CollectorAudience<'a> {
    pub(crate) viewers: Vec<CollectorViewerBinding>,
    pub(crate) router: PhantomData<&'a ()>,
}

impl CollectorAudience<'_> {
    /// Number of viewers, including the collector itself.
    pub fn len(&self) -> usize {
        self.viewers.len()
    }

    /// Whether the audience is empty (never true for a tracked collector).
    pub fn is_empty(&self) -> bool {
        self.viewers.is_empty()
    }

    /// The viewers in session-table order, the collector itself included.
    pub fn viewers(&self) -> impl ExactSizeIterator<Item = &CollectorViewerBinding> {
        self.viewers.iter()
    }
}

/// One audience entry: the viewer's session and the collector wire id that
/// viewer actually knows (its admitted baseline, or `1` for itself).
#[derive(Debug)]
pub struct CollectorViewerBinding {
    pub(crate) session: PlayerSessionKey,
    pub(crate) wire: i32,
    pub(crate) is_self: bool,
}

impl CollectorViewerBinding {
    /// The viewer's sealed session key (routing data, no liveness proof).
    pub const fn session(&self) -> &PlayerSessionKey {
        &self.session
    }

    /// The collector's wire id as this viewer knows it.
    pub const fn collector_wire_id(&self) -> i32 {
        self.wire
    }

    /// Whether this entry is the collector itself.
    pub const fn is_collector_self(&self) -> bool {
        self.is_self
    }
}

/// Proof that exactly this original collector session was retired after the
/// genuine detach of its admission; its take audience is provably empty.
#[derive(Debug)]
pub struct CollectorRetirementProof<'a> {
    pub(crate) player: PlayerId,
    pub(crate) session: PlayerSessionKey,
    pub(crate) router: PhantomData<&'a ()>,
}

impl CollectorRetirementProof<'_> {
    /// The retired collector player.
    pub const fn player(&self) -> PlayerId {
        self.player
    }

    /// The original session key the retired admission was reserved under.
    pub const fn session(&self) -> &PlayerSessionKey {
        &self.session
    }
}

/// The kind of a fatal G3 adoption invariant break (§8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdoptionInvariantKind {
    /// The reservation was minted by another router.
    ForeignReservation,
    /// The reservation was already admitted, cancelled or adopted.
    StaleReservation,
    /// Reservation and handle name different players.
    PlayerMismatch,
    /// The view does not belong to this handle, generation or owner lineage.
    ViewMismatch,
    /// The admission is already bound to another session or record.
    AdmissionAlreadyBound,
    /// The router's lineage counter is exhausted.
    LineageExhausted,
}

/// A fatal, typed G3 adoption invariant break. The reserved entry (if it was
/// this router's) is `Poisoned`; the driver fail-stops and reports the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("native adoption invariant broken for {player}: {kind:?}")]
pub struct AdoptionInvariant {
    pub(crate) player: PlayerId,
    pub(crate) kind: AdoptionInvariantKind,
}

impl AdoptionInvariant {
    /// The reservation's player.
    pub const fn player(&self) -> PlayerId {
        self.player
    }

    /// The broken invariant.
    pub const fn kind(&self) -> AdoptionInvariantKind {
        self.kind
    }
}

/// Outcome of [`native_detached`](crate::SessionRouter::native_detached).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDetachOutcome {
    /// The genuine detach fact of the stored original admission was confirmed.
    Confirmed,
    /// A leaving record exists, but the view carries no matching fact yet.
    NotDetached,
    /// No leaving native entry of this player exists (trigger only).
    NoRecord,
}

/// Report of [`retire_detached_native_sessions`](crate::SessionRouter::retire_detached_native_sessions).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeRetirementReport {
    pub(crate) retired: usize,
    pub(crate) waiting: usize,
}

impl NativeRetirementReport {
    /// Sessions retired in this call (mapping removed, removes queued).
    pub const fn retired(self) -> usize {
        self.retired
    }

    /// Detached sessions whose view did not prove retirement yet.
    pub const fn waiting(self) -> usize {
        self.waiting
    }
}

/// Report of [`acknowledge_retired_native_records`](crate::SessionRouter::acknowledge_retired_native_records).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeAcknowledgeReport {
    pub(crate) acknowledged: usize,
    pub(crate) removed: usize,
    pub(crate) waiting: usize,
}

impl NativeAcknowledgeReport {
    /// Retired records whose lifecycle cell was acknowledged now.
    pub const fn acknowledged(self) -> usize {
        self.acknowledged
    }

    /// Superseded records removed after their acknowledgement.
    pub const fn removed(self) -> usize {
        self.removed
    }

    /// Records still waiting for pins to reach zero or for a ticket.
    pub const fn waiting(self) -> usize {
        self.waiting
    }
}

/// Outcome of a genuine pose producer call
/// ([`native_player_changed`](crate::SessionRouter::native_player_changed) /
/// [`native_resync`](crate::SessionRouter::native_resync)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePoseOutcome {
    /// The session's pose was updated from the genuine view.
    Updated,
    /// Updated, and a mandatory position sync to the genuine pose was queued.
    Synced,
    /// No native session holds this admission (stale intent, discarded).
    Stale,
}
