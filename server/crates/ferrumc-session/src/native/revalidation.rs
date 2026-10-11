//! Closure-scoped endpoint revalidation (Root R3-A1, .2c amendment R4).
//!
//! The session owns every stored admission handle. The driver supplies one
//! closure that receives the stored handle together with a single-use
//! [`NativeRevalidationProbe`] and may borrow its simulation shard shared or
//! exclusively. It derives the live view inside the closure body and submits
//! it; only [`NativeRevalidationProbe::submit`] can produce a
//! [`NativeRevalidationVerdict`]. The view therefore lives only as long as the
//! shard borrow inside one closure call, never as long as the handle.
//!
//! The contract's first form, `FnMut(&OwnedPlayerHandle) ->
//! Option<ConnectedOwnedPlayerView<'_>>`, does not borrow-check: `'_` binds to
//! the handle argument, so a view borrowed from the captured shard escapes the
//! closure (E0597):
//!
//! ```compile_fail,E0597
//! use ferrumc_math::ShardPos;
//! use ferrumc_sim::{ConnectedOwnedPlayerView, OwnedPlayerHandle, SimShard};
//! fn old(_: &mut dyn FnMut(&OwnedPlayerHandle) -> Option<ConnectedOwnedPlayerView<'_>>) {}
//! fn driver() {
//!     let shard = SimShard::new(ShardPos::new(0, 0));
//!     old(&mut |h| shard.connected_owned_player_view(h).ok());
//! }
//! ```
//!
//! An explicit view lifetime compiles only for a shared capture; with the
//! exclusive shard borrow R3-A1 requires, the captured `&mut` cannot escape the
//! `FnMut` body:
//!
//! ```compile_fail
//! use ferrumc_math::ShardPos;
//! use ferrumc_sim::{ConnectedOwnedPlayerView, OwnedPlayerHandle, OwnedPlayerInput, SimShard};
//! fn explicit<'s>(_: &mut dyn FnMut(&OwnedPlayerHandle) -> Option<ConnectedOwnedPlayerView<'s>>) {}
//! fn driver() {
//!     let mut shard = SimShard::new(ShardPos::new(0, 0));
//!     let exclusive = &mut shard;
//!     explicit(&mut |h| {
//!         let _ = exclusive.enqueue_owned_player_input(OwnedPlayerInput::drain());
//!         exclusive.connected_owned_player_view(h).ok()
//!     });
//! }
//! ```

use ferrumc_core::GameMode;
use ferrumc_math::Vec3;
use ferrumc_sim::{ConnectedOwnedPlayerView, GenerationBinding, OwnedPlayerHandle};

/// Single-use probe for one stored session handle in one revalidation call.
///
/// Not `Clone`, no public constructor; consumed by [`submit`](Self::submit).
/// A probe cannot be submitted twice:
///
/// ```compile_fail,E0382
/// use ferrumc_session::{NativeRevalidationProbe, NativeRevalidationVerdict};
/// fn twice(probe: NativeRevalidationProbe<'_>) -> NativeRevalidationVerdict {
///     let _first = probe.submit(None);
///     probe.submit(None)
/// }
/// ```
///
/// and it cannot escape the closure call it was handed to:
///
/// ```compile_fail
/// use ferrumc_session::{NativeShardReceiver, SessionRouter};
/// fn escape(router: &mut SessionRouter, lane: &NativeShardReceiver) {
///     let mut stash = None;
///     let _ = router.revalidate_native_endpoints(lane, &mut |_, probe| {
///         stash = Some(probe);
///         loop {}
///     });
/// }
/// ```
#[derive(Debug)]
pub struct NativeRevalidationProbe<'p> {
    pub(crate) expected: &'p OwnedPlayerHandle,
    pub(crate) generation: GenerationBinding,
    pub(crate) seq: u64,
}

impl NativeRevalidationProbe<'_> {
    /// Submits the live view the driver derived for the probed handle, or
    /// `None` when the simulation reports the player detached.
    ///
    /// The view is compared here against the stored handle (exact owner,
    /// player and admission) and the registration's full generation; only
    /// scalar pose data leaves this call. This is the only way to obtain a
    /// verdict.
    pub fn submit(self, view: Option<ConnectedOwnedPlayerView<'_>>) -> NativeRevalidationVerdict {
        let outcome = match view {
            None => VerdictOutcome::Detached,
            Some(view)
                if view.player() == self.expected.player()
                    && view.matches_owned_player(self.expected)
                    && view.generation() == self.generation =>
            {
                VerdictOutcome::Matched(LivePose {
                    position: view.position(),
                    look: view.look(),
                    game_mode: view.game_mode(),
                })
            }
            Some(_) => VerdictOutcome::Mismatch,
        };
        NativeRevalidationVerdict {
            seq: self.seq,
            outcome,
        }
    }
}

/// Sealed result of one [`NativeRevalidationProbe::submit`].
///
/// It carries the router-local probe sequence; the session rejects a verdict
/// produced for any other probe. It cannot be forged:
///
/// ```compile_fail
/// use ferrumc_session::NativeRevalidationVerdict;
/// fn forge() -> NativeRevalidationVerdict {
///     NativeRevalidationVerdict { seq: 0 }
/// }
/// ```
#[derive(Debug)]
#[must_use]
pub struct NativeRevalidationVerdict {
    pub(crate) seq: u64,
    pub(crate) outcome: VerdictOutcome,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum VerdictOutcome {
    Matched(LivePose),
    Mismatch,
    Detached,
}

/// Scalar pose copied out of a genuine live view.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LivePose {
    pub(crate) position: Vec3,
    pub(crate) look: [f32; 2],
    pub(crate) game_mode: GameMode,
}

/// Report of one [`revalidate_native_endpoints`](crate::SessionRouter::revalidate_native_endpoints) call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeRevalidation {
    pub(crate) revalidated: usize,
    pub(crate) refused: usize,
    pub(crate) detached: usize,
    pub(crate) open: usize,
}

impl NativeRevalidation {
    /// Sessions rebound to the current receiver.
    pub const fn revalidated(self) -> usize {
        self.revalidated
    }

    /// Sessions whose view did not match (`NativeEndpointRevalidationRefused`);
    /// each was set leaving and stays unbound.
    pub const fn refused(self) -> usize {
        self.refused
    }

    /// Sessions whose simulation player is detached; they stay suspended
    /// without blocking progress.
    pub const fn detached(self) -> usize {
        self.detached
    }

    /// Sessions of this registration that still await revalidation.
    pub const fn open(self) -> usize {
        self.open
    }
}
