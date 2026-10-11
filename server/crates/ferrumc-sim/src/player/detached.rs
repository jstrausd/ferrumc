//! Readonly status and authority-free receipts for detached native players.
//!
//! These values describe actual retained Sim state at the time of the call;
//! none of them carries a handle, query, lease or other mutation authority.

use ferrumc_core::PlayerId;
use ferrumc_storage::OwnershipTransferId;

use super::OwnershipPendingStatus;

/// Pure data about one detached admission; private fields, getters only.
///
/// `saved_after_detach` is set exclusively by Sim when it applies the genuine
/// confirmed autosave completion of exactly this admission while detached.
/// The latest such confirmation wins; any later mutation of the player and any
/// newly admitted pending of the player clear it again.
///
/// ```compile_fail
/// use ferrumc_sim::DetachedOwnedPlayerStatus;
/// fn forge() -> DetachedOwnedPlayerStatus {
///     DetachedOwnedPlayerStatus { saved_after_detach: None, blocking: None, dirty: false }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetachedOwnedPlayerStatus {
    pub(crate) saved_after_detach: Option<OwnershipTransferId>,
    pub(crate) blocking: Option<(OwnershipTransferId, OwnershipPendingStatus)>,
    pub(crate) dirty: bool,
}
impl DetachedOwnedPlayerStatus {
    /// Returns the latest autosave confirmed after detach for this admission.
    pub const fn saved_after_detach(&self) -> Option<OwnershipTransferId> {
        self.saved_after_detach
    }
    /// Returns the player's retained pending operation and its actual phase.
    pub const fn blocking(&self) -> Option<(OwnershipTransferId, OwnershipPendingStatus)> {
        self.blocking
    }
    /// Reports unsaved native changes, including not yet applied deferred values.
    pub const fn dirty(&self) -> bool {
        self.dirty
    }
}

/// Authority-free receipt of a completed detached release.
///
/// It names only the released player and the confirmed native bytes that left
/// the owner's player accounting; there is no public constructor.
///
/// ```compile_fail
/// use ferrumc_sim::ReleasedOwnedPlayer;
/// fn forge(receipt: &ReleasedOwnedPlayer) -> ReleasedOwnedPlayer {
///     ReleasedOwnedPlayer { player: receipt.player(), freed_player_bytes: 0 }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleasedOwnedPlayer {
    pub(crate) player: PlayerId,
    pub(crate) freed_player_bytes: usize,
}
impl ReleasedOwnedPlayer {
    /// Returns the UUID identity whose native state was removed.
    pub const fn player(&self) -> PlayerId {
        self.player
    }
    /// Returns the confirmed encoded bytes removed from the owner's player accounting.
    pub const fn freed_player_bytes(&self) -> usize {
        self.freed_player_bytes
    }
}

/// Typed release-gate refusal; the shard is left completely unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DetachedReleaseRefusal {
    /// The current admission is still connected.
    #[error("player is still connected")]
    Connected,
    /// The owner never selected collector lifecycle mode, so no cell was acknowledged.
    #[error("collector lifecycle is not initialized")]
    LifecycleNotInitialized,
    /// A concrete queued input still names this player.
    #[error("an ownership input for this player is queued")]
    QueuedInput,
    /// A reserved or retained Take publication names this player as collector.
    #[error("a publication pin names this player")]
    PublicationPin,
    /// A pending item/pair operation of this player holds the entity owner
    /// reservation, including its affected chunks.
    #[error("an item or chunk reservation belongs to this player")]
    ItemReservation,
    /// Another pending operation, including bootstrap, belongs to this player.
    #[error("a pending operation belongs to this player")]
    Pending,
    /// The lifecycle cell of this admission was not genuinely acknowledged.
    #[error("the admission lifecycle cell is not acknowledged")]
    LifecycleNotAcknowledged,
    /// Another retained lifecycle cell still binds this player as collector.
    #[error("another lifecycle cell binds this player")]
    CollectorBinding,
    /// The native state is dirty or holds deferred values.
    #[error("detached player has unsaved changes")]
    Unsaved,
    /// No autosave of this admission was confirmed after detach.
    #[error("no confirmed save after detach")]
    NotSaved,
}
