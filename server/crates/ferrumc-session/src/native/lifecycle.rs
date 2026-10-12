//! Closure-scoped lifecycle probes for D3, D5, D7/D8 and the unbound
//! retirement (Root R3-A1 form, Root decision 2(a) of NET-04.2e2).
//!
//! The session owns every original admission handle. The driver supplies one
//! closure that receives the stored handle together with a single-use
//! [`NativeLifecycleProbe`]; it derives the genuine
//! [`CollectorLifecycleView`](ferrumc_sim::CollectorLifecycleView) from its
//! shard borrow inside the closure body and submits it. Only scalar facts and,
//! for D7/D8, the owned lifecycle ticket leave the view. D7/D8 is one closure:
//! the verdict holds the ticket and [`NativeLifecycleVerdict::acknowledge`]
//! lends it to the driver's acknowledgement exactly once, after the view's
//! shared borrow has ended:
//!
//! ```no_run
//! use ferrumc_session::{NativeShardReceiver, SessionRouter};
//! use ferrumc_sim::SimShard;
//! struct NativeShardSlot {
//!     shard: SimShard,
//!     lane: NativeShardReceiver,
//! }
//! fn d7_d8(router: &mut SessionRouter, slot: &mut NativeShardSlot) {
//!     let _lane = &slot.lane;
//!     let shard = &mut slot.shard;
//!     let _ = router.acknowledge_retired_native_records(&mut |handle, probe| {
//!         probe
//!             .submit(shard.collector_lifecycle_view(handle).ok())
//!             .acknowledge(|ticket| shard.acknowledge_collector_lifecycle(ticket))
//!     });
//! }
//! ```
//!
//! The contract form `FnMut(&OwnedPlayerHandle) -> Option<CollectorLifecycleView<'_>>`
//! does not borrow-check for the same reason as the revalidation form (E0597):
//!
//! ```compile_fail,E0597
//! use ferrumc_math::ShardPos;
//! use ferrumc_sim::{CollectorLifecycleView, OwnedPlayerHandle, SimShard};
//! fn old(_: &mut dyn FnMut(&OwnedPlayerHandle) -> Option<CollectorLifecycleView<'_>>) {}
//! fn driver() {
//!     let shard = SimShard::new(ShardPos::new(0, 0));
//!     old(&mut |h| shard.collector_lifecycle_view(h).ok());
//! }
//! ```

use ferrumc_sim::{
    CollectorLifecycleTicket, CollectorLifecycleView, GenerationBinding, OwnedPlayerHandle,
    PlayerOwnershipError,
};

/// Single-use probe for one stored original handle in one lifecycle call.
///
/// Not `Clone`, no public constructor; consumed by [`submit`](Self::submit):
///
/// ```compile_fail,E0382
/// use ferrumc_session::{NativeLifecycleProbe, NativeLifecycleVerdict};
/// fn twice(probe: NativeLifecycleProbe<'_>) -> NativeLifecycleVerdict {
///     let _first = probe.submit(None);
///     probe.submit(None)
/// }
/// ```
///
/// It cannot escape the closure call it was handed to:
///
/// ```compile_fail
/// use ferrumc_session::SessionRouter;
/// fn escape(router: &mut SessionRouter) {
///     let mut stash = None;
///     let _ = router.retire_detached_native_sessions(&mut |_, probe| {
///         stash = Some(probe);
///         loop {}
///     });
/// }
/// ```
#[derive(Debug)]
pub struct NativeLifecycleProbe<'p> {
    pub(crate) expected: &'p OwnedPlayerHandle,
    pub(crate) generation: GenerationBinding,
    pub(crate) seq: u64,
    /// The router authorized D7/D8 for this record; only then is the ticket taken.
    pub(crate) ack: bool,
}

impl NativeLifecycleProbe<'_> {
    /// Submits the genuine lifecycle view the driver derived for the probed
    /// original handle, or `None` when the simulation refused it.
    ///
    /// The view is compared here against the stored handle (exact owner,
    /// player and admission) and the registration's full generation; the
    /// detach fact must name the same admission and generation. This is the
    /// only way to obtain a verdict.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "consuming the view ends the driver's shared shard borrow before acknowledge"
    )]
    pub fn submit(self, view: Option<CollectorLifecycleView<'_>>) -> NativeLifecycleVerdict {
        let facts = view.as_ref().map(|view| {
            let matches =
                view.matches_owned_player(self.expected) && view.generation() == self.generation;
            let detached = matches
                && view.detach().is_some_and(|fact| {
                    fact.player() == self.expected.player()
                        && fact.matches_owned_player(self.expected)
                        && fact.generation() == self.generation
                });
            let live = matches
                && view
                    .live()
                    .is_some_and(|live| live.matches_owned_player(self.expected));
            LifecycleFacts {
                matches,
                detached,
                live,
                reserved_pins: view.reserved_take_pins(),
                retained_pins: view.retained_take_pins(),
            }
        });
        let ticket = match (&view, facts) {
            (Some(view), Some(facts)) if self.ack && facts.detached => view.retirement_ticket(),
            _ => None,
        };
        NativeLifecycleVerdict {
            seq: self.seq,
            facts,
            ticket,
            acknowledged: None,
        }
    }
}

/// Scalar facts copied out of one genuine lifecycle view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LifecycleFacts {
    pub(crate) matches: bool,
    pub(crate) detached: bool,
    pub(crate) live: bool,
    pub(crate) reserved_pins: usize,
    pub(crate) retained_pins: usize,
}

/// Sealed result of one [`NativeLifecycleProbe::submit`].
///
/// It carries the router-local probe sequence; a verdict of another probe is
/// rejected. It cannot be forged:
///
/// ```compile_fail
/// use ferrumc_session::NativeLifecycleVerdict;
/// fn forge() -> NativeLifecycleVerdict {
///     NativeLifecycleVerdict { seq: 0 }
/// }
/// ```
///
/// and the lent lifecycle ticket cannot be smuggled out of the acknowledgement:
///
/// ```compile_fail
/// use ferrumc_session::NativeLifecycleVerdict;
/// use ferrumc_sim::CollectorLifecycleTicket;
/// fn smuggle(verdict: NativeLifecycleVerdict) -> Option<&'static CollectorLifecycleTicket> {
///     let mut stash = None;
///     let _ = verdict.acknowledge(|ticket| {
///         stash = Some(ticket);
///         Ok(())
///     });
///     stash
/// }
/// ```
#[derive(Debug)]
#[must_use]
pub struct NativeLifecycleVerdict {
    pub(crate) seq: u64,
    pub(crate) facts: Option<LifecycleFacts>,
    pub(crate) ticket: Option<CollectorLifecycleTicket>,
    pub(crate) acknowledged: Option<bool>,
}

impl NativeLifecycleVerdict {
    /// D8: lends the owned lifecycle ticket to the driver's
    /// `acknowledge_collector_lifecycle` exactly once, if the router authorized
    /// the acknowledgement and the view minted a ticket (detached, source pins
    /// zero). Without a ticket the closure is not called. The outcome is
    /// recorded in the verdict for the router.
    pub fn acknowledge(
        mut self,
        ack: impl FnOnce(&CollectorLifecycleTicket) -> Result<(), PlayerOwnershipError>,
    ) -> Self {
        if let Some(ticket) = self.ticket.take() {
            self.acknowledged = Some(ack(&ticket).is_ok());
        }
        self
    }
}
