//! Retention tokens, the bounded leave-save state machine and its typed
//! quarantine (NET-04.2e2, contract §7.1–§7.3, R2-A1).
//!
//! The session is the single producer of the leave-save and of the release
//! (B2). The driver holds only the opaque [`NativeRetentionToken`]; the record
//! in the session table keeps the original admission handle until release.
//!
//! All counters run on the driver's `applied` value (successfully applied
//! normal boundaries), never on wall time. Every number below is a chosen
//! contract bound, not a measurement.

use std::num::NonZeroU64;

use ferrumc_core::PlayerId;
use ferrumc_sim::{
    DetachedOwnedPlayerStatus, OwnershipPendingStatus, OwnershipTransferId, PlayerOwnershipError,
};

use super::NativeShardInput;

/// Maximum counted leave-save attempts before typed quarantine (§7.3).
pub const MAX_LEAVE_SAVE_ATTEMPTS: u8 = 8;
/// Maximum disconnect resends while the status still reports a connected
/// player (§7.3 point 1). §7.3 amendment V1 (Root): a resend counts only after
/// the previous disconnect left the lane (or was submitted) and at least one
/// normal boundary applied it; a lane backlog never consumes one.
pub const MAX_DISCONNECT_RESENDS: u8 = 8;
/// Applied boundaries before a refused or vanished leave-save is resubmitted.
pub const LEAVE_SAVE_RESUBMIT_DELAY: u64 = 20;
/// Applied boundaries per counted attempt while a pending stays unconfirmed.
pub const LEAVE_SAVE_COUNT_INTERVAL: u64 = 100;
/// Applied boundaries a non-uncertain foreign pending may block before its
/// waiting time starts to count attempts.
pub const LEAVE_SAVE_BLOCKING_GRACE: u64 = 1200;

/// Opaque, copyable retention token handed to the driver at D2 or adoption.
///
/// It carries only the player and the session-assigned admission lineage; it
/// grants no authority, builds no input and names no handle. The lineage comes
/// from a router-local checked monotonic counter and is never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NativeRetentionToken {
    pub(crate) player: PlayerId,
    pub(crate) lineage: NonZeroU64,
}

impl NativeRetentionToken {
    /// The retained player.
    pub const fn player(self) -> PlayerId {
        self.player
    }

    /// The router-local admission lineage (diagnostics only).
    pub const fn lineage(self) -> u64 {
        self.lineage.get()
    }
}

/// Why a leave-save was quarantined (§7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeaveSaveQuarantineReason {
    /// The disconnect was not applied after the bounded resends.
    DisconnectNotApplied,
    /// The status closure reported a stale or foreign handle for a record that
    /// is neither superseded nor released (Root: stale and foreign alike).
    StatusInvariant,
    /// The record's own leave-save pending was quarantined by the simulation.
    PendingQuarantined,
    /// The foreign pending blocking the leave-save was quarantined.
    BlockingQuarantined,
    /// The counted attempts exceeded [`MAX_LEAVE_SAVE_ATTEMPTS`].
    AttemptsExhausted,
}

/// Typed, visible leave-save quarantine of one retention record.
///
/// The driver quarantines the player's L1 reservation; token, record and the
/// simulation player stay until the final save of the stop protocol. A later
/// genuine confirmation still allows the release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaveSaveQuarantine {
    pub(crate) player: PlayerId,
    pub(crate) reason: LeaveSaveQuarantineReason,
    pub(crate) attempts: u8,
}

impl LeaveSaveQuarantine {
    /// The quarantined player.
    pub const fn player(self) -> PlayerId {
        self.player
    }

    /// The typed reason.
    pub const fn reason(self) -> LeaveSaveQuarantineReason {
        self.reason
    }

    /// Counted attempts at the time of quarantine.
    pub const fn attempts(self) -> u8 {
        self.attempts
    }
}

/// One step of [`service_native_retention`](crate::SessionRouter::service_native_retention).
#[derive(Debug)]
#[non_exhaustive]
#[allow(
    clippy::large_enum_variant,
    reason = "returned once per serviced record and moved straight into the simulation queue"
)]
pub enum NativeRetentionStep {
    /// A session-built gated class-2 input (leave-save autosave or the
    /// disconnect of an adopted admission). If the simulation refuses its
    /// enqueue, the driver calls
    /// [`native_retention_unsent`](crate::SessionRouter::native_retention_unsent).
    Submit(NativeShardInput),
    /// Nothing to do in this boundary.
    Waiting,
    /// `saved_after_detach` of this admission is present (D9 for L1).
    SaveConfirmed,
    /// Retries exhausted or a status invariant: typed quarantine.
    Quarantined(LeaveSaveQuarantine),
    /// A genuine rejoin superseded this record; the driver ends its token.
    /// No leave-save and no release follow; the record waits only for D7/D8.
    Superseded,
}

/// Read-only phase of one retention record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRetentionPhase {
    /// Disconnect requested, no detach fact confirmed yet.
    Leaving,
    /// The genuine detach fact of the original admission was confirmed (D3).
    Detached,
    /// Retired (D5 or unbound retirement); `acknowledged` after D7/D8.
    Retired {
        /// The lifecycle cell was acknowledged (D7/D8).
        acknowledged: bool,
    },
    /// A genuine rejoin superseded the record (§7.4).
    Superseded {
        /// The old lifecycle cell was acknowledged.
        acknowledged: bool,
    },
}

/// Read-only leave-save progress of one retention record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLeaveSaveState {
    /// Waiting until the disconnect is applied.
    AwaitDetach,
    /// A leave-save may be submitted.
    Due,
    /// A leave-save was submitted and is not yet observed applied.
    Submitted,
    /// The record's own leave-save pending is in flight.
    Pending,
    /// A foreign pending of the same player blocks the leave-save.
    Waiting,
    /// `saved_after_detach` is present.
    Confirmed,
    /// Typed quarantine.
    Quarantined(LeaveSaveQuarantineReason),
}

/// Read-only diagnostics of one retention record; grants no authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRetentionStatus {
    pub(crate) phase: NativeRetentionPhase,
    pub(crate) leave_save: NativeLeaveSaveState,
    pub(crate) attempts: u8,
    pub(crate) bound: bool,
}

impl NativeRetentionStatus {
    /// The record phase.
    pub const fn phase(self) -> NativeRetentionPhase {
        self.phase
    }

    /// The leave-save state.
    pub const fn leave_save(self) -> NativeLeaveSaveState {
        self.leave_save
    }

    /// Counted leave-save attempts.
    pub const fn attempts(self) -> u8 {
        self.attempts
    }

    /// `false` for an adopted, never bound admission (G3).
    pub const fn bound(self) -> bool {
        self.bound
    }
}

/// The internal leave-save state (§7.3); `at`/`since`/`sent_at` are `applied`
/// values of the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeaveSave {
    /// `sent_at`: the `applied` value at which the session first saw the last
    /// disconnect handed to the simulation, a lane disconnect taken from its
    /// lane or a gated disconnect submitted (§7.3 amendment V1, Root).
    AwaitDetach {
        sent_at: Option<u64>,
        resends: u8,
    },
    Due {
        not_before: u64,
    },
    Submitted {
        id: OwnershipTransferId,
        at: u64,
    },
    Pending {
        id: OwnershipTransferId,
        since: u64,
        counted: u64,
    },
    Waiting {
        since: u64,
        blocking: OwnershipTransferId,
        counted: u64,
    },
    Confirmed,
    Quarantined(LeaveSaveQuarantineReason),
}

/// The last submit the session handed out, for `native_retention_unsent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmitKind {
    Disconnect { resend: bool },
    Autosave,
}

/// How the record's disconnect reaches the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisconnectPath {
    /// A mapped session: the disconnect travels on the lane.
    Lane(LaneDisconnect),
    /// No mapped session (adopted or retired): the disconnect is a gated submit.
    Submit,
}

/// Where a mapped session's last lane disconnect is (§7.3 amendment V1, Root).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaneDisconnect {
    /// Retained in the session; the router retries it.
    Retained,
    /// In the lane behind other items; the receiver has not taken it yet.
    Queued,
    /// Taken by the receiver (handed to the simulation, §6.2), or no lane
    /// disconnect is needed any more.
    Taken,
}

/// The quarantine a stop phase A final save renewed (F1, round 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FinalSave {
    reason: LeaveSaveQuarantineReason,
    attempts: u8,
}

/// What the router must do after one state-machine step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    SubmitDisconnect,
    SubmitAutosave(OwnershipTransferId),
    ResendLaneDisconnect,
    Waiting,
    Confirmed,
    Quarantined(LeaveSaveQuarantineReason),
    StatusInvariant,
}

/// The retention part of a native table entry: lineage plus leave-save.
#[derive(Debug)]
pub(crate) struct Retention {
    pub(crate) lineage: NonZeroU64,
    pub(crate) save: LeaveSave,
    pub(crate) attempts: u8,
    pub(crate) last_submit: Option<SubmitKind>,
    /// Set once the stop phase A final save was renewed (at most once).
    pub(crate) final_save: Option<FinalSave>,
}

impl Retention {
    pub(crate) const fn new(lineage: NonZeroU64) -> Self {
        Self {
            lineage,
            save: LeaveSave::AwaitDetach {
                sent_at: None,
                resends: 0,
            },
            attempts: 0,
            last_submit: None,
            final_save: None,
        }
    }

    pub(crate) const fn state(&self) -> NativeLeaveSaveState {
        match self.save {
            LeaveSave::AwaitDetach { .. } => NativeLeaveSaveState::AwaitDetach,
            LeaveSave::Due { .. } => NativeLeaveSaveState::Due,
            LeaveSave::Submitted { .. } => NativeLeaveSaveState::Submitted,
            LeaveSave::Pending { .. } => NativeLeaveSaveState::Pending,
            LeaveSave::Waiting { .. } => NativeLeaveSaveState::Waiting,
            LeaveSave::Confirmed => NativeLeaveSaveState::Confirmed,
            LeaveSave::Quarantined(reason) => NativeLeaveSaveState::Quarantined(reason),
        }
    }

    pub(crate) const fn confirmed(&self) -> bool {
        matches!(self.save, LeaveSave::Confirmed)
    }

    /// One bounded step of §7.3, in the contract's decision order.
    pub(crate) fn step(
        &mut self,
        applied: u64,
        submit_allowed: bool,
        path: DisconnectPath,
        status: &Result<DetachedOwnedPlayerStatus, PlayerOwnershipError>,
        ids: &mut dyn FnMut() -> OwnershipTransferId,
    ) -> Decision {
        self.last_submit = None;
        let status = match status {
            Ok(status) => *status,
            // 1. The disconnect is not applied yet.
            Err(PlayerOwnershipError::Ineligible("connected player")) => {
                return self.await_detach(applied, submit_allowed, path);
            }
            // Root decision: stale and foreign handles alike are quarantine,
            // never success and never silent loss.
            Err(PlayerOwnershipError::StaleHandle) => {
                return match self.save {
                    LeaveSave::Quarantined(reason) => Decision::Quarantined(reason),
                    _ => self.quarantine(LeaveSaveQuarantineReason::StatusInvariant),
                };
            }
            // SnapshotOnly, NotInitialized, scope or any other refusal: a
            // wiring break; the driver fail-stops, the record is unchanged.
            Err(_) => return Decision::StatusInvariant,
        };
        // A later genuine clarification may still confirm a quarantined record.
        if status.saved_after_detach().is_some() {
            self.save = LeaveSave::Confirmed;
            return Decision::Confirmed;
        }
        match self.save {
            LeaveSave::Confirmed => Decision::Confirmed,
            LeaveSave::Quarantined(reason) => Decision::Quarantined(reason),
            LeaveSave::AwaitDetach { .. } => {
                self.save = LeaveSave::Due { not_before: 0 };
                self.due(applied, submit_allowed, status, ids)
            }
            LeaveSave::Due { not_before } => {
                if applied < not_before {
                    Decision::Waiting
                } else {
                    self.due(applied, submit_allowed, status, ids)
                }
            }
            LeaveSave::Submitted { id, at } => self.submitted(applied, id, at, status),
            LeaveSave::Pending { id, since, counted } => {
                self.pending(applied, id, since, counted, status)
            }
            LeaveSave::Waiting { since, counted, .. } => {
                self.waiting(applied, submit_allowed, since, counted, status, ids)
            }
        }
    }

    fn await_detach(
        &mut self,
        applied: u64,
        submit_allowed: bool,
        path: DisconnectPath,
    ) -> Decision {
        let LeaveSave::AwaitDetach { sent_at, resends } = self.save else {
            return match (self.save, self.final_save) {
                (LeaveSave::Quarantined(reason), _) => Decision::Quarantined(reason),
                // F1 (round 2): the final save renewed a `DisconnectNotApplied`
                // record whose player is still connected. That is legitimate,
                // no wiring break: the record returns to its quarantine without
                // an attempt and without a resend; the driver goes on.
                (
                    LeaveSave::Due { .. },
                    Some(FinalSave {
                        reason: LeaveSaveQuarantineReason::DisconnectNotApplied,
                        attempts,
                    }),
                ) => {
                    self.attempts = attempts;
                    self.quarantine(LeaveSaveQuarantineReason::DisconnectNotApplied)
                }
                // Any other record left `AwaitDetach` only on a detached
                // status, and a detached admission never becomes connected.
                _ => Decision::StatusInvariant,
            };
        };
        match (path, sent_at) {
            // A retained lane disconnect is retried by the router itself.
            // §7.3 amendment V1 (Root): a disconnect still queued behind other
            // lane items consumes no resend, however many boundaries pass.
            (DisconnectPath::Lane(LaneDisconnect::Retained | LaneDisconnect::Queued), _) => {
                Decision::Waiting
            }
            (DisconnectPath::Lane(LaneDisconnect::Taken), None) => {
                self.save = LeaveSave::AwaitDetach {
                    sent_at: Some(applied),
                    resends,
                };
                Decision::Waiting
            }
            (DisconnectPath::Submit, None) => {
                if !submit_allowed {
                    return Decision::Waiting;
                }
                self.save = LeaveSave::AwaitDetach {
                    sent_at: Some(applied),
                    resends,
                };
                self.last_submit = Some(SubmitKind::Disconnect { resend: false });
                Decision::SubmitDisconnect
            }
            (path, Some(at)) => {
                if applied <= at {
                    return Decision::Waiting;
                }
                if resends >= MAX_DISCONNECT_RESENDS {
                    return self.quarantine(LeaveSaveQuarantineReason::DisconnectNotApplied);
                }
                match path {
                    DisconnectPath::Submit if !submit_allowed => Decision::Waiting,
                    DisconnectPath::Submit => {
                        self.save = LeaveSave::AwaitDetach {
                            sent_at: Some(applied),
                            resends: resends + 1,
                        };
                        self.last_submit = Some(SubmitKind::Disconnect { resend: true });
                        Decision::SubmitDisconnect
                    }
                    // The resend counts once it is taken and applied in turn
                    // (§7.3 amendment V1); until then `sent_at` stays open.
                    DisconnectPath::Lane(_) => {
                        self.save = LeaveSave::AwaitDetach {
                            sent_at: None,
                            resends: resends + 1,
                        };
                        Decision::ResendLaneDisconnect
                    }
                }
            }
        }
    }

    fn due(
        &mut self,
        applied: u64,
        submit_allowed: bool,
        status: DetachedOwnedPlayerStatus,
        ids: &mut dyn FnMut() -> OwnershipTransferId,
    ) -> Decision {
        match status.blocking() {
            None if !submit_allowed => Decision::Waiting,
            None => {
                if self.attempts >= MAX_LEAVE_SAVE_ATTEMPTS {
                    return self.quarantine(LeaveSaveQuarantineReason::AttemptsExhausted);
                }
                self.attempts += 1;
                let id = ids();
                self.save = LeaveSave::Submitted { id, at: applied };
                self.last_submit = Some(SubmitKind::Autosave);
                Decision::SubmitAutosave(id)
            }
            Some((_, OwnershipPendingStatus::Quarantined)) => {
                self.quarantine(LeaveSaveQuarantineReason::BlockingQuarantined)
            }
            // R2-A1: a foreign pending blocks; waiting is no attempt.
            Some((blocking, _)) => {
                self.save = LeaveSave::Waiting {
                    since: applied,
                    blocking,
                    counted: 0,
                };
                Decision::Waiting
            }
        }
    }

    fn submitted(
        &mut self,
        applied: u64,
        id: OwnershipTransferId,
        at: u64,
        status: DetachedOwnedPlayerStatus,
    ) -> Decision {
        // Only a normal boundary after the submit applied the whole queue.
        if applied <= at {
            return Decision::Waiting;
        }
        match status.blocking() {
            Some((own, OwnershipPendingStatus::Quarantined)) if own == id => {
                self.quarantine(LeaveSaveQuarantineReason::PendingQuarantined)
            }
            Some((own, _)) if own == id => {
                self.save = LeaveSave::Pending {
                    id,
                    since: applied,
                    counted: 0,
                };
                Decision::Waiting
            }
            // Refused with `Reserved` by a foreign pending: waiting does not count.
            Some((blocking, _)) => {
                self.attempts = self.attempts.saturating_sub(1);
                self.save = LeaveSave::Waiting {
                    since: applied,
                    blocking,
                    counted: 0,
                };
                Decision::Waiting
            }
            // Refused for another reason (for example capacity): the attempt counts.
            None => {
                self.save = LeaveSave::Due {
                    not_before: applied.saturating_add(LEAVE_SAVE_RESUBMIT_DELAY),
                };
                Decision::Waiting
            }
        }
    }

    fn pending(
        &mut self,
        applied: u64,
        id: OwnershipTransferId,
        since: u64,
        counted: u64,
        status: DetachedOwnedPlayerStatus,
    ) -> Decision {
        match status.blocking() {
            Some((own, OwnershipPendingStatus::Quarantined)) if own == id => {
                self.quarantine(LeaveSaveQuarantineReason::PendingQuarantined)
            }
            Some((own, _)) if own == id => {
                let intervals = applied.saturating_sub(since) / LEAVE_SAVE_COUNT_INTERVAL;
                self.save = LeaveSave::Pending {
                    id,
                    since,
                    counted: counted.max(intervals),
                };
                self.count(intervals.saturating_sub(counted))
            }
            // The own pending vanished without confirmation (definitive refusal).
            _ => {
                self.save = LeaveSave::Due {
                    not_before: applied.saturating_add(LEAVE_SAVE_RESUBMIT_DELAY),
                };
                Decision::Waiting
            }
        }
    }

    fn waiting(
        &mut self,
        applied: u64,
        submit_allowed: bool,
        since: u64,
        counted: u64,
        status: DetachedOwnedPlayerStatus,
        ids: &mut dyn FnMut() -> OwnershipTransferId,
    ) -> Decision {
        let Some((current, phase)) = status.blocking() else {
            // Resumption as soon as nothing blocks.
            self.save = LeaveSave::Due { not_before: 0 };
            return self.due(applied, submit_allowed, status, ids);
        };
        let waited = applied.saturating_sub(since);
        let intervals = match phase {
            OwnershipPendingStatus::Quarantined => {
                return self.quarantine(LeaveSaveQuarantineReason::BlockingQuarantined);
            }
            OwnershipPendingStatus::Uncertain => waited / LEAVE_SAVE_COUNT_INTERVAL,
            OwnershipPendingStatus::Prepared
            | OwnershipPendingStatus::InFlight
            | OwnershipPendingStatus::NeedsRefresh => {
                waited.saturating_sub(LEAVE_SAVE_BLOCKING_GRACE) / LEAVE_SAVE_COUNT_INTERVAL
            }
        };
        self.save = LeaveSave::Waiting {
            since,
            blocking: current,
            counted: counted.max(intervals),
        };
        self.count(intervals.saturating_sub(counted))
    }

    fn count(&mut self, new_attempts: u64) -> Decision {
        let add = u8::try_from(new_attempts).unwrap_or(u8::MAX);
        self.attempts = self.attempts.saturating_add(add);
        if self.attempts > MAX_LEAVE_SAVE_ATTEMPTS {
            self.quarantine(LeaveSaveQuarantineReason::AttemptsExhausted)
        } else {
            Decision::Waiting
        }
    }

    fn quarantine(&mut self, reason: LeaveSaveQuarantineReason) -> Decision {
        self.save = LeaveSave::Quarantined(reason);
        Decision::Quarantined(reason)
    }

    /// Rolls back the submit of the last step after a simulation enqueue
    /// refusal; the attempt does not count. Returns `false` when there is none.
    pub(crate) fn unsent(&mut self) -> bool {
        match (self.last_submit.take(), self.save) {
            (Some(SubmitKind::Autosave), LeaveSave::Submitted { .. }) => {
                self.attempts = self.attempts.saturating_sub(1);
                self.save = LeaveSave::Due { not_before: 0 };
                true
            }
            (Some(SubmitKind::Disconnect { resend }), LeaveSave::AwaitDetach { resends, .. }) => {
                self.save = if resend {
                    // Retry the resend at the next allowed submit.
                    LeaveSave::AwaitDetach {
                        sent_at: Some(0),
                        resends: resends.saturating_sub(1),
                    }
                } else {
                    LeaveSave::AwaitDetach {
                        sent_at: None,
                        resends,
                    }
                };
                true
            }
            _ => false,
        }
    }

    /// Stop phase A final save: one renewed attempt for a quarantined record.
    ///
    /// A renewed `DisconnectNotApplied` record whose player is still connected
    /// returns to that quarantine at its next step, without an attempt (F1).
    pub(crate) fn renew_final_save(&mut self) -> bool {
        let LeaveSave::Quarantined(reason) = self.save else {
            return false;
        };
        if self.final_save.is_some() {
            return false;
        }
        self.final_save = Some(FinalSave {
            reason,
            attempts: self.attempts,
        });
        self.attempts = MAX_LEAVE_SAVE_ATTEMPTS - 1;
        self.save = LeaveSave::Due { not_before: 0 };
        true
    }
}
